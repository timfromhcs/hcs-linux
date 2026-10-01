# HCS Linux 2.0 — VirtualBox GUI Gate and Bare-Metal Vulkan Plan

Status: work plan
Companion to: `docs/V2_NEXT_STEPS_PLAN.md`
Date: VirtualBox 7.2.10r174163, Windows host, HVCI/Memory Integrity active

---

## 0. The constraint that decides everything

I measured this rather than guessing, and it is not what the plan wanted to hear.

```
VBoxManage.exe: error: Invalid --graphicscontroller argument 'virtio'
```

**This VirtualBox build on a Windows host has no `virtio` graphics controller.**
The only options are `vmsvga` (which is VirtualBox's own `umugfx`) and legacy
`vboxvga`. There is no way to give the guest a modern DRM device, because
VirtualBox does not implement one on this platform. QEMU is not installed on this
host either.

So the VirtualBox gate cannot be satisfied the way the previous plan assumed. It
cannot be satisfied by choosing better virtual hardware, because the hardware
VirtualBox offers is exactly the hardware that does not work.

Two consequences follow, and they split the work:

1. **VirtualBox QA must work on `umugfx`.** Not around it, not by switching
   hypervisors. On it.
2. **Vulkan is irrelevant to VirtualBox** and must not be part of the QA
   critical path. It is a bare-metal concern only.

The second point matters most, because the previous plan made Vulkan load-bearing
for QA. That was wrong: `umugfx` has no Vulkan implementation at all, not even a
software one, so a VirtualBox QA run that depends on Vulkan can never pass.

---

## 1. Part One — VirtualBox GUI-first gate

Goal: a real desktop on the live ISO inside VirtualBox, a real Calamares install
GUI, and an honest 32-stage verdict.

### 1.1 Why `umugfx` refuses to work, precisely

The guest log from the current image:

```
[    2.804763] umugfx 0000:00:02.0: [drm] Capabilities2: grow otable, intra surf
[    2.804763] umugfx 0000:00:02.0: [drm] umugfx seems to be running on an
                            unsupported hypervisor.
[    2.804763] umugfx 0000:00:02.0: [drm] *ERROR* This configuration is likely
                            broken.
[    2.804763] umugfx 0000:00:02.0: [drm] fb0: umugfxdrmfb frame buffer device
[    2.804763] umugfx 0000:00:02.0: [drm] Console: switching to colour frame
                            buffer device 128x48
[    2.804763] umugfx 0000:00:02.0: umugfx 2.21.0 for 0000:00:02.0 on minor 0
[    3.464161] ata1.00: SATA link up 3.0 Gbps
```

Three distinct problems, and conflating them is how the last three releases went
wrong:

**A. The guest kernel module is wrong, not just absent.** `umugfx` is a *kernel*
driver. Mesa has no control over it. The `*ERROR*` lines come from the guest-side
module under VirtualBox Guest Additions, and the hypervisor complaint is
VirtualBox's own code detecting that it is running somewhere it does not expect
(HVCI/Memory Integrity is active on this host).

**B. `128x48` is the giveaway.** `Console: switching to colour frame buffer device
128x48` means the DRM device has **no usable mode**. There is no display resolution
to present a desktop into. niri cannot acquire a VT it was never offered.

**C. Nothing here is a rendering problem.** Even if the mode were correct, there is
no GPU code in the image to render with. Two separate failures stacked on top of
each other, which is why "add Mesa" alone will not fix the VM.

### 1.2 Options, in the order they are worth trying

Each step has an observable acceptance test. If a step fails, stop and take the
next — do not carry a broken assumption forward.

#### Step 1 — Give the guest the Mesa software stack anyway

`umugfx` is useless for hardware rendering, but **llvmpipe needs no hardware at
all**. It rasterises in the CPU. If a DRM device and a VT can be obtained, Mesa
can present through it in software. This is the cheapest experiment and it is
worth running before anything else, because if it works the whole gate unblocks.

* Add `libgbm1`, `mesa-vulkan-drivers`, `libgl1-mesa-dri`, `libegl-mesa0`,
  `libglx-mesa0`, `mesa-utils`, `mesa-va-drivers`, `vulkan-tools`, `libdrm2`.
* Note the split: `libgbm1` (kernel/GBM side) and `libgl1-mesa-dri` (rendering)
  are **different packages** and both are needed. Installing only
  `mesa-vulkan-drivers` gets a Vulkan ICD and still no GL — which is a real trap
  that costs hours if not anticipated.
* Force software rendering in `start-desktop.sh`: `WLR_RENDERER=pixman`,
  `LIBGL_ALWAYS_SOFTWARE=1`, and niri's `debug.disable-cursor-plane`.
* **Acceptance:** `vulkaninfo --summary` reports lavapipe; `glxinfo -B` under Xvfb
  reports llvmpipe; niri reports a renderer and the framebuffer changes.

#### Step 2 — Fix the 128x48 mode

If the DRM device still offers no mode, the VT is unusable regardless of
rendering.

* Check whether `video=1024x768` (already on the cmdline) is being honoured or
  ignored by `umugfx`.
* Try VirtualBox's `--vram 256` and confirm the guest sees a non-trivial
  `fb0`.
* If `umugfx` genuinely cannot provide a mode under HVCI, that is a VirtualBox
  bug on this host configuration and it is not ours to fix — see §1.4.

#### Step 3 — X11 fallback session

niri is not the only compositor, and if `umugfx` only supports a legacy X
stack, a small X11 session may be the honest thing the QA gate measures.

* This is a **QA-only fallback session**, not the product. HCS still ships niri.
  The gate's job is to prove the image boots, draws, and runs the apps; the
  compositor choice is secondary to that proof.
* Requires `xserver-xorg-video-fbdev` plus a minimal WM, with Xvfb for the
  headless RAM reference path.
* **Acceptance:** a window from `hcs-settings` appears in a screenshot and the
  capture gate accepts the frame.

#### Step 4 — Nested/virtualised second layer

If VirtualBox cannot render on this host at all, run the guest **inside** a
nested context that provides a proper DRM device:

* Enable nested virtualisation in the guest and boot QEMU/KVM with `virtio-gpu`
  inside VirtualBox. Slow, but it gives a real `virtio` DRM device with a mode,
  and `venus`/`virgl` or llvmpipe on top.
* This keeps the VirtualBox host driver as the outer harness — evidence disk,
  screenshots, boot control — while the graphics path becomes a normal one.
* **Acceptance:** identical to Step 1, but with a real mode from virtio-gpu.

### 1.3 What the QA driver has to change

The driver was built before the GPU question was asked, and it makes assumptions
that the measurements have now invalidated.

* **Graphics device:** pin `vmsvga` explicitly rather than inheriting a default,
  and document why. A silent default that changes between VirtualBox versions is
  exactly the kind of thing that made this run ambiguous.
* **VRAM:** raise from 128 MB to 256 MB, and assert it.
* **3D acceleration:** currently `on`. Record it, because llvmpipe makes it
  irrelevant and a reviewer will otherwise assume acceleration was measured.
* **Boot order:** the first run booted the DVD and reached a live shell; the
  second reached the installer path. Make the entry explicit per cohort instead
  of relying on `dvd,hdd`.
* **A stage that proves a frame was drawn.** Today `DONE` is the only signal.
  Add a heartbeat stage that captures the framebuffer early, so "the guest hung at
  graphics init" is distinguishable from "the guest never started".

### 1.4 The fallback position, stated honestly

If none of the four steps yields a drawable frame on this host, the correct
outcome is to **record VirtualBox as an unsupported QA environment and move the
GUI gate to QEMU/KVM or bare metal** — with the reason written down as the
measured `umugfx` failure, not as "could not get it working".

A gate that passes on a renderer nobody will use is worse than a gate that does
not run. This project has already shipped three releases whose gates could not
have caught the absence of a compositor; a green gate must mean something.

---

## 2. Part Two — Vulkan and GPU for bare metal

Explicitly **not** part of the VirtualBox gate. `umugfx` has no Vulkan path, so
Vulkan cannot be tested in VirtualBox at any point, and the QA plan must not wait
for it.

### 2.1 Target matrix

| Hardware | Kernel driver | Vulkan ICD | Mesa GL/EGL | Package to add |
|---|---|---|---|---|
| Intel iGPU Gen 8+ / Arc | `i915` | `iris` | `iris` | firmware only |
| AMD Radeon | `amdgpu` | `radv` | `radeonsi` | firmware only |
| NVIDIA proprietary | `nvidia` | NVIDIA | NVIDIA blob | **post-install only** |
| NVIDIA open modules | `nvidia` | NVIDIA | NVIDIA | **post-install only** |
| **No GPU (floor)** | none | **`lilavulkan`/lavapipe** | `llvmpipe` | `mesa-vulkan-drivers` |
| QEMU/KVM | `virtio-gpu` | `venus` | `virgl`/`llvmpipe` | `mesa-vulkan-drivers` |
| Apple Silicon | `drm-apple` | `asahi` | — | out of scope for x86 target |

### 2.2 The universal-Vulkan-drivers myth

`mesa-vulkan-drivers` is not a universal driver. It is a metapackage containing a
loader plus **four separate ICDs**, each for different hardware:

* `intel_icd` / `iris`
* `amd_icd` / `radeonsi`
* `lvp_icd` / lavapipe (CPU)
* `swrast_icd` (legacy GL-on-Vulkan fallback)

Whichever is present at runtime is chosen by the loader from the actual device. The
loader is the "universal" part; the ICDs are not interchangeable, and installing
the metapackage gives no guarantee that a *particular* GPU is accelerated.

The honest framing for the README:

> The desktop runs on any x86-64 machine. Hardware acceleration is available on
> Intel and AMD through Mesa. NVIDIA requires a separately installed driver. On a
> machine with no supported GPU, software rendering still produces a correct
> desktop.

### 2.3 Packages for the bare-metal image

Separate from the QA image, and installed into it deliberately:

* `mesa-vulkan-drivers` — the loader and all Mesa ICDs
* `libgl1-mesa-dri` — GL/DRI drivers (llvmpipe included). **Not** the same package
  as the Vulkan metapackage.
* `libegl-mesa0`, `libglx-mesa0`, `libgbm1` — EGL/GLX/GBM
* `libdrm2`, `libdrm-amdgpu1`, `libdrm-intel1`, `libdrm-nouveau2`, `libdrm-radeon1`
* `mesa-va-drivers` — VA-API for `hcs-shot` and video
* `vulkan-tools` — for `vulkaninfo`, as the acceptance test
* `mesa-utils`, `mesa-utils-bin` — `glxinfo`, `es2_info`
* `firmware-linux-free`
* `non-free-firmware` plus `firmware-iwlwifi`, `firmware-atheros`,
  `firmware-realtek`, `firmware-brcm80211`

Licence note: `non-free-firmware` is a redistribution exception, not a free
licence. It must appear in the third-party notices and in the README, not be
buried in a package list.

**NVIDIA stays out of the image.** It is not redistributable, it is large, and the
module must match the kernel ABI exactly. Document it as a post-install step.

### 2.4 Bare-metal acceptance tests

Each must pass on real hardware before any GPU support is claimed:

* `vulkaninfo --summary` names a non-`llvmpipe` device
* `glxinfo -B` reports the expected renderer (iris / radeonsi / NVIDIA)
* `vkcube` runs — a real Vulkan triangle, not just an ICD that enumerates
* niri reports a hardware GLES renderer rather than software
* Wi-Fi associates after boot (proves firmware, which is the usual silent failure)
* `hcs-shot` captures the screen (proves VA-API/DRM lease)
* Frame time measured, not estimated, under the reference scenes

### 2.5 A hardware gate that cannot lie

The VirtualBox lesson generalises: a driver being *installed* is not a driver
*working*.

* A bare-metal gate that runs `vulkaninfo` and asserts a hardware ICD, refusing
  to accept llvmpipe as evidence of GPU support.
* A gate that greps `dmesg` for `*ERROR*` DRM lines and fails on them.
* A gate that asserts niri reports a hardware renderer.
* Explicit hardware inventory recorded per test run — model, driver, ICD, kernel
  version — so a claim can be traced to a machine.

---

## 3. Part Three — Live-boot GUI and Install GUI

Two different GUIs, two different proofs, and they have never both been seen.

### 3.1 Live-boot GUI (the session that must appear first)

Sequence in the VM, all photographable:

1. Boot menu appears (GRUB, own menu file)
2. Kernel handoff — systemd PID 1 confirmed
3. `hcs-banner.service` writes the banner to tty1
4. `live-boot` mounts the squashfs
5. `hcs-desktop.service` starts seatd and niri
6. Compositor reports a renderer
7. **Neural Glass shell draws** — first real frame
8. Taskbar, Start monogram, tray visible
9. `HCS+Space` cycles the keyboard layout, `HCS` opens the omnibar
10. Each app opens as a window: `hcs-settings`, `hcs-term`, `hcs-docs`, `hcs-chat`

Steps 6–10 are where every previous release stopped.

### 3.2 Install GUI (Calamares)

Currently never verified. The image ships Calamares but nothing has ever shown it
running.

* `hcs_install=1` must select the installer path without breaking PID 1.
* Calamares needs a VT, a display, and `xcb`/`wayland` support — so it inherits
  every problem above, plus its own Qt/xcb requirements.
* The install cohort must be a **separate boot**, not a stage inside the live
  session. Installing to the attached disk is not a thing that can happen
  sequentially in one desktop session.
* Capture: module selection, disk partitioning, encryption (LUKS2), user creation,
  and the final summary screen.

### 3.3 The QA suite must become boot cohorts

The current 32-stage manifest runs every stage in one desktop session. That
cannot honestly represent GRUB, the installer, an installed system, rollback or
amnesic mode — those are separate boots by definition.

Four cohorts, each a distinct boot with a distinct kernel cmdline:

| Cohort | Cmdline | What it proves |
|---|---|---|
| `live` | `hcs_session=graphical hcs.qa=1` | session, apps, all GUI stages |
| `installer` | `hcs_install=1 hcs.qa=1` | Calamares GUI end to end |
| `installed` | normal boot from disk | the installed system boots and draws |
| `amnesic` | `hcs_amnesic=1 hcs.qa=1` | nothing persists, nothing reaches host disks |

Each cohort gets its own GRUB entry, its own evidence disk, its own verdict. A
single `result.json` per cohort, and an aggregate that is GREEN only if all four
are.

---

## 4. Order of work

Sequential where the dependencies are real, parallel where they are not.

| # | Task | Depends on | Verifiable here |
|---|---|---|---|
| 1 | Add Mesa/GBM/firmware packages, force llvmpipe | — | **yes**, in the VM |
| 2 | Fix or diagnose the 128x48 mode | 1 | **yes** |
| 3 | Confirm a software renderer exists | 1, 2 | **yes**, `glxinfo` |
| 4 | niri acquires a VT and draws | 3 | **yes**, screenshot |
| 5 | Split the suite into four boot cohorts | — | **yes**, suite validation |
| 6 | Driver: pin `vmsvga`, 256 MB VRAM, heartbeat stage | — | **yes** |
| 7 | Live-boot GUI stages photograph green | 4, 5, 6 | **yes** |
| 8 | Calamares cohort | 4 | **yes** |
| 9 | Installed-system cohort | 8 | **yes** |
| 10 | Amnesic cohort | 7 | **yes** |
| 11 | Bare-metal Vulkan + firmware packages | — | **partly**, build only |
| 12 | Bare-metal hardware acceptance | 11 | **no** — needs the machine |
| 13 | Real starter model, licensed, digest-pinned | — | **partly**, build + smoke |

Items 12 and 13's hardware portion cannot be verified on this host and will be
recorded as such rather than assumed.

---

## 5. Definition of done

**VirtualBox gate**
1. A screenshot exists in which the Neural Glass shell is drawn.
2. That screenshot is graded GREEN by `grade_capture.py`, running on the host.
3. The Calamares GUI is photographed in at least four distinct stages.
4. All four boot cohorts return their own verdict, and the aggregate is GREEN.
5. `vulkaninfo` is **not** part of this gate, because `umugfx` cannot satisfy it.

**Bare-metal gate**
1. `vulkaninfo --summary` names a hardware ICD on real hardware.
2. `vkcube` renders.
3. niri reports a hardware renderer.
4. Wi-Fi associates without manual firmware intervention.
5. The hardware inventory for the run is recorded in the QA report.

**Honesty gate**
1. No capability is claimed in the README that a gate does not verify.
2. VirtualBox is either GREEN with evidence, or documented as unsupported with
   the measured reason.
