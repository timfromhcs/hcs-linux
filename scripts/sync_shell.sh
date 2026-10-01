#!/usr/bin/env bash
# sync_shell.sh — copy the shell sources into the ISO payload tree.
#
# The Neural Glass shell lives in `src/hcs-shell/` (so it is developed and
# version-controlled next to the crates that drive it) and is staged into
# `config/includes.chroot/usr/share/hcs/shell/` for the image.
#
# v1 copied these by hand, which is how the payload contract could pass while the
# shell in the image was three releases out of date. This script makes the copy
# the only path, and `--check` turns a stale payload into a gate failure.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
SRC="${REPO_ROOT}/src/hcs-shell"
DST="${REPO_ROOT}/config/includes.chroot/usr/share/hcs/shell"

CHECK_ONLY=0
[ "${1:-}" = "--check" ] && CHECK_ONLY=1

if [ ! -d "${SRC}" ]; then
    echo "[ERROR] shell sources missing at ${SRC}" >&2
    exit 1
fi

mkdir -p "${DST}"

STATUS=0
for f in "${SRC}"/*.qml "${SRC}"/*.kdl; do
    [ -f "$f" ] || continue
    name="$(basename "$f")"
    if cmp -s "$f" "${DST}/${name}" 2>/dev/null; then
        continue
    fi
    if [ "${CHECK_ONLY}" -eq 1 ]; then
        echo "  [STALE] ${name}"
        STATUS=1
    else
        cp -f "$f" "${DST}/${name}"
        echo "  [SYNC] ${name}"
    fi
done

# The reverse direction matters too: a component deleted from src/ must not
# linger in the payload, or the image ships a file nothing references any more.
for f in "${DST}"/*.qml "${DST}"/*.kdl; do
    [ -f "$f" ] || continue
    name="$(basename "$f")"
    if [ ! -f "${SRC}/${name}" ]; then
        if [ "${CHECK_ONLY}" -eq 1 ]; then
            echo "  [ORPHAN] ${name} exists only in the payload tree"
            STATUS=1
        else
            rm -f "$f"
            echo "  [REMOVE] ${name}"
        fi
    fi
done

if [ "${CHECK_ONLY}" -eq 1 ]; then
    if [ "${STATUS}" -ne 0 ]; then
        echo "[FAIL] the staged shell does not match src/hcs-shell" >&2
        echo "       run scripts/sync_shell.sh to update the payload" >&2
        exit 1
    fi
    echo "[OK] the staged shell matches src/hcs-shell"
fi

exit 0
