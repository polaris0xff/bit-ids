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
Priority: P0 | Effort: M | Status: OPEN

Problem: A client announces about an info hash and requests pieces of a real
piece layout. Without a torrent the lab can accept a connection and cannot make
a build say anything, so every observer below it has nothing to observe.

Premise: The torrent can be generated rather than committed, and generating it
is what makes it citable. `capture.fixture_digest` in
`crates/bit-ids/src/record.rs` already requires a digest of the fixture a run
used, and a generated torrent whose bytes are a function of its declared inputs
is reproducible from the record. Read rather than measured: no torrent
generator exists in the tree.

Approach: A module in `bit-ids-lab` that builds the info dictionary, the piece
layout and the payload from declared parameters, encodes it with
`bit_ids_wire::bencode` rather than a second encoder, and derives the info hash
from the encoded info dictionary. The payload is generated bytes, never a
copyrighted file. The `.torrent` bytes and the digest the manifest cites come
out of one function, so the digest cannot describe something other than what
the client was handed.

Prove: `cargo test --workspace --locked --test synthetic_torrent` checks that the
generated document round-trips through `bit_ids_wire::bencode`, that the info
hash is the digest of the encoded info dictionary and not of the whole
document, that identical parameters produce identical bytes, and that one
changed parameter changes both the bytes and the digest.

## OBS-09: Raw evidence journal and bundle writer

Source: split out of `OBS-01` on 2026-09-04
Priority: P0 | Effort: M | Status: OPEN

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
Priority: P0 | Effort: L | Status: OPEN

Problem: The BEP 10 client string, extension map, metadata fields, request
sizes, and early message order add identity beyond the initial handshake.

Premise: A minimally responsive synthetic peer can elicit these messages
without exchanging copyrighted payload data.

Approach: Negotiate extensions, record bencoded bytes and ordered messages,
and vary allowed features one at a time.

Prove: `cargo test --workspace --locked --test extended_peer` validates canonical fixtures,
unknown extension keys, ordering, and size limits.

## OBS-06: Adjacent protocol observer suite

Source: bit-cli T-234 MSE and web-seed surfaces
Priority: P1 | Effort: M | Status: OPEN

Problem: MSE choices, web-seed HTTP behavior, DHT, PEX, and local discovery
can expose stable client identity outside the core flows.

Premise: These surfaces can be isolated and enabled individually after the
core lab is safe.

Approach: Add one bounded protocol module at a time, each with raw evidence,
a disabled-by-default capability, and an egress-negative test.

Prove: each module's focused Rust suite passes and the lab proves no packet
leaves its allowed address set.

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
