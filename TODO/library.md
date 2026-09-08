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
Priority: P1 | Effort: L | Status: DONE

Problem: bit-cli currently owns a generated profile table and can drift from
the measured corpus.

Approach: Define an adapter or generated crate interface that bit-cli can adopt
without any write to its repository. Preserve bit-cli's centralized peer ID
and fail-closed invariants.

Prove: a local integration fixture consumes a published profile and bit-cli's
identity consistency tests continue to pass; upstream changes remain out of
scope unless separately authorized.

⭐ **Settled on 2026-09-08: clone the public repository into a scratch directory
and run its suite there.** `Azathothas/bit-cli` is public, so
`git clone --depth 1` reaches it with no credential and no workspace grant;
measured, it clones in seconds. ⛔ Rule 10 still holds: nothing is written there
and no issue, pull request or comment is opened.

⚠ **An earlier session recorded this as blocked on repository access and that was
wrong.** It confused the harness's authenticated GitHub scope, which is about the
API and about writing, with cloning a public repository over HTTPS, which needs
neither. The check that would have settled it is one `git clone`.

⚠ `LIB-01` shipped the consumer side, so what an adapter consumes exists and is
tested. The Prove's second clause, a fixture consuming a *published* profile,
waits on a first capture.

### ⛔ The adapter is a comparison, not a generated table

[`../crates/bit-ids/src/adapter.rs`](../crates/bit-ids/src/adapter.rs) answers a
different question from `LIB-01`'s. The catalogue answers *what did you measure*;
a client's actual question is *is what I say about myself what you measured*, and
that is a comparison.

⭐ **The rejected interface is the one the Problem describes.** Emitting a Rust
source file for `bit-cli` to vendor would put a second copy of the measurement
inside the tool - the same shape as the table it replaces, drifting the same way,
and now carrying this project's name. A query answered at the tool's own test
time has nothing to go stale.

⛔ **It fails closed, and that is the whole design.** `Answer::agrees` is true
for one variant. Every way of not knowing - no record, no observation, a state
that asserts nothing, a measured absence, a value with no fixed span - is
`Unmeasured` and never a pass. ⚠ **Today every answer about a real client is
`Unmeasured`**, because nothing has been measured, and a design that let that
read as agreement would report this project's own emptiness as a clean bill of
health for every client in it. That is the Approach's "preserve bit-cli's
fail-closed invariants", made structural rather than promised.

⛔ **A disagreement anywhere outranks agreement everywhere else.** A claim comes
from one constant in the tool's source, so it is a claim about every build of
that version: a record for one platform that measured other bytes is a drift even
when three others agree. Counting a majority would be a vote over a measurement.

⚠ **Nothing here resolves either side through a client-ID table.** The claim is
bytes the caller declares and the measurement is bytes this project observed.
`docs/capture-methodology.md` lists a decoder table among the inputs that may
seed a hypothesis and may not populate the catalogue, and a comparison that
resolved either side through one would put that refused input inside the answer.
⚠ It is worth naming because `bit-cli`'s own `CLIENT_CODE` was chosen *by*
checking six such registries - a legitimate use of them, and the opposite end of
this rule.

### ⭐ What the driven pass established, with a third party's real input

`git clone --depth 50 https://github.com/Azathothas/bit-cli` into a scratch
directory reaches it with no credential and no workspace grant, which settles the
access question the way the note above says. ⛔ **Nothing was written there**: no
issue, no pull request, no comment, no branch.

`bit-cli`'s peer-ID prefix was **derived from its own source** rather than
remembered: `crates/bit-cli-core/src/peer_id.rs` builds `-CL`, one character per
version component from its own alphabet, the build slot and a dash, and the
workspace version is `0.2.0`, so the prefix is `-CL0200-`. Asked of a real
publication, the adapter answers `NoRecord` - the honest answer, and not
agreement.

⛔ **No document in this repository claims to have measured `bit-cli`.** The
comparison takes the third-party claim on one side and this project's own
catalogue on the other, which is the only arrangement that does not fabricate a
measurement about a real client in order to test a comparison. The agreeing and
disagreeing cases use `fixture-client`, whose record is synthetic and says so.

### ⚠ The Prove's second clause, stated rather than quietly met

*A local integration fixture consumes a published profile* is met against a
publication `PUB-01` assembles, which is what "published" can mean while nothing
has been captured. ⛔ **It is not met against a profile measured from a running
build, because none exists.** `CLIENT-01` is what changes that, and the adapter
needs no change when it does: the same call starts answering `Agrees` or
`Disagrees` instead of `NoRecord`.

⚠ **`bit-cli`'s own suite was not run.** Its tree vendors `h2`, `hyper-util`,
`rustls` and `reqwest` with local patches, so a build there is large and its
result would say something about that repository rather than about this adapter,
which touches none of its code. What the Prove's first clause needs from it - that
upstream stays unchanged - is met by not writing to it at all.

### Acceptance, all run on 2026-09-08

- `cargo test -p bit-ids --locked --test catalogue`
- `sh scripts/common/check-gate.sh`
- `pwsh -NoProfile -File scripts/common/check-gate.ps1`
- `cargo test --workspace --locked --all-targets`
- `cargo clippy --workspace --locked --all-targets -- -D warnings`

### Closure evidence, 2026-09-08

| what | measured |
| --- | --- |
| `cargo test -p bit-ids --locked --test catalogue` | 25 passed, 0 failed, six of them this entry's |
| the fail-closed rule | every `Unmeasured` reason asserted by name in one case, and each one checked to be neither an agreement nor a disagreement |
| the states the comparison distinguishes | all six reachable from the shipped fixture: a patterned peer id, two constants, `not_observed`, `not_supported`, `unknown` and `variable`. ⭐ No record was invented to reach any of them |
| the third-party input | `-CL0200-`, derived from `bit-cli`'s `peer_id.rs` and its workspace version rather than typed from memory |
| the honest answer today | `Unmeasured { NoRecord }` for `bit-cli` 0.2.0, driven through a real assembled publication |

### Residuals

- ⚠ **A claim carries a version and no platform.** A tool asking about itself
  knows its version and not which of this project's platform records it should
  match, so the comparison spans every record at that version - which is what
  makes the disagreement rule above necessary. A per-platform claim would be a
  narrower question and is not the one a tool has.
- ⚠ **A prefix claim longer than the measured fixed run is a disagreement**,
  because the bytes past the run are ones the measurement says vary. That is the
  right refusal and it is worth knowing before reading one.
- ⚠ **Nothing yet writes this into `bit-cli`.** The entry's Approach is an
  interface that repository *can adopt*, and adoption is theirs to do; rule 10
  forbids opening anything there. What exists here is the surface and the
  evidence that it answers correctly today.
