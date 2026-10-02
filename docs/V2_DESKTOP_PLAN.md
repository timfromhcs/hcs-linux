# HCS Linux 2.0 — The plan to a working desktop

Written after a repository audit, upstream research, and the run that finally
located the remaining blocker. Researched claims are cited; anything not
established is marked as such.

Repo state at time of writing: `main` and `dev` both at `0bf200b` (merged).

---

## Part 1 — What was actually blocking, and what is now settled

### 1.1 Settled by research, not by argument

| Question | Answer | Source |
|---|---|---|
| Can niri render without a GPU? | **Not in v26.04.** An `ensure!` rejects software EGL, no second fallback | `src/backend/tty.rs` |
| Is that permanent? | **No.** One open upstream PR | niri-wm/niri#3959 |
| Does that PR apply to v26.04? | **Yes** — verified below | this session |
| Can this host pass GPU acceleration through? | **No.** WSLg has no `/dev/dri`; vGPU behind a Parsec virtual display adapter | `Win32_VideoController`, `/dev/dri` |
| Does VirtualBox offer a modern DRM device here? | **No.** `--graphicscontroller virtio` is rejected; only VMSVGA | `VBoxManage` |
| Is Guest Additions redistributable? | **No.** Personal/test use only | Oracle licence |
| Is `union=aufs` an alternative? | **No.** aufs left the kernel in 5.18; this is 7.2.8 | `live-boot(7)`, kernel |
| Is bare `toram` a fix? | **No.** The message appears with and without it | two runs, this session |

### 1.2 The verification that changes the plan

The blocker for #3959 was: *does the diff apply to v26.04, or only to main?*
niri main is 165 commits past the v26.04 tag, so this was genuinely open.

The PR relaxes the assertion to reference two identifiers:

```rust
ensure!(!is_software || node == self.primary_node, ...)
```

Both must be in scope. Checked against the tagged source:

```
ensure! at line: 787
enclosing fn:    device_added  (line 736)
  744:  let node = DrmNode::from_dev_id(device_id)?;
  746:  if node == self.primary_node {
```

**Both are bound before line 787, inside the same function.** The change is
mechanically applicable to v26.04.

The two failed patch attempts earlier were **my own hand-rolled regex**, not
upstream incompatibility:

1. it dropped the comma after the macro condition, so the file did not parse
2. the fix for that — wrapping the condition in parentheses — hit the known Rust
   macro ambiguity, where an argument beginning with `(` is parsed as the start
   of a sequence. The error was byte-identical and still pointed at the string.

Neither is a reason to doubt the patch. The correct fix is the current one:
`ensure!(egl_device.is_software() && node != self.primary_node, "…")` — no
parentheses, comma present.

**So the remaining work on this is one build cycle, not an open question.**

---

## Part 2 — The best path, and what to do if it fails

Each step has a decision point and a named fallback. No step is a dead end.

### 2.1 Step 1 — make the boot fast and observable

**Why first:** without it, nothing after this is measurable. Every previous
diagnosis failed on the same ambiguity — a screenshot cannot tell a slow boot
from a stopped one.

Already implemented on `dev`:
* the QA driver attaches the ISO to a **hard-disk slot**, not a DVD slot
* `hcs-diagnostics.service` is **long-lived** and writes a heartbeat to the
  evidence VHD every 30 s

**Do next:** run it. Read the heartbeat and the console output.

| Outcome | Meaning | Next |
|---|---|---|
| Login shell in < 300 s, heartbeat advancing | Step 1 succeeded | go to Step 2 |
| Login shell > 300 s but heartbeat advancing | still slow, still working | Step 1c, then Step 2 |
| Heartbeat present then stops | a specific point of failure — **the answer we have never had** | Step 1d |

**1c — if still slow:** shrink the image. Move out of the *live* cohort:
`ffmpeg`, `fonts-noto-cjk` (very large), `calamares` (installer only),
`tesseract-ocr{,-eng,-deu}` (QA only). Keep the firmware families.

**1d — if the heartbeat stops:** the timestamp in the file is exactly *when* it
stopped. That converts a vague stall into a bounded window, and the console log
around it usually names the unit. **This is the single most valuable piece of
information still uncollected.**

**Fallback if the heartbeat mechanism itself fails:** the session already writes
every log line to `/dev/console`, which a screenshot reads. That is the last
resort and it works, because a login shell was reached once — which proves the
console path is live.

### 2.2 Step 2 — apply niri #3959

**Why this is the best path:** it converts a two-compositor architecture into
one. Today the image ships niri *and* labwc, because a machine with no GPU cannot
run niri and the QA VM is such a machine. With #3959 applied, niri renders under
llvmpipe — confirmed by the PR author on ARM (including blur) and by a second
report on Hyper-V/Windows 11, which is exactly this host's configuration — and the
fallback stops being needed.

```bash
NIRI_SOFTWARE_RENDERING=1 bash scripts/build_base.sh sid
```

**Currently defaults to OFF.** That was a deliberate reversal: defaulting it on
meant a patch failure took down the base build twice, at the stage where every
other gate is supposed to be trustworthy. §1.2 establishes the patch is
applicable, so the flag can now be flipped deliberately.

**Decision tree:**

| Outcome | Action |
|---|---|
| Builds, niri runs, shell draws | **one compositor. Delete the labwc path.** |
| Builds, niri runs, no shell | shell problem, not compositor — `start-session.sh` / Quickshell on Wayland |
| Fails to apply (context mismatch) | apply the full upstream diff with `git apply`, resolving by hand; scope is confirmed so this is mechanical |
| Fails to compile for another reason | `NIRI_SOFTWARE_RENDERING=0`, keep labwc, and raise it upstream — it is 1 file and already mergeable |

**Fallback if the patch is abandoned:** keep the labwc fallback, which works.
The cost is real but bounded: two compositors, documented as two, and the QA
gate explicitly labelled as not describing the shipped compositor.

**Alternative worth considering, not first:** build niri from `main` (165 commits
past the tag) rather than patching the tag. `main` has 165 commits of upstream
work, and the patch is "mergeable", so it may land there soon. But that trades a
verified release for a moving branch — a worse trade for a shipped image.

### 2.3 Step 3 — get the QA agent correct for the renderer it will see

Two issues, both better fixed **before** the first successful run than after:

1. **`grim` screenshots Wayland.** If the labwc fallback is ever used, grim fails
   on every capture and the run reports "capture failed" — which reads as a
   desktop failure rather than a tooling mismatch. Needs an ImageMagick
   `import -window root` path, branched on `compositor.info`. ImageMagick and
   xdotool are already in the package list.
2. **The renderer must be recorded, not assumed.** A frame captured under
   llvmpipe is valid proof a desktop drew, and is **never** proof of performance.
   `run-niri.sh` now measures this rather than hardcoding `hardware`; the QA
   report must present software and hardware results as different measurements.
   **A software-rendered screenshot must never be quoted as evidence of GPU
   performance.**

### 2.4 Step 4 — the desktop itself

| # | Step | Acceptance |
|---|---|---|
| 1 | `/run/hcs/compositor.info` present | names the compositor, renderer, and why |
| 2 | niri acquires a VT and maps a window | `niri msg --json windows` non-empty |
| 3 | The Neural Glass shell draws | screenshot with the taskbar and Start menu visible |
| 4 | `grade_capture.py` accepts the frame | host-side GREEN |
| 5 | Each GUI app opens as a window | all 12 apps |
| 6 | Four boot cohorts | `live`, `installer`, `installed`, `amnesic`, each with its own verdict |

### 2.5 Step 5 — the honest performance gate

Once a frame exists, performance has to be measured rather than asserted:

* frame time under the reference scenes, **recorded separately for hardware and
  llvmpipe**
* RAM per app against the existing 250 MB gate
* cold boot to first pixel as a tracked number

A pass on llvmpipe satisfies the *drawing* gate and nothing else. The gate must be
structured so that is not over-readable.

---

## Part 3 — Everything still outstanding, in dependency order

| Item | Depends on | Notes |
|---|---|---|
| Desktop in a VM | Steps 1–4 | the open claim |
| 32-stage QA suite | desktop | four cohorts, not one session |
| A real starter model | nothing | `placeholder.gguf` is not a model |
| `hcs-agent-bridge` | decision | in the master plan, never built |
| `hcs-update`, `hcs-persist`, `hcs-recall`, `hcs-shot`, `hcs-term`, `hcs-notes` | desktop | QML over mock data |
| sid → snapshot pin | nothing | rolling sid is not reproducible |
| `sources.lock.yaml` | nothing | still holds placeholder pins |
| SBOM + third-party notices | image | regenerate from the built image |
| `verify_package_availability.py` | nothing | still targets trixie, not sid |

**On the fork decision:** applying #3959 means pinning
`bill88t/niri@b0131978` rather than the upstream tag, and the supply-chain record
must name the fork and the commit. That is a real cost. It is also strictly
cheaper than shipping two compositors forever, and the patch is small, reviewed
and mergeable — so the cost is temporary and bounded, while the two-compositor
cost is neither.

---

## Part 4 — What would make this session's mistakes impossible to repeat

The pattern behind every wrong turn this session was: **a plausible story, one
check, no cross-check.** Three fixes were applied to a hook that was never
executed; a missing-module theory rested on a grep returning zero; `toram` was
removed and committed as a fix before the removal was tested in isolation.

Gates now in place, one per bug:

* bare `toram` with unset `overlay-size` fails the build
* `run-niri.sh` may not force a software renderer; `run-labwc.sh` must
* session unit enablement must resolve **in the guest namespace** (`test -e` on an
  absolute unit path checks the host's root and calls a correct link broken)
* `niri-renderer` must exist, name a known capability, and cite its provenance
* the renderer kind must be measured, never asserted
* the bootstrap hook must produce the `hcs` user and a correct getty override, or
  the build aborts
* a boot that reaches a login shell is a gate **separate** from "a desktop was
  drawn" — two milestones, two verdicts

**The one still missing, and the most valuable:** the VM gate must distinguish
slow from stopped via the heartbeat, and must fail when the evidence disk comes
back unformatted rather than leaving that for a human to notice hours later.

---

## Part 5 — Definition of done

1. The QA driver attaches the ISO to a disk and boots in under 300 s,
   deterministically.
2. The heartbeat advances, so the gate can say "still booting" rather than
   guessing.
3. `/run/hcs/compositor.info` names the compositor and the renderer honestly.
4. **A screenshot exists in which the Neural Glass shell is drawn**, graded GREEN
   by the host-side checker.
5. `niri#3959` applied or explicitly abandoned, with the reason recorded.
6. All four boot cohorts return their own verdict.
7. A real licensed model produces real output.
8. Every gate green on a tag, with no `continue-on-error` in the release path.
9. The README states only what the gates verify, and links only to things that
   exist.
