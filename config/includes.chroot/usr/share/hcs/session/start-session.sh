#!/bin/bash
# HCS Linux — session bootstrap (v2 plan §5 W1)
#
# Runs inside the graphical session. Starts, in this order:
#   1. runtime dirs and log rotation
#   2. the HCS daemons (hcsd, hcs-modeld) on their Unix sockets
#   3. the Neural Glass shell (Quickshell) — taskbar, Start menu, Control Center
#   4. seatd / pipewire / portal, required for GUI apps to start at all
#   5. the first-run assistant, once
#
# Every step is logged to /var/log/hcs/session.log. The script never blocks the
# compositor: a failed step logs and continues, because a partial desktop is
# still more useful than a black screen.
#
# The QA agent is started only when hcs.qa=1 is on the kernel command line, so
# production sessions never carry an automation socket.

set -uo pipefail

LOG=/var/log/hcs/session.log
mkdir -p /var/log/hcs /run/hcs /run/user/"$(id -u)"

log() {
    printf '[%s] %s\n' "$(date -u +%H:%M:%S)" "$*" >> "${LOG}"
    printf '[%s] %s\n' "$(date -u +%H:%M:%S)" "$*"
}

step() {
    local name="$1"; shift
    log "START ${name}"
    if "$@" >>"${LOG}" 2>&1; then
        log "OK    ${name}"
    else
        log "FAIL  ${name} (continuing — partial desktop beats no desktop)"
    fi
}

# ---------------------------------------------------------------- 1. runtime

# Log rotation: a desktop that runs for months must not fill the disk.
rotate_logs() {
    for f in /var/log/hcs/*.log; do
        [ -f "$f" ] || continue
        if [ "$(stat -c %s "$f" 2>/dev/null || echo 0)" -gt 5242880 ]; then
            mv -f "$f" "${f}.1" 2>/dev/null || true
        fi
    done
}
step "log rotation" rotate_logs

export XDG_RUNTIME_DIR="/run/user/$(id -u)"
export WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-1}"
export XDG_SESSION_TYPE=wayland
export XDG_CURRENT_DESKTOP=HCS
export QT_QPA_PLATFORM="${QT_QPA_PLATFORM:-wayland}"
export QT_WAYLAND_DISABLE_WINDOWDECORATION=1
export GDK_BACKEND=wayland
export MOZ_ENABLE_WAYLAND=1
export XDG_DATA_DIRS="/usr/local/share:/usr/share:${XDG_DATA_DIRS:-}"
export PATH="/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"

# The AI/renderer stack must stay within the RAM budget (v2 plan §6).
export HCS_RAM_BUDGET_IDLE_MB="${HCS_RAM_BUDGET_IDLE_MB:-6144}"
export HCS_RAM_BUDGET_PEAK_MB="${HCS_RAM_BUDGET_PEAK_MB:-8192}"
export HCS_GUI_APP_BUDGET_MB="${HCS_GUI_APP_BUDGET_MB:-250}"
# CPU-only is the default path: no GPU may be required to reach a desktop.
export HCS_SOFTWARE_RENDERER=1
export LIBGL_ALWAYS_SOFTWARE=1
export WLR_RENDERER="${WLR_RENDERER:-pixman}"

chmod 700 "${XDG_RUNTIME_DIR}" 2>/dev/null || true

# ---------------------------------------------------------------- 2. daemons

start_daemons() {
    mkdir -p /run/hcs
    # hcsd is the system orchestrator; hcs-modeld owns GGUF residency.
    setsid /usr/bin/hcsd --socket /run/hcs/hcsd.sock >>"${LOG}" 2>&1 &
    sleep 0.4
    setsid /usr/bin/hcs-modeld --socket /run/hcs/hcs-modeld.sock >>"${LOG}" 2>&1 &
    return 0
}
step "HCS daemons" start_daemons

# ---------------------------------------------------------------- 3. shell

start_shell() {
    if [ ! -f /usr/share/hcs/shell/shell.qml ]; then
        echo "shell.qml missing — the desktop cannot be drawn" >&2
        return 1
    fi
    setsid quickshell -p /usr/share/hcs/shell >>"${LOG}" 2>&1 &
    # Give the shell time to map its first frame before the screenshot gates
    # start looking for pixels. This is a settle, not a race: it is bounded and
    # the QA agent still verifies that the expected windows actually appeared.
    sleep 2
    return 0
}
step "Neural Glass shell" start_shell

# ---------------------------------------------------------------- 4. session

start_session_services() {
    for svc in seatd pipewire pipewire-pulse wireplumber xdg-desktop-portal \
               xdg-desktop-portal-wlr xdg-desktop-portal-gtk; do
        if command -v "${svc}" >/dev/null 2>&1; then
            setsid "${svc}" >>"${LOG}" 2>&1 &
            sleep 0.2
        fi
    done
    return 0
}
step "session services" start_session_services

# Apply the saved keyboard layout. Layout switching must never disturb the HCS
# key bindings, so we only ever set the XKB layout, never the keymap wholesale.
apply_keyboard() {
    local layout="de"
    if [ -r /etc/hcs/keyboard.conf ]; then
        # shellcheck disable=SC1091
        . /etc/hcs/keyboard.conf
        layout="${HCS_XKB_LAYOUT:-de}"
    fi
    local variant=""
    if [ -n "${HCS_XKB_VARIANT:-}" ]; then
        variant=",${HCS_XKB_VARIANT}"
    fi
    if command -v setxkbmap >/dev/null 2>&1; then
        setxkbmap -layout "${layout}${variant}" -print >/dev/null 2>&1 || true
    fi
    export XKB_DEFAULT_LAYOUT="${layout}"
    return 0
}
step "keyboard layout" apply_keyboard

# ---------------------------------------------------------------- 5. first run

maybe_first_run() {
    local stamp=/var/lib/hcs/.first-run-done
    if [ ! -f "${stamp}" ]; then
        mkdir -p /var/lib/hcs
        setsid /usr/bin/hcs-welcome >>"${LOG}" 2>&1 &
        touch "${stamp}"
        log "first-run assistant launched"
    fi
}
step "first run" maybe_first_run

# ---------------------------------------------------------------- 6. QA agent

maybe_qa_agent() {
    if ! grep -q 'hcs\.qa=1' /proc/cmdline 2>/dev/null; then
        return 0
    fi
    if [ ! -x /usr/bin/hcs-qa-agent ]; then
        log "hcs.qa=1 but /usr/bin/hcs-qa-agent missing"
        return 1
    fi
    log "QA agent enabled (hcs.qa=1)"
    setsid /usr/bin/hcs-qa-agent >>"${LOG}" 2>&1 &
    return 0
}
step "QA agent" maybe_qa_agent

log "session bootstrap complete"
