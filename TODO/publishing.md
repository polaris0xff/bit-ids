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
Priority: P0 | Effort: L | Status: OPEN

Problem: A branch publisher can accidentally force-push, drop prior records,
or expose partial output.

Approach: Publish from a validated temporary tree with ancestry checks,
least-privilege permissions, concurrency control, and a single atomic update.

Prove: integration tests refuse non-fast-forward and deletion scenarios, then
append a fixture release while preserving every prior digest.

## PUB-03: Multi-format GitHub release publisher

Source: operator request for many formats and paths
Priority: P1 | Effort: L | Status: OPEN

Problem: JSON alone is inconvenient for streaming, tabular analysis, embedded
tools, and compact transfer.

Approach: Publish deterministic JSON, JSONL, CSV, SQLite, and CBOR plus checksums
and schema documentation, all derived from the one assembled bundle.

Prove: release asset digests match the assembled manifest and cross-format
tests reconstruct equivalent normalized records.

## PUB-04: Stable raw and index access paths

Source: operator direct-GitHub access requirement
Priority: P1 | Effort: M | Status: OPEN

Problem: Consumers need documented immutable and current URLs without scraping
the repository UI.

Approach: Publish versioned raw paths, content-addressed evidence paths, latest
indexes, release assets, and integrity metadata with explicit stability rules.

Prove: a link checker fetches every documented path through the approved
GitHub read route and verifies its digest.
