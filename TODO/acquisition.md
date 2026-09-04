# Acquisition entries

## ACQ-01: Acquisition route schema

Source: operator same-version multi-source requirement
Priority: P0 | Effort: L | Status: OPEN

Problem: Package, official download, release artifact, and source-build routes
need comparable provenance despite different metadata.

Premise: Resolver input, immutable source identity, retrieved bytes, installed
version output, and host facts form a common route record.

Approach: Define typed route kinds and evidence requirements; do not treat the
candidate routes in the catalogue as verified availability.

Prove: schema tests cover all catalogue route kinds and reject an acquisition
without a content digest or observed installed version.

## ACQ-02: Latest stable release resolver

Source: operator latest-stable-only scope and bit-cli tag-selection guards
Priority: P0 | Effort: L | Status: OPEN

Problem: Lexical tag sorting and package channel labels can select previews,
older trains, or incomparable versions.

Premise: Stable selection must be product-specific, time-stamped, fail closed,
and retain every candidate considered.

Approach: Implement Rust resolvers with explicit prerelease rules and shell
fetch wrappers. Record source responses; never derive profile fields from
release source code.

Prove: resolver fixtures cover stable, prerelease, missing, divergent, and
non-semantic version sets, and a live dry run emits a complete decision trace.

## ACQ-03: Same-version multi-route verifier

Source: operator requirement to install the same build from two or more routes
Priority: P0 | Effort: L | Status: OPEN

Problem: Equal display versions can identify repackaged or patched binaries,
while different version formats can identify the same upstream build.

Premise: Installed version output, upstream version mapping, artifact hashes,
binary metadata, and observed profile comparison can make equality explicit.

Approach: Require at least two independent routes on the same host family;
classify byte-identical, build-equivalent, divergent, or unresolved and block
publication for the last two.

Prove: an end-to-end fixture accepts two equivalent routes and rejects version
label equality when binary or observed identity evidence conflicts.

## ACQ-04: Disposable-host execution boundary

Source: proprietary installers and active network client execution
Priority: P0 | Effort: L | Status: OPEN

Problem: Running untrusted clients on persistent CI hosts can leak state,
escape the lab, contaminate later samples, or violate package terms.

Premise: Fresh VM images, restricted egress, non-secret accounts, and verified
teardown can make captures reproducible and bounded.

Approach: Define Linux and Windows runner contracts, deny public BitTorrent
traffic, snapshot inputs, wipe the guest, and attest cleanup after every run.

Prove: the runner test detects forbidden egress and persistent state, then
shows a fresh host fingerprint for the next job.

## ACQ-05: Artifact cache and authenticity evidence

Source: repeatability and upstream availability risk
Priority: P1 | Effort: M | Status: OPEN

Problem: Upstream URLs and package indexes change, but storing installers may
be disallowed.

Premise: Hashes, signatures, certificate metadata, package receipts, and
permitted artifacts can preserve verification without illegal redistribution.

Approach: Implement a policy-aware cache that stores bytes only when allowed
and otherwise stores sufficient authenticity and retrieval evidence.

Prove: cache tests enforce the licence register and reproduce artifact
identity after a simulated source URL change.
