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
- [x] **Verification & release:** Gates 1-4 + 6 PASS, Gate 7 published 2026-09-30:
  `HCS-Linux-1.0.0-amd64.iso` (235.47 MB, SHA256
  `f61816cafc9daf830742e84a31e18651872d71575e590009de09c2c4ff54614d`),
  SBOM/MODEL/BUILD manifests + THIRD-PARTY-NOTICES regenerated (v1.0.0),
  CI run 36750197514 green, PR #3 (dev→main) open,
  Release https://github.com/timfromhcs/hcs-linux/releases/tag/v1.0.0 (6 assets).
- [x] **Gate 5 VirtualBox 16-stage QA (v1.0.1):** PASS 16/16 on VirtualBox 7.2.10
  (2026-09-30, `qa/reports/install_qa_report.json`): GRUB menu `HCS Linux 1.0.1 Live
  Desktop`, live banner `HCS LINUX 1.0.1`, installed HDD-boot banner `HCS LINUX 1.0.1`
  — all screenshot-verified (`qa/screenshots/01..16_*`, entropy audit 32/32 PASS).
  Forensics on the 1.0.0 run found + fixed: literal `${VERSION}` banner (heredoc
  quoting), stale alpha VDI, black framebuffer timing — see CHANGELOG 1.0.1.
- [ ] **SHA256SUMS.gpg:** blocked (no maintainer release-signing key available);
  NOT fabricated — maintainer signs with release key on merge.

Open follow-ups (need maintainer decision): PR #3 merge, Gate-5-VirtualBox-Lauf, GPG-Signatur.
