# Foundation entries

## FOUND-01: Repository workspace and policy skeleton

Source: operator request and Azathothas/TEMPLATE bootstrap method
Priority: P0 | Effort: L | Status: DONE

Problem: An empty remote cannot support repeatable research or unattended CI.

Premise: A lean Rust workspace, explicit 0BSD licence, routed documentation,
target catalogue, work corpus, and local gate are the minimum coherent base.

Approach: Adopt the template's durable policies and checks, remove template
scaffolding, and specialize every front door for bit-ids.

Prove: `cargo test --workspace`, both fast gate runners, and the three review
passes named in `docs/history/BOOTSTRAP-REVIEW.md` pass before the bootstrap
commit is pushed.

Closure evidence: the bootstrap commit contains the complete skeleton and the
commands and review findings are recorded in
[`../docs/history/BOOTSTRAP-REVIEW.md`](../docs/history/BOOTSTRAP-REVIEW.md).

## FOUND-02: Reproducible Rust dependency and action pins

Source: b-ids workflow architecture and template public-CI policy
Priority: P0 | Effort: L | Status: OPEN

Problem: Floating dependencies can change the observer or publisher without a
reviewed repository change.

Premise: Cargo.lock plus immutable action SHAs are necessary but do not by
themselves prove dependency provenance.

Approach: Commit the lockfile, deny unreviewed Git dependencies, inventory all
action SHAs, and add an update procedure with changelog review.

Prove: `cargo metadata --locked --format-version 1` succeeds and the project
checker rejects a workflow action referenced by a tag.

## FOUND-03: Deterministic protocol fixture suite

Source: bit-cli peer, tracker, and loopback tests; reference sweep
Priority: P0 | Effort: L | Status: OPEN

Problem: Live captures alone cannot distinguish an observer regression from a
client behavior change.

Premise: Byte-exact fixtures for announces, handshakes, extended handshakes,
and message sequences can exercise parsers without external software.

Approach: Store small generated fixtures with documented provenance and assert
both parse output and deterministic re-encoding in Rust tests.

Prove: `cargo test --workspace fixtures` passes twice with identical fixture
digests.

## FOUND-04: Third-party licence and redistribution register

Source: public-repository policy and proprietary client scope
Priority: P1 | Effort: M | Status: OPEN

Problem: Free observation data does not grant permission to redistribute
installers, client assets, or third-party code.

Premise: The corpus can publish our measurements while retaining only hashes
and source locations for binaries that cannot be redistributed.

Approach: Record licence, artifact redistribution rule, source evidence, and
required notices for every dependency and acquisition route.

Prove: the licence checker reports every catalogue target and dependency with
a non-empty disposition and rejects bundled proprietary artifacts.
