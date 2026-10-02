#!/bin/bash
# HCS Linux — run niri, the product compositor.
#
# Split out of start-desktop.sh when the compositor choice became a decision
# rather than a constant. niri requires hardware acceleration: it asserts that
# its EGL device is not software, so reaching this script on a machine with no
# supported GPU would produce a compositor that starts and never draws.
set -uo pipefail
LOG=/var/log/hcs/session.log
# ALSO TO THE CONSOLE.
#
# The journal, /var/log and the evidence disk are all reachable only from inside
# a guest that booted far enough to write them. When a unit like this one fails,
# the reason goes to the journal -- which is exactly where nobody can read it
# from, because the reason is that the session never came up.
#
# So every log line also goes to /dev/console, which a getty maps to tty1 and a
# screenshot can read. It is a crude instrument and it is the only one that works
# when the failure is "the desktop did not start".
log() { printf '[%s] [niri] %s\n' "$(date -u +%H:%M:%S)" "$*" | tee -a "${LOG}" >/dev/console 2>/dev/null; }

export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/hcs}"
export WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-1}"
export XDG_SESSION_TYPE=wayland
export XDG_CURRENT_DESKTOP=HCS
export QT_QPA_PLATFORM="${QT_QPA_PLATFORM:-wayland}"
export QT_WAYLAND_DISABLE_WINDOWDECORATION=1
export GDK_BACKEND=wayland
export XDG_DATA_DIRS="/usr/local/share:/usr/share:${XDG_DATA_DIRS:-}"
export PATH="/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"
export LIBGL_DRIVERS_PATH="${LIBGL_DRIVERS_PATH:-/usr/lib/x86_64-linux-gnu/dri}"
mkdir -p "${XDG_RUNTIME_DIR}" "${HOME:-/root}"

# Decide what renderer this actually is, BEFORE anything is recorded.
#
# The distinction is not cosmetic: a frame captured under llvmpipe is valid proof
# that a desktop drew, and is never proof of performance. Writing "hardware"
# unconditionally -- which is what this used to do -- is how a software-rendered
# run gets quoted as a hardware result.
#
# So: if this niri build can accept a software renderer, ask Mesa which one is in
# fact in use rather than assuming either way.
HCS_RENDERER_KIND="hardware"
if [ -f /usr/share/hcs/session/niri-renderer ] \
   && grep -q '^capability=software' /usr/share/hcs/session/niri-renderer; then
    if [ -n "$(lspci 2>/dev/null | grep -iE 'vga|3d|display')" ] \
       && [ -e /dev/dri ]; then
        HCS_RENDERER_KIND="hardware"
    else
        HCS_RENDERER_KIND="software"
        export LIBGL_ALWAYS_SOFTWARE=1
        export GALLIUM_DRIVER="${GALLIUM_DRIVER:-llvmpipe}"
    fi
fi
HCS_FALLBACK_REASON="none"
[ "${HCS_RENDERER_KIND}" = "hardware" ] || HCS_FALLBACK_REASON="no-hardware-gpu"
export HCS_RENDERER="${HCS_RENDERER_KIND}"

{
    printf 'compositor=niri\n'
    printf 'renderer=%s\n' "${HCS_RENDERER_KIND}"
    printf 'fallback_reason=%s\n' "${HCS_FALLBACK_REASON}"
    printf 'niri_build=%s\n' "$(grep -m1 '^patch=' /usr/share/hcs/session/niri-renderer 2>/dev/null | cut -d= -f2)"
} > /run/hcs/compositor.info
log "renderer: ${HCS_RENDERER_KIND} (${HCS_FALLBACK_REASON})"

# ---------------------------------------------------------------- compositor

setsid /usr/bin/niri \
    --session \
    --config /usr/share/hcs/shell/config.kdl \
    >>/var/log/hcs/niri.log 2>&1 &
NIRI_PID=$!

# Wait for the IPC socket: the earliest point at which a screenshot could contain
# anything other than a console. Bounded, because niri failing to open the VT
# must be a fast, visible failure and not an indefinite wait.
for _ in $(seq 1 60); do
    kill -0 "${NIRI_PID}" 2>/dev/null || break
    if ls "${XDG_RUNTIME_DIR}"/niri.wayland-* >/dev/null 2>&1; then break; fi
    sleep 0.5
done

if ! kill -0 "${NIRI_PID}" 2>/dev/null; then
    log "FAIL niri exited during startup"
    tail -10 /var/log/hcs/niri.log 2>/dev/null | while read -r l; do log "  niri: $l"; done
    exit 1
fi
log "niri is up (pid ${NIRI_PID})"

# The compositor skipping software EGL devices is the single most likely failure
# here, and it logs at DEBUG. Surface it explicitly, because otherwise the symptom
# is a black screen with no explanation anywhere.
if grep -q 'software EGL renderers are skipped' /var/log/hcs/niri.log 2>/dev/null; then
    log "FAIL niri skipped the software renderer."
    if [ -f /usr/share/hcs/session/niri-renderer ] \
       && grep -q '^capability=software' /usr/share/hcs/session/niri-renderer; then
        log "     This build was supposed to allow it (niri#3959), so the patch did"
        log "     not take effect. Rebuild without NIRI_SOFTWARE_RENDERING=1, or"
        log "     investigate why the relaxation did not reach this binary."
    else
        log "     This is niri's documented behaviour (upstream issue #218): this is"
        log "     an unmodified build and rejects software EGL by design."
    fi
    exit 1
fi

# ---------------------------------------------------------------- shell

# start-session.sh brings up hcsd, hcs-modeld, Quickshell and the rest. It runs
# *after* niri: Quickshell is a Wayland client and has nothing to connect to
# otherwise.
if [ -x /usr/share/hcs/session/start-session.sh ]; then
    /usr/share/hcs/session/start-session.sh >>/var/log/hcs/session-bootstrap.log 2>&1
    log "session bootstrap finished"
else
    log "WARN start-session.sh is missing"
fi

log "desktop startup complete"
wait "${NIRI_PID}"