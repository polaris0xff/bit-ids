# Consumer library entries

## LIB-01: Rust consumer library

Source: operator request for direct tool consumption and b-ids library pattern
Priority: P1 | Effort: L | Status: OPEN

Problem: Rust tools should not reimplement schema validation, downloads,
indexes, and profile selection.

Approach: Provide no-network parsing by default plus opt-in verified retrieval,
typed lookup APIs, explicit schema compatibility, and embedded minimal indexes.

Prove: public API tests load release fixtures, reject digest mismatch, and
select a platform/version profile without network access.

## LIB-02: bit-cli integration adapter

Source: reference repository consumer requirement
Priority: P1 | Effort: L | Status: OPEN

Problem: bit-cli currently owns a generated profile table and can drift from
the measured corpus.

Approach: Define an adapter or generated crate interface that bit-cli can adopt
without any write to its repository. Preserve bit-cli's centralized peer ID
and fail-closed invariants.

Prove: a local integration fixture consumes a published profile and bit-cli's
identity consistency tests continue to pass; upstream changes remain out of
scope unless separately authorized.
