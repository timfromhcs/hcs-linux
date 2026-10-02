#!/bin/bash
# HCS Linux — software fallback session: labwc on Xvfb.
#
# THIS IS NOT THE PRODUCT COMPOSITOR.
#
# niri is. It runs on hardware acceleration and it is what a real user sees.
# This exists because niri asserts, in its own source, that a software EGL
# device is rejected -- so on a machine with no supported GPU there is no niri
# session at all, and an image that cannot draw a desktop is not a desktop.
#
# The fallback chain:
#   Xvfb      a software X server. No GPU, no VT, no DRM.
#   labwc     a wlroots X11 window manager. Renders with pixman on X11.
#   Quickshell  the same Neural Glass shell, rendering through X11 instead of
#              Wayland. Qt supports both; the shell does not care.
#
# Every part is software. That is the point: it reaches a drawable desktop
# exactly where the hardware path cannot, which is every VM without 3D
# acceleration and every machine with no supported GPU.
set -uo pipefail
LOG=/var/log/hcs/session.log
log() { printf '[%s] [labwc] %s\n' "$(date -u +%H:%M:%S)" "$*" >>"${LOG}"
        printf '[%s] [labwc] %s\n' "$(date -u +%H:%M:%S)" "$*"; }

SCREEN="${HCS_QA_WIDTH:-1280}x${HCS_QA_HEIGHT:-800}x24"

# --- X server -------------------------------------------------------------
if [ -x /usr/bin/Xvfb ]; then
    # +extension GLX +render for Qt's OpenGL path; without GLX some Qt widgets
    # refuse to initialise and the app exits with no useful message.
    Xvfb ":99" -screen 0 "${SCREEN}" -nolisten tcp \
        +extension GLX +extension RANDR +extension RENDER \
        >>/var/log/hcs/xvfb.log 2>&1 &
    XVFB_PID=$!
    log "Xvfb started on :99 at ${SCREEN}"

    for _ in $(seq 1 40); do
        [ -S /tmp/.X11-unix/X99 ] && break
        kill -0 "${XVFB_PID}" 2>/dev/null || { log "Xvfb died - see /var/log/hcs/xvfb.log"; exit 1; }
        sleep 0.25
    done
    [ -S /tmp/.X11-unix/X99 ] || { log "Xvfb never came up"; exit 1; }
else
    log "Xvfb is not installed; the software fallback cannot run"
    exit 1
fi

export DISPLAY=:99
export LIBGL_ALWAYS_SOFTWARE=1
export GALLIUM_DRIVER=llvmpipe
export QT_QPA_PLATFORM=xcb
export XDG_CURRENT_DESKTOP=HCS
export XDG_SESSION_TYPE=x11
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/hcs}"
mkdir -p "${XDG_RUNTIME_DIR}"
export HOME="${HOME:-/home/hcs}"

# --- compositor -----------------------------------------------------------
if [ -x /usr/bin/labwc ]; then
    labwc --rcfile /usr/share/hcs/session/labwcrc \
        >>/var/log/hcs/labwc.log 2>&1 &
    LABWC_PID=$!
    log "labwc started (pid ${LABWC_PID})"
else
    log "labwc is not installed; nothing will manage windows"
    exit 1
fi

# Wait for the X server to answer, using a tool that is actually installed
# rather than assuming one. xdpyinfo comes with x11-utils, which is in the
# package list precisely so this check has something to use.
if [ -x /usr/bin/xdpyinfo ]; then
    for _ in $(seq 1 60); do
        xdpyinfo -display :99 >/dev/null 2>&1 && break
        sleep 0.25
    done
    xdpyinfo -display :99 >/dev/null 2>&1 \
        && log "X server is answering queries" \
        || { log "X server is up but not answering; the session cannot be trusted"; exit 1; }
fi

# Record which compositor ran, so the QA report says what was photographed
# instead of leaving a reader to guess from a screenshot.
{
    printf 'compositor=labwc\n'
    printf 'renderer=software\n'
    printf 'display=%s\n' "${SCREEN}"
    printf 'fallback_reason=no-hardware-gl\n'
} > /run/hcs/compositor.info

# --- the shell ------------------------------------------------------------
# Same start-session.sh the hardware path uses, so the daemons, the theme and the
# app registry are identical. Only the display protocol differs.
if [ -x /usr/share/hcs/session/start-session.sh ]; then
    /usr/share/hcs/session/start-session.sh >>/var/log/hcs/session-bootstrap.log 2>&1
    log "session bootstrap finished (X11)"
else
    log "WARN start-session.sh is missing"
fi

# Signal readiness for anything waiting on the desktop. Written by the shell
# itself once it has mapped a window; this is the earlier signal, that a window
# manager exists at all.
touch /run/hcs/session-x11-ready

log "software session ready"
# Stay alive so the X server and compositor survive.
wait "${LABWC_PID}"