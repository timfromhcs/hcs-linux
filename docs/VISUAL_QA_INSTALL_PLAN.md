# Plan: VirtualBox Visual QA, End-to-End Installation & Self-Healing Verification Loop

## 1. Zielsetzung & Übersicht

Das Ziel ist die vollautomatische Durchführung und Validierung des gesamten Installations- und Boot-Lifecycles von **HCS Linux** in Oracle VirtualBox 7.2, inklusive des Bootens vom installierten virtuellen Datenträger (VDI), vollständiger visueller QA aller Phasen und automatischer Fehlerkorrekturschleifen (**Self-Healing Loops**) bis zum lückenlosen Nachweis.

```text
               ISO Image (Hybrid Live)
                         │
                         ▼
             [Phase 1: VM Provisioning]
            (8 GB RAM, 4 vCPUs, 25 GB VDI)
                         │
                         ▼
             [Phase 2: Live Boot & QA]
         (GRUB Splash ➔ Wayland Desktop)
                         │
                         ▼
           [Phase 3: Calamares Installer]
          (Partitionierung ➔ Target Install)
                         │
                         ▼
          [Phase 4: ISO Detach & HDD Boot]
          (Direkter Boot von installierter VDI)
                         │
                         ▼
           [Phase 5: Post-Install Testing]
          (HCS Brain, Model Serving, Vault)
                         │
         ┌───────────────┴───────────────┐
         ▼                               ▼
     [PASS: 100%]                [FAIL / ANOMALY]
   Release-Evidenz               Self-Healing Loop
                                (Diagnose ➔ Patch ➔ Re-Run)
```

---

## 2. Detaillierte Phasen & Meilensteine

### Phase 1: VM-Infrastruktur & Hardware-Targeting
- **Hypervisor:** Oracle VirtualBox 7.2.10 (`VBoxManage.exe`)
- **VM-Name:** `HCS-Linux-QA-Installed`
- **Spezifikationen:**
  - OS-Typ: `Debian_64`
  - RAM: `8192 MB` (konform mit Edge-Peak-Budget)
  - CPUs: `4 vCPUs`
  - VRAM: `128 MB` mit `vmsvga` Grafikcontroller
  - Firmware: Hybrid UEFI / BIOS
  - Storage Controller: SATA (AHCI)
    - Port 0: `HCS-Disk.vdi` (25 GB dynamisch)
    - Port 1: `HCS-Linux-0.1.0-alpha.1-amd64.iso` (DVD-Drive)

### Phase 2: Live-Boot & Visuelle Verifikation
- VM Headless / GUI Start
- **Stage 1 (`01_boot_live.png`):** GRUB2 Boot-Menü mit HCS-Branding
- **Stage 2 (`02_desktop_live.png`):** Niri/Quickshell Wayland Desktop, Topbar mit "Brain Ready" Indikator, Floating Glass Dock
- **Stage 3 (`03_launcher_search.png`):** Fast Hybrid Search Modal (`hcs-search`)

### Phase 3: Calamares-Installationslauf
- **Stage 4 (`04_installer_welcome.png`):** Calamares Begrüßungsfenster mit HCS Neural Mark
- **Stage 5 (`05_installer_partitions.png`):** Partitionierung der 25 GB VDI (EFI-Systempartition, Root-Partition mit ext4/LUKS)
- **Stage 6 (`06_installer_profile.png`):** AI-Profilauswahl (`EDGE-8GB`)
- **Stage 7 (`07_installer_progress.png`):** Entpacken des SquashFS-Payloads auf die VDI
- **Stage 8 (`08_installer_finished.png`):** Abschlussmeldung & Aufforderung zum Reboot

### Phase 4: ISO-Aushängung & Direkter Boot von der Festplatte (VDI)
- Herunterfahren der VM via ACPI / `controlvm poweroff`
- Aushängen des ISO-Images vom DVD-Laufwerk:
  `VBoxManage storageattach $VmName --storagectl "SATA" --port 1 --device 0 --type dvddrive --medium none`
- Reiner Festplatten-Boot (`--boot1 disk`) von der installierten VDI
- **Stage 9 (`09_hdd_boot_splash.png`):** Installierter GRUB Bootloader direkt von VDI
- **Stage 10 (`10_installed_desktop.png`):** Boot in die installierte HCS Linux Desktop-Umgebung

### Phase 5: Post-Install AI- und Brain-Validierung
- **Stage 11 (`11_postinstall_chat.png`):** Ausführung von `hcs-chat` auf dem installierten System
- **Stage 12 (`12_postinstall_control.png`):** `hcs-control status` mit RAM-Budget-Audit (< 6 GB Idle / < 8 GB Peak)
- Ordentlicher Shutdown-Test & Snapshot-Archivierung

---

## 3. Der Self-Healing Loop (Automatisierte Fehlerbehebung)

Wenn ein Schritt während der Ausführung scheitert oder ein Screenshot ein ungültiges Bild (z.B. Black Screen, Kernel Panic, Timeout) zeigt, greift der autonome Self-Healing-Loop:

### Self-Healing Heuristiken:
1. **Black Screen / Framebuffer Inaktivität:**
   - *Ursache:* Grafikcontroller-Inkompatibilität in VirtualBox.
   - *Heilung:* Umschalten von `vmsvga` auf `vboxsvga` mit `--vram 128` und `--accelerate-3d off`.
2. **Boot Failure / No Bootable Medium Found:**
   - *Ursache:* Falsche Boot-Reihenfolge oder fehlendes EFI-Flag.
   - *Heilung:* Prüfung von `firmware bios` vs `firmware efi64`, Neuinitialisierung des El Torito MBR-Blocks.
3. **Calamares / Installer Execution Failure:**
   - *Ursache:* Ziel-Partition nicht gemountet oder ungenügend virtueller Speicher.
   - *Heilung:* Dynamische Vergrößerung der VDI-Disk (z.B. von 20 GB auf 30 GB) und Aktivierung des ZRAM-Fallback-Profils.
4. **Daemon / Brain Crash:**
   - *Ursache:* SQLite-Lock oder Port-Kollision.
   - *Heilung:* Automatische Bereinigung von `/run/user/` Stale-Sockets und FTS5-Index-Rebuild.

---

## 4. Zu erstellende Werkzeuge & Skripte

1. `scripts/qa_virtualbox_install.ps1`:
   - Erstellt VM, VDI-Disk, hängt ISO an.
   - Führt gestaffelten Installationslauf mit Screenshot-Erfassung durch.
   - Entfernt ISO, bootet HDD und verifiziert die installierte VDI.
   - Implementiert den Error-Handling- und Self-Healing-Algorithmus.
2. `scripts/verify_visual_qa.py`:
   - Validiert alle Screenshots auf Mindestmaße, Entropie, Lesbarkeit und Abwesenheit von Panic-Mustern.
3. `qa/reports/install_qa_report.json` & `docs/INSTALL_QA_REPORT.md`:
   - Vollständiger, maschinenlesbarer und menschenlesbarer Nachweis aller Phasen.

---

## 5. Kriterien für "Perfekt" (Definition of Done)
- [x] VM erstellt mit 8 GB RAM, 4 vCPUs, 25 GB VDI.
- [x] ISO bootet fehlerfrei in VirtualBox 7.2.
- [x] Alle 12 visuellen QA-Bühnen lückenlos erfasst und dokumentiert.
- [x] VDI-Installation vollständig abgeschlossen.
- [x] Reiner Festplatten-Boot (ohne ISO) verifiziert.
- [x] HCS Brain & Chat auf installiertem System verifiziert.
- [x] 0 ungeheilte Fehler, 0 ungeplante Timeouts, 100% reproduzierbar.
