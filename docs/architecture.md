# Architecture

This is the technical authority. When another document conflicts with it, that
document is stale.

## 1. Product and unit

The product is an immutable profile for one target, one exact stable version,
one platform/architecture/package, and one capture run. Its schema identifier
is `bit-ids/profile/1`, also exposed by the `bit-ids` crate.

A target may be an application or a library-backed reference harness. A
profile describes observed behavior, not the name a decoder guesses from a
peer-ID prefix.

## 2. Evidence flow

```text
release discovery
      |
      v
two independent acquisition routes
      |  install, then ask each running binary/library for its exact version
      v
same-version gate
      |
      v
disposable isolated host + generated torrent fixture
      |
      +----> Rust active observer: tracker + peer + adjacent protocols
      |
      +----> independent connector: aria2c/libtorrent report or packet oracle
      v
raw evidence bundles
      |
      v
independent parsers + field-by-field correlation
      |             disagreement remains provisional
      v
validated profile
      |
      v
append-only data branch -> deterministic formats -> tagged release
```

No arrow reads identity values from client source code.

## 3. Components

| component | owns | does not own |
| --- | --- | --- |
| `bit-ids` crate | public types, schema identity, validation, stable-version resolution, where a record is filed and what a successor tree may do to it, and eventually embedded/pinned catalogue access | capture, installation, network mutation, and the filesystem: the store rules are pure over a tree a caller has already read |
| `bit-ids-wire` crate | byte-exact codecs for the observed surfaces, and the fixture corpus every observer parses against | sockets, timing, and any mapping from a peer-ID prefix to a client name |
| `bit-ids-lab` crate | the sockets: binding them on loopback and nowhere else, the run deadline, the ordered byte record, endpoint shutdown, the synthetic torrent a capture hands a client, and writing a run out as content-addressed evidence | every protocol, and assembling or publishing a store |
| `bit-ids-probe` | what each surface answers with, and what an exchange was observed to carry, one module per surface | sockets, the run clock, and client launch or package installation |
| acquisition scripts | retrieval: fetching a release listing or artifact and keeping the exact bytes | parsing, ordering or deciding anything, all of which are Rust's |
| client drivers | launch/configure one target against the isolated fixture | deciding whether the observation is valid |
| reference connectors | independent live observation of overlapping fields | filling gaps by inference |
| corpus tool | normalize, correlate, validate, supersede and assemble | modifying an existing published profile |
| workflows | isolate jobs, move checked artifacts between jobs, append/publish with narrow permissions | protocol or schema rules |

Core components are Rust. Shell scripts orchestrate existing binaries and CI.

## 4. Profile model

The record is the `Profile` type in
[`../crates/bit-ids/src/record.rs`](../crates/bit-ids/src/record.rs). The Rust
types are the schema; there is no separate schema document to drift from them.

| section | required content | owned by |
| --- | --- | --- |
| `schema`, `id` | versioned schema and deterministic opaque record identifier | `SCHEMA-01` |
| `target` | canonical ID, display name, kind, edition/engine relationship | `SCHEMA-01` |
| `build` | exact stable version, platform, architecture, package format and executable digest | `SCHEMA-01` |
| `acquisition` | at least two independent route records: kind, resolver, delivery, original URL, immutable source identity, artifact digest and signature, and the evidence of the installed version | `ACQ-01` |
| `capture` | UTC instant, runner image, kernel, isolation mode, fixture digest, which route's install was observed, observer and connector versions | `SCHEMA-02`, `ACQ-03` |
| `observations` | typed surface records, each preserving raw bytes/order and a parsed view | `SCHEMA-01` |
| `corroboration` | connector identities, overlapping fields, normalization, outcome and disagreement details | `SCHEMA-03` |
| `normalizations` | every transformation a comparison applied, declared once | `SCHEMA-03` |
| `evidence` | relative paths plus size and SHA-256 for every raw artifact | `SCHEMA-01` |
| `supersedes` | absent for an original record; the prior record ID for a correction | `SCHEMA-01` |
| `adjudication` | why a correction corrects; absent on an original | `SCHEMA-03` |

One section carries only what a profile-level invariant needs until the entry
that owns it lands: `capture` carries run identity and the connector list, and
`SCHEMA-02`'s manifest carries the rest of the run. Every field that exists is
read by the validator; none is a placeholder.

### Field states

An identity field is in exactly one of six states, and none of them is a null.

| state | means | needs |
| --- | --- | --- |
| `unknown` | nobody has measured it | no value and no evidence |
| `not_observed` | the observer created the condition and the build emitted nothing | a positive control |
| `not_supported` | the build cannot expose the surface at all | a positive control |
| `constant` | every sample carried the same bytes | the bytes and a sample count |
| `patterned` | a fixed span and a varying span, tiling the value exactly | the runs, and at least two samples |
| `variable` | samples differed with no fixed span to report | at least two samples and at least two distinct values |

`unknown` is the only state that asserts nothing, and it is the only one that
may cite no evidence. Every other state is a claim about a build, so a claim
with no recoverable bytes behind it is refused as an unproven field.

An absence is not free. `not_observed` and `not_supported` require an evidence
entry of kind `positive_control`, because an observer that was never listening
and a build that never answered otherwise produce the same record.

### Record identity

`id` is `record:sha256:` followed by the digest of a domain-separated,
length-prefixed encoding of the identity tuple: schema, target, version,
platform, architecture, package and capture. Validation re-derives it and
refuses a record whose declared identifier disagrees with its own contents, so
the identifier cannot drift from the measurement it names. `record:`
distinguishes it from a content digest; both are SHA-256 and only one of them
digests a file.

### Canonical forms

One value has one spelling. Bytes are lowercase hexadecimal, digests carry
their algorithm, instants are `YYYY-MM-DDTHH:MM:SSZ` with no offset and no
fractional part, identifiers are lowercase hyphen-separated, and evidence paths
are relative with no `.` or `..` segment. A non-canonical spelling is refused
rather than normalized: two spellings of one value are two records that differ
in bytes and agree in meaning, which the append-only store cannot survive.

`acquisition`, `capture.connectors`, `observations`, `corroboration` and
`evidence` are each sorted and unique. Order is checked, never imposed: a
validator that sorted on the way in would accept two byte-different files as
one record. Re-reading a record and writing it back reproduces the original
bytes exactly, which is what a rebuilt tree needs in order to be comparable to
the one already published. `PUB-01` owns the tree.

### Reading and writing

[`Profile::from_json`](../crates/bit-ids/src/json.rs) reads a record and
[`Profile::to_json`](../crates/bit-ids/src/json.rs) writes one, and both
validate. `Profile` does not derive `Deserialize`; the derive sits on a private
field mirror and the hand-written implementation validates, so no serde route
produces an unvalidated record either. In-memory construction stays open, which
is what a builder and a test need, and the write path is what catches it.

The schema identifier is read before any other field, so a record from a later
generation is told its version is unsupported rather than that some field is
unknown.

Every refusal carries a stable code, listed in
[`../crates/bit-ids/src/validate.rs`](../crates/bit-ids/src/validate.rs) and
planted against one by one in
[`../crates/bit-ids/tests/profile_schema.rs`](../crates/bit-ids/tests/profile_schema.rs).
`CORPUS-02` extends the set with the invariants that only a whole store can
answer.

### The run manifest

A profile says what a build put on the wire. The run manifest,
`bit-ids/manifest/1` in
[`../crates/bit-ids/src/manifest.rs`](../crates/bit-ids/src/manifest.rs), says
what was running when it did. It is a second document, kept beside the raw
bytes it describes rather than inside the profile, because a replay needs the
whole run and a consumer of the catalogue needs only the record.

| section | required content |
| --- | --- |
| `schema`, `capture` | versioned schema and the capture run this describes |
| `target`, `version`, `platform`, `arch`, `package` | which build |
| `host` | image, kernel, operating system, architecture, and whether the host is destroyed after the run |
| `isolation` | disposable host kind, what the target could reach, and why if that was more than loopback |
| `clocks` | UTC either side of the run, and a monotonic elapsed time |
| `tools` | every implementation that took part, at the version that took part, with its role |
| `acquisition` | per route: where the artifact came from, when, its digest and size, what was done about its signature |
| `phases` | the steps of the section 8 state machine the run actually walked, in order, with the clock either side |
| `evidence` | per artifact: kind, readable path, size, digest, the tool that produced it, the phase it came out of, and whether anything was scrubbed |
| `redactions` | what class of value was removed from which artifact, and how many |

Evidence is content-addressed. The store path is **derived** from the digest,
never recorded beside it, so it cannot disagree with the bytes it names and two
runs that captured identical bytes land on one object.

An absence in the manifest is as constrained as one in the profile. A run that
reached beyond loopback must say why. A host that is not disposable is refused
outright. A phase cannot be skipped, because a phase nobody ran is a phase
nobody can produce evidence for. And an artifact marked redacted must have a
declaration saying what was taken out of it, so that "raw" cannot quietly mean
"edited".

### Agreement, and what it is not

A corroboration entry keeps what **each** connector saw, in
[`../crates/bit-ids/src/agreement.rs`](../crates/bit-ids/src/agreement.rs):
which connector, which artifact the value was read out of, what was applied
before comparing, and the value itself. Choosing one observation and recording
only that would throw away the disagreement along with the evidence of it.

An observation is one of three things. Bytes, an absence the connector actually
looked for, or **out of scope**: this connector cannot see this surface at all.

⛔ **The third one is the whole point.** A field only one connector could see
has nothing disagreeing with it, and calling that agreement is the easiest
mistake in the model. Overlap counts only the observations that were in a
position to see the field, and an outcome of `exact` or `normalized` over fewer
than two of those is refused.

A comparison that needed a transformation names it, and the transformation is
declared once in `normalizations` with two properties it must both have: order
survives it, and bytes it does not understand survive it. A normalization that
discards either cannot be used to reach agreement, which is the rule in
section 6 made checkable rather than remembered.

### Valid is not publishable

These are different questions and the record set needs both.

A disagreement has to be **recordable**. A run whose observers differ moves to
`provisional` with its evidence retained, per section 8, and a schema that
refused to express the conflict would lose exactly the evidence that matters.
So [`validate`](../crates/bit-ids/src/validate.rs) accepts a record carrying a
conflict, and it must.

A disagreement must not be **published**.
[`publishable`](../crates/bit-ids/src/agreement.rs) is the separate gate, and
it refuses two things: a field whose observations disagree, and a field that
asserts a measurement no second connector could see. The second is
`capture-methodology.md`'s provisional-until-a-second-route rule; a record
carrying a provisional field is itself provisional.

A record that supersedes another carries an adjudication: when it was settled,
which of the four causes it was, what was decided, and the evidence behind it.
The causes are the ones a disagreement actually has: a parser defect, a timing
effect, genuine client variability, or the observer perturbing what the target
offered. An original record has nothing to adjudicate and carrying one is
refused.

### Binding the two documents

The manifest and the profile overlap on purpose, and
[`bind`](../crates/bit-ids/src/manifest.rs) compares every value they share:
capture, target, the build tuple, each evidence artifact, each connector and
its version, which tool was the observer, the route set and the artifact
digests, and whether the profile's capture instant falls inside the run.

⛔ **The overlap is the point, and so is the check.** A value in two places with
nothing comparing them is the copy a reader trusts being the wrong one. Pairing
a manifest with the profile of a different run of the same build is the
realistic version of that mistake, and it is refused.

### Where a record is filed

The two documents above are files in an append-only tree, and
[`../crates/bit-ids/src/store.rs`](../crates/bit-ids/src/store.rs) is where a
path comes from and what a successor tree may do to one.
[`publishing.md`](publishing.md) carries the layout; `CORPUS-01` owns the rules.

⛔ **A path is derived from the record's identity tuple, never chosen.** It is
the tuple `RecordId` digests, in full. A path built from fewer components files
two measurements at one name and one of them silently wins; a path built from
more files one measurement under two names, which the append-only rule then
protects both of. The published layout was missing `package` until the
derivation was written and compared against the identifier.

⛔ **A version is a measurement and not an identifier.** `Version` accepts what
the build printed, `../../etc` included, so the store is what refuses a version
that cannot be a path segment. It blocks publication rather than escaping it,
because an escape that is not injective merges two measurements into one
directory and one that is injective needs bytes `RelPath` refuses.

⚠ **Two paths that differ only in case are one file on half of the capture
matrix**, and so is a segment Windows resolves to a device. Those are checked
over a whole tree rather than at derivation, because a tree is read off a disk
somebody else wrote and a segment can arrive without this crate having derived
it. The same rule runs at both doors.

### What only a whole store can answer

[`../crates/bit-ids/src/corpus.rs`](../crates/bit-ids/src/corpus.rs) is the
store-level pass and `CORPUS-02` owns it. Its rules each need more than one
document, which is why none of them sits in the record validator: a reader with
one record cannot know whether the bytes it cites exist.

⛔ **A citation is resolved against the store, never against the other
document.** `bind` above compares the profile and the manifest with each other,
so two documents agreeing about an artifact nobody wrote satisfy it completely.
`E-CRP-03` refuses a citation the store cannot resolve, `E-CRP-04` and
`E-CRP-05` refuse stored bytes that disagree with what the run declared, and
`E-CRP-06` refuses a file in a run's bundle that the run does not declare, which
is the same question asked from the other end.

`E-CRP-01` and `E-CRP-02` pair each record with its run in both directions, and
`E-CRP-07` refuses a correction naming a record the store does not carry.

⚠ **The valid-and-publishable split holds here too.** `validate_corpus` refuses
what must be true of any store, and `publishable_view` separately reports which
records may enter a published view. A store of provisional records is a correct
store, and `CORPUS-03` builds views on that report rather than on a refusal.

### The views a consumer reads instead of the records

[`../crates/bit-ids/src/index.rs`](../crates/bit-ids/src/index.rs) derives the
lookups and the latest view, and `CORPUS-03` owns it. Six lookups: by target, by
measured peer prefix, by measured client string, by platform, by version and by
capture instant, plus one latest row per target, platform, architecture and
package.

⛔ **Derived and never authoritative.** Every row names the record it came from,
so a reader who doubts a row opens the measurement. `rows_resolve` checks each
row against the store rather than against the builder that emitted it, because a
builder compared with its own output agrees with itself.

⛔ **The peer-prefix lookup is section 5's rule inverted, not waived.** No codec
may map a peer-ID prefix to a client name, and this maps a prefix *this project
measured* to the record that measured it. A patterned value whose first span
varies has no prefix and produces no row, because a row keyed on an empty string
answers every lookup.

⚠ **A latest view needs an ordering, and a version string has none.** The
resolver's `VersionScheme` comparison is the project's only one and both callers
share it. A target with no declared scheme, or a version the scheme cannot order,
blocks the view under `E-VIW-01` or `E-VIW-02` rather than being skipped, for the
reason section 7 gives about resolution: a skip yields an older answer delivered
confidently.

⚠ Only publishable records enter a view, and how many were left out is reported
rather than inferred.

⛔ **A corrected record leaves every view and stays in the store.** `CORPUS-04`
owns that, and the two counts are kept apart because they mean opposite things
about a measurement: an excluded record was never publishable, and a superseded
one was, until a later run said it is no longer current. The `corrections` list
carries each superseded identifier, the record that directly corrects it, and
the record at the end of the chain, so a consumer holding an old identifier can
find what answers now without fetching every record to read its `supersedes`.

⚠ **`by` and `current` are two facts.** They differ exactly when a correction
was itself corrected, and a row that stopped at the first step would hand a
reader a record that no longer answers either. The walk is bounded, because a
record identifier digests the identity tuple and not `supersedes`, so two
records can name each other and neither supersedes itself.

⛔ **A fork is refused rather than resolved**, under `E-VIW-03`. Two records
correcting one measurement has no answer to what replaces it, and choosing
between them would publish one adjudication and discard the other in silence.
⚠ Neither that nor the cycle refusal has a counterpart in
[`corpus.rs`](../crates/bit-ids/src/corpus.rs): both are caught when a view is
derived, which is before publication and after a caller who derives none.

## 5. Observation surfaces

The codecs live in
[`../crates/bit-ids-wire/src/`](../crates/bit-ids-wire/src/), one module per
surface, and they hold one invariant: **decode then encode reproduces the input
byte for byte**. Every retention requirement below is destroyed by the
convenient implementation, which is a map of decoded strings, and a round trip
is the cheapest check that catches all of them at once, because a decoder that
dropped a detail has nothing to write back.

⛔ **They observe rather than impose.** Unsorted bencode keys, the non-canonical
integer `i-0e`, a bare `\n` where the grammar says `\r\n`, an unassigned message
id and a non-standard handshake protocol string are all recorded rather than
refused: each is a difference between builds, and a decoder that refused one
would turn an observation into a parse failure. A byte-string length prefix with
a leading zero is the one deviation refused instead, because it is an artefact
of an encoder's integer formatter rather than a value the build chose.

⛔ **No codec maps a peer-ID prefix, a user agent or a BEP 10 `v` string to a
client name.** `capture-methodology.md` lists a decoder table among the inputs
that may seed a hypothesis and may not populate the catalogue, and a codec that
answered "this is client X" would put that refused input inside the one
component every observer trusts.

### The fixture corpus

[`../crates/bit-ids-wire/tests/fixtures/`](../crates/bit-ids-wire/tests/fixtures/)
holds byte-exact synthetic transcripts, `bit-ids/wire-fixture/1`, one per file
with its own provenance. `FOUND-03` owns it and
[its README](../crates/bit-ids-wire/tests/fixtures/README.md) says what each one
proves survives a decode.

A fixture exists because a live capture cannot separate an observer regression
from a client behaviour change: both arrive as "the parse looks different this
week", and both of its inputs moved. Fixture bytes provably did not, so a parse
that changed against one is the parser.

⛔ **A fixture is never evidence.** Its origin is `synthetic`, there is no
`captured` origin to blur that, and every fixture carries the peer ID
`bit-ids-fixture-0001`, checked by pulling it out through the codec rather than
by scanning the bytes. Every frame is bytes the target emitted; the observer's
own replies prove nothing about a build and are not fixture material.

`index.json` carries the digest of each fixture and of the corpus, derived the
same domain-separated, length-prefixed way as a record identifier. That is what
makes "the digests are identical across two runs" something the suite asserts
rather than something a person compares by eye.

### Tracker HTTP

Keep the raw request line and header block before parsing. The parsed view
retains query and header order, duplicate fields, percent-encoding hex case,
the 20 peer-ID bytes, key shape/lifetime, `numwant`, compact/no-peer-ID flags,
event behavior and address-family extensions.

### Tracker UDP

Keep datagrams in order with direction and monotonic timestamps. Parse connect,
announce and scrape actions, transaction IDs, keys, events, `num_want`, address
family and retry cadence.

### Peer wire

Keep the complete handshake and the bounded initial message transcript. Parse
the peer ID as bytes, all eight reserved bytes, extension negotiation, the raw
BEP 10 dictionary, extension IDs, advertised client string, request queue,
port/encryption/upload fields and early message order.

### Adjacent protocols

DHT, PEX, MSE and web seed are separate optional surfaces. Absence is recorded
only after a positive control proves the observer could see the surface.

### The lab a client is pointed at

Something has to hold the sockets those codecs read from, and
[`../crates/bit-ids-lab/`](../crates/bit-ids-lab/) is it. `OBS-01` owns it.

⛔ **Every socket the lab creates is created by one function**, in
[`../crates/bit-ids-lab/src/bind.rs`](../crates/bit-ids-lab/src/bind.rs), which
refuses any address that is not loopback before the syscall and reads the
address back off the socket afterwards. Those are two facts, not one: a bind
request is what was asked for and `local_addr` is what the kernel gave. A second
place calling `TcpListener::bind` would be a gate on one of two doors into the
same action, so a test greps this crate's own source for that rather than
leaving the rule as a comment.

The port is always zero, so the operating system chooses. Two labs on one host
that both named a port collide in whichever one starts second, which reads as a
flake rather than as the configuration error it is.

A lab holds one deadline and stops itself when it passes, rather than a timeout
per read: a client that connects and then says nothing would otherwise hold a
run open until the CI job's own timeout, which reports as infrastructure. Whether
the deadline ended the run is recorded, because a lab that ran out of time and
one that was told to stop leave the same empty journal.

The journal is the ordered record of every byte each endpoint moved, with the
direction it travelled. ⛔ **The order is the sequence the segments were appended
in, under one lock, and not their millisecond offsets.** Endpoints run on their
own threads and two segments can share a millisecond, so ordering by offset can
put a reply before the request that caused it.

⚠ **A stream segment is what the observer read, not provably what the target
wrote.** Section 5 keeps write segmentation because a handshake and a bitfield in
one write is a different observation from the same bytes in two, and TCP
preserves no write boundaries to recover. One segment per read is the closest an
observer gets. The read buffer is larger than any message these surfaces carry,
so no message is split by the buffer size; a burst larger than the buffer still
spans two segments and nothing in the bytes distinguishes that from two writes.
A datagram has no such gap, and its buffer is above the largest a host can
deliver, so `recv_from` never reports a truncated packet as a whole one.

The lab dials as well as accepts, through the same guard. ⭐ `OBS-04` exists
because a build can behave differently as the side that dialled and the side that
accepted, so a lab that could only accept would measure half of the peer surface.
The dialling side speaks first, so a dial carries opening bytes: a responder is
only called once something has arrived, and in that role nothing will until the
observer has introduced itself.

⚠ **A responder is told which connection it is serving.** One responder serves
every connection an endpoint accepts, so without an identity a peer observer has
nowhere to keep per-connection state and sends a second handshake down the first
connection. The journal carries the connection too, so a transcript of two
concurrent peer connections separates back into them. A datagram has none.

⭐ **A datagram responder is told the source address instead, because some
surfaces are that address.** BEP 5's `announce_peer` carries `implied_port`,
whose meaning is *use the source port of this packet rather than the port in this
message*, so an observer blind to the source cannot record what port the build
announced; BEP 14's claimed `Port` beside the port an announce actually arrived
from is the weaker form of the same measurement. `OBS-06` named the need and
`OBS-11` is what required it.

⛔ **The address reaches the record and never a syscall.** A UDP source address
is a claim by the sender that nothing verified, so it is target-controlled input
like the payload beside it. What a responder returns is addressed by
[`bind::send_to`](../crates/bit-ids-lab/src/bind.rs) and by nothing else, which
is what makes handing the same unverified value to an observer safe.

⚠ **A journal segment does not carry it, so the live and recorded readings of one
datagram differ on purpose.** A transcript holds the endpoint, the connection,
the offset, the direction and the bytes; an analysis pass over an evidence bundle
therefore has no source to compare against and says so, rather than inventing
one. A surface whose published field *is* the source port would need the
transcript widened first, and `OBS-11` carries that as its own question rather
than assuming either answer.

⛔ **The lab speaks no protocol.** The observers supply a responder per surface.
That is what lets one deadline, one loopback guard and one journal serve every
surface instead of each observer growing its own, and it is why the codecs in
section 5 have no sockets in them.

### Where the lab may send, and which surfaces are switched on

⛔ **A bind guard is half the containment, and for a while it was the whole of
it.** Where a socket listens and where it sends are different questions, and the
second one had no answer: a datagram endpoint replied on its bound socket
directly, to an address the sender had chosen.
[`../TODO/observer.md`](../TODO/observer.md) carries what that made possible.
`bind::send_to` is the one place an outbound datagram is addressed now, and it
refuses any destination outside loopback.

⚠ **That guard checks before the syscall and cannot check after.** A bind reports
`local_addr` and a dial reports `peer_addr`, so both read back what the kernel
did. A datagram socket reports nothing about the packet it just sent. There is no
after-the-fact reading of where a datagram went, so the destination is decided in
one place or nowhere.

⚠ **The sweep's needle list was the reason the hole lasted.** Every needle on it
named a constructor, and a send is a method on a socket that already exists, so
the whole category was missing rather than one entry.

Some surfaces a client reaches for without being asked: it multicasts a local
discovery announce, gossips peers over `ut_pex`, and queries a DHT bootstrap
node. Each carries identity, and each also names a destination of its own, which
is what makes them different from the core surfaces a client only ever reaches
where the lab pointed it. `OBS-06` calls them adjacent, and
[`../crates/bit-ids-lab/src/adjacent.rs`](../crates/bit-ids-lab/src/adjacent.rs)
holds the switch each one is behind.

⭐ **The switch is a value, not a flag.** `Capability::enable(surface)` is the
only way to make a `Capability`, and an adjacent observer takes one. A boolean
defaulting to false is turned on by a later `..Default::default()` in a struct
nobody re-read; a type with no `Default` and one constructor is turned on only by
somebody writing the line, and the line names the surface. ⚠ The switch records
that an operator meant to run the surface. It says nothing about where the
surface sends, which is the guard's job.

⛔ **The surface vocabulary has one home and it is the record's.**
`bit_ids::observation::Surface` already named `dht`, `pex`, `mse` and `web_seed`,
because a published field says which surface it was observed on. The lab
re-exports it and adds only what the lab knows. ⚠ A field path spells a surface
in lower snake case and a lab endpoint name is a `Slug`, which is hyphenated, so
`adjacent::endpoint_name` is the only converter and a test holds the two
spellings together.

### The torrent a client is handed

A client announces about an info hash and asks for pieces of a real piece
layout, so the lab also generates the torrent in
[`../crates/bit-ids-lab/src/torrent.rs`](../crates/bit-ids-lab/src/torrent.rs).
`OBS-08` owns it. The payload is generated bytes and never a copyrighted file.

⭐ **It is generated rather than committed because that is what makes it
citable.** `capture.fixture` is a digest of the metainfo a run used, and a
torrent whose bytes are a function of its declared spec can be rebuilt from the
record and compared against that digest. A committed file could only be trusted.

⛔ **The payload's byte stream is therefore part of the published contract.**
Changing the arithmetic invalidates every fixture digest already recorded, and
it does so silently: the properties a generator is naturally tested for, being
reproducible, seed-dependent and stable under a longer request, all survive any
change to it. The stream is `SplitMix64`, stated in full where it is
implemented, and the acceptance suite compares it against that specification
restated independently and anchored to the published reference's own output.
Changing it means superseding the records that cite it.

⚠ **Two digests of two different things, and confusing them is the trap.** The
info hash is SHA-1 of the **encoded info dictionary**, fixed by BEP 3 and the
one algorithm here this project does not choose; it is the value a client
announces. `capture.fixture` is SHA-256 of the **whole metainfo file**, which is
this project's own canonical digest of the artefact. Different algorithms over
different byte ranges, and a record citing one where it meant the other would
name a torrent nobody used. The `.torrent` bytes and both digests come out of
one function, so a digest cannot describe something other than what the client
was handed.

### What a run leaves on disk

A profile cites evidence and a manifest describes it, and something has to write
it. [`../crates/bit-ids-lab/src/evidence.rs`](../crates/bit-ids-lab/src/evidence.rs)
is that, and `OBS-09` owns it. One artifact per endpoint, each a
`bit-ids/transcript/1` document carrying every segment's bytes, direction,
connection and offset, plus the `EvidenceRecord` rows a manifest carries. The
shape is section 4's and is not invented here.

⛔ **The digest is of the file, and the file is compared against the buffer.** A
writer that digests what it meant to write cannot detect a short write, and the
manifest would then be internally consistent and describe an artifact nobody
has. Reading it back closes half of that and only half, because a truncated file
digests to a value that matches itself. One comparison serves both the write and
a later verification, so the guard that cannot be provoked at write time is
proved by the caller that can be.

⛔ **A path is gated on where it resolves, not only on how it is spelled.** The
canonical form already refuses `..`, a leading separator and a backslash, and
those are rules about text: a symlink in a reused bundle root satisfies every
one and still lands the artifact outside, with the manifest citing a path that
reads as inside. The root is resolved once and every artifact's directory must
resolve under it.

⛔ **A transcript is never scrubbed.** The bytes a build put on the wire are the
measurement, and editing one changes the identity being measured; a peer ID is
exactly the sort of high-entropy token a scrubber reaches for. Scrubbing belongs
to text a host produced, where every removal is declared with its count so that
`raw` cannot quietly mean `edited`. ⚠ The scrubber replaces what the caller
names rather than guessing: a capture harness knows its own hostname, account
and variables, and only an IPv4 address has a shape worth matching on by itself.

### The observers

[`../crates/bit-ids-probe/`](../crates/bit-ids-probe/) holds one module per
surface. Each is a responder handed to a lab endpoint: it decodes with the
section 5 codec, keeps what arrived, and answers.

⚠ **What an observer answers is part of the experiment, not a detail.** A client
that receives the wrong shape of tracker response reports an error and changes
what it does next, and that change would be recorded as identity when it is the
observer's doing. So the HTTP tracker observer reads `compact` and `no_peer_id`
out of the announce and answers the shape that was asked for. Where a request
leaves the choice open, the observer's choice is recorded as a condition of the
run rather than treated as a default.

⛔ **An observer frames requests with the codec's own framer and never a second
one.** [`tracker_http::head_end`](../crates/bit-ids-wire/src/tracker_http.rs) is
that framer. A framer that disagreed with the decoder would answer one request
while the decoder read another, and both halves would look correct alone.

A request the codec refuses is answered with a bencoded failure and is not kept
as an observation, because a head that did not decode is not an announce. The
bytes are not lost: the lab recorded them before the responder saw them, and
that is what the suite asserts rather than assuming.

⭐ **What an observer offers is a condition of the measurement, not a setting.**
A build sends a BEP 10 extended handshake because it was asked for one, and what
it puts in its extension map may differ with what it was offered. So the offer is
recorded beside the answer, and the reserved block is derived from the same value
the extended handshake is: a run that says it offered the extension protocol
cannot have sent a zero reserved block.

⛔ **Offering nothing and negotiating anyway is not a state the observer can
hold.** The conditions worth running are not offered, offered and silent, and
offered with a handshake; a flag beside an optional handshake makes a fourth that
means the observer invented a negotiation, and whatever the build did about that
would be recorded as identity.

⚠ **What an observer keeps is bounded and the overflow is counted.** The lab's
deadline bounds how long a target can talk and not how fast, so a build that
announces in a loop would otherwise grow the record until the host runs out of
memory. A record that kept a cap's worth with nothing saying how many there were
is a measurement with no denominator. ⛔ Every list an observer grows is bounded
by that cap, not only the obvious one: the UDP observer's refusal list was
unbounded while its datagram list was capped, which is a bound on one of two
lists that grow together.

The UDP exchange is stateful, and that is an observation rather than an
obstacle. BEP 15 makes a client connect before it announces, and an announce
carrying a connection id the observer never issued means the build reused a
stale one, invented one, or skipped the connect. So the id is checked and a
mismatch is answered with the protocol's own error action.

⚠ **The connection ids are a contiguous deterministic range**, which inverts the
rule in [`conventions/code.md`](conventions/code.md) that identifiers come from a
cryptographic random source. The value protects nothing, the only party who can
see it is the build under measurement on a loopback socket, and two runs of one
capture have to produce comparable transcripts. A random id would make every
recorded exchange differ in bytes for no measurement.

⚠ **A connect request carries no connection id.** Its first eight bytes are BEP
15's magic value, and reporting those as a connection id hands a caller a number
that looks like one. The codec returns nothing for that case and
`opens_with_protocol_id` is the reader for those bytes.

## 6. Connector contract

The primary connector is the project-owned Rust active observer. It terminates
or initiates the local protocol interactions needed to make the target expose
its behavior and writes byte-exact evidence.

The independent connector is selected per surface:

- `aria2c` JSON-RPC or another stock CLI peer for peer identity and live peer
  behavior;
- a stock libtorrent alert client when it exposes a field aria2c does not;
- `tshark`/`dumpcap`, or a platform packet oracle with equivalent raw output,
  for wire-byte corroboration.

At least two connectors participate in every capture. Each overlapping field
is `exact`, `normalized`, `disagrees` or `not_corroborated`. Only the first two
are publishable. A normalization is named and tested; it cannot discard order
or unknown bytes merely to obtain agreement.

## 7. Acquisition equivalence

A route is distinct when its resolver and delivery mechanism are independent,
for example a system package manager and a vendor release URL. Two package
manager aliases pointing at the same manifest are one route.

⛔ **That rule is enforced rather than remembered.** A route in
[`../crates/bit-ids/src/acquisition.rs`](../crates/bit-ids/src/acquisition.rs)
records what resolved it and what delivered it as two separate values, and
`E-ACQ-07` and `E-ACQ-08` refuse a record whose routes share either. Counting
routes without that check leaves the two-route rule satisfiable by asking one
index twice under two names, which is the failure it exists to prevent.

A route also names the **immutable identity** of what it asked for, typed to the
kind: a release asset is a repository, a tag and a file name; a package is an
index, a name and an exact version; a source build is a full commit. `E-ACQ-05`
refuses a kind carrying another kind's identity, and `E-ACQ-06` refuses an
abbreviated commit, which is the shape `FOUND-02` measured passing an action-pin
rule written to refuse floating refs.

### Choosing which version to acquire

Before any of that, something has to decide what the newest stable release *is*,
and [`../crates/bit-ids/src/resolution.rs`](../crates/bit-ids/src/resolution.rs)
is where that happens. `ACQ-02` owns it and `bit-ids/resolution/1` is the
document it writes.

⛔ **Version strings are not comparable in general.** Sorting tags as text puts
`4.1.10` before `4.1.9`. A channel label is no better: a project can publish a
preview without setting the flag. So a target declares how it spells versions,
the resolver compares only what that scheme can order, and a candidate it cannot
order **blocks the resolution** instead of being skipped. Skipping is the
dangerous case: it yields an older version, selected confidently, with nothing
saying a newer one was seen and not understood.

The one thing that settles an unorderable candidate is a second signal rather
than a guess. A candidate published strictly before the winner cannot be the
newest whatever its tag says, so it is recorded as predating the selection. With
no publication date, or one at or after the winner's, it still blocks.

Stability is judged by both signals and pessimistically: anything the source or
the version text calls a prerelease is not stable. Being wrong that way costs a
skipped release. Being wrong the other way publishes a preview build as the
stable one, which is a measurement about a build nobody runs.

The resolution keeps **every** candidate with the verdict it got, the exact
bytes each source answered with, and the instant the decision was made. A
selection nobody can re-derive is a claim, not a measurement.

The gate compares the version reported by the installed executable or harness,
not the requested version. It also records artifact digests. Different bytes
may still represent the same version and become useful packaging observations;
they are never silently collapsed.

### Keeping the artifact, and what may be kept

An upstream URL moves and a package index drops an old version, so an artifact
this project measured may stop being retrievable from where it came from.
[`cache`](../crates/bit-ids/src/cache.rs) is what survives that, and `ACQ-05`
owns it.

⛔ **The identity is the digest and never the location.** A retrieval from a new
URL is recorded against the artifact the digest already names, so nothing has to
be reproduced when a source moves: the lookup that finds it never mentions a URL.
⚠ A repeated retrieval adds nothing, because a cache that grew a row per
re-run would report a popularity contest rather than a provenance.

⛔ **Keeping the bytes is a permission and not a capability.** `FOUND-04`'s
register in [`../catalogue/licences.toml`](../catalogue/licences.toml) carries a
disposition per target, `E-CAC-01` refuses stored bytes where it says refused,
and `E-CAC-02` refuses a target it does not mention at all rather than defaulting
to permitted. ⚠ Every row says refused today, so the cache stores nothing and
keeps the digest, the size, the signature status and every place the bytes were
found.

⚠ The disposition is passed in and never read from a file by this crate. Its
rules are pure over data a caller has already read, and the register has one
parser: `check-licences`, which reports the permitted set through `--permitted`.

### What equal version labels are worth

`E-ACQ-04` already forces every route to report the version the record declares,
so by the time anything compares them the labels agree. That is the easy half
and worth almost nothing alone: a distribution can patch a build and keep the
upstream version string.
[`../crates/bit-ids/src/equivalence.rs`](../crates/bit-ids/src/equivalence.rs)
is what decides whether the labels are backed by anything, and `ACQ-03` owns it.

⛔ **A run observes one installed build.** The record says which, in
`capture.observed_route`, and `build.executable` must be that route's install.
Without it a reader assumes both routes were watched, when only one was. The
executable digest is recorded **per route** for the same reason.

Four outcomes, and only two publish:

| outcome | means | publishes |
| --- | --- | --- |
| `byte_identical` | every route installed the same executable bytes, so there is one build and observing one observed it | yes |
| `build_equivalent` | the installs differ and each was observed in its own capture, with no overlapping field disagreeing | yes |
| `divergent` | equal labels over evidence that conflicts | no |
| `unresolved` | not enough evidence to say | no |

⚠ **A single capture of two byte-different installs is `unresolved`, not
`build_equivalent`.** Nothing put the other bytes on the wire, so nothing can
say they behave the same, and calling it equivalent would publish a claim about
a build this project never ran. Reaching `build_equivalent` needs a capture per
route, which is what `classify_across` compares.

The two refusals are separate codes because the fix differs: a divergence needs
adjudicating, and an unresolved record needs a second capture through the other
route.

⭐ **An installed version is a measurement, so it cites evidence like one.** A
route records the command it ran to ask, and the evidence entry holding what the
build answered. `E-ACQ-09` refuses a citation that resolves to nothing and
`E-ACQ-10` refuses one that resolves to anything but process output: a version
read out of a packet capture or a filename is not the build speaking.

The signature disposition is recorded in both documents and `E-BND-13` compares
them, because nothing else forces them into step and a record publishing
`verified` over a run that says `not_checked` is the publishable half of a claim
nobody made. `ACQ-05` owns the authenticity evidence behind the status.

⚠ `candidate_routes` in [`../catalogue/clients.toml`](../catalogue/clients.toml)
is a research lead and never an availability claim. Nothing reads it at run
time; one test reads it as a vocabulary to keep in step with `RouteKind`.

For libraries, the two routes are normally a language package registry/module
proxy and the matching upstream Git tag. Both build the same committed harness
and must report the same library version.

## 8. State machines

### Capture state

```text
planned -> resolved -> acquired-twice -> installed -> observed
        -> corroborated -> validated -> published
```

Any mismatch moves to `provisional`, with evidence retained. It does not skip
forward. A new stable release creates a new planned record; it never edits the
old one.

### Published state

```text
new record -> append commit on data -> included in later aggregates/releases
correction -> new record with supersedes -> append commit
```

There is no update or delete transition.

## 9. Branches and permissions

`main` contains source, schemas, target definitions, documentation and
workflows. `data` contains only the assembled publication tree. Capture matrix
lanes have `contents: read`; they upload artifacts. A collector validates and
assembles. A separate publisher with job-scoped `contents: write` appends a
commit without force and reads the branch back.

A pushed `v1.*` tag is the only release trigger. Releases and the data branch
consume the same artifact assembled once, and
[`../crates/bit-ids/src/release.rs`](../crates/bit-ids/src/release.rs) is what
assembles it. `PUB-01` owns it.

⛔ **Two documents describe the bundle and they cover different sets.** A
document cannot state its own digest, so `MANIFEST.json` describes everything
except itself and `SHA256SUMS`, and `SHA256SUMS` covers everything except
itself. Between them every published byte is covered exactly once.

⚠ **A media type is looked up and never defaulted.** An extension this build
does not know blocks the assembly rather than shipping as opaque bytes, because
a consumer handed the wrong type mis-parses silently and the failure lands on
the reader.

⛔ **The append rule is checked before the push, not trusted to the push.** A
branch protection setting refuses a force-push and says nothing about a commit
that deletes a file, which is the shape a latest-only regeneration takes. The
comparison in [`section 4`](#where-a-record-is-filed) is what refuses that, and
[`../scripts/publishing/publish-data.sh`](../scripts/publishing/publish-data.sh)
runs it over the fetched branch before it commits anything. `PUB-02` owns it.

⛔ **The rule is about the canonical roots and not about every byte.**
`profiles/` and `raw/` carry measurements and never change; the manifest, the
checksums and the generated indexes exist in order to change, and an index that
did not move when a record was appended would be one that had stopped describing
the store. `store::CANONICAL_ROOTS` is the list.

⚠ A push is not completion. The branch is fetched again, compared against what
was there, and every digest in its own `SHA256SUMS` verified, before the run
reports that it happened.

## 10. Limits

- There are no measured profiles yet. The observer layer is complete: all four
  core surfaces have observers, the lab generates the torrent that makes a
  client announce about anything at all, and a run's transcript becomes the
  content-addressed evidence a manifest cites. There is now somewhere durable to
  put a record as well. What stands between here and a first capture is a client
  adapter and a host the [`capture-host.md`](capture-host.md) guards permit.
- ⚠ Nothing assembles a store yet. `PUB-01` builds the publication tree and
  `PUB-02` runs the append comparison over the fetched `data` branch; until both
  exist, the store and corpus checks are pointed at directories by hand or at
  one written by `build-store`.
- ⚠ No observer has been driven by a stock `BitTorrent` client. Each was driven
  by an independent client written from the specification, which is a weaker
  control: it shares this project's reading of the protocol. `OBS-07` owns the
  stock-client positive controls, and they need a host the
  [`capture-host.md`](capture-host.md) guards permit, which the session host
  this was written on is not.
- ⛔ A Windows capture is not permitted. The disposable-host guards in
  [`capture-host.md`](capture-host.md) read `/proc/net/route` and
  `/etc/machine-id`, so there is no boundary to run before an install on
  Windows. `CI-03` owns the pair.
- `dht`, `pex`, `mse` and `web_seed` have no codec in `bit-ids-wire` and no
  fixture. A fixture on one of those surfaces is refused with `E-FIX-07` rather
  than silently passing. `OBS-11` owns the three that need one; `pex` rides
  inside a peer-wire transcript, so a fixture for it is a `peer_wire` fixture and
  `OBS-06` reads it there. ⚠ `local_discovery` has a codec now, because a BEP 14
  announce is read by the HTTP codec that already existed, and no fixture: one
  belongs in the corpus when a real announce has been captured.
- Windows packet corroboration needs a route that works on hosted runners or a
  disposable self-hosted runner; `OBS-07` owns the independent-control
  decision and `CI-03` owns the runner.
- Proprietary clients may have only one legitimate acquisition route. Such a
  target remains open until a second independent route is verified; the rule
  is not weakened.
- A client may randomize identity per torrent, session or connection. Sample
  counts and lifetimes are measured rather than assumed from one run.
- Market-share data prioritizes captures but is not evidence about any
  identity field.
