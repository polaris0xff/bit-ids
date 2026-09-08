# Client adapter entries

Every adapter resolves the latest stable release at run time, proves at least
two same-version acquisition routes on each supported host family, disables
public peer discovery, drives the isolated lab, and produces two-connector
evidence. Candidate routes remain hypotheses until the entry closes.

## CLIENT-01: qBittorrent capture adapter

Source: operator scope, upstream repository, and August 2026 priority sample
Priority: P1 | Effort: L | Status: OPEN

Problem: The highest-priority observed client lacks a reproducible live
profile across Linux and Windows acquisition routes.

Approach: Automate the Web UI or command controls, compare official releases
with package-manager routes, and capture all core observer surfaces.

Prove: a trusted run publishes agreeing Linux and Windows profiles for the
same stable version with two verified routes per host.

### What was measured on 2026-09-05, and why the entry stays open

⛔ **The acceptance cannot run on a session host, and three routes were tried
before saying so.** It is the same shape as `OBS-01`'s: an acceptance naming a
Windows capture that this repository is not permitted to perform at all.

1. Run it whole. Refused on a session host: `--egress` exits 1 there, so
   installing and driving a client would be the capture that boundary exists to
   refuse. ⚠ **That is a fact about a session host and never was one about
   hosted runners.** A runner is a fresh virtual machine per job and its default
   route can be deleted before the install, which is what `CI-03`'s workflow
   does; the Windows guard pair exists now too.
2. Run the Linux half alone. Refused for the first half of the same reason: a
   capture on one platform is still a capture, and the boundary does not care
   how much of the acceptance it satisfies.
3. Split off the provable prefix and close that. ⭐ **Partly possible, and the
   part that is was done.** Deciding which version to acquire needs no host, so
   it was run. What it cannot do is leave a record: `CORPUS-01` owns the
   append-only store and is open, so a measurement has nowhere durable to go and
   would be a file nobody can cite. The split is therefore not filed as an entry
   yet; it becomes one once there is a store to write into.

⭐ **The resolver met a real target for the first time.**
`sh scripts/acquisition/fetch-releases.sh qbittorrent/qBittorrent <file>`
answered through `https://api.gh.pkgforge.dev/`, which is the route
`docs/AGENTS.md` rule 8 names, and
`cargo run -p bit-ids --example resolve-stable -- qbittorrent release- 3 3 ...`
selected **5.2.3**, published 2026-07-07, over three superseded candidates with
every verdict kept and a digest of the bytes that were read. ⚠ One source is not
two: the resolution is single-sourced and `ACQ-01`'s independence rule is about
acquiring the artifact, which is a separate requirement this did not touch.

⛔ **The listing answered with four releases, and that is a property of the
source rather than of the mirror.** It looked like truncation and it is not:
page two of the same endpoint is empty, so four is the whole answer, while the
`tags` endpoint on the same route answers with at least a hundred, beginning
`release-5.2.3`, `release-5.2.2`, `release-5.2.1`, `release-5.2.0`,
`release-5.1.4`. The project tags far more versions than it publishes as
releases.

⭐ **That matters to every future resolution and it is not a defect in the
resolver.** `resolve` decides from the candidates it is handed, and the caller
chooses the endpoint; a caller that reads only `releases` is selecting from a
different and much smaller population than the project's versions, and a
resolution saying `candidates: 4` is a claim about that endpoint rather than
about the target. It changed nothing here because `5.2.3` leads both
populations, and it would change the answer for a target that stops minting
release objects. ⚠ A second source for a resolution has to be a different
endpoint or a different index, not the same one asked twice, which is the
independence rule `ACQ-01` already states for routes.

Measured from the same listing, the release offers a Linux `AppImage`, a Windows
`x64_setup.exe` and a source `tar.xz`, each with a detached `.asc` signature
beside it. That is two platform routes and a signature disposition to verify, and
`ACQ-05` owns the authenticity evidence.

### The observer a client can actually be pointed at, measured 2026-09-08

⭐ **The lab now hands out a torrent that names its own tracker**, which is what
a stock build needs and what nothing here provided before.
[`client-capture`](../crates/bit-ids-probe/examples/client-capture.rs) runs the
real `HttpTracker` observer, generates the torrent with `announce` set to the
endpoint the operating system just gave it, writes it to a path, and prints the
address, the info hash and the fixture digest before the line a driver waits on.

⛔ **`evidence-bundle` could not be driven by a client and that was structural
rather than an omission.** Its torrent carries a hard-coded
`http://127.0.0.1:6969/announce`, so the only thing that could reach its
observer was something told the endpoint separately - which `curl` is and a
stock build is not. A client reads the address out of the file or it does not
announce at all.

⭐ **A reader this project did not write confirmed the file says what the
observer claims.** `torf` 4.3.1, from the package index into a virtualenv, took
the announce URL out of the generated `.torrent` and it matched both the address
printed by the observer and the port `curl` then announced to; `torf` also
re-derived the info hash `4bc6a5c90be5c4fc6a3c4281f2400af2d2c7700d` from the
metainfo in the written bundle, which the observer had printed independently.
⚠ Parsing a file is not a capture and needs no disposable host.

Driven on 2026-09-08 with `curl/8.5.0` announcing as `driven-by-curl-000001`:
one announce kept, query key order
`info_hash,peer_id,port,uploaded,downloaded,left,compact,event,key`, header
order `Host,User-Agent,Accept`, two segments, both artifacts verifying.

⚠ **The peer surface is dialled rather than offered, and the reason is a
constructor.** `TrackerResponse` is cloned into the responder while the lab is
still being built, and `Lab` binds port zero, so a tracker answer naming this
lab's own peer port would have to predict a port that does not exist yet. The
client's listen port is an argument instead and the lab dials it once an
announce proves the build read the torrent. ⭐ That is also the stronger role:
the side that dials sends its handshake first, so what comes back is the build
answering.

⚠ **What this still does not establish is a client.** No build is installed on
any host this project may capture on, so every run of it so far was driven by
`curl`. The adapter is what changes that.

### What the info hash cost, measured 2026-09-08

⛔ **Writing that measurement into this record turned CI run 67 red on both
lanes**, and the check was right to fire: an info hash is forty hex digits, and
`check-no-secrets --public` hunts long hex because that is the shape of a
credential. ⭐ The rule is narrowed rather than switched off, to the phrase and
both backticks this project spells one with; a bare forty-digit run elsewhere in
a sentence is still a finding. ⚠ A local gate had been green over this file
minutes earlier, because the checks re-run after the edit were the ones the edit
looked like it touched and `--public` is a second invocation of a check whose
first invocation passes.

## CLIENT-02: qBittorrent Enhanced capture adapter

Source: operator scope and upstream Enhanced Edition repository
Priority: P1 | Effort: L | Status: OPEN

Problem: Enhanced Edition may alter observable identity and must not inherit a
qBittorrent profile by name or source similarity.

Approach: Acquire and drive it independently, retaining flavor metadata and
comparing observations only after both profiles exist.

Prove: two-route live runs either publish a distinct corroborated profile or
prove byte-for-byte observable equivalence without copying source-derived data.

## CLIENT-03: uTorrent capture adapter

Source: operator scope and August 2026 priority sample
Priority: P1 | Effort: L | Status: OPEN

Problem: The proprietary Windows client lacks reproducible active evidence.

Approach: Use a disposable Windows runner, official and package-manager
routes, silent automation where supported, and UI automation only as a bounded
fallback.

Prove: two same-version Windows installations emit agreeing connector records
and leave no state or traffic outside the laboratory.

## CLIENT-04: BitComet capture adapter

Source: operator scope and bit-cli historical identity mismatch
Priority: P1 | Effort: L | Status: OPEN

Problem: Historical BitComet-like identity assumptions make guessed mapping
especially unsafe.

Approach: Drive official and package-managed Windows builds through the lab
and retain raw handshake, tracker, and extension behavior.

Prove: a two-route Windows capture passes agreement without consulting a
source-code peer-ID mapping.

## CLIENT-05: aria2 capture adapter and connector

Source: operator scope and requirement for an independent CLI connector
Priority: P1 | Effort: L | Status: OPEN

Problem: aria2 is both a required target and a useful independently controlled
client, but those roles must not create circular corroboration.

Approach: Use JSON-RPC for lifecycle control, capture aria2 as a target with a
separate packet oracle, and use it as a connector only for other targets.

Prove: tests reject aria2 self-corroboration and a live two-route capture
publishes a profile supported by the Rust observer plus packet decoding.

## CLIENT-06: Transmission capture adapter

Source: operator scope and bit-cli source-profile generator study
Priority: P1 | Effort: L | Status: OPEN

Problem: Existing source-derived formulas are useful hypotheses but violate
this corpus's live-only evidence rule.

Approach: Automate transmission-remote, compare official and package routes,
and test all formula expectations solely against emitted traffic.

Prove: a two-route Linux and Windows run derives every published field from
raw observations and labels source expectations only as non-authoritative notes.

## CLIENT-07: Deluge capture adapter

Source: operator scope and upstream project
Priority: P1 | Effort: L | Status: OPEN

Problem: Deluge's daemon and UI separation can change automation and engine
version evidence across packages.

Approach: Drive the daemon through its supported interface, record bundled
engine identity, and compare official packaging with host package routes.

Prove: live Linux and Windows records prove product and embedded-engine
versions plus agreeing observed surfaces.

## CLIENT-08: BitTorrent capture adapter

Source: operator scope and August 2026 priority sample
Priority: P1 | Effort: L | Status: OPEN

Problem: The proprietary Windows product must be distinguished empirically
from uTorrent despite related ownership and code lineage.

Approach: Isolate installations and compare emitted fields only after separate
two-route captures complete.

Prove: the profile contains independent raw evidence and no inherited field
from the uTorrent adapter.

## CLIENT-09: BiglyBT capture adapter

Source: operator scope and upstream project
Priority: P1 | Effort: L | Status: OPEN

Problem: Java runtime and packaging differences can affect observable client
behavior and must be represented, not flattened.

Approach: Record JVM and package metadata, automate the supported interface,
and acquire official and package-manager builds on both host families.

Prove: runs correlate product, JVM, and host facts with agreeing protocol
observations for the same stable release.

## CLIENT-10: Tixati capture adapter

Source: operator scope and August 2026 priority sample
Priority: P1 | Effort: L | Status: OPEN

Problem: The proprietary Linux and Windows client has no open control source
and may require UI-driven setup.

Approach: Prefer documented controls, use bounded UI automation if necessary,
and verify installation and cleanup independently of the UI.

Prove: two-route captures on supported hosts pass connector agreement and
re-run unattended from a clean guest.

## CLIENT-11: KTorrent capture adapter

Source: operator scope and KDE upstream
Priority: P1 | Effort: L | Status: OPEN

Problem: Linux distribution packages may trail or patch the upstream stable
release.

Approach: Compare a verified official build route with a distribution package
and block publication when their upstream build identities diverge.

Prove: two Linux routes resolve to one stable build and produce agreeing live
profiles, or emit a documented blocking divergence.

## CLIENT-12: Free Download Manager capture adapter

Source: operator scope and August 2026 priority sample
Priority: P1 | Effort: L | Status: OPEN

Problem: FDM combines general download behavior with BitTorrent support and
needs a torrent-specific controlled path.

Approach: Exercise only synthetic torrents, compare official and package
routes, and separate HTTP download headers from BitTorrent identity fields.

Prove: Linux and Windows runs produce torrent-specific two-connector evidence
without contacting a public swarm.

## CLIENT-13: Zona capture adapter

Source: operator scope and August 2026 priority sample
Priority: P1 | Effort: L | Status: OPEN

Problem: Zona is proprietary, Windows-oriented, and may not expose unattended
controls or a dependable second package route.

Approach: First measure current availability and terms, then use a disposable
Windows adapter. Keep the entry open if two independent routes cannot be
verified; never substitute an inferred profile.

Prove: a same-version two-route capture passes, or the entry records a precise
blocker and no Zona profile is published.
