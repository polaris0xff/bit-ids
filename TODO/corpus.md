# Corpus entries

## CORPUS-01: Append-only canonical store

Source: b-ids data branch architecture and operator publication requirement
Priority: P0 | Effort: L | Status: OPEN

Problem: Regenerating a latest-only dataset would erase older stable-release
records and destroy auditability.

Approach: Store run manifests and profiles under immutable product, version,
platform, route-set, and capture identifiers. New releases append; corrections
add records rather than rewriting evidence.

Prove: the validator rejects deletion or byte changes against the prior data
branch and accepts a new version directory.

## CORPUS-02: Semantic corpus validator

Source: operator accuracy requirement
Priority: P0 | Effort: L | Status: OPEN

Problem: Schema validity alone cannot prove route count, connector independence,
field provenance, agreement, stable status, or evidence reachability.

Approach: Implement all publication invariants in Rust with stable diagnostic
codes and adversarial fixtures.

Prove: `cargo test --workspace --locked --test corpus_validator` rejects one fixture for each
invariant and validates the complete golden corpus.

## CORPUS-03: Deterministic indexes and latest views

Source: b-ids consumer-oriented indexes
Priority: P0 | Effort: L | Status: OPEN

Problem: Consumers need convenient latest and lookup views without making
those derived files authoritative.

Approach: Generate sorted indexes by client, peer prefix, BEP 10 client value,
platform, version, and capture instant from canonical records only.

Prove: two clean builds have identical digests and every index row resolves to
one canonical profile.

## CORPUS-04: Supersession and correction records

Source: append-only publication constraint
Priority: P1 | Effort: M | Status: OPEN

Problem: A proven bad record must stop appearing in current views without
deleting the historical evidence.

Approach: Define signed correction records naming the original digest, reason,
replacement, and review evidence; derive current views accordingly.

Prove: fixtures retain the original bytes, exclude a superseded record from
latest views, and expose the full correction chain.
