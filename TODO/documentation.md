# Documentation entries

## DOC-01: Public data and library documentation

Source: operator publication requirement
Priority: P1 | Effort: M | Status: OPEN

Problem: Formats and Rust APIs are unusable without field semantics, examples,
compatibility rules, and integrity instructions.

Approach: Generate field references from the schema where possible and add
small verified examples for raw URLs, release assets, SQLite, and Rust lookup.

Prove: documentation examples run in CI against the assembled fixture release.

## DOC-02: Contributor capture-run handbook

Source: future external contribution path
Priority: P2 | Effort: M | Status: OPEN

Problem: A contributor can submit plausible output that lacks isolation,
two-route identity, independent observation, or redistribution review.

Approach: Document host preparation, safe acquisition, lab execution, evidence
review, correction flow, and exactly what cannot be accepted.

Prove: a clean-room walkthrough follows only the handbook and produces a
validator-accepted fixture submission without undocumented steps.
