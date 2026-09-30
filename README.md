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
| **Desktop Shell** | `hcs-shell` | Glass-morphic Wayland shell interface designed for Niri and Quickshell. |
| **Security & Privacy** | `hcs-security` | Tor integration, NFTables isolation, Vault LUKS management, and amnesic live sessions. |

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

### Minimum (Edge Profile)
- **CPU:** 64-bit x86_64 with SSE4.2 / AVX support (4+ cores recommended)
- **RAM:** 8 GB DDR4/DDR5
- **Storage:** 32 GB free storage (SSD strongly recommended)
- **Display:** 1080p resolution (1920x1080)

### Recommended (Standard Profile)
- **CPU:** 8+ cores x86_64 with AVX2 / AVX-512
- **RAM:** 16 GB DDR4/DDR5
- **Storage:** 64 GB+ NVMe SSD
- **GPU:** Optional Vulkan-compatible iGPU or dGPU for offload acceleration

---

## Installation & Live USB

1. Download the verified ISO: `HCS-Linux-0.1.0-alpha.1-amd64.iso`
2. Verify the SHA-256 checksum:
   ```bash
   sha256sum -c SHA256SUMS
   ```
3. Write to a USB drive (replace `/dev/sdX` with your USB block device):
   ```bash
   sudo dd if=HCS-Linux-0.1.0-alpha.1-amd64.iso of=/dev/sdX bs=4M status=progress conv=fsync
   ```
4. Boot your computer from the USB drive. Select **HCS Live Desktop** or launch the Calamares installer to install HCS Linux to your hard drive.

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

# Verify ISO structure and checksum
make verify
```

---

## Verification & Known Limitations

- **Evidence-Based Quality:** HCS Linux is continually audited across unit, integration, stress, and visual regression suites.
- **RAM Budget Monitoring:** Real-time RSS monitoring enforces that idle RAM does not exceed 6 GB and peak does not exceed 8 GB under standard Edge profiles.
- **Known Limitations:**
  - Heavy 4B reasoning models require on-demand loading and may exhibit higher token generation latency on older quad-core processors.
  - Full Tor anonymity is restricted to designated Private Mode workspaces and Tor Browser sessions; standard network traffic is subject to conventional firewall and local DNS rules.

---

## License & Attribution

- **HCS-Owned Code:** Licensed under the [Apache License, Version 2.0](LICENSE).
- **Third-Party Software & Models:** Debian, llama.cpp, Niri, Quickshell, Calamares, and Qwen models retain their respective open source licenses as documented in [NOTICE](NOTICE) and `licenses/`.
