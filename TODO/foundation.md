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
Priority: P0 | Effort: L | Status: DONE

Problem: Floating dependencies can change the observer or publisher without a
reviewed repository change.

Premise: Cargo.lock plus immutable action SHAs are necessary but do not by
themselves prove dependency provenance.

Approach: Commit the lockfile, deny unreviewed Git dependencies, inventory all
action SHAs, and add an update procedure with changelog review.

Decision: no separate register of pins. The lockfile is the crate inventory and
the workflow line is the action inventory, so a register would be a third copy
of commits that already live in two places with a check between them. The
inventory requirement is met instead by making both places checkable: a
lockfile entry must carry a crates.io source and a checksum, and an action pin
must carry the version comment that `check-remote-items` resolves against the
tag it names.

Prove: `cargo metadata --locked --format-version 1` succeeds and the project
checker rejects a workflow action referenced by a tag.

Closure evidence: `cargo metadata --locked --format-version 1` exits 0. The
pin rule was inverted from a denylist of known floating refs to an allowlist of
immutable forms, and seven defects were planted against the result on
2026-09-04, each refused by both halves with the same exit code: a tag, a
branch name, an abbreviated commit, a pin with no version comment, a lockfile
entry with a git source, a lockfile entry with no checksum, and a manifest git
dependency.

⭐ Three of those seven passed the old rule: a branch called anything other
than `main` or `master`, an abbreviated commit, and a bare commit with no
comment. That is measured, by running the old expression against each planted
case, and it is the argument for an allowlist over a denylist.

[`../docs/supply-chain.md`](../docs/supply-chain.md) carries the three layers
and the update procedure.

Capability note: `pwsh` was absent at session start, so the PowerShell halves
were unexercised. It was installed from the upstream release tarball, and both
halves were then compared on every planted case above rather than on a clean
tree alone. ⚠ `check-twins` compares answers on the tree it runs against, so a
rule that only differs on a defect is invisible to it; that gap is why the
comparison was run per mutation.

Door sweep findings, both fixed: the pin rule read `.github/workflows/` only,
so a composite action under `.github/actions/` would have carried unchecked
`uses:` lines with the same permissions. The scope now covers both and was
proven with a throwaway composite action carrying a floating pin, refused by
both halves. And the enumeration turned up a fourth layer the rule cannot
reach at all, `go install mvdan.cc/sh/v3/cmd/shfmt@v3.14.0` in a `run:` script;
it is safe for a reason particular to Go's checksum database rather than by
the form of the pin, so it is argued in `docs/supply-chain.md` rather than
waved through.

Residual: `check-remote-items` verifies the pins a pull request proposes, not
the pins already in the tree, so a tag moved or deleted upstream after a merge
is not detected. Named in `docs/supply-chain.md` and carried by `CI-04`. ⚠ It
needs `gh` and was not run in this session; the CI Linux lane runs it.

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
