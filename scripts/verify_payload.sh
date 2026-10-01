#!/bin/bash
# verify_payload.sh — Gate 4: ISO payload contract (HCS Linux v2 plan §7.1 G4)
#
# The v1 ISO build silently lost files: `cp ... || true` swallowed missing target
# directories, so `.desktop` launchers, icons, wallpapers and the manuals never
# reached the payload while the build still reported success. This script turns
# that class of bug into a hard build failure.
#
# Usage:
#   scripts/verify_payload.sh <rootfs-dir> [version]
#   scripts/verify_payload.sh --squashfs <file.squashfs> [version]
#
# Exit code 0 = contract satisfied. Non-zero = list of violations on stderr.

set -uo pipefail

ROOTFS_DIR=""
SQUASHFS=""
VERSION=""
POSITIONAL=()

while [ $# -gt 0 ]; do
    case "$1" in
        --squashfs)
            SQUASHFS="${2:-}"
            shift 2 || shift
            ;;
        --version)
            VERSION="${2:-}"
            shift 2 || shift
            ;;
        -*) shift ;;
        *)
            POSITIONAL+=("$1")
            shift
            ;;
    esac
done

# The first positional is the tree, the second the version. Assigning them by
# name rather than folding both into one loop variable is what stops a trailing
# version argument from silently replacing the directory being checked.
if [ "${#POSITIONAL[@]}" -ge 1 ]; then
    ROOTFS_DIR="${POSITIONAL[0]}"
fi
if [ "${#POSITIONAL[@]}" -ge 2 ]; then
    VERSION="${POSITIONAL[1]}"
fi
[ -n "${VERSION}" ] || VERSION="unknown"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

if [ -z "${ROOTFS_DIR}" ] && [ -z "${SQUASHFS}" ]; then
    echo "usage: verify_payload.sh <rootfs-dir> | --squashfs <file.squashfs>" >&2
    exit 2
fi

FAILURES=0
CHECKED=0

fail() {
    echo "  [MISSING] $1" >&2
    FAILURES=$((FAILURES + 1))
}

# check_file <path-in-rootfs> <human-label> <min-bytes>
check_file() {
    local path="$1" label="$2" min="${3:-1}"
    CHECKED=$((CHECKED + 1))
    local full="${ROOTFS_DIR}${path}"
    if [ ! -f "${full}" ]; then
        fail "${label} (${path})"
        return
    fi
    local size
    size=$(wc -c < "${full}" 2>/dev/null || echo 0)
    if [ "${size}" -lt "${min}" ]; then
        echo "  [TOO SMALL] ${label} (${path}): ${size} bytes < ${min}" >&2
        FAILURES=$((FAILURES + 1))
    fi
}

# check_dir <path-in-rootfs> <human-label>
check_dir() {
    local path="$1" label="$2"
    CHECKED=$((CHECKED + 1))
    if [ ! -d "${ROOTFS_DIR}${path}" ]; then
        fail "${label} directory (${path})"
    fi
}

# check_exec <path-in-rootfs> — binary must exist and be an ELF executable
check_exec() {
    local path="$1" label="$2"
    CHECKED=$((CHECKED + 1))
    local full="${ROOTFS_DIR}${path}"
    if [ ! -f "${full}" ]; then
        fail "${label} (${path})"
        return
    fi
    if [ ! -x "${full}" ]; then
        echo "  [NOT EXECUTABLE] ${label} (${path})" >&2
        FAILURES=$((FAILURES + 1))
        return
    fi
    # ELF magic — catches the shell-script-overwrites-binary class of bug.
    local magic
    magic=$(head -c 4 "${full}" 2>/dev/null | od -An -tx1 | tr -d ' \n')
    if [ "${magic}" != "7f454c46" ]; then
        echo "  [NOT AN ELF BINARY] ${label} (${path}): magic=${magic:-none}" >&2
        FAILURES=$((FAILURES + 1))
    fi
}

echo "=================================================="
echo "  HCS Linux — ISO Payload Contract (Gate 4)"
echo "  version: ${VERSION}"
echo "=================================================="

if [ -n "${SQUASHFS}" ]; then
    if ! command -v unsquashfs >/dev/null 2>&1; then
        echo "  [ERROR] unsquashfs not available; cannot verify ${SQUASHFS}" >&2
        exit 3
    fi
    TMP=$(mktemp -d)
    trap 'rm -rf "${TMP}"' EXIT
    unsquashfs -no-progress -d "${TMP}" "${SQUASHFS}" >/dev/null 2>&1 || {
        echo "  [ERROR] unsquashfs failed for ${SQUASHFS}" >&2
        exit 3
    }
    ROOTFS_DIR="${TMP}"
    echo "  Extracted ${SQUASHFS} -> ${TMP}"
fi

# ---------------------------------------------------------------- executables
# Every HCS binary that must exist as a REAL ELF binary.
#
# Two classes of mistake are guarded against here:
#   * `hcs-docs` must not be a shell script — the native Slint viewer owns that
#     name, and v1 shipped a bash script that silently replaced the binary.
#   * a Windows PE binary must not be staged. Cross-building on the dev host
#     produces `MZ` binaries, and an ISO full of them boots to nothing.
#
# Only crates that actually have a binary appear. `hcs-modeld`, `hcs-memory`,
# `hcs-agents`, `hcs-security`, `hcs-ui` and `hcs-actions` are libraries linked
# into the binaries below; requiring them as executables would make the contract
# impossible to satisfy and therefore ignored.
for bin in hcs hcsd hcs-chat hcs-monitor hcs-control hcs-search hcs-diagnose \
           hcs-docs hcs-settings hcs-image hcs-mcp hcs-rag-ingest hcs-installer \
           hcs-updater hcs-fm hcs-term hcs-shot hcs-notes hcs-update \
           hcs-persist hcs-recall hcs-qa-agent hcs-a11y-check hcs-gui-shots; do
    check_exec "/usr/bin/${bin}" "native binary ${bin}"
done

# Shell helpers (these ARE allowed to be scripts)
for helper in hcs-tor-switch hcs-docs-html hcs-welcome; do
    check_file "/usr/bin/${helper}" "helper script ${helper}" 40
done

# ---------------------------------------------------------------- directories
check_dir /usr/share/applications "Start-Menu launchers"
check_dir /usr/share/hcs/docs/manuals "offline manuals"
check_dir /usr/share/hcs/docs/assets "docs assets"
check_dir /usr/share/hcs/shell "Neural Glass shell"
check_dir /usr/share/hcs/branding "branding"
check_dir /usr/share/hcs/wallpapers "wallpapers"
check_dir /usr/share/hcs/icons "app icons"
check_dir /usr/share/hcs/theme "theme system"
check_dir /usr/share/hcs/session "session bootstrap"
check_dir /etc/hcs "system configuration"
check_dir /usr/share/plymouth/themes/hcs "Plymouth boot theme"
check_dir /var/log/hcs "runtime logs"

# ---------------------------------------------------------------- launchers
# Every GUI program must be launchable from the Start menu. A missing .desktop
# means the app exists but the user cannot reach it (this was B-04).
for app in hcs-chat hcs-monitor hcs-control hcs-search hcs-diagnose \
           hcs-docs hcs-docs-html hcs-settings hcs-fm hcs-term hcs-shot \
           hcs-notes hcs-update hcs-rag-ingest hcs-welcome; do
    check_file "/usr/share/applications/${app}.desktop" "launcher ${app}.desktop" 60
done

# ---------------------------------------------------------------- assets
for icon in hcs-chat hcs-control hcs-docs hcs-image hcs-monitor hcs-search \
            hcs-security hcs-terminal hcs-tor hcs-updater hcs-fm hcs-term \
            hcs-shot hcs-notes hcs-settings hcs-update; do
    check_file "/usr/share/hcs/icons/${icon}.svg" "icon ${icon}.svg" 200
done

for wp in neural_glass_dark frosted_titanium cybernetic_stealth; do
    check_file "/usr/share/hcs/wallpapers/${wp}.png" "wallpaper ${wp}.png" 10000
done

for logo in hcs.svg hcs-mark.svg hcs-mono.svg hcs-boot.svg; do
    check_file "/usr/share/hcs/branding/${logo}" "branding ${logo}" 200
done

# ---------------------------------------------------------------- manuals
for manual in 01_getting_started 02_ai_brain_guide 03_cpu_image_studio \
              04_security_pentest 05_tor_anonymity 06_developer_manual; do
    check_file "/usr/share/hcs/docs/manuals/${manual}.md" "manual ${manual}.md" 500
done
check_file /usr/share/hcs/docs/index.html "HTML docs portal" 500
check_file /usr/share/hcs/docs/cheatsheet.json "cheatsheet" 100

# ---------------------------------------------------------------- shell + theme
for qml in shell.qml taskbar.qml start_menu.qml cheatsheet.qml \
           control_drawer.qml control_center.qml notification_center.qml \
           widgets.qml snap_flyout.qml taskview.qml stage.qml \
           clipboard.qml notifications.qml; do
    check_file "/usr/share/hcs/shell/${qml}" "shell component ${qml}" 100
done
check_file /usr/share/hcs/shell/config.kdl "niri config" 200

check_file /usr/share/hcs/theme/colors.toml "theme source of truth" 200

# ---------------------------------------------------------------- session + boot
check_file /usr/share/hcs/session/start-session.sh "session bootstrap" 200
check_file /etc/hcs/session.conf "session configuration" 40
check_file /usr/share/plymouth/themes/hcs/hcs.plymouth "Plymouth theme" 100
check_file /usr/share/plymouth/themes/hcs/hcs.script "Plymouth script" 100

# ---------------------------------------------------------------- keyboard
# The HCS key is the Windows key; layouts are QWERTZ (de) and QWERTY (en) with
# at least DE/EN/FR/ES available (v2 plan §5 W1b).
check_file /usr/share/hcs/config/keyboard-layouts.conf "keyboard layout registry" 100

# ---------------------------------------------------------------- starter models
# License-clean starter models are baked into the ISO; anything else downloads.
if [ -d "${ROOTFS_DIR}/var/lib/hcs/models" ]; then
    MODEL_COUNT=$(find "${ROOTFS_DIR}/var/lib/hcs/models" -name '*.gguf' 2>/dev/null | wc -l)
    CHECKED=$((CHECKED + 1))
    if [ "${MODEL_COUNT}" -lt 1 ]; then
        fail "baked starter model (.gguf) in /var/lib/hcs/models"
    else
        echo "  [OK] ${MODEL_COUNT} baked starter model(s) present"
    fi
    check_file /var/lib/hcs/models/BUILT-IN.md "starter model manifest" 100
else
    fail "baked model directory /var/lib/hcs/models"
fi

# ---------------------------------------------------------------- init + os
check_file /sbin/init "system init" 500
check_file /etc/os-release "os-release" 100
check_file /etc/issue "issue banner" 20

grep -q "HCS LINUX ${VERSION}" "${ROOTFS_DIR}/etc/issue" 2>/dev/null || {
    echo "  [BAD VERSION] /etc/issue does not carry 'HCS LINUX ${VERSION}'" >&2
    FAILURES=$((FAILURES + 1))
}
CHECKED=$((CHECKED + 1))

grep -q "VERSION=\"${VERSION}\"" "${ROOTFS_DIR}/etc/os-release" 2>/dev/null || {
    echo "  [BAD VERSION] /etc/os-release VERSION is not ${VERSION}" >&2
    FAILURES=$((FAILURES + 1))
}
CHECKED=$((CHECKED + 1))

# ---------------------------------------------------------------- QA suite

# The guest drives the VM run itself, so the manifest and every scenario it
# names have to be in the image. Checked by name: a renamed scenario used to
# leave a stage that silently "skipped" while the run still reported passes.
check_file /usr/share/hcs/qa/suite.json "QA suite manifest" 100
check_dir /usr/share/hcs/qa/scenarios "QA scenarios"

if [ -f "${ROOTFS_DIR}/usr/share/hcs/qa/suite.json" ]; then
    while IFS= read -r scenario; do
        [ -n "${scenario}" ] || continue
        if [ ! -f "${ROOTFS_DIR}/usr/share/hcs/qa/scenarios/${scenario}" ]; then
            echo "  [MISSING] usr/share/hcs/qa/scenarios/${scenario} (named by the manifest)" >&2
            FAILURES=$((FAILURES + 1))
        else
            CHECKED=$((CHECKED + 1))
        fi
    done < <(python3 -c "
import json
with open('${ROOTFS_DIR}/usr/share/hcs/qa/suite.json', encoding='utf-8') as f:
    d = json.load(f)
for name in sorted({s['scenario'] for s in d['stages']}):
    print(name)
")
    CHECKED=$((CHECKED + 1))
fi

# ---------------------------------------------------------------- report
echo ""
echo "  checks run: ${CHECKED}"
if [ "${FAILURES}" -eq 0 ]; then
    echo "  [PASS] payload contract satisfied"
    exit 0
fi
echo "  [FAIL] ${FAILURES} contract violation(s) — the build must not ship this ISO." >&2
exit 1
