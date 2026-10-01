# HCS Linux v2.0.0 — QA status, stated plainly

Last updated during the v2.0.0 stabilisation pass. This file exists because the
alternative is a release note that says things are better than they are.

## The headline

**No v2.0.0 ISO has booted to a desktop. The image the build produces is not a
bootable Linux system.**

`HCS-Linux-2.0.0-qa-amd64.iso` (303 MB) builds cleanly, passes the payload
contract with 143 checks, and passes ISO9660 structural verification. It also
contains 152 files, of which 27 are HCS binaries, 105 are HCS data files, 17
are HCS config files, and one is an HCS init script.

It contains no `/bin/sh`, no libc, no systemd, no `niri`, no `quickshell`, no
`grim`, no `tesseract`. There is no Debian base system in it at all.

## Why

`scripts/build_iso.sh` never assembles a base system. It stages a directory of
HCS binaries and assets, then copies **the build host's own kernel and initrd**
into the image:

```
scripts/build_iso.sh:441   cp -L "/boot/vmlinuz" "${ISO_STAGING}/live/vmlinuz"
scripts/build_iso.sh:453   cp -L "/boot/initrd.img" "${ISO_STAGING}/live/initrd.img"
```

Those two lines are the whole kernel story. On the WSL host used for this build
they resolved to Ubuntu 26.04's kernel and a companion cpio containing one file.
`config/package-lists/hcs-core.list.chroot`, which declares 75 packages, is read
by nothing in the build. `debootstrap` and `live-build` are installed on the
build host and never invoked.

## Why the checks did not catch it

The v2 CI job named "The session stack is actually declared" greps the package
list for the strings `niri` and `quickshell` and reports success:

```yaml
grep -q "^${pkg}\$" config/package-lists/hcs-core.list.chroot
```

That asserts a word appears in a text file. It is the same class of mistake as
v1's "clippy 0 warnings" claim, and it was added in the same commit series.

## The part that cannot be fixed by writing more code

`niri` and `quickshell` — the compositor and the shell the entire v2 desktop
design is built on — are **not packaged in any Debian or Ubuntu suite**. Checked
against the full binary-amd64 indexes:

| base                    | niri | quickshell |
|-------------------------|------|------------|
| Debian 13 (trixie)      | no   | no         |
| Debian 12 (bookworm)    | no   | no         |
| Ubuntu 24.04 (noble)    | no   | no         |
| Ubuntu 25.04 (plucky)   | no   | no         |
| Ubuntu 26.04 (questing) | no   | no         |

So `apt-get install` against that list cannot succeed, and no amount of
correctness in the rest of the pipeline produces a desktop. This is a base
decision with three honest options:

1. **Build niri and quickshell from source into the image.** Both are Rust
   projects and both build against Qt 6, which trixie does ship. This keeps the
   Debian base and the supply chain unchanged, and costs build time and a
   vendoring/pinning decision.
2. **Choose a compositor and shell trixie does ship** — sway or wayfire with
   waybar/fuzzel, or GNOME. Cheapest and most honest, but it abandons the
   niri + Quickshell design the plan is written around, and the QML shell would
   need porting to whatever the alternative shell is.
3. **Adopt a third-party binary repository.** Fastest, and it changes where
   every binary in the image comes from. That is a supply-chain decision with
   real consequences and should be made deliberately and recorded, not adopted
   because it was convenient.

Also missing from trixie and needing a decision either way:
`xdg-desktop-portal-hyprland` (irrelevant if the compositor is not Hyprland),
`qt6-declarative` (a virtual name; the real packages are `qml6-module-qtquick*`),
and `plymouth-theme-spinner` (shipped as part of `plymouth-themes`).

## What is genuinely verified

These are real, reproducible, and did not depend on a booting image:

| Gate | Result | Notes |
|------|--------|-------|
| `cargo fmt --all -- --check` | PASS | |
| `cargo clippy --workspace --all-targets -D warnings` | PASS | 0 warnings, Linux and Windows toolchains |
| `cargo test --workspace` | PASS | 316 tests |
| Python unit tests | PASS | 38 tests |
| Security audit | PASS | 0 unredacted secrets |
| GUI render, 4 themes | PASS | 9/9 each, blank-frame gate |
| GUI visual regression | PASS | 9/9 per theme, per-platform reference sets |
| GUI RAM budget | PASS | 8/8 apps under 250 MB |
| Accessibility | PASS | 16 contrast checks, 12 views keyboard-reachable |
| Shell bindings | PASS | 44/44 resolve to real commands; no duplicate combos |
| Cheatsheet consistency | PASS | generated from config.kdl, not hand-maintained |
| RAG retrieval | PASS | answers cite a manual section; out-of-scope refused |
| ISO payload contract | PASS | 143 checks against a real staged tree |
| ISO structure | PASS | ISO9660 verified, checksum recorded |
| **Graphical session in a VM** | **NOT RUN** | no bootable image exists |
| **32-stage VirtualBox suite** | **NOT RUN** | driver is fixed and tested; nothing to run it against |

## About the VirtualBox gate itself

`scripts/qa_virtualbox_v2.ps1` and the in-guest agent were rebuilt, because the
version that shipped could only ever end in "the QA agent never reported in":

- the host waited for a journal the guest never wrote there;
- it called `VBoxManage guestcontrol` against a live ISO with no Guest Additions;
- it passed scenarios to a path that did not exist inside the guest;
- the agent never populated `windows` or `last_capture_text`, so every
  `AssertWindow` and `AssertText` failed;
- the GRUB menu had no `hcs.qa=1`, so the agent refused to start;
- the pass/fail decision lived in PowerShell no test could reach, so a run with
  every stage skipped still reported a pass count.

The driver now attaches a blank disk, boots a dedicated QA image whose default
entry carries the flag, and reads the guest's own results back off that disk. The
guest needs no guest additions, no shared folder and no socket. The decision
logic lives in `scripts/grade_qa_evidence.py` and is covered by
`tests/unit/test_qa_grading.py`, which pins the rules: a skipped stage is not a
pass, an abort is not a pass, a counter that disagrees with the stage rows is
not a pass, and the host's independent re-grade of every frame overrides the
guest's own verdict.

It is correct and it is tested. It has never been executed against a booting
image, and this file does not pretend otherwise.
