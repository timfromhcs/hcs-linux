# V1 Stable Plan — Acceptance Checklist Status (2026-09-30)

Source: `docs/V1_STABLE_RELEASE_MASTER_PLAN.md` §7. Checked against this workspace.

- [x] **Taskbar (`taskbar.qml`):** `src/hcs-shell/taskbar.qml` + synced to
  `config/includes.chroot/usr/share/hcs/shell/` — 52px bottom, Start 36x36, search pill,
  tasklist w/ 2px cyan underline, TOR SHIELD + AI footprint + tray cluster.
- [x] **Start Menu (`start_menu.qml`):** 560x640 r18, omnibar, 8 pinned tiles, recents,
  avatar + Lock/Sleep/Restart/Power Off. Synced to includes.chroot.
- [x] **Brand identity:** 3 wallpapers (`neural_glass_dark`, `frosted_titanium`,
  `cybernetic_stealth`), Plymouth theme (`assets/boot/plymouth/hcs/` → includes.chroot),
  new `hcs-image/hcs-docs/hcs-tor` SVG icons.
- [x] **CPU image gen (`src/hcs-image`):** lib (engine/memory_guard/postprocess) + `hcs-image`
  binary + `hcs image` subcommand; workspace member; `config/models/image.json`;
  `vendor/PINNED.md`; 6 unit tests PASS; CLI verified (`--help`, txt2img RAM-gate run).
- [x] **Pentest arsenal:** `hcs-security.list.chroot` expanded to full §4.3 list
  (Debian purity, no Kali); `TorTransparentProxy::enable_rules` + zero-DNS-leak test;
  HITL gate + test; `hcs-tor-switch` script in includes.chroot; `pentester` agent role.
- [x] **Tor switch + browser:** nftables fail-closed rules (9040/9053, debian-tor bypass,
  loopback, IPv6 drop); `tor` + `torbrowser-launcher` + `nftables` in package list;
  taskbar TOR SHIELD pill wired to `hcs-tor-switch toggle`; `hcs security tor enable`
  prints applied nft rules (verified).
- [x] **Developer workstation:** `hcs-developer.list.chroot` (rustc/cargo, node/npm, nvim,
  codium, lldb, shellcheck...); `hcs dev init/test/debug` (verified).
- [x] **Offline docs + onboarding:** `hcs-docs` portal (index + 6 manuals + theme.css +
  search.js + cheatsheet.json), `hcs-docs` + `hcs-welcome` launchers, `cheatsheet.qml`
  (Super+//F1, 3 cards) synced to shell dir.
- [x] **Calamares OEM:** branded `show.qml` slideshow, `partition.conf` (LUKS2 AES-XTS-512,
  automated options), `users.conf` (groups, HW AI-profile convention).
- [~] **Verification & release:** Gates 1-3 PASS + Gate-4 SBOM/manifests regenerated
  (`dist/SBOM.spdx.json`, `MODEL/BUILD-MANIFEST.json`, `THIRD-PARTY-NOTICES.txt`);
  Gates 4-7 env-blocked/pending — see `dist/RELEASE-NOTES-1.0.0.md` for exact commands.

Open follow-ups (need maintainer decision): version bump → 1.0.0, real ISO build on
Debian host, VirtualBox 16-stage run, CI push, PR + GPG signing.
