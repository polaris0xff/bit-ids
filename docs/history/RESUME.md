# Resume

**Task:** Take the work order in `TODO/PROGRESS.md` in dependency order,
committing and pushing each green unit to `main`.

**Resume point:** ⭐ **`CORPUS-01` and `CORPUS-02` are closed.** There is
somewhere durable to put a record, the append rule is checked rather than
trusted, and a whole store is validated rather than one document at a time. The
next item is `CORPUS-03`, the deterministic indexes and latest views: fully
provable here, with `publishable_view` as the report it builds on and
`examples/build-store.rs` writing a store to build indexes over. `PUB-01` follows
and is what first assembles a tree for the append comparison to run against.

⛔ The client entries stay behind the corpus work on a measurement rather than a
preference. A client entry's acceptance needs a capture, a capture needs a host
`assert-disposable.sh --egress` does not refuse, and a session host is refused.
`TODO/clients.md` carries the three routes that were tried on 2026-09-05.

⛔ A Windows capture is not permitted yet. The disposable-host guards read
`/proc/net/route` and `/etc/machine-id`, so there is no boundary to run before
an install on Windows. `CI-03` owns the pair; `docs/capture-host.md` carries
both contracts.

**In flight:** Nothing. Both entries landed whole, each with its entry, index,
summary and record updated in the same change. No files half-edited.

**Tree:** Clean and level with `origin/main`, on `main`, full clone. ⚠ The
container started with `main` 15 commits **behind** `origin/main` and the
checkout on a `claude/*` branch; both were reconciled before any reading, so
re-measure rather than trusting a branch name. Measured on this host:

| command | result |
| --- | --- |
| `sh scripts/common/check-gate.sh` | 14 checks, 13 passed, 0 failed, 1 skipped |
| `pwsh -File scripts/common/check-gate.ps1` | 14 checks, 10 passed, 0 failed, 4 skipped |
| `cargo test --workspace --locked --all-targets` | 34 binaries, 317 passed, 0 failed |
| `cargo test --workspace --locked --doc` | 2 passed, 0 failed |
| `sh scripts/corpus/check-store.sh` | 14 cases, 14 passed, 0 failed |
| `sh scripts/corpus/check-corpus.sh` | 14 cases, 14 passed, 0 failed |
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
