# Current progress

State instant: 2026-09-04
Baseline commit: `2fb8548` on `main`
Total: 55
Open: 44
In progress: 0
Blocked: 0
Done: 11

## Current state

The whole `SCHEMA-*` group is closed, along with `FOUND-01` through `FOUND-03`,
`ACQ-01` through `ACQ-04`. Foundations are finished, and acquisition has its
record shape, a resolver that chooses the version, a verifier that says what two
routes agreeing is worth, and a boundary that runs before anything is installed.
⚠ No capture is possible yet regardless: there is no observer. `OBS-01` through
`OBS-05` are what is missing, and they are the critical path now.

`OBS-01` was the one XL entry and was split, which is what its own approach line
asked for if the acceptance could not stay atomic. It could not: the Prove named
a known client fixture and a Linux-and-Windows comparison, and neither is
available, so the entry could not have closed however well it was implemented.
The supervisor stays in `OBS-01` and is provable on one Linux host today. The
synthetic torrent is `OBS-08`, the durable evidence journal is `OBS-09`, and the
cross-platform comparison is `OBS-10`, which names the three open entries that
block it. Nothing was dropped in the split.

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

A route now says how it was independent rather than asserting that it was. It
records what resolved it and what delivered it separately, and a record whose
routes share either is refused: the two-route rule was otherwise satisfiable by
asking one index twice under two names. The identity of what each route asked
for is typed to its kind, so a release asset is a tag and a file name and a
source build is a whole commit, and an installed version cites the process
output the build printed rather than being asserted.

Choosing the newest stable release is a decision that fails closed and keeps
its reasoning. Version strings are not comparable in general, so a target
declares how it spells them and a candidate the scheme cannot order blocks the
resolution rather than being skipped, because a skip produces an older version
selected confidently. A candidate published before the winner is settled by that
second signal rather than by a guess. Every candidate keeps its verdict, and the
bytes each source answered with are digested into the document.

Equal version labels are the question, not the answer. Every route already has
to report the version the record declares, so what remained was whether that is
backed by anything. A run observes one installed build, the record now says
which, and the executable digest is per route. Two routes that installed the
same bytes are one build and publish; two that differ with only one of them
observed are unresolved and do not, because nothing put the other bytes on the
wire. Reaching a positive verdict over differing bytes takes a capture through
each route.

A client is installed only on a host a guard has refused to disqualify, and the
guard runs before the install rather than in the record. The manifest already
refused to record a capture on a host somebody keeps; that cannot stop one,
because by the time a manifest exists an untrusted installer has run. Two
independent guards now run first: one refuses a host that already ran a capture,
which is how a survived host produces evidence of itself rather than being
trusted to declare it, and one refuses a host with a route off loopback, read
from the kernel's own table without probing anything.
[`../docs/capture-host.md`](../docs/capture-host.md) carries both runner
contracts. ⛔ The guards are Linux-only, so a Windows capture is not permitted
yet.

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

1. Implement `OBS-01`, the lab supervisor, then `OBS-02` through `OBS-05` on
   top of it. Each parses against the `bit-ids-wire` fixture corpus rather than
   against a live client. This is the critical path: the boundary and the
   acquisition record exist, and what is missing is something to observe with.
   `OBS-08` is needed before the first client capture rather than before the
   observers, because an observer can be driven with fixture bytes and a client
   cannot be made to announce without a torrent.
2. `CLIENT-01`, `CLIENT-06`, and `CLIENT-05` as the first complete vertical
   captures, on Linux only until `CI-03` provides the Windows guard pair.
3. `ACQ-05`, the artifact cache, and `FOUND-04`, the licence register, before
   the first proprietary client is acquired.
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
