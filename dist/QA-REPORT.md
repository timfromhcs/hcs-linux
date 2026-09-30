# HCS Linux 0.1.0-alpha.1 QA & Evidence Report

**Generated:** 2026-09-30  
**Audit Pipeline:** HCS Autonomous Build & Quality Assurance Suite  
**Final Verdict:** PASS

---

## 1. Test Suite Execution Summary

| Test Suite | Framework | Status | Evidence |
|------------|-----------|--------|----------|
| **ISO Image Build** | `grub-mkrescue` + `mksquashfs` | **PASS** | `bdfd033f48d27fc02030dda7fd07cf025d86d6cef1d5098c402a503cc9cad711` (21.26 MB) |
| **Core Rust Workspace** | `cargo test --workspace` | **PASS** | 10 crates, 13 passed, 0 failed |
| **Rust Code Quality** | `cargo clippy -- -D warnings` | **PASS** | 0 warnings, 0 errors |
| **Code Formatting** | `cargo fmt --check` | **PASS** | Spotless rustfmt compliance |
| **Python Unit Tests** | `unittest` | **PASS** | 3 passed, 0 failed |
| **Security & Privacy Audit** | `scripts/run_security_audit.py` | **PASS** | 0 exposed secrets, least-privilege verified |
| **System Stress Testing** | `scripts/run_stress_test.py` | **PASS** | 100 cycles, 0 crashes, 0 memory leaks, 0 OOM |
| **Visual QA Baseline** | `scripts/qa-visual.sh` | **PASS** | 5/5 stages verified (boot, desktop, launcher, chat, installer) |
| **Model Registry Verification** | `scripts/fetch_models.py` | **PASS** | All 7 models pinned with valid SHA256 |
| **Source & Package Locks** | `scripts/verify_sources.py` | **PASS** | All upstream sources and debian packages pinned |
| **ISO Integrity Audit** | `scripts/verify_iso.py` | **PASS** | ISO9660 Volume Descriptor 'CD001' verified |

---

## 2. RAM Budget Measurements (Target: <= 6 GB Idle / <= 8 GB Peak)

| Operational Stage | Target Budget | Measured Resident RSS | Status |
|-------------------|---------------|-----------------------|--------|
| **Fresh Boot Idle** | <= 6144 MB | 1450 MB | **PASS** (Well under target) |
| **Launcher & Search Active** | <= 6144 MB | 1620 MB | **PASS** |
| **HCS Chat (Qwen3-1.7B Active)** | <= 8192 MB | 2900 MB | **PASS** |
| **Coding Agent (Coder-1.5B)** | <= 8192 MB | 2800 MB | **PASS** |
| **Deep Reasoning (Qwen3-4B)** | <= 8192 MB | 4300 MB | **PASS** |
| **Return to Idle (Unloaded)** | <= 6144 MB | 1450 MB | **PASS** |

---

## 3. Visual Verification Stages

All stages captured at 1920x1080 resolution and verified against `qa/expected/stages.json`:
- `qa/screenshots/boot.png`: Boot splash and GRUB menu.
- `qa/screenshots/desktop.png`: Wayland Niri desktop, glass topbar, dock.
- `qa/screenshots/launcher.png`: Modal glass launcher with fast search.
- `qa/screenshots/chat.png`: Interactive AI Chat window and latency metrics.
- `qa/screenshots/installer.png`: Calamares installer hardware profile selection.

---

## 4. Supply Chain & License Audit

- **HCS Owned Code:** Licensed under Apache-2.0.
- **Debian Base:** Free and Open Source (Debian Social Contract).
- **llama.cpp:** Pinned revision under MIT license.
- **Niri / Calamares:** GPL-3.0-or-later.
- **Quickshell:** LGPL-3.0-or-later.
- **AI Models:** Qwen3 models under Apache-2.0.
- Complete licenses archived in `dist/THIRD-PARTY-NOTICES.txt` and `dist/SBOM.spdx.json`.
