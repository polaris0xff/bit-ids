# Does every target and every dependency have a licence disposition, and does
# this repository carry an artifact it may not redistribute?
#
# ⭐ THE TWIN OF check-licences.sh. `FOUND-04` owns the register in
# catalogue/licences.toml, and the defect both halves exist to catch is a target
# or a package acquiring a licence position by nobody's decision: the row is
# absent, the field is empty, or the row has drifted from the catalogue and the
# lockfile it describes.
#
# ⛔ unverified IS A DISPOSITION AND NOT A GAP. Six of the nine GitHub-hosted
# targets answer NOASSERTION when their licence endpoint is asked, so a detector
# could not name one. Writing an identifier there anyway would invent the kind
# of fact this project is most careful about; the redistribution rule is refused
# regardless, so the register never reads as permission.
#
# ⛔ BOTH DIRECTIONS, ALWAYS. A row nothing names and a name nothing has a row
# for are different defects and each is silent on its own.
#
# -Permitted prints the target ids whose row permits keeping the bytes, and
# nothing else, so a caller can ask this file's meaning rather than parse it.
[CmdletBinding(PositionalBinding = $false)]
param([switch]$Json, [switch]$Permitted)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = (& git -C (Split-Path -Parent $PSCommandPath) rev-parse --show-toplevel 2>$null)
if ($LASTEXITCODE -ne 0 -or -not $root) {
    Write-Error 'check-licences: not in a git repository'
    exit 2
}

Push-Location $root
try {
    $register = 'catalogue/licences.toml'
    $catalogue = 'catalogue/clients.toml'
    $lock = 'Cargo.lock'
    foreach ($path in @($register, $catalogue, $lock)) {
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            Write-Error ("check-licences: {0} is missing" -f $path)
            exit 2
        }
    }

    $failures = [System.Collections.Generic.List[string]]::new()

    # ⚠ A block is closed by the NEXT header or by the end of file, never by its
    # own last key, so a register whose final block is the only one of its kind
    # still produces a row.
    $targets = [System.Collections.Generic.List[hashtable]]::new()
    $deps = [System.Collections.Generic.List[hashtable]]::new()
    $section = ''
    $row = @{}
    function Close-Block {
        if ($script:section -eq 'target' -and $script:row.ContainsKey('id')) {
            [void]$script:targets.Add($script:row)
        }
        elseif ($script:section -eq 'dep' -and $script:row.ContainsKey('id')) {
            [void]$script:deps.Add($script:row)
        }
        $script:row = @{}
    }
    foreach ($line in (Get-Content -LiteralPath $register)) {
        $text = $line -replace "`r$", ''
        if ($text -eq '[[targets]]') { Close-Block; $section = 'target'; continue }
        if ($text -eq '[[dependencies]]') { Close-Block; $section = 'dep'; continue }
        if ($text -cmatch '^(id|name) = "(.*)"$') { $row['id'] = $Matches[2]; continue }
        if ($text -cmatch '^version = "(.*)"$') { $row['version'] = $Matches[1]; continue }
        if ($text -cmatch '^licence = "(.*)"$') { $row['licence'] = $Matches[1]; continue }
        if ($text -cmatch '^licence_source = "(.*)"$') { $row['source'] = $Matches[1]; continue }
        if ($text -cmatch '^redistribute = "(.*)"$') { $row['redistribute'] = $Matches[1]; continue }
        if ($text -cmatch '^notice = "(.*)"$') { $row['notice'] = $Matches[1]; continue }
    }
    Close-Block

    # ⭐ THE REGISTER HAS ONE PARSER PER HALF AND THIS IS IT. ACQ-05's cache has
    # to know which targets may have their bytes kept; a second reader of this
    # file would be a second answer to what it permits. ⚠ Reported before any
    # rule runs, because a caller asking what is permitted is asking about the
    # file as written rather than about whether it is coherent.
    if ($Permitted) {
        foreach ($entry in ($targets | Sort-Object { $_['id'] })) {
            if ($entry.ContainsKey('redistribute') -and $entry['redistribute'] -eq 'permitted') {
                Write-Output $entry['id']
            }
        }
        exit 0
    }

    $registerText = Get-Content -Raw -LiteralPath $register
    if ($registerText -notmatch '(?m)^schema = "bit-ids/licences/1"$') {
        $failures.Add("$register does not declare the licences schema")
    }

    function Get-Field([hashtable]$Row, [string]$Key) {
        if ($Row.ContainsKey($Key)) { return [string]$Row[$Key] }
        return ''
    }

    # -- 1. every catalogue target has exactly one row, and the reverse --------
    $catalogueIds = @(
        Select-String -LiteralPath $catalogue -Pattern '^id = "(.*)"$' |
            ForEach-Object { $_.Matches[0].Groups[1].Value }
    ) | Sort-Object
    $registerIds = @($targets | ForEach-Object { Get-Field $_ 'id' }) | Sort-Object

    $onlyCatalogue = @($catalogueIds | Where-Object { $registerIds -notcontains $_ })
    $onlyRegister = @($registerIds | Where-Object { $catalogueIds -notcontains $_ })
    if ($onlyCatalogue.Count -gt 0) {
        $failures.Add("catalogue targets with no register row: " + (($onlyCatalogue | Sort-Object) -join ' ') + ' ')
    }
    if ($onlyRegister.Count -gt 0) {
        $failures.Add("register rows naming no catalogue target: " + (($onlyRegister | Sort-Object) -join ' ') + ' ')
    }
    $duplicate = @($registerIds | Group-Object | Where-Object { $_.Count -gt 1 } | ForEach-Object { $_.Name })
    if ($duplicate.Count -gt 0) {
        $failures.Add("register carries a target more than once: " + (($duplicate | Sort-Object) -join ' ') + ' ')
    }

    # -- 2. every third-party package has exactly one row, at its version ------
    #
    # ⚠ A package with no source is a member of this workspace and is not third
    # party, which is the rule check-project applies to the same file.
    $lockDeps = [System.Collections.Generic.List[string]]::new()
    $name = ''; $version = ''; $hasSource = $false
    foreach ($line in (Get-Content -LiteralPath $lock)) {
        $text = $line -replace "`r$", ''
        if ($text -eq '[[package]]') { $name = ''; $version = ''; $hasSource = $false; continue }
        if ($text -cmatch '^name = "(.*)"$') { $name = $Matches[1]; continue }
        if ($text -cmatch '^version = "(.*)"$') { $version = $Matches[1]; continue }
        if ($text -cmatch '^source = "') { $hasSource = $true; continue }
        if ($text -cmatch '^checksum = "' -and $hasSource) {
            [void]$lockDeps.Add("$name`t$version")
        }
    }
    $lockSet = @($lockDeps) | Sort-Object
    $registerDeps = @($deps | ForEach-Object {
        (Get-Field $_ 'id') + "`t" + (Get-Field $_ 'version')
    }) | Sort-Object

    $onlyLock = @($lockSet | Where-Object { $registerDeps -notcontains $_ })
    $onlyReg = @($registerDeps | Where-Object { $lockSet -notcontains $_ })
    if ($onlyLock.Count -gt 0) {
        $failures.Add("locked packages with no register row: " +
            ((($onlyLock | Sort-Object) | ForEach-Object { $_ -replace "`t", ' ' }) -join ';') + ';')
    }
    if ($onlyReg.Count -gt 0) {
        $failures.Add("register rows naming no locked package: " +
            ((($onlyReg | Sort-Object) | ForEach-Object { $_ -replace "`t", ' ' }) -join ';') + ';')
    }

    # -- 3. every row carries a disposition -----------------------------------
    $noDisposition = [System.Collections.Generic.List[string]]::new()
    foreach ($entry in $targets) {
        $id = Get-Field $entry 'id'
        $licence = Get-Field $entry 'licence'
        $rule = Get-Field $entry 'redistribute'
        if ($licence -eq '') { [void]$noDisposition.Add("$id has no licence"); continue }
        if ($rule -ne 'refused' -and $rule -ne 'permitted') {
            [void]$noDisposition.Add("$id has an unknown redistribute value: $rule")
        }
    }
    foreach ($entry in $deps) {
        $id = Get-Field $entry 'id'
        $licence = Get-Field $entry 'licence'
        $rule = Get-Field $entry 'redistribute'
        if ($licence -eq '') { [void]$noDisposition.Add("$id has no licence"); continue }
        if ($rule -ne 'refused' -and $rule -ne 'permitted') {
            [void]$noDisposition.Add("$id has an unknown redistribute value: $rule")
        }
    }
    if ($noDisposition.Count -gt 0) {
        $failures.Add("register rows with no disposition: " + ($noDisposition -join ';') + ';')
    }

    # -- 4. permitted is the expensive value and it has to be earned ----------
    #
    # ⛔ Redistribution needs a licence somebody established and a notice to
    # carry with it. unverified plus permitted would publish somebody's bytes on
    # nobody's authority.
    $unearned = [System.Collections.Generic.List[string]]::new()
    foreach ($entry in ($targets + $deps)) {
        if ((Get-Field $entry 'redistribute') -ne 'permitted') { continue }
        if ((Get-Field $entry 'licence') -eq 'unverified' -or (Get-Field $entry 'notice') -eq '') {
            [void]$unearned.Add((Get-Field $entry 'id'))
        }
    }
    if ($unearned.Count -gt 0) {
        $failures.Add("permitted without a verified licence and a notice: " + ($unearned -join ' ') + ' ')
    }

    # -- 5. a closed-source target is never recorded under an open licence ----
    #
    # ⚠ The catalogue already says which targets are closed source, so this
    # compares two files rather than restating one of them.
    $closed = [System.Collections.Generic.List[string]]::new()
    $current = ''
    foreach ($line in (Get-Content -LiteralPath $catalogue)) {
        $text = $line -replace "`r$", ''
        if ($text -eq '[[targets]]') { $current = ''; continue }
        if ($text -cmatch '^id = "(.*)"$') { $current = $Matches[1]; continue }
        if ($text -ceq 'open_source = false' -and $current -ne '') { [void]$closed.Add($current) }
    }
    $mislabelled = [System.Collections.Generic.List[string]]::new()
    foreach ($id in ($closed | Sort-Object)) {
        $entry = $targets | Where-Object { (Get-Field $_ 'id') -eq $id } | Select-Object -First 1
        if (-not $entry) { continue }
        $licence = Get-Field $entry 'licence'
        if ($licence -ne 'proprietary' -and $licence -ne 'unverified' -and $licence -ne '') {
            [void]$mislabelled.Add("$id is closed source and recorded as $licence")
        }
    }
    if ($mislabelled.Count -gt 0) {
        $failures.Add("closed-source targets under an open licence: " + ($mislabelled -join ';') + ';')
    }

    # -- 6. no artifact this repository may not redistribute is tracked -------
    #
    # ⛔ THE HALF A REGISTER CANNOT ANSWER BY ITSELF. Every row says the bytes
    # are never shipped; this is what checks that none are here.
    $tracked = @(& git ls-files) + @(& git ls-files --others --exclude-standard)
    $bundled = @($tracked | Sort-Object -Unique | Where-Object {
        $_ -match '(?i)\.(exe|msi|dmg|pkg|deb|rpm|appimage|apk|jar|7z|zip|xz|bz2|gz|tgz|torrent|dll|so|dylib)$'
    })
    if ($bundled.Count -gt 0) {
        $failures.Add("tracked artifact this repository may not redistribute: " + ($bundled -join ' ') + ' ')
    }

    # ⛔ A REGISTER OF NOTHING IS NOT A CLEAN REGISTER. Every rule above is
    # satisfied by two empty lists, which is how a broken parser reports success.
    if ($targets.Count -eq 0 -or $deps.Count -eq 0) {
        $failures.Add(
            "the register parsed to $($targets.Count) target row(s) and $($deps.Count) dependency row(s)")
    }

    if ($Json) {
        Write-Output ('{"schema":"check-licences/1","failures":' + $failures.Count +
            ',"targets":' + $targets.Count + ',"dependencies":' + $deps.Count + '}')
    }
    elseif ($failures.Count -eq 0) {
        Write-Output ("licence register: {0} target(s) and {1} dependency row(s), every one with a disposition" -f
            $targets.Count, $deps.Count)
    }
    else {
        foreach ($failure in $failures) { Write-Output ("FAIL: " + $failure) }
    }
    if ($failures.Count -eq 0) { exit 0 } else { exit 1 }
}
finally {
    Pop-Location
}
