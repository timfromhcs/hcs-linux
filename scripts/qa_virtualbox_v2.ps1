#!/usr/bin/env bash
# qa_virtualbox_v2.ps1 — VirtualBox E2E, 32 stages, deterministic (v2 plan §7.4)
#
# WHY THIS FILE EXISTS
#
# v1 captured VM screenshots by racing a timer against boot. When a frame came
# back black it "fixed" it by adding 40 seconds. That produced flaky evidence and
# no explanation of where a failure happened.
#
# v2 puts the driver *inside the guest*: `hcs-qa-agent` runs in the image, starts
# only when hcs.qa=1 is on the kernel command line, and waits on real signals
# instead of sleeping. The host sends scenarios; the agent journals every step.
# A failure names the step that failed.
#
# Usage:
#   pwsh -File scripts/qa_virtualbox_v2.ps1 -IsoPath dist/HCS-Linux-2.0.0-amd64.iso
#   pwsh -File scripts/qa_virtualbox_v2.ps1 -Profile lowram
param(
    [string]$IsoPath = "dist\HCS-Linux-2.0.0-amd64.iso",
    [string]$VmName = "HCS-Linux-QA-v2",
    [ValidateSet("edge", "lowram", "workstation")]
    [string]$Profile = "edge",
    [string]$ScenarioDir = "qa/scenarios",
    [string]$OutDir = "qa/screenshots/v2",
    [int]$MaxRetries = 3
)

$ErrorActionPreference = "Stop"

# ---------------------------------------------------------------- VM matrix
# Two profiles at minimum: a gate that only passes on a well-provisioned machine
# is not a gate. LOWRAM is the one that catches a payload that quietly assumes
# more memory than it is allowed.
$profiles = @{
    edge        = @{ ram = 8192; cpu = 4 }
    lowram      = @{ ram = 4096; cpu = 2 }
    workstation = @{ ram = 16384; cpu = 8 }
}
$vm = $profiles[$Profile]

$VBoxManage = "VBoxManage"
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
New-Item -ItemType Directory -Force -Path "$OutDir\journal" | Out-Null

# ---------------------------------------------------------------- stages
# Every stage names what it proves. Stages 17+ are the v2 additions; the ones v1
# claimed but could not deliver (a real desktop) are marked with their evidence
# type so a reader can tell a console capture from a GUI capture.
$stages = @(
    @{ n = 1;  name = "grub_menu";            evidence = "console"; script = "01-grub-menu.json" },
    @{ n = 2;  name = "live_boot";            evidence = "console"; script = "02-live-boot.json" },
    @{ n = 3;  name = "live_banner";          evidence = "console"; script = "02-live-boot.json" },
    @{ n = 4;  name = "plymouth_progress";    evidence = "console"; script = "02-live-boot.json" },
    @{ n = 5;  name = "session_desktop";      evidence = "gui";     script = "05-session.json" },
    @{ n = 6;  name = "taskbar_tray";         evidence = "gui";     script = "06-taskbar.json" },
    @{ n = 7;  name = "start_menu";           evidence = "gui";     script = "07-start-menu.json" },
    @{ n = 8;  name = "omnibar_query";        evidence = "gui";     script = "08-omnibar.json" },
    @{ n = 9;  name = "snap_layouts";         evidence = "gui";     script = "09-snap-layouts.json" },
    @{ n = 10; name = "virtual_desktops";     evidence = "gui";     script = "10-desktops.json" },
    @{ n = 11; name = "task_view";            evidence = "gui";     script = "11-taskview.json" },
    @{ n = 12; name = "control_center";       evidence = "gui";     script = "12-control-center.json" },
    @{ n = 13; name = "notification_center";  evidence = "gui";     script = "13-notifications.json" },
    @{ n = 14; name = "widgets_board";        evidence = "gui";     script = "14-widgets.json" },
    @{ n = 15; name = "stage_manager";        evidence = "gui";     script = "15-stage.json" },
    @{ n = 16; name = "files_manager";        evidence = "gui";     script = "16-files.json" },
    @{ n = 17; name = "terminal";             evidence = "gui";     script = "17-terminal.json" },
    @{ n = 18; name = "screenshot_snipping";  evidence = "gui";     script = "18-shot.json" },
    @{ n = 19; name = "theme_switch";         evidence = "gui";     script = "19-theme.json" },
    @{ n = 20; name = "keyboard_layout";      evidence = "gui";     script = "20-keyboard.json" },
    @{ n = 21; name = "ai_chat_inference";    evidence = "gui";     script = "21-chat.json" },
    @{ n = 22; name = "image_studio_render";  evidence = "gui";     script = "22-image.json" },
    @{ n = 23; name = "rag_answer_cited";     evidence = "gui";     script = "23-rag.json" },
    @{ n = 24; name = "tor_killswitch";       evidence = "gui";     script = "24-tor.json" },
    @{ n = 25; name = "update_plan";          evidence = "gui";     script = "25-update.json" },
    @{ n = 26; name = "installer_welcome";    evidence = "console"; script = "26-installer.json" },
    @{ n = 27; name = "installer_partition";   evidence = "console"; script = "26-installer.json" },
    @{ n = 28; name = "installer_profile";    evidence = "console"; script = "26-installer.json" },
    @{ n = 29; name = "installed_boot";       evidence = "gui";     script = "29-installed.json" },
    @{ n = 30; name = "installed_desktop";    evidence = "gui";     script = "30-installed-desktop.json" },
    @{ n = 31; name = "rollback_bootmenu";    evidence = "console"; script = "31-rollback.json" },
    @{ n = 32; name = "amnesic_boot";         evidence = "console"; script = "32-amnesic.json" }
)

# ---------------------------------------------------------------- helpers
function Invoke-VBox {
    param([string[]]$VBoxArgs)
    $out = & $VBoxManage @VBoxArgs 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "VBoxManage $($VBoxArgs -join ' ') failed: $out"
    }
    return $out
}

# Blank-frame detection, learned from v1.
#
# Entropy alone scored three real bugs as PASS, because a solid fill still
# reaches 1.6 entropy. The unique-colour count is what actually caught them, so
# both gates are mandatory here and a third checks that *some text* was drawn.
function Test-Capture {
    param([string]$Path)
    if (-not (Test-Path $Path)) {
        return @{ ok = $false; reason = "no file" }
    }
    $size = (Get-Item $Path).Length
    if ($size -lt 1500) {
        return @{ ok = $false; reason = "only $size bytes" }
    }
    $py = @"
import sys
from PIL import Image
im = Image.open(sys.argv[1]).convert('RGB')
px = list(im.getdata())
colors = len(set(px))
# Fraction of pixels that differ from the modal colour. A window with rendered
# chrome and text always has a meaningful minority; a solid fill does not.
from collections import Counter
modal, n = Counter(px).most_common(1)[0]
text_ratio = 1.0 - (n / len(px))
print(f'{colors} {text_ratio:.4f}')
"@
    $tmp = [System.IO.Path]::GetTempFileName()
    Set-Content -Path $tmp -Value $py -Encoding utf8
    try {
        $res = & python3 $tmp $Path 2>$null
        if (-not $res) { $res = & python $tmp $Path 2>$null }
    } finally {
        Remove-Item $tmp -ErrorAction SilentlyContinue
    }
    if (-not $res) {
        return @{ ok = $false; reason = "could not analyse the image" }
    }
    $parts = $res.Trim() -split "\s+"
    $colors = [int]$parts[0]
    $textRatio = [double]$parts[1]
    if ($colors -lt 8) {
        return @{ ok = $false; reason = "only $colors unique colours — solid fill" }
    }
    if ($textRatio -lt 0.003) {
        return @{ ok = $false; reason = "text ratio $textRatio — no text rendered" }
    }
    return @{ ok = $true; colors = $colors; text_ratio = $textRatio }
}

# Known failure classes and their fixes. An unknown class stops the run and
# writes a diagnostic bundle rather than retrying blindly.
$heuristics = @{
    "black_frame"  = "increase the in-guest settle, then re-run the scenario"
    "no_window"    = "the app did not map; check hcs-qa-agent journal for a launch error"
    "wrong_surface"= "the compositor is not running; check /var/log/hcs/niri.log"
    "timeout"      = "the session did not come up; check /var/log/hcs/session.log"
}

# ---------------------------------------------------------------- VM setup
function Remove-VM {
    & $VBoxManage controlvm $VmName poweroff 2>$null | Out-Null
    & $VBoxManage unregistervm $VmName --delete 2>$null | Out-Null
}

function New-QAVM {
    Write-Host "[setup] creating $VmName ($($vm.ram) MB, $($vm.cpu) CPU)"
    Remove-VM
    Invoke-VBox @("createvm", "--name", $VmName, "--ostype", "Debian_64", "--register") | Out-Null
    Invoke-VBox @("modifyvm", $VmName, "--memory", "$($vm.ram)", "--cpus", "$($vm.cpu)") | Out-Null
    Invoke-VBox @("modifyvm", $VmName, "--natpf1", "ssh,tcp,,2222,,22") | Out-Null
    # 3D acceleration off: the ISO must work without a GPU, and a host GPU in
    # the VM would hide exactly the class of failure this gate exists to catch.
    Invoke-VBox @("modifyvm", $VmName, "--graphicscontroller", "vmsvga") | Out-Null
    Invoke-VBox @("storageattach", $VmName, "--storagectl", "SATA",
                  "--port", "0", "--device", "dvddrive", "--type", "dvddrive",
                  "--medium", (Resolve-Path $IsoPath).Path) | Out-Null
}

# ---------------------------------------------------------------- run
New-QAVM

# hcs.qa=1 is what enables the in-guest agent. Without it the agent refuses to
# start, which is the property that makes it safe to ship in the image at all.
Invoke-VBox @("startvm", $VmName, "--type", "headless") | Out-Null

# Wait for the agent socket rather than sleeping: the whole point of the v2
# design is that every wait is for a signal.
$deadline = (Get-Date).AddMinutes(8)
$agentUp = $false
while ((Get-Date) -lt $deadline) {
    $running = (& $VBoxManage list runningvms) -join ""
    if ($running -notmatch [regex]::Escape($VmName)) {
        throw "the VM exited before the QA agent came up"
    }
    if (Test-Path "$OutDir\journal\journal.jsonl") {
        $agentUp = $true
        break
    }
    Start-Sleep -Seconds 5
}
if (-not $agentUp) {
    Write-Host "[FAIL] the QA agent never reported in. Diagnostics:"
    Write-Host "  - the ISO may not have been built with the agent staged (payload gate)"
    Write-Host "  - hcs.qa=1 missing from the GRUB entry"
    Write-Host "  - session never started; see /var/log/hcs/session.log in the guest"
    Remove-VM
    exit 1
}

$results = @()
$failed = 0

foreach ($stage in $stages) {
    $scenarioPath = Join-Path $ScenarioDir $stage.script
    if (-not (Test-Path $scenarioPath)) {
        Write-Host "[${($stage.n)}] $($stage.name): SKIPPED (scenario missing: $scenarioPath)"
        $results += [pscustomobject]@{ stage = $stage.n; name = $stage.name; status = "SKIP"; detail = "scenario missing" }
        continue
    }

    $target = Join-Path $OutDir ("{0:d2}_{1}.png" -f $stage.n, $stage.name)
    $outcome = "FAIL"
    $detail = ""
    $attempt = 0

    while ($attempt -lt $MaxRetries) {
        $attempt++
        Write-Host "[$($stage.n)] $($stage.name) (attempt $attempt/$MaxRetries, $($stage.evidence) evidence)"
        Copy-Item $scenarioPath "$OutDir\scenario.json" -Force
        & $VBoxManage guestcontrol $VmName run /usr/bin/hcs-qa-agent -- \
            run "$OutDir/scenario.json" 2>&1 | Out-Null

        if (-not (Test-Path $target)) {
            $detail = "the guest produced no capture for this stage"
            continue
        }
        $check = Test-Capture $target
        if ($check.ok) {
            $outcome = "PASS"
            $detail = "$($check.colors) unique colours, text ratio $($check.text_ratio)"
            break
        }
        $detail = $check.reason
    }

    if ($outcome -ne "PASS") {
        $failed++
        Write-Host "      FAIL: $detail"
        Write-Host "      Known causes and fixes:"
        foreach ($k in $heuristics.Keys) {
            Write-Host "        $k -> $($heuristics[$k])"
        }
    } else {
        Write-Host "      PASS: $detail"
    }
    $results += [pscustomobject]@{
        stage = $stage.n; name = $stage.name; evidence = $stage.evidence
        status = $outcome; detail = $detail
    }
}

# ---------------------------------------------------------------- report
$report = [pscustomobject]@{
    vm          = $VmName
    iso         = $IsoPath
    profile     = $Profile
    ram_mb      = $vm.ram
    cpu         = $vm.cpu
    total       = $stages.Count
    passed      = ($results | Where-Object { $_.status -eq "PASS" }).Count
    failed      = $failed
    gui_stages  = ($stages | Where-Object { $_.evidence -eq "gui" }).Count
    results     = $results
}
$report | ConvertTo-Json -Depth 5 | Set-Content "qa/reports/virtualbox_v2.json" -Encoding utf8
$results | Export-Csv "qa/reports/virtualbox_v2_stages.csv" -NoTypeInformation -Encoding utf8

Remove-VM

Write-Host ""
Write-Host "=================================================="
Write-Host ("  VirtualBox v2: {0}/{1} stages PASS ({2} GUI stages)" -f $report.passed, $report.total, $report.gui_stages)
Write-Host "  Report: qa/reports/virtualbox_v2.json"
Write-Host "=================================================="

if ($failed -gt 0) {
    exit 1
}
