# HCS Linux 2.0 — The complete plan to a stable release

Everything this project tried, everything it learned, every method available,
ranked from smartest to least. Written so that a reader — or an agent — can pick
up at any point and know exactly what is true, what is believed, and what to do
next.

Repo state: `main` and `dev` merged at `0bf200b`.

---

## Part 0 — The one-paragraph version

v2 boots to a real Debian sid system with systemd as PID 1, a live user, working
session units, a complete Mesa/Vulkan stack and a Wayland compositor. It does not
yet draw a desktop. The reason is now known and is not a mystery: **niri rejects
software rendering by design**, this host is a VM with no GPU passthrough, and
VirtualBox's only device is one Smithay does not support. Every VM run in this
project was a guest with no Guest Additions and therefore no 3D. The fix is one
unmerged upstream PR — niri-wm/niri#3959 — verified to apply to the pinned
release, one file, and reported working by two people including on Hyper-V, which
is this host's own configuration. Build, boot, photograph, grade. Then finish the
product.

---

## Part 1 — Every method, ranked smartest to least

The ranking is by **information gained per unit of cost and risk**. That is the
axis that matters here, because the expensive mistake was not choosing a worse
method — it was spending twenty-minute build-and-boot cycles to learn one fact at
a time.

### Tier S — read the source and the upstream tracker first

| Method | Cost | Why it ranks first |
|---|---|---|
| **Read the compositor's source** | minutes | Would have answered "can niri render in software" definitively on day one, instead of after three build cycles and a wrong plan |
| **Check upstream PR/issue state via API** | seconds | `gh api repos/niri-wm/niri/pulls/3959` returns `state`, `merged`, `changed_files`. The whole architectural answer was one call |
| **Quote the manpage, do not paraphrase it** | minutes | `live-boot(7)` says `toram` copies "the whole read-only media" and that `overlay-size` "has currently no effect when booting with toram". Both were half-remembered and both were load-bearing |
| **Measure the host before designing for it** | minutes | `Win32_VideoController` + `/dev/dri` in WSL would have shown immediately that no GPU passthrough exists, instead of proposing QEMU and Guest Additions as solutions |
| **Verify a diff applies before designing around it** | minutes | Checking that `node` and `self.primary_node` are in scope at line 787 settled the biggest open question in the project |

**The pattern:** every wrong turn in this project came from a Tier-D method being
used where a Tier-S method was available. The information existed; it was not
looked up.

### Tier A — cheap, local, fast, and they were skipped

| Method | Cost | Catches |
|---|---|---|
| `bash -n` on every script | seconds | The class of bug in `build_base.sh` |
| `shellcheck -S warning` | seconds | Quoting, unset variables, namespace mistakes |
| `cargo test --workspace` | minutes | Logic regressions |
| `pytest tests/unit` | seconds | Grader and gate logic |
| `verify_payload.sh` against a staged tree | seconds | Contract violations |
| **A regression test per bug** | minutes | All eight structural bugs had none |
| **Assert a file was *executed*, not merely present** | seconds | The hook that never ran |

The eight real bugs were all structural: an unreached hook, a dangling
enablement, a namespace mismatch, a missing graphics stack. **None was covered by
a test.** All were found by booting a VM and reading a screenshot — twenty
minutes per attempt, one fact at a time.

### Tier B — local reproduction before the hypervisor

| Method | Cost | Why |
|---|---|---|
| **Xvfb + labwc** | seconds | Reproduces "no GPU, software rendering" with no VM at all |
| **WSLg as a Wayland client host** | minutes | Runs the shell and apps without a boot, if `/dev/dri` exists |
| A loop device mounting the squashfs | minutes | Reproduces the overlay without booting |
| `qemu-system-x86_64 -kernel … -initrd …` | minutes | Reproduces initramfs behaviour on Linux, far faster than VirtualBox on Windows |

WSLg was available the whole time and **has no `/dev/dri` on this host**, so it
could not have solved the desktop problem — but it would have answered *why* in
minutes. It was only checked near the end.

### Tier C — the hypervisor, used as a measurement instrument

| Method | Cost | Verdict |
|---|---|---|
| **Attach the ISO to a disk slot, not a DVD** | ~1 h | **Best single change.** Faster *and* more representative — a read-only block device is what a real USB stick presents |
| **Heartbeat on the evidence disk** | ~1 h | **The gate that was missing.** Turns "hung?" into "stopped at T" |
| Log the session to `/dev/console` | ~30 min | Crude, works when nothing else can, already implemented |
| Two verdicts: "reached a login shell" and "drew a desktop" | minutes | Makes a slow boot visibly slow |
| `--uart1` serial console | ~1 h | **Does not work on this host.** `VBoxManage` reports a pointer-sized garbage IRQ for every value 0–255, and drops a hand-edited `<Serial>` element |
| Nested QEMU inside VirtualBox | days | Only if the outer device is unusable |

### Tier D — what was actually done, and why each was the wrong call

Recorded because the failures are the useful part. Every one shares a shape: **a
plausible story, one check, no cross-check.**

| Attempt | Story | What one check showed | Lesson |
|---|---|---|---|
| Three fixes to `01-hcs-setup.hook.chroot` | "the getty override is wrong" | **the hook was never executed** — `build_base.sh` never called it | A file that is never run is not a component |
| `LIBGL_ALWAYS_SOFTWARE=1` as a conservative default | "software rendering is the floor" | niri's `ensure!` rejects it | I wrote the bug, then hunted it for hours |
| Add Mesa packages | "no graphics stack" | true, but not why no desktop | a real gap and a red herring simultaneously |
| `toram` to fix a slow boot | "the medium is slow" | the message appears with and without it | removed, committed as the fix, **the disproving screenshot came later** |
| `overlay not supported` read as fatal | "it stopped there" | same image: login shell at 700 s, frozen at 650 s | it is slow, not hung |
| Missing-module theory | "overlayfs is absent" | re-run grep: **3 matches** | trusted one grep that returned 0 |
| Replace live-boot | "it gives no diagnostics" | true, but the boot had already succeeded once | nearly abandoned a working component |

### Tier E — not attempted, deliberately

| Method | Why not |
|---|---|
| Ship Guest Additions in the ISO | Oracle's licence forbids redistribution |
| `union=aufs` | aufs left the kernel in 5.18; this is 7.2.8 |
| Bare `toram` | documented conflict with the overlay tmpfs |
| Baking an NVIDIA driver | not redistributable, large, must match the kernel ABI |
| A green gate on a renderer nobody uses | worse than a gate that does not run — the exact failure of three prior releases |
| Patching the image to hide a failure | produces a passing gate that describes nothing |

---

## Part 2 — The attempt protocol, for the record

Every VM run, its result, and what it actually established.

| # | Run | Change under test | Result | Established |
|---|---|---|---|---|
| 1 | `AI`, `HCS-Linux-QA-v2` | first real ISO | banner, no desktop | the image boots |
| 2 | `HCS-GPUTest` | 3D accel, virtio attempt | `umugfx: unsupported hypervisor`, frame frozen 20 min, 300 % CPU | VirtualBox has no virtio controller here |
| 3 | `HCS-Verify` / `HCS-Boot` | autologin fix | `Authentication failure` | **the boot completed.** Misread as a login bug for three cycles |
| 4 | `HCS-Render2` | renderer packages | kernel log frozen at 2.6 s | still slow |
| 5 | `HCS-D3`, `HCS-D4` | `toram` | `overlay not supported` | `toram` did not help |
| 6 | `HCS-Fix` | `toram` removed, `overlay-size=2g` | **`hcs@hcs-live:~$` at ~700 s** | the boot works, and is slow |
| 7 | `HCS-Log` | console logging | frozen at 650 s | non-deterministic → marginal, not hung |
| 8 | `HCS-X11` | labwc fallback, toram removed | `overlay not supported` | message is non-fatal |
| 9 | `niri` patch v1 | regex without comma | compile error | failed loudly, as intended |
| 10 | `niri` patch v2 | parentheses "fix" | **identical error** | Rust macro ambiguity: an argument starting with `(` |
| 11 | scope check | verify #3959 vs v26.04 | `node` at 744, `self.primary_node` at 746, `ensure!` at 787 | **the patch applies** |

**Reading the log as a whole:** the project never lacked a working boot. It
lacked a way to tell a working boot from a slow one, and a way to read the
compositor's requirements from anything but a screenshot. Both are now in place.

---

## Part 3 — Phases to a stable v2.0.0

Each phase has an exit criterion that can fail.

### Phase 0 — unblock measurement *(must come first)*

| Task | Exit criterion |
|---|---|
| Run the QA driver with the ISO on a disk | login shell **< 300 s, deterministic** |
| Heartbeat advancing on the evidence disk | gate can report "still booting" |
| Console log readable | every session line reaches `/dev/console` |

If the heartbeat advances then stops, **the timestamp is the answer.** Note it.

### Phase 1 — one compositor

| Task | Exit criterion |
|---|---|
| `NIRI_SOFTWARE_RENDERING=1` build | niri builds with software rendering |
| Boot | `compositor.info` says `niri`, `renderer=software` |
| **If it works** | delete the labwc path — one compositor is the goal |
| **If it does not** | `=0`, keep labwc, raise it upstream (1 file, already mergeable) |

### Phase 2 — the desktop, and the QA agent that can judge it

| Task | Exit criterion |
|---|---|
| X11 capture branch (`import`/`xdotool`) | no "capture failed" on either compositor |
| Renderer recorded, never asserted | a software frame is never quoted as a hardware result |
| Shell draws | a screenshot with the taskbar and Start menu visible |
| `grade_capture.py` | host-side **GREEN** |
| All 12 GUI apps open as windows | each photographed |
| Four boot cohorts | `live`, `installer`, `installed`, `amnesic`, each with its own verdict |

### Phase 3 — the product that is not a desktop

| Task | State |
|---|---|
| A real licensed, digest-pinned GGUF; `placeholder.gguf` removed | `placeholder.gguf` is not a model |
| `hcs-update`, `hcs-persist`, `hcs-recall` | QML over mock data |
| `hcs-shot`, `hcs-term`, `hcs-notes` | QML over mock data |
| `hcs-agent-bridge` | in the master plan, never built — build it or delete it from the plan |
| Calamares installer GUI | ships, never verified |

### Phase 4 — reproducibility and supply chain

| Task | Why |
|---|---|
| Pin sid to a snapshot | sid is rolling; today's build is not tomorrow's |
| Real pins in `sources.lock.yaml` | still holds placeholders |
| Regenerate SBOM + third-party notices from the image | by hand is not evidence |
| `verify_package_availability.py --base debian:sid` | still targets trixie; it has been hiding drift |
| Name the niri fork in the provenance record | #3959 pins `bill88t/niri@b0131978` |

### Phase 5 — release

| Task | Criterion |
|---|---|
| Every gate green on a tag | no `continue-on-error` in the release path |
| README rewritten **from the gates** | a capability appears only if a gate verifies it |
| `v1.1.0` link resolved | it pointed at a release that does not exist; corrected to v1.0.1 |
| Performance measured, not asserted | frame time, RAM, boot-to-pixel — hardware and software recorded separately |

---

## Part 4 — The gates, which are the durable output

One gate per bug. This is what stops the next three releases from shipping
without a compositor:

* bare `toram` with unset `overlay-size` fails the build
* `run-niri.sh` must not force a software renderer; `run-labwc.sh` must
* session unit enablement must resolve **in the guest namespace** — `test -e` on
  an absolute unit path checks the *host's* root and reports a correct link broken
* `niri-renderer` must exist, name a known capability, and cite its provenance
* the renderer kind is measured, never asserted
* the bootstrap hook must produce the `hcs` user and a correct getty override
* `/sbin/init` must be a symlink to systemd, and no GRUB entry may pass `init=`
* 185 payload checks; a violation fails the build

**Two more, and they are the ones that mattered most:**

* **a boot that reaches a login shell is a gate separate from "a desktop was
  drawn"** — two milestones, two verdicts, so a slow boot is visibly slow rather
  than invisibly absent
* **the evidence disk coming back unformatted must fail the VM stage**, not sit
  unnoticed until a human notices hours later. That check alone would have turned
  this session from hours into one cycle.

---

## Part 5 — Definition of done for v2.0.0

1. ISO boots in under 300 s, deterministically, from a disk.
2. The heartbeat advances; the gate can distinguish slow from stopped.
3. A screenshot exists in which the **Neural Glass shell is drawn**, graded GREEN
   by the independent host checker.
4. `niri#3959` applied or explicitly abandoned, with the reason recorded.
5. All four boot cohorts return their own verdict.
6. Every GUI app opens as a window.
7. A real licensed model produces real output on the target machine.
8. sid is snapshot-pinned; SBOM and notices generated from the image.
9. Every gate green on a tag, no `continue-on-error`.
10. The README states only what the gates verify, and links only to things that
    exist.

---

## Part 6 — Corrections kept in public

**The `toram` fix was wrong.** Removed and committed as the cause; the message
appears with and without it. Kept off as configuration hygiene, gated against
return.

**"lavapipe as the guaranteed floor" was wrong.** It contradicted niri's source.
Software rendering is not a floor for v26.04; it is a rejected input. The
correction is better than the original claim, because it says the limitation is
one unmerged PR away rather than permanent.

**The two failed patch attempts were my own regex, not upstream
incompatibility** — a dropped comma, then the Rust macro ambiguity where an
argument beginning with `(` is parsed as a sequence start.

All three are kept rather than deleted. The failure they share — one plausible
story, one check, no cross-check — is the failure this project keeps making, and
a corrected history is worth more than a clean-looking one.
