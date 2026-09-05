# Current progress

State instant: 2026-09-06
Baseline commit: `ad0b68f` on `main`
Total: 58
Open: 28
In progress: 0
Blocked: 0
Done: 30

## Current state

The whole `SCHEMA-*` group is closed, along with `FOUND-01` through `FOUND-03`,
`ACQ-01` through `ACQ-04`. Foundations are finished, and acquisition has its
record shape, a resolver that chooses the version, a verifier that says what two
routes agreeing is worth, and a boundary that runs before anything is installed.
All four core observer surfaces are done: both trackers, the peer handshake and
BEP 10. ⭐ `OBS-08` and `OBS-09` are closed too, so the observer layer is
complete: there is a torrent to point a client at, and a run's transcript now
becomes the content-addressed evidence a manifest cites. ⭐ `CORPUS-01`
through `CORPUS-03` and `PUB-01` and `PUB-02` are closed as well, so there is
somewhere durable to put the answer, something that checks a whole store rather
than one record, the consumer-facing views over it, a bundle assembled once and
described by itself, and a publisher that appends it and reads the branch back. **A first vertical
capture is possible from here**, on a host the `ACQ-04` guards permit, and what
stands between is a client adapter rather than any missing machinery.

`OBS-01` was the one XL entry and was split, which is what its own approach line
asked for if the acceptance could not stay atomic. It could not: the Prove named
a known client fixture and a Linux-and-Windows comparison, and neither is
available, so the entry could not have closed however well it was implemented.
The supervisor stayed in `OBS-01` and is closed. The synthetic torrent is
`OBS-08`, the durable evidence journal is `OBS-09`, and the cross-platform
comparison is `OBS-10`, which names the three open entries that block it.
Nothing was dropped in the split.

The `bit-ids-lab` crate is that supervisor. Every socket in it is created by one
function, which refuses an address outside loopback before the syscall and reads
the address back from the socket afterwards, because a bind request and a bound
address are different facts. A lab holds a deadline and stops itself, records
every byte each endpoint moved in the order it moved them with the direction it
travelled, and releases every port on shutdown or on drop. It speaks no
protocol: the observers supply a responder per surface, which is what lets one
deadline, one loopback guard and one journal serve every surface rather than
each observer growing its own.

⭐ The lab also generates the torrent a capture hands a client, and generating it
rather than committing it is what makes it citable: the bytes are a function of
the declared spec, so `capture.fixture` in a record can be re-derived and
checked. Two digests of two different things live there and confusing them is
the trap: the info hash is SHA-1 of the encoded info dictionary, which is the
value a client announces and the one algorithm this project does not choose, and
`capture.fixture` is SHA-256 of the whole metainfo file.

⚠ **A generated fixture is only citable while its byte stream holds still.** The
payload comes from `SplitMix64` seeded by the spec, and a drift in that
arithmetic silently invalidates every fixture digest already recorded while
staying reproducible, seed-dependent and prefix-stable, which is everything a
naive test asserts. The acceptance suite compares the stream against the
algorithm restated from its specification and anchored to the published
reference's own first words for seed zero.

⭐ The lab also writes a run out. One artifact per endpoint, each a
`bit-ids/transcript/1` document carrying every segment's bytes, direction,
connection and offset, plus the manifest rows describing them. The digest is of
the file rather than of the buffer and the file is compared against the buffer,
because a writer that reports the digest of what it meant to write cannot detect
a short write and a truncated file digests to a value matching itself.

⛔ **A transcript is never scrubbed and the type has no argument for it.** The
bytes a build put on the wire are the measurement, and a peer ID is exactly the
sort of high-entropy token a scrubber reaches for. Scrubbing belongs to text a
host produced, where every removal is declared with its count so `raw` cannot
quietly mean `edited`, and the scrubber replaces what the caller names rather
than guessing: a capture knows its own hostname and account, and only an IPv4
address has a shape worth matching on.

The `bit-ids-probe` crate is where those responders live, one module per surface.
Both tracker surfaces and the peer wire are done. The HTTP one keeps the exact head bytes and
answers a bencoded response whose shape is read out of the announce, because a
client that receives the wrong shape reports an error and changes what it does
next, and that change would be recorded as identity when it is the observer's
doing. The UDP one holds the BEP 15 exchange: a client connects before it
announces, so an announce carrying a connection id the tracker never issued means
the build reused a stale one, invented one, or skipped the connect, and each is
answered with the protocol's error action and recorded with its reason. The peer-wire one holds both roles and BEP 10. Both roles, because a build can
behave differently as the side that dialled and the side that accepted: driven with one
external peer in both roles at once, the two sides produced different reserved
blocks and different peer IDs. All three frame with the codec's own reader rather
than a second one, keep the bytes of what they refuse, and bound every list they
grow while counting what they stopped keeping.

The lab grew what the peer wire needed. It dials, through the same loopback guard
every bind goes through, and a responder is now told which connection it is
serving: one responder serves every connection an endpoint accepts, so without an
identity a peer observer would send a second handshake down the first connection.
The journal carries the connection too, so a transcript of two concurrent peer
connections can be separated back into them.

⭐ What an observer offers is a condition of the measurement rather than a
setting. A build sends an extended handshake because it was asked for one, and
what it puts in its map may differ with what it was offered, so the offer is
recorded beside the answer and the reserved block is derived from the same value
the extended handshake is.

⛔ No observer has been driven by a stock client. Each was driven by an
independent client written from the specification, which shares this project's
reading of the protocol and is a weaker control than `OBS-07`'s stock clients. It
cannot be closed on a session host: `assert-disposable.sh --egress` refuses one
with a public route, and running a client there would be the capture the boundary
exists to refuse.

`CORPUS-01` is the append-only store, and it is two rules. A record's path is
derived from the identity tuple `RecordId` digests, in full and nothing else,
because that is the only choice under which the path and the identifier cannot
disagree. A published path then never changes and never disappears; a correction
appends a record carrying `supersedes` rather than editing the one it corrects.

⛔ **The published layout was not injective over that tuple and the derivation
is what found it.** `docs/publishing.md` filed a profile with no `package`
segment while the identity tuple carries one, so a `deb` and an `AppImage` of
one version on one platform were two records at one file name. Whether their
capture identifiers also differ is not the question: `capture.id` is only
documented unique per target, version, platform and architecture, so the
collision rested on a uniqueness rule nothing states or checks.

⛔ **A version is a measurement and not an identifier, and `Version` accepts
`../../etc`.** That is measured rather than assumed, and it has to stay that way:
imposing a grammar on a version string would refuse builds that number
themselves some other way. So the store refuses a version that cannot be a path
segment instead, rather than escaping it, because an escape that is not
injective merges two measurements into one directory and an injective one needs
bytes `RelPath` refuses.

⚠ Two more hazards make one file out of two paths on half of the capture matrix,
and they are checked against a whole tree rather than at derivation.
[`../docs/architecture.md`](../docs/architecture.md) section 4 says which and
why.

`CORPUS-02` is the store-level pass. ⛔ **Most of what its Problem names was
already enforced per record**, by `SCHEMA-01`, `SCHEMA-03` and `ACQ-01` through
`ACQ-03`. What nothing could answer was whether a citation resolves to bytes,
and the reason is structural: `bind` compares the two documents, so a run that
agreed with itself about an artifact nobody wrote passed everything this project
had. Only a store turns a citation into bytes.

⚠ Valid and publishable stay separate at store level too. `validate_corpus`
refuses what must hold of any store; `publishable_view` reports which records may
enter a published view, and a store of provisional records is a correct store.
`CORPUS-03` builds the views on that report.

⭐ The acceptance needed a corpus and the schema fixtures are not one: they
declare digests for artifacts nobody wrote, which is the defect `E-CRP-03`
refuses. `examples/build-store.rs` writes one instead, artifacts first, then each
document rewritten to describe the bytes actually put down.

⭐ `scripts/corpus/check-store.sh` and `check-corpus.sh` plant each refusal
against a real filesystem and both are in the `sh` gate. They share
`store-lib.sh` rather than a copy of it. ⚠ Both are named skips on the PowerShell
half: the plants include a symbolic link and a named pipe, neither of which an
unprivileged Windows session can create. The rules themselves are not
platform-specific and the Rust suite exercises every one of them on both CI
lanes.

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
the update procedure. The observers added two workspace members and no new
third-party crate; `OBS-08` added `sha1`, whose lockfile diff is one package
because `sha2` already brought the same RustCrypto tree.

No identity profile has been captured. The only records in the tree are
synthetic: the schema fixtures under
[`../crates/bit-ids/tests/fixtures/`](../crates/bit-ids/tests/fixtures/), which
describe a target that does not exist, and the wire fixtures under
[`../crates/bit-ids-wire/tests/fixtures/`](../crates/bit-ids-wire/tests/fixtures/),
which were written by hand from published BEPs. Neither is evidence about
anything.

`CORPUS-03` is the consumer's side: lookups by target, measured peer prefix,
measured client string, platform, version and capture instant, plus a latest view
per build line. ⛔ **Every row names the record it came from**, so a reader who
doubts one opens the measurement; a derived file that answered a question the
records could not is a file that invented one.

⛔ **The peer-prefix lookup is the opposite of the decoder table the codecs
refuse, not an exception to it.** Its key is the fixed span of a peer ID this
project measured, and it resolves to the record that measured it.

⚠ A latest view needs an ordering and `Version` deliberately has none, so
`ACQ-02`'s scheme comparison became public and both callers share it. The scheme
is supplied on the command line rather than defaulted; moving it into
`catalogue/clients.toml` is worth doing and belongs to `ACQ-02`.

`PUB-01` assembles the bundle. Two documents describe it and they cover
different sets on purpose: the manifest cannot state its own digest, so the
checksums cover the manifest and the manifest covers everything else. ⭐ **A
media type is looked up and never guessed**, and that rule paid on its first
driven run by refusing a real evidence bundle over a file type the table did not
carry.

⭐ The strongest control on that bundle is not this project's code:
`sha256sum -c` reads the checksum file back, so a run that agreed with itself
about what it wrote is still caught.

`PUB-02` appends that bundle to the data branch. The append rule is checked
before the push rather than after, because a branch protection setting refuses a
force and says nothing about a commit that deletes a file. Nothing re-enables
force: no flag, a branch name carrying `+` or `:` is refused before a refspec
exists, and the harness reads the publisher's own source for one. ⛔ **The
publisher has never run against the real remote** and will not until there is a
measured record to publish; its acceptance runs against a bare repository the
harness creates and deletes.

⛔ **Driving it found that the append rule and the derived files collided.** A
second publication changes `MANIFEST.json`, `SHA256SUMS` and the indexes by
design, and treating every published path as immutable made a correct second
publication impossible. `CANONICAL_ROOTS` names the roots the rule is about.

⭐ `CI-01` is closed, and with it the last open `P0`. ⛔ **Its Problem was largely
overtaken before it started**, because the Linux lane already delegates to the
gate and the Rust suite already covers the schema and the fixtures on both lanes.
Its Prove was not. No lane could see a check that had stopped running, and
nothing anywhere established that an injected defect turns the pipeline red.

The first is answered by counting a gap the runner declares apart from a skip it
observed, which is what made `--strict` usable on the lane that needed it most:
that lane now reports zero skipped. The second is
`scripts/ci/check-workflow.sh`, which plants eight classes of defect into a
scratch copy of the working tree and runs the offending step against each. ⛔ It
reads every command out of the workflow by job and step name, so it cannot drift
from CI, and it is kept out of `check-gate.sh` because two of its cases run the
gate.

The publisher has a workflow now, carrying the job-scoped write permission and
the concurrency group `PUB-02` left as residuals. ⛔ **It cannot fire on its
own**, its dry run is the default, and it has never run: its first step wants a
bundle from a capture run and there are no captures.

⭐ `ACQ-05` is closed as well, so an artifact survives a source that moved: the
identity is the digest, a new location is a retrieval against the artifact
already known, and the cache keeps bytes only where the register permits, which
today is nowhere. ⚠ Nothing writes a cache document yet, because the first one
worth writing is a real acquisition's.

⭐ `OBS-06` is closed over local discovery and peer exchange, and the three
heavier surfaces are split out as `OBS-11`. ⛔ **The lab had no egress guard and
the door sweep is what found it**: every socket went through `bind.rs` and every
*send* did not, so a datagram endpoint answered on the address the sender wrote
on the packet. There is one door for outbound datagrams now, `.send_to(` is on
the sweep's needle list, and an adjacent surface is behind a capability that has
to be constructed rather than a flag that defaults to false. ⚠ Nothing proves no
packet left the host; that needs a capture on the interface and `CI-03` owns the
host that could.

⭐ `FOUND-04` is closed, so every catalogue target and every third-party package
has a recorded licence disposition. ⛔ **Six of the nine targets with a GitHub
upstream have no licence a detector can name**, and those rows say `unverified`
rather than carrying an identifier nobody established. Every row refuses
redistribution, which is the policy rather than a consequence of the licences,
and `check-licences` also refuses an installer-shaped file in the tree.

⭐ `PUB-03` is closed as well, so the record set has consumer-facing renderings:
a combined JSON carrying each record's own bytes, one compact document per line,
a tabular view that publishes what it omits, and deterministic CBOR. ⛔ **Which
records they carry is `CORPUS-04`'s answer rather than a second filter**, so a
retracted measurement leaves the table at the moment it leaves the lookups.
⚠ The SQLite rendering is split out as `PUB-05` and is blocked on a dependency
decision the operator owns.

⭐ `CORPUS-04` is closed too, so a correction changes an answer rather than only
being recordable. A superseded record leaves every view and keeps its path and
its bytes, and the derived document carries the chain, so a consumer holding an
identifier from last month can find what answers now. ⚠ Two counts are kept
apart because they mean opposite things: an excluded record was never
publishable, and a superseded one was.

## Work order

1. `CLIENT-01`, `CLIENT-06`, and `CLIENT-05` as the first complete vertical
   captures, on Linux only until `CI-03` provides the Windows guard pair.
   ⛔ **They stay behind the corpus work on a measurement rather than a
   preference:** their acceptance needs a capture, a capture needs a host
   `assert-disposable.sh --egress` does not refuse, and a session host is
   refused. `TODO/clients.md` carries the three routes that were tried on
   2026-09-05. ⭐ Neither the observer layer nor the store blocks them any more;
   `CI-03` and a host are what remain.
2. `OBS-11`, the three adjacent surfaces `OBS-06` split out. ⭐ **The only
   remaining item that needs no capture host**: the containment, the switch and
   the sweep are built, so each of DHT, web seed and MSE is a protocol module
   and its acceptance. `OBS-07` and `OBS-10` are the other two observer entries
   and both need a client build, so they wait on the same host the clients do.
3. `CI-02` through `CI-04`, of which `CI-03` is what a capture host needs.
   `CI-02`'s own acceptance is fixture-driven and could move before one.
4. The remaining client and engine breadth, behind the same capture host.
5. `PUB-04` and `PUB-05`, then the consumer library, public documentation and
   refinements. ⚠ `PUB-04`'s Prove fetches every documented path and nothing has
   ever been published, so its paths do not exist; a scratch bare repository is a
   real remote and is how its shape can be driven before one does. `PUB-05` is
   blocked on the operator decision above.

## Pending operator decisions

⛔ **One, and `PUB-05` carries it in full: how the SQLite rendering gets
written.** `rusqlite` brings a vendored C library and a build script into a
workspace whose lints say `unsafe_code = "forbid"`; writing the file format here
means new unaudited code producing B-tree pages in the component that publishes
evidence. The recommendation is the crate, pinned, with the exception recorded
against that one dependency rather than the workspace lint relaxed. ⚠ Nothing
is blocked behind the answer except that one rendering: `PUB-03` shipped the
other four.

Candidate package routes and proprietary-client availability remain measurements
for their acquisition entries, not bootstrap decisions.

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

⛔ **A prover that could not run reported nothing, and the gate read that as a
skip.** Exporting `CARGO_TARGET_DIR`, which a great many Rust developers do, put
every built example somewhere the harnesses did not look, so all five corpus and
publishing provers exited 2 and the whole tier silently stopped proving
anything. Two places composed that path and fixing one left the other; `CI-01`
carries both. ⚠ The lesson is about the status rather than the path: exit 2 is
the honest answer for a harness that cannot run, and a tier of them answering it
at once still looked like a green gate.

⚠ `check-twins` compares the two halves' answers on the tree it runs against.
A rule that differs only on a defect the tree does not contain is invisible to
it, so a changed pair is compared per planted mutation, not on a clean tree
alone.

⭐ **The same hazard is not confined to the shell twins.** `FOUND-03` planted
nine lossy defects in the Rust codecs and two were missed on the first pass,
each because the corpus lacked the shape that would have failed: no fixture used
a bare newline, and no path reached the bencode encoder at all. A corpus only
tests the defects it contains an example of.

⚠ **A constant every test reads is a constant no test can check.** `OBS-08`
found two of that shape before planting against them: nothing pinned
`PIECE_HASH_LEN`, so narrowing it re-chunked the `pieces` string and the
comparison against it together, and the test spec was built at
`MIN_PIECE_LENGTH`, which made the declared piece length and the floor
indistinguishable. Pin a specification's own values to their literals, and build
a fixture at a value no default or bound also has.

⭐ **Third-party readers are installable here and are a much stronger driven
pass than a decoder written for the purpose.** `libtorrent` 2.1.1.0 and `torf`
4.3.1 install into a virtualenv from the package index and read a `.torrent`
without touching the network. Parsing a file is not a capture and needs no
disposable host; running a client still does.

⭐ **A driven pass gets its strength from the client knowing what it sent.**
`OBS-09`'s reads the bundle back with the same Python client that put the bytes
on the wire, so the transcript can be checked against what actually happened
rather than against what the lab believed happened. A reader that only
re-computes digests is checking the writer against itself.

⚠ **`grep -F` splits a pattern containing a newline into separate
alternatives**, so a unique multi-line literal counts as the sum of its lines and
a plant verifier built on it reports NOT-PLANTED over a plant that would have
applied. Measured on 2026-09-05 while reviewing `CORPUS-01`, where it miscounted
three plants. It fails safe, and it still costs the guards it silently declines
to prove. Count a multi-line literal with something that understands one, or
refuse it outright as `scripts/corpus/check-store.sh` does.

⭐ **The corpus harnesses need `cargo`, `sha256sum` and, for `check-store`,
`mkfifo`, and exit 2 without them.** The Linux CI lane runs the gate with
`--strict`, so such a skip would be a failure there; all three are present on
`ubuntu-24.04`.

⛔ **A sourced shell library shares one namespace with its caller.** A harness
assigned its own `ROWS` for a row count, overwrote the library's accumulator, and
printed ten passes over eight lines. The globals are prefixed now and
`store_report` compares the rows it holds against the count it is about to
print, because a naming convention is a rule nobody checks.

⛔ **A plant that survives is not automatically a gap, and a plant that is
refused is not automatically proof.** Of eight over the index builder, two
survived because the mutation was equivalent on the fixture data and one
"refusal" on an earlier pass was a harness exit of 2. Read what the plant
actually changed before believing either answer.

⛔ **A harness exit of 2 is *could not run*, never *refused*.** A review pass
counted it as a refusal on one of its two paths and reported a guard proved over
a plant that had not compiled. Separate the two statuses on every path, not on
the one where it first bit.

⚠ **`shellcheck` answers differently depending on how the files were grouped on
its command line.** A script that sources another is clean when both are handed
to one invocation and warns when checked alone, because it cannot follow a source
it was not given. CI passes every script at once and a contributor checking one
file does not, so the directives belong in each file rather than in the
invocation.
