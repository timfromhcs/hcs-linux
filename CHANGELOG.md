# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.1] - 2026-09-30

### Fixed
- ISO guest banner and GRUB menu entries now render the release version: `build_iso.sh`
  heredocs unquoted so `${VERSION}` expands host-side (`HCS LINUX ${VERSION}` literal fixed).
- Installed-system VDI re-provisioned from current rootfs; `provision_installed_vdi.sh`
  takes a version argument for its GRUB entry (was hardcoded `0.1.0-alpha.1`).
- VirtualBox QA: desktop-baseline capture delay 14s → 40s (was shooting a black
  framebuffer mid-boot); QA stage names aligned to the 16 plan stages; default ISO
  paths of both VirtualBox QA scripts point at the current stable ISO.

## [1.0.0] - 2026-09-30

### Added
- Windows-like Neural Glass bottom taskbar (`taskbar.qml`), Silicon Valley Start Menu
  (`start_menu.qml`) and Super+/ cheatsheet HUD (`cheatsheet.qml`).
- Wallpaper suite (Obsidian Neural Gradient, Frosted Titanium, Cybernetic Stealth),
  Plymouth flicker-free monogram theme, Image/Docs/Tor SVG icons.
- Offline CPU image generation crate `hcs-image` (SD 1.5 LCM Q4, txt2img/img2img,
  strict on-demand RAM gate) plus `hcs image` and `hcs dev` CLI workflows.
- Full Debian-13-native security arsenal, fail-closed nftables Tor transparent proxy
  (`hcs-tor-switch`, TransPort 9040 / DNSPort 9053, zero DNS leaks) and HITL-gated
  `hcs agent run --role pentester`.
- Offline `hcs-docs` portal (6 manuals), `cheatsheet.json`, `hcs-welcome` onboarding tour.
- Calamares OEM branding slideshow, LUKS2 (AES-XTS-512) partition module, HW AI-profile
  provisioning (`EDGE-8GB` / `LOWRAM-4GB` / `WORKSTATION-16GB`).
- Production ISO `HCS-Linux-1.0.0-amd64.iso` with SPDX SBOM, SHA256SUMS and manifests.

## [0.1.0-alpha.1] - 2026-09-30

### Added
- Canonical repository architecture and bootstrap configuration.
- Debian 13 (Trixie) live-build configuration and automated hybrid ISO generation pipeline.
- Core daemon `hcsd` implementing Unix domain socket API and HCS Brain orchestration.
- Model daemon `hcs-modeld` with on-demand GGUF loading, explicit unloading, and RSS tracking.
- Memory management engine `hcs-memory` backed by SQLite FTS5 lexical search and hybrid retrieval.
- Subagent execution framework `hcs-agents` with least-privilege capability permissions.
- Modern Wayland shell `hcs-shell` design specifications for Niri and Quickshell.
- Deterministic evaluation, self-scoring judge protocol, and experience ledger.
- Hardware profiling and Edge model catalog with SHA-256 verification manifests.
- Calamares installer branding and offline documentation suite.
