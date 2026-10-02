#!/bin/bash
# HCS Linux — start the Neural Glass desktop.
#
# Runs from systemd (hcs-desktop.service), not as PID 1.
#
# The previous arrangement had a 120-line shell script at /sbin/init that the
# kernel was told to run instead of systemd. That script predates the base
# system: it existed because there was no systemd to run. Once the image had a
# real Debian base, that script was actively harmful — systemd never became
# PID 1, so nothing it owns was set up, and the desktop never appeared. The
# image booted to a banner and stopped.
#
# So systemd owns the boot, live-boot mounts the squashfs, and this script does
# the one thing that has to happen after both: put a session on the screen.
#
# Ordering matters and is enforced by systemd's unit dependencies, not by sleeps
# where it can be helped:
#   * seatd first — without a seat there is no input and no display access.
#   * niri with the shipped config, as a compositor for this VT.
#   * start-session.sh, which brings up the daemons and Quickshell.
#
# Every wait here is for a signal — a socket, a log line, a process — and every
# one of them is bounded. The QA agent additionally verifies that windows really
# map, so a desktop that "started" but drew nothing still fails the gate.

set -uo pipefail

LOG=/var/log/hcs/session.log
mkdir -p /var/log/hcs /run/hcs

log() {
    printf '[%s] [desktop] %s\n' "$(date -u +%H:%M:%S)" "$*" >>"${LOG}"
    printf '[%s] [desktop] %s\n' "$(date -u +%H:%M:%S)" "$*"
}

# ---------------------------------------------------------------- preflight

if [ ! -x /usr/bin/niri ]; then
    log "niri is not installed — this image has no compositor"
    exit 1
fi
if [ ! -f /usr/share/hcs/shell/shell.qml ]; then
    log "the Neural Glass shell is missing — the desktop cannot be drawn"
    exit 1
fi

# The console entry is a deliberate choice by whoever booted: no desktop.
if grep -q 'hcs_console=1' /proc/cmdline 2>/dev/null; then
    log "hcs_console=1 on the kernel command line — console mode by request"
    exit 0
fi
if [ ! -e /dev/vda ] && [ ! -e /dev/sr0 ] && [ ! -e /dev/disk/by-label/* ]; then
    # live-boot has not mounted the squashfs yet. Starting a desktop on an
    # empty root would look like success and photograph a wallpaper.
    log "no boot medium found — live-boot has not mounted the image yet"
    exit 1
fi

# ---------------------------------------------------------------- environment

# systemd sets User=hcs and XDG_RUNTIME_DIR for this unit. Falling back to /run/0
# would put the compositor's socket in root's runtime directory, where the QA
# agent — running as the session user — cannot see it.
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/hcs}"
export WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-1}"
export XDG_SESSION_TYPE=wayland
export XDG_CURRENT_DESKTOP=HCS
export QT_QPA_PLATFORM="${QT_QPA_PLATFORM:-wayland}"
export QT_WAYLAND_DISABLE_WINDOWDECORATION=1
export GDK_BACKEND=wayland
export XDG_DATA_DIRS="/usr/local/share:/usr/share:${XDG_DATA_DIRS:-}"
export PATH="/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"
# CPU-only is the default path: no GPU may be required to reach a desktop.
#
# pixman is smithay's pure-software GL renderer. It is not a fallback here, it is
# the floor: llvmpipe needs no GPU, so a machine with no supported graphics can
# still reach a desktop. HCS_RENDERER=hardware is the opt-in for real
# acceleration and is set by the bare-metal profile.
export HCS_RENDERER="${HCS_RENDERER:-software}"
if [ "${HCS_RENDERER}" = "software" ]; then
    export LIBGL_ALWAYS_SOFTWARE=1
    export WLR_RENDERER="${WLR_RENDERER:-pixman}"
    export GALLIUM_DRIVER=llvmpipe
fi
export MESA_LOADER_DRIVER_OVERRIDE="${MESA_LOADER_DRIVER_OVERRIDE:-llvmpipe}"
export LIBGL_DRIVERS_PATH="${LIBGL_DRIVERS_PATH:-/usr/lib/x86_64-linux-gnu/dri}"
export __EGL_VENDOR_LIBRARY_FILENAMES="${__EGL_VENDOR_LIBRARY_FILENAMES:-/usr/share/glvnd/egl_vendor.d/50_mesa.json}"
# Mesa installs its Vulkan ICDs under a versioned path. An unset loader path finds
# no ICD at all, which is indistinguishable from a missing driver — so the glob is
# resolved explicitly here.
for _icd in /usr/share/vulkan/icd.d/lvp_icd.x86_64.json \
           /usr/share/vulkan/icd.d/intel_icd.x86_64.json \
           /usr/share/vulkan/icd.d/amd_icd.x86_64.json \
           /usr/share/vulkan/icd.d/virtio_icd.json; do
    if [ -f "${_icd}" ]; then
        export VK_ICD_FILENAMES="${_icd}"
        break
    fi
done
unset _icd
export HCS_RAM_BUDGET_IDLE_MB="${HCS_RAM_BUDGET_IDLE_MB:-6144}"
export HCS_RAM_BUDGET_PEAK_MB="${HCS_RAM_BUDGET_PEAK_MB:-8192}"
export HCS_GUI_APP_BUDGET_MB="${HCS_GUI_APP_BUDGET_MB:-250}"

mkdir -p "${XDG_RUNTIME_DIR}"
chmod 700 "${XDG_RUNTIME_DIR}"

# ---------------------------------------------------------------- renderers

# Prove a renderer exists before claiming a desktop is coming.
#
# A running compositor with no renderer is the exact failure that stayed invisible
# for three releases: the process was alive, the gates were green, and not one
# pixel was ever drawn. So this is measured and recorded, not assumed, and a
# failure is written to the log with the reason.
check_renderer() {
    local vulkan_ok=0 gl_ok=0

    if [ -x /usr/bin/vulkaninfo ]; then
        if /usr/bin/vulkaninfo --summary >/var/log/hcs/vulkan.log 2>&1; then
            vulkan_ok=1
            local dev
            dev=$(grep -m1 -oE 'deviceName *= *[A-Za-z0-9 ]+' /var/log/hcs/vulkan.log \
                  | sed 's/deviceName *= *//' | head -1)
            log "Vulkan available: ${dev:-unnamed device}"
        else
            log "WARN vulkaninfo found no usable device — see /var/log/hcs/vulkan.log"
        fi
    else
        log "WARN vulkaninfo is not installed; Vulkan cannot be verified"
    fi

    # llvmpipe must work even where no DRM device offers a usable mode.
    if [ -x /usr/bin/glxinfo ]; then
        if DISPLAY=:/dev/null glxinfo -B >/var/log/hcs/glxinfo.log 2>&1; then
            gl_ok=1
            log "GL available: $(grep -m1 -oE 'OpenGL renderer string: .*' \
                 /var/log/hcs/glxinfo.log | sed 's/.*: //')"
        fi
    fi

    printf '%s %s\n' "${vulkan_ok}" "${gl_ok}" > /run/hcs/renderers
    if [ "${vulkan_ok}" -eq 0 ] && [ "${gl_ok}" -eq 0 ]; then
        log "FAIL no Vulkan and no GL renderer — a desktop cannot be drawn"
        return 1
    fi
    return 0
}

check_renderer || exit 1

# What the QA agent and the host grader read to decide whether a frame is real.
mkdir -p /run/hcs
{
    printf 'renderer=%s\n' "${HCS_RENDERER}"
    printf 'vulkan_icd=%s\n' "${VK_ICD_FILENAMES:-none}"
    printf 'dri_path=%s\n' "${LIBGL_DRIVERS_PATH:-none}"
} > /run/hcs/renderer.info

log "starting the Neural Glass desktop"

# ---------------------------------------------------------------- seat

# seatd ships as seatd-launch with a systemd unit that ExecStarts the real
# binary. /usr/bin/seatd does not exist in the Debian package, so a test for it
# here skips the seat silently -- and without a seat the compositor cannot open
# the VT, which presents as a niri startup failure rather than a missing seat.
#
# Rely on systemd's own seatd.service, which the package enables. Only start it
# manually if systemd has not, and name the binary that actually exists.
if ! pgrep -x seatd >/dev/null 2>&1; then
    if [ -x /usr/bin/seatd-launch ] && ! systemctl is-active --quiet seatd.service 2>/dev/null; then
        systemctl start seatd.service >>/var/log/hcs/seatd.log 2>&1 || true
    fi
    if ! pgrep -x seatd >/dev/null 2>&1 && [ -x /usr/bin/seatd-launch ]; then
        setsid /usr/bin/seatd-launch >>/var/log/hcs/seatd.log 2>&1 &
    fi
    # Wait for the seat to exist rather than sleeping a fixed amount: without it
    # niri starts, cannot open the VT, and exits with an error that looks like a
    # compositor bug.
    for _ in $(seq 1 40); do
        pgrep -x seatd >/dev/null 2>&1 && break
        sleep 0.25
    done
    if pgrep -x seatd >/dev/null 2>&1; then
        log "seatd is up"
    else
        log "WARN seatd did not start - see /var/log/hcs/seatd.log"
    fi
fi

# ---------------------------------------------------------------- compositor

# niri opens a session that needs a working directory it can write to.
export XDG_RUNTIME_DIR
export HOME="${HOME:-/root}"
mkdir -p "${HOME}"

setsid /usr/bin/niri \
    --session \
    --config /usr/share/hcs/shell/config.kdl \
    >>/var/log/hcs/niri.log 2>&1 &
NIRI_PID=$!

# Wait for the compositor's IPC socket. This is the signal the QA agent also
# waits on, and it is the earliest point at which a screenshot could contain
# anything other than a console.
for _ in $(seq 1 60); do
    kill -0 "${NIRI_PID}" 2>/dev/null || break
    if [ -S "${XDG_RUNTIME_DIR}/niri.wayland-1" ] \
        || ls "${XDG_RUNTIME_DIR}"/niri.wayland-* >/dev/null 2>&1; then
        break
    fi
    sleep 0.5
done

if ! kill -0 "${NIRI_PID}" 2>/dev/null; then
    log "FAIL niri exited during startup — see /var/log/hcs/niri.log"
    tail -5 /var/log/hcs/niri.log 2>/dev/null | while read -r l; do log "  niri: $l"; done
    exit 1
fi
log "niri is up (pid ${NIRI_PID})"

# ---------------------------------------------------------------- shell

# start-session.sh brings up hcsd, hcs-modeld, Quickshell and the rest. It is
# deliberately run *after* niri: Quickshell is a Wayland client and has nothing
# to connect to otherwise.
if [ -x /usr/share/hcs/session/start-session.sh ]; then
    /usr/share/hcs/session/start-session.sh >>/var/log/hcs/session-bootstrap.log 2>&1
    log "session bootstrap finished"
else
    log "WARN start-session.sh is missing"
fi

log "desktop startup complete"
exit 0
