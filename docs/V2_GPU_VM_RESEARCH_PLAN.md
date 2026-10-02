# HCS Linux 2.0 — GPU and VM Plan (researched)

Written after reading niri's source, Smithay's tracker, VirtualBox's own manuals
and the WSLg architecture docs. This replaces the guesswork in
`docs/V2_NEXT_STEPS_PLAN.md` and `docs/V2_VIRTUALBOX_AND_VULKAN_PLAN.md` where
they conflict with what I found.

---

## 0. Host inventory — measured, not assumed

```
Parsec Virtual Display Adapter   0.45.0.0
AMD Radeon(TM) Graphics          32.0.21043.12001   4293918720 (4 GB)
```

**There is a real AMD GPU.** `radeonsi` / `radv` are among Mesa's best drivers,
and AMD is explicitly in the supported matrix of both Options B and C below.

This is the single most useful fact in the document, and it arrived after the
plan was drafted — which is the correct order, because it means the plan is
driven by a measurement instead of by an assumption. Three consequences:

1. **Option B (QEMU + `virtio-vga-gl` + SPICE OpenGL) is viable.** The
   configuration reported working in niri #2570 needs a host `rendernode` to
   hand the guest; this host has one.
2. **Option C (WSLg + Mesa D3D12) is viable.** Mesa's D3D12 backend gives WSLg
   hardware-accelerated OpenGL on exactly this class of Windows GPU.
3. **Bare-metal GPU acceptance is no longer impossible here.** `vulkaninfo`
   naming `radeonsi`/`AMD` is testable on this host. That reclassifies a whole
   row from `UNTESTED` to `TESTABLE`.

Also noted: a **Parsec Virtual Display Adapter** is installed. That is a
virtual display, and it means the host may be a remote/VM-hosted session — which
is relevant because HVCI/Memory Integrity is active and VirtualBox reported
`umugfx ... running on an unsupported hypervisor`. Nested virtualisation
explanations and hypervisor-detection failures are the same class of problem.
Whether `nested-hw-virt` can be enabled is a Phase 0 question now.

---

## 1. The finding that changes everything

**niri cannot use a software renderer. By design. From its own source.**

`src/backend/tty.rs`:

```rust
let display = unsafe? EGLDisplay::new(gbm.clone())?;
let egl_device = EGLDevice::device_for_display(&display)?;

// Software EGL devices (e.g., llvmpipe/softpipe) are rejected for now. They have some
// problems (segfault on importing dmabufs from other renderers) and need to be
// excluded from some places like DRM leasing.
ensure!(
    !egl_device.is_software(),
    "software EGL renderers are skipped"
);
```

Upstream issue [#218 "software renderer"](https://github.com/niri-wm/niri/issues/218),
maintainer's answer: *"There's no software rendering support at the moment."*
Referenced by PR #3959 "Software rendering" — which is why it needs checking
whether 26.04 contains it.

Users confirm the observable behaviour in the wild:

```
niri[2238]: DEBUG niri::backend::tty: failed to initialize renderer,
            falling back to primary gpu: software EGL renderers are skipped
```

…and, from the QEMU thread (#2570), a user's summary of the whole situation:

> **niri doesn't support software rendering, you must have 3d acceleration
> turned on to run it in a VM.**
>
> …niri has a hard dependency on OpenGL. It's too bad it doesn't give a better
> error message but instead just hangs.

### 1.1 What this invalidates in my own plan

`docs/V2_NEXT_STEPS_PLAN.md` proposed **lavapipe as "the guaranteed floor"** and
`docs/V2_VIRTUALBOX_AND_VULKAN_PLAN.md` proposed llvmpipe for the VM. Both are
wrong for niri. Not "slow", not "degraded" — **rejected by an `ensure!` that
returns an error before the compositor starts.**

And `start-desktop.sh` currently exports `LIBGL_ALWAYS_SOFTWARE=1` and
`GALLIUM_DRIVER=llvmpipe`. I wrote that configuration to *cause* the problem. It
must be removed, not tuned.

This also explains a hang rather than an error message: niri's failure path on a
software EGL device logs at DEBUG and falls back to the primary GPU, which in a
VM is also software, and there is no second fallback. The user-visible result is
a compositor that never draws — which is exactly what every VM run showed.

---

## 2. So the real requirement is: hardware 3D acceleration in the guest

niri's own wiki, one line, for the entire VM story:

> ### Virtual Machines
> To run niri in a VM, make sure to enable 3D acceleration.

VirtualBox's manual on what that requires:

> The Oracle VirtualBox Guest Additions contain experimental hardware 3D support…
> The Guest Additions must be installed.

And on the mechanism:

> VirtualBox implements 3D acceleration by installing an additional hardware 3D
> driver inside the guest when the Guest Additions are installed. This driver
> acts as a hardware 3D driver… the host then performs the requested 3D
> operation using the host's programming interfaces.

### 2.1 Consequence: Guest Additions is not optional

`umugfx` — the kernel driver in my guest logs — ships **in the Guest Additions
ISO**, not in Debian. So the base image has no `umugfx` at all unless we install
it. Every VirtualBox run so far has been a guest with **no Guest Additions**:
no `vboxvideo`/`umugfx` module, no modesetting, no 3D.

That reframes the `128x48` framebuffer. It is not "VirtualBox offers no mode".
It is **"the guest has no driver, so the kernel fell back to the text console
geometry"**.

### 2.2 Licensing — this is the part that must not be skipped

VirtualBox Guest Additions is **not redistributable**. Oracle's Personal Use and
Test license permits personal/evaluation use, not distribution inside an OS ISO.

So there is a hard constraint:

* **QA VM** — install Guest Additions from the attached ISO at test time. Legal,
  not shipped, and it is exactly what Oracle documents.
* **Shipping ISO** — must NOT contain Guest Additions.

Consequence: **the QA desktop and the shipped ISO are not the same artifact.**
The QA run proves the session works with Oracle's driver present; the shipped
image boots on real hardware with Mesa instead. Both need stating separately and
the README must not conflate them.

This is a genuine constraint on the project, not an obstacle to route around.

---

## 3. Why VirtualBox specifically is the hard case

Measured, not assumed:

```
$ VBoxManage modifyvm test --graphicscontroller virtio
VBoxManage.exe: error: Invalid --graphicscontroller argument 'virtio'
$ VBoxManage list ostypes | grep -i debian
(no results — Debian is not even an ostype in this build)
```

Available: `vmsvga`, `vboxvga`, `vboxsvga`, `qemuramfb`. VirtualBox's own 7.2
documentation states **"Only VMSVGA is supported as a graphics controller"**
for current configurations.

Cross-checking the QEMU reports in #2570: people run niri in QEMU with
`virtio-vga-gl` + `accel3d` **when the host has a real GPU** backing
`rendernode`. Without a host GPU it does not work, for the same reason.

**There is no configuration of VirtualBox that gives niri a hardware renderer
without Guest Additions, and no configuration that gives Guest Additions a
better device than VMSVGA.**

---

## 4. The options, honestly ranked

### Option A — VirtualBox + Guest Additions + `vboxvideo`/`umugfx` (recommended)

**This is what niri's maintainer means by "enable 3D acceleration", and it is
the documented VirtualBox path.**

How:
1. Attach the Guest Additions ISO to the QA VM (`VBoxManage storageattach …
   --medium path\to\Oracle_VirtualBox_7.2.10.170229-Windows_x86-64.iso`).
2. Boot the QA ISO, install Guest Additions, reboot, then re-run the suite.
3. Confirm `lsmod` shows `vboxvideo`, `drm_info` lists a real connector with
   modes, `eglinfo` reports the `vbox` renderer and `niri` logs a hardware
   renderer — not `software EGL renderers are skipped`.

Known risk, from the manual: *"3D acceleration feature is only available for
certain guests… Linux guests have OpenGL 4.1 support with Mesa3D drivers."*
OpenGL 4.1 is below what modern Wayland compositors usually want, and the
`virtualbox-guest-utils` version must match the host version or 3D silently
reverts to software (a well-known, long-standing failure — VirtualBox ticket
#15384).

Cost: one ISO to fetch, one boot cycle to test. **Do this first**, because it is
both the documented path and the cheapest experiment that could work.

### Option B — Replace the QA hypervisor with QEMU/KVM

`virtio-vga-gl` + `accel3d` + SPICE OpenGL is the configuration reported working
in #2570, but it requires a **host GPU** with a `rendernode` to hand to the
guest. If this host has no usable GPU, Option B fails for the same underlying
reason as Option A without Guest Additions.

Status on this host: QEMU is **not installed**. Whether the host GPU can be
passed through is unknown and is a Phase 0 question.

### Option C — Test the session outside a hypervisor: WSLg

WSLg is a Weston instance presenting Wayland over RDP with a Mesa D3D12 driver
backed by the host GPU. A compositor that runs as a **client** of an existing
Wayland display does not need DRM, GBM or a VT — which sidesteps niri's actual
constraint rather than trying to satisfy it.

Cost: WSL2 is already present. No ISO, no boot cycle.

Limitations, stated plainly:
* It tests the **shell, apps and workflows**, not the boot path. GRUB, live-boot,
  the installer and rollback remain untested by this route.
* Rendering goes through D3D12/RDP, so performance numbers are not bare-metal
  numbers.
* It requires the host GPU driver to expose `/dev/dri` under WSLg; if it does
  not, Mesa falls back to `llvmpipe` and niri rejects it again.

Value: this is the **only** option that can be attempted in minutes rather than
boot cycles, and it covers apps and workflows — the largest untested surface.
Use it to unblock Phase 2b/2c while Option A is tried properly.

### Option D — Make the shipped image work without niri

If VirtualBox cannot be made to work and no hardware is available for testing,
the honest options are to ship a **second, QA-only compositor** that supports
software rendering, or to gate the graphical release on hardware testing.

Candidates that do work in software: **Sway** with the pixman backend (sway has
long-standing llvmpipe support), **Labwc** (X11, works on fbdev/Xvfb), or
**Cage**. This is a real product decision, not a workaround: HCS would ship two
compositors, and the README would have to say which is which.

Cost: significant. **Only if A, B and C all fail**, and it must be a decision,
not a default.

---

## 5. Revised plan

### Phase 0 — resolve the remaining unknowns cheaply (minutes, no boot cycle)

The host GPU question is answered: **AMD Radeon, 4 GB**. What remains:

1. **Can VirtualBox pass through nested virtualisation?** HVCI is active and
   VirtualBox's `umugfx` complains about an unsupported hypervisor. Check
   `VBoxManage showvminfo | grep -i nested`. A Parsec virtual display adapter
   suggests a nested context, which is a plausible cause.
2. **Fetch the matching Guest Additions ISO** for VirtualBox 7.2.10
   (`Oracle_VirtualBox_7.2.10.170229-Windows_x86-64.iso`). Version must match
   the host exactly — a mismatch silently reverts 3D to software (VirtualBox
   ticket #15384, a long-standing failure).
3. **Verify the host render node under WSLg:** in the WSL shell, `ls
   /dev/dri/renderD*`. If present, Mesa D3D12 has something to accelerate with.
4. **Install QEMU** for the `virtio-vga-gl` experiment — `winget install
   qemu.qemu`, then `qemu-system-x86_64 --version`.

### Phase 1 — Option C in parallel (apps and workflows, not the boot)

Run the Neural Glass shell and every app as WSLg clients. Record which work,
which need hardware, which are broken. This converts the largest untested
surface from "unknown" to a list, without waiting on the boot path.

### Phase 2 — Option A, one boot (the actual goal)

Install Guest Additions in the QA VM, verify hardware renderer, run the four
cohorts. This is the path niri's own documentation prescribes.

### Phase 3 — correct the code that is actively wrong

| Change | Reason |
|---|---|
| Remove `LIBGL_ALWAYS_SOFTWARE=1` | niri rejects software EGL by design |
| Remove `GALLIUM_DRIVER=llvmpipe` | same; it forces the rejected path |
| Replace the "lavapipe is the guaranteed floor" claim | false for niri |
| Stop treating llvmpipe as the QA answer | false for niri |
| Add a gate: `eglinfo` renderer must not be software | prevents a regression that is currently invisible |

That last row matters most: a session that starts with a software renderer is
*already broken*, and nothing in the project notices.

### Phase 4 — README, corrected

The "CPU-first" claim needs restating. It cannot mean "renders in software",
because niri will not. It means:

> The desktop does not require a discrete GPU. Intel and AMD integrated
> graphics are fully supported. A machine with no supported GPU cannot run the
> HCS session.

That is a real limitation and it is better than a claim that would fail on the
first unsupported machine.

---

## 6. What I would not do

* **Not** ship Guest Additions in the ISO. Oracle's licence forbids it.
* **Not** claim software rendering works. Researched, from source, it does not.
* **Not** paper over a failing VM gate with an X11 fallback and call the
  graphical release done. It can be a *documented QA fallback*, labelled as not
  being the product.
* **Not** pick one of the two remaining blockers (VM graphics vs `local-fs`)
  and act on it. `local-fs` is still unobserved, and the diagnostics that would
  observe it have never run.

---

## 7. Questions only you can answer

**Question 1 is answered** — the host has an AMD Radeon, so Options B and C are
viable. Two remain:

1. **Is a software-rendering fallback acceptable as a product decision?** That
   is: ship a second compositor (Sway/pixman, labwc, or Cage) so HCS works on a
   machine with no supported GPU. Researched answer: niri alone cannot do this,
   because it rejects software EGL by design. This contradicts the "no GPU
   needed" position taken so far, so I will not assume it — it means shipping
   two compositors and documenting which is which.
2. **Is testing against Guest Additions acceptable**, given the shipped ISO will
   not contain it? Oracle's licence forbids redistributing it. A QA run with
   Guest Additions would prove the session works with Oracle's driver present,
   while describing an artifact users will not have. **It would not be
   evidence about the shipped ISO**, and the README must not present it as such.

With the AMD GPU confirmed, my recommendation changes to:

* **Option B (QEMU + `virtio-vga-gl`)** as the primary QA target — it gives the
  guest a *standard* DRM device that needs no Oracle driver at all, so the QA
  result describes an artifact closer to the shipped ISO.
* **Option A (VirtualBox + Guest Additions)** kept as the documented VirtualBox
  path, but explicitly labelled as not describing the shipped image.
* **Option C (WSLg)** for the apps-and-workflows surface while the boot path is
  rebuilt, since it needs no boot at all.