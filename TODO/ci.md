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

### ⭐ The lane was thirty minutes and one step was twenty-six of them

**Measured on run 83, 2026-09-09, before anything was changed**, because "CI is
slow" is not a place to start optimising:

| step | of a 30.5-minute Linux lane |
| --- | ---: |
| *Workflow acceptance* | **25.9 min** |
| *Repository gate* | 3.5 min |
| everything else together | under 1 min |
| the whole Windows lane | 2.4 min |

⛔ **And the gate is not one gate.** `check-workflow` plants against it, so
several of its cases run the whole thing: **nine gate runs** inside that one
step. A second saved in the gate is nine seconds saved on the lane, which is why
the gate was measured next rather than the harness.

⚠ **The gate was three checks.** On this host its 29 checks took **198 seconds**
and 183 of them were `check-twins` at 90.7s, `check-capture-client` at 47.9s and
`check-capture` at 44.6s; everything else together was under fifteen. Serialising
twenty-six sub-second checks behind those three was the whole cost.

⭐ **Three changes, each measured, none of them weakening a verdict.**

| change | measured |
| --- | --- |
| the gate's checks run concurrently, verdicts read in list order | 198s to 100s |
| `check-twins` runs its twelve pairs concurrently, and each pair's two halves at once | 90.7s to **69s**, taking the gate to 73s |
| the two socket harnesses run after that batch rather than inside it | back up to **118-125s**, and correct under load |
| *Workflow acceptance* becomes a job of its own, beside the two lanes | it no longer serialises behind a gate it repeats |

⛔ **THE THIRD ROW IS A COST PAID ON PURPOSE AND IT IS MOST OF THE SAVING.** The
73-second gate was **wrong**: `check-capture` drives real sockets against a
three-second deadline, and under a batch of twenty-seven other checks it reported
*a run that recorded no bytes is refused (exit 1, but did not say 'recorded no
bytes at all')* - a refusal that arrived for a reason the case had not planted.
⚠ The same harness passed alone on the same tree minutes later, which is what
identifies it as load and not a defect. A faster gate that is red for no defect
is worse than a slow one.

⚠ **Raising the deadline was tried first, and measured, and rejected.** It looked
free - those harnesses wait on the observer's own line rather than on a delay -
and it is not, because several of their cases are ones the deadline itself has to
end: `check-capture` goes from **45 seconds at `3` to 79 at `6`**, and
`check-capture-client` from **48 at `5` to 168 at `20`**. That is roughly eight
to eleven seconds of gate per second of deadline, paid nine times over by
`check-workflow`, to buy back forty-seven.

⭐ **So the shipped gate is 118-125 seconds rather than 73**, measured twice at
each stage, and the reason it is not 73 is written where the schedule is.

### ⭐ What it did to the lane, measured on run 85

| | run 83, before | run 85, after |
| --- | ---: | ---: |
| **wall clock** | **30.5 min** | **21.3 min** |
| Linux gate | 30.5 min | **3.7 min** |
| Workflow acceptance | (inside the Linux gate, 25.9 min) | 21.2 min |
| Windows gate | 2.4 min | 2.6 min |

⭐ **The number a contributor feels is the first row of the lane, and it went from
thirty minutes to under four.** Compile, tests, lints and the gate now answer in
3.7 minutes; the acceptance harness answers separately and no longer holds them.

⛔ **And the wall is now that harness alone**, which is where the next work is
and why the residual below names sharding rather than anything inside the gate.
⚠ Both fast jobs were green while it was still running, which is the split doing
what it was written for rather than a claim about it.

⛔ **Every exit code is still read from the process that produced it.** `wait
"$pid"` returns that child's status and nothing else's, which is the same
guarantee the serial form had; there is no pipeline anywhere in either change,
because a pipeline's status is this repository's oldest refusal.

⛔ **And the rows are still in list order.** Each check and each pair writes to a
file of its own and its row is assembled at its own index, so a report cannot
come out in the order things happened to finish - which would make two runs over
one tree produce two different reports.

⭐ **The concurrency is safe because every check is hermetic**, which each one's
own header already claimed: each makes its own scratch directory, binds only
loopback, and reads the tree without writing to it. ⚠ `tree-unchanged` is the row
that keeps that claim honest, and it stayed green through the change.

⭐ **One thing had to be added rather than only reordered.** Most of the checks
call `cargo build --example` and cargo locks the target directory, so started at
once they queued behind each other and the concurrency bought nothing. The
examples are built once, before the queue - the same work, done once instead of
nine times. ⚠ Its failure is deliberately not a gate row: a build that fails
there fails again inside whichever harness needed it, where it is reported with
that check's own name.

⚠ **What was NOT changed is what `check-workflow` runs.** It executes the
workflow's own step commands, read out of `ci.yml` by job and step name, so
passing `--fast` to make its nine inner gate runs cheaper would mean changing
what the lane itself runs. The saving comes from the gate being faster, not from
it doing less.

### ⭐ A rule this repository said it had and did not, closed 2026-09-10

⛔ **`check-project` pinned the target set with a HARDCODED LIST of seventeen
ids**, asking that each appear in `catalogue/clients.toml` and in
`docs/client-matrix.md` - while that document claimed the set was "pinned by
`check-project` against the catalogue in both directions". ⚠ A target added to
the catalogue and forgotten in the matrix was caught by nothing, in either half,
and so was a matrix row naming a target the catalogue had dropped. The claim had
already been withdrawn in place on 2026-09-09; nothing had made it true.

⭐ **Both halves derive the set from the catalogue now and compare it against the
ids in the matrix's own rows, refusing a difference either way.** ⛔ And refusing
a set too small to be real, which is the guard `ACQ-01`'s catalogue scan already
carries: two empty sets agree perfectly, so a parser that stopped matching would
report a pinned matrix over nothing at all.

⚠ **The list is deleted rather than extended.** A list is a value in two places,
and this file already records three counts that went stale in prose for exactly
that reason; extending it only resets the clock.

Guard mutation, three plants, both halves run on each and their output compared:
a target added to the catalogue alone, a matrix row the catalogue does not carry,
and a matrix id renamed away from the catalogue - which fires BOTH directions at
once and is the case that says the two comparisons are separate. All three
refused, the twins agreeing character for character, and the clean tree accepted.

### Residuals

- ⚠ `check-workflow.sh` is not in `check-gate.sh` and cannot be, so a
  contributor's local gate does not run it. The workflow runs it on every push
  and it is this entry's acceptance; a gate runner that listed it would re-enter
  itself.
- ⚠ The concurrency is unbounded: every check is launched at once. On a
  four-processor host that is why the gate stops at 73 seconds rather than at the
  90 one check needs - the machine is saturated, not the ordering. A bound would
  be worth having on a host that cannot afford twenty-nine processes, and nothing
  here has met one.
- ⚠ `check-gate.ps1` is still serial. The Windows lane is 2.4 minutes end to end
  and its gate is 59 seconds, so there is nothing there to win; the two halves
  now differ in how they schedule and not in what they answer, and
  `check-twins` does not pair the runners.
- ⚠ **`check-workflow` is now the lane, and the next win is sharding it across
  runners rather than anything inside it.** Measured after the change: 764
  seconds locally, of which nine gate runs at roughly 70 seconds each are about
  ten minutes. ⛔ Those nine cannot be overlapped within one job - the gate
  already saturates four processors, which is why it stops at 73 seconds rather
  than the 69 its longest check needs - so the only way down is a shard per
  runner.
  ⛔ **It is deliberately not done here, and the reason is the failure mode.** A
  shard selector that silently claims no case leaves a rule nobody runs while
  every lane stays green, which is the exact defect this entry's own `--strict`
  work exists to prevent. It needs the partition to be provable by construction -
  `index mod N`, with the runner printing what it ran and skipped - and the
  workflow to be checked for running all `N`. That is a change to the harness
  every other guard is proved by, and it should be made as its own unit with its
  own plants rather than alongside a speedup.
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
- ⛔ **A row this gate was documented as having did not exist.**
  [`../docs/conventions/shell.md`](../docs/conventions/shell.md) section 5 says
  the repository compares every tracked file's working-tree line endings against
  what `.gitattributes` resolves for it, naming `git ls-files --eol` as the way.
  Nothing did, until 2026-09-08. ⚠ Found by breaking it rather than by reading
  it: a file-writing tool that emits LF left `check-project.ps1` at `w/lf` under
  `attr/text eol=crlf`, a full gate passed over that tree, and the only thing
  that reported it was an incidental warning from `git commit`. ⭐ The check is a
  row in both halves now, planted in both directions - LF where CRLF is required
  and CRLF where LF is - because they are different branches, and `git diff`
  prints nothing for either.

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

### ⛔ And then run 59, which the fixed reporting caught in one line

⛔ **A refusal message that a harness matches on must not go through a display
layer.** `capture-run.ps1` reported through `Write-Error`, which inside a script
renders a source-context block and **wraps the message to the host's width**.
Measured on 2026-09-08: the refusal `the host is claimed by run [capture-0001],
not [capture-0002]` is one line at width 200 and **two** at width 80, so a
fixed-string match succeeds on a developer host and fails on a CI runner. The
case went red with the right exit code and the wrong message.

⚠ **Ten `.ps1` files already wrote through `[Console]::Error.WriteLine` and five
did not**, which is the BOM shape a third time: a convention held on most of the
paths into one mistake. All five write the bytes now, and `check-project`
refuses `Write-Error` in a tracked `.ps1`, in both halves.

⭐ **The rule's needle is an INVOCATION, not the word, and its first run proved
why: it fired on its own PowerShell twin**, because the failure message that
rule raises contains the name. A rule has to be describable in the file that
enforces it, so a match is the name in command position and a comment line is
skipped outright. ⚠ That is the needle-list lesson in a new shape - a needle
that matches its own description.

Five cases, both halves on each with `--json` compared: the clean tree; an
invocation at a statement start; an invocation after a pipe, which is the shape
`assert-disposable.ps1` used; the name in a comment and inside a string,
accepted by both; and the clean tree again. ⭐ And the end-to-end half at the
width that broke it: with `[Console]::Error.WriteLine` the fixed-string match is
1, with `Write-Error` it is 0.

⭐ **Run 59 is also what shows the reporting fix works.** Its log named the
failing case on its own line - `❌ ps a host claimed by another run is refused` -
where run 58's contained no failure at all.

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
| CI run 59, first attempt | ⛔ **Linux lane RED again**, over `Write-Error` wrapping a refusal message at the runner's console width. ⭐ Its log named the failing case in one line, which is the reporting fix working |
| CI run 60 | ⭐ **both lanes green**, first attempt after both fixes, on `0a37c2a` |

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

One more over the gate's own wiring: `check-capture.sh` moved aside and the gate
run with `--strict`. It reports `SKIP check-capture (not present)` and exits 1,
so a harness that vanished cannot read as a pass. ⭐ **And a second, independent
door fired at the same time**: `check-docs` refused `scripts/README.md`'s link to
the missing file. Neither was written to catch the other, which is what makes the
pair worth recording.

Three more over the workflow, planted together rather than one at a time: an
added `pull_request` trigger, the build step moved after the route is cut, and
the capture step pointed at a script that is not in the tree. ⚠ **Together is
defensible here and would not be for the eleven above**: each is a different
property, read by a different reader, reported on its own line, so a refusal
stays attributable. The clean tree either side is the control.

### Residuals

- ⭐ **Closed by `CI-06`, which dispatched it.** The three things only a run
  establishes were all answered, and the run also found a defect in this file's
  own Windows restore step that no reading here had: the inverted egress guard's
  refusal was left in `$LASTEXITCODE` and GitHub reads that as the step's
  verdict. ⚠ Everything this entry says about the workflow's *shape* still
  stands; what it could not say is what a runner does with it.
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

## CI-06: The first dispatched capture run

Source: `CI-03`'s residual, which is a workflow that has never been dispatched
Priority: P1 | Effort: L | Status: DONE

Problem: `.github/workflows/capture.yml` is checked in every way a reader can
check it and has never run. Three facts only a dispatch establishes: that
`Get-NetRoute`'s real output matches the fixtures `check-runner.ps1` proves the
Windows guard against, that deleting and restoring a default route works on a
hosted runner of each platform, and that the evidence bundle survives the
upload.

Approach: Dispatch both jobs, read the run back, download the artifact, verify
it against its own `SHA256SUMS` with a reader this project did not write, and
compare each job's reported fingerprint against the next run's. Record what the
route save and restore actually printed on each platform.

Prove: `sh scripts/capture/check-capture.sh` still passes, both dispatched jobs
end green, each uploaded bundle verifies under `sha256sum -c`, the two runs of
one platform report different fingerprints, and the attestation carried in each
artifact names `kind=fixture`.

### ⭐ Capture run 1, and the thing no reading had caught

The first dispatch was on `b992a35`, the commit `CI-03` closed at. ⛔ **The Linux
job was green end to end on the first attempt and the Windows job failed**, and
the failure is worth more than the success.

Three facts stood unmeasured and the dispatch answered all three:

| fact | what the run said |
| --- | --- |
| `Get-NetRoute`'s real output matches the fixtures `check-runner.ps1` proves the guard against | ⭐ it does. *Assert containment* ran the guard with no `-RouteTable` on a real `windows-2025` host and printed `no route off this host (read Get-NetRoute)`, which is the guard reading the live cmdlet and agreeing with the fixture corpus |
| a hosted runner's default route can be deleted and put back | ⭐ deleted on both platforms; restored on both. ⛔ The Windows *step* nevertheless failed, for a reason that is not the route |
| the evidence bundle survives the upload | ⭐ on Linux, yes. On Windows the upload never ran, because the step before it failed |

### ⛔ The defect: a restore that worked, and a step that failed on it

`Restore the route` ends by running the egress guard **inverted**. A host that
can reach the network again is one the guard REFUSES, so a refusal there is the
proof the route came back and a pass would mean it had not. The Windows job's
guard printed `assert-disposable: a public route exists; a capture host reaches
loopback only` - its refusal, exit 1, exactly as designed - and the step then
failed with `Process completed with exit code 1`.

⛔ **Nothing in the block was wrong. What failed was what the block left
behind.** GitHub's `pwsh` wrapper reads the residual `$LASTEXITCODE` as the
step's verdict, so a guard succeeding at its job set the code that failed the
step, the upload was skipped as a consequence, and a finished capture was thrown
away over a route that had already come back. ⚠ The `Write-Error` branch never
ran: the log carries no such line, which is how the two are told apart.

⭐ **The Linux twin never had this**, because `if sh …; then` consumes the
status and an `if` whose condition is false is a compound command with status 0.
⛔ **One of two paths into one mistake**, which is the shape
[`../docs/methodology/reviews.md`](../docs/methodology/reviews.md) calls the most
recurring hole there is - and it was invisible to every check in the repository
because the wrapper that reads the code is GitHub's, not this project's.

Both restore steps now end in an explicit `exit`, so the step's status is a
decision rather than a residue.

### ⛔ And the same guard's three codes were folded into two, on the green half

Reading the fix exposed a second defect in the half that had passed. The Linux
step asked `if …; then route-not-restored; fi`, which treats the guard's **2**
the same as its **1**. Those are `could not run` and `refuses`, and
[`../docs/history/RESUME.md`](../docs/history/RESUME.md) has carried that
distinction since a review pass counted one as the other. A guard that could not
run says nothing about the routing table, and uploading on it is uploading from a
host nothing established was reachable. Both halves now read all three codes and
name which one they got.

⚠ **This one was never going to fail a run.** `--egress` answers 2 only when it
cannot read a routing table at all, which has not happened on a hosted runner. It
is a latent conflation found by making the two halves symmetrical, and that is
the argument for making them symmetrical.

### ⛔ What the door sweep found: the same language behind a second door

`check-project` carries three rules over PowerShell - a UTF-8 BOM, the native
command preference beside `$ErrorActionPreference = 'Stop'`, and no `Write-Error`
where a machine reads the message. ⛔ **All three iterate `git ls-files '*.ps1'`
and none of them had ever reached a `pwsh` block inside a workflow**, which is
the same language, on the same runners, with the same two hazards.

⚠ **And `capture.yml` was carrying a live instance of each**: the `Write-Error`
above, and three blocks setting `Stop` with nothing saying what a native exit
code means. The rule that turned CI red twice last session had a whole second
door open the entire time.

⭐ The two rules now read the `pwsh` blocks of every workflow as well, and a
third rule exists that has no `.ps1` counterpart: **a block that reads
`$LASTEXITCODE` ends in an explicit `exit`.** That one is workflow-scoped on
purpose - a `.ps1`'s own exit code is its author's to choose, and it is only in a
workflow step that a harness reads whatever was left over.

⚠ The rules enumerate `git ls-files --others --exclude-standard` beside the
tracked set, as the `.ps1` rules already did. An uncommitted new workflow is part
of the tree the next push carries, and it is also how `check-workflow.sh` plants
against these three rules.

### ⭐ What the driven pass measured that the dispatch could not

⛔ **Nothing in this repository ran a capture step's body**, which is why the two
restore blocks were driven by hand here: each lifted verbatim and run against a
stub guard exiting 0, 1 and 2 in turn, with the exit code read from the process
that produced it. `check-workflow.sh` reads `capture.yml`'s step names, their
order and the *Capture* step's command; it executes steps out of `ci.yml` alone.
⭐ `CI-08`'s `check-step-bodies.sh` runs them as a gate row since 2026-09-09, so
what this paragraph describes is how it was done before there was a harness.

⛔ **The `pwsh` half has to be run the way GitHub runs it or the defect does not
reproduce.** The wrapper prepends `$ErrorActionPreference = 'stop'` and
**appends** `if ((Test-Path -LiteralPath variable:/LASTEXITCODE)) { exit
$LASTEXITCODE }`, then invokes `pwsh -command ". '<file>'"`. That append is the
entire mechanism. ⚠ A first version of the driver omitted it and reported the
broken block **passing**, which would have been a harness saying the defect was
not there over the block that had just failed on a runner.

⭐ With the wrapper faithful: the block as it stood on capture run 1 exits **1**
over a guard that refuses, which is run 1's failure reproduced on this machine;
the block as it stands now exits **0** over the same input, and **1** over a
guard that passes or one that could not run. The `sh` half answers identically
on all three.

⛔ **And a fact about the wrapper worth pinning: a dot-sourced script's exit code
collapses.** Measured across 0, 1, 2, 3 and 42 on pwsh 7.4.6: through
`-command ". '<file>'"` every non-zero value arrives as **1**, while `-File`
preserves it exactly. ⚠ So a `pwsh` step's exit code says *failed* and never says
which refusal fired - which is a second, independent argument for
`[Console]::Error.WriteLine`, because the message is the only channel that
survives. [`../docs/conventions/shell.md`](../docs/conventions/shell.md) section
8 carries the table.

### Guard mutation over the three new rules

Both halves of `check-project` were run on every case and their **exit codes and
their whole output** compared, because `check-twins` compares the pair on the
tree it runs against and a rule that differs only on a defect this tree does not
contain is invisible to it.

Fourteen cases: the tree as it stands; a block setting `Stop` with no native
preference; `Write-Error` at a statement start; `Write-Error` after a pipe, which
is the shape `assert-disposable.ps1` itself once used; the residual exit code
verbatim; every needle in comment position; both needles in a step whose shell is
not `pwsh`; a block that stops on nothing, asked for no preference it has no use
for; a single-line `run:` with no block scalar; a block that reads
`$LASTEXITCODE` and ends on an explicit `exit`; the three scope plants below; and
the tree again after every restore. ⭐ **All fourteen landed on the intended
verdict and the two halves agreed on all fourteen, character for character.**
Each refusal was also checked to be named by the rule under test rather than by
some other rule going red.

⭐ The five acceptances are the half that matters most here. A rule that refused
a needle in a comment, or one in a `bash` step, is a rule somebody switches off.

### ⛔ What the door sweep found in the new rules themselves

The first version matched `shell: pwsh` in `.github/workflows/*.yml` and nothing
else, which is the defect this entry is about, in the code written to catch it.
`shell: powershell` is Windows PowerShell 5.1 rather than a different language; a
composite action under `.github/actions/` carries its own steps and the same
permissions, which is the argument `check-project`'s **action-pin rule already
makes in a comment** two hundred lines above; and a workflow may be `.yaml`.
⚠ **None of the three exists in this tree**, which is precisely when a scope is
easiest to get wrong: every reading agrees with the correct rule on every file
here. The scope is the pin rule's now, and it is proved by planting each of the
three fixtures the tree lacks.

### ⛔ And what the per-plant comparison found that a clean tree could not

Widening the scope broke the PowerShell half outright, and **both halves went on
agreeing perfectly on the clean tree** while it was broken. `Resolve-Path
-Relative` prepends `./` to most paths and **not** to one that already begins
with a dot, so a fixed `Substring(2)` ate the `.g` of `.github`, every file then
failed its `Test-Path`, and that half examined nothing at all. A file set only
appears in the output when something in it fails, so the clean tree could not
show it. ⭐ **Seven of the fourteen cases went red at once and named it**, which
is the whole argument for comparing the twins per planted mutation rather than on
a clean tree - the argument `check-twins.sh` makes about itself and `CI-07`
carries. The root is stripped now rather than a prefix assumed.

⚠ **The extractor was checked against what it hunts for**, because a reader
answering "clean" over an empty set passes on any tree: it finds every `pwsh`
step in the tree, which is the same set `shell: pwsh` appears on.

### ⛔ What the claim audit found: this entry's own Prove, refuted on one platform

The Prove asks that *the two runs of one platform report different
fingerprints*, which is how a host that was not destroyed would be caught.
Measured across the two dispatches:

| platform | run 1 | run 2 | same? |
| --- | --- | --- | --- |
| Linux | `051ae9dd…` | `49f8714f…` | ⭐ different, as the Prove asks |
| Windows | `c5cde27f…` | `c5cde27f…` | ⛔ **identical** |

⛔ **And both Windows hosts were fresh.** Each run's *Claim the host* step
succeeded, and that step writes an exclusively created marker under
`ProgramData` and refuses if one is already there. Run 1 wrote one; run 2 found
none. So the comparison the Prove asks for reports "a survived host" over two
hosts that demonstrably were not.

⭐ **That is structural, and recording it as a defect to patch would be the
error.** A fingerprint has to differ between two hosts *and* survive a reboot
within one, so that a machine which rebooted rather than being destroyed still
reads as itself. Every input with the second property is a property of the
**image**, and hosted runners are clones of one image. ⚠ Adding a per-boot value
would buy the first requirement by destroying the second, which is the failure
the guard exists to catch.

⛔ **The claim marker is what actually answers the question, and always was.** It
detects a survived host by finding its own marker rather than by comparing a
value against a previous run's, which is the difference
[`../docs/capture-host.md`](../docs/capture-host.md) draws between detecting a
failure and trusting a claim. The workflow's Windows step summary said "A repeat
means the host was not destroyed" and now says what the value is instead.

⚠ **What is not established is WHICH of the guard's inputs is constant across
clones.** The digest is over the machine GUID, the install date, the build label,
the computer name, the OS version and the user name; at least one of the varying
candidates does not vary, and nothing here says which. Settling it needs a mode
that prints a digest per part and two more dispatches. ⚠ **The alternative was
considered and rejected**: it widens `CI-03`'s mutation-proven guard for an
answer that changes nothing, because the fix is the same whichever input it is.
It is a residual below.

### Acceptance, all run on 2026-09-08

- `sh scripts/capture/check-capture.sh`
- `sh scripts/ci/check-workflow.sh`
- `sh scripts/common/check-gate.sh`
- `pwsh -NoProfile -File scripts/common/check-gate.ps1`
- `cargo test --workspace --locked --all-targets`
- `cargo clippy --workspace --locked --all-targets -- -D warnings`

### Closure evidence, 2026-09-08

| what | measured |
| --- | --- |
| capture run 1, first dispatch ever | ⛔ Linux green, **Windows red on *Restore the route*** over a restore that had worked |
| capture run 2, after the fix | ⭐ **both jobs green**, every step of both, first attempt |
| `Get-NetRoute` against the fixtures | ⭐ the Windows guard ran with no `-RouteTable` on a real host and answered `no route off this host (read Get-NetRoute)`, in both runs |
| the default route, deleted and put back | ⭐ both platforms, both runs. The inverted guard is the verdict: it refused after the restore, which is a routing table with a public route on it |
| the evidence bundle through the upload | ⭐ every bundle survived: `sha256sum -c` exits 0 over each, naming the metainfo and the transcript |
| the attestation | ⭐ `kind=fixture`, `measured_build=none`, `stock_client=false` in every artifact |
| what put the bytes on the wire | `curl` 8.5.0 on the Linux runner and `curl` 8.16.0 with Schannel on the Windows one. The transcript carries the announce, its `User-Agent`, and `key=<run-id>`, the token the driver knows it sent |
| the fingerprint comparison | ⭐ different on Linux; ⛔ identical on Windows, over two hosts whose claims both succeeded. The paragraph above is the finding |
| `sh scripts/ci/check-workflow.sh` | 63 cases, 63 passed, 0 failed |
| `sh scripts/common/check-gate.sh` | 26 checks, 25 passed, 0 failed, 1 skipped, 0 unavailable |
| `pwsh -File scripts/common/check-gate.ps1` | 26 checks, 12 passed, 0 failed, 1 skipped, 13 unavailable |
| `cargo test --workspace --locked --all-targets` | 50 binaries, 542 passed, 0 failed |
| guard mutation, the three new rules | 14 cases, both halves on each, exit codes and whole output compared; all 14 on the intended verdict and the halves identical on all 14. ⛔ Seven of them went red first and caught a break in the PowerShell half that the clean tree agreed with perfectly |
| driven pass, the restore blocks | 13 cases. Run 1's failure reproduced on this machine, and the fixed block exits 0 over the same input |
| CI run 62 | ⭐ both lanes green on `1938672` |

⭐ **The strongest control here is not this project's code.** `sha256sum -c` and
`Get-FileHash` verified the evidence, `curl` put the bytes on the wire from both
runner images, and the run that decided whether the fix works is GitHub's, on
hosts this session cannot reach.

### Residuals

- ⚠ **Which of the Windows fingerprint's inputs is constant across clones is not
  established.** A `-FingerprintParts` mode printing a digest per part would
  settle it in two dispatches; it was rejected for now because it widens
  `CI-03`'s mutation-proven guard for an answer that changes no decision. The
  finding that matters - that the comparison is not a freshness signal there - is
  measured and recorded.
- ⭐ **Closed on 2026-09-09.** This residual said nothing in the tree runs a
  capture step's body, and named `CI-08` as carrying the harness that would
  close it. `scripts/ci/check-step-bodies.sh` is that harness: it runs both
  restore blocks under GitHub's own wrapper form against stubbed guards, and it
  refuses the Windows block in the shape that failed capture run 1. ⚠ What
  remains true is the sentence's premise about `check-workflow.sh`, which still
  reads `capture.yml` statically and executes steps out of `ci.yml` alone.
- ⚠ **The `sh` restore step's three-code reading is not driven on a runner.**
  `--egress` answers 2 only when it cannot read a routing table at all, which has
  not happened on a hosted runner; the branch is driven against a stub here and
  by nothing on the real path.
- ⚠ **The upload/download pairing is still unexercised.** The capture uploads
  with `actions/upload-artifact` v7 and the publisher downloads with
  `actions/download-artifact` v8. `CI-09` owns it; what these two runs establish
  is only that a v7 upload survives and can be fetched over the REST route.
- ⚠ **What was captured is a fixture and every attestation says so.** No client
  was installed, no stock build was observed, and nothing was published.
  `CLIENT-01` points a real build at the same lab.
- ⛔ **Reading the run back is what found the last one.** The five new cases each
  run the whole gate, and the Linux lane went from 13.4 minutes on run 61 to
  **24.4 of the 30 it then had** on run 62, 20.4 of them inside *Workflow
  acceptance*. It passed, with under six minutes to spare, and a lane that runs
  out of time reports as infrastructure rather than as a defect. The budget is 45
  now and the measurement is in the workflow beside it. ⚠ The Windows lane is
  unaffected at 2.6 minutes: it does not run this harness.

## CI-07: PowerShell halves for the declared gate rows

Source: every `n/a` row on the Windows lane, each naming a missing half
Priority: P1 | Effort: L | Status: OPEN

Problem: The Windows gate declares every mutation harness unavailable except
`check-runner`, whose PowerShell half exists. Each of the rest is an `sh`
harness with no PowerShell implementation, so the guards they prove are proved
on one platform and asserted on the other. ⛔ `--strict`
permits a declared row forever, which is correct and is also why the gap does
not shrink by itself.

⚠ **This Source said `thirteen` and the runner declares fourteen.** Found by a
claim audit on 2026-09-08, and it is the SECOND time a count of these rows has
gone stale in prose: `CI-01` records the first, where six had become twelve, and
its own fix says in as many words that no count of them is written into prose
anywhere. One had survived here. The number is gone rather than corrected,
because correcting it only resets the clock.

Approach: Write the missing halves, starting with the ones whose subject is not
platform-specific at all. ⚠ `check-store` plants a symbolic link and a named
pipe, which an unprivileged Windows session cannot create; that one needs its
plant set reconsidered rather than translated, and a half that silently skipped
two plants would report a smaller pass under the same name.

### ⭐ The first step is ONE library, not fifteen harnesses. Found 2026-09-09

⛔ **Every declared-unavailable harness sources
[`../scripts/corpus/store-lib.sh`](../scripts/corpus/store-lib.sh), and there is
no PowerShell twin of it.** `check-cache`, `check-corpus`, `check-store`,
`check-indexes`, `check-release`, `check-formats`, `check-publish`,
`check-access`, `check-catalogue`, `check-examples`, `check-handbook`,
`check-staleness`, `check-release-route`, `check-capture-client` and
`check-step-bodies` all begin the same way. ⭐ Measured:
`find scripts -name '*lib*.ps1'` returns nothing, while twelve `check-*.ps1`
files exist - every one of them a harness that needs no shared library.

⭐ **So this entry is one library and then thin twins, rather than fifteen
translations.** `store-lib.sh` is 321 lines and twelve functions:
`store_require`, `store_build`, `store_workdir`, `row`, `fail`, `pass`, `place`,
`tree_digest`, `tree_files`, `replace_once`, `store_probe_guards` and
`store_report`. ⚠ Four of those are the mutation-probe machinery, and
`check-twins` compares the two halves' answers per planted mutation - so a
`store-lib.ps1` whose `replace_once` or `store_probe_guards` differed in
semantics would make every twin that used it disagree at once. That is an
argument for writing the library carefully first, and against porting a harness
by inlining the few helpers it happens to need.

⚠ **Not every declared row is a missing half.** `check-examples` runs the ```sh
fenced blocks of `docs/consuming.md` as a reader would; there is no `sh` on a
Windows runner, so its `n/a` is a fact about the platform rather than work
nobody has done. ⛔ Counting all eighteen declared rows as this entry's backlog
overstates it, and the sweep should say which are which before any are written.

Prove: `pwsh -NoProfile -File scripts/common/check-gate.ps1 -Strict` passes with
fewer declared rows than it has today, each new half is mutation-proven against
the same plants as its twin, and `check-twins` compares the pair per planted
mutation rather than on a clean tree.

### ⭐ THE SWEEP, done 2026-09-09: the declared rows are FOUR kinds, not one

⛔ **This entry's backlog was every declared row, and that overstates it.** The
Approach already said `check-store` needs its plants reconsidered rather than
translated, and the paragraph below it said `check-examples` is a platform fact.
Nothing had gone through the rest. ⚠ Measured by reading what each harness
actually drives - the Rust examples it builds and the `sh` scripts it runs -
rather than by reading the reason each row declares, because those reasons were
written one at a time.

**A. The harness is the only missing half, and `store-lib.ps1` is most of it.**
Each drives Rust examples this project builds on both lanes, plus the shared
library. Nothing in the subject is `sh`.

| row | what it drives |
| --- | --- |
| `check-cache` | `cache-scenario`, and `check-licences`, which HAS a PowerShell twin |
| `check-corpus` | `build-store`, `validate-corpus`, `check-store` |
| `check-indexes` | `build-store`, `build-indexes` |
| `check-release` | `build-store`, `build-indexes`, `assemble-release` |
| `check-formats` | `build-store`, `build-formats`, `assemble-release` |
| `check-catalogue` | the four above plus `catalogue-lookup` |
| `check-staleness` | `build-store`, `resolve-stable`, `survey-staleness` |
| `check-assemble` | `assemble-capture` |

⭐ **So the library is the first step and it is not a translation of fifteen
harnesses.** `store-lib.sh` is twelve functions, four of them the mutation-probe
machinery, and `check-twins` compares the two halves' answers per planted
mutation - so a `store-lib.ps1` whose `replace_once` or `store_probe_guards`
differed in semantics would make every twin that used it disagree at once.

**B. The plants need reconsidering, not translating.** `check-store` plants a
symbolic link and a named pipe, which an unprivileged Windows session cannot
create. ⚠ A half that silently skipped two plants would report a smaller pass
under the same name.

**C. The SUBJECT is `sh` and has no PowerShell half, so the harness cannot
precede it.** Writing the harness first would be a twin proving a script that
does not exist.

| row | the subject it has no half of |
| --- | --- |
| `check-publish`, `check-access` | `publish-data.sh` |
| `check-capture-client` | `capture-client.sh` |
| `check-release-route` | `resolve-release.sh` |
| `check-source-route` | `resolve-source.sh` |

**D. A fact about the platform rather than work nobody has done.** ⛔ These
close on nothing this entry can write.

| row | why |
| --- | --- |
| `check-examples`, `check-handbook` | they RUN the ```sh fenced blocks of a document, and there is no `sh` on a Windows runner |
| `check-gate-rows` | it runs `check-gate.sh --rows`, which is the `sh` runner itself |
| `check-step-bodies` | it lifts and runs a Linux-only workflow's step bodies; the `pwsh` half of its subject is what it already runs |
| `check-capture` | its subject's PowerShell half exists and the capture workflow exercises it |

⚠ **No count of any of these is written here**, for the reason this entry
already records twice: a number in prose is a value in two places with nothing
comparing them, and both previous counts went stale. `check-gate.ps1` declares
each row with its own reason and `check-gate-rows` compares the two lists.

⭐ **`CI-06` built and ran the last clause of that Prove and did not keep it.**
Its three new rules were proved by planting into a scratch workflow, running both
halves of `check-project` on each plant, and comparing their exit codes and their
whole output: eleven cases, four refusals, six acceptances and a control either
side, all agreeing character for character. ⚠ It lived in a session scratch
directory and is therefore evidence rather than a control, which is exactly the
gap this entry names. ⛔ **And it is the cheap half of what `CI-06` had to do
instead**: `check-workflow.sh` proves the same three rules by running the whole
gate per plant, which cost the Linux lane eleven minutes. A permanent per-plant
twin harness would let that shrink back to one gate-level case.

### ⭐ THE LIBRARY EXISTS AND THE FIRST CLASS-A ROW IS REAL. 2026-09-10

⭐ **[`../scripts/corpus/store-lib.ps1`](../scripts/corpus/store-lib.ps1) is the
first step this entry named**, and
[`../scripts/acquisition/check-cache.ps1`](../scripts/acquisition/check-cache.ps1)
is the first harness twin that proves it. `check-gate.ps1` runs `check-cache` as
a row rather than declaring it: measured on 2026-09-10, that lane went from 13
passed and 20 unavailable to **14 passed and 19 unavailable**, over 34 rows both
runners agree on.

⛔ **THE TWO HALVES CANNOT DISAGREE ABOUT THE CACHE AND THAT IS NOT THE POINT.**
Both drive the same Rust example, so the subject is identical by construction;
what they can differ on is the machinery underneath - the plant probes, the row
accounting and the verdict - which is exactly what the library supplies.
⚠ Their `--json` answers are byte-identical on a clean tree, and a clean tree
proves nothing about a pair, so three defects were planted in the PowerShell
library one at a time and the two halves compared on each:

| plant in `store-lib.ps1` | the halves |
| --- | --- |
| a multi-line literal accepted rather than refused | ⛔ disagree, caught |
| an ambiguous literal accepted rather than refused | ⛔ disagree, caught |
| a no-op edit reported as planted | ⛔ disagree, caught |

⛔ **AND `check-twins` COULD ONLY REACH `common/`.** Every pair it compared lived
there, so the directory was spelled once in `compare_pair` and each call site
named a bare file - and the first class-A twin is in `acquisition/`. A comparison
that could reach one directory would have left it uncompared, which is the shape
that whole file exists to refuse, arriving in its own plumbing. The paths are
relative to `scripts/` now and all twelve call sites carry their directory.

⛔ **THREE FUNCTIONS ARE DELIBERATELY ABSENT FROM THE LIBRARY.** `place`,
`tree_digest` and `tree_files` are used only by `check-store`, `check-corpus` and
`check-indexes`, and `check-store` is class B. ⚠ A function nothing calls is a
function nobody knows works, and shipping three of those would make the library
look more complete than it is measured to be; they land with the first twin that
exercises them.

### ⭐ AND A SECOND CLASS-A ROW, WHICH NEEDED NO NEW LIBRARY FUNCTION

⭐ **`check-catalogue` is a row on both lanes now**, and that is the sweep's
finding measured a second time: `check-cache` proved the library, and this
harness uses the same four calls with nothing added. That lane reports **15
passed and 19 unavailable** over 35 rows both runners agree on.

⛔ **WHERE THIS PAIR REALLY DIFFERS IS THE HARNESS, NOT THE SUBJECT.** Both
halves drive the same five Rust examples over the same fixture publication, so
the answers are identical by construction; what differs is how each PLANTS a
defect and restores the publication afterwards. A half that restored it
differently would report the same eighteen cases over a different experiment.

#### ⛔ AND THE LIMIT OF THE TWIN COMPARISON, MEASURED RATHER THAN ASSUMED

⛔ **`check-twins` COMPARES A VERDICT, SO IT CANNOT SEE A WEAKENED ASSERTION.**
Four defects were planted in the PowerShell half one at a time:

| plant | the pair |
| --- | --- |
| a plant's restore is removed | ⛔ disagree, caught |
| a case expects the wrong error code | ⛔ disagree, caught |
| a case is dropped from one half | ⛔ disagree, caught |
| a passing case's assertion is replaced by `$true` | ⭐ **agree, survived** |
| a second passing case's assertion is replaced by `$true` | ⭐ **agree, survived** |

⚠ **The last two are the comparison working as specified, not a defect in it.**
A case that already passed still passes, so the counts do not move and the JSON
is identical. ⛔ **So a twin comparison proves the two halves reach the same
VERDICTS and says nothing about whether either reached them for a reason.** That
is the same blind spot this entry already records from the other side - a rule
differing only on a defect the tree does not contain is invisible to it - and it
is why a new half is mutation-proved against its own subject as well as compared
against its twin.

⚠ **WHAT IS LEFT IS THE REST OF CLASS A**, which the table above this section
lists: `check-corpus`, `check-indexes`, `check-release`, `check-formats`,
`check-staleness` and `check-assemble`. ⛔ No count of them is written here, for
the reason this entry already records twice. ⚠ Three of those need
`tree_digest`, `tree_files` or `place`, which `store-lib.ps1` deliberately does
not carry yet.

### ⛔ What the door sweep found on 2026-09-08: the two runners' row lists

**Nothing compares the set of rows `check-gate.sh` runs against the set
`check-gate.ps1` names.** The `sh` runner derives its provers from a `for` list;
the `ps1` runner declares each one by hand as a run or an `Add-Unavailable`. A
prover added to the first and forgotten in the second is simply absent from the
Windows lane, which stays green because it never hears of it.

⚠ **It bit immediately.** `check-capture-client` went into the `sh` list, and the
Windows lane would have run twenty-six rows to the Linux lane's twenty-seven with
nothing anywhere naming the one that was missing. The row is written, so that
instance is closed; the class is not. ⛔ `check-twins` cannot see it either,
because it deliberately does not pair the gate runners: a harness that runs both
gates, running inside the gate, would re-enter itself.

⭐ **The cheap fix belongs with this entry's other half.** Give each runner a
mode that prints its row names and nothing else, then compare the two lists. That
needs no gate run at all, so it can live in `check-project` rather than in
`check-workflow`. Acceptance: with a prover added to the `sh` list alone the
comparison refuses, and with the pair in step it passes.

### ⭐ That half is done, 2026-09-09, and it is its own row rather than a rule in `check-project`

`--rows` and `-Rows` print the names each runner's own queue would have used and
run nothing, and
[`../scripts/common/check-gate-rows.sh`](../scripts/common/check-gate-rows.sh)
compares the two lists. ⛔ **The names come out of the calls that would have run
the checks**, so a row the mode does not print is a row that runner does not run;
a separate list would be the value in two places this comparison exists to catch.

⚠ **It is a gate row rather than a `check-project` rule because it RUNS both
runners**, which is a different kind of thing from reading the tree - and it can,
because a rows mode executes no check and therefore cannot re-enter the gate.
That is the same contract that keeps `check-twins` from pairing the two runners.

⚠ **It compares sets rather than sequences.** The `sh` half runs its checks
concurrently and the PowerShell half is serial, so the order a row appears in is
not a fact about what either lane runs; a duplicate still shows up, as a count
that does not match.

⛔ **The comparison refuses two short lists**, because a runner that printed
nothing agrees perfectly with another that printed nothing - which is the shape
`check-one-home` records about its own first run.

⭐ **Six cases, four of them plants:** a prover added to the `sh` list alone, one
dropped from it, a row declared on the PowerShell lane alone, a control either
side, and a probe that the rows output carries no verdict text of any kind, which
is what says the mode ran nothing.

### ⛔ Writing it found a live instance of this repository's own PowerShell hazard

**Measured 2026-09-09, on the first invocation.** `[switch]$Rows` collided with
the runner's own `$rows` accumulator: PowerShell variable names are
case-insensitive, so the parameter and the local are ONE variable, the ArrayList
was assigned to a `SwitchParameter`, and **every** invocation of that runner
failed to bind with `Cannot convert value "System.Collections.ArrayList"`.

⛔ **That is the third instance of one class here.** `docs/conventions/shell.md`
section 8 records `$args` inside a function, and `CI-03` records `[switch]$Marker`
against a local called `$marker` in `assert-disposable.ps1` - which failed to bind
in every mode, and was also found by running the guard once rather than by reading
it. ⚠ The rule that document already states is *name locals so they cannot
collide*, and the local is renamed rather than the parameter, because the flag has
to match the `sh` half's.

⚠ **And the two runners' row LABELS had to be made to agree on one row.** Each
half spelled its own flag - `check-no-secrets --public` against
`check-no-secrets -Public` - so a label built from the flag made the lists differ
on a row both lanes have, which is a false difference in the one comparison that
exists to find real ones. Both name the question now.

## CI-08: Runner-default drift, swept rather than waited for

Source: `$PSNativeCommandUseErrorActionPreference`, found by CI going red
Priority: P1 | Effort: L | Status: OPEN

Problem: A PowerShell default changed between 7.4 and 7.5 and turned a green
lane red over a script nobody had edited. That default was one of a class: every
behaviour a script inherits from its host rather than states is a defect waiting
for an image bump, and this project found the first one by being bitten.

Approach: Enumerate what the shell scripts, the PowerShell scripts and the
workflows inherit rather than state - shell options, output encodings, locale,
`$ErrorActionPreference` and its native-command companion, `git` defaults,
`cargo` environment variables, and the runner images' own tool versions. State
each one or record why inheriting it is safe. ⛔ The doctor already reports tool
versions; what is missing is the comparison against what the code assumes.

Prove: a rule per stated default in `check-project`, both halves, each
mutation-proven; and a driven pass that runs the gate under a deliberately
hostile environment - a different locale, a narrow console, `CARGO_TARGET_DIR`
set, and `TMPDIR` moved - with the same verdict.

⭐ **`CI-06` added the first three rules of this shape and left a harness gap it
owns rather than closes.** Nothing in the tree runs a capture step's *body*:
`check-workflow.sh` reads `capture.yml`'s step names, their order and the
*Capture* step's command, and executes steps out of `ci.yml` alone. So the two
restore blocks are proved statically by `check-project` and dynamically only by a
dispatch. ⚠ A harness that lifted each step body out of the workflow and ran it
under GitHub's own wrapper form - prepending `$ErrorActionPreference = 'stop'`,
appending `if ((Test-Path -LiteralPath variable:/LASTEXITCODE)) { exit
$LASTEXITCODE }`, invoking `pwsh -command ". '<file>'"` - against stubbed guards
would close it, and it is this entry's shape rather than `CI-06`'s: it is about
what a script inherits from its host. Acceptance would be that harness refusing
the *Restore the route* block as it stood on capture run 1 and accepting it as it
stands.

### ⭐ A capture step's body has now been run, by hand, and it found a defect

**Measured 2026-09-09.** `capture-client.yml`'s *Resolve the release artifact*
step was lifted out of the workflow with the same reader `check-workflow` already
uses and executed verbatim on this host, with `RUNNER_TEMP` pointed at a scratch
directory. ⛔ **It failed, and the cause was in the step rather than in anything
it called**: the block redirected into `$RUNNER_TEMP/release/url.txt` while the
directory was created by `resolve-release.sh` itself. A shell opens `>` targets
before it execs, so the step would have died on the redirection - on a runner,
after the claim, with a diagnosis naming a file rather than the mistake.

⚠ **No reader would have found it.** The script creates the directory and the
step names it; the two are correct separately and wrong composed, which is the
composition failure `docs/methodology/gate.md` part (b) exists for. Every
PowerShell rule in `check-project` passed the file, and so did every ordering
case in `check-workflow`.

⛔ **The harness gap this entry owns is still open, and running it by hand is why
its shape is now clear.** Two things stand between the measurement above and a
case:

1. ⚠ **This step reaches a vendor.** A case that fetched a release listing on
   every Linux lane would make CI depend on GitHub's release endpoint being up,
   which is a flake this repository would then be reasoning about instead of the
   rule. `resolve-release --listing` makes the source a file, and a **fixed step
   body cannot pass that flag** - so the seam a harness needs is one the step can
   inherit rather than be given, and that is a decision rather than an omission.
2. ⚠ **Most capture steps cannot run here at all.** The claim writes under
   `/var/lib`, the cut deletes a default route, and the capture needs both. This
   one is the first step of any capture workflow that a session host can execute,
   which is what made it available to try.

⭐ **What it establishes is that the reader is enough.** `step_command` lifted a
capture workflow's block correctly on the first attempt, so what the harness
needs is the environment and the seam, not a new parser.

### ⛔ The hang this entry owns: a second reading refuted, and a clock on it

**Client capture run 5, 2026-09-09.** The `release` route runs no package
operation of any kind, and *Upload the install logs* hung exactly as it does on
the `package` route. `TODO/clients.md` carries the table; what belongs here is
what it does to this entry.

⭐ **The step's own work finishes in five seconds and the step runs for
thirteen minutes.** The artifact was written at 03:17:23Z into a step that began
at 03:17:18Z, and the step had not returned at 03:30:43Z. Earlier runs could say
only that the artifact was complete; this one times it.

⛔ **Two named causes have now been refuted and neither was found by reading.**
`NEEDRESTART_MODE` fell to install logs saying `0 newly installed`; "the aria2
package rather than the route" fell to a route that touches no package index.
⚠ A third guess is what this entry exists to avoid: what is left is a runner
default nobody has swept, and the sweep is the Approach above rather than another
dispatch.

### ⛔ Run 6: the bound does not fire, and the step moved instead

**`timeout-minutes: 5` was the mitigation run 5 argued for, and run 6 measured it
not working.** Both lanes sat in that step far past the bound:

| lane | step began | its artifact was written | still running at |
| --- | --- | --- | --- |
| `package` | 03:50:11Z | 03:50:12Z, **one second in**, 3247 bytes | 04:07:39Z, **17 minutes** |
| `release` | 03:52:32Z | 03:52:34Z, **two seconds in**, 43179 bytes | 04:07:39Z, **15 minutes** |

⛔ **So GitHub's own per-step bound does not stop it**, which is a fact about the
failure and not only about the fix: `timeout-minutes` is enforced by the runner
process, so a step it cannot end is a runner that is not enforcing its own bound.
⚠ Naming what that implies would be the third guess this entry exists to avoid.

⭐ **The size question is closed by the same run.** The package lane's artifact is
**3247 bytes** - two logs and two stderr files - and it hangs identically to a
lane whose artifact is thirteen times larger. It is not the upload's volume.

⭐ **What did work needs no diagnosis at all: the step moved to the end of the
job.** It sat between the install and the route cut on the reasoning that a job
which never reached the capture would otherwise upload nothing - and `if:
always()` runs after a failed step wherever it sits, so the early position bought
nothing the last position does not. From there a hang costs the job's tail rather
than the measurement, because the capture and the evidence upload have already
happened. ⚠ `check-workflow` asserts the new constraint the move creates: every
upload is after *Restore the route*, because between the cut and the restore this
host has no way off itself.

⚠ **The bound is removed rather than kept alongside.** A bound measured not to
bound the one failure it was added for is decoration, and decoration in a
workflow reads as a control the next reader will trust.

⚠ That is a mitigation and not an answer, and this entry stays open on the answer.

### ⚠ Three routes to the answer that were tried and did not give one

⛔ **The hung jobs' logs are not retrievable.** An authenticated read of both of
run 6's jobs answers **HTTP 404**, as it does for run 4's aria2 job: a job whose
runner was killed or cancelled has no log to fetch. ⭐ That is why the artifact
route matters and why the install logs go up at all - the log is the one piece of
evidence a killed job does not leave, and this is now measured rather than
inferred from two dispatches that "lost their diagnosis".

⚠ **The last line of a job that DID complete is `Cleaning up orphan
processes`**, read from run 4's qbittorrent job. That is the runner's own
post-job step and it appears on every successful job, so it is a pointer and not
a finding: it says the runner tracks processes a step leaves behind, which is a
place to look rather than a cause.

⛔ **And the aria2 package install leaves nothing running, measured here.** The
adapter's package route was run directly on this host - no claim, no
`install-client`, because installing a product and asking its version is not a
capture - and the process count was **82 before and 82 after**, with nothing
matching `apt`, `dpkg`, `aria2`, `needrestart` or `unattended` alive afterwards.
⚠ This host is not the runner image, and it proved that in the same run: `aria2`
was **absent** here, so the install actually installed, where on `ubuntu-24.04`
it is a no-op. So the negative result is about this host and narrows rather than
settles.

⚠ **What would discriminate is a non-aria2 job reaching that step**, which every
completed job so far has passed through without hanging. With the step at the end
of the job, a `transmission` lane on the `release` route reaches it after an
install that refuses by design - so one dispatch would say whether the step or
the target is the subject. That is a case this entry can name rather than a guess
about a mechanism.

### ⭐ Run 7 ran that case, and it moves the boundary twice

**Four lanes: two transmission and two aria2, over both routes.**

⛔ **The step is not the subject.** Both transmission lanes ran *Upload the
install logs* in **one second** - one after a green capture and one after an
install that refused - in the same run, on the same image, as two aria2 lanes
that hung.

⛔ **And the hang is not attached to that step at all.** Moved to the end of the
job, it stopped hanging; what hung instead was *Install the client*, the step it
used to follow. Run 6's aria2 package install took **six seconds**; run 7's took
**ten minutes** and never completed. ⚠ Nothing about the install changed between
those two runs except which step comes after it.

⭐ **So the subject is the boundary after the aria2 install, not any particular
action.** That is a sharper statement than four dispatches could make and it is
what this entry now owns: whatever the aria2 install leaves behind, the runner
does not finish the adjacent step over it, and it does so whether that step is a
`uses:` upload or a `run:` block.

⚠ **Two local reproductions came back negative and neither settles it.** The
aria2 package route run directly on this host left 82 processes before and after,
and `aria2c --version` under the same bound `install-client` uses left 78 before
and after. ⛔ This host is not the runner image - `aria2` was absent here, so the
install actually installed, where on `ubuntu-24.04` it is a no-op - so both are
about a different code path than the one that hangs.

⚠ **And the move has a cost this entry records rather than trades away.** With
the install step hanging, run 7 produced no aria2 artifact at all, where runs 3
to 6 produced one. The four already collected say what that route does; what was
bought instead is where the hang sits.

### The second half of the Prove is done, measured 2026-09-08

⭐ **The gate gives the same verdict under a hostile environment.**
`CARGO_TARGET_DIR` pointed elsewhere, `TMPDIR` moved, `COLUMNS=80` and the locale
changed: `27 checks: 26 passed, 0 failed, 1 skipped, 0 unavailable`, row for row
identical to the ordinary run. ⛔ `CARGO_TARGET_DIR` is the one that had fired
before - it once put every built example where the harnesses did not look and
five provers exited 2 at once - so this is a regression that stayed fixed rather
than a variable nobody had tried.

⛔ **The first locale run tested nothing, and finding that out is what asking
"what would have made this fire" is for.** `LC_ALL=C` on a host whose locale is
already `POSIX` changes nothing, which is a trap
[`../docs/conventions/shell.md`](../docs/conventions/shell.md) states in as many
words about a different check. The host was measured - `LANG` empty, `LC_CTYPE`
`POSIX` - and the run repeated under `C.utf8`, which is the setting that differs.
Same verdict again.

### Three defaults are stated rather than inherited now

⭐ **`set -u` is the first code line of every executable script**, checked in
both halves and mutation-proved twice: a script without it, and a script that has
it below something else. ⚠ The rule reads the FIRST code line rather than
anywhere in the file, because this tree contains a heredoc whose body begins
`set -u`, so a looser rule would accept a script whose only `set -u` belongs to a
stub it writes. Measured before writing it: the convention already held in all
forty, which is what makes the precise rule the true one.
⚠ [`../scripts/corpus/store-lib.sh`](../scripts/corpus/store-lib.sh) is exempt by
name and for a reason - it is sourced, so an option set there changes the
caller's shell and stays changed, the same shared-namespace hazard that made this
repository prefix that library's globals.

⭐ **A cargo output path is asked for, never composed.** Five places compose one
and all five honour `CARGO_TARGET_DIR`; a sixth that did not is refused now, in
both halves. That is `CI-01`'s defect turned into a rule: two places composed
that path and fixing one left the other.

⭐ **A tool version the code assumes is compared against what installs it.**
`check-project` reads `SHFMT_VERSION` out of
[`../scripts/doctor/provision.sh`](../scripts/doctor/provision.sh) and refuses any
workflow or script naming a different one. That is the first row of "the runner
images' own tool versions" in the Approach above.

### ⭐ The harness gap is closed, and the instrument it needed was not the one this entry asked for

**Written 2026-09-09.**
[`../scripts/ci/check-step-bodies.sh`](../scripts/ci/check-step-bodies.sh) lifts
a block out of `capture.yml` or `capture-client.yml` and runs it under GitHub's
own form: `bash -e` for a default `run:`, and for a `shell: pwsh` step the
`pwsh -command ". '<file>'"` invocation with `$ErrorActionPreference = 'stop'`
prepended and the residual-exit epilogue appended.

⭐ **The acceptance this entry wrote is met.** It refuses the Windows *Restore
the route* block in the form that failed capture run 1 - the explicit `exit 0`
taken away, so the wrapper reads the guard's own refusal as the step's verdict -
and accepts it as it stands, over a stub guard that refuses. Both halves of the
`sh` block's three-code reading are cases too.

⛔ **But the reason a body was worth RUNNING turned out to be a second fact this
entry had not stated: a step does not end when its command exits.** The runner
reads a step's output through a pipe, so the step is over when the command has
gone *and* that pipe has reached end of file. A process the body leaves behind
holding the step's stdout keeps it open, with an exit code of 0 sitting in it.
⚠ Every harness in this repository redirects a step body's output to a **file**,
and a file has no reader to wait on, so no existing check could see the class at
all.

⭐ **So the harness records two facts per body and keeps them apart**, and the
two failures are reported differently: a body that refuses, and a body that will
not end.

⚠ **What sent the instrument there is a measurement rather than a hypothesis.**
Client capture runs 6 and 7 ran the same aria2 install command from commits whose
only functional difference is where a later step sits - `git diff` over the two
says so, and nothing in `install-client.sh` or the adapter changed - and that
step took **six seconds** in one and had not returned after **sixteen minutes**
in the other. A command whose duration depends on which step follows it is not a
command that is slow. ⛔ **That is not a fourth cause and is not offered as one.**
It is where an instrument can be pointed, which is what this entry asked for
instead of another dispatch.

⛔ **And the same block is proved to hang here, in the step it hangs in there.**
`capture-client.yml`'s *Install the client* body, run against this repository's
real `install-client.sh` and a stub product, ends over a product that behaves and
does **not** end over one that leaves a single process on the step's output.

⚠ **What it does not establish is a runner image.** A body that ends here is not
a body that ends there. What it proves is the shape, and that no check in this
tree could previously tell the two apart.

⚠ **THE WRAPPER PASSES `-NoProfile` AND GITHUB'S DOES NOT, and that departure is
stated rather than silent.** A `shell: pwsh` step really does inherit whatever
profile its host has, so the faithful invocation would omit the flag; a gate row
that loaded a contributor's profile is a row that goes red for something outside
this repository, and every other `pwsh` invocation here passes the flag for
exactly that reason. ⛔ **So what this harness proves about a `pwsh` body is
proved with no profile loaded**, and a defect a profile would cause is outside
what it can see. Decided by the operator on 2026-09-09, after the flag was
briefly removed on the fidelity argument alone.

#### Guard mutation over the instrument, 2026-09-09

Seven plants into a scratch copy of the whole tree, one at a time, each verified
to have applied before its result was believed, with the clean copy run either
side.

| plant | outcome |
| --- | --- |
| the instrument always answers CLOSED | ⭐ refused, by the two cases that assert a hang |
| the instrument always answers NOT CLOSED | ⭐ refused, by every case that asserts a body ends |
| the pipe becomes an ordinary file, which is what every other harness uses | ⭐ refused, by the same two |
| GitHub's residual-exit epilogue dropped from the wrapper | ⭐ refused, by the run-1 case alone |
| the close bound raised above the planted leak | ⭐ refused, by the two hang cases |
| the close bound set to zero | ⭐ **could not run**, which is the guard below |
| the `pwsh` prologue dropped | ⚠ **survived**, and the reason is this entry's own rule |

⚠ **A surviving plant is a question, and this one has an answer.** Dropping the
prologue changes nothing for these bodies because every `pwsh` block in this tree
sets `$ErrorActionPreference` itself - which is the rule `check-project` enforces
and this entry wrote. The plant would bite the day that rule stopped holding, so
it is recorded rather than turned into a case that passes for a reason of its
own.

⛔ **`timeout 0` MEANS NO LIMIT, and a ceiling edited to zero is therefore an
infinite one.** Found by planting it: the harness waited for the leaking body
until the leak ended by itself and then reported the pipe as closed. A bound
below one second is refused now with exit 2.

⚠ **Two earlier shapes of the wait were measured and rejected**, and both are
this harness's own subject arriving in its instrument. A `kill -0` poll on a
one-second granularity cost a whole second on every case that DID close, because
a reader whose last writer has just gone has usually not been scheduled yet - and
on a saturated host it could report a closed pipe as open, which is a red row for
no defect in a gate that runs thirty checks at once. A `( sleep N && kill ) &`
watchdog fixed the cost and left an orphaned `sleep` per case. What ships is
`timeout N tail --pid` on the reader, which blocks, bounds, and leaves nothing.

⭐ **And the reader is one home now.**
[`../scripts/ci/workflow-step.sh`](../scripts/ci/workflow-step.sh) is what lifts
a step out of a workflow, and `check-workflow.sh` asks it rather than carrying a
second parser. ⚠ The extraction was compared against the reader it replaced over
every job and step in every workflow here - sixty-six of them - and the output is
identical on all of them. ⛔ It answers with an exit status rather than a string,
and `scripts/README.md` names the three it distinguishes. `check-workflow.sh`
collapses two of them, as it always has; the new harness does not, because a step
it names and the workflow no longer has is rot rather than a rule that quietly
passed.

### ⭐ And what the instrument says to do about the install step

**Changed 2026-09-09.** *Install the client* sends the route's output to a file
and prints it afterwards, so nothing the product spawns inherits the step's own
output pipe. Three cases in the harness carry it: the block ends over a product
that behaves, it ends over one that leaks a single process, and a planted `3>&1`
- one extra descriptor onto the step's own output, written before the
redirection - brings the hang straight back. ⛔ Without that third case the
second passes equally over a block that never had the problem.

⚠ **THE ORDER OF THE REDIRECTIONS IS THE PLANT, and the first version of it was
wrong**: a shell applies them left to right, so `>log 2>&1 3>&1` points fd 3 at
the log rather than at the step's pipe, and the case reported the hang not
happening over a plant that had duplicated the wrong file. It is the same class
as an in-place `sed` editing a line nobody named - a plant that applied
somewhere other than where the case says.

⭐ **The step also names what is holding the file now.**
[`../scripts/ci/report-holders.sh`](../scripts/ci/report-holders.sh) lists every
process with an open descriptor on the route's log and prints the process table
beside it, into `holders.log` in the workdir the `always()` upload collects. ⚠ It
runs under `sudo` because the install did: an unprivileged reader of `/proc`
reports nobody holding a file that root processes are holding, which is a
diagnostic that answers confidently and wrongly. Driven here against a known
holder: it named the process by pid, comm and arguments, reported `0 holder(s)`
for a path nothing held, and exited 2 with no arguments.

⛔ **It is a mitigation and its outcome is the measurement.** If an aria2 lane
now gets past *Install the client*, the class is what the hang was; if it hangs
anyway, this reading is refuted by something that separates it rather than by
another guess. ⚠ Either way the lane leaves `holders.log` behind, which is the
first positive evidence this project will have about what that route leaves
running.

### ⛔ Run 8 answered it, and the answer is no

**Measured 2026-09-09.** Both aria2 lanes sat in *Install the client* from
09:13:18Z and had not returned at 09:35:17Z, twenty-two minutes later, on a run
where nothing the route spawned could inherit the step's output pipe. ⛔ **So
that class is refuted**, the way the three before it were.

⭐ **What the same run establishes is where the step is NOT.** `install-client`
bounds the install call at 420 seconds and kills 20 seconds after that, so the
last moment that call could have ended is 440 seconds in; the step ran three
times as long. ⛔ Whatever is slow or stopped is outside the bounded call, which
leaves the unbounded parts of that script - its command substitutions and its
digests - and the step itself.

⚠ **The dispatch changed two things at once and that is a defect in the
experiment, not only in the record.** A holder report was added to the same step,
it walks every process's descriptors, and it runs on the branch a 22-minute step
cannot distinguish from a slow install. ⛔ It is bounded by `timeout 60` now, and
the step no longer waits on the install at all.

### ⭐ So the next step bounds itself and records a timeline from inside the window

**Written 2026-09-09.** The install runs in the background and the step's own
shell watches it, because that shell is outside every bound that has failed:
`timeout` ends its own child and cannot end a shell blocked in a substitution
around it, and `timeout-minutes` is the runner's and was measured on run 6 not to
end the step at all.

⭐ **Every tick leaves a process snapshot** - `pid,ppid,pgid,stat,etimes,comm,args`
every five seconds into `watchdog.log` in the uploaded workdir. ⚠ `etimes` beside
`stat` is what separates a command that is SLOW from one that is stopped, which
is the question this entry cannot answer from timings alone.

⛔ **And the deadline is what makes any of it reach a reader.** A job whose runner
is killed leaves no log and no artifact; a step that ends leaves both. Measured
again on run 8: rule 8's route answers **302** for a running job's log and
redirects to a blob that answers `BlobNotFound`, while a general-purpose GitHub
tool answers a bare **404** for the same job - so the log is not written until
the job finishes, and only an uploaded artifact survives.

⭐ **The bound is seen to fire rather than assumed to.** Two cases in
`check-step-bodies`: a product made slower than the deadline is killed and the
step still ENDS, and a product slower than one tick but inside the deadline is
not killed. ⚠ Without the second, the first passes equally over a block that
kills every install it is given.

### ⛔ Run 9: the watchdog in the step's own shell did not fire either

**Dispatched 2026-09-09 as `["aria2","transmission"] × ["package"]` on
`42bd206`**, with transmission as the control.

| lane | outcome |
| --- | --- |
| `transmission` `package` | ⭐ **green: a complete capture**, install 121s, whole job three minutes |
| `aria2` `package` | ⛔ *Install the client* began 10:13:54Z and had not returned at 10:28:45Z - **fifteen minutes** |

⛔ **The deadline was 480 seconds and it passed by seven minutes.** That loop runs
in the step's own shell, so a shell that was looping would have fired it. ⚠ So
the step's shell is not reaching the loop, or is not running at all - which is a
fact about the step rather than about the install, and it is the third bound
measured not to fire here.

⭐ **And transmission is the control that keeps this attributable.** Same run,
same image, same step, same `apt-get`: 121 seconds and green. ⚠ That install is
itself far slower than the 14 seconds earlier runs recorded, so these hosts are
slow today - and slow is exactly what the aria2 lane is not, because a slow
install would have been ended by `install-client`'s own 420-second bound.

### ⭐ So the bound moves outside the shell entirely

[`../scripts/acquisition/install-step.sh`](../scripts/acquisition/install-step.sh)
holds what the step used to do inline, and the workflow runs it as
`timeout -k 30 540 sh …`. ⛔ **The bounded process is now the step's own**, so
nothing inside the step has to be reachable for the bound to work.

⚠ **What that buys is not a faster failure. It is a job that reaches its uploads
at all**: runs 7, 8 and 9 each ended with no log and no artifact, so an aria2
lane that hangs has taught nothing three times running. ⭐ A step that ends leaves
`watchdog.log`, `step.log` and `holders.log` behind, and `stat` beside `etimes`
is what separates a command that is slow from one that is stopped.

⭐ **Two cases hold it**, both in `check-step-bodies`: the bound around the step's
own process fires and the step ends with coreutils' 124, and a product that
finishes inside the bound is not killed.

### ⛔ Run 10: that bound did not fire either, and four is a pattern

**Dispatched 2026-09-09 as `["aria2"] × ["package","release"]` on `3b5793d`.**
Both lanes entered *Install the client* at 10:57:50Z and 10:57:58Z, and neither
had returned at 11:10:12Z - **twelve minutes** against a `timeout -k 30 540`
around the step's own command, which would have ended it at 570 seconds.

⛔ **So four independent bounds have now been measured not to fire on this
step**, each at a different level:

| the bound | where it sits | measured |
| --- | --- | --- |
| `timeout -k 20 420` on the adapter call | inside `install-client` | run 8: the step ran three times past it |
| `timeout-minutes: 5` | the runner's, on the step | run 6: the step ran seventeen minutes |
| a watchdog loop with a 480s deadline | the step's own shell | run 9: passed by seven minutes |
| `timeout -k 30 540` on the step's command | the step's own process | run 10: passed by three minutes |

⛔ **And no such job has ever produced a log or an artifact** - runs 7, 8, 9 and
10. The log blob is never written, which is why rule 8's route answers 302 to a
`BlobNotFound`, and the run ends `cancelled` around the job timeout.

⚠ **That combination is what separates a fifth reading from the four guesses
before it.** A step that were merely stuck would still leave a runner able to
enforce one of four bounds and to upload a log; nothing here does either. What
the evidence describes is a job that stops being served, not a command that does
not return - and `TODO/clients.md` carries transmission passing through the same
step on the same image in the same runs.

⛔ **It is still a reading and it is not recorded as a cause.** What it changes is
where to look: at the host rather than at the shell.

### ⭐ So the next dispatch bisects with step NAMES, which is the one signal that survives

When nothing inside a job survives - no log, no artifact, no bound - the only
thing an outside reader still has is **which step the API last reported in
progress**. Three bounded probes now run before the install, each named, so a job
that wedges localises itself to a handful of commands without needing any of the
things these runs do not produce.

⚠ **The third probe is the asymmetry the whole thread rests on.**
`install-client` asks the adapter for a version *before* the route runs; `aria2`
ships on `ubuntu-24.04` and `transmission-daemon` does not, so on an aria2 lane
that call executes the preinstalled product and on a transmission lane it finds
no binary and refuses without running anything. ⛔ That difference is present in
every hung run and absent from every green one, and no dispatch has yet isolated
it.

⚠ **The probes guard nothing and each ends in a `true`**, so a diagnostic cannot
fail a capture. They come out when the question is answered.

### Acceptance for the harness, run on 2026-09-09

- `sh scripts/ci/check-step-bodies.sh`
- `sh scripts/common/check-gate.sh`
- `sh scripts/ci/check-workflow.sh`

⛔ **What is left of this entry is the sweep**, unchanged: the Approach asks for
an enumeration of what the shell scripts, the PowerShell scripts and the
workflows inherit rather than state, and three defaults are stated so far. ⚠ And
a second question this entry owns: the Linux lane pins `shfmt` and takes
`shellcheck` and `pwsh` from the runner image, so a session host runs a MORE
pinned set of tools than the lane it exists to match.

### ⭐ THE SWEEP IS AN INSTRUMENT NOW, NOT A READING. 2026-09-10

⛔ **This entry's Approach asks for an enumeration of what the scripts inherit
rather than state, and an enumeration written by READING is a list of the
defaults somebody thought of.**
[`../scripts/ci/check-defaults.sh`](../scripts/ci/check-defaults.sh) runs five
checks under six environments and compares their machine-readable answer against
the baseline: a difference is a default that check inherits, named by the
variable that produced it, whether or not anybody had thought of it.

| the environment | what it would change if a check read it |
| --- | --- |
| `LC_ALL=C` | byte collation and byte character classes |
| `LC_ALL=C.utf8` | multibyte collation and classes |
| `TMPDIR` | where a harness puts the tree it plants in |
| `CARGO_TARGET_DIR` | where a built example lands - the one that has already bitten |
| `POSIXLY_CORRECT` | GNU tool behaviour in several places |
| `SOURCE_DATE_EPOCH` | read by build tooling that wants a fixed clock |

⭐ **The result is that all five answer identically under all six**, and that
sentence is worth exactly as much as the controls behind it.

#### ⛔ Two controls, because "no difference" and "no experiment" look identical

⭐ **A probe that READS the variable is run first and must answer differently.**
An instrument whose environment never reached the child would report perfect
agreement over every subject; this exits **2**, not 0, if that probe agrees -
`could not run` rather than a subject that passed.

⭐ **And a planted environment-sensitive answer is caught.** `check-licences` was
made to print `SOURCE_DATE_EPOCH` in its own JSON, and the run named the subject,
the variable and both answers - and did **not** fire on the other five.

#### ⛔ AND ONE ROW IS BLIND ON THIS HOST, MEASURED BY REPLANTING THE REAL DEFECT

⛔ **The historical `CARGO_TARGET_DIR` defect was replanted - `store_build`
reverted to composing `$ROOT/target` and ignoring the variable - AND THIS FILE
DID NOT CATCH IT.** `$ROOT/target` already held the example from an earlier
build, so the defective path resolved to a **stale binary** and the check
answered normally.

⚠ **So that row fires on a clean checkout and is blind on any host that has built
before**, which is every host a contributor runs it on twice. ⭐ The condition is
reported as its own row rather than hidden: a reader told "the same answer under
all 6" without being told that one of the six could not have answered otherwise
has been told something weaker than it sounds.

#### ⚠ Two things this instrument cannot perturb, stated rather than dropped

⛔ **`IFS` does not survive into a child.** Measured here:
`env IFS=: sh -c 'printf %s "$IFS"'` prints the default, because a shell resets
it at startup. A case setting it would report a guard proved by a value the child
never saw, so it is a control row saying so.

⚠ **`umask` is a shell attribute rather than an environment variable**, so `env`
cannot pass it, and setting it in this harness would change the modes of
everything the harness itself writes. It belongs to whatever check reads a file
mode, and nothing here does.

#### ⚠ What it costs, and the subject that was dropped for it

**28 seconds**, because every subject runs once per environment. ⛔ That is
concurrent with the rest of the gate and free in local wall-clock terms, and it
is **not** free inside `check-workflow`, which runs the whole gate about ten
times and is already the CI wall clock. `check-docs` is the one subject left out:
it resolves links and parses fenced blocks, which is the least plausible thing
for a locale or a scratch directory to reach. ⚠ `check-markers` stays because it
decodes UTF-8 **by hand**, which is the most plausible.

#### ⛔ And a defect in this file, found by running it rather than by reading it

**`grep -c` over a file with no matches PRINTS `0` and EXITS 1**, so
`$(grep -c ... || printf 0)` ran the fallback as well and the value became two
lines. It went into a row, and `store_report` counted fourteen rows against eight
cases and refused. ⭐ That self-check is why this was a red line on the first run
rather than a miscounted report nobody read.

### ⛔ THE GATE WAS SMALLER THAN THE LANE AND NOTHING SAID SO. 2026-09-10

⛔ **CI run 113 failed on *Shell syntax and style* over one `SC2016`, on a commit
whose local gate had reported 34 of 35 passing minutes earlier.** The gate ran
neither `shellcheck` nor `shfmt` over the scripts. That is this entry's own
subject arriving from the other side: not a default inherited from the host, but
a STEP the lane runs and the local gate did not.

⚠ **And the workflow already claimed otherwise.** `.github/workflows/ci.yml`
installs the pinned `shfmt` on the Windows lane with the comment *"several of the
cases below run the gate, and the gate runs shell checks"*. No row did. ⛔ That is
the fourth instance this repository has recorded of a rule a document says it has
and does not, and the comment is corrected rather than deleted, because the
sentence is true now.

⭐ **[`../scripts/common/check-shell.sh`](../scripts/common/check-shell.sh) is the
row**, running CI's own command over every tracked script found rather than
listed. ⛔ Two rows and not one, because `&&` between two checks is one check and
a reader of a red row has to know which tool refused. ⭐ And a third that counts
what was swept: two clean tools over a `find` that stopped matching answer
exactly as they do over a clean tree.

⚠ **`provision.sh` had been installing both tools all along**, with a header
saying a host without them "runs a gate that is quietly smaller than CI's". The
tools were there; the row was not.

Guard mutation, one plant per tool, each verified to apply: a removed
`shellcheck` directive and an added indent. Both refused, and each row named
itself rather than the other.

### ⚠ Residual, filed 2026-09-09: `check-step-bodies` is a load-sensitive row

⛔ **A gate row that fails under load and passes alone is the same class this
entry is about** - a behaviour inherited from the host rather than stated - and
it is now measured **twice** on one session host. Both times the gate reported
`FAIL check-step-bodies (exit 1)`; both times
`sh scripts/ci/check-step-bodies.sh` run alone immediately afterwards reported
`24 cases: 24 passed, 0 failed`, exit 0; and both times the next gate over the
same tree was green.

⭐ **The mechanism is in the harness's own subject.** Its cases time things: a
bound that must fire as `124`, a case named "a product slower than one tick and
inside the bound survives", and whether a step's output pipe has reached end of
file. A loaded host moves a case across its own boundary, which is a timing
assumption inherited from whatever else is running.

⚠ **What is NOT established is that this explains every red seen today.**
`check-workflow` failed one `gate_control` case on each of two runs, a different
case each time, and those cases report only "the clean tree failed a check"
without naming which - and the harness deletes its workdir, so the naming log is
gone. ⛔ It is consistent with this row flaking and it is not proof of it.

⛔ **It can turn the CI Linux lane red over a correct tree**, because that lane
runs the gate with `--strict`. Not observed there yet: CI runs 93 and 94 were
green on all three jobs.

#### ⭐ DIAGNOSED AND FIXED on 2026-09-09, and the first two readings were wrong

⛔ **The obvious reading was that `CLOSE_SECONDS=3` is too tight, and it was
wrong.** Two reproductions were attempted against it and both came back green:
twelve CPU spinners on four cores, then the harness beside `check-capture`,
`check-capture-client`, `check-store`, `check-catalogue` and `check-examples` -
which is the faithful shape, because the gate backgrounds every check and runs
them concurrently. **24 of 24 passed** in each.

⭐ **What found it was capturing the failing run's own output.** The gate prints
the failing check's log; a loop of gate runs reproduced the red on the first
attempt, and it names one case:

```text
❌ sh  one leaked descriptor onto the step's output brings it back:
       the output closed, so the planted hang did not happen
```

⛔ **So the bound was not too tight - the PLANT EXPIRED.** That case asserts the
pipe is held open, and it reported the pipe closing. The leaking process is
spawned *inside* the body, and the pipe is only examined after the body exits
plus `CLOSE_SECONDS`, so the plant only proves what it claims while

```text
leak duration > body duration + CLOSE_SECONDS
```

⚠ **`sleep 8` was ample for the two `probe` cases and marginal for this one.**
Their bodies are `echo` and `exit 0`; this one's body is the whole *Install the
client* step - a `timeout` around `install-step.sh`, which runs `install-client`
under `sudo`, a watchdog loop and a bounded holder report. Under a full gate the
body reached roughly five seconds and ate the eight.

⭐ **Fixed:** the duration is now `LEAK_SECONDS=45`, named once and used at every
plant instead of a literal, with the relation checked rather than commented -
`LEAK_SECONDS` must be a positive integer and must exceed `CLOSE_SECONDS`, both
mutation-proven (exit 2, each naming its own reason). ⚠ The cost is stated in the
file: a `sleep` orphan can outlive the check by up to 45 seconds where the old
value bounded that at 8.

**Verified twice over:** 5 consecutive full gate runs green, against 3 red in
roughly 9 before the change - and `sh scripts/ci/check-workflow.sh` came back
**97 cases, 97 passed, 0 failed** on the final tree. ⭐ That last one matters
because it is the harness that was failing: it runs the whole gate about ten
times, and both of its runs before the fix lost exactly one `gate_control` case,
a different one each time. After the fix it lost none. ⚠ That is a rate on one host and not a proof; the failure is
load-dependent, and `LEAK_SECONDS=4` still passes on an idle host. ⛔ Which is
also why the guard compares the two bounds: the relation is what matters, and a
number that happens to work today is what went stale.

## CI-09: The capture-to-publisher path, end to end

Source: two workflows that have each never run, joined by an artifact
Priority: P1 | Effort: L | Status: OPEN

Problem: The capture workflow uploads with `actions/upload-artifact` v7 and the
publisher downloads with `actions/download-artifact` v8. Neither has run, so the
pairing is unexercised, and the publisher's first step is a download of a bundle
no run has ever produced.

Approach: Take a dispatched capture's artifact through the publisher's dry run,
which is its default, and read what the download step actually handed it.
⛔ Nothing may be published until a measured record exists, so the dry run is
the whole of this entry and the push stays refused.

### ⭐ What run 14 supplies, stated exactly. 2026-09-09

⛔ **The route blocker on this entry is gone and the connector blocker is not.**
`capture-client` run 14 acquired one target through two routes on two hosts and
captured on BOTH, which is the first time this project has had a capture per
route.

⭐ **That is precisely the input `equivalence::classify_across` needs.** Read the
four outcomes rather than assume them: `ByteIdentical` is every route installing
the same bytes; `BuildEquivalent` is installs that differ in bytes where every
one was **observed** and no overlapping field disagrees; `Divergent` is equal
versions over conflicting evidence; `Unresolved` is not enough evidence, and the
module says in as many words that it is what a **single** capture of two
byte-different installs produces. ⛔ Run 14's installs differ in bytes and both
were captured, so this pair is the first that can reach `BuildEquivalent` -
which `publishable()` accepts - instead of `Unresolved`, which it refuses.

⚠ **What it does NOT supply is a `Profile`, and `classify_across` takes two of
them.** Run 14 produced two install records, two attestations and two evidence
bundles. Assembling those into two records is this entry's work and it needs a
store to write into.

⛔ **AND A PUBLISHED RECORD NEEDS MORE THAN THIS ENTRY CAN GIVE IT.** `E-PUB-02`
keeps any measured field provisional while only one connector could see it, and
both lanes of run 14 used one - this project's own Rust observer. So the honest
target for this entry is a record that is **written, stored and provisional**;
`OBS-07` owns the second connector that would make it publishable. ⚠ Two gates,
and running them together is what made the old handoff say no record could be
written at all.

Prove: the publisher's dry run completes against a real capture artifact, its
`sha256sum -c` step passes on the downloaded bundle, and
`sh scripts/ci/check-workflow.sh` still asserts the publisher's dispatch-only
trigger and its `dry_run` default.

### ⛔ SOMETHING TRIED TO ASSEMBLE RUN 14 AND IT CANNOT BECOME A RECORD. 2026-09-09

⭐ **[`assemble-capture`](../crates/bit-ids-probe/examples/assemble-capture.rs)
is what this entry was missing**: it reads a lane's capture artifact and its
install artifact and writes the `Profile` they support, deriving every field
from a document rather than from a lane's name.
[`check-assemble.sh`](../scripts/capture/check-assemble.sh) proves it over
synthetic lanes - 15 cases, and the control is the half that matters, because
every refusal below passes equally over an assembler that refuses everything.

⛔ **Driven over run 14's four real artifacts, it refuses.** The section above
said run 14 "is precisely the input `equivalence::classify_across` needs". It is
not, and the reasons were invisible to every reading because none of them is in
an attestation:

| what the artifacts say | what refuses it |
| --- | --- |
| both lanes' `resolution.txt` differed **only in their timestamps** - one `source_url`, one `listing_sha256`, one `asset_url` | ⛔ `E-ACQ-07`: two routes sharing a resolver are one route - ⭐ **repaired**: the source route resolves its own tag from the repository's refs, `ACQ-02` |
| every attestation declares **one** connector | ⛔ `E-CAP-01`, at the **validity** gate |
| nothing records how the artifact was **packaged**, and `package` is in `StoreKey` | ⛔ a record filed under a guessed package is the non-injective path `store.rs` refuses |
| no document the capture path writes carries the source route's commit | ⛔ `E-ACQ-06`: a source identity needs a full object name - ⭐ **repaired below** |

⚠ **`classify_across` is NOT in that table, because it never ran on run 14**: it
takes two `Profile`s and neither exists. What is measured is that the pair's
**shape** reaches `Divergent` - `check-assemble` builds two records differing
only in their peer-ID tails and reads that verdict off them.

⭐ **The tool reports all of them in one run** rather than the first it trips on.
⚠ It did not at first: adding the `package` derivation moved that refusal in
front of the commit one and two gaps a previous run had named vanished from the
report. Each derivation is probed separately now, because this file's own header
promises a reader every field the capture path would have to record.

⚠ **The first is a design decision rather than an oversight, and
`capture-client.yml` argues for it in a comment**: *"THE SOURCE LANE RESOLVES
THROUGH THE SAME STEP, because both routes must land on ONE stable version... a
second resolution per lane would let a vendor whose newest release moves between
two reads hand the two routes different versions"*. ⛔ **Absolute 4 already
answers that worry**: version equality is *checked after installation*, not
trusted. Two independent resolutions that landed on two versions would be caught
by the comparison, and that is the correct outcome - it means the vendor moved
mid-capture and the pair is not a pair. Forcing one resolution manufactures the
agreement instead of measuring it, which is what `E-ACQ-07` exists to refuse.
⚠ `aria2-next.sh`'s own comment claims the two routes "differ in resolver - the
releases API against git refs". The artifacts refute it: the source lane is
*handed* `BIT_IDS_RELEASE_TAG` and resolves nothing.

### ⛔ AND THIS ENTRY'S OWN TABLE ABOUT CONNECTORS WAS WRONG

The table above records that a one-connector capture *validates* and is refused
only by `E-PUB-02` at publication. ⚠ **That is true of one shape and false of
the one this project produces**, measured by stripping the golden fixture two
ways and reading each exit code unpiped:

| what "one connector" means | `validate-profile` |
| --- | --- |
| two declared in `capture.connectors`, one **observing** each field | ⭐ exit 0, `valid`, then `provisional, not publishable` with six `E-PUB-02` rows |
| one **declared** in `capture.connectors` | ⛔ exit 1, `refused`, `invalid document`, `E-CAP-01` |

⛔ **Every capture this project has run is the second row.** So `OBS-07` is not a
publishability nicety this entry can note and move past: a second connector is a
**validity** requirement, exactly as `E-ACQ-01`'s second route is, and the
"written, stored and provisional" target this entry set itself is unreachable
without it.

### ⛔ `BuildEquivalent` is unreachable for a real client through this path

`classify_across` compares every field both records measured and calls any
disagreement `Divergent`. A peer ID carries a per-connection random tail, and a
single capture can only ever state such a field as `constant` with one sample -
`patterned` and `variable` both need two. ⛔ **So two records of two lanes
necessarily disagree on `peer_wire/peer_id` and land on `Divergent`.** Proved
both ways in `check-assemble`: a pair whose observations agree reaches
`build_equivalent`, and a pair differing only in its peer-ID tails is
`divergent`. ⭐ `SCHEMA-04`'s sampling model is where several captures become a
`patterned` field; it sits above the record and nothing has run it.

### ⭐ What was repaired here rather than only recorded

- `aria2-next.sh`'s source route runs `git rev-parse HEAD` after its clone and
  writes `source-commit` beside its log; `install-client` records
  `source_commit` and **refuses a `source` route whose commit is absent or
  abbreviated**, so the gap fails at the install rather than at an assembly a
  dispatch later.
- ⭐ **And every adapter now says how its route packaged the build**, written to
  `<workdir>/package` and copied into the install record, with a route that
  installed and did not say refused there. ⛔ It is per **route**: `qbittorrent`
  delivers a `.deb` one way and an AppImage the other, and a record calling both
  `elf-binary` would say they delivered one form. `adapters/README.md` is the
  contract. That closes the third row of the table above; the first two remain.
- [`parse_transcript_document`](../crates/bit-ids-lab/src/evidence.rs) is the
  transcript writer's inverse, beside it, refusing anything that writer does not
  emit - uppercase hex, a reordered key, a missing final newline. ⛔ Nothing
  could read a bundle back before it; the assembler needed one, and a second
  parser in the assembler would have been a second reading of this project's own
  format. Two tests pin the pair, one of them byte for byte.
- `store_build` takes a package, because `assemble-capture` lives in
  `bit-ids-probe`: it decodes transcripts with `bit-ids-wire`, and `bit-ids`
  cannot depend on that crate - the dependency runs the other way.

### ⭐ What the assembler declared, and what now produces it. 2026-09-10

A connector other than the observer reports what it saw in
`connector/<id>.txt` inside the bundle, one `field_path=value` line per field the
observer measured, where the value is lowercase hex, `absent` or `out_of_scope`.
⛔ **A declared connector silent on a field is refused rather than recorded as
silent**, which is `E-COR-07`'s rule applied where the record is written.

⭐ **`OBS-07` closed on 2026-09-10 and the producer exists.**
`scripts/capture/connectors/cpython-stdlib.py` writes that file and
`capture-client` requires it, so the `E-CAP-01` row of this entry's own refusal
table is repaired. ⚠ **Unproved on a runner**: no dispatch has taken the step.

⭐ **And this entry's assembler gained the line it was missing.** It wrote a
record and never said whether it could be PUBLISHED - found the moment a lane
could carry a connector conflict, because the record kept the conflict, `to_json`
accepted it and the report printed a star and a path. It prints
`publishable` or `provisional, not publishable` with every blocker now, and
`check-assemble` asserts that a lane whose connector read different bytes lands
on `E-PUB-01` for exactly the altered fields.

### ⚠ Residuals

- ⛔ **It writes a `Profile` and not the `RunManifest` that has to sit beside
  one.** A manifest carries the run's ordered phases, both clocks, the sampling
  plan and the host facts; an attestation carries a start, a finish, a platform
  string and a claim fingerprint. Inventing the rest would produce a document
  `bind` then compares against a record that agrees with it for no reason. ⚠ So
  the store this writes is a store of records with no runs, which `check-store`
  accepts and a publication would not.
- ⛔ **The door sweep found two more readers of a transcript, and both are
  `grep`.** `capture-run.sh` and `capture-client.sh` each assert a token appears
  in `tracker-http.transcript.json` with a fixed-string match. ⚠ A substring
  match cannot tell what the BUILD sent from what the lab sent, because both
  directions are in the document; the peer-ID check is meant to establish the
  first. `parse_transcript_document` is what would answer it properly, and
  wiring a Rust reader into the capture runner is a change to the capture path
  that needs a dispatch to prove. Filed rather than made unproven.
- ⚠ **`display_name` is the target identifier**, because nothing a capture
  uploads carries a display name and `catalogue/clients.toml` does. Reading it
  would mean a TOML dependency for one cosmetic field. It is not in the identity
  tuple, so unlike `platform`, `arch` and `package` it cannot file a record at
  the wrong path.
- ⚠ **The artifact digest is the installed executable's**, because nothing the
  capture uploads carries a digest of the bytes that arrived. `ACQ-05` is where
  a retrieval digest would come from.

### ⛔ What is left, and it is no longer this entry's alone

The publisher's dry run still cannot be reached, for the reason below and now
for several more. ⭐ The order is fixed rather than open: `OBS-07`'s second
connector, an independent resolution for the source route, and a recorded
package format are all **prerequisites for a record existing**, and the v7/v8
question sits behind them.

### What a real capture artifact answered on 2026-09-08, and what it refused

⭐ **A v7 upload survives a download and verifies.** The transmission bundle from
client capture run 4 was downloaded through `AGENTS.md` rule 8's route,
unauthenticated, and `sha256sum -c SHA256SUMS` inside it reports `OK` for all
three evidence files. That is the first time anything in this repository read a
capture bundle back outside the run that wrote it, and it is a reader this
project did not write.

⛔ **But the pairing cannot be exercised, and that is stronger than this entry's
premise.** The problem above says the publisher downloads a bundle *no run has
produced*. It is worse: nothing in the tree **can** produce it. The four uploads
here are `install-*`, `capture-client-*`, `capture-linux-*` and
`capture-windows-*`, and the publisher asks for `bundle`. Its first step fails on
every run that exists and on every run that could be dispatched today, so the
v7/v8 question cannot be reached at all.

⛔ **And a capture bundle is not a publication bundle**, so renaming would not
join them either. A capture carries `capture/SHA256SUMS` over three evidence
files and no `MANIFEST.json`; the publisher expects both documents at the root of
what it downloaded, and `publish-data.sh` pushes that tree as the publication.
What sits between them is `assemble-release`, which reads a **store of records** -
and no record has been written, so there is nothing to assemble.

⚠ **The producer is deliberately not stubbed, and this is the residual.** Every
record in the store today is synthetic, so a job that assembled one and uploaded
it as `bundle` would put a publishable-looking artifact one boolean away from
being pushed to the data branch. The missing piece is a record, which is
`CLIENT-01`'s remaining gap rather than this entry's.

### ⛔ What that record actually needs, measured 2026-09-09

**A single-route capture cannot become a record at all**, and the work order said
it could. `docs/history/RESUME.md` has carried "a single-route, single-connector
capture VALIDATES and refuses to publish"; half of that is right and the half
this entry waits on is not.

| the capture that exists | what the library does | where it bites |
| --- | --- | --- |
| one connector | **validates**, then `E-PUB-02` refuses to publish it | publishability |
| one route | `E-ACQ-01`: *1 route(s); two independent routes are required* | ⛔ **validity** |

⚠ **Measured rather than read**, by stripping one route and one connector from
the golden fixture and running `validate-profile` over each. The single-connector
copy validates and reports `provisional, not publishable` with six `E-PUB-02`
rows; the single-route copy is refused outright.

⭐ **Both facts were already in the suite and only the prose disagreed.**
`profile_schema.rs` plants `E-ACQ-01` by truncating the route list, in the loop
that asserts each plant is a **validation** refusal, and
`agreement_refuses_to_publish_a_measurement_no_second_connector_saw` reads its
document back through `Profile::from_json` - which validates - before asking
`publishable`. Nothing was wrong in the code; a sentence in a handoff was, and it
is the sentence this entry's dependency was written from.

⛔ **So `Profile::to_json` will not write it and the store cannot hold it.** Every
capture this project has run - transmission twice, qBittorrent once - is one
route, so none of them can become a record however much of the assembling code is
written. ⭐ `E-ACQ-01` is right and the document was wrong: the product IS the
two-route claim, and a record carrying one route would publish a weaker
measurement under the same schema.

⭐ **What unblocks it is a two-route capture, and there is exactly one target it
can be run on.** aria2 is the only target whose two routes currently resolve the
same version, `capture-client.yml` can now dispatch a route per host, and
`resolve-release` chooses the artifact the release route fetches. That dispatch
is `CLIENT-05`'s and this entry waits on its output.

⭐ **What is fixed is that the gap can no longer be invisible.** `check-project`
compares every `download-artifact` name against every `upload-artifact` name in
the tree, with `${{ ... }}` normalised to `*` on both sides, and refuses a
download nothing produces. The publisher declares this one with
`bit-ids:no-producer=CI-09`, the declaration must name an entry `INDEX.md`
really carries, and ⛔ **a declaration over a name that HAS gained a producer is
refused too**, so the marker cannot outlive its reason: the day the producer
lands, the gate says so.

⚠ **The rule's own first version was wrong in two ways, and only planting found
either.** It scanned forward from `uses:` and took the first `name:` it met, and
it read `name:` alone:

- a step written `- with:` / `name:` / `uses:`, which YAML permits because a
  mapping has no key order, made it report the **step's** name as the artifact's;
- `download-artifact` also accepts `pattern:`, so every download written that way
  was skipped in silence.

⛔ **Neither shape exists in this tree**, which is the condition
[`reviews.md`](../docs/methodology/reviews.md) names as the easiest place to get
a scope wrong: every reading agrees on every file. Both are planted now, and the
artifact name is taken from inside `with:`, identified by the column of that key
rather than by being the first one seen. ⚠ The first plant *passed* before the
name was checked, because a rule that never found the row and a rule that found
it and accepted it both exit 0; removing the declaration is what separated them.

⭐ Six states, both halves, same verdict on each: the clean tree, a declaration
removed, a declaration naming no real entry, a declaration gone stale, a
`pattern:` download, and a reordered step.

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
