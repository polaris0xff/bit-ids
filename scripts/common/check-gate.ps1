# check-gate.ps1 - run the whole local gate, in one command, and read every
# exit code from the process that produced it.
#
# ⭐ THE TWIN OF check-gate.sh, and the only one that runs at all on a native
# PowerShell session with no POSIX layer. It runs the .ps1 half of every pair,
# which is the half that host can actually execute.
#
# ⚠ IT IS NOT IN check-twins.sh's PAIR LIST, and that is deliberate rather than
# an omission. This runner INVOKES check-twins, so comparing the two runners
# from inside check-twins would recurse. scripts/README.md carries the same
# reasoning for why check-twins itself has no twin.
#
# The defect this exists to catch is a gate that is a LIST. A list run by hand
# is run in the order somebody recalls it. ⛔ The session that first wrote the
# sh half ran its gate five times and typed a different subset each time.
#
# ⭐ IT DELEGATES. It holds no rules of its own. Every verdict is some other
# script's, read unpiped.
#
# -- ⛔ A SKIPPED CHECK IS A SKIP, NEVER A PASS -----------------------------
#
# A runner that quietly dropped a check and printed green would be the row in
# docs/conventions/forbidden-patterns.md that reads *a step that exits 0 having
# done nothing it was asked to do*. A skip is counted, named, and printed on
# its own line. ⭐ -Strict makes a skip a failure, which is what a CI job should
# pass, since there the tools are installed on purpose and a skip means the
# install broke.
#
# -- ⛔ AND A CHECK THIS HOST CANNOT RUN IS NOT A CHECK THAT BROKE ------------
#
# ⛔ THIS LANE IS WHERE THAT DISTINCTION WAS MISSING AND IT COST THE FLAG.
# Six rows below are checks Windows genuinely cannot run, so -Strict would have
# refused every correct tree and the workflow ran without it. Measured on
# 2026-09-06: with check-project.ps1 rewritten to exit 2, this runner's own CI
# invocation still exited 0. A check that stopped running was indistinguishable
# from one that never could.
#
# ⭐ So a row is one of two kinds. A DECLARED unavailability is written here,
# with a reason and the entry that owns it, and prints as `n/a`. An OBSERVED
# skip is a check that ran and answered 2, or one whose file is gone, and
# prints as `SKIP`. -Strict refuses the second and permits the first.
#
# ⚠ -Fast counts as declared, because the caller asked for it rather than the
# host failing to supply it.
#
# ⛔ AND ZERO PASSES IS RED WHATEVER THE SKIPS SAY. The sh half produced
# exactly that on its first run, through a broken presence test: nine skips,
# zero failures, and a green verdict over nothing at all.
#
# Usage:
#   pwsh -NoProfile -File scripts/common/check-gate.ps1
#   pwsh -NoProfile -File scripts/common/check-gate.ps1 -Fast
#   pwsh -NoProfile -File scripts/common/check-gate.ps1 -Strict
#   pwsh -NoProfile -File scripts/common/check-gate.ps1 -Json
#
# Exit codes: 0 nothing failed, 1 something failed, 2 could not run.
#
# ⛔ Read the exit code from this process, unpiped.

# ⛔ PositionalBinding IS OFF. A stray expanded argument must fail to bind
# rather than land on the next free parameter. A sibling script in this
# directory committed under a fabricated author for exactly that reason.
[CmdletBinding(PositionalBinding = $false)]
param(
    [switch]$Json,
    [switch]$Fast,
    [switch]$Strict
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# ⛔ RESOLVED FROM THIS SCRIPT'S OWN LOCATION, not from the working directory.
# A runner found by a relative path runs a different set depending on who
# called it.
$here = Split-Path -Parent $PSCommandPath

$pass = 0
$fail = 0
$skip = 0
$na = 0
$rows = New-Object System.Collections.ArrayList
function Add-Row([string]$T) { [void]$rows.Add('  ' + $T) }

# ⛔ DECLARED HERE, WITH A REASON, OR IT IS NOT DECLARED. The reason is the
# whole difference between this row and an observed skip: one is a fact about
# the platform that somebody wrote down and can be argued with, and the other
# is a check that stopped working.
function Add-Unavailable([string]$Name, [string]$Reason) {
    Add-Row ('n/a   ' + $Name + '  (' + $Reason + ')')
    $script:na++
}

$logFile = Join-Path ([System.IO.Path]::GetTempPath()) ("checkgate." + $PID + ".log")

function Invoke-Check([string]$Name, [string]$Script, [string[]]$ExtraArgs = @()) {
    $path = Join-Path $here $Script
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        Add-Row ("SKIP  " + $Name + "  (not present)")
        $script:skip++
        return
    }
    # ⛔ THE EXIT CODE IS TAKEN FROM THE PROCESS, UNPIPED. Output is redirected
    # to a file and $LASTEXITCODE is read on the next line. Piping into
    # anything reports the pipeline's status, so a check that failed reads
    # green, and that is the defect this repository is most emphatic about.
    $argv = @('-NoProfile', '-File', $path) + $ExtraArgs
    & pwsh @argv *> $logFile
    $rc = $LASTEXITCODE
    switch ($rc) {
        0 { Add-Row ("✅ ok    " + $Name); $script:pass++ }
        2 {
            $first = ''
            if (Test-Path -LiteralPath $logFile) {
                $first = (Get-Content -LiteralPath $logFile -TotalCount 1 -ErrorAction SilentlyContinue)
            }
            if ($first -and $first.Length -gt 60) { $first = $first.Substring(0, 60) }
            Add-Row ("SKIP  " + $Name + "  (" + $first + ")")
            $script:skip++
        }
        default {
            Add-Row ("❌ FAIL  " + $Name + "  (exit " + $rc + ")")
            $script:fail++
            if (-not $Json -and (Test-Path -LiteralPath $logFile)) {
                Get-Content -LiteralPath $logFile -TotalCount 12 -ErrorAction SilentlyContinue |
                    ForEach-Object { Write-Output ('          ' + $_) }
            }
        }
    }
}

foreach ($c in 'check-docs', 'check-markers', 'check-one-home', 'check-placeholders',
                'check-control-bytes', 'check-changelog', 'check-no-secrets', 'check-project') {
    Invoke-Check $c ($c + '.ps1')
}

# ⚠ -Public is a DIFFERENT question from the default run, not a stricter one.
# Emails, absolute home paths and long hex are legitimate content in a private
# project, so this is a second call rather than a flag on the first.
Invoke-Check 'check-no-secrets -Public' 'check-no-secrets.ps1' @('-Public')

# ⚠ NEEDS gh AND THE NETWORK, so it exits 2 on a machine without them and that
# reads as a skip rather than a pass. Correct: nothing was verified.
Invoke-Check 'check-remote-items' 'check-remote-items.ps1'

# ⛔ DECLARED, NEVER OMITTED. check-runner mutation-proves the disposable-host
# guards, and those guards read /proc/net/route, so they are Linux-only today.
# A Windows capture host needs its own pair and ACQ-04 carries that as a named
# residual. ⚠ Leaving the row out entirely would make this lane look like it
# checked something it never looked at, which is the difference between a skip
# and a pass that this whole runner is about.
Add-Unavailable 'check-runner' 'the guards are Linux-only; ACQ-04 residual'

# ⛔ DECLARED FOR THE SAME REASON. check-store plants a symbolic link and a
# named pipe in a disposable tree, and neither is available to an unprivileged
# Windows session; check-corpus shares its harness. ⚠ The rules they prove are
# not Linux-only and the Rust suite exercises every one of them on both lanes;
# what this lane does not do is plant them against a real filesystem. CI-03
# owns the Windows runner.
Add-Unavailable 'check-store' 'the plants need a POSIX filesystem; CI-03 residual'
Add-Unavailable 'check-corpus' 'shares that harness; CI-03 residual'
Add-Unavailable 'check-indexes' 'shares that harness; CI-03 residual'
Add-Unavailable 'check-release' 'shares that harness; CI-03 residual'
Add-Unavailable 'check-publish' 'shares that harness; CI-03 residual'

# ⭐ THE SLOW ONE, and ⚠ it is the one part of this gate that needs a POSIX
# shell: check-twins runs the sh half of every pair, so it cannot run on a host
# without one. That is reported as a skip, never as a pass.
# ⚠ THE TWO BRANCHES BELOW ARE DIFFERENT KINDS AND ARE COUNTED DIFFERENTLY.
# -Fast is the caller declining the check, so it is declared. A host with no
# POSIX shell is a capability this runner probed for and did not find, which is
# the same shape as a missing gh, so it is an observed skip and -Strict refuses
# it: nothing compared the two halves, and no flag was passed saying that was
# intended.
if ($Fast) {
    Add-Unavailable 'check-twins' '-Fast'
}
elseif (-not (Get-Command sh -ErrorAction SilentlyContinue)) {
    Add-Row 'SKIP  check-twins  (no POSIX shell on this host)'
    $skip++
}
else {
    $twins = Join-Path $here 'check-twins.sh'
    if (Test-Path -LiteralPath $twins -PathType Leaf) {
        & sh $twins *> $logFile
        $rc = $LASTEXITCODE
        if ($rc -eq 0) { Add-Row '✅ ok    check-twins'; $pass++ }
        else { Add-Row ("❌ FAIL  check-twins  (exit " + $rc + ")"); $fail++ }
    }
    else { Add-Row 'SKIP  check-twins  (not present)'; $skip++ }
}

Remove-Item -LiteralPath $logFile -ErrorAction SilentlyContinue

$total = $pass + $fail + $skip + $na

# ⛔ A RUN THAT PASSED NOTHING IS NOT A GREEN RUN.
# ⚠ -Strict reads $skip and never $na. A declared row is a documented gap and
# refusing it would refuse every correct tree on this lane.
if ($pass -eq 0)               { $rc = 1 }
elseif ($Strict -and $skip -gt 0) { $rc = 1 }
elseif ($fail -gt 0)           { $rc = 1 }
else                           { $rc = 0 }

if ($Json) {
    Write-Output ('{"schema":"check-gate/2","total":' + $total + ',"passed":' + $pass +
                  ',"failed":' + $fail + ',"skipped":' + $skip +
                  ',"unavailable":' + $na +
                  ',"strict":' + $(if ($Strict) { 'true' } else { 'false' }) + '}')
    exit $rc
}

Write-Output ''
$rows | ForEach-Object { Write-Output $_ }
Write-Output ''
Write-Output ("{0} checks: {1} passed, {2} failed, {3} skipped, {4} unavailable" -f $total, $pass, $fail, $skip, $na)

if ($skip -gt 0) {
    Write-Output '⚠ A SKIP IS NOT A PASS. Those checks did not run and nothing about'
    Write-Output 'their subject was verified. Pass -Strict to make a skip a failure.'
}
if ($na -gt 0) {
    Write-Output '⚠ An n/a row is a gap this runner declares, with the reason beside it.'
    Write-Output '-Strict permits those and refuses a SKIP, so read both numbers.'
}
if ($pass -eq 0) {
    Write-Output '❌ NOTHING RAN. Zero checks passed, so this is red whatever the skips say.'
}
elseif ($fail -gt 0) {
    Write-Output '❌ the gate is red.'
}
else {
    Write-Output '✅ nothing failed.'
    Write-Output '⚠ That is part (a) of the gate only. Driving the real thing and the'
    Write-Output 'deep reviews are the other two, and each is blind to what this catches.'
    Write-Output 'docs/methodology/gate.md.'
}
exit $rc
