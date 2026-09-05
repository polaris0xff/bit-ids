# Session record: 2026-09-05, the observers

The session summary printed to the operator, saved so it survives the chat. It
carries no work order; [`../../TODO/PROGRESS.md`](../../TODO/PROGRESS.md) has
that and is where it is correct.

Baseline `2fb8548`, work range `2fb8548..39b6fa7`, then the checkpoint commit
that follows it.

| row | measured |
| --- | --- |
| Commits | 6 closing work over `2fb8548..39b6fa7`, all pushed and read back from `origin/main`, then one checkpoint commit |
| Work | 6 entries completed, 1 checkpointed in flight, 0 failed. 1 new entry filed |
| Changes | 41 files over `2fb8548..5887270`, +8209 / -112 |
| Size | 39,998 tracked lines over 169 files at `5887270`, from 31,901 over 147 at `2fb8548`. Delta +8,097. ⚠ Measured from the committed trees, not the working copy: a size read from the working copy changes when this file is written |
| Checks | gate 12 checks, 11 passed, 0 failed, 1 skipped, both halves |
| Suite | 25 binaries, 250 passed, 0 failed, plus 2 doctests |
| Mutation | 93 defects planted across 7 guards, 1 not refused and named below |
| CI | the six work commits green on both the Linux and Windows lanes, read back from the API; the checkpoint commit's run confirmed separately |
| Cost | no money. Network: one crate added from the registry, plus three tool downloads |
| Health | tree clean, level with `origin/main`, nothing deployed |

## What closed

| entry | what it establishes |
| --- | --- |
| the `OBS-01` split | three entries out of one XL, because its acceptance named a client and a Windows run that do not exist |
| `OBS-01` | the lab: every socket bound on loopback by one function, a deadline, an ordered byte record, ports released |
| `OBS-02` | the HTTP tracker observer, keeping the order, the duplicates and the encoding an HTTP server destroys |
| `OBS-03` | the UDP tracker observer, where the stateful BEP 15 exchange is itself the measurement |
| `OBS-04` | the peer handshake in both roles, because a build can differ by role |
| `OBS-05` | BEP 10, where what the observer offers is a condition of the run |

`OBS-08` is in flight: the torrent generator and its unit tests are in the tree
and its acceptance suite, mutation pass and driven pass are not. `CI-05` was
filed for the acceptance-command class below.

## Defects found, by the pass that found them

⭐ **The two that cost the most were both in the probes rather than the code.**

**Guard mutation.** 93 planted across seven guards. The passes found: a
responder offered its buffer once per read, so a client sending two units in one
write and waiting for two answers would have waited forever; a send-once flag
that could be cleared with nothing noticing, twice, once for the handshake and
once for the extended handshake; a datagram buffer shrinkable below any fixture;
a head framer shortenable by a byte because no head in the corpus ended in bare
newlines; and an acceptance command that skipped the library's own tests.

⛔ **Three times a mutation script reported a guard failing to fire over source
it had not mutated.** Twice because the script did not check its edits applied,
and the third time in the one script that never got the guard after the first
two were fixed. Two other plants silently stopped matching when the code moved
underneath them. Every plant now compares the file's checksum either side and
uses literal replacement.

**Door sweep.** `check-project` compared one row of `TODO/SUMMARY.md` against
the index and eleven against nothing. The observers' records were unbounded, and
the UDP refusal list stayed unbounded after the datagram list was capped. A
second `Content-Length` header took the first value. A connection id was read by
the codec in one place and by a raw byte slice in another. A dial on a stopped
lab wrote bytes nothing would answer. Six response encoders were public with one
internal caller each and no external one.

**Claim audit.** `Datagram::connection_id` reported BEP 15's magic value as a
connection id for a connect request. Three sentences had nothing behind them:
what every current client accepts, what a client does with an unsorted
dictionary, and a component row saying the probe crate writes evidence. The
`OBS-08` entry named a record field that does not exist.

**What was measured but never verified.** The eight older test files' function
count was taken by grep and is now confirmed by the runner's own counts. That
`check-remote-items` runs in CI was documentation until this session: the Linux
lane runs the gate with `--strict`, which makes a skip a failure, and the six
work commits went green on both lanes.

**Writing the tests.** A test wrote a fixed byte budget and asserted a socket
close had arrived by then, which is a scheduling outcome it does not control. It
passed alone and failed twice in three loaded runs.

## The one guard that is not refuted

`Stream::rebuilds_from_raw` returning `true` unconditionally is not caught. Its
`Ok` arm compares an encode against the bytes, which holds for every transcript
a correct codec can produce, so nothing this crate can send makes it false. It
is a codec-regression detector: planting a lossy `Message::encode` in
`bit-ids-wire` **is** refused, so what would have to be true for the guard to
fire is that the codec became lossy and `FOUND-03`'s round-trip invariant
stopped catching it first.

## What this session could not do

- ⛔ No observer has been driven by a stock `BitTorrent` client. Each was driven
  by an independent client written from the specification, in Python or by
  `curl`, which is a weaker control: it shares this project's reading of the
  protocol. It cannot be closed on a session host, because
  `assert-disposable.sh --egress` exits 1 here, so running a client would be the
  capture that boundary exists to refuse. `OBS-07` owns the stock controls and
  `CI-03` owns the runner.
- `check-remote-items` still cannot run here, for the reason the previous
  session measured.
- No capture was taken and none could be: `OBS-08` has no torrent finished and
  `OBS-09` writes no evidence yet.
