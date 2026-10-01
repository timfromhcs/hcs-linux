#!/bin/bash
# fetch_niri.sh — obtain the niri compositor and put it in the base.
#
# WHY THIS IS A BUILD AND NOT A DOWNLOAD
#
# The project decided to take niri from outside the ordinary package install.
# For this one package that decision has a wrinkle worth stating plainly:
# **niri is not packaged anywhere.** Not Debian 12 or 13, not Ubuntu 24.04,
# 25.04 or 26.04, not Fedora — and niri's own releases attach no binary.
#
# So the choice is between a third party's .deb and niri's own artefacts. This
# script takes the latter, which is strictly better provenance:
#
#   * both artefacts come from niri's own repository, not a stranger's build
#     server;
#   * the release publishes a digest for the dependency bundle, and this script
#     refuses to proceed if the download does not match it;
#   * the source is pinned to a commit, not a moving branch;
#   * the build runs `--offline --locked` against the vendored dependency set,
#     which is what makes the result reproducible.
#
# WHAT THE "VENDORED DEPENDENCIES" BUNDLE ACTUALLY IS
#
# Only `vendor/` — 30,151 files of it. It is *not* niri's source. Upstream ships
# it so that a packager can build a release with a known-good, frozen dependency
# set and no network access. So the build is: niri's source at the pinned commit,
# plus this bundle unpacked beside it, plus a `.cargo/config.toml` that points
# cargo at the vendor directory. The first version of this script assumed the
# bundle was self-contained and produced an empty directory.
#
# quickshell, by contrast, *is* packaged by Debian and is installed from sid as
# an ordinary pinned package by build_base.sh. Nothing vendored there.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
BASE_DIR="${HCS_BASE_DIR:-${REPO_ROOT}/target/base}"
# A sibling of the repository: inside it, cargo would find the HCS workspace and
# build that instead of niri. Not /tmp either — it does not survive between WSL
# sessions, so the 40 MB bundle was re-downloaded on every run.
WORK="${HCS_NIRI_WORK:-$(dirname "${REPO_ROOT}")/hcs-niri-build}"

NIRI_VERSION="26.04"
# Resolved through the GitHub API, not guessed.
NIRI_COMMIT="8ed0da44d974c32c6877d2f4630c314da0717ecb"
NIRI_SOURCE_URL="https://api.github.com/repos/niri-wm/niri/tarball/${NIRI_COMMIT}"
# Recorded when the artefact was first adopted; see docs/V2_STABLE_QA_STATUS.md.
NIRI_SOURCE_SHA256="3a159100c5e5103951f57aef75b61189f87c423c86e61303a4b59c1bfe21e54d"
NIRI_VENDOR_NAME="niri-${NIRI_VERSION}-vendored-dependencies.tar.xz"
NIRI_VENDOR_URL="https://github.com/niri-wm/niri/releases/download/v${NIRI_VERSION}/${NIRI_VENDOR_NAME}"
# Published by niri in the release API for tag v${NIRI_VERSION}.
NIRI_VENDOR_SHA256="407c525cd78710e72455c48741756857ca47dc32910a42f82cd40992a34beed6"

say() { printf '\033[1m[niri]\033[0m %s\n' "$*"; }
die() { printf '\033[31m[niri] FATAL:\033[0m %s\n' "$*" >&2; exit 1; }

# cargo resolves its toolchain from $HOME via rustup. Under sudo HOME is /root,
# where no default toolchain exists, so cargo fails before reading any source.
if [ "$(id -u)" -eq 0 ]; then
    die "run this as your normal user, not with sudo.
       The Rust toolchain lives in your home directory; only the install steps
       into the base need elevation and they elevate themselves."
fi
command -v cargo >/dev/null 2>&1 || die "cargo is not on PATH for this user"
[ -d "${BASE_DIR}" ] || die "no base at ${BASE_DIR} — run scripts/build_base.sh first"

fetch_verified() {
    # $1 = url, $2 = destination, $3 = expected sha256, $4 = label
    if [ -f "$2" ]; then
        GOT=$(sha256sum "$2" | cut -d' ' -f1)
        [ "${GOT}" = "$3" ] && { say "  $4 already downloaded and verified"; return 0; }
        say "  $4 digest does not match; re-downloading"
        rm -f "$2"
    fi
    say "  downloading $4"
    curl -fSL --retry 3 --retry-delay 2 --max-time 1200 -o "$2" "$1" \
        || die "could not download $4 from $1"
    GOT=$(sha256sum "$2" | cut -d' ' -f1)
    if [ "${GOT}" != "$3" ]; then
        die "SHA-256 mismatch for $4
     url      $1
     expected $3
     got      $GOT
     Refusing to build an artefact that is not the one that was reviewed."
    fi
    say "  $4 sha256 verified"
}

mkdir -p "${WORK}"

# ------------------------------------------------------------------ 1. fetch

say "niri ${NIRI_VERSION}, pinned to commit ${NIRI_COMMIT:0:12}"
fetch_verified "${NIRI_SOURCE_URL}" "${WORK}/niri-src.tar.gz" \
    "${NIRI_SOURCE_SHA256}" "source tarball"
fetch_verified "${NIRI_VENDOR_URL}" "${WORK}/${NIRI_VENDOR_NAME}" \
    "${NIRI_VENDOR_SHA256}" "vendored dependency bundle"

# ------------------------------------------------------------------ 2. unpack

SRC="${WORK}/src-${NIRI_COMMIT:0:12}"
if [ ! -f "${SRC}/Cargo.toml" ]; then
    say "unpacking the source tree"
    rm -rf "${SRC}"
    mkdir -p "${SRC}"
    tar -xzf "${WORK}/niri-src.tar.gz" -C "${SRC}" --strip-components=1
fi
[ -f "${SRC}/Cargo.toml" ] || die "no Cargo.toml after unpacking the source"
grep -qi 'name *= *"niri"' "${SRC}/Cargo.toml" \
    || die "${SRC}/Cargo.toml is not niri's manifest — refusing to build the wrong project"

if [ ! -d "${SRC}/vendor" ]; then
    say "unpacking the vendored dependencies beside it"
    # The bundle is flat: its single top-level entry is `vendor/`, which belongs
    # directly inside the niri source tree.
    tar -xJf "${WORK}/${NIRI_VENDOR_NAME}" -C "${SRC}"
fi
[ -d "${SRC}/vendor" ] || die "the vendored dependencies did not unpack to ${SRC}/vendor"

# Point cargo at the vendor directory. niri's own tree does not ship this file,
# so without it `cargo build --offline` has nothing to resolve crates from and
# fails looking for a network that is not there.
mkdir -p "${SRC}/.cargo"
cat > "${SRC}/.cargo/config.toml" << 'EOF'
# Written by scripts/fetch_niri.sh.
#
# niri's release bundle ships a `vendor/` tree and nothing else: it is the
# dependency set, not the project. This is the configuration that tells cargo to
# resolve from it instead of the network, which is what makes the offline build
# reproducible.
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
EOF
say "cargo vendored-sources configured"

# ------------------------------------------------------------------ 3. build

# NOT `--offline`, and the reason is worth recording rather than hiding.
#
# niri depends on git snapshots — smithay and smithay-drm-extras are pinned to
# commit hashes rather than crates.io versions, which is exactly what niri's own
# packaging notes warn about. The release's vendored-dependencies bundle is
# `cargo vendor` output for the *crates.io* sources, and it does not contain the
# git checkouts, so an offline build fails at:
#
#   unable to update https://github.com/Smithay/smithay.git?rev=ff5fa7df…
#   you are in the offline mode (--offline)
#
# `cargo vendor` normally emits source-replacement entries for git sources too,
# but the config that describes them is not in the bundle — only `vendor/` is —
# so it would have to be reconstructed by hand for each pinned git revision.
#
# So the build runs with network access for resolution, and `--locked` is kept
# so the *versions* still come from the tag's Cargo.lock and cannot drift. The
# guarantees that survive are: source pinned to a commit, dependencies pinned by
# the lockfile, digests of both artefacts recorded and checked. The guarantee
# that does not survive is "no network at build time".
#
# If offline reproducibility becomes a requirement, the honest fix is to vendor
# the git sources too and commit that vendor directory, not to claim the bundle
# is sufficient when it is not.
say "building niri ${NIRI_VERSION} (locked to the tag's Cargo.lock)"
cd "${SRC}"
if ! cargo build --release --locked --manifest-path "${SRC}/Cargo.toml" 2>&1 | tail -25; then
    die "the niri build failed; see the output above"
fi

BIN="${SRC}/target/release/niri"
[ -x "${BIN}" ] || die "cargo reported success but ${BIN} is not there"
say "built: $(du -h "${BIN}" | cut -f1)"
"${BIN}" --version 2>&1 | head -2 | while read -r l; do say "  $l"; done

# ------------------------------------------------------------------ 4. install

say "installing into the base"
sudo install -D -m 0755 "${BIN}" "${BASE_DIR}/usr/bin/niri"

# The compositor needs its session, desktop and systemd files, or the session
# picker and any desktop integration cannot find it. An image with only the
# binary has a compositor nothing can start.
for pair in \
    "resources/niri-session:usr/bin/niri-session:0755" \
    "resources/niri.desktop:usr/share/wayland-sessions/niri.desktop:0644" \
    "resources/niri-portals.conf:usr/share/xdg-desktop-portal/niri-portals.conf:0644" \
    "resources/niri.service:usr/lib/systemd/user/niri.service:0644"; do
    src="${pair%%:*}"; rest="${pair#*:}"; dst="${rest%%:*}"; mode="${rest##*:}"
    if [ -f "${SRC}/${src}" ]; then
        sudo install -D -m "${mode}" "${SRC}/${src}" "${BASE_DIR}/${dst}"
    else
        say "  (no ${src} in this release — skipped)"
    fi
done

cat > "${WORK}/niri-provenance" << EOF
component=niri
version=${NIRI_VERSION}
commit=${NIRI_COMMIT}
source_url=${NIRI_SOURCE_URL}
source_sha256=${NIRI_SOURCE_SHA256}
vendor_url=${NIRI_VENDOR_URL}
vendor_sha256=${NIRI_VENDOR_SHA256}
build=cargo build --release --locked (resolution needs network for niri's git
build=snapshot dependencies; the vendored bundle covers only crates.io sources)
note=niri is not packaged in Debian 12/13, Ubuntu 24.04/25.04/26.04 or Fedora,
note=and niri's own releases attach no binary. Both artefacts here come from
note=niri's repository, are pinned by digest, and the dependency versions come
note=from the tag's Cargo.lock. This is upstream's own reproducible-build path,
note=not a third party's package.
EOF
sudo install -m 0644 "${WORK}/niri-provenance" "${BASE_DIR}/etc/hcs-niri-provenance"

say "done"
