<#
.SYNOPSIS
    HCS Linux — VirtualBox end-to-end gate (v2 plan §7.4).

.DESCRIPTION
    Boots the QA image in a headless VirtualBox VM and collects the evidence the
    in-guest agent produced.

    WHY THE DRIVER IS THIS SHAPE

    The v2 driver shipped broken in a way that no test could catch, because every
    piece of it depended on something that was never true:

      * it waited for a host-side journal that the guest never wrote there;
      * it invoked `VBoxManage guestcontrol`, which needs Guest Additions and
        credentials that a live ISO does not have;
      * it passed the scenario to a path inside the guest that did not exist;
      * it expected screenshots on the host that nothing had copied.

    So the run could only ever end in "the QA agent never reported in".

    The agent now runs the whole suite *inside* the guest and writes its results
    to a disk the host attached. That removes the host from the data path
    entirely: no guest additions, no shared folder, no socket, no keystrokes
    sent at a boot menu. The host's whole job is to attach a blank disk, boot,
    wait for a DONE marker, and read the disk back.

    The QA image is a separate build (scripts/build_iso.sh --qa) whose default
    boot entry carries hcs.qa=1. The production image cannot start the agent at
    all, so a mistyped flag here cannot reach a user's machine.

.PARAMETER IsoPath
    The QA image. Must be built with `scripts/build_iso.sh <version> <arch> --qa`.

.PARAMETER Profile
    The memory profile. Two are run for a release because a gate that only
    passes on a well-provisioned machine is not a gate.

.PARAMETER TimeoutMinutes
    How long to wait for the guest to write its DONE marker.

.EXAMPLE
    pwsh -File scripts/qa_virtualbox_v2.ps1 -IsoPath dist/HCS-Linux-2.0.0-qa-amd64.iso
#>
[CmdletBinding()]
param(
    [string]$IsoPath = "dist\HCS-Linux-2.0.0-qa-amd64.iso",
    [string]$VmName = "HCS-Linux-QA-v2",
    [ValidateSet("edge", "lowram", "workstation")]
    [string]$Profile = "edge",
    [string]$OutDir = "qa/screenshots/v2",
    [int]$TimeoutMinutes = 45,
    [string]$VBoxManagePath = ""
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

# ------------------------------------------------------------------ locate VBoxManage

function Resolve-VBoxManage {
    param([string]$Hint)
    if ($Hint -and (Test-Path -LiteralPath $Hint)) { return $Hint }
    $cmd = Get-Command VBoxManage -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    foreach ($p in @(
            "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe",
            "C:\Program Files (x86)\Oracle\VirtualBox\VBoxManage.exe")) {
        if (Test-Path -LiteralPath $p) { return $p }
    }
    throw "VBoxManage not found. Install VirtualBox, or pass -VBoxManagePath."
}

$VBoxManage = Resolve-VBoxManage -Hint $VBoxManagePath
Write-Host "[setup] VBoxManage: $VBoxManage"

# ------------------------------------------------------------------ profile matrix

# LOWRAM is the profile that catches a payload which quietly assumes more memory
# than it is allowed. A gate that only runs on the biggest machine is decoration.
$profiles = @{
    edge        = @{ ram = 8192;  cpu = 4; label = "EDGE-8GB" }
    lowram      = @{ ram = 4096;  cpu = 2; label = "LOWRAM-4GB" }
    workstation = @{ ram = 16384; cpu = 8; label = "WORKSTATION-16GB" }
}
$vm = $profiles[$Profile]

# ------------------------------------------------------------------ preflight

function Fail {
    param([string]$Message, [string[]]$Hints = @())
    Write-Host ""
    Write-Host "[FAIL] $Message" -ForegroundColor Red
    foreach ($h in $Hints) { Write-Host "       $h" }
    exit 1
}

if (-not (Test-Path -LiteralPath $IsoPath)) {
    Fail "The QA image is not there: $IsoPath" @(
        "It is a separate build from the production image, because the automated",
        "boot entry carries hcs.qa=1 and the production image must not have it:",
        "",
        "  bash scripts/build_iso.sh 2.0.0 amd64 --qa",
        "",
        "This gate cannot run against the production ISO. That is the point: the",
        "agent refuses to start without the flag, so a normal image is safe to ship."
    )
}

$isoFull = (Resolve-Path -LiteralPath $IsoPath).Path
if ($isoFull -notmatch '-qa-') {
    Fail "That does not look like a QA image: $isoFull" @(
        "Expected a name containing '-qa-', e.g. HCS-Linux-2.0.0-qa-amd64.iso.",
        "Booting the production image would simply never start the agent, and the",
        "run would fail with no useful information."
    )
}

# The evidence disk. A blank VHD the guest formats as FAT32 and writes into; the
# host reads it back with nothing but Mount-DiskImage, so the evidence never
# depends on third-party tooling being installed on the machine that grades it.
$evidenceDir = Join-Path $OutDir "evidence"
$shotsOut = Join-Path $OutDir "screenshots"
New-Item -ItemType Directory -Force -Path $evidenceDir | Out-Null
New-Item -ItemType Directory -Force -Path $shotsOut | Out-Null
$evidenceVhd = Join-Path $evidenceDir "evidence.vhd"
if (Test-Path -LiteralPath $evidenceVhd) { Remove-Item -LiteralPath $evidenceVhd -Force }

# ------------------------------------------------------------------ VM lifecycle

function Invoke-VBox {
    param([Parameter(Mandatory)][string[]]$VBoxArgs)
    $out = & $VBoxManage @VBoxArgs 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "VBoxManage $($VBoxArgs -join ' ') failed:`n$($out -join "`n")"
    }
    return $out
}

function Remove-QAVM {
    & $VBoxManage controlvm $VmName poweroff 2>$null | Out-Null
    Start-Sleep -Seconds 2
    & $VBoxManage unregistervm $VmName --delete 2>$null | Out-Null
}

function New-EvidenceVhd {
    # 512 MB dynamic. The frames are PNGs of a 1280x800 desktop; 32 of them plus
    # the journal is a few tens of megabytes, and FAT32 needs headroom anyway.
    Invoke-VBox @("createvm", "--name", "$VmName-disk", "--ostype", "Debian_64",
                  "--register", "--vhd") | Out-Null
    try {
        Invoke-VBox @("modifyvhd", "$VmName-disk.vhd", "--resize", "512") | Out-Null
    } finally {
        & $VBoxManage unregistervm "$VmName-disk" --delete 2>$null | Out-Null
    }
    if (-not (Test-Path -LiteralPath $evidenceVhd)) {
        throw "VHD was not created at $evidenceVhd"
    }
}

function New-QAVM {
    Write-Host "[setup] creating $VmName ($($vm.label): $($vm.ram) MB, $($vm.cpu) CPU)"
    Remove-QAVM
    Invoke-VBox @("createvm", "--name", $VmName, "--ostype", "Debian_64", "--register") | Out-Null
    Invoke-VBox @("modifyvm", $VmName, "--memory", "$($vm.ram)", "--cpus", "$($vm.cpu)") | Out-Null
    # 3D acceleration off on purpose: the image must reach a desktop without a
    # GPU, and a host GPU inside the VM would hide exactly the class of failure
    # this gate exists to catch.
    Invoke-VBox @("modifyvm", $VmName, "--graphicscontroller", "vmsvga",
                  "--vram", "32") | Out-Null
    # 1280x800 to match the render specs, so a VM frame is comparable with a
    # headless-render reference.
    Invoke-VBox @("modifyvm", $VmName, "--firmware", "bios") | Out-Null
    Invoke-VBox @("storageattach", $VmName, "--storagectl", "SATA",
                  "--port", "0", "--device", "dvddrive", "--type", "dvddrive",
                  "--medium", $isoFull) | Out-Null
    Invoke-VBox @("storageattach", $VmName, "--storagectl", "SATA",
                  "--port", "1", "--device", "harddisk", "--type", "hdd",
                  "--medium", $evidenceVhd) | Out-Null
    # No NAT forwarding and no guest additions: nothing needs to reach the guest,
    # and the agent does not listen on anything.
}

function Mount-Evidence {
    <#
        Returns the mounted volume's drive letter, or $null.
        Read-only: the evidence is the record of the run, and grading it must
        never be able to change it.
    #>
    try {
        $img = Mount-DiskImage -ImagePath $evidenceVhd -Access ReadOnly -PassThru -ErrorAction Stop
        Start-Sleep -Seconds 2
        $vol = $img | Get-Volume -ErrorAction SilentlyContinue |
               Where-Object { $_.DriveLetter } | Select-Object -First 1
        if ($vol) { return [string]$vol.DriveLetter }
        return $null
    } catch {
        Write-Host "       (evidence volume is not mountable yet: $($_.Exception.Message))"
        return $null
    }
}

function Unmount-Evidence {
    param($Image)
    if ($Image) {
        try { $Image | Dismount-DiskImage -ErrorAction SilentlyContinue } catch { }
    }
}

# ------------------------------------------------------------------ run

New-EvidenceVhd
New-QAVM

Write-Host "[run] booting the QA image (no keystrokes: the QA entry is the default)"
Invoke-VBox @("startvm", $VmName, "--type", "headless") | Out-Null

# Wait for the guest to write DONE. This is a wait for a *signal* — a marker the
# guest writes only after every result file is flushed — not a sleep, and not a
# guess about how long a boot takes.
$deadline = (Get-Date).AddMinutes($TimeoutMinutes)
$image = $null
$letter = $null
$donePath = $null
$lastNote = ""

while ((Get-Date) -lt $deadline) {
    $running = (& $VBoxManage list runningvms 2>$null) -join ""
    if ($running -notmatch [regex]::Escape($VmName)) {
        Remove-QAVM
        Fail "The VM powered itself off before finishing." @(
            "A live ISO that exits early has usually lost its session. Reproduce",
            "with:  VBoxManage startvm $VmName   and watch the console.",
            "If it boots to a text console, the graphical session is not starting;",
            "the same failure is visible in the guest at /var/log/hcs/session.log."
        )
    }

    if (-not $letter) {
        $letter = Mount-Evidence
        if ($letter) {
            Write-Host "[run] evidence volume mounted at ${letter}:"
            $image = Get-DiskImage -ImagePath $evidenceVhd
        }
    }

    if ($letter) {
        $candidate = "${letter}:\DONE"
        if (Test-Path -LiteralPath $candidate) {
            $donePath = $candidate
            break
        }
        $files = @(Get-ChildItem -LiteralPath "${letter}:\" -ErrorAction SilentlyContinue)
        $note = "evidence disk is present, $($files.Count) file(s) so far"
        if ($note -ne $lastNote) {
            Write-Host "[run] $note"
            $lastNote = $note
        }
    }

    Start-Sleep -Seconds 10
}

if (-not $donePath) {
    Unmount-Evidence -Image $image
    Remove-QAVM
    Fail "The guest never wrote its DONE marker within $TimeoutMinutes minutes." @(
        "The agent writes DONE last, after every result file is flushed, so its",
        "absence means the run did not finish rather than that it is slow.",
        "",
        "What to check, in order:",
        "  1. Is the image a QA build?  ls dist/*-qa-*.iso",
        "  2. Does the guest have a session at all? Boot it visibly and read",
        "     /var/log/hcs/session.log inside the guest.",
        "  3. Is the suite staged?  hcs-qa-agent validate-suite  in the guest.",
        "  4. Did the evidence disk appear as /dev/vdb? If not, the second SATA",
        "     attachment is wrong and the results stayed on the guest's RAM disk."
    )
}

Write-Host "[run] DONE marker found; collecting evidence"
# Power the VM off before reading, so nothing is mid-write.
& $VBoxManage controlvm $VmName acpipowerbutton 2>$null | Out-Null
Start-Sleep -Seconds 5
$stillRunning = (& $VBoxManage list runningvms 2>$null) -join ""
if ($stillRunning -match [regex]::Escape($VmName)) {
    & $VBoxManage controlvm $VmName poweroff 2>$null | Out-Null
    Start-Sleep -Seconds 3
}
if (-not $image) { $image = Get-DiskImage -ImagePath $evidenceVhd }
Unmount-Evidence -Image $image
Start-Sleep -Seconds 2
$image = Mount-DiskImage -ImagePath $evidenceVhd -Access ReadOnly -PassThru
$letter = ($image | Get-Volume | Where-Object { $_.DriveLetter } | Select-Object -First 1).DriveLetter
if (-not $letter) {
    Remove-QAVM
    Fail "The evidence disk has no readable volume." @(
        "The guest should have formatted it as FAT32. If it did not, /dev/vdb was",
        "missing or mkfs.vfat is not installed (see the package list)."
    )
}

# Copy everything out. The evidence is a record, so it is copied, not moved.
robocopy "${letter}:\" $evidenceDir /E /NFL /NDL /NJH /NJS /NP | Out-Null
if ($LASTEXITCODE -ge 8) {
    Write-Host "[WARN] robocopy reported $LASTEXITCODE; some files may be missing"
}
Unmount-Evidence -Image $image
Remove-QAVM

# ------------------------------------------------------------------ grade

# The pass/fail decision lives in scripts/grade_qa_evidence.py, not inline here.
# That script is covered by tests/unit/test_qa_grading.py, which is the point:
# the v2 driver decided pass/fail in PowerShell with no test reaching it, and
# reported a green run for a suite in which every stage had been skipped.
$resultFile = Join-Path $evidenceDir "result.json"
if (-not (Test-Path -LiteralPath $resultFile)) {
    Fail "The guest wrote DONE but no result.json. That is a bug in the agent." @(
        "DONE is written after result.json, so its absence means the write failed.",
        "The journal and any frames are still in $evidenceDir for inspection."
    )
}

New-Item -ItemType Directory -Force -Path "qa/reports" | Out-Null
$graderScript = Join-Path $PSScriptRoot "grade_qa_evidence.py"
$verdictFile = "qa/reports/virtualbox_v2.json"

Write-Host ""
Write-Host "[grade] re-checking the evidence independently of the guest..."
& python $graderScript $evidenceDir --report $verdictFile
$green = ($LASTEXITCODE -eq 0)

$verdict = Get-Content -LiteralPath $verdictFile -Raw | ConvertFrom-Json

# Name the frames the way a reviewer will look for them: by stage.
$shotsSrc = Join-Path $evidenceDir "shots"
if (Test-Path -LiteralPath $shotsSrc) {
    foreach ($f in Get-ChildItem -LiteralPath $shotsSrc -Filter *.png -Recurse) {
        Copy-Item -LiteralPath $f.FullName `
            -Destination (Join-Path $shotsOut "$($f.Directory.Name)_$($f.Name)") -Force
    }
}

# ------------------------------------------------------------------ report

$frames = @($verdict.host_recheck).Count
$disagreements = @($verdict.host_recheck | Where-Object { -not $_.ok }).Count

Write-Host ""
Write-Host "=================================================="
Write-Host ("  HCS Linux {0} - VirtualBox gate" -f $verdict.version)
Write-Host ("  Profile:      {0} ({1} MB, {2} CPU)" -f $verdict.profile, $vm.ram, $vm.cpu)
Write-Host ("  Stages:       {0}/{1} PASS" -f $verdict.passed, $verdict.total)
Write-Host ("    GUI:        {0}" -f $verdict.gui_stages)
Write-Host ("    Console:    {0}" -f $verdict.console_stages)
Write-Host ("  Frames:       {0} re-checked on the host, {1} disagreed" -f $frames, $disagreements)
if ($verdict.failed -gt 0) { Write-Host ("    FAILED:     {0}" -f $verdict.failed) }
if ($verdict.skipped -gt 0) { Write-Host ("    SKIPPED:    {0}  (a skipped stage is not a pass)" -f $verdict.skipped) }
Write-Host ("  Verdict:      {0}" -f $(if ($green) { "GREEN" } else { "NOT GREEN" }))
Write-Host "  Report:       $verdictFile"
Write-Host "  Evidence:     $evidenceDir"
Write-Host "  Frames:       $shotsOut"
Write-Host "=================================================="

if (-not $green) {
    Write-Host ""
    Write-Host "Why it is not green:"
    foreach ($r in $verdict.reasons) { Write-Host "  - $r" }
    Write-Host ""
    Write-Host "Each stage states what it proves in the manifest; the frames for a"
    Write-Host "failing stage are under $shotsSrc."
    exit 1
}

exit 0