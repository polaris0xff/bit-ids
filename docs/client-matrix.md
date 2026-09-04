# Client matrix

This is the human-readable scope. The machine-readable home is
[`../catalogue/clients.toml`](../catalogue/clients.toml). Candidate routes are
hypotheses for `ACQ-*` work, not claims that the requested version is currently
available there.

## Priority evidence

The August 2026 TorrentAnalytics page was read on 2026-09-04 through the
operator-specified read proxy after the direct fetch route was unavailable. It
reports connection share from DHT discovery followed by TCP handshakes. The
requested families appear as qBittorrent 48.71%, uTorrent 13.41% plus a
separate `uTorrent` row at 3.12%, BitComet 5.35%, libtorrent 3.94% plus
`libTorrent` 2.05%, Transmission 3.19%, Zona 2.66%, Deluge 2.13%, BitTorrent
1.97%, BiglyBT 1.67%, Tixati 1.49%, qBittorrent Enhanced 0.73%, KTorrent 0.62%,
FDM 0.23%, rqbit 0.03% and aria2 0.01%.

Those labels are reproduced as the source presented them; similar-looking rows
are not merged. The page cannot establish which implementation produced an
unknown prefix. Its passive method makes it prioritization evidence only.

Wikipedia's usage-share page, read the same day, has a newest table from March
2020. It corroborates that most named applications have long-standing visible
usage, but it is not current market-share evidence.

Sources:

- <https://torrentanalytics.net/top_client?period=2026-08>
- <https://en.wikipedia.org/wiki/Usage_share_of_BitTorrent_clients>

## Application targets

| id | platforms | licence class | candidate route A | candidate route B | entry |
| --- | --- | --- | --- | --- | --- |
| `qbittorrent` | Linux, Windows | open source | vendor/GitHub-linked release artifact | distro/WinGet/Flatpak package | `CLIENT-01` |
| `qbittorrent-enhanced` | Linux, Windows | open source fork | GitHub release | community package registry | `CLIENT-02` |
| `utorrent` | Windows | proprietary | vendor installer | WinGet/community package | `CLIENT-03` |
| `bitcomet` | Windows | proprietary | vendor installer | package registry | `CLIENT-04` |
| `aria2` | Linux, Windows | open source | GitHub release | distro/WinGet package | `CLIENT-05` |
| `transmission` | Linux, Windows where supported | open source | upstream release/build | distro/WinGet package | `CLIENT-06` |
| `deluge` | Linux, Windows | open source | upstream/PyPI release | distro/Windows package | `CLIENT-07` |
| `bittorrent` | Windows | proprietary | vendor installer | package registry | `CLIENT-08` |
| `biglybt` | Linux, Windows | open source | GitHub release | distro/package registry | `CLIENT-09` |
| `tixati` | Linux, Windows | proprietary freeware | vendor package | package registry | `CLIENT-10` |
| `ktorrent` | Linux | open source | KDE/Flatpak release | distro package | `CLIENT-11` |
| `fdm` | Linux, Windows | proprietary | vendor package | package registry | `CLIENT-12` |
| `zona` | Windows | proprietary | vendor package | package registry if independently verifiable | `CLIENT-13` |

An entry does not weaken the two-route rule when a candidate disappears. It
records three routes considered, keeps the target open and moves to the next
unblocked client.

## Library targets

| id | harness | route A | route B | entry |
| --- | --- | --- | --- | --- |
| `libtorrent` | minimal Rust/C++ or Python-binding client pinned to one stable libtorrent | package registry/system package | matching GitHub tag | `ENGINE-01` |
| `anacrolix-torrent` | minimal Go client using `github.com/anacrolix/torrent` | Go module proxy | matching GitHub tag | `ENGINE-02` |
| `rqbit` | minimal Rust client using the stable rqbit/librqbit release | crates.io package | matching GitHub tag | `ENGINE-03` |

Library harnesses are not allowed to choose an identity value for convenience.
The profile records both library version and harness commit, and the observer
measures what the built harness actually emits.

## Platform rule

Capture every platform a stable upstream release genuinely supports and the
project can acquire twice. Windows-only software stays Windows-only. A missing
Linux row is not filled by Wine unless Wine itself becomes a separately named
platform/packaging dimension.
