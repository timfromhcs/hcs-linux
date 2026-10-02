# HCS Linux — Agent Handoff: current state to stable v3

**Read this document completely before changing anything.** It exists because
this project has a repeated failure mode where each fix looked correct, was
committed, and changed nothing observable. Knowing *why* is most of the value.

> **Credentials.** A GitHub token (`ghp_…`) and a Hugging Face token (`hf_…`)
> were pasted in plain text into this project's chat history. The HF token carries
> `repo.write` and `inference.endpoints.write`. **Rotate both before doing
> anything else.** Do not commit them, do not put them in `.env`, do not pass them
> on.

---

## 0. The one-paragraph state

v2 boots to a real Debian sid system: systemd is PID 1, a live user exists and
autologins, session units are enabled and resolve, the Mesa/Vulkan stack is
complete, and niri 26.04 is built with **software-rendering support patched in**
so it can run on a machine with no GPU. The image is 1.5 GB and boots in a
VirtualBox VM. **It has never been observed drawing a desktop**, and the reason
the desktop is not yet visible has never been observed from inside the guest.

Three things block v2.0. All three are known, in order, with the fix for the first
one already written.

---

## 1. Repo and branch state

| Item | Value |
|---|---|
| Repo | `github.com/timfromhcs/hcs-linux` |
| `main` | merged from `dev` at `0bf200b`, **plus** later commits |
| `dev` | working branch; push here, open a PR to `main` |
| Working tree | should be clean; check `git status` |
| CI | green on `dev` |
| Latest release | **v1.0.1** (there is no v1.1.0 — see §7) |
| Open PRs | none |

**On `main`: it contains unreleased v2 work.** That was a deliberate decision to
merge before verification. The README says so at the top of the file. If you
change that policy, update the README in the same commit.

---

## 2. Exact commands

The build runs in WSL2 on the ext4 filesystem. **Never build in `/mnt/d`** — the
Windows filesystem boundary dominates the runtime.

```bash
# sync the repo into the build root (excludes build products)
sudo rsync -a --exclude target/ --exclude dist/ --exclude .git/ \
    /mnt/d/HCSLINUX/ /home/tim/hcs-build/

# 1. build the base system  (~6 min cold, ~1 min warm)
cd /home/tim/hcs-build
sudo env NIRI_SOFTWARE_RENDERING=1 bash scripts/build_base.sh sid

# 2. build the ISO         (~8 min)
sudo bash scripts/build_iso.sh 2.0.0 amd64 --qa

# copy artefacts back for the host-side QA driver
cp dist/HCS-Linux-2.0.0-qa-amd64.iso dist/SHA256SUMS /mnt/d/HCSLINUX/dist/
```

**`NIRI_SOFTWARE_RENDERING=1` is required** and must be passed to
`build_base.sh` — it is forwarded explicitly through the inner `sudo` to
`fetch_niri.sh`. Without it, niri is built unmodified and rejects software EGL.
The build **echoes the value before using it**, so if you see
`NIRI_SOFTWARE_RENDERING=0` in the log, that is the value that was used.

Everything at once:

```bash
bash scripts/autonomous_loop.sh          # base + ISO + VM boot + capture grading
bash scripts/autonomous_loop.sh --no-vm  # build and verify only
```

`autonomous_loop.sh` rebuilds the base **first, always**, and says why in a
comment: the bootstrap hook runs during the base build, and re-running only
`build_iso.sh` produces a faithfully rebuilt image on top of a base that was
never rebuilt. Three build cycles were lost to exactly that.

---

## 3. Architecture, and why it is this way

### 3.1 Two compositors

```
pick-compositor.sh
├── niri   ← hardware GL present, OR this build supports software rendering
└── labwc  ← no hardware GL AND niri was built unmodified
             (Xvfb + pixman, X11, no GPU anywhere in the chain)
```

Both are in the image. `run-niri.sh` is the product compositor.
`run-labwc.sh` is a fallback for an unmodified niri build.

**The compositor's rendering capability is a property of how it was COMPILED,
not of the machine.** So `fetch_niri.sh` writes
`/usr/share/hcs/session/niri-renderer` at build time with its provenance, and the
session reads that file. Do not probe at boot — that would mean starting niri to
find out, which is the failure being avoided.

### 3.2 The niri software-rendering patch — the central finding

niri v26.04 refuses to run on a software renderer. From `src/backend/tty.rs`:

```rust
// Software EGL devices (e.g., llvmpipe/softpipe) are rejected for now.
ensure!(
    !egl_device.is_software(),
    "software EGL renderers are skipped"
);
```

The `ensure!` fails, the render node is skipped, there is no second fallback, and
the observable result is a compositor that starts and never draws. Upstream
issue #218 documents it.

That limitation is **one unmerged upstream PR** away from not existing:

> **niri-wm/niri#3959 "Software rendering"** — open, mergeable, **1 file,
> +102/−65**, fixes #218, head `bill88t/niri@b0131978`.

It relaxes the assertion to reference `node` and `self.primary_node`, disables
dma-buf and DRM leasing when a software renderer is active, and threads a
`primary_renderer_is_software` flag through the backend.

**Verified to apply to v26.04** — this is the check that made it safe to act:

```
ensure! at line 787
enclosing fn: device_added (line 736)
  744:  let node = DrmNode::from_dev_id(device_id)?;
  746:  if node == self.primary_node {
```

Both identifiers are bound before the `ensure!`, in the same function. niri's
`main` is 165 commits past the v26.04 tag, so this was genuinely open — do not
assume it without re-checking if niri is upgraded.

The patch is applied by `fetch_niri.sh` as an in-place edit with asserted
preconditions, **not** a blob diff, and it is idempotent:

```
ensure!(
    egl_device.is_software() && node != self.primary_node,
    "software EGL renderers are skipped on non-primary nodes"
);
```

**Two things that will bite you if you re-derive this:**

1. **No parentheses around the macro condition.** A macro argument that *starts*
   with `(` is ambiguous to the Rust parser — it reports *"no rules expected … while
   trying to match sequence start"* at the string literal, three tokens away from
   the real fault. The error is byte-identical whether or not you add them.
2. **The comma after the condition is required.**

Escape hatch: `NIRI_SOFTWARE_RENDERING=0` builds the unmodified release. It
**defaults to 0** — deliberately. Defaulting it on meant a patch failure took down
the base build twice, at the stage where every other gate is supposed to be
trustworthy. Enable it deliberately; it is now known to work.

### 3.3 Base and live-boot

Debian **sid**, debootstrapped by `scripts/build_base.sh`. sid is not a
preference: quickshell 0.3.1 requires Qt 6.11.2 with an exact private ABI, and
trixie ships Qt 6.8.

The image boots via `live-boot` with `boot=live` and `union=overlay` (the
default). `overlay-size=2g` is explicit because it otherwise claims 50 % of RAM.

---

## 4. Verified vs not verified

### Verified

| Claim | How |
|---|---|
| Base builds: 37,288 files, Debian sid | `build_base.sh` |
| niri 26.04 built with software rendering | `NIRI_SOFTWARE_RENDERING=1`, compiles |
| ISO builds, 187 payload checks pass | `build_iso.sh` |
| Boot completes to a login shell | `hcs@hcs-live:~$` observed once, ~700 s |
| `hcs` autologins | getty@tty1 drop-in, asserted at build time |
| systemd is PID 1 | `/sbin/init -> ../lib/systemd/systemd`, gated |
| Bootstrap hook runs and creates `hcs` | asserted after it runs |
| Session units enabled and resolving | resolved **in the guest namespace** |
| 9 Vulkan ICDs, GBM, EGL, llvmpipe, lavapipe | gated |
| Graph package list has no Mesa/Vulkan | fixed, gated |

### Not verified — do not claim these

| Claim | State |
|---|---|
| **A desktop is drawn** | **never observed** |
| Boot time acceptable for automation | ~700 s and non-deterministic |
| Why the desktop does not appear | **unknown** |
| 32-stage QA suite | never run |
| Local AI inference | `placeholder.gguf` is not a model |
| GPU hardware | no physical machine; WSLg has no `/dev/dri` |

---

## 5. The three blockers, in order

### Blocker 1 — the VM boot is slow, and that is why nothing is measurable

The image is 1.5 GB read over an **emulated optical device**. Boots take ~700 s
and sometimes do not finish. This has been misread as a hang repeatedly.

**Diagnosis is complete; the fix is one command.**

Diagnosed by reading the image, not by trial:

```
MBR signature at 0x1FE:  55 aa        ← present
El Torito boot record:    0x8800      ← present
Partition entry at 0x1BE:
  00 00 02 00 ee 19 d8 7d 01 00 00 00 37 eb 1b 00
  └┬┘ └┬┘ └┬┘
   │   │   └── type 0xEE = GPT-protective: "this is GPT, read the header"
   │   └────── boot flag 0x00
   └────────── 0x00
```

The image **is** hybrid. What it lacks is a partition entry the firmware will
accept. Type `0xEE` tells the firmware to read a GPT header that an ISO9660 image
does not have. Setting the boot flag to `0x80` was tried and the firmware still
reported *"No bootable medium found"* — correctly, because the flag is not what
it consults first.

**The fix** — one of:

```bash
# install the syslinux MBR template, then apply it after grub-mkrescue
sudo apt-get install -y syslinux
xorriso -indev "$ISO" -outdev "$ISO.tmp" \
        -boot_image any mbr_file=/usr/lib/syslinux/isohdpfx.bin
# or build directly:
xorriso -as mkisofs -o out.iso -isohybrid-mbr /path/isohdpfx.bin <staging>
```

Then attach the ISO to a **hard-disk slot** in VirtualBox, not a DVD slot:

```powershell
& $vbox storageattach $vm --storagectl SATA --port 0 --device 0 --type hdd --medium $iso
& $vbox modifyvm $vm --bootorder hdd
```

Attach `evidence.vhd` to port 1, and **detach before `unregistervm --delete`** —
`--delete` destroys the evidence.

**Mitigation already in place:** the heartbeat (§7). It distinguishes slow from
stopped without the fast path, which is why it was built first.

### Blocker 2 — the desktop's failure has never been observed

`hcs-diagnostics.service` writes `dmesg`, failed units, the journal, the DRM
device list, the modes each card offers, the active VT and seat state to the
evidence VHD as FAT32, which Windows mounts natively. It depends on no Guest
Additions, no network and no graphics.

It has never run, because the boot never reached the point where it would. The
evidence disk has come back with **no filesystem** on every run, which is the
only reason you know that.

**The moment the boot completes, read the disk:**

```bash
wsl --mount --vhd <path>/evidence.vhd --bare
# inspect: /mnt/wslg/... or via lsblk, then read the diag-* directory
wsl --unmount <path>/evidence.vhd
```

It contains `SUMMARY.txt` (start with this), `heartbeat`, `compositor.info`,
`dmesg.txt`, `systemd-failed.txt`, `journal-hcs.txt`, `graphics.txt`.

Note `Mount-DiskImage` fails without admin — use `wsl --mount --vhd`, which works.

### Blocker 3 — the QA agent cannot judge an X11 fallback, and must not confuse renderers

`grim` screenshots Wayland. labwc on Xvfb is X11. A fallback run therefore reports
"capture failed", which reads as a desktop failure rather than a tooling
mismatch. ImageMagick (`import -window root`) and `xdotool` are in the package
list; the agent has no branch for them.

And: **a frame captured under llvmpipe is valid proof a desktop drew, and is
never proof of performance.** `run-niri.sh` now measures the renderer rather than
asserting `hardware`, and the QA report must present software and hardware
results as different measurements. This is gated.

---

## 6. Bug log — every real bug, and the gate that now catches it

These are all in `verify_payload.sh` or `build_base.sh`. If you remove one of
these checks, you are re-opening a bug that shipped before.

| # | Bug | Gate |
|---|---|---|
| 1 | The ISO was a 152-file directory with the **build host's** kernel | system probe in `build_base.sh` |
| 2 | `/sbin/init` overridden; systemd was never PID 1 | must be a symlink to systemd |
| 3 | **No graphics packages at all** | GBM/EGL/llvmpipe/ICD/lavapipe checks |
| 4 | **The bootstrap hook never ran** | hook must produce `hcs` + a correct getty override, or the build aborts |
| 5 | `seatd` looked for at `/usr/bin/seatd` | Debian ships `seatd-launch`; script now asks systemd |
| 6 | Session unit enabled by a **dangling symlink** | must resolve **in the guest namespace** |
| 7 | Autologin on `tty6` while GRUB passes `console=tty1` | getty override on tty1 |
| 8 | Diagnostics ordered **after** the thing it diagnoses | now `After=basic.target`, long-lived |
| 9 | GRUB menu drawn despite `timeout=0` | `menu_timeout`/`terminal_output` set |
| 10 | Renderer forced to software, which niri rejects | conditional-force check |
| 11 | `niri-renderer` written only to the discarded source tree | marker must exist in the image, cite provenance |
| 12 | `sudo` reset the env; `NIRI_SOFTWARE_RENDERING=1` never arrived | forwarded explicitly and echoed |
| 13 | Bare `toram` with unset `overlay-size` | fails the build |

Two more are the ones that mattered most, and one does not exist yet:

* **A boot reaching a login shell is a gate separate from "a desktop was drawn."**
  Two milestones, two verdicts.
* **An unformatted evidence disk must fail the VM stage.** *Not yet implemented —
  this is the single highest-value missing gate. It would have turned months of
  debugging into one build cycle.*

---

## 7. Traps — the pattern behind every wrong turn

**Every mistake in this project had the same shape: a plausible story, one check,
no cross-check.** They are listed because they will recur:

* **Three fixes were applied to a hook that was never executed.** Each was correct
  on its own terms and changed nothing observable — the worst failure mode,
  because it looks like progress.
* **A missing-module theory rested on a grep returning 0**, which on re-running
  returned 3 matches. `overlay not supported` comes from live-boot's *shutdown*
  hook, not from a mount failure.
* **`toram` was removed and committed as the fix before the removal was tested in
  isolation.** The disproving screenshot arrived after the commit.
* **A screenshot was read as "stopped"** when it meant "no progress in that
  interval". A screenshot is one instant; it cannot measure time. This is why the
  heartbeat exists.
* **The plan recommended attaching the ISO to a disk** before anyone checked that
  an ISO9660 image has no usable partition table for that.

**Rules that follow:**

1. Read the source or the upstream tracker **before** designing around a
   constraint. A Tier-S check costs minutes; a Tier-D discovery costs a
   20-minute boot cycle.
2. Test a change **in isolation** before building on it.
3. A fix that produces no observable change in a build log is a hypothesis, not
   a fix.
4. Every fix gets a gate that fails when the condition returns.
5. Quote documentation verbatim. `live-boot(7)` says `toram` copies "the whole
   read-only media" and that `overlay-size` "has currently no effect when booting
   with toram". Both were load-bearing and both were half-remembered.

Two corrections are kept in public rather than deleted, in
`docs/V2_STABLE_V2_PLAN.md` §6.

---

## 8. Path to stable v3

Each phase has an exit criterion that **can fail**. Do not skip ahead.

### Phase 0 — unblock measurement *(do this first)*

| Task | Exit criterion |
|---|---|
| Fix the hybrid MBR (§5 blocker 1) | ISO boots from a **hard-disk slot** |
| Run the QA driver | login shell **< 300 s, deterministic** |
| Read the evidence VHD | **FAT32 present**; `SUMMARY.txt` readable |
| Read the heartbeat | the gate can say "still booting" vs "hung" |

If the heartbeat advances then stops, **its timestamp is the answer** — a bounded
window that no previous run produced.

### Phase 1 — one compositor

`NIRI_SOFTWARE_RENDERING=1` already builds and records
`capability=software`. Once a screenshot exists from the **niri** path, delete the
labwc fallback — one compositor is the goal. Until then keep it.

### Phase 2 — the desktop, and the QA agent that can judge it

| Task | Exit criterion |
|---|---|
| X11 capture branch in `hcs-qa-agent` | no "capture failed" on either compositor |
| Renderer recorded, never asserted | software frames never quoted as hardware |
| The Neural Glass shell draws | screenshot with taskbar and Start menu |
| `grade_capture.py` | host-side **GREEN** |
| All 12 GUI apps open as windows | each photographed |
| Four boot cohorts | `live`, `installer`, `installed`, `amnesic`, each its own verdict |

**Add the missing gate:** an unformatted evidence disk fails the VM stage.

### Phase 3 — the product that is not a desktop

| Item | State |
|---|---|
| A real licensed, digest-pinned GGUF | `placeholder.gguf` is not a model |
| `hcs-update`, `hcs-persist`, `hcs-recall` | QML over mock data |
| `hcs-shot`, `hcs-term`, `hcs-notes` | QML over mock data |
| `hcs-agent-bridge` | in the master plan, never built — build it or delete it |
| Calamares installer GUI | ships, never verified |

### Phase 4 — reproducibility and supply chain

| Task | Why |
|---|---|
| Pin sid to a snapshot | sid is rolling; today's build is not tomorrow's |
| Real pins in `sources.lock.yaml` | still holds placeholders |
| Regenerate SBOM + third-party notices from the image | by hand is not evidence |
| `verify_package_availability.py --base debian:sid` | still targets trixie; it has been hiding drift |
| Name `bill88t/niri@b0131978` in the provenance record | the patch pins a fork |

### Phase 5 — release

* Every gate green on a tag, **no `continue-on-error`** in the release path.
* README rewritten **from the gates**. A capability appears only if a gate
  verifies it.
* Resolve the `v1.1.0` link (§9).
* Performance measured, not asserted: frame time, RAM, boot-to-pixel — hardware
  and software recorded separately.

### v3 scope beyond v2

v3 should be **the release that ships a desktop**, with the compositors settled,
the model real, the daemons finished, and the QA suite green on all four
cohorts. Do not add features before Phase 2 exits — nothing after it is
measurable, and that has been the binding constraint for the entire project.

---

## 9. Known documentation defects

* **The README's download link pointed at `releases/tag/v1.1.0`, which does not
  exist.** Corrected to v1.0.1. Check whether the CI/CD release is published
  under a different tag before assuming v1.0.1 is right.
* `docs/V2_NEXT_STEPS_PLAN.md` and `docs/V2_VIRTUALBOX_AND_VULKAN_PLAN.md`
  contain claims that were later **falsified** — specifically that lavapipe is a
  guaranteed floor and that llvmpipe is the QA answer. Superseded by this
  document and by `docs/V2_GPU_VM_RESEARCH_PLAN.md`.

---

## 10. Reference files

| File | What it is |
|---|---|
| **`docs/V2_STABLE_V2_PLAN.md`** | every method ranked, every attempt logged, phases to stable |
| `docs/V2_STATUS_AND_FIX_PLAN.md` | status, all attempts including the wrong ones |
| `docs/V2_GPU_VM_RESEARCH_PLAN.md` | the niri software-rendering research and options |
| `docs/V2_EXECUTION_PLAN.md` | phase ordering |
| `docs/V2_BOOT_STALL_FIX_PLAN.md` | the boot, including the corrected toram diagnosis |
| `docs/V2_STABLE_QA_STATUS.md` | measured state with evidence |
| `scripts/autonomous_loop.sh` | one-command build + boot + grade |
| `scripts/fetch_niri.sh` | niri build **and the #3959 patch** |
| `scripts/verify_payload.sh` | ~40 gates; read it to understand the invariants |
| `scripts/build_base.sh` | debootstrap, hook invocation, assertions |
| `scripts/build_iso.sh` | payload, GRUB, ISO, boot flag |
| `scripts/qa_virtualbox_v2.ps1` | the QA driver |
| `config/includes.chroot/usr/share/hcs/session/` | `pick-compositor.sh`, `run-niri.sh`, `run-labwc.sh`, `boot-diagnostics.sh` |
| `config/hooks/live/01-hcs-setup.hook.chroot` | live user, getty, keyboard — **runs during the base build** |

---

## 11. First three actions for a new agent

1. **Rotate both tokens.** They are in plain text in this project's chat history.
2. **Run the build with `NIRI_SOFTWARE_RENDERING=1`** and confirm 187 checks pass
   and the log says `recorded: niri can render without a GPU`.
3. **Fix the hybrid MBR** (§5 blocker 1) and boot from a hard-disk slot. That
   single change should produce the first evidence disk with a filesystem on it,
   and `SUMMARY.txt` is the answer to a question this project has never been able
   to ask.
