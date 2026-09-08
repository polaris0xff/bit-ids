# capture-run.ps1 - the capture itself, on a Windows host that has already been
# claimed and already had its default routes deleted.
#
# ⭐ THE TWIN OF capture-run.sh. It exists for the reason assert-disposable.ps1
# exists: the sh half reads /proc/net/route and calls sha256sum, and a Windows
# capture host has neither. CI-03 owns the pair.
#
# ⛔ IT BUILDS NOTHING. By the time this runs there is no network, because the
# workflow deleted the default routes in both address families before calling
# it. A capture that discovered a missing dependency here would have to restore
# egress to fix it, on a host running a binary this project downloaded minutes
# ago. The observer is a path to an ALREADY BUILT binary and a missing one is
# `could not run`.
#
# -- ⛔ WHAT THIS IS NOT ------------------------------------------------------
#
# A FIXTURE capture. Nothing here installs a client and no stock BitTorrent
# build is observed; CLIENT-01 owns that, and `kind=fixture` in the attestation
# is what stops a bundle being mistaken for one.
#
# -- ⭐ THE DRIVER AND THE VERIFIER ARE BOTH SOMEBODY ELSE'S CODE -------------
#
# `curl.exe` ships with Windows and is on the hosted image; it is a complete
# HTTP client and it puts real bytes through the observer. `Get-FileHash` is
# .NET's SHA-256 rather than this project's. ⚠ THE TWO HALVES GENUINELY DIFFER
# HERE and the attestation says which was used: the sh half hands its checksum
# file to coreutils `sha256sum -c`, which does not exist on Windows. A twin that
# reported the same value would be describing a run that did not happen.
#
# ⚠ `curl` IS RESOLVED AS AN APPLICATION, never by name. Get-Command finds
# aliases and functions too, and a name that resolves to a cmdlet on one host
# and a binary on another is the aliased-tool hazard docs/conventions/shell.md
# section 8 names.
#
# Usage:
#   pwsh -NoProfile -File scripts/capture/capture-run.ps1 -RunId <id> -Out <dir>
#        -Observer <path> [-Seconds <n>] [-RouteTable <path>]
#
# Exit codes: 0 the capture ran and its evidence verifies, 1 a guard refuses,
# 2 could not run.
#
# ⛔ Read the exit code from this process, unpiped.

[CmdletBinding(PositionalBinding = $false)]
param(
    [string]$RunId = '',
    [string]$Out = '',
    [string]$Observer = '',
    [int]$Seconds = 15,
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

# ⛔ NO `break` INSIDE ForEach-Object; it unwinds the script and exits 0. That
# defect shipped in assert-disposable.ps1 and made every misuse report success.
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

function Deny([string]$Message) {
    Write-Error -ErrorAction Continue "capture-run: $Message"
    exit 1
}
function Cannot([string]$Message) {
    Write-Error -ErrorAction Continue "capture-run: $Message"
    exit 2
}

if ([string]::IsNullOrEmpty($RunId)) { Cannot '-RunId is required' }
if ([string]::IsNullOrEmpty($Out)) { Cannot '-Out is required' }
if ([string]::IsNullOrEmpty($Observer)) { Cannot '-Observer is required' }

# ⚠ The same slug rule the guard applies, restated because this value becomes a
# token on the wire and a key in a document before the guard is consulted.
if ($RunId -notmatch '^[a-z0-9-]+$') {
    Cannot "run id must be lowercase a-z0-9-: $RunId"
}
if ($Seconds -lt 1) { Cannot "-Seconds must be at least 1: $Seconds" }

$here = Split-Path -Parent $PSCommandPath
$root = Split-Path -Parent (Split-Path -Parent $here)
$guard = Join-Path $root 'scripts' 'acquisition' 'assert-disposable.ps1'
if (-not (Test-Path -LiteralPath $guard -PathType Leaf)) {
    Cannot "$guard is not present"
}

$curl = (Get-Command curl -CommandType Application -ErrorAction SilentlyContinue |
        Select-Object -First 1)
if ($null -eq $curl) { Cannot 'curl not found' }

# ⛔ THE BUILD RAN BEFORE THE NETWORK WAS CUT, OR IT DID NOT RUN.
if (-not (Test-Path -LiteralPath $Observer -PathType Leaf)) {
    Cannot "the observer $Observer is not present; it is built before the routes are deleted, never here"
}

# ⛔ A SECOND CAPTURE MUST NOT WRITE OVER THE FIRST ONE'S EVIDENCE.
if (Test-Path -LiteralPath $Out) {
    Deny "$Out already exists; a capture never writes into another run's evidence"
}

# ⛔ Unpiped, and $LASTEXITCODE read on the next line.
function Invoke-Guard {
    param([string[]]$Arguments, [string]$Log)
    & pwsh -NoProfile -File $guard @Arguments *> $Log
    return $LASTEXITCODE
}

$scratch = Join-Path ([System.IO.Path]::GetTempPath()) ("capturerun." + $PID)
$null = New-Item -ItemType Directory -Force -Path $scratch
try {
    $guardLog = Join-Path $scratch 'guard'

    # ⛔ THE MARKER'S PATH IS ASKED FOR, NEVER COMPOSED.
    $rc = Invoke-Guard -Arguments @('-Marker') -Log $guardLog
    if ($rc -ne 0) { Cannot 'the guard could not report its marker path' }
    $marker = (Get-Content -LiteralPath $guardLog -TotalCount 1)
    if ([string]::IsNullOrEmpty($marker)) { Cannot 'the guard reported an empty marker path' }

    # ⛔ THE CLAIM IS CHECKED HERE AS WELL AS RUN EARLIER. The claim is its own
    # workflow step, so a step order that dropped it leaves this capturing on a
    # host nothing claimed. A guard on one of two paths is the one-gated-door
    # defect docs/methodology/reviews.md calls the most recurring hole there is.
    if (-not (Test-Path -LiteralPath $marker -PathType Leaf)) {
        Deny "the host was never claimed; $marker does not exist"
    }
    $claimedRun = ''
    foreach ($line in (Get-Content -LiteralPath $marker)) {
        if ($line -cmatch '^run=(.*)$') { $claimedRun = $Matches[1]; break }
    }
    if ($claimedRun -cne $RunId) {
        Deny "the host is claimed by run [$claimedRun], not [$RunId]"
    }

    # ⛔ AND THE ROUTES ARE READ AGAIN AT THE MOMENT OF CAPTURE. The workflow ran
    # the same guard a step earlier; what this adds is that nothing between the
    # two steps put a route back.
    if ([string]::IsNullOrEmpty($RouteTable)) {
        $egressRc = Invoke-Guard -Arguments @('-Egress') -Log $guardLog
        $egressSource = 'Get-NetRoute'
    }
    else {
        $egressRc = Invoke-Guard -Arguments @('-Egress', '-RouteTable', $RouteTable) -Log $guardLog
        $egressSource = $RouteTable
    }
    if ($egressRc -eq 1) {
        Deny 'egress is open; a default route is still there and a capture would leak'
    }
    if ($egressRc -ne 0) {
        Cannot "egress could not be established from $egressSource"
    }

    $rc = Invoke-Guard -Arguments @('-Fingerprint') -Log $guardLog
    if ($rc -ne 0) { Cannot 'the host fingerprint could not be read' }
    $fingerprint = (Get-Content -LiteralPath $guardLog -TotalCount 1)

    $null = New-Item -ItemType Directory -Force -Path $Out
    $Out = (Resolve-Path -LiteralPath $Out).Path
    $bundle = Join-Path $Out 'bundle'
    $log = Join-Path $Out 'observer.log'
    $errLog = Join-Path $Out 'observer.err'
    $sums = Join-Path $Out 'SHA256SUMS'
    $attest = Join-Path $Out 'attestation.txt'

    $started = [System.DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')

    # ⚠ Two files rather than one: Start-Process cannot send both streams to the
    # same path. The sh half merges them, which is an honest difference and not
    # drift; nothing downstream reads the error file except a failure report.
    $proc = Start-Process -FilePath $Observer -ArgumentList @($bundle, "$Seconds") `
        -RedirectStandardOutput $log -RedirectStandardError $errLog `
        -NoNewWindow -PassThru

    # ⚠ Waiting on the LINE rather than on a fixed delay. A sleep long enough for
    # the worst case is dead time on every other run and a race on a busier host.
    $waited = 0
    $serving = $false
    while ($waited -lt 30) {
        $text = ''
        if (Test-Path -LiteralPath $log) {
            $text = (Get-Content -LiteralPath $log -Raw -ErrorAction SilentlyContinue)
        }
        if ($null -ne $text -and $text -cmatch '(?m)^serving for ') { $serving = $true; break }
        if ($proc.HasExited) { break }
        Start-Sleep -Seconds 1
        $waited++
    }

    $tracker = ''
    foreach ($line in @(Get-Content -LiteralPath $log -ErrorAction SilentlyContinue)) {
        $parts = $line -split '\s+'
        if ($parts.Count -ge 3 -and $parts[0] -ceq 'endpoint' -and $parts[1] -ceq 'tracker-http') {
            $tracker = $parts[2]
            break
        }
    }
    if ([string]::IsNullOrEmpty($tracker)) {
        if (-not $proc.HasExited) { $proc.Kill() }
        $proc.WaitForExit()
        Deny "the observer never reported a tracker-http endpoint (serving=$serving)"
    }

    # ⭐ A TOKEN THE DRIVER KNOWS IT SENT. BEP 3's `key` is an opaque client
    # value, so the observer keeps it verbatim and the exact bytes are
    # recoverable from the transcript below.
    $infoHash = '%5a%5a%5a%5a%5a%5a%5a%5a%5a%5a%5a%5a%5a%5a%5a%5a%5a%5a%5a%5a'
    $announce = "http://$tracker/announce?info_hash=$infoHash&peer_id=bit-ids-fixture-0001"
    $announce = $announce + '&port=6881&uploaded=0&downloaded=0&left=0&compact=1'
    $announce = $announce + "&event=started&key=$RunId"

    $body = Join-Path $scratch 'body'
    $driverErr = Join-Path $Out 'driver.err'
    # ⚠ --noproxy is not tidiness. A runner with a proxy variable set would send
    # the announce to the proxy, which is off this host, which is the one thing
    # the containment exists to stop.
    $driverStatus = (& $curl.Source -sS --noproxy '*' --max-time 20 -o $body -w '%{http_code}' $announce 2> $driverErr)
    $driverRc = $LASTEXITCODE

    $proc.WaitForExit()
    $observerRc = $proc.ExitCode

    if ($driverRc -ne 0) { Deny "the driver could not reach the observer (curl exit $driverRc)" }
    if ($observerRc -ne 0) { Deny "the observer exited $observerRc" }

    $lines = @(Get-Content -LiteralPath $log)
    $segments = ''
    foreach ($line in $lines) {
        $parts = $line -split '\s+'
        if ($parts.Count -ge 2 -and $parts[0] -ceq 'segments:') { $segments = $parts[1]; break }
    }
    if ([string]::IsNullOrEmpty($segments)) { Deny 'the observer reported no segment count' }
    if ([int]$segments -le 0) { Deny 'the run recorded no bytes at all' }

    # -- the evidence, checked by something else ------------------------------
    #
    # ⭐ THE ROWS ARE THE OBSERVER'S OWN CLAIMS AND Get-FileHash IS WHAT TESTS
    # THEM. The bundle writer already read its files back against the buffer it
    # wrote; that is the writer checking itself.
    $rows = New-Object System.Collections.ArrayList
    $checked = 0
    foreach ($line in $lines) {
        $parts = $line -split '\s+'
        if ($parts.Count -lt 5 -or $parts[0] -cne 'evidence') { continue }
        $relative = $parts[2]
        $declared = ($parts[4] -replace '^sha256:', '').ToLowerInvariant()
        [void]$rows.Add("$declared  bundle/$relative")
        $file = Join-Path $bundle ($relative -replace '/', [System.IO.Path]::DirectorySeparatorChar)
        if (-not (Test-Path -LiteralPath $file -PathType Leaf)) {
            Deny "the observer described $relative and no such file was written"
        }
        $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $file).Hash.ToLowerInvariant()
        if ($actual -cne $declared) {
            Deny "the evidence does not verify against the digests the observer declared: $relative"
        }
        $checked++
    }
    if ($checked -eq 0) { Deny 'the observer described no evidence' }
    Set-Content -LiteralPath $sums -Value $rows

    # ⛔ AND THE TRANSCRIPT MUST CARRY WHAT THE DRIVER SENT. Everything above is
    # satisfied by a bundle of empty artifacts that verify against their own
    # empty digests. This is the only case that says a client's bytes reached
    # the record.
    $tokenBytes = [System.Text.Encoding]::ASCII.GetBytes("key=$RunId")
    $tokenHex = ($tokenBytes | ForEach-Object { $_.ToString('x2') }) -join ''
    $transcript = Join-Path $bundle 'tracker-http.transcript.json'
    $carried = $false
    if (Test-Path -LiteralPath $transcript -PathType Leaf) {
        $text = (Get-Content -LiteralPath $transcript -Raw)
        if ($null -ne $text -and $text.Contains($tokenHex)) { $carried = $true }
    }
    if (-not $carried) { Deny 'the transcript does not carry the bytes the driver sent' }

    $finished = [System.DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')
    $observerDigest = (Get-FileHash -Algorithm SHA256 -LiteralPath $Observer).Hash.ToLowerInvariant()
    $driverVersion = (& $curl.Source --version | Select-Object -First 1)
    $platform = ([System.Runtime.InteropServices.RuntimeInformation]::OSDescription + ' ' +
        [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture)

    # ⚠ Key=value, one per line, the shape assert-disposable's own marker uses.
    # Nothing in Rust parses it. ⛔ THE KEY SET MATCHES THE SH HALF'S EXACTLY;
    # check-capture.sh compares them, because a field in one twin and not the
    # other is the commonest shape of drift and the one a value comparison
    # cannot see.
    $doc = New-Object System.Collections.ArrayList
    [void]$doc.Add('bit-ids/capture-attestation/1')
    [void]$doc.Add("run=$RunId")
    [void]$doc.Add('kind=fixture')
    [void]$doc.Add('measured_build=none')
    [void]$doc.Add('stock_client=false')
    [void]$doc.Add("platform=$platform")
    [void]$doc.Add("started_at=$started")
    [void]$doc.Add("finished_at=$finished")
    [void]$doc.Add("host_fingerprint=$fingerprint")
    [void]$doc.Add("claim_marker=$marker")
    [void]$doc.Add("claimed_run=$claimedRun")
    [void]$doc.Add('egress=closed')
    [void]$doc.Add("egress_source=$egressSource")
    [void]$doc.Add("observer=$Observer")
    [void]$doc.Add("observer_sha256=$observerDigest")
    [void]$doc.Add('driver=curl')
    [void]$doc.Add("driver_version=$driverVersion")
    [void]$doc.Add("driver_status=$driverStatus")
    [void]$doc.Add("segments=$segments")
    [void]$doc.Add("evidence=$checked")
    [void]$doc.Add('evidence_verified_by=Get-FileHash')
    Set-Content -LiteralPath $attest -Value $doc

    # ⛔ READ BACK, NOT ASSUMED.
    $written = @(Get-Content -LiteralPath $attest -ErrorAction SilentlyContinue)
    if ($written -cnotcontains "run=$RunId") { Deny 'the attestation was not written' }

    Write-Output $fingerprint
    Write-Output "capture-run: $segments segment(s), $checked artifact(s), verified by Get-FileHash"
    Write-Output "capture-run: attestation $attest"
    exit 0
}
finally {
    Remove-Item -LiteralPath $scratch -Recurse -Force -ErrorAction SilentlyContinue
}
