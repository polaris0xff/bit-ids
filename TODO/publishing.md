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

### Closure evidence, 2026-09-05

| what | measured |
| --- | --- |
| `sh scripts/publishing/check-publish.sh` | 13 cases, 13 passed, 0 failed |
| driven pass | first publication, a second version appended, an identical bundle pushing nothing, a deletion and a rewrite each refused with the branch left at its prior commit |
| read-back | every publication fetched again, compared against the prior tree, and its `SHA256SUMS` verified with `sha256sum -c` |

⚠ Every refusal case checks the commit count on the branch as well as the exit
code, because a publisher that refuses loudly and half-pushes anyway is worse
than one that does neither.

### Residuals

- ⚠ Concurrency control in the Approach is git's own: two publishers racing make
  the second push non-fast-forward, which git refuses and the harness measures.
  A workflow-level concurrency group belongs to `CI-01`.
- ⚠ Least-privilege permissions are a workflow property, and no workflow calls
  this yet. `docs/publishing.md` carries the job-scoped `contents: write`
  contract and `CI-01` wires it.
- ⚠ The publisher has never run against the real remote and will not until there
  is a measured record to publish. Everything in the tree today is synthetic.

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
