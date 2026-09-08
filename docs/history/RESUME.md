# Resume

**Task:** Take the work order in `TODO/PROGRESS.md` in dependency order,
committing and pushing each green unit to `main`.

**Resume point:** ⭐ **`CI-02`, `PUB-04` and `LIB-01` are closed.** The first two
were the items the work order's ordering made look more blocked than they were;
the third was reachable all along and nothing had noticed. `TODO/ci.md`,
`TODO/publishing.md` and `TODO/library.md` carry them.

⛔ **Everything left in the work order is behind a capture host or the operator
decision below.** `CLIENT-01`, `CLIENT-06`, `CLIENT-05`, `OBS-07` and `OBS-10`
need a host `assert-disposable.sh --egress` does not refuse; `CI-03` is what
would supply one and is the next item; `CI-04` follows it; `PUB-05` is blocked on
the dependency decision. ⚠ `LIB-02`, `DOC-01` and `DOC-02` are the three that may
also be reachable without a host, and none was examined this session: read their
entries rather than assuming either way.

**In flight:** Nothing.

**Tree:** Re-measure it. This file is a claim about a tree that has moved.

⚠ **The container started on `claude/session-start-gate-setup-djq28p` with
`user.name` set to an agent, and the clone was shallow at 50 commits.** All three
were corrected before any editing: the branch to `main` per rule 7, the identity
to the operator's own per rule 11, and the clone with `git fetch --unshallow`.
`HEAD..origin/main` was 0 either way. ⛔ The identity is read from the history
with `git log --format='%an <%ae>' | sort -u`, never typed here: writing it into
a tracked file is what `check-no-secrets --public` refuses, and it refused this
paragraph's first draft. Re-measure all four rather than trusting this file.

⭐ `pwsh` 7.4.6, `shellcheck` 0.10.0 and `shfmt` 3.14.0 were installed at session
start from the commands in `TODO/PROGRESS.md`, and the `chmod +x` on the
PowerShell tarball was needed exactly as that note says.

## What this session learned, in the order it hurts

⛔ **A collision test is not an encoding test, and two plants proved it.**
`request_key_components_are_length_prefixed_rather_than_joined` compared one pair
of tuples under one separator. Dropping the platform from the request key
entirely, and replacing every length prefix with a separator byte, both left the
whole Rust suite green; only the harness's independent `python3` derivation
caught them. ⭐ The fix is general: pin the encoding against its own restated
specification, byte for byte, and vary each component of the key in turn.

⛔ **Two guards answering one code mask each other.** `LIB-01`'s per-file digest
comparison and its manifest-against-the-bytes comparison both report `E-LIB-02`,
so deleting either left every case green: the surviving one still produced the
code the cases asserted. Separate them by the path a refusal names, and give
each shape a case of its own.

⛔ **A published document with a writer and no reader hides a live defect.**
`Release::manifest_json` had none, so a consumer compared a bundle against a
manifest re-derived from that bundle and agreed with itself: a described file
that is missing is not described either. Two refusals existed and neither could
fire. `Indexes::to_json` had the same gap with milder consequences.

⭐ **A surviving plant is sometimes the design working.** Reading a record with
`serde_json::from_str` rather than `Profile::from_json` changed nothing, because
`Profile`'s hand-written `Deserialize` validates too. Read what a plant changed
before calling it a gap.

⛔ **A refusal the deriver cannot reach is a refusal nothing tests.** `PUB-04`'s
`contract` derives a path's stability, its integrity and its order, so three of
its five refusals can never fire on anything it produced: the two halves agree by
construction, and blanking each left the crate and the harness green. They are
reachable from a **file**, which is the door a consumer's copy comes through, so
they have document-level cases now. ⚠ The same pass reported a false SURVIVED
before that was believed, because its test selection ran `--lib` and the new
cases are an integration target.

⛔ **A documented path with no producer is worth deriving rather than reading.**
`docs/publishing.md` carried two `routes/` paths nothing had ever written, and
they cannot be written: the layout omits `<package>` while the acquisition routes
differ per package, which is `CORPUS-01`'s non-injective-path finding in a second
place. Deriving the documented set from an assembled release is what found it.

⛔ **A count written in prose is a value in two places with nothing comparing
them.** `VersionScheme::components` said "both callers use it" and named two;
`CI-02` is the third, and the sentence a reader checks before writing a fourth
implementation of the ordering had gone stale. Two more of the same shape were
found in one sweep: `scripts/README.md` said `store-lib.sh` is sourced by "all
eight of the harnesses above" over a set that two files differ from.

⛔ **Vocabulary written in a consumer grows a third spelling next.**
`ReleaseChannel`'s published spelling was written inside `staleness` because that
is where it was first needed. `docs/architecture.md` section 5 had already made
the rule for `Surface`, and it applies to every published name.

⭐ **A dead-code lint on a shared support module is a seam telling you it is
two modules.** `examples/support/reader.rs` held a store reader and a
`--scheme` parser, and the first example to need one and not the other compiled
a parser nothing called. `support/scheme.rs` is the split.

⚠ **A clippy `missing_panics_doc` is worth taking as a design note.** The
survey's request construction was written with an `expect` over an invariant
`judge` upholds. Written as a `filter` instead, a future verdict wired into
`opens_work` and not into `judge` becomes an `E-REQ-07` refusal naming the
verdict rather than a panic.

⚠ **`check-one-home` fires on a summary that restates its own record.** Two
CHANGELOG lines had to be rewritten to point at `TODO/ci.md` rather than repeat
it, which is the guard working: the changelog is a pointer and the entry is the
home.

## Carried forward, unchanged

⛔ **Nothing has ever been published and no capture has been taken.** The
publisher has never run against this repository's own remote and must not until a
measured record exists; everything in the tree is synthetic and says so. ⚠ And
nothing schedules the staleness monitor: `CI-02` built the comparison and its
driving surface, and no capture request has ever been opened.

⛔ **No observer has been driven by a stock `BitTorrent` client.** `curl`,
`libtorrent`'s bencode and a Python MSE initiator are independent implementations
written from specifications, which share this project's reading of the protocol.
`OBS-07` owns the stock-client controls and `CI-03` owns the runner.

⛔ **A Windows capture is not permitted.** The disposable-host guards read
`/proc/net/route` and `/etc/machine-id`, so there is no boundary to run before an
install on Windows. `CI-03` owns the pair.

⛔ **Run the gate with `sh scripts/common/check-gate.sh`, after the last edit.**
The last edit is the one made while writing the record, not the one that felt
like the end of the work. ⚠ **The gate is not the whole of part (a)**: `cargo
clippy`, `cargo fmt --check` and the test suite are separate rows, and a clippy
failure passed the gate twice in an earlier session before being caught.
⚠ `check-workflow.sh` is not in the gate and is run separately, because two of
its cases run the gate. `check-staleness.sh`, `check-access.sh` and
`check-catalogue.sh` **are** in the gate, because none of them runs one.
⛔ **It happened again this session**: the gate was green at 23 checks while
`cargo clippy` was red over a `len() > 0` in a test. The gate is not the whole of
part (a).

⛔ `check-remote-items` cannot be made to run here and installing `gh` does not
fix it. It is the one observed skip, and it is why the gate exits 1 under
`--strict` on this host and 0 on the Linux lane.

⛔ **A harness exit of 2 is *could not run*, never *refused***, and a plant that
did not compile is the same. Separate the two on every path. ⚠ A plant that did
not **apply** is a third status and is neither: of eighteen aimed at
`staleness.rs`, three failed to match the formatted source on the first pass and
say nothing at all about the guards they were aimed at.

⛔ **Read what a plant actually changed before believing it either way.** A
surviving plant is a question, not a verdict.

⚠ **Two twins agreeing is not two twins being right.** Compare them per planted
input, and give every planted input a declared expected outcome.

⛔ **A sweep proves a negative and its needle list is what rots.** `LIB-01`'s
"no network access" is established by the crate having no way to reach one, and
`check-catalogue.sh` checks its own needles against `bit-ids-lab`, which really
does carry sockets. A needle list that has stopped matching reports the same
clean answer over a crate full of them.

⭐ **The strongest control available here is a reader this project did not
write.** `sha256sum -c` verifies a release, `cbor2` reads a canonical encoding,
`libtorrent` and `torf` read a generated torrent, `curl` is a complete HTTP
client, Python's `pow` is an arbitrary-precision modular exponentiation, and
Python's `hashlib` re-derives a capture request's identifier from the encoding
restated in the source.

⭐ **A push path can be driven for real with no network and no credential.** A
bare repository in a scratch directory is a remote as far as git is concerned,
and both `check-publish` and `check-access` use one. ⚠ **An immutability claim
needs two publications**: "this path never changes" is a comparison between
trees, and a checker given one can only read a label off a document that asserted
it. The other half is that at least one path must have moved, or the comparison
passes over two identical publications.

⛔ **No GitHub URL has ever been fetched**, because nothing has ever been
published. `docs/publishing.md` carries the three forms and they are
unexercised.

⚠ Read each CI run's failing **step** before its conclusion: a failure above
*Rust check* is the runner's network rather than the tree.

⛔ **The nine commit stamps before 2026-09-06T07:56Z are fabricated**, which is
why `CHANGELOG.md`'s ordering is not monotonic. They are not retro-corrected:
rewriting somebody else's record of when they worked is worse than the gap it
closes. Read the machine clock with `date -u +%Y-%m-%dT%H:%M:%SZ`.

## Pending operator decision

⛔ **One, and `PUB-05` carries it in full: how the SQLite rendering gets
written.** `rusqlite` brings a vendored C library and a build script into a
workspace whose lints say `unsafe_code = "forbid"`. The recommendation is the
crate, pinned, with the exception recorded against that one dependency rather
than the workspace lint relaxed. ⚠ Nothing is blocked behind the answer except
that one rendering.

⭐ The previous session's record is
[`SESSION-2026-09-06-ADJACENT.md`](SESSION-2026-09-06-ADJACENT.md), which carries
what each of its five review passes swept. ⚠ It is linked from here and nowhere
else, so a rewrite of this file that drops the link orphans it, and
`check-docs` refuses that: an unlinked page is not read, so it is not corrected.

**Paste:** Read `docs/AGENTS.md` in full, follow its start protocol, and take the
first unblocked item from `TODO/PROGRESS.md`. Work on `main`. Re-measure the
clone, branch, identity and remote rather than trusting this file.
