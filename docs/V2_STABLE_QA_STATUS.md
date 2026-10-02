# HCS Linux 2.0 — Verified State

Every line below was measured on a real VirtualBox VM booting a real ISO, or
refused because it could not be measured. Nothing here is inferred from a build
log alone.

Last measured: ISO `1cf820d5…`, 175 payload checks, kernel 7.2.8+deb14-amd64.

---

## What works

| Capability | Evidence |
|---|---|
| The ISO boots | VirtualBox VM reaches the kernel log and `/init` |
| systemd is PID 1 | `/sbin/init -> ../lib/systemd/systemd`, gated in `verify_payload.sh` |
| A real base exists | 36,769 files, Debian sid, built by `build_base.sh`, not assembled by hand |
| Graphics stack is present | 9 Vulkan ICDs incl. lavapipe, GBM, EGL, llvmpipe, GL drivers |
| niri is installed | `fetch_niri.sh`, pinned commit, both digests verified |
| The live user exists | `hcs` in `/etc/passwd`, asserted after the hook runs |
| The session unit is reachable | absolute symlink, resolved in the guest namespace, asserted |
| Payload contract | 175 checks, failing the build on a violation |

## What does not work, and why

| Capability | State | Cause |
|---|---|---|
| Graphical desktop in a VM | **FAIL** | boot does not reach a usable VT; see below |
| 32-stage QA suite | **NOT RUN** | blocked by the desktop |
| Local AI inference | **FAIL** | `placeholder.gguf` is not a model |
| GPU acceleration | **NOT TESTED** | no physical machine in this loop |
| VirtualBox as a QA environment | **UNSUPPORTED** | `--graphicscontroller virtio` does not exist in 7.2.10 on Windows; QEMU is not installed |

## Bugs found by running the image

Each of these was invisible to the build and invisible in review. Each is now
gated.

1. **The ISO was not a bootable system.** `build_iso.sh` copied the build host's
   `/boot` and staged a 152-file directory inside an ISO. Two releases shipped
   that.

2. **`/sbin/init` suppressed systemd.** A shell script written over the base's
   own init, with GRUB told to run it. The image booted to a banner and stopped.

3. **No graphics stack existed.** grepping the package list for `mesa`,
   `vulkan`, `libgl`, `firmware` or `xf86` returned nothing. A compositor with no
   drivers for it.

4. **The bootstrap hook never ran.** `config/hooks/live/01-hcs-setup.hook.chroot`
   creates the live user, the getty override and the keyboard default. It was
   committed, reviewed, correct — and unreached, because `build_base.sh` never
   invoked it. **Three separate fixes were applied to that hook before anyone
   checked whether it ran at all.**

5. **`/usr/bin/seatd` does not exist.** Debian ships `seatd-launch` plus a unit.
   The session script tested for a filename the package does not use, so the seat
   was never created — presenting as a compositor failure.

6. **The session unit was enabled by a dangling symlink.** The wants link pointed
   at `../hcs-desktop.service` → `/etc/systemd/system/…`, while the unit lives in
   `/usr/lib/systemd/system/…`. Both are valid unit directories. systemd treats a
   dangling wants link as "nothing to do", so the desktop never started and
   **nothing reported it**: no failed unit, no red status.

7. **The autologin was on the wrong VT.** `getty@tty6` while GRUB passes
   `console=tty1` and the desktop claims tty1, producing a console that showed
   `Authentication failure` — a symptom of two getties colliding on one VT.

## The remaining blocker

The boot reaches systemd's `local-fs` stage and stops. It does not reach
`multi-user.target`, which is why the boot diagnostics never run and the evidence
disk comes back unformatted.

Two changes moved it measurably further, and both were about the observation
rather than the guest:

| Change | Result |
|---|---|
| Baseline | froze at `2.6 s` of kernel time, mid initramfs |
| `toram` on every kernel cmdline | reaches `paths.target`, plymouth, `local-fs-pre.target` |

`toram` was the right call for the wrong reason: the medium is read over an
emulated optical device, which is far slower than the hardware the image is
built for, so a slow boot and a hung boot were indistinguishable. Removing the
variable did not fix the boot — it revealed where the boot actually stops.

**What has been ruled out, with evidence:**

* overlayfs in the initramfs — present (`overlay.ko.xz`), along with squashfs,
  isofs, loop, ext4, fat
* the bootstrap hook — runs, `hcs` exists, `getty@tty1` autologins
* the session units — absolute symlinks, resolve in the guest namespace
* the graphics stack — 9 Vulkan ICDs incl. lavapipe, GBM, EGL, llvmpipe
* GRUB waiting at a menu — the menu is now suppressed on the QA image

**What has not been ruled out:**

* `umugfx` offering no usable mode (`128x48` in the earlier guest log). Nothing
  has yet read a DRM mode list from a running guest, because the diagnostics that
  would do so never get to run.
* Anything between `local-fs.target` and `multi-user.target` — that window is
  where the boot now stops and it has not been observed from the inside.

The next diagnostic must answer one question: what is `local-fs.target` waiting
on. `systemctl list-jobs` and the failed-units list, read from a guest that
reaches far enough to write them, will answer it in one boot.

## The instrument, and why it is not enough yet

`hcs-diagnostics.service` writes `dmesg`, the failed-units list, the journal,
the DRM device list, the modes each card offers, the active VT and seat state to
the evidence VHD as FAT32, which Windows mounts natively. It depends on no Guest
Additions, no network and no graphics — if the desktop never draws, it still
answers why.

It has not run yet, because it is ordered after `multi-user.target` and the boot
does not reach it. **Its ordering is the next thing to change**: it should be
ordered against `sysinit.target` or `basic.target` so it reports on the stall
that is stopping it. An instrument that cannot observe the failure it was built
for is not yet an instrument.

## What was claimed wrongly and corrected

An earlier commit asserted overlayfs was missing from the initramfs, from a grep
that returned zero. Run properly it returns three matches, `overlay.ko.xz`
included. The module was always there; `overlay not supported` comes from
live-boot's shutdown hook, not from a mount attempt.

That correction is recorded here because the pattern — a plausible story, one
check, no cross-check — is the same one behind bugs 4, 5 and 6.

## Gates that must not be softened

* `verify_payload.sh` fails on a regular file at `/sbin/init`, on a dangling
  session unit, on a missing GBM/EGL/llvmpipe/lavapipe, on a `init=` GRUB
  override.
* `build_base.sh` aborts if the bootstrap hook fails, or if it completes without
  producing the `hcs` user and a correct getty override.
* `grade_capture.py` is the only thing allowed to say a desktop was drawn.