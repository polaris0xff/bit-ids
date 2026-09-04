# Resume

**Task:** Take the work order in `TODO/PROGRESS.md` in dependency order,
committing and pushing each green unit to `main`.

**Resume point:** `FOUND-03`, `ACQ-01` and `ACQ-02` are closed. The next item
is `ACQ-03`, the same-version multi-route verifier. It needs a version scheme
per target in the catalogue, which `ACQ-02` deliberately left to it.

**In flight:** Nothing. The tree is coherent.

**Tree:** Clean and level with `origin/main`, on `main`.
`sh scripts/common/check-gate.sh` is green: 10 passed, 0 failed, 1 skipped.
`cargo test --workspace --locked --all-targets` is 144 passed, 0 failed.
`cargo fmt --all -- --check`, `cargo check`, `cargo clippy` at
`--workspace --locked --all-targets`, `shellcheck` and `shfmt -d -i 2 -ci` all
exit 0.

⭐ Install `pwsh`, `shellcheck` and `shfmt` before touching a script.
`TODO/PROGRESS.md` carries the commands, including the `chmod +x` the
PowerShell tarball needs on this image and which the previous version of that
snippet omitted.

⛔ `check-remote-items` cannot be made to run here and installing `gh` does not
fix it. `gh` 2.63.2 installs and then reports the environment token invalid, and
the other GitHub route this harness has is scoped to this repository alone, so
an action pin in another repository cannot be resolved by either route. The CI
Linux lane runs it on every push. A skip is not a pass.

⭐ A live release listing is reachable from here through
`https://api.gh.pkgforge.dev/`, which is what `docs/AGENTS.md` rule 8
prescribes. Direct `api.github.com` answers 403 on this host, so that route is
not a preference.

⚠ `check-twins` compares the two halves' answers on the tree it runs against,
so a rule that differs only on a defect the tree does not contain is invisible
to it. Compare a changed pair per planted mutation, not on a clean tree alone.

⭐ The same hazard is not confined to the shell twins. `FOUND-03` planted nine
lossy defects in the Rust codecs and two were missed on the first pass, each
because the corpus lacked the shape that would have failed. A corpus only tests
the defects it contains an example of. The mutation loop is worth re-running
whenever a codec changes.

**Paste:** Read `docs/AGENTS.md` in full, follow its start protocol, and take
the first unblocked item from `TODO/PROGRESS.md`. Work on `main`. Re-measure
the clone, branch, identity and remote rather than trusting this file.
