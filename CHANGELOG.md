# Changelog

Nothing is released yet. Entries accumulate here until the first
`v1.*` tag, which is what `PUB-03` creates.

## Unreleased

### 2026-09-06T23:10:00Z

- `OBS-06`, the adjacent protocol observer suite, over local discovery and peer
  exchange. Record: [`TODO/observer.md`](TODO/observer.md).
- ⛔ The lab had no egress guard and a door sweep is what found it. Every socket
  went through `bind.rs`; every send did not. `endpoint::serve_datagram` answered
  on the bound socket with the source address the sender wrote on the packet, and
  nothing verifies a UDP source address. `bind::send_to` is the one door now.
- ⚠ The sweep's needle list is why it lasted: every needle on it named a
  constructor, and a send is a method on a socket that already exists, so the
  whole category was missing rather than one entry.
- ⚠ A send guard checks before the syscall and cannot check after. A bind reads
  `local_addr` and a dial reads `peer_addr`; a datagram socket reports nothing
  about the packet it sent.
- ⭐ An adjacent surface is behind a value that has to be constructed, not a flag
  whose default is false. `Capability::enable(surface)` is the only constructor,
  and a boolean default is what a later `..Default::default()` flips.
- ⛔ A second `Surface` enum was written for the lab and removed.
  `bit_ids::observation::Surface` already named four of these, so the lab
  re-exports the record vocabulary and `local_discovery` was added to it.
- Three surfaces split out as `OBS-11`: message stream encryption, the DHT and
  web seeding are each an `L` on their own and `TODO/RULES.md` says to split
  before execution.
- Twenty-three plants, twenty-two refused. The two survivors were findings: a
  test named for a guarantee it did not check, and one guard nothing can refute,
  which is kept with the reason written where it is.
- Deployment: nothing deployed. No capture was taken and nothing was published.
  The lab never joined the multicast group BEP 14 names; the driven run sent
  three announces to a loopback address from a client on the same host.

### 2026-09-06T21:45:00Z

- `ACQ-05`, the artifact cache and its authenticity evidence. Record:
  [`TODO/acquisition.md`](TODO/acquisition.md).
- ⭐ The second half of the Prove asks for less than it sounds like. Reproducing
  an artifact's identity after a source URL change needs no reproduction once the
  identity is the digest: a retrieval from a new location is recorded against the
  artifact the digest already names, and the lookup never mentions a URL.
- ⚠ A repeated retrieval adds nothing. A cache that grew a row every time
  somebody re-ran an acquisition would report a popularity contest rather than a
  provenance.
- ⛔ Keeping the bytes is a permission and not a capability. `E-CAC-01` refuses
  stored bytes where the register refuses them, and `E-CAC-02` refuses a target
  the register does not mention rather than defaulting to permitted.
- ⛔ The register is asked, never re-read. `check-licences` gained `--permitted`
  on both halves, the Rust cache takes a disposition map, and `check-cache.sh`
  fills it from that call, so the tie is one parser rather than two readings.
- ⛔ The refusal case needed a control and the harness carries it: the same
  scenario is run with a target explicitly permitted, and the two runs are
  asserted to differ on exactly one line. Without it the policy half would pass
  equally over a cache that can never store anything.
- Nine plants over `cache.rs`, all refused, each verified to compile first.
- Deployment: nothing deployed. No capture was taken. Nothing was published, and
  nothing was fetched: the scenario's bytes are generated and are nobody's
  installer.

### 2026-09-06T19:30:00Z

- `FOUND-04`, the third-party licence and redistribution register. Record:
  [`TODO/foundation.md`](TODO/foundation.md).
- ⛔ The measurement is the finding: six of the nine targets with a GitHub
  upstream answer `NOASSERTION` when their licence endpoint is asked, so a
  detector cannot name one. Those rows say `unverified` and name who was asked,
  rather than carrying an identifier nobody established.
- ⭐ The twenty-two dependency rows are read out of each package's own manifest
  at the version the lockfile pins. `libc` is not built for this host's target,
  so its manifest was fetched before it was read rather than assumed from its
  siblings.
- ⛔ Every row refuses redistribution, which is the policy and not a consequence
  of the licences. `permitted` exists so the check can refuse it over an
  unverified licence or with no notice.
- ⛔ The twin comparison found a defect a clean tree could not show. On an empty
  register the two halves disagreed, because `grep -c .` prints `0` and exits 1,
  so a `|| printf 0` fallback made the count two zeroes on two lines and the
  guard that refuses a register of nothing was disabled by exactly the input it
  exists to catch. `wc -l` replaces it.
- `check-licences` joins `check-twins` and both gate runners as the tenth pair.
- Deployment: nothing deployed. No capture was taken. Nothing was published.

### 2026-09-06T17:05:00Z

- `PUB-03`, the multi-format publisher, closed over four of its five renderings.
  Record: [`TODO/publishing.md`](TODO/publishing.md).
- ⛔ The SQLite rendering is split out as `PUB-05` and is blocked on an operator
  decision rather than on work. Both routes to it cost something this project has
  been deliberate about; [`docs/supply-chain.md`](docs/supply-chain.md) carries
  the argument and the entry carries the recommendation. Nothing was dropped and
  the split is recorded.
- ⛔ Every rendering is a function of the canonical document. The combined JSON
  carries each record's own bytes verbatim, so slicing one out yields exactly
  what was published and digested; JSONL and CBOR are produced by reading that
  document back; the tabular cells are read out of it by pointer.
- ⛔ Which records are rendered is `CORPUS-04`'s answer and not a second filter.
  A renderer that selected on its own would have kept publishing a retracted
  measurement in the table, which is the rendering a reader is least likely to
  cross-check.
- ⭐ The CBOR encoder is written here and checked against `cbor2` 6.1.4 rather
  than trusted, and not by a round trip: that reader's own canonical encoding of
  what it read is byte-identical to ours. Two implementations of RFC 8949
  section 4.2.1 agreeing is a stronger result than one agreeing with itself.
- ⚠ `formats/bit-ids-v1.columns.json` publishes the columns and the seven record
  sections no row can hold, so a consumer reading only the CSV can discover what
  it is not being told.
- ⛔ A test got something wrong, correctly. It first asserted the corrected
  record's identifier appears nowhere; that is false by design, because a
  correction names what it corrects. The rule is that it is not published as a
  record, so identifiers are compared rather than text searched.
- `check-formats.sh` joins the `sh` gate as the seventh mutation prover, and the
  two examples now share one store reader rather than growing a second copy.
- Deployment: nothing deployed. No capture was taken. Nothing was published.

### 2026-09-06T14:40:00Z

- `CORPUS-04`, supersession and correction records. Record:
  [`TODO/corpus.md`](TODO/corpus.md).
- ⚠ Half the Approach already existed: `supersedes`, `adjudication` and
  `E-CRP-07` were all in place. What none of them did was change an answer, so a
  corrected record was still in every lookup and could still be the latest view's
  reply.
- ⛔ A superseded record now leaves every view and stays in the store, and the
  two counts are kept apart: an excluded record was never publishable, a
  superseded one was.
- The `corrections` list carries the superseded identifier, the record that
  directly corrects it, and the end of the chain. ⚠ The last two differ exactly
  when a correction was itself corrected, which is the case a single-step row
  answers wrongly while looking right.
- ⚠ Only a publishable correction retracts anything. A provisional one would
  otherwise leave the build line answering nothing at all.
- ⛔ A fork is refused rather than resolved, and the chain walk is bounded
  because a cycle is constructible: a record identifier digests the identity
  tuple and not `supersedes`, so two records can name each other while neither
  supersedes itself.
- ⭐ The retention half needed two stores rather than one. Finding the original
  still there proves it was written, not that it was left alone, so
  `build-store --correct V` writes the correction beside it and the harness
  compares that record's bytes between a store built with the correction and one
  built without.
- ⛔ The claim audit corrected this entry's own draft. It was about to record
  that a cycle is a store the corpus validator accepts; asserting that instead
  of writing it down showed the validator refusing the store for an unrelated
  reason. The residual is real and the reason was wrong.
- Deployment: nothing deployed. No capture was taken. Nothing was published.

### 2026-09-06T12:15:00Z

- `CI-01` closed, the complete cross-platform quality gate, and with it the last
  open `P0`. Record: [`TODO/ci.md`](TODO/ci.md).
- ⭐ The Windows lane reports **zero skipped** on CI run 37 under `-Strict`: ten
  passed, none failed, and seven rows named unavailable with the entry that owns
  each. That is the Prove's first half measured rather than argued.
- `.github/workflows/publish-data.yml` carries the job-scoped `contents: write`
  and the concurrency group `PUB-02` left as residuals. ⛔ It has no automatic
  trigger and its dry run is the default, so it cannot fire on its own and a
  dispatch by accident still pushes nothing; both are cases in the harness.
- ⚠ Its group does not cancel in flight, unlike the gate's. Cancelling a gate run
  costs a rerun; cancelling a publisher between its append comparison and its
  read-back leaves a branch nobody has verified.
- ⚠ The token reaches git as a header rather than inside a remote URL. The first
  version wrote it into the URL and `check-no-secrets` refused the file, which is
  the guard working rather than an obstacle.
- ⛔ That workflow has never run and cannot succeed today: its first step wants
  the assembled bundle of a capture run, and there are no captures.
- The harness now covers every workflow rather than the one it was written for,
  and its scratch tree carries the origin remote, without which
  `check-remote-items` could not run there and the gate cases never saw a clean
  tree exit 0.
- Deployment: nothing deployed. No capture was taken. Nothing was published. No
  remote was written other than this repository's `main`.

### 2026-09-06T10:30:00Z

- `CI-01` in progress, the cross-platform quality gate. Record:
  [`TODO/ci.md`](TODO/ci.md).
- ⛔ The Windows lane ran without `--strict` and could not do otherwise: six of
  its rows are checks that platform cannot run, so the flag would have refused
  every correct tree. A check that stopped running was therefore counted beside
  them and the lane stayed green. Measured by rewriting `check-project.ps1` to
  `exit 2`, which that lane's own invocation exited 0 over.
- Both gate runners now count a declared unavailability apart from an observed
  skip. A declared row carries its reason and prints as `n/a`; `--strict`
  refuses only the observed kind, so both lanes ask strictly now.
- `scripts/ci/check-workflow.sh` plants a defect of each class into a scratch
  copy of the working tree and runs the offending workflow step against it.
  ⭐ Every command it runs is read out of the workflow by job and step name, so
  it cannot drift from CI and a step that has gone is reported rather than
  passed over.
- ⛔ It is deliberately absent from `check-gate.sh`: two of its cases run the
  gate, and a runner listed in the gate that also runs the gate re-enters
  itself. The workflow calls it as a step of its own.
- ⛔ The door sweep found one assumption behind two doors. `store_build` and
  `publish-data.sh` each composed an example path as `target/debug/examples`
  while cargo obeys `CARGO_TARGET_DIR`, so on a host exporting that variable all
  five corpus and publishing provers exited 2 and the publisher refused to run.
  Exit 2 is a skip to the gate, so the tier proved nothing and said so nowhere.
- The workflow declares per-job permissions and timeouts, caches downloaded
  crates and build artifacts under separate keys, and pins `actions/cache`.
- Deployment: nothing deployed. No capture was taken. No remote was written
  other than this repository's `main`.

### 2026-09-05T15:00:00Z

- `PUB-02`, the append-only data branch publisher. Record:
  [`TODO/publishing.md`](TODO/publishing.md).
- ⛔ Driving it found that the append rule and the derived files collided: a
  correct second publication changes `MANIFEST.json`, `SHA256SUMS` and the
  indexes by design, and treating every published path as immutable refused it.
  `store::CANONICAL_ROOTS` now names the roots the rule is about.
- The append comparison runs before the push, not after, because a branch
  protection setting refuses a force and says nothing about a commit that deletes
  a file. A refusal ends the run with nothing pushed and the branch where it was.
- No force, asserted three ways: no flag, a branch name carrying `+` or `:` is
  refused before a refspec exists, and the harness reads the publisher's own
  source with the comments stripped. ⚠ The first version of that source check
  matched the header sentence explaining the rule and reported the guard broken.
- ⭐ That git refuses a divergent push under a plain refspec is measured on this
  host rather than taken from the manual, because the publisher's whole
  non-fast-forward defence rests on it.
- ⛔ The acceptance touches no real remote: it creates a bare repository in a
  scratch directory and deletes it. The publisher has never run against this
  repository's own remote and will not until a measured record exists.
- ⛔ The closing mutation pass found that the read-back could not catch a remote
  that discarded the push: it compared the fetched tree only against the prior
  one, and a rewound ref appends to the prior tree because it is the prior tree.
  The publisher now compares against the bundle as well, and a `post-receive`
  hook that rewinds the ref is a case in the harness.
- Deployment: nothing deployed. No capture was taken. No remote was written
  other than this repository's `main`.

### 2026-09-05T14:20:00Z

- `PUB-01`, the deterministic release assembler. Record:
  [`TODO/publishing.md`](TODO/publishing.md).
- Two documents describe the bundle and cover different sets, because neither
  can state its own digest: `MANIFEST.json` describes everything except itself
  and `SHA256SUMS`, and `SHA256SUMS` covers everything except itself.
- ⭐ The media-type rule paid on its first driven run, refusing a real evidence
  bundle over `fixture/generated.torrent`, which every capture writes and the
  table did not carry. A default would have shipped it as opaque bytes.
- ⭐ The strongest control is a reader this project did not write: `sha256sum -c`
  agrees with every row, so a run that agreed with itself about what it wrote is
  still caught.
- ⚠ The archives and databases in that entry's Prove are `PUB-03`'s. This
  assembles the tree once and describes it; the renderings inherit the
  determinism rather than each recomputing it.
- Guard mutation: 10 plants over the assembler, 9 refused. `entries.sort()` is
  recorded as unrefuted with what would have to change for it to fire, the same
  shape and the same treatment as `CORPUS-03`'s `latest.sort()`.
- Deployment: nothing deployed. No capture was taken.

### 2026-09-05T13:40:00Z

- `CORPUS-03`, the deterministic indexes and latest views. Record:
  [`TODO/corpus.md`](TODO/corpus.md).
- Six lookups and one latest row per build line, every row naming the record it
  came from. `rows_resolve` checks them against the store rather than against the
  builder that emitted them.
- ⭐ The peer-prefix lookup inverts the rule the codecs hold rather than waiving
  it: its key is the fixed span of a peer ID this project measured.
- ⚠ `VersionScheme::components` is now public, so the resolver and the latest
  view share one version ordering. A target with no declared scheme blocks the
  view rather than being ordered under an assumed shape.
- ⭐ The acceptance caught a real defect before the mutation pass did: the
  ranking carried a `Version` comparison beside the numeric one, and `Version` is
  ordered as text, so `1.2.9` beat `1.2.10` and the view answered a superseded
  build. One total key replaced it.
- ⛔ The first mutation round refused two plants of eight. Two survivors were bad
  plants; four were real gaps, now closed: the sort was invisible because the
  store was read in one order, nothing excluded a provisional record, the
  varying-first peer-ID rule was never reached, and `E-VIW-10` was asserted where
  it could not fire.
- ⛔ A harness overwrote the shared library's row accumulator with its own
  variable and printed ten passes over eight lines. The globals are prefixed and
  `store_report` now compares the rows it holds against the count it prints.
- ⛔ `validate_corpus` reported a store of nine orphan artifacts as valid: the
  bundle sweep ran per manifest and there was no manifest to run it. It sweeps
  the whole tree now, and `E-CRP-08` covers the record root.
- `build-store` takes `--version`, so a store can be written at a version a
  scheme can order; the fixture's own is deliberately not one.
- Deployment: nothing deployed. No capture was taken.

### 2026-09-05T12:55:00Z

- `CORPUS-02`, the semantic corpus validator. Record:
  [`TODO/corpus.md`](TODO/corpus.md).
- ⛔ Evidence reachability was the only invariant in that entry's Problem that
  nothing could already answer. `bind` compares the profile against the manifest,
  so two documents agreeing about an artifact nobody wrote satisfied every check
  this project had. `E-CRP-03` resolves a citation against the store instead, and
  `E-CRP-04` to `E-CRP-06` compare the stored bytes with what the run declared,
  in both directions.
- ⚠ Route count, connector independence, field provenance, agreement and stable
  status were already enforced per record. The entry says which code holds each,
  rather than adding a second spelling of it.
- ⭐ `examples/build-store.rs` generates the golden corpus the acceptance needs,
  writing the artifacts first and then each document to describe the bytes
  actually put down, out through `to_json`. The entry says why a committed one
  could not serve.
- `validate_corpus` refuses what must hold of any store; `publishable_view`
  separately reports which records may enter a published view, because a store of
  provisional records is a correct store.
- ⛔ The entry's `Prove` selected one integration binary with `--test`, which
  skips the library's own tests. Rewritten and the original recorded, per the
  residual `CI-05` left.
- ⛔ A review plant reported a refusal that was not one: the harness counted a
  harness exit of 2, *could not run*, as a refusal on its filesystem path. It now
  separates them and the plant was rewritten to compile.
- ⚠ `shellcheck` answers differently depending on how the files are grouped on
  its command line, so the two harnesses carry their own directives rather than
  depending on CI passing every script at once. The shared helpers moved to
  `scripts/corpus/store-lib.sh` rather than being copied.
- Guard mutation: 8 plants over the corpus validator, 8 refused;
  `check-corpus.sh` plants 9 defects against a real store and refuses all 9, with
  4 harness self-guards.
- Deployment: nothing deployed. No capture was taken.

### 2026-09-05T12:10:00Z

- `CORPUS-01`, the append-only canonical store. Record:
  [`TODO/corpus.md`](TODO/corpus.md).
- A record's path is derived from the identity tuple `RecordId` digests, in full,
  and a published path never changes or disappears. `E-STO-*` carries both
  halves: the structural rules a tree must satisfy to be checked out at all, and
  the comparison between a published tree and the successor a run proposes.
- ⛔ The published layout was not injective over the identity tuple. A profile
  was filed with no `package` segment while the tuple carries one, so a `deb` and
  an `AppImage` of one version on one platform were two records at one file name.
  [`docs/publishing.md`](docs/publishing.md) is amended and a test pins it.
- ⛔ `Version` accepts `../../etc`, measured rather than assumed, because a
  version string is what the build printed. The store refuses a version that
  cannot be a path segment rather than escaping it, since a non-injective escape
  merges two measurements into one directory.
- ⭐ The driven pass found what the suite could not: the placement reader
  selected a record by name and opened it, so a named pipe blocked it forever
  while `validate_tree` already refused that entry kind. One action, two doors,
  one gate. The reader now takes the kind from the walk.
- ⛔ The door sweep found a second spelling of the layout, in the recogniser that
  decides whether the placement check runs at all. A layout change would have
  left it recognising nothing, with the suite still green. The recognisers moved
  beside the composer and a round-trip test closes the loop.
- Guard mutation: 10 plants over the store source, 10 refused; 9 planted against
  a real filesystem, 9 refused; 4 harness self-guards exercised. ⚠ `grep -F`
  splits a multi-line pattern into alternatives and miscounted three plants as
  ambiguous, which fails safe and still leaves guards unproven; the committed
  harness refuses a multi-line literal outright.
- `scripts/corpus/check-store.sh` joins the `sh` gate and is a named skip on the
  PowerShell half, because its plants are a symbolic link and a named pipe.
- Deployment: nothing deployed. No capture was taken.

### 2026-09-05T11:20:00Z

- The session's four closing reviews. Record:
  [`docs/history/SESSION-2026-09-05-EVIDENCE.md`](docs/history/SESSION-2026-09-05-EVIDENCE.md).
- ⛔ The door sweep found the capture path untested as a whole. Each leg was
  covered and nothing drove a torrent through an observer into a bundle, which
  is the composition class `gate.md` names.
  `crates/bit-ids-probe/tests/generated_torrent.rs` closes it, and the two
  plants written for it are refused.
- `REDACTED` was public with no reader outside its own module. The acceptance
  suite now asserts the constant and its literal, so the placeholder a bundle
  reader recognises cannot drift and the constant is not surface with no
  consumer.
- ⭐ The claim audit's fourth lens paid: the qBittorrent release listing
  answered with four releases and that is the source's own shape, not
  truncation. Page two is empty and `tags` answers with at least a hundred, so
  a resolution reading only `releases` selects from a much smaller population
  than the target's versions. Recorded in [`TODO/clients.md`](TODO/clients.md).
- `TODO/RULES.md` now states that `Closure evidence` is a dated measurement and
  a `Prove` is a live command, which is the distinction `CI-05`'s check encodes
  and nothing had written down.
- One code comment argued from platform folklore where the acceptance suite
  builds the case; it now points at the test.
- ⛔ The Windows CI lane is red on `f9239a5` at *Install pinned Rust toolchain*,
  a TCP connect timeout to `static.rust-lang.org` before any repository code
  ran, with every later step skipped and the Linux lane green. Not a defect in
  the change, and not evidence about the tree either.
- Deployment: nothing deployed. No capture was taken.

### 2026-09-05T10:55:00Z

- ⛔ Moved the client entries behind `CORPUS-01` in the work order, on a
  measurement rather than a preference. A client acceptance needs a capture, a
  capture needs a host `assert-disposable.sh --egress` does not refuse, and a
  session host is refused; the Windows guard pair does not exist at all. Record:
  [`TODO/clients.md`](TODO/clients.md).
- ⭐ The resolver met a real target for the first time.
  `fetch-releases.sh qbittorrent/qBittorrent` answered through
  `api.gh.pkgforge.dev` and `resolve-stable` selected 5.2.3, published
  2026-07-07, over three superseded candidates with every verdict kept and a
  digest of the bytes read. The release offers a Linux AppImage, a Windows
  setup.exe and a source tarball, each with a detached signature.
- ⚠ The run has nowhere durable to be recorded, which is what put `CORPUS-01`
  first: a measurement with no store is a file nobody can cite.
- ⚠ The listing answered with four releases, fewer than the project has, and
  whether the mirror paginates or answers a subset was not established. Nothing
  depends on it yet; the next entry to use the route measures it.
- Deployment: nothing deployed. Network: one release listing read, no artifact
  downloaded and nothing installed.

### 2026-09-05T10:30:00Z

- Closed `CI-05`. `check-project` and its PowerShell twin now refuse a `cargo
  test` invocation that selects by test **name**, because a filter matching none
  prints `running 0 tests` for every binary and exits 0. Record:
  [`TODO/ci.md`](TODO/ci.md).
- ⚠ The entry claimed all nine bare-filter acceptance commands had been
  rewritten and that was false: only the observer entries were. Five `Prove`
  commands were still of that form, in `FOUND-03` and all four `SCHEMA-*`
  entries. Each is corrected, and each corrected command was run before it was
  written down.
- ⛔ The door sweep found the second door and it is the one that matters more.
  An entry's `Prove` is run by a person; a workflow's `run:` is run by every
  push, and a bare filter there reports green over zero tests with nobody
  reading it. Both are covered, with separate extractors and one tokeniser.
- ⭐ Scoped to `Prove:` paragraphs rather than carrying an exclusion list. A
  `Closure evidence` paragraph records what was run on a past tree and rewriting
  one would falsify the record, and two entries have to quote the command that
  caused the defect. Every bare filter left in the tree is one of those two.
- Guard mutation: 21 cases, each verified to have changed the file, both halves
  compared on exit code **and** output. All 21 landed on the intended verdict
  and the twins agreed on all 21. The cases include the three that would make a
  careless rule fire on correct usage.
- The `forbidden-patterns.md` row now points at the check instead of asking a
  reader to remember the rule, which is what that page asks for.
- Deployment: nothing deployed.

### 2026-09-05T09:45:00Z

- Closed `OBS-09`, the raw evidence journal and bundle writer, which completes
  the observer layer. A run's transcript now becomes one
  `bit-ids/transcript/1` artifact per endpoint plus the manifest rows describing
  them, and a first vertical capture is possible from here. Record:
  [`TODO/observer.md`](TODO/observer.md).
- ⛔ The digest is of the file and the file is compared against the buffer. A
  writer that digests what it meant to write cannot detect a short write, and a
  truncated file digests to a value matching itself, so reading back closes only
  half of it. One comparison serves the write path and a later `verify`, so the
  guard that cannot be provoked at write time is proved by the caller that can.
- ⛔ The door sweep found a gate on how a path is spelled and none on where it
  resolves. A symlink in a reused bundle root satisfies every canonical-path
  rule and lands the artifact outside, with the manifest citing a path that
  reads as inside. The root is now resolved once and every artifact's directory
  must resolve under it; a path already held by a symlink, a file or a directory
  is refused rather than followed or overwritten.
- ⭐ That check has a second half that is easy to miss: without resolving the
  root, it refuses **every** write on a host whose root is itself reached
  through a symlink. The acceptance suite builds that case rather than arguing
  from platform folklore, and the mutation dropping it is refused by that test.
- ⛔ A transcript is never scrubbed and the type has no argument for it: the
  bytes a build put on the wire are the measurement. Scrubbing belongs to text a
  host produced, with every removal declared and counted, and the scrubber
  replaces what the caller names rather than guessing.
- An endpoint the transcript plan does not name is refused, not defaulted. A
  derived identifier and an assumed kind produce a manifest that validates and
  lies, because `E-MAN-52` and `E-MAN-53` only require the tool and the phase to
  name something the run declares.
- Guard mutation: 35 plants, 34 refused. ⚠ The first round refused 26, and all
  six misses were real gaps in the tests rather than equivalent mutants: the
  transcript schema, the producing tool and the phase were each asserted against
  a constant that moves with the code, or not at all. `TODO/observer.md` records
  it beside the `OBS-08` finding it repeats.
- Driven by a Python client that sent the bytes itself and then read the bundle
  off disk: 32 comparisons, 32 agree, including that each transcript holds
  exactly what that client sent and read back, request before answer. Four
  negative controls, and a rerun into the same root refused by name.
- `serde_json` added as a dev-dependency of `bit-ids-lab`. The lockfile diff is
  one line and no new package, because `bit-ids` already depends on it.
  [`docs/supply-chain.md`](docs/supply-chain.md) carries the layers.
- Deployment: nothing deployed. No capture was taken.

### 2026-09-05T08:30:00Z

- Closed `OBS-08`, the synthetic torrent. The generator was checkpointed in the
  tree last session; what landed now is the acceptance suite at
  `crates/bit-ids-lab/tests/synthetic_torrent.rs`, the guard-mutation pass and a
  driven pass. Record: [`TODO/observer.md`](TODO/observer.md).
- ⭐ The acceptance suite reads the info hash out of the **file**, by walking the
  raw metainfo and cutting out the byte range the `info` key maps to. Comparing
  a re-encode of the value the generator kept cannot see an info hash naming a
  dictionary the file does not contain, because both halves move together; the
  mutation pass plants exactly that and it is refused by this test alone.
- ⛔ Pinned the payload's byte stream, which nothing was checking. A generated
  torrent is citable only while its bytes are a function of its declared inputs,
  and a drift in the `SplitMix64` arithmetic invalidates every
  `capture.fixture` already recorded while staying reproducible, seed-dependent
  and prefix-stable, which is all a naive test asserts. Four plants that nothing
  else catches are now refused.
- ⚠ Pinned `PIECE_HASH_LEN`, `MIN_PIECE_LENGTH` and `MAX_PAYLOAD_BYTES` to their
  literals, and moved the test spec off the piece-length floor. A constant every
  test reads is a constant no test can check: narrowing the piece hash re-chunked
  the `pieces` string and the comparison against it in one step, and a spec built
  at the floor made the declared piece length and the constant indistinguishable.
- `piece()` now checks both halves of its offset. The unchecked addition beside a
  checked multiplication is a guard on one of two arithmetic steps; it is
  unreachable on a 64-bit target, which is why the mutation for it is the one
  this entry could not refute.
- Guard mutation: 33 plants, 31 refused. Each was a literal replacement required
  to match exactly once, verified against the file's SHA-256 either side, with
  the acceptance exit code read unpiped.
- Driven by `libtorrent` 2.1.1.0, the engine `ENGINE-01` targets, and `torf`
  4.3.1, neither of which shares this project's reading of BEP 3, over a file
  written by a new
  `cargo run -p bit-ids-lab --example synthetic-torrent`. 26 comparisons, 26
  agree, including four negative controls: a reader that agrees with everything
  has agreed with nothing.
- Deployment: nothing deployed. No capture was taken and none is possible until
  `OBS-09` writes the evidence a manifest cites.

### 2026-09-05T07:50:00Z

- ⚠ Corrected the session record's CI row. It reported the seventh run as
  confirmed when that run had been cancelled, and a cancelled run is no
  evidence. Record:
  [`docs/history/SESSION-2026-09-05-OBSERVERS.md`](docs/history/SESSION-2026-09-05-OBSERVERS.md).
- The class now has a row in
  [`docs/conventions/forbidden-patterns.md`](docs/conventions/forbidden-patterns.md).
- Deployment: nothing deployed.

### 2026-09-05T07:20:00Z

- Checkpointed `OBS-08`, which stays OPEN. The synthetic torrent generator and
  its unit tests are in `crates/bit-ids-lab/src/torrent.rs`; the acceptance
  suite, the guard-mutation pass and the driven pass are not done and the entry
  names all three. Record: [`TODO/observer.md`](TODO/observer.md).
- Added `sha1` 0.11.0, the first third-party crate since `SCHEMA-01`. The info
  hash is SHA-1 by BEP 3, and the lockfile diff is one package because `sha2`
  already brought the same RustCrypto tree.
  [`docs/supply-chain.md`](docs/supply-chain.md) carries the argument.
- ⚠ `check-no-secrets --public` refused RFC 3174's test vectors as long hex, and
  was right to: forty lowercase hex digits is what a token looks like. Narrowed
  rather than switched off, and proven with a credential beside an allowed
  vector on one line.
- The door sweep made six response encoders private: one internal caller each
  and no external one is API surface with no consumer.
- ⛔ The session's guard-mutation pass found a defect in its own probe for the
  third time: one script never received the checksum guard the first two were
  given, and reported the twins agreeing over source it had not mutated.
- The session record is saved at
  [`docs/history/SESSION-2026-09-05-OBSERVERS.md`](docs/history/SESSION-2026-09-05-OBSERVERS.md).
- Deployment: no data branch, release or capture service was created.

### 2026-09-05T06:30:00Z

- Closed `OBS-05`, the BEP 10 and early-message observer, which completes all
  four core observer surfaces. Record: [`TODO/observer.md`](TODO/observer.md).
- ⭐ What an observer offers is a condition of the measurement, and the type says
  so: the reserved block is derived from the same value the extended handshake
  is, so a run cannot claim an offer it did not make.
- ⛔ Three states rather than a flag beside an option. The fourth state a flag
  would allow means the observer invented a negotiation, and the guard-mutation
  pass found that deleting the guard against it changed no test result.
- Fourteen defects planted one at a time, all fourteen refused. The first round
  found four misses and all four were real, including a send-once flag that
  could be cleared with nothing noticing. That is the second entry running in
  which a send-once flag went unproven.
- Driven with the BEP 3 peer from `OBS-04`, extended to negotiate BEP 10. The
  peer answered with a deliberately unsorted extension map and an unregistered
  top-level key, and the observer recorded the map in the order sent.
- Deployment: no data branch, release or capture service was created.

### 2026-09-05T05:10:00Z

- Closed `OBS-04`, the peer-wire handshake observer, in both roles. Record:
  [`TODO/observer.md`](TODO/observer.md).
- ⭐ The lab dials now, and the dial went into the loopback guard rather than
  beside it. `OBS-01`'s door sweep had already put `TcpStream::connect` on the
  list its own test greps for, so there was nowhere else to put it.
- The responder signature grew a connection identity. One responder serves every
  connection an endpoint accepts, so without it a peer observer sends a second
  handshake down the first connection. The journal carries the connection too,
  which is what separates a transcript of two concurrent peer connections.
- Nineteen defects planted one at a time, eighteen refused. The one that is not
  is `rebuilds_from_raw` returning true unconditionally, and the entry says what
  would have to be true for it to fire: it is a codec-regression detector, and
  planting a lossy encoder in `bit-ids-wire` **is** refused.
- The door sweep found three on the dial path: a stopped lab wrote opening bytes
  nothing would answer, a role could be attached to the wrong side, and a
  connection past the observer's cap was left buffering rather than closed.
- ⚠ A test asserted a scheduling outcome, passed alone, and failed twice in
  three loaded runs. That is this session's second finding of that shape. The
  whole workspace suite now runs four times in succession with no failure.
- Driven with a BEP 3 peer written from the specification, in both roles at
  once. The two roles produced different reserved blocks and different peer IDs,
  which is the role dependence the entry exists for.
- Deployment: no data branch, release or capture service was created.

### 2026-09-05T03:40:00Z

- Closed `OBS-03`, the UDP tracker observer, which completes both tracker
  surfaces. Record: [`TODO/observer.md`](TODO/observer.md).
- ⭐ The BEP 15 exchange is stateful and that is the measurement. A client
  connects before it announces, so an announce carrying a connection id the
  tracker never issued means the build reused a stale one, invented one or
  skipped the connect. Each is answered with the protocol's error action and
  recorded with its reason.
- The connection ids are a contiguous deterministic range, which inverts this
  project's rule about identifiers coming from a random source. The entry
  carries the argument and the rejected alternative.
- Seventeen defects planted one at a time, all seventeen refused on the first
  round, which is the first round this session to miss nothing.
- The door sweep found one rule enforced in one of two places, twice. The
  connection id was read by the codec for an announce and by the observer's own
  byte slice for a scrape. And the datagram list was capped while the refusal
  list was not.
- ⚠ The claim audit found `Datagram::connection_id` reporting BEP 15's magic
  value as a connection id for a connect request. `TODO/observer.md` says how
  that was found.
- Driven with a BEP 15 client written from the specification in Python, which
  connected, announced, and was refused when it used an id the tracker never
  issued.
- ⛔ No observer has been driven by a stock `BitTorrent` client, and none can be
  on a session host. `TODO/PROGRESS.md` carries which guard refuses it and why.
  `OBS-07` owns the stock-client controls and `CI-03` owns the runner.
- Deployment: no data branch, release or capture service was created.

### 2026-09-05T02:30:00Z

- Closed `OBS-02`. The `bit-ids-probe` crate holds the observers, one module per
  surface, and the HTTP tracker is the first. It keeps the exact head bytes,
  decodes with `bit-ids-wire`, and answers a bencoded response. Record:
  [`TODO/observer.md`](TODO/observer.md).
- ⚠ What an observer answers is part of the experiment. A client that asked for
  a compact peer list and got a peer list reports an error and changes what it
  does next, and that change would be recorded as identity when it is the
  observer's doing, so `compact` and `no_peer_id` are read out of the announce
  and honoured.
- ⛔ Framing uses the codec's own `head_end`, added to `bit-ids-wire` rather than
  written a second time in the observer. `TODO/observer.md` says what a second
  framer costs.
- Seventeen defects planted one at a time, all seventeen refused. ⚠ The first
  round's script used `sed` with `|` as its delimiter over Rust closures and four
  plants silently matched nothing, which is the same probe defect as the round
  before. The replacement asserts the match count instead.
- ⭐ The first round also found that nothing exercised a head terminated with
  bare newlines, so the framer's answer could be shortened by a byte with every
  test still passing. A corpus only tests the defects it contains an example of.
- The door sweep found the record unbounded, so a build announcing in a loop
  would grow it until the host ran out of memory; the cap now counts what it
  stopped keeping. A second `Content-Length` header was taking the first value,
  and two lengths that disagree cannot both frame the rest of the connection, so
  the request is refused.
- Driven with `curl`. Two announces answered `200`, recorded with curl's query
  order, curl's header spelling, the percent-encoding case per value, and a peer
  ID of 20 bytes for one and 11 for the other. The 11-byte one was reported
  rather than refused: the width is the record layer's rule, and a build that
  sends the wrong one is the measurement.
- Deployment: no data branch, release or capture service was created.

### 2026-09-05T01:20:00Z

- Closed `OBS-01`. The `bit-ids-lab` crate is the loopback observation lab that
  `OBS-02` through `OBS-05` plug into: it binds the sockets, holds the run
  deadline, records every byte in order with its direction, and releases every
  port on shutdown or on drop. Record: [`TODO/observer.md`](TODO/observer.md).
- ⛔ Every socket in the crate is created by one function, which refuses an
  address outside loopback before the syscall and reads the address back off the
  socket afterwards. A bind request and a bound address are different facts, and
  only the second says where traffic can reach.
- The lab speaks no protocol. That is what lets one deadline, one loopback guard
  and one journal serve every surface instead of each observer growing its own.
- No new third-party crate. `std::net` with one thread per endpoint, over an
  async runtime, for the reason [`docs/supply-chain.md`](docs/supply-chain.md)
  requires in the entry: the lab serves a handful of local connections and the
  runtime would be a large dependency in the component that must be reviewable.
- ⛔ The entry's acceptance command ran nothing and exited 0. `cargo test`
  filters by test name and a filter matching none succeeds, so it printed
  `running 0 tests` for every binary in the workspace. The nine `cargo test`
  acceptance commands in `TODO/` were all of that form; all nine now name a
  target or a package, and `CI-05` is filed for the check that would stop it
  returning.
- Fifteen defects planted one at a time, all fifteen refused. The first round
  found three misses, and one was a defect in the shipped code rather than in
  the tests: the responder was offered its buffer once per read, so a client
  sending two units in one write and waiting for two answers would have waited
  forever.
- ⚠ A finding about the probe rather than the code. The first mutation script
  did not check that its edits applied and one pattern silently matched nothing,
  so a run over unmutated source read as a guard that failed to fire. Every
  plant now compares the file's checksum either side.
- The door sweep added outbound connections to the greps that hold the
  one-door rule, because `OBS-04` is authored to dial, and refused a lab capped
  at zero connections, which accepted every connection and closed it at once.
- Deployment: no data branch, release or capture service was created.

### 2026-09-05T00:10:00Z

- Split `OBS-01`, which was the one XL entry and carried an instruction to split
  itself if its acceptance could not stay atomic. It could not: the Prove named
  a known client fixture and a Linux-and-Windows comparison, and no client
  adapter exists while a Windows capture is not permitted at all. The supervisor
  stays in `OBS-01` at L, the synthetic torrent is `OBS-08`, the evidence
  journal is `OBS-09`, and the cross-platform comparison is `OBS-10` with its
  three blocking entries named. Record:
  [`TODO/observer.md`](TODO/observer.md).
- ⛔ `check-project` compared one row of `TODO/SUMMARY.md` against the index and
  eleven against nothing. Setting `Observer` to 9 over ten open observer entries
  passed the whole gate. Found by planting the count while checking that the
  split's arithmetic was held by something, not by reading the file.
- The category-to-identifier mapping now lives in `TODO/SUMMARY.md` as a
  `prefix` column rather than in the checks, so the two twins read one mapping
  instead of holding one each. Both directions are checked: a prefix with no row
  and a row naming nothing are both refusals.
- ⚠ The first version of that check used `^\| [A-Z]` in both halves and the two
  halves disagreed, because PowerShell's `-match` is case-insensitive and awk's
  bracket expression is not. The regex was character for character identical.
  A data row is now recognised by its shape. Both new classes have rows in
  [`docs/conventions/forbidden-patterns.md`](docs/conventions/forbidden-patterns.md).
- Eight defects were planted one at a time and both halves refused all eight and
  agreed on every one, with the exit codes read unpiped.
- Deployment: no data branch, release or capture service was created.

### 2026-09-04T19:20:00Z

- Closed `ACQ-04`, and with it the acquisition group's blocking work. A client
  is now installed only on a host two independent guards have refused to
  disqualify, and they run before the install. Record:
  [`TODO/acquisition.md`](TODO/acquisition.md).
- ⛔ The boundary could not live in the record. `E-MAN-30` refuses to record a
  capture on a host somebody keeps, and cannot stop one: by the time a manifest
  exists, an untrusted installer has already run somewhere.
- A host is claimed by writing a marker and refused when one is already there,
  so a survived host produces evidence of itself. A provisioner token was the
  rejected design: a runner misconfigured to persist its disk still carries one
  and still means it.
- The egress guard reads the kernel's routing table and probes nothing. Reaching
  out from a machine the guard exists to establish is contained would be the
  wrong order.
- Two defects found by driving it. The egress test used gawk-only functions, so
  on a POSIX awk it reported "could not establish" over a machine with a plain
  default route; it failed closed, but a guard that cannot run on a minimal
  image does not run where it matters. And the runner test claimed the real
  machine, so it passed once and failed every run after, which only running it
  twice finds.
- The manifest records what the guard read and when. A claim stamped after the
  run started is refused, because that is a report rather than a boundary.
- `check-runner` joins the gate on every run. The Windows gate reports it as a
  skip naming the reason: ⛔ the guards are Linux-only, so a Windows capture is
  not permitted rather than permitted with a warning.
- Deployment: no data branch, release or capture service was created.

### 2026-09-04T18:30:00Z

- Closed `ACQ-03`. Every route already had to report the version the record
  declares, so the labels always agreed; what was missing was whether that
  agreement was backed by anything. Record:
  [`TODO/acquisition.md`](TODO/acquisition.md).
- Two schema changes carry the rest: an executable digest per route rather than
  one per record, and a capture that says which route's install went on the
  wire. One digest collapsed the very difference this entry detects, and a
  record silent about which install was watched let a reader assume both were.
- ⛔ A single capture of two byte-different installs is unresolved, not
  equivalent. Nothing put the other bytes on the wire, so nothing can say they
  behave the same. Reaching a positive verdict over differing bytes takes a
  capture through each route, which is what the cross-record comparison is for.
- Two routes that installed identical bytes are one build, and observing one
  observed it. That is the only case where a single capture settles it.
- The refusals are two codes, not one: a divergence needs adjudicating and an
  unresolved record needs a second capture. Both are asked from the existing
  publication gate rather than beside it.
- A door sweep found the cross-record comparison guarding against two captures
  of one route but not against one record passed twice, which would have agreed
  on every field for the most trivial reason available.
- Deployment: no data branch, release or capture service was created.

### 2026-09-04T17:35:00Z

- Closed `ACQ-02`. Something can now decide which version to acquire, and the
  decision is a document that keeps every candidate it weighed, the bytes each
  source answered with, and the instant it was made. Record:
  [`TODO/acquisition.md`](TODO/acquisition.md).
- ⛔ Version strings are not comparable in general. Sorting tags as text puts
  `4.1.10` before `4.1.9`, and a project can publish a preview without setting
  the prerelease flag. A target declares how it spells versions, both stability
  signals are believed, and a candidate the scheme cannot order blocks the
  resolution rather than being skipped: skipping yields an older version chosen
  confidently, with nothing saying a newer one was seen and not understood.
- What settles an unorderable candidate is a second signal, not a looser rule.
  One published strictly before the winner cannot be the newest whatever its tag
  says. With no date, or a later one, it still blocks.
- The shell half fetches and nothing else. The digest in a resolution is then of
  what arrived rather than of what a parser reconstructed.
- Driven against four real projects through the route `docs/AGENTS.md` rule 8
  prescribes, since direct `api.github.com` answers 403 from this host. The
  first run failed closed over 51 of `transmission`'s own decade-old tags and
  was right to; that is what produced the publication-order rule.
- Two defects the tests found: `4.1` and `4.1.0` were ordered rather than
  compared equal, because a shorter component vector sorts first; and a schema
  check `from_json` could never reach was deleted rather than left as a guard
  nobody knows works.
- CI now finds every tracked shell script rather than listing two directories.
  A list is what let a new directory arrive unchecked on the lane meant to
  check it.
- Deployment: no data branch, release or capture service was created.

### 2026-09-04T16:40:00Z

- Closed `ACQ-01`. A route record now carries the typed kind, what resolved it,
  what delivered it, the original URL, the immutable identity of what it asked
  for, the artifact digest and signature, and the evidence of the installed
  version. Record: [`TODO/acquisition.md`](TODO/acquisition.md).
- ⛔ Two routes are two routes only if nothing they depend on is shared. The
  resolver and the delivery mechanism are separate values and a record whose
  routes share either is refused, because the two-route rule was otherwise
  satisfiable by asking one index twice under two names.
- The identity of what a route asked for is typed to its kind, so a release
  asset is a repository, a tag and a file name, and a source build is a full
  commit. An abbreviated commit is refused: that is the shape `FOUND-02`
  measured passing an action-pin rule written to refuse floating references.
- An installed version cites the process output the build printed. A version
  read out of a packet capture or a filename is not the build speaking, and the
  citation is checked to resolve and to be process output.
- The signature disposition is compared across the two documents. That
  comparison can fail, which is why it exists where `E-BND-10` was deleted:
  nothing else forces the run's record and the record's claim into step.
- A new code silently reused `E-BND-12`, which the capture-instant comparison
  already held. Two checks under one code is worse than an unclear message, and
  the manifest coverage test refused the change until the renumbered one had a
  planted defect.
- Driving the validators found what the suite could not: the signature
  diagnostic printed a spelling that appears nowhere in the document it was
  telling an operator to go and read.
- Deployment: no data branch, release or capture service was created.

### 2026-09-04T15:40:00Z

- Closed `FOUND-03`, the last foundation piece before acquisition. A new
  `bit-ids-wire` crate carries byte-exact codecs for the HTTP tracker, the UDP
  tracker and the peer wire, plus a synthetic fixture corpus with a committed
  digest index. Record: [`TODO/foundation.md`](TODO/foundation.md).
- The invariant is that decode then encode reproduces the input byte for byte.
  Every retention rule in `docs/architecture.md` section 5, meaning query and
  header order, duplicate fields, percent-encoding hex case, all eight reserved
  bytes and early message order, is destroyed by the convenient implementation, and a
  round trip catches all of them at once because a decoder that dropped a
  detail has nothing to write back.
- The codecs observe rather than impose. Unsorted bencode keys, `i-0e`, a bare
  newline terminator, an unassigned message id and a non-standard handshake
  protocol string are recorded, because each is a difference between builds and
  refusing one turns an observation into a parse failure. Nothing maps a
  peer-ID prefix or a `v` string to a client name, which would put a refused
  input inside the component every observer trusts.
- Nine lossy defects were planted in the codecs one at a time and every one was
  refused by the fixture corpus alone. Two were not caught on the first attempt
  and both are fixed here: the corpus never reached `bencode::encode`, because a
  message re-encodes from payload bytes held verbatim, and no fixture carried a
  bare newline for a terminator-repairing decoder to fail against.
- A door sweep found three holes and all three are fixed: `FixtureIndex`
  derived `Deserialize`, so `serde_json::from_str` skipped the corpus-digest
  check; `load_directory` filtered for `*.json` over a non-recursive listing, so
  a fixture in a subdirectory would have been silently skipped rather than
  refused; and the HTTP head cap only fired when there was no blank line at all,
  so a head that ended just past the cap parsed in full.
- The corpus was replayed over real loopback TCP one byte per write and decoded
  incrementally. Reading the same announce datagram with the wrong direction
  yields an unassigned action and refuses to be an announce, rather than a
  plausible wrong answer.
- No new third-party crate. The lockfile diff is the workspace member and
  nothing else.
- Deployment: no data branch, release or capture service was created.

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
