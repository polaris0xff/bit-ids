# Session record, 2026-09-08: routes, and the checks that were only prose

The saved summary of the session that read the aria2 hang out of an artifact
rather than dispatching for it, found that no route in this tree had ever been
shown to install anything, and acquired one target twice.

⚠ A saved summary, not a diary. The current state lives in
[`../../TODO/PROGRESS.md`](../../TODO/PROGRESS.md) and the entries.

---

## 1. The aria2 hang, answered by refuting the question

Three sessions had reasoned about `NEEDRESTART_MODE`. The install logs were
uploaded on `always()` by then, so this session downloaded them - through rule
8's route, unauthenticated, which also settles that an artifact zip does not need
`gh` - and both hung runs say the same thing:

```text
aria2 is already the newest version (1.37.0+debian-1build3).
0 upgraded, 0 newly installed, 0 to remove
```

⛔ **No package was ever installed**, so `needrestart` never ran and the letter in
that variable could not have been the cause under any value. Run 3 carried `a`
and run 4 carried `l`, and the two jobs hung identically.

⛔ **And the hang is not in the install step.** The job step records put it
somewhere else:

| run | install step | where the job stopped |
| --- | --- | --- |
| 1 | never returned | *Install the client*, unbounded |
| 2 | never returned | *Install the client*, a bound that sent TERM and waited |
| 3 | success in 6s | *Upload the install logs* |
| 4 | success in 6s | *Upload the install logs* |

⚠ **The artifact that step produced is complete and downloadable**, so its work
finished and the step still did not return. Four runs cannot separate what holds
a runner open after that, and naming a cause would be a guess with an artifact
beside it. `CI-08` owns runner defaults; this is recorded there rather than
solved here.

⭐ **The adapter is not the problem**, and it got its first driven pass: a real
install in 8s, the no-op repeat in 3s, `version` answering `1.37.0`.

---

## 2. The finding that outlived the hang

A route is contracted to install the target. **Nothing established that it did.**

- `aria2` ships on the `ubuntu-24.04` image, so `apt-get install aria2` there
  prints `already the newest version`, installs nothing and exits 0.
- Every `release` route in the tree fetched an artifact into the workdir and
  never made it the executable the adapter asks, so a release route reported
  whatever the package route had left on `PATH`.

⛔ **Two routes in that state satisfy every existing check.** They declare
different resolvers and different delivery mechanisms, so `E-ACQ-07` and
`E-ACQ-08` pass; they agree on the version because it is one binary; and
`ACQ-03` classifies the pair `byte_identical`, which is its **strongest**
verdict. `equivalence.rs` even records the reason: *"every route installed
&lt;digest&gt;, so there is one build and observing one observed it"* - exactly
true, and exactly the wrong conclusion. ⚠ The evidence gets better the more
completely nothing was acquired, which is why no check comparing declarations
could ever have caught it.

⭐ `install-client` asks the adapter for a version and for its executable before
the route runs and after, and records `preexisting_version`,
`preexisting_binary`, `preexisting_binary_sha256`, `installed_binary`,
`installed_binary_sha256` and `acquired`.

---

## 3. Two routes, one target, measured

The first time this project has held two independent installs of one target at
once - and it needed no disposable host, because building a product and asking
its version is not a capture.

| | route A | route B |
| --- | --- | --- |
| resolver | Ubuntu 24.04 package index | the vendor's GitHub release |
| delivery | `apt-get install aria2` | source tarball, configured and compiled |
| reported version | `aria2 version 1.37.0` | `aria2 version 1.37.0` |
| the executable | 14584 bytes, stripped, links `libaria2.so.0` | 2833064 bytes stripped, self-contained |
| features | Async DNS, BitTorrent, Firefox3 Cookie, GZip, HTTPS, Message Digest, Metalink, XML-RPC, SFTP | BitTorrent, GZip, HTTPS, Message Digest |
| cost | seconds | 19s configure, 126s `make -j4` |

Three things follow, all recorded in `ACQ-03`:

⛔ **One version, two builds.** Features are what a build does on the wire, so
every adapter's `version` keeps the build's whole answer on stderr now, which
both callers already write into evidence.

⛔ **`installed_executable` records a shim** for the package route. The protocol
code is in `libaria2-0`, whose digest the record model has nowhere to put.

⛔ **A source route can never reach `byte_identical`.** Two builds of one tarball
at two prefixes differ, because `configure --prefix` is compiled in.

⚠ **And aria2 is the only target whose two routes currently resolve the same
version.** Ubuntu ships Transmission 4.0.5 against upstream 4.1.3 and qBittorrent
4.6.3 against upstream 5.2.3, and rule 5 forbids backfilling to make a pair
agree. The first two-route capture is `CLIENT-05`'s.

---

## 4. What the checks were, and what they only said they were

Three separate rules turned out to exist in prose alone. Each was found by
breaking something rather than by reading.

⛔ **A line-endings check.** `conventions/shell.md` section 5 has described one,
by name and with the command, for as long as it has described the problem.
Nothing implemented it. Found when a file-writing tool that emits LF left
`check-project.ps1` at `w/lf` under `attr/text eol=crlf`: a full gate passed over
that tree and the only thing that reported it was an incidental warning from
`git commit`. `git diff` prints nothing for that state in either direction.

⛔ **A count of the declared PowerShell rows.** `CI-07` said `thirteen`; the
runner declares fourteen. That is the second time such a count went stale in
prose - `CI-01` records the first, six becoming twelve - and `CI-01`'s own fix
says no count of them is written into prose anywhere. One had survived. The
number is removed rather than corrected.

⛔ **The index of session records.** `docs/history/README.md` carries a note
saying three records were missing from it until 2026-09-08. A fourth was missing
from it by the end of that same day, held in the tree only by a link from
`RESUME.md`, which is overwritten every session. The note observed the pattern
and changed nothing. It is a check now.

⚠ **And a claim about the tree that was simply wrong**: `RESUME.md` said the
session records were "linked from here and nowhere else". The index links them
too - which made the sentence false in the direction that mattered, because the
one record it was true of was the one missing from the index.

⭐ **The line-endings rule caught its own author within the hour.** Writing the
PowerShell half of the history-index check with a tool that emits LF left
`check-docs.ps1` at `w/lf` under `attr/text eol=crlf`, and the gate refused it -
the same defect, in the same file type, by the same mechanism, on the first
occasion after the rule existed. ⚠ Before it existed, that state reached a
commit and only `git` mentioned it, in a warning.

---

## 5. What was added to the gate

Each of these is in both halves and mutation-proved per plant rather than by
comparing the halves on a clean tree.

| rule | plants it refuses |
| --- | --- |
| a `download-artifact` name no `upload-artifact` produces | undeclared; a declaration naming no real entry; a declaration gone stale; a `pattern:` download; a step written `- with:` / `name:` / `uses:` |
| priority and effort agree between `INDEX.md` and the entry | a priority changed on the index side; an effort changed on the body side |
| working-tree line endings agree with `.gitattributes` | LF where CRLF is required; CRLF where LF is |
| `set -u` is the first code line of every executable script | a script without it; a script with it below something else |
| a cargo output path names `CARGO_TARGET_DIR` | a composed `$ROOT/target/debug/examples` |
| the `shfmt` version agrees between the workflow and the pin | the workflow drifts; the pin disappears |
| every `SESSION-*.md` is listed in the history index | a record dropped from the index |

⚠ **The artifact rule's own first version was wrong twice**, and only planting
found either: it scanned forward from `uses:` and took the first `name:` it met,
so a step whose `with:` precedes `uses:` reported the STEP's name; and it read
`name:` alone while `download-artifact` also accepts `pattern:`. ⛔ The first
plant PASSED before its reported name was read, because a rule that never found
the row and a rule that found it and accepted it both exit 0.

---

## 6. `FOUND-05`, and a collision inside one file

`sh scripts/doctor/provision.sh` installs `pwsh`, `shellcheck` and `shfmt` in
four seconds on a host with none of them, verifying every download against a
pinned digest before anything is executed.

⛔ **A host without `pwsh` does not run a smaller gate, it runs a RED one.**
Measured by taking all three away: `--strict` answered `FAIL check-capture` and
`SKIP check-twins`. With them back the only row left is `check-remote-items`.

⛔ **The first version reported two tools wrong that it had installed
correctly.** `fetch_verified` and `provide` both used `_want`; a POSIX shell
function has no locals, so the caller compared a version against a digest.
⭐ `shfmt` was the one row that reported correctly, because `go install` is the
single route that never calls the fetcher - so the two tools that downloaded were
exactly the two that lied. Invisible to `shellcheck` and to reading either
function alone.

⚠ **The three pins are not the same kind of evidence.** PowerShell publishes
`hashes.sha256` and its line matched the bytes downloaded here, so that one is
corroborated. The other two publish no checksum file, so theirs are immutability
pins and not authenticity ones, and the file says which is which.

---

## 7. The review passes

**Door sweep.** Enumerated every consumer of the install record: the workflow
`cat`s it and uploads it, the harness asserts on it, and no Rust reads it - so
the format change is contained. Then swept the change itself and found two doors
in the new artifact rule that the tree does not exercise, and one in my own
placement of `provision.sh`: `scripts/doctor/README.md` promises a read-only
pass, so the installer says in both READMEs that it is the exception and that the
doctor does not call it.

**Guard mutation.** Every rule above was planted against. Two plants found
defects in the guards themselves: the `bit-ids:no-producer` prefix was stripped
by a counted length that was one out, so every marker read as `I-09` and the
clean tree failed; and the artifact parser took the wrong `name:`. ⚠ One plant
was invalid and saying so is the point: removing `RESUME.md`'s link to a session
record did not orphan it, because the index links it too, so that pass
established nothing until it was repeated with a real orphan.

**Claim audit.** Verified the counts that remain in current-state prose: `six
renderings` is held by `check-formats` naming all six, and `eight kinds` by
`RouteKind::ALL`. Both would fire if a rendering or a kind were added, which is
what a number behind a check means. The counts that had gone stale were the ones
behind nothing. Also read `equivalence.rs` rather than inferring what `classify`
would do with two no-op routes, which is where the `byte_identical` sentence in
section 2 comes from.

**A fourth, on the driven pass.** The hostile-environment run of the gate gave
the same verdict as the ordinary one - and the locale half of it had established
nothing, because `LC_ALL=C` on a host already reporting `POSIX` changes nothing.
The host was measured and the run repeated under `C.utf8`.

---

## 8. What is still open

⛔ **Nothing has been published and no record exists in the store.** Every capture
so far is one route, one connector, one platform.

⛔ **The capture-to-publisher path cannot be exercised.** The publisher downloads
an artifact named `bundle` and nothing in the tree produces that name; a capture
bundle is not a publication bundle; and what sits between them is an assembler
that reads records, of which none has been written. The gap is declared in the
workflow and enforced by `check-project` rather than left to be rediscovered.

⛔ **The release route has never run on a capture host**, and the capture workflow
still passes `--route package` only. Wiring the second route into it is the next
change, and the open question in it is which asset a resolver should pick.
