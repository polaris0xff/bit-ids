# Current progress

State instant: 2026-09-04
Baseline commit: `528411a`, the `main` bootstrap commit
Total: 52
Open: 50
In progress: 0
Blocked: 0
Done: 2

## Current state

`SCHEMA-01` is closed. The `bit-ids` crate now carries the published record
shape, the six field states, the derived record identifier, the canonical value
forms and the publication invariants, with one read path and one write path
through JSON and every diagnostic code planted against.
[`../docs/architecture.md`](../docs/architecture.md) section 4 is the reference
for it.

No identity profile has been captured. The only records in the tree are the
synthetic schema fixtures under
[`../crates/bit-ids/tests/fixtures/`](../crates/bit-ids/tests/fixtures/), which
describe a target that does not exist and are not evidence about anything.

`SCHEMA-01` added the first three third-party crates: `serde`, `serde_json` and
`sha2`. `Cargo.lock` is committed and CI builds `--locked`, but nothing yet
refuses a Git dependency or an unreviewed pin.

## Work order

1. `FOUND-02`, moved ahead of the rest of the schema work. Dependencies now
   exist, so the provenance guard is worth more before more entries add crates
   than after. It was second in the foundation group already and nothing
   depends on it waiting.
2. `SCHEMA-02` and `SCHEMA-03`, which extend the `capture` and `corroboration`
   sections `SCHEMA-01` left thin.
3. `FOUND-03` and `SCHEMA-04` against deterministic fixtures.
4. `ACQ-01` through `ACQ-04`; do not install proprietary clients earlier.
5. Split `OBS-01`, then implement `OBS-02` through `OBS-05`.
6. `CLIENT-01`, `CLIENT-06`, and `CLIENT-05` as the first complete vertical
   captures.
7. `CORPUS-01` through `CORPUS-03`, then `PUB-01` through `PUB-03`.
8. `CI-01` through `CI-04`, followed by remaining client and engine breadth.
9. Consumer library, public documentation, and refinements.

## Pending operator decisions

None. Candidate package routes and proprietary-client availability are
measurements for their acquisition entries, not bootstrap decisions.

## Known gaps in the local gate

`check-remote-items` and `check-twins` skip on a host without `gh` or `pwsh`,
which is the case on the container these sessions run in. The PowerShell half
of every paired check is therefore exercised only by the CI Windows lane. A
skip is not a pass; treat a change to a `.ps1` half as unverified locally.
