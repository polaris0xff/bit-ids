# Scripts

Project scripts resolve the repository from their own location and may be run
from any working directory.

- [`doctor/`](doctor/README.md) reports host capabilities without changing the
  host.
- [`acquisition/`](acquisition/fetch-releases.sh) retrieves a release listing
  and keeps the exact bytes. It does not parse, sort or decide.
- [`acquisition/check-cache.sh`](acquisition/check-cache.sh) drives the artifact
  cache through a source that moved and asks `check-licences --permitted` what
  the register allows, so the tie between the two is a call rather than a second
  reading of the register.
- [`corpus/check-store.sh`](corpus/check-store.sh) plants, in a disposable tree,
  every defect the append-only store exists to refuse, and reads each exit code
  from the process that produced it.
- [`corpus/check-corpus.sh`](corpus/check-corpus.sh) does the same for the
  store-level invariants, against a store `build-store` wrote.
- [`corpus/check-indexes.sh`](corpus/check-indexes.sh) proves the three things a
  derived file owes: byte-identical clean builds, rows that resolve back to the
  records they came from, and a corrected record that has left the views while
  its bytes are still filed where they were.
- [`publishing/check-release.sh`](publishing/check-release.sh) assembles a
  release twice, compares the bytes, and hands the checksum file to `sha256sum
  -c`, which is a reader this project did not write.
- [`publishing/check-formats.sh`](publishing/check-formats.sh) renders every
  published format over a store that carries a correction, compares two renders
  as bytes, checks that the corrected record is not published as a record in any
  of them, and hands the assembled checksum file to `sha256sum -c`. ⚠ It does
  not decode the CBOR: this project has no CBOR reader, and using its own
  encoder to read it back would be checking the writer against itself.
- [`publishing/publish-data.sh`](publishing/publish-data.sh) appends an
  assembled bundle to the data branch: the append rule is checked before the
  push, nothing re-enables force, and the branch is read back and verified
  before the run says it happened.
- [`publishing/check-publish.sh`](publishing/check-publish.sh) drives that
  publisher against a bare repository it creates in a scratch directory, so the
  push path runs for real with no network and no credential.
- [`publishing/check-access.sh`](publishing/check-access.sh) publishes twice to
  such a repository, fetches each publication back over git, and checks every
  documented access path against what came back: that it resolves, that
  `sha256sum` agrees with its stated digest, and that every path the contract
  calls immutable is byte-identical across the two. ⚠ At least one current path
  must have moved, or the immutability comparison passes over two identical
  publications.
- [`common/check-examples.sh`](common/check-examples.sh) extracts every shell
  example out of [`../docs/consuming.md`](../docs/consuming.md) and runs it
  against a publication it assembles. ⛔ The page is what runs, never a copy
  beside it, and its own selection rule is checked: a run that took every fenced
  block is a run whose language rule has stopped applying.
- [`publishing/check-catalogue.sh`](publishing/check-catalogue.sh) drives the
  consumer library against a real publication: it opens, it answers `1.2.10`
  over `1.2.3`, and every defect a consumer could be handed is refused. ⭐ It
  also sweeps the crate for sockets and transports, because "no network access"
  is a property of what the code cannot do rather than of what one run did, and
  it checks its own needle list against the crate that owns the sockets.
- [`ci/check-staleness.sh`](ci/check-staleness.sh) drives the staleness monitor
  over real stores and real resolutions: a new stable release opens one request,
  a preview and a release already measured open none, and a second run over a
  tracker holding the first run's request opens nothing. ⭐ It re-derives the
  request identifier with `python3`'s SHA-256, which is an implementation this
  project did not write.
- [`ci/check-workflow.sh`](ci/check-workflow.sh) copies the working tree into a
  scratch repository, plants a defect of each class the pipeline exists to
  catch, and runs the offending workflow step against it. Every command it runs
  is read out of `.github/workflows/ci.yml` by job and step name, so a harness
  that has drifted from CI reports a missing step rather than a pass.
- [`corpus/store-lib.sh`](corpus/store-lib.sh) is sourced by twelve of the
  harnesses above and by `publishing/publish-data.sh`, and is never run.
  ⚠ The count is measured rather than "all of them": `acquisition/check-runner.sh`
  is listed above and does **not** source it, and the publisher is not a harness
  at all. A sentence saying "all of the harnesses above" was already describing a
  set two files differ from, which is the shape a claim audit exists to find. ⚠ It sits under `corpus/` because that is where the first harness
  to need it was, and a publishing check sources it across directories rather
  than growing a second copy. It holds what a mutation harness needs: build an example, make a scratch
  tree, digest a directory, verify a plant landed, count a row.
- [`common/check-gate.sh`](common/check-gate.sh) and
  [`common/check-gate.ps1`](common/check-gate.ps1) run the local gate.
- `common/check-project.sh` and `common/check-project.ps1` validate bit-ids
  structure, catalogue coverage, todo counts, action pins, and the shell-first
  implementation rule.
- `common/check-licences.sh` and `common/check-licences.ps1` check the register
  in `catalogue/licences.toml` against the catalogue and the lockfile in both
  directions, refuse a row with no disposition, and refuse an installer-shaped
  file in the tree.

Shell is the default orchestration language. Rust owns parsing, normalization,
validation, indexing, and publishing. Python requires a recorded need that
cannot reasonably be met by those two layers.

`check-twins.sh` has no PowerShell twin because it executes and compares both
halves of every listed pair. The gate runners are deliberately absent from its
pair list because including a runner would recurse.

⚠ The twin rule is about `common/`, where every script is a check that emits a
comparable `--json` verdict. `acquisition/fetch-releases.sh` has no twin and
does not belong in the pair list: what it emits depends on the network, so
running two implementations against one tree would compare the clock rather
than the answer, which is the exact failure `check-twins.sh` documents. A
Windows capture host will need a PowerShell fetcher; `ACQ-04` owns the runner
contract and is where that lands, rather than a second implementation written
now with nothing exercising it.

`acquisition/check-runner.sh` and `acquisition/check-cache.sh`, the three
`corpus/check-*.sh` harnesses, the three `publishing/check-*.sh` ones and
`ci/check-staleness.sh`, `publishing/check-access.sh` and
`publishing/check-catalogue.sh` and `common/check-examples.sh` are the
mutation provers, and none has a twin. All twelve run in the `sh` gate and are reported as declared rows in the
PowerShell one. `check-runner` proves guards that read `/proc/net/route`, so it has nothing
to prove on Windows until `CI-03` writes the Windows pair. The six corpus and publishing
provers hold rules that are not platform-specific at all, and the Rust suite
exercises every one of them on both CI lanes; ⚠ what they plant includes a
symbolic link and a named pipe against a real filesystem, and neither is
available to an unprivileged Windows session. A second
implementation that skipped those two plants would report a smaller pass under
the same name, which is the shape `check-twins.sh` calls invisible drift.

⚠ `check-staleness` is declared on the PowerShell lane for a **different**
reason, and its row says which: it plants nothing on a filesystem and needs no
POSIX-only feature. It is an `sh` harness with no PowerShell half, and it needs
`python3` for the independent derivation. Copying the neighbouring reason would
have recorded a gap that closes on the wrong event.

`ci/check-workflow.sh` is a thirteenth mutation prover and the one deliberately
kept **out** of the gate. Two of its cases run the workflow's own *Repository
gate* step, so a runner listed in the gate that also invokes the gate would
re-enter itself; `check-gate.sh` keeps `check-twins` out of its pair list for
that reason and this is the same contract. The workflow runs it as a step of its
own instead, after every other step has passed, so it still runs on every push.

⚠ A script that sources another is clean to `shellcheck` only when both are
handed to one invocation, because it will not follow a source it was not given.
CI passes every script at once and a contributor checking one file does not, so
the directives live in each file rather than in how it is called.

⚠ A prover that builds an example resolves it through `CARGO_TARGET_DIR` when
that is set. Composing the path as `target/debug/examples` instead was measured
on 2026-09-06 to make all five corpus and publishing provers exit 2 on a host
with that variable exported, which the gate reads as a skip rather than as a
failure, so a whole tier of guards stopped proving anything and said so nowhere.
