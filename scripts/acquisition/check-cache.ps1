# check-cache.ps1 - the PowerShell twin of check-cache.sh.
#
# ⭐ THE FIRST HARNESS TWIN `CI-07` HAS, and it exists to prove `store-lib.ps1`
# as much as to prove the cache. That entry's sweep classified the declared rows
# into four kinds and put this one in class A: the subject is a Rust example both
# lanes build, and the only missing half was the harness.
#
# ⛔ WHAT THIS PAIR IS FOR IS NOT A SECOND OPINION ABOUT THE CACHE. Both halves
# drive the SAME example, so they cannot disagree about the cache's behaviour;
# what they can disagree about is the machinery underneath - the plant probes,
# the row accounting and the verdict - which is exactly what
# `store-lib.ps1` newly supplies and what `check-twins` compares per run.
#
# `ACQ-05`'s Prove is both halves at once: the cache tests enforce the licence
# register, and the artifact's identity survives a source URL change. The sh half
# carries the argument for every case; this one asks the same questions in the
# same order so the two reports are comparable line for line.
#
# ⚠ NOTHING HERE PLANTS IN A TRACKED FILE. The register is checked out, not
# scratch, so the probe guards plant in a COPY of it.
#
# Usage:
#   pwsh -NoProfile -File scripts/acquisition/check-cache.ps1
#   pwsh -NoProfile -File scripts/acquisition/check-cache.ps1 -Json
#
# Exit codes: 0 every case held, 1 one did not, 2 could not run.
[CmdletBinding(PositionalBinding = $false)]
param([switch]$Json)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
# ⛔ A NATIVE COMMAND'S EXIT CODE IS THE VERDICT HERE, SO IT MUST NOT THROW.
# This defaults to $true from PowerShell 7.5, which turns every non-zero exit
# into a terminating error under $ErrorActionPreference = 'Stop': a scenario that
# REFUSES stops being a code this file can read and becomes an exception nobody
# caught. docs/conventions/shell.md section 8.
$PSNativeCommandUseErrorActionPreference = $false

$here = Split-Path -Parent $PSCommandPath
$root = Split-Path -Parent (Split-Path -Parent $here)

# ⚠ $ME is read by store-lib.ps1, which this dot-sources on the next line.
$ME = 'check-cache'
. (Join-Path $root (Join-Path 'scripts' (Join-Path 'corpus' 'store-lib.ps1')))

Assert-StoreTools cargo go
$scenario = Build-StoreExample -Root $root -Example 'cache-scenario'

$work = New-StoreWorkdir -Tag 'checkcache'
try {
    # ⭐ THE LICENCE ANSWER COMES FROM THE GO BINARY NOW, CI-10. The register's
    # reader was an .sh half and a hand-written .ps1 twin; it is one program, so
    # this caller and its sh twin ask the same implementation rather than two.
    # ⛔ A missing binary is 'could not run' and never an empty permitted list.
    $licences = Join-Path $work 'bit-check'
    if ($IsWindows) { $licences = $licences + '.exe' }
    $toolDir = Join-Path $root (Join-Path 'tools' 'check')
    Push-Location $toolDir
    & go build -o $licences .
    $buildRc = $LASTEXITCODE
    Pop-Location
    if ($buildRc -ne 0) {
        [Console]::Error.WriteLine('check-cache: tools/check did not build')
        exit 2
    }

    # ⛔ The answer is kept in a FILE rather than a variable, so an empty answer
    # and a failed call are distinguishable - which is the sh half's reason and
    # is not weaker here.
    $permittedPath = Join-Path $work 'permitted'
    & $licences check-licences --permitted > $permittedPath 2> (Join-Path $work 'permitted.err')
    if ($LASTEXITCODE -ne 0) {
        [Console]::Error.WriteLine("check-cache: check-licences -Permitted exited $LASTEXITCODE")
        exit 2
    }

    # ⚠ THE OUTER @() IS LOAD-BEARING UNDER Set-StrictMode. A read of a file with
    # one line yields a string rather than an array, and `.Count` on a string is
    # its length.
    $permitted = @(Get-Content -LiteralPath $permittedPath | Where-Object { $_ -ne '' })
    if ($permitted.Count -eq 0) {
        Add-StorePass 'register  nothing in the register permits keeping an artifact''s bytes'
    } else {
        Add-StoreFail ("register  the register permits $($permitted.Count) target(s): " +
            ($permitted -join ' '))
    }

    $scenarioArgs = @()
    foreach ($id in $permitted) { $scenarioArgs += @('--permitted', $id) }

    $refusedOut = Join-Path $work 'refused.out'
    $refusedErr = Join-Path $work 'refused.err'
    & $scenario @scenarioArgs > $refusedOut 2> $refusedErr
    $refusedRc = $LASTEXITCODE
    if ($refusedRc -eq 0) {
        Add-StorePass 'scenario  the cache behaves as the model says under the register''s answer'
    } else {
        $why = (@(Get-Content -LiteralPath $refusedErr | Select-Object -First 3) -join ' ')
        Add-StoreFail "scenario  exit ${refusedRc}: $why"
    }

    $refusedText = @(Get-Content -LiteralPath $refusedOut)

    # ⛔ THE IDENTITY HALF OF THE PROVE. One artifact and two retrievals means the
    # digest named the same thing from both locations.
    if (($refusedText | Where-Object { $_.Contains('artifacts: 1') }) -and
        ($refusedText | Where-Object { $_.Contains('retrievals: 2') })) {
        Add-StorePass 'identity  a moved source adds a retrieval and not an artifact'
    } else {
        $saw = (@($refusedText | Where-Object { $_ -match '^(artifacts|retrievals):' }) -join ' ')
        Add-StoreFail "identity  a moved source produced: $saw"
    }

    if ($refusedText | Where-Object { $_.Contains('keeping nothing: accepted') }) {
        Add-StorePass 'policy    a cache that keeps no bytes is accepted'
    } else {
        Add-StoreFail 'policy    a cache that keeps no bytes was refused'
    }

    if ($refusedText | Where-Object { $_.Contains('keeping the bytes: refused as E-CAC-01') }) {
        Add-StorePass 'E-CAC-01  keeping the bytes is refused while the register refuses them'
    } else {
        $saw = (@($refusedText | Select-Object -Last 2) -join ' ')
        Add-StoreFail "E-CAC-01  the refusal did not appear: $saw"
    }

    # ⛔ THE CONTROL THE REFUSAL NEEDS. Without this the case above passes over a
    # cache that can never store anything, which is a different program from one
    # that asks the register.
    $permittedOut = Join-Path $work 'permitted.out'
    $permittedErr2 = Join-Path $work 'permitted.err2'
    & $scenario --permitted aria2 > $permittedOut 2> $permittedErr2
    $permittedRunRc = $LASTEXITCODE
    $permittedText = @(Get-Content -LiteralPath $permittedOut)
    if ($permittedRunRc -ne 0) {
        $why = (@(Get-Content -LiteralPath $permittedErr2 | Select-Object -First 2) -join ' ')
        Add-StoreFail "control   the permitted run exited ${permittedRunRc}: $why"
    } elseif ($permittedText | Where-Object { $_.Contains('keeping the bytes: permitted by the register') }) {
        Add-StorePass 'control   the same cache keeps the bytes when a target is permitted'
    } else {
        Add-StoreFail 'control   the permitted run did not keep the bytes'
    }

    # ⚠ And the two runs differ only in that line, so the refusal is about the
    # permission and not about anything else the scenario did.
    $diff = @(Compare-Object -ReferenceObject $refusedText -DifferenceObject $permittedText -SyncWindow 0)
    if ($diff.Count -eq 0) {
        Add-StoreFail 'control   the refused and permitted runs are identical, so nothing changed'
    } elseif ($diff.Count -eq 2) {
        Add-StorePass 'control   the two runs differ on exactly the permission line'
    } else {
        Add-StoreFail "control   the two runs differ in $($diff.Count) line(s), expected 2"
    }

    # ⚠ The probe guards need a file to plant in, and it is a copy rather than
    # the register itself.
    $register = Join-Path $work 'register.toml'
    Copy-Item -LiteralPath (Join-Path $root (Join-Path 'catalogue' 'licences.toml')) `
        -Destination $register -Force
    Test-StoreProbeGuards -Path $register `
        -Present 'artifact_policy = "measurements-only"' -Ambiguous 'redistribute'

    exit (Write-StoreReport -Schema 'check-cache/1' -Noun 'cases' -AsJson:$Json)
} finally {
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
}
