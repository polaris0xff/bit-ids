# Work summary

Every count here is derived from the rows in [`INDEX.md`](INDEX.md) and checked
against them by `check-project`. The `prefix` column is what makes that
possible: it says which identifiers the row counts, so the check needs no
mapping of its own and the two twins cannot hold different ones.

| category | prefix | open | in progress | blocked | done | total |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Foundation | `FOUND` | 0 | 0 | 0 | 5 | 5 |
| Schema | `SCHEMA` | 0 | 0 | 0 | 4 | 4 |
| Observer | `OBS` | 1 | 0 | 0 | 10 | 11 |
| Acquisition | `ACQ` | 0 | 0 | 0 | 5 | 5 |
| Clients | `CLIENT` | 13 | 0 | 0 | 1 | 14 |
| Engines | `ENGINE` | 3 | 0 | 0 | 0 | 3 |
| Corpus | `CORPUS` | 0 | 0 | 0 | 4 | 4 |
| Library | `LIB` | 0 | 0 | 0 | 2 | 2 |
| Publishing | `PUB` | 0 | 0 | 0 | 5 | 5 |
| CI | `CI` | 5 | 0 | 0 | 5 | 10 |
| Documentation | `DOC` | 0 | 0 | 0 | 2 | 2 |
| Total | | 22 | 0 | 0 | 43 | 65 |

Effort inventory: 1 S, 13 M, 49 L, 0 XL. The observer-lab entry was the XL one and
was split on 2026-09-04, because its acceptance named a client fixture and a
Windows run that this repository cannot produce yet.
[`observer.md`](observer.md) carries what moved where.
