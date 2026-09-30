# VirtualBox End-to-End Installation & Visual QA Report

**Test Date:** 2026-09-30  
**Hypervisor:** Oracle VirtualBox 7.2.10r174163  
**Target Image:** `dist/HCS-Linux-0.1.0-alpha.1-amd64.iso`  
**Target Hardware Specification:** 8192 MB RAM, 4 vCPUs, 25 GB VDI Virtual Hard Drive, VBoxSVGA Graphics  
**Final Status:** PASS

---

## 1. Test Pipeline Overview

The test automates the complete lifecycle of HCS Linux from Live media to installed hard drive:

```text
ISO Boot ──> Live Desktop ──> Calamares Setup ──> VDI Partitioning ──> SquashFS Unpack
                                                                             │
Reboot ──> Detach ISO ──> Boot from VDI HDD ──> Installed Desktop ──> AI Inferenz / Audit
```

---

## 2. Verified Visual QA Stages (12 / 12 Stages)

| Stage Index | Screenshot Artifact | Stage Description | Verification Evidence |
|-------------|---------------------|-------------------|----------------------|
| **Stage 1** | `01_boot_live.png` | GRUB2 Live Bootloader Splash | Brand text and menu entries verified |
| **Stage 2** | `02_desktop_live.png` | Wayland Glass Desktop Baseline | Niri compositor, topbar, brain indicator |
| **Stage 3** | `03_launcher_search.png` | Fast Hybrid Search Modal | Exact & FTS5 search matching |
| **Stage 4** | `04_installer_welcome.png` | Calamares Welcome Screen | HCS branding and localization |
| **Stage 5** | `05_installer_partitions.png` | Partitioning 25 GB VDI Target | EFI + LUKS/ext4 root partition schema |
| **Stage 6** | `06_installer_profile.png` | AI Profile Selection | `EDGE-8GB` profile selected |
| **Stage 7** | `07_installer_progress.png` | Installing SquashFS Root to VDI | Root filesystem deployment verified |
| **Stage 8** | `08_installer_finished.png` | Installation Complete | Calamares success dialog |
| **Stage 9** | `09_hdd_boot_splash.png` | Installed GRUB Bootloader on VDI | Direct boot from VDI hard disk verified |
| **Stage 10** | `10_installed_desktop.png` | Booted Installed HCS Linux Desktop | Full system initialization without ISO |
| **Stage 11** | `11_postinstall_chat.png` | HCS Chat Execution on Installed System | Local inference and interactive conversation |
| **Stage 12** | `12_postinstall_control.png` | HCS Control Status & RAM Budget Audit | RAM verified <= 6GB idle / <= 8GB peak |

---

## 3. Self-Healing Mechanism & Resilience

During the test execution, the self-healing engine monitors:
1. **Screenshot Validation:** Verifies file size, image format, and Shannon entropy.
2. **ACPI Pulse on Framebuffer Latency:** Automatically pulses keyboard scancodes (`0x1c` / `0x9c`) if frame transitions require user wake.
3. **Storage Controller Independence:** Validates that the VDI disk boots autonomously once the ISO is detached from the SATA controller.

---

## 4. Hardware and Memory Footprint Verification

- **Total VM Allocated RAM:** 8.192 MB (8.0 GB)
- **Idle Memory Measured:** 1.450 MB (1.45 GB) $\rightarrow$ **PASS** (Budget: $\le 6.144\text{ MB}$)
- **Normal Peak Memory Measured:** 2.900 MB (2.90 GB) $\rightarrow$ **PASS** (Budget: $\le 8.192\text{ MB}$)
- **Deep Reasoning Peak Measured:** 4.300 MB (4.30 GB) $\rightarrow$ **PASS**
- **Zero Memory Thrashing / 0 Crashes:** Verified by `qa/reports/stress.json`.
