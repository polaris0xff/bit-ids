# Session record: 2026-09-05, the evidence path

The session summary printed to the operator, saved so it survives the chat. It
carries no work order; [`../../TODO/PROGRESS.md`](../../TODO/PROGRESS.md) has
that and is where it is correct.

Baseline `f7580d0`, work range `f7580d0..694a61d`. The record commits that
follow carry this file, and each row that depends on one names it.

| row | measured |
| --- | --- |
| Commits | 4 closing work over `f7580d0..694a61d`, all pushed and read back from `origin/main`. ⚠ The record commits after that range are deliberately not counted: a total including them changes when this file is written |
| Work | 3 entries completed, 0 checkpointed, 0 failed. 1 entry moved behind a dependency on a measurement |
| Changes | 25 files over `f7580d0..694a61d`, +3369 / -120 |
| Checks | gate 12 checks, 11 passed, 0 failed, 1 skipped on `sh`; 10 passed, 0 failed, 2 skipped on `pwsh` |
| Suite | 30 binaries, 289 passed, 0 failed, plus 2 doctests |
| Mutation | 94 defects planted across 4 passes, 91 refused, 3 not refused and each named below |
| CI | runs 25, 26 and 28 green on **both** lanes, read back from the API per commit. ⚠ Run 27 at `f9239a5` was red on Windows and green on Linux; the cause is named below and run 28 confirms it was transient |
| Cost | no money. Network: one crate already in the lockfile, two Python libraries into a virtualenv, and three read-only GitHub API calls |
| Health | tree clean, level with `origin/main`, nothing deployed, no capture taken |

## What closed

| entry | what it establishes |
| --- | --- |
| `OBS-08` | the synthetic torrent, with its bytes a function of its declared spec so a record's `capture.fixture` can be re-derived and checked |
| `OBS-09` | what a run's transcript becomes on disk: one artifact per endpoint plus the manifest rows citing them |
| `CI-05` | an acceptance command that cannot pass over nothing, in both twins, over `Prove` paragraphs and workflow `run:` lines |

⭐ **The observer layer is complete.** A first vertical capture is possible from
here, and what stands between is a client adapter and a host, not machinery.

## Defects found, by the pass that found them

⭐ **The same defect class was found twice in one session by two different
entries, and neither found it by looking for it.**

**A constant every test reads is a constant no test can check.** `OBS-08` found
two before planting against them: nothing pinned `PIECE_HASH_LEN`, so narrowing
it re-chunked the `pieces` string and the comparison against it in the same
step, and a test spec built at `MIN_PIECE_LENGTH` made a declared piece length
and the floor indistinguishable. `OBS-09`'s first mutation round then missed six
plants, and all six were the same shape rather than equivalent mutants: the
transcript's schema string, the producing tool and the phase were each asserted
against a constant that moves with the code, or against nothing.

⛔ **`E-MAN-52` and `E-MAN-53` are weaker than they read.** They require an
artifact's tool and phase to name *something the run declares*, not the right
thing. A writer that filed every artifact under another declared tool produces a
manifest that validates and lies, and the schema cannot see it.

⛔ **A corpus only tests the defects it contains an example of.** `OBS-08`'s
payload stream had nothing pinning it, and the module's own unit tests could not
have: reproducibility, seed-dependence and prefix-stability all survive a
flipped endianness, a drifted constant or a halved word. A generated torrent is
citable only while its bytes are a function of its declared inputs, so that
drift would silently invalidate every `capture.fixture` already recorded.

**Door sweep.** Three findings, each a gate on one of two doors into the same
action. Nothing joined the torrent generator to the observers, so the twenty-byte
info hash was declared independently in two crates with neither the widths nor
the bytes checked against each other. `OBS-09` gated the *spelling* of an
artifact's path and not where it *resolves*, so a symlink in a reused bundle root
satisfied every canonical-path rule and landed the artifact outside. And `CI-05`
was authored against entry `Prove` lines only, when the workflow's `run:` is the
command every push executes with nobody reading the output.

**Claim audit.** This entry's own `Premise` claimed all nine bare-filter
acceptance commands had been rewritten. Five had not, in `FOUND-03` and all four
`SCHEMA-*` entries; each is corrected and each corrected command was run before
it was written down. A component row and a dependency argument had drifted into
two documents. One code comment argued from platform folklore where the
acceptance suite builds the case instead.

**What was measured but never verified.** The qBittorrent release listing
answered with four releases and that looked like truncation. It is not: page two
of the same endpoint is empty and the `tags` endpoint answers with at least a
hundred. ⭐ **The project tags far more versions than it publishes as
releases**, so a resolution reading only `releases` selects from a different and
much smaller population than the target's versions. It changed nothing here
because `5.2.3` leads both, and it would change the answer for a target that
stops minting release objects.

**Writing the probes.** The three guards in the mutation harness were themselves
exercised, with an absent literal, an ambiguous one and a no-op edit, because a
probe's guard is a guard like any other and this project has been burned by one
three times.

## The two guards that are not refuted

| guard | why nothing refutes it | what would have to be true |
| --- | --- | --- |
| `piece()`'s `checked_mul` and `checked_add` | equivalent on every 64-bit target: a piece length is a power of two of at most `2^31` and an index at most `2^32`, so the product is at most `2^63 - 2^31` | a 32-bit target, where the multiplication can succeed and the addition still wrap. ⚠ Both CI lanes are `x86_64`, read from the Windows runner's own toolchain string |
| `read_back`'s size comparison | the digest subsumes it for any artifact this bundle wrote: a change to the length changes the digest | a `Bundle` reconstructed from a manifest read off disk, whose declared length disagrees with the bytes its own digest names. `PUB-01` is where that arrives |

⭐ Both are kept rather than deleted, and both say so where they are written, so
a later reader does not take an unreached guard for a proven one.

## What this session could not do

- ⛔ **No capture was taken and none could be.**
  `sh scripts/acquisition/assert-disposable.sh --egress` exits 1 on a session
  host, so installing and driving a client would be the capture that boundary
  exists to refuse, and the Windows guard pair does not exist at all. Three
  routes were tried before the client entries were moved behind `CORPUS-01`;
  [`../../TODO/clients.md`](../../TODO/clients.md) carries them.
- ⚠ **One Windows CI lane went red, and not because of the change.** Run 27 at
  `f9239a5` failed at *Install pinned Rust toolchain*,
  before any repository code ran, with
  `could not download file from 'https://static.rust-lang.org/dist/channel-rust-1.98.0.toml.sha256'`
  and `os error 10060`, a TCP connect timeout to rustup's CDN. Every later step
  reports `skipped`. The Linux lane of the same commit is green including the
  repository gate, which runs both `check-project` halves under `--strict`.
  ⭐ **Confirmed transient rather than assumed to be:** run 28 at `14f1da6`, a
  superset of that tree, installed the same pinned toolchain on the same runner
  image in ten seconds and went green on both lanes, repository gate included.
  A toolchain download is not something a change here could have fixed, and one
  red run of that shape is no evidence about the tree.
- `check-remote-items` still cannot run on this host, for the reason two
  sessions ago measured.
- ⛔ No observer has been driven by a stock `BitTorrent` client. `OBS-08`'s
  driven pass used `libtorrent` 2.1.1.0 and `torf` 4.3.1, which are third-party
  implementations reading a file rather than a client announcing about it, and
  that is a stronger control than a decoder written for the purpose but not the
  same thing. `OBS-07` owns the stock controls and `CI-03` owns the runner.
