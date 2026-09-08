# CI entries

## CI-01: Complete cross-platform quality gate

Source: template gate method and operator best-in-class CI requirement
Priority: P0 | Effort: L | Status: DONE

Problem: The bootstrap CI verifies code and documents but not future schema,
fixtures, corpus determinism, or publication invariants.

Approach: Add focused Linux and Windows jobs, immutable action pins, explicit
permissions, concurrency, timeouts, caching by lockfile, and strict local gates.

Prove: every required check appears as a non-skipped CI result, and injected
failures in docs, schema, fixtures, and Rust each turn the workflow red.

⚠ **Most of the Problem was overtaken before this entry was started.** The
Linux lane already delegates to `check-gate.sh --strict`, which runs the corpus
and publishing provers, and the Rust suite already covers the schema and the
fixtures on both lanes. What was actually missing was the two halves of the
Prove, and neither was a matter of adding jobs.

### What the first half turned out to be: a flag nobody could use

⛔ **The Windows lane ran without `--strict`, and it had to.** Six of that
runner's rows are checks Windows genuinely cannot run, so the flag would have
refused every correct tree. ⚠ The consequence is the one that matters: a check
that *stopped* running there was counted as a skip beside those six, and the
lane stayed green. Measured on 2026-09-06 by rewriting `check-project.ps1` to
`exit 2`; that lane's own invocation exited 0.

⭐ The runner now counts a **declared** unavailability apart from an **observed**
skip. A declared row is written into the runner with its reason and the entry
that owns it and prints as `n/a`; an observed one is a check that answered 2 or
whose file has gone. `--strict` refuses only the second, so both lanes ask
strictly and the six documented gaps stay documented.

### What the second half turned out to be: a harness that cannot drift

`scripts/ci/check-workflow.sh` copies the working tree into a scratch
repository, plants a defect of each class, and runs the offending step against
it. ⛔ **Every command it runs is read out of the workflow by job and step
name.** A harness holding its own copy of a build command proves that command
refuses a defect and says nothing about the one CI runs, and a step this file
names and the workflow no longer has is reported rather than passed over.

⛔ **It is kept out of `check-gate.sh` and that is a shared contract.** Two of
its cases run the workflow's *Repository gate* step, so a runner inside the gate
that also runs the gate would re-enter itself. The workflow calls it as a step
of its own instead.

⚠ **The gate cases needed a control the exit code could not give.** The
workflow runs the gate with `--strict`, and a developer host with no
authenticated `gh` has an observed skip of its own, so the clean tree exits 1
here and 0 on the lane. The control reads the runner's failure count and records
which host it is on; the plants still read the exit code unpiped.

### ⛔ The door sweep found the same assumption behind two doors

`store_build` composed an example's path as `root/target/debug/examples`, and
cargo obeys `CARGO_TARGET_DIR`. With that variable set the build succeeded, the
binary landed elsewhere, and the check for it fired: exit 2, which the gate
reads as a **skip**. ⚠ All five corpus and publishing provers therefore proved
nothing at all on such a host and reported it nowhere. Fixing that one exposed
the second door: `publish-data.sh` composes the same path itself, so the
publisher exited 2 saying its own append checker was missing. Both resolve the
variable now. Found by driving this entry, whose harness sets it.

### The publisher's workflow, which `PUB-02` left as two residuals

`.github/workflows/publish-data.yml` carries the job-scoped `contents: write`
and the concurrency group. ⛔ **It has no automatic trigger and its dry run is
the default**, because nothing may be published until a measured record exists
and everything in the tree is synthetic; a push trigger here would be one merge
away from publishing that. ⚠ Its group does not cancel in flight, unlike the
gate's: cancelling a gate run costs a rerun, and cancelling a publisher between
its append comparison and its read-back leaves a branch nobody has verified.

⚠ **The token reaches git as a header rather than inside a remote URL.** The
first version wrote it into the URL and `check-no-secrets` refused the file,
which is the guard working: a credential in a remote is visible to anything
listing processes on the runner.

⛔ **The workflow has never run and cannot succeed today.** Its first step
downloads the assembled bundle from a capture run, no such run exists, and that
is the honest state rather than a guard. What is proved is its shape.

### Acceptance, all run on 2026-09-06

- `sh scripts/ci/check-workflow.sh`
- `sh scripts/common/check-gate.sh`
- `pwsh -NoProfile -File scripts/common/check-gate.ps1`
- `cargo test --workspace --locked --all-targets`

### Closure evidence, 2026-09-06

| what | measured |
| --- | --- |
| `sh scripts/ci/check-workflow.sh` | 34 cases, 34 passed, 0 failed |
| `sh scripts/common/check-gate.sh` | 17 checks, 16 passed, 0 failed, 1 skipped, 0 unavailable |
| `pwsh -File scripts/common/check-gate.ps1` | 17 checks, 10 passed, 0 failed, 1 skipped, 6 unavailable |
| `cargo test --workspace --locked --all-targets` | 36 binaries, 337 passed, 0 failed |
| CI run 37, Windows lane | 17 checks, 10 passed, 0 failed, **0 skipped**, 7 unavailable, under `-Strict` |
| CI run 37, Linux lane | green through *Workflow acceptance*, which reported 27 cases passing at that commit |
| driven pass | eight defect classes planted one at a time, each refused by the step that owns it, with the clean tree accepted either side |

⭐ **The Windows row is the Prove's first half, measured rather than argued.**
Zero skipped means every check that lane can run, ran. The seven `n/a` rows are
the six platform gaps and `-Fast`, each named with the entry that owns it.

### Guard mutation

Eight probes over the runner split, run on a scratch worktree: a check rewritten
to `exit 2`, one rewritten to `exit 1`, and one deleted outright, against each
half, plus the clean control on both. All eight landed on the intended verdict.
⛔ The blind spot reproduced exactly: without the flag, the `exit 2` plant left
the runner at 0.

The harness carries nineteen more, being its four inherited probe guards, five
mutation probes over its own static readers, and the ten plant-and-control pairs
in its case list. ⚠ Its static readers are checked against a workflow with each
property **removed**, because a reader that answers present over a file that
lacks it would pass on any workflow at all.

### Residuals

- ⚠ `check-workflow.sh` is not in `check-gate.sh` and cannot be, so a
  contributor's local gate does not run it. The workflow runs it on every push
  and it is this entry's acceptance; a gate runner that listed it would re-enter
  itself.
- ⚠ On a host with no authenticated `gh` the harness's gate cases cannot use the
  exit code as their control, because the clean tree exits 1 there. They read
  the runner's failure count instead and say so in the row. `check-remote-items`
  is the only observed skip in either half.
- ⚠ `append_once` verifies that a file changed but has not itself been refuted,
  the way `replace_once` has. Every plant that uses it did land, which is
  evidence and not a proof.
- ⚠ `CI-03` still owns the Windows capture runner, and the six `n/a` rows on that
  lane close only when it lands.

## CI-02: Stable-release staleness monitor

Source: operator automatic maintenance requirement
Priority: P1 | Effort: L | Status: DONE

Problem: New stable client versions must create bounded work without silently
overwriting previous records or repeatedly opening duplicates.

Approach: Schedule source-specific resolvers, compare against data indexes,
deduplicate by product/version/channel/platform, and open or update one tracked
capture request.

Prove: `sh scripts/ci/check-staleness.sh` and
`cargo test -p bit-ids --locked --test staleness` both pass, with a new stable
release opening one request, a preview and a release already measured opening
none, and a second run over the tracker the first one filled opening nothing.

### Decision: the identifier is derived, and that is the whole of "no duplicate"

⛔ **A request identifier is a digest of its key, never an allocated token.** Two
runs over the same facts derive the same identifier, so a tracker keyed on it
cannot hold two; a counter, a timestamp or a random token would each make the
second run's request a different request, which is the defect the Problem names.
`RequestKey` digests the four components the Approach lists, domain-separated and
length-prefixed the way `RecordKey` is, because joining them with a separator
lets two tuples encode to one string the moment a component carries the
separator.

⚠ **Architecture and package are deliberately not in the key.** They are outcomes
of the acquisition and are unknown when a request is opened, so a request that
carried them would multiply one release into a request per packaging, most of
which no route can satisfy. The question a request answers is whether the
selected version has a measurement on the platform at all; a package variant
lagging is coverage rather than staleness.

### Decision: the survey takes the whole resolution, not a version

⭐ **Nothing here judges stability, and that is why the preview case is not a
tautology.** A preview is refused by `ACQ-02`'s resolver, and `survey` takes the
`Resolution` rather than a `Version` so it cannot be handed a selection that came
from somewhere else. Every case in both the suite and the harness therefore
builds the release list a source would answer with and resolves it first.
A second stability rule here would be a second place for the answer to differ.

### Decision: the comparison is against the views, not the store

⛔ **An index is what a consumer reads instead of the records, and a monitor is a
consumer.** A record in the store and in no view is a measurement nobody can look
up, so counting it as coverage closes work a consumer cannot see was done.
⭐ The corollary is free: `CORPUS-04` drops a superseded record from every view,
so a retracted measurement re-opens its capture with nothing added here.

⚠ **The driving example re-derives the views rather than parsing the published
index document.** `index::build` is the one derivation, and a second reader of
that document would be a second reading of it. The `data` branch *is* the store,
so pointing `survey-staleness` at a checkout of it is the real path.

### ⛔ Four verdicts open nothing, and each is a comparison that did not hold

`regressed` is the one worth naming: a measurement newer than the selection means
the source dropped a release or the resolver read the wrong one, and a request
there asks a runner to capture a downgrade. `unresolved` is reported rather than
skipped, because a target whose resolution has been blocked for a month otherwise
looks like a target with no work. `ambiguous` is `ACQ-02`'s rule applied here:
`components` pads to the scheme's width, so `1.2.10` and `1.2.10.0` compare
equal, and calling that current leaves a differently spelled measurement standing
for the selection while calling it stale opens a request for a capture already
taken. `unorderable` blocks for the reason the resolver blocks.

⭐ `Staleness::opens_work` is the **one** answer to whether a verdict asks for a
capture. `survey` filters on it and `validate_requests` checks against it, so a
verdict wired into one and forgotten in the other is not expressible: it becomes
an `E-REQ-07` refusal naming the verdict.

### ⛔ What the guard mutation pass found, which nothing else could have

Eighteen plants over `staleness.rs`, one at a time into a scratch copy, each
compiled before being judged so that *could not run* stays separate from
*refused*. ⚠ The first pass reported fourteen refusals and four non-results, and
the four were the harness's own defects rather than surviving guards: three
literals no longer matched the formatted source and one plant did not compile.
They were repaired and re-run rather than counted either way, because a plant
that did not apply says nothing about the guard it was aimed at.
⛔ **Two of the refusals were by the harness alone, and that is the finding.**
Removing the platform from the request key, and replacing the length prefixes
with a separator join, both left every Rust case green:
`request_key_components_are_length_prefixed_rather_than_joined` collides only
under a `-` join and nothing varied the platform at all. ⚠ That is a test whose
**name** claimed more than it **checked**, which
[`../docs/methodology/reviews.md`](../docs/methodology/reviews.md) names as a
shape to test for specifically.

⭐ The repair is to pin the encoding rather than to add another collision pair.
`request_key_bytes_are_the_encoding_restated_independently` compares
`canonical_bytes` against the format restated from its own doc comment, so any
dropped component, any reordering and any join moves the bytes;
`a_request_id_moves_with_every_component_of_its_key` varies each of the four in
turn. Re-measured after the repair: all five key plants, including two written
only to check the repair, are refused by the unit tests alone.

Re-measured after the repair, over the whole set: eighteen plants, eighteen
refused, none surviving and none unable to run.

### What the door sweep found, which the mutation pass could not

Three, each about a door this entry opened rather than about the code it wrote.

⛔ **`VersionScheme::components` said "both callers use it" and named two.**
This is the third. A count in prose is a value in two places with nothing
comparing them, and it went stale in the one paragraph a reader checks before
writing a fourth implementation of the ordering. It says "every" now and names
all three questions.

⛔ **The channel's published spelling was written inside `staleness`.**
`docs/architecture.md` section 5 already made this rule for `Surface`: the
vocabulary has one home and it is the record's. A spelling of `ReleaseChannel`
living in a consumer is the shape where the next consumer writes a third. It is
`ReleaseChannel::as_str` beside the type now, with the serde pairing checked
where it always was.

⚠ **`scripts/README.md` said `store-lib.sh` is sourced by "all eight of the
harnesses above".** Measured: nine harnesses source it, `publish-data.sh` does
too and is not a harness, and `check-runner.sh` is listed above and does not
source it. The sentence was describing a set two files differ from before this
entry touched it. It carries the measured count and both exceptions now.

⚠ A fourth was looked for and not found: nothing else in the tree reaches
`survey`, `RequestId` or `validate_requests`, and `serde_json::to_string` over a
`RequestSet` does bypass validation, which is the same open in-memory
construction `Profile` has by design. For that to be a defect the write path
would have to stop validating, and `staleness_writes_nothing_it_would_refuse_to_read`
is what would fire.

### Acceptance, all run on 2026-09-08

- `sh scripts/ci/check-staleness.sh`
- `cargo test -p bit-ids --locked --test staleness`
- `sh scripts/common/check-gate.sh`
- `pwsh -NoProfile -File scripts/common/check-gate.ps1`

### Closure evidence, 2026-09-08

| what | measured |
| --- | --- |
| `sh scripts/ci/check-staleness.sh` | 19 cases, 19 passed, 0 failed |
| `cargo test -p bit-ids --locked --test staleness` | 29 passed, 0 failed |
| `cargo test --workspace --locked --all-targets` | 46 binaries, 503 passed, 0 failed |
| `sh scripts/common/check-gate.sh` | 21 checks, 20 passed, 0 failed, 1 skipped, 0 unavailable |
| `pwsh -File scripts/common/check-gate.ps1` | 21 checks, 11 passed, 0 failed, 1 skipped, 9 unavailable |
| guard mutation over `staleness.rs` | 18 plants, 18 refused, 0 survived, 0 could not run. ⚠ Two were refused by the harness alone before the tests were repaired |
| driven pass | a real store at 1.2.10, a real release list carrying 1.3.0, resolved and surveyed: one request opened; the tracker filled from it and re-surveyed twice, the same identifier `already_open` both times and the two documents identical apart from the clock |
| independent verification | `python3`'s SHA-256, over the encoding restated from the doc comment, re-derives `request:sha256:d6ce71ee…` byte for byte |

⭐ **The strongest control here is not this project's code.** The request
identifier is what the whole no-duplicate property rests on, and a survey
compared against its own encoder agrees with itself. Python's `hashlib` is a
SHA-256 this project did not write, and the harness carries that comparison as a
case rather than leaving it to a session that ran it once.

### Residuals

- ⚠ **Nothing schedules this yet.** The Approach says "schedule source-specific
  resolvers", and what exists is the comparison and its driving surface, not a
  cron trigger or an issue writer. A workflow that opened issues would need
  `issues: write` on this repository, and `docs/security/remote-ops.md` governs
  that; the tracker's shape is `--open`'s JSON array, which any writer can
  produce. It is a residual rather than a gap in the Prove: the Prove asks
  whether a request is created, updated and not duplicated, and all three are
  driven.
- ⚠ `check-staleness` needs `python3`, so it exits 2 on a host without one.
  Driven rather than asserted: run under a `PATH` carrying every other tool it
  needs and no python3, it prints `python3 not found` and exits 2, which the gate
  reads as a skip and never as a pass. The Linux lane runs the gate with
  `--strict` and `ubuntu-24.04` carries python3.
- ⚠ `survey-staleness` builds the views under the schemes its resolutions carry,
  so a store holding a target no resolution covers blocks under `E-VIW-01`. That
  is the right refusal and it means a real run monitors every target in the store
  or none. It is a residual because nothing today has more than one target in a
  store.
- ⚠ The survey's exit code says whether a request was opened and says nothing
  about a blocked line. The alternative, a third code, was rejected: a caller
  that only wants to know about new work would have to special-case it. Blocked
  lines are named in the summary and carried in the document.
- ⚠ An open request for a line a run was not asked about is left alone rather
  than retired. A survey that retired work outside its input would delete a
  capture request because a target was left out of one run's arguments.

## CI-03: Trusted capture runner matrix

Source: proprietary clients, multiple host families, and active observation
Priority: P1 | Effort: L | Status: OPEN

Problem: Public pull-request jobs cannot safely run installers, privileged
network isolation, or publication credentials.

Approach: Separate untrusted validation from trusted capture, use fresh Linux
and Windows guests, environment protection, no fork secrets, per-client
timeouts, retained evidence, and manual approval where terms require it.

Prove: a forked change cannot reach capture credentials or runners; a trusted
fixture capture attests isolation and uploads its complete evidence bundle.

### ⛔ A hosted runner is a capture host, and recording otherwise was an error

Earlier sessions recorded nineteen entries as waiting on a host that
`assert-disposable.sh` would not refuse, and treated that as an external
blocker. It is not one.

- `--claim` writes a marker and refuses if one exists. A hosted runner is a fresh
  virtual machine per job, so no marker exists and it passes.
- `--egress` reads `/proc/net/route`, sends no packet, and refuses only because a
  default route exists. Deleting the default route satisfies it, and runners have
  passwordless sudo.

Measured on 2026-09-08 with the guard's own optional route-table argument: exit 1
over this host's real table, exit 0 over the same table with the default route
stripped, exit 0 over a loopback-only table. ⚠ **The guard was testable in one
command the whole time.** A blocker nobody tested is the shape this entry now
exists to stop being.

### The Windows guard pair, which is the part that is done

`assert-disposable.ps1` is the twin of the sh guard: `-Claim` writes an
exclusively created marker under `ProgramData`, `-Egress` reads `Get-NetRoute`,
and `-Fingerprint` digests the machine GUID with the install identity and the
computer name.

⛔ **It checks both address families.** A Windows host with IPv4 unplugged and
IPv6 up still reaches the internet, so a guard reading only `0.0.0.0/0` would
pass it.

⭐ **`-RouteTable` takes a file, which is what makes the logic provable on a host
with no `Get-NetRoute` at all.** `check-runner.ps1` drives every case against
fixtures, and `check-runner` is a real row on the PowerShell gate lane rather
than a declared gap. ⚠ The count is not written here: this paragraph said
"eleven" for as long as it took the claim audit to reach it, and the harness
prints its own total. ⚠ What that does **not** establish is that `Get-NetRoute`'s
real output matches the fixtures; only a Windows job running the guard with no
`-RouteTable` does, and that is part of the workflow below.

### Guard mutation over the Windows pair

Seven plants, one at a time. Four were refused on the first pass and three
survived; reading what each changed separated one real limit from two weak cases.

⛔ **Two of the harness's own cases passed for the wrong reason.** Three of the
guard's refusals share exit 2 and two share exit 1, so a case asserting the code
alone passes when a different refusal fired: blanking the mode check left both
mode cases green, because the branch they then fell into also answers 2. Every
case that shares a code now asserts what the guard said as well.

⚠ **One survivor is a guard nothing can refute and it is kept.** Rewriting the
marker's exclusive create to a plain create changes no behaviour a test can
observe, because the `Test-Path` check answers first on every constructible
input. It matters only in the window between that check and the open, which is
the race it exists for. Recorded rather than removed, the way `PUB-01` records
`entries.sort()`.

⛔ **Writing the harness found two defects in the guard on its first run.**
`Show-Usage` used `break` inside `ForEach-Object`, which does not stop the
pipeline: it breaks the enclosing loop, and with none it unwinds the script,
which exits 0. Every misuse of the guard therefore printed usage and reported
success. And `@(...) | Where-Object` yields a scalar for a single match under
`Set-StrictMode`, so `.Count` threw and every invocation exited 1 with a property
error.

### What remains: the capture workflow

A `workflow_dispatch`-only workflow with one job per platform:

1. check out, then claim the host before anything else writes to it;
2. install the toolchain and build **while the network still exists**, because
   after the route goes nothing can be fetched and a capture that discovered a
   missing dependency under containment would have to restore egress to fix it;
3. delete the default route in both address families;
4. run the egress guard, which must now pass;
5. capture;
6. restore the route only to upload, after the measurement is finished and on a
   host that is destroyed either way;
7. upload the evidence bundle and print the fingerprint the next job compares
   against.

⛔ **There is no `pull_request` trigger and that absence is the fork guard.** A
fork cannot cause a workflow to run in the base repository, so there is no
job-level condition for anyone to weaken. `check-workflow.sh` is where the
absence is asserted, because an absence is what a later edit restores unnoticed.

⚠ **A first draft of that workflow was written this session and removed**, because
it called `capture-run.sh` and `capture-run.ps1`, which do not exist. A workflow
naming a script nobody wrote is a file that reads as finished and fails on its
first dispatch.

## CI-04: Build provenance and supply-chain hardening

Source: public autonomous publisher threat model
Priority: P1 | Effort: L | Status: OPEN

Problem: A valid-looking release can still be produced by an unexpected source
revision, dependency set, or workflow identity.

Approach: Add artifact attestations, dependency review, SBOMs, minimal token
permissions, environment rules, signed release metadata, and reproducible
builder facts.

Prove: release verification binds every asset to the expected repository,
workflow, commit, lockfile, and checksum manifest.

## CI-05: Acceptance commands that cannot pass over nothing

Source: found while closing `OBS-01` on 2026-09-05
Priority: P1 | Effort: S | Status: DONE

Problem: An entry's `Prove` is the acceptance, and one of them ran nothing while
exiting 0. `cargo test --workspace --locked lab_supervisor` filters by test
**name**; the name matched none, so every binary in the workspace printed
`running 0 tests` and the command succeeded. Nothing in the gate can tell that
from an acceptance that passed.

Premise: Measured on 2026-09-05, not read. The nine `cargo test` acceptance
commands in `TODO/` were all of the bare-filter form, and they worked only
because of a convention nothing checks: all 110 test functions across the eight
pre-existing test files begin with their file's name. `lab_supervisor` did not,
and the acceptance went green over zero tests.

⚠ **This entry claimed all nine had been rewritten, and that was false.** Only
the observer entries were. Re-measured while closing this one: five `Prove`
commands were still of the bare-filter form, in `FOUND-03` and in all four
`SCHEMA-*` entries. They are corrected now, and each corrected command was run
before it was written down. That the claim survived a session is the argument for
this entry: a fact about the record that only a person checks is a fact that
drifts.

Approach: A rule in `scripts/common/check-project.sh` and its PowerShell twin
that reads every `cargo test` invocation and refuses a bare word argument unless
it follows a flag that takes a value. ⚠ The parsing is the work: a code span can
wrap across lines, so the paragraph is joined before the spans are found, and the
two twins must agree per planted mutation rather than on a clean tree.

⭐ **Scoped to `Prove:` paragraphs, and that scope is the whole rule rather than
an exclusion list.** A `Prove` is the live acceptance and has to be runnable. A
`Closure evidence` paragraph records what was run on a past tree, and rewriting
one would falsify the record; and this entry and `OBS-01` both have to quote the
command that caused the defect. A rule that fired on those is a rule somebody
switches off, and the seven remaining bare filters in the tree are all of exactly
those two kinds.

⛔ **The door sweep found a second door and it is the one that matters more.**
An entry's `Prove` is the acceptance a person runs; the workflow's `run:` is the
one every push runs, and a bare filter there would report green over zero tests
on every commit with nobody reading it. The rule covers
`.github/workflows/*.yml` as well, with separate extractors and one tokeniser,
because a rule on one of two doors into the same mistake is the shape
`docs/methodology/reviews.md` names.

Decision: a shape rule over a naming convention. Renaming every test function to
start with its file's name would also make the bare filter work, and it makes
the filter's correctness depend on a convention no check holds, while
`docs/conventions/code.md` asks for test names that describe behaviour.

Prove: `sh scripts/common/check-project.sh` and
`pwsh -File scripts/common/check-project.ps1` both exit 1 when a `Prove` line is
rewritten to the bare-filter form, both exit 0 on the tree as it stands, and
both agree on every planted mutation.

Closure evidence: run on 2026-09-05. Both halves exit 0 on the tree as it
stands, and `sh scripts/common/check-gate.sh` and
`pwsh -File scripts/common/check-gate.ps1` both pass with `check-twins` green.

Guard mutation: 21 cases planted one at a time, 16 into a `Prove` paragraph and
5 into the workflow, each verified to have changed the file. Both halves were run
on every one and their **exit codes and their output** compared, because
`check-twins` compares the two on the tree it runs against and a rule that
differs only on a defect the tree does not contain is invisible to it. All 21
landed on the intended verdict and the twins agreed on all 21, character for
character.

⭐ The cases are the shape space rather than one example: a bare filter with and
without other flags, a filter after `--`, a filter after a value-taking flag, a
filter after a flag whose value was attached with `=`, a filter in a code span
that wraps across lines, a target in a span that wraps, two commands in one
`Prove`, a `Prove` running no `cargo test` at all, a bare filter in a paragraph
that is not a `Prove`, and a commented-out one in the workflow. The last three
are the ones that would make the rule fire on correct usage, and each is
accepted.

Residual: the rule cannot see the other half of the class, which is
`--test <target>` skipping the library's own tests. That form runs something, so
it is not "passing over nothing", and refusing it outright would fire on a
package with no library tests. `docs/conventions/forbidden-patterns.md` carries
the class. ⚠ Two open entries still carry it in their `Prove`, `OBS-10` and
`CORPUS-02`, and both are corrected when they close, as `OBS-09`'s was.
