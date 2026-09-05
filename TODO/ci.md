# CI entries

## CI-01: Complete cross-platform quality gate

Source: template gate method and operator best-in-class CI requirement
Priority: P0 | Effort: L | Status: OPEN

Problem: The bootstrap CI verifies code and documents but not future schema,
fixtures, corpus determinism, or publication invariants.

Approach: Add focused Linux and Windows jobs, immutable action pins, explicit
permissions, concurrency, timeouts, caching by lockfile, and strict local gates.

Prove: every required check appears as a non-skipped CI result, and injected
failures in docs, schema, fixtures, and Rust each turn the workflow red.

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
Priority: P1 | Effort: S | Status: OPEN

Problem: An entry's `Prove` is the acceptance, and one of them ran nothing while
exiting 0. `cargo test --workspace --locked lab_supervisor` filters by test
**name**; the name matched none, so every binary in the workspace printed
`running 0 tests` and the command succeeded. Nothing in the gate can tell that
from an acceptance that passed.

Premise: Measured on 2026-09-05, not read. The nine `cargo test` acceptance
commands in `TODO/` were all of the bare-filter form, and they worked only
because of a convention nothing checks: all 110 test functions across the eight
pre-existing test files begin with their file's name. `lab_supervisor` did not,
and the acceptance went green over zero tests. All nine were rewritten to name a
target or a package.

Approach: A rule in `scripts/common/check-project.sh` and its PowerShell twin
that reads every `cargo test` invocation inside a code span in `TODO/*.md` and
refuses a bare word argument unless it follows `-p`, `--package`, `--test`,
`--example` or `--bin`. ⚠ The parsing is the work: a code span can wrap across
lines, so the file is joined before the spans are found, and the two twins must
agree per planted mutation rather than on a clean tree.

Decision: a shape rule over a naming convention. Renaming every test function to
start with its file's name would also make the bare filter work, and it makes
the filter's correctness depend on a convention no check holds, while
`docs/conventions/code.md` asks for test names that describe behaviour.

Prove: `sh scripts/common/check-project.sh` and
`pwsh -File scripts/common/check-project.ps1` both exit 1 when a `Prove` line is
rewritten to the bare-filter form, both exit 0 on the tree as it stands, and
both agree on every planted mutation.
