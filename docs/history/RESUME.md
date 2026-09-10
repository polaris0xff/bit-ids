# Resume

**Task:** Take the work order in [`../../TODO/PROGRESS.md`](../../TODO/PROGRESS.md)
in order, committing and pushing each green unit to `main`.

⛔ **Nothing is blocked.** Every question an earlier session recorded as needing
the operator is answered in that file under *Settled decisions*. Do not re-raise
them, and do not record a new blocker without running the command that would
settle it.

**Tree:** Re-measure it. This file is a claim about a tree that has moved. Check
the branch, the remote, the clone depth, `git status` and `HEAD..origin/main`
before editing anything.

**The container may start on a `claude/*` branch with `user.name` set to an
agent and a shallow clone.** All three were true at the start of the last seven
sessions. Correct them before any edit: the branch to `main` per rule 7, the
identity to the operator's own per rule 11, and the clone with
`git fetch --unshallow`. ⛔ Read the identity out of the history with
`git log --format='%an <%ae>' | sort -u`; never type it into a tracked file,
which is what `check-no-secrets --public` refuses.

**Install the three tools with one command:** `sh scripts/doctor/provision.sh`.
About four seconds on a host with none of them, every download verified against a
pinned digest first. ⛔ **Without `pwsh` the gate does not merely shrink - it goes
RED**: `check-capture` FAILS and `check-twins` skips.

**AND `go` IS A GATE DEPENDENCY NOW.** `CI-10` moved six rules into
[`../../tools/check/`](../../tools/check/), and `check-bitcheck`, `check-cache`
and `check-defaults` each build that binary themselves. Without it those rows are
a SKIP, which `--strict` turns into a red lane. Go was already on both runner
images; this host had 1.24.7 with nothing to install.

---

## Where the work is

**In flight:** `CI-10`, with all three of its named deliverables landed and the
rest of the port open. See below.

⭐ **Measured at the start of the 2026-09-10 session, rather than carried:** the
container started on `claude/ci-10-go-port-x40l0l` with `user.name=Claude` and a
shallow clone - all three, for the eighth session running. Corrected to `main`,
to the operator identity read out of `git log`, and unshallowed. The tree was
clean and level with `origin/main` at `2aa84bf`, and the baseline gate over it was
**35 checks, 34 passed, 0 failed, 1 skipped**, the skip being
`check-remote-items`.

### ⭐ CI IS 2.8x FASTER AND EVERY NUMBER IS FROM A RUNNER

| run | what changed | *Workflow acceptance* |
| --- | --- | ---: |
| 122 | the layer as it stood | **32.6 min** |
| 123 | four twin pairs deleted | **22.2 min** |
| 124 | sharded across four runners | **11.8 min** (longest of 9.6, 11.2, 11.7, 11.8) |

The whole run's wall clock is now that shard: the Linux gate is 4.4 minutes and
the Windows gate 2.5. ⛔ **And the acceptance bound came DOWN to 20** from the 45
a previous session raised it to, which is `CI-10`'s own Prove.

### ⛔ THE MEASUREMENT THAT WAS WRITTEN UP WRONG, AND HOW IT WAS CAUGHT

Mid-session this file's entry said deleting the twin layer bought nothing,
because the local gate went from 119 seconds to 129.7. **That is true of this host
and false of a runner**, where the same change bought 10.4 minutes. This session
host has **four** processors and a hosted `ubuntu-24.04` runner has **two**, so a
gate that runs its checks concurrently is CPU-bound there and not here.
⭐ `docs/methodology/gate.md` already names it - *local is not production* - and
the failure was the SCOPE claimed for a correct measurement, not the measurement.
⛔ Read a wall clock on the machine whose wall clock you are claiming.

### What the checking layer looks like now

**Six rules are one Go binary** in `tools/check/`: `check-changelog`,
`check-control-bytes`, `check-licences`, `check-markers`, `check-one-home`,
`check-placeholders`. Both gate runners invoke it, so those rows are the SAME row
on both lanes rather than an `sh` row here and a hand-written twin there.
⛔ **Twelve files are deleted, not translated.** `check-twins` went from 69
seconds to about 15 and from twelve pairs to **eight**.

⛔ **A PAIR MAY ONLY LEAVE THAT LIST ONE WAY.**
`sh scripts/common/check-bitcheck.sh --compare` runs every case against BOTH
deleted halves and refuses any difference in exit code or in the `--json` line.
It has run at 50 cases with all three implementations agreeing. It takes about
nine minutes, because it starts a PowerShell half once per case; the default mode
is 1.1 seconds and is the permanent gate row.

⚠ **The module has an empty require list and therefore no `go.sum`.** Nothing is
fetched at build time, so a build needs no network and `CI-04`'s dependency
surface does not grow by a language. Keep it that way.

### ⚠ What is left of the port, and why it is not urgent

**Eight pairs remain**, together about **12 seconds** of PowerShell against the
**96** the layer started at. ⛔ So the remaining WALL-CLOCK value is small and the
DRIFT value is unchanged. `check-project` is 997 lines and is its own unit;
`check-docs`, `check-no-secrets`, `check-cache`, `check-catalogue`,
`check-remote-items` and `mine-repo` are the rest.

**Two things bit while porting and will bite again.** A check's own
implementation must be exempt from itself where it spells the patterns it looks
for - all three implementations of `check-placeholders` are. And a HARNESS that
plants a pattern cannot spell it either: `check-bitcheck` assembles its needles
with `printf`, the way the marker harness already did.

**Next, in order:**

0. **`CI-10` continues where it is cheapest, not first.** Its three named
   deliverables are landed and proved on runners. What remains is eight pairs.
1. **`CLIENT-01`, `CLIENT-06`, `CLIENT-05`**, the first complete vertical
   captures.
2. ⛔ **THE ONE THING STILL BETWEEN A CAPTURE AND A RECORD IS A DISPATCH.** Every
   refusal that stopped `capture-client` run 14 is repaired in the tree and NOT
   ONE is proved on a runner: the source route's own resolution, the recorded
   package format, the source commit, and the second connector. ⭐ A dispatch of
   `capture-client` on `aria2-next` is what turns four repairs into a measured
   record, and `assemble-capture` over its artifacts is the acceptance.
   Nothing in this tree can press that button.
3. **`CI-09`**, which sits behind that dispatch.
4. **`CI-07`**, whose class-A backlog keeps shrinking as `CI-10` ports pairs
   rather than writing twins for them.
5. **`CI-08`'s** load-sensitive `check-step-bodies` row.

---

## ⛔ The gate's wall clock is NOT where this project's prose said it was

⚠ **`check-capture-client` is 105 seconds** and `CI-01` recorded 47.9. Nothing
changed its `SECS=5` deadline; it has grown to **111 cases** one at a time while
the number in the record stood still. It and `check-capture` run AFTER the
concurrent batch, alone and on purpose, so the local gate is
`max(batch) + max(those two)` and the second term is four fifths of it.

That is why deleting 53 seconds of twin layer moved the local gate by nothing.
It is unowned by `CI-10` - a harness that grew is not a defect - and it belongs to
whoever next opens `CI-01`.

---

## How this project is checked

⛔ **Run the gate with one command, `sh scripts/common/check-gate.sh`, after the
last edit.** ⭐ It is **36 checks** and about **130 seconds** on this host.

**A count of its rows goes stale.** `sh scripts/common/check-gate.sh --rows`
prints the list, and `check-gate-rows` compares it against the other lane's.

**The gate is not the whole of part (a).** `cargo clippy`, `cargo fmt --check`,
the test suite and `sh scripts/ci/check-workflow.sh` are separate.

⭐ **`check-workflow` SHARDS NOW.** `--shard i/N` selects units by `index mod N`,
`--units` lists the names and runs nothing, and CI runs four shards.
⛔ **The controls are NOT sharded**: a shard carrying only plants goes green over
a tree where every step is broken, so every shard runs all three. That is a floor
on what a shard costs, paid on purpose, and it is why four shards are 2.1x rather
than 4x on this host - 1282 seconds unsharded against a longest shard of 606.

**`sh scripts/ci/check-shards.sh` is what keeps the partition honest** and it
costs seconds. It asserts coverage and disjointness for every N from 1 to 6, that
each shard names every control, that no variable is assigned in one unit and read
in another, and that six malformed selectors are refused rather than clamped.
⛔ **It found three real defects the moment it existed**, all in the sharding: a
`--units` mode that ignored `--shard`, and two variables crossing a unit boundary.
Both of those were loud only because of `set -u`.

⛔ **DO NOT RUN THE GATE WHILE `check-workflow` IS RUNNING**, or `git add` while
it is, which corrupts its plant-and-restore accounting.

⛔ **TWO CHECKS JOINED BY `&&` ARE ONE CHECK.** Run each and read each status.

⛔ **Read exit codes from the process that produced them, unpiped.**

---

## What a review has to know before it starts

These are the defect classes this project has shipped and caught.

⛔ **A guard that sees ONE SPELLING of the thing it forbids reports clean over the
others.** The unit-coupling check matched `NAME=` at the start of a line, so it
missed a `for` variable and an assignment inside a `case` branch - and both were
real couplings in the file it was reading.

⛔ **A one-gated-door defect arrives in the change that fixes something else.**
`check-defaults` gated the Go build in front of one subject and left its sibling
outside; a missing command answers 127 under every environment, so that row would
have reported *the same answer under all 6* over a subject that never started.

⛔ **A guard that nothing can refute is still worth keeping, and saying so is the
work.** Removing the empty-register refusal left every case green, because the
plant is refused by a different rule first. ⚠ That is *a check that passes because
a different code path happens to satisfy it*; the case label now says which rule
refuses it rather than claiming a proof.

⚠ **A plant that does not COMPILE is a third status.** One did here, the harness
answered 2, and it is counted as neither refused nor survived.

⛔ **A step does not end when its command exits.**

⛔ **A shell applies redirections left to right.** ⛔ **`timeout 0` MEANS NO LIMIT.**

⛔ **A PowerShell `[switch]` collides with a local differing only in case.**

**A count in prose is a value in two places with nothing comparing them.**

**An in-place `sed` edits every line that matches, not the one you meant.**

⛔ **A Python text-mode rewrite of a `.ps1` silently converts CRLF to LF**, and
`git diff` shows nothing for it. `git ls-files --eol` is the only thing that does.
It happened this session and was caught that way.

⛔ **An apostrophe inside a single-quoted `awk` program ends the string.** A
comment written inside one turned the rest of a harness into shell that `shfmt`
refused to parse. `docs/conventions/shell.md` section 1 names the class.

**A gate run must leave the working tree as it found it.**

⛔ **A green local gate does not mean a green lane, because a DEFAULT can change
under you**, and **a step's exit status is whatever the block LEFT BEHIND**
unless it is a decision.

⚠ **A surviving plant is a question, not a verdict**, a harness exit of 2 is
*could not run*, and a plant that did not **apply** is a third status.

**A plant whose expected outcome is a PASS proves nothing.** ⚠ Except where the
rule's risk is over-strictness: six of `check-bitcheck`'s cases plant something
that must be ACCEPTED, because that is where a port fails.

⛔ **A rule a document says this repository has is not a rule it has.** Grep for
the check before believing it runs.

**The strongest control available is a reader this project did not write.**

⛔ **A CAPTURE'S WORKDIR IS ITS EVIDENCE BUNDLE**, so rule 12 applies to what an
adapter writes there.

---

## Facts a session must not restate wrongly

⛔ **The publisher cannot run at all.** It downloads an artifact named `bundle`
and nothing in this tree produces that name.

⛔ **Nothing has been published and no measured record exists.** ⭐ Builds HAVE
been measured: Transmission 4.0.5 four times, qBittorrent 4.6.3 once, and
`aria2-next` 2.7.5 on runs 11, 12 and 14. ⚠ Those are evidence bundles and
attestations, not `Profile`s.

⛔ **A capture declaring ONE connector is INVALID, not merely unpublishable.**
`E-CAP-01` fires inside `validate`. ⭐ Since 2026-09-10 the capture path declares
two and refuses to run otherwise.

⛔ **THE PEER ID DIFFERS BETWEEN SURFACES INSIDE ONE RUN.** The tracker announce
and the peer-wire handshake of a single capture carry different twelve-byte tails
after `-qB5230-`, so the tail is per-CONNECTION rather than per-run, and a
`constant` on either field would be false.

⛔ **A stock `aria2-next` announces as qBittorrent 5.2.3.0**, observed on the wire
rather than read from a table.

⛔ **Two lanes of one dispatch are not two routes if one resolution fed both.**

⛔ **A single capture can only state a varying field as `constant` with one
sample**, so `BuildEquivalent` is unreachable for a real client through this path.

⛔ **A hosted Windows runner's fingerprint is not a freshness signal.** The claim
marker is what detects a survived host.

⛔ **The nine commit stamps before 2026-09-06T07:56Z are fabricated**, and so are
the subjects of `62e1a68`, `d64c51c`, `facf9a9`, `e2d1891` and `aba7142` on
2026-09-10. They are not retro-corrected. ⚠ The lesson is narrower than "read the
clock": one read at the start of a session is not a stamp for the commits that
follow it. ⭐ Every stamp in this session's five commits was read immediately
before its commit.

**No repository owner or name is hardcoded anywhere in this tree.**

`check-remote-items` cannot run on a session host and installing `gh` does not
fix it. It is the one observed skip.

Every session record is listed in [`README.md`](README.md), and `check-docs`
refuses one that page does not link.
