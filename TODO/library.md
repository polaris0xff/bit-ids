# Consumer library entries

## LIB-01: Rust consumer library

Source: operator request for direct tool consumption and b-ids library pattern
Priority: P1 | Effort: L | Status: DONE

Problem: Rust tools should not reimplement schema validation, downloads,
indexes, and profile selection.

Approach: Provide no-network parsing by default plus opt-in verified retrieval,
typed lookup APIs, explicit schema compatibility, and embedded minimal indexes.

Prove: `cargo test -p bit-ids --locked --test catalogue` and
`sh scripts/publishing/check-catalogue.sh` both pass, with a publication opened
from bytes, a moved byte refused, and a platform and version selected with no
network reachable from the crate at all.

### Decision: no socket, and that is structural rather than a default

⛔ **"No network access" is proved by the crate having no way to reach one.** A
[`Catalogue`] is opened over bytes the caller already holds, and opt-in
retrieval is a trait the caller implements: the consumer fetches, and `retrieve`
decides whether what came back is the publication's. A library that fetched
would need a transport in a workspace that has argued for every dependency it
has, and "no network by default" would be a flag somebody could set.

⚠ It is checked rather than asserted. `check-catalogue.sh` sweeps the crate for
every socket type, address type and HTTP client, and reads its dependency list.
⭐ **The needle list is itself checked against `bit-ids-lab`**, which really does
carry sockets, because a sweep whose needles have stopped matching reports the
same clean answer over a crate full of them. That is `OBS-06`'s finding applied
to a sweep written afterwards.

### ⛔ What building a consumer found: two published documents with no reader

`Indexes::to_json` and `Release::manifest_json` both wrote a document nothing
could read back. Neither gap was cosmetic.

⛔ **The manifest one was a live defect.** Without a reader, a consumer compares
a bundle against a manifest it re-derived *from that bundle*, and that agrees
with itself over any bytes at all: a described file that is missing is not
described either, and a carried file nobody described is described. `E-LIB-01`
and `E-LIB-03` both existed and neither could fire, which a mutation pass showed
by deleting them and watching nothing change. Reading the published manifest is
what makes them reachable.

⚠ The index one was a format nobody had round-tripped. Both readers now have a
bytes-in-bytes-out case, which is the direction that matters: a reader dropping
a field still writes a document, just not this one.

### ⛔ What the mutation pass found: two guards masking each other

Twelve plants over `catalogue.rs`. On the first pass five survived, and reading
what each one changed separated three real weaknesses from two that were not.

⛔ **The per-file digest comparison and the manifest-against-the-bytes
comparison both report `E-LIB-02`, and each masked the other.** Deleting either
left every case green, because the surviving one still produced that code and
the cases asserted only the code. They are separated now by the path a refusal
names, with a case per shape: a record whose bytes moved, and a manifest whose
bytes moved while its digests still match. ⚠ The checksum comparison had the
same problem from the other direction and now has a case of its own.

⭐ **One survivor was the design working and is not a finding.** Reading a record
with `serde_json::from_str` rather than `Profile::from_json` changes nothing,
because `Profile` has a hand-written `Deserialize` that validates: `SCHEMA-01`
closed that door deliberately, and the plant is a second demonstration of it.

⚠ **One survivor is a mutation that is equivalent on the fixture.** Blanking the
lookup half of the row resolution left the `latest` half catching the same
identifier, because the fixture's absent record appears in both views. A case
covering the `latest` half was added; for the lookup half to be provable, a
fixture would need a lookup row naming an absent record where the latest row does
not, which no publication this project produces contains today.

### Acceptance, all run on 2026-09-08

- `cargo test -p bit-ids --locked --test catalogue`
- `sh scripts/publishing/check-catalogue.sh`
- `sh scripts/common/check-gate.sh`
- `cargo clippy --workspace --locked --all-targets -- -D warnings`

### Closure evidence, 2026-09-08

| what | measured |
| --- | --- |
| `cargo test -p bit-ids --locked --test catalogue` | 18 passed, 0 failed |
| `sh scripts/publishing/check-catalogue.sh` | 18 cases, 18 passed, 0 failed |
| `cargo test --workspace --locked --all-targets` | 50 binaries, 541 passed, 0 failed |
| `sh scripts/common/check-gate.sh` | 23 checks, 22 passed, 0 failed, 1 skipped, 0 unavailable |
| `pwsh -File scripts/common/check-gate.ps1` | 23 checks, 11 passed, 0 failed, 1 skipped, 11 unavailable |
| guard mutation over `catalogue.rs` | 12 plants; 10 refused, 1 equivalent on the fixture, 1 not a defect. The first pass had five survivors and three were real |
| driven pass | a two-version publication built, indexed, rendered and assembled, then opened by the library: `latest` answers 1.2.10 over 1.2.3, the target lookup answers both records, and the caller's out-of-band digest is checked |
| no network | the crate names no socket type, address type or HTTP client, and depends on `serde`, `serde_json` and `sha2`. The same needles do match `bit-ids-lab` |

⭐ **The ordering case is the one a text sort gets wrong**, and it runs through
the published index rather than around it: `1.2.3` sorts after `1.2.10` as text,
and a library that answered it would point a consumer at a superseded build with
complete confidence.

### Residuals

- ⚠ **"Embedded minimal indexes" in the Approach is not built.** A crate
  embedding a pinned copy of the catalogue needs a published catalogue to embed,
  and nothing has been published. The shape is `Bundle`, which takes bytes from
  anywhere including `include_bytes!`, so an embedding consumer needs no new API;
  what is missing is the bytes.
- ⚠ The library reads the published index rather than re-deriving it, because
  the ordering a latest row rests on is `ACQ-02`'s version scheme and a bundle
  does not carry one. A consumer re-deriving it would need a scheme nobody
  published.
- ⚠ `LIB-02` is the `bit-cli` adapter and is separate. Nothing here writes to
  another repository.
- ⚠ `Catalogue::open` does not run `validate_corpus`. It resolves every index row
  against the records the bundle carries, which is the question a consumer's
  answer depends on; whether every citation resolves to bytes is `CORPUS-02`'s,
  and a consumer that fetched only the records has no evidence to check.

## LIB-02: bit-cli integration adapter

Source: reference repository consumer requirement
Priority: P1 | Effort: L | Status: OPEN

Problem: bit-cli currently owns a generated profile table and can drift from
the measured corpus.

Approach: Define an adapter or generated crate interface that bit-cli can adopt
without any write to its repository. Preserve bit-cli's centralized peer ID
and fail-closed invariants.

Prove: a local integration fixture consumes a published profile and bit-cli's
identity consistency tests continue to pass; upstream changes remain out of
scope unless separately authorized.

⛔ **Blocked on repository access, measured on 2026-09-08 rather than assumed.**
The Prove names bit-cli's own identity consistency tests, and `Azathothas/bit-cli`
is not reachable from this harness: the session's GitHub scope is this repository
alone, and a repository listing filtered on `bit-cli` returns nothing. Adopting
the adapter without running those tests would be publishing a claim about a
repository this project cannot open.

⚠ Two things this does **not** block. `LIB-01` shipped the consumer side, so what
an adapter would consume exists and is tested. And the second half of the Prove,
a local integration fixture consuming a published profile, waits on a published
profile, which waits on a capture host.

The event that unblocks it: read access to `Azathothas/bit-cli` in a session that
can run its test suite. ⛔ Rule 10 still holds either way, so nothing is written
there.
