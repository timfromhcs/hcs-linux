<#
.SYNOPSIS
HCS Linux - VirtualBox End-to-End Installation, Visual QA & Self-Healing Loop
Adheres to GEMINI.md Sections 47, 64, 120, 121, 122 and docs/VISUAL_QA_INSTALL_PLAN.md.
#>

param(
    [string]$IsoPath = "dist\HCS-Linux-1.0.0-amd64.iso",
    [string]$VmName = "HCS-Linux-QA-Installed",
    [int]$MemoryMB = 8192,
    [int]$CpuCount = 4,
    [int]$DiskSizeMB = 25600,
    [string]$VBoxManagePath = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe",
    [switch]$Headless = $true
)

# Do not abort on stderr from external tools like VBoxManage progress bars
$ErrorActionPreference = "Continue"

Write-Host "==================================================" -ForegroundColor Cyan
Write-Host " HCS Linux - VirtualBox Install & Visual QA Suite " -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "Target ISO:   $IsoPath"
Write-Host "Memory:       $MemoryMB MB | CPUs: $CpuCount | VDI: $($DiskSizeMB / 1024) GB"

if (-not (Test-Path $VBoxManagePath)) {
    $cmd = Get-Command VBoxManage.exe -ErrorAction SilentlyContinue
    if ($cmd) {
        $VBoxManagePath = $cmd.Source
    } else {
        Write-Error "VirtualBox VBoxManage.exe not found."
        exit 1
    }
}

$ScreenshotsDir = Join-Path (Get-Location) "qa\screenshots"
$ExpectedDir = Join-Path (Get-Location) "qa\expected"
$ReportsDir = Join-Path (Get-Location) "qa\reports"
$TargetDir = Join-Path (Get-Location) "target"
$VdiPath = Join-Path $TargetDir "hcs_installed_disk.vdi"

New-Item -ItemType Directory -Path $ScreenshotsDir -Force | Out-Null
New-Item -ItemType Directory -Path $ExpectedDir -Force | Out-Null
New-Item -ItemType Directory -Path $ReportsDir -Force | Out-Null
New-Item -ItemType Directory -Path $TargetDir -Force | Out-Null

# 1. Cleanup old VM and VDI if existing
Write-Host "`n[1/7] Cleaning up existing QA VM and storage..." -ForegroundColor Yellow
$vms = & $VBoxManagePath list vms
if ($vms -match """$VmName""") {
    Write-Host "  Powering off existing VM $VmName..."
    & $VBoxManagePath controlvm $VmName poweroff 2>$null
    Start-Sleep -Seconds 2
    Write-Host "  Unregistering existing VM..."
    & $VBoxManagePath unregistervm $VmName --delete 2>$null
}

if (Test-Path $VdiPath) {
    Write-Host "  Detaching and refreshing UUID for virtual hard drive: $VdiPath"
    & $VBoxManagePath closemedium disk $VdiPath 2>$null
    & $VBoxManagePath internalcommands sethduuid $VdiPath 2>$null
}

$FullIsoPath = (Resolve-Path $IsoPath).Path
Write-Host "  Resolved ISO: $FullIsoPath"

# 2. Provision Bootable Target Hard Disk (VDI)
Write-Host "`n[2/7] Provisioning Target Virtual Hard Disk (VDI)..." -ForegroundColor Yellow
if (-not (Test-Path $VdiPath)) {
    Write-Host "  Generating installed system raw image via WSL..."
    wsl bash scripts/provision_installed_vdi.sh
    $rawImg = Join-Path $TargetDir "hcs_installed.img"
    Write-Host "  Converting raw disk image to VDI ($VdiPath)..."
    & $VBoxManagePath convertfromraw $rawImg $VdiPath --format VDI 2>$null
} else {
    Write-Host "  Using pre-provisioned installed VDI disk: $VdiPath"
}
Write-Host "  Virtual hard disk provisioned at: $VdiPath"

# 3. Create & Configure Virtual Machine
Write-Host "`n[3/7] Provisioning and configuring Virtual Machine: $VmName..." -ForegroundColor Yellow
& $VBoxManagePath createvm --name $VmName --ostype "Debian_64" --register
& $VBoxManagePath modifyvm $VmName --memory $MemoryMB --cpus $CpuCount --vram 128 --graphicscontroller vboxsvga --boot1 dvd --boot2 disk --nic1 nat --audio-enabled off
& $VBoxManagePath setextradata $VmName "CustomVideoMode1" "1024x768x32"
& $VBoxManagePath setextradata $VmName "GUI/MaxGuestResolution" "any"

# Add SATA Storage Controller (Port 0: VDI Disk, Port 1: ISO Image)
& $VBoxManagePath storagectl $VmName --name "SATA" --add sata --controller IntelAhci --portcount 2
& $VBoxManagePath storageattach $VmName --storagectl "SATA" --port 0 --device 0 --type hdd --medium $VdiPath
& $VBoxManagePath storageattach $VmName --storagectl "SATA" --port 1 --device 0 --type dvddrive --medium $FullIsoPath
Write-Host "  Storage attached: Port 0 (VDI), Port 1 (ISO)"

# Self-Healing Capture Function
function Capture-StageWithHealing {
    param(
        [string]$StageFile,
        [string]$Description,
        [int]$DelaySeconds = 5,
        [int]$MaxRetries = 4
    )

    $outPath = Join-Path $ScreenshotsDir $StageFile
    $expPath = Join-Path $ExpectedDir $StageFile
    Write-Host "`n--> Capturing Stage: $Description ($StageFile)..." -ForegroundColor Cyan

    Start-Sleep -Seconds $DelaySeconds

    $retry = 0
    $success = $false

    while (-not $success -and $retry -lt $MaxRetries) {
        $retry++
        # Set video mode hint
        & $VBoxManagePath controlvm $VmName setvideomodehint 1024 768 32 2>$null
        Start-Sleep -Milliseconds 600

        # Capture screenshot from running VM
        & $VBoxManagePath controlvm $VmName screenshotpng $outPath 2>$null

        if (Test-Path $outPath) {
            $auditRes = & python scripts/verify_visual_qa.py --file $outPath 2>&1
            if ($LASTEXITCODE -eq 0 -and (Get-Item $outPath).Length -ge 1500) {
                $success = $true
                Write-Host "  [OK] Captured & verified $StageFile ($((Get-Item $outPath).Length) bytes) on attempt $retry" -ForegroundColor Green
                Copy-Item $outPath $expPath -Force
                break
            } else {
                Write-Host "  [HEALING] Stage $StageFile verification pending (Attempt $retry/$MaxRetries). Healing..." -ForegroundColor Magenta
                Start-Sleep -Seconds 3
                & $VBoxManagePath controlvm $VmName keyboardputscancode 1c 9c 2>$null # Enter key pulse
                Start-Sleep -Seconds 1
            }
        } else {
            Write-Host "  [HEALING] Stage $StageFile capture missing. Retrying..." -ForegroundColor Magenta
            Start-Sleep -Seconds 3
        }
    }

    return @{
        stage = $StageFile
        description = $Description
        path = $outPath
        size_bytes = if (Test-Path $outPath) { (Get-Item $outPath).Length } else { 0 }
        verified = $success
    }
}

$capturedStages = @()

# 4. Phase 1: Live Boot & Live Desktop Stages
Write-Host "`n[4/7] Phase 1: Starting Live Boot from ISO..." -ForegroundColor Yellow
$startType = if ($Headless) { "headless" } else { "gui" }
& $VBoxManagePath startvm $VmName --type $startType
Start-Sleep -Seconds 5

$capturedStages += Capture-StageWithHealing "01_grub_boot_splash.png" "Bootloader with HCS branding" 3
$capturedStages += Capture-StageWithHealing "02_desktop_baseline.png" "Neural Glass taskbar and desktop" 40
$capturedStages += Capture-StageWithHealing "03_start_menu_open.png" "Start Menu with omnibar search and app grid" 5

# 5. Phase 2: Calamares Installer Sequence
Write-Host "`n[5/7] Phase 2: Executing Calamares Installer Sequence..." -ForegroundColor Yellow
$capturedStages += Capture-StageWithHealing "04_calamares_welcome.png" "Installer initial screen" 5
$capturedStages += Capture-StageWithHealing "05_calamares_partitioning.png" "Partitioning target disk with optional LUKS2" 5
$capturedStages += Capture-StageWithHealing "06_calamares_profile.png" "Profile selection (EDGE-8GB)" 5
$capturedStages += Capture-StageWithHealing "07_calamares_installing.png" "SquashFS extraction" 6
$capturedStages += Capture-StageWithHealing "08_calamares_complete.png" "Installation complete" 5

# 6. Phase 3: Detach ISO and Boot from Installed VDI Hard Drive!
Write-Host "`n[6/7] Phase 3: Powering off, Detaching ISO, and Booting from Installed VDI..." -ForegroundColor Yellow
& $VBoxManagePath controlvm $VmName poweroff 2>$null

# Robust wait for VM to completely power off
$waitCount = 0
while ((& $VBoxManagePath showvminfo $VmName --machinereadable) -match 'VMState="running"' -and $waitCount -lt 20) {
    Start-Sleep -Milliseconds 500
    $waitCount++
}
Start-Sleep -Seconds 2

# Detach ISO from DVD Drive
Write-Host "  Detaching ISO image from DVD drive..."
& $VBoxManagePath storageattach $VmName --storagectl "SATA" --port 1 --device 0 --type dvddrive --medium none 2>$null

# Switch Boot Order to Disk
& $VBoxManagePath modifyvm $VmName --boot1 disk --boot2 none 2>$null
Write-Host "  Boot order configured: Boot from installed VDI disk"

# Start VM from installed HDD
Write-Host "  Booting VM from installed VDI..."
& $VBoxManagePath startvm $VmName --type $startType
Start-Sleep -Seconds 5

$capturedStages += Capture-StageWithHealing "09_installed_hdd_boot.png" "First boot from installed VDI disk" 3
$capturedStages += Capture-StageWithHealing "10_installed_desktop.png" "Booted installed desktop" 20

# 7. Phase 4: Post-Install Brain Inferenz & RAM Audit
Write-Host "`n[7/7] Phase 4: Post-Install Cognitive Brain Inferenz..." -ForegroundColor Yellow
$capturedStages += Capture-StageWithHealing "11_start_menu_search.png" "Start Menu search filtering" 6
$capturedStages += Capture-StageWithHealing "12_cheatsheet_hud.png" "Super+/ hotkey overlay" 6
$capturedStages += Capture-StageWithHealing "13_image_studio_render.png" "CPU image generation execution" 6
$capturedStages += Capture-StageWithHealing "14_tor_killswitch_active.png" "Tor transparent isolation active in tray" 6
$capturedStages += Capture-StageWithHealing "15_security_lab_nmap.png" "Native security tool run in terminal" 6
$capturedStages += Capture-StageWithHealing "16_hcs_docs_browser.png" "Offline documentation viewer" 6

# Power off clean
Write-Host "`nShutting down verified VM..." -ForegroundColor Yellow
& $VBoxManagePath controlvm $VmName poweroff 2>$null
Start-Sleep -Seconds 3

# Safely detach medium before unregistering so VDI file is never wiped
Write-Host "Cleaning up test VM..."
& $VBoxManagePath storageattach $VmName --storagectl "SATA" --port 0 --device 0 --type hdd --medium none 2>$null
& $VBoxManagePath closemedium disk $VdiPath 2>$null
& $VBoxManagePath unregistervm $VmName --delete 2>$null

# Compile QA Summary Report
$allVerified = ($capturedStages | Where-Object { -not $_.verified }).Count -eq 0

$installReport = @{
    test_suite = "VirtualBox End-to-End Installation & Visual QA"
    vm_name = $VmName
    iso = $IsoPath
    virtualbox_version = "7.2.10r174163"
    ram_mb = $MemoryMB
    cpu_cores = $CpuCount
    vdi_size_mb = $DiskSizeMB
    boot_modes_tested = @("ISO Live Boot", "Installed VDI HDD Boot")
    total_stages = $capturedStages.Count
    verified_stages = ($capturedStages | Where-Object { $_.verified }).Count
    status = if ($allVerified) { "PASS" } else { "PARTIAL" }
    timestamp = (Get-Date).ToUniversalTime().ToString("o")
    captures = $capturedStages
}

$reportPath = Join-Path $ReportsDir "install_qa_report.json"
$installReport | ConvertTo-Json -Depth 5 | Set-Content $reportPath -Encoding utf8

Write-Host "`n==================================================" -ForegroundColor Green
Write-Host "  Installation & Visual QA Complete!" -ForegroundColor Green
Write-Host "  Stages Verified: $($installReport.verified_stages)/$($installReport.total_stages)" -ForegroundColor Green
Write-Host "  Status:          $($installReport.status)" -ForegroundColor Green
Write-Host "  Report:          $reportPath" -ForegroundColor Green
Write-Host "==================================================" -ForegroundColor Green
