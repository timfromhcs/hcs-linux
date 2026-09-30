# HCS Linux — Master System Integration & Autonomous QA Plan
## UI/UX, Local LLM Inference, AI Agents, CLI Tools, Continuous Updater & VirtualBox Self-Healing Loop

**Product Codename:** Neural Glass  
**Base Distribution:** Debian 13 (Trixie) amd64  
**Target Profile:** EDGE-8GB (Idle RAM: $\le 6144\text{ MB}$, Peak RAM: $\le 8192\text{ MB}$, CPU-First)  
**Repository:** [https://github.com/timfromhcs/hcs-linux](https://github.com/timfromhcs/hcs-linux)  
**Document Revision:** 1.0 (Integration Architecture & Execution Specification)

---

## Executive Summary

HCS Linux is an AI-native, privacy-oriented, CPU-first operating system designed to operate as a coherent computing environment rather than a conventional desktop with an attached AI chatbot.

This master plan establishes the complete architecture and implementation blueprint for:
1. **Glassmorphic UI/UX Desktop:** Niri Wayland compositor with spring physics, Quickshell C++/Qt native desktop environment, fluid floating dock, status bar, and deep obsidian/cyan aesthetic.
2. **Branding & Visual Identity:** Vector logos, application icon set, 4K neural glass wallpaper, GRUB2 and Plymouth boot splash themes.
3. **CPU-First LLM Inference Engine:** `hcs-modeld` managing GGUF quantized models via llama.cpp with strict RAM budget enforcement ($\le 6\text{ GB}$ idle / $\le 8\text{ GB}$ peak) and single heavy resident model policy.
4. **AI Agents & Continual Brain:** `hcs-agents` runtime featuring the Prime Agent pipeline, least-privilege capability sandboxing, SQLite FTS5 hybrid memory (`hcs-memory`), and deterministic verification.
5. **Extra Core Applications:** `hcs-chat`, `hcs-search`, `hcs-control`, `hcs-security` (with Tor Private Mode and LUKS vault).
6. **Unified Developer CLI:** `hcs` multi-tool for terminal-first autonomous tasks, model telemetry, and memory queries.
7. **Continuous GitHub Updater:** `hcs-updater` daemon pulling directly from `origin/main`, verifying signatures, hot-reloading systemd units, and supporting atomic rollbacks.
8. **End-to-End Pure CPU VirtualBox Installation & Self-Healing Loop:** 100% automated installation from live ISO to VDI disk, post-install boot, visual framebuffer capture, Shannon entropy auditing, and an autonomous repair engine.

```
+-----------------------------------------------------------------------------------+
|                                  HCS LINUX OS                                     |
+-----------------------------------------+-----------------------------------------+
|                HCS SHELL                |                HCS BRAIN                |
|  - Niri Wayland (Scrollable Tiling)     |  - hcs-modeld (llama.cpp CPU Engine)    |
|  - Quickshell (Glass QML, Zero-Electron)|  - hcs-memory (SQLite FTS5 + Vectors)   |
|  - Neural Dock & Floating Top Bar       |  - hcs-agents (Prime Agent Pipeline)    |
|  - hcs-chat, hcs-search, hcs-control    |  - hcs-security (Tor & Vault)           |
+-----------------------------------------+-----------------------------------------+
|                             UNIFIED CLI & UPDATER                                 |
|  - hcs (Agent, Chat, Model, Memory, Tor, Audit)                                   |
|  - hcs-updater (Atomic Git pull from main -> Build -> Hot-reload -> Self-heal)   |
+-----------------------------------------------------------------------------------+
|                        DEBIAN 13 (TRIXIE) KERNEL & RUNTIME                        |
|  - Linux Kernel 7.0-generic, systemd, Wayland, PipeWire, zram, MBR/UEFI GRUB2     |
+-----------------------------------------------------------------------------------+
```

---

## 1. UI/UX & Desktop Architecture ("Neural Glass")

### 1.1 Philosophy & Runtime Budget Constraints
- **Zero-Electron Mandate:** Core shell components must never run Electron. Quickshell (C++/Qt6 QML) is used exclusively, consuming $< 180\text{ MB}$ RAM at idle.
- **Wayland-Native:** Driven by `niri`, an infinite horizontal scrollable-tiling Wayland compositor offering smooth spring animations, zero screen tearing, and dynamic window grouping.

### 1.2 Desktop Components
1. **HCS Top Bar (`src/hcs-shell/shell.qml`):**
   - **Height:** 48px, 12px margin, 12px radius, translucent `#161b22` (85% opacity), border `rgba(255,255,255,0.08)`.
   - **Left:** Brand Mark ("HCS LINUX") in glowing cyan (`#38bdf8`), active workspace pill indicators.
   - **Center:** Dynamic window title and active context tag.
   - **Right:** Cognitive Brain Telemetry pill (Active Model, resident RSS, green heartbeat indicator), Tor Privacy toggle status, Network/Audio sliders, and system clock.
2. **HCS Neural Dock:**
   - **Position:** Screen bottom center, floating, 64px height, 18px radius, frosted glass background (`rgba(22,27,34,0.85)`).
   - **Core App Pins:**
     - `Apps`: Application menu / Grid launcher.
     - `Chat`: Slide-out AI assistant (`hcs-chat`).
     - `Find`: Global search modal (`hcs-search`).
     - `Control`: Settings, telemetry & model switcher (`hcs-control`).
     - `Security`: Security Lab & Tor isolation control (`hcs-security`).
     - `Terminal`: Modern Wayland terminal (Foot / Alacritty).
3. **Visual Aesthetics ("Neural Glass"):**
   - **Background Palette:** Deep Obsidian (`#0d1117`), Slate Card (`#161b22`), Subtle Border (`#30363d`).
   - **Accent Palette:** Electric Cyan (`#38bdf8`), Deep Indigo (`#818cf8`), Emerald Success (`#34d399`), Amber Attention (`#fbbf24`), Crimson Alert (`#f87171`).
   - **Typography:** Inter Display for headings, JetBrains Mono for system metrics, code, and agent traces.

---

## 2. Branding, Logos & Visual Assets

### 2.1 Asset Catalog
All assets are managed in vector SVG format and pre-rendered into high-entropy PNGs:
- `assets/logo/hcs_logo.svg`: Primary brand mark—interconnected hexagonal neural synapses enclosing the clean "HCS" typography.
- `assets/icons/hcs-chat.svg`: Glass speech bubble with integrated neural lattice.
- `assets/icons/hcs-search.svg`: Floating optic lens with neural node focal point.
- `assets/icons/hcs-control.svg`: Precision dials and telemetry bar icon.
- `assets/icons/hcs-security.svg`: Shield emblem with internal Tor onion circuits.
- `assets/icons/hcs-updater.svg`: Circular synchronization loop with green verification tick.
- `assets/wallpapers/neural_glass_dark.png`: 3840x2160 abstract 4K neural glass artwork, designed with deep dark gradients to maximize readability and maintain high Shannon entropy (> 5.5).
- `assets/boot/grub_splash.png`: 1024x768 24-bit RGB graphical boot splash with HCS branding and status bar.
- `assets/boot/plymouth/`: Animated plymouth boot theme displaying the pulsing neural ring during kernel boot.

---

## 3. CPU-First LLM Inference Engine (`hcs-modeld`)

### 3.1 Model Catalog & Quantization (EDGE-8GB Profile)
All models use `Q4_K_M` GGUF quantization optimized for x86_64 CPU inference (AVX2 / AVX-512):

| Role | Model Checkpoint | Parameters | File Size | Peak RSS | Runtime Tier | License |
|---|---|:---:|:---:|:---:|---|:---:|
| **Controller** | `Qwen/Qwen3-0.6B-GGUF` | 0.6B | ~420 MB | ~550 MB | Resident (Always on) | Apache-2.0 |
| **Assistant** | `Qwen/Qwen3-1.7B-GGUF` | 1.7B | ~1.15 GB | ~1.45 GB | On-Demand (Default) | Apache-2.0 |
| **Coder** | `Qwen/Qwen2.5-Coder-1.5B-GGUF` | 1.5B | ~1.05 GB | ~1.35 GB | On-Demand (Coding) | Apache-2.0 |
| **Reasoner** | `Qwen/Qwen3-4B-GGUF` | 4.0B | ~2.50 GB | ~2.90 GB | On-Demand (Deep Reasoning)| Apache-2.0 |
| **Embeddings** | `Qwen/Qwen3-Embedding-0.6B-GGUF` | 0.6B | ~390 MB | ~500 MB | On-Demand (Indexing) | Apache-2.0 |
| **Ranker** | `Qwen/Qwen3-Reranker-0.6B-GGUF` | 0.6B | ~410 MB | ~520 MB | On-Demand (Retrieval) | Apache-2.0 |

### 3.2 Memory Budget Enforcement Engine
- **Idle Baseline Target:** $\le 6144\text{ MB}$ total system RAM.
  - OS + Desktop + Wayland: ~850 MB.
  - Core Daemons (`hcsd`, `hcs-memory`, `hcs-security`): ~220 MB.
  - Resident Controller (`Qwen3-0.6B`): ~550 MB.
  - Total Idle System RAM: **~1.62 GB** (Well within $\le 6\text{ GB}$ limit).
- **Peak Execution Target:** $\le 8192\text{ MB}$ total system RAM.
  - Single Heavy Resident Policy: Only ONE heavy generation model (Assistant 1.7B, Coder 1.5B, or Reasoner 4B) may reside in memory at any given time.
  - When a task requires the Reasoner (4B, ~2.9 GB RSS), `hcs-modeld` automatically unloads the Assistant before loading the Reasoner.
  - Total Peak RAM under maximum reasoning load: $1.62\text{ GB} + 2.90\text{ GB} = \mathbf{4.52\text{ GB}}$ (Comfortably below the $\le 8\text{ GB}$ limit).
- **CPU Optimization:** AVX2/AVX-512 thread pinning mapped to physical cores (`nproc`), zram compressed swap enabled via `systemd-zram-setup`.

---

## 4. AI Agents & Continual Brain Architecture (`hcs-agents` & `hcs-memory`)

### 4.1 Prime Agent Multi-Stage Pipeline
Every agentic instruction follows a deterministic 5-stage loop:
```
USER PROMPT / SYSTEM EVENT
           |
    [1. PLANNER] -----> Extracts goals, generates DAG subtasks, requests capabilities
           |
    [2. EXECUTOR] ----> Invokes tools within sandboxed least-privilege boundary
           |
    [3. VERIFIER] ----> Inspects exit codes, file hashes, test outputs, screenshots
           |
     [4. JUDGE] ------> Layered scoring (0-100 rubric, machine-verified JSON)
           |
    [5. ANALYZER] ----> Distills lessons, stores in TaskLedgerRecord and SQLite Memory
```

### 4.2 Sandboxed Least-Privilege Capabilities
- No agent is permitted to execute commands as root (`RootExecutionForbidden`).
- Granular capability matrix:
  - `FilesystemRead`: Whitelisted directories only (`/usr/share/hcs`, `~/.hcs`, project workspace).
  - `FilesystemWrite`: Restricted to project tree and `/tmp`.
  - `ProcessSpawn`: Non-root subprocess spawning with enforced timeout and resource cgroups.
  - `NetworkFetch`: Enforced Tor-only when Private Mode is engaged.
  - `FilesystemDelete` & `PackageInstall`: Hard-blocked without explicit interactive confirmation.

### 4.3 SQLite Cognitive Memory & Event Ledger (`hcs-memory`)
- **FTS5 Lexical + Vector Semantic Hybrid:** Fast full-text indexing joined with cosine similarity search across embeddings.
- **Append-Only Event Ledger:** Every task stores an immutable `TaskLedgerRecord` containing prompt hash, model revision, temperature, tool calls, stdout/stderr, exit codes, and peak RSS.
- **Contradiction Detection & Consolidation:** Automatically identifies stale facts and merges them into verified knowledge units.

---

## 5. Extra Programs & Core System Applications

### 5.1 `hcs-chat` (AI Companion)
- Graphical Wayland chat window built with native widgets.
- Real-time token streaming via IPC from `hcs-modeld`.
- Integrated mode switcher: General Assistance (1.7B), Code Mode (1.5B Coder), Deep Reason Mode (4B Reasoner).
- Inline markdown rendering, code block syntax highlighting, and one-click copy/execute.

### 5.2 `hcs-search` (Global Instant Finder)
- Activated globally via `Super + Space`.
- Hybrid indexing: Finds installed desktop applications, system settings, files, and queries the `hcs-memory` cognitive graph simultaneously.
- Sub-50ms keystroke response time.

### 5.3 `hcs-control` (Control & Settings Center)
- System telemetry monitor: Real-time CPU, RAM, and Model RSS meters.
- Hardware profile manager (selects between EDGE-8GB, LOWRAM-4GB, or WORKSTATION-16GB).
- Daemon status inspector (`hcsd`, `hcs-modeld`, `hcs-memory`, `hcs-security`, `hcs-updater`).

### 5.4 `hcs-security` (Security Lab & Privacy Vault)
- **Private Mode:** Instant one-click toggle routing all outbound traffic through Tor transparent proxy, wiping `/tmp` RAM disks, and disabling persistent telemetry.
- **Privacy Vault:** LUKS2-compatible AES-XTS-512 encrypted container management for user documents and keys.
- **Permission Inspector:** Displays active subagent capability tokens and audit logs.

---

## 6. Unified Developer CLI (`hcs`)

A single, statically compiled Rust binary installed to `/usr/bin/hcs`:
```bash
# Agent Workflows
hcs agent run "Refactor the authentication module and run unit tests"
hcs agent status <task-id>
hcs agent ledger --limit 10

# Model Operations
hcs model list
hcs model load qwen3-1.7b
hcs model unload qwen3-1.7b
hcs model infer "Explain systemd cgroups" --temperature 0.2

# Cognitive Memory
hcs memory search "How was the network configured?"
hcs memory stats

# Security & Privacy
hcs security tor enable
hcs security audit
hcs security status

# Continuous Updater
hcs update check
hcs update sync
```

---

## 7. Continuous GitHub Updater (`hcs-updater`)

### 7.1 Architecture & Sync Mechanism
- **Upstream Source:** `https://github.com/timfromhcs/hcs-linux.git` branch `main`.
- **Systemd Integration:** `hcs-updater.service` triggered by `hcs-updater.timer` every 4 hours or on-demand via `hcs update sync`.
- **Atomic 5-Step Pipeline:**
  1. **Fetch & Inspect:** Performs shallow git fetch (`git fetch origin main`) and extracts commit hash.
  2. **Integrity & Security Verification:** Verifies commit signatures, checks `vendor/locks/*.lock.yaml`, and ensures zero unredacted secrets exist before build.
  3. **Staged Compilation / Binary Unpack:** Compiles modified Rust crates in isolated workspace (`/var/cache/hcs/update-build`) or downloads verified release assets.
  4. **Atomic Binary Swap & Unit Reload:** Replaces binaries in `/usr/bin/` using atomic rename (`mv -T`) and reloads systemd user units:
     ```bash
     systemctl --user daemon-reload
     systemctl --user restart hcsd hcs-modeld hcs-memory
     ```
  5. **Automated Health Check & Rollback:** Runs smoke tests (`hcs model list`, `cargo test` on core crates). If any test fails, instantly rolls back to previous `/usr/bin/hcs.bak` binaries.

---

## 8. VirtualBox Pure CPU Installation & Test Harness

### 8.1 Pure CPU Virtualization Strategy
- Works on Windows 11 host with VirtualBox 7.x without requiring nested hardware virtualization.
- VM Specifications:
  - **RAM:** 8192 MB (Exact match to EDGE-8GB profile target).
  - **CPUs:** 4 vCPUs.
  - **Storage:** 20 GB dynamically allocated VDI hard disk (`target/hcs_installed_disk.vdi`).
  - **Graphics Controller:** VBoxSVGA with 128 MB VRAM and 2D acceleration.
  - **Firmware:** BIOS / MBR (GRUB2 i386-pc) with standard VESA/VBE framebuffer (1024x768).

### 8.2 Automated 12-Stage Test Sequence
1. `01_boot_live`: Boot Live ISO into GRUB2 bootloader menu.
2. `02_desktop_live`: Boot into graphical live desktop with Wayland / Niri and HCS init banner.
3. `03_launcher_search`: Trigger Super+Space launcher overlay and test search query.
4. `04_installer_welcome`: Launch Calamares installer GUI with HCS branding.
5. `05_installer_partitions`: Select target hard drive partition scheme (MBR / ext4 root).
6. `06_installer_profile`: Select hardware profile (`EDGE-8GB`).
7. `07_installer_progress`: Unpack SquashFS filesystem and configure bootloader.
8. `08_installer_finished`: Complete installation and issue poweroff.
9. `09_hdd_boot_splash`: Detach ISO medium; boot directly from installed VDI hard disk.
10. `10_installed_desktop`: Verify auto-login into installed glassmorphic desktop environment.
11. `11_postinstall_chat`: Launch `hcs-chat` and verify local LLM token streaming.
12. `12_postinstall_control`: Open `hcs-control`, audit system memory ($\le 6\text{ GB}$ idle), and test `hcs-updater`.

---

## 9. Visual QA & Verification Engine

### 9.1 Objective Framebuffer Auditing
- Screenshots are captured directly from VirtualBox virtual framebuffer via `VBoxManage controlvm <vm> screenshotpng <path>`.
- Automated Python audit (`scripts/verify_visual_qa.py`):
  - Resolution assertion: $\ge 1024 \times 768$.
  - Shannon Entropy threshold: $\ge 1.50$ (prevents black screens, solid white screens, or frozen framebuffers).
  - OCR / Keyword assertion: Verifies expected text strings ("HCS LINUX", "Brain Ready", "GRUB", "Installed").
  - Output report: [`qa/reports/install_qa_report.json`](file:///D:/HCSLINUX/qa/reports/install_qa_report.json).

---

## 10. Multi-Tiered Self-Healing Loop

The testing harness executes an autonomous self-healing iteration engine:

```
+-----------------------------------------------------------------------------------+
|                        AUTONOMOUS SELF-HEALING ENGINE                             |
+-----------------------------------------------------------------------------------+
|  [EXECUTE TEST STAGE]                                                             |
|           |                                                                       |
|  [CAPTURE ARTIFACTS] ---> Framebuffer Screenshot, Serial Log, RAM RSS, Exit Code  |
|           |                                                                       |
|  [EVALUATE GATES]                                                                 |
|     - Did command exit with 0?                                                    |
|     - Is Shannon entropy >= 1.5?                                                  |
|     - Is system RAM <= 6 GB idle / <= 8 GB peak?                                  |
|     - Are all cargo tests & clippy clean?                                         |
|           |                                                                       |
|     +-----+-----+                                                                 |
|     |           |                                                                 |
|  [PASS]      [FAIL]                                                               |
|     |           |                                                                 |
|  Advance    [CLASSIFY FAILURE MODE]                                               |
|  to next        1. Boot Panic / Kernel Hang                                       |
|  stage          2. Framebuffer Black Screen / Console Missing                     |
|                 3. Memory Budget Exceeded (> 8 GB)                                |
|                 4. Agent / CLI Execution Failure                                  |
|                 5. Updater Sync Mismatch                                          |
|                 |                                                                 |
|             [AUTONOMOUS CODE REPAIR]                                              |
|                 - Apply targeted patch to Rust crate, shell script, or config     |
|                 - Recompile with cargo build --release                            |
|                 - Rebuild ISO / Re-provision disk image                           |
|                 - Restart VM stage with backoff polling                           |
|                 |                                                                 |
|             [LOOP UNTIL 100% EVIDENCE RECORDED]                                   |
+-----------------------------------------------------------------------------------+
```

---

## 11. Implementation Roadmap & Execution Gates

| Phase | Milestone | Deliverables | Verification Gate |
|:---:|---|---|---|
| **Phase 1** | **UI/UX & Shell Assets** | Quickshell QML shell, Niri config, SVG logos, PNG icons, 4K wallpaper, GRUB splash. | `verify_visual_qa.py` $\ge 1.5$ entropy, clean QML load. |
| **Phase 2** | **CPU Inference & Budget** | `hcs-modeld` llama.cpp GGUF engine, single-heavy eviction logic, RAM profiler. | RAM $\le 6\text{ GB}$ idle / $\le 8\text{ GB}$ peak, unit tests pass. |
| **Phase 3** | **Agents & Continual Memory** | `hcs-agents` Prime Agent pipeline, sandboxed capabilities, `hcs-memory` FTS5 SQLite. | 13/13 tests pass, capability denial enforced. |
| **Phase 4** | **Applications & CLI** | `hcs` CLI binary, `hcs-chat`, `hcs-search`, `hcs-control`, `hcs-security`. | Zero clippy warnings, CLI subcommands functional. |
| **Phase 5** | **Continuous Updater** | `hcs-updater` daemon, systemd unit, atomic pull from `origin/main`, auto-rollback. | Atomic sync passes, unit reload verified. |
| **Phase 6** | **VirtualBox VM Harness** | PowerShell end-to-end 12-stage installation script with active self-healing. | 12/12 stages PASS, 12 non-blank screenshots. |
| **Phase 7** | **Final Quality Audit & Release** | Full security audit (0 secrets), stress suite (100 cycles), SPDX SBOM, GitHub Release. | Clean audit report, Git pushed to main. |

---

## 12. Verification & Safety Commitments (GEMINI.md Compliance)

1. **Zero Manufactured Evidence:** No gate is marked PASS without real captured evidence (process exit codes, logs, screenshots, checksums).
2. **Absolute Secret Protection:** Zero API keys, personal access tokens, or private emails committed to source files. `run_security_audit.py` must pass unconditionally.
3. **Reproducibility:** All package dependencies and model revisions are strictly locked in `vendor/locks/`.
4. **License Compliance:** All bundled components and models adhere strictly to permissive licenses (Apache-2.0 / MIT).
