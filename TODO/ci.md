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
