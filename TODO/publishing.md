# Publishing entries

## PUB-01: Deterministic release assembler

Source: b-ids one-time assembly design
Priority: P0 | Effort: L | Status: OPEN

Problem: Multiple jobs rebuilding formats can publish different bytes under
one release label.

Approach: Validate canonical records, assemble all formats once in a clean
workspace, generate a checksum manifest, and pass that immutable bundle to
every destination.

Prove: two independent assembly runs produce byte-identical archives,
databases, indexes, and checksum manifests.

## PUB-02: Protected append-only data branch publisher

Source: operator request and b-ids data branch
Priority: P0 | Effort: L | Status: OPEN

Problem: A branch publisher can accidentally force-push, drop prior records,
or expose partial output.

Approach: Publish from a validated temporary tree with ancestry checks,
least-privilege permissions, concurrency control, and a single atomic update.

Prove: integration tests refuse non-fast-forward and deletion scenarios, then
append a fixture release while preserving every prior digest.

## PUB-03: Multi-format GitHub release publisher

Source: operator request for many formats and paths
Priority: P1 | Effort: L | Status: OPEN

Problem: JSON alone is inconvenient for streaming, tabular analysis, embedded
tools, and compact transfer.

Approach: Publish deterministic JSON, JSONL, CSV, SQLite, and CBOR plus checksums
and schema documentation, all derived from the one assembled bundle.

Prove: release asset digests match the assembled manifest and cross-format
tests reconstruct equivalent normalized records.

## PUB-04: Stable raw and index access paths

Source: operator direct-GitHub access requirement
Priority: P1 | Effort: M | Status: OPEN

Problem: Consumers need documented immutable and current URLs without scraping
the repository UI.

Approach: Publish versioned raw paths, content-addressed evidence paths, latest
indexes, release assets, and integrity metadata with explicit stability rules.

Prove: a link checker fetches every documented path through the approved
GitHub read route and verifies its digest.
