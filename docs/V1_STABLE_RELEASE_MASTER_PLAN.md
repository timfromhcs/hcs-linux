# HCS Linux — Master Roadmap & Engineering Blueprint to v1.0.0 Stable Release ISO

> **Product Target:** HCS Linux v1.0.0 Stable Release ISO (Bootable, Installable, Reproducible, Local-First AI Operating System)  
> **Base Distribution:** Debian 13.7 (Codename `trixie`) amd64 | Linux Kernel 6.12+ LTS  
> **Desktop Environment:** Neural Glass 3.0 (Quickshell QtQuick / Niri Wayland Compositor)  
> **Resource Budget (Strict):** $\le 6144\text{ MB}$ RAM Idle | $\le 8192\text{ MB}$ RAM Peak Heavy Inference  
> **Repository:** [https://github.com/timfromhcs/hcs-linux](https://github.com/timfromhcs/hcs-linux)  
> **Document Status:** Active Master Plan for Public v1.0.0 Stable ISO Download  

---

## 1. Vision & Release Objective

HCS Linux v1.0.0 is the definitive **AI-native, privacy-oriented, CPU-first desktop operating system**. It bridges the gap between consumer elegance ("Silicon Valley Grade" glassmorphic UI, familiar Windows-like taskbar and start menu controls) and deep technical power (curated native pentesting/security lab, offline pure-CPU text-to-image/image-to-image generation, full developer workflow, and fail-closed Tor anonymity).

```
                      +-------------------------------------------------+
                      |             HCS LINUX v1.0.0 STABLE             |
                      +-------------------------------------------------+
                                               |
                  +----------------------------+----------------------------+
                  |                                                         |
         NEURAL GLASS 3.0 DESKTOP                                  HCS COGNITIVE CORE
   * Windows-like Taskbar & Tray                             * Single-Model-Resident Rule
   * Modern Silicon Valley Start Menu                        * Qwen3-0.6B Resident Controller
   * StatusNotifierItem (PipeWire, Net)                      * Qwen3-1.7B / 4B On-Demand Reasoner
   * Super+/ Hotkey Cheatsheet HUD                           * Native Rust MCP Server Suite
   * Dynamic Obsidian/Titanium Theme                         * SQLite FTS5 Memory Graph
                  |                                                         |
                  +----------------------------+----------------------------+
                                               |
                                     SYSTEM CAPABILITIES
                  +----------------------------+----------------------------+
                  |                                                         |
           CREATIVE & DEV                                            SECURITY & LAB
   * CPU Image Gen (sd.cpp LCM Q4)                           * Curated Debian Pentest Suite
   * Txt2Img & Img2Img in <= 2GB RAM                         * System-Wide Tor nftables Proxy
   * Coding/Testing/Debugging Suite                          * 1-Click Taskbar Tor Killswitch
   * Preconfigured LSP & VSCodium                            * Hardened Tor Browser Bundle
   * Offline Docs & AI Onboarding                            * Autonomous Security Agent
                  |                                                         |
                  +----------------------------+----------------------------+
                                               |
                                 DEBIAN 13 TRIXIE BASE (LTS)
```

---

## 2. Core Pillars of the v1.0.0 Stable Release

### Pillar 1: Silicon Valley Grade UI/UX with Windows-Like Controls
* **Ergonomic Bottom Taskbar:**
  * **Brand Start Monogram:** Neural hexagonal shield icon; pressing the `Super` (Windows) key or clicking opens the Start Menu.
  * **Active Tasklist:** Running application icons with glowing focus indicators, window grouping, minimizable/maximizable tabs, and hover thumbnail previews.
  * **Integrated System Tray (StatusNotifierItem / SNI):** Native PipeWire audio master slider, Wi-Fi/Ethernet network selector, battery/power profile indicator, Tor Killswitch status pill, clock with drop-down calendar, and Action Center button.
* **Modern Start Menu (Silicon Valley Grade):**
  * Hybrid Spotlight / Start Menu layout.
  * Omnibar search input: Typing instantly queries installed apps, local files, or initiates a direct prompt to HCS Brain (`"Ask Brain..."`).
  * Pinned core applications (HCS Chat, Code Studio, Security Lab, CPU Image Studio, Files, Terminal).
  * Recent files and cognitive memories.
  * Power actions: Sleep, Lock, Restart, Power Off.
* **Window Management & Hotkeys:**
  * Familiar Windows-like keyboard control: `Alt+Tab` graphical window switcher, `Super+D` show desktop, `Super+E` open File Manager, `Super+L` lock screen, `Super+Left/Right` window snap.
  * Global Hotkey Cheatsheet HUD: Pressing `Super + /` or `F1` dims the screen and displays a sleek glassmorphic cheatsheet overlay with all shortcuts and gestures.
* **First-Boot Experience (`hcs-welcome`):**
  * Beautiful onboarding walkthrough introducing AI agents, Tor privacy toggle, and offline documentation.

### Pillar 2: Custom Brand Identity & Visual Assets
* **Geometric Neural Logo:** Official HCS Linux emblem—an obsidian hexagonal shield with a glowing cyan neural core and vector SVG typography.
* **Silicon Valley Wallpaper Suite:**
  * Default: *Obsidian Neural Gradient* (Deep charcoal `#0b0f17`, glowing cyan/indigo waves).
  * Alternate: *Frosted Titanium* (Architectural light glass for daylight productivity).
  * Security Lab: *Cybernetic Stealth* (Monochrome slate with violet Tor accents).
* **Plymouth Boot Splash Theme:** Clean, flicker-free boot experience showing the glowing HCS monogram smoothly cross-fading into the Niri/Quickshell Wayland desktop.
* **Consistent Iconography:** Curated SVG icon pack covering all system tools, launchers, and third-party apps.

### Pillar 3: Offline CPU Text-to-Image & Image-to-Image Generation (`hcs-image`)
* **Underlying Engine:** Statically linked C++ runtime powered by `stable-diffusion.cpp` (AVX2 and AVX-512 vector acceleration enabled).
* **Target Model Quantization:**
  * **SD 1.5 LCM (Latent Consistency Model)** or **SD-Turbo** quantized to `q4_0` / `q8_0` GGUF.
  * Disk footprint: $\approx 1.6\text{ GB}$.
  * Execution RAM: $\approx 1.8 - 2.2\text{ GB}$ peak RSS on pure CPU.
* **Performance Benchmark:**
  * 512x512 image generation in 4 to 8 sampling steps.
  * Execution time: 15–25 seconds on modern quad-core x86_64 CPU without GPU.
* **Modes Supported:**
  * **Text-to-Image (txt2img):** High-fidelity prompt rendering with negative prompt filtering and guidance scale.
  * **Image-to-Image (img2img):** Input image modification, style transfer, and canvas sketch refinement.
* **Memory Lifecycle & Budget Protection:**
  * **Strict On-Demand Rule:** The image model is NEVER kept resident in memory.
  * Lifecycle: Allocate buffer $\rightarrow$ Load quantized weights $\rightarrow$ Infer $\rightarrow$ Save PNG $\rightarrow$ Completely deallocate weights $\rightarrow$ Reclaim buffer.
  * Peak memory during generation is guaranteed to stay within the $\le 8192\text{ MB}$ operating budget.
* **Frontend Access:**
  * CLI tool: `/usr/bin/hcs image "a sleek futuristic cybernetic workstation in neon glass" --steps 6 -o render.png`
  * GUI Studio: Dedicated "Image Studio" tab in `hcs-chat` with real-time prompt feedback, seed controls, and gallery preview.

### Pillar 4: Curated Security Lab, Pentest & Hacking/Cracking Suite
* **Strict Debian 13 Purity:** All tools are sourced directly from Debian 13.7 `trixie` repositories and pinned upstream releases—**no Kali repository pollution**, preserving Debian stability and reproducibility.
* **Curated Tool Arsenal:**
  * **Network & Discovery:** `nmap`, `wireshark` / `tshark` (configured for non-root packet capture), `tcpdump`, `socat`, `masscan`, `dnsutils`.
  * **Web & API Assessment:** `sqlmap`, `nikto`, `gobuster`, `ffuf`, `nuclei`, `curl`, `wget`.
  * **Password Auditing & Cracking:** `john` (John the Ripper), `hashcat`, `hydra`, `aircrack-ng`.
  * **Reverse Engineering & Binary Triage:** `gdb`, `lldb`, `strace`, `ltrace`, `binwalk`, `radare2`.
* **Autonomous AI Security Agent (`hcs-agent --role pentester`):**
  * Integrated with the native `hcs-mcp` bash and scanner tools.
  * Capable of running structured network audits, inspecting configuration files, discovering misconfigurations, and drafting vulnerability remediation reports.
  * Enforces Human-in-the-Loop confirmation before executing offensive payloads or port sweeps.
* **Stealth & Privacy Guard:**
  * Random MAC address spoofing on network interface activation.
  * Anti-tracking hosts file blocking known surveillance and ad-tech telemetry.

### Pillar 5: Tor Transparent Network Proxy & Tor Browser Integration
* **One-Click Taskbar Switch:**
  * "TOR SHIELD" button located directly in the system tray and Control Drawer.
  * Clicking activates system-wide transparent isolation in $< 200\text{ ms}$.
* **Fail-Closed `nftables` Architecture:**
  * Intercepts all outgoing TCP traffic and redirects it to Tor `TransPort 9040`.
  * Intercepts all UDP/TCP DNS queries (port 53) and redirects them to Tor `DNSPort 9053`—**zero DNS leaks**.
  * Restricts direct internet access exclusively to the `debian-tor` system user (`skuid debian-tor accept`).
  * Drops all IPv6 traffic or routes IPv6 through Tor onion circuits if configured.
  * **Hardware Killswitch:** If the Tor service terminates or loses circuit connectivity, `nftables` immediately drops all outgoing traffic to prevent IP disclosure.
* **Tor Browser Bundle:**
  * Pre-packaged, hardened Tor Browser (`torbrowser-launcher`) with fingerprint randomization, letterboxing, and DuckDuckGo/SearXNG onion search defaults.

### Pillar 6: Complete Developer Workflow (Coding, Testing, Debugging)
* **Pre-Configured Runtimes & Compilers:**
  * Rust (`rustc`, `cargo` latest stable).
  * Python 3.12 (with pip, virtualenv, and standard libraries).
  * Node.js / TypeScript runtime (`fnm` lightweight manager).
  * Git with pre-configured aliases and safe credential helpers.
* **Developer Editors & Tools:**
  * **Neovim (HCS Edition):** Pre-configured with LSP (Rust-Analyzer, Pyright), Treesitter, and native HCS MCP Agent code completion.
  * **VSCodium / VS Code OSS:** Zero-telemetry lightweight build equipped with Rust Analyzer, Python, and the HCS Linux MCP Extension.
* **Unified Developer CLI (`hcs dev`):**
  * `hcs dev init [rust|python|node]`: Instant project scaffolding with CI templates and git pre-commit hooks.
  * `hcs dev test`: Runs unit, integration, and lint checks inside a sandboxed container.
  * `hcs dev debug <binary>`: Wraps GDB/LLDB with terminal visualization.

### Pillar 7: Comprehensive UI/UX Help Documents & Offline Manuals
* **Built-in Offline Documentation Browser (`hcs-docs`):**
  * Full graphical documentation viewer accessible from Start Menu and command line.
  * Contains complete user guides:
    1. *Getting Started with HCS Linux*
    2. *Mastering the AI Brain & MCP Agents*
    3. *CPU Image Generation Studio Guide*
    4. *Security Lab & Ethical Pentesting Playbook*
    5. *Tor Privacy, Anonymity & OpSec Guidelines*
    6. *Developer SDK & Custom MCP Tool Creation*
* **Interactive AI Help:** Users can highlight any paragraph in the docs and click "Explain with HCS Brain" for instant contextual clarification without requiring an internet connection.

### Pillar 8: Calamares OEM Installer & Production ISO Polish
* **Calamares OEM System Installer:**
  * Custom Silicon Valley HCS branding slides during installation.
  * Automatic partitioning with optional **LUKS2 Full-Disk Encryption** (AES-XTS-512) and TPM2 unsealing.
  * Hardware detection: Automatically chooses between `EDGE-8GB`, `LOWRAM-4GB`, or `WORKSTATION-16GB` AI profile.
  * Timezone, locale, keyboard layout, and user account creation.
* **Hybrid Bootable ISO Image:**
  * BIOS (i386-pc) and UEFI (x86_64-efi) bootable with GRUB2.
  * Compressed SquashFS root using XZ compression.
  * ISO verifiable with SHA256SUMS and cryptographic GPG signatures.

---

## 3. Engineering Phases to v1.0.0 Stable Release

```
+-------------------------------------------------------------------------------------------------+
|                                 v1.0.0 IMPLEMENTATION ROADMAP                                   |
+-------------------------------------------------------------------------------------------------+
|                                                                                                 |
|  [PHASE 1] Silicon Valley Taskbar, Start Menu & Custom Branding Assets                          |
|    - Implement bottom taskbar (PanelWindow) with start button, tasklist & SNI tray              |
|    - Implement Silicon Valley Start Menu with omnibar search & HCS Brain prompt                 |
|    - Design SVG logos, Silicon Valley wallpaper pack, Plymouth splash theme                     |
|                                                                                                 |
|  [PHASE 2] Offline CPU Text-to-Image / Image-to-Image Generation (hcs-image)                    |
|    - Vendor stable-diffusion.cpp with AVX2 CPU optimizations                                    |
|    - Package SD 1.5 LCM / SD-Turbo Q4_0 quantized weights (<= 2 GB RAM)                        |
|    - Build /usr/bin/hcs-image CLI and "Image Studio" GUI in hcs-chat                            |
|                                                                                                 |
|  [PHASE 3] Curated Security Lab, Pentest Arsenal & Tor Network Killswitch                       |
|    - Add Debian 13 security packages (nmap, wireshark, john, hashcat, hydra, sqlmap)            |
|    - Implement fail-closed nftables transparent Tor proxy script & taskbar toggle               |
|    - Package and test hardened Tor Browser launcher integration                                 |
|                                                                                                 |
|  [PHASE 4] Complete Developer Suite & Offline Help Documentation Center                         |
|    - Configure Rust, Python, Node, Neovim, and VSCodium developer workflows                     |
|    - Build hcs-docs offline browser and Super+/ cheatsheet overlay                              |
|    - Create hcs-welcome first-boot onboarding tour                                              |
|                                                                                                 |
|  [PHASE 5] Calamares OEM Installer Polish & LUKS2 Full-Disk Encryption                          |
|    - Polish Calamares branding QML and partitioning modules                                     |
|    - Configure LUKS2 encryption and automated user provisioning                                 |
|                                                                                                 |
|  [PHASE 6] Automated End-to-End Testing (VirtualBox Pure CPU & Cloud CI)                        |
|    - Validate complete 16-stage VirtualBox installation and visual QA                          |
|    - Stress test simultaneous RAM budget (Taskbar + AI Inference + Image Gen)                   |
|    - Verify GitHub Actions CI build on dev branch                                               |
|                                                                                                 |
|  [PHASE 7] Final Production ISO Build, Signing & Release Publication                            |
|    - Generate production ISO (HCS-Linux-1.0.0-amd64.iso)                                        |
|    - Generate SPDX 2.3 SBOM, SHA256SUMS, and release notes                                      |
|    - Open Pull Request and publish v1.0.0 Stable Release                                        |
|                                                                                                 |
+-------------------------------------------------------------------------------------------------+
```

---

## 4. Detailed Component Technical Specifications

### 4.1 Taskbar & Start Menu Specification (`src/hcs-shell/taskbar.qml` & `start_menu.qml`)

#### Taskbar Architecture:
* **Geometry:** Anchored to bottom edge (`anchors.bottom: true`, `anchors.left: true`, `anchors.right: true`), height $52\text{ px}$.
* **Background:** Acrylic frosted obsidian (`#0d1117`, opacity 0.90) with subtle upper border (`rgba(255, 255, 255, 0.08)`).
* **Left Section:**
  * Start Monogram Button: Obsidian rounded tile with glowing cyan HCS emblem ($36\times 36\text{ px}$). Clicking toggles Start Menu.
  * Desktop Search Pill: Quick prompt bar reading `"Search apps or ask AI..."`.
* **Center Section:**
  * Running Tasks Container: Dynamically populated from Wayland toplevel foreign window protocol.
  * Active window highlighted with cyan underline indicator ($2\text{ px}$).
  * Clicking active window minimizes it; clicking inactive window brings it to focus.
* **Right Section (System Tray & Control Cluster):**
  * Tor Shield Pill: Displays Tor status (Green = Active/Isolated, Gray = Direct). Clicking toggles killswitch.
  * AI Cognitive Footprint: Displays active model and RAM (`Qwen3-0.6B | 550MB`).
  * Audio Volume Icon: Hover/click reveals PipeWire master slider.
  * Network Indicator: Shows Wi-Fi SSID or Ethernet link.
  * Clock & Calendar: Formatted as `HH:mm \n dd.MM.yyyy`; clicking opens interactive month calendar.
  * Notification / Action Center Bell: Badge counter for unread notifications.

#### Start Menu Architecture:
* **Geometry:** Anchored bottom-left above taskbar ($560\text{ px}$ width, $640\text{ px}$ height, rounded corners $18\text{ px}$).
* **Top Header:** Omnisearch input with auto-completion and instant fuzzy matching.
* **Middle Section:**
  * Grid of Pinned Applications: 4 columns of high-res SVG app tiles (HCS Chat, Image Studio, Security Lab, Files, Settings, Terminal, VSCodium, Tor Browser).
  * Recent Files & Knowledge Nodes: Quick access to recent documents and cognitive memory entries.
* **Bottom Footer:**
  * User profile avatar and username.
  * Action controls: Lock, Sleep, Restart, Power Off.

---

### 4.2 CPU Image Generation Engine Specification (`src/hcs-image/`)

#### Model Configuration:
* **Primary Checkpoint:** Stable Diffusion 1.5 LCM (Latent Consistency Model) converted to GGUF `q4_0`.
* **Resolution:** $512\times 512$ default (configurable to $256\times 256$, $512\times 768$).
* **Sampling Steps:** 4–8 steps (LCM requires only 4–8 steps compared to 30–50 steps of standard DDIM/Euler).
* **Guidance Scale (CFG):** 1.5–2.0.

#### Binary Architecture:
```text
src/hcs-image/
├── Cargo.toml
├── src/
│   ├── main.rs          # CLI entry point (/usr/bin/hcs-image)
│   ├── engine.rs        # Wrapper spawning /usr/lib/hcs/sd-cpp backend
│   ├── memory_guard.rs  # Strict RAM budget verification prior to inference
│   └── postprocess.rs   # Image optimization & metadata tagging
└── vendor/
    └── sd-cpp/          # Pinned stable-diffusion.cpp source (CMake build)
```

#### Memory Management Lifecycle:
1. `hcs-image` inspects current system RAM via `/proc/meminfo`.
2. If free RAM $< 2500\text{ MB}$, triggers automatic garbage collection and signals `hcs-modeld` to flush caches.
3. Loads quantized model weights via `mmap` directly into buffer.
4. Executes diffusion sampling with multi-threaded AVX2 instructions (`-t $(nproc)`).
5. Writes target PNG to output path.
6. Terminates inference process, completely releasing all virtual and physical memory.
7. Total RAM peak verified: $\le 2.2\text{ GB}$ (leaving $> 3.8\text{ GB}$ for system and background tasks).

---

### 4.3 Security Lab & Curated Pentest Suite Specification

#### Debian 13 Package List (`config/package-lists/hcs-security.list.chroot`):
```text
# Network Reconnaissance & Sniffing
nmap
wireshark
tshark
tcpdump
socat
masscan
netcat-traditional
dnsutils

# Web & API Security
sqlmap
nikto
gobuster
ffuf
curl
wget

# Password Auditing & Cracking
john
hashcat
hydra
aircrack-ng

# Binary Analysis & Debugging
gdb
lldb
strace
ltrace
binwalk

# Cryptography & Steganography
steghide
qrencode
openssl
```

#### Transparent Tor Proxy & Killswitch Script (`/usr/bin/hcs-tor-switch`):
* **Enable Flow:**
  1. Checks if `tor` system service is running (`systemctl is-active tor`).
  2. Applies `nftables` redirection rules:
     ```bash
     nft add table ip hcs_tor
     nft add chain ip hcs_tor output { type nat hook output priority -100 \; }
     nft add rule ip hcs_tor output skuid debian-tor counter accept
     nft add rule ip hcs_tor output ip daddr 127.0.0.0/8 counter accept
     nft add rule ip hcs_tor output ip protocol udp th dport 53 counter redirect to :9053
     nft add rule ip hcs_tor output ip protocol tcp counter redirect to :9040
     ```
  3. Updates DNS resolver in `/etc/resolv.conf` to `nameserver 127.0.0.1`.
  4. Tests circuit reachability (`hcs-security --check-tor`).
  5. Emits D-Bus notification: `"Tor Transparent Mode Active — All traffic anonymized."`
* **Disable Flow:**
  1. Flushes table `nft delete table ip hcs_tor`.
  2. Restores standard NetworkManager DNS configuration.
  3. Emits D-Bus notification: `"Tor Disabled — Direct networking restored."`

---

### 4.4 Documentation & Help System Specification (`src/hcs-docs/`)

#### Structure of `/usr/share/hcs/docs/`:
```text
/usr/share/hcs/docs/
├── index.html                    # Offline Silicon Valley style documentation portal
├── assets/
│   ├── css/theme.css             # Obsidian dark / frosted titanium styles
│   └── js/search.js              # Lunr.js instant offline client-side search
├── manuals/
│   ├── 01_getting_started.md     # Installation, desktop tour, and basics
│   ├── 02_ai_brain_guide.md      # Prompting, model tiers, and MCP tools
│   ├── 03_cpu_image_studio.md    # Txt2img, img2img, and prompt engineering
│   ├── 04_security_pentest.md    # Tool usage, ethical hacking, and audits
│   ├── 05_tor_anonymity.md       # OpSec, transparent proxy, and leaks
│   └── 06_developer_manual.md    # Compiling, building plugins, and MCP
└── cheatsheet.json               # Keybindings database for Super+/ HUD
```

#### Global Cheatsheet HUD (`src/hcs-shell/cheatsheet.qml`):
* Triggered by `Super + /` or `F1`.
* Full-screen frosted acrylic backdrop (`blur: 32px`).
* Organizes keybindings into clean cards:
  * **System & Window Control:** `Super` (Start Menu), `Super+D` (Show Desktop), `Super+Q` (Close Window), `Alt+Tab` (Window Switcher).
  * **HCS Applications:** `Super+Return` (HCS Chat), `Super+Space` (Search), `Super+M` (Monitor), `Super+I` (Image Studio), `Super+T` (Terminal).
  * **Privacy & Security:** `Super+Alt+T` (Tor Killswitch Toggle), `Super+V` (Redacted Clipboard).

---

## 5. Non-Negotiable RAM Budget Verification Matrix

Every feature in the v1.0.0 Stable Release must rigorously comply with the RAM budget defined in GEMINI.md Section 2:

| State / Activity | Active Components | Measured Target RAM | Hard Cap |
|---|---|---|---|
| **System Idle Baseline** | Niri + Quickshell (Taskbar + Tray) + `hcsd` | $\approx 480\text{ MB}$ | $\le 6144\text{ MB}$ |
| **Idle with Resident AI Controller** | Baseline + `hcs-modeld` (Qwen3-0.6B Q4_K_M) | $\approx 1050\text{ MB}$ | $\le 6144\text{ MB}$ |
| **Active Chat & Assistant Inference** | Baseline + `hcs-modeld` (Qwen3-1.7B Q4_K_M) | $\approx 2200\text{ MB}$ | $\le 6144\text{ MB}$ |
| **Active Coding / Hard Reasoning** | Baseline + `hcs-modeld` (Qwen3-4B Q4_K_M) | $\approx 4100\text{ MB}$ | $\le 8192\text{ MB}$ |
| **CPU Text-to-Image Generation** | Baseline + `hcs-image` (SD 1.5 LCM Q4_0) | $\approx 3100\text{ MB}$ | $\le 8192\text{ MB}$ |
| **Heavy Multitasking Peak** | Baseline + Chat (1.7B) + Tor Browser + VSCodium | $\approx 5400\text{ MB}$ | $\le 8192\text{ MB}$ |

> [!IMPORTANT]
> **Single Heavy Model Rule Enforced:** If a user initiates CPU Image Generation while a 4B reasoner is resident, `hcs-modeld` automatically unloads the 4B model into idle swap/cache, runs the image generation, and restores the LLM state only when complete. At no point do two heavy models occupy RAM simultaneously.

---

## 6. End-to-End Test & Verification Protocol

Before publishing the final v1.0.0 Stable Release ISO, the build must pass the following 7 gates:

### Gate 1: Workspace Build & Static Analysis
```bash
cargo fmt --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```
*Threshold:* 0 warnings, 0 errors, 100% test pass rate across all crates.

### Gate 2: Security & License Audit
```bash
python scripts/run_security_audit.py
python scripts/verify_sources.py
```
*Threshold:* 0 unredacted secrets or credentials, 100% pinned upstream sources with valid SPDX licenses.

### Gate 3: System Stress Suite
```bash
python scripts/run_stress_test.py
```
*Threshold:* 1000 memory transactions, 100 model transitions, 0 OOM crashes, 0 file descriptor leaks.

### Gate 4: Production ISO Generation
```bash
wsl bash scripts/build_iso.sh 1.0.0 amd64
```
*Threshold:* Hybrid ISO generated, valid ISO9660 volume descriptor, verified SHA256 checksums in `dist/SHA256SUMS`.

### Gate 5: VirtualBox Pure-CPU 16-Stage Installation & Visual QA
```powershell
powershell -ExecutionPolicy Bypass -File scripts/qa_virtualbox_install.ps1 -IsoPath "dist/HCS-Linux-1.0.0-amd64.iso"
```
*16 Verified Stages:*
1. `01_grub_boot_splash.png`: Bootloader with HCS branding.
2. `02_desktop_baseline.png`: Silicon Valley Neural Glass taskbar and desktop.
3. `03_start_menu_open.png`: Start Menu with omnibar search and app grid.
4. `04_calamares_welcome.png`: Installer initial screen.
5. `05_calamares_partitioning.png`: Partitioning target disk with optional LUKS2.
6. `06_calamares_profile.png`: Profile selection (`EDGE-8GB`).
7. `07_calamares_installing.png`: SquashFS extraction.
8. `08_calamares_complete.png`: Installation complete.
9. `09_installed_hdd_boot.png`: First boot from installed VDI disk.
10. `10_installed_desktop.png`: Booted installed desktop.
11. `11_start_menu_search.png`: Start Menu search filtering.
12. `12_cheatsheet_hud.png`: Super+/ hotkey overlay.
13. `13_image_studio_render.png`: CPU image generation execution.
14. `14_tor_killswitch_active.png`: Tor transparent isolation active in tray.
15. `15_security_lab_nmap.png`: Native security tool run in terminal.
16. `16_hcs_docs_browser.png`: Offline documentation viewer.

*Threshold:* 16/16 stages verified, Shannon entropy $> 1.5$ per image.

### Gate 6: Cloud CI Verification
* Push branch `dev` to GitHub remote.
* GitHub Actions workflow `HCS Linux CI` must pass 100% green (`completed / success`).

### Gate 7: Release Publication & Signing
* Open Pull Request from `dev` to `main`.
* Generate signed release artifacts:
  * `HCS-Linux-1.0.0-amd64.iso`
  * `SHA256SUMS` and `SHA256SUMS.gpg`
  * `SBOM.spdx.json`
  * `THIRD-PARTY-NOTICES.txt`
  * `BUILD-MANIFEST.json` and `MODEL-MANIFEST.json`

---

## 7. Deliverables & Acceptance Checklist

- [ ] **Windows-like Neural Glass Taskbar (`taskbar.qml`):** Bottom-anchored with Start Button, active window tabs, and SNI system tray applets.
- [ ] **Modern Start Menu (`start_menu.qml`):** Spotlight-style search, pinned apps, recent memories, and power controls.
- [ ] **Custom Brand Identity:** High-resolution SVG logos, Silicon Valley wallpaper collection, and Plymouth boot splash.
- [ ] **Offline CPU Image Generation (`src/hcs-image`):** Statically linked `stable-diffusion.cpp` engine with SD 1.5 LCM Q4_0 model, txt2img, and img2img.
- [ ] **Curated Pentest Arsenal:** Debian 13 native security packages (`nmap`, `wireshark`, `john`, `hashcat`, `hydra`, `sqlmap`).
- [ ] **Tor Transparent Switch & Tor Browser:** Fail-closed `nftables` routing, zero DNS leaks, and pre-packaged Tor Browser bundle.
- [ ] **Developer Workstation:** Rust, Python, Node, Neovim, and VSCodium pre-configured with HCS MCP integration.
- [ ] **Offline Documentation & Onboarding:** `hcs-docs` viewer, `Super+/` hotkey cheatsheet HUD, and `hcs-welcome` walkthrough.
- [ ] **Calamares OEM Installer:** LUKS2 encryption support, automated profile detection, and branded installation slides.
- [ ] **Verification & Release:** 16-stage VirtualBox pure-CPU QA pass, green GitHub Actions CI, and published v1.0.0 Stable Release ISO.
