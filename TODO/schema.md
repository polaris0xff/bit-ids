# Schema entries

## SCHEMA-01: Versioned identity profile schema

Source: bit-cli T-234 surface inventory and b-ids profile model
Priority: P0 | Effort: L | Status: OPEN

Problem: A peer ID alone cannot describe a BitTorrent client's observable
identity, and an unversioned record cannot evolve safely.

Premise: Identity spans peer ID, tracker requests, reserved bits, BEP 10,
early peer messages, and adjacent protocols.

Approach: Define a strict Rust-owned schema with explicit unknown, absent,
constant, patterned, and variable states plus per-field evidence references.

Prove: `cargo test --workspace profile_schema` validates golden records and
rejects unknown schema versions and unproven fields.

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
