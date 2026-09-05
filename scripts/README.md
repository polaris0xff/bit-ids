# Scripts

Project scripts resolve the repository from their own location and may be run
from any working directory.

- [`doctor/`](doctor/README.md) reports host capabilities without changing the
  host.
- [`acquisition/`](acquisition/fetch-releases.sh) retrieves a release listing
  and keeps the exact bytes. It does not parse, sort or decide.
- [`corpus/check-store.sh`](corpus/check-store.sh) plants, in a disposable tree,
  every defect the append-only store exists to refuse, and reads each exit code
  from the process that produced it.
- [`common/check-gate.sh`](common/check-gate.sh) and
  [`common/check-gate.ps1`](common/check-gate.ps1) run the local gate.
- `common/check-project.sh` and `common/check-project.ps1` validate bit-ids
  structure, catalogue coverage, todo counts, action pins, and the shell-first
  implementation rule.

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

`acquisition/check-runner.sh` and `corpus/check-store.sh` are the two mutation
provers, and neither has a twin. Both run in the `sh` gate and are reported as
named skips in the PowerShell one. `check-runner` proves guards that read
`/proc/net/route`, so it has nothing to prove on Windows until `CI-03` writes
the Windows pair. `check-store` proves rules that are not platform-specific at
all, and the Rust suite exercises every one of them on both CI lanes; ⚠ what it
plants is a symbolic link and a named pipe against a real filesystem, and
neither is available to an unprivileged Windows session. A second
implementation that skipped those two plants would report a smaller pass under
the same name, which is the shape `check-twins.sh` calls invisible drift.
