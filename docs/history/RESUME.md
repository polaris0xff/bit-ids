# Resume

**Task:** Take the work order in `TODO/PROGRESS.md` in dependency order,
committing and pushing each green unit to `main`.

**Resume point:** `SCHEMA-01`, `FOUND-02`, `SCHEMA-02` and `SCHEMA-03` are
closed. The next item is `SCHEMA-04`, the variability and repeated-sampling
model.

**In flight:** Nothing. The tree is coherent.

**Tree:** Clean and level with `origin/main`, on `main`.
`sh scripts/common/check-gate.sh` is green: 10 passed, 0 failed, 1 skipped.
`shellcheck`, `shfmt -d -i 2 -ci`, `cargo fmt --all -- --check`, and
`cargo check`, `cargo test` and `cargo clippy` at
`--workspace --locked --all-targets` all exit 0.

⭐ Install `pwsh`, `shellcheck` and `shfmt` before touching a script.
`TODO/PROGRESS.md` carries the three commands. Without them this host runs a
smaller gate than CI does, and that turned CI red twice in the last session.
The only check that still cannot run locally is `check-remote-items`, which
needs `gh`.

⚠ `check-twins` compares the two halves' answers on the tree it runs against,
so a rule that differs only on a defect the tree does not contain is invisible
to it. Compare a changed pair per planted mutation, not on a clean tree alone.

**Paste:** Read `docs/AGENTS.md` in full, follow its start protocol, and take
the first unblocked item from `TODO/PROGRESS.md`. Work on `main`. Re-measure
the clone, branch, identity and remote rather than trusting this file.
