# Work rules

## Status and closure

Statuses are `OPEN`, `IN_PROGRESS`, `BLOCKED`, or `DONE`. Only one entry may
be `IN_PROGRESS` unless the work is explicitly split across independent
agents. An entry becomes `DONE` only after its `Prove` commands pass and the
evidence is recorded in that entry in place.

A blocker does not close work. Record what was tried, the exact external fact
that prevents progress, and the event that would unblock it.

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
