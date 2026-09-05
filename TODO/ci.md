# CI entries

## CI-01: Complete cross-platform quality gate

Source: template gate method and operator best-in-class CI requirement
Priority: P0 | Effort: L | Status: IN_PROGRESS

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

## CI-02: Stable-release staleness monitor

Source: operator automatic maintenance requirement
Priority: P1 | Effort: L | Status: OPEN

Problem: New stable client versions must create bounded work without silently
overwriting previous records or repeatedly opening duplicates.

Approach: Schedule source-specific resolvers, compare against data indexes,
deduplicate by product/version/channel/platform, and open or update one tracked
capture request.

Prove: fixtures create one request for a new stable release, none for a preview
or known release, and no duplicate after repeated runs.

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
