# HCS Linux v2.0.0 — QA status, stated plainly

Last updated during the v2.0.0 stabilisation pass. This file exists because the
alternative is a release note that says things are better than they are.

## The headline

**The image boots, but it does not reach a desktop.** That is one real step
further than it was, and it is worth being precise about the difference.

`HCS-Linux-2.0.0-qa-amd64.iso` (1.4 GB) builds, passes the payload contract with
157 checks, and **boots**. It is a real Debian system: 37,051 files, its own
kernel `7.2.8+deb14-amd64` with 4,216 modules, niri 26.04, quickshell, systemd,
Calamares, grim, tesseract. It mounts, starts, and runs HCS binaries.

What it does not do is show a desktop. It boots into `hcs-live`, prints the
banner, and sits there. The reason is below, and it is a single leftover file.

## What is now true

- A real base system is assembled by `scripts/build_base.sh`: Debian sid,
  debootstrapped, the declared package list installed into it, verified to be a
  system and not a directory.
- The image takes the **base's** kernel and initrd. It no longer copies the build
  host's.
- niri is built from its own pinned commit and dependency bundle, with both
  artefact digests verified. quickshell comes from Debian sid as a pinned package.
- The payload gate asserts the image is a Linux system — a shell, libc, init,
  compositor, shell, the QA tools, kernel modules for the *shipped* kernel, and a
  minimum file count. It would now fail the 152-file payload that two releases
  shipped.

## Why it still does not reach a desktop

The image overrides `init=/sbin/init`, and `/sbin/init` is a 120-line shell
script left over from the era when there was no system to run. It prints a
banner, starts niri by hand, and then either `exec`s systemd or loops. In the
real run the banner repeats, so it is being re-entered — and no desktop appears.

The fix is architectural, not a patch: **stop overriding init.** The base's own
`/sbin/init` is a symlink to systemd, `live-boot` is already installed to mount
the squashfs, and the session should be started by a systemd unit rather than by
a PID 1 shell script. The banner script predates the base system and has no
reason to exist now.

Until that is done, the honest verdict is: **the 32-stage VirtualBox suite cannot
pass**, because no desktop is drawn.

## What was wrong before

`scripts/build_iso.sh` never assembled a base system. It staged a directory of
HCS binaries and assets, then copied **the build host's own kernel and initrd**
into the image:

```
scripts/build_iso.sh:441   cp -L "/boot/vmlinuz" "${ISO_STAGING}/live/vmlinuz"
```

Those two lines are the whole kernel story. On the WSL host used for this build
they resolved to Ubuntu 26.04's kernel and a companion cpio containing one file.
`config/package-lists/hcs-core.list.chroot` was read by nothing.
`debootstrap` and `live-build` were installed on the build host and never invoked.

The result had 152 files: 27 HCS binaries, 105 HCS data files, 17 HCS config
files, and one HCS init script. No `/bin/sh`, no libc, no systemd, no compositor.
It could not boot at all.

## Why the checks did not catch it

The v2 CI job named "The session stack is actually declared" greps the package
list for the strings `niri` and `quickshell` and reports success:

```yaml
grep -q "^${pkg}\$" config/package-lists/hcs-core.list.chroot
```

That asserts a word appears in a text file. It is the same class of mistake as
v1's "clippy 0 warnings" claim. `scripts/verify_package_availability.py` is the
replacement: it asks the distribution, honours `Provides` so virtual package
names are not falsely accused, and reports what is genuinely absent.

## The base-system decision, as taken

The decision was to take the desktop stack from external sources rather than
compile it in-tree. What that turned into:

| component | source | provenance |
|---|---|---|
| quickshell 0.3.1 | Debian **sid**, ordinary pinned package | first-party Debian |
| niri 26.04 | niri's own repo, pinned commit, digests verified | first-party upstream |
| everything else | Debian sid package list | first-party Debian |

The base moved from trixie to sid because quickshell 0.3.1-1+b1 requires
`libqt6core6t64 >= 6.11.2` and `qt6-base-private-abi (= 6.11.2)`, and trixie ships
Qt 6.8 — so taking that one binary in a trixie base would have dragged the whole
Qt 6.11 stack with it.

niri is not packaged in Debian 12 or 13, in Ubuntu 24.04/25.04/26.04, or in
Fedora, and its own releases attach no binary. So for that component the
"external binary" route does not exist and the build uses niri's own artefacts.

Two limits, stated rather than hidden:

- The build is **not offline**. niri pins smithay and smithay-drm-extras to git
  commit hashes rather than crates.io versions, and the release's vendored bundle
  contains no git checkouts, so `--offline` fails at resolution. Versions still
  come from the tag's `Cargo.lock` and both digests are verified.
- **sid is an unstable base.** That is a real cost of the decision.

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
| ISO payload contract | PASS | 157 checks against a real staged tree |
| ISO structure | PASS | ISO9660 verified, checksum recorded |
| **Image boots** | **PASS** | 1.4 GB image, Debian sid base, own kernel 7.2.8 |
| **Graphical session in a VM** | **FAIL** | boots to a console; `/sbin/init` overrides systemd |
| **32-stage VirtualBox suite** | **NOT RUN** | nothing to photograph until a desktop is drawn |

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

It is correct and it is tested. It has been run against the real image, and the
run is what found the remaining blocker: the driver got the VM booted, attached
the evidence disk and polled for the DONE marker exactly as designed, and the
guest never wrote one — because there is no desktop for the QA agent to
photograph.

The driver itself needed four fixes that only a real run could reveal: this
VirtualBox build has no `createvhd` (it is `createmedium`), no storage
controller on a default VM (it is `storagectl --add`), `--device` takes a number
rather than a type name, and `--bootorder` does not exist. All four produced a
bare usage dump and no other clue.
