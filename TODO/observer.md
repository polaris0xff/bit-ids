# Observer entries

All observer services bind only to disposable local or isolated laboratory
interfaces. Each stores exact received bytes before parsing them.

## OBS-01: Isolated Rust loopback observation lab

Source: operator active-probing rule and bit-cli loopback architecture
Priority: P0 | Effort: L | Status: DONE

Split on 2026-09-04. The entry was authored at XL with the instruction to split
transport services into follow-up entries before implementation if the
acceptance could not remain atomic. It could not, for a reason the original
Prove states in its own sentence: it asks for a known client fixture and for
Linux and Windows agreement, and neither exists. No client adapter is written,
and a Windows capture is not permitted at all until `CI-03` supplies the
disposable-host guard pair. An entry whose acceptance cannot run is an entry
nothing can close.

What moved out, each to its own entry with its own acceptance: the synthetic
torrent to `OBS-08`, the durable evidence journal to `OBS-09`, and the
cross-platform equality half of the original Prove to `OBS-10`. The transport
services were already separate as `OBS-02` through `OBS-05`. What is left here
is the supervisor those five plug into, and it is provable on this host today.

Problem: Client adapters need a deterministic fake torrent environment that
never depends on public trackers, DHT, or peers. Nothing in the tree binds a
socket: `rg 'std::net|TcpListener|UdpSocket' crates` returns nothing on
`2fb8548`, so there is no endpoint for an observer to be.

Premise: A supervisor that owns every bind can hold the property the transport
observers must not each re-implement, which is that a lab endpoint is on
loopback and nowhere else. Measured rather than assumed: the address a listener
actually got is read back from the socket after binding, because a bind request
and a bound address are different facts.

Approach: A `bit-ids-lab` workspace member carrying the supervisor. It hands out
endpoints, refuses a bind to any address outside loopback before the syscall and
verifies the bound address after it, allocates ephemeral ports so two labs on
one host cannot collide, holds a deadline per lab rather than per read so a
silent client cannot hang a run, records every byte each endpoint received in
order with a monotonic offset, and shuts every endpoint down on drop. Blocking
`std::net` and one thread per endpoint, not an async runtime: the lab serves a
handful of local connections and an async runtime is a large dependency for a
scheduler this workload does not need. Rejected alternatives are recorded in the
Decision below.

Decision: `std::net` with threads over `tokio`. `tokio` would add a dependency
tree an order of magnitude larger than everything this workspace has, into the
one component that must be reviewable, and `docs/supply-chain.md` requires the
argument in the entry. The blocking API costs one thread per endpoint, which is
bounded by the surface count, and gives per-socket timeouts directly. Also
rejected: binding port 0 and reporting the requested address, which is what
makes a lab look loopback-only while a misconfiguration listens elsewhere.

Prove: `cargo test -p bit-ids-lab --locked --all-targets` covers a refused
non-loopback bind, a bound address read back from the socket, two labs on one
host with distinct ports, a deadline expiring on a client that connects and
sends nothing, the ordered byte record, and every port released after shutdown.

⚠ The Prove was authored as `cargo test --workspace --locked lab_supervisor`
and that command runs nothing. `cargo test` filters by test **name**, and a
filter matching none exits 0: it printed `running 0 tests` for every binary in
the workspace and reported success. The closed entries' filters do match, by a
convention that holds exactly: measured on 2026-09-05, all 110 test functions
across the eight pre-existing test files start with their file's name, and
nothing checks that. `--test lab_supervisor` was the first correction and was
also wrong, more quietly: it selects the integration target and skips the
library's own tests, so the guard-mutation pass found the bound-address readback
uncovered by the acceptance while a unit test for it existed.
[`../docs/conventions/forbidden-patterns.md`](../docs/conventions/forbidden-patterns.md)
carries the class and `CI-05` is the check that would stop it returning.

Closure evidence: run on 2026-09-05.
`cargo test -p bit-ids-lab --locked --all-targets` reports 27 passed, 0 failed,
over 9 library tests and 18 acceptance tests. `cargo test --workspace --locked
--all-targets` reports 17 binaries and 184 passed, 0 failed: 9 test files, 3
library suites and 5 examples, which is every one on disk.
`cargo test --workspace --locked --doc` reports 2 passed, 0 failed.
`cargo fmt --all -- --check`, `cargo check`, `cargo clippy -- -D warnings` at
`--workspace --locked --all-targets`, `shellcheck`, `shfmt -d -i 2 -ci` and
`sh scripts/common/check-gate.sh` all exit 0.

Guard mutation: 15 defects planted one at a time, each verified to have changed
the file, all 15 refused with the exit code read unpiped. Two rounds were needed
and the first found three misses, all real: the acceptance skipped the library
tests, the responder was offered its buffer once per read rather than until it
stopped consuming, and shrinking the datagram buffer to four bytes changed no
result because no fixture datagram was larger than three. The second is a defect
in the shipped code, not the tests: a client sending two units in one write and
waiting for two answers would have waited forever.

⚠ A finding about the probe rather than the code: the first mutation script did
not check that its edits applied, and one `sed` pattern silently matched
nothing. The run went green over unmutated source and was read as a guard that
failed to fire. Every plant now compares the file's checksum either side.

Driven on 2026-09-05 with two clients that are not this project's test harness.
`cargo run -p bit-ids-lab --example loopback-lab` printed its two endpoints on
`127.0.0.1` with ports the kernel chose; `curl` to the stream endpoint returned
`HTTP/1.1 200 OK` and a `python3` datagram to the other came back reversed. The
transcript recorded four segments in order, request before answer on each
endpoint, with the request's percent-encoding intact in the recorded bytes, and
`deadline expired: true`, so the deadline stopped the run rather than a client
closing.

Residual: a stream segment is what the observer read, not provably what the
target wrote, because TCP does not preserve write boundaries at all. The buffer
is larger than any message these surfaces carry, so no message is split by the
buffer size; a burst larger than the buffer still spans two segments, and no
reading of the bytes can tell that from two writes.
`crates/bit-ids-lab/src/journal.rs` states the limit where a reader of the
journal will find it.

## OBS-08: Synthetic torrent for the observation lab

Source: split out of `OBS-01` on 2026-09-04
Priority: P0 | Effort: M | Status: DONE

Problem: A client announces about an info hash and requests pieces of a real
piece layout. Without a torrent the lab can accept a connection and cannot make
a build say anything, so every observer below it has nothing to observe.

Premise: The torrent can be generated rather than committed, and generating it
is what makes it citable. `capture.fixture` in `crates/bit-ids/src/record.rs`
already requires a digest of the metainfo a run used, and a generated torrent
whose bytes are a function of its declared inputs is reproducible from the
record. ⚠ This entry was authored naming that field `capture.fixture_digest`,
which does not exist; the field is `capture.fixture` and the entry was wrong
about the tree it was written against.

Approach: A module in `bit-ids-lab` that builds the info dictionary, the piece
layout and the payload from declared parameters, encodes it with
`bit_ids_wire::bencode` rather than a second encoder, and derives the info hash
from the encoded info dictionary. The payload is generated bytes, never a
copyrighted file. The `.torrent` bytes and the digest the manifest cites come
out of one function, so the digest cannot describe something other than what
the client was handed.

Prove: `cargo test -p bit-ids-lab --locked --all-targets` checks that the
generated document round-trips through `bit_ids_wire::bencode`, that the info
hash is the digest of the encoded info dictionary and not of the whole
document, that identical parameters produce identical bytes, and that one
changed parameter changes both the bytes and the digest.

Closure evidence: run on 2026-09-05.
`cargo test -p bit-ids-lab --locked --all-targets` reports 53 passed, 0 failed
over 5 binaries: 14 library tests, 21 in `lab_supervisor`, 18 in
`synthetic_torrent`, which holds the four properties the Prove names and the
layout a client would refuse the torrent over, and two examples carrying none.
`cargo test --workspace --locked --all-targets` reports 28 binaries and 270
passed, 0 failed, up from 25 and 250, and `--doc` reports 2 passed. `cargo fmt --all -- --check`,
`cargo clippy --workspace --locked --all-targets -- -D warnings`,
`sh scripts/common/check-gate.sh` and `pwsh -File scripts/common/check-gate.ps1`
all exit 0.

⭐ **The acceptance suite reads the info hash out of the file rather than out of
the generator.** Comparing `encode(torrent.info())` against the same encoder's
output cannot see an info hash naming a dictionary the file does not contain,
because both halves move together. So the suite walks the raw metainfo, cuts out
the byte range the `info` key maps to, and hashes that. The guard-mutation pass
below is what proves the distinction is load-bearing: an info dictionary given
one extra key on its way into the file, with the hash still over the original,
is refused by that test and by nothing else.

⭐ **Two gaps in the suite were found by asking what each planted defect would
have to survive, before planting it.** Nothing pinned `PIECE_HASH_LEN`,
`MIN_PIECE_LENGTH` or `MAX_PAYLOAD_BYTES`: every test reads them, so each moves
with them and none can see one drift, and a piece hash narrowed to sixteen bytes
would re-chunk the `pieces` string and the comparison against it in one step.
And the test spec was built at `MIN_PIECE_LENGTH`, which makes `piece length`
and the floor indistinguishable, so a generator writing the constant instead of
the declared value read as correct. Both are closed and both plants are refused.

Guard mutation: 33 defects planted one at a time over
`crates/bit-ids-lab/src/torrent.rs`, each by literal replacement required to
match exactly once, each verified to have changed the file by comparing its
SHA-256 either side, each acceptance exit code read unpiped, and the tree
restored and re-run clean afterwards. 31 refused. The two that are not are one
defect in two halves and are named below.

⛔ **The payload's byte stream had nothing pinning it, and the module's own unit
tests cannot pin it.** They assert reproducibility and seed-dependence, and both
survive any change to the arithmetic: a flipped endianness, a drifted constant
or a halved word still produce a stream that is reproducible, seed-dependent and
prefix-stable. A generated torrent is citable only while its bytes are a function
of its declared inputs, so a silent drift there invalidates every
`capture.fixture` already recorded. The suite now compares the payload against
`SplitMix64` restated from the specification, anchored to the first five words
the public-domain reference emits for seed zero. Four plants that nothing else
catches are refused by it.

Driven on 2026-09-05 by two third-party implementations, neither of which shares
this project's reading of BEP 3:
`cargo run -p bit-ids-lab --example synthetic-torrent -- <path>` writes the file
and prints what the generator derived from it, and `libtorrent` 2.1.1.0, the
engine `ENGINE-01` targets, and `torf` 4.3.1 each read it back. ⚠ Which clients
embed libtorrent is not asserted here: this session measured a version and a
parse, not a client population, and `docs/client-matrix.md` carries what the
project actually knows about engine relationships. 26 comparisons, 26 agree:
the info hash, name,
piece length, piece count, total size, the `private` flag, the announce URL, the
creation date, `capture.fixture` over the file on disk, and every piece hash
against a payload regenerated from the declared seed alone. ⭐ libtorrent hands
back the info dictionary's own bytes, so the info hash was recomputed over the
range **it** located rather than the one this project's decoder did, and the two
ranges agree.

⛔ **Four negative controls, because a reader that agrees with everything has
agreed with nothing.** One flipped bit in a piece hash changes the info hash for
both readers, and a truncated file is refused by both. The agreement above is
therefore a result rather than a property of the readers.

⭐ **The door sweep found the one thing neither the suite nor the driven pass
could: nothing joined the generator to the observers.** The info hash is
declared twenty bytes wide in two crates, as `PIECE_HASH_LEN` here and as
`INFO_HASH_LEN` in `bit-ids-wire`, and no code passed one to the other, so
neither the widths nor the bytes were checked against each other. That is the
composition class `docs/methodology/gate.md` names: each part correct, the
assembly untested. `crates/bit-ids-probe/tests/generated_torrent.rs` closes it,
in the probe crate because that is the only one that depends on both. It hands
the generated array to `PeerWire`, which makes the compiler check the width, and
drives the value over a real socket at both surfaces a capture uses to identify
a torrent: the peer handshake, where the observer must answer about the same
torrent or a client drops it, and the announce query, where the twenty raw bytes
go through percent-encoding. Covered by the workspace command above rather than
by this entry's `-p bit-ids-lab` one. Three plants, all refused: an observer
answering with a zeroed info hash, a handshake decoded one byte off, and a
percent decoder dropping the low nibble.

⚠ The info hash is not reproduced here as a number. `check-no-secrets --public`
refuses a bare run of forty hex digits in prose and is right to, because that is
what a token looks like; narrowing that rule a second time to publish a value
nobody needs is how a rule ends up switched off. The example above prints it and
the spec regenerates it.

⚠ **What the driven pass still is not.** These are libraries reading a file, not
a client downloading a torrent. libtorrent accepting the metainfo and agreeing on
every piece hash is strong evidence the layout is one a client will verify
against, and it is not the same as a build announcing about it. `OBS-07` owns the
stock-client controls and they need a host the `docs/capture-host.md` guards
permit, which a session host is not.

The one guard that is not refuted: `piece()` computes its offset with
`checked_mul` and `checked_add`, and replacing either with the unchecked
operator is refused by nothing. It is an equivalent mutant on every 64-bit
target and both CI lanes are 64-bit: a piece length is a power of two of at most
`2^31` and an index at most `2^32`, so the product is at most `2^63 - 2^31` and
the sum at most `2^63`, both far below `usize::MAX`. What would have to be true
for it to fire is a 32-bit target, where the multiplication can succeed and the
addition still wrap. ⭐ The `checked_add` is there **because** the mutation was
equivalent: asking why exposed a checked multiplication next to an unchecked
addition, which is a guard on one of two arithmetic steps, and the second step
cost one line.

Residual: the generator produces a single-file torrent only. A multi-file
`info` dictionary is a different shape, with a `files` list instead of `length`,
and a build can behave differently on one. Nothing needs it yet, because a
capture makes a client announce and request a piece, and one file does that.
`OBS-06` or a client entry files it if a target turns out to differ.

Decision taken here: `sha1` 0.11.0 was added, which is the first third-party
crate since `SCHEMA-01`, and `docs/supply-chain.md` requires the argument in the
entry. The info hash is SHA-1 by BEP 3 and this project does not get to choose
otherwise; the workspace already depends on `sha2` 0.11.0, the same RustCrypto
release train with the same construction and the same `digest` traits, so this
adds one crate and no new maintainer to trust. The rejected alternative was
implementing SHA-1 here: new unaudited cryptographic code in the component that
decides whether two captures are of the same torrent, to save one small
dependency. ⭐ The dependency is checked against RFC 3174's own vectors rather
than trusted, which is what `the_sha1_implementation_matches_the_published_vectors`
is for.

⚠ Two digests of two different things, and confusing them is the trap this entry
carries: the info hash is SHA-1 of the encoded info dictionary, and
`capture.fixture` is SHA-256 of the whole metainfo file. Different algorithms
over different byte ranges.

`check-no-secrets --public` refused the RFC 3174 vectors as long hex, correctly:
forty lowercase hex digits is exactly what a token looks like. Narrowed rather
than switched off, per `docs/security/secrets.md`, and anchored to a constant
named for its RFC and to exactly forty digits. Proven on 2026-09-05 with five
cases in both twins: a bare hex run, a credential beside an allowed vector on
one line, the same value under an unallowed name, a 64-digit value under the
allowed name, and the clean tree. Both halves agreed on all five.

## OBS-09: Raw evidence journal and bundle writer

Source: split out of `OBS-01` on 2026-09-04
Priority: P0 | Effort: M | Status: DONE

Problem: `OBS-01` keeps what the lab observed in memory, which is what its own
tests can assert against and is not what a capture publishes. A run has to
leave content-addressed artifacts a manifest can cite, and nothing writes them.

Premise: The shape is already specified and already checked.
`crates/bit-ids/src/manifest.rs` requires per artifact a kind, a readable path,
a size, a digest, the tool that produced it, the phase it came out of, and
whether anything was scrubbed, and it derives the store path from the digest
rather than recording it. So this entry writes to a contract that exists rather
than inventing one.

Approach: A writer in `bit-ids-lab` that takes the supervisor's ordered byte
record and emits one artifact per endpoint plus the manifest rows describing
them. The digest is computed over the bytes written, read back from the file
rather than from the buffer, because a writer that reports the digest of what
it meant to write cannot detect a short write. A redaction declaration is
emitted whenever anything was scrubbed, so `raw` cannot quietly mean `edited`.

Prove: `cargo test --workspace --locked --test evidence_journal` writes a bundle to a
temporary directory, reads every artifact back off disk, and binds the manifest
it produced against a profile with `bit_ids::manifest::bind`, exiting non-zero
if any shared value disagrees. A truncated artifact is planted and the digest
comparison must refuse it.

⚠ The Prove names `--test evidence_journal`, which selects the integration
target and skips the library's own tests. That is the shape `OBS-01` was
corrected for and `../docs/conventions/forbidden-patterns.md` carries; the
acceptance run is `cargo test -p bit-ids-lab --locked --all-targets`, which is
this file and the module's unit tests together. `CI-05` is the check for it.

Closure evidence: run on 2026-09-05.
`cargo test -p bit-ids-lab --locked --all-targets` reports 71 passed, 0 failed
over 7 binaries: 17 library tests, 15 in `evidence_journal`, 21 in
`lab_supervisor`, 18 in `synthetic_torrent`, and three examples carrying none.
`cargo test --workspace --locked --all-targets` reports 30 binaries and 288
passed, 0 failed, up from 28 and 270. `cargo fmt --all -- --check`,
`cargo clippy --workspace --locked --all-targets -- -D warnings`,
`sh scripts/common/check-gate.sh` and `pwsh -File scripts/common/check-gate.ps1`
all exit 0.

⭐ **The bind test uses `bit-ids`'s own golden manifest and profile, included
rather than copied.** The bundle's rows are spliced into both documents under
the identifiers the golden profile already cites, each document keeping its own
shape, and both are parsed through the validating route so that what `bind`
reports is a disagreement between them rather than a defect inside one. Splicing
into one document and not the other is the control: without it, a `bind` that
passed over the real artifacts would also have passed had the writer produced
nothing at all.

⛔ **An endpoint the plan does not name is refused rather than defaulted.** The
writer was first authored to derive an identifier from the endpoint name and
assume `observer_stream`, which gives a run that grew a surface an artifact
nobody planned, filed under whatever the writer guessed, and `E-MAN-52` and
`E-MAN-53` cannot see it: they only require the tool and the phase to name
something the run declares. Failing closed costs one line in the plan.

Guard mutation: 35 defects planted one at a time over
`crates/bit-ids-lab/src/evidence.rs`, each by literal replacement required to
match exactly once, each verified to have changed the file by comparing its
SHA-256 either side, each acceptance exit code read unpiped, and the tree
restored and re-run clean afterwards. 34 refused. ⭐ The first round refused 26
and the six it missed were all real gaps in the tests rather than equivalent
mutants: the transcript's schema string, the tool and the phase were each
asserted only against the constant or not at all, so a writer that filed every
artifact under another declared tool produced a manifest that validates and
lies. That is the same shape `OBS-08` found in its own constants, twice in one
session.

⛔ **The door sweep found a gate on the spelling of a path and none on where it
resolves.** `RelPath` refuses `..`, a leading separator and a backslash, and
every one of those is a rule about the text; a symlink sitting in a reused
bundle root satisfies all of them and lands the artifact outside, with the
manifest citing a path that reads as inside. The writer now resolves the root
once and requires every artifact's directory to resolve under it, and refuses a
path already held by a symlink, a file or a directory. ⭐ Resolving the root is
the half that is easy to miss: without it the check refuses **every** write on a
host whose root is itself behind a symlink, which is `/tmp` on macOS and a bind
mount anywhere, and it reads as a broken filesystem rather than as this check.

⭐ **The occupied check and the read-back split on principle rather than on
convenience.** The first refuses what would be followed or overwritten; the
second catches what would be swallowed. A character device planted at an
artifact path accepts every byte and returns none, which is the one case a check
on existence cannot tell from a healthy write and a check on the bytes can, so
refusing it too would leave the read-back with no reachable failure and no way
to know it works.

Driven on 2026-09-05 by a client that is not this project's test harness.
`cargo run -p bit-ids-lab --example evidence-bundle -- <root> 6` runs a lab on
two surfaces, writes the bundle, verifies it and prints what a manifest would
carry; a Python client connects to both endpoints, sends bytes it chose, and
then reads the bundle off disk. 32 comparisons, 32 agree: the file, its size,
its digest recomputed independently, the content-addressed store path
re-derived from that digest, the schema and endpoint each transcript names, and
⭐ **the transcript holding exactly the bytes that client sent and exactly what
it read back, request recorded before answer.** The client is the authority on
what went over the wire, which is what makes that last one an outside opinion
rather than the lab agreeing with itself. Four negative controls: a truncated
artifact and a length-preserving edit both stop matching, and a rerun into the
same root is refused with `peer-wire.transcript.json is already taken`.

Decision taken here: `serde_json` was added as a **dev**-dependency of
`bit-ids-lab`, and `docs/supply-chain.md` requires the argument. The lockfile
diff is one line and no new package: `bit-ids` already depends on it, so no
maintainer joins the trust set. It is used only by the acceptance suite, to
splice the bundle's rows into the golden documents. ⛔ The transcript itself is
serialised by hand rather than through a derive, and
[`../docs/supply-chain.md`](../docs/supply-chain.md) carries why.

The one guard that is not refuted: `read_back` compares the file's size and its
digest, and dropping the size comparison is refused by nothing. For an artifact
this bundle wrote the digest subsumes it, because any change to the length
changes the digest. What it catches is a record whose declared length disagrees
with the bytes its own digest names, and reaching that needs a `Bundle`
reconstructed from a manifest read off disk rather than one this process just
wrote. `PUB-01` is where that arrives; the comparison is kept and the reason is
stated where it is written.

Residual: nothing reads a `bit-ids/transcript/1` document back into typed
segments. The driven pass parses one in Python and the acceptance suite parses
one with `serde_json`, so the format is proven readable, and a Rust reader
belongs with the consumer that needs it rather than here. `LIB-01` and `PUB-01`
are the candidates.

## OBS-10: Cross-platform normalized-event equality

Source: split out of `OBS-01` on 2026-09-04
Priority: P1 | Effort: M | Status: OPEN

Problem: The original `OBS-01` Prove asked that a known client fixture produce
identical normalized events on Linux and Windows. That is the half of it that
catches a platform difference in the observer, and it is blocked on two things
this repository does not have: any client adapter, and a permitted Windows
capture host.

Premise: The blockers are named rather than assumed. `TODO/INDEX.md` carries
`CLIENT-01` through `CLIENT-13` all open, so no adapter can be driven, and
`docs/capture-host.md` states that the disposable-host guards read
`/proc/net/route` and `/etc/machine-id`, so a Windows capture is not permitted
until `CI-03` supplies the Windows pair.

Approach: Once `OBS-02` through `OBS-05` and one client adapter exist, run the
same lab and the same client build on both platforms, normalize the event
stream to the declared identity fields, and compare. A difference is a finding
about the observer until the same bytes are shown to differ, because the client
is one build and the observer is two compilations.

Prove: a matrix job runs `cargo test --workspace --locked --test cross_platform_events`
on Linux and Windows and both produce the same normalized event digest for one
client build.

Blocked by: `OBS-02` through `OBS-05`, one of `CLIENT-01` through `CLIENT-13`,
and `CI-03` for the Windows guard pair. Status stays OPEN rather than BLOCKED
because nothing external prevents progress; the dependencies are this
repository's own open work.

## OBS-02: HTTP tracker observer

Source: bit-cli T-234 and tracker request-order tests
Priority: P0 | Effort: L | Status: DONE

Problem: HTTP announces expose peer ID, user agent, header set, query order,
encoding, key, numwant, and event behavior that a peer-ID table omits.

Premise: A byte-preserving HTTP endpoint can observe these fields without
guessing how a client constructed them.

Approach: Capture request line and headers before normalization, return valid
tracker responses, and repeat lifecycle events under controlled torrents.

Prove: `cargo test -p bit-ids-probe --locked --all-targets` checks raw ordering,
binary query values, repeated requests, and malformed input behavior.

Closure evidence: run on 2026-09-05.
`cargo test -p bit-ids-probe --locked --all-targets` reports 19 passed, 0 failed.
`cargo test --workspace --locked --all-targets` reports 20 binaries and 203
passed, 0 failed: 10 test files, 4 library suites and 6 examples, which is every
one on disk. `cargo test --workspace --locked --doc` reports 2 passed.
`cargo fmt --all -- --check`, `cargo check`, `cargo clippy -- -D warnings` at
`--workspace --locked --all-targets`, `shellcheck`, `shfmt -d -i 2 -ci` and
`sh scripts/common/check-gate.sh` all exit 0.

The observer is `crates/bit-ids-probe/src/tracker_http.rs`, a responder for a
`bit-ids-lab` stream endpoint. It keeps the exact head bytes, decodes with
`bit_ids_wire::tracker_http`, and answers a bencoded response whose shape is read
out of the announce. Frames with `tracker_http::head_end`, which was added to the
codec rather than written a second time here: a framer that disagrees with its
decoder answers one request while the decoder reads another, and both halves look
correct alone.

Guard mutation: 17 defects planted one at a time, each verified to have changed
the file, all 17 refused. Two rounds. ⚠ The first round's script used `sed` with
`|` as its delimiter over Rust closures and four of its plants silently matched
nothing, which is the probe defect `docs/methodology/reviews.md` records about
this template's own patch script, hit for the second time in one session. The
second script replaces literal strings and asserts the match count.

The first round also found two real gaps. ⭐ Nothing exercised a head terminated
with bare newlines, so the framer's answer could be shortened by a byte and every
test still passed: every other head in the corpus ends `\r\n\r\n` or mixes the
two. And a head the codec refuses is answered and not kept as an announce, which
nothing asserted was safe; the bytes survive because the lab recorded them first,
and there is now a test holding the two halves to that.

Door sweep: the record was unbounded, so a build announcing in a loop would grow
it until the host ran out of memory, and the cap now counts what it stopped
keeping rather than discarding it silently. A second `Content-Length` header was
taking the first value; two lengths may disagree and there is no reading of them
that frames the rest of the connection correctly, so the request is refused.

Driven on 2026-09-05 with `curl`, which is not this project's test harness.
`cargo run -p bit-ids-probe --example http-tracker` printed an announce URL on
`127.0.0.1`; two announces through it were answered `200` and recorded with the
query keys in curl's order, the header names in curl's order and spelling, the
percent-encoding case per value, and a peer ID of 20 bytes for the first and 11
for the second. ⭐ The 11-byte peer ID was reported rather than refused, which is
the intended behaviour: BEP 3 fixes the width and `bit_ids::observation` enforces
it in a record, but a build that sends 19 is the measurement this project exists
to take. The compact announce was answered with a six-byte peer string and the
`compact=0` announce with a peer list, so the shape followed the request.

Residual: nothing here is evidence about any build. Every announce driven through
this observer was written by this project, because no client is installed and
`OBS-08` has not generated a torrent for one to announce about.

## OBS-03: UDP tracker observer

Source: bit-cli T-234 UDP key and numwant inventory
Priority: P0 | Effort: L | Status: DONE

Problem: UDP announces carry identity-adjacent fields and binary layout not
visible to the HTTP observer.

Premise: A local BEP 15 responder can drive connect and announce transactions
and preserve every datagram.

Approach: Implement strict transaction matching, deterministic responses,
packet capture, and parsed views with no lossy string conversion.

Prove: `cargo test -p bit-ids-probe --locked --all-targets` covers connect,
announce, timeout, retry, key, event, numwant, and rejection cases.

Closure evidence: run on 2026-09-05.
`cargo test -p bit-ids-probe --locked --all-targets` reports 35 passed, 0 failed.
`cargo test --workspace --locked --all-targets` reports 22 binaries and 219
passed, 0 failed: 11 test files, 4 library suites and 7 examples, which is every
one on disk. `cargo test --workspace --locked --doc` reports 2 passed.
`cargo fmt --all -- --check`, `cargo check`, `cargo clippy -- -D warnings` at
`--workspace --locked --all-targets`, `shellcheck`, `shfmt -d -i 2 -ci` and
`sh scripts/common/check-gate.sh` all exit 0.

The observer is `crates/bit-ids-probe/src/tracker_udp.rs`, a responder for a
`bit-ids-lab` datagram endpoint. ⭐ The exchange is stateful and that is the
measurement rather than an obstacle: BEP 15 makes a client connect before it
announces, so an announce carrying a connection id this tracker never issued
means the build reused a stale one, invented one, or skipped the connect. Each is
answered with the protocol's own error action and recorded as a refusal with its
reason.

Decision: the connection ids are a contiguous deterministic range, which inverts
`docs/conventions/code.md`'s rule that identifiers come from a cryptographic
random source. The value protects nothing, the only party who can see it is the
build under measurement on a loopback socket, and two runs of one capture have to
produce comparable transcripts; a random id would make every recorded exchange
differ in bytes for no measurement. The range also makes membership arithmetic
rather than a set that grows for as long as a client keeps connecting. The
rejected alternative is a fixed id, which cannot tell a build that echoed what it
was given from one that ignored it.

Guard mutation: 17 defects planted one at a time, each verified to have changed
the file, all 17 refused on the first round. That is the first round in this
session to miss nothing, and the corpus was written against the two lessons the
earlier rounds cost.

Door sweep: two findings, both about one rule enforced in one of two places. The
connection id was read by the codec for an announce and by the observer's own
byte slice for a scrape, so it is now read once, by the codec, for both. And the
datagram list was capped while the refusal list was not, so a build sending
garbage in a loop would have grown the second without limit.

⚠ Claim audit: `Datagram::connection_id` reported BEP 15's magic value as a
connection id for a connect request, which is a number that looks like one. A
test assertion written the other way round is what found it. A connect now
reports none, and `opens_with_protocol_id` is the reader for those bytes.

Driven on 2026-09-05 with a BEP 15 client written from the specification in
Python, independent of the Rust codec in language and in code. It connected and
received `0x6269745f69640001`, announced and received interval 60, seeders 1 and
one compact peer `127.0.0.1:6881`, then announced with an id the tracker never
issued and received action 3 carrying
`connection id was never issued by this tracker`. The observer recorded three
datagrams with the peer ID, `key 0xcafebabe`, `event 2`, `num_want -1` and port
51413 intact.

⛔ Residual, and it is the same one `OBS-02` carries: no stock `BitTorrent`
client has driven this. An independent client written from the same
specification shares this project's reading of it, so it is a weaker control than
`OBS-07`'s stock clients. It cannot be closed on this host:
`sh scripts/acquisition/assert-disposable.sh --egress` exits 1 here with
`a public route exists`, so running a client here would be the capture the
boundary in `docs/capture-host.md` exists to refuse. `CI-03` owns the runner.

## OBS-04: Peer-wire handshake observer

Source: bit-cli centralized peer ID and handshake call-site sweep
Priority: P0 | Effort: L | Status: DONE

Problem: Peer ID, protocol string, info hash, and reserved feature bits must
be captured from bytes emitted by the running client.

Premise: Both incoming and outgoing loopback connections are required because
clients can vary behavior by role.

Approach: Implement active dial and accept roles, preserve handshakes, label
direction, and correlate each stream to its acquisition and torrent manifest.

Prove: `cargo test -p bit-ids-probe -p bit-ids-lab --locked --all-targets`
exercises both roles and rejects a transcript that cannot be rebuilt from raw
bytes.

Closure evidence: run on 2026-09-05. `cargo test -p bit-ids-probe --locked
--all-targets` reports 48 passed and `-p bit-ids-lab` 33 passed, 0 failed.
`cargo test --workspace --locked --all-targets` reports 24 binaries and 238
passed, 0 failed: 12 test files, 4 library suites and 8 examples, which is every
one on disk. `cargo test --workspace --locked --doc` reports 2 passed.
`cargo fmt --all -- --check`, `cargo check`, `cargo clippy -- -D warnings` at
`--workspace --locked --all-targets`, `shellcheck`, `shfmt -d -i 2 -ci` and
`sh scripts/common/check-gate.sh` all exit 0. The whole workspace suite was run
four times in succession with no failure, after one test was found to be
timing-dependent.

⭐ The entry's premise about roles turned out to need a change in `OBS-01`'s
crate, not just a new module. Nothing in `bit-ids-lab` dialled out, and the
door sweep for `OBS-01` had already put `TcpStream::connect` on the list its
own test greps for, so the dial landed in the loopback guard rather than beside
it: `bind::dial` refuses a non-loopback address before the syscall, reads the
peer back afterwards, and bounds the wait, because a connect to an address that
drops rather than refuses outlasts any capture deadline and the lab cannot
interrupt a thread inside a syscall.

Decision: the responder signature grew a `ConnectionId`. One responder serves
every connection an endpoint accepts, so without an identity a peer observer has
nowhere to keep per-connection state and would send a second handshake down the
first connection. The rejected alternative was a responder factory returning a
fresh closure per connection, which hides the identity the journal also needs:
`Segment` now carries the connection, so a transcript of two concurrent peer
connections can be separated back into them.

Guard mutation: 19 defects planted one at a time, each verified to have changed
the file, 18 refused. Two rounds; the first found that the observer's handshake
could be re-sent on every read with every test still passing, because none of
them read again after the handshake exchange.

⚠ The one that is still not refused is `rebuilds_from_raw` returning `true`
unconditionally, and it says something true about that guard. Its `Ok` arm
compares `encode()` against the bytes, which holds for every transcript a
correct codec can produce, so nothing this crate can send makes it false. It is
a codec-regression detector, and planting a lossy `Message::encode` in
`bit-ids-wire` **is** refused, so what would have to be true for the guard to
fire is that the codec became lossy and `FOUND-03`'s own round-trip invariant
stopped catching it first.

Door sweep: three findings on the dial path. A dial on a stopped lab wrote its
opening bytes before the worker's first stop check, so a handshake went on the
wire that nothing was going to answer. A responder taking a `Role` let the
accepting role be attached to a dial, which makes the observer wait for bytes it
was supposed to send, so there are two constructors and no role parameter. And a
connection past the stream cap was left open buffering until the lab's byte cap
fired, which is the same refusal arriving later and less legibly.

⚠ One finding was in a test rather than the code, and it is this session's
second of that shape: the pending-cap test wrote a fixed byte budget and asserted
the close had arrived by then, which is a scheduling outcome it does not control.
It passed alone and failed twice in three loaded workspace runs. It now writes
until the close arrives, with a bound that turns a hang into a failure, and
asserts the deterministic half separately.

Driven on 2026-09-05 with a BEP 3 peer written from the specification in Python,
in both roles at once. `cargo run -p bit-ids-probe --example peer-wire` accepted
one connection and dialled another. The accepted side recorded reserved
`0000000000100005`, peer ID `-qB5000-dialerside01`, and the messages `5(2)`,
`0(0)`, a keep-alive and `254(10)` in that order, keeping the unassigned id with
its payload. The dialled side recorded reserved `0000000000100001` and peer ID
`-LT2090-listenerside`. ⭐ The two roles produced different reserved blocks and
different peer IDs, which is the role dependence this entry exists for. Both
transcripts rebuilt byte for byte.

Residual: the same one `OBS-02` and `OBS-03` carry. No stock `BitTorrent` client
has driven this, and none can on a session host.

## OBS-05: BEP 10 and early-message observer

Source: bit-cli T-234 extended-handshake and message-order inventory
Priority: P0 | Effort: L | Status: DONE

Problem: The BEP 10 client string, extension map, metadata fields, request
sizes, and early message order add identity beyond the initial handshake.

Premise: A minimally responsive synthetic peer can elicit these messages
without exchanging copyrighted payload data.

Approach: Negotiate extensions, record bencoded bytes and ordered messages,
and vary allowed features one at a time.

Prove: `cargo test -p bit-ids-probe --locked --all-targets` validates canonical
fixtures, unknown extension keys, ordering, and size limits.

Closure evidence: run on 2026-09-05.
`cargo test -p bit-ids-probe --locked --all-targets` reports 58 passed, 0 failed.
`cargo test --workspace --locked --all-targets` reports 25 binaries and 248
passed, 0 failed: 13 test files, 4 library suites and 8 examples, which is every
one on disk. `cargo test --workspace --locked --doc` reports 2 passed.
`cargo fmt --all -- --check`, `cargo check`, `cargo clippy -- -D warnings` at
`--workspace --locked --all-targets`, `shellcheck`, `shfmt -d -i 2 -ci` and
`sh scripts/common/check-gate.sh` all exit 0.

BEP 10 lives in the same module as the handshake, because it is the same
connection: one read path for the peer surface rather than two that would each
have to frame it.

⭐ **What the observer offers is a condition of the measurement, and the type
now says so.** A build sends an extended handshake because it was asked for one,
and its map may differ with what it was offered, so the offer is recorded beside
the answer. The reserved block is derived from the same value the extended
handshake is, so a run that says it offered BEP 10 cannot have sent a zero
reserved block.

Decision: three states, not a flag plus an option. `NotOffered`,
`OfferedSilent` and `Offered(handshake)` are the conditions worth running; a
`bool` beside an `Option` makes a fourth that means "offer nothing and send an
extended handshake anyway", which is the observer inventing a negotiation. The
guard-mutation pass found that deleting the guard against that fourth state
changed no test result, and the enum is why it cannot be written now.

Guard mutation: 14 defects planted one at a time, each verified to have changed
the file, all 14 refused. Two rounds; the first found four misses and all four
were real. One was the state above. One was the extended handshake being
re-sent on every read, which is the second time in two entries that a
send-once flag was cleared with nothing noticing. Two were the `v` and `reqq`
keys being dropped from the observer's own offer, which nothing asserted because
the tests only read the `m` map.

Driven on 2026-09-05 with the BEP 3 peer from `OBS-04`, extended to negotiate
BEP 10. The observer offered `0000000000100000`, which is the extension protocol
and neither the DHT nor the fast extension, and sent
`d1:md11:ut_metadatai1e6:ut_pexi2ee4:reqqi250e1:v17:bit-ids-fixture/0e`, sorted
at both levels. The peer answered with a deliberately unsorted map and an
unregistered top-level key, and the observer recorded
`ut_pex=1, ut_metadata=3, lt_donthave=7` **in the order sent**, with
`v` as `qBittorrent/5.0.0` and `reqq` as 500. The dialled connection's peer
offered the protocol and sent no extended handshake, which is reported as such
rather than as an absence of the offer.

Residual: the same one the other three observers carry. No stock `BitTorrent`
client has driven this.

## OBS-06: Adjacent protocol observer suite

Source: bit-cli T-234 MSE and web-seed surfaces
Priority: P1 | Effort: M | Status: DONE

Split on 2026-09-06. The entry named five surfaces at `M`, and three of them are
not `M`: message stream encryption is a Diffie-Hellman exchange with an
obfuscated header and an RC4 keystream, a DHT is a Kademlia RPC surface with its
own routing table, and a web seed is an HTTP server with range requests. Each is
its own `L`. `TODO/RULES.md` says a cross-cutting item should be split before
execution rather than after, so the three moved to `OBS-11` with an acceptance of
their own and nothing was dropped. What is closed here is the containment the
whole suite needs and the two surfaces that are genuinely bounded.

Problem: MSE choices, web-seed HTTP behavior, DHT, PEX, and local discovery
can expose stable client identity outside the core flows.

Premise: These surfaces can be isolated and enabled individually after the
core lab is safe.

Approach: Add one bounded protocol module at a time, each with raw evidence,
a disabled-by-default capability, and an egress-negative test.

Prove: each module's focused Rust suite passes and the lab proves no packet
leaves its allowed address set.

⛔ **The egress guard did not exist and the door sweep is what found it.**
`bind.rs` opened with "every socket this crate creates is created here", and it
was true: a bind and a dial both went through it. Where a socket *sends* went
through nothing. `endpoint::serve_datagram` answered on the bound socket
directly, with `socket.send_to(&send, from)`, and `from` is the source address
the sender wrote on the packet. Nothing verifies a UDP source address, so a
local process able to forge one could have aimed a lab's replies off the host,
out of a socket the loopback guard had already approved.

⚠ That hole was invisible for as long as the sweep's needle list was. The list
named `TcpListener::bind`, `UdpSocket::bind`, `TcpStream::connect` and
`UdpSocket::connect`, and every one of those is a *constructor*. A send is a
method on a socket that already exists, so the whole category was missing rather
than one entry. `.send_to(` is on the list now, spelled as a method call so it
distinguishes `socket.send_to(..)` from `bind::send_to(..)`, which is the door.

⛔ **A `send_to` guard cannot read back and the bind guards can.** A bind reports
`local_addr` and a dial reports `peer_addr`, so both check what was asked for and
then what the kernel did. A datagram socket reports nothing about the packet it
just sent. So the destination is decided before the syscall or not at all, and
`bind::send_to` is the one place that decides.

⭐ **The switch is a value that has to be constructed, not a flag whose default
is false.** `Capability::enable(Surface::LocalDiscovery)` is the only way to make
a `Capability`, and an adjacent observer takes one. A boolean defaulting to
`false` is turned on by a later `..Default::default()` in a struct nobody
re-read; a type with no `Default` and one constructor is turned on only by
somebody writing the line, and the line names the surface.

⚠ **The switch is not the containment and the module says so.** It records that
an operator meant to run the surface. Where the surface sends is `bind::send_to`,
and a module that goes around it is caught by the sweep rather than by the
capability.

⛔ **A second `Surface` enum was written here and it was wrong.**
`bit_ids::observation::Surface` already named `dht`, `pex`, `mse` and `web_seed`,
because a published record has to say which surface a field was observed on. A
copy in the lab would have spelled the same surfaces differently in the
containment layer and in the record, which is the divergent-copies defect
`check-one-home` exists for one layer up. `bit_ids_lab::adjacent` re-exports the
record vocabulary and adds only what the lab knows: which surfaces are adjacent,
how each reaches out, and the switch. `local_discovery` was added to that
vocabulary rather than kept as a fifth private name; nothing has ever been
published, so no consumer held a record whose vocabulary this widens.

⚠ A field path spells a surface in lower snake case and a lab endpoint name is a
`Slug`, which is hyphenated. Two spellings of one thing is how they drift, so
`endpoint_name` is the only converter and a test asserts it equals the field-path
spelling with underscores replaced, for every surface, and that each result
parses as a `Slug`.

⭐ **Local discovery is parsed by the HTTP codec that already exists.** A BEP 14
announce is an HTTP request with a different method, and
`bit_ids_wire::tracker_http::HttpRequest` already preserves header case, header
order and line terminators. A second head parser in the observer would have been
two readings of one grammar, and they would have disagreed first about exactly
the things this observer exists to record.

⛔ **It answers nothing, on any input.** BEP 14 defines no reply. A response
would be this project inventing a protocol and then recording what a client did
when it received one, which is a measurement of this code. The driven run shows
three inbound segments and no outbound one.

⚠ **A refused announce is kept.** `Refusal` says a build sent something BEP 14
does not describe, which is a finding about the build and not a reason to drop
evidence. Every check runs rather than stopping at the first, because the set of
things a build gets wrong is more identifying than whichever one a
short-circuiting reader stopped at.

⭐ **Peer exchange opens no socket at all.** It reads a `Stream` the peer-wire
observer already recorded. That needed `Stream::recorded`, which turns bytes back
into the same reading the live path made; without it an analysis over an evidence
bundle would re-implement the decode and the copy would disagree first about the
partial trailing message.

⛔ **`ut_pex` has no reserved id, and reading it as one would be wrong twice.**
BEP 10 lets the peer choose the id and announce it in the extension map, so the
observer matches on what the peer offered. It also reserves 0 for the handshake
and defines mapping an extension to 0 as disabling it, so `ut_pex: 0` means the
build does not do peer exchange; reading that as "id 0" would attribute every
extended handshake in the stream to `ut_pex`.

⚠ **A refusal variant that nothing could produce was the finding on the first
draft.** `BeforeHandshake` was unreachable: a single forward pass cannot know
message 0 is `ut_pex` until message 1 says so, so a build that gossiped before
announcing its extension map read as one that never gossiped. The read is two
passes now, the variant fires, and `gossip_sent_before_the_extended_handshake_is_attributed_and_reported`
is the case. Deleting the variant would have been the other repair and it would
have deleted the finding with it.

Prove, run on 2026-09-06: `cargo test -p bit-ids-lab -p bit-ids-probe --locked
--all-targets` covers the guard, the switch and both modules; the workspace is 40
binaries and 392 tests. `cargo test -p bit-ids-probe --locked --test
adjacent_surfaces` is 7 cases.

⛔ **The egress claim is made three ways because no one of them is enough.** A
unit test on the guard proves it refuses and not that it is reached. The source
sweep proves nothing went around it and not that it is right. A driven run proves
what crossed a socket and not what a different input would have done. The
addresses used are the ones BEP 14 itself fixes, `239.192.152.143:6771` and
`[ff15::efc0:988f]:6771`, rather than an address chosen to pass. ⚠ And a refusal
is not the same as an inability to send, so the same socket is shown reaching a
loopback destination in the same test file.

Driven on 2026-09-06 by a client that is not this project's test harness.
`cargo run -p bit-ids-probe --example local-discovery -- 6` printed its loopback
address and a Python client sent three announces to it: one exactly as the
specification writes it, one from a build that lower-cases its field names,
reorders them, names two torrents in one announce, sends no cookie and ends with
a bare newline, and one that is not BEP 14 at all. The run reported 3 announces
and 3 segments, all inbound; `Host, Port, Infohash, cookie` and `infohash,
infohash, port, host` as sent; the trailer of the first as `Blank`, the second as
`None`; both info hashes of the second; and two findings on the third, an info
hash that is not forty hexadecimal characters and a port of zero. Then it printed
the guard refusing `239.192.152.143:6771`.

Twenty-three plants over the new guards, each verified to compile first.
Twenty-two refused and one is not, below. ⚠ **Two of the twenty-three survived
the first pass and both were findings rather than harness defects.**

⛔ The first was a mis-named test. `the_reply_path_of_any_datagram_endpoint_is_the_guarded_one`
asserted a loopback echo and nothing about the guard, so reverting
`serve_datagram` to `socket.send_to(..)` left every assertion in it true.
Proving that routing behaviourally would need a datagram arriving with a forged
source address, which needs a raw socket. The plant is refused by
`no_module_outside_the_bind_guard_reaches_the_network`, which reads the source,
and the test is renamed to what it checks with the limit written above it.

⛔ The second is the guard nothing refutes. Removing `.filter(|id| *id != 0)`
from the attribution pass survived, because no candidate message can carry
extended id 0: `is_handshake` is `extended_id == 0`, so an id-0 message always
takes the handshake branch and never reaches that loop. The same mistake is
refuted twice elsewhere, by `offers_peer_exchange` and by the `OfferedDisabled`
refusal, and both of those are plants that failed as they should. The line is
kept as defence against a later change to how a handshake is recognised, and the
source says so where it is written rather than letting it read as a check that
does work.

`check-no-secrets` gained one narrowed allowance on both twins, for the BEP 14
`Infohash:` field, which is a torrent's own identifier and never a credential; it
is anchored to the field name and to exactly forty digits, the way the RFC 3174
vector allowance is.

⛔ **The first version of that allowance was unsound and both twins agreed it was
fine.** `{40}` with no trailing anchor matches the first forty characters of a
longer run and blanks them, and the remainder falls under the twenty-four
character threshold, so a forty-six digit value written after `Infohash: ` was
reported by neither half. ⚠ Comparing the twins is what `FOUND-04` learned to do
per planted input; what found this is the other half of that lesson, which is
that **each planted input needs a declared expected outcome**. Two halves that
agree with each other are not two halves that are right. Seven inputs, each with
an expectation: the field in both spellings passes, a bare forty-hex string is
refused, and so are the field at a different length, the field without its space,
and a token-shaped value sitting beside it.

⛔ This entry put a red `Rust lints` on both CI lanes once. Clippy was clean, a
last line was written into the fixture test while the record was being written,
and `cargo fmt --check` was re-run while clippy was not. `format_collect` fired
on it. It reproduced locally on the first attempt, which is the argument for
running the whole gate after the last edit rather than after the last edit that
felt like the work.

Residual: **nothing here proves no packet left the host.** The three checks above
prove every destination was decided by one guard, that nothing went around it,
and what actually crossed a socket in one run. A capture on the interface is what
would prove the negative, and it needs privileges `docs/capture-host.md` does not
grant a test runner. `CI-03` owns the host that could.

Residual: a datagram responder does not see the source address of the packet it
is answering, so the observer cannot compare the port a build claims in its
`Port:` field with the port the datagram actually came from. That is a real
identity signal and it needs the lab's `DatagramResponder` to carry the peer.
`OBS-11` carries it, because a DHT observer needs the same thing for its own
reasons.

Residual: `Trailer::Other` and `Refusal::NotBtSearch` are reachable and are not
what any real build is expected to send. They are recorded because the announce
is kept whole either way, and a build that sends one is a finding rather than a
parse error.

## OBS-11: Message stream encryption, DHT and web-seed observers

Source: split out of `OBS-06` on 2026-09-06
Priority: P1 | Effort: L | Status: DONE

Problem: `OBS-06` named five adjacent surfaces and closed over two. The three
that remain each carry identity the core flows do not: which cipher and padding
a build offers before anything is readable, how it words a DHT query and what it
puts in its own node id, and what it sends as a user agent and a range header
when it fetches from a web seed.

Premise: The containment is already built and is not per surface. `OBS-06` left
one guarded door for outbound datagrams, one switch type keyed on the record
vocabulary, and a source sweep over both crates. What each of these three needs
is a protocol module and its acceptance, not a new argument about egress.

⛔ **A DHT observer is the one that can leave.** Its first act in a real client
is a query to a bootstrap node this project does not own, and the lab must be
what the build talks to instead. `bind::send_to` refuses every destination
outside loopback, so the work is to make the observer answer well enough that a
build keeps talking, not to add a guard.

⚠ **A web-seed observer needs the torrent to name it.** BEP 19 puts the URL in
the torrent's `url-list`, so `OBS-08`'s `TorrentSpec` has to carry a loopback URL
and the fixture's declared spec has to say so, or the build fetches whatever the
fixture happened to contain.

⚠ **MSE is the one that has to come first in a stream.** It negotiates before any
observer can read the peer wire, so a lab offering it changes what `OBS-04` and
`OBS-05` see. The offer is a condition of the measurement and has to be recorded
beside the result, the way `Offer` already is for BEP 10.

Approach: One module at a time, in the order DHT, web seed, MSE, each with raw
evidence and its own capability, and each with the datagram or stream responder
it needs. Carried over from `OBS-06`: a `DatagramResponder` that sees the source
address of the packet it answers, which local discovery needs to compare a
claimed port against an observed one and a DHT needs to answer a query at all.

Prove: each module's focused Rust suite passes, a driven run by a client that is
not this project's test harness records what it sent, and the lab refuses the
real bootstrap and multicast addresses each protocol names.

### The carried-over prerequisite, done on 2026-09-06

⭐ **A datagram responder is handed the source address of the packet it answers**,
which is what a DHT needs to answer a query at all and what local discovery
needed to compare a claimed port against an observed one.
`bit_ids_lab::endpoint::DatagramResponder` is `Fn(SocketAddr, &[u8]) ->
Option<Vec<u8>>` now, and `LabBuilder::datagram` takes the same shape.

⛔ **The address reaches an observer's record and never a syscall**, which is the
whole argument for handing over an unverified value. Nothing verifies a UDP
source address, so it is target-controlled input exactly like the payload beside
it; what a responder *returns* is addressed by `bind::send_to` and by nothing
else, so the surface that could act on a forged address is still the one door
`OBS-06` closed.

⛔ **A journal segment does not carry a source address, so the live and the
recorded reading of one datagram genuinely differ**, and `local_discovery` says
so with a state rather than papering over it: `PortClaim::NotObserved` is what an
analysis pass over an evidence bundle gets. ⚠ **That is a real limit and not a
finished answer.** A DHT `announce_peer` with `implied_port: 1` publishes a port
that exists only in the packet header, so re-deriving that field from a bundle
would need `bit-ids/transcript/1` widened to carry the source of a datagram
segment. The decision belongs with the DHT module, where the requirement is
concrete, and is taken there rather than here.

⚠ **`PortClaim` describes and never refuses, which is the opposite of what the
name suggests to a reader in a hurry.** BEP 14's `Port` is the peer port a build
listens on and the announce leaves from whatever source port its multicast socket
holds, so `Differs` is the ordinary case for a *conforming* build. Filing it as a
`Refusal` would have filed one against nearly every client this project will
measure, and the acceptance asserts the announce is still conforming in exactly
that case.

⚠ **`tracker_udp`'s responder takes the source and does not read it, on
purpose.** BEP 15's announce carries an `IP address` field whose zero value means
*use the source address of this packet*, so the same comparison is available
there and it is `OBS-03`'s to make: recording it widens that entry's `Announce`
and its acceptance. Named here so the omission reads as a decision.

Prove, run on 2026-09-06: `cargo test -p bit-ids-probe --locked --test
adjacent_surfaces` is 8 cases, one of them
`the_responder_is_handed_the_port_the_datagram_actually_came_from`, which
compares the port the client's own socket reports against the port the lab's
`recv_from` reported. ⭐ **Neither end of that comparison comes from the test.**
Two plants were refused: handing the responder `socket.local_addr()` instead of
the sender's address failed that case alone, and dropping the source inside
`read_from` failed the two unit cases that read it. The workspace is 40 binaries
and 395 tests.

### The DHT codec, done on 2026-09-06

⭐ **`bit_ids_wire::dht` reads BEP 5's KRPC**, so `Surface::Dht` has a codec and
two committed fixtures and is no longer refused with `E-FIX-07`.

⛔ **The module carries the whole decoded document rather than a struct of
extracted fields**, and that is the round-trip invariant rather than a shortcut.
[`../docs/architecture.md`](../docs/architecture.md) section 10 says which fields
that keeps and why named ones could not write them back. A plant that sorted the
dictionary on the way in was refused by the unit case and, independently, by the
fixture corpus.

⛔ **The `v` string is bytes and is never resolved to a client name**, for the
reason `lib.rs` gives about peer-ID prefixes: this crate is the one component
every observer trusts, and `capture-methodology.md` lists a decoder table among
the inputs that may seed a hypothesis and may not populate the catalogue.

⛔ **Nothing is refused except bencode that will not decode.** A message with no
`y`, a `y` that is not `q`, `r` or `e`, a node id of the wrong width, a top-level
list rather than a dictionary: each is a finding about the build and each is
reported by `Message::departures` while the bytes are kept. A codec that refused
a query with no `y` would turn *this build omits `y`* into a parse failure.

⚠ **A `y` of the wrong type and a missing `y` are two observations**, and
collapsing them was one of the four plants. So was dropping the bytes that
followed a dictionary inside one datagram, which are a departure and are written
back rather than tidied away.

⛔ **The `E-FIX-07` negative control had to move, and that is a finding in its
own right.** Two places named `dht` as *the surface with no codec*, which stopped
being true the moment this landed: a control that keeps passing while asserting
the opposite of what it was written for is exactly what a guard-mutation pass
exists to catch. Both name `mse` now, which is still uncovered.

⚠ **"Peer ID" was the wrong name for what the corpus guard reads.** A KRPC
message carries a *node* id; BEP 5 fixes it at the same twenty bytes and
`bit-ids-fixture-0001` is twenty bytes, so it serves as both, but calling them
one thing would be the divergent vocabulary `check-one-home` exists for. The
check is `..._carry_only_the_synthetic_identity_token` now and says which field
each surface offers.

⛔ **And nothing checked a version string until this surface put a `v` on a
second one.** BEP 5's `v` and BEP 10's `v` are both free-form vendor tags, so a
fixture carrying a real one would read as a measurement of that client exactly
the way a peer-ID prefix would. `no_fixture_carries_a_version_string_that_could_name_a_real_build`
holds both to `bit-ids-fixture/0` and asserts it read at least two, because a
sweep that found nothing reports nothing wrong.

⭐ **Driven by `libtorrent` 2.1.1.0, which this project did not write**, from a
virtualenv, reading the two committed fixtures. Both halves matter and the second
is the load-bearing one:

- the conforming fixture has sorted keys and canonical integers, so
  `lt.bencode(lt.bdecode(raw))` is **byte-identical** to what is committed, on
  both frames. Two implementations agreeing beats one agreeing with itself;
- ⛔ the unusual fixture's re-encode is **177 bytes against the 178 sent**.
  `libtorrent` sorts the keys and writes `i1e` where the build wrote `i01e`, so
  it loses exactly the byte and exactly the ordering this codec exists to keep.
  The evidence loss is *shown against a reader that suffers it* rather than
  argued about, and our own re-encode of the same bytes is byte-exact.

⚠ It also reads `implied_port` as `1` from `i01e`, which is the control on our
own reading of the value: keeping the digit text has not changed what the number
means.

⚠ The script is not committed and that follows the tree's existing practice: a
third-party reader needs a package install, so it belongs in an entry's driven
pass rather than in the gate, the way `cbor2` sits in `PUB-03` and `torf` in
`OBS-08`. `scripts/publishing/check-formats.sh` records the same argument.

Prove, run on 2026-09-06: `cargo test -p bit-ids-wire --locked` is 48 unit cases
and 20 corpus cases. Four plants over the codec were each refused by a named
case and none failed to compile: dropping the trailing bytes on encode, sorting
the dictionary on decode, collapsing a wrong-typed `y` into an absent one, and
narrowing `NODE_ID_LEN` to 19. ⚠ The sort plant was **also** refused by the
fixture corpus on its own, checked separately, because a lib failure stops cargo
before the integration binary runs and the first reading credited only the unit
case.

### The DHT observer, done on 2026-09-06

⭐ **`bit_ids_probe::dht` answers BEP 5**, behind `Capability::enable(Surface::Dht)`
like every adjacent surface. It is the first observer on this side that answers
at all: BEP 14 defines no reply so `local_discovery` is silent, BEP 5 defines
several, and a build that queries and hears nothing retries, backs off and stops,
so a silent observer would measure a build talking to a black hole rather than a
build talking to a DHT.

⛔ **A third door was found, and it is not a socket.** A `find_node` or
`get_peers` answer hands the build addresses it will then dial *itself*, so those
packets leave the build's socket and `bind::send_to` is never called on them: a
routable address offered that way reaches the network exactly as surely as one
the lab sent, and every guard this project had was blind to it.
`bind::check_offered` is the guard now, and the three questions are where the lab
listens, where the lab sends, and where the lab tells the target to go.

⛔ **The hazard was already written down, on the wrong surface.**
`adjacent::reaches` said `pex` "hands out peer addresses a client will then
dial" and said nothing of the kind about `dht`, which does the same thing through
a different field while also querying out. A hazard recorded against one surface
but not the sibling that shares it is the one-gated-door defect
`docs/methodology/reviews.md` calls the most recurring hole there is. Both halves
are on the `dht` line now.

⚠ **The guard refuses and never substitutes.** Quietly swapping a routable
address for a loopback one would put bytes in a transcript that the observer
chose, and the record would read as though the build had been offered them.
`OfferedPeers` is a type whose only constructor checks, so a caller cannot append
a routable address after construction, and a list with one good address and one
bad offers nothing rather than silently dropping the bad one.

⭐ **The token is issued and then checked**, the way the UDP tracker's connection
id is. An `announce_peer` carrying a token this observer never issued means the
build reused a stale one, invented one, or skipped the `get_peers`, and each is
answered with BEP 5's own protocol error and recorded with its reason.

⛔ **`implied_port` is what the whole prerequisite was for.** BEP 5 says that when
it is set the `port` argument is ignored and the source port of the packet is
used, so `AnnouncedPort` reports which of the two the build actually announced. A
reading that took `port` regardless would record a number the build explicitly
told it to disregard, and a plant that did exactly that was refused.

⚠ **A guard proved only from another crate is a guard this crate leaves
unproven.** Blanking `check_offered` was refused by two cases in `bit-ids-probe`
and by nothing in `bit-ids-lab`, which owns the rule, so a reader of `bind.rs`
would have found it untested. It has a case beside it now, driven with the
address a real build's default names and with a control that shows the guard can
say yes.

⚠ **A node id one byte wrong is a node id every build reports as malformed.** The
first spelling of the observer's own was twenty-one bytes and read as twenty. It
is a `&[u8; NODE_ID_LEN]` with a compile-time assertion now, so a later edit stops
the build rather than one suite somebody could have skipped.

Prove, run on 2026-09-06: `cargo test -p bit-ids-probe --locked --test
adjacent_surfaces` is 10 cases, two of them the DHT driven run and the
bootstrap-address refusal; the `dht` module is 13 unit cases. Four plants were
each refused by a named case and none failed to compile: `check_offered`
accepting anything, `OfferedPeers::of` skipping the guard, `implied_port` being
ignored, and any token being accepted.

### The web-seed observer, done on 2026-09-06

⭐ **`bit_ids_probe::web_seed` answers BEP 19**, and it is the surface where a
build's identity is not the build's. A `User-Agent` here is usually the HTTP
library's own string rather than the client's, and so are the header order, the
capitalisation and whether an `Accept-Encoding` or a `Connection` header appears
at all. Two clients built on one library look alike here and different on every
other surface, which is a distinction nothing else this project observes can
draw.

⭐ **The torrent names the endpoint, which is what the entry said this needed.**
`TorrentSpec` carries `web_seeds` and the generator writes BEP 19's `url-list`.
⚠ **An empty list writes no key at all**, so a spec that names no web seed
produces exactly the bytes it produced before, and every `capture.fixture` digest
recorded against such a spec is unmoved. A key written as an empty list would
have moved all of them for no measurement.

⛔ **A `url-list` entry is the third door at its most direct**: the torrent tells
the build where to go and the build goes there on its own socket.
`torrent::WebSeed` is the only way to make one and its constructor is
`bind::check_offered`. ⭐ **It holds an address and a path rather than a URL
string**, so there is no URL to parse: a parser that disagreed with the build's
about where the authority ends would approve a URL the build resolves elsewhere.

⛔ **It serves the torrent's own payload.** A seed answering anything else makes
every piece fail its hash check, so the build blacklists the seed and the run
measures a build reacting to a broken server. ⚠ A `Range` gets a `206` and the
exact span, because answering `200` with the whole file is legal HTTP and changes
what the build does next; a range past the end is clamped, which HTTP requires,
and one starting past the end gets `416`, which is the protocol's own answer.

⚠ **A multi-range request and a non-`bytes` unit are recorded and answered as
though no range had been sent**, rather than with an error the build would then
be measured reacting to. A `HEAD` is a finding rather than a refusal: a build
that asks for the length first is doing something many do not.

⭐ **Driven by `curl` 8.5.0, a complete HTTP client nobody here wrote**, over
three fetches against `cargo run -p bit-ids-probe --example web-seed`. It
produced a real measurement rather than only a pass: curl's header order is
`Host, Range, User-Agent, Accept` on a ranged fetch and `Host, User-Agent,
Accept` on a plain one, which is exactly the library-shaped signal this surface
exists to record. The three answers were `206` with 64 bytes and
`Content-Range: bytes 64-127/65536`, `416` over a range starting past the end,
and `200` with all 65536. ⛔ **Both bodies were compared against the torrent's own
payload and matched**, `payload[64..128]` and the whole file, so the seed serves
bytes a build's piece hashes would accept rather than bytes that merely have the
right length.

⛔ **The driven run turned the gate red, and the cause was not this work.**
`synthetic-torrent` defaulted its output path to `synthetic.torrent`, so running
it the obvious way wrote a `.torrent` into the directory `cargo run` was invoked
from, which is the repository root. `check-licences` reads `git ls-files` **and**
the untracked files that are not ignored, so it refuses that file as an artifact
this project may not redistribute, and the next `check-gate` goes red for
something the example silently left behind. The path is required now and the
example exits 2 with the reason. ⚠ **An ignore rule was the other repair and it
is the wrong one**: `.gitignore`'s own header says an ignore is a deletion nobody
notices, and hiding a redistributable-shaped artifact from the check that exists
to find one is worse than the red gate. This is a driven-pass finding in the
strict sense: nothing in the suite could have produced it, because no test runs
that example from the repository root.

Prove, run on 2026-09-06: the `web_seed` module is 11 unit cases and
`a_web_seed_fetch_is_answered_with_the_torrents_own_payload` drives the whole
path over a real TCP connection, through `bind::dial`, asserting the status line,
the `Content-Range`, the served bytes and both journal directions.

### The MSE observer, done on 2026-09-06

⭐ **`bit_ids_wire::mse` and `bit_ids_probe::mse` complete the entry.** MSE has
no BEP; it is the de-facto protocol encryption every major client implements, and
it is obfuscation rather than security. ⭐ **That is what makes it observable:**
the shared secret is unauthenticated, so the lab performs the exchange as the
receiving side and reads what the build offered.

⛔ **It comes first or not at all, which is why the offer is a condition of the
measurement.** A build that encrypts sends its `BitTorrent` handshake inside
`IA`, so its peer ID is not on the wire in the clear and `OBS-04` sees nothing it
recognises. ⭐ **Reading it back out of `IA` is `OBS-04`'s measurement arriving
through a second door**, which is what `SCHEMA-03` calls corroboration: one peer
ID, two observations.

⚠ **The arithmetic is written out and no dependency was added.** MSE fixes the
768-bit MODP group of RFC 2409, so the exchange needs modular exponentiation over
a 768-bit modulus. ⭐ **The modular multiply is double-and-add rather than
schoolbook-with-division**: every step is a shift by one, a compare and a
conditional subtraction, and there is no long division, which is the part of a
bignum that is hardest to get right and hardest to test. `sha1` was added to
`bit-ids-wire`, which moves **no package** in the lockfile because `bit-ids-lab`
already depends on it; the diff is one line.

⛔ **Nothing here is a security primitive and the module says so.** There is no
constant-time discipline because there is nothing to protect: the only party on
the far side is a binary this project installed minutes earlier, on a loopback
socket, and MSE's own threat model does not include the peer.

⭐ **Both primitives are checked against implementations nobody here wrote.**
`tests/mse_arithmetic.rs` compares four 768-bit public keys and a shared secret,
digit for digit, against values Python's arbitrary-precision `pow` computed, and
carries a control on the control: `2^1 mod P` is 2 and `2^0 mod P` is 1 for any
modulus, closed forms no wrong implementation lands on by accident. `RC4` is
checked against RFC 6229's published vector. ⚠ **The offset is the whole
subtlety**: MSE discards 1024 keystream bytes, so a freshly keyed cipher produces
the stream at offset 1024 rather than the offset-zero block, and the constant
pins the discard as well as the cipher. The first draft's comment named the wrong
one; an independent Python `RC4` confirmed both offsets.

⛔ **A mutation pass found a refusal nothing could produce.** Deleting the
verification check entirely left every test passing, because the only case
exercising a wrong key relies on random plaintext, and random plaintext trips the
pad-length check first and reports `Unreadable`. `VerificationFailed` was
therefore unreachable, which is the shape `OBS-06` found in `peer_exchange`'s
`BeforeHandshake`. The case that reaches it is a build that keys its stream
correctly and writes the wrong constant, assembled by hand because `initiate`
writes the right one by construction. Re-planted afterwards and refused.

⚠ **And an assertion that was too narrow was corrected rather than forced.** A
case required `VerificationFailed` over a mismatched torrent; the observer
answered `Unreadable` with `padC is 64448 bytes` and the observer was right, so
the guarantee asserted is now the true one: a wrong key never reads as a
conforming exchange, by whichever route.

Prove, run on 2026-09-06: `cargo test -p bit-ids-wire -p bit-ids-probe --locked`
covers both halves; the wire module is 11 unit cases plus 3 arithmetic controls
and the probe module is 10. Five plants, four refused immediately and the fifth
after the gap above was closed: the `RC4` discard removed, the modular reduction
skipped, the verification never checked, the selection always counted as offered,
and the padding cap not enforced.

### The driven runs, and what they are worth

⭐ **Each of the three was driven by something that is not this project's test
harness**, which is what the entry's Prove asks for:

- **web seed** by `curl` 8.5.0, over three fetches;
- **DHT** by a `libtorrent`-encoded KRPC exchange over a raw socket: a `ping`, a
  `get_peers` that took a token, an `announce_peer` carrying `implied_port`, and
  an `announce_peer` presenting a token the observer never issued. ⭐ **The
  `implied_port` case is the payoff of the whole prerequisite chain**: the
  observer recorded `Implied { observed: 37466, stated: Some(6881) }`, taking the
  source port rather than the number in the message, and the Python driver
  independently reported the same 37466. The forged token was answered with BEP
  5's error 203;
- **MSE** by a complete initiator written from the specification in Python, using
  Python's own `pow`, its own `RC4` and its own `SHA-1` framing. ⭐ **Two
  independent implementations completed one handshake**: the verification
  constant decrypted to eight zero bytes, the selection came back as `RC4`, and
  the observer recorded `padA` 73, `padC` 19, `crypto_provide` `0x3` and the peer
  ID `python-mse-driver001` recovered from inside the encrypted `IA`.

⛔ **None of them is a stock `BitTorrent` client and that limit is unchanged.**
Each is an independent implementation written from a specification, which shares
this project's reading of the protocol and is a weaker control than `OBS-07`'s
stock clients. `OBS-07` needs a host `assert-disposable.sh --egress` does not
refuse, and a session host is refused.

⚠ **The scripts are not committed**, which follows the practice `PUB-03`'s
`cbor2` and `OBS-08`'s `torf` already set: a third-party reader needs a package
install, so it belongs in an entry's driven pass rather than in the gate. What is
committed is the driving surface each one points at, `cargo run -p bit-ids-probe
--example dht`, `--example mse` and `--example web-seed`.

Residual: ⚠ **`mse` and `web_seed` still have no fixture and are still refused
with `E-FIX-07`.** A fixture needs a codec that round-trips a whole transcript
byte for byte, and `mse` has protocol primitives and a partial reader rather than
one: the encrypted section cannot be re-encoded without the key, so a transcript
fixture on that surface would need the run's secret beside it. A web-seed fetch is
an HTTP request the existing codec already round-trips, so a fixture for it is
cheap and belongs in the corpus once a real fetch has been captured.

Residual: ⚠ **`crypto_select` is answered but nothing after it is read.** The
payload stream that follows is `RC4` under `keyB` and this observer stops at the
handshake, because `OBS-04` reads a peer wire and this module reads a
negotiation, and one decoding the other's bytes is how two readings of one stream
disagree. A capture that wants the post-handshake stream needs the two composed,
which is a `CLIENT-*` adapter's job.

Status: DONE on 2026-09-06.

## OBS-07: Known-client positive controls

Source: reference sweep finding that self-consistency can hide observer bugs
Priority: P1 | Effort: M | Status: OPEN

Problem: The first-party observer and normalizer could agree with themselves
while decoding the protocol incorrectly.

Premise: Small independent implementations with declared fixed identities can
serve as positive controls without becoming corpus sources.

Approach: Run aria2 and a stock libtorrent harness plus raw packet decoding;
compare their known emitted bytes to first-party observations.

Prove: mutation tests cause every deliberately altered field to produce a
connector conflict or failed fixture assertion.
