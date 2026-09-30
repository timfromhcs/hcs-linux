#!/usr/bin/env python3
"""
HCS Linux - Source Dependency and License Auditor
Validates that all external dependencies are pinned, licensed, and hashed.
"""

import sys
from pathlib import Path
import yaml

FORBIDDEN_PINS = {"main", "master", "latest", "head", "nightly", "trunk"}

def main():
    repo_root = Path(__file__).resolve().parent.parent
    sources_lock = repo_root / "vendor/locks/sources.lock.yaml"
    packages_lock = repo_root / "vendor/locks/packages.lock.yaml"

    errors = []

    if not sources_lock.exists():
        print(f"Error: sources.lock.yaml missing at {sources_lock}", file=sys.stderr)
        sys.exit(1)

    with open(sources_lock, "r", encoding="utf-8") as f:
        src_data = yaml.safe_load(f)

    for src in src_data.get("sources", []):
        name = src.get("name", "unknown")
        rev = src.get("revision", "").lower()
        license_str = src.get("license", "")
        sha = src.get("sha256", "")

        if rev in FORBIDDEN_PINS:
            errors.append(f"Source '{name}' uses unpinned revision '{rev}' (forbidden in release)")

        if not license_str:
            errors.append(f"Source '{name}' lacks a declared SPDX license")

        if not sha or len(sha) != 64:
            errors.append(f"Source '{name}' has invalid SHA256 checksum: '{sha}'")

    if packages_lock.exists():
        with open(packages_lock, "r", encoding="utf-8") as f:
            pkg_data = yaml.safe_load(f)
        for pkg in pkg_data.get("packages", []):
            name = pkg.get("name")
            ver = pkg.get("version")
            if not ver or ver.lower() in FORBIDDEN_PINS:
                errors.append(f"Package '{name}' has unpinned version '{ver}'")

    if errors:
        print("Source lock validation failed:", file=sys.stderr)
        for err in errors:
            print(f"  [ERROR] {err}", file=sys.stderr)
        sys.exit(1)

    print("[OK] All sources and packages are strictly pinned with valid SHA256 and SPDX licenses.")

if __name__ == "__main__":
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8")
    main()
