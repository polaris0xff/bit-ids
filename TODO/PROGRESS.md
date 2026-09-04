# Current progress

State instant: 2026-09-04
Baseline commit: `ba07d65` on `main`
Total: 52
Open: 45
In progress: 0
Blocked: 0
Done: 7

## Current state

The whole `SCHEMA-*` group is closed, along with `FOUND-01` through `FOUND-03`.
Foundations are finished; acquisition is next.

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

A lifetime claim is a function of the samples. The manifest records what the
run varied, the classifier says what those runs prove and `unknown` for
everything they do not, and `bind` refuses a field claiming variation the run
could not have produced.

The new `bit-ids-wire` crate carries byte-exact codecs for the three surfaces an
observer will speak, being the HTTP tracker, the UDP tracker and the peer wire,
and the fixture
corpus every observer from `OBS-02` onward parses against. One invariant holds
all of it together: decode then encode reproduces the input byte for byte, which
is the cheapest check that catches every retention rule in
[`../docs/architecture.md`](../docs/architecture.md) section 5 at once. The
codecs observe rather than impose, and none of them maps a peer-ID prefix to a
client name.

The supply chain is pinned at all three layers and each pin has a check behind
it. [`../docs/supply-chain.md`](../docs/supply-chain.md) carries the layers and
the update procedure. `FOUND-03` added a workspace member and no new
third-party crate.

No identity profile has been captured. The only records in the tree are
synthetic: the schema fixtures under
[`../crates/bit-ids/tests/fixtures/`](../crates/bit-ids/tests/fixtures/), which
describe a target that does not exist, and the wire fixtures under
[`../crates/bit-ids-wire/tests/fixtures/`](../crates/bit-ids-wire/tests/fixtures/),
which were written by hand from published BEPs. Neither is evidence about
anything.

## Work order

1. `ACQ-01` through `ACQ-04`; do not install proprietary clients earlier.
2. Split `OBS-01`, then implement `OBS-02` through `OBS-05`. Each parses
   against the `bit-ids-wire` fixture corpus rather than against a live client.
3. `CLIENT-01`, `CLIENT-06`, and `CLIENT-05` as the first complete vertical
   captures.
4. `CORPUS-01` through `CORPUS-03`, then `PUB-01` through `PUB-03`.
5. `CI-01` through `CI-04`, followed by remaining client and engine breadth.
6. `FOUND-04`, the licence and redistribution register, before the first
   proprietary client is acquired.
7. Consumer library, public documentation, and refinements.

## Pending operator decisions

None. Candidate package routes and proprietary-client availability are
measurements for their acquisition entries, not bootstrap decisions.

## Known gaps in the local gate

⭐ **`pwsh`, `shellcheck` and `shfmt` are absent on a fresh container and all
three are worth installing before touching a script.** Without `pwsh` the
PowerShell half of every paired check goes unexercised; without the other two,
the CI lane runs shell checks this host never did. Both gaps turned CI red two
sessions ago, once each, on defects a local run would have caught in seconds.

⚠ The `chmod` is not optional. The PowerShell tarball extracts `pwsh` without
the executable bit on this image, and the failure reads as
`Permission denied` rather than as a missing file.

```sh
curl -fsSL -o /tmp/pwsh.tar.gz https://github.com/PowerShell/PowerShell/releases/download/v7.4.6/powershell-7.4.6-linux-x64.tar.gz
mkdir -p /opt/pwsh && tar -xzf /tmp/pwsh.tar.gz -C /opt/pwsh
chmod +x /opt/pwsh/pwsh && ln -sf /opt/pwsh/pwsh /usr/local/bin/pwsh
curl -fsSL https://github.com/koalaman/shellcheck/releases/download/v0.10.0/shellcheck-v0.10.0.linux.x86_64.tar.xz | tar -xJ -C /tmp
install -m755 /tmp/shellcheck-v0.10.0/shellcheck /usr/local/bin/shellcheck
curl -fsSL -o /usr/local/bin/shfmt https://github.com/mvdan/sh/releases/download/v3.14.0/shfmt_v3.14.0_linux_amd64
chmod +x /usr/local/bin/shfmt
```

With those three present the whole CI pipeline runs locally except
`check-remote-items`.

⛔ **`check-remote-items` cannot be made to run on this host, and installing
`gh` does not fix it.** Measured on 2026-09-04: `gh` 2.63.2 installs from the
upstream release tarball and then reports `The token in GH_TOKEN is invalid`, so
the check exits 2 with `gh is not authenticated` rather than with `gh not
found`. The other GitHub route this harness has is scoped to this repository
alone, so a pin in `actions/checkout` cannot be resolved through it either. A
skip is not a pass; the CI Linux lane is what runs this check.

⚠ `check-twins` compares the two halves' answers on the tree it runs against.
A rule that differs only on a defect the tree does not contain is invisible to
it, so a changed pair is compared per planted mutation, not on a clean tree
alone.

⭐ **The same hazard is not confined to the shell twins.** `FOUND-03` planted
nine lossy defects in the Rust codecs and two were missed on the first pass,
each because the corpus lacked the shape that would have failed: no fixture used
a bare newline, and no path reached the bencode encoder at all. A corpus only
tests the defects it contains an example of.
