# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] - 2.0.0

The desktop release. Full plan: `docs/V2_STABLE_RELEASE_MASTER_PLAN.md`.
Nothing here is released until every gate in `make gate-all` passes.

### The system boots into a desktop

- `niri`, `quickshell`, `seatd`, `pipewire`, `xdg-desktop-portal` and Plymouth are
  now in the core package list. v1 shipped only the Wayland *client* libraries,
  so no compositor could ever start and the live ISO could only print a console
  banner.
- `config/hooks/live/01-hcs-setup.hook.chroot` installs the shell, theme, session
  and documentation trees and **fails** if any of them is absent.
- `scripts/verify_payload.sh` (Gate 4) checks 112 payload invariants, including
  that every binary is a real ELF executable. This is what stops a Windows
  cross-build or a shell script from being staged as if it were the product.

### Fixed — the claims that were not true

- **The native `hcs-docs` GUI now reaches the ISO.** `build_iso.sh` copied the
  Rust binaries and then overwrote `hcs-docs` with a 298-byte shell script. The
  browser portal is now `hcs-docs-html`.
- **The payload no longer loses files silently.** `cp … || true` swallowed a
  missing target directory, so launchers, icons, wallpapers and manuals were
  dropped while the build reported success. Missing payload now fails the build.
- **Gate 1 is green.** `cargo fmt` and `cargo clippy --workspace --all-targets
  -- -D warnings` both pass. The 1.1.0 release notes claimed "clippy 0 warnings"
  while two `redundant closure` errors were live.
- **`auto/config` no longer declares 0.1.0-alpha.1.** The ISO's own metadata
  contradicted its boot banner.
- The `xcs_search` and settings entry points no longer rely on a
  `pub` function that was never reachable.

### The HCS key, and layouts

- The Windows-key position is the **HCS key**, and every binding, cheatsheet
  entry and manual is written as `HCS+…`. Users arriving from Windows get
  identical muscle memory; the docs never name a foreign key.
- QWERTZ is the default layout; `HCS+Space` cycles QWERTZ / EN-US / FR / ES /
  IT / GB. The HCS key is a modifier, so **no layout can move it**.
- Interface language (de/en/fr/es/it) is a separate choice from the keyboard
  layout, because a user may want a German UI on a French keyboard.
- `hcs settings set-keyboard us`, `hcs settings locale fr`.

### New programs

- **`hcs-fm`** — file manager: tabs, search with visible filter pills, date
  narrowing (`YYYY-MM-DD`), preview pane, explicit `Open With`, and the
  `HCS+.` hand-off to a terminal.
- **`hcs-term`** — terminal: `vte`-style grid over ANSI colour, and an `Alt+,`
  profile picker listing the AI profiles (Ptyxis pattern).
- **`hcs-shot`** — screenshot: region/window/screen, delay, OCR extraction,
  colour picker, screen record. Refuses rather than writing a black PNG.
- **`hcs-notes`** — Markdown notes with offline search and session restore, so a
  crash does not lose a draft.
- **`hcs-actions`** — the action registry. One table, three front ends: the
  omnibar, the CLI and an agent. Privileged actions require confirmation;
  colliding quick keys are reported as ambiguous rather than guessed.
- **`hcs-update`** — level-based updates (1 security … 5 experimental), where
  level 5 can *never* be applied automatically, plus snapshot planning and a
  rollback that refuses to discard a newer state without `--force`.
- **`hcs-persist`** — amnesic sessions and opt-in LUKS persistent storage with
  the `active`/`enabled`/`masked` state machine.
- **`hcs-recall`** — a local, encrypted timeline. Credential and payment-card
  patterns are filtered *before* a snapshot is written, and the feature refuses
  to exist in an amnesic session.
- **`hcs-a11y-check`** — the focus-order, WCAG-contrast, motion and text-scaling
  gate that `docs/GUI_BUILD_PLAN.md` §4 step 7 promised and v1 never built.
- **`hcs-qa-agent`** — the deterministic guest-side QA driver (see Testing).

### Window management

Snap Layouts with Snap Groups, virtual desktops, Task View, Alt+Tab with live
preview, and a Stage Manager that reduces the rest of the windows to a strip.
Dual-mode by design: **floating** (Windows-style, the default, because that is
what users arrive expecting) and **tiling**, switched with `HCS+Alt+T`.

### Theme: one file drives everything

`/usr/share/hcs/theme/colors.toml` is the single source of truth. A preset
declares its own wallpaper and a `glass_min_opacity` floor, so a translucent
control can never ship unreadable — the reason macOS needed four Liquid Glass
revisions before its Control Center was legible again. A fourth preset,
**High Contrast**, is a first-class accessibility theme validated against WCAG AAA
by Gate 10, and it is rendered and regression-tested like the other three.

### Retrieval that cites its sources

- `hcs rag query` retrieves over heading-delimited chunks of the six manuals
  with IDF weighting, a heading boost and stopword removal. **Every answer cites
  the manual section it came from, and an out-of-scope question is refused** —
  a guess about a security feature is worse than no answer.
- `hcs ask --selection` explains whatever is on screen (Visual Intelligence
  pattern).
- `tests/unit/test_rag_quality.py` gates retrieval on recall@5, top-1 accuracy,
  citation presence, citation resolution, refusal and latency.

### The agent control surface

`hcs` grew `theme`, `window`, `desktop`, `ask`, `rag`, `privacy`, `recall`,
`actions` and `settings`, and every subcommand answers `--json`. This is
deliberate: a desktop reachable only by clicking is a desktop an agent cannot
help with.

### Testing

- **Thirteen gates** in `make gate-all`, four of them new: the payload contract,
  the accessibility gate, the retrieval gate and the manual release checklist.
- `scripts/qa_virtualbox_v2.ps1` runs **32 VM stages** against an in-guest QA
  driver. v1 raced a timer against boot and "fixed" black frames with a 40-second
  delay; the v2 agent waits on real signals and journals every step, so a failure
  names the step that failed.
- The blank-frame check gained a **text-pixel ratio** gate, because entropy alone
  scored three real rendering bugs as PASS.
- `hcs-gui-shots` covers all four themes; `config/gui_apps.json` declares the
  apps the RAM gate measures, and the audit now measures eight real processes
  rather than one.

### Models

- Only models whose licence permits redistribution of the **weights** are baked
  into the ISO, decided by `scripts/stage_starter_models.py` against
  `config/models/registry.yaml` so the rule has one implementation. Everything
  else downloads on demand and is hash-verified before `hcs-modeld` loads it.

### Documentation

- `docs/THREAT_MODEL.md` — a release gate. What is defended, against whom, and
  explicitly what is not, including the honest answer that **secure deletion is
  not offered** because overwriting is not reliable on flash storage.
- All six manuals rewritten with real content; retrieval quality depends on it.
- `docs/V2_STABLE_RELEASE_MASTER_PLAN.md` — the plan this release follows.

## [1.1.0] - 2026-09-30

The GUI release: every CLI app keeps its terminal interface and gains a real
Neural Glass window. Full plan: `docs/GUI_BUILD_PLAN.md`.

### Added
- `src/hcs-ui` — GUI foundation: single `.slint` kit (palette type, three theme
  presets, twelve shared widgets), a headless software-renderer harness, and the
  native Docs viewer and Settings app.
- `hcs-chat --gui`: chat plus an **Image Studio** tab (txt2img/img2img controls
  clamped to the SD 1.5 LCM limits, RAM gate and Single-Heavy-Model warning).
- `hcs-monitor --gui`: RAM bars against the idle/peak budget, model residency,
  daemon states. Values come from the same `Telemetry` model as `--json`.
- `hcs-control --gui`: Tor shield toggle reusing `TorTransparentProxy` rules,
  AI profile selection (LOWRAM-4GB / EDGE-8GB / WORKSTATION-16GB), LUKS2 vault.
- `hcs-search --gui`: Spotlight omnibar with ranked app/file/memory hits.
- `hcs-diagnose --gui`: triage cards, log tail, verify-and-apply.
- `hcs-docs`: **native** offline markdown viewer (pulldown-cmark) — no webview,
  no browser, works with the network disconnected.
- `hcs-settings`: theme, wallpaper, privacy defaults, shortcuts, and the
  mandatory Slint license disclosure.
- `scripts/verify_gui.py` (headless render audit + visual regression) and
  `scripts/gui_ram_audit.py` (≤ 250 MB per GUI app).
- CI jobs `gui` (render + regression + RAM, all three themes) and `gui-wayland`
  (headless Weston), and `.desktop` launchers for all GUI apps.

### Changed
- Slint 1.18.1 is now a pinned source, used under the **Slint Royalty-free
  License 2.0** (`LicenseRef-Slint-Royalty-free-2.0`) rather than GPLv3, so the
  project stays Apache-2.0. The required disclosure is rendered in
  Settings → About Slint and in `hcs_ui::disclosure()`.

### Fixed
- Headless harness handed every component the *same* window adapter, so only the
  last view rendered and all others produced black PNGs. Each component now gets
  its own adapter.
- Slint globals do not initialise across file imports, leaving `root.palette`
  default-constructed (transparent colours → invisible window). Rust now assigns
  the palette explicitly.
- Chat window drew both tabs stacked; each tab is now visibility-toggled.
- Long card values overflowed their cards; `GlassCard` now wraps text.

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
