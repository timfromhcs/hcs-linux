#!/bin/bash
# HCS Linux — autonomous build-and-verify loop.
#
# ONE command to build the base, build the ISO, boot it, photograph it, grade
# the result, and report. No mocks, no simulated passes: the only thing this
# script will print PASS for is something it observed.
#
# Why this exists as a script and not as a habit:
#
# The autologin fix took three build cycles to find, and every cycle looked
# identical from the outside. The fix was correct, committed, pushed — and
# completely inert, because build_iso.sh was re-run but build_base.sh was not,
# and the hook that creates the live user and the getty override runs during the
# BASE build. The image was rebuilt faithfully from a base that had never been
# rebuilt. That is not a subtle bug; it is a missing dependency between two
# scripts that each claim to produce a working image, and anyone running them in
# the wrong order will do exactly what I did.
#
# So the dependency is encoded here rather than remembered. That is the whole
# value: not speed, but that the ORDER OF OPERATIONS is a property of the build
# system instead of a property of whoever is paying attention.
#
# Usage:
#   scripts/autonomous_loop.sh                 # full loop
#   scripts/autonomous_loop.sh --no-vm         # build and verify only
#   scripts/autonomous_loop.sh --base-only     # rebuild the base, then stop
set -uo pipefail

VERSION="${VERSION:-2.0.0}"
ARCH="${ARCH:-amd64}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_ROOT="${BUILD_ROOT:-${HOME}/hcs-build}"
LOG_DIR="${LOG_DIR:-${HOME}}"
RUN_VM=1
BASE_ONLY=0

for arg in "$@"; do
    case "${arg}" in
        --no-vm)     RUN_VM=0 ;;
        --base-only) BASE_ONLY=1 ;;
        --build-root=*) BUILD_ROOT="${arg#*=}" ;;
        -h|--help)   sed -n '2,22p' "$0"; exit 0 ;;
        *) echo "unknown argument: ${arg}" >&2; exit 2 ;;
    esac
done

BASE_LOG="${LOG_DIR}/hcs-loop-base.log"
ISO_LOG="${LOG_DIR}/hcs-loop-iso.log"
SHOT_DIR="${REPO_ROOT}/qa/screenshots/v2/loop"
DRAW=0
SHA="none"
CHECKS=0
ELAPSED=0
FROZEN=0
LAST=""

say()  { printf '\n==> %s\n' "$*"; }
ok()   { printf '    OK   %s\n' "$*"; }
bad()  { printf '    FAIL %s\n' "$*"; }
warn() { printf '    WARN %s\n' "$*"; }

# ------------------------------------------------------------- 1. sync tree

say "1/6  syncing the working tree into the build root"
# target/ and dist/ are excluded: they are build products, and copying a 3 GB
# base tree across the Windows filesystem boundary every iteration would
# dominate the runtime of the thing that is supposed to be fast.
rsync -a --delete --exclude 'target/' --exclude 'dist/' --exclude '.git/' \
    "${REPO_ROOT}/" "${BUILD_ROOT}/"
ok "synced to ${BUILD_ROOT}"

# ---------------------------------------------------------------- 2. base

say "2/6  building the base system"
say "     the live-user and getty hook runs HERE. Skipping this step is how the"
say "     autologin fix stayed inert for three cycles."
if sudo bash "${BUILD_ROOT}/scripts/build_base.sh" sid >"${BASE_LOG}" 2>&1; then
    ok "base: $(grep -oE 'base: [0-9]+ files, [0-9.]+G' "${BASE_LOG}" | tail -1)"
else
    bad "base build failed — see ${BASE_LOG}"
    tail -20 "${BASE_LOG}" >&2
    exit 1
fi

[ "${BASE_ONLY}" -eq 1 ] && { say "base-only: stopping"; exit 0; }

# ---------------------------------------------------------------- 3. ISO

say "3/6  building the ISO"
if sudo bash "${BUILD_ROOT}/scripts/build_iso.sh" "${VERSION}" "${ARCH}" --qa \
        >"${ISO_LOG}" 2>&1; then
    CHECKS=$(grep -oE 'checks run: [0-9]+' "${ISO_LOG}" | tail -1 | grep -oE '[0-9]+')
    SHA=$(grep -oE 'SHA-256: [0-9a-f]+' "${ISO_LOG}" | tail -1 | cut -d' ' -f2)
    ok "ISO built: ${CHECKS} payload checks, sha256 ${SHA}"
else
    bad "ISO build FAILED the payload contract — this is the gate working"
    grep -E '\[FAIL\]|\[ERROR\]|violation' "${ISO_LOG}" | head -20 >&2
    exit 1
fi

[ "${RUN_VM}" -eq 0 ] && { say "--no-vm: stopping after the build"; exit 0; }

# ----------------------------------------------------------- 4. boot it up

say "4/6  booting the ISO in VirtualBox"
VBOX="${VBOX:-/mnt/c/Program Files/Oracle/VirtualBox/VBoxManage.exe}"
[ -x "${VBOX}" ] || VBOX="$(command -v VBoxManage || true)"
VM_NAME="${VM_NAME:-HCS-Loop}"
mkdir -p "${SHOT_DIR}"

if [ -z "${VBOX}" ] || [ ! -x "${VBOX}" ]; then
    warn "VBoxManage not found; the boot leg cannot run here"
    say "6/6  summary: build PASS, boot NOT RUN"
    printf 'version=%s\niso_sha256=%s\npayload_checks=%s\ndesktop_drawn=0\nreason=vboxmanage-not-found\n' \
        "${VERSION}" "${SHA}" "${CHECKS}" >"${SHOT_DIR}/loop-report.txt"
    exit 0
fi

# A stale VM from a previous iteration looks exactly like a working one until
# you check the UUID, so clear it explicitly rather than trusting the name.
"${VBOX}" controlvm "${VM_NAME}" poweroff >/dev/null 2>&1 || true
sleep 5
"${VBOX}" unregistervm "${VM_NAME}" --delete >/dev/null 2>&1 || true

"${VBOX}" createvm --name "${VM_NAME}" --ostype Other_64 --register >/dev/null
"${VBOX}" modifyvm "${VM_NAME}" --memory 6144 --cpus 4 --vram 256 --firmware bios >/dev/null
"${VBOX}" modifyvm "${VM_NAME}" --graphicscontroller vmsvga >/dev/null
"${VBOX}" storagectl "${VM_NAME}" --name SATA --add sata --portcount 2 --hostiocache on >/dev/null
EVID="${SHOT_DIR}/evidence.vhd"
rm -f "${EVID}"
"${VBOX}" createmedium disk --filename "${EVID}" --size 512 --format VHD --variant Fixed >/dev/null
"${VBOX}" storageattach "${VM_NAME}" --storagectl SATA --port 0 --device 0 --type hdd \
    --medium "${EVID}" >/dev/null
"${VBOX}" storageattach "${VM_NAME}" --storagectl SATA --port 1 --device 0 --type dvddrive \
    --medium "${BUILD_ROOT}/dist/HCS-Linux-${VERSION}-qa-${ARCH}.iso" >/dev/null
"${VBOX}" modifyvm "${VM_NAME}" --bootorder dvd,hdd >/dev/null
"${VBOX}" startvm "${VM_NAME}" --type headless >/dev/null
ok "VM started: ${VM_NAME}"

# ------------------------------------------------------------ 5. watch it

say "5/6  observing the boot (10 minutes)"
BOOT_WINDOW=600
while [ "${ELAPSED}" -lt "${BOOT_WINDOW}" ]; do
    sleep 60
    ELAPSED=$((ELAPSED + 60))
    FRAME="${SHOT_DIR}/frame-$(printf '%03d' "${ELAPSED}").png"
    if ! "${VBOX}" controlvm "${VM_NAME}" screenshotpng "${FRAME}" >/dev/null 2>&1; then
        bad "screenshot failed at ${ELAPSED}s — the VM is gone"
        break
    fi
    if [ -n "${LAST}" ]; then
        A=$(sha256sum "${LAST}" | cut -d' ' -f1)
        B=$(sha256sum "${FRAME}" | cut -d' ' -f1)
        if [ "${A}" = "${B}" ]; then
            FROZEN=$((FROZEN + 1))
            warn "frame unchanged for $((FROZEN * 60))s"
        else
            [ "${FROZEN}" -gt 0 ] && ok "frame changed after $((FROZEN * 60))s frozen — booting, not hung"
            FROZEN=0
        fi
    fi
    LAST="${FRAME}"
done

# The frame gate is the only thing in this project that can say "something was
# actually drawn". A build log cannot, and a clean boot cannot.
if [ -n "${LAST}" ] && [ -f "${LAST}" ] && command -v python3 >/dev/null 2>&1; then
    if python3 "${REPO_ROOT}/scripts/grade_capture.py" "${LAST}" >"${SHOT_DIR}/capture-grade.txt" 2>&1; then
        ok "final frame passes the capture gate — something WAS drawn"
        DRAW=1
    else
        warn "final frame does NOT pass the capture gate — the desktop did not draw"
    fi
fi

# -------------------------------------------------------------- 6. verdict

say "6/6  verdict"
cat >"${SHOT_DIR}/loop-report.txt" <<EOF
version=${VERSION}
iso_sha256=${SHA}
payload_checks=${CHECKS}
frames_captured=$((ELAPSED / 60))
longest_freeze_s=$((FROZEN * 60))
final_frame=${LAST:-none}
desktop_drawn=${DRAW}
generated=$(date -u +%Y-%m-%dT%H:%M:%SZ)
EOF
cat "${SHOT_DIR}/loop-report.txt"

"${VBOX}" controlvm "${VM_NAME}" poweroff >/dev/null 2>&1 || true
sleep 5
"${VBOX}" unregistervm "${VM_NAME}" --delete >/dev/null 2>&1 || true

if [ "${DRAW}" -eq 1 ]; then
    ok "build verified AND a desktop was drawn"
    exit 0
fi
bad "build verified, desktop NOT drawn. Report above; do not claim otherwise."
exit 1
