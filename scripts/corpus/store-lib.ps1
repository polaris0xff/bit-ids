# store-lib.ps1 - the PowerShell twin of store-lib.sh.
#
# ⛔ IT IS DOT-SOURCED, NEVER RUN. It defines functions and initialises three
# accumulators, so dot-sourcing it twice resets the counters and running it does
# nothing else. `CI-07`'s sweep found that EVERY declared-unavailable harness
# whose subject is portable begins by sourcing `store-lib.sh`, and that no
# PowerShell twin of it existed - so this file is the first step of that entry
# rather than a translation of fifteen harnesses.
#
# A caller sets $ME to its own name for diagnostics, then calls
# Assert-StoreTools, Build-StoreExample and New-StoreWorkdir before anything
# else.
#
# -- ⛔ WHAT IS DELIBERATELY ABSENT ------------------------------------------
#
# `place`, `tree_digest` and `tree_files` have no twin here yet. They are used
# only by `check-store`, `check-corpus` and `check-indexes`, and `check-store` is
# `CI-07` class B - its plants are a symbolic link and a named pipe, which an
# unprivileged Windows session cannot create, so its plant set needs
# reconsidering rather than translating. ⛔ A function nothing calls is a
# function nobody knows works, and shipping three of those would make this
# library look more complete than it is measured to be. They land with the first
# twin that exercises them.
#
# -- ⚠ THE HAZARD THIS FILE IS MOST EXPOSED TO -------------------------------
#
# A PowerShell variable name is case-insensitive, so a parameter and a local
# differing only in case are ONE variable. This repository has recorded three
# instances - `$args`, `[switch]$Marker` against `$marker`, and `[switch]$Rows`
# against `$rows` - and all three were found by running something once rather
# than by reading it. Every accumulator here is `$script:`-scoped and prefixed,
# which is the same answer `store-lib.sh` reached after its own collision.

Set-StrictMode -Version Latest

# ⛔ PREFIXED AND SCRIPT-SCOPED, BECAUSE A DOT-SOURCED FILE SHARES ONE SCOPE WITH
# ITS CALLER. `store-lib.sh` records what the unprefixed names cost: a caller
# assigned its own ROWS for a row count, overwrote the accumulator, and the run
# printed ten passes over eight lines.
$script:StorePass = 0
$script:StoreFail = 0
$script:StoreRows = ''

# Every tool the harness needs, checked once and named when absent.
#
# ⛔ Exit 2, never 1. A machine that cannot run a check has not failed it, and
# the gate runner reads 2 as a skip.
function Assert-StoreTools {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Tool)
    foreach ($name in $Tool) {
        if (-not (Get-Command $name -ErrorAction SilentlyContinue)) {
            [Console]::Error.WriteLine("${ME}: $name not found")
            exit 2
        }
    }
}

# Builds one example and returns the path to it.
#
# ⛔ It checks the binary is there afterwards. A build that exits 0 having
# produced nothing is the "step that exits 0 having done nothing" row, and every
# case downstream of it would report a guard that failed to fire.
#
# ⛔ CARGO_TARGET_DIR IS ASKED FOR RATHER THAN ASSUMED AWAY. Composing the path
# as root/target while cargo obeys the environment is what silently stopped five
# provers on a host whose only oddity was a variable a great many Rust developers
# set; `store-lib.sh` carries the measurement.
#
# ⚠ `.exe` IS ASKED FOR THE SAME WAY. This twin runs on Windows, where the
# example cargo just built is `<name>.exe`, and a path composed without it exists
# on neither platform for a reason a reader would have to guess at.
function Build-StoreExample {
    param(
        [Parameter(Mandatory = $true)][string]$Root,
        [Parameter(Mandatory = $true)][string]$Example,
        [string]$Package = 'bit-ids'
    )
    & cargo build --manifest-path (Join-Path $Root 'Cargo.toml') -p $Package --locked `
        --example $Example *> $null
    if ($LASTEXITCODE -ne 0) {
        [Console]::Error.WriteLine("${ME}: cannot build the $Example example")
        exit 2
    }
    $target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $Root 'target' }
    foreach ($leaf in @($Example, "$Example.exe")) {
        $candidate = Join-Path $target (Join-Path 'debug' (Join-Path 'examples' $leaf))
        if (Test-Path -LiteralPath $candidate -PathType Leaf) { return $candidate }
    }
    $shown = Join-Path $target (Join-Path 'debug' (Join-Path 'examples' $Example))
    [Console]::Error.WriteLine("${ME}: $shown is not executable after a successful build")
    exit 2
}

# A scratch directory that the caller removes. ⚠ The caller installs the cleanup,
# because a `try/finally` opened inside a function closes when the function
# returns, and hiding that is worse than writing it twice.
function New-StoreWorkdir {
    param([Parameter(Mandatory = $true)][string]$Tag)
    $base = if ($env:TMPDIR) { $env:TMPDIR } else { [System.IO.Path]::GetTempPath() }
    $dir = Join-Path $base ".$Tag.$PID"
    try {
        New-Item -ItemType Directory -Path $dir -Force -ErrorAction Stop | Out-Null
    } catch {
        [Console]::Error.WriteLine("${ME}: cannot write to $dir")
        exit 2
    }
    return $dir
}

function Add-StoreRow {
    param([Parameter(Mandatory = $true)][string]$Text)
    $script:StoreRows = $script:StoreRows + "  $Text`n"
}

function Add-StoreFail {
    param([Parameter(Mandatory = $true)][string]$Text)
    Add-StoreRow "❌ $Text"
    $script:StoreFail = $script:StoreFail + 1
}

function Add-StorePass {
    param([Parameter(Mandatory = $true)][string]$Text)
    Add-StoreRow "✅ $Text"
    $script:StorePass = $script:StorePass + 1
}

# ⛔ EXACTLY ONCE, OR NOT AT ALL, AND LITERAL ON BOTH SIDES. A literal that
# matches twice edits something other than what the case names, and one that
# matches nothing edits nothing while the case still reports a guard that failed
# to fire.
#
# ⛔ THE COUNTING AND THE REPLACING SPEAK ONE LANGUAGE. `store-lib.sh` records
# what it cost when they did not: it counted with `grep -F`, which is literal,
# and replaced with `sed`, which is a regular expression - so `a.b` counted once
# at the end of `axb then a.b` and then replaced `axb`, and a literal carrying a
# `/` could not be planted at all. Here `IndexOf` and `Substring` are string
# operations with no pattern language behind them.
#
# ⚠ SINGLE-LINE LITERALS ONLY, AND THAT IS CHECKED RATHER THAN ASSUMED. The sh
# half refuses one because `grep -F` splits a pattern containing a newline into
# separate alternatives and miscounts it; this half has no such defect and
# refuses one anyway, because a pair whose halves accept different inputs is a
# pair `check-twins` cannot compare.
function Invoke-ReplaceOnce {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Literal,
        [Parameter(Mandatory = $true)][string]$Replacement
    )
    if ($Literal.Contains("`n") -or $Literal.Contains("`r")) { return $false }
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return $false }
    $text = [System.IO.File]::ReadAllText($Path)

    # ⚠ COUNTED WITH OVERLAP-FREE ADVANCEMENT, which is what `grep -o -F` does.
    # Counting with `Split` would answer one more than the occurrences and
    # counting with a regex would reintroduce the pattern language this function
    # exists to keep out.
    $count = 0
    $at = $text.IndexOf($Literal, [System.StringComparison]::Ordinal)
    $first = $at
    while ($at -ge 0) {
        $count = $count + 1
        $at = $text.IndexOf($Literal, $at + $Literal.Length, [System.StringComparison]::Ordinal)
    }
    if ($count -ne 1) { return $false }

    $after = $text.Substring(0, $first) + $Replacement + $text.Substring($first + $Literal.Length)
    if ($after -ceq $text) { return $false }

    # ⚠ A TEMP FILE BESIDE THE ORIGINAL, THEN A MOVE. A killed run leaves the
    # original intact rather than a half-written file.
    $temp = "$Path.replace-once"
    try {
        [System.IO.File]::WriteAllText($temp, $after)
        Move-Item -LiteralPath $temp -Destination $Path -Force -ErrorAction Stop
    } catch {
        if (Test-Path -LiteralPath $temp) { Remove-Item -LiteralPath $temp -Force }
        return $false
    }
    return $true
}

# The self-guards every harness that plants defects owes. ⭐ A probe's guard is a
# guard like any other, and this project has been burned by an unverified plant
# three times.
function Test-StoreProbeGuards {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Present,
        [Parameter(Mandatory = $true)][string]$Ambiguous
    )
    if (Invoke-ReplaceOnce -Path $Path -Literal 'a literal this file does not carry' -Replacement 'x') {
        Add-StoreFail 'probe    an absent literal was reported as planted'
    } else {
        Add-StorePass 'probe    an absent literal is refused'
    }

    if (Invoke-ReplaceOnce -Path $Path -Literal $Ambiguous -Replacement 'x') {
        Add-StoreFail 'probe    an ambiguous literal was reported as planted'
    } else {
        Add-StorePass 'probe    an ambiguous literal is refused'
    }

    if (Invoke-ReplaceOnce -Path $Path -Literal $Present -Replacement $Present) {
        Add-StoreFail 'probe    a no-op edit was reported as planted'
    } else {
        Add-StorePass 'probe    a no-op edit is refused'
    }

    if (Invoke-ReplaceOnce -Path $Path -Literal "$Present`n" -Replacement 'x') {
        Add-StoreFail 'probe    a multi-line literal was reported as planted'
    } else {
        Add-StorePass 'probe    a multi-line literal is refused'
    }

    # ⛔ AND THE TWO SHAPES THAT MADE THE COUNTING AND THE REPLACING DISAGREE IN
    # THE SH HALF. Each needs a file with specific content, so each writes its
    # own rather than editing the caller's - ⛔ in a scratch directory of its own,
    # never beside the caller's file, because two callers hand this a TRACKED
    # path and a sibling written next to it is an untracked dropping in the
    # source tree.
    $probeDir = Join-Path ([System.IO.Path]::GetTempPath()) ".storeprobe.$PID"
    try {
        New-Item -ItemType Directory -Path $probeDir -Force -ErrorAction Stop | Out-Null
    } catch {
        Add-StoreFail 'probe    a scratch directory for the plant probes could not be made'
        return
    }

    $metachar = Join-Path $probeDir 'metachar'
    [System.IO.File]::WriteAllText($metachar, "axb then a.b`n")
    if ((Invoke-ReplaceOnce -Path $metachar -Literal 'a.b' -Replacement 'PLANTED') -and
        ([System.IO.File]::ReadAllText($metachar).TrimEnd("`r", "`n") -ceq 'axb then PLANTED')) {
        Add-StorePass 'probe    a literal carrying a regex metacharacter plants where it occurs'
    } else {
        $saw = [System.IO.File]::ReadAllText($metachar).TrimEnd("`r", "`n")
        Add-StoreFail "probe    a metacharacter literal planted [$saw]"
    }

    $slash = Join-Path $probeDir 'slash'
    [System.IO.File]::WriteAllText($slash, "keep a/b here`n")
    if ((Invoke-ReplaceOnce -Path $slash -Literal 'a/b' -Replacement 'PLANTED') -and
        ([System.IO.File]::ReadAllText($slash).TrimEnd("`r", "`n") -ceq 'keep PLANTED here')) {
        Add-StorePass 'probe    a literal carrying a path separator can be planted'
    } else {
        $saw = [System.IO.File]::ReadAllText($slash).TrimEnd("`r", "`n")
        Add-StoreFail "probe    a literal carrying a slash planted [$saw]"
    }

    Remove-Item -LiteralPath $probeDir -Recurse -Force -ErrorAction SilentlyContinue
}

# The shared verdict. ⛔ A run that passed nothing is red whatever else it says:
# zero failures out of zero cases executed is the shape these runners exist to
# refuse.
function Write-StoreReport {
    param(
        [Parameter(Mandatory = $true)][string]$Schema,
        [Parameter(Mandatory = $true)][string]$Noun,
        [switch]$AsJson
    )
    $total = $script:StorePass + $script:StoreFail

    # ⛔ THE REPORT CHECKS ITSELF. The row list and the counters are two records
    # of one fact, and a value in two places with nothing comparing them is the
    # copy a reader trusts being the wrong one.
    $rowCount = @($script:StoreRows -split "`n" | Where-Object { $_ -ne '' }).Count
    if ($rowCount -ne $total) {
        [Console]::Error.WriteLine(
            "${ME}: $rowCount rows recorded, $total counted; the report does not describe itself")
        return 1
    }

    $rc = if ($script:StorePass -eq 0 -or $script:StoreFail -gt 0) { 1 } else { 0 }

    if ($AsJson) {
        [Console]::Out.WriteLine(
            '{"schema":"' + $Schema + '","total":' + $total +
            ',"passed":' + $script:StorePass + ',"failed":' + $script:StoreFail + '}')
        return $rc
    }

    [Console]::Out.WriteLine('')
    [Console]::Out.Write($script:StoreRows)
    [Console]::Out.WriteLine('')

    # ⛔ THE FAILING ROWS ARE REPRINTED AT THE END, AND THAT IS NOT DECORATION.
    # The gate shows an excerpt of a failed check's log, and a harness like this
    # prints dozens of passing rows before the one that failed.
    if ($script:StoreFail -gt 0) {
        [Console]::Out.WriteLine('the case(s) that failed:')
        foreach ($line in ($script:StoreRows -split "`n")) {
            if ($line.Contains('❌')) { [Console]::Out.WriteLine($line) }
        }
    }

    [Console]::Out.WriteLine(
        "$total ${Noun}: $($script:StorePass) passed, $($script:StoreFail) failed")
    if ($script:StorePass -eq 0) {
        [Console]::Out.WriteLine('❌ NOTHING RAN. Zero cases passed, so this is red whatever else it says.')
    } elseif ($script:StoreFail -gt 0) {
        [Console]::Out.WriteLine('❌ a guard did not refuse its defect.')
    } else {
        [Console]::Out.WriteLine('✅ every planted defect was refused, and the clean tree was not.')
    }
    return $rc
}
