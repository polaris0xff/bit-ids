# Resume

**Task:** Take the work order in `TODO/PROGRESS.md` in dependency order,
committing and pushing each green unit to `main`.

**Resume point:** ⭐ **The observer layer is complete.** `OBS-08` and `OBS-09`
both closed, so there is a torrent to point a client at and a run's transcript
becomes the content-addressed evidence a manifest cites. **A first vertical
capture is possible from here** on a host the `ACQ-04` guards permit, and what
stands between is a client adapter rather than any missing machinery.

⛔ **The next item is `CORPUS-01`, not a client, and that is a measured
reordering rather than a preference.** A client entry's acceptance needs a
capture, a capture needs a host `assert-disposable.sh --egress` does not refuse,
and a session host is refused. The provable prefix was run anyway on 2026-09-05
and ran out of somewhere to put its answer: the resolver selected qBittorrent
5.2.3 from a real release listing, correctly, and there is no store to record it
in. `CLIENT-01` carries the three routes that were tried, the measurement, and
what would unblock it.

⭐ `CORPUS-01` is fully provable here: an append-only store and a validator that
refuses a deletion or a byte change against the prior tree. No host, no network,
no client.

⛔ A Windows capture is not permitted yet. The disposable-host guards read
`/proc/net/route` and `/etc/machine-id`, so there is no boundary to run before
an install on Windows. `CI-03` owns the pair; `docs/capture-host.md` carries
both contracts.

**Tree:** Clean and level with `origin/main`, on `main`. Measured on this host:

| command | result |
| --- | --- |
| `sh scripts/common/check-gate.sh` | 12 checks, 11 passed, 0 failed, 1 skipped |
| `pwsh -File scripts/common/check-gate.ps1` | 12 checks, 10 passed, 0 failed, 2 skipped |
| `cargo test --workspace --locked --all-targets` | 30 binaries, 289 passed, 0 failed, three runs in succession |
| guard mutation, 4 passes | 94 plants, 91 refused, 3 named in the record |
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

⚠ **A red Windows lane may be a toolchain download rather than a defect.** Run
27 at `f9239a5` failed at *Install pinned Rust toolchain*, before any repository
code ran, with a TCP connect timeout to `static.rust-lang.org` (`os error
10060`), every later step `skipped`, and the Linux lane green. Run 28 at
`14f1da6` installed the same pinned toolchain on the same image and went green
on both lanes, so it was transient. ⭐ Read the failing **step** before reading
the run: a failure above *Rust check* is the runner's network, not the tree's.

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

⭐ **A bare test filter in a `Prove` or a workflow `run:` is now refused by
`check-project`**, in both twins, because `cargo test <name>` exits 0 over
nothing when no test name matches. `CI-05` closed it. ⚠ The other half of the
class is still a reading: `--test <target>` skips the library's own tests, and
`OBS-10` and `CORPUS-02` both still carry that form in their `Prove`. Write
every `cargo test` acceptance as `-p <package> --locked --all-targets`.

⛔ No observer has been driven by a stock `BitTorrent` client and none can be on
a session host: `sh scripts/acquisition/assert-disposable.sh --egress` exits 1
here, so running one would be the capture that boundary refuses. `OBS-07` owns
the stock-client controls and `CI-03` owns the runner.

⭐ **The last session's record is
[`SESSION-2026-09-05-EVIDENCE.md`](SESSION-2026-09-05-EVIDENCE.md).** It carries
the three guards that are not refuted and what would have to be true for each to
fire, so a later pass does not mistake an unreached guard for a proven one.

**Paste:** Read `docs/AGENTS.md` in full, follow its start protocol, and take
the first unblocked item from `TODO/PROGRESS.md`. Work on `main`. Re-measure
the clone, branch, identity and remote rather than trusting this file.
