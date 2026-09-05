# Resume

**Task:** Take the work order in `TODO/PROGRESS.md` in dependency order,
committing and pushing each green unit to `main`.

**Resume point:** Foundations, schema, acquisition, the observation lab and both
tracker observers are closed. ⚠ No capture is possible yet regardless: the peer
wire has no observer and no torrent exists for a client to announce about.
`OBS-04` is in flight, `OBS-05` follows it, and `OBS-08` is what a client needs
before it will say anything at all.

⛔ A Windows capture is not permitted yet. The disposable-host guards read
`/proc/net/route` and `/etc/machine-id`, so there is no boundary to run before
an install on Windows. `CI-03` owns the pair; `docs/capture-host.md` carries
both contracts.

**In flight:** `OBS-04`, the peer-wire handshake observer. Nothing is
half-edited; the tree is clean at every commit.

⛔ `OBS-04` needs the lab to dial out, and nothing in `bit-ids-lab` does yet. The
loopback guard covers binding and a test greps the crate for
`TcpStream::connect` outside `bind.rs`, so the dial lands in the guard rather
than beside it.

**Tree:** Clean and level with `origin/main`, on `main`, at `4453f88`.
Measured on this host after closing `OBS-03`:

| command | result |
| --- | --- |
| `sh scripts/common/check-gate.sh` | 12 checks, 11 passed, 0 failed, 1 skipped |
| `pwsh -File scripts/common/check-gate.ps1` | 12 checks, 10 passed, 0 failed, 2 skipped |
| `cargo test --workspace --locked --all-targets` | 22 binaries, 219 passed, 0 failed |
| `cargo test --workspace --locked --doc` | 2 passed, 0 failed |
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

⭐ The same hazard is not confined to the shell twins, and it cost three more
misses this session: a datagram buffer shrunk to four bytes with no fixture
larger than three, and a head framer shortened by a byte with no head terminated
in bare newlines. A corpus only tests the defects it contains an example of.

⛔ **A mutation script that does not check its own edits applied is a script that
reports guards failing to fire over unmutated source.** That happened twice here,
which is the defect `docs/methodology/reviews.md` records about this template's
own patch script. Compare the file either side of every plant, and prefer literal
string replacement to a regular expression.

⚠ An acceptance command that names a bare test filter can exit 0 having run
nothing. `CI-05` is the check for it; until that exists, write every `cargo test`
acceptance as `-p <package>` or `--test <target>`.

⛔ No observer has been driven by a stock `BitTorrent` client and none can be on
a session host: `sh scripts/acquisition/assert-disposable.sh --egress` exits 1
here, so running one would be the capture that boundary refuses.

**Paste:** Read `docs/AGENTS.md` in full, follow its start protocol, and take
the first unblocked item from `TODO/PROGRESS.md`. Work on `main`. Re-measure
the clone, branch, identity and remote rather than trusting this file.
