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
| `bit-ids` crate | public types, schema identity, validation and eventually embedded/pinned catalogue access | capture, installation or network mutation |
| `bit-ids-probe` | active Rust tracker/peer/DHT/web-seed endpoints and raw evidence writing | client launch or package installation |
| acquisition scripts | stable-version discovery, two-route download/install and version equality | identity parsing |
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
| `acquisition` | at least two route records, original URLs, resolver evidence, artifact digest/signature, installed-version evidence | `ACQ-01` |
| `capture` | UTC instant, runner image, kernel, isolation mode, fixture digest, observer and connector versions | `SCHEMA-02` |
| `observations` | typed surface records, each preserving raw bytes/order and a parsed view | `SCHEMA-01` |
| `corroboration` | connector identities, overlapping fields, normalization, outcome and disagreement details | `SCHEMA-03` |
| `evidence` | relative paths plus size and SHA-256 for every raw artifact | `SCHEMA-01` |
| `supersedes` | absent for an original record; the prior record ID for a correction | `SCHEMA-01` |

Three sections carry only what a profile-level invariant needs until the entry
that owns them lands. `acquisition` carries route identity and installed
versions, `capture` carries run identity and the connector list, and
`corroboration` carries the per-field outcome. Every field that exists is read
by the validator; none is a placeholder.

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

## 5. Observation surfaces

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

The gate compares the version reported by the installed executable or harness,
not the requested version. It also records artifact digests. Different bytes
may still represent the same version and become useful packaging observations;
they are never silently collapsed.

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
consume the same artifact assembled once.

## 10. Limits

- There are no measured profiles yet.
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
