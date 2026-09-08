# Validate bit-ids-specific repository invariants.
[CmdletBinding(PositionalBinding = $false)]
param([switch]$Json)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
# ⛔ A NATIVE COMMAND'S EXIT CODE IS THE VERDICT HERE, SO IT MUST NOT THROW.
# This defaults to $true from PowerShell 7.5, which turns every non-zero exit
# into a terminating error under $ErrorActionPreference = 'Stop': a guard that
# REFUSES stops being a code a caller can read and becomes an exception nobody
# caught. docs/conventions/shell.md section 8.
$PSNativeCommandUseErrorActionPreference = $false
$root = (& git -C (Split-Path -Parent $PSCommandPath) rev-parse --show-toplevel 2>$null)
if ($LASTEXITCODE -ne 0 -or -not $root) {
    [Console]::Error.WriteLine('check-project: not in a git repository')
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
    # ⛔ EVERY FIELD THE TWO PLACES BOTH CARRY IS COMPARED, NOT THE STATUS ALONE.
    # ⚠ Measured on 2026-09-08: FOUND-05 was P2 in the index and P1 in its own
    # entry, and had been since the commit that created both - a value in two
    # places that nothing had ever compared. The index's priority table is
    # derived from the index's own rows, so it agreed with the wrong half.
    # ⛔ Keep this identical to the sh twin.
    $indexPairs = @($rows | ForEach-Object {
        $parts = $_ -split '\|'
        [pscustomobject]@{
            Id       = $parts[1].Trim()
            Priority = $parts[2].Trim()
            Effort   = $parts[3].Trim()
            Status   = $parts[4].Trim()
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
            } elseif ($currentId -and $line -match '^Priority: (?<priority>[^|]+) \| Effort: (?<effort>[^|]+) \| Status: (?<status>[A-Z_]+)$') {
                $bodyPairs.Add("$currentId|$($Matches.priority.Trim())|$($Matches.effort.Trim())|$($Matches.status)")
                $currentId = ''
            }
        }
    }
    $indexPairText = @($indexPairs | ForEach-Object { "$($_.Id)|$($_.Priority)|$($_.Effort)|$($_.Status)" } | Sort-Object)
    $bodyPairText = @($bodyPairs | Sort-Object)
    if ($bodyPairText.Count -ne $indexPairText.Count -or
        @(Compare-Object $indexPairText $bodyPairText).Count -ne 0) {
        $failures.Add('TODO IDs, priorities, efforts or statuses disagree between index and category bodies')
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

    # ⛔ AN ARTIFACT A WORKFLOW DOWNLOADS IS ONE SOME WORKFLOW UPLOADS. Two
    # workflows joined by a name nobody compares are not joined at all, and the
    # failure is a dispatch that dies on its first step.
    # ⚠ Measured in this tree on 2026-09-08: publish-data.yml downloads `bundle`
    # and nothing here produces that name.
    # ⭐ Names are templated, so every `${{ ... }}` becomes `*` on both sides and
    # the download's shape is matched against each upload's. A download with no
    # `name:` takes every artifact and names nothing to check.
    # ⭐ A missing producer may be DECLARED with `bit-ids:no-producer=<ENTRY>`
    # inside the step; the entry must be one TODO/INDEX.md carries, and a
    # declaration over a name that has gained a producer is refused.
    # ⛔ Keep this identical to the sh twin, scope included.
    $artRows = [System.Collections.Generic.List[psobject]]::new()
    foreach ($wf in $workflows) {
        $n = 0
        $stepIndent = -1
        $withIndent = -1
        $kind = ''
        $val = ''
        $marker = '-'
        $at = 0
        $newStep = -1
        foreach ($line in (Get-Content -LiteralPath $wf.FullName)) {
            $n++
            $indent = ($line -replace '^([ \t]*).*$', '$1').Length
            $body = $line -replace '^[ \t]*', ''
            # ⚠ The prefix is stripped by the literal rather than by its length.
            # The sh twin counted the characters first and was one out.
            $declared = [regex]::Match($body, 'bit-ids:no-producer=[A-Z]+-[0-9]+')
            if ($declared.Success) {
                $marker = $declared.Value -replace '^bit-ids:no-producer=', ''
                continue
            }
            if ($body -eq '' -or $body.StartsWith('#')) { continue }
            $flush = $false
            if ($body -match '^-[ \t]' -or $body -eq '-') {
                if ($stepIndent -lt 0 -or $indent -le $stepIndent) { $flush = $true; $newStep = $indent }
            } elseif ($stepIndent -ge 0 -and $indent -le $stepIndent) {
                $flush = $true; $newStep = -1
            }
            if ($flush) {
                if ($kind -ne '' -and $val -ne '') {
                    $artRows.Add([pscustomobject]@{
                            Kind    = $kind
                            Where   = "$($wf.Name):$at"
                            Name    = $val
                            Pattern = [regex]::Replace($val, '\$\{\{[^}]*\}\}', '*')
                            Marker  = $marker
                        })
                }
                $kind = ''; $val = ''; $marker = '-'; $at = 0; $withIndent = -1
                $stepIndent = $newStep
            }
            if ($body -match 'uses:[ \t]*actions/(upload|download)-artifact@') {
                $kind = if ($body -clike '*upload-artifact*') { 'upload' } else { 'download' }
                $at = $n
            }
            if ($withIndent -ge 0 -and $indent -le $withIndent) { $withIndent = -1 }
            if ($body -match '^(-[ \t]+)?with:[ \t]*$') {
                # THE COLUMN THAT MATTERS IS THE with: KEY, NOT THE LINE. In
                # `- with:` the dash is part of the indent, so the key sits two
                # columns further right than the line begins and the other keys
                # of that step sit at the SAME column as the key.
                $withIndent = $indent
                if ($body -match '^-[ \t]') {
                    $withIndent = ([regex]::Match($line, '^[ \t]*-[ \t]+')).Length
                }
                continue
            }
            if ($withIndent -ge 0 -and $indent -gt $withIndent -and $body -match '^(name|pattern):[ \t]') {
                $val = ($body -replace '^(name|pattern):[ \t]*', '') -replace '[ \t]+$', ''
                $val = $val -replace '^["'']', '' -replace '["'']$', ''
                if ($at -eq 0) { $at = $n }
            }
        }
        if ($kind -ne '' -and $val -ne '') {
            $artRows.Add([pscustomobject]@{
                    Kind    = $kind
                    Where   = "$($wf.Name):$at"
                    Name    = $val
                    Pattern = [regex]::Replace($val, '\$\{\{[^}]*\}\}', '*')
                    Marker  = $marker
                })
        }
    }
    $artUploads = @($artRows | Where-Object { $_.Kind -eq 'upload' } | ForEach-Object { $_.Pattern })
    $indexLines = @()
    if (Test-Path -LiteralPath 'TODO/INDEX.md') {
        $indexLines = @(Get-Content -LiteralPath 'TODO/INDEX.md')
    }
    $artProblems = [System.Collections.Generic.List[string]]::new()
    foreach ($row in @($artRows | Where-Object { $_.Kind -eq 'download' })) {
        $found = $false
        foreach ($up in $artUploads) {
            # ⚠ -clike, not -like. PowerShell's wildcard match is
            # case-INSENSITIVE by default and the sh twin's `case` is not.
            if ($row.Pattern -clike $up) { $found = $true; break }
        }
        if ($found) {
            if ($row.Marker -ne '-') {
                $artProblems.Add("$($row.Where) declares no-producer=$($row.Marker) over [$($row.Name)], which an upload-artifact in this tree now produces")
            }
            continue
        }
        if ($row.Marker -eq '-') {
            $artProblems.Add("$($row.Where) downloads [$($row.Name)], which no upload-artifact in this tree produces")
            continue
        }
        $known = @($indexLines | Where-Object { $_.StartsWith("| $($row.Marker) |") })
        if ($known.Count -eq 0) {
            $artProblems.Add("$($row.Where) declares no-producer=$($row.Marker), which is not an entry in TODO/INDEX.md")
        }
    }
    if ($artProblems.Count -gt 0) {
        $failures.Add('workflow artifact name: ' + ($artProblems -join '; '))
    }

    # ⛔ EVERY TRACKED FILE'S WORKING-TREE ENDINGS AGREE WITH WHAT .gitattributes
    # RESOLVES FOR IT. conventions/shell.md section 5 described this check for as
    # long as it described the problem, and until 2026-09-08 it did not exist.
    # ⚠ Found by being broken: a tool that emits LF left check-project.ps1 at
    # w/lf under attr/text eol=crlf, the gate was green, and only an incidental
    # warning from git commit said so. A missing or extra carriage return is
    # invisible to git diff because the index is normalised either way, so the
    # working tree is the only place it can be read.
    # ⭐ Git's own answer per path, not a second table. ⚠ w/none carries no
    # evidence either way and is not a disagreement.
    # ⛔ Keep this identical to the sh twin.
    $eolProblems = [System.Collections.Generic.List[string]]::new()
    $eolLines = @(& git ls-files --eol)
    foreach ($line in $eolLines) {
        $tab = $line.IndexOf("`t")
        if ($tab -lt 0) { continue }
        $head = $line.Substring(0, $tab)
        $path = $line.Substring($tab + 1)
        $attrMatch = [regex]::Match($head, 'eol=[a-z]+')
        if (-not $attrMatch.Success) { continue }
        $attr = $attrMatch.Value -replace '^eol=', ''
        $fields = $head -split ' +'
        $w = $fields[1] -replace '^w/', ''
        if ($w -eq 'none') { continue }
        if ($w -cne $attr) {
            $eolProblems.Add("$path is w/$w under eol=$attr")
        }
    }
    if ($eolProblems.Count -gt 0) {
        $failures.Add('line endings disagree with .gitattributes: ' + ($eolProblems -join '; '))
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

    # ⛔ A .ps1 CARRYING NON-ASCII NEEDS A UTF-8 BOM. Windows PowerShell 5.1
    # decodes a BOM-less file as the system ANSI code page, so every marker in it
    # is mis-decoded; docs/conventions/shell.md section 8 is the rule.
    #
    # ⚠ Eleven files had the BOM and four did not, which is a rule enforced on
    # most of the paths into one mistake. ⭐ The test is on the BYTES: an
    # ASCII-only file needs nothing and is not asked for a BOM it has no use for.
    $ps1Files = @(
        & git ls-files '*.ps1'
        & git ls-files --others --exclude-standard '*.ps1'
    ) | Sort-Object -Unique
    $bomless = [System.Collections.Generic.List[string]]::new()
    foreach ($ps1 in $ps1Files) {
        if (-not (Test-Path -LiteralPath $ps1 -PathType Leaf)) { continue }
        $bytes = [System.IO.File]::ReadAllBytes($ps1)
        $hasNonAscii = $false
        foreach ($b in $bytes) { if ($b -gt 126 -or ($b -lt 32 -and $b -ne 9 -and $b -ne 10 -and $b -ne 13)) { $hasNonAscii = $true; break } }
        if (-not $hasNonAscii) { continue }
        if ($bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF) { continue }
        $bomless.Add($ps1)
    }
    if ($bomless.Count -gt 0) {
        $failures.Add('a .ps1 with non-ASCII and no UTF-8 BOM is mis-decoded by PowerShell 5.1: ' +
            ($bomless -join ' '))
    }

    # ⛔ A .ps1 THAT STOPS ON ERRORS MUST ALSO SAY WHAT A NATIVE COMMAND'S EXIT
    # CODE MEANS. $PSNativeCommandUseErrorActionPreference is $false in 7.4 and
    # $true from 7.5, where a non-zero exit becomes a terminating error under
    # $ErrorActionPreference = 'Stop': a guard that REFUSES stops being a code the
    # caller can read. ⛔ Found by CI, not by a reading - sixteen files relied on
    # the 7.4 default. The rule takes no judgement: a file that sets the one
    # preference sets the other.
    $nativePref = [System.Collections.Generic.List[string]]::new()
    foreach ($ps1 in $ps1Files) {
        if (-not (Test-Path -LiteralPath $ps1 -PathType Leaf)) { continue }
        $text = (Get-Content -LiteralPath $ps1 -Raw)
        if ($null -eq $text) { continue }
        if (-not $text.Contains("ErrorActionPreference = 'Stop'")) { continue }
        if ($text.Contains('PSNativeCommandUseErrorActionPreference')) { continue }
        $nativePref.Add($ps1)
    }
    if ($nativePref.Count -gt 0) {
        $failures.Add('a .ps1 stops on errors without saying what a native exit code means: ' +
            ($nativePref -join ' '))
    }

    # ⛔ Write-Error RENDERS, AND A HARNESS MATCHES ON THE STRING. Called inside
    # a script it emits a source-context block and WRAPS the message to the
    # host's width: one refusal is a single line at width 200 and two at width
    # 80, so a fixed-string match succeeds on a developer host and fails on a CI
    # runner. [Console]::Error.WriteLine writes the bytes.
    #
    # ⚠ THE NEEDLE IS AN INVOCATION, NOT THE WORD. Its first run fired on THIS
    # FILE, because the failure message it raises contains the name; a rule has
    # to be describable in the file that enforces it.
    $writeErr = [System.Collections.Generic.List[string]]::new()
    foreach ($ps1 in $ps1Files) {
        if (-not (Test-Path -LiteralPath $ps1 -PathType Leaf)) { continue }
        foreach ($line in (Get-Content -LiteralPath $ps1)) {
            if ($line -match '^\s*#') { continue }
            if ($line -notmatch '(^|[|;{])\s*Write-Error(\s|$)') { continue }
            $writeErr.Add($ps1)
            break
        }
    }
    if ($writeErr.Count -gt 0) {
        $failures.Add('a .ps1 reports through Write-Error, whose rendering wraps by host width: ' +
            ($writeErr -join ' '))
    }

    # -- THE SAME LANGUAGE BEHIND A SECOND DOOR ------------------------------
    #
    # The three rules above iterate git ls-files '*.ps1', so none of them ever
    # reached a pwsh block inside a workflow. Those blocks are the same language
    # with the same two hazards, and capture.yml carried a live instance of
    # each. A rule on one of several paths into the same mistake is the
    # one-gated-door defect docs/methodology/reviews.md names.
    #
    # AND A THIRD HAZARD THAT ONLY EXISTS HERE, found by CI-06's first dispatch:
    # GitHub's pwsh wrapper reads the block's residual $LASTEXITCODE as the
    # step's verdict, so an INVERTED guard - one whose refusal is the outcome
    # the step wants - fails the step by succeeding at its job. A block that
    # reads $LASTEXITCODE ends in an explicit exit, so the status is a decision.
    #
    # The stream is one line per block, '##BLOCK <workflow>:<step>' followed by
    # the body with one space prefixed, and check-project.sh emits it
    # identically.
    $emitBlock = {
        param($state, $workflow, $sink)
        if ($state.Step -ne '' -and $state.IsPwsh -and $state.Body.Count -gt 0) {
            [void]$sink.Add('##BLOCK ' + $workflow + ':' + $state.Step)
            foreach ($b in $state.Body) { [void]$sink.Add(' ' + $b) }
        }
        $state.Step = ''
        $state.IsPwsh = $false
        $state.Body.Clear()
        $state.InRun = $false
        $state.RunInd = -1
    }

    $stream = [System.Collections.Generic.List[string]]::new()
    # THE SCOPE IS THE PIN RULE'S, FOR THE PIN RULE'S REASON. A composite action
    # under .github/actions/ carries its own steps and the same permissions, and
    # 'shell: powershell' is Windows PowerShell 5.1 rather than a different
    # language. A rule matching only pwsh in only workflows/ would be a gate on
    # one of several doors into the same mistake. There is no composite action
    # here yet and no 'shell: powershell' line, which is when a scope is easiest
    # to get wrong. A glob rather than git ls-files, so an uncommitted new
    # workflow is read without asking for it separately.
    $workflowFiles = @(
        Get-ChildItem -Path '.github/workflows' -Filter '*.yml' -File -ErrorAction SilentlyContinue
        Get-ChildItem -Path '.github/workflows' -Filter '*.yaml' -File -ErrorAction SilentlyContinue
        Get-ChildItem -Path '.github/actions' -Filter 'action.yml' -File -Recurse -Depth 1 -ErrorAction SilentlyContinue
        Get-ChildItem -Path '.github/actions' -Filter 'action.yaml' -File -Recurse -Depth 1 -ErrorAction SilentlyContinue
    ) | ForEach-Object {
        # ⚠ The label has to be the same string the sh half prints, so the path
        # is made repository-relative with forward slashes rather than left as
        # whatever this platform's provider returned.
        #
        # ⛔ THE ROOT IS STRIPPED RATHER THAN A PREFIX ASSUMED.
        # `Resolve-Path -Relative` prepends `./` to most paths and NOT to one
        # that already starts with a dot, so a fixed Substring(2) ate the `.g`
        # of `.github` and every file then failed its Test-Path. ⚠ Both halves
        # agreed perfectly on the clean tree while that was true, because a file
        # set only shows in the output when something in it fails; the per-plant
        # comparison is what caught it.
        $full = $_.FullName
        if ($full.StartsWith($root)) { $full = $full.Substring($root.Length) }
        $full.TrimStart([char]'/', [char]'\') -replace '\\', '/'
    } | Sort-Object -Unique
    foreach ($wf in $workflowFiles) {
        if (-not (Test-Path -LiteralPath $wf -PathType Leaf)) { continue }
        $state = [pscustomobject]@{
            Step = ''
            IsPwsh = $false
            Body = [System.Collections.Generic.List[string]]::new()
            InRun = $false
            RunInd = -1
        }
        $keyInd = -1
        foreach ($rawLine in [System.IO.File]::ReadAllLines($wf)) {
            $line = $rawLine -replace "`r$", ''
            $hit = [regex]::Match($line, '[^ ]')
            $ind = if ($hit.Success) { $hit.Index } else { -1 }
            if ($state.InRun) {
                if ($ind -lt 0) { [void]$state.Body.Add(''); continue }
                if ($ind -ge $state.RunInd) { [void]$state.Body.Add($line.Substring($state.RunInd)); continue }
                $state.InRun = $false
            }
            if ($ind -lt 0) { continue }
            if ($line -match '^ *- name:') {
                & $emitBlock $state $wf $stream
                $step = $line -replace '^ *- name:[ \t]*', ''
                $step = $step -replace '^["'']', ''
                $step = $step -replace '["'']$', ''
                $state.Step = $step
                $keyInd = $ind + 2
                continue
            }
            if ($state.Step -eq '') { continue }
            if ($ind -lt $keyInd) { & $emitBlock $state $wf $stream; continue }
            if ($ind -ne $keyInd) { continue }
            if ($line -match '^ *shell:[ \t]*(pwsh|powershell)[ \t]*$') { $state.IsPwsh = $true; continue }
            if ($line -match '^ *run:') {
                $value = $line -replace '^ *run:[ \t]*', ''
                if ($value -in @('|', '>', '|-', '>-')) {
                    $state.InRun = $true
                    $state.RunInd = $keyInd + 2
                    continue
                }
                [void]$state.Body.Add($value)
            }
        }
        & $emitBlock $state $wf $stream
    }

    $wfNative = [System.Collections.Generic.List[string]]::new()
    $wfWriteErr = [System.Collections.Generic.List[string]]::new()
    $wfResidue = [System.Collections.Generic.List[string]]::new()
    $current = ''
    $hasStop = $false
    $hasNative = $false
    $hasCode = $false
    $lastLine = ''
    $closeBlock = {
        if ($current -eq '') { return }
        if ($hasStop -and -not $hasNative) { $wfNative.Add('[' + $current + ']') }
        if ($hasCode -and $lastLine -notmatch '^exit(\s|$)') { $wfResidue.Add('[' + $current + ']') }
    }
    foreach ($streamLine in $stream) {
        if ($streamLine.StartsWith('##BLOCK ')) {
            & $closeBlock
            $current = $streamLine.Substring(8)
            $hasStop = $false
            $hasNative = $false
            $hasCode = $false
            $lastLine = ''
            continue
        }
        $bodyLine = if ($streamLine.StartsWith(' ')) { $streamLine.Substring(1) } else { $streamLine }
        $bare = $bodyLine -replace '^\s+', ''
        if ($bare.StartsWith('#')) { continue }
        if ($bare -ne '') { $lastLine = $bare }
        if ($bodyLine.Contains("ErrorActionPreference = 'Stop'")) { $hasStop = $true }
        if ($bodyLine.Contains('PSNativeCommandUseErrorActionPreference')) { $hasNative = $true }
        if ($bodyLine.Contains('$LASTEXITCODE')) { $hasCode = $true }
        if ($bare -match '(^|[|;{])\s*Write-Error(\s|$)') {
            $label = '[' + $current + ']'
            if (-not $wfWriteErr.Contains($label)) { $wfWriteErr.Add($label) }
        }
    }
    & $closeBlock

    if ($wfNative.Count -gt 0) {
        $failures.Add('a workflow pwsh block stops on errors without saying what a native exit code means: ' +
            ($wfNative -join ' '))
    }
    if ($wfWriteErr.Count -gt 0) {
        $failures.Add('a workflow pwsh block reports through Write-Error, whose rendering wraps by host width: ' +
            ($wfWriteErr -join ' '))
    }
    if ($wfResidue.Count -gt 0) {
        $failures.Add('a workflow pwsh block reads $LASTEXITCODE and lets it fall through as the step''s verdict: ' +
            ($wfResidue -join ' '))
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
