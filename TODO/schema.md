# Schema entries

## SCHEMA-01: Versioned identity profile schema

Source: bit-cli T-234 surface inventory and b-ids profile model
Priority: P0 | Effort: L | Status: DONE

Problem: A peer ID alone cannot describe a BitTorrent client's observable
identity, and an unversioned record cannot evolve safely.

Premise: Identity spans peer ID, tracker requests, reserved bits, BEP 10,
early peer messages, and adjacent protocols.

Approach: Define a strict Rust-owned schema with explicit unknown, absent,
constant, patterned, and variable states plus per-field evidence references.

Decision: `serde`, `serde_json` and `sha2` are dependencies rather than
hand-written parsers. A JSON reader written here would be a new silent-corruption
surface in the one layer that must never corrupt, and `deny_unknown_fields` is
exactly the strictness the record needs. The alternative, a zero-dependency
crate, lost on that risk; `FOUND-02` pins what was added and `Cargo.lock` is
committed, which CI already enforces with `--locked`.

Decision: absence, `not_observed` and `not_supported`, requires an evidence
entry of kind `positive_control`. Without it an observer that was never
listening and a build that never answered produce the same record, and
`docs/architecture.md` section 5 already said absence needs a control.

Decision: `id` is derived from the identity tuple and re-derived on validation
rather than stored independently. Storing it loose would put one value in two
places with nothing checking that they agree.

Prove: `cargo test --workspace profile_schema` validates golden records and
rejects unknown schema versions and unproven fields.

Closure evidence: `cargo test --workspace profile_schema` reports 22 passed,
0 failed on 2026-09-04. `cargo fmt --all -- --check`,
`cargo check --workspace --locked --all-targets`,
`cargo test --workspace --locked --all-targets`,
`cargo clippy --workspace --locked --all-targets -- -D warnings`,
`cargo test --workspace --locked --doc` and `sh scripts/common/check-gate.sh`
all exit 0, each read from its own process.

The record shape is [`../crates/bit-ids/src/record.rs`](../crates/bit-ids/src/record.rs),
the field states [`../crates/bit-ids/src/observation.rs`](../crates/bit-ids/src/observation.rs),
the invariants [`../crates/bit-ids/src/validate.rs`](../crates/bit-ids/src/validate.rs)
and the one read and write path
[`../crates/bit-ids/src/json.rs`](../crates/bit-ids/src/json.rs).
[`../docs/architecture.md`](../docs/architecture.md) section 4 is the reference.

Every diagnostic code has a planted defect proving it refuses; a test reads the
validator's own source and fails when a code has none. Five further mutations
were run by hand and each turned the suite red: disabling the unproven-field
guard failed 3 tests, disabling the positive-control guard failed 2, deleting a
row from the planted table named the uncovered code, pointing the code scan at
the wrong file tripped its own sanity assertion, and removing the record-id
domain separator failed 9.

Door sweep finding, fixed: `Profile` derived `Deserialize`, so
`serde_json::from_str::<Profile>` returned an unvalidated record while the
crate documentation said `from_json` was the only way in. Reproduced with a
throwaway example that read `unproven-field.json` and got a record back. The
derive moved to a private field mirror, `Deserialize for Profile` now
validates, and
`profile_schema_validates_on_every_serde_route_not_just_from_json` holds it.

Claim audit findings, fixed: the mutation count above was read from truncated
output and said 7; `cargo clippy` had been run without `--all-targets`, which
does not lint test code and was hiding a real failure; and a changelog sentence
described an earlier draft of the absence rule that never existed.

Driven pass: `cargo run --example validate-profile -- PATH` over all four
fixtures, a missing file and no argument. Exit 0 on both valid records, 1 on
the unsupported schema and on the unproven field with `E-OBS-05` and its field
path on stderr, 2 on the two routes that could not run. That is the failure
semantics in
[`../docs/capture-methodology.md`](../docs/capture-methodology.md), read from
the process rather than through a pipe.

Residual: `build.platform`, `build.arch` and `build.package` are validated as
identifiers, not against the catalogue vocabulary. Closing that needs the
catalogue reader, which is `CORPUS-02`.

## SCHEMA-02: Raw evidence and run manifest schema

Source: b-ids evidence bundles and operator accuracy requirement
Priority: P0 | Effort: L | Status: DONE

Problem: A normalized profile without acquisition and packet provenance cannot
be independently replayed or audited.

Premise: Immutable byte captures, connector outputs, logs, versions, clocks,
host facts, and hashes are sufficient to reproduce a normalization verdict.

Approach: Define canonical manifests with content-addressed evidence paths,
redaction declarations, tool versions, and ordered run phases.

Decision: the manifest is a second document beside the raw bytes, not a larger
`capture` section inside the profile. A replay needs the whole run and a
consumer of the catalogue needs only the record, and one document serving both
would make every consumer carry the run. The cost is an overlap, which `bind`
pays for by comparing every shared value.

Decision: the content-addressed path is derived from the digest rather than
stored beside it. A stored path is a second copy of the digest that can
disagree with the bytes it names.

Decision: phases are the state machine in `docs/architecture.md` section 8, and
a run advances one step at a time or falls to `provisional`. Skipping is
refused because a phase nobody ran is a phase nobody can produce evidence for.

Prove: `cargo test --workspace evidence_manifest` round-trips a complete run
and rejects missing digests, connector versions, or acquisition identity.

Closure evidence: `cargo test --workspace evidence_manifest` reports 15 passed,
0 failed on 2026-09-04. The three rejections the acceptance names are covered:
a record with `sha256` removed is refused during deserialization, a connector
whose version differs between the documents is `E-BND-07`, and acquisition
identity is `E-BND-09` and `E-BND-11` across the pair plus `E-MAN-10` through
`E-MAN-13` within the run. `cargo fmt --all -- --check`, `cargo check`,
`cargo test` and `cargo clippy` at `--workspace --locked --all-targets`,
`shellcheck`, `shfmt -d -i 2 -ci` and `sh scripts/common/check-gate.sh` all
exit 0.

Every diagnostic code has a planted defect, 30 for the manifest and 10 for the
binding, and a test reads the module's own source and fails when a code has
none. Three further mutations were run by hand: dropping a row from the binding
table named the uncovered code, disabling the redaction cross-check failed 2
tests, and letting a run skip a phase failed 2.

Driven pass: `cargo run --example validate-run -- MANIFEST PROFILE`. Exit 0 on
the golden pair; exit 1 with `E-BND-01` and `E-BND-12` when the manifest is
paired with the profile of a different capture run of the same build, which is
the realistic version of the mistake the two-document design exists to catch;
exit 1 on a refused manifest and exit 2 on the two routes that could not run.

⭐ Guard-mutation finding, fixed: `E-BND-10` compared the installed version
across the two documents and could not fail. The manifest already requires
every route to have installed its own recorded version, the profile requires
the same of its build, and `E-BND-03` requires those to agree, so the fourth
comparison was unreachable while the other three held. It was found while
trying to plant a defect for it, and removed rather than left as a guard nobody
knows works.

Door sweep: the only route from bytes to a `RunManifest` is `from_json`, and
`RunManifest` does not derive `Deserialize` for the reason `SCHEMA-01` found
the hard way; `evidence_manifest_validates_on_every_serde_route_not_just_from_json`
holds it. `bind` answers agreement and not validity, which is now stated on it,
because a caller who built a document in memory has not had its own invariants
run.

Residual: `acquisition` carries the identity a replay needs, not the full route
record. Resolver evidence, package metadata and the mirrors tried belong to
`ACQ-01`.

## SCHEMA-03: Connector agreement and conflict model

Source: operator requirement for at least two correlated connectors
Priority: P0 | Effort: L | Status: DONE

Problem: Two observers can disagree through a parser defect, timing effect, or
client variability; silently choosing one corrupts the corpus.

Premise: Agreement is meaningful only for explicitly overlapping fields and
after both raw observations remain available.

Approach: Model independent observations, comparable projections, agreement,
non-overlap, and blocking conflicts. Require an adjudication record for any
later correction.

Decision: validity and publishability are separate gates. A disagreement has to
be recordable or the project loses the evidence of one, and
`docs/architecture.md` section 8 already says such a run moves to provisional
with its evidence retained. So `validate` accepts a record carrying a conflict
and `publishable` refuses it. Folding the two together would have made the
conflict unwritable, which is the opposite of keeping it.

Decision: a connector that cannot see a surface says so, in `out_of_scope`,
rather than being left out. Left out, a single observation looks like a pair
that happened to agree; named, it is the reason that connector's silence proves
nothing.

Decision: `corroboration` replaced its `connectors` list with per-connector
observations, so the record keeps what each one saw rather than a verdict. The
list was derivable from the observations and keeping both would have been one
value in two places.

Prove: `cargo test --workspace agreement` proves conflicts are unpublishable
and that non-overlapping observations are not falsely called agreement.

Closure evidence: `cargo test --workspace agreement` reports 14 passed, 0
failed on 2026-09-04, and the whole suite is 53 passed, 0 failed.
`agreement_refuses_to_publish_a_conflict` is the first half of the acceptance
and `agreement_is_not_claimed_over_a_field_one_connector_could_not_see` the
second. `cargo fmt --all -- --check`, `cargo check`, `cargo test` and
`cargo clippy` at `--workspace --locked --all-targets`, `shellcheck`,
`shfmt -d -i 2 -ci` and `sh scripts/common/check-gate.sh` all exit 0.

Every diagnostic code has a planted defect: `E-COR-01` through `E-COR-19`,
`E-NRM-01`, `E-NRM-02` and `E-ADJ-01` through `E-ADJ-05` in
`tests/profile_schema.rs`, and `E-PUB-01` and `E-PUB-02` in `tests/agreement.rs`,
each with a coverage test that reads the module's own source. Three further
mutations were run by hand: letting an overlap of one be called agreement
failed 2 tests, letting a conflict be published failed 4, and allowing a lossy
normalization failed 2.

Driven pass: `cargo run --example validate-profile -- RECORD`. The golden
record prints `publishable` and exits 0. A record whose connectors disagree on
`peer_wire/reserved` still reads and validates, prints
`provisional, not publishable` with `E-PUB-01`, and exits 1. That is the
distinction the entry exists for, shown on the real path.

⭐ Door-sweep finding, fixed: `is_publishable`, which answers for one outcome,
and `publishable`, which answers for a record, are two public rules with
nothing holding them together. `agreement_keeps_the_per_outcome_and_per_record_rules_in_step`
now drives a record through all four outcomes and asserts the two answers
match.

Residual: the record states which normalization was applied and that it
preserves order and unknown bytes. Nothing executes it, so the declaration is
an assertion by the capture tool rather than a property this crate verifies.
`OBS-07` owns the controls that would test one against real bytes.

## SCHEMA-04: Variability and repeated-sampling model

Source: bit-cli random peer-ID suffix and tracker key behavior
Priority: P0 | Effort: M | Status: DONE

Problem: One run cannot distinguish stable bytes, per-process randomness,
per-torrent randomness, and platform-dependent behavior.

Premise: Controlled restart and torrent permutations expose these classes
without source inference.

Approach: Define a sampling plan, observation scope, minimum repeats, and
confidence-free exact classifications derived only from captured runs.

Decision: a classification is a function of the samples, never a confidence. A
dimension the run did not vary yields `unknown` rather than a best guess, so a
value that held still inside one process is not called persistent: only a
restart separates a stored value from a regenerated one.

Decision: the plan lives in the run manifest and the claim lives in the
profile, and `bind` holds them together. A field claiming variation needs a run
that varied something, and a field cannot rest on more samples than the plan
could produce.

Prove: `cargo test --workspace variability` classifies fixed prefixes and
changing suffixes correctly from fixture samples and rejects a variability
claim backed by one sample. ⚠ The command is an amendment; the entry named the
acceptance without one, and the model requires the acceptance to be a command.

Closure evidence: `cargo test --workspace variability` reports 12 passed, 0
failed on 2026-09-04, and the whole suite is 65 passed, 0 failed.
`variability_separates_a_fixed_prefix_from_a_changing_suffix` is the first half
of the acceptance and `variability_refuses_to_call_anything_stable_from_one_sample`
the second. Each of the four lifetimes is classified from a plan that varied
the dimension it names, and `variability_will_not_name_a_dimension_the_run_did_not_vary`
holds the boundary. `cargo fmt --all -- --check`, `cargo check`, `cargo test`
and `cargo clippy` at `--workspace --locked --all-targets`, `shellcheck`,
`shfmt -d -i 2 -ci` and `sh scripts/common/check-gate.sh` all exit 0.

`E-BND-20` and `E-BND-21` connect the plan to the record and both are planted
against; the manifest coverage test refused the change until they were, which
is the guard working before anyone remembered to.

Residual: the classifier is not yet called by anything that writes a record.
A capture tool turns samples into a `FieldState` with it, and that tool is
`OBS-02` onward.
