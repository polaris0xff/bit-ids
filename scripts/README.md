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
- [`ci/check-workflow.sh`](ci/check-workflow.sh) copies the working tree into a
  scratch repository, plants a defect of each class the pipeline exists to
  catch, and runs the offending workflow step against it. Every command it runs
  is read out of `.github/workflows/ci.yml` by job and step name, so a harness
  that has drifted from CI reports a missing step rather than a pass.
- [`corpus/store-lib.sh`](corpus/store-lib.sh) is sourced by all eight of the
  harnesses above and is never run. ⚠ It sits under `corpus/` because that is where the first harness
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
`corpus/check-*.sh` harnesses and the three `publishing/check-*.sh` ones are the
mutation provers, and none has a twin. All eight run in the `sh` gate and are reported as declared rows in the
PowerShell one. `check-runner` proves guards that read `/proc/net/route`, so it has nothing
to prove on Windows until `CI-03` writes the Windows pair. The six corpus and publishing
provers hold rules that are not platform-specific at all, and the Rust suite
exercises every one of them on both CI lanes; ⚠ what they plant includes a
symbolic link and a named pipe against a real filesystem, and neither is
available to an unprivileged Windows session. A second
implementation that skipped those two plants would report a smaller pass under
the same name, which is the shape `check-twins.sh` calls invisible drift.

`ci/check-workflow.sh` is a ninth mutation prover and the one deliberately
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
