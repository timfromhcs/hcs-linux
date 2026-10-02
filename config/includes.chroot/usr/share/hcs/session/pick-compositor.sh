#!/bin/bash
# HCS Linux — pick a compositor that can actually draw here.
#
# WHY THIS FILE EXISTS
#
# niri cannot run on a software renderer. That is not a performance opinion, it
# is an assertion in niri's own source:
#
#     let egl_device = EGLDevice::device_for_display(&display)?;
#     // Software EGL devices (e.g., llvmpipe/softpipe) are rejected for now.
#     ensure!(
#         !egl_device.is_software(),
#         "software EGL renderers are skipped"
#     );
#
# Upstream issue #218: "There's no software rendering support at the moment."
# When the assertion fails there is no second fallback, so the observable result is
# a compositor that starts and never draws -- which is exactly what every
# VirtualBox run in this project showed.
#
# So HCS ships two compositors and chooses honestly at boot:
#
#   niri   -- the product compositor. Requires hardware acceleration. Wayland,
#             DRM/GBM, seatd, niri's own config.
#   labwc  -- the software fallback. X11 on top of Xvfb, rendered by pixman with
#             no GPU whatsoever. Not the product; it exists so the image reaches
#             a drawable desktop on machines with no supported GPU, which
#             includes every VM that cannot do 3D acceleration.
#
# The fallback is chosen when, and only when, hardware acceleration is genuinely
# unavailable. It is recorded in /run/hcs/compositor so the QA agent and the
# README can say which one ran, rather than a screenshot implying one or the
# other.

set -uo pipefail
LOG=/var/log/hcs/session.log
mkdir -p /var/log/hcs /run/hcs

log() {
    printf '[%s] [compositor] %s\n' "$(date -u +%H:%M:%S)" "$*" >>"${LOG}"
    printf '[%s] [compositor] %s\n' "$(date -u +%H:%M:%S)" "$*"
}

# Did anything give us a usable hardware GL? Ask EGL rather than infer it from a
# driver package being installed -- presence in the package list is not presence
# of a device, which is the assumption that produced three releases of false
# confidence.
have_hardware_gl() {
    [ -d /dev/dri ] || { log "no /dev/dri: there is no DRM device to render with"; return 1; }

    # EGL_EXT_device_drm is what niri actually requires, and the reason a bare
    # virtio-gpu node is rejected by it (niri #2570). Check for the extension
    # rather than for a device node, because a node existing says nothing about
    # whether it can back a compositor.
    if [ -x /usr/bin/eglinfo ]; then
        local out
        out=$(timeout 20 /usr/bin/eglinfo 2>&1)
        if printf '%s' "${out}" | grep -q 'EGL_EXT_device_drm'; then
            if printf '%s' "${out}" | grep -qi 'llvmpipe\|softpipe\|swrast'; then
                log "EGL present but the only renderer is software"
                return 1
            fi
            log "hardware EGL with EGL_EXT_device_drm is present"
            return 0
        fi
        log "EGL present but without EGL_EXT_device_drm, which niri requires"
        return 1
    fi
    log "eglinfo is unavailable; cannot determine the renderer"
    return 1
}

mode="${HCS_COMPOSITOR:-auto}"
[ -x /usr/bin/niri ] || { log "niri is not installed"; mode="${mode}"; }

if [ "${mode}" = "auto" ]; then
    if have_hardware_gl; then
        mode="niri"
    else
        mode="labwc"
    fi
fi

case "${mode}" in
niri)
    log "using niri (requires hardware acceleration)"
    exec /usr/share/hcs/session/run-niri.sh
    ;;

labwc)
    log "using labwc on Xvfb -- software rendering, no GPU required"
    log "NOTE: this is the fallback compositor, not the product compositor."
    log "      It is chosen because no usable hardware GL was found."
    exec /usr/share/hcs/session/run-labwc.sh
    ;;

*)
    log "unknown compositor '${mode}'; falling back to labwc"
    exec /usr/share/hcs/session/run-labwc.sh
    ;;
esac