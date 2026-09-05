# Resume

**Task:** Take the work order in `TODO/PROGRESS.md` in dependency order,
committing and pushing each green unit to `main`.

**Resume point:** Foundations, schema and acquisition are closed. ⚠ No capture
is possible yet regardless, because there is no observer. This session splits
`OBS-01` and then implements the lab and the transport observers, each against
the `bit-ids-wire` fixture corpus rather than a live client.

⛔ A Windows capture is not permitted yet. The disposable-host guards read
`/proc/net/route` and `/etc/machine-id`, so there is no boundary to run before
an install on Windows. `CI-03` owns the pair; `docs/capture-host.md` carries
both contracts.

**In flight:** Splitting `OBS-01`. Nothing is half-edited.

**Tree:** Clean and level with `origin/main`, on `main`, at `2fb8548`.
Measured at the start of this session, on this host:

| command | result |
| --- | --- |
| `sh scripts/common/check-gate.sh` | 12 checks, 11 passed, 0 failed, 1 skipped |
| `pwsh -File scripts/common/check-gate.ps1` | 12 checks, 10 passed, 0 failed, 2 skipped |
| `cargo test --workspace --locked --all-targets` | 14 binaries, 157 passed, 0 failed |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo check --workspace --locked --all-targets` | exit 0 |
| `cargo clippy --workspace --locked --all-targets -- -D warnings` | exit 0 |
| `shellcheck` over every tracked `*.sh` | exit 0 |
| `shfmt -d -i 2 -ci` over every tracked `*.sh` | exit 0 |

⭐ Install `pwsh`, `shellcheck` and `shfmt` before touching a script.
`TODO/PROGRESS.md` carries the commands, including the `chmod +x` the
PowerShell tarball needs on this image. All three installed cleanly here and
`check-twins` compared both halves as a result.

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
whenever a codec or a guard changes.

**Paste:** Read `docs/AGENTS.md` in full, follow its start protocol, and take
the first unblocked item from `TODO/PROGRESS.md`. Work on `main`. Re-measure
the clone, branch, identity and remote rather than trusting this file.
