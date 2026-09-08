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
- ⛔ **This entry said the `n/a` rows on the Windows lane "close only when
  `CI-03` lands", and that named the wrong event.** `CI-03` has landed and not
  one of them closed: they are `sh` harnesses, and what they need is a
  PowerShell half, which the capture runner matrix was never going to write.
  ⚠ The count was wrong too: it said six, and the runner had twelve rows
  carrying that label by the time anyone re-read it. Both are corrected in
  `check-gate.ps1`, where each row now names the event that would actually close
  it, and no count of them is written into prose anywhere. Found by a claim audit while closing `CI-03`, which
  is the pass that reads a sentence against the tree rather than against the
  sentence next to it.

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
Priority: P1 | Effort: L | Status: DONE

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

### The capture workflow, which is the rest of it

[`../.github/workflows/capture.yml`](../.github/workflows/capture.yml) is
`workflow_dispatch`-only with one job per platform, and the step order is the
containment rather than a convention:

1. check out, then claim the host before anything else writes to it;
2. install the toolchain and build **while the network still exists**, because
   after the route goes nothing can be fetched and a capture that discovered a
   missing dependency under containment would have to restore egress to fix it;
3. delete the default route in both address families, saving them first;
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

⚠ **A first draft of that workflow was written the previous session and
removed**, because it called `capture-run.sh` and `capture-run.ps1`, which did
not exist. A workflow naming a script nobody wrote is a file that reads as
finished and fails on its first dispatch. ⭐ **The scripts exist now, and so does
the rule that would have caught the draft**: `check-workflow.sh` pulls every
`scripts/…` path out of every workflow and refuses one that is not in the tree.
It is refuted against a copy naming a script that is not there, because a reader
answering "present" over any input would have passed the case on the draft too.

### ⛔ The step order is enforced, because every wrong order reads as plausible

Building after the route is deleted cannot work, and the only way to repair it
under containment is to restore the route on the host that exists to have none.
Capturing before the guard measures a host nobody established was contained.
Uploading before the restore is a step that cannot reach GitHub. ⚠ **None of
those is visible to any other check in this repository**, and each is a
two-line move in a diff. So `check-workflow.sh` reads the ordered step names of
each capture job and asserts every precedence the containment rests on, with a
step it names and the workflow no longer has reported rather than passed over. The reader is
refuted against a copy with two step names swapped.

⚠ **And a capture job whose `Capture` step ran `true` would satisfy all six.**
That is a separate case: the step's command must name the runner.

### The runner itself, and the three things it refuses that no workflow can

`capture-run.sh` and `capture-run.ps1` take an already built observer and a host
that a previous step claimed and contained.

⛔ **It builds nothing, and a missing observer is `could not run`.** That is what
makes the step order enforced rather than remembered: a workflow that moved the
build after the containment gets a refusal naming the binary, not a cargo
invocation reaching for a network that is gone.

⛔ **It re-reads the claim marker and the routing table itself.** Both were
already checked by earlier steps, and that is the point: a step order that
dropped the claim leaves the capture running on a host nothing claimed, and a
gate on one of two paths into the same action is the one-gated-door defect. ⭐ The
marker's path is asked for through `assert-disposable --marker`, which is a new
mode added for this and is the only derivation of that path; composing
`$STATE_DIR/host-claimed` a second time would go on reading the old place the
day the state directory moves.

⭐ **The driver and the verifier are both somebody else's code.** `curl` is a
complete HTTP client on both runner images and puts real bytes through the
tracker observer; `sha256sum -c` and `Get-FileHash` test the digests the
observer declared. The bundle writer already reads its files back against the
buffer it wrote, which is the writer checking itself.

⛔ **And the announce carries `key=<run-id>`, a token the driver knows it
sent.** Every other check here is satisfied by a bundle of empty artifacts that
verify against their own empty digests. This is the only one that says a
client's bytes reached the record, and it is the same argument `OBS-09` makes
about reading a bundle back with the client that wrote it.

⚠ **What it captures is a fixture and the attestation says so in fields rather
than in prose**: `kind=fixture`, `measured_build=none`, `stock_client=false`.
Nothing is installed and no stock build is observed; `CLIENT-01` is what points
a real one at the same lab. A bundle that outlived its context would otherwise
read as a measurement of a client.

### ⛔ What driving the PowerShell half found, in one command

`[switch]$Marker` collided with the existing `$markerPath` local, which was
called `$marker`. PowerShell variable names are case-insensitive, so the
`param()` switch and the local were **one variable**: the script assigned a
string to a `SwitchParameter` and every single invocation of the guard, in every
mode, failed to bind. ⚠ It is the exact hazard
[`../docs/conventions/shell.md`](../docs/conventions/shell.md) section 8 records
about `$args`, and it was found by running the guard once rather than by reading
it.

### ⚠ And what the harness found about itself on its first run

`check-capture.sh`'s stub observer streamed the real observer's output through
`awk`, with `fflush()` after every line. **`fflush` is about the wrong end of the
pipe**: mawk reads its INPUT in blocks, so a second into a five-second run the
log held nothing, the runner dialled a port whose process had already gone, and
six cases reported the driver's refusal instead of their own. Measured against
`cat` in the same pipeline, which had three lines at the same instant. The
splitter is a `read` loop now.

⛔ **And a `no-segments` mode written into the wrong stub fired the wrong
guard.** The runner checks the segment count before the evidence, so removing
the evidence rows produced `described no evidence` and the segment guard stayed
untested. Two guards over one input with the earlier one masking the later,
inside the harness written to stop exactly that. It is a second, mid-stream stub
now, which substitutes as well as drops so that a missing `segments:` line and
`segments: 0` are separate cases.

### ⛔ What CI found that no local run could: a default that changed under us

⛔ **Run 58's Linux lane went red on `check-capture` while the Windows lane went
green, and the local gate had been green on both halves minutes earlier.** The
cause is `$PSNativeCommandUseErrorActionPreference`, which is `$false` in
PowerShell 7.4 and **`$true` from 7.5**. Under `$ErrorActionPreference = 'Stop'`
it turns a native command's non-zero exit into a terminating error, so
`capture-run.ps1` calling the guard and reading `$LASTEXITCODE` got an uncaught
exception every time the guard **refused**. ⚠ The two cases that went red are
exactly the two that assert a refusal.

⚠ **It is this repository's oldest rule broken by an upgrade rather than by an
edit.** Nothing in the tree changed; the runner image did. The Windows lane
stayed green because its `pwsh` is older, so two lanes disagreed about one
script for a reason neither of them printed.

⭐ **Sixteen `.ps1` files relied on that default and one carried a comment
stating it as a guarantee** - `check-control-bytes.ps1` said the preference "is
false by default from pwsh 7.4", which is a fact about one version read as a
promise. Every one of the sixteen sets it explicitly now, and `check-project`
refuses a `.ps1` that sets `$ErrorActionPreference = 'Stop'` without it. The
rule takes no judgement, so there is nothing to argue about per file.

Four cases over the new rule, both halves run on each and their `--json`
compared: the clean tree; the preference removed from one file, refused by both
naming it; a `.ps1` that stops on nothing, accepted by both because it is not
asked for a preference it has no use for; and the clean tree again. ⭐ And the
end-to-end half, which is the one that matters: with the preference forced
`$true`, `check-capture` reproduces run 58's two failures exactly; with it
`$false`, the harness is green. The fix is what carries it, not the version.

### ⛔ And a red gate that did not say what failed

⚠ **Run 58's log reported `FAIL check-capture (exit 1)` and then eleven
PASSING rows.** `check-gate` printed the first twelve lines of the failed
check's log, and a mutation harness prints dozens of passing rows before the one
that failed, so the CI log did not contain the failure at all and it had to be
reproduced locally to be seen. The excerpt is the **tail** now, in both halves,
and `store_report` reprints the failing rows immediately above its summary so
the tail lands on them.

### Acceptance, all run on 2026-09-08

- `sh scripts/capture/check-capture.sh`
- `sh scripts/acquisition/check-runner.sh`
- `pwsh -NoProfile -File scripts/acquisition/check-runner.ps1`
- `sh scripts/ci/check-workflow.sh`
- `sh scripts/common/check-gate.sh`
- `pwsh -NoProfile -File scripts/common/check-gate.ps1`
- `cargo test --workspace --locked --all-targets`

### Closure evidence, 2026-09-08

| what | measured |
| --- | --- |
| `sh scripts/capture/check-capture.sh` | 41 cases, 41 passed, 0 failed |
| `sh scripts/acquisition/check-runner.sh` | 13 guard cases, 13 passed, 0 failed |
| `pwsh -File scripts/acquisition/check-runner.ps1` | 15 guard cases, 15 passed, 0 failed |
| `sh scripts/ci/check-workflow.sh` | 58 cases, 58 passed, 0 failed |
| `sh scripts/common/check-gate.sh` | 26 checks, 25 passed, 0 failed, 1 skipped, 0 unavailable |
| `pwsh -File scripts/common/check-gate.ps1` | 26 checks, 12 passed, 0 failed, 1 skipped, 13 unavailable |
| `cargo test --workspace --locked --all-targets` | 50 binaries, 542 passed, 0 failed |
| `cargo clippy --workspace --locked --all-targets -- -D warnings` | clean |
| driven pass, sh | the guards refused in order on a real host: no claim, a claim naming another run, a table with a default route, an unbuilt observer, an output directory that already held a run. Then the capture ran: `curl` announced, the transcript carried its request including `User-Agent: curl/8.5.0`, `sha256sum -c` verified both artifacts, and the attestation was read back |
| driven pass, PowerShell | the same set through `capture-run.ps1` under `pwsh` on Linux, against a `Get-NetRoute`-shaped fixture table. ⛔ It failed to bind on the first attempt and that is the `$Marker` collision above |
| independent readers | `curl` 8.5.0 put the bytes on the wire, `sha256sum` verified the sh half's evidence and `Get-FileHash` the PowerShell half's. None of the three is this project's code |
| CI run 58, first attempt | ⛔ **Linux lane RED**, Windows lane green, over the PowerShell 7.5 default described above. The local gate had been green on both halves minutes earlier, which is the whole point of recording it |
| CI run 58, what it cost | one cycle, because the gate's failure excerpt was the first twelve lines of a harness that prints its failures last. Both are fixed |

### Guard mutation

Eleven plants into the two capture runners, one at a time, each verified to have
changed the file before it was judged: the claim-marker check, the claimed-run
comparison, the egress refusal, the existing-output refusal, the observer-present
refusal, the segment-count refusal, the zero-segment refusal, the digest
verification and the driver-token check in the `sh` half, and the claim and token
checks in the PowerShell one. ⛔ **Eleven refused, none survived, none failed to
apply, and in every case the harness row that went red was the one the plant was
aimed at** rather than some other row going red for its own reasons.

⚠ **The plant verifier reproduced this repository's own `grep -F` defect on its
first run.** Five of the eleven literals span two lines, `grep -o -F | wc -l`
counted each as 2, and all five were reported NOT-PLANTED over plants that would
have applied. `store-lib.sh`'s `replace_once` carries the same finding from
`CORPUS-01`, in a comment the harness author had read. Counted with something
that understands a multi-line literal, all eleven are unique.

Three more over the workflow, planted together rather than one at a time: an
added `pull_request` trigger, the build step moved after the route is cut, and
the capture step pointed at a script that is not in the tree. ⚠ **Together is
defensible here and would not be for the eleven above**: each is a different
property, read by a different reader, reported on its own line, so a refusal
stays attributable. The clean tree either side is the control.

### Residuals

- ⛔ **The workflow has never been dispatched, and that is the honest state.**
  Everything about it a reader can check is checked and both runners are driven
  for real on every gate, but three things only a run establishes: that
  `Get-NetRoute`'s real output matches the fixtures `check-runner.ps1` proves
  the guard against, that deleting and restoring a default route works on a
  hosted runner, and that the artifact survives the upload. ⚠ It is a residual
  and not a blocker: nothing prevents a dispatch.
- ⚠ **What it captures is a fixture.** No client is installed and no stock build
  is observed; the attestation says `kind=fixture`, `measured_build=none`,
  `stock_client=false`. `CLIENT-01` points a real build at the same lab.
- ⚠ **The restore step is the least proved thing in the file.** `ip route add`
  over a line `ip route show default` printed works on the hosts this was
  written against and is not exercised anywhere: `check-capture` runs against a
  route table it wrote itself and never touches the machine's. ⭐ Two things
  make that survivable. A failed add is **reported and does not end the step**,
  because under `set -e` the first refusal would skip the upload and throw away
  a finished capture over a route; and the inverted egress guard after it is the
  verdict, so an add is an attempt and the routing table is the fact. A guard
  that still passes there means the route never came back, which is a named
  failure rather than a network error naming nothing.
- ⚠ **Nothing pins the runner's PowerShell version, and nothing should.** The
  fix is that every `.ps1` states the behaviour it needs rather than inheriting
  it, which is version-independent. ⚠ What remains unproved is the rest of the
  7.5 surface: this found one default that changed by being bitten by it, and a
  second would be found the same way.
- ⚠ **The artifact pin is verified and the pairing is not.** The capture uploads
  with `actions/upload-artifact` v7.0.1 and the publisher downloads with
  `actions/download-artifact` v8.0.1. The pin was resolved and then re-read
  through a second route, `raw.githubusercontent.com`, which confirms the commit
  carries the *Upload a Build Artifact* action and accepts every input this
  workflow passes it. What no read establishes is that a v8 download reads a v7
  upload: neither workflow has ever run.
- ⚠ **Cancelling a dispatch between *Cut the route* and *Restore the route*
  spends the host.** The concurrency group does not cancel in flight, which stops
  a second dispatch doing it, and nothing stops a person pressing the button.
- ⚠ `capture-run` reads the observer's report from its stdout, so the observer's
  output contract is a shape held in two places. `check-capture`'s stub is what
  compares them, and it compares them against the runner rather than against a
  written specification.
- ⚠ The new BOM rule below has no fixture in the tree that exercises its
  ASCII-only branch, because every `.ps1` here carries markers. The mutation
  pass plants one; nothing holds it permanently, which is the same shape
  `check-twins.sh` warns about and the reason that pass found the drift in the
  first place.

### ⛔ What the door sweep found, one file away from this entry

Writing `capture-run.ps1` raised the question of whether a `.ps1` needs a UTF-8
BOM. [`../docs/conventions/shell.md`](../docs/conventions/shell.md) section 8
says one carrying non-ASCII does, because Windows PowerShell 5.1 decodes a
BOM-less file as the system ANSI code page. ⚠ **Eleven files had it and four did
not**, and every one of the four carries this project's markers on hundreds of
lines. Nothing was failing, because both CI lanes run `pwsh` 7 - and
`check-twins` still falls back to `powershell` when `pwsh` is absent, which is
where it would have bitten.

⭐ **A convention held in eleven places and broken in four is the one-gated-door
defect**, so the fix is both: the four files carry the BOM now, and
`check-project` refuses a `.ps1` that has non-ASCII and no BOM, in both halves.
The test is on the bytes rather than on a list, so an ASCII-only file is never
asked for a BOM it has no use for.

⛔ **The two halves disagreed on their first run and `check-twins` could not
have seen it.** A `.ps1` keeps CRLF, so the sh half's `[^ -~<tab>]` matched the
carriage return on every line and demanded a BOM for a file that is pure ASCII;
the PowerShell half excluded 9, 10 and 13 explicitly. ⚠ `check-twins` compares
the two on the tree it runs against, and **no `.ps1` in this tree is
ASCII-only**, so the branch that differed had nothing to exercise it. That blind
spot is written in `check-twins.sh` itself, and the fixture that found it is the
one the tree lacks.

Five cases, both halves run on each, exit codes and `--json` compared: the clean
tree; a BOM stripped from a file that carries markers, refused by both naming
that file; a new ASCII-only `.ps1` with no BOM, accepted by both; a new `.ps1`
with one marker and no BOM, refused by both naming it; and the clean tree again
after every restore. All five agree, character for character.

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
