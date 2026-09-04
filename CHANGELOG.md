# Changelog

Nothing is released yet. Entries accumulate here until the first
`v1.*` tag, which is what `PUB-03` creates.

## Unreleased

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
