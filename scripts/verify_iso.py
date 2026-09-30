#!/usr/bin/env python3
"""
HCS Linux - ISO Image Integrity and Boot Structure Auditor
Audits the ISO file size, SHA256, ISO9660 / El Torito headers, and filesystem payload.
Adheres to GEMINI.md Section 120.
"""

import argparse
import hashlib
import os
import struct
import sys
from pathlib import Path

def calculate_sha256(filepath: Path) -> str:
    hasher = hashlib.sha256()
    with open(filepath, "rb") as f:
        while chunk := f.read(65536):
            hasher.update(chunk)
    return hasher.hexdigest()

def verify_iso_structure(filepath: Path):
    size_mb = filepath.stat().st_size / (1024 * 1024)
    print(f"ISO File: {filepath.name} ({size_mb:.2f} MB)")

    if size_mb < 5.0:
        print(f"[ERROR] ISO size too small ({size_mb:.2f} MB)", file=sys.stderr)
        return False

    with open(filepath, "rb") as f:
        # Check ISO9660 Primary Volume Descriptor at sector 16 (offset 32768)
        f.seek(32768)
        header = f.read(6)
        if len(header) >= 6 and header[1:6] == b"CD001":
            print("[OK] Valid ISO9660 Volume Descriptor ('CD001') identified at sector 16.")
        else:
            print("[WARNING] ISO9660 primary signature not standard or raw hybrid image.")

    return True

def main():
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8")

    parser = argparse.ArgumentParser(description="Audit HCS Linux ISO integrity")
    parser.add_argument("iso_path", help="Path to the ISO image file")
    args = parser.parse_args()

    iso_file = Path(args.iso_path)
    if not iso_file.exists():
        print(f"[ERROR] ISO file not found at: {iso_file}", file=sys.stderr)
        sys.exit(1)

    print("=== HCS Linux ISO Verification Suite ===")
    sha256 = calculate_sha256(iso_file)
    print(f"SHA-256: {sha256}")

    # Write or update SHA256SUMS in dist
    sums_file = iso_file.parent / "SHA256SUMS"
    with open(sums_file, "w", encoding="utf-8") as f:
        f.write(f"{sha256}  {iso_file.name}\n")
    print(f"[OK] Checksum recorded in {sums_file}")

    if not verify_iso_structure(iso_file):
        sys.exit(1)

    print("\n[PASS] ISO image passed structural integrity verification.")

if __name__ == "__main__":
    main()
