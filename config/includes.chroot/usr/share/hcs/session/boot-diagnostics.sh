#!/bin/bash
# HCS Linux — boot diagnostics onto the evidence disk.
#
# WHY THIS EXISTS
#
# Three consecutive VM runs produced a frozen framebuffer and nothing else. A
# screenshot is a picture of the console; when the thing that would draw the
# console is what is broken, the console cannot report on itself. So the only
# instrument available reported the same symptom three times and located no cause.
#
# This is the way out that does not depend on the graphics stack, on Guest
# Additions, or on a network. The evidence VHD is attached as an ordinary disk;
# the guest formats it FAT32 and writes the boot's own account of itself to it;
# the host mounts the VHD with nothing but the operating system it already has
# and reads the text.
#
# If the desktop never draws, this still works. That is the entire point.
set -uo pipefail

EV="${HCS_DIAG_DISK:-}"
OUT=/mnt/hcs-diag
LOG=/var/log/hcs/diag.log

mkdir -p /var/log/hcs
log() { printf '[diag] %s\n' "$*" | tee -a "${LOG}"; }

# Reuse the same discovery the QA agent uses: by label, else by shape. Two
# components guessing differently about where the disk is would be a bug of
# exactly the kind this project keeps producing.
find_disk() {
    local d
    d=$(blkid -l HCSQA -o device 2>/dev/null | head -1)
    [ -n "${d}" ] && [ -b "${d}" ] && { echo "${d}"; return 0; }
    for d in /dev/disk/by-path/* /dev/sd? /dev/vd?; do
        [ -b "${d}" ] || continue
        case "$(readlink -f "${d}")" in
            /dev/sr0*|/dev/vda*|/dev/sda*) continue ;;
        esac
        local s
        s=$(blockdev --getsz "${d}" 2>/dev/null) || continue
        [ "${s}" -gt 131072 ] || continue
        echo "${d}"
        return 0
    done
    return 1
}

[ -z "${EV}" ] && { EV=$(find_disk) || { log "no evidence disk attached; nothing to write to"; exit 1; }; }
log "evidence disk: ${EV}"

# A blank VHD has no filesystem. Format it. FAT32 because the host must read it
# with nothing but Windows, and ext4 would need a driver nobody should trust for
# release evidence.
if ! blkid "${EV}" >/dev/null 2>&1; then
    mkfs.vfat -F 32 -n HCSQA "${EV}" >>"${LOG}" 2>&1 || { log "mkfs failed"; exit 1; }
    log "formatted ${EV}"
fi
mkdir -p "${OUT}"
mount -o rw,noatime "${EV}" "${OUT}" 2>/dev/null || { log "mount failed"; exit 1; }
log "mounted at ${OUT}"

D="${OUT}/diag-$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "${D}"

# --- the boot's own account of itself ------------------------------------

# Kernel first, and in full. The interesting lines are the ones nobody expects.
dmesg >"${D}/dmesg.txt" 2>&1
cat /proc/cmdline >"${D}/cmdline.txt" 2>&1

# systemd's verdict on every unit, which is where "started but did nothing"
# becomes visible.
systemctl list-units --all --no-pager --plain >"${D}/systemd-units.txt" 2>&1
systemctl --failed --no-pager --plain >"${D}/systemd-failed.txt" 2>&1

# The journal, filtered to the services that matter. Full journal is megabytes
# and the host has to read it over a mounted image.
journalctl -b --no-pager -u hcs-desktop -u hcs-banner -u seatd \
    -u live-config >"${D}/journal-hcs.txt" 2>&1
journalctl -b --no-pager -p warning >"${D}/journal-warnings.txt" 2>&1

for f in /var/log/hcs/session.log /var/log/hcs/niri.log \
         /var/log/hcs/seatd.log /var/log/hcs/vulkan.log \
         /var/log/hcs/glxinfo.log /var/log/hcs/session-bootstrap.log; do
    [ -f "${f}" ] && cp -f "${f}" "${D}/" 2>/dev/null
done
[ -f /run/hcs/renderer.info ] && cp -f /run/hcs/renderer.info "${D}/" 2>/dev/null

# --- what the graphics stack actually reports ----------------------------

# The question this whole session turns on: does the guest see a usable
# renderer? Ask the tools, do not infer it from a screenshot.
{
    echo "=== DRM devices (the compositor's only way to present) ==="
    ls -l /dev/dri/ 2>&1
    echo
    echo "=== modesetting modes the DRM device offers ==="
    for c in /sys/class/drm/card*/status /sys/class/drm/card*/modes; do
        [ -e "${c}" ] && { echo "--- ${c}"; cat "${c}"; }
    done
    echo
    echo "=== fbset / framebuffer geometry ==="
    fbset -i 2>&1 | head -20
    echo
    echo "=== VT state ==="
    cat /sys/class/tty/tty0/active 2>&1
    echo
    echo "=== seats ==="
    loginctl list-sessions 2>&1
    echo
    echo "=== vulkaninfo ==="
    command -v vulkaninfo >/dev/null 2>&1 && timeout 30 vulkaninfo --summary 2>&1 || echo "vulkaninfo absent"
    echo
    echo "=== egl/glx ==="
    command -v eglinfo >/dev/null 2>&1 && timeout 30 eglinfo 2>&1 | head -30 || echo "eglinfo absent"
    echo
    echo "=== mesa driver resolution ==="
    ls -l /usr/lib/x86_64-linux-gnu/dri/ 2>&1 | head -30
} >"${D}/graphics.txt" 2>&1

# --- a plain summary, first thing anyone reads ---------------------------

{
    echo "HCS Linux boot diagnostic"
    echo "generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo
    echo "cmdline:      $(cat /proc/cmdline 2>/dev/null)"
    echo "uptime:       $(cut -d' ' -f1 /proc/uptime)s"
    echo
    echo "PID 1:        $(readlink -f /proc/1/exe 2>/dev/null)"
    echo "default tgt:  $(systemctl get-default 2>/dev/null)"
    echo
    echo "niri running: $(pgrep -x niri >/dev/null && echo yes || echo NO)"
    echo "quickshell:   $(pgrep -f quickshell >/dev/null && echo yes || echo NO)"
    echo "seatd:        $(pgrep -x seatd >/dev/null && echo yes || echo NO)"
    echo
    echo "DRM devices:"
    ls /dev/dri/ 2>&1 | sed 's/^/  /'
    echo
    echo "failed units:"
    systemctl --failed --no-pager --plain --no-legend 2>/dev/null | sed 's/^/  /' || true
    echo
    echo "hcs-desktop.service:"
    systemctl status hcs-desktop.service --no-pager -n 30 2>&1 | sed 's/^/  /' || true
} >"${D}/SUMMARY.txt" 2>&1

sync
echo "HCS-DIAG-COMPLETE" >"${D}/DONE"
sync
log "diagnostics written to ${D}"
umount "${OUT}" 2>/dev/null || true
exit 0