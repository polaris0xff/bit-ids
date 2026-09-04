# Acquisition entries

## ACQ-01: Acquisition route schema

Source: operator same-version multi-source requirement
Priority: P0 | Effort: L | Status: DONE

Problem: Package, official download, release artifact, and source-build routes
need comparable provenance despite different metadata.

Premise: Resolver input, immutable source identity, retrieved bytes, installed
version output, and host facts form a common route record.

Approach: Define typed route kinds and evidence requirements; do not treat the
candidate routes in the catalogue as verified availability.

Decision: the route record is one shape with a typed source identity, not one
record shape per kind. What the eight kinds have in common is the shape of the
claim, which is what the premise says: something decided a version, something
delivered bytes, the bytes have a digest, and the installed thing was asked its
own version. Only the identity of what was asked for genuinely differs, so that
is the enum and the rest is flat. Eight parallel record types was the rejected
alternative: it would have made every consumer match on the kind to read a
digest.

Decision: `SignatureStatus` moved out of the manifest module into the route
module. Both documents record it and `bind` compares them, so a second copy of
the enum is a value in two places that can disagree about what its own variants
mean.

Decision: the catalogue-vocabulary test scans `clients.toml` rather than adding
a `toml` dependency. One hand-maintained file read by one test does not carry
the argument the supply-chain rules require for a third-party crate. The scan
fails loud instead of quietly finding nothing: it refuses a result too small to
be real, and that guard was planted against by renaming the key it reads.

Prove: schema tests cover all catalogue route kinds and reject an acquisition
without a content digest or observed installed version.

Closure evidence: run on 2026-09-04. `cargo test --workspace --locked
--all-targets` is 123 passed, 0 failed. `tests/acquisition.rs` asserts every
`candidate_routes` entry in the catalogue parses as a `RouteKind` and every
variant is asked for by the catalogue, over 42 scanned entries. A content digest
and an installed version are non-optional fields, so a record without either
fails to deserialize; what needed new invariants was everything around them.

⭐ The route record grew six invariants, each planted against in
[`../crates/bit-ids/tests/profile_schema.rs`](../crates/bit-ids/tests/profile_schema.rs):

- `E-ACQ-05` a kind carrying another kind's source identity, which reads as a
  complete record and names nothing anyone can resolve again;
- `E-ACQ-06` an abbreviated commit, the shape `FOUND-02` measured passing an
  action-pin rule written to refuse floating refs;
- `E-ACQ-07` and `E-ACQ-08` two routes sharing a resolver or a delivery
  mechanism. ⛔ This is the one that matters. Without it the two-route rule is
  satisfiable by asking one index twice under two names, which is the failure
  `architecture.md` section 7 exists to prevent rather than a technicality;
- `E-ACQ-09` an installed version citing evidence that is not in the record;
- `E-ACQ-10` one citing evidence that is not process output;
  [`../docs/architecture.md`](../docs/architecture.md) section 7 carries why.

`E-BND-13` compares the signature disposition across the two documents. It was
written after checking it can fail, which `E-BND-10` could not: nothing else
forces the run's record and the profile's into step, so `verified` over a run
that says `not_checked` is publishable without it.

Driven pass: the two example validators were run against the golden pair, which
they accept, and against three hand-planted records. Every refusal named the
route, the field and the reason. ⚠ It found a real defect the suite could not:
`E-BND-13` printed the Rust `Debug` spelling, telling an operator the run
recorded `Unsigned` over a document that says `unsigned`, and sending them to
look for a value written nowhere. `SignatureStatus` now has one canonical
spelling and the diagnostic quotes it.

⚠ Found while wiring the bind check: `E-BND-12` was already taken by the
capture-instant comparison, and the new code silently reused it. Two checks
under one code is worse than an unclear message, because a caller acting on the
class cannot tell which fired. Renumbered to `E-BND-13`, and the manifest
coverage test refused the change until it had a planted defect.

Residual: `origin` is an HTTPS URL for every kind, including a package-manager
route where the natural reference is a pool path rather than a page. That has
held for every catalogue route so far and is recorded here rather than assumed
to hold forever.

Residual: a bare `serde_json::from_str::<AcquisitionRoute>` produces an
unvalidated route. That is true of `Build` and `Capture` too and is not a new
door: the route-level invariants need the build tuple and the evidence list, so
a route alone cannot answer them. `Profile` remains the validated boundary.

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
