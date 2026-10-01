# HCS Linux v1.1.0 Stable — Release Notes (The GUI Release)

> Target ISO: `HCS-Linux-1.1.0-amd64.iso` (281.69 MB, hybrid BIOS+UEFI, SquashFS XZ)
> Base: Debian 13.7 trixie amd64, Kernel 6.12+ LTS. Desktop: Neural Glass 3.0.
> Plan: `docs/GUI_BUILD_PLAN.md`

## What is new

Every CLI app keeps its terminal interface and now also has a real Neural Glass
window. The shell (Quickshell/QML) and the Calamares installer are unchanged.

| App | Flag | What the GUI adds |
|---|---|---|
| `hcs-chat` | `--gui` | Chat + **CPU Image Studio** tab: txt2img/img2img, steps/CFG/resolution (clamped to the SD 1.5 LCM limits), RAM gate and Single-Heavy-Model warning, seed/output, gallery. |
| `hcs-monitor` | `--gui` | RAM bars against the idle/peak budget, model residency, daemon states. Numbers come from the same `Telemetry` model as `--json`. |
| `hcs-control` | `--gui` | Tor shield toggle reusing the exact `TorTransparentProxy` nftables rules, AI profile selection, LUKS2 vault. |
| `hcs-search` | `--gui` | Spotlight omnibar, ranked app/file/memory hits, keyboard-first. |
| `hcs-diagnose` | `--gui` | Triage cards, log tail, verify-and-apply. |
| `hcs-docs` | (new binary) | **Native** offline markdown viewer (pulldown-cmark). No webview, no browser, works with the network disconnected. |
| `hcs-settings` | (new binary) | Theme, wallpaper, privacy defaults, shortcuts, and the Slint license disclosure. |

Three themes ship: **Obsidian** (default), **Frosted Titanium**, **Cybernetic
Stealth**. Theme choice persists to `$XDG_CONFIG_HOME/hcs/theme.json`.

## Licensing (important)

The GUIs are built with **Slint 1.18.1**, used under the **Slint Royalty-free
License 2.0** (`LicenseRef-Slint-Royalty-free-2.0`) rather than its GPLv3 option,
so HCS Linux remains Apache-2.0. That license requires disclosing the use of
Slint, which is done in Settings → *About Slint* and in `hcs_ui::disclosure()`.
`dist/THIRD-PARTY-NOTICES.txt` documents this.

## Verification

- **Gate 1 PASS** — `cargo fmt --check` clean, `cargo clippy --workspace -D warnings`
  0 warnings, `cargo test --workspace` green (45 test binaries).
- **Gate 2 PASS** — security audit 0 secrets, sources 100 % pinned, Slint added to
  `vendor/locks/sources.lock.yaml` with the license rationale.
- **Gate 3 PASS** — stress suite: 0 crashes / OOM / FD leaks.
- **Gate 4 PASS** — `HCS-Linux-1.1.0-amd64.iso`, SHA-256
  `3d4a19b319c594a095b02444fbffed577c5562d971bed7856b7af00ca39dc70b`, ISO9660 verified.
  Payload grew 235 MB → 282 MB (GUI binaries + Slint runtime).
- **Gate 5 PASS (18/18)** — VirtualBox 7.2.10, ISO + installed-VDI boot, banner
  reports `HCS LINUX 1.1.0`.
  *Stated plainly:* the live ISO boots to a **text console**, so VM stages 17-18
  prove the guest boots with the GUI toolchain staged — they are **not** GUI pixel
  evidence. Real GUI pixels are verified host-side: `verify_gui.py` renders
  **9 views × 3 themes = 27 references** in `qa/expected/gui/`, all reviewed.
  In-guest GUI screenshots remain an open item (needs the live session to start
  niri + Quickshell).
- **GUI gates PASS** — headless render 9/9; visual regression 9/9 per theme;
  RAM audit **11.2 MB** per GUI app against the 250 MB budget (way under the
  6144 MB idle / 8192 MB peak system caps).
- **Gate 6** — GitHub Actions `HCS Linux CI` green, including the new `gui` and
  `gui-wayland` jobs.
- **Gate 7** — PR + GitHub Release with ISO, SHA256SUMS, SBOM, notices, manifests.
  Open: `SHA256SUMS.gpg` (no maintainer signing key — deliberately not fabricated)
  and PR merge.

## Three real bugs the GUI gates caught

Worth calling out, because the obvious check (image entropy) scored all three as
PASS — a flat fill still reaches 1.6 entropy. Only the unique-colour count in
`verify_gui.py` exposed them:

1. **Shared window adapter** — the headless harness returned the *same* window for
   every component, so only the last view rendered and 7 of 8 views were solid
   black. Each component now gets its own adapter.
2. **Cross-file globals** — Slint globals do not initialise across file imports,
   so `root.palette` was a default-constructed (fully transparent) `Palette` and
   every widget was invisible. Rust now assigns the palette explicitly.
3. **Stacked tabs** — the chat window drew both tabs on top of each other.

Plus: long card values overflowed their cards (now wrapped), and chat
transcripts grew unbounded (now capped at 8000 chars).

## Build note

The GUI crates need `pkg-config libfontconfig-dev libfreetype-dev` on Linux
(Slint's software renderer with system fonts) plus a font package such as
`fonts-dejavu-core`. See `make help` and the CI workflow.

## RAM budget compliance (§5 matrix)

Idle ~480 MB → +0.6B resident ~1050 MB → 1.7B chat ~2200 MB → 4B reason ~4100 MB
→ image gen ~3100 MB → heavy multitask ~5400 MB. All ≤ 6144 idle / 8192 peak.
