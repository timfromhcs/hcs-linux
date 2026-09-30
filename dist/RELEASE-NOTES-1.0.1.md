# HCS Linux v1.0.1 Stable — Release Notes

> Target ISO: `HCS-Linux-1.0.1-amd64.iso` (hybrid BIOS+UEFI, SquashFS XZ, SHA256SUMS)
> Base: Debian 13.7 trixie amd64, Kernel 6.12+ LTS. Desktop: Neural Glass 3.0 (Niri + Quickshell).

## Fixes vs v1.0.0 (from VirtualBox Gate-5 screenshot forensics)
- Guest boot banner and GRUB menu now render the release version (`HCS LINUX 1.0.1`,
  `HCS Linux 1.0.1 Live Desktop`) — `build_iso.sh` heredocs unquoted so `${VERSION}`
  expands host-side (previously printed a literal `${VERSION}` / empty entry).
- Installed-system VDI re-provisioned from the current rootfs; `provision_installed_vdi.sh`
  accepts a version argument for its GRUB entry (was hardcoded `0.1.0-alpha.1`).
- VirtualBox QA pipeline: desktop-baseline capture delay 14s → 40s (was shooting a black
  framebuffer mid-boot), stage names aligned to the 16 Master-Plan stages, both QA scripts
  default to the current stable ISO.
- README: current-stable section (features, download, verification evidence), 1.0.1
  install instructions, `hcs-image` subsystem row.

## Verification (this workspace, 2026-09-30)
- Gate 1 PASS: `cargo fmt --check` clean, `cargo clippy --workspace -- -D warnings` 0 warnings,
  `cargo test --workspace` 100% pass.
- Gate 2 PASS: `run_security_audit.py` (0 secrets), `verify_sources.py` (100% pinned, SPDX),
  `fetch_models.py --verify-only` clean, Python unit tests 3/3.
- Gate 3 PASS: `run_stress_test.py` (100 cycles, 0 crashes/OOM/FD-leaks).
- Gate 4 PASS: `HCS-Linux-1.0.1-amd64.iso` (235.47 MB, SquashFS XZ, 12 ELF v1.0.1 binaries
  incl. `hcs-image` + 3 helper scripts), SHA256
  `cbbd66fc92d8e821083112a9c7c0fc420026e7938bd855b6886f7bdead6c9c3e`,
  ISO9660 verified via `verify_iso.py`.
- Gate 5 PASS: VirtualBox 7.2.10, **16/16 stages verified** (GRUB 1.0.1 menu, live banner
  1.0.1, installed HDD-boot banner 1.0.1 — all screenshot-verified), report
  `qa/reports/install_qa_report.json`.
- Gate 6: push `dev` → GitHub Actions `HCS Linux CI` must be green.
- Gate 7: PR `dev`→`main` (this release), tag `v1.0.1`, GitHub Release with ISO,
  SHA256SUMS, SBOM.spdx.json, THIRD-PARTY-NOTICES.txt, BUILD/MODEL-MANIFEST.json.
  Open: SHA256SUMS.gpg (no maintainer signing key — not fabricated), PR merge.

## RAM budget compliance (§5 matrix)
Idle ~480MB → +0.6B resident ~1050MB → 1.7B chat ~2200MB → 4B reason ~4100MB →
image gen ~3100MB → heavy multitask ~5400MB. All ≤ 6144 idle / 8192 peak caps.
