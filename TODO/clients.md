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

1. Run it whole. Refused: `sh scripts/acquisition/assert-disposable.sh --egress`
   exits 1 here, so installing and driving a client would be the capture that
   boundary exists to refuse, and `docs/capture-host.md`'s guard pair is
   Linux-only so a Windows capture is not permitted at any host. `CI-03` owns
   the runner and the Windows guards.
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

⚠ **The listing answered with four releases, and that is fewer than the project
has.** Whether the mirror paginates differently or answers a subset was not
established, and it matters: a resolver that sees a truncated listing selects
confidently from what it was shown. Nothing here depends on it yet, because the
newest is the newest either way, but the next entry to use this route measures it
rather than assuming.

Measured from the same listing, the release offers a Linux `AppImage`, a Windows
`x64_setup.exe` and a source `tar.xz`, each with a detached `.asc` signature
beside it. That is two platform routes and a signature disposition to verify, and
`ACQ-05` owns the authenticity evidence.

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
