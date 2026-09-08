# assert-disposable.ps1 - refuse to install or run a client on a Windows host
# somebody keeps.
#
# ⭐ THE TWIN OF assert-disposable.sh, and the reason it exists is that the sh
# half reads /proc/net/route and /etc/machine-id. Neither is on Windows, so a
# Windows capture had no boundary to run before the install and was refused
# outright. CI-03 owns the pair.
#
# ⛔ THIS RUNS BEFORE THE INSTALL. The run manifest already refuses to RECORD a
# capture whose host was not disposable, and that guard cannot stop one: by the
# time a manifest exists, an untrusted installer has run. A record-layer refusal
# is a report, not a boundary.
#
# -- ⭐ TWO INDEPENDENT GUARDS, AND WHY THEY ARE NOT ONE ---------------------
#
#   -Claim        the host has never been used for a capture before
#   -Egress       the host cannot reach anything but loopback
#
# They fail for different reasons and neither implies the other. A fresh host
# with an open route leaks the capture onto the public network. A firewalled
# host that already ran a capture contaminates this one with the last one's
# state.
#
# -- ⛔ -Claim DETECTS THE FAILURE RATHER THAN TRUSTING THE CLAIM -------------
#
# The obvious design is a token the provisioner writes saying "this host is
# disposable". ⚠ That is a promise, and a promise fails silently: a runner
# misconfigured to persist its disk still carries the token. So this claims the
# host by WRITING a marker and refuses if one is already there. A second capture
# on one host means the host survived the first, whatever anything claimed.
#
# ⚠ THE MARKER LIVES WHERE A REAL TEARDOWN DESTROYS IT. ProgramData is the
# Windows counterpart of /var/lib: it survives a reboot, so a host that rebooted
# rather than being destroyed still reads as claimed. TEMP would not, and a
# guard cleared by a reboot reports a survived host as fresh.
#
# -- ⛔ THE ROUTE SOURCE IS AN ARGUMENT, FOR THE REASON THE SH HALF GIVES -----
#
# -RouteTable takes a file of `Get-NetRoute` output so the runner test can drive
# this against fixtures on a machine that has no Get-NetRoute at all, which is
# every Linux host this repository is developed on. A seam a test can use is a
# seam a misconfiguration can use, so a passing run prints which source it read.
#
# Usage:
#   pwsh -NoProfile -File scripts/acquisition/assert-disposable.ps1 -Claim <run-id>
#   pwsh -NoProfile -File scripts/acquisition/assert-disposable.ps1 -Egress [-RouteTable <path>]
#   pwsh -NoProfile -File scripts/acquisition/assert-disposable.ps1 -Fingerprint
#   pwsh -NoProfile -File scripts/acquisition/assert-disposable.ps1 -Marker
#
# ⛔ -Marker PRINTS THE MARKER'S PATH AND IS THE ONLY DERIVATION OF IT, for the
# reason the sh half gives: a caller that composed ProgramData\bit-ids\host-claimed
# itself would go on reading the old place the day the state directory moves,
# and report a host nobody claimed as fresh. capture-run.ps1 is that caller.
#
# Exit codes: 0 the guard passed, 1 the guard refuses, 2 could not run.
#
# ⛔ Read the exit code from this process, unpiped.

# ⛔ PositionalBinding IS OFF, for the reason the gate runner gives: a stray
# expanded argument must fail to bind rather than land on the next free
# parameter.
[CmdletBinding(PositionalBinding = $false)]
param(
    [switch]$Claim,
    [switch]$Egress,
    [switch]$Fingerprint,
    [switch]$Marker,
    [string]$RunId = '',
    [string]$RouteTable = '',
    [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
# ⛔ A NATIVE COMMAND'S EXIT CODE IS THE VERDICT HERE, SO IT MUST NOT THROW.
# This defaults to $true from PowerShell 7.5, which turns every non-zero exit
# into a terminating error under $ErrorActionPreference = 'Stop': a guard that
# REFUSES stops being a code a caller can read and becomes an exception nobody
# caught. docs/conventions/shell.md section 8.
$PSNativeCommandUseErrorActionPreference = $false

# ⛔ NO `break` INSIDE ForEach-Object, AND THAT IS NOT STYLE. `break` there does
# not stop the pipeline: it breaks the enclosing loop, and with none it unwinds
# the whole script, which exits 0. The first version did exactly that, so every
# misuse of this guard printed usage and reported success. `check-runner.ps1`
# caught it on its first run, over two cases written to check the opposite.
function Show-Usage {
    $lines = @(Get-Content -LiteralPath $PSCommandPath | Select-Object -Skip 1)
    $out = New-Object System.Collections.ArrayList
    foreach ($line in $lines) {
        if ($line -notmatch '^#') { break }
        [void]$out.Add(($line -replace '^# ?', ''))
    }
    $out
}

if ($Help) { Show-Usage; exit 0 }

$stateDir = $env:BIT_IDS_STATE_DIR
if ([string]::IsNullOrEmpty($stateDir)) {
    $programData = $env:ProgramData
    if ([string]::IsNullOrEmpty($programData)) { $programData = '/var/lib' }
    $stateDir = Join-Path $programData 'bit-ids'
}
$markerPath = Join-Path $stateDir 'host-claimed'

# A value that differs between two hosts and survives within one.
#
# ⚠ SEVERAL INPUTS RATHER THAN ONE, which is the sh half's rule and holds here
# for the same reason: any single source is a coincidence away from colliding.
# The machine GUID is Windows' counterpart of /etc/machine-id, and the install
# date and computer name differ where a cloned image's GUID might not.
function Get-HostFingerprint {
    $parts = New-Object System.Collections.ArrayList
    try {
        $key = 'HKLM:\SOFTWARE\Microsoft\Cryptography'
        if (Test-Path -LiteralPath $key) {
            $guid = (Get-ItemProperty -LiteralPath $key -Name MachineGuid -ErrorAction SilentlyContinue).MachineGuid
            if ($guid) { [void]$parts.Add($guid) }
        }
    }
    catch { }
    try {
        $install = 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion'
        if (Test-Path -LiteralPath $install) {
            $id = (Get-ItemProperty -LiteralPath $install -ErrorAction SilentlyContinue)
            if ($id.InstallDate) { [void]$parts.Add([string]$id.InstallDate) }
            if ($id.BuildLabEx) { [void]$parts.Add([string]$id.BuildLabEx) }
        }
    }
    catch { }
    # ⚠ These three exist on every platform PowerShell runs on, so a fingerprint
    # is never empty even where the registry is not. A guard whose fingerprint
    # is empty exits 2 below rather than claiming with nothing.
    [void]$parts.Add([System.Environment]::MachineName)
    [void]$parts.Add([System.Environment]::OSVersion.VersionString)
    [void]$parts.Add([System.Environment]::UserName)

    $text = ($parts -join "`n") + "`n"
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($text)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        ($sha.ComputeHash($bytes) | ForEach-Object { $_.ToString('x2') }) -join ''
    }
    finally { $sha.Dispose() }
}

# ⛔ Reports whether anything but loopback is reachable, WITHOUT sending a packet
# anywhere. The kernel's own routing table says what the host would do; probing a
# real host to see whether egress works would be reaching out from a machine this
# guard exists to establish is contained, which is the wrong order.
#
# ⚠ A default route is `0.0.0.0/0` in IPv4 and `::/0` in IPv6. BOTH ARE CHECKED,
# and that is not symmetry for its own sake: a Windows host with IPv4 unplugged
# and IPv6 up still reaches the internet, and a guard reading only the first
# would pass it.
#
# Returns 0 a public route exists, 1 none does, 2 could not establish.
function Test-PublicRoute {
    param([string]$Source)

    $lines = $null
    if (-not [string]::IsNullOrEmpty($Source)) {
        if (-not (Test-Path -LiteralPath $Source -PathType Leaf)) { return 2 }
        $lines = Get-Content -LiteralPath $Source -ErrorAction SilentlyContinue
        if ($null -eq $lines) { return 2 }
    }
    else {
        if (-not (Get-Command Get-NetRoute -ErrorAction SilentlyContinue)) { return 2 }
        try {
            $lines = Get-NetRoute -ErrorAction Stop |
                ForEach-Object { "$($_.InterfaceAlias) $($_.DestinationPrefix)" }
        }
        catch { return 2 }
        if ($null -eq $lines) { return 2 }
    }

    foreach ($line in $lines) {
        if ($line -match '(^|\s)(0\.0\.0\.0/0|::/0)(\s|$)') { return 0 }
    }
    return 1
}

# ⚠ THE OUTER @() IS load-bearing UNDER Set-StrictMode. A pipeline that yields
# exactly one object yields the object, not a one-element array, so `.Count`
# throws and every invocation of this script exits 1 with a property error. It
# did, on the first run of all eight cases.
$modes = @(@($Claim, $Egress, $Fingerprint, $Marker) | Where-Object { $_ })
if ($modes.Count -ne 1) {
    Show-Usage | Write-Error -ErrorAction Continue
    exit 2
}

if ($Fingerprint) {
    Write-Output (Get-HostFingerprint)
    exit 0
}

if ($Marker) {
    Write-Output $markerPath
    exit 0
}

if ($Egress) {
    $verdict = Test-PublicRoute -Source $RouteTable
    $read = if ([string]::IsNullOrEmpty($RouteTable)) { 'Get-NetRoute' } else { $RouteTable }
    switch ($verdict) {
        0 {
            Write-Error -ErrorAction Continue `
                'assert-disposable: a public route exists; a capture host reaches loopback only'
            exit 1
        }
        2 {
            Write-Error -ErrorAction Continue `
                "assert-disposable: $read is unreadable, so egress could not be established. That is not a pass."
            exit 2
        }
        default {
            Write-Output "no route off this host (read $read)"
            exit 0
        }
    }
}

# -Claim
if ($RunId -notmatch '^[a-z0-9-]+$') {
    Write-Error -ErrorAction Continue "assert-disposable: run id must be lowercase a-z0-9-: $RunId"
    exit 2
}

if (Test-Path -LiteralPath $markerPath) {
    Write-Error -ErrorAction Continue `
        'assert-disposable: this host already ran a capture, so it was not destroyed'
    exit 1
}

try { $null = New-Item -ItemType Directory -Force -Path $stateDir -ErrorAction Stop }
catch {
    Write-Error -ErrorAction Continue "assert-disposable: cannot create $stateDir"
    exit 2
}

$print = Get-HostFingerprint
if ([string]::IsNullOrEmpty($print)) {
    Write-Error -ErrorAction Continue 'assert-disposable: the host fingerprint is empty'
    exit 2
}

# ⛔ CREATED EXCLUSIVELY. Two captures racing on one host must not both believe
# they claimed it, so the file is opened CreateNew and a second opener fails
# rather than truncating. This is `set -C` in the sh half.
#
# ⚠ NOT REFUTED, AND KEPT. The Test-Path check above answers first on every input
# a test can construct, so rewriting CreateNew to Create changes no observable
# behaviour and check-runner.ps1 stays green over it. It matters only in the
# window between that check and this open, which is exactly the race it exists
# for and which no test here can enter. A reader should not take an unreached
# guard for a proven one. Same shape and same treatment as `entries.sort()` in
# PUB-01.
$stamp = [System.DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')
$body = "bit-ids/host-claim/1`nrun=$RunId`nclaimed_at=$stamp`nfingerprint=$print`n"
try {
    $stream = [System.IO.File]::Open(
        $markerPath, [System.IO.FileMode]::CreateNew,
        [System.IO.FileAccess]::Write, [System.IO.FileShare]::None)
    try {
        $bytes = [System.Text.Encoding]::UTF8.GetBytes($body)
        $stream.Write($bytes, 0, $bytes.Length)
    }
    finally { $stream.Dispose() }
}
catch {
    Write-Error -ErrorAction Continue 'assert-disposable: another capture claimed this host first'
    exit 1
}

Write-Output "$print (claimed $markerPath)"
exit 0
