# Resume

**Task:** Take the work order in `TODO/PROGRESS.md` in dependency order,
starting at `SCHEMA-01`, committing and pushing each green unit to `main`.

**Resume point:** `SCHEMA-01` is closed and pushed. The next item is
`FOUND-02`, which `PROGRESS.md` moved ahead of the remaining schema work
because `SCHEMA-01` introduced the first third-party crates.

**In flight:** Nothing. The tree is coherent.

**Tree:** Clean and level with `origin/main`, on `main`.
`sh scripts/common/check-gate.sh` is green: 9 passed, 0 failed, 2 skipped.
`cargo fmt --all -- --check`, `cargo check`, `cargo test` and `cargo clippy`,
each `--workspace --locked --all-targets`, exit 0.

⚠ The two skips are `check-remote-items` (no `gh` on this host) and
`check-twins` (no `pwsh`). The PowerShell half of every paired check is
therefore unexercised here, and `scripts/common/check-no-secrets.ps1` was
changed in this session alongside its `sh` twin. Only the CI Windows lane has
run it. Read that pair before trusting it.

**Paste:** Read `docs/AGENTS.md` in full, follow its start protocol, and take
the first unblocked item from `TODO/PROGRESS.md`. Work on `main`. Re-measure
the clone, branch, identity and remote rather than trusting this file.
