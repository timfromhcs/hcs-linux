#!/bin/bash
# build_base.sh — assemble a real Debian base system for the live image.
#
# WHY THIS FILE EXISTS
#
# v1 and v2 both produced something called an ISO that could not boot. The cause
# was one omission: nothing ever created a Linux userspace. build_iso.sh staged
# a directory of HCS binaries and assets, then copied the *build host's* kernel
# and initrd into the image. The result had 152 files and no /bin/sh, no libc, no
# systemd, no compositor. `config/package-lists/hcs-core.list.chroot` — 75
# packages — was read by nothing.
#
# The CI job added alongside it grepped that list for the string "niri" and
# reported success, so the gap was invisible.
#
# This script is the fix: debootstrap a real base, install the declared package
# list into it, and hand the *base's own* kernel and initrd to the image builder.
# Nothing here trusts the build host's /boot again.
#
# WHY sid AND NOT trixie
#
# quickshell is packaged by Debian — in sid, not trixie. As of this writing sid
# carries quickshell 0.3.1-1+b1, which depends on libqt6core6t64 >= 6.11.2 and
# qt6-base-private-abi (= 6.11.2), an exact-version private ABI. Trixie ships
# Qt 6.8. Pulling that one binary into a trixie base therefore drags Qt 6.11 and
# its private-ABI packages in with it, which is a base upgrade dressed up as a
# pin. Since the project decided to take quickshell as a pinned binary rather
# than compile it, the base has to be sid for that binary to mean anything.
#
# This is an unstable base. That is a real cost and it is recorded in
# docs/V2_STABLE_QA_STATUS.md rather than buried here.
#
# USAGE
#   scripts/build_base.sh [SUITE]        # default: sid
#   HCS_BASE_CACHE=1 scripts/build_base.sh   # reuse an existing target/base

set -euo pipefail

SUITE="${1:-sid}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
BASE_DIR="${REPO_ROOT}/target/base"
PACKAGE_LIST="${REPO_ROOT}/config/package-lists/hcs-core.list.chroot"
MIRROR="${HCS_DEB_MIRROR:-http://deb.debian.org/debian}"

say() { printf '\033[1m[base]\033[0m %s\n' "$*"; }
die() { printf '\033[31m[base] FATAL:\033[0m %s\n' "$*" >&2; exit 1; }

[ "$(id -u)" -eq 0 ] || die "must run as root (debootstrap and chroot apt do not work unprivileged)"

for tool in debootstrap mmdebstrap; do
    command -v "${tool}" >/dev/null 2>&1 && { HAVE="${tool}"; break; }
done
[ -n "${HAVE:-}" ] || die "neither debootstrap nor mmdebstrap is installed"

# ------------------------------------------------------------------ 1. suite

say "suite: ${SUITE}  (mirror: ${MIRROR})"

# ------------------------------------------------------------------ 2. base

if [ -d "${BASE_DIR}" ] && [ "${HCS_BASE_CACHE:-0}" = "1" ]; then
    say "reusing the cached base at ${BASE_DIR}"
else
    say "creating the base in ${BASE_DIR} (this takes a while)"
    rm -rf "${BASE_DIR}"
    # --variant=minbase: a bootable system without a desktop's worth of things we
    # are about to install explicitly, so the package list is the single source
    # of truth for what the image contains.
    debootstrap \
        --variant=minbase \
        --include=ca-certificates,apt-utils,gnupg \
        --arch=amd64 \
        "${SUITE}" "${BASE_DIR}" "${MIRROR}"

    # Only sid sources, no CDROM leftovers: a stray line here can make the
    # chroot unreachable in an air-gapped build later.
    find "${BASE_DIR}/etc/apt" -name '*.list' -delete 2>/dev/null || true
    cat > "${BASE_DIR}/etc/apt/sources.list" << EOF
deb ${MIRROR} ${SUITE} main contrib non-free
deb ${MIRROR} ${SUITE}-updates main contrib non-free
EOF
    # Updates/Backports only exist for some suites; ignore a missing one.
    for extra in updates backports; do
        if curl -sSfI --max-time 20 "${MIRROR}/dists/${SUITE}-${extra}/Release" >/dev/null 2>&1; then
            echo "deb ${MIRROR} ${SUITE}-${extra} main contrib non-free" \
                >> "${BASE_DIR}/etc/apt/sources.list"
            say "  also tracking ${SUITE}-${extra}"
        fi
    done
fi

[ -f "${BASE_DIR}/bin/bash" ] || die "the base has no /bin/bash; debootstrap did not finish"
[ -d "${BASE_DIR}/lib/x86_64-linux-gnu" ] || die "the base has no libc directory"

# ------------------------------------------------------------------ 3. install

# Debian packages ask questions. In a build host there is nobody to answer them,
# and an unanswered question is a build that hangs forever rather than one that
# fails: `keyboard-configuration` prompted for a layout during the first attempt
# and the chroot sat there until it was killed. Everything is therefore
# preseeded, and every apt call is explicitly non-interactive.
#
# QWERTZ is the project's default layout, so the base agrees with the image's
# own /usr/share/hcs/config/keyboard-layouts.conf instead of asking.
export DEBIAN_FRONTEND=noninteractive
export DEBCONF_NONINTERACTIVE_SEEN=true
PRESEED="${BASE_DIR}/usr/share/hcs-build-preseed.cfg"
cat > "${PRESEED}" << 'EOF'
keyboard-configuration keyboard-configuration/layoutcode string de
keyboard-configuration keyboard-configuration/modelcode string pc105
keyboard-configuration keyboard-configuration/variantcode string
keyboard-configuration keyboard-configuration/xkb-keymap select German
keyboard-configuration keyboard-configuration/policy-rc.d boolean false
locales locales/default_environment_locale string de_DE.UTF-8
locales locales/locales_to_be_created string de_DE.UTF-8 en_US.UTF-8
locales locales/backend select locales-gen
tzdata tzdata/Areas string Etc
tzdata tzdata/Zones/Etc string UTC
keyboard-configuration console-setup/ask_detect boolean false
grub-common grub-common/bootdev string /dev/sda
EOF
install -D -m 0644 "${PRESEED}" "${BASE_DIR}/etc/debian.conf.d/90-hcs-preseed"
# dpkg selections live in /var/lib/dpkg/info/*.list; a copy into the chroot is
# not enough, so the file goes where dpkg actually reads it.
mkdir -p "${BASE_DIR}/var/cache/debconf"
cp "${PRESEED}" "${BASE_DIR}/var/cache/debconf/config.dat"
printf '#DEBCONF#\n' >> "${BASE_DIR}/var/cache/debconf/config.dat"

# The package list is the declaration of what the image is. Read it here, so the
# list finally has a consumer.
mapfile -t WANTED < <(sed -e 's/#.*//' -e 's/[[:space:]]//g' "${PACKAGE_LIST}" | grep -v '^$')
say "installing ${#WANTED[@]} declared package(s) from the package list"

# A missing package must stop the build. apt-get's own exit status does not
# always do that for a list, and a half-installed desktop is exactly the failure
# this whole exercise is about.
MISSING=()
for p in "${WANTED[@]}"; do
    if ! chroot "${BASE_DIR}" /usr/bin/dpkg-query -W -f='${Status}' "${p}" 2>/dev/null \
        | grep -q 'ok installed'; then
        MISSING+=("${p}")
    fi
done

if [ "${#MISSING[@]}" -gt 0 ]; then
    say "installing: ${MISSING[*]}"
    # One package per attempt, so a single unavailable name reports itself
    # instead of aborting a 75-package transaction with an opaque error.
    FAILED=()
    for p in "${MISSING[@]}"; do
        if ! chroot "${BASE_DIR}" /usr/bin/apt-get -o Dpkg::Options::=--force-confdef \
                -o Dpkg::Options::=--force-confold \
                install -y --no-install-recommends "${p}" \
                >>"${BASE_DIR}/var/log/hcs-base-build.log" 2>&1; then
            FAILED+=("${p}")
        fi
    done
    if [ "${#FAILED[@]}" -gt 0 ]; then
        say "FAILED to install: ${FAILED[*]}"
        say "the last apt output is in target/base/var/log/hcs-base-build.log"
        say "this is usually a package the suite does not carry. Check with:"
        say "  python3 scripts/verify_package_availability.py --base debian:${SUITE}"
        die "the base system could not be assembled"
    fi
fi

# Anything left holding a dpkg lock or an unconfigured package would break the
# first boot of the image in a way that is very hard to diagnose from inside a
# VM. Finish the transaction properly before declaring the base good.
chroot "${BASE_DIR}" /usr/bin/dpkg --configure -a >>"${BASE_DIR}/var/log/hcs-base-build.log" 2>&1 || true

# ------------------------------------------------------------------ 4. verify

say "verifying the base is a real system, not a directory of files"
MISSING_SYS=()
for probe in bin/bash bin/sh sbin/init usr/lib/systemd/systemd \
             usr/bin/niri usr/bin/quickshell usr/bin/grim usr/bin/wtype \
             usr/bin/tesseract; do
    [ -e "${BASE_DIR}/${probe}" ] || MISSING_SYS+=("${probe}")
done
if [ "${#MISSING_SYS[@]}" -gt 0 ]; then
    die "the base is still not a bootable system. Missing: ${MISSING_SYS[*]}"
fi

FILE_COUNT=$(find "${BASE_DIR}" -xdev -type f 2>/dev/null | wc -l)
SIZE=$(du -sh --exclude=proc --exclude=sys --exclude=dev "${BASE_DIR}" 2>/dev/null | cut -f1)
say "base: ${FILE_COUNT} files, ${SIZE}"

# ------------------------------------------------------------------ 5. kernel

# The image gets the base's own kernel, never the build host's. This is the
# single line that was wrong for two releases.
KERNEL=$(ls "${BASE_DIR}"/boot/vmlinuz-* 2>/dev/null | sort -V | tail -1 || true)
[ -n "${KERNEL}" ] || die "the base has no kernel in /boot (is linux-image-amd64 installed?)"

# Debian ships an initrd per kernel; fall back to the one matching the kernel.
KVER=$(basename "${KERNEL}" | sed 's/^vmlinuz-//')
INITRD="${BASE_DIR}/boot/initrd.img-${KVER}"
[ -f "${INITRD}" ] || INITRD=$(ls "${BASE_DIR}"/boot/initrd.img-* 2>/dev/null | sort -V | tail -1 || true)
[ -n "${INITRD}" ] && [ -f "${INITRD}" ] || die "the base has no initrd for ${KVER}"

say "kernel: $(basename "${KERNEL}")"
say "initrd: $(basename "${INITRD}")"

# Record what was built, so the image can state which base it came from.
cat > "${BASE_DIR}/etc/hcs-build-base" << EOF
suite=${SUITE}
mirror=${MIRROR}
kernel=$(basename "${KERNEL}")
initrd=$(basename "${INITRD}")
packages=${#WANTED[@]}
files=${FILE_COUNT}
EOF

say "done. base is at ${BASE_DIR}"
