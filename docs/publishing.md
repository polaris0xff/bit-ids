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
raw/v1/<target>/<version>/<platform>/<arch>/<capture-id>/...
profiles/v1/<target>/<version>/<platform>/<arch>/<capture-id>.json
routes/v1/<target>/latest/<platform>/<arch>.json
routes/v1/<target>/<version>/<platform>/<arch>.json
indexes/v1/profiles.json
formats/bit-ids-v1.json
formats/bit-ids-v1.jsonl
formats/bit-ids-v1.csv
formats/bit-ids-v1.sqlite3
formats/bit-ids-v1.cbor
```

`latest` selects the newest validated stable version only. It is a generated
pointer, not a profile, and a prerelease can never move it.

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
