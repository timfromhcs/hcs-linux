#!/bin/bash
# build_iso.sh — HCS Linux live ISO builder (v2 plan §5 W0/W1)
#
# CHANGES vs. the v1 builder, and why each one exists:
#
#  - The `|| true` on payload copies is GONE. In v1 a missing target directory
#    meant icons, wallpapers, .desktop launchers and manuals silently never
#    reached the ISO while the build printed SUCCESS. Missing payload now fails
#    the build via scripts/verify_payload.sh (Gate 4).
#  - `hcs-docs` is no longer overwritten by a shell script. The name belongs to
#    the native Slint viewer; the HTML portal is `hcs-docs-html`.
#  - The init script can start a real graphical session (niri + Quickshell) and
#    keeps the tty1 banner path as a fallback, so the console QA stages and the
#    graphical stages can both be captured from one ISO.
#  - License-clean starter models are baked into the payload; everything else
#    downloads on first use.
#
# Usage: scripts/build_iso.sh [VERSION] [ARCH]

set -euo pipefail

VERSION="${1:-2.0.0}"
ARCH="${2:-amd64}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
DIST_DIR="${REPO_ROOT}/dist"
ROOTFS_DIR="${REPO_ROOT}/target/rootfs"

# --stage-only stops after the payload is staged and verified, before squashfs.
#
# This exists so the payload contract can be checked on a build host that has no
# mksquashfs or grub-mkrescue — the CI host, or a developer on Windows. Without
# it, the only way to learn that a launcher is missing is to build a 295 MB
# image and unpack it again.
STAGE_ONLY=0
for a in "$@"; do
    [ "${a}" = "--stage-only" ] && STAGE_ONLY=1
done

# --qa builds the *QA* image: same bits, different default boot entry.
#
# The QA agent refuses to start unless hcs.qa=1 is on the kernel command line,
# which is the property that makes it safe to ship in a production image at all.
# Putting the flag in the default GRUB entry of a separate ISO means the
# VirtualBox driver never has to send keystrokes to pick a menu entry — the
# v2 driver did, and a missed keystroke produced a run that booted normally and
# reported "the agent never came up" with no way to tell why.
#
# The production ISO is unaffected: its default entry has no QA flag, so the
# agent cannot start.
QA=0
for a in "$@"; do
    [ "${a}" = "--qa" ] && QA=1
done

# Resolved before the banner prints, so the build log names the image it is
# actually making. It used to print the production name during a QA build, which
# is exactly the sort of small lie that makes a release note wrong.
ISO_NAME="HCS-Linux-${VERSION}-${ARCH}.iso"
[ "${QA}" -eq 1 ] && ISO_NAME="HCS-Linux-${VERSION}-qa-${ARCH}.iso"
FINAL_ISO="${DIST_DIR}/${ISO_NAME}"

echo "=================================================="
echo "          HCS Linux ISO Build Pipeline            "
echo "=================================================="
echo "Version: ${VERSION}"
echo "Arch:    ${ARCH}"
if [ "${QA}" -eq 1 ]; then
    echo "Mode:    QA image (the automated boot entry is the default)"
fi
if [ "${STAGE_ONLY}" -eq 1 ]; then
    echo "Mode:    stage only (no squashfs, no ISO)"
fi
echo "Target:  ${FINAL_ISO}"

mkdir -p "${DIST_DIR}"

# ------------------------------------------------------------------ 1. build

echo "[1/7] Verifying native HCS binaries in release mode..."
if [ ! -f "${REPO_ROOT}/target/release/hcsd" ] && [ ! -f "${REPO_ROOT}/target/release/hcsd.exe" ]; then
    cargo build --release --workspace
fi

# ------------------------------------------------------------------ 1b. base

# The base system is built by scripts/build_base.sh, not here.
#
# For two releases this script staged a directory of HCS binaries and assets and
# called it a rootfs, then copied the *build host's* kernel and initrd into the
# image. The result had 152 files and no /bin/sh, no libc and no compositor, so
# it could not boot; the package list was read by nothing. See
# docs/V2_STABLE_QA_STATUS.md.
#
# The base is a real debootstrapped Debian system with the declared package list
# installed into it, and the image takes its kernel and initrd from there. It is
# built separately because it takes a long time and rarely changes: build it once
# with `sudo scripts/build_base.sh sid`, then reuse it.
BASE_DIR="${REPO_ROOT}/target/base"
if [ ! -d "${BASE_DIR}" ] || [ ! -f "${BASE_DIR}/bin/bash" ]; then
    echo "[1b] No base system at ${BASE_DIR}." >&2
    echo "" >&2
    echo "     Build it first:" >&2
    echo "         sudo bash scripts/build_base.sh sid" >&2
    echo "" >&2
    echo "     It debootstraps Debian, installs the declared package list and" >&2
    echo "     provisions niri. It needs root and takes a while. Reuse it with" >&2
    echo "     --cache on subsequent runs." >&2
    exit 1
fi
echo "[1b] Using the base system at ${BASE_DIR}"
if [ -f "${BASE_DIR}/etc/hcs-build-base" ]; then
    # Printed so the image's provenance is visible in the build log rather than
    # being something you have to go looking for afterwards.
    sed 's/^/       /' "${BASE_DIR}/etc/hcs-build-base"
fi

# ------------------------------------------------------------------ 2. layout

echo "[2/7] Assembling the image root filesystem from the base..."
# The base *is* the root filesystem. It is copied rather than moved so a failed
# build leaves the base intact for the next attempt.
rm -rf "${ROOTFS_DIR}"
mkdir -p "$(dirname "${ROOTFS_DIR}")"
# The extraction target has to exist before tar writes into it: `tar -x -C` does
# not create the directory it is told to extract into, and the error surfaces as
# "Cannot open: No such file or directory" with no mention of which path.
mkdir -p "${ROOTFS_DIR}"
# -a preserves ownership, symlinks and device nodes; the exclusions are the
# mount points and the kernel the image supplies itself.
tar -C "${BASE_DIR}" -cf - \
    --exclude=./proc --exclude=./sys --exclude=./dev --exclude=./run \
    --exclude=./tmp --exclude=./boot \
    . | tar -C "${ROOTFS_DIR}" -xf -
echo "       root filesystem: $(find "${ROOTFS_DIR}" -xdev -type f 2>/dev/null | wc -l) file(s)"

# Every directory a later copy targets MUST exist up front. In v1 these were
# missing, which is exactly why `cp ... || true` was masking real failures.
mkdir -p \
    "${ROOTFS_DIR}/bin" \
    "${ROOTFS_DIR}/sbin" \
    "${ROOTFS_DIR}/dev" \
    "${ROOTFS_DIR}/proc" \
    "${ROOTFS_DIR}/sys" \
    "${ROOTFS_DIR}/run" \
    "${ROOTFS_DIR}/tmp" \
    "${ROOTFS_DIR}/etc" \
    "${ROOTFS_DIR}/var/lib/hcs" \
    "${ROOTFS_DIR}/var/log/hcs" \
    "${ROOTFS_DIR}/usr/bin" \
    "${ROOTFS_DIR}/usr/sbin" \
    "${ROOTFS_DIR}/usr/lib/systemd/user" \
    "${ROOTFS_DIR}/usr/share/hcs/branding" \
    "${ROOTFS_DIR}/usr/share/hcs/shell" \
    "${ROOTFS_DIR}/usr/share/hcs/theme" \
    "${ROOTFS_DIR}/usr/share/hcs/session" \
    "${ROOTFS_DIR}/usr/share/hcs/config" \
    "${ROOTFS_DIR}/usr/share/hcs/docs" \
    "${ROOTFS_DIR}/usr/share/hcs/docs/manuals" \
    "${ROOTFS_DIR}/usr/share/hcs/docs/assets" \
    "${ROOTFS_DIR}/usr/share/hcs/qa" \
    "${ROOTFS_DIR}/usr/share/hcs/qa/scenarios" \
    "${ROOTFS_DIR}/usr/share/hcs/icons" \
    "${ROOTFS_DIR}/usr/share/hcs/wallpapers" \
    "${ROOTFS_DIR}/usr/share/hcs/scripts" \
    "${ROOTFS_DIR}/usr/share/applications" \
    "${ROOTFS_DIR}/usr/share/plymouth/themes/hcs" \
    "${ROOTFS_DIR}/etc/hcs" \
    "${ROOTFS_DIR}/etc/calamares/branding/hcs" \
    "${ROOTFS_DIR}/etc/skel" \
    "${ROOTFS_DIR}/var/lib/hcs/models" \
    "${ROOTFS_DIR}/var/log/hcs"

# copy_dir <src> <dst> — hard failure if the source does not exist.
copy_dir() {
    local src="$1" dst="$2" label="${3:-$1}"
    if [ ! -d "${src}" ]; then
        echo "  [ERROR] required directory missing: ${label} (${src})" >&2
        exit 1
    fi
    cp -rf "${src}/." "${dst}/"
}

# Crates that are libraries and deliberately have no binary. Listing them here
# keeps the copy list and the payload contract in agreement — a name in the copy
# list with nothing to copy is exactly the kind of silent gap the contract
# exists to catch.
LIBRARY_ONLY_CRATES="hcs-modeld hcs-memory hcs-agents hcs-security hcs-ui hcs-actions hcs-a11y"

# copy_file <src> <dst> <min-bytes> <label>
copy_file() {
    local src="$1" dst="$2" min="$3" label="$4"
    if [ ! -f "${src}" ]; then
        echo "  [ERROR] required file missing: ${label} (${src})" >&2
        exit 1
    fi
    local size
    size=$(wc -c < "${src}")
    if [ "${size}" -lt "${min}" ]; then
        echo "  [ERROR] ${label} is only ${size} bytes, expected >= ${min} (${src})" >&2
        exit 1
    fi
    cp -f "${src}" "${dst}"
    chmod 755 "${dst}" 2>/dev/null || true
}

# ------------------------------------------------------------------ 3. identity

cat > "${ROOTFS_DIR}/etc/os-release" << EOF
NAME="HCS Linux"
VERSION="${VERSION}"
ID=hcslinux
ID_LIKE=debian
PRETTY_NAME="HCS Linux ${VERSION} (Neural Glass)"
VERSION_ID="${VERSION}"
HOME_URL="https://github.com/timfromhcs/hcs-linux"
SUPPORT_URL="https://github.com/timfromhcs/hcs-linux/issues"
BUG_REPORT_URL="https://github.com/timfromhcs/hcs-linux/issues"
EOF

printf 'HCS LINUX %s \n \l' "${VERSION}" > "${ROOTFS_DIR}/etc/issue"

cat > "${ROOTFS_DIR}/etc/hostname" << EOF
hcs-live
EOF

# ------------------------------------------------------------------ 4. binaries

echo "[3/7] Staging native binaries..."
# Only crates that actually have a binary are staged. `hcs-modeld` and
# `hcs-memory` are libraries linked into `hcsd`; asking for them as binaries
# produced a payload contract that could never pass, which is worse than no
# contract because it trains you to ignore it.
for bin in hcs hcsd hcs-chat hcs-search hcs-control hcs-installer hcs-updater \
           hcs-mcp hcs-monitor hcs-rag-ingest hcs-diagnose hcs-image \
           hcs-settings hcs-docs hcs-fm hcs-term hcs-shot hcs-notes \
           hcs-update hcs-persist hcs-recall hcs-qa-agent hcs-a11y-check \
           hcs-gui-shots; do
    if [ -f "${REPO_ROOT}/target/release/${bin}" ]; then
        cp -f "${REPO_ROOT}/target/release/${bin}" "${ROOTFS_DIR}/usr/bin/"
        chmod 755 "${ROOTFS_DIR}/usr/bin/${bin}"
    elif [ -f "${REPO_ROOT}/target/release/${bin}.exe" ]; then
        cp -f "${REPO_ROOT}/target/release/${bin}.exe" "${ROOTFS_DIR}/usr/bin/${bin}"
        chmod 755 "${ROOTFS_DIR}/usr/bin/${bin}"
    else
        echo "  [ERROR] required binary not built: ${bin}" >&2
        echo "          run: cargo build --release --workspace" >&2
        MISSING_BINARIES=1
    fi
done

if [ "${MISSING_BINARIES:-0}" -eq 1 ]; then
    echo "[ERROR] refusing to build an ISO with a missing binary." >&2
    exit 1
fi

# Shell helpers. hcs-docs is deliberately absent from this list: it is a native
# binary. hcs-docs-html is the browser portal (v1 had both named hcs-docs).
for helper in hcs-tor-switch hcs-docs-html hcs-welcome; do
    copy_file "${REPO_ROOT}/config/includes.chroot/usr/share/hcs/scripts/${helper}" \
              "${ROOTFS_DIR}/usr/bin/${helper}" 40 "${helper}"
done

# ------------------------------------------------------------------ 5. assets

echo "[4/7] Staging assets, launchers, session and theme..."

copy_dir "${REPO_ROOT}/assets/logo"        "${ROOTFS_DIR}/usr/share/hcs/branding"   "branding"
copy_dir "${REPO_ROOT}/assets/icons"       "${ROOTFS_DIR}/usr/share/hcs/icons"      "app icons"
copy_dir "${REPO_ROOT}/assets/wallpapers"  "${ROOTFS_DIR}/usr/share/hcs/wallpapers" "wallpapers"
copy_dir "${REPO_ROOT}/src/hcs-shell"      "${ROOTFS_DIR}/usr/share/hcs/shell"      "Neural Glass shell"
copy_dir "${REPO_ROOT}/config/includes.chroot/usr/share/applications" \
         "${ROOTFS_DIR}/usr/share/applications" "Start-Menu launchers"
copy_dir "${REPO_ROOT}/config/includes.chroot/usr/share/hcs/docs" \
         "${ROOTFS_DIR}/usr/share/hcs/docs" "offline documentation"
copy_dir "${REPO_ROOT}/config/includes.chroot/usr/share/hcs/theme" \
         "${ROOTFS_DIR}/usr/share/hcs/theme" "theme system"
copy_dir "${REPO_ROOT}/config/includes.chroot/usr/share/hcs/session" \
         "${ROOTFS_DIR}/usr/share/hcs/session" "session bootstrap"
copy_dir "${REPO_ROOT}/config/includes.chroot/usr/share/hcs/config" \
         "${ROOTFS_DIR}/usr/share/hcs/config" "keyboard registry"
# The QA suite ships in the image so the guest can drive itself. The agent
# refuses to start without the kernel flag, so shipping this in the production
# ISO adds files but no capability.
copy_dir "${REPO_ROOT}/config/includes.chroot/usr/share/hcs/qa" \
         "${ROOTFS_DIR}/usr/share/hcs/qa" "QA suite"
copy_dir "${REPO_ROOT}/config/includes.chroot/usr/share/plymouth" \
         "${ROOTFS_DIR}/usr/share/plymouth" "Plymouth theme"
copy_dir "${REPO_ROOT}/config/installer/calamares" \
         "${ROOTFS_DIR}/etc/calamares" "Calamares installer"
copy_dir "${REPO_ROOT}/config/models" "${ROOTFS_DIR}/etc/hcs/models" "model configuration"

copy_file "${REPO_ROOT}/config/includes.chroot/etc/hcs/session.conf" \
          "${ROOTFS_DIR}/etc/hcs/session.conf" 40 "session.conf"
copy_file "${REPO_ROOT}/config/includes.chroot/etc/hcs/mcp_servers.json" \
          "${ROOTFS_DIR}/etc/hcs/mcp_servers.json" 20 "mcp_servers.json"
copy_file "${REPO_ROOT}/config/includes.chroot/usr/share/hcs/config/keyboard-layouts.conf" \
          "${ROOTFS_DIR}/usr/share/hcs/config/keyboard-layouts.conf" 100 "keyboard-layouts.conf"

# Default locale/keyboard state. QWERTZ (de) is the default audience; en-US is
# one shortcut away and FR/ES/IT/GB are available from the same registry.
cat > "${ROOTFS_DIR}/etc/hcs/keyboard.conf" << 'EOF'
HCS_XKB_LAYOUT="de"
HCS_XKB_VARIANT=""
EOF
printf 'de\n' > "${ROOTFS_DIR}/etc/hcs/locale"

# ------------------------------------------------------------------ 6. models

echo "[5/7] Baking license-clean starter models..."

# Only models whose licence permits redistribution of the *weights* may ship
# inside the image. The decision is made by the staging script against
# config/models/registry.yaml, not here, so the rule has exactly one
# implementation. Everything else downloads on demand via `hcs model fetch`.
python3 "${SCRIPT_DIR}/stage_starter_models.py" \
    --stage "${ROOTFS_DIR}/var/lib/hcs/models" \
    --cache "${HCS_MODEL_CACHE:-${REPO_ROOT}/models/cache}" \
    --max-mb "${HCS_BAKED_MODEL_MB:-1024}" || {
        echo "  [ERROR] starter model staging failed; refusing to ship an unverified payload." >&2
        exit 1
    }

# ------------------------------------------------------------------ 7. session

echo "[6/7] Wiring the session into systemd (no init override)..."

# THIS USED TO WRITE /sbin/init.
#
# For two releases this stage wrote a 120-line shell script over the base's
# /sbin/init and told GRUB to run it instead of systemd. That script predates the
# base system: it existed because there was no systemd to run, and it mounted
# /proc itself and launched niri by hand.
#
# Once the image had a real Debian base, that override was the reason the
# desktop never appeared. systemd never became PID 1, so nothing it owns was set
# up; the image booted, printed the banner from the script, and sat there. The VM
# run captured exactly that — a console with a banner, three times over.
#
# So: keep the base's /sbin/init (a symlink to systemd), let live-boot mount the
# squashfs, and start the session from a unit. If the desktop cannot start, the
# unit fails and getty@tty1 still gives a login on the same VT.
if [ -L "${ROOTFS_DIR}/sbin/init" ]; then
    echo "       /sbin/init -> $(readlink "${ROOTFS_DIR}/sbin/init")"
elif [ -f "${ROOTFS_DIR}/sbin/init" ]; then
    echo "  [ERROR] /sbin/init is a regular file, so something is still overriding systemd." >&2
    echo "          Refusing to ship an image whose PID 1 is a shell script." >&2
    exit 1
fi
ln -sf /sbin/init "${ROOTFS_DIR}/init"

# The session units and the session scripts.
mkdir -p "${ROOTFS_DIR}/usr/lib/systemd/system" \
         "${ROOTFS_DIR}/etc/systemd/system/multi-user.target.wants" \
         "${ROOTFS_DIR}/usr/share/hcs/session"

copy_file "${REPO_ROOT}/config/includes.chroot/usr/lib/systemd/system/hcs-desktop.service" \
          "${ROOTFS_DIR}/usr/lib/systemd/system/hcs-desktop.service" 644 "hcs-desktop.service"
copy_file "${REPO_ROOT}/config/includes.chroot/usr/lib/systemd/system/hcs-banner.service" \
          "${ROOTFS_DIR}/usr/lib/systemd/system/hcs-banner.service" 644 "hcs-banner.service"
copy_file "${REPO_ROOT}/config/includes.chroot/usr/share/hcs/session/start-desktop.sh" \
          "${ROOTFS_DIR}/usr/share/hcs/session/start-desktop.sh" 755 "start-desktop.sh"

# Enabled by symlink rather than `systemctl enable`, which cannot run in a chroot
# with no systemd running. The wants directory is the same mechanism, and it is
# visible in the staged tree, so a reviewer can see the desktop is on by default.
ln -sf ../hcs-desktop.service \
      "${ROOTFS_DIR}/etc/systemd/system/multi-user.target.wants/hcs-desktop.service"
ln -sf ../hcs-banner.service \
      "${ROOTFS_DIR}/etc/systemd/system/multi-user.target.wants/hcs-banner.service"

# The console banner the boot stages photograph.
mkdir -p "${ROOTFS_DIR}/usr/share/hcs/branding"
cat > "${ROOTFS_DIR}/usr/share/hcs/branding/issue-banner.txt" << EOF
================================================================================
                        HCS LINUX ${VERSION}
              AI-Native, Privacy-Oriented, CPU-First Operating System
================================================================================
EOF

# Nothing here may replace systemd. A regular file at /sbin/init means a previous
# stage wrote one, and that is a hard stop rather than something to work around.
if [ ! -L "${ROOTFS_DIR}/sbin/init" ]; then
    echo "  [ERROR] systemd is not PID 1 in the staged image; refusing to continue." >&2
    exit 1
fi
# ------------------------------------------------------------------ 8. verify

echo "[7/7] Verifying payload contract (Gate 4)..."
"${SCRIPT_DIR}/verify_payload.sh" "${ROOTFS_DIR}" "${VERSION}"

if [ "${STAGE_ONLY}" -eq 1 ]; then
    echo ""
    echo "=================================================="
    echo "  Payload staged and verified."
    echo "  Staged tree: ${ROOTFS_DIR}"
    echo "  Stopped before squashfs (--stage-only)."
    echo "=================================================="
    exit 0
fi

# ------------------------------------------------------------------ 9. squashfs

echo "[8/8] Compressing root filesystem into SquashFS..."
ISO_STAGING="${REPO_ROOT}/target/iso_staging"
rm -rf "${ISO_STAGING}"
mkdir -p "${ISO_STAGING}/live" "${ISO_STAGING}/boot/grub"

mksquashfs "${ROOTFS_DIR}" "${ISO_STAGING}/live/filesystem.squashfs" -comp xz -noappend

# The kernel and initrd come from the BASE, never from the build host.
#
# This is the line that was wrong for two releases. `cp -L /boot/vmlinuz` on the
# build host put Ubuntu 26.04's kernel into a "Debian" image, and the initrd
# that came with it contained a single file. Nothing in the image was Debian's;
# the ISO could not boot, and no gate noticed because none of them looked.
#
# The base's own kernel is the one whose modules and ABI match the Debian
# userspace now sitting in the squashfs.
echo "  Embedding the base system's kernel..."
BASE_KVER=$(sed -n 's/^kernel=vmlinuz-//p' "${BASE_DIR}/etc/hcs-build-base" 2>/dev/null | head -1)
if [ -z "${BASE_KVER}" ]; then
    BASE_KVER=$(ls "${BASE_DIR}"/boot/vmlinuz-* 2>/dev/null | sed 's|.*/vmlinuz-||' | sort -V | tail -1)
fi
[ -n "${BASE_KVER}" ] && [ -f "${BASE_DIR}/boot/vmlinuz-${BASE_KVER}" ] || {
    echo "  [ERROR] The base has no kernel in ${BASE_DIR}/boot." >&2
    echo "          Is linux-image-amd64 in config/package-lists/hcs-core.list.chroot?" >&2
    exit 1
}
cp -L "${BASE_DIR}/boot/vmlinuz-${BASE_KVER}" "${ISO_STAGING}/live/vmlinuz"
echo "  kernel: vmlinuz-${BASE_KVER}"

if [ -f "${BASE_DIR}/boot/initrd.img-${BASE_KVER}" ]; then
    cp -L "${BASE_DIR}/boot/initrd.img-${BASE_KVER}" "${ISO_STAGING}/live/initrd.img"
elif [ -f "${BASE_DIR}/boot/initrd.img" ]; then
    cp -L "${BASE_DIR}/boot/initrd.img" "${ISO_STAGING}/live/initrd.img"
else
    echo "  [ERROR] The base has no initrd for ${BASE_KVER}. Aborting." >&2
    exit 1
fi
echo "  initrd: $(basename "${ISO_STAGING}/live/initrd.img") ($(du -h "${ISO_STAGING}/live/initrd.img" | cut -f1))"

# The kernel modules must be in the squashfs, or the kernel boots and then cannot
# find a single driver for its own root filesystem.
echo "  Staging kernel modules into the root filesystem..."
mkdir -p "${ROOTFS_DIR}/lib/modules/${BASE_KVER}"
if [ -d "${BASE_DIR}/lib/modules/${BASE_KVER}" ]; then
    cp -a "${BASE_DIR}/lib/modules/${BASE_KVER}/." "${ROOTFS_DIR}/lib/modules/${BASE_KVER}/"
    echo "       $(find "${ROOTFS_DIR}/lib/modules/${BASE_KVER}" -type f | wc -l) module file(s)"
else
    echo "  [ERROR] The base has no modules for ${BASE_KVER}. Aborting." >&2
    exit 1
fi

# ------------------------------------------------------------------ 10. grub

# The QA image boots straight into the automated session; the production image
# cannot, because the agent refuses to start without the flag. See --qa above.
if [ "${QA}" -eq 1 ]; then
    GRUB_TIMEOUT=0
    GRUB_QA_ENTRY=1
else
    GRUB_TIMEOUT=10
    GRUB_QA_ENTRY=0
fi

# Unquoted heredoc so ${VERSION} and ${GRUB_*} expand host-side (body contains
# no other $).
cat > "${ISO_STAGING}/boot/grub/grub.cfg" << EOF
set timeout=${GRUB_TIMEOUT}
set default=0

insmod all_video
insmod font
insmod gfxterm
insmod vbe
insmod vga

set gfxmode=1280x800,auto
set gfxpayload=keep
terminal_output gfxterm

set menu_color_normal=light-gray/black
set menu_color_highlight=cyan/black

menuentry "HCS Linux ${VERSION} Live Desktop" {
    linux /live/vmlinuz boot=live console=tty1 console=tty0 video=1024x768 hcs_session=graphical
    initrd /live/initrd.img
}

menuentry "HCS Linux ${VERSION} Live Desktop (Amnesic / Tor)" {
    linux /live/vmlinuz boot=live console=tty1 console=tty0 video=1024x768 hcs_session=graphical hcs_amnesic=1 hcs_private=1
    initrd /live/initrd.img
}

menuentry "Install HCS Linux (Calamares)" {
    linux /live/vmlinuz boot=live console=tty1 console=tty0 video=1024x768 hcs_install=1
    initrd /live/initrd.img
}

menuentry "HCS Linux ${VERSION} Recovery Console" {
    linux /live/vmlinuz boot=live single console=tty1 console=tty0 video=1024x768 hcs_console=1
    initrd /live/initrd.img
}

menuentry "Boot previous installation (rollback)" {
    linux /live/vmlinuz boot=live single console=tty1 console=tty0 video=1024x768 hcs_rollback=1
    initrd /live/initrd.img
}

menuentry "HCS Linux ${VERSION} QA (automated — no interaction)" {
    linux /live/vmlinuz boot=live console=tty1 console=tty0 video=1024x768 hcs_session=graphical hcs.qa=1 hcs.qa.profile=virtualbox
    initrd /live/initrd.img
}
EOF

# The QA entry is what the QA image must land on. `set default` above is 0, so
# in a QA build the automated entry is moved to the top rather than relying on
# the driver to count menu rows — a missed keystroke used to be indistinguishable
# from a broken image.
if [ "${GRUB_QA_ENTRY}" -eq 1 ]; then
    python3 - "${ISO_STAGING}/boot/grub/grub.cfg" << 'PY'
import re, sys
path = sys.argv[1]
text = open(path, encoding="utf-8").read()
m = re.search(r'menuentry "HCS Linux [^\n]*QA \(automated[^\n]*\{.*?\n\}\n', text, re.S)
if not m:
    sys.exit("could not find the QA menuentry to promote")
qa = m.group(0)
text = text.replace(qa, "")
# Insert ahead of the first menuentry.
first = re.search(r'menuentry ', text)
text = text[:first.start()] + qa + "\n" + text[first.start():]
open(path, "w", encoding="utf-8").write(text)
print("[grub] QA entry promoted to default (this is the QA image)")
PY
fi

mkdir -p "${ISO_STAGING}/boot/branding"
cp -rf "${REPO_ROOT}/assets/logo/." "${ISO_STAGING}/boot/branding/"

# ------------------------------------------------------------------ 11. iso

echo "[9/9] Generating bootable hybrid ISO with grub-mkrescue..."
grub-mkrescue -o "${FINAL_ISO}" "${ISO_STAGING}"

echo "[10/10] Auditing generated ISO checksum and headers..."
( cd "${DIST_DIR}" && sha256sum "$(basename "${ISO_NAME}")" > SHA256SUMS )
python3 "${SCRIPT_DIR}/verify_iso.py" "${FINAL_ISO}"

echo ""
echo "=================================================="
echo "  [SUCCESS] HCS Linux Live ISO Build Complete!    "
echo "  ISO Path: ${FINAL_ISO}                          "
echo "=================================================="
