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

    $summaryLines = @(Get-Content -LiteralPath 'TODO/SUMMARY.md')
    $summary = $summaryLines | Where-Object { $_ -match '^\| Total \|' } | Select-Object -First 1
    $expectedSummary = "| Total | | $($openRows.Count) | $($inProgressRows.Count) | $($blockedRows.Count) | $($doneRows.Count) | $($rows.Count) |"
    if ($summary -ne $expectedSummary) {
        $failures.Add("TODO/SUMMARY.md total is '$summary', computed '$expectedSummary'")
    }

    # ⛔ THE TOTAL ROW WAS THE ONLY ROW CHECKED, AND IT IS ONE OF TWELVE. The
    # eleven category rows are derived from the same index and nothing compared
    # them, so Observer could read 9 over ten open observer entries and this
    # check exited 0.
    #
    # ⭐ The mapping from a category to the identifiers it counts is declared in
    # TODO/SUMMARY.md rather than here, so this twin and the sh twin read one
    # mapping instead of holding one each. ⛔ Keep this identical to the sh
    # twin: both directions are checked, so a prefix with no row and a row
    # naming nothing are both failures.
    #
    # ⚠ A DATA ROW IS RECOGNISED BY ITS SHAPE, NOT BY ITS CASE. The first
    # version matched '^\| [A-Z]' and this twin let the `category` header
    # through, because PowerShell's -match is case-insensitive and awk's
    # bracket expression is not. The two regexes were character for character
    # identical and answered differently.
    $byPrefix = @{}
    foreach ($pair in $indexPairs) {
        $prefix = $pair.Id -replace '-\d\d$', ''
        if (-not $byPrefix.ContainsKey($prefix)) {
            $byPrefix[$prefix] = @{ OPEN = 0; IN_PROGRESS = 0; BLOCKED = 0; DONE = 0; TOTAL = 0 }
        }
        $byPrefix[$prefix][$pair.Status]++
        $byPrefix[$prefix].TOTAL++
    }
    $summaryBad = [System.Collections.Generic.List[string]]::new()
    $declaredPrefixes = @{}
    foreach ($line in $summaryLines) {
        if ($line -notmatch '^\|') { continue }
        $parts = @($line -split '\|' | ForEach-Object { $_.Trim() })
        if ($parts.Count -lt 9) { continue }
        $prefix = $parts[2] -replace '`', ''
        if (-not $prefix) { continue }
        if (@($parts[3..7] | Where-Object { $_ -notmatch '^\d+$' }).Count -gt 0) { continue }
        if ($declaredPrefixes.ContainsKey($prefix)) {
            $summaryBad.Add("duplicate row for $prefix")
            continue
        }
        $declaredPrefixes[$prefix] = $true
        $counts = if ($byPrefix.ContainsKey($prefix)) { $byPrefix[$prefix] }
        else { @{ OPEN = 0; IN_PROGRESS = 0; BLOCKED = 0; DONE = 0; TOTAL = 0 } }
        if ([int]$parts[3] -ne $counts.OPEN -or
            [int]$parts[4] -ne $counts.IN_PROGRESS -or
            [int]$parts[5] -ne $counts.BLOCKED -or
            [int]$parts[6] -ne $counts.DONE -or
            [int]$parts[7] -ne $counts.TOTAL) {
            $summaryBad.Add($prefix)
        }
    }
    foreach ($prefix in $byPrefix.Keys) {
        if (-not $declaredPrefixes.ContainsKey($prefix)) {
            $summaryBad.Add("$prefix has no row")
        }
    }
    foreach ($prefix in $declaredPrefixes.Keys) {
        if (-not $byPrefix.ContainsKey($prefix)) {
            $summaryBad.Add("$prefix names nothing in the index")
        }
    }
    if ($summaryBad.Count -gt 0) {
        $failures.Add('TODO/SUMMARY.md category rows disagree: ' + (($summaryBad | Sort-Object) -join ' '))
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

    # ⛔ AN ALLOWLIST OF IMMUTABLE FORMS, NOT A DENYLIST OF FLOATING ONES.
    # This used to name the floating refs it knew: main, master and vN.N.N. A
    # branch called anything else, or an abbreviated commit, is just as mutable
    # and passed. ⭐ Inverting it means a form nobody thought of fails closed.
    #
    # ⚠ THE VERSION COMMENT IS PART OF THE PIN. A 40-hex ref alone is
    # unreviewable; check-remote-items.ps1 resolves the comment against the tag
    # it names, so a pin without one is a pin that check never examines.
    # ⛔ Keep this identical to the sh twin.
    #
    # ⛔ THE SCOPE INCLUDES COMPOSITE ACTIONS, NOT WORKFLOWS ALONE. A composite
    # action under .github/actions/ carries its own `uses:` lines and runs with
    # the same permissions, so a rule that read only .github/workflows/ would be
    # a gate on one of two doors into the same operation.
    $pinProblems = [System.Collections.Generic.List[string]]::new()
    $workflows = @()
    if (Test-Path -LiteralPath '.github/workflows') {
        $workflows += @(Get-ChildItem -LiteralPath '.github/workflows' -File |
            Where-Object { $_.Extension -in '.yml', '.yaml' })
    }
    if (Test-Path -LiteralPath '.github/actions') {
        $workflows += @(Get-ChildItem -LiteralPath '.github/actions' -File -Recurse |
            Where-Object { $_.Name -in 'action.yml', 'action.yaml' })
    }
    if ($workflows.Count -gt 0) {
        foreach ($wf in $workflows) {
            $n = 0
            foreach ($line in (Get-Content -LiteralPath $wf.FullName)) {
                $n++
                if ($line -notmatch '^\s*(-\s+)?uses:\s') { continue }
                $ref = $line -replace '^[^:]*uses:\s*', ''
                $comment = ''
                $split = [regex]::Match($ref, '\s+#')
                if ($split.Success) {
                    $comment = $ref.Substring($split.Index)
                    $ref = $ref.Substring(0, $split.Index)
                }
                $ref = ($ref -replace '\s+$', '') -replace '^["'']', '' -replace '["'']$', ''

                # A local action is this repository, reviewed with everything else.
                if ($ref -like './*') { continue }

                $at = $ref.LastIndexOf('@')
                if ($at -lt 0) {
                    $pinProblems.Add("$($wf.Name):$n carries no ref at all: $ref")
                    continue
                }
                $pinned = $ref.Substring($at + 1)

                if ($ref -like 'docker://*') {
                    if (-not $pinned.StartsWith('sha256:')) {
                        $pinProblems.Add("$($wf.Name):$n container is not pinned to a digest: $ref")
                        continue
                    }
                    $digest = $pinned.Substring(7)
                    if ($digest.Length -ne 64 -or $digest -cnotmatch '^[0-9a-f]+$') {
                        $pinProblems.Add("$($wf.Name):$n container digest is not a sha256: $ref")
                    }
                    continue
                }

                if ($pinned.Length -ne 40 -or $pinned -cnotmatch '^[0-9a-f]+$') {
                    $pinProblems.Add("$($wf.Name):$n not pinned to a 40-character commit: $ref")
                    continue
                }
                if ($comment -notmatch '#\s*\S') {
                    $pinProblems.Add("$($wf.Name):$n pin carries no version comment, so nothing can check it: $ref")
                }
            }
        }
    }
    if ($pinProblems.Count -gt 0) {
        $failures.Add('workflow action pin: ' + ($pinProblems -join '; '))
    }

    # ⛔ A DEPENDENCY THIS PROJECT DID NOT REVIEW CANNOT REACH THE OBSERVER OR
    # THE PUBLISHER. Cargo.lock is the inventory: a package with no `source` is
    # a member of this workspace, and every other one must come from the
    # crates.io registry with a checksum. A git or path dependency appears here
    # as some other source, so this one test covers the shape whatever the
    # manifest said.
    $registry = 'registry+https://github.com/rust-lang/crates.io-index'
    $packages = [System.Collections.Generic.List[object]]::new()
    $current = $null
    foreach ($line in (Get-Content -LiteralPath 'Cargo.lock')) {
        if ($line -eq '[[package]]') {
            if ($null -ne $current) { [void]$packages.Add($current) }
            $current = [pscustomobject]@{ Name = ''; Source = ''; Checksum = $false }
            continue
        }
        if ($null -eq $current) { continue }
        if ($line -match '^name = "(.*)"$') { $current.Name = $Matches[1]; continue }
        if ($line -match '^source = "(.*)"$') { $current.Source = $Matches[1]; continue }
        if ($line -match '^checksum = "(.*)"$') { $current.Checksum = $true; continue }
    }
    if ($null -ne $current) { [void]$packages.Add($current) }

    $lockProblems = [System.Collections.Generic.List[string]]::new()
    foreach ($package in $packages) {
        if ($package.Name -eq '' -or $package.Source -eq '') { continue }
        if ($package.Source -ne $registry) {
            $lockProblems.Add("  $($package.Name) is not from the crates.io registry: $($package.Source)")
        } elseif (-not $package.Checksum) {
            $lockProblems.Add("  $($package.Name) has no checksum")
        }
    }
    if ($lockProblems.Count -gt 0) {
        $failures.Add("unreviewed dependency source:`n" + ($lockProblems -join "`n"))
    }

    # ⛔ AN ACCEPTANCE COMMAND MUST NOT BE ABLE TO PASS OVER NOTHING. `cargo
    # test` with a bare word selects by test NAME, and a filter matching none
    # prints `running 0 tests` for every binary and exits 0. `OBS-01`'s Prove did
    # exactly that and was read as an acceptance that passed; measured on
    # 2026-09-05, ten more invocations in TODO/ were of the same shape and had
    # only ever worked because every test function happened to begin with its
    # file's name, which is a convention nothing held. CI-05 is the entry.
    #
    # ⛔ TWO SOURCES, ONE TOKENISER. An entry's `Prove` is the acceptance a
    # person runs and the workflow's `run:` is the one every push runs, and a
    # bare filter in either exits 0 over nothing. A rule on one of two doors into
    # the same mistake is the shape docs/methodology/reviews.md names, so the
    # extractors are separate and the judgement is not.
    #
    # ⚠ SCOPED TO `Prove:` PARAGRAPHS IN TODO, and that is the whole rule rather
    # than an exclusion list. A `Prove` is the live acceptance and must be
    # runnable; a `Closure evidence` paragraph records what was actually run on a
    # past tree and rewriting it would falsify the record, and the two entries
    # that document this defect have to be able to quote the command that caused
    # it. A rule that fired on those would be a rule somebody switches off.
    #
    # ⚠ A code span wraps across lines, so the paragraph is joined before the
    # spans are found.
    $candidates = [System.Collections.Generic.List[object]]::new()
    foreach ($todo in (& git ls-files 'TODO/*.md')) {
        if (-not (Test-Path -LiteralPath $todo -PathType Leaf)) { continue }
        $inProve = $false
        $buffer = ''
        $startLine = 0
        $number = 0
        foreach ($line in (Get-Content -LiteralPath $todo)) {
            $number++
            $closing = $false
            if ($line -match '^[ \t]*$') { $closing = $true }
            elseif ($line -match '^Prove:') { $closing = $true }
            if ($closing -and $inProve -and $buffer -ne '') {
                $parts = $buffer -split '`'
                for ($k = 1; $k -lt $parts.Count; $k += 2) {
                    if ($parts[$k] -match '^cargo[ \t]+test([ \t]|$)') {
                        $candidates.Add([pscustomobject]@{ File = $todo; Line = $startLine; Command = $parts[$k] })
                    }
                }
            }
            if ($closing) { $inProve = $false; $buffer = '' }
            if ($line -match '^Prove:') {
                $inProve = $true
                $startLine = $number
                $buffer = $line
                continue
            }
            if ($inProve) { $buffer = "$buffer $line" }
        }
        if ($inProve -and $buffer -ne '') {
            $parts = $buffer -split '`'
            for ($k = 1; $k -lt $parts.Count; $k += 2) {
                if ($parts[$k] -match '^cargo[ \t]+test([ \t]|$)') {
                    $candidates.Add([pscustomobject]@{ File = $todo; Line = $startLine; Command = $parts[$k] })
                }
            }
        }
    }
    foreach ($workflow in (& git ls-files '.github/workflows/*.yml')) {
        if (-not (Test-Path -LiteralPath $workflow -PathType Leaf)) { continue }
        $number = 0
        foreach ($line in (Get-Content -LiteralPath $workflow)) {
            $number++
            # A commented-out command is not one every push runs.
            if ($line -match '^[ \t]*#') { continue }
            if ($line -notmatch 'cargo[ \t]+test') { continue }
            $command = $line -replace '^.*cargo[ \t]+test', 'cargo test'
            $command = $command -replace '[ \t]*\\[ \t]*$', ''
            $candidates.Add([pscustomobject]@{ File = $workflow; Line = $number; Command = $command })
        }
    }

    $valueFlags = @(
        '-p', '-j', '-F', '--package', '--exclude', '--test', '--bin', '--example',
        '--bench', '--features', '--target', '--target-dir', '--manifest-path',
        '--profile', '--jobs', '--message-format', '--color', '--config',
        '--test-threads', '--skip'
    )
    $proveProblems = [System.Collections.Generic.List[string]]::new()
    foreach ($candidate in $candidates) {
        $tokens = @($candidate.Command -split '[ \t]+' | Where-Object { $_ -ne '' })
        $expect = $false
        for ($i = 2; $i -lt $tokens.Count; $i++) {
            $token = $tokens[$i]
            if ($token.StartsWith('-')) {
                # A flag that takes a value consumes the next bare word, which is
                # then a target or a package rather than a name filter.
                $expect = (-not $token.Contains('=')) -and ($valueFlags -ccontains $token)
                continue
            }
            if ($expect) { $expect = $false; continue }
            $proveProblems.Add("  $($candidate.File):$($candidate.Line) selects tests by name, so it exits 0 over nothing: $($candidate.Command)")
            break
        }
    }
    if ($proveProblems.Count -gt 0) {
        $failures.Add("an acceptance that can pass over nothing:`n" + ($proveProblems -join "`n"))
    }

    # ⚠ The lockfile test above is the authority; this one fires earlier and
    # names the manifest line, so the report points at the file somebody edited
    # rather than at the file cargo generated.
    $manifests = @(
        & git ls-files '*Cargo.toml'
        & git ls-files --others --exclude-standard '*Cargo.toml'
    ) | Sort-Object -Unique
    $gitDeps = [System.Collections.Generic.List[string]]::new()
    foreach ($manifest in $manifests) {
        if (-not (Test-Path -LiteralPath $manifest -PathType Leaf)) { continue }
        $k = 0
        foreach ($line in (Get-Content -LiteralPath $manifest)) {
            $k++
            if ($line -match '(^|[{,]\s*)git\s*=') { $gitDeps.Add("${manifest}:${k}:$line") }
        }
    }
    if ($gitDeps.Count -gt 0) {
        $failures.Add('git dependency in a manifest: ' + ($gitDeps -join '; '))
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
