# Resume

**Task:** Take the work order in `TODO/PROGRESS.md` in dependency order,
committing and pushing each green unit to `main`.

**Resume point:** All four core observer surfaces are closed: both trackers, the
peer handshake and BEP 10. ⚠ No capture is possible yet regardless. `OBS-08`
generates the torrent a client needs before it will announce about anything, and
`OBS-09` writes a run's transcript out as the evidence a manifest cites.

⛔ **`OBS-08` is in flight and is the first thing to pick up.**
`crates/bit-ids-lab/src/torrent.rs` carries the generator and its own unit
tests, which pass. Its entry names exactly what is missing: an acceptance suite
at `crates/bit-ids-lab/tests/synthetic_torrent.rs`, a guard-mutation pass over
the generator, and a driven pass that reads the generated `.torrent` with
something that is not this code. Nothing is half-edited; the tree is green and
the checkpoint is deliberate.

⛔ A Windows capture is not permitted yet. The disposable-host guards read
`/proc/net/route` and `/etc/machine-id`, so there is no boundary to run before
an install on Windows. `CI-03` owns the pair; `docs/capture-host.md` carries
both contracts.

**Tree:** Clean and level with `origin/main`, on `main`. Measured on this host
at the end of the session:

| command | result |
| --- | --- |
| `sh scripts/common/check-gate.sh` | 12 checks, 11 passed, 0 failed, 1 skipped |
| `pwsh -File scripts/common/check-gate.ps1` | 12 checks, 10 passed, 0 failed, 2 skipped |
| `cargo test --workspace --locked --all-targets` | 25 binaries, 250 passed, 0 failed |
| `cargo test --workspace --locked --doc` | 2 passed, 0 failed |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --locked --all-targets -- -D warnings` | exit 0 |
| `shellcheck` and `shfmt -d -i 2 -ci` over every tracked `*.sh` | exit 0 |

⭐ Install `pwsh`, `shellcheck` and `shfmt` before touching a script.
`TODO/PROGRESS.md` carries the commands, including the `chmod +x` the
PowerShell tarball needs on this image. All three installed cleanly here.

⛔ `check-remote-items` cannot be made to run here and installing `gh` does not
fix it. ⭐ **That it runs in CI is now verified rather than assumed:** the Linux
lane runs the gate with `--strict`, which turns a skip into a failure, and every
push this session went green.

⚠ `check-twins` compares the two halves' answers on the tree it runs against,
so a rule that differs only on a defect the tree does not contain is invisible
to it. Compare a changed pair per planted mutation, not on a clean tree alone.

⛔ **A mutation script that does not check its own edits applied reports guards
failing to fire over unmutated source.** That happened three times this session,
the third in the one script that never got the guard after the first two were
fixed. Compare the file's checksum either side of every plant, prefer literal
string replacement to a regular expression, and ⚠ **re-check a probe's patterns
after the code moves**: two of them silently stopped matching when counts and
signatures changed.

⭐ A corpus only tests the defects it contains an example of. It cost four
misses here: a datagram buffer shrunk below any fixture, a head framer shortened
by a byte with no head terminated in bare newlines, and twice a send-once flag
cleared with nothing reading again to notice.

⚠ An acceptance command that names a bare test filter can exit 0 having run
nothing, and `--test <target>` skips the library's own tests. `CI-05` is the
check for it; until then write every `cargo test` acceptance as
`-p <package> --all-targets`.

⛔ No observer has been driven by a stock `BitTorrent` client and none can be on
a session host: `sh scripts/acquisition/assert-disposable.sh --egress` exits 1
here, so running one would be the capture that boundary refuses. `OBS-07` owns
the stock-client controls and `CI-03` owns the runner.

**Paste:** Read `docs/AGENTS.md` in full, follow its start protocol, and take
the first unblocked item from `TODO/PROGRESS.md`. Work on `main`. Re-measure
the clone, branch, identity and remote rather than trusting this file.
