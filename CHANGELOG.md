# Changelog

Nothing is released yet. Entries accumulate here until the first
`v1.*` tag, which is what `PUB-03` creates.

## Unreleased

### 2026-09-04T14:47:22Z

- Closed `SCHEMA-04`, and with it the whole schema group. A run records what it
  varied, and a classifier turns samples into per-byte lifetimes, so a peer ID
  comes out as the shape it actually has: a fixed prefix and a suffix the build
  regenerates. Record: [`TODO/schema.md`](TODO/schema.md).
- Nothing here is a confidence. A dimension the run never varied yields
  `unknown` rather than a guess, so a value that held still inside one process
  is not called persistent: only a restart separates a stored value from a
  regenerated one, and one sample yields `unknown` for everything.
- `bind` now refuses a field claiming variation from a run that varied nothing,
  and a field resting on more samples than the plan could produce. The manifest
  coverage test refused the change until both had a planted defect.
- Deployment: no data branch, release or capture service was created.

### 2026-09-04T14:40:23Z

- Closed `SCHEMA-03`. Corroboration now keeps what each connector saw, the
  artifact it read the value out of and what was applied before comparing,
  rather than a verdict and a list of names. Record:
  [`TODO/schema.md`](TODO/schema.md).
- A connector that cannot see a surface says so. Left out, a single observation
  looks like a pair that happened to agree, which is the easiest false
  agreement available; named, it is the reason that connector's silence proves
  nothing. An outcome of exact or normalized over fewer than two observers who
  could actually see the field is refused.
- Validity and publishability became separate gates, and both are needed. A
  disagreement has to be recordable or the project loses the evidence of one,
  so a conflicted record reads and validates; `publishable` is what refuses to
  ship it. A record that supersedes another now has to say why, out of the four
  causes a disagreement actually has.
- A normalization used to reach agreement must declare that order and unknown
  bytes survive it. That rule was written in the architecture and enforced by
  nothing.
- A door sweep found `is_publishable` and `publishable` answering the same
  question at different scopes with nothing holding them together. A test now
  drives a record through all four outcomes and asserts the two agree.
- Deployment: no data branch, release or capture service was created.

### 2026-09-04T14:29:09Z

- Closed `SCHEMA-02`. The run manifest records how a capture was produced: the
  host, the isolation it ran under, both clocks, every tool at the version that
  ran, where each artifact came from, the phases of the state machine the run
  walked, the content-addressed evidence and what was scrubbed from it. Record:
  [`TODO/schema.md`](TODO/schema.md).
- It is a second document rather than a larger section inside the profile, so
  a consumer of the catalogue does not have to carry a whole run. `bind`
  compares every value the two share, which is what stops a deliberate overlap
  from becoming drift. Pairing a manifest with the profile of a different run
  of the same build is refused.
- Absence stays as constrained as it is in a profile: a run that reached beyond
  loopback says why, a host that is not disposable is refused outright, a phase
  cannot be skipped, and an artifact marked redacted must say what was taken out
  of it so that "raw" cannot quietly mean "edited".
- One guard was written and then deleted. `E-BND-10` compared the installed
  version across the two documents and could not fail, because three existing
  invariants already implied it. It was found while trying to plant a defect
  for it, which is the only way that class of dead guard shows up.
- `ProfileError` is now `DocumentError` and carries the schema it expected. It
  covered both documents while naming only one, and its message told a reader
  with a manifest problem to go and look at the profile schema.
- Deployment: no data branch, release or capture service was created.

### 2026-09-04T14:11:29Z

- Closed `FOUND-02`. The action pin rule is now an allowlist of forms that are
  immutable by construction rather than a denylist of the floating ones
  somebody thought of, and the lockfile is checked for a crates.io source and a
  checksum on every package. Record: [`TODO/foundation.md`](TODO/foundation.md).
- Three shapes passed the old rule and are refused now: a branch named anything
  other than `main` or `master`, an abbreviated commit, and a bare commit with
  no version comment. The comment is load-bearing, because it is what
  `check-remote-items` resolves against the tag it claims to be.
- Added [`docs/supply-chain.md`](docs/supply-chain.md) with the three pinned
  layers and the procedure for updating one. No register of pins was added: the
  lockfile and the workflow already hold those commits, and a third copy would
  be the drift the rules exist to prevent.
- Deployment: no data branch, release or capture service was created.

### 2026-09-04T14:02:39Z

- Fixed `check-docs.ps1`, which resolved a link going up more than two
  directory levels to the wrong path and reported it broken. `[^/]+` matches
  `..` as readily as a directory name and PowerShell's `-replace` is global, so
  one call collapsed a real segment and the `../..` pair after it. It now
  replaces one leftmost match per pass, which is what the `sed` loop in the sh
  twin was already doing. Record:
  [`docs/conventions/forbidden-patterns.md`](docs/conventions/forbidden-patterns.md).
- The defect was latent from the bootstrap and surfaced by the first link in
  the tree that goes up four levels, added with the schema fixtures. That link
  is now the standing regression case: `check-twins` compares the two halves
  against it on every run.
- Deployment: no data branch, release or capture service was created.

### 2026-09-04T13:57:55Z

- Closed `SCHEMA-01`. The `bit-ids` crate carries the versioned profile record,
  the six field states, the derived record identifier, the canonical value
  forms and every publication invariant this schema owns, read and written
  through one validating path. Record:
  [`TODO/schema.md`](TODO/schema.md).
- The unproven-field rule is the point of it: a state that asserts anything
  about a build must cite recoverable evidence, and a claim that a build
  produced nothing must cite a positive control. Without the second half, an
  observer that was never listening and a build that never answered would be
  the same record.
- Added the first third-party crates, `serde`, `serde_json` and `sha2`, with
  the lockfile committed. A hand-written JSON reader was rejected: it would put
  a new silent-corruption surface in the one layer that must not corrupt.
- `Profile` no longer derives `Deserialize`. It did until the door sweep on
  this entry found that `serde_json::from_str::<Profile>` handed back an
  unvalidated record while the crate documentation said `from_json` was the
  only way in. The derive now sits on a private field mirror and every serde
  route validates.
- `check-no-secrets` grew two exclusions, for registry lockfile digests and for
  the algorithm-tagged digests and observed bytes a profile record is made of.
  Its long-hex rule had also been dropping whole lines to allow one item on
  them, so a credential beside a pinned action commit was never reported. Both
  halves now delete the allowed item and re-test what is left. Record:
  [`docs/conventions/forbidden-patterns.md`](docs/conventions/forbidden-patterns.md).
- Deployment: no data branch, release or capture service was created.

### 2026-09-04

- Added the initial repository foundation, research sweep, architecture,
  machine-readable target catalogue, Rust library skeleton, CI gate and full
  implementation backlog. Record: [`TODO/PROGRESS.md`](TODO/PROGRESS.md).
- Deployment: no data branch, release or capture service was created.
