# HCS Linux

**AI-Native • Local-First • Privacy-Oriented • CPU-First Linux Distribution**

```text
                 HCS LINUX
                     │
          ┌──────────┴──────────┐
          │                     │
      HCS SHELL             HCS BRAIN
          │                     │
   modern Wayland          local AI + memory
   glass / smooth          agents + skills
   dock / launcher         tools + planner
   workspaces              verifier + learner
          │                     │
          └──────────┬──────────┘
                     │
                Linux + Apps
```

HCS Linux is a modern operating system built from the ground up to integrate local machine intelligence into everyday computing. Rather than treating artificial intelligence as an external cloud service or a floating chatbot window, HCS Linux unifies the Wayland desktop compositor, an SQLite-backed cognitive memory layer, on-demand GGUF model serving, and a scoped agent runtime into one cohesive, local-first system.

---

## Current Stable Release: v1.1.0

**Download:** [HCS-Linux-1.1.0-amd64.iso](https://github.com/timfromhcs/hcs-linux/releases) · verify with `SHA256SUMS` (`sha256sum -c SHA256SUMS`) · [Release notes](https://github.com/timfromhcs/hcs-linux/releases/tag/v1.1.0)

### v2.0.0 is in development

The next stable release is **v2.0.0**, built to
[`docs/V2_STABLE_RELEASE_MASTER_PLAN.md`](docs/V2_STABLE_RELEASE_MASTER_PLAN.md).
It is **not released yet** and nothing on this page claims otherwise.

Follow progress on the `dev` branch. Current verified state:

| Capability | State | Evidence |
|---|---|---|
| Bootable ISO | **PASS** | 1.5 GB image boots in VirtualBox |
| systemd is PID 1 | **PASS** | `/sbin/init -> ../lib/systemd/systemd`, gated against regression |
| Session unit starts | **PASS** | `niri` initialises; visible in the guest log |
| GPU / Vulkan support | **FAIL** | the image ships **no** Mesa, Vulkan or firmware packages |
| VirtualBox + Wayland | **FAIL** | VirtualBox's `umugfx` is unsupported by Smithay; boot hangs at graphics init |
| Graphical desktop in a VM | **NOT RUN** | blocked by the two rows above |
| 32-stage VirtualBox suite | **NOT RUN** | blocked by the desktop |
| Local AI inference | **FAIL** | the baked `placeholder.gguf` is not a model |

**The honest summary: v2 boots, and has no graphics stack.** A real base system
now exists — 37,000+ files, Debian sid, systemd, `niri` 26.04, its own kernel
7.2.8 with 4,216 modules — and `niri` starts and then refuses the hardware. The
image contains no `mesa-vulkan-drivers`, no `libdrm`, no firmware and no GPU
drivers whatsoever, because the package list never contained any.

There is no "universal GPU driver" and this project will not claim one. The plan
is a guaranteed software-Vulkan floor (`lavapipe`) with named accelerations on
top, detailed in
**[`docs/V2_NEXT_STEPS_PLAN.md`](docs/V2_NEXT_STEPS_PLAN.md)**.

Already merged on `dev`:

- **A real base system and boot path**, replacing a payload directory that had
  been shipped inside an ISO. `systemd` owns the boot; the session starts from
  `hcs-desktop.service` rather than from a shell script masquerading as init.
- **The HCS key.** The Windows-key position is the HCS key, and every shortcut
  is written as `HCS+…`. QWERTZ by default, `HCS+Space` cycles
  QWERTZ / EN-US / FR / ES / IT / GB. The HCS key is a modifier, so no layout
  can move it.
- **New programs:** `hcs-fm` (files), `hcs-term` (terminal), `hcs-shot`
  (screenshot + OCR + colour pick), `hcs-notes`, `hcs-actions` (one action
  registry for the omnibar, the CLI and agents), `hcs-update` (level-based
  updates with snapshots and rollback), `hcs-persist` (amnesic sessions),
  `hcs-recall` (local encrypted timeline).
- **Window management:** Snap Layouts, virtual desktops, Task View, Stage
  Manager, and a floating/tiling mode switch.
- **One theme file** (`colors.toml`) plus a WCAG-AAA **High Contrast** preset.
- **Retrieval that cites its sources**, and refuses rather than guessing.
- **Thirteen gates** in `make gate-all`, including an ISO payload contract that
  fails the build when a file is missing, and a 32-stage VirtualBox run driven
  from inside the guest.

---

## What v1.1.0 shipped

- **Neural Glass desktop:** Windows-like bottom taskbar (Start monogram, tasklist, tray), Start Menu with omnibar, `HCS+/` cheatsheet HUD.
- **Offline CPU Image Studio:** `hcs image "prompt" --steps 6 -o render.png` (SD 1.5 LCM Q4, 512×512 in 4–8 steps, ≤2.2 GB peak, strict on-demand lifecycle).
- **Security Lab & Tor Shield:** Debian-native arsenal (`nmap`, `wireshark`, `sqlmap`, `john`, `hashcat`, `hydra`, …), fail-closed nftables transparent proxy (`hcs-tor-switch`, TransPort 9040 / DNSPort 9053, zero DNS leaks), HITL-gated `hcs agent run --role pentester`.
- **Developer suite:** Rust / Python 3.12 / Node, Neovim LSP + VSCodium, `hcs dev init|test|debug`.
- **Offline docs:** `hcs-docs` portal (6 manuals), `cheatsheet.json`, `hcs-welcome` onboarding tour.
- **Installer:** Calamares OEM branding, optional LUKS2 (AES-XTS-512) full-disk encryption, `EDGE-8GB` / `LOWRAM-4GB` / `WORKSTATION-16GB` AI profiles.
- **Real GUIs:** every CLI app also has a Neural Glass window —
  `hcs-chat --gui` (chat + CPU Image Studio), `hcs-monitor --gui`,
  `hcs-control --gui`, `hcs-search --gui`, `hcs-diagnose --gui`, plus
  `hcs-docs` and `hcs-settings`. Three themes, ~11 MB RSS per app, rendered
  with a software rasteriser so it runs without a GPU.
  Built with [Slint](https://slint.dev) under the Royalty-free License 2.0.

**What v1.1.0 was verified against** (7 gates): `cargo fmt` clean ·
`clippy -D warnings` 0 warnings · `cargo test` 100 % · security audit 0 secrets ·
stress 0 crashes/OOM · production ISO built + ISO9660-verified ·
**VirtualBox 16/16 stages PASS** · GitHub Actions CI green.

> **Known gap in v1.1.0, corrected in v2.0.0:** the live ISO booted to a *text
> console*, not a graphical session, so the VirtualBox GUI stages could not
> produce real GUI pixels — the compositor was not even in the image. This is
> stated in `docs/V1_STABLE_QA_STATUS.md` and is the first thing v2 fixes.

---

## Key Characteristics & Engineering Targets

- **Local-First & Private:** All standard inference, cognitive memory indexing, and event logging occur locally on your machine. No telemetry or prompt forwarding to external clouds.
- **CPU-First Architecture:** Engineered to run smoothly on contemporary 64-bit multi-core CPUs using optimized AVX/AVX2 kernels without requiring a discrete high-end GPU.
- **Strict RAM Budget Targets:**
  - *Idle Baseline Target:* $\le$ 6 GB RAM
  - *Normal/Heavy Peak Target:* $\le$ 8 GB RAM
- **Deterministic Outcome Verification:** System actions and code execution are audited against objective evidence (exit codes, test suites, filesystem state, screenshot diffs) rather than uncritical model self-reports.
- **Reproducible Base:** Built on Debian 13 (Trixie) with live-build, systemd resource controls, Niri Wayland compositor, and Calamares system installer.

---

## System Architecture

```text
                       HCS BRAIN
                           │
             ┌─────────────┼─────────────┐
             │             │             │
          ROUTER        MEMORY        POLICY
             │             │             │
      ┌──────┼──────┐      │       capabilities
      │      │      │      │
   CHAT    CODE   REASON  RAG
      │      │      │      │
      └──────┼──────┴──────┘
             │
        PRIME AGENT
             │
      subagents / tools
             │
          verifier
             │
          outcome
             │
      judge / analyzer
             │
       lesson / memory
```

### Core Subsystems

| Subsystem | Binary / Service | Description |
|-----------|------------------|-------------|
| **Core Brain Daemon** | `hcsd` | Central IPC daemon listening on local Unix socket `/run/user/<uid>/hcsd.sock`. Manages task execution, routing, and lifecycle. |
| **Model Serving Daemon** | `hcs-modeld` | On-demand GGUF model loader and supervisor. Enforces single-heavy-model resident rules and active RAM RSS tracking. |
| **Memory Engine** | `hcs-memory` | Multi-class cognitive store (working, episodic, semantic, skill, preference) with SQLite FTS5 lexical matching and hybrid retrieval. |
| **Agent Runtime** | `hcs-agents` | Scoped subagent orchestrator with granular capability permissions (`filesystem.read`, `process.spawn`, etc.). |
| **Desktop Shell** | `hcs-shell` | Glass-morphic Wayland shell: bottom taskbar, Start Menu, cheatsheet HUD (Quickshell/QML). |
| **Security & Privacy** | `hcs-security` | Tor integration, NFTables isolation, Vault LUKS management, and amnesic live sessions. |
| **CPU Image Studio** | `hcs-image` | Offline pure-CPU text-to-image / image-to-image (SD 1.5 LCM Q4) with strict on-demand RAM lifecycle. |
| **GUI Foundation** | `hcs-ui` | Neural Glass widget kit, themes, headless render harness, offline docs viewer and settings. |
| **Action Registry** | `hcs-actions` | One table of everything the system can *do*: the omnibar, the CLI and agents all read it. Privileged actions require confirmation. |
| **File Manager** | `hcs-fm` | Tabs, search with visible filter pills, date narrowing, preview pane, explicit `Open With`, terminal hand-off. |
| **Terminal** | `hcs-term` | ANSI grid with a profile picker that lists the AI profiles. |
| **Screenshot** | `hcs-shot` | Region / window / screen capture, OCR text extraction, colour picker, screen record. |
| **Notes** | `hcs-notes` | Markdown with offline search and session restore. |
| **Update Manager** | `hcs-update` | Risk levels 1–5 (level 5 never automatic), pre-update snapshots, rollback from the boot menu. |
| **Session Persistence** | `hcs-persist` | Amnesic sessions (Tails guarantees) and opt-in LUKS persistent storage. |
| **Recall** | `hcs-recall` | Local encrypted timeline with a credential/payment filter applied *before* storage. |
| **Accessibility Gate** | `hcs-a11y-check` | Focus order, WCAG contrast, Reduce Motion, text scaling — a release gate. |
| **QA Agent** | `hcs-qa-agent` | Deterministic in-guest driver for the VirtualBox stage run. |

---

## Model Ecosystem & Roles

HCS Linux utilizes a specialized multi-role candidate pool:

- **HCS Controller (`Qwen3-0.6B`):** Resident or fast-loading intent classifier, tool selector, and short command synthesizer.
- **HCS Assistant (`Qwen3-1.7B`):** Interactive assistant for conversational tasks, planning, and memory summarization.
- **HCS Reasoner (`Qwen3-4B Q4_K_M`):** On-demand high-tier model for complex diagnostic and multi-step reasoning tasks.
- **HCS Coder (`Qwen2.5-Coder-1.5B`):** Local coding agent for workspace modifications, debugging, and configuration.
- **Memory Embeddings & Reranker (`Qwen3-Embedding-0.6B`, `Qwen3-Reranker-0.6B`):** Hybrid semantic memory search and candidate lesson ranking.

---

## Hardware Requirements

### v1.1.0 (current stable release)

These requirements are for **v1.1.0**, which is the only released image.

- **CPU:** 64-bit x86_64 with SSE4.2 / AVX support (4+ cores recommended)
- **RAM:** 8 GB DDR4/DDR5
- **Storage:** 32 GB free storage (SSD strongly recommended)
- **Display:** 1080p resolution (1920x1080)

### v2.0.0 (in development — read this before you try it)

v2.0.0 boots, but it **cannot draw a desktop yet**. The requirements below are
the intended target, not a verified configuration:

- **No GPU is required** — a CPU-only machine is the design goal, via the
  `lavapipe` software Vulkan implementation. That floor does not exist in the
  current build.
- **No GPU is supported either.** The image ships no Mesa, no Vulkan and no
  firmware. On real hardware you will get a kernel framebuffer, not a desktop.
- **VirtualBox will not work**, by design of both projects: VirtualBox's `umugfx`
  driver is not supported by Smithay and `niri` refuses it. Use `virtio-gpu`
  (QEMU/KVM) or bare metal when v2 is ready.
- **Physical hardware has never been tested.** No claim of hardware support is
  made anywhere in this repository.

See [`docs/V2_NEXT_STEPS_PLAN.md`](docs/V2_NEXT_STEPS_PLAN.md) for the graphics
work that closes these gaps.

### Recommended (Standard Profile)
- **CPU:** 8+ cores x86_64 with AVX2 / AVX-512
- **RAM:** 16 GB DDR4/DDR5
- **Storage:** 64 GB+ NVMe SSD
- **GPU:** Optional Vulkan-compatible iGPU or dGPU for offload acceleration

---

## Installation & Live USB

1. Download the verified ISO: `HCS-Linux-1.1.0-amd64.iso` from the [releases page](https://github.com/timfromhcs/hcs-linux/releases)
2. Verify the SHA-256 checksum:
   ```bash
   sha256sum -c SHA256SUMS
   ```
3. Write to a USB drive (replace `/dev/sdX` with your USB block device):
   ```bash
   sudo dd if=HCS-Linux-1.1.0-amd64.iso of=/dev/sdX bs=4M status=progress conv=fsync
   ```
4. Boot your computer from the USB drive. Select **HCS Linux 1.1.0 Live Desktop**, **…Live Desktop (Amnesic / Tor)**, or **Install HCS Linux (Calamares)**.

### Keyboard

The **HCS key** sits where the Windows key sits and does what it does, but it
is called HCS everywhere in the product. QWERTZ is the default layout; press
`HCS+Space` to cycle QWERTZ / EN-US / FR / ES / IT / GB. The HCS key is a
modifier, so no layout change can move it. Interface language is a separate
choice: `hcs settings locale fr`.

---

## Building from Source

Building HCS Linux utilizes Debian 13 (Trixie) live-build. On Windows hosts, the build runs within WSL2 ext4 storage.

```bash
# Clone the repository
git clone https://github.com/timfromhcs/hcs-linux.git
cd hcs-linux

# Prepare build dependencies and verify lockfiles
make fetch

# Build native core daemons
make native

# Run test and verification suites
make test

# Generate the hybrid bootable Live ISO
make iso

# Verify ISO structure, payload contract and checksum
make verify
make payload

# Every release gate, in order
make gate-all
```

`make gate-all` runs: `gate-static` (fmt, clippy `--all-targets`, tests) →
`gate-security` → `gate-supply` (pinned sources + the starter-model licence
report) → `gui` (render, regression and RAM across all four themes) → `a11y` →
`iso` → `verify` → `payload` → `gate-stress`.

`make payload` is the gate that would have caught v1 shipping an ISO whose
launchers, icons, wallpapers and manuals were silently missing: it verifies 112
payload invariants, including that every staged binary is a real ELF executable.

Individual pieces:

```bash
bash scripts/sync_shell.sh            # stage src/hcs-shell into the payload tree
bash scripts/sync_shell.sh --check    # fail if the staged copy is stale
python scripts/generate_qa_scenarios.py   # write the in-guest QA scenarios
python scripts/generate_icons.py          # regenerate the app icon set
python scripts/stage_starter_models.py --check   # which models may be baked
```

---

## Verification & Known Limitations

- **Evidence-based quality:** unit, integration, stress, security, retrieval and
  visual-regression suites, plus 13 release gates in `make gate-all`.
- **RAM budget:** measured, not estimated. Idle ≤ 6144 MB, peak ≤ 8192 MB,
  each GUI app ≤ 250 MB. `hcs-monitor --json` and the gate read the same numbers.
- **Retrieval honesty:** every `hcs rag` answer cites the manual section it came
  from, and a question outside the documented scope is refused rather than
  answered hopefully.
- **Threat model:** [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md) is a release
  gate. It states what is defended, against whom, and what is **not** protected.

### Known limitations

- Heavy 4B reasoning models require on-demand loading and are slower on older
  quad-core processors.
- Tor anonymity applies when the kill switch is engaged. It is off by default, and
  the tray shows the state at all times rather than hiding it.
- **Secure file deletion is not offered.** Overwriting is not reliable on SSDs and
  flash storage, so the recommended mitigations are: do not save the file, encrypt
  the volume, overwrite the whole device, or destroy it.
- Recall is local only and requires an explicit opt-in; it does not exist at all
  in an amnesic session, because a feature that silently records nothing is
  indistinguishable from one that records nothing useful.

### Known limitations of v2.0.0 (development build)

These are the ones that matter, and they are stated plainly because the previous
generations of this project overstated their own status:

- **No desktop.** The image boots to systemd and starts `niri`, but the image
  contains no graphics drivers, so nothing is drawn. This is a package-list gap,
  not a compositor bug.
- **No GPU support of any kind.** No Mesa, no Vulkan, no firmware, no kernel
  driver configuration. There is no accelerated path and no software path yet.
- **VirtualBox cannot run it.** VirtualBox's `umugfx` is unsupported by Smithay
  and the boot hangs during graphics initialisation. `virtio-gpu` (QEMU/KVM) or
  bare metal is required.
- **No working AI model.** The ISO bakes a `placeholder.gguf`, which is not a
  model. Local inference does not work.
- **The 32-stage VirtualBox suite has never run**, because there is no desktop
  to photograph.
- **Untested on real hardware.** Nothing in this repository is evidence of
  physical-hardware support.

The v1.1.0 live ISO booted to a text console because the compositor was not in
the image at all. v2.0.0 fixes that specific problem — the compositor is now in
the image and it starts — and then runs into the next one, which is that it has
nothing to talk to.

---

## License & Attribution

- **HCS-Owned Code:** Licensed under the [Apache License, Version 2.0](LICENSE).
- **Third-Party Software & Models:** Debian, llama.cpp, Niri, Quickshell, Calamares, and Qwen models retain their respective open source licenses as documented in [NOTICE](NOTICE) and `licenses/`.
