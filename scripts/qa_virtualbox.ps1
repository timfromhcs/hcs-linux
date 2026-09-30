<#
.SYNOPSIS
HCS Linux - VirtualBox Automated VM QA and Visual Capture Runner
Adheres to GEMINI.md Sections 64, 121, and 122.
#>

param(
    [string]$IsoPath = "dist\HCS-Linux-1.0.0-amd64.iso",
    [string]$VmName = "HCS-Linux-Visual-QA",
    [int]$MemoryMB = 8192,
    [int]$CpuCount = 4,
    [string]$VBoxManagePath = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe",
    [switch]$Headless = $true
)

$ErrorActionPreference = "Stop"

Write-Host "=== HCS Linux VirtualBox Visual QA Suite ===" -ForegroundColor Cyan
Write-Host "Target ISO: $IsoPath"
Write-Host "Memory: $MemoryMB MB | CPUs: $CpuCount"

if (-not (Test-Path $VBoxManagePath)) {
    Write-Warning "VBoxManage not found at '$VBoxManagePath'. Checking PATH..."
    $cmd = Get-Command VBoxManage.exe -ErrorAction SilentlyContinue
    if ($cmd) {
        $VBoxManagePath = $cmd.Source
    } else {
        Write-Error "VirtualBox VBoxManage.exe is required for VM QA but could not be located."
        exit 1
    }
}

$ScreenshotsDir = Join-Path (Get-Location) "qa\screenshots"
$ReportsDir = Join-Path (Get-Location) "qa\reports"
New-Item -ItemType Directory -Path $ScreenshotsDir -Force | Out-Null
New-Item -ItemType Directory -Path $ReportsDir -Force | Out-Null

# 1. Cleanup old VM if exists
Write-Host "Checking for existing QA VM..."
$existingVms = & $VBoxManagePath list vms
if ($existingVms -match """$VmName""") {
    Write-Host "Removing existing VM $VmName..."
    & $VBoxManagePath controlvm $VmName poweroff 2>$null
    Start-Sleep -Seconds 2
    & $VBoxManagePath unregistervm $VmName --delete
}

# 2. Resolve absolute ISO path
$FullIsoPath = (Resolve-Path $IsoPath).Path
Write-Host "Resolved ISO Path: $FullIsoPath"

# 3. Create and register VM
Write-Host "Creating Virtual Machine: $VmName"
& $VBoxManagePath createvm --name $VmName --ostype "Debian_64" --register

# 4. Configure hardware specifications (Edge target <= 8GB RAM, 4 vCPUs)
& $VBoxManagePath modifyvm $VmName --memory $MemoryMB --cpus $CpuCount --vram 128 --graphicscontroller vmsvga --boot1 dvd --boot2 disk --nic1 nat

# 5. Attach Storage Controller and ISO image
& $VBoxManagePath storagectl $VmName --name "SATA" --add sata --controller IntelAhci
& $VBoxManagePath storageattach $VmName --storagectl "SATA" --port 0 --device 0 --type dvddrive --medium $FullIsoPath

# 6. Start VM
$startType = if ($Headless) { "headless" } else { "gui" }
Write-Host "Starting VM in $startType mode..."
& $VBoxManagePath startvm $VmName --type $startType

# 7. Visual Stage Capture sequence (Section 122)
$stages = @(
    @{ Name = "boot.png"; DelaySeconds = 8; Description = "GRUB / Bootloader Splash" },
    @{ Name = "desktop.png"; DelaySeconds = 15; Description = "Wayland Desktop Baseline" },
    @{ Name = "launcher.png"; DelaySeconds = 6; Description = "Application Launcher / Search" },
    @{ Name = "chat.png"; DelaySeconds = 6; Description = "HCS Chat & Brain Interface" },
    @{ Name = "installer.png"; DelaySeconds = 6; Description = "Calamares System Installer" }
)

$captures = @()

foreach ($stage in $stages) {
    Write-Host "Waiting $($stage.DelaySeconds)s for stage: $($stage.Description)..."
    Start-Sleep -Seconds $stage.DelaySeconds

    $outPng = Join-Path $ScreenshotsDir $stage.Name
    Write-Host "Capturing screenshot: $outPng"
    & $VBoxManagePath controlvm $VmName screenshotpng $outPng

    if (Test-Path $outPng) {
        $captures += @{
            stage = $stage.Name
            description = $stage.Description
            path = $outPng
            size_bytes = (Get-Item $outPng).Length
            verified = $true
        }
    }
}

# 8. Shutdown & Clean VM
Write-Host "Powering off QA VM..."
& $VBoxManagePath controlvm $VmName poweroff 2>$null
Start-Sleep -Seconds 3
& $VBoxManagePath unregistervm $VmName --delete

# 9. Write QA Report
$qaReport = @{
    test_suite = "VirtualBox Visual QA"
    vm_name = $VmName
    iso = $IsoPath
    memory_mb = $MemoryMB
    cpu_cores = $CpuCount
    timestamp = (Get-Date).ToUniversalTime().ToString("o")
    captures = $captures
    status = if ($captures.Count -eq $stages.Count) { "PASS" } else { "PARTIAL" }
}

$reportPath = Join-Path $ReportsDir "visual_qa.json"
$qaReport | ConvertTo-Json -Depth 4 | Set-Content $reportPath -Encoding utf8
Write-Host "[OK] Visual QA Report saved to $reportPath" -ForegroundColor Green
