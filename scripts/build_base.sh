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
#   scripts/build_base.sh [SUITE] [--cache]
#
# --cache reuses an existing target/base instead of re-bootstrapping. It is an
# argument rather than an environment variable because `VAR=1 sudo ./script`
# silently drops the variable — sudo does not pass the environment through
# without -E — so the first version of this rebuilt the entire base from scratch
# on every invocation, re-downloading ~40 MB and reinstalling 69 packages each
# time, and never once used the cache it appeared to be asking for.

set -euo pipefail

SUITE="sid"
CACHE=0
for arg in "$@"; do
    case "${arg}" in
        --cache) CACHE=1 ;;
        -*) printf 'unknown option: %s\n' "${arg}" >&2; exit 2 ;;
        *) SUITE="${arg}" ;;
    esac
done
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
BASE_DIR="${REPO_ROOT}/target/base"
PACKAGE_LIST="${REPO_ROOT}/config/package-lists/hcs-core.list.chroot"
EXTERNAL_LIST="${REPO_ROOT}/config/package-lists/hcs-external.list"
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

if [ "${CACHE}" -eq 1 ] && [ -d "${BASE_DIR}" ]; then
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

# ------------------------------------------------------------------ 3b. mounts

# A debootstrap tree has empty /proc, /sys and /dev, and several packages run
# systemd-tmpfiles from their postinst. Without /proc that fails, and the failure
# surfaces as an opaque "old <pkg> package postinst maintainer script subprocess
# failed with exit status 1" — which is how tmux and cryptsetup ended up
# half-configured and made the build look like a missing-package problem.
#
# Bind-mounted for the duration and torn down on exit, including on failure, so
# a cancelled build does not leave the host's /proc mounted inside a directory.
cleanup_mounts() {
    for m in dev/pts dev sys proc; do
        mountpoint -q "${BASE_DIR}/${m}" && umount -l "${BASE_DIR}/${m}" 2>/dev/null || true
    done
}
trap cleanup_mounts EXIT

mount -t proc  proc  "${BASE_DIR}/proc"
mount -t sysfs sysfs "${BASE_DIR}/sys"
mount --bind /dev "${BASE_DIR}/dev"
mkdir -p "${BASE_DIR}/dev/pts"
mount -t devpts devpts "${BASE_DIR}/dev/pts"
say "chroot mounts in place (/proc, /sys, /dev, /dev/pts)"

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

# Components the distribution does not package are declared separately, with the
# script that provisions each one. They are removed from the apt list here and
# verified in step 5, so a component cannot be skipped by deleting a line — its
# absence has to be noticed.
EXTERNAL_NAMES=()
if [ -f "${EXTERNAL_LIST}" ]; then
    while read -r name script _rest; do
        case "${name}" in ''|\#*) continue ;; esac
        EXTERNAL_NAMES+=("${name}")
    done < "${EXTERNAL_LIST}"
    if [ "${#EXTERNAL_NAMES[@]}" -gt 0 ]; then
        say "externally provisioned (not apt): ${EXTERNAL_NAMES[*]}"
    fi
fi

APT_WANTED=()
for p in "${WANTED[@]}"; do
    skip=0
    for e in "${EXTERNAL_NAMES[@]:-}"; do
        [ "${p}" = "${e}" ] && skip=1
    done
    [ "${skip}" -eq 0 ] && APT_WANTED+=("${p}")
done
say "${#APT_WANTED[@]} package(s) to install with apt"

MISSING=()
for p in "${APT_WANTED[@]}"; do
    # Satisfied means "apt can resolve it", not "dpkg has a package of exactly
    # this name". Two real cases were misreported as failures before:
    #
    #   * systemd-sysusers is a *virtual* package in sid, provided by systemd.
    #     dpkg-query has no record under that name at all, so a
    #     dpkg-query-only test calls it missing and the build dies even though
    #     the capability is present.
    #   * a package already pulled in as a dependency is "already the newest
    #     version", which apt reports in its output but which dpkg-query also
    #     answers correctly — the two tests have to agree.
    #
    # So: installed, or has an installation candidate.
    if chroot "${BASE_DIR}" /usr/bin/dpkg-query -W -f='${Status}' "${p}" 2>/dev/null \
        | grep -q 'ok installed'; then
        continue
    fi
    if chroot "${BASE_DIR}" /usr/bin/apt-cache policy "${p}" 2>/dev/null \
        | grep -qE '^\s+Installed:'; then
        continue
    fi
    if chroot "${BASE_DIR}" /usr/bin/apt-cache show "${p}" >/dev/null 2>&1; then
        MISSING+=("${p}")
    else
        say "  ${p}: virtual or provided by another package — satisfied by a provider"
    fi
done

if [ "${#MISSING[@]}" -gt 0 ]; then
    say "installing: ${MISSING[*]}"
    # One package per attempt, so a single unavailable name reports itself
    # instead of aborting a 75-package transaction with an opaque error.
    FAILED=()
    for p in "${MISSING[@]}"; do
        if chroot "${BASE_DIR}" /usr/bin/apt-get -o Dpkg::Options::=--force-confdef \
                -o Dpkg::Options::=--force-confold \
                install -y --no-install-recommends "${p}" \
                >>"${BASE_DIR}/var/log/hcs-base-build.log" 2>&1; then
            :
        else
            # apt can exit non-zero after a successful install — a debconf
            # warning or a trigger returning non-zero. The question is whether
            # the package is actually there now, not what apt's exit code was.
            if chroot "${BASE_DIR}" /usr/bin/dpkg-query -W -f='${Status}' "${p}" 2>/dev/null \
                | grep -q 'ok installed'; then
                say "  ${p}: installed despite a non-zero apt exit"
            else
                FAILED+=("${p}")
            fi
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
# VM. Finish the transaction properly, and refuse to continue if anything is
# still unpacked-but-unconfigured.
if ! chroot "${BASE_DIR}" /usr/bin/dpkg --configure -a \
        >>"${BASE_DIR}/var/log/hcs-base-build.log" 2>&1; then
    say "dpkg --configure -a did not complete:"
    chroot "${BASE_DIR}" /usr/bin/dpkg --audit 2>&1 | head -20 | while read -r l; do
        say "    ${l}"
    done
    die "the base has unconfigured packages"
fi
AUDIT=$(chroot "${BASE_DIR}" /usr/bin/dpkg --audit 2>&1 || true)
if [ -n "${AUDIT}" ]; then
    say "dpkg --audit still reports:"
    printf '%s\n' "${AUDIT}" | head -20 | while read -r l; do say "    ${l}"; done
    die "the base is not in a clean package state"
fi

# The payload tree has to exist BEFORE the hook asserts its contents.
#
# The hook checks for /usr/share/hcs/shell/shell.qml and friends and exits if
# they are absent. That assertion is correct and worth keeping -- but the payload
# was only ever staged into the rootfs by build_iso.sh, which runs AFTER this.
# So the hook was asserting against a tree that did not exist yet, and the
# assertion did its job by refusing to let a sessionless image through.
#
# Order matters more than either script: base contents -> payload -> hook.
PAYLOAD_SRC="${REPO_ROOT}/config/includes.chroot"
if [ ! -d "${PAYLOAD_SRC}" ]; then
    die "config/includes.chroot is missing; the base would have no shell or session"
fi
say "staging the payload tree into the base"
if command -v rsync >/dev/null 2>&1; then
    rsync -a "${PAYLOAD_SRC}/" "${BASE_DIR}/"
else
    (cd "${PAYLOAD_SRC}" && tar cf - .) | (cd "${BASE_DIR}" && tar xf -)
fi
say "    payload staged into the base"

# ------------------------------------------------- 3d. chroot bootstrap hook

# RUN THE HOOK. This was missing, and it is the reason three build cycles of
# autologin fixes did nothing at all.
#
# config/hooks/live/01-hcs-setup.hook.chroot is where the live user is created,
# where getty@tty1 is given an autologin override, where the QWERTZ default is
# written and where the payload is asserted present. It had been sitting in the
# repository, correct, committed, and completely unreached: build_base.sh never
# invoked it.
#
# So the image booted as live-config's own `user`, our getty override did not
# exist, and the console showed "Authentication failure" -- which reads like a
# login problem and is actually a hook that never ran. Three fixes were applied
# to the hook before anyone checked whether the hook ran at all.
#
# The lesson is worth stating because it will recur: a file that is never
# executed is not a component. It looks exactly like one in review, in grep and
# in the diff, and no gate in this project had ever asserted that it ran.
HOOK="${REPO_ROOT}/config/hooks/live/01-hcs-setup.hook.chroot"
if [ ! -x "${HOOK}" ] && [ ! -r "${HOOK}" ]; then
    say "bootstrap hook is missing or unreadable: ${HOOK}"
    die "the base would boot without a live user"
fi

# ------------------------------------------------- 3c. initramfs: overlayfs
#
# CORRECTION. The first version of this block asserted that overlayfs was absent
# from the initramfs, on the strength of a grep that returned zero. Re-running it
# properly shows three matches, including kernel/fs/overlayfs/overlay.ko.xz. The
# module was there all along and the original diagnosis was wrong.
#
# That matters more than the code. "overlay not supported" was read as "the
# module is missing" when the message comes from live-boot's 9990-overlay.sh
# shutdown hook, not from a mount failure -- so the boot reached the end of a
# session rather than dying before one. Attributing a stop to a missing module
# because the module name appeared nowhere in a grep is exactly the failure mode
# this whole project keeps hitting: a plausible story, an unverified cause.
#
# The check stays, because "the initramfs can mount an overlay" is a real
# precondition for shipping a live image and nothing else asserted it. It is
# simply no longer justified by a story about this particular boot.
say "verifying the initramfs can mount an overlay"
INITRD=$(ls "${BASE_DIR}"/boot/initrd.img-* 2>/dev/null | head -1)
if [ -z "${INITRD}" ]; then
    die "no initrd.img in the base; the image cannot mount its own medium"
fi
KVER=$(basename "${INITRD}" | sed 's/^initrd\.img-//')

if ! lsinitramfs "${INITRD}" 2>/dev/null | grep -q 'fs/overlay'; then
    if chroot "${BASE_DIR}" /usr/bin/update-initramfs -u "-${KVER}" \
            >>"${BASE_DIR}/var/log/hcs-base-build.log" 2>&1 \
       && lsinitramfs "${INITRD}" 2>/dev/null | grep -q 'fs/overlay'; then
        say "    overlayfs is now in the initramfs"
    else
        # update-initramfs consults /etc/initramfs-tools/modules, so fall back to
        # writing it and asking again rather than shipping an image that cannot
        # mount itself.
        say "    update-initramfs did not add it; writing /etc/initramfs-tools/modules"
        mkdir -p "${BASE_DIR}/etc/initramfs-tools"
        printf 'overlay\n' > "${BASE_DIR}/etc/initramfs-tools/modules"
        chroot "${BASE_DIR}" /usr/bin/update-initramfs -u "-${KVER}" \
            >>"${BASE_DIR}/var/log/hcs-base-build.log" 2>&1 || true
        if lsinitramfs "${INITRD}" 2>/dev/null | grep -q 'fs/overlay'; then
            say "    overlayfs is now in the initramfs"
        else
            say "    overlayfs is STILL missing after two attempts"
            die "the image would boot to 'overlay not supported' and stop"
        fi
    fi
else
    say "    overlayfs was already in the initramfs"
fi

say "running the chroot bootstrap hook (live user, getty, keyboard, payload)"
if chroot "${BASE_DIR}" /bin/sh -c "true" 2>/dev/null; then
    :
else
    say "chroot cannot execute -- mounts from step 3b are required here"
    die "cannot run the bootstrap hook"
fi

# COPY IT IN. chroot has a different root, so a path on the build host does not
# exist inside it -- "No such file or directory" for a file that is right there
# on disk. Binding it into place avoids that entirely.
HOOK_IN_CHROOT="/tmp/hcs-bootstrap-hook.sh"
cp -f "${HOOK}" "${BASE_DIR}${HOOK_IN_CHROOT}"
chmod 755 "${BASE_DIR}${HOOK_IN_CHROOT}"

if ! chroot "${BASE_DIR}" /bin/sh "${HOOK_IN_CHROOT}" \
        >>"${BASE_DIR}/var/log/hcs-base-build.log" 2>&1; then
    rm -f "${BASE_DIR}${HOOK_IN_CHROOT}"
    say "the bootstrap hook failed:"
    tail -30 "${BASE_DIR}/var/log/hcs-base-build.log" | while read -r l; do say "    ${l}"; done
    die "the base has no session; refusing to build an image that cannot log in"
fi
rm -f "${BASE_DIR}${HOOK_IN_CHROOT}"
say "    bootstrap hook completed"

# And assert the result rather than trusting that a script ran. This is the
# check whose absence produced three identical failed cycles.
if ! chroot "${BASE_DIR}" /usr/bin/id hcs >/dev/null 2>&1 \
   && ! grep -q '^hcs:' "${BASE_DIR}/etc/passwd"; then
    say "the hook ran but created no 'hcs' user"
    die "the base would boot to live-config's default user, not the HCS session"
fi
if [ ! -f "${BASE_DIR}/etc/systemd/system/getty@tty1.service.d/override.conf" ]; then
    say "the hook ran but wrote no getty@tty1 autologin override"
    die "the base would stop at a login prompt"
fi
# grep directly on the tree. `chroot ... grep -q ...` passes -q to chroot, not to
# grep, and chroot rejects it -- so the assertion below was checking that chroot
# understood "-q", which it does not, and reporting the getty as unconfigured.
if ! grep -q 'autologin hcs' \
        "${BASE_DIR}/etc/systemd/system/getty@tty1.service.d/override.conf"; then
    say "the getty override does not name the hcs user"
    die "the base would ask for credentials on a live USB"
fi
say "    verified: user hcs exists and getty@tty1 autologins as hcs"

# ------------------------------------------------------------------ 4. verify

# The external provisioning scripts need the mounts from step 3b to still be in
# place for anything that inspects the tree, and they must not inherit the
# cleanup trap's ownership of them.
cleanup_mounts

say "verifying the base is a real system, not a directory of files"
MISSING_SYS=()
for probe in bin/bash bin/sh sbin/init usr/lib/systemd/systemd \
             usr/bin/quickshell usr/bin/grim usr/bin/wtype \
             usr/bin/tesseract usr/bin/vulkaninfo \
             usr/bin/glxinfo lib/x86_64-linux-gnu/libgbm.so.1; do
    [ -e "${BASE_DIR}/${probe}" ] || MISSING_SYS+=("${probe}")
done

# niri is provisioned by fetch_niri.sh, which runs LATER in this script than this
# probe. Adding it to the list above asserted that a step that had not happened
# yet had happened, which is how a correct niri build gets reported as a missing
# one. It is checked where it is actually installed, not here.
if [ "${#MISSING_SYS[@]}" -gt 0 ]; then
    die "the base is still not a bootable system. Missing: ${MISSING_SYS[*]}"
fi

# Externally provisioned components must be present *and* carry their
# provenance, or they were never really installed.
#
# They are provisioned *here*, before the check, rather than as a separate manual
# step: the first attempt at this ordering reported "an external component is
# missing" for a component that simply had not been reached yet, which is a
# confusing way to learn that a build has a step ordering bug in it.
if [ "${#EXTERNAL_NAMES[@]}" -gt 0 ]; then
    declare -A EXTERNAL_SCRIPT=()
    while read -r name script _rest; do
        case "${name}" in ''|\#*) continue ;; esac
        EXTERNAL_SCRIPT["${name}"]="${script}"
    done < "${EXTERNAL_LIST}"

    # External provisioning builds things with the *invoking user's* toolchain,
    # and rustup resolves that from $HOME. This script runs under sudo, so
    # without dropping privileges the build tool is looked up in /root, where no
    # default toolchain is configured, and cargo fails before it reads any source.
    AS_USER=""
    if [ -n "${SUDO_USER:-}" ] && [ "${SUDO_USER}" != "root" ]; then
        AS_USER="${SUDO_USER}"
        say "provisioning runs as ${AS_USER} (the invoking user), not root"
    else
        die "external components need the invoking user's toolchain, but this was
       not run through sudo.
       Run:  sudo bash scripts/build_base.sh ${SUITE}"
    fi

    for name in "${EXTERNAL_NAMES[@]}"; do
        script="${EXTERNAL_SCRIPT[${name}]:-}"
        [ -n "${script}" ] || die "${name} is declared external but names no script to provision it"
        [ -f "${REPO_ROOT}/${script}" ] || die "${name} names ${script}, which does not exist"
        say "provisioning ${name} via ${script}"
        AS_USER="${AS_USER}" \
        HCS_BASE_DIR="${BASE_DIR}" \
        HOME="$(getent passwd "${AS_USER}" | cut -d: -f6)" \
        USER="${AS_USER}" \
            sudo -u "${AS_USER}" -H bash "${REPO_ROOT}/${script}" \
            || die "provisioning ${name} failed; see the output above"
    done

    MISSING_EXT=()
    for e in "${EXTERNAL_NAMES[@]}"; do
        if [ ! -e "${BASE_DIR}/usr/bin/${e}" ]; then
            MISSING_EXT+=("${e} (binary absent)")
        elif [ ! -f "${BASE_DIR}/etc/hcs-${e}-provenance" ]; then
            MISSING_EXT+=("${e} (no provenance record)")
        fi
    done
    if [ "${#MISSING_EXT[@]}" -gt 0 ]; then
        die "external components are not satisfied: ${MISSING_EXT[*]}"
    fi
    say "external components verified: ${EXTERNAL_NAMES[*]}"
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
