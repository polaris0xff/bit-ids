# Current progress

State instant: 2026-09-09
Total: 64
Open: 22
In progress: 0
Blocked: 0
Done: 42

⚠ Those five counts are compared against
[`INDEX.md`](INDEX.md) by `check-project.sh` on every gate, so they cannot go
stale silently. ⛔ Nothing else in this header is checked, which is why no commit
is named here: a hash written into the commit that changes it can only name the
one before, and a session reading it would take this file's own work for
unreflected.

## Current state

⭐ **One identity has now been observed from a running build**, and it is not
in the store: the first client capture ran on 2026-09-08 and left an evidence
bundle and an attestation, not a `Profile`. Nothing has been published. Every
record in the tree is still synthetic and says so: the schema fixtures under
[`../crates/bit-ids/tests/fixtures/`](../crates/bit-ids/tests/fixtures/) describe
a target that does not exist, and the wire fixtures under
[`../crates/bit-ids-wire/tests/fixtures/`](../crates/bit-ids-wire/tests/fixtures/)
were written by hand from published BEPs. Nothing has been published.
⚠ **Three FIXTURE captures have run on hosted runners** and not one of them
measured a build: no client is installed, and each of those attestations says
`kind=fixture`, `measured_build=none`, `stock_client=false`.
⭐ **The CLIENT capture workflow is a different path and it has now measured
builds**: runs 11 and 12 on 2026-09-09 each attest `kind=client`,
`stock_client=true`, `measured_build=2.7.5` for `aria2-next`, with an evidence
bundle that verifies outside the run that wrote it. ⛔ Still not a `Profile`, and
the reason is no longer `E-ACQ-01`: run 14 gained a second lane, and assembling
it found `E-ACQ-07`, `E-CAP-01`, an unrecorded package format and `E-ACQ-06`
instead. The table under *Limits* carries all four.

What exists is every layer a capture passes through, and each one is closed:

| layer | crate or scripts | what it owns |
| --- | --- | --- |
| record shape | `bit-ids` | the published record, the run manifest, agreement, sampling, and one validating read and write path |
| acquisition | `bit-ids`, `scripts/acquisition/` | which version is newest, whether two routes agree, where an artifact came from, and the boundary that runs before an install |
| wire | `bit-ids-wire` | byte-exact codecs for every observed surface and the fixture corpus each is parsed against |
| lab | `bit-ids-lab` | the sockets, the deadline, the ordered byte record, the generated torrent, and the evidence a run leaves |
| observers | `bit-ids-probe` | what each surface answers with, one module per surface |
| store | `bit-ids` | where a record is filed, what a successor tree may do to it, and the views a consumer reads |
| capture | `scripts/capture/`, `.github/workflows/capture.yml` | the host a capture is allowed to run on, the order that keeps it contained, and the evidence a run uploads |
| publishing | `bit-ids`, `scripts/publishing/` | the assembled bundle, the append-only push, the six renderings including the queryable one, and the access contract |
| consuming | `bit-ids` | reading a publication back, verified, with no way to reach a network |
| maintenance | `bit-ids`, `scripts/ci/` | what a new stable release creates, and what it must not create twice |

[`../docs/architecture.md`](../docs/architecture.md) is the technical authority
for all of it. The rules below are the ones a reader gets wrong most often.

### The rules that are load-bearing

⛔ **A published value is observed from a running build.** Source code, client-ID
tables, release notes, UI labels, self-reports and swarm statistics may set
priority and may not populate a field.

⛔ **A path is derived from the record's whole identity tuple, never composed.**
A path built from fewer components files two measurements at one name.
`store::StoreKey` is the one derivation, and `docs/publishing.md` records the two
layouts that were found non-injective by comparing a derivation against a
document rather than by reading either.

⛔ **`VersionScheme::components` is the project's only version ordering and every
caller uses it.** The resolver picks the newest release, `CORPUS-03` picks the
newest record, and `CI-02` asks whether the first is newer than the second.
Sorting version text answers `1.2.9` over `1.2.10`.

⛔ **Valid and publishable are separate gates at every level.** A record carrying
a disagreement validates, because refusing it would lose the evidence of the
disagreement; `publishable` is what refuses it. The same split holds for a store
and for a view.

⛔ **Two documents describe a publication and they cover different sets.**
`MANIFEST.json` describes everything but itself and `SHA256SUMS`; `SHA256SUMS`
covers everything but itself. Exactly one published file is covered by neither,
and a consumer establishes it out of band.

⛔ **Every socket, every outbound datagram and every address handed to a build
goes through one door.** `bind.rs` binds and dials, `bind::send_to` addresses,
and `bind::check_offered` guards an address the lab puts *inside* a message for
the target to dial itself. The third leaves the build's socket, so no guard on
this project's own sockets can see it.

⛔ **A transcript is never scrubbed and the type has no argument for it.** The
bytes a build put on the wire are the measurement. Scrubbing belongs to text a
host produced, and every removal is declared with its count.

⛔ **A capture request's identifier is a digest of its key.** Two runs over the
same facts derive the same identifier, so a tracker keyed on it cannot hold two.

⛔ **Nothing in the `bit-ids` crate can reach a network.** A consumer opens a
publication over bytes it already holds; retrieval is a trait the caller
implements, so the fetching lives in the consumer and the verifying in the
library.

### Limits, stated rather than implied

⭐ **The HTTP tracker observer has now been driven by stock clients.** On
2026-09-08 hosted runners installed Transmission 4.0.5 and qBittorrent 4.6.3
from the Ubuntu package index, cut their own default routes, and handed each
build a torrent naming this project's lab. Transmission announced twice carrying
peer ID `2d5452343035302d756435383564356171646f73`, and qBittorrent once carrying
peer ID `2d7142343633302d596939654d4d7e38664f7866`.
⛔ **That is one observer, one platform, one route and one connector, per
build.** Every other observer is still driven only by an implementation written
from the specification, which shares this project's reading of the protocol, and
`OBS-07` owns the controls that close the rest.
⛔ **And "one connector" is a VALIDITY defect rather than a publication one**,
measured on 2026-09-09: `E-CAP-01` refuses a record declaring one connector as an
*invalid document*. `CI-09` carries the two commands that separate it from the
shape `E-PUB-02` catches.

⛔ **The publisher has never run against this repository's own remote** and must
not until a measured record exists. Its acceptance runs against a bare
repository the harness creates and deletes.

⛔ **No published URL has ever been fetched**, because nothing has been
published. `docs/publishing.md` carries the forms and says they are unexercised.

⚠ **Nothing schedules the staleness monitor.** `CI-02` built the comparison and
its driving surface; no capture request has ever been opened.

⛔ **THE TWO-ROUTE CAPTURE IS NOT TWO ROUTES, MEASURED BY ASSEMBLING IT.**
`capture-client` run 14 on 2026-09-09 acquired `aria2-next` through its
`release` and `source` routes on two hosts. Both report **2.7.5**, both
`acquired=yes`, the installed binaries have different digests, and both bundles
verify with `sha256sum -c` outside the runs that wrote them.
⛔ **And it cannot become a `Profile`.** `assemble-capture` was driven over its
four artifacts and refuses. Four reasons, none of them in an attestation:

| what the artifacts say | what refuses it |
| --- | --- |
| both lanes' `resolution.txt` differ **only in their timestamps** | ⛔ `E-ACQ-07`: two routes sharing a resolver are one route |
| every attestation declares **one** connector | ⛔ `E-CAP-01`, at the **validity** gate |
| nothing recorded how the artifact was **packaged**, and `package` is in `StoreKey` | ⛔ a guessed one files two packagings of a version at one path - ⭐ **repaired**: every adapter declares it per route |
| nothing the capture path writes carries the source route's commit | ⛔ `E-ACQ-06`: a source identity needs a full object name |

⭐ **The last is repaired**: the adapter records `rev-parse HEAD` and
`install-client` refuses a `source` route without a full object name.
⛔ **The other three are prerequisites for a record existing at all**, and
`CI-09` carries each with the command that measured it.
⚠ **`BuildEquivalent` is unreachable for a real client through this path**, and
`CI-09` carries why and what was measured instead.

⭐ **CLIENT CAPTURES HAVE NOW REACHED THE *Capture* STEP AND UPLOADED VERIFIED
BUNDLES.** `capture-client` runs 11 and 12 measured `aria2-next` 2.7.5 on
2026-09-09, in about two minutes each, every step green. Both observed a peer ID
whose first eight bytes are `-qB5230-` - qBittorrent 5.2.3.0's prefix, emitted by
a different product - with different twelve-byte tails, so the prefix is stable
across runs and the tail is per-run. ⛔ Still attestations and bundles rather than
`Profile`s: that target has one acquisition route and `E-ACQ-01` refuses a record
with one. `CLIENT-14` carries it and is closed.

⭐ **The capture workflow has been dispatched twice and run 2 is green on both
platforms.** ⛔ **Run 1 bought a defect no reading had found**: its Windows job
went red on *Restore the route* over a restore that had worked. That step runs
the egress guard inverted - a host that can reach the network again is one the
guard refuses - and the refusal's exit code was left in `$LASTEXITCODE`, which
GitHub's `pwsh` wrapper reads as the step's verdict. Every path through both
restore steps ends in an explicit `exit` now.

⭐ **`Get-NetRoute`'s real output does match the fixtures.** The Windows guard
ran with no `-RouteTable` on a real host, in both runs, and agreed with the
corpus `check-runner.ps1` proves it against.

⛔ **The Windows host fingerprint is not a freshness signal and the workflow no
longer says it is.** Two Windows jobs on two hosts that were both fresh - each
run's claim succeeded - reported the same fingerprint, while the two Linux jobs
reported different ones. ⚠ It is structural: a fingerprint must differ between
hosts and survive a reboot within one, and on a cloned image every input with the
second property is a property of the image. ⭐ The claim marker is what detects a
survived host, by finding its own marker rather than by comparing anything.
`CI-06` carries all of it.

⚠ **What that workflow captures is a fixture, and its attestation says so in
fields.** `kind=fixture`, `measured_build=none`, `stock_client=false`.

⭐ **A second capture workflow now exists that installs a product**, and every
layer beneath it is written and mutation-proved: an observer that hands out a
torrent naming its own tracker, an adapter contract with a file per target, an install
step that runs while the network is up, and a runner that refuses to attest to a
build it did not see announce. ⛔ **Not one of them has installed anything.** A
session host is not disposable, so the whole path was driven here against a stub
adapter and `curl`; a dispatch is what establishes whether a stock build behaves
the way the adapters assume.

⚠ **`mse` and `web_seed` have protocol code and no fixture**, so a fixture on
either is refused with `E-FIX-07`. `local_discovery` and `pex` have codecs and
no fixture of their own, for reasons `docs/architecture.md` section 10 gives.

⭐ **A `release` route has now acquired a build from a published binary, in 1.2
seconds.** `aria2-next` 2.7.4 was resolved from its own releases, fetched,
verified against the vendor's `sha256` sidecar by `sha256sum -c`, installed, and
asked its version - which it answered as the build rather than as a filename.
⚠ On a session host, not a capture host, and the product was then driven over its
own JSON-RPC interface rather than captured. ⛔ Every discovery surface was read
back from the running build and one of them was live by default:
`bt-port-mapping` had it bound to **UDP 1900**, found by reading `/proc` rather
than the help text. `CLIENT-14` carries the measurements.

⛔ **No route has been shown to acquire anything ON A CAPTURE HOST, and two of
them provably did not.** `aria2` ships on `ubuntu-24.04`, so the `package` route there is an
`apt-get install` that prints `already the newest version` and exits 0; every
`release` route in the tree fetches its artifact into the workdir and never makes
it the executable the adapter asks. ⚠ Two such routes satisfy `E-ACQ-07` and
`E-ACQ-08`, agree on a version because it is one binary, and reach `ACQ-03` as
`byte_identical`, which is its strongest verdict. ⭐ `install-client` asks the
host before the route runs now and records `preexisting_version` and `acquired`,
so the fact is in the record; ⛔ **nothing yet refuses a pair on it**, and
`ACQ-03` carries that as a residual.

⭐ **The aria2 hang is bounded and three recorded causes are refuted.** Both
runs 3 and 4 say `0 newly installed`, so no package operation happened,
`needrestart` never ran, and the letter in `NEEDRESTART_MODE` could not have been
the cause; run 5's `release` route touches no package index and hung identically;
and two transmission lanes ran the upload step in one second in the same run as
two aria2 lanes that hung. ⚠ **This paragraph said the hang is in *Upload the
install logs*, and run 7 moved it**: with that step at the end of the job, what
hung was *Install the client*. `TODO/clients.md` carries the per-run table.

⛔ **And the sharpest measurement is a comparison of two runs rather than of two
steps.** Runs 6 and 7 ran the same aria2 install from commits whose only
functional difference is where a later step sits - `git diff` over the two says
so - and that step took **six seconds** in one and had not returned after
**sixteen minutes** in the other. ⭐ A command whose duration depends on which
step follows it is not a command that is slow, so `CI-08`'s instrument asks the
question a step's exit code cannot: a runner ends a step when the command has
gone **and** its output pipe has reached end of file.

## Work order

⛔ **Nothing here is blocked.** Every question an earlier session recorded as
needing the operator is answered under *Settled decisions* below, and the capture
host that nineteen entries were said to wait on was never a blocker. Take these
in order.

⭐ **`CI-06` is closed and the button has been pressed.** Both platforms captured
green on run 2, so nothing below waits on a runner question any more.
⭐ **`PUB-05` is closed too**, so every path `docs/publishing.md` promises is
written and the dependency question under *Settled decisions* is spent.
⭐ **And `LIB-02`**: the bit-cli adapter is a comparison that fails closed, and
every answer it gives about a real client today is *not measured*, because
nothing is. The clone question under *Settled decisions* is spent too.

0. ⭐ **`CLIENT-14` IS CLOSED.** It was first by operator direction on 2026-09-09,
   after ten dispatches produced no aria2 capture: every lane stopped inside
   *Install the client*, four independent bounds were measured not to fire on
   that step, five readings of it were refuted, and no such job ever produced a
   log or an artifact. ⛔ The direction was to change the target rather than to
   keep diagnosing - `AnInsomniacy/aria2-next`, acquired from its own releases and
   driven over RPC - and that target captured on the first dispatch.
   ⭐ **THE CAPTURE RAN AND EVERY STEP PASSED. capture-client run 11 is the
   ELEVENTH dispatch and the FIRST to reach the *Capture* step**, in 2 minutes 1
   second, with an install step of one second. It attests `kind=client`,
   `stock_client=true`, `measured_build=2.7.5`, `acquired=yes`, `egress=closed`,
   and its evidence bundle verifies with `sha256sum -c` outside the run that
   wrote it.
   ⛔ **AND THE IDENTITY IS NOT THE PRODUCT'S OWN**: the measured peer ID is
   `-qB5230-s2QbzYjt(LOQ`, whose first eight bytes are qBittorrent 5.2.3.0's
   prefix. A stock `aria2-next` announces as qBittorrent, observed on the wire
   rather than read from a table.
   ⛔ **The second route is a measured absence**: no package index carries this
   fork, so the target has ONE route and `E-ACQ-01` still refuses a record with
   one. ⚠ So run 11 produced an attestation and an evidence bundle, not a
   `Profile`. `CLIENT-05` stays open on the hang, which run 11 does not diagnose:
   a different target installing cleanly is not an explanation of why aria2 does
   not.
1. **`CLIENT-01`, `CLIENT-06`, `CLIENT-05`**, the first complete vertical
   captures. ⭐ Every layer below the product is written and proved, and
   `capture-client.yml` is the workflow that runs them. ⚠ What remains is a
   dispatch and what it teaches: no adapter has ever installed a build, and the
   Windows half of each Prove is untouched because the adapters are `sh`.
   `TODO/clients.md` carries the routes and what each adapter assumes.
2. ⛔ **`OBS-07`, AND IT MOVED UP BECAUSE IT IS A VALIDITY REQUIREMENT.** This
   order used to place it third, after `CI-09`, on the reading that a
   single-connector record *validates* and is merely held back from publication
   by `E-PUB-02`. ⚠ **Measured on 2026-09-09 by stripping the golden fixture two
   ways and reading each exit code unpiped**, that is true of a record declaring
   two connectors where only one observed each field, and false of one declaring
   one connector: `E-CAP-01` refuses it as an **invalid document**. Every capture
   this project has run declares one. So a second connector is a prerequisite for
   a record existing, exactly as a second route is.
3. **A second, independent resolution for the `source` route.** ⛔ Run 14's two
   lanes read one listing, which `E-ACQ-07` calls one route.
   `capture-client.yml` argues for that in a comment - one resolution keeps the
   versions equal - and ⭐ **absolute 4 already answers the worry**: version
   equality is checked *after installation*, not trusted beforehand. Two
   independent resolutions landing on two versions is a vendor that moved
   mid-capture, and catching it is the correct outcome rather than something to
   design around.
4. **`CI-09`**, the capture-to-publisher path, which now sits behind those.
   ⭐ `assemble-capture` and `check-assemble` are written and the refusals above
   are its measurement. ⛔ **The v7/v8 question still cannot be reached**, because
   the publisher downloads `bundle` and nothing in the tree produces that name,
   and a capture bundle is not a publication bundle in any case: what sits
   between them is `assemble-release`, which reads a store of records, and no
   record exists. The gap is declared in the workflow and enforced by
   `check-project`.
5. **`OBS-10`**, which needs a second platform, so it follows the captures.
6. **`CI-07` and `CI-08`**, which harden the gate rather than extend it: the
   declared PowerShell rows, and the host defaults the scripts inherit rather
   than state. ⭐ **`FOUND-05` is closed**: `sh scripts/doctor/provision.sh`
   installs the three tools a session used to install by hand, verifying each
   download against a pinned digest first.
   ⭐ **`CI-08` has written the harness `CI-06` needed**: a capture workflow's
   step bodies now run as a gate row, under GitHub's own wrapper form, and the
   Windows restore block is refused in the shape that failed capture run 1.
   ⛔ What is left of that entry is the sweep it was opened for. ⛔ And a second
   question: CI pins `shfmt` and takes `shellcheck` and `pwsh` from the runner
   image, so a session host now runs a MORE pinned set of tools than the lane it
   is meant to match.
7. **`CI-04`**, provenance and supply-chain hardening, once a release exists to
   bind attestations to.
8. The remaining client and engine breadth, then refinements.

## Settled decisions

⭐ **All four were settled by the operator on 2026-09-08 and none blocks
anything.** They are recorded here so no session re-raises them.

| question | answer |
| --- | --- |
| how the SQLite rendering gets written | ⭐ **Spent.** `rusqlite` 0.37.0 with `bundled` and `serialize`, checked against python's own SQLite on every gate. `PUB-05` closed on it, and nothing was relaxed: `unsafe_code = "forbid"` binds this workspace's crates and a dependency compiles under its own. |
| how `LIB-02` reaches bit-cli's tests | ⭐ **Spent.** The clone works with no credential and no grant, measured; `LIB-02` closed on it and wrote nothing there. ⚠ Its suite was not run: that tree vendors and patches four HTTP crates, so a build there says something about it rather than about the adapter, which touches none of its code. |
| whether Windows captures are permitted | yes. The guard pair exists and is mutation-proven; a hosted `windows-latest` runner is a fresh virtual machine per job, and its default routes are removed before the capture. `CI-03`. |
| what happens to a first measured record | it publishes automatically once the capture is green. No manual gate. |

⛔ **A capture host was never the blocker it was recorded as, and that error
stood for several sessions.** A hosted runner is a fresh virtual machine per job,
so `--claim` passes; `--egress` reads `/proc/net/route`, sends no packet, and
refuses only because a default route exists. Deleting the default route satisfies
it. Measured on 2026-09-08 with the guard's own route-table argument: exit 0 over
a table with the default route stripped, exit 1 over the same table with it.
⚠ The guard was testable in one command the whole time and no session ran it.

## Known gaps in the local gate

⛔ **A green subset of the gate is not a green gate, and `--public` is the row
that proves it.** `check-no-secrets` runs twice in the gate and only the second
invocation carries the public rules, so re-running "the checks this edit
touched" after writing a record passed while the lane went red on a forty-digit
info hash. ⚠ Run `sh scripts/common/check-gate.sh`, which is the list; a subset
chosen by hand is not the same gate twice.

⭐ **`pwsh`, `shellcheck` and `shfmt` are absent on a fresh container, and one
command installs all three:**

```sh
sh scripts/doctor/provision.sh
```

⛔ **Run it before touching a script.** Without `pwsh` the PowerShell half of
every paired check goes unexercised and `check-capture` **fails** rather than
skipping; without the other two, the CI lane runs shell checks this host never
did. ⚠ Measured on 2026-09-08 by taking all three away: a `--strict` gate
reported `FAIL check-capture` and `SKIP check-twins`, and with them back the only
row left is `check-remote-items`.

⚠ **The commands used to live here as prose and no longer do**, which is what
`FOUND-05` was for: every download is verified against a pinned digest before it
is executed, the `chmod` the PowerShell tarball needs is in the script rather
than in a sentence somebody re-types, and `--check` reports what a host has
without installing anything.

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
