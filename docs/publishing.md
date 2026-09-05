# Publishing

Nothing is published yet. This document is the contract the `PUB-*` and
`CI-*` entries implement.

## Data branch

The `data` branch is append-only and contains only generated publication
artifacts. It is never force-pushed. A run that assembles identical bytes
pushes nothing.

```text
LICENSE
MANIFEST.json
SHA256SUMS
raw/v1/<target>/<version>/<platform>/<arch>/<package>/<capture-id>/manifest.json
raw/v1/<target>/<version>/<platform>/<arch>/<package>/<capture-id>/...
profiles/v1/<target>/<version>/<platform>/<arch>/<package>/<capture-id>.json
routes/v1/<target>/latest/<platform>/<arch>.json
routes/v1/<target>/<version>/<platform>/<arch>.json
indexes/v1/profiles.json
formats/bit-ids-v1.json
formats/bit-ids-v1.jsonl
formats/bit-ids-v1.csv
formats/bit-ids-v1.sqlite3
formats/bit-ids-v1.cbor
```

⛔ **A record's path carries its whole identity tuple, and `<package>` is in it
because the record identifier digests it.** This layout omitted that segment
until `CORPUS-01` derived the path in code and compared the two: a `deb` and an
`AppImage` of one version on one platform are two records the identifier tells
apart, and they were one file. Whether they also differ in capture identifier is
not the question, because `capture.id` is only documented unique per target,
version, platform and architecture, so the collision was resting on a uniqueness
rule nothing states or checks. The derivation is
[`store::StoreKey`](../crates/bit-ids/src/store.rs) and it is the only place a
path is composed.

`latest` selects the newest validated stable version only. It is a generated
pointer, not a profile, and a prerelease can never move it.
[`index`](../crates/bit-ids/src/index.rs) derives it and the lookup indexes
beside it, with `cargo run -p bit-ids --example build-indexes -- STORE OUT` as
the driving surface and
[`check-indexes.sh`](../scripts/corpus/check-indexes.sh) as its prover. ⚠ It
selects nothing at all for a target whose version scheme is not declared, rather
than ordering under an assumed one.

## Append-only, and what checks it

A published path never changes and never disappears; a correction appends a
record carrying `supersedes`. [`store`](../crates/bit-ids/src/store.rs) holds
both halves of that as `E-STO-*` refusals: the structural rules a tree must
satisfy to be checked out at all on every platform in the matrix, and the
comparison between a published tree and the successor a run proposes.

`cargo run -p bit-ids --example check-store -- PRIOR NEXT` is the driving
surface, and [`scripts/corpus/check-store.sh`](../scripts/corpus/check-store.sh)
plants each refusal in a disposable tree and reads the exit code back.

Whether a tree is a coherent corpus at all is the separate question
[`corpus`](../crates/bit-ids/src/corpus.rs) answers, under `E-CRP-*`, with
`cargo run -p bit-ids --example validate-corpus -- STORE` as its driving surface
and [`check-corpus.sh`](../scripts/corpus/check-corpus.sh) as its mutation
prover. ⭐ `build-store` writes a store for either to be pointed at.

⚠ **A version is not a path segment and the store is what says so.**
[`Version`](../crates/bit-ids/src/canonical.rs) accepts whatever the installed
build printed, `../../etc` included, because imposing a grammar on a measurement
would refuse builds that number themselves some other way. `E-STO-01` blocks
publication for a version that cannot be a segment rather than mangling it into
one, because an escape that is not injective merges two measurements into one
directory.

## Assembly

One Rust assembler consumes checked capture artifacts, builds the publication
tree once, and reads no wall clock. The data-branch job and release job consume
the same uploaded tree. `MANIFEST.json` describes every file, media type,
schema and SHA-256 digest. `SHA256SUMS` permits ordinary transport checks.

CSV is a lossy tabular view and names which nested fields it omits. JSON and
CBOR carry the complete normalized model. SQLite provides indexed tables and
foreign-key integrity. Raw binary evidence is never embedded into CSV.

## Workflow permissions

- capture lanes: `contents: read`, no secrets, upload artifacts only;
- collector/validator: `contents: read`, cannot push;
- data publisher: job-scoped `contents: write`, append-only push without force;
- release publisher: job-scoped `contents: write`, triggered only by a pushed
  `v1.*` tag;
- pull requests from forks: no write token and no secrets.

Every third-party action is pinned to a full commit. Dependency automation
proposes pin updates; CI resolves the commit and checks the action runtime.

## Release assets

A tagged release will contain the manifest, checksums, each generated format,
and deterministic `.tar.gz` and `.zip` archives. A published tag or asset is
immutable. `latest` is the hosting platform's release pointer; no Git tag is
moved.

## Read-back

A successful push is not completion. The workflow fetches the remote branch,
compares the commit and tree object it intended to publish, then verifies every
manifest digest. Release creation similarly reads the release and asset list
back from GitHub.
