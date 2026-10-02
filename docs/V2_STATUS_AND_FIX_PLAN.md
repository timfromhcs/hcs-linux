# HCS Linux 2.0 — Status, all attempts, and the plan to actually fix it

Written at the end of a long debugging session, so that the next attempt starts
from what was measured rather than from what was believed. Every claim below is
either observed with evidence or explicitly marked as not established.

---

## Part 1 — Where it actually stands

### 1.1 Verified, with evidence

| Capability | State | Evidence |
|---|---|---|
| A real base system is built | **PASS** | `build_base.sh` produces 36,769 files, Debian sid, 2.4 GB |
| The ISO boots | **PASS** | VirtualBox VM reaches `squashfs: version 4.0` in live-boot |
| Boot reaches a login shell | **PASS (once)** | `hcs@hcs-live:~$` observed at ~700 s |
| Autologin as `hcs` works | **PASS** | `hcs@hcs-live:~$`, getty@tty1 drop-in |
| systemd is PID 1 | **PASS** | `/sbin/init -> ../lib/systemd/systemd`, gated |
| The bootstrap hook runs | **PASS** | `hcs` in `/etc/passwd`, asserted after it runs |
| Session units enabled and resolving | **PASS** | absolute symlinks verified in-guest, gated |
| Graphics stack is present | **PASS** | 9 Vulkan ICDs incl. lavapipe, GBM, EGL, llvmpipe |
| niri is installed | **PASS** | `fetch_niri.sh`, pinned commit, both digests |
| Software fallback compositor | **PASS** | `pick-compositor.sh`, labwc on Xvfb, gated |
| Payload contract | **PASS** | 185 checks |
| Console logging of the session | **PASS** | every session log line also goes to `/dev/console` |

### 1.2 Not verified — the open claims

| Capability | State | Why |
|---|---|---|
| Boot time acceptable for automation | **FAIL** | ~700 s, and non-deterministic (650 s run never got there) |
| **A desktop is drawn** | **NOT RUN** | no run has reached the session start reliably |
| The desktop's actual failure reason | **UNKNOWN** | no run has produced the diagnostics |
| 32-stage QA suite | **NOT RUN** | blocked by the above |
| Local AI inference | **FAIL** | `placeholder.gguf` is not a model |
| GPU hardware | **UNTESTED** | no physical machine; WSLg has no `/dev/dri` |

### 1.3 The three constraints that shape everything

1. **niri cannot run on a software renderer.** From `src/backend/tty.rs`:
   `ensure!(!egl_device.is_software(), "software EGL renderers are skipped")`.
   Upstream issue #218: *"There's no software rendering support at the moment."*
   There is no second fallback.
2. **This host cannot pass GPU acceleration through.** WSLg has no `/dev/dri`; the
   AMD Radeon is a vGPU behind a Parsec virtual display adapter; VirtualBox
   reports `umugfx: running on an unsupported hypervisor`. VirtualBox 7.2 on
   Windows has no `virtio` graphics controller (`--graphicscontroller virtio` is
   rejected), and VirtualBox's own docs say only VMSVGA is supported.
3. **Guest Additions is not redistributable.** Oracle's licence allows personal
   and test use, not embedding in an OS image. So a QA run with Guest Additions
   would **not** describe the shipped ISO.

Consequence, and it is the whole architecture: **the shipped image ships two
compositors.** niri when hardware GL exists, labwc on Xvfb when it does not.

---

## Part 2 — Every attempt made, and what it taught

Listed in order, including the ones that were wrong. The wrong ones are the
useful ones.

### 2.1 Bugs that were real and are now fixed

| # | Bug | How it was invisible |
|---|---|---|
| 1 | ISO was a 152-file directory with the **build host's** kernel | Nothing checked that it was a system |
| 2 | `/sbin/init` overridden, so systemd was never PID 1 | No gate looked at the file |
| 3 | **No graphics drivers at all** | Gates checked files were *present*, not that a renderer *existed* |
| 4 | **The bootstrap hook never ran** | Correct, committed, reviewed — and unreached. Three fixes applied to it before anyone checked whether it ran |
| 5 | `seatd` looked for at `/usr/bin/seatd` | Debian ships `seatd-launch` + a unit |
| 6 | Session unit enabled by a **dangling symlink** | systemd treats it as "nothing to do". No failed unit, no red status |
| 7 | Autologin on `tty6` while GRUB passes `console=tty1` | Two getties on one VT → "Authentication failure" |
| 8 | Diagnostics ordered **after** the thing it diagnoses | Never executed; noticed only because the disk came back unformatted |
| 9 | GRUB menu drawn despite `timeout=0` | A menu that auto-selects looks like a hung boot to a screenshot |
| 10 | Renderer forced to software, which niri rejects | I wrote the bug and then hunted it for hours |

### 2.2 Attempts that did not work, and why

**`toram` on the kernel cmdline — WRONG, and I asserted it as the fix.**
Read from `live-boot(7)`: `toram` copies "the whole read-only media" to RAM, and
`overlay-size` defaults to 50% of RAM with "no effect when booting with toram". So
I removed it and committed that as the cause. **It was not the cause.** With
`toram` gone and `overlay-size=2g` set, `overlay not supported` still appears. It
appears with and without `toram`. Kept off anyway as configuration hygiene.
Gated, so it does not come back.

**Reading `overlay not supported` as fatal — WRONG.**
Two runs, same image family: `HCS-Fix` reached a login shell at ~700 s;
`HCS-Log` was frozen at 650 s. So the line is not the failure. The boot is
**slow and marginal**, right at the edge of the observation window.

**Treating "frame unchanged" as "stopped" — WRONG, three times.**
Two identical frames mean "no progress *in that interval*", not "no progress". A
screenshot is a single instant; it cannot measure time. Every "stall" in this
project is a snapshot of a slow boot.

**Attributing the niri failure to missing Mesa — WRONG, twice.**
The package list genuinely had no `mesa`/`vulkan`/`libgl`/`firmware`, so adding
them was correct. But the deeper cause is that niri refuses software EGL at all,
so a machine with no GPU cannot run it regardless of which Mesa packages are
present. Adding Mesa fixed a real gap and did not fix the desktop.

**`--graphicscontroller virtio` — impossible.**
VirtualBox 7.2.10 on Windows rejects it. There is no way to give the guest a
modern DRM device here.

**`--uart1` for a serial log — failed.**
VBoxManage reports `Invalid IRQ number of the serial port 0: 4048261072`, a
pointer-sized garbage value, for every IRQ from 0 to 255 and for the `.vbox` XML
edit route. Editing `<Hardware><Serial>` directly is dropped when VBoxManage
rewrites the config. **So there is no working serial channel on this host**, which
is why console logging was the only remaining option.

**`Mount-DiskImage` for reading the evidence VHD — needs admin.**
`Der Client fehlt ein erforderliches Recht`. Worked around with
`wsl --mount --vhd --bare` after detaching the medium from VirtualBox.
**Note:** `unregistervm --delete` destroys the evidence disk. Must detach first.

### 2.3 The pattern behind the mistakes

Every wrong turn had the same shape: **a plausible story, one check, no
cross-check.**

* Three fixes applied to a hook that was never executed.
* A missing-module theory built on a grep that returned zero, later found to
  return three matches.
* An overlay theory built on a manpage paragraph and committed before the removal
  was tested in isolation — the disproving screenshot arrived after the commit.
* A screenshot read as "stopped" when it meant "slow".

The cost was measured: each of these cost 20–40 minutes of build-and-boot cycles.
The mitigation is not care. It is that **every fix now gets a gate that fails when
the condition returns**, and every fix is tested in isolation before anything is
built on it.

---

## Part 3 — Why the boot is slow, and how to fix it

### 3.1 The diagnosis

The image is 1.5 GB. It is being read over an **emulated optical device**, which
is orders of magnitude slower than a block device. live-boot mounts and verifies
the squashfs from that medium, and the whole session then reads from it.

That is why the boot sits at the `squashfs: version 4.0` line for minutes, why one
run got through in 700 s and another did not get through in 650 s, and why no
observation ever looked like progress.

### 3.2 Step 1 — attach the ISO as a disk, not a DVD

**Expected effect: largest single win. A change to the QA driver, not the image.**

```powershell
# instead of:
& $vbox storageattach $vm --storagectl SATA --port 1 --device 0 --type dvddrive --medium $iso

# attach the ISO image as a plain hard disk:
& $vbox storageattach $vm --storagectl SATA --port 1 --device 0 --type hdd --medium $iso
& $vbox modifyvm $vm --bootorder hdd
```

Two things make this correct rather than merely faster:

* The medium becomes a normal read-only block device, which is **closer to how a
  real USB stick presents itself** than an ISO9660 DVD image is. The QA result is
  more representative, not just quicker.
* Combined with `live-media=/dev/sda` on the kernel cmdline, live-boot stops
  scanning every block device looking for `/live`. The manpage documents this
  parameter; skipping the scan is both faster and deterministic.

**Acceptance:** boot reaches a login shell in well under 300 s, on every run.

### 3.3 Step 2 — a heartbeat, so "slow" and "stopped" are distinguishable

This is the gate whose absence caused the session. Without it, every future
diagnosis inherits the same ambiguity.

Three cheap signals, in increasing order of reliability:

1. **Console timestamps.** live-boot already prints timestamps with `debug` on the
   cmdline; systemd prints them per unit. One frame then carries progress.
2. **A guest heartbeat.** `hcs-diagnostics.service` already writes to the evidence
   VHD; have it rewrite a timestamp file every 30 s. The host polls the file size
   or mtime — with `wsl --mount --vhd` this is cheap.
3. **A frame-counter artefact.** The session writes its own render count to a file
   on the evidence disk. Increasing means the compositor is alive.

Option 2 is the best value: it works on the X11 path where there is no Wayland
counter, and it is readable from the host without Guest Additions.

**Acceptance:** the VM gate distinguishes "still booting" from "hung" and reports
which, rather than inferring from frame equality.

### 3.4 Step 3 — shrink the image

1.5 GB is paid on every read. Candidates to move out of the *live* cohort into the
*installer* cohort, or out entirely:

| Package | Size class | Needed for a desktop? |
|---|---|---|
| `ffmpeg` | very large | no |
| `fonts-noto-cjk` | very large | only for CJK |
| `calamares` | large | only the installer cohort |
| `tesseract-ocr` + eng + deu | moderate | only the QA agent |
| firmware families | moderate | yes, keep |

**Acceptance:** ISO materially smaller; boot time improves further.

### 3.5 Step 4 — what to do about live-boot itself

If the boot is still not fast enough after Steps 1–3, then live-boot becomes the
thing to replace. Its cost here is that it does slow discovery, builds an overlay
to make a read-only squashfs writable, and provides no useful diagnostics on
failure.

The alternative is the standard "read-only root" arrangement, well documented and
in production elsewhere: mount the squashfs read-only, mount a tmpfs for the
overlay's `upperdir`/`workdir`, mount the overlay, `pivot_root`, exec `/sbin/init`.
The kernel can also mount a squashfs directly as `root=` with no overlay at all,
with tmpfs mounts for `/run`, `/tmp` and `/var` — which is what many live images
actually do.

**This is a real rewrite, not a tweak. Do it only if Steps 1–3 are insufficient**,
and only after measuring what they achieve.

---

## Part 4 — Getting to a desktop, once the boot is fast

Ordered so that each step's failure is still diagnosable.

| # | Step | Acceptance |
|---|---|---|
| 1 | Fast boot (Step 3.2) | login shell in < 300 s, every run |
| 2 | Evidence disk readable | FAT32 present, `SUMMARY.txt` mounts on the host |
| 3 | Read `SUMMARY.txt` | names the real blocker — this is the answer we have never had |
| 4 | Confirm the compositor choice | `/run/hcs/compositor.info` names `labwc` and why |
| 5 | labwc on Xvfb starts | `xdpyinfo` answers; a window maps |
| 6 | Quickshell draws the shell | screenshot with the Neural Glass shell visible |
| 7 | `grade_capture.py` accepts the frame | host-side, GREEN |
| 8 | Apps as windows | each of the 12 GUI apps opens |
| 9 | Four boot cohorts | `live`, `installer`, `installed`, `amnesic`, each with its own verdict |

Steps 3 and 4 are the ones that unblock everything. They need only a fast enough
boot, which Step 1 provides.

### 4.1 The QA agent cannot use grim on the fallback path

`grim` screenshots Wayland. The labwc path is X11 on Xvfb. So on the fallback:

* capture with `import -window root` (ImageMagick is now in the package list)
* drive input with `xdotool` (also now in the package list)

`hcs-qa-agent` must branch on `compositor.info`. If it does not, it will report
"capture failed" on every software-rendered run and that will be read as a
desktop failure rather than a tooling mismatch. **This is the next bug waiting to
happen and it is worth fixing before the first successful run, not after.**

---

## Part 5 — Order of work

| Priority | Work | Cost | Unblocks |
|---|---|---|---|
| 1 | ISO as disk + `live-media=` (§3.2) | minutes | everything |
| 2 | Heartbeat in the diagnostics (§3.3) | ~1 h | reliable diagnosis |
| 3 | Read `SUMMARY.txt`, name the blocker | 1 boot | the desktop work |
| 4 | QA agent: X11 capture path (§4.1) | ~2 h | honest visual QA |
| 5 | labwc + shell draws (§4 steps 5–7) | unknown | the first real screenshot |
| 6 | Shrink the image (§3.4) | ~1 day | faster still |
| 7 | Replace live-boot, if needed (§3.5) | days | only if 1–6 fall short |

Steps 1 and 2 are the whole of what I would do next. They are small, they are
certain, and between them they convert "the boot is slow and I cannot tell"
into "here is the specific unit that is blocking".

---

## Part 6 — Two corrections to keep visible

**The `toram` fix was wrong.** I removed it and committed it as the cause. It is
not the cause. The gate I added is still correct as configuration hygiene — bare
`toram` and a default `overlay-size` really are in tension — but it did not fix
this stall. See §2.2.

**The "lavapipe as the guaranteed floor" claim was wrong.** It was in
`docs/V2_NEXT_STEPS_PLAN.md` and it contradicted niri's source. Software
rendering is not a floor for niri; it is a rejected input. The corrected
architecture is two compositors, in §1.3.

Both are recorded rather than deleted, because the failure mode they share —
one plausible story, one check — is the failure mode this project keeps hitting,
and the corrections are more useful to the next reader than the claims were.

---

## Part 7 — Still outstanding, independent of the above

* **v1.1.0 release.** The README on `main` links `releases/tag/v1.1.0`, and
  `gh release view v1.1.0` reports *release not found*. Either the CI/CD release
  exists under a different tag, or the README is wrong. **Unresolved and
  reported, not silently patched.**
* **A real starter model.** `placeholder.gguf` is not a model.
* **sid → snapshot pin**, for reproducible builds.
* **`sources.lock.yaml`** still holds placeholder pins; SBOM and third-party
  notices not regenerated from the built image.
* **`hcs-agent-bridge`** listed in the master plan, never built.
* **Rotate the GitHub token.** It was pasted in plain text and is still in this
  session's history.
