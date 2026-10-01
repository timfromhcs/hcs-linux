#!/bin/bash
# fetch_niri.sh — obtain the niri compositor and put it in the base.
#
# WHY THIS IS A BUILD AND NOT A DOWNLOAD
#
# The project decided to take niri from an external source rather than compile it
# as part of the ordinary package install. There is a problem with that decision
# for this one package: **niri is not packaged anywhere.** Not in Debian 12 or
# 13, not in Ubuntu 24.04, 25.04 or 26.04, not in Fedora, and niri's own GitHub
# releases attach no binary — the only asset on release v26.04 is a vendored
# source tarball.
#
# So the choice is between trusting a third party's .deb or trusting niri's own
# hash-pinned release artefact. This script takes the second, which is strictly
# better provenance:
#
#   * it comes from niri's own release, not a stranger's build server;
#   * upstream publishes it precisely so packagers can build the release
#     "completely offline" from a known-good dependency set, which is the
#     reproducibility property a supply chain needs;
#   * the SHA-256 below is upstream's, published in the GitHub release API, and
#     the script refuses to proceed if the download does not match it.
#
# quickshell, by contrast, *is* packaged by Debian and is installed from sid by
# build_base.sh as an ordinary pinned package. No vendoring needed there.
#
# USAGE
#   scripts/fetch_niri.sh                 # build into target/base
#   HCS_BASE_DIR=… scripts/fetch_niri.sh  # build into another tree

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
BASE_DIR="${HCS_BASE_DIR:-${REPO_ROOT}/target/base}"
WORK="${REPO_ROOT}/target/niri-build"

NIRI_VERSION="26.04"
NIRI_ARCHIVE="niri-${NIRI_VERSION}-vendored-dependencies.tar.xz"
# Published in the GitHub release API for tag v26.04, not guessed.
NIRI_URL="https://github.com/niri-wm/niri/releases/download/v${NIRI_VERSION}/${NIRI_ARCHIVE}"
NIRI_SHA256="407c525cd78710e72455c48741756857ca47dc32910a42f82cd40992a34beed6"

say() { printf '\033[1m[niri]\033[0m %s\n' "$*"; }
die() { printf '\033[31m[niri] FATAL:\033[0m %s\n' "$*" >&2; exit 1; }

[ -d "${BASE_DIR}" ] || die "no base at ${BASE_DIR} — run scripts/build_base.sh first"

# ------------------------------------------------------------------ 1. fetch

mkdir -p "${WORK}"
TARBALL="${WORK}/${NIRI_ARCHIVE}"

if [ ! -f "${TARBALL}" ]; then
    say "downloading ${NIRI_ARCHIVE} (${NIRI_VERSION})"
    curl -fSL --retry 3 --retry-delay 2 --max-time 900 -o "${TARBALL}" "${NIRI_URL}" \
        || die "could not download ${NIRI_URL}"
fi

say "verifying the SHA-256 against the one upstream published"
GOT=$(sha256sum "${TARBALL}" | cut -d' ' -f1)
if [ "${GOT}" != "${NIRI_SHA256}" ]; then
    # Loudly, and without continuing. A mismatch here means the artefact is not
    # the one that was reviewed, which is the whole reason the hash exists.
    die "SHA-256 mismatch for ${NIRI_ARCHIVE}
     expected ${NIRI_SHA256}
     got      ${GOT}
     The download does not match niri's published digest. Refusing to build it."
fi
say "sha256 ok: ${GOT}"

# ------------------------------------------------------------------ 2. unpack

SRC="${WORK}/niri-${NIRI_VERSION}"
if [ ! -d "${SRC}" ]; then
    say "unpacking"
    rm -rf "${SRC}"
    mkdir -p "${SRC}"
    tar -xJf "${TARBALL}" -C "${SRC}" --strip-components=1
fi

# ------------------------------------------------------------------ 3. build

# Built on the host, with the vendored dependency set, then copied into the base.
# Not built inside the chroot: niri needs Rust >= 1.85 plus the wayland and pipewire
# headers, and the vendored tree already pins the crates.
say "building niri ${NIRI_VERSION} from the vendored dependency set"
cd "${SRC}"
export CARGO_NET_OFFLINE=true
# The release bundle vendors crates, so cargo must not reach the network: an
# offline build is what makes the artefact reproducible.
cargo build --release --offline --locked \
    2>&1 | tail -20 || die "the niri build failed; see the output above"

BIN="${SRC}/target/release/niri"
[ -x "${BIN}" ] || die "cargo reported success but ${BIN} is not there"
say "built: $(du -h "${BIN}" | cut -f1)"

# ------------------------------------------------------------------ 4. install

say "installing into the base"
install -D -m 0755 "${BIN}" "${BASE_DIR}/usr/bin/niri"

# The compositor needs its session and desktop files, or the session picker and
# any desktop environment integration will not find it. Without these the image
# has a binary that nothing can start.
install -D -m 0644 "${SRC}/resources/niri-session" "${BASE_DIR}/usr/bin/niri-session" 2>/dev/null \
    || say "  (no niri-session resource in this release)"
install -D -m 0644 "${SRC}/resources/niri.desktop" "${BASE_DIR}/usr/share/wayland-sessions/niri.desktop" 2>/dev/null \
    || say "  (no niri.desktop resource in this release)"
install -D -m 0644 "${SRC}/resources/niri-portals.conf" "${BASE_DIR}/usr/share/xdg-desktop-portal/niri-portals.conf" 2>/dev/null \
    || say "  (no niri-portals.conf resource in this release)"

# The systemd user unit, so a session started through logind finds a compositor.
UNIT=$(ls "${SRC}"/resources/niri.service 2>/dev/null | head -1 || true)
if [ -n "${UNIT}" ]; then
    install -D -m 0644 "${UNIT}" "${BASE_DIR}/usr/lib/systemd/user/niri.service"
fi

# Where it came from, recorded in the base so the image can state its provenance.
cat > "${BASE_DIR}/etc/hcs-niri-provenance" << EOF
component=niri
version=${NIRI_VERSION}
source=${NIRI_URL}
sha256=${NIRI_SHA256}
note=Built from niri's own hash-pinned vendored release bundle. niri publishes no
note=prebuilt binary in any distribution or on its own releases page, so this is
note=the upstream-sanctioned offline-reproducible artefact rather than a third
note=party package.
EOF

say "version in the base:"
chroot "${BASE_DIR}" /usr/bin/niri --version 2>/dev/null | head -2 || \
    "${BASE_DIR}/usr/bin/niri" --version 2>/dev/null | head -2 || \
    say "  (could not execute niri --version inside the base)"

say "done"
