# Resume

**Task:** Take the work order in `TODO/PROGRESS.md` in dependency order,
committing and pushing each green unit to `main`.

**Resume point:** ⭐ **`CORPUS-01` through `CORPUS-03` are closed.** There is
somewhere durable to put a record, the append rule and the store-level
invariants are checked rather than trusted, and the consumer-facing views are
derived and proved. The next item is `PUB-01`, the deterministic release
assembler: it is what first builds a publication tree for `CORPUS-01`'s
comparison to run over, and `build-store` and `build-indexes` already write the
two halves it has to assemble. `CORPUS-04` follows, because a superseded record
cannot drop out of the latest view until supersession chains exist.

⛔ The client entries stay behind the corpus work on a measurement rather than a
preference. A client entry's acceptance needs a capture, a capture needs a host
`assert-disposable.sh --egress` does not refuse, and a session host is refused.
`TODO/clients.md` carries the three routes that were tried on 2026-09-05.

⛔ A Windows capture is not permitted yet. The disposable-host guards read
`/proc/net/route` and `/etc/machine-id`, so there is no boundary to run before
an install on Windows. `CI-03` owns the pair; `docs/capture-host.md` carries
both contracts.

**In flight:** Nothing. Three entries landed whole, each with its entry, index,
summary and record updated in the same change. No files half-edited.

**Tree:** Clean and level with `origin/main`, on `main`, full clone. ⚠ The
container started with `main` 15 commits **behind** `origin/main` and the
checkout on a `claude/*` branch; both were reconciled before any reading, so
re-measure rather than trusting a branch name. Measured on this host:

| command | result |
| --- | --- |
| `sh scripts/common/check-gate.sh` | 15 checks, 14 passed, 0 failed, 1 skipped |
| `pwsh -File scripts/common/check-gate.ps1` | 15 checks, 10 passed, 0 failed, 5 skipped |
| `cargo test --workspace --locked --all-targets` | 35 binaries, 329 passed, 0 failed |
| `cargo test --workspace --locked --doc` | 2 passed, 0 failed |
| `sh scripts/corpus/check-store.sh` | 14 cases, 14 passed, 0 failed |
| `sh scripts/corpus/check-corpus.sh` | 14 cases, 14 passed, 0 failed |
| `sh scripts/corpus/check-indexes.sh` | 10 cases, 10 passed, 0 failed |
| `cargo fmt`, `cargo clippy --workspace --locked --all-targets -- -D warnings`, `shellcheck`, `shfmt -d -i 2 -ci` | exit 0 |

⛔ **Run the gate with `sh scripts/common/check-gate.sh`, not from memory.** A
hand-typed subset after the last doc edit is what put a red `check-one-home` on
both CI lanes this session. The gate is one command precisely because a list run
by hand is run in the order somebody recalls it.

⭐ `pwsh` 7.4.6, `shellcheck` 0.10.0 and `shfmt` 3.14.0 were installed at session
start from the commands in `TODO/PROGRESS.md`. Install them before touching a
script; the `chmod +x` on the PowerShell tarball is not optional.

⛔ `check-remote-items` cannot be made to run here and installing `gh` does not
fix it. The CI Linux lane runs the gate with `--strict`, which turns that skip
into a failure, so the check is exercised there and only there.

⚠ Read each CI run's failing **step** before its conclusion: a failure above
*Rust check* is the runner's network rather than the tree. The workflow sets
`cancel-in-progress`, so a cancelled run is no evidence either way.

⚠ `check-twins` compares the two halves' answers on the tree it runs against, so
a rule that differs only on a defect the tree does not contain is invisible to
it. Compare a changed pair per planted mutation, not on a clean tree alone.

⛔ A mutation script that does not verify its own edits applied reports guards
failing to fire over unmutated source. Compare the file's digest either side of
every plant, require the literal to match exactly once, and exercise the probe's
own guards.

⚠ **And count a multi-line literal with something that understands one.**
`grep -F` splits a pattern containing a newline into separate alternatives, so a
unique multi-line literal counts as the sum of its lines and reads as ambiguous.
Measured on 2026-09-05: it miscounted three `CORPUS-01` plants. It fails safe
and still leaves those guards unproven.

⛔ **And read a harness exit of 2 as *could not run*, never as *refused*.** A
review pass counted it as a refusal on one of its two paths and reported a guard
proved over a plant that had not compiled. Separate the two on every path.

⚠ **`shellcheck` answers differently depending on how the files are grouped on
its command line.** A script that sources another is clean when both are handed
to one invocation and warns when checked alone. CI passes every script at once;
a contributor checking one file does not.

⛔ **A sourced shell library shares one namespace with its caller.** A harness
assigned its own `ROWS`, overwrote the accumulator, and printed ten passes over
eight lines. `store_report` now compares the rows it holds against the count it
prints, because a naming convention is a rule nobody checks.

⛔ **Read what a plant actually changed before believing it either way.** Of
eight over the index builder, two survived because the mutation was equivalent
on the fixture data rather than because a guard was missing, and four survived
because the tests really were thin. A surviving plant is a question, not a
verdict.

⛔ **A constant every test reads is a constant no test can check.** Found twice
last session by two entries, neither looking for it. Pin a specification's
values to their literals, and build a fixture at a value no default or bound
also has.

⭐ `E-MAN-52` and `E-MAN-53` only require an artifact's tool and phase to name
something the run declares, not the right thing. Assert those against what the
writer was given.

⛔ No observer has been driven by a stock `BitTorrent` client and none can be on
a session host: `sh scripts/acquisition/assert-disposable.sh --egress` exits 1
here. `OBS-07` owns the stock-client controls and `CI-03` owns the runner.

⭐ The last session's record is
[`SESSION-2026-09-05-EVIDENCE.md`](SESSION-2026-09-05-EVIDENCE.md).

**Paste:** Read `docs/AGENTS.md` in full, follow its start protocol, and take
the first unblocked item from `TODO/PROGRESS.md`. Work on `main`. Re-measure the
clone, branch, identity and remote rather than trusting this file.
