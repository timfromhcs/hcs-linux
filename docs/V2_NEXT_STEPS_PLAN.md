# HCS Linux 2.0 — Graphics, Hardware and Next-Steps Plan

Status: active work plan
Last updated: after commit `3fbaf30` (image boots; graphical session blocked)

---

## 0. What we know right now, because it is measured and not assumed

Three things happened in sequence, and each one was only knowable by running the
image rather than reasoning about it.

### 0.1 The image boots, and systemd is PID 1

The blocker from the previous cycle was `/sbin/init`. A 120-line shell script
from the era when there was no system to run was being written over the base's
own init, and GRUB was told to run it instead of systemd. systemd never became
PID 1, so nothing it owns was set up.

That is fixed. `/sbin/init` is a symlink to systemd again, the GRUB entries no
longer pass `init=/sbin/init`, and the session starts from
`hcs-desktop.service`. There is now a gate that fails the build if `/sbin/init`
is ever a regular file again, or if a GRUB entry reintroduces the override.

### 0.2 niri starts, and then rejects the graphics hardware

This is the finding that shapes this whole plan. The live kernel log from a
VirtualBox 7.2 VM, captured from the guest framebuffer:

```
[    2.804763] umugfx 0000:00:02.0: [drm] Capabilities2: grow otable, intra surf
[    2.804763] umugfx 0000:00:02.0: [drm] umugfx seems to be running on an
                            unsupported hypervisor.
[    2.804763] umugfx 0000:00:02.0: [drm] *ERROR* This configuration is likely
                            broken.
[    2.804763] umugfx 0000:00:02.0: [drm] *ERROR* Please switch to a supported
                            graphics device to avoid problems.
[    2.804763] umugfx 0000:00:02.0: [drm] Console: switching to colour frame
                            buffer device 128x48
[    2.804763] umugfx 0000:00:02.0: umugfx 2.21.0 for 0000:00:02.0 on minor 0
```

Read carefully, because this is the whole problem in six lines:

* **niri loaded.** Version 2.804763 initialised. Smithay saw the DRM device,
  enumerated its capabilities, and then refused it.
* **`umugfx` is VirtualBox's own guest kernel driver.** It is not Mesa, it is not
  a real DRM driver in the normal sense, and it has no Vulkan implementation at
  all. Smithay — and therefore niri — does not support it.
* **The machine then hung.** The framebuffer was byte-identical across three
  screenshots taken minutes apart. The boot does not fail loudly; it stalls
  during graphics initialisation.

So the desktop is no longer blocked by our own code. It is blocked by the
absence of a graphics stack in the image, plus a VirtualBox device that modern
Wayland compositors do not support.

### 0.3 The image ships with no graphics stack whatsoever

This is the part that should have been caught before the first release, and the
package list confirms it. Searching `config/package-lists/hcs-core.list.chroot`
for `mesa`, `vulkan`, `libgl`, `firmware` or `xf86` returns **nothing**.

The image has a Wayland compositor and no drivers for it. It has a GPU-agnostic
product promise and no GPU code. Every release so far declared "CPU-first" and
was read as "no GPU needed", when the actual situation was "no GPU support was
ever written".

---

## 1. The hardware question, answered honestly

There is no universal GPU driver. Anyone claiming one exists for Vulkan is
either describing a software rasteriser or describing a wrapper. What a real
distribution ships is a *stack* with several mutually exclusive back ends, one of
which always works.

| Hardware | Kernel driver | Vulkan ICD | Reality |
|---|---|---|---|
| Intel iGPU (Gen 8+) | `i915` | `iris` (Mesa) | Supported, good. Best default. |
| Intel Arc | `i915` | `iris` | Supported. |
| AMD Radeon | `amdgpu` | `radv` (Mesa) | Supported, best Vulkan in Mesa. |
| NVIDIA proprietary | `nvidia` | NVIDIA ICD | Supported, but a non-redistributable blob. |
| NVIDIA open modules | `nvidia` | NVIDIA ICD | Supported on recent cards. |
| Any x86, no GPU | none | `lilavulkan` / `lavapipe` (Mesa) | **Always works.** |
| VirtualBox | `umugfx` (current) | **none** | Does not work. Needs `virtio-gpu`. |
| QEMU/KVM | `virtio-gpu` | `virtio` / `virgl` | Works. |
| Apple Silicon | `drm-apple` / Asahi | `asahi` | Works on supported hardware. |

### 1.1 The design decision this forces

**The software rasteriser is not a fallback, it is the floor.**

`lavapipe` is a conformant Vulkan implementation in pure CPU code. It is slower
than hardware and it is always correct. Making it the guaranteed path means the
desktop cannot fail to draw because of hardware — which is what makes the
CPU-first claim true rather than aspirational, and it is what makes the whole
product testable in a VM on a machine with no GPU at all.

Everything else is an acceleration on top of that floor, and an optional one.

This is also what turns the VirtualBox gate green without special-casing
VirtualBox: if `lavapipe` is present, the QA VM has a working Vulkan device
regardless of what VirtualBox exposes.

### 1.2 What "universal" has to mean for this project

> Any x86-64 machine, with or without a supported GPU, reaches the HCS desktop.

Not "one driver for all hardware". That would be a lie. The honest version is a
testable floor plus named accelerations, and the plan below builds exactly that.

---

## 2. Phase A — the graphics stack (blocks everything else)

Nothing further can be verified until a frame is actually drawn. This phase is
sequential and each step is individually testable.

### A1. Software Vulkan as the guaranteed path

* Add `mesa-vulkan-drivers` (provides `lilavulkan`, the loader and the ICDs) and
  `libdrm2` to the core package list.
* Add `vulkan-tools` so `vulkaninfo` is available — as the acceptance test, not
  as a convenience.
* Set `VK_ICD_FILENAMES` and `LIBGL_DRIVERS_PATH` explicitly in
  `start-desktop.sh`. Mesa ships its drivers under a versioned path
  (`/usr/lib/x86_64-linux-gnu/dri/`) and an unset loader path silently yields no
  ICD at all, which is indistinguishable from a missing driver.
* **Acceptance:** in the QA VM, `vulkaninfo --summary` reports a device
  (lavapipe is acceptable and expected) and niri reports a working GlesRenderer
  and renderer. Screenshot must show the shell.

### A2. Mesa GL/EGL for the fallback renderer

* Add `libgl1-mesa-dri`, `libegl-mesa0`, `libglx-mesa0`, `mesa-utils` and
  `libgl1-mesa-dri` for the DRI drivers.
* SwiftShader (`mesa-vulkan-drivers` includes it) is the other safety net for
  OpenGL-ES, which Quickshell/Qt needs even when Vulkan is present.
* **Acceptance:** `glxinfo`/`es2_info` succeed under Xvfb; the compositor reports
  a software renderer rather than failing.

### A3. Firmware

* Add `firmware-linux-free`, `firmware-linux-nonfree` (as `non-free-firmware` on
  sid), `firmware-iwlwifi`, `firmware-atheros`, `firmware-realtek`,
  `firmware-brcm80211`.
* Without these, Wi-Fi and many GPUs come up with no output at all, and on
  NVIDIA the display stays dark with no diagnostic.
* Note on licensing: `non-free-firmware` is a redistribution exception, not a
  free-software licence. It must be called out in the third-party notices and in
  the README, not buried. If a strictly-free-only build is wanted, that is a
  separate image variant and a separate decision.

### A4. Real hardware drivers

* Intel: nothing extra beyond firmware. `i915` is in-kernel, `iris` ships with
  Mesa.
* AMD: nothing extra beyond firmware. `amdgpu` is in-kernel, `radv` ships with
  Mesa.
* NVIDIA: **do not bundle.** Ship a documented post-install step. Reasons: the
  driver is not redistributable, it is large, and the module must match the
  kernel ABI exactly. Recommending it in the README is correct; baking it into
  the image is not.

### A5. Rebuilding the QA VM onto virtio-gpu

VirtualBox cannot use its own `vmsvga` device with a Wayland compositor, so the
QA gate must move to the one device VirtualBox does support:

* Switch the VM to `virtio` graphics with 3D acceleration.
* Attach disks over a `virtio-blk` or NVMe controller so the guest sees
  `/dev/vda`/`/dev/vdb` — which also fixes the evidence-disk detection that
  currently has to guess between `/dev/vdb` and `/dev/sdb`.
* Keep the SATA fallback path working, because `find_evidence_disk()` now
  handles both and that is deliberate.

### A6. A graphics capability gate

The lesson from three failed releases is that a declared capability is not a
capability. So:

* A gate that runs `vulkaninfo --summary` in the VM and fails if no ICD is
  found.
* A gate that greps the guest journal for `*ERROR*` DRM messages and fails on
  them.
* A gate that asserts the compositor reports a renderer, not just that the
  process is alive. A running compositor with no renderer is exactly the failure
  that has been invisible until now.

---

## 3. Phase B — the rest of the product

Ordered by how much they block a truthful release.

### B1. Real starter models

`placeholder.gguf` is baked in and is not a model. A release that claims
local AI with a placeholder is exactly the kind of claim this project has been
correcting elsewhere, so it has to go.

* Source at least one genuinely small, permissive GGUF (Apache-2.0 or MIT
  weights, not merely a permissive licence on the code).
* Pin by digest, record size and licence in the third-party notices.
* Wire `hcs-modeld` to load it and prove inference end to end.
* **Acceptance:** a token is generated on the target hardware, and the QA
  journal contains the output.

### B2. Split the QA suite into real boot cohorts

The current 32-stage manifest runs every stage in one desktop session, which
cannot honestly represent GRUB, the installer, an installed system, rollback, or
amnesic mode — those are different boots by definition. Each cohort needs its
own boot, and the driver needs to select the entry and reboot between them.

### B3. Finish the daemons that are still mocks

`hcs-update`, `hcs-persist`, `hcs-recall`, `hcs-shot`, `hcs-term` and
`hcs-notes` render UI over placeholder data. Each needs its real implementation
behind the interface it already exposes.

### B4. `hcs-agent-bridge`

Listed in the master plan, still absent. Decide whether it is in scope for 2.0;
if it is, build it; if it is not, delete it from the plan so the two documents
stop disagreeing.

### B5. Supply-chain honesty

* Pin sid to a snapshot timestamp. sid is rolling, so today's build and
  tomorrow's rebuild are not the same system.
* Replace the placeholder entries in `vendor/locks/sources.lock.yaml` with the
  real niri commit and digests actually used.
* Regenerate the SBOM and third-party notices from the built image rather than
  from a hand-maintained list.
* Extend `verify_package_availability.py` to target `debian:sid`; CI currently
  checks trixie with `continue-on-error`, which has been hiding drift.

### B6. Rendering and performance validation

Once a frame is drawn, measure rather than assume:

* RAM budget per app (the existing 250 MB gate) measured on a real session.
* Compositor frame time under the reference scenes.
* Cold-boot time to first pixel as a tracked number.

---

## 4. Testability — what I can verify here, now

Ranked by how much confidence each result carries.

| What | How | Confidence |
|---|---|---|
| Boot reaches a desktop in a VM | VirtualBox + framebuffer screenshots + journal | High — already have the harness |
| Vulkan present and correct | `vulkaninfo --summary` in guest | High |
| Software rendering works | Force `lavapipe`, screenshot | High |
| Capture gate rejects fake screens | `tests/unit/test_capture_gate.py` | High — already green |
| Host grading is independent of the guest | `tests/unit/test_qa_grading.py` | High — already green |
| Package and payload gates | existing CI + `verify_payload.sh` | High — 164 checks green |
| Real hardware acceleration | needs the physical machine | **Cannot verify here** |
| Wi-Fi, Bluetooth, suspend | needs the physical machine | **Cannot verify here** |

The last two rows matter as much as the rest. This document should not, and will
not, be used to claim hardware support for a machine nobody has run it on.

---

## 5. Definition of done for 2.0

1. `vulkaninfo` reports a device on real hardware and in the QA VM.
2. The shell draws in a VM, photographed by the capture gate and graded GREEN by
   the independent host checker.
3. All four boot cohorts of the QA suite pass from their own boots.
4. A real licensed model produces real output on the target machine.
5. Every gate is green on a tag, with no `continue-on-error` in the release
   workflow.
6. The README states exactly what has been verified and on what.

---

## 6. Honest status table

| Capability | State | Evidence |
|---|---|---|
| Bootable image | **PASS** | 1.5 GB ISO, boots in VirtualBox |
| systemd is PID 1 | **PASS** | `/sbin/init -> ../lib/systemd/systemd`, gated |
| Session unit starts | **PASS** | niri initialises, visible in guest log |
| GPU / Vulkan support | **FAIL** | no Mesa or Vulkan in the image at all |
| VirtualBox + Wayland | **FAIL** | `umugfx` unsupported by Smithay; boot hangs |
| Graphical desktop in a VM | **NOT RUN** | blocked by the two rows above |
| 32-stage QA suite | **NOT RUN** | blocked by the desktop |
| Local AI inference | **FAIL** | `placeholder.gguf` is not a model |
| Hardware validation | **UNTESTED** | no physical machine in this loop |
