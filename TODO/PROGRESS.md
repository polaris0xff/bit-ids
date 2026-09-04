# Current progress

State instant: 2026-09-04
Baseline commit: `c107f67` on `main`
Total: 52
Open: 47
In progress: 0
Blocked: 0
Done: 5

## Current state

`SCHEMA-01`, `FOUND-02`, `SCHEMA-02` and `SCHEMA-03` are closed.

The `bit-ids` crate carries the published record shape, the six field states,
the derived record identifier, the canonical value forms and the publication
invariants, with one validating read path and one validating write path.
[`../docs/architecture.md`](../docs/architecture.md) section 4 is the
reference.

The run manifest carries the rest of a capture: host, isolation, both clocks,
every tool at the version that ran, the routes the build came through, the
phases of the state machine the run walked, the content-addressed evidence and
what was scrubbed from it. `bind` compares every value the manifest and the
profile share, so the deliberate overlap between the two documents cannot
drift.

Corroboration keeps what each connector saw rather than a verdict, and a
connector that cannot see a surface says so, so a field only one observer could
reach is never called agreement. Validity and publishability are separate
gates: a record carrying a disagreement reads and validates, which is what
keeps the evidence of one, and `publishable` refuses it.

The supply chain is pinned at all three layers and each pin has a check behind
it. [`../docs/supply-chain.md`](../docs/supply-chain.md) carries the layers and
the update procedure.

No identity profile has been captured. The only records in the tree are the
synthetic schema fixtures under
[`../crates/bit-ids/tests/fixtures/`](../crates/bit-ids/tests/fixtures/), which
describe a target that does not exist and are not evidence about anything.

Two checks were found to be weaker than they read. `check-docs.ps1` resolved a
link going up more than two levels to the wrong path, which turned CI red on
the first push and had been latent since the bootstrap. `check-changelog` was
parsing no entries at all, because the file wrote entries at the heading level
the parser reads as a section, so its four rules were satisfied by having
nothing to check. Both are fixed and both now refuse the shape that shipped.

## Work order

1. `SCHEMA-04`, then `FOUND-03`. `SCHEMA-04` closes the schema group and the
   record already carries sample counts for it to constrain; `FOUND-03` gives
   every later observer the byte-exact fixtures it parses against.
2. `ACQ-01` through `ACQ-04`; do not install proprietary clients earlier.
3. Split `OBS-01`, then implement `OBS-02` through `OBS-05`.
4. `CLIENT-01`, `CLIENT-06`, and `CLIENT-05` as the first complete vertical
   captures.
5. `CORPUS-01` through `CORPUS-03`, then `PUB-01` through `PUB-03`.
6. `CI-01` through `CI-04`, followed by remaining client and engine breadth.
7. Consumer library, public documentation, and refinements.

## Pending operator decisions

None. Candidate package routes and proprietary-client availability are
measurements for their acquisition entries, not bootstrap decisions.

## Known gaps in the local gate

`check-remote-items` needs `gh` and the network and skips without them. A skip
is not a pass.

⭐ **`pwsh`, `shellcheck` and `shfmt` are absent on a fresh container and all
three are worth installing before touching a script.** Without `pwsh` the
PowerShell half of every paired check goes unexercised; without the other two,
the CI lane runs shell checks this host never did. Both gaps turned CI red in
this session, once each, on defects a local run would have caught in seconds.

```sh
curl -fsSL -o /tmp/pwsh.tar.gz https://github.com/PowerShell/PowerShell/releases/download/v7.4.6/powershell-7.4.6-linux-x64.tar.gz
mkdir -p /opt/pwsh && tar -xzf /tmp/pwsh.tar.gz -C /opt/pwsh
ln -sf /opt/pwsh/pwsh /usr/local/bin/pwsh
curl -fsSL https://github.com/koalaman/shellcheck/releases/download/v0.10.0/shellcheck-v0.10.0.linux.x86_64.tar.xz | tar -xJ -C /tmp
install -m755 /tmp/shellcheck-v0.10.0/shellcheck /usr/local/bin/shellcheck
curl -fsSL -o /usr/local/bin/shfmt https://github.com/mvdan/sh/releases/download/v3.14.0/shfmt_v3.14.0_linux_amd64
chmod +x /usr/local/bin/shfmt
```

With those three present the whole CI pipeline runs locally except
`check-remote-items`, which needs `gh`.

⚠ `check-twins` compares the two halves' answers on the tree it runs against.
A rule that differs only on a defect the tree does not contain is invisible to
it, so a changed pair is compared per planted mutation, not on a clean tree
alone.
