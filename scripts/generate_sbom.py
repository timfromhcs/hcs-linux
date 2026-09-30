#!/usr/bin/env python3
"""
HCS Linux - Release Artifact and SBOM Generator
Produces SPDX 2.3 SBOM, THIRD-PARTY-NOTICES.txt, and build/model manifests.
Adheres to GEMINI.md Sections 70, 71, 109, and 180.
"""

import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path
import yaml

def main():
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8")

    repo_root = Path(__file__).resolve().parent.parent
    dist_dir = repo_root / "dist"
    dist_dir.mkdir(parents=True, exist_ok=True)

    sources_lock = repo_root / "vendor/locks/sources.lock.yaml"
    models_lock = repo_root / "vendor/locks/models.lock.yaml"
    packages_lock = repo_root / "vendor/locks/packages.lock.yaml"

    with open(sources_lock, "r", encoding="utf-8") as f:
        sources_data = yaml.safe_load(f)
    with open(models_lock, "r", encoding="utf-8") as f:
        models_data = yaml.safe_load(f)
    with open(packages_lock, "r", encoding="utf-8") as f:
        packages_data = yaml.safe_load(f)

    timestamp = datetime.now(timezone.utc).isoformat()

    # 1. Generate SPDX 2.3 SBOM
    packages = []
    # Add HCS first-party package
    packages.append({
        "SPDXID": "SPDXRef-Package-HCS-Linux",
        "name": "HCS Linux",
        "versionInfo": "1.0.1",
        "downloadLocation": "https://github.com/timfromhcs/hcs-linux",
        "licenseConcluded": "Apache-2.0",
        "licenseDeclared": "Apache-2.0",
        "copyrightText": "Copyright 2026 HCS Linux Contributors",
        "supplier": "Organization: HCS Linux Project"
    })

    for src in sources_data.get("sources", []):
        packages.append({
            "SPDXID": f"SPDXRef-Package-{src['name']}",
            "name": src["name"],
            "versionInfo": src.get("version", "pinned"),
            "downloadLocation": src.get("upstream_url", "NOASSERTION"),
            "licenseConcluded": src.get("license", "NOASSERTION"),
            "licenseDeclared": src.get("license", "NOASSERTION"),
            "checksums": [{
                "algorithm": "SHA256",
                "checksumValue": src.get("sha256", "")
            }]
        })

    for mod in models_data.get("models", []):
        packages.append({
            "SPDXID": f"SPDXRef-Model-{mod['id']}",
            "name": mod["id"],
            "versionInfo": mod.get("revision", "pinned"),
            "downloadLocation": f"https://huggingface.co/{mod['repo']}",
            "licenseConcluded": mod.get("license", "Apache-2.0"),
            "licenseDeclared": mod.get("license", "Apache-2.0"),
            "checksums": [{
                "algorithm": "SHA256",
                "checksumValue": mod.get("sha256", "")
            }]
        })

    sbom = {
        "spdxVersion": "SPDX-2.3",
        "dataLicense": "CC0-1.0",
        "SPDXID": "SPDXRef-DOCUMENT",
        "name": "HCS-Linux-1.0.1-SBOM",
            "documentNamespace": f"https://github.com/timfromhcs/hcs-linux/spdxdocs/1.0.1/{timestamp}",
        "creationInfo": {
            "creators": ["Tool: HCS SBOM Generator 1.0", "Organization: HCS Linux Project"],
            "created": timestamp
        },
        "packages": packages
    }

    sbom_path = dist_dir / "SBOM.spdx.json"
    with open(sbom_path, "w", encoding="utf-8") as f:
        json.dump(sbom, f, indent=2)
    print(f"[OK] Generated {sbom_path}")

    # 2. Generate MODEL-MANIFEST.json
    model_manifest = {
        "manifest_version": "1.0",
        "generated_at": timestamp,
        "models": models_data.get("models", [])
    }
    model_manifest_path = dist_dir / "MODEL-MANIFEST.json"
    with open(model_manifest_path, "w", encoding="utf-8") as f:
        json.dump(model_manifest, f, indent=2)
    print(f"[OK] Generated {model_manifest_path}")

    # 3. Generate BUILD-MANIFEST.json
    build_manifest = {
        "product": "HCS Linux",
        "version": "1.0.1",
        "architecture": "amd64",
        "base_distribution": "Debian 13 (Trixie)",
        "build_timestamp": timestamp,
        "target_ram_budget": "<= 6144 MB idle / <= 8192 MB peak",
        "source_locks_hash": sources_lock.name,
        "models_count": len(models_data.get("models", [])),
        "packages_count": len(packages_data.get("packages", []))
    }
    build_manifest_path = dist_dir / "BUILD-MANIFEST.json"
    with open(build_manifest_path, "w", encoding="utf-8") as f:
        json.dump(build_manifest, f, indent=2)
    print(f"[OK] Generated {build_manifest_path}")

    # 4. Generate THIRD-PARTY-NOTICES.txt
    notices_lines = [
        "HCS LINUX THIRD-PARTY NOTICES",
        "=============================",
        "This file lists open source components, upstream libraries, and AI model weights",
        "used in HCS Linux distributions, along with their respective licensing terms.",
        "",
        "--------------------------------------------------------------------------------",
        ""
    ]

    for src in sources_data.get("sources", []):
        notices_lines.append(f"Component: {src['name']}")
        notices_lines.append(f"Version:   {src.get('version')}")
        notices_lines.append(f"Upstream:  {src.get('upstream_url')}")
        notices_lines.append(f"License:   {src.get('license')}")
        notices_lines.append(f"Notes:     {src.get('notes', '')}")
        notices_lines.append("\n" + ("-" * 60) + "\n")

    for mod in models_data.get("models", []):
        notices_lines.append(f"Model ID:  {mod['id']}")
        notices_lines.append(f"Repository: https://huggingface.co/{mod['repo']}")
        notices_lines.append(f"Filename:   {mod['filename']}")
        notices_lines.append(f"License:    {mod['license']}")
        notices_lines.append(f"SHA-256:    {mod['sha256']}")
        notices_lines.append("\n" + ("-" * 60) + "\n")

    notices_path = dist_dir / "THIRD-PARTY-NOTICES.txt"
    with open(notices_path, "w", encoding="utf-8") as f:
        f.write("\n".join(notices_lines))
    print(f"[OK] Generated {notices_path}")

if __name__ == "__main__":
    main()
