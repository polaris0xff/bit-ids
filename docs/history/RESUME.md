# Resume

**Task:** Take the work order in `TODO/PROGRESS.md` in dependency order,
committing and pushing each green unit to `main`.

**Resume point:** ⭐ **`OBS-11` is closed, and with it the observer layer.** Every
surface a build reaches for now has a module: both trackers, the peer handshake,
BEP 10, local discovery, peer exchange, the DHT, web seeding and message stream
encryption. `TODO/observer.md` carries all of it.

⛔ **Everything remaining in the work order is behind a capture host or an
operator decision.** `CLIENT-01`, `CLIENT-06`, `CLIENT-05`, `OBS-07` and `OBS-10`
need a host `assert-disposable.sh --egress` does not refuse; `CI-03` is what
would supply one and is itself the next unblocked item; `PUB-05` is blocked on
the dependency decision below. ⚠ **`CI-02`'s acceptance is fixture-driven and
`PUB-04`'s can be driven against a scratch bare repository**, so neither is as
blocked as the ordering suggests, and both are reachable without a host. Read
`TODO/PROGRESS.md`'s work order rather than assuming this list.

**In flight:** Nothing. `OBS-11` landed in five commits, each green and pushed
before the next began, with entry, index, summary, changelog and progress record
moved in the same change as the code.

**Tree:** Clean and level with `origin/main`, on `main`, full clone, `origin` is
`https://github.com/polaris0xff/bit-ids`.

⚠ **The container started on `claude/bit-ids-session-start-az23bd` with
`user.name` set to an agent**, and both were corrected before any editing: the
branch to `main` per rule 7, the identity to the operator's own per rule 11.
Local `main` was 30 behind and fast-forwarded. Re-measure all four rather than
trusting this file.

⭐ `pwsh` 7.4.6, `shellcheck` 0.10.0 and `shfmt` 3.14.0 were installed at session
start from the commands in `TODO/PROGRESS.md`, and the `chmod +x` on the
PowerShell tarball was needed exactly as that note says. Measured on this host at
session end:

| command | result |
| --- | --- |
| `sh scripts/common/check-gate.sh` | 20 checks, 19 passed, 0 failed, 1 skipped, 0 unavailable |
| `pwsh -File scripts/common/check-gate.ps1` | 20 checks, 11 passed, 0 failed, 1 skipped, 8 unavailable |
| `sh scripts/ci/check-workflow.sh` | 34 cases, 34 passed, 0 failed |
| `cargo test --workspace --locked --all-targets` | 44 binaries, 465 passed, 0 failed |
| `cargo test --workspace --locked --doc` | 3 passed, 0 failed |
| `cargo fmt --all --check`, `cargo clippy --workspace --locked --all-targets -- -D warnings` | exit 0 |

## What this session learned, in the order it hurts

⛔ **There is a third door and it is not a socket.** Where the lab listens and
where the lab sends are two questions `OBS-06` answered. Where the lab tells the
**target** to go is a third: a DHT `values` list, a BEP 19 `url-list` and a
tracker's peer list all hand the build addresses it dials itself, on its own
socket, so `bind::send_to` is never called on those packets. `bind::check_offered`
is the guard.

⛔ **And the door sweep found the same hole in the two oldest observers.**
`OfferedPeer`, shared by both tracker surfaces, had public fields and no check at
all since `OBS-02`. The enumeration named DHT and web seed; the grep named the
trackers. That is `docs/methodology/reviews.md` working exactly as written: the
list you wrote from memory has never been complete.

⛔ **A refusal variant nothing can produce is a guard that is not there.**
Deleting MSE's verification check left every test passing, because the only case
exercising a wrong key relies on random plaintext and random plaintext trips the
pad-length check first. The same shape `OBS-06` found in `peer_exchange`. ⚠ In
the same pass an assertion that was too narrow was corrected rather than forced:
the observer was right and the test was wrong.

⛔ **A constant written from memory is not a control.** MSE's `RC4` vector was
recalled rather than looked up, and a test asserting it proves only that the
implementation agrees with the constant. An independent Python `RC4` confirmed
it and found the comment named the wrong offset: MSE discards 1024 keystream
bytes, so a freshly keyed cipher produces the stream at offset 1024, not the
offset-zero block.

⚠ **A property claimed from one sample is a sample.** `curl`'s header order was
stated as a property after one run; five fetches across two request shapes now
back it. The project's own field-state model already requires two samples before
calling anything constant.

⛔ **The nine commit stamps before this session are fabricated.** They read
2026-09-06T10:30Z to 23:55Z, evenly spaced and round, while their committer dates
run 2026-09-05T17:42Z to 23:37Z. `conventions/git.md` section 3 says read it from
the machine with `date -u` and never type it. ⚠ **This session's stamps are
machine-read and therefore go backwards against the record above them**, which is
why its `CHANGELOG.md` entry sits in stamp order rather than at the top. The old
ones are not retro-corrected: rewriting somebody else's record of when they
worked is worse than the gap it closes.

⛔ **`synthetic-torrent` turned the gate red by being run.** It defaulted its
output into the repository root and `check-licences` refuses a `.torrent` there,
reading untracked-but-not-ignored files as well as tracked ones. The path is
required now. ⚠ An ignore rule was the other repair and it is the wrong one: an
ignore is a deletion nobody notices. ⭐ Nothing in the suite could have found it,
because no test runs that example from the repository root.

⚠ **A control has to name something still uncovered.** The `E-FIX-07` negative
control named `dht` as *the surface with no codec*, in two places, which stopped
being true the moment the codec landed. Both name `mse` now. A control that keeps
passing while asserting the opposite of what it was written for is what a
mutation pass exists to catch.

## Carried forward, unchanged

⛔ **Nothing has ever been published and no capture has been taken.** The
publisher has never run against this repository's own remote and must not until a
measured record exists; everything in the tree is synthetic and says so. Its
workflow has no automatic trigger, defaults to a dry run, and cannot succeed
today: its first step wants the bundle of a capture run.

⛔ **No observer has been driven by a stock `BitTorrent` client.** `curl`,
`libtorrent`'s bencode and a Python MSE initiator are independent implementations
written from specifications, which share this project's reading of the protocol.
`OBS-07` owns the stock-client controls and `CI-03` owns the runner.

⛔ **A Windows capture is not permitted.** The disposable-host guards read
`/proc/net/route` and `/etc/machine-id`, so there is no boundary to run before an
install on Windows. `CI-03` owns the pair.

⛔ **Run the gate with `sh scripts/common/check-gate.sh`, after the last edit.**
The last edit is the one made while writing the record, not the one that felt
like the end of the work. ⚠ **The gate is not the whole of part (a)**: `cargo
clippy`, `cargo fmt --check` and the test suite are separate rows in the table
above, and a clippy failure passed the gate twice this session before being
caught. ⚠ `check-workflow.sh` is not in the gate and is run separately, because
two of its cases run the gate.

⛔ `check-remote-items` cannot be made to run here and installing `gh` does not
fix it. It is the one observed skip above, and it is why the gate exits 1 under
`--strict` on this host and 0 on the Linux lane.

⛔ **A harness exit of 2 is *could not run*, never *refused***, and a plant that
did not compile is the same. Separate the two on every path.

⛔ **Read what a plant actually changed before believing it either way.** A
surviving plant is a question, not a verdict.

⚠ **Two twins agreeing is not two twins being right.** Compare them per planted
input, and give every planted input a declared expected outcome. The
`check-no-secrets` allowance added this session was planted against five inputs
on that discipline.

⚠ **A hex allowance must be anchored at both ends.** An unanchored `{40}` once
blanked the first forty characters of a longer run and let the remainder fall
under the threshold. The `MSE_` allowance is anchored on the name, on both quotes
and on exactly 192 digits.

⭐ **The strongest control available here is a reader this project did not
write.** `sha256sum -c` verifies a release, `cbor2` reads a canonical encoding,
`libtorrent` and `torf` read a generated torrent, `curl` is a complete HTTP
client, and Python's `pow` is an arbitrary-precision modular exponentiation.

⭐ **A push path can be driven for real with no network and no credential.** A
bare repository in a scratch directory is a remote as far as git is concerned.

⚠ Read each CI run's failing **step** before its conclusion: a failure above
*Rust check* is the runner's network rather than the tree.

## Pending operator decision

⛔ **One, and `PUB-05` carries it in full: how the SQLite rendering gets
written.** `rusqlite` brings a vendored C library and a build script into a
workspace whose lints say `unsafe_code = "forbid"`. The recommendation is the
crate, pinned, with the exception recorded against that one dependency rather
than the workspace lint relaxed. ⚠ Nothing is blocked behind the answer except
that one rendering.

⭐ The session record is
[`SESSION-2026-09-06-ADJACENT.md`](SESSION-2026-09-06-ADJACENT.md), which carries
what each of the five review passes swept and the three findings that were not
about the code being written.

**Paste:** Read `docs/AGENTS.md` in full, follow its start protocol, and take the
first unblocked item from `TODO/PROGRESS.md`. Work on `main`. Re-measure the
clone, branch, identity and remote rather than trusting this file.
