# HCS Linux 2.0 — The boot stall: researched, and then corrected

## 0. Correction first

I wrote this document asserting that bare `toram` caused the stall, removed
`toram`, and committed that as the fix. **That was wrong.**

With `toram` removed and `overlay-size=2g` set, `overlay not supported` still
appears in the guest console. The gate I added for it is correct as a statement
about the two options being in tension — but it did not fix this stall.

What the same runs also showed:

* `HCS-Fix` (toram removed): **reached a logged-in shell** at ~700 s.
* `HCS-Log` (toram removed, console logging added): **frozen** at 650 s.

Same image family, same VM shape. Different outcome. That is not a stall; it is a
boot that is **slow and marginal**, right at the edge of the observation window.

So the real defect is not a hang. It is a boot that takes ten to twelve minutes
and sometimes does not finish inside it. Every "frozen frame" observation in this
project is a snapshot of that.

I am recording the wrong fix rather than quietly deleting it, because the error
is the useful part: I removed `toram` on a mechanism I had read in a manpage, and
then did not test the removal in isolation before building on top of it. The
screenshot table below looked like confirmation. It was not, because the one run
that disproved it came after the commit.

---

## 1. What the manpage does establish

These parts stand, because they are quoted, not inferred:

> **`toram`** — live-boot will try to **copy the whole read-only media** to the
> computer's RAM before mounting the root filesystem. This could need a lot of
> ram, according to the space used by the read-only media.

> **`overlay-size=SIZE`** — the size of the tmpfs mount used for the upperdir union
> root mount. **By default, 50% of available RAM.** *(No effect when booting with
> toram.)*

So `toram` and a default overlay size *are* in tension. `toram` should stay off,
and `overlay-size` should stay explicit — as configuration hygiene, not as the fix
for this stall.

> **`union=aufs`** is not an option. aufs was removed from the Linux kernel in
> 5.18; this is Debian sid on 7.2.8.

---

## 2. The actual defect: the boot is slow, not hung

### 2.1 The evidence

| Observation | Reading |
|---|---|
| `HCS-Fix` reached `hcs@hcs-live:~$` at ~700 s | the boot **does** complete |
| `HCS-Log` frozen at 650 s, identical frame at 320 s and 650 s | marginal, not deterministic |
| ISO is 1.5 GB, read over an emulated optical device | seconds to tens of seconds per access |

The console output stops right after `squashfs: version 4.0` — live-boot has the
filesystem and is doing slow block I/O to mount and verify it. No error, no
panic, no failed unit. Just very slow progress.

### 2.2 Why this was invisible for so long

Three separate mistakes, all the same shape:

1. **Screenshots cannot measure time.** Every observation is a single frame at an
   arbitrary moment. A boot that takes 700 s looks identical to a boot that hangs
   at 320 s if you only ever look at 320 s.
2. **I kept treating "no change" as "stopped".** A frame-identical pair means "no
   progress *in that interval*", not "no progress".
3. **The fix I tried was justified by a pre-change observation.** See §0.

### 2.3 What would have caught it

* **A heartbeat, not a frame.** The loop needs something the guest emits
  periodically — a timestamp on the console, or the frame counter advancing. A
  VM gate that cannot distinguish "slow" from "stopped" cannot be automated.
* **A longer, fixed observation window** with progress markers, rather than a
  timeout that assumes failure.

---

## 3. The fix, in order of expected effect

### Step 1 — stop reading the ISO over an emulated optical device (largest win)

An emulated DVD is orders of magnitude slower than a block device. The QA driver
should attach the ISO to a **hard-disk slot** and boot from disk. This changes the
medium from ISO9660-over-emulated-optical to a plain read-only block device,
which is also closer to how a real USB stick presents itself.

This is a change to the QA driver, not to the image, and it is the one change most
likely to turn a 700 s boot into a 90 s boot.

### Step 2 — shrink the image

1.5 GB is large for a live image, and the size is paid on every read. The base is
2.4 GB unpacked. Candidates for a separate installer cohort rather than every
boot:

* `ffmpeg` (large, not needed to reach a desktop)
* the non-CJK and CJK font sets (`fonts-noto-cjk` is very large)
* `calamares` — only the `installer` cohort needs it
* the firmware family set

Splitting these out is real work and should be done after Step 1, since Step 1 may
make it unnecessary.

### Step 3 — raise the observation window and add progress markers

The loop must be able to say "still booting" versus "hung". Until it can, every
future diagnosis inherits this same ambiguity.

### Step 4 — keep the software fallback compositor

Unaffected by any of this. niri still rejects software EGL by design; labwc on
Xvfb is still the correct fallback for a machine with no GPU.

---

## 4. Current verified position

| Claim | State |
|---|---|
| Boot completes to a login shell | **PASS** (observed once, ~700 s) |
| `hcs` autologin works | **PASS** (`hcs@hcs-live:~$`) |
| systemd is PID 1 | **PASS** (gated) |
| Bootstrap hook runs, `hcs` created, getty override applied | **PASS** (gated) |
| Session units present and enabled and resolving | **PASS** (gated) |
| Graphics stack present | **PASS** (9 ICDs incl. lavapipe, GBM, EGL, llvmpipe) |
| Software fallback compositor implemented | **PASS** (picked, gated) |
| Console logging of the session | **PASS** (this build) |
| Boot time acceptable for an automated gate | **FAIL** — marginal at 650 s |
| Desktop drawn | **NOT RUN** — not yet reached reliably |
| Boot failure reason for the desktop | **UNKNOWN** — no run has produced it |

---

## 5. Gates to prevent a repeat

* **No bare `toram` while `overlay-size` is unset.** Stands as configuration
  hygiene; it was not this stall.
* **The VM gate must distinguish slow from stopped.** A heartbeat or a progress
  marker, and an observation window sized in minutes rather than seconds. This is
  the gate whose absence caused this session.
* **A boot that reaches a login shell is itself a gate**, distinct from "a desktop
  was drawn". Two milestones, two verdicts, so a slow boot is visibly slow rather
  than invisibly absent.

