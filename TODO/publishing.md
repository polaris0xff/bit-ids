# Publishing entries

## PUB-01: Deterministic release assembler

Source: b-ids one-time assembly design
Priority: P0 | Effort: L | Status: DONE

Problem: Multiple jobs rebuilding formats can publish different bytes under
one release label.

Approach: Validate canonical records, assemble all formats once in a clean
workspace, generate a checksum manifest, and pass that immutable bundle to
every destination.

Prove: two independent assembly runs produce byte-identical archives,
databases, indexes, and checksum manifests.

Acceptance, both run on 2026-09-05:

- `cargo test -p bit-ids --locked --all-targets`
- `sh scripts/publishing/check-release.sh`

⚠ **The archives and the databases in that Prove are `PUB-03`'s and are not
here.** This assembles the tree once, describes it, and checksums it; the JSON,
JSONL, CSV, SQLite and CBOR renderings and the `.tar.gz` and `.zip` archives are
derived from this bundle rather than beside it, which is what the Approach means
by one immutable bundle passed to every destination. The determinism this entry
proves is the property those inherit.

### Decision: two documents that describe different sets

`MANIFEST.json` carries a media type, a schema and a digest for every file
except itself and `SHA256SUMS`, because a document cannot state its own digest.
`SHA256SUMS` covers everything except itself, the manifest included. Between them
every published byte is covered exactly once, and a reader who assumed either
covered everything would find the gap precisely where the other one is.

### Decision: a media type is looked up and never guessed

An extension this build does not know blocks the assembly under `E-REL-01`
rather than being published as `application/octet-stream`. ⭐ **The rule paid on
its first driven run:** it refused a real evidence bundle over
`fixture/generated.torrent`, which every capture writes and the table did not
carry. A default would have shipped it as opaque bytes and the failure would have
landed on a consumer parsing it.

### Closure evidence, 2026-09-05

| what | measured |
| --- | --- |
| `sh scripts/publishing/check-release.sh` | 13 cases, 13 passed, 0 failed |
| `cargo test -p bit-ids --locked --all-targets` | 7 release unit tests, 0 failed |
| `cargo test --workspace --locked --all-targets` | 36 binaries, 336 passed, 0 failed |
| guard mutation over `release.rs` | 10 plants; 9 refused, 1 named below as unrefuted |
| driven pass | a two-version store plus its indexes and a licence assembles to 24 described files and 25 checksum rows, byte-identical across two independent assemblies |
| independent verification | `sha256sum -c SHA256SUMS` agrees with every row, `MANIFEST.json` included |

⭐ **The strongest control here is not this project's code.** `sha256sum -c` is
a third-party reader of the checksum file, so a run that agreed with itself
about what it wrote would still be caught by it. Comparing two of the
assembler's own summaries would not have been.

⛔ One plant reported nothing rather than a refusal: it did not compile, so the
harness exited 2, and the review harness names *could not run* separately from
*refused* on both of its paths since `CORPUS-02` taught it to.

### The guard that is not refuted

| guard | why nothing refutes it | what would have to be true |
| --- | --- | --- |
| `entries.sort()` | the entries come out of a `StoreTree`, which is an ordered map, and `ReleaseEntry` orders on its path first, so removing it changes no output that can be produced today | `assemble` takes anything but an ordered map. It is what makes the manifest a function of the set of files rather than of the container the caller used |

⚠ It is the same shape as `CORPUS-03`'s `latest.sort()`, and both are kept for
the same reason and marked the same way.

### Residuals

- ⚠ `E-REL-11`, a present file nobody described, is unreachable through the
  driving example: it writes both documents and then refuses a second run over
  them under `E-REL-04`. It is planted in the unit tests instead. `PUB-02` reads
  a tree it did not assemble and is where it becomes reachable.
- ⚠ Nothing publishes yet. `PUB-02` pushes the assembled bundle to the `data`
  branch with the append comparison run over the fetched branch first.

## PUB-02: Protected append-only data branch publisher

Source: operator request and b-ids data branch
Priority: P0 | Effort: L | Status: DONE

Problem: A branch publisher can accidentally force-push, drop prior records,
or expose partial output.

Approach: Publish from a validated temporary tree with ancestry checks,
least-privilege permissions, concurrency control, and a single atomic update.

Prove: integration tests refuse non-fast-forward and deletion scenarios, then
append a fixture release while preserving every prior digest.

Acceptance, run on 2026-09-05:

- `sh scripts/publishing/check-publish.sh`

⛔ **Nothing in that acceptance touches a real remote.** Every case runs against
a bare repository the harness creates in a scratch directory and deletes on exit,
so the push path is exercised for real with no network and no credential.
`SECURITY.md` and `docs/security/remote-ops.md` are why that matters, and it is
not a weaker test: the publisher's own `git push` runs, and its refusals are read
from the process that produced them.

### What was found: the append rule and the derived files collided

⛔ **A correct second publication was refused, and driving this entry is what
surfaced it.** `CORPUS-01`'s `append_only` treated every published path as
immutable. But `MANIFEST.json`, `SHA256SUMS` and the generated indexes exist in
order to change: an index that did not move when a record was appended would be
one that had stopped describing the store. Applying the rule to those made a
second publication impossible.

`CANONICAL_ROOTS` now names the roots the rule is about, being `profiles/` and
`raw/`, and a derived path may change and may disappear because the whole
derived set is rebuilt from the canonical one. ⚠ Whether a consumer-facing path
is allowed to vanish is a different question and `PUB-04` owns it.

### Decision: no force, asserted three ways rather than omitted once

git refuses a non-fast-forward push by default, so the guard is that nothing
re-enables it. The publisher passes no force flag, the branch name is refused if
it carries a `+` or a `:` before any refspec exists, and
`check-publish.sh` reads the publisher's own source for a forcing flag with the
comments stripped first.

⚠ **The comment-stripping is not tidiness.** The first version of that case
matched the publisher's header sentence explaining that it uses no force and
reported the guard broken, so a source check that reads prose fires on the
documentation of the rule it enforces. The stripper is itself exercised against
a line that really is a force flag.

⭐ **That git refuses a divergent push under a plain refspec is measured on this
host rather than taken from the manual.** The publisher's whole
non-fast-forward defence rests on it.

### What the closing mutation pass found, which the entry's own did not

⛔ **The read-back could not catch a remote that discarded the push.** It
compared what came back against the prior tree, and a `post-receive` hook that
accepted the push and then moved the ref back leaves a branch that appends to the
prior tree perfectly, because it *is* the prior tree. "What came back appends to
what was there" and "what came back is what I pushed" are two facts, and only the
second notices that remote.

The publisher now compares the fetched tree against the bundle by a digest over
every file, before the append comparison and the checksum verification. The
planted hook is a case in the harness, so the guard has been seen to refuse.

⚠ It was found by planting against the read-back and watching the harness stay
green, which is the whole argument for planting against a guard rather than
reading it.

### Closure evidence, 2026-09-05

| what | measured |
| --- | --- |
| `sh scripts/publishing/check-publish.sh` | 15 cases, 15 passed, 0 failed |
| guard mutation over `publish-data.sh` | 5 plants, 5 refused |
| driven pass | first publication, a second version appended, an identical bundle pushing nothing, a deletion and a rewrite each refused with the branch left at its prior commit |
| read-back | every publication fetched again, compared against the bundle, against the prior tree, and its `SHA256SUMS` verified with `sha256sum -c` |

⚠ Every refusal case checks the commit count on the branch as well as the exit
code, because a publisher that refuses loudly and half-pushes anyway is worse
than one that does neither.

### Residuals

- Concurrency control in the Approach is git's own: two publishers racing make
  the second push non-fast-forward, which git refuses and the harness measures.
  ⭐ `CI-01` added the workflow-level group on top, and it does **not** cancel in
  flight: cancelling a gate run costs a rerun, while cancelling a publisher
  between its append comparison and its read-back leaves a branch nobody has
  verified.
- Least-privilege permissions are a workflow property, and `CI-01` wired the
  workflow that carries them. The publish job is the only one in this repository
  with `contents: write`, and it is job-scoped rather than inherited.
- ⚠ The publisher has never run against the real remote and will not until there
  is a measured record to publish. Everything in the tree today is synthetic.

## PUB-03: Multi-format GitHub release publisher

Source: operator request for many formats and paths
Priority: P1 | Effort: L | Status: DONE

Problem: JSON alone is inconvenient for streaming, tabular analysis, embedded
tools, and compact transfer.

Approach: Publish deterministic JSON, JSONL, CSV, SQLite, and CBOR plus checksums
and schema documentation, all derived from the one assembled bundle.

Prove: release asset digests match the assembled manifest and cross-format
tests reconstruct equivalent normalized records.

⛔ **The SQLite rendering is split out and `PUB-05` carries it**, because it is
the one format here that cannot be written without a decision the operator owns:
a new third-party dependency carrying a C library and `unsafe` code, into a
workspace whose lints say `unsafe_code = "forbid"`. Nothing was dropped. The
other four are here and the split is recorded rather than the entry being
quietly narrowed.

### Decision: the encoders are written here, and the argument is not "small"

`docs/supply-chain.md` asks that a dependency be argued for in the entry that
adds one. This entry adds none. JSON and JSONL and CSV need no encoder beyond
what the crate already has, and the CBOR encoder is written here for the reason
the wire codecs are: the published bytes are what a digest names, so the encoder
has to be one this project can read. ⚠ The subset is small and closed **because
the input is a JSON document**, not an arbitrary value: no tags, no indefinite
lengths, no floats.

⭐ **And a hand-written encoder is exactly the thing to check against somebody
else's.** `cbor2` 6.1.4 does not merely parse the file: re-encoding what it read,
with its own canonical flag, produces **the same bytes**. Two independent
implementations of RFC 8949 section 4.2.1 agreeing byte for byte is a much
stronger result than a round trip through this project's own reader would have
been.

### Decision: every rendering is a function of the canonical document

⛔ **The combined JSON carries each record's own bytes**, verbatim rather than
re-indented, so a reader who slices one out has exactly what was published and
digested. JSONL and CBOR are produced by reading that document back and
re-emitting it, so neither can carry a field the published JSON does not, and
the tabular cells are read out of it by pointer rather than off the typed
record. A second rendering of a value the canonical form already spells is the
`canonical.rs` hazard applied to documents.

⚠ **The tabular view says what it omits in a file rather than in prose.**
`formats/bit-ids-v1.columns.json` names the columns and the seven record
sections no row can hold, so a consumer reading only the CSV can discover what
it is not being told. `the_omitted_sections_are_ones_no_column_reads` checks
both directions of that claim.

### ⛔ What the door sweep found: the record set is `CORPUS-04`'s

A renderer that filtered the store on its own would have kept publishing a
corrected record in the tabular view while the lookups had stopped naming it,
and the table is the rendering a reader is least likely to cross-check.
`Indexes::records` is the one answer to which records are published and this
asks it. ⭐ The driven run shows it working: a store of three records renders
two, and the one it leaves out is the one a correction retracted.

### Acceptance, all run on 2026-09-06

- `cargo test --workspace --locked --all-targets`
- `sh scripts/publishing/check-formats.sh`
- `sh scripts/common/check-gate.sh`

### Closure evidence, 2026-09-06

| what | measured |
| --- | --- |
| `sh scripts/publishing/check-formats.sh` | 16 cases, 16 passed, 0 failed |
| `cargo test --workspace --locked --all-targets` | 37 binaries, 350 passed, 0 failed |
| driven pass | a three-record store with one correction rendered into five files twice, byte-identical, then assembled into a release whose manifest describes every one |
| independent verification | `sha256sum -c` agrees with every published digest, and has been seen to refuse a file whose bytes moved |
| independent verification | `cbor2` 6.1.4 decodes the CBOR to the same value as the JSON, and its own canonical encoding of that value is byte-identical to ours |

⚠ **The suite cannot decode the CBOR and says so.** A test that read it back
with this project's own encoder would be checking the writer against itself, so
the Rust case asserts the file is non-empty and the identifiers are compared in
the two textual renderings. `cbor2` is what closes it, in the driven pass, the
same way `libtorrent` and `torf` closed `OBS-08`'s.

### ⛔ What a test got wrong, correctly

`a_superseded_record_is_in_no_rendering` first asserted that the corrected
record's identifier appears nowhere in any rendering, and it failed. That is
false by design: a correction names what it corrects, so the identifier is in
the published bytes as the value of `supersedes`. The rule is that the record is
not **published as a record**, so the case compares identifiers rather than
searching text, and `check-formats.sh` carries a case asserting the correction
still names what it corrects, because a version of this that weakened the
assertion the wrong way would pass over a correction that had forgotten to.

### Residuals

- ⚠ `PUB-05` owns the SQLite rendering and the dependency decision behind it.
  Until it lands, `docs/publishing.md`'s layout lists a file nothing writes, and
  it says so.
- ⚠ The CBOR encoder covers what a JSON document contains and refuses anything
  else under `E-FMT-03`. A float is the case that would need the one rule this
  subset does not hold, and the record model has no float today.

## PUB-05: SQLite rendering of the published records

Source: split out of `PUB-03` on 2026-09-06
Priority: P1 | Effort: M | Status: OPEN

Problem: `docs/publishing.md` promises `formats/bit-ids-v1.sqlite3`, with
indexed tables and foreign-key integrity, and nothing writes it. A consumer who
wants to query the catalogue rather than parse it has no route.

Premise: measured while closing `PUB-03`, not read. The other four renderings
need no dependency at all. This one needs either a third-party crate or a
hand-written encoder for a file format that is genuinely hard to get right.

⚠ **The dependency question was settled on 2026-09-08 and this entry is not
blocked.** The answer is the crate, and the reasoning below is kept because the
cost it names is real and has to be paid deliberately. Both routes cost
something this project has been deliberate about:

- `rusqlite` brings `libsqlite3-sys`, a vendored C library and a build script,
  into a workspace whose lints say `unsafe_code = "forbid"`. It is the largest
  dependency this repository would have taken, and `docs/supply-chain.md` asks
  for that argument in the entry rather than in a commit message.
- Writing the file format here means new unaudited code producing B-tree pages
  and varints in the component that publishes evidence, to avoid one crate. That
  is the argument `OBS-08` rejected for SHA-1, and it is stronger here because
  the format is larger.

⭐ **Settled: take `rusqlite` with the bundled feature**, pinned, checked against
a database a reader this project did not write can open, and record the `unsafe`
exception against this one dependency rather than relaxing the workspace lint.
⚠ `sqlite3` is not installed on a session host, so the independent reader is
provisioned the way `cbor2` was: a virtualenv, or the `sqlite3` package.

Approach: once decided, derive the tables from the same canonical documents the
other renderings use, so no rendering can carry a field another does not.

Prove: the database opens in a reader this project did not write, every record
in it round-trips to the same normalized record as the JSON rendering, and two
builds over one store produce byte-identical files.

## PUB-04: Stable raw and index access paths

Source: operator direct-GitHub access requirement
Priority: P1 | Effort: M | Status: DONE

Problem: Consumers need documented immutable and current URLs without scraping
the repository UI.

Approach: Publish versioned raw paths, content-addressed evidence paths, latest
indexes, release assets, and integrity metadata with explicit stability rules.

Prove: `sh scripts/publishing/check-access.sh` and
`cargo test -p bit-ids --locked --test access` both pass, with every documented
path resolving in a publication fetched back over git, every stated digest
recomputed by `sha256sum`, and every path the contract calls immutable
byte-identical across two publications.

### ⛔ What deriving the path set found: a documented path nothing writes

`docs/publishing.md` carried `routes/v1/<target>/<version>/<platform>/<arch>.json`
and `routes/v1/<target>/latest/<platform>/<arch>.json` in its layout, and nothing
has ever written either. That alone is the "a file a summary claims that is not
on disk" row.

⛔ **And it cannot be written in that shape.** The path omits `<package>` while
the acquisition routes differ per package, so a `deb` and an `AppImage` of one
version on one platform would be two different route sets at one file name. That
is exactly the non-injective layout `CORPUS-01` found in the profile path, in a
second place, found here the same way: by deriving the documented set from an
assembled release instead of reading it off the document. Both paths are removed
from the layout, with the reason recorded there. ⚠ A per-route view is still
worth having and needs the identity tuple in full; it is a new rendering rather
than a path that document can promise.

### Decision: two facts per path, and both derived

A consumer is told how long a path's bytes are good for, and which published
document proves them. Neither is declared per path.

⛔ **Stability comes from `store::CANONICAL_ROOTS` and never from a second
list.** That constant is already what the append-only rule is about, so a path is
`immutable` exactly when the publisher refuses to rewrite it. A second
enumeration would be one rule spelled twice, and the copy that drifted would be
the one telling a consumer to cache a file that moves.

⛔ **Integrity is a function of the path**, and exactly one published file is
covered by neither document: `MANIFEST.json` describes everything but itself and
`SHA256SUMS`, and `SHA256SUMS` covers everything but itself. So the checksum file
is the one a consumer verifies out of band, and the contract says so per path
rather than leaving a reader to work out where the gap is.

⚠ **There is no default stability**, which is `PUB-01`'s media-type rule applied
to caching. [`../docs/publishing.md`](../docs/publishing.md) carries the classes
and the refusal; what matters here is that it fired on its first run, over
`routes/`.

### ⛔ What the mutation pass found: three refusals nothing could reach

Twelve plants over `access.rs`. Eight were refused on the first pass and three
survived, all three for one reason: `contract` derives a path's stability from
`is_canonical_path`, its integrity from its kind, and its order by sorting, so
`E-ACC-03`, `E-ACC-04` and `E-ACC-05` can never fire on anything the deriver
produced. The two halves agree by construction, and blanking each left the whole
crate and the whole harness green.

⚠ **They are not dead code.** A contract arrives from a file as often as from a
deriver, and `AccessContract::from_json` is the door a consumer's copy comes
through. What was missing was a test where they are reachable, which is at the
document level: `crates/bit-ids/tests/access.rs` builds a real contract, plants
into the document, and reads the refusal. All three are refused there now, along
with the two that were already reachable.

⚠ **And the mutation harness reported a false SURVIVED before that was
believed.** It ran `cargo test --lib`, and the new cases are an integration
target, so the tests written to catch the plants were not run at all. A plant
whose verdict comes from a selection that excludes its own test says nothing
about the guard, which is the same class as a harness exit of 2.

### Acceptance, all run on 2026-09-08

- `sh scripts/publishing/check-access.sh`
- `cargo test -p bit-ids --locked --test access`
- `sh scripts/common/check-gate.sh`

### Closure evidence, 2026-09-08

| what | measured |
| --- | --- |
| `sh scripts/publishing/check-access.sh` | 16 cases, 16 passed, 0 failed |
| `cargo test -p bit-ids --locked --test access` | 12 passed, 0 failed |
| guard mutation over `access.rs` | 12 plants, 12 refused after the three unreachable refusals were given document-level cases. ⚠ One plant's literal had to be corrected before it applied; a plant that did not apply is neither a refusal nor a survivor |
| driven pass | a one-record catalogue published to a bare repository and fetched back over git, then a two-record one; 20 contract paths on the first fetch, 11 immutable and 9 current, every path resolving and every digest recomputed |
| immutability | every immutable path of the first publication byte-identical in the second, with 7 current paths moved and the immutable count going 11 to 22 |
| independent verification | `sha256sum -c` recomputes every digest the contract states, over the bytes that came back from the remote |

⭐ **The immutability rule needs two publications and nothing else can prove
it.** "This path never changes" is a comparison between trees, so a checker given
one publication can only read a label off a document that asserted it. ⚠ And a
checker that only asserted the immutable paths did not move would pass over a
second publication identical to the first, so "at least one current path moved"
is a case rather than an assumption.

### Residuals

- ⛔ **No GitHub URL has been fetched, because nothing has ever been published.**
  `docs/publishing.md` carries the three forms, and what is proved here is the
  path set, both classes and every digest against a publication pushed to a bare
  repository and fetched back over git. The URL forms close when a first real
  publication exists, which needs a measured record, which needs a capture host.
- ⚠ Release assets are named in the Approach and are `PUB-03`'s and `CI-04`'s.
  The contract is over the `data` branch, which is what a consumer reads without
  a release; a tagged release carries the same bytes under the platform's own
  asset URLs and `CI-04` owns binding those to a workflow identity.
- ⚠ `check-access` needs `python3`, so it exits 2 without one, which is a skip
  and never a pass. `ubuntu-24.04` carries it and the Linux lane runs the gate
  with `--strict`.
