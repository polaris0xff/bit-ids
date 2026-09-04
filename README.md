# bit-ids

A dated, provenance-carrying catalogue of identities observed from real
BitTorrent clients and BitTorrent libraries on live, isolated protocol
connections.

The project records what an exact stable build puts on the wire: peer IDs,
tracker requests, handshake capability bits, extension handshakes, early
message order, and the surrounding acquisition and capture evidence. It is
designed to run primarily in GitHub Actions and to accrete new records as
stable releases appear.

The repository is currently a foundation and work plan. It contains no
published client profiles yet. A record is not data until the capture and
corroboration gates described in
[`docs/capture-methodology.md`](docs/capture-methodology.md) pass.

## The rule that defines the product

**An identity is observed from a running client, never derived from source
code, copied from a registry, inferred from a version, or harvested from an
uncontrolled public swarm.** Source and market-share material may prioritize a
target or explain a result. It cannot populate a profile.

Every published profile must:

- name the exact stable client version, platform, architecture, package and
  acquisition route;
- prove that two distinct acquisition routes resolved the same version;
- carry the raw evidence required to parse the observation again;
- use the Rust observer plus at least one independent connector;
- record agreement, disagreement, and fields a connector could not check;
- remain immutable after publication. A correction is a new record.

## What is captured

| surface | representative fields |
| --- | --- |
| peer ID | all 20 bytes, style, prefix, suffix alphabet and observed lifetime |
| HTTP tracker | raw request target, parameter order and encoding, header names/order, `User-Agent`, key shape, `numwant`, event behavior |
| UDP tracker | connect/announce fields, key, event, `num_want`, retransmission behavior |
| peer handshake | reserved bytes, info-hash binding and peer ID |
| extension handshake | raw bencode, extension key set and values such as `v`, `reqq`, `p`, `e`, `upload_only` and `metadata_size` |
| early peer wire | bitfield/fast-extension/extended-handshake order and repeatability |
| adjacent surfaces | MSE negotiation, DHT messages, peer exchange and web-seed request shape when reachable |

[`docs/architecture.md`](docs/architecture.md) is the technical authority.

## Scope

The starting target set covers qBittorrent, qBittorrent Enhanced, uTorrent,
BitComet, aria2, Transmission, Deluge, BitTorrent, BiglyBT, Tixati, KTorrent,
FDM, Zona, libtorrent, anacrolix/torrent and rqbit. The maintained matrix and
candidate acquisition routes live in
[`docs/client-matrix.md`](docs/client-matrix.md).

Only the newest stable release is pursued. Historical versions are not
backfilled. When a new stable release ships, the old record remains and the
catalogue grows.

## Repository layout

| path | purpose |
| --- | --- |
| [`docs/AGENTS.md`](docs/AGENTS.md) | standalone orientation and router for every work session |
| [`TODO/PROGRESS.md`](TODO/PROGRESS.md) | current state and the only work order |
| [`TODO/INDEX.md`](TODO/INDEX.md) | every work item and its status |
| [`catalogue/clients.toml`](catalogue/clients.toml) | machine-readable target scope and candidate acquisition routes |
| [`crates/bit-ids/`](crates/bit-ids/) | Rust consumer/model foundation |
| [`scripts/`](scripts/) | shell-first checks and orchestration plus the cross-platform doctor |
| `data` branch | append-only raw captures, profiles, indexes and generated formats once publishing exists |

## Start here

```bash
git clone https://github.com/polaris0xff/bit-ids
cd bit-ids
sh scripts/doctor/doctor.sh
sh scripts/common/check-gate.sh --fast
```

On native Windows:

```powershell
pwsh -NoProfile -File scripts\doctor\doctor.ps1
pwsh -NoProfile -File scripts\common\check-gate.ps1 -Fast
```

Then read [`docs/AGENTS.md`](docs/AGENTS.md) and take the first item from the
work order in [`TODO/PROGRESS.md`](TODO/PROGRESS.md).

## Publishing

The planned `data` branch exposes raw and normalized files at stable paths.
Tagged releases will package the identical assembled tree as JSON, JSONL, CSV,
SQLite, CBOR and deterministic archives, with a manifest and SHA-256 checksums.
No release or data branch exists yet; the open work is tracked under `PUB-*`
and `CI-*` in [`TODO/INDEX.md`](TODO/INDEX.md).

## Licence

Code, documentation and measurements produced by this project are released
under the [0BSD licence](LICENSE). Third-party client binaries are measured,
never redistributed.
