# Current progress

State instant: 2026-09-08
Total: 63
Open: 26
In progress: 0
Blocked: 0
Done: 37

⚠ Those five counts are compared against
[`INDEX.md`](INDEX.md) by `check-project.sh` on every gate, so they cannot go
stale silently. ⛔ Nothing else in this header is checked, which is why no commit
is named here: a hash written into the commit that changes it can only name the
one before, and a session reading it would take this file's own work for
unreflected.

## Current state

No identity has been measured. Every record in the tree is synthetic and says
so: the schema fixtures under
[`../crates/bit-ids/tests/fixtures/`](../crates/bit-ids/tests/fixtures/) describe
a target that does not exist, and the wire fixtures under
[`../crates/bit-ids-wire/tests/fixtures/`](../crates/bit-ids-wire/tests/fixtures/)
were written by hand from published BEPs. Nothing has been published and no
capture has been taken.

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
| publishing | `bit-ids`, `scripts/publishing/` | the assembled bundle, the append-only push, the renderings, and the access contract |
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

⛔ **No observer has been driven by a stock client.** Each was driven by an
independent implementation written from the specification, which shares this
project's reading of the protocol. `OBS-07` owns the stock-client controls.

⛔ **The publisher has never run against this repository's own remote** and must
not until a measured record exists. Its acceptance runs against a bare
repository the harness creates and deletes.

⛔ **No published URL has ever been fetched**, because nothing has been
published. `docs/publishing.md` carries the forms and says they are unexercised.

⚠ **Nothing schedules the staleness monitor.** `CI-02` built the comparison and
its driving surface; no capture request has ever been opened.

⛔ **The capture workflow has never been dispatched.** `CI-03` built it and
everything about it that a reader can check is checked: the absent
`pull_request` trigger, the step order, the scripts it names, and the two
runners it calls, which are driven for real by `check-capture` on every gate.
⚠ What only a dispatch establishes is on the record: that `Get-NetRoute`'s real
output matches the fixtures `check-runner.ps1` proves the guard against, that
deleting the default route on a hosted runner and putting it back works, and
that an artifact upload survives the round trip.

⚠ **What that workflow captures is a fixture, and its attestation says so in
fields.** `kind=fixture`, `measured_build=none`, `stock_client=false`. Nothing
installs a client yet; `CLIENT-01` is what points a real build at the same lab.

⚠ **`mse` and `web_seed` have protocol code and no fixture**, so a fixture on
either is refused with `E-FIX-07`. `local_discovery` and `pex` have codecs and
no fixture of their own, for reasons `docs/architecture.md` section 10 gives.

## Work order

⛔ **Nothing here is blocked.** Every question an earlier session recorded as
needing the operator is answered under *Settled decisions* below, and the capture
host that nineteen entries were said to wait on was never a blocker. Take these
in order.

1. **`CI-06`**, the first dispatched capture run. `CI-03` built the workflow and
   nothing has ever pressed the button, so three facts stand unmeasured: whether
   `Get-NetRoute`'s real output matches the fixtures the Windows guard is proved
   against, whether a hosted runner's default route can be deleted and put back,
   and whether the evidence bundle survives the upload. It is first because
   every client capture depends on the answer.
2. **`PUB-05`**, the SQLite rendering, which needs no capture and no runner. The
   dependency question is settled below.
3. **`LIB-02`**, the bit-cli adapter, which needs neither. Clone the public
   repository into a scratch directory and run its suite there.
4. **`CLIENT-01`, `CLIENT-06`, `CLIENT-05`**, the first complete vertical
   captures, once `CI-06` has run. `TODO/clients.md` carries the acquisition
   routes.
5. **`CI-09`**, the capture-to-publisher path, once a real capture artifact
   exists to hand the publisher's dry run.
6. **`OBS-07` and `OBS-10`**, which need a stock client build and a second
   platform, so they follow the captures.
7. **`CI-07`, `CI-08` and `FOUND-05`**, which harden the gate rather than extend
   it: the thirteen declared PowerShell rows, the host defaults the scripts
   inherit rather than state, and the three tools a session installs by hand.
   Each is independent of every capture.
8. **`CI-04`**, provenance and supply-chain hardening, once a release exists to
   bind attestations to.
9. The remaining client and engine breadth, then refinements.

## Settled decisions

⭐ **All four were settled by the operator on 2026-09-08 and none blocks
anything.** They are recorded here so no session re-raises them.

| question | answer |
| --- | --- |
| how the SQLite rendering gets written | `rusqlite` with the bundled feature, pinned, with the `unsafe` exception recorded against that one dependency rather than the workspace lint relaxed. Check the output opens in a reader this project did not write. `PUB-05`. |
| how `LIB-02` reaches bit-cli's tests | clone the public `Azathothas/bit-cli` into a scratch directory and run its suite there. ⛔ Nothing is written to it; rule 10 holds. |
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
