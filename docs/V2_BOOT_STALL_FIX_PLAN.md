# HCS Linux 2.0 — Fixing the boot stall (researched)

## 1. The cause, and I introduced it

`live-boot(7)` states two things that together explain everything:

> **`toram`** — Adding this parameter, live-boot will try to **copy the whole
> read-only media** to the computer's RAM before mounting the root filesystem.
> This could need a lot of ram, according to the space used by the read-only
> media.

> **`overlay-size=SIZE`** — The size of the tmpfs mount (**used for the upperdir
> union root mount**) in bytes… **By default, 50% of available RAM will be used.**
>
> *(Note: this option has currently no effect when booting with toram.)*

And from the overlay mount requirements in live-boot's own helper:

> *overlayfs requires: + a workdir to become mounted + workdir and upperdir to
> reside under the same mount + workdir and upperdir to be in separate
> directories*

So the chain is:

1. `toram` copies the entire medium — the 1.5 GB ISO and its squashfs — into a
   tmpfs.
2. Overlay then needs a **separate** tmpfs for `upperdir`/`workdir`, sized at 50%
   of RAM by default.
3. There is not enough RAM left, the overlay mount fails, and live-boot prints
   `overlay not supported` and stops.

`toram` consumed the very resource overlay depends on.

### 1.1 The evidence that this is mine, from my own screenshots

Every successful boot in this project predates `toram`. Every stall follows it.

| Run | `toram`? | Result |
|---|---|---|
| `gpu-01` | no | kernel log, `umugfx` probe |
| `v-02` | no | **reached getty** — `Authentication failure` |
| `b-02` | no | **reached getty** — `Authentication failure` |
| `f-08` | **yes** | `ooum: overlay not supported` — **stalled** |
| `g-09` | **yes** | kernel log frozen at 2.6 s — **stalled** |
| `x-07` | **yes** | `ooum: overlay not supported` — **stalled** |

The `Authentication failure` screen I spent three build cycles treating as a
getty/autologin bug was the boot **succeeding**. It was never the problem. I was
debugging the message instead of the milestone behind it.

### 1.2 Why I added it, and why that reasoning was wrong

I added `toram` to a build that "froze at 2.6 s of kernel time", reasoning that
the emulated optical device was slow and that removing the variable would
separate a slow boot from a hung one. It did separate them — it revealed where the
boot stops — and then I kept the change.

But the slow-boot observation was made on a *larger* image (after adding labwc,
imagemagick and Qt), and the stall it was meant to diagnose turned out to be
caused by the fix for the stall before it. I optimised against a symptom whose
cause I had not established, and the optimisation was itself the regression.

**The lesson is not "be careful with toram".** It is that a measurement taken
before the relevant variable changed is not evidence about the system after it.
I used a pre-change observation to justify a change, then used the resulting
behaviour as further evidence.

---

## 2. The fix

### Step 1 — remove `toram` from every kernel cmdline (the actual fix)

One-line change in `scripts/build_iso.sh`. Expected result: the boot returns to
reaching getty, and the diagnostics service at `basic.target` finally runs.

**This is the highest-value single change available and it costs one build.**

### Step 2 — keep the software fallback compositor

Unchanged by this bug. niri still rejects software EGL by design, and
`pick-compositor.sh` still chooses labwc on Xvfb when there is no hardware GL.
Both were correct and neither caused the stall.

### Step 3 — if the boot is genuinely slow, fix the cause instead

`toram` was treating a symptom. The real options, in order of preference:

**3a. Attach the ISO as a disk, not a DVD, for QA.**
An emulated optical device is orders of magnitude slower than a block device. The
QA driver can attach the same ISO to a SATA/SATA-controller HDD slot and boot
from disk. This removes the performance problem *and* the medium is then a normal
read-only block device, which is closer to how a real USB stick behaves than an
ISO9660 DVD image is.

**3b. Set `overlay-size=` explicitly.**
Currently it defaults to 50% of RAM. An explicit value (say `2g`) stops
overlay from competing with the rest of the system for memory, and makes the
allocation visible instead of emergent.

**3c. Reduce image size.**
The base is 2.4 GB. `ffmpeg`, `tesseract`, `calamares`, the firmware family set
and the non-CJK/CJK font sets are large and not all needed in every boot cohort.
Splitting the installer payload out of the live cohort is the cleanest version of
this.

**Explicitly not doing:**
* `union=aufs`. aufs was removed from the Linux kernel in 5.18; Debian sid runs
  7.2. There is no aufs to switch to.
* `toram=<list>`. It still copies the medium; the narrower forms only reduce how
  much. The memory conflict remains.

---

## 3. Verification — and why it must be checked, not assumed

Each of these is a claim that could be wrong, and this project has spent a
session producing exactly that kind of wrong claim.

| # | Check | Pass condition |
|---|---|---|
| 1 | Boot with `toram` removed | reaches getty or a desktop — **not** `overlay not supported` |
| 2 | `hcs-diagnostics.service` runs | evidence VHD comes back **formatted** (FAT32) |
| 3 | `SUMMARY.txt` readable from the host | mounted via `wsl --mount --vhd` |
| 4 | `compositor.info` present | names niri or labwc, and why |
| 5 | Capture gate | final frame passes `grade_capture.py` |

Check 2 is the one that matters most: the evidence disk has come back with **no
filesystem** on every run so far, which is the only reason I know the diagnostics
never executed. If it comes back formatted, the whole instrument is finally
working and check 3 becomes the answer to the original question.

---

## 4. Order of work

1. Remove `toram`, rebuild, boot. **One build cycle answers the main question.**
2. Read the evidence VHD. Name the actual blocker from `SUMMARY.txt` and
   `list-jobs`.
3. Only then decide what else is needed — with a guest that can finally be asked.

Steps after 2 are unknown because they depend on what step 2 says. Producing a
longer plan now would be the same mistake as the two plans that were already
wrong: reasoning about a system whose only diagnostics had never run.

---

## 5. Gates to prevent a repeat

* **No kernel cmdline may carry bare `toram`** while `overlay-size` defaults to
  50% of RAM. Checked in `verify_payload.sh` by reading `build_iso.sh`, with the
  reason in the message — because the next person to add it will have no way to
  know it defeats the overlay.
* **The overlay contract is asserted, not assumed.** If the image is built for a
  live boot, it must carry an `overlay-size` that leaves headroom, or `toram`.
* **A boot that does not reach the diagnostics is itself a gate failure.** The
  evidence disk returning unformatted should fail the VM stage, not be noticed by
  a human two hours later. That is the check that would have turned this session
  from hours into one build cycle.
