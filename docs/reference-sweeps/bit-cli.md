# bit-cli reference sweep

This sweep was performed on 2026-09-04 before the bit-ids architecture and
work order were finalized. It separates mechanisms worth adopting from data
sources that this project must not treat as measurement.

## Material and depth

| repository or source | revision or instant | depth |
| --- | --- | --- |
| Azathothas/TEMPLATE | [`620616638320`](https://github.com/Azathothas/TEMPLATE/commit/620616638320) | bootstrap method, chosen todo model, conventions, security, gate, scripts |
| Azathothas/b-ids | [`e7e521edaafd`](https://github.com/Azathothas/b-ids/commit/e7e521edaafd) | profile/evidence design, Rust crate boundaries, data branch, deterministic publishing, CI |
| Azathothas/bit-cli | [`cce8131231ab`](https://github.com/Azathothas/bit-cli/commit/cce8131231ab) | two code passes, tracker search, issue and discussion sweep |
| Wikipedia usage-share page | retrieved 2026-09-04 | scope cross-check; newest useful table on page is March 2020 |
| TorrentAnalytics August 2026 view | retrieved 2026-09-04 | prioritization sample only; its passive collection is not profile evidence |

The bit-cli tracker sweep found nine open items, all dependency update pull
requests, no standalone issues, no identity-related comments, and no
discussions at that instant. This limits tracker evidence; it does not imply
that the implementation has no identity defects.

## bit-cli pass one: shape and intent

The first pass read the requested files in full, followed their documentation
references, and mapped the feature surface.

`crates/bit-cli-core/src/peer_id.rs` owns one process-wide identity. It emits a
20-byte printable peer ID with the fixed `-CL` client code and a base-62 random
suffix. Central ownership and byte-level tests are the important pattern.

`scripts/make-client-profile.ps1` is a guarded source-derived generator for
qBittorrent and Transmission profiles. It resolves release tags, applies
product-specific formatting and checksum rules, verifies invariants, and fails
closed. `scripts/check-client-profile.ps1` independently checks the committed
output. These are useful examples of deterministic generation and independent
validation, but their source-reading method is prohibited as bit-ids corpus
evidence.

The surrounding peer documentation and T-234 show that identity is
multi-surface: peer ID, HTTP tracker user agent, headers, query ordering and
keys, UDP tracker values, handshake reserved bits, BEP 10 values and key set,
early message order, MSE choices, and web-seed behavior.

## bit-cli pass two: call sites and failure history

The second pass traced how identity reaches the network. `engine.rs` creates a
session peer ID and places it in session options; `trackers.rs` creates a
tracker peer ID; `tracker.rs` serializes binary peer ID and info hash values in
a deliberately tested query order; benchmark and web-seed code introduce
additional controlled roles. The tests explicitly assert one peer-ID prefix
across relevant paths.

T-236 records why that matters: earlier paths exposed six inconsistent peer
identities, including rQ- and BitComet-like values, until identity was
centralized and a live announce check proved the fix. The lesson for bit-ids
is that static code inspection can miss the value actually emitted by another
path. The corpus therefore accepts only active observations from installed
builds.

## TEMPLATE and b-ids conclusions

The TEMPLATE bootstrap is a dependency-ordered adoption method rather than a
directory copy. bit-ids selected its todo model, standalone agent router,
public-repository safety rules, twin checks, and three-part gate. Project facts
replace template choices and template-only directories are not retained.

b-ids demonstrates a profile at one exact product build, platform, channel,
and capture instant with raw evidence and per-field provenance. Its useful
publishing properties are an append-only data branch, one deterministic
assembly whose bytes feed every destination, immutable action references,
least-privilege jobs, and consumer-oriented derived formats. bit-ids adopts
those properties while keeping its initial crate and workflow set deliberately
small.

## Adopted conclusions and work links

| conclusion | resulting work |
| --- | --- |
| Model a product profile at one version, platform, route set, and capture instant. | [`SCHEMA-01`](../../TODO/schema.md#schema-01-versioned-identity-profile-schema), [`SCHEMA-02`](../../TODO/schema.md#schema-02-raw-evidence-and-run-manifest-schema) |
| Capture the full multi-surface identity rather than only peer ID. | [`OBS-02 through OBS-06`](../../TODO/observer.md) |
| Centralize protocol parsing and make raw bytes replayable. | [`FOUND-03`](../../TODO/foundation.md#found-03-deterministic-protocol-fixture-suite), [`OBS-01`](../../TODO/observer.md#obs-01-isolated-rust-loopback-observation-lab) |
| Require independent corroboration and block conflicts. | [`SCHEMA-03`](../../TODO/schema.md#schema-03-connector-agreement-and-conflict-model), [`OBS-07`](../../TODO/observer.md#obs-07-known-client-positive-controls) |
| Treat version formulas and source tables only as hypotheses. | [`ACQ-02`](../../TODO/acquisition.md#acq-02-latest-stable-release-resolver), [`CLIENT-06`](../../TODO/clients.md#client-06-transmission-capture-adapter) |
| Preserve every stable release and derive consumer views deterministically. | [`CORPUS-01`](../../TODO/corpus.md#corpus-01-append-only-canonical-store), [`PUB-01`](../../TODO/publishing.md#pub-01-deterministic-release-assembler) |
| Let downstream Rust tools consume the measured result without duplicating policy. | [`LIB-01`](../../TODO/library.md#lib-01-rust-consumer-library), [`LIB-02`](../../TODO/library.md#lib-02-bit-cli-integration-adapter) |

## Rejected shortcuts

The following may guide an experiment but can never populate a published
field: reading a client's source or release script, copying a peer-ID table,
decoding identities from a third-party crawler, inferring one product from a
shared engine, or accepting two parsers that consume the same normalized
intermediate output as independent connectors.

TorrentAnalytics describes DHT crawling, connecting to discovered peers, and
extracting client information from peer IDs and extended headers. That is
useful for prioritizing coverage. It is passive third-party observation of
unknown installations and therefore cannot satisfy the controlled acquisition,
exact-version, two-route, or two-connector rules in
[`../capture-methodology.md`](../capture-methodology.md).
