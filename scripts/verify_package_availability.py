#!/usr/bin/env python3
"""Check that every package in the image's package list actually exists.

WHY THIS EXISTS

`config/package-lists/hcs-core.list.chroot` is the declaration of what the live
image is supposed to contain. For v1 and v2 nothing ever read it: `build_iso.sh`
staged a directory of HCS binaries and assets, copied the *build host's* kernel
and initrd, and called the result an ISO. No Debian base was ever assembled, so
the list described a system that did not and could not exist.

The CI job added for v2 made this worse rather than better. It grepped the list
for the strings `niri` and `quickshell` and reported success — a check that the
word appears in a text file, presented as "a real graphical session can be
assembled from the package list".

This script asks the distribution instead: does a package with this name exist
in the target suite? Run against Debian 13 trixie it finds that neither `niri`
nor `quickshell` is packaged in Debian at all, nor in Ubuntu. So the list cannot
be installed as written, and the desktop the plan describes cannot be assembled
from it.

Usage:
  python3 scripts/verify_package_availability.py
  python3 scripts/verify_package_availability.py --base debian:13
  python3 scripts/verify_package_availability.py --packages-file <list>
  python3 scripts/verify_package_availability.py --index Packages-main.txt  # offline

Exit code 0 = every package exists. Non-zero = the list cannot be installed.
"""

from __future__ import annotations

import argparse
import gzip
import io
import re
import sys
import urllib.request
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DEFAULT_LIST = REPO / "config/package-lists/hcs-core.list.chroot"

# Where the binary-amd64 package index lives, per base.
INDEX_URLS = {
    "debian:13": "http://deb.debian.org/debian/dists/trixie/main/binary-amd64/Packages.gz",
    "debian:12": "http://deb.debian.org/debian/dists/bookworm/main/binary-amd64/Packages.gz",
}

PKG_RE = re.compile(r"^Package:\s+(\S+)\s*$")
PROVIDES_RE = re.compile(r"^Provides:\s+(.+)$")


def read_list(path: Path) -> list[str]:
    """Package names from a Debian-style list file: comments and blanks out."""
    out = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.split("#", 1)[0].strip()
        if not line:
            continue
        out.append(line)
    return out


def _names(text: str) -> tuple[set[str], set[str]]:
    """(real package names, names satisfied via Provides).

    A virtual package has no index entry of its own — `qt6-declarative` is
    provided by the qml6-module-* packages, for instance. Treating that as
    "missing" would be a false accusation, so Provides is honoured.
    """
    real: set[str] = set()
    provided: set[str] = set()
    for line in text.splitlines():
        m = PKG_RE.match(line)
        if m:
            real.add(m.group(1))
            continue
        m = PROVIDES_RE.match(line)
        if m:
            for alt in m.group(1).split(","):
                name = alt.split("(")[0].strip()
                if name:
                    provided.add(name)
    return real, provided


def load_index(url: str, timeout: int = 180) -> tuple[set[str], set[str]]:
    with urllib.request.urlopen(url, timeout=timeout) as r:
        raw = r.read()
    try:
        text = gzip.decompress(raw).decode("utf-8", "replace")
    except OSError:
        text = raw.decode("utf-8", "replace")
    return _names(text)


def load_index_file(path: Path) -> tuple[set[str], set[str]]:
    raw = path.read_bytes()
    if raw[:2] == b"\x1f\x8b":
        raw = gzip.decompress(raw)
    return _names(raw.decode("utf-8", "replace"))


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[1])
    ap.add_argument("--base", default="debian:13", choices=sorted(INDEX_URLS))
    ap.add_argument("--packages-file", default=str(DEFAULT_LIST))
    ap.add_argument("--index", help="use a local Packages[.gz] instead of fetching")
    args = ap.parse_args(argv[1:])

    wanted = read_list(Path(args.packages_file))
    print(f"=== package availability ({args.base}) ===")
    print(f"  list:    {Path(args.packages_file).name} ({len(wanted)} package(s))")

    try:
        if args.index:
            available, provided = load_index_file(Path(args.index))
            source = str(args.index)
        else:
            available, provided = load_index(INDEX_URLS[args.base])
            source = INDEX_URLS[args.base]
    except Exception as exc:  # noqa: BLE001
        print(f"[ERROR] could not read the package index: {exc}", file=sys.stderr)
        return 2
    print(f"  index:   {source}")
    print(f"  indexed: {len(available)} package(s), {len(provided)} provided name(s)")
    print()

    def satisfied(name: str) -> bool:
        return name in available or name in provided

    missing = [p for p in wanted if not satisfied(p)]
    present = [p for p in wanted if satisfied(p)]

    for p in present:
        kind = "" if p in available else "  (virtual, provided)"
        print(f"  [OK]   {p}{kind}")
    for p in missing:
        print(f"  [MISS] {p} — not in {args.base}")

    print()
    if missing:
        print(f"[FAIL] {len(missing)} of {len(wanted)} declared packages do not exist "
              f"in {args.base}.")
        print("       The image cannot be assembled from this list. A list that names a")
        print("       package the distribution does not ship is not a plan, and a CI")
        print("       check that greps the list for a word cannot tell the difference.")
        print()
        print("       This is a base-system decision, not a typo:")
        print("         * build the missing pieces from source into the image, or")
        print("         * choose a compositor and shell the base distribution ships, or")
        print("         * adopt a third-party repository, which is a supply-chain")
        print("           decision that has to be made deliberately and recorded.")
        return 1

    print(f"[OK] all {len(wanted)} declared packages exist in {args.base}.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
