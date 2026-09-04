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
Priority: P0 | Effort: L | Status: OPEN

Problem: A normalized profile without acquisition and packet provenance cannot
be independently replayed or audited.

Premise: Immutable byte captures, connector outputs, logs, versions, clocks,
host facts, and hashes are sufficient to reproduce a normalization verdict.

Approach: Define canonical manifests with content-addressed evidence paths,
redaction declarations, tool versions, and ordered run phases.

Prove: `cargo test --workspace evidence_manifest` round-trips a complete run
and rejects missing digests, connector versions, or acquisition identity.

## SCHEMA-03: Connector agreement and conflict model

Source: operator requirement for at least two correlated connectors
Priority: P0 | Effort: L | Status: OPEN

Problem: Two observers can disagree through a parser defect, timing effect, or
client variability; silently choosing one corrupts the corpus.

Premise: Agreement is meaningful only for explicitly overlapping fields and
after both raw observations remain available.

Approach: Model independent observations, comparable projections, agreement,
non-overlap, and blocking conflicts. Require an adjudication record for any
later correction.

Prove: `cargo test --workspace agreement` proves conflicts are unpublishable
and that non-overlapping observations are not falsely called agreement.

## SCHEMA-04: Variability and repeated-sampling model

Source: bit-cli random peer-ID suffix and tracker key behavior
Priority: P0 | Effort: M | Status: OPEN

Problem: One run cannot distinguish stable bytes, per-process randomness,
per-torrent randomness, and platform-dependent behavior.

Premise: Controlled restart and torrent permutations expose these classes
without source inference.

Approach: Define a sampling plan, observation scope, minimum repeats, and
confidence-free exact classifications derived only from captured runs.

Prove: fixture tests classify fixed prefixes and changing suffixes correctly
and reject a variability claim backed by one sample.
