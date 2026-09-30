# HCS Linux 0.1.0-alpha.1 Release Notes

**Release Date:** September 30, 2026  
**Codename:** Neural Glass  
**Target Profile:** EDGE-8GB (Baseline: <= 6 GB RAM, Peak: <= 8 GB RAM)

---

## Highlights

- **Local-First Cognitive Operating System:** Debian 13 (Trixie) base unified with Wayland (Niri compositor + Quickshell) and native local AI daemons.
- **Strict RAM Budget Enforcement:** Resident `Qwen3-0.6B` Controller with on-demand `Qwen3-1.7B` Assistant, `Qwen2.5-Coder-1.5B`, and `Qwen3-4B` Reasoner. Enforces single heavy resident model policy to guarantee peak RAM stays <= 8 GB.
- **SQLite Cognitive Memory Engine:** FTS5 lexical matching combined with semantic embedding retrieval, contradiction detection, and automatic memory consolidation.
- **Scoped Subagent Runtime:** Granular capability permissions (`filesystem.read`, `process.spawn`, etc.) with non-root enforcement and full experience event ledger capture.
- **Built-in Privacy & Vault:** Integrated Tor routing, NFTables network policy, and LUKS2-compatible AES-XTS-512 encrypted storage.
- **Branded Calamares Installer:** Full-disk and encrypted installation with hardware profile selection.

---

## Release Artifacts

| File | Description |
|------|-------------|
| `HCS-Linux-0.1.0-alpha.1-amd64.iso` | Bootable hybrid Live ISO for USB/VM |
| `SHA256SUMS` | SHA-256 verification hashes |
| `SBOM.spdx.json` | SPDX 2.3 Software Bill of Materials |
| `THIRD-PARTY-NOTICES.txt` | Complete third-party licenses and notices |
| `MODEL-MANIFEST.json` | Model catalog, quantization tiers, and hashes |
| `BUILD-MANIFEST.json` | Build environment and dependency lock states |
| `QA-REPORT.md` | Full automated test, stress, and visual QA audit |

---

## Known Limitations

1. **CPU Inference Latency on 4B Model:** Loading the on-demand `Qwen3-4B` reasoner on older quad-core CPUs may exhibit slower token generation compared to the standard 1.7B assistant.
2. **Tor Isolation Scope:** Transparent Tor routing is strictly bound to Private Mode workspaces and Tor Browser sessions; standard desktop mode connects via conventional default route with local DNS privacy.
3. **Experimental Hardware:** NVIDIA proprietary driver packaging requires manual installation via Developer Mode or supplementary driver packs.
