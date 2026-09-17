# Session record, 2026-09-17: the last privilege and the last twin

A saved summary of one session's measurements. ⚠ Not a diary and not current
truth: [`../../TODO/PROGRESS.md`](../../TODO/PROGRESS.md) is the state and each
entry carries its own evidence.

## What it set out to do

The work order's item 0 was `ACQ-06`: rootless, portable client installation,
the last open P0, with the whole capture path behind it. That closed. Four more
units followed it, and three of them exist because something was asked that had
never been asked before.

1. ⭐ **No install on the capture path needs a privilege any more**, and the
   dispatch that proved it also refuted the reason it was wanted.
2. ⭐ **A bound does not have to be a wrapper**, which dissolved a recorded
   blocker rather than working around it.
3. ⛔ **The assembler wrote a store `validate_corpus` refuses and said nothing**,
   and asking the question found a second defect in what it writes.
4. ⛔ **The last `common/` twin is deleted**, and the comparison that allowed it
   found three defects in the half that was about to go.

## 1. Rootless installation, and a hang that is not privilege

⛔ **Every install wrote to `/usr/local/bin` under `sudo`.** `ACQ-06` replaced
them with one installer: `scripts/acquisition/install-rootless.sh` derives a
prefix the current user already owns, fetches with a bound, settles the digest
before anything is made executable, and refuses an unwritable prefix by name
rather than escalating. The claim guard's state directory moved with it, so
`install-client`'s marker check is unprivileged too.

⭐ **Driven as uid 1001, not as root.** `useradd -m`, a NOPASSWD line, and the
harness re-run under `su` - because this host is `root` and a hosted runner is
not, a difference that had already hidden two real defects for eight dispatches.
26 cases green as an unprivileged user, and the same harness repeated with `sudo`
absent from `PATH` entirely is byte-identical.

⛔ **AND THE DISPATCH REFUTED THE HOPE.** `capture-client` run 32 is the first
dispatch of the rootless tree: both lanes wedged in *Install the client*, both
jobs were cancelled at forty minutes, and the run uploaded zero artifacts.
Removing the privilege did not end the hang. ⚠ That is recorded and closed:
absolute 16, a hang is a dead end, and thirteen bounds had already failed.
The same run measured the unprivileged claim at 0s and the resolve step - which
fetches through the rootless installer - at 1s, on a real runner.

## 2. A bound that is not a wrapper

`mine-repo` cloned with no bound in either half. The recorded blocker was that
`timeout.exe` on Windows is a PAUSE, so a wrapped `.ps1` would sleep for the
bound and then clone, and the two halves would need two idioms where
`check-twins` compares that pair.

⛔ **That premise was the defect.** `git -c http.lowSpeedLimit -c
http.lowSpeedTime` is git's own limit on a transfer that has stopped moving, and
it is spelled identically on every platform git runs on. Driven against a
listener that accepts a connection and never answers: the shipped settings,
extracted from the two files rather than retyped, abandoned the clone at 60
seconds with exit 128; the same clone with them removed was still waiting at 25.

## 3. The store nobody asked about

`CI-09` carried a residual reading *the store this writes is a store of records
with no runs, which `check-store` accepts and a publication would not*. Half of
that was never checked and it put the gate a whole release away.

⭐ **`validate_corpus` refuses such a store today**, `E-CRP-01` per record - *a
record without its run cannot be replayed*. The gate existed all along. What was
missing was the asking: a green two-lane assembly printed two stars and two
paths, and a reader took that for a store a consumer could open.

⛔ **AND ASKING IT FOUND A REAL DEFECT.** The first version of the question
handed the validator an EMPTY `StoreTree`, which narrowed it without saying so -
`E-CRP-06` is checked over the tree. Reading the tree back off the disk, with
the length and digest of the bytes that arrived rather than the record's claim
about them, surfaced this: each record's evidence list is built over ALL the
lanes, because a field citing the other route's install record is what makes the
pair comparable, and the write loop copied only its OWN lane's four files. Every
record cited four artifacts at paths under its own evidence root that nothing had
ever written to.

⛔ **`E-CRP-03` is exactly that refusal and it cannot fire**, because it is
checked per run manifest and no step of this path writes one. The harness
accepted it too, counting *at least one* copied file where half the files satisfy
that. ⭐ The fix is one loop, and the assembler now names any citation it did not
carry, since no rule can.

## 4. The last twin, and what the comparison found

`check-project` was the biggest pair there was: 997 lines of `sh` against a
hand-written PowerShell twin, twenty-nine refusals over this repository's own
invariants. ⭐ It is one Go check now and both halves are deleted.

⛔ **It left `check-twins`' list the only way a pair may**, and the comparison
earned its keep. 49 planted cases against all three implementations found THREE
disagreements, every one of them the twin's:

| planted | the twin |
| --- | --- |
| a client matrix with no rows | **CRASHED**: `.Count` on `$null` |
| the Total row's open count changed | refused for a second reason as well |
| an index row duplicated | reported **9** failures where the others reported 10 |

1. `@(...) | Sort-Object` wraps the INPUT and leaves the output unwrapped, so an
   empty set arrived as `$null` - turning the refusal *a result too small to be
   real* exists for into a crash. The rule was written because two empty sets
   agree perfectly, and the twin could not reach it.
2. The Total row was compared as a whole LINE against a reconstructed string,
   where the `sh` half compares five counts. One rule with two meanings.
3. Two checks joined by `-or` are one check, which is this repository's own rule
   arriving inside a check.

⭐ All three were repaired in the twin and only then did the run come back clean:
143 cases, 143 passed, agreeing on the exit code and byte for byte on the
`--json` line.

⛔ **And the harness could not print its own failures.** `store_report`'s
self-check counted rows by LINES, and a row may be several - a disagreement is
the label plus the two JSON lines that differ. Three real failures made the list
six lines longer than the count, the self-check fired, and the one run with
something to say returned 1 having printed nothing at all. It counts row STARTS
now, in both halves of `store-lib`.

## What none of this was found by

⚠ **Not by reading.** The privilege question was answered by driving as uid 1001;
the clone bound by a listener that never answers; the corpus gap by handing a
validator the tree instead of nothing; the three twin defects by planting shapes
this tree does not contain. ⛔ `check-twins` compares two halves on the tree it
runs against, and this tree has no empty client matrix, no altered Total row and
no duplicated index row - its own documented blind spot, three more times.

## The defect classes this session added

- ⛔ **A rule whose precondition is absent reads clean, forever.** `E-CRP-03`
  needs a run manifest; with none in the store, four bad citations per record
  were invisible to every gate.
- ⛔ **A validator handed an EMPTY input answers about nothing and looks green.**
- ⛔ **A report that cannot print its own failures is a silent run.**
- ⚠ **`@(...) | Sort-Object` wraps the input and leaves the output unwrapped.**
- ⚠ **A count in prose is a value in two places**, and this session's own first
  draft said *twenty-four rules* where the greppable number is twenty-nine.
