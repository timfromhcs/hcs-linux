# HCS Linux 2.0 — Plan

Revised after a repository audit and after checking upstream niri. The headline
is that the architecture changes: **niri can have software rendering, one
upstream PR away, and we already build niri from source.**

---

## Part 0 — Repository audit

| Item | State |
|---|---|
| Working tree | clean |
| `dev` | `debf21e`, **6 commits ahead of `main`** |
| `main` | `b66e69b` |
| CI on `dev` | green (latest run, `debf21e`) |
| Open PRs | **none** — PR #5 was merged and auto-closed |
| Latest release | **v1.0.1**. There is no `v1.1.0` |

### 0.1 Two problems in the repository itself

**`main` currently contains a known-broken build.** The six commits `dev` is
ahead by include:

* the removal of bare `toram` from every kernel cmdline,
* the removal of the forced software renderer from the niri path,
* the two-compositor session and its gates,
* console logging of the session.

So `main` ships an ISO with `LIBGL_ALWAYS_SOFTWARE=1` forced on a compositor that
rejects software EGL, and a boot configuration that is worse than the one already
proven wrong. This is the direct consequence of the earlier decision to merge
`dev` into `main` before the work was verified.

**The README's download link is dead.** It points at
`releases/tag/v1.1.0`; `gh release view v1.1.0` returns *release not found*, and
the newest release is v1.0.1. Either the CI/CD release is published under a
different tag, or the README is wrong. **Not patched silently** — it needs one of
those two answers, and it is not a documentation detail: it is the primary
install instruction.

---

## Part 1 — The finding that changes the architecture

### 1.1 niri's software-rendering ban is one unmerged PR away

Everything in the previous plan rested on niri being unable to render in
software. That is true of **v26.04, which is what we pin** — but the ban is not a
design commitment. It is a single `ensure!` that upstream is actively discussing
removing.

**PR niri-wm/niri#3959, "Software rendering"**

| Property | Value |
|---|---|
| State | **open**, unmerged, `mergeable: true` |
| Size | **1 file, +102 / −65** |
| Head | `bill88t/niri@b0131978` |
| Last activity | 2026-10-01 (yesterday) |
| Fixes | issue #218 — *"software renderer"*, the canonical report of this limitation |

The change, from the diff:

```rust
 let egl_device = EGLDevice::device_for_display(&display)?;
// was: ensure!(!egl_device.is_software(), "software EGL renderers are skipped");
+    let is_software = egl_device.is_software();
+    ensure!(
+        !is_software || node == self.primary_node,
```

with dma-buf and DRM leasing disabled when a software renderer is active, and a
`primary_renderer_is_software` flag threaded through the backend.

The author's description:

> Seriously, this is all it takes for software rendering. It removes the
> arbitrary prohibition. Disables dma-buf. Disables DRM leasing. Leaves a nice
> flag behind other unsafe things may latch onto in the future.

Maintainer review, `YaLTeR`, 2026-05-02:

> Change looks fairly reasonable. I wouldn't mind if someone more knowledgeable
> like @cmeissl would take a look too though

**Two independent confirmations that it works**, which is what makes this more than
a proposal:

* the author: an ARM Orion O6N board with llvmpipe — *"Every kind of app works,
  and all effects, including blur, also work."*
* `longtimejones`, 2026-05-15: *"This works on Hyper-V 10.0.26100.7019 on Windows
  11 Pro 25H2."*

That second one matters specifically here: **Hyper-V on Windows is this host's
situation** — a virtual machine with no GPU passthrough. The exact configuration
that makes niri unusable today is the one that was reported working.

Also note the author's reasoning for why the ban is wrong on the merits:

> It allows usage in VMs that lack passthrough graphics, servers, legacy and
> experimental systems. Gating it when it very clearly works properly just isn't
> making sense.

### 1.2 Why this matters more than any other finding here

We already build niri from source. `scripts/fetch_niri.sh` downloads a
hash-pinned source and vendor bundle, verifies both digests, builds, and installs.
Applying one patch to one file before `cargo build` is a small, contained change
to a script that already exists.

The consequences of doing so:

| Without the patch | With the patch |
|---|---|
| Two compositors ship: niri and labwc | **One compositor: niri** |
| A machine with no GPU gets a fallback WM that is not the product | **The product compositor everywhere** |
| The README must explain which compositor ran | One story |
| "CPU-first" is contradicted by niri's own design | "CPU-first" becomes true |
| QA and the shipped image differ structurally | They differ in configuration only |

This is the difference between *shipping two desktops and documenting the split*
and *shipping one desktop*. It should be the primary path, with the labwc
fallback retained only until this is proven.

### 1.3 What the patch does **not** do, and the honesty requirement

It does not make niri fast. llvmpipe is a CPU rasteriser; a full desktop in a VM
will be slow and some effects will cost real time. It makes niri **draw**.

So the README must not say "renders fast without a GPU". It should say:

> The desktop runs without a discrete GPU. On systems with no hardware
> acceleration it uses software rendering, which is correct but slower.

And a gate must distinguish the two: a run on llvmpipe is a valid desktop and a
valid capture, but it is **not** evidence of hardware performance. Frame time on
llvmpipe says nothing about frame time on an AMD GPU, and the two must never be
reported as the same measurement.

### 1.4 The risk, stated plainly

This is an **unmerged upstream PR**. Building on it means:

* pinning a fork (`bill88t/niri@b0131978`) rather than the upstream release, so
  the supply-chain record must name the fork and the commit;
* carrying a patch that upstream may change or reject;
* owning the consequences if it breaks something upstream later mitigates.

The mitigations are real: it is 1 file, it is mergeable, the maintainer has
reviewed it favourably, it is being actively triaged in the `area:session` label,
and two people have run it. And the alternative — shipping two compositors
forever — is a permanent cost for a temporary one.

**Decision needed from you.** My recommendation is: **apply it, behind a
build-time flag, defaulting on, and keep labwc as a fallback until a real
screenshot exists from the niri path.** If it fails, the flag turns it off and
we are no worse off than now.

---

## Part 2 — The boot, which blocks every measurement

Independent of §1, and still the reason no desktop has ever been photographed.

### 2.1 Diagnosis

The image is 1.5 GB, read over an **emulated optical device**. live-boot mounts
and verifies the squashfs from that medium and the session then reads from it.

Evidence that this is slowness and not a hang:

| Run | `toram` | Result |
|---|---|---|
| `v-02`, `b-02` | no | reached getty |
| `HCS-Fix` | no | **reached `hcs@hcs-live:~$` at ~700 s** |
| `HCS-Log` | no | frozen at 650 s |
| `f-08`, `g-09`, `x-07` | yes | `overlay not supported` |

`overlay not supported` appears **with and without** `toram` and is not fatal. One
run got through; another did not, same image family. That is a boot sitting on the
edge of the observation window.

Three mistakes produced the confusion, all the same shape — a plausible story,
one check, no cross-check:

1. Screenshots cannot measure time, so a 700 s boot looks like a hang at 320 s.
2. "Frame unchanged between two samples" was read as "stopped", three times. It
   means "no progress *in that interval*".
3. `toram` was removed and committed as the fix before the removal was tested in
   isolation. The disproving screenshot arrived after the commit.

### 2.2 Step 1 — attach the ISO as a hard disk, not a DVD

Largest single win, and a change to the QA driver rather than the image.

```powershell
& $vbox storageattach $vm --storagectl SATA --port 1 --device 0 --type hdd --medium $iso
& $vbox modifyvm $vm --bootorder hdd
```

Two reasons it is right rather than merely fast:

* The medium becomes a normal read-only block device, which is **closer to what
  a real USB stick presents** than an ISO9660 DVD. The QA result becomes more
  representative, not just quicker.
* Combined with `live-media=/dev/sda` on the kernel cmdline, live-boot stops
  scanning every block device for `/live` — documented in `live-boot(7)`, and it
  makes the boot deterministic instead of merely slow.

**Acceptance:** login shell in under 300 s, on every run, not most runs.

### 2.3 Step 2 — a heartbeat, so "slow" and "stopped" are distinguishable

The absent gate that cost this session. Three options, in increasing order of
reliability:

1. **`debug` on the cmdline** — live-boot and systemd both emit per-unit
   timestamps, so a single frame carries progress. Free.
2. **Guest heartbeat file** — `hcs-diagnostics.service` rewrites a timestamp on
   the evidence VHD every 30 s; the host polls mtime via `wsl --mount --vhd`.
   Works on the X11 path too, where there is no Wayland counter.
3. **Render counter** — the session writes a frame count to the evidence disk.
   Increasing proves the compositor is alive and drawing.

**Acceptance:** the VM gate reports "still booting" versus "hung", rather than
inferring either from frame equality.

### 2.4 Step 3 — shrink the image

1.5 GB is paid on every read. Move out of the *live* cohort:

| Package | Class | Needed to reach a desktop? |
|---|---|---|
| `ffmpeg` | very large | no |
| `fonts-noto-cjk` | very large | only for CJK |
| `calamares` | large | installer cohort only |
| `tesseract-ocr{,-eng,-deu}` | moderate | QA agent only |
| firmware families | moderate | **keep** |

### 2.5 Step 4 — replace live-boot, only if 2.2–2.4 are insufficient

live-boot costs slow device discovery, an overlay over a read-only squashfs, and
no useful diagnostics on failure. The standard alternative is the read-only root
arrangement: mount the squashfs read-only, tmpfs for the overlay upper/work,
overlay, `pivot_root`, exec `/sbin/init` — or mount the squashfs directly as
`root=` with no overlay at all, as many live images do.

**A rewrite, not a tweak. Only after measuring what 2.2–2.4 achieve.**

---

## Part 3 — Reaching a desktop

Ordered so each failure stays diagnosable. §2.2 and §2.3 first: without them,
nothing after this is measurable.

| # | Step | Acceptance |
|---|---|---|
| 1 | ISO as disk (§2.2) | login shell < 300 s, every run |
| 2 | Heartbeat (§2.3) | gate distinguishes slow from stopped |
| 3 | Evidence VHD readable | FAT32 present, mounts on the host |
| 4 | Read `SUMMARY.txt` | **names the real blocker — never yet obtained** |
| 5 | Apply niri #3959, niri path chosen | `/run/hcs/compositor.info` says `niri`, `renderer=software` |
| 6 | niri draws under llvmpipe | shell window visible |
| 7 | `grade_capture.py` accepts the frame | host-side GREEN |
| 8 | labwc path retested | only if kept; fallback until step 7 |
| 9 | Apps as windows | all 12 GUI apps open |
| 10 | Four boot cohorts | `live`, `installer`, `installed`, `amnesic`, each its own verdict |

### 3.1 The QA agent cannot use grim on an X11 fallback

`grim` screenshots Wayland. labwc on Xvfb is X11. So a fallback run needs
`import -window root` (ImageMagick) and `xdotool` for input — both now in the
package list, but the agent has no branch for them.

Without it, every software-rendered run reports "capture failed", and that will be
read as a desktop failure rather than a tooling mismatch. **Worth fixing before
the first successful run.** If niri #3959 works, this becomes a fallback-only
concern rather than the main path — one more reason to try the patch first.

### 3.2 Capture and grading must distinguish renderers

`grade_capture.py` must record whether the frame came from hardware or llvmpipe,
and the QA report must present them as different measurements. A screenshot is
proof that something was drawn; it is never proof of performance.

---

## Part 4 — Order of work

| Priority | Work | Cost | Unblocks |
|---|---|---|---|
| 1 | ISO as disk + `live-media=` (§2.2) | ~1 h | every measurement |
| 2 | Heartbeat (§2.3) | ~1 h | reliable diagnosis |
| 3 | **Decide on niri #3959** (§1.4) | — | the architecture |
| 4 | Read `SUMMARY.txt` | 1 boot | the real blocker |
| 5 | Apply #3959 if approved, rebuild, boot | ~1 day | one-compositor desktop |
| 6 | QA agent X11 branch (§3.1) | ~2 h | honest visual QA |
| 7 | Renderer-aware grading (§3.2) | ~2 h | honest performance claims |
| 8 | Shrink the image (§2.4) | ~1 day | faster still |
| 9 | Apps, cohorts | days | the 32-stage suite |
| 10 | Replace live-boot if needed (§2.5) | days | last resort |

Items 1 and 2 are small, certain, and independent of every open question. They
are what I would do next regardless of how §1.4 is decided.

---

## Part 5 — Gates, so this class of bug cannot recur

Every fix in this session got a gate. That is the only durable improvement.

* **No bare `toram` while `overlay-size` is unset.** Configuration hygiene, not the
  fix for the stall — but correct, and it stops `toram` returning.
* **`run-niri.sh` must not force a software renderer**; `run-labwc.sh` must.
  Both are gated, and the gate caught the wrong default immediately.
* **Session unit enablement must resolve in the guest namespace.** `test -e` on an
  absolute unit path checks the *host's* root and reports a correct link as
  broken. The gate resolves against `ROOTFS_DIR`.
* **A boot that reaches a login shell is a separate gate from "a desktop was
  drawn."** Two milestones, two verdicts, so a slow boot is visibly slow rather
  than invisibly absent. **This is the gate whose absence cost the session.**
* **The VM gate must report progress**, not infer it from frame equality.
* **A renderer claim must be gated on the renderer being hardware.** A desktop on
  llvmpipe is a desktop, but it is never evidence of GPU performance.

---

## Part 6 — Open questions

For you:

1. **Apply niri PR #3959?** My recommendation is yes, behind a build flag,
   defaulting on, keeping labwc until a real niri screenshot exists. It is an
   unmerged upstream PR, so it means pinning a fork and owning the patch — but the
   alternative is shipping two compositors permanently. See §1.4.
2. **`main` is 6 commits behind and contains a known-broken build.** Should it be
   fast-forwarded now, or left until the boot is fast and a desktop is
   photographed? I recommend leaving it and saying so in the README, because a
   `main` that claims more than it can show is the failure mode this project has
   repeated.
3. **v1.1.0.** Does the CI/CD release exist under a different tag, or is the
   README wrong? It is the primary install instruction and it is currently a dead
   link.
4. **A real starter model.** `placeholder.gguf` is not a model, and no gate can
   substitute for one.

For me, to be answered by measurement rather than argument:

* Boot time after §2.2 — is it under 300 s, and is it deterministic?
* What does `SUMMARY.txt` name as the blocker? Never yet obtained.
* Does niri with #3959 draw under llvmpipe in this VM?

---

## Part 7 — Two corrections kept visible

**The `toram` fix was wrong.** Removed and committed as the cause; the message
appears with and without it. Kept off as configuration hygiene, gated against
return, and recorded here rather than deleted.

**"lavapipe as the guaranteed floor" was wrong.** It contradicted niri's source.
Software rendering is not a floor for v26.04; it is a rejected input. §1 is the
correction — and it is a better correction than the original, because it says the
limitation is one unmerged PR away rather than permanent.

Both are kept because the failure they share — one plausible story, one check, no
cross-check — is the failure this project keeps making, and a corrected plan is
worth more than a clean-looking history.
