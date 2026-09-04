# Scripts

Project scripts resolve the repository from their own location and may be run
from any working directory.

- [`doctor/`](doctor/README.md) reports host capabilities without changing the
  host.
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
