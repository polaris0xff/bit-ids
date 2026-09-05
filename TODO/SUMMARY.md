# Work summary

Every count here is derived from the rows in [`INDEX.md`](INDEX.md) and checked
against them by `check-project`. The `prefix` column is what makes that
possible: it says which identifiers the row counts, so the check needs no
mapping of its own and the two twins cannot hold different ones.

| category | prefix | open | in progress | blocked | done | total |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Foundation | `FOUND` | 0 | 0 | 0 | 4 | 4 |
| Schema | `SCHEMA` | 0 | 0 | 0 | 4 | 4 |
| Observer | `OBS` | 3 | 0 | 0 | 8 | 11 |
| Acquisition | `ACQ` | 0 | 0 | 0 | 5 | 5 |
| Clients | `CLIENT` | 13 | 0 | 0 | 0 | 13 |
| Engines | `ENGINE` | 3 | 0 | 0 | 0 | 3 |
| Corpus | `CORPUS` | 0 | 0 | 0 | 4 | 4 |
| Library | `LIB` | 2 | 0 | 0 | 0 | 2 |
| Publishing | `PUB` | 2 | 0 | 0 | 3 | 5 |
| CI | `CI` | 3 | 0 | 0 | 2 | 5 |
| Documentation | `DOC` | 2 | 0 | 0 | 0 | 2 |
| Total | | 28 | 0 | 0 | 30 | 58 |

Effort inventory: 1 S, 13 M, 44 L, 0 XL. The observer-lab entry was the XL one and
was split on 2026-09-04, because its acceptance named a client fixture and a
Windows run that this repository cannot produce yet.
[`observer.md`](observer.md) carries what moved where.
