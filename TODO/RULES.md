# Work rules

## Status and closure

Statuses are `OPEN`, `IN_PROGRESS`, `BLOCKED`, or `DONE`. Only one entry may
be `IN_PROGRESS` unless the work is explicitly split across independent
agents. An entry becomes `DONE` only after its `Prove` commands pass and the
evidence is recorded in that entry in place.

A blocker does not close work. Record what was tried, the exact external fact
that prevents progress, and the event that would unblock it.

## ⛔ A hang is a dead end, not a subject

⛔ **When something hangs, it is a DEAD END. Take a different route.** Do not
diagnose it, do not instrument it, and above all do not add a bound: record the
hang with its evidence, pick the alternative that does not depend on the hanging
thing, and go.

⚠ **This is a rule because it has been broken expensively.** `capture-client`
runs 3 to 31 spent **thirteen bounds** and more than a dozen dispatches on one
step that could not be ended - by `timeout` inside the script, by `timeout`
around it, by a privileged `timeout`, by a watchdog, by the job's own
`timeout-minutes`, and finally by GitHub's own step supervisor. Every one of them
failed the same way, and a cancelled job uploads nothing, so most of those
dispatches measured nothing at all.

⛔ **Two bounds is the limit.** If a second bound does not end it, the thing is
unboundable from where you are standing and no further bound will help. ⭐ The
question then is never *why does it hang* but **what path avoids it entirely** -
`ACQ-06` is what that question produced here, and it removes the privilege the
hang needed rather than measuring it.

⚠ **A hang that costs a dispatch costs more than it looks.** A run that is
cancelled leaves no log and no artifact, so the next reading is built on the
absence of evidence, which is where wrong causes get recorded as located ones.

⚠ **`Closure evidence` is a dated measurement of the tree at closure, not a
claim about the tree now.** Counts in it go stale the moment the next entry
lands and that is correct: rewriting one to match today would falsify what was
run. A `Prove` is the opposite and must stay runnable against the current tree,
which is why `CI-05`'s check reads `Prove` paragraphs and leaves closure
evidence alone.

## Priority and effort

- `P0`: foundational correctness, evidence integrity, or publication safety.
- `P1`: required coverage or a documented capability.
- `P2`: useful refinement after required coverage is operating.

Effort uses `S` for under a day, `M` for a few days, `L` for roughly a focused
week, and `XL` for a cross-cutting item that should be split before execution
when practical.

## Evidence

Published identity fields must come from active, replayable interaction with
the exact installed build. Source reading, peer-ID tables, search results, and
third-party statistics may create a hypothesis or set priority; they are not
corpus evidence. At least two independent connectors must observe each run,
and overlapping fields must agree before publication.

## Synchronization

A status change updates the entry, [`INDEX.md`](INDEX.md),
[`SUMMARY.md`](SUMMARY.md), and [`PROGRESS.md`](PROGRESS.md) in the same
change. Main documentation is amended in place. Narrative history belongs in
[`../docs/history/`](../docs/history/), never appended to current-state docs.
