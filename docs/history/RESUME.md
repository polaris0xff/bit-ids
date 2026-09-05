# Resume

**Task:** Take the work order in `TODO/PROGRESS.md` in dependency order,
committing and pushing each green unit to `main`.

**Resume point:** ⭐ **`CI-01` is closed, and with it the last open `P0`.** Both
gate runners separate a declared unavailability from an observed skip, so both CI
lanes run strictly and the Windows lane reports zero skipped;
`scripts/ci/check-workflow.sh` plants eight classes of defect and runs the
offending workflow step against each; the workflow declares per-job permissions
and timeouts and caches by lockfile; and the publisher has a workflow carrying
the job-scoped write permission and a concurrency group.

⭐ **`CORPUS-04` is closed as well.** A superseded record leaves every view and
keeps its path and its bytes, the derived document carries the correction chain
so an old identifier still finds what answers now, and a fork and a cycle are
each refused.

⭐ **And `PUB-03`**, over four of its five renderings: a combined JSON carrying
each record's own bytes, one compact document per line, a tabular view that
publishes what it omits, and deterministic CBOR. ⛔ The SQLite one is split out
as `PUB-05` and is **blocked on an operator decision** about a dependency, which
`TODO/PROGRESS.md` carries under pending decisions. The next unblocked item is
`CLIENT-01`, `CLIENT-06` or `CLIENT-05`, and each of those needs a capture host
this session does not have.

⭐ **And `FOUND-04`**, the licence register: every catalogue target and every
third-party package now has a recorded disposition, and six of the nine targets
with a GitHub upstream turn out to have no licence a detector can name, so those
rows say `unverified` rather than inventing one.

⭐ **And `ACQ-05`**, the artifact cache: the identity is the digest, so a source
that moved adds a retrieval rather than an artifact, and the bytes are kept only
where the register permits, which today is nowhere.

⭐ **And `OBS-06`**, over local discovery and peer exchange, with message stream
encryption, the DHT and web seeding split out as `OBS-11`. ⛔ **The lab had no
egress guard**: every socket went through `bind.rs` and every send did not, so a
datagram endpoint replied to the source address the sender wrote on the packet.
`bind::send_to` is the one door now and `.send_to(` is on the sweep's needle
list. An adjacent surface is behind a capability that has to be constructed.

⭐ **`OBS-11` is the next item that needs no capture host.** The containment, the
switch and the sweep are built, so each of DHT, web seed and MSE is a protocol
module and its acceptance. ⚠ Everything else the work order lists is behind a
capture host or an operator decision: `CLIENT-01`, `CLIENT-06`, `CLIENT-05`,
`OBS-07` and `OBS-10` need a host, `CI-03` is what would supply one, and `PUB-05`
is blocked on the decision below. ⚠ `CI-02`'s acceptance is fixture-driven and
`PUB-04`'s can be driven against a scratch bare repository, so neither is as
blocked as the ordering suggests. Read `TODO/PROGRESS.md`'s work order rather
than assuming this list.

⛔ **Nothing has ever been published.** The publisher has never run against this
repository's own remote and must not until a measured record exists; everything
in the tree today is synthetic and says so. Its workflow exists but has no
automatic trigger, defaults to a dry run, and cannot succeed at all today: its
first step wants the assembled bundle of a capture run and there are no
captures.

⛔ The client entries stay behind this work on a measurement rather than a
preference. A client entry's acceptance needs a capture, a capture needs a host
`assert-disposable.sh --egress` does not refuse, and a session host is refused.
`TODO/clients.md` carries the three routes that were tried on 2026-09-05.

⛔ A Windows capture is not permitted yet. The disposable-host guards read
`/proc/net/route` and `/etc/machine-id`, so there is no boundary to run before
an install on Windows. `CI-03` owns the pair; `docs/capture-host.md` carries
both contracts.

**In flight:** Nothing. `CI-01`, `CORPUS-04`, `PUB-03`, `FOUND-04`, `ACQ-05` and
`OBS-06` each landed whole, with entry, index, summary and record updated in the
same change. No files half-edited.

**Tree:** Clean and level with `origin/main`, on `main`, full clone. ⚠ The
container started on a stale `claude/*` branch whose commits had already been
merged: `origin` had deleted that branch, local `main` was 22 behind, and the
two were reconciled before any editing. Re-measure rather than trusting a branch
name. Measured on this host:

| command | result |
| --- | --- |
| `sh scripts/common/check-gate.sh` | 20 checks, 19 passed, 0 failed, 1 skipped, 0 unavailable |
| `pwsh -File scripts/common/check-gate.ps1` | 20 checks, 11 passed, 0 failed, 1 skipped, 8 unavailable |
| `sh scripts/ci/check-workflow.sh` | 34 cases, 34 passed, 0 failed |
| `cargo test --workspace --locked --all-targets` | 40 binaries, 392 passed, 0 failed |
| `cargo test --workspace --locked --doc` | 3 passed, 0 failed |
| `sh scripts/corpus/check-store.sh` | 14 cases, 14 passed, 0 failed |
| `sh scripts/corpus/check-corpus.sh` | 14 cases, 14 passed, 0 failed |
| `sh scripts/corpus/check-indexes.sh` | 15 cases, 15 passed, 0 failed |
| `sh scripts/publishing/check-release.sh` | 13 cases, 13 passed, 0 failed |
| `sh scripts/publishing/check-formats.sh` | 16 cases, 16 passed, 0 failed |
| `sh scripts/publishing/check-publish.sh` | 15 cases, 15 passed, 0 failed |
| `sh scripts/common/check-licences.sh` | 16 target rows and 22 dependency rows, all with a disposition |
| `sh scripts/acquisition/check-cache.sh` | 11 cases, 11 passed, 0 failed |
| `cargo test -p bit-ids-probe --locked --test adjacent_surfaces` | 7 cases, 7 passed, 0 failed |
| `cargo fmt`, `cargo clippy --workspace --locked --all-targets -- -D warnings`, `shellcheck`, `shfmt -d -i 2 -ci` | exit 0 |

⛔ **A check run before the last edit is a check that did not run.** `OBS-06`
put a red `Rust lints` on both CI lanes: clippy was clean, then one more line
was written, and `cargo fmt --check` was re-run while clippy was not. The lint
reproduced locally on the first try afterwards, which is the whole point. Run the
gate after the last edit, and the last edit is the one made while writing the
record, not the one that felt like the end of the work.

⛔ **Run the gate with `sh scripts/common/check-gate.sh`, not from memory.** A
hand-typed subset after a doc edit is what put a red `check-one-home` on both CI
lanes two sessions ago. The gate is one command precisely because a list run by
hand is run in the order somebody recalls it.

⚠ `check-workflow.sh` is **not** in that gate and is run separately. Two of its
cases run the gate, so listing it there would make the gate re-enter itself,
which is the same contract that keeps `check-twins` out of the runners' pair
list.

⭐ `pwsh` 7.4.6, `shellcheck` 0.10.0 and `shfmt` 3.14.0 were installed at session
start from the commands in `TODO/PROGRESS.md`. Install them before touching a
script; the `chmod +x` on the PowerShell tarball is not optional.

⛔ `check-remote-items` cannot be made to run here and installing `gh` does not
fix it. It is the one observed skip in the table above, and it is why the gate
exits 1 under `--strict` on this host and 0 on the Linux lane. Read that
difference before calling a strict run broken.

⛔ **A gap a runner declares and a check that stopped working are different
facts, and one flag could not tell them apart.** That is what kept `--strict`
off the Windows lane, where six rows are genuinely unavailable, and a check
rewritten to `exit 2` left that lane green. Measured on 2026-09-06.

⛔ **Exporting `CARGO_TARGET_DIR` silently disarmed five mutation provers.** Two
places composed an example's path as `target/debug/examples` while cargo obeys
the variable, so each harness exited 2, which the gate reads as a skip. Fixing
one left the other, which is the door sweep working exactly as intended.

⭐ **Assert a residual instead of writing it down, and it may not survive.**
`CORPUS-04`'s entry was about to record that a cycle is a store the corpus
validator accepts; the assertion showed the validator refusing that store for an
entirely unrelated reason. The residual was real and the reason was wrong, which
is the difference a claim audit exists to find.

⚠ **The retention half of a correction needs two stores, not one.** Reading the
corrected store and finding the original still there proves it was written, not
that it was left alone. Build the store with and without, and compare the earlier
record's bytes between them.

⚠ **A refusal case needs a control that shows the thing can succeed.** `ACQ-05`'s
cache refuses to keep bytes because the register refuses them, and that case
passes equally over a cache that can never store anything. The harness runs the
same scenario with a target permitted and asserts the two runs differ on exactly
one line.

⛔ **Two twins agreeing is not two twins being right.** `FOUND-04` learned to
compare the halves per planted input rather than on a clean tree. `OBS-06` found
the other half of that: a planted input needs a declared expected outcome too. A
new hex allowance written as `{40}` with no trailing anchor blanked the first
forty characters of a longer run and let the remainder fall under the reporting
threshold, so a forty-six digit value went unreported by both halves, which
agreed with each other perfectly.

⚠ **A test's name is a claim and it can be false.** `OBS-06` planted a revert of
the datagram reply path back to an unguarded send, and the test named for that
guarantee still passed: it asserted a loopback echo, which works either way. What
refuses the revert is the source sweep. A mutation pass is what tells a name from
a check, and a survivor is a question about which of the two is wrong.

⚠ **A needle list of constructors does not cover a method.** The lab's door sweep
named `TcpListener::bind`, `UdpSocket::bind` and two `connect`s, and every one of
those makes a socket. A send is a method on a socket that already exists, so the
sweep was blind to a whole category rather than to one entry, and the unguarded
reply path survived until `OBS-06` looked for what the list did not name.

⛔ **`grep -c .` prints 0 and EXITS 1 on an empty file.** A `|| printf 0`
fallback then fires on the one input that matters and the variable becomes two
zeroes on two lines, which the next comparison rejects as a non-number. That
disabled a guard whose whole subject was an empty file, and only comparing the
two halves per planted mutation showed it. Count with `wc -l`.

⭐ **Two implementations agreeing beats one agreeing with itself.** `PUB-03`'s
CBOR encoder is checked against `cbor2` 6.1.4, and not by a round trip: that
reader's own canonical encoding of what it read is byte-identical to ours. A
third-party reader belongs in an entry's driven pass rather than in the gate,
which is where `libtorrent` and `torf` sit too.

⚠ **An assertion can fail for the right reason.** A case asserting a corrected
record's identifier appears nowhere is false by design, because a correction
names what it corrects. Compare identifiers, not text.

⚠ A harness that runs CI's commands must read them **out of the workflow**. A
copy of a build command in a harness proves something about the copy. Ask by job
and step name, and treat a step that has gone as a failure rather than as
nothing to do.

⚠ **A control is worth as much as the plant.** The gate cases here could not use
the exit code as their control, because this host has an observed skip of its
own and the clean tree exits 1. They read the runner's failure count instead and
record which host they ran on; the plants still read the code unpiped.

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
on the fixture data rather than because a guard was missing. A surviving plant is
a question, not a verdict.

⚠ **And count a multi-line literal with something that understands one.**
`grep -F` splits a pattern containing a newline into separate alternatives, so a
unique multi-line literal counts as the sum of its lines and reads as ambiguous.
It fails safe and still leaves those guards unproven.

⛔ **And read a harness exit of 2 as *could not run*, never as *refused*.** A
review pass counted it as a refusal on one of its two paths and reported a guard
proved over a plant that had not compiled. Separate the two on every path.

⭐ **The strongest control available here is a reader this project did not
write.** `sha256sum -c` verifies a release's checksum file, and `libtorrent` and
`torf` read a generated `.torrent`. A run comparing two of its own summaries is
checking the writer against itself.

⭐ **A push path can be driven for real with no network and no credential.** A
bare repository in a scratch directory is a remote as far as git is concerned,
so a publisher's refusals and its read-back are measured rather than reasoned
about.

⛔ **A constant every test reads is a constant no test can check.** Pin a
specification's values to their literals, and build a fixture at a value no
default or bound also has.

⛔ No observer has been driven by a stock `BitTorrent` client and none can be on
a session host: `sh scripts/acquisition/assert-disposable.sh --egress` exits 1
here. `OBS-07` owns the stock-client controls and `CI-03` owns the runner.

⚠ Read each CI run's failing **step** before its conclusion: a failure above
*Rust check* is the runner's network rather than the tree. The workflow sets
`cancel-in-progress`, so a cancelled run is no evidence either way.

⭐ The last session's record is
[`SESSION-2026-09-05-CORPUS.md`](SESSION-2026-09-05-CORPUS.md). It carries what
each review pass found, the two guards it could not refute and what would have to
be true for each to fire, and the four harness defects that would each have
reported a guard proved or broken when it was neither.

**Paste:** Read `docs/AGENTS.md` in full, follow its start protocol, and take
the first unblocked item from `TODO/PROGRESS.md`. Work on `main`. Re-measure the
clone, branch, identity and remote rather than trusting this file.
