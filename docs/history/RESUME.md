# Resume

**Task:** Take the work order in `TODO/PROGRESS.md` in dependency order,
committing and pushing each green unit to `main`.

**Resume point:** `OBS-08` is closed, so there is now a torrent to point a
client at. ⚠ No capture is possible yet regardless. `OBS-09` writes a run's
transcript out as the content-addressed evidence a manifest cites, nothing does
that today, and it is the only entry between here and a first vertical capture
on a host the `ACQ-04` guards permit. It is the next item and nothing blocks it:
`crates/bit-ids/src/manifest.rs` already specifies the artifact shape, so the
entry writes to a contract that exists.

⛔ A Windows capture is not permitted yet. The disposable-host guards read
`/proc/net/route` and `/etc/machine-id`, so there is no boundary to run before
an install on Windows. `CI-03` owns the pair; `docs/capture-host.md` carries
both contracts.

**Tree:** Clean and level with `origin/main`, on `main`. Measured on this host:

| command | result |
| --- | --- |
| `sh scripts/common/check-gate.sh` | 12 checks, 11 passed, 0 failed, 1 skipped |
| `pwsh -File scripts/common/check-gate.ps1` | 12 checks, 10 passed, 0 failed, 2 skipped |
| `cargo test --workspace --locked --all-targets` | 28 binaries, 270 passed, 0 failed |
| `cargo test --workspace --locked --doc` | 2 passed, 0 failed |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --locked --all-targets -- -D warnings` | exit 0 |
| `shellcheck` and `shfmt -d -i 2 -ci` over every tracked `*.sh` | exit 0 |

⭐ Install `pwsh`, `shellcheck` and `shfmt` before touching a script.
`TODO/PROGRESS.md` carries the commands, including the `chmod +x` the
PowerShell tarball needs on this image. All three installed cleanly here.

⭐ **A third-party torrent reader is installable on a session host and is a much
stronger driven pass than a decoder written for the purpose.** `libtorrent`
2.1.1.0 and `torf` 4.3.1 install into a virtualenv from the package index and
read a `.torrent` without touching the network. Parsing a file is not a capture
and needs no disposable host; running a client still does. `OBS-08`'s driven
pass used both, and `OBS-09` can drive an evidence bundle the same way.

⛔ `check-remote-items` cannot be made to run here and installing `gh` does not
fix it. ⭐ **That it runs in CI is verified rather than assumed:** the Linux lane
runs the gate with `--strict`, which turns a skip into a failure. ⚠ Read each
run's conclusion rather than counting runs: the workflow sets
`cancel-in-progress`, so a push landing while the previous run is still going
leaves it `cancelled`, which is not a pass and not a failure either.

⚠ `check-twins` compares the two halves' answers on the tree it runs against,
so a rule that differs only on a defect the tree does not contain is invisible
to it. Compare a changed pair per planted mutation, not on a clean tree alone.

⛔ **A mutation script that does not check its own edits applied reports guards
failing to fire over unmutated source.** That happened three times two sessions
ago. `OBS-08`'s harness requires the literal to match exactly once, compares the
file's SHA-256 either side of every plant, and reports a plant that did not
apply as `NOT-PLANTED` rather than counting it. ⭐ Those three guards were
themselves exercised, with an absent literal, an ambiguous one and a no-op edit,
because a probe's guard is a guard like any other.

⭐ **A constant every test reads is a constant no test can check.** `OBS-08`
found two: nothing pinned `PIECE_HASH_LEN`, so narrowing it re-chunked the
`pieces` string and the comparison against it together, and a test spec built at
`MIN_PIECE_LENGTH` made a declared piece length and the floor indistinguishable.
Pin a specification's values to their literals, and build a fixture at a value
no default or bound also has.

⭐ **A corpus only tests the defects it contains an example of.** `OBS-08`'s
payload stream had nothing pinning it: reproducibility, seed-dependence and
prefix-stability all survive any change to the arithmetic, so the module's own
unit tests could not see a drift that would invalidate every fixture digest
already recorded.

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
