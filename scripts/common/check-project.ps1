# Validate bit-ids-specific repository invariants.
[CmdletBinding(PositionalBinding = $false)]
param([switch]$Json)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = (& git -C (Split-Path -Parent $PSCommandPath) rev-parse --show-toplevel 2>$null)
if ($LASTEXITCODE -ne 0 -or -not $root) {
    Write-Error 'check-project: not in a git repository'
    exit 2
}

Push-Location $root
try {
    $failures = [System.Collections.Generic.List[string]]::new()
    $required = @(
        'README.md', 'LICENSE', 'Cargo.toml', 'Cargo.lock',
        'catalogue/clients.toml', 'TODO/INDEX.md', 'TODO/PROGRESS.md',
        'TODO/SUMMARY.md', 'docs/AGENTS.md',
        'docs/reference-sweeps/bit-cli.md', '.github/workflows/ci.yml'
    )
    foreach ($path in $required) {
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            $failures.Add("missing $path")
        }
    }

    $catalogue = if (Test-Path -LiteralPath 'catalogue/clients.toml') {
        Get-Content -Raw -LiteralPath 'catalogue/clients.toml'
    } else { '' }
    $ids = @(
        'qbittorrent', 'qbittorrent-enhanced', 'utorrent', 'bitcomet', 'aria2',
        'transmission', 'deluge', 'bittorrent', 'biglybt', 'tixati', 'ktorrent',
        'fdm', 'zona', 'libtorrent', 'anacrolix-torrent', 'rqbit'
    )
    foreach ($id in $ids) {
        if ($catalogue -notmatch ('(?m)^id = "' + [regex]::Escape($id) + '"$')) {
            $failures.Add("missing target $id")
        }
        $matrixNeedle = '| ' + [char]96 + $id + [char]96 + ' |'
        if (-not (Select-String -LiteralPath 'docs/client-matrix.md' -SimpleMatch -Pattern $matrixNeedle -Quiet)) {
            $failures.Add("client matrix lacks target $id")
        }
    }

    $index = if (Test-Path -LiteralPath 'TODO/INDEX.md') {
        Get-Content -LiteralPath 'TODO/INDEX.md'
    } else { @() }
    $rows = @($index | Where-Object {
        $_ -match '^\| (FOUND|SCHEMA|OBS|ACQ|CLIENT|ENGINE|CORPUS|LIB|PUB|CI|DOC)-\d\d '
    })
    $indexPairs = @($rows | ForEach-Object {
        $parts = $_ -split '\|'
        [pscustomobject]@{
            Id = $parts[1].Trim()
            Priority = $parts[2].Trim()
            Status = $parts[4].Trim()
        }
    })
    $openRows = @($indexPairs | Where-Object Status -eq 'OPEN')
    $inProgressRows = @($indexPairs | Where-Object Status -eq 'IN_PROGRESS')
    $blockedRows = @($indexPairs | Where-Object Status -eq 'BLOCKED')
    $doneRows = @($indexPairs | Where-Object Status -eq 'DONE')
    $invalidStatuses = @($indexPairs | Where-Object Status -notin @('OPEN', 'IN_PROGRESS', 'BLOCKED', 'DONE'))
    if ($invalidStatuses.Count -gt 0) {
        $failures.Add('TODO index contains invalid statuses')
    }

    $duplicates = @($indexPairs | Group-Object Id | Where-Object Count -gt 1)
    if ($duplicates.Count -gt 0) {
        $failures.Add('TODO index contains duplicate IDs')
    }

    $bodyPairs = [System.Collections.Generic.List[string]]::new()
    foreach ($file in Get-ChildItem -LiteralPath 'TODO' -Filter '*.md' -File) {
        $currentId = ''
        foreach ($line in Get-Content -LiteralPath $file.FullName) {
            if ($line -match '^## (?<id>(FOUND|SCHEMA|OBS|ACQ|CLIENT|ENGINE|CORPUS|LIB|PUB|CI|DOC)-\d\d):') {
                $currentId = $Matches.id
            } elseif ($currentId -and $line -match '^Priority: .* \| Status: (?<status>[A-Z_]+)$') {
                $bodyPairs.Add("$currentId|$($Matches.status)")
                $currentId = ''
            }
        }
    }
    $indexPairText = @($indexPairs | ForEach-Object { "$($_.Id)|$($_.Status)" } | Sort-Object)
    $bodyPairText = @($bodyPairs | Sort-Object)
    if ($bodyPairText.Count -ne $indexPairText.Count -or
        @(Compare-Object $indexPairText $bodyPairText).Count -ne 0) {
        $failures.Add('TODO IDs or statuses disagree between index and category bodies')
    }

    function Get-DeclaredCount([string]$Path, [string]$Key) {
        $match = Get-Content -LiteralPath $Path |
            Where-Object { $_ -match ('^' + [regex]::Escape($Key) + ': (?<count>\d+)$') } |
            Select-Object -First 1
        if ($match -and $match -match ':\s+(?<count>\d+)$') {
            return [int]$Matches.count
        }
        return -1
    }
    $computed = [ordered]@{
        'Total' = $rows.Count
        'Open' = $openRows.Count
        'In progress' = $inProgressRows.Count
        'Blocked' = $blockedRows.Count
        'Done' = $doneRows.Count
    }
    foreach ($path in @('TODO/INDEX.md', 'TODO/PROGRESS.md')) {
        foreach ($item in $computed.GetEnumerator()) {
            $declared = Get-DeclaredCount $path $item.Key
            if ($declared -ne $item.Value) {
                $failures.Add("$path declares $($item.Key)=$declared, computed $($item.Value)")
            }
        }
    }

    $summary = Get-Content -LiteralPath 'TODO/SUMMARY.md' |
        Where-Object { $_ -match '^\| Total \|' } | Select-Object -First 1
    $expectedSummary = "| Total | $($openRows.Count) | $($inProgressRows.Count) | $($blockedRows.Count) | $($doneRows.Count) | $($rows.Count) |"
    if ($summary -ne $expectedSummary) {
        $failures.Add("TODO/SUMMARY.md total is '$summary', computed '$expectedSummary'")
    }

    foreach ($priority in @('P0', 'P1', 'P2')) {
        $priorityRow = $index | Where-Object { $_ -match "^\| $priority \|" } |
            Select-Object -First 1
        $parts = @($priorityRow -split '\|' | ForEach-Object Trim)
        $items = @($indexPairs | Where-Object Priority -eq $priority)
        $expected = @(
            @($items | Where-Object Status -eq 'OPEN').Count,
            @($items | Where-Object Status -eq 'IN_PROGRESS').Count,
            @($items | Where-Object Status -eq 'BLOCKED').Count,
            @($items | Where-Object Status -eq 'DONE').Count,
            $items.Count
        )
        $declared = if ($parts.Count -ge 8) {
            @([int]$parts[2], [int]$parts[3], [int]$parts[4], [int]$parts[5], [int]$parts[6])
        } else { @(-1, -1, -1, -1, -1) }
        if (@(Compare-Object $expected $declared -SyncWindow 0).Count -ne 0) {
            $failures.Add("TODO priority table disagrees for $priority")
        }
    }

    $python = @(& git ls-files '*.py'; & git ls-files --others --exclude-standard '*.py')
    if ($python.Count -gt 0) {
        $failures.Add('Python exists without an approved exception: ' + ($python -join ', '))
    }

    $floating = @()
    if (Test-Path -LiteralPath '.github/workflows') {
        $floating = @(Get-ChildItem -LiteralPath '.github/workflows' -File |
            Select-String -Pattern 'uses: +[^ ]+@(main|master|v[0-9]+([.]([0-9]+))*)(?:\s+#.*)?\s*$')
    }
    if ($floating.Count -gt 0) {
        $failures.Add('workflow action is not pinned to an immutable commit')
    }

    if ($Json) {
        [ordered]@{
            schema = 'check-project/2'
            failures = $failures.Count
            todo_entries = $rows.Count
            open = $openRows.Count
            in_progress = $inProgressRows.Count
            blocked = $blockedRows.Count
            done = $doneRows.Count
        } | ConvertTo-Json -Compress
    } elseif ($failures.Count -eq 0) {
        Write-Output "bit-ids project invariants pass ($($rows.Count) entries; $($openRows.Count) open; $($inProgressRows.Count) in progress; $($blockedRows.Count) blocked; $($doneRows.Count) done)"
    } else {
        $failures | ForEach-Object { Write-Output "FAIL: $_" }
    }
    if ($failures.Count -gt 0) { exit 1 }
    exit 0
} finally {
    Pop-Location
}
