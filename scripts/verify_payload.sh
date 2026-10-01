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

# ---------------------------------------------------------------- base system

# The check that would have caught two unbootable releases.
#
# v1 and v2 both produced an "ISO" with 152 files: HCS binaries, HCS data, and
# one HCS init script. No /bin/sh, no libc, no systemd, no compositor. The
# payload gate passed anyway, because every check it made was about *HCS's own
# files* — the launchers, icons, manuals and binaries it staged — and never once
# asked whether the image was a Linux system.
#
# So: a real system has a userspace. If any of these are missing, the image is a
# directory of files wearing an ISO's clothes.
check_file /bin/bash   "base: shell"            500
check_file /bin/sh     "base: POSIX shell"      100
check_dir  /lib/x86_64-linux-gnu "base: libc"   50
check_file /usr/lib/systemd/systemd "base: init system" 100
check_file /usr/bin/niri        "base: compositor (niri)"    100
check_file /usr/bin/quickshell  "base: desktop shell"        100
check_file /usr/bin/grim        "base: screenshot capture"   50
check_file /usr/bin/tesseract   "base: OCR (QA assert_text)" 50
check_file /usr/bin/wtype       "base: synthetic input"      20
check_dir  /lib/modules         "base: kernel modules"       1
check_file /etc/hcs-build-base  "base: provenance record"   10
check_file /etc/hcs-niri-provenance "base: niri provenance"  10

# The image cannot boot with a kernel whose modules are missing, and it cannot
# boot with the *build host's* kernel either. The modules directory has to match
# the kernel the image ships, so read the version out of the provenance record
# and confirm the modules for exactly that version are present.
if [ -f "${ROOTFS_DIR}/etc/hcs-build-base" ]; then
    KVER=$(sed -n 's/^kernel=vmlinuz-//p' "${ROOTFS_DIR}/etc/hcs-build-base" | head -1)
    if [ -n "${KVER}" ] && [ -d "${ROOTFS_DIR}/lib/modules/${KVER}" ]; then
        NMOD=$(find "${ROOTFS_DIR}/lib/modules/${KVER}" -name '*.ko*' 2>/dev/null | wc -l)
        echo "  [OK] kernel modules for ${KVER} (${NMOD} module(s))"
        CHECKED=$((CHECKED + 1))
    else
        echo "  [MISS] kernel modules for ${KVER:-<unknown>} — the image cannot boot" >&2
        FAILURES=$((FAILURES + 1))
    fi
fi

# A shell that is not really a shell, and a libc that is not really a libc, are
# both possible and both invisible to an existence check.
if [ -d "${ROOTFS_DIR}/bin" ]; then
    NFILE=$(find "${ROOTFS_DIR}" -xdev -type f 2>/dev/null | wc -l)
    if [ "${NFILE}" -lt 5000 ]; then
        echo "  [THIN] the root filesystem holds only ${NFILE} files." >&2
        echo "          A bootable Linux system has tens of thousands. This looks" >&2
        echo "          like a payload directory rather than a system." >&2
        FAILURES=$((FAILURES + 1))
    else
        echo "  [OK] root filesystem holds ${NFILE} file(s)"
        CHECKED=$((CHECKED + 1))
    fi
fi

# ---------------------------------------------------------------- PID 1

# systemd must be PID 1.
#
# This is the second time the image has shipped with something else at
# /sbin/init. The first was a banner script in an image with no systemd to
# displace; the second was that same script written over a perfectly good base,
# with GRUB told to run it — which suppressed systemd entirely and left the image
# booting to a console and stopping there. Both were invisible to the gates,
# because no gate had ever looked at this file.
#
# So the gate looks at it now, and it checks that it is a *symlink to systemd*,
# not merely that something exists there.
if [ -L "${ROOTFS_DIR}/sbin/init" ]; then
    INIT_TARGET=$(readlink "${ROOTFS_DIR}/sbin/init")
    case "${INIT_TARGET}" in
        *systemd*)
            echo "  [OK] PID 1 is systemd (/sbin/init -> ${INIT_TARGET})"
            CHECKED=$((CHECKED + 1))
            ;;
        *)
            echo "  [FAIL] /sbin/init points at ${INIT_TARGET}, not systemd." >&2
            FAILURES=$((FAILURES + 1))
            ;;
    esac
else
    echo "  [FAIL] /sbin/init is not a symlink. Something is overriding systemd." >&2
    FAILURES=$((FAILURES + 1))
fi

# And no GRUB entry may ask the kernel for a different init.
if grep -q 'init=/sbin/init' "${REPO_ROOT}/scripts/build_iso.sh" 2>/dev/null; then
    echo "  [FAIL] a GRUB entry still passes init=/sbin/init" >&2
    FAILURES=$((FAILURES + 1))
else
    echo "  [OK] no GRUB entry overrides init"
    CHECKED=$((CHECKED + 1))
fi

# ---------------------------------------------------------------- graphics

# A compositor with no drivers for it is not a desktop.
#
# For three releases this image shipped a Wayland compositor and zero graphics
# packages, and no gate noticed, because every gate checked that files were
# PRESENT and none checked that the compositor had anything to talk to. These
# checks are about the renderer existing, not about the renderer working — the
# working part can only be proven inside a running VM.
#
# The three failure modes are deliberately separate, because conflating them is
# how "drivers installed, nothing renders" happens:
#   * DRM/GBM  — does the image have a way to talk to a display device at all
#   * GL/EGL   — can it rasterise
#   * Vulkan   — does it have ICDs
check_file /usr/lib/x86_64-linux-gnu/libgbm.so.1  "GBM (DRM access)"   100
check_file /usr/lib/x86_64-linux-gnu/libEGL.so.1  "EGL"               100
check_file /usr/lib/x86_64-linux-gnu/dri/swrast_dri.so "llvmpipe (software GL)" 100
check_file /usr/bin/vulkaninfo                     "vulkaninfo"         20
check_file /usr/bin/glxinfo                        "glxinfo"            20

# The loader and the ICDs are different things. A missing loader finds no device;
# a loader with no ICD finds no device. Both look identical from the guest.
if [ -x "${ROOTFS_DIR}/usr/bin/vulkaninfo" ]; then
    ICD_COUNT=0
    for icd in "${ROOTFS_DIR}"/usr/share/vulkan/icd.d/*.json; do
        [ -f "${icd}" ] && ICD_COUNT=$((ICD_COUNT + 1))
    done
    if [ "${ICD_COUNT}" -gt 0 ]; then
        echo "  [OK] ${ICD_COUNT} Vulkan ICD(s) present"
        CHECKED=$((CHECKED + 1))
    else
        echo "  [FAIL] no Vulkan ICD. mesa-vulkan-drivers was not installed into the base." >&2
        FAILURES=$((FAILURES + 1))
    fi
fi

# lavapipe specifically: the software Vulkan floor the CPU-first claim rests on.
#
# The glob is deliberate. Mesa names this ICD lvp_icd.x86_64.json on some
# releases and lvp_icd.json on others, and the arch-suffixed guess fails on the
# build that matters -- which is how this gate reported "no lavapipe" against an
# image that had nine ICDs including lavapipe. A gate that cries wolf gets
# ignored, and this one would have been ignored.
LVP=$(ls "${ROOTFS_DIR}"/usr/share/vulkan/icd.d/lvp_icd*.json 2>/dev/null | head -1)
if [ -n "${LVP}" ]; then
    echo "  [OK] lavapipe present — CPU-only rendering has a floor"
    CHECKED=$((CHECKED + 1))
else
    echo "  [FAIL] no lavapipe ICD — a machine with no GPU cannot reach a desktop" >&2
    FAILURES=$((FAILURES + 1))
fi

# Firmware. Without it Wi-Fi and many GPUs come up silently broken.
#
# A WARN, not a FAIL, and deliberately so. These are redistribution-exempt
# blobs that sid repackages, and a repackage can legitimately drop a family
# without breaking the desktop at all. The warning names exactly which family is
# absent so it can be acted on, rather than failing a build over wireless.
FW_MISSING=""
for fw in iwlwifi ath10k rtl; do
    [ -e "${ROOTFS_DIR}/usr/lib/firmware/${fw}" ] || FW_MISSING="${FW_MISSING} ${fw}"
done
if [ -z "${FW_MISSING}" ]; then
    echo "  [OK] firmware families present (iwlwifi, ath10k, rtl)"
    CHECKED=$((CHECKED + 1))
else
    echo "  [WARN] firmware families absent:${FW_MISSING} — Wi-Fi may be silently broken"
    echo "         (not a build failure: these are redistribution-exempt blobs)"
    CHECKED=$((CHECKED + 1))
fi

# And the session must not be told to render in a way that needs hardware it
# cannot prove it has.
if grep -q 'HCS_RENDERER:-software' \
    "${ROOTFS_DIR}/usr/share/hcs/session/start-desktop.sh" 2>/dev/null; then
    echo "  [OK] software rendering is the default; hardware is opt-in"
    CHECKED=$((CHECKED + 1))
else
    echo "  [FAIL] the session does not default to software rendering" >&2
    FAILURES=$((FAILURES + 1))
fi

# ---------------------------------------------------------------- QA suite
check_file /usr/lib/systemd/system/hcs-desktop.service "session unit"    100
check_file /usr/lib/systemd/system/hcs-banner.service  "boot banner unit" 100
check_file /usr/share/hcs/session/start-desktop.sh     "desktop starter"  500
check_file /usr/share/hcs/branding/issue-banner.txt    "console banner"   50
# Resolve against the staged tree, not against /. See build_iso.sh for why: an
# absolute unit path is correct in the guest's namespace and absent from the
# build host's, so [ -e ] on the raw link gets both cases exactly backwards.
hcs_link_resolves() {
    local link="$1" target resolved
    target=$(readlink "${link}" 2>/dev/null) || return 1
    case "${target}" in
        /*) resolved="${ROOTFS_DIR}${target}" ;;
        *)  resolved="$(dirname "${link}")/${target}" ;;
    esac
    [ -e "${resolved}" ]
}

if [ -L "${ROOTFS_DIR}/etc/systemd/system/multi-user.target.wants/hcs-desktop.service" ]; then
    # Enabled is not the same as reachable. A link that resolves to a path with no
    # unit in it is treated by systemd as "nothing to do", so the session is
    # silently never started and there is no failed unit to look at afterwards.
    if hcs_link_resolves "${ROOTFS_DIR}/etc/systemd/system/multi-user.target.wants/hcs-desktop.service"; then
        echo "  [OK] the desktop session is enabled and the link resolves"
        CHECKED=$((CHECKED + 1))
    else
        echo "  [FAIL] hcs-desktop.service is enabled but DANGLING — the desktop" >&2
        echo "         would never start, and systemd would not report it." >&2
        FAILURES=$((FAILURES + 1))
    fi
else
    echo "  [FAIL] hcs-desktop.service is not enabled — the desktop would never start" >&2
    FAILURES=$((FAILURES + 1))
fi

# The same trap for the banner, and for the units themselves.
for _u in hcs-desktop.service hcs-banner.service; do
    _l="${ROOTFS_DIR}/etc/systemd/system/multi-user.target.wants/${_u}"
    if [ -e "${ROOTFS_DIR}/usr/lib/systemd/system/${_u}" ] && hcs_link_resolves "${_l}"; then
        echo "  [OK] ${_u} exists and is reachable"
        CHECKED=$((CHECKED + 1))
    else
        echo "  [FAIL] ${_u} missing or dangling" >&2
        FAILURES=$((FAILURES + 1))
    fi
done

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
