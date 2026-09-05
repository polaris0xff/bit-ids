# Resume

**Task:** Take the work order in `TODO/PROGRESS.md` in dependency order,
committing and pushing each green unit to `main`.

**Resume point:** ⭐ **The observer layer is complete.** `OBS-08` and `OBS-09`
both closed, so there is a torrent to point a client at and a run's transcript
becomes the content-addressed evidence a manifest cites. **A first vertical
capture is possible from here** on a host the `ACQ-04` guards permit, and what
stands between is a client adapter rather than any missing machinery.

The next item is `CLIENT-01`, then `CLIENT-06` and `CLIENT-05`. ⚠ Read that
entry's own acceptance first: a capture needs a disposable host and
`sh scripts/acquisition/assert-disposable.sh --egress` exits 1 on a session
host, so what can close here is the adapter and its fixtures rather than a
capture. Say which half was done.

⛔ A Windows capture is not permitted yet. The disposable-host guards read
`/proc/net/route` and `/etc/machine-id`, so there is no boundary to run before
an install on Windows. `CI-03` owns the pair; `docs/capture-host.md` carries
both contracts.

**Tree:** Clean and level with `origin/main`, on `main`. Measured on this host:

| command | result |
| --- | --- |
| `sh scripts/common/check-gate.sh` | 12 checks, 11 passed, 0 failed, 1 skipped |
| `pwsh -File scripts/common/check-gate.ps1` | 12 checks, 10 passed, 0 failed, 2 skipped |
| `cargo test --workspace --locked --all-targets` | 30 binaries, 288 passed, 0 failed |
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
and needs no disposable host; running a client still does.

⭐ **A driven pass gets its strength from the client knowing what it sent.**
`OBS-09`'s reads the bundle back with the same Python client that put the bytes
on the wire, so a transcript is checked against what actually happened rather
than against what the lab believed happened. A reader that only re-computes
digests is checking the writer against itself.

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

⛔ **A constant every test reads is a constant no test can check, and this
session hit it twice.** `OBS-08` found two: nothing pinned `PIECE_HASH_LEN`, so
narrowing it re-chunked the `pieces` string and the comparison against it
together, and a test spec built at `MIN_PIECE_LENGTH` made a declared piece
length and the floor indistinguishable. `OBS-09`'s mutation pass then found the
same shape again, in a transcript schema asserted only against the constant that
spells it. Pin a specification's values to their literals, and build a fixture
at a value no default or bound also has.

⭐ **`E-MAN-52` and `E-MAN-53` are weaker than they look.** They require an
artifact's tool and phase to name *something the run declares*, not the right
thing, so a writer that filed every artifact under another declared tool
produces a manifest that validates and lies. Assert those against what the
writer was given.

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
