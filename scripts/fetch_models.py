#!/usr/bin/env python3
"""
HCS Linux - Headless Model Fetcher and Checksum Auditor
Audits and downloads GGUF models adhering to vendor/locks/models.lock.yaml and config/models/registry.yaml.
"""

import argparse
import hashlib
import json
import os
import sys
from pathlib import Path
import yaml

def calculate_sha256(filepath: Path) -> str:
    hasher = hashlib.sha256()
    with open(filepath, "rb") as f:
        while chunk := f.read(65536):
            hasher.update(chunk)
    return hasher.hexdigest()

def main():
    parser = argparse.ArgumentParser(description="Fetch or verify HCS GGUF models")
    parser.add_argument("--verify-only", action="store_true", help="Only verify existing models against lockfile")
    parser.add_argument("--models-dir", default="vendor/models", help="Target models directory")
    parser.add_argument("--lockfile", default="vendor/locks/models.lock.yaml", help="Path to models lockfile")
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parent.parent
    lock_path = repo_root / args.lockfile
    models_dir = repo_root / args.models_dir
    models_dir.mkdir(parents=True, exist_ok=True)

    if not lock_path.exists():
        print(f"Error: Lockfile not found at {lock_path}", file=sys.stderr)
        sys.exit(1)

    with open(lock_path, "r", encoding="utf-8") as f:
        lock_data = yaml.safe_load(f)

    models = lock_data.get("models", [])
    print(f"Loaded {len(models)} model specifications from {lock_path}")

    all_verified = True
    for model in models:
        model_id = model["id"]
        filename = model["filename"]
        expected_sha = model["sha256"]
        target_file = models_dir / filename

        print(f"\nChecking [{model_id}]: {filename}")
        if target_file.exists():
            actual_sha = calculate_sha256(target_file)
            if actual_sha.lower() == expected_sha.lower():
                print(f"  [OK] Checksum verified: {actual_sha[:16]}...")
            else:
                print(f"  [ERROR] Checksum mismatch! Expected {expected_sha}, got {actual_sha}", file=sys.stderr)
                all_verified = False
        else:
            if args.verify_only:
                print(f"  [INFO] Model file not yet fetched locally (Lock definition valid): {filename}")
            else:
                print(f"  [DOWNLOAD] Fetching {model['repo']} / {filename}...")
                try:
                    from huggingface_hub import hf_hub_download
                    token = os.environ.get("HF_TOKEN")
                    hf_hub_download(
                        repo_id=model["repo"],
                        filename=filename,
                        revision=model.get("revision"),
                        local_dir=str(models_dir),
                        token=token
                    )
                    actual_sha = calculate_sha256(target_file)
                    if actual_sha.lower() == expected_sha.lower():
                        print(f"  [OK] Downloaded and verified: {actual_sha[:16]}...")
                    else:
                        print(f"  [ERROR] Checksum mismatch on downloaded file!", file=sys.stderr)
                        all_verified = False
                except Exception as e:
                    print(f"  [ERROR] Fetch failed: {e}", file=sys.stderr)
                    all_verified = False

    if not all_verified:
        sys.exit(1)
    print("\nModel registry validation complete.")

if __name__ == "__main__":
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8")
    main()
