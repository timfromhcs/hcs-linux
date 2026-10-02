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
export HCS_RENDERER=hardware
export LIBGL_DRIVERS_PATH="${LIBGL_DRIVERS_PATH:-/usr/lib/x86_64-linux-gnu/dri}"
mkdir -p "${XDG_RUNTIME_DIR}" "${HOME:-/root}"

{
    printf 'compositor=niri\n'
    printf 'renderer=hardware\n'
    printf 'fallback_reason=none\n'
} > /run/hcs/compositor.info

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
    log "FAIL niri rejected the only available renderer as software."
    log "     This is niri's documented behaviour (upstream issue #218): it does"
    log "     not support software rendering. A hardware GPU is required."
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