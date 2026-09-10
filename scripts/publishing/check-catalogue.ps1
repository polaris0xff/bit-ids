# check-catalogue.ps1 - the PowerShell twin of check-catalogue.sh.
#
# ⭐ THE SECOND HARNESS TWIN `CI-07` HAS, and it needed no new library function:
# `check-cache` proved `store-lib.ps1`, and this row uses the same four calls.
# That is the sweep's finding turned into a second measurement - the class-A rows
# wait on ONE library, not on fifteen translations.
#
# ⛔ WHAT THIS PAIR CAN DISAGREE ABOUT. Both halves drive the same five Rust
# examples, so the publication and the answers are identical by construction;
# what differs is the harness - how each plants a defect, restores the
# publication, and counts a row. ⚠ Which is exactly where a twin is worth having:
# `check-catalogue.sh` restores the publication after every plant, and a half
# that restored it differently would report the same 18 cases over a different
# experiment.
#
# The sh half carries the argument for every case; this asks the same questions
# in the same order so the two reports are comparable line for line.
#
# ⚠ THE DIGEST COMES FROM .NET RATHER THAN FROM `sha256sum`, because a Windows
# runner has no such command. It is the same algorithm over the same file, and
# the value is only ever compared against one this run computed.
#
# Usage:
#   pwsh -NoProfile -File scripts/publishing/check-catalogue.ps1
#   pwsh -NoProfile -File scripts/publishing/check-catalogue.ps1 -Json
#
# Exit codes: 0 every case held, 1 one did not, 2 could not run.
[CmdletBinding(PositionalBinding = $false)]
param([switch]$Json)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
# ⛔ A NATIVE COMMAND'S EXIT CODE IS THE VERDICT HERE, SO IT MUST NOT THROW.
# docs/conventions/shell.md section 8.
$PSNativeCommandUseErrorActionPreference = $false

$here = Split-Path -Parent $PSCommandPath
$root = Split-Path -Parent (Split-Path -Parent $here)

# ⚠ $ME is read by store-lib.ps1, dot-sourced on the next line.
$ME = 'check-catalogue'
. (Join-Path $root (Join-Path 'scripts' (Join-Path 'corpus' 'store-lib.ps1')))

Assert-StoreTools cargo
$builder = Build-StoreExample -Root $root -Example 'build-store'
$indexer = Build-StoreExample -Root $root -Example 'build-indexes'
$formatter = Build-StoreExample -Root $root -Example 'build-formats'
$assembler = Build-StoreExample -Root $root -Example 'assemble-release'
$lookup = Build-StoreExample -Root $root -Example 'catalogue-lookup'

$work = New-StoreWorkdir -Tag 'checkcatalogue'
try {
    $bundle = Join-Path $work 'bundle'
    $scheme = 'fixture-client:-:3:3'
    $outPath = Join-Path $work 'out'
    $errPath = Join-Path $work 'err'

    function Build-Publication {
        if (Test-Path -LiteralPath $bundle) {
            Remove-Item -LiteralPath $bundle -Recurse -Force
        }
        New-Item -ItemType Directory -Path (Join-Path $bundle (Join-Path 'indexes' 'v1')) -Force | Out-Null
        & $builder --version 1.2.3 --version 1.2.10 $bundle *> $null
        if ($LASTEXITCODE -ne 0) { return $false }
        & $indexer --scheme $scheme $bundle (Join-Path $bundle (Join-Path 'indexes' (Join-Path 'v1' 'profiles.json'))) *> $null
        if ($LASTEXITCODE -ne 0) { return $false }
        & $formatter --scheme $scheme $bundle $bundle *> $null
        if ($LASTEXITCODE -ne 0) { return $false }
        Copy-Item -LiteralPath (Join-Path $root 'LICENSE') -Destination (Join-Path $bundle 'LICENSE') -Force
        & $assembler $bundle *> $null
        return ($LASTEXITCODE -eq 0)
    }

    # ⛔ Unpiped. Output to a file, $LASTEXITCODE on the next line.
    $script:RC = 0
    function Invoke-Ask {
        param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Question)
        & $lookup $bundle @Question > $outPath 2> $errPath
        $script:RC = $LASTEXITCODE
    }
    function Get-Out { @(Get-Content -LiteralPath $outPath -ErrorAction SilentlyContinue) }
    function Get-Err { @(Get-Content -LiteralPath $errPath -ErrorAction SilentlyContinue) }

    if (-not (Build-Publication)) {
        [Console]::Error.WriteLine('check-catalogue: cannot build the fixture publication')
        exit 2
    }
    $digest = 'sha256:' + (Get-FileHash -Algorithm SHA256 `
            -LiteralPath (Join-Path $bundle 'SHA256SUMS')).Hash.ToLowerInvariant()

    Invoke-Ask latest fixture-client linux x86-64 tar-gz
    if ($RC -eq 0) {
        Add-StorePass 'clean    a real publication opens and answers'
    } else {
        Add-StoreFail ("clean    the publication was refused (exit $RC): " +
            ((Get-Err | Select-Object -First 3) -join ' '))
    }

    # ⛔ THE ORDERING. As text, 1.2.3 sorts after 1.2.10.
    if (Get-Out | Where-Object { $_ -match ' 1\.2\.10$' }) {
        Add-StorePass 'ordering  latest answers 1.2.10 over 1.2.3'
    } else {
        Add-StoreFail ('ordering  latest answered ' + ((Get-Out) -join ' '))
    }

    # ⚠ A build line the catalogue does not carry answers nothing rather than the
    # nearest one, and the exit code says so.
    Invoke-Ask latest fixture-client windows x86-64 tar-gz
    # ⚠ THE OUTER @() IS LOAD-BEARING UNDER Set-StrictMode, which is the same
    # note assert-disposable.ps1 already carries. A pipeline that yields nothing
    # unrolls to $null, and `.Count` on $null is a terminating error rather than
    # zero - so the first run of this file died on the one case whose expected
    # answer is an EMPTY one.
    $absent = @(Get-Out)
    if ($RC -eq 1 -and $absent.Count -eq 0) {
        Add-StorePass 'absent   an unmeasured build line answers nothing, with exit 1'
    } else {
        Add-StoreFail "absent   exit $RC with $($absent.Count) row(s)"
    }

    Invoke-Ask lookup target fixture-client
    $rows = @(Get-Out | Where-Object { $_ -ne '' })
    if ($RC -eq 0 -and $rows.Count -eq 2) {
        Add-StorePass 'lookup   the target index answers both published records'
    } else {
        Add-StoreFail "lookup   exit $RC with $($rows.Count) row(s)"
    }

    # ⛔ THE OUT-OF-BAND DIGEST, WHICH IS THE ONE FILE NOTHING IN A PUBLICATION
    # PROVES. Both branches are cases: the right digest is accepted and a wrong
    # one is refused, because a check that only ever passes is not a check.
    & $lookup $bundle --expect-checksums $digest lookup target fixture-client > $outPath 2> $errPath
    $RC = $LASTEXITCODE
    if ($RC -eq 0 -and (Get-Err | Where-Object { $_.Contains("verified against the caller's digest") })) {
        Add-StorePass 'outofband  the caller''s digest is checked and the summary says so'
    } else {
        Add-StoreFail ("outofband  exit ${RC}: " + ((Get-Err | Select-Object -First 2) -join ' '))
    }

    # ⚠ COMPUTED, NEVER TYPED. A run of sixty-four hex digits written into a
    # tracked file is the shape `check-no-secrets --public` refuses, and it is
    # right to: this is a digest spelled without its algorithm anywhere else.
    $wrong = 'sha256:' + ('0' * 64)
    & $lookup $bundle --expect-checksums $wrong lookup target fixture-client > $outPath 2> $errPath
    $RC = $LASTEXITCODE
    if ($RC -eq 1 -and (Get-Err | Where-Object { $_.Contains('E-LIB-05') })) {
        Add-StorePass 'E-LIB-05  a checksum file the caller did not expect is refused'
    } else {
        Add-StoreFail "E-LIB-05  expected exit 1 with E-LIB-05, got $RC"
    }

    # ⚠ And a run with no digest says it trusted the file, rather than reading as
    # though it had verified it.
    Invoke-Ask lookup target fixture-client
    if (Get-Err | Where-Object { $_.Contains('checksums trusted') }) {
        Add-StorePass 'outofband  a run given no digest says the checksum file was trusted'
    } else {
        Add-StoreFail 'outofband  the summary does not say which verification ran'
    }

    # ⛔ THE PLAN, AND BOTH CLASSES NON-EMPTY. A plan that is all one class
    # classified nothing.
    Invoke-Ask plan
    $planned = Get-Out
    $immutable = @($planned | Where-Object { $_ -like 'immutable *' }).Count
    $current = @($planned | Where-Object { $_ -like 'current *' }).Count
    if ($RC -eq 0 -and $immutable -gt 0 -and $current -gt 0) {
        Add-StorePass "plan     $immutable immutable and $current current path(s), so neither class is empty"
    } else {
        Add-StoreFail "plan     exit $RC, $immutable immutable, $current current"
    }
    $outOfBand = @($planned | Where-Object { $_ -like '* out_of_band *' }).Count
    if ($outOfBand -eq 1) {
        Add-StorePass 'plan     exactly one path is proved by neither document'
    } else {
        Add-StoreFail "plan     $outOfBand path(s) marked out_of_band"
    }

    # -- The refusals, planted one at a time ---------------------------------
    #
    # ⛔ EVERY PLANT IS RESTORED AND THE RESTORED PUBLICATION IS RE-ASKED. A
    # harness that planted six defects in a row would be measuring their
    # composition rather than each one.
    function Test-Plant {
        param([string]$Name, [string]$Code)
        Invoke-Ask lookup target fixture-client
        if ($RC -ne 1) {
            Add-StoreFail "$Code  ${Name}: expected exit 1, got $RC"
            return
        }
        if (-not (Get-Err | Where-Object { $_.Contains($Code) })) {
            Add-StoreFail ("$Code  ${Name}: refused, but not as ${Code}: " +
                ((Get-Err | Select-Object -First 2) -join ' '))
            return
        }
        if (-not (Build-Publication)) {
            Add-StoreFail "$Code  ${Name}: could not restore the publication"
            return
        }
        Invoke-Ask lookup target fixture-client
        if ($RC -ne 0) {
            Add-StoreFail "$Code  ${Name}: the restored publication is not clean (exit $RC)"
            return
        }
        Add-StorePass "$Code  $Name"
    }

    $record = @(Get-ChildItem -LiteralPath (Join-Path $bundle 'profiles') -Filter '*.json' -Recurse -File |
        Sort-Object -Property FullName | Select-Object -First 1)[0].FullName
    Add-Content -LiteralPath $record -Value ''
    Test-Plant 'a record whose bytes moved after publication' 'E-LIB-02'

    Remove-Item -LiteralPath (Join-Path $bundle (Join-Path 'formats' 'bit-ids-v1.csv')) -Force
    Test-Plant 'a described file the publication does not carry' 'E-LIB-01'

    [System.IO.File]::WriteAllText((Join-Path $bundle (Join-Path 'formats' 'bit-ids-v1.extra.json')), "x`n")
    Test-Plant 'a carried file nobody described' 'E-LIB-03'

    Remove-Item -LiteralPath (Join-Path $bundle (Join-Path 'indexes' (Join-Path 'v1' 'profiles.json'))) -Force
    Test-Plant 'a publication with no index' 'E-LIB-08'

    Remove-Item -LiteralPath (Join-Path $bundle 'MANIFEST.json') -Force
    Test-Plant 'a publication with no manifest' 'E-LIB-08'

    # ⚠ The index rewritten to another generation, with the two root documents
    # reassembled over it so the digest check cannot fire first. Without that
    # this case would pass as E-LIB-02 and say nothing about compatibility.
    $indexPath = Join-Path $bundle (Join-Path 'indexes' (Join-Path 'v1' 'profiles.json'))
    $indexText = [System.IO.File]::ReadAllText($indexPath)
    [System.IO.File]::WriteAllText($indexPath, $indexText.Replace('bit-ids/index/1', 'bit-ids/index/2'))
    Remove-Item -LiteralPath (Join-Path $bundle 'MANIFEST.json') -Force
    Remove-Item -LiteralPath (Join-Path $bundle 'SHA256SUMS') -Force
    & $assembler $bundle *> $null
    Test-Plant 'an index document from another generation' 'E-LIB-06'

    # -- ⛔ THE NO-NETWORK CLAIM, SWEPT RATHER THAN OBSERVED -----------------
    $needles = 'std::net|TcpStream|TcpListener|UdpSocket|SocketAddr|reqwest|ureq|hyper::|curl::|\.connect\(|to_socket_addrs'
    $src = Join-Path $root (Join-Path 'crates' (Join-Path 'bit-ids' 'src'))
    $hits = @(Get-ChildItem -LiteralPath $src -Recurse -File |
        Select-String -Pattern $needles)
    if ($hits.Count -gt 0) {
        Add-StoreFail ('network  the crate names a socket or an HTTP client: ' +
            (($hits | Select-Object -First 2 | ForEach-Object { $_.Line.Trim() }) -join ' '))
    } else {
        Add-StorePass 'network  no socket, address type or HTTP client is named anywhere in the crate'
    }

    # ⚠ THE SWEEP IS CHECKED AGAINST A FILE THAT REALLY CARRIES ONE. A needle
    # list that had stopped matching anything would report the same clean answer
    # over a crate full of sockets.
    $lab = Join-Path $root (Join-Path 'crates' (Join-Path 'bit-ids-lab' 'src'))
    $labHits = if (Test-Path -LiteralPath $lab) {
        @(Get-ChildItem -LiteralPath $lab -Recurse -File | Select-String -Pattern $needles)
    } else { @() }
    if ($labHits.Count -gt 0) {
        Add-StorePass 'network  and the same needles do match the crate that owns the sockets'
    } else {
        Add-StoreFail 'network  the needle list matches nothing even in bit-ids-lab; it has stopped sweeping'
    }

    # And the dependency list, because a socket can arrive through a crate rather
    # than through a line of source.
    $manifest = Join-Path $root (Join-Path 'crates' (Join-Path 'bit-ids' 'Cargo.toml'))
    if (Get-Content -LiteralPath $manifest | Where-Object { $_ -match '^(reqwest|ureq|hyper|curl|tokio|async-std|isahc)' }) {
        Add-StoreFail 'network  the crate depends on a transport'
    } else {
        Add-StorePass 'network  and its dependency list carries no transport'
    }

    exit (Write-StoreReport -Schema 'check-catalogue/1' -Noun 'cases' -AsJson:$Json)
} finally {
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
}
