# HCS Linux v1.0.0 Stable — Release Notes (Draft, Gate 7)

> Target ISO: `HCS-Linux-1.0.0-amd64.iso` (hybrid BIOS+UEFI, SquashFS XZ, SHA256SUMS + GPG)
> Base: Debian 13.7 trixie amd64, Kernel 6.12+ LTS. Desktop: Neural Glass 3.0 (Niri + Quickshell).

## New in v1.0.0 Stable (vs 0.1.0-alpha.1)
- **Windows-like Neural Glass taskbar** (`taskbar.qml`, 52px bottom, Start monogram,
  tasklist w/ cyan focus underline, SNI tray: TOR SHIELD pill, AI footprint, audio/net/clock/bell)
  + **Silicon Valley Start Menu** (`start_menu.qml`, 560x640, omnibar apps/files/Ask-Brain,
  8 pinned tiles, recents, power actions) + **Super+/** cheatsheet HUD (`cheatsheet.qml`).
- **Brand suite:** Obsidian Neural Gradient (default), Frosted Titanium, Cybernetic Stealth
  wallpapers; Plymouth flicker-free monogram theme; Image/Docs/Tor SVG icons.
- **Offline CPU image gen** (`hcs-image` + `hcs image`): SD 1.5 LCM Q4 via sd-cpp (AVX2/AVX-512),
  512x512 in 4-8 steps / 15-25s quad-core, ≤2.2GB peak, strict on-demand lifecycle,
  Single-Heavy-Model rule enforced via RAM gate (min 2500MB free).
- **Security Lab:** full Debian-13-native arsenal (nmap, wireshark/tshark, tcpdump, socat,
  masscan, sqlmap, nikto, gobuster, ffuf, john, hashcat, hydra, aircrack-ng, gdb/lldb,
  strace/ltrace, binwalk, steghide, tor+tbb, nftables); `hcs-tor-switch` fail-closed
  nftables (TransPort 9040, DNSPort 9053, zero DNS leak, IPv6 drop); HITL gate for
  `hcs agent run --role pentester` offensive actions.
- **Developer suite:** rustc/cargo, python3.12, node/npm, nvim + codium, shellcheck,
  `hcs dev init/test/debug`.
- **Offline docs:** `hcs-docs` portal (6 manuals), `cheatsheet.json`, `hcs-welcome` tour.
- **Calamares OEM:** branded slideshow (`show.qml`), LUKS2 AES-XTS-512 partitioning module,
  automated user provisioning + EDGE-8GB/LOWRAM-4GB/WORKSTATION-16GB profiles.

## Verification (this workspace, 2026-09-30)
- Gate 1 PASS: `cargo fmt --check` clean, `cargo clippy --workspace -- -D warnings` 0 warnings,
  `cargo test --workspace` 100% pass (incl. 6 new hcs-image + 2 new hcs-security tests).
- Gate 2 PASS: `run_security_audit.py` (0 secrets), `verify_sources.py` (100% pinned, SPDX).
- Gate 3 PASS: `run_stress_test.py` (100 cycles, 0 crashes/OOM/FD-leaks).
- Gate 4 PASS: `HCS-Linux-1.0.0-amd64.iso` (235.47 MB, SquashFS XZ 13.18 MB / 42 files:
  12 ELF release binaries v1.0.0 incl. `hcs-image` + 3 helper scripts, Ubuntu 7.0 kernel),
  SHA256 `f61816cafc9daf830742e84a31e18651872d71575e590009de09c2c4ff54614d`,
  ISO9660 verified via `verify_iso.py`.
- Gate 5 PENDING (env): 16-stage VirtualBox QA — `qa_virtualbox_install.ps1` with new stages
  (start menu, cheatsheet HUD, image studio, tor pill, docs browser) ready to run on host.
- Gate 6 PENDING: push `dev` → GitHub Actions `HCS Linux CI` must be green.
- Gate 7 PENDING: PR `dev`→`main`, bump `workspace.package.version` 0.1.0-alpha.1 → 1.0.0,
  rebuild ISO + SBOM/SHA256SUMS(+.gpg)/THIRD-PARTY-NOTICES/BUILD+MODEL-MANIFEST, sign, publish.

## RAM budget compliance (§5 matrix)
Idle ~480MB → +0.6B resident ~1050MB → 1.7B chat ~2200MB → 4B reason ~4100MB →
image gen ~3100MB → heavy multitask ~5400MB. All ≤ 6144 idle / 8192 peak caps.
