# Observer entries

All observer services bind only to disposable local or isolated laboratory
interfaces. Each stores exact received bytes before parsing them.

## OBS-01: Isolated Rust loopback observation lab

Source: operator active-probing rule and bit-cli loopback architecture
Priority: P0 | Effort: XL | Status: OPEN

Problem: Client adapters need a deterministic fake torrent environment that
never depends on public trackers, DHT, or peers.

Premise: A local tracker, seeder, peer, DNS/TLS endpoint, and orchestration
boundary can drive all core surfaces while blocking escape traffic.

Approach: Build a Rust lab supervisor with ephemeral endpoints, synthetic
torrents, strict egress assertions, timeouts, raw event journals, and cleanup.
Split transport services into follow-up entries before implementation if the
acceptance cannot remain atomic.

Prove: integration tests run a known client fixture with networking denied
outside the lab and produce identical normalized events on Linux and Windows.

## OBS-02: HTTP tracker observer

Source: bit-cli T-234 and tracker request-order tests
Priority: P0 | Effort: L | Status: OPEN

Problem: HTTP announces expose peer ID, user agent, header set, query order,
encoding, key, numwant, and event behavior that a peer-ID table omits.

Premise: A byte-preserving HTTP endpoint can observe these fields without
guessing how a client constructed them.

Approach: Capture request line and headers before normalization, return valid
tracker responses, and repeat lifecycle events under controlled torrents.

Prove: `cargo test --workspace http_tracker` checks raw ordering, binary query
values, repeated requests, and malformed input behavior.

## OBS-03: UDP tracker observer

Source: bit-cli T-234 UDP key and numwant inventory
Priority: P0 | Effort: L | Status: OPEN

Problem: UDP announces carry identity-adjacent fields and binary layout not
visible to the HTTP observer.

Premise: A local BEP 15 responder can drive connect and announce transactions
and preserve every datagram.

Approach: Implement strict transaction matching, deterministic responses,
packet capture, and parsed views with no lossy string conversion.

Prove: `cargo test --workspace udp_tracker` covers connect, announce, timeout,
retry, key, event, numwant, and rejection cases.

## OBS-04: Peer-wire handshake observer

Source: bit-cli centralized peer ID and handshake call-site sweep
Priority: P0 | Effort: L | Status: OPEN

Problem: Peer ID, protocol string, info hash, and reserved feature bits must
be captured from bytes emitted by the running client.

Premise: Both incoming and outgoing loopback connections are required because
clients can vary behavior by role.

Approach: Implement active dial and accept roles, preserve handshakes, label
direction, and correlate each stream to its acquisition and torrent manifest.

Prove: `cargo test --workspace peer_wire` exercises both roles and rejects a
profile whose normalized handshake cannot be rebuilt from raw bytes.

## OBS-05: BEP 10 and early-message observer

Source: bit-cli T-234 extended-handshake and message-order inventory
Priority: P0 | Effort: L | Status: OPEN

Problem: The BEP 10 client string, extension map, metadata fields, request
sizes, and early message order add identity beyond the initial handshake.

Premise: A minimally responsive synthetic peer can elicit these messages
without exchanging copyrighted payload data.

Approach: Negotiate extensions, record bencoded bytes and ordered messages,
and vary allowed features one at a time.

Prove: `cargo test --workspace extended_peer` validates canonical fixtures,
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
