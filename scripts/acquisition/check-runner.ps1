# check-runner.ps1 - prove the Windows disposable-host guards refuse what they
# exist to refuse.
#
# ⭐ THE TWIN OF check-runner.sh, and it exists because the guards stopped being
# Linux-only. CI-03 wrote assert-disposable.ps1; this is what says it works, and
# it is the reason `check-runner` is no longer a declared gap on the PowerShell
# lane.
#
# ⛔ EVERY CASE READS THE EXIT CODE FROM THE PROCESS THAT PRODUCED IT. pwsh's
# $LASTEXITCODE after `& pwsh -File ...` is that process's, and a pipeline's is
# not. This repository's oldest stated rule.
#
# ⚠ EVERY CLAIM GOES TO A SCRATCH DIRECTORY. The sh half's own header records
# what happens otherwise: a case run without the override claimed the real
# machine, passed once, and failed on every run afterwards. On a persistent
# runner that is a check which goes green the day it is written and red forever,
# for a reason that looks like the guard being broken rather than the test.
#
# ⭐ AND THE ROUTE FIXTURES ARE WHY THIS RUNS ANYWHERE. assert-disposable.ps1
# takes -RouteTable so the routing source can be a file, which means the guard's
# logic is provable on a host with no Get-NetRoute at all. What is NOT proved
# here is that Get-NetRoute's real output matches these fixtures; the capture
# workflow's own windows job is what establishes that, by running the guard with
# no -RouteTable on a real Windows host.
#
# Usage:
#   pwsh -NoProfile -File scripts/acquisition/check-runner.ps1
#   pwsh -NoProfile -File scripts/acquisition/check-runner.ps1 -Json
#
# Exit codes: 0 every guard refused its defect, 1 one did not, 2 could not run.

[CmdletBinding(PositionalBinding = $false)]
param([switch]$Json)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$here = Split-Path -Parent $PSCommandPath
$guard = Join-Path $here 'assert-disposable.ps1'
if (-not (Test-Path -LiteralPath $guard -PathType Leaf)) {
    Write-Error -ErrorAction Continue "check-runner: $guard is not present"
    exit 2
}

$work = Join-Path ([System.IO.Path]::GetTempPath()) ("checkrunner." + $PID)
$null = New-Item -ItemType Directory -Force -Path $work
try {
    $pass = 0
    $fail = 0
    $rows = New-Object System.Collections.ArrayList

    function Add-Pass([string]$Name) {
        [void]$rows.Add("  ✅ ok    $Name"); $script:pass++
    }
    function Add-Fail([string]$Name) {
        [void]$rows.Add("  ❌ FAIL  $Name"); $script:fail++
    }

    # ⛔ Unpiped. Output is redirected to a file and $LASTEXITCODE is read on the
    # next line.
    function Invoke-Guard {
        param([string[]]$Arguments)
        $log = Join-Path $work 'out'
        & pwsh -NoProfile -File $guard @Arguments *> $log
        return $LASTEXITCODE
    }

    # ⛔ A CASE MAY REQUIRE THE VERDICT AS WELL AS THE CODE, AND SOME MUST. Three
    # of this guard's refusals share exit 2, and two share exit 1, so a case
    # asserting the code alone passes when a different refusal fired. A mutation
    # pass found exactly that: blanking the mode check left both mode cases green,
    # because the Egress branch it then fell into also answers 2.
    function Expect {
        param([int]$Want, [string]$Name, [string[]]$Arguments, [string]$Saying = '')
        $got = Invoke-Guard -Arguments $Arguments
        $said = ''
        $log = Join-Path $work 'out'
        if (Test-Path -LiteralPath $log) {
            $said = (Get-Content -LiteralPath $log -Raw -ErrorAction SilentlyContinue)
        }
        if ($null -eq $said) { $said = '' }
        if ($got -ne $Want) { Add-Fail "$Name (wanted exit $Want, got $got)" }
        elseif ($Saying -and $said -notmatch [regex]::Escape($Saying)) {
            Add-Fail "$Name (exit $Want, but did not say '$Saying')"
        }
        else { Add-Pass $Name }
    }

    # --- forbidden egress ---------------------------------------------------
    # A table with a default route is a host that can reach the public network,
    # and a capture there puts an untrusted client on it.
    $open = Join-Path $work 'route-open'
    "Ethernet 0.0.0.0/0`nEthernet 192.168.0.0/24" | Set-Content -LiteralPath $open
    Expect 1 'an IPv4 default route is refused' @('-Egress', '-RouteTable', $open)

    # ⛔ AND THE IPv6 ONE SEPARATELY. A Windows host with IPv4 unplugged and IPv6
    # up still reaches the internet, and a guard reading only the first family
    # would pass it. That is the one-gated-door defect on a protocol rather than
    # on a code path.
    $v6 = Join-Path $work 'route-v6'
    "Ethernet ::/0`nEthernet fe80::/64" | Set-Content -LiteralPath $v6
    Expect 1 'an IPv6 default route is refused on its own' @('-Egress', '-RouteTable', $v6)

    # ⚠ A prefix that merely starts with the default's digits is not a default
    # route. Without this case a substring match would pass every one of the
    # cases above and refuse a host whose only route is 0.0.0.0/8.
    $near = Join-Path $work 'route-near'
    "Ethernet 0.0.0.0/8`nEthernet 10.0.0.0/8" | Set-Content -LiteralPath $near
    Expect 0 'a non-default prefix that looks like one is not egress' @('-Egress', '-RouteTable', $near)

    # Loopback only, which is what a capture host looks like.
    $loop = Join-Path $work 'route-loopback'
    "Loopback 127.0.0.0/8`nLoopback ::1/128" | Set-Content -LiteralPath $loop
    Expect 0 'loopback only passes' @('-Egress', '-RouteTable', $loop)

    # ⛔ An unreadable table is exit 2, not exit 0. Not knowing is not a pass.
    Expect 2 'an unreadable table is not a pass' @(
        '-Egress', '-RouteTable', (Join-Path $work 'no-such-table')) 'could not be established'

    # --- persistent state ---------------------------------------------------
    $fresh = Join-Path $work 'host-a'
    $env:BIT_IDS_STATE_DIR = $fresh
    try {
        $first = Invoke-Guard -Arguments @('-Claim', '-RunId', 'capture-0001')
        if ($first -eq 0) { Add-Pass 'a fresh host claims' }
        else { Add-Fail "a fresh host claims (exit $first)" }

        # ⛔ TWO INDEPENDENT REFUSALS GUARD A SECOND CLAIM, AND THEY MASKED EACH
        # OTHER. The marker check and the exclusive create both answer exit 1, so
        # deleting either left this case green. The message is what separates
        # them: this asserts the marker check fired, and the exclusive create is
        # the one below.
        Expect 1 'a second capture on one host is refused' `
            @('-Claim', '-RunId', 'capture-0002') 'already ran a capture'

        # ⚠ Destroying the marker is what a real teardown does. This shows the
        # NEXT job is unblocked once it happens, which is the other half of the
        # same guard: it must refuse a survived host without refusing every host
        # forever.
        Remove-Item -LiteralPath $fresh -Recurse -Force
        $third = Invoke-Guard -Arguments @('-Claim', '-RunId', 'capture-0003')
        if ($third -eq 0) { Add-Pass 'a torn-down host claims again for the next job' }
        else { Add-Fail "a torn-down host claims again for the next job (exit $third)" }

        # ⚠ A run id that is not a slug is exit 2, not a claim under a name the
        # record could not carry.
        $bad = Join-Path $work 'host-b'
        $env:BIT_IDS_STATE_DIR = $bad
        Expect 2 'a run id that is not a slug is refused' @('-Claim', '-RunId', 'Capture 0001') 'run id must be lowercase'

        # ⛔ AND TWO MODES AT ONCE IS REFUSED. A caller that passed both switches
        # would otherwise get whichever branch is written first, silently.
        Expect 2 'two modes at once is refused' @('-Claim', '-RunId', 'x', '-Egress') 'assert-disposable.ps1 -Claim'
        Expect 2 'no mode at all is refused' @() 'assert-disposable.ps1 -Claim'
    }
    finally {
        Remove-Item Env:\BIT_IDS_STATE_DIR -ErrorAction SilentlyContinue
    }

    # The fingerprint the next job compares against.
    $log = Join-Path $work 'print'
    & pwsh -NoProfile -File $guard -Fingerprint *> $log
    $printRc = $LASTEXITCODE
    $print = (Get-Content -LiteralPath $log -TotalCount 1 -ErrorAction SilentlyContinue)
    if ($printRc -eq 0 -and $print -match '^[0-9a-f]{64}$') {
        Add-Pass 'the host fingerprint is a sha256'
    }
    else {
        Add-Fail "the host fingerprint is not a sha256 (exit $printRc)"
    }

    $total = $pass + $fail
    if ($Json) {
        # ⚠ Built by interpolation rather than with -f. A .NET format string's
        # {0} is indistinguishable from an unfilled placeholder to
        # check-placeholders, which refused the first version of this line. The
        # guard is right to be blunt about that shape, so the line avoids it.
        Write-Output ('{"schema":"check-runner/1","total":' + $total +
            ',"passed":' + $pass + ',"failed":' + $fail +
            ',"fingerprint":"' + $print + '"}')
        if ($fail -gt 0) { exit 1 }
        exit 0
    }

    Write-Output ''
    $rows | ForEach-Object { Write-Output $_ }
    Write-Output ''
    Write-Output ("$total guard case(s): $pass passed, $fail failed")
    Write-Output ("host fingerprint for the next job: $print")
    # ⛔ Zero cases passing is not a green run, whatever the failure count says.
    if ($pass -eq 0) { Write-Output '❌ NOTHING RAN.'; exit 1 }
    if ($fail -gt 0) { Write-Output '❌ a guard did not refuse what it exists to refuse.'; exit 1 }
    Write-Output '✅ every guard refused its own defect.'
    exit 0
}
finally {
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
}
