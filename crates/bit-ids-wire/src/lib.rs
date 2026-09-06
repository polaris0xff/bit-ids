//! Byte-exact codecs for the surfaces a capture observes, and the fixture
//! corpus every observer parses against.
//!
//! # What this crate is for
//!
//! A published profile says what a build put on the wire. When next week's
//! capture of the same build parses differently, exactly one of two things
//! happened: the build changed, or the observer did. A live capture cannot tell
//! those apart, because both of its inputs moved. A fixture holds bytes that
//! provably did not move, so a parse that changed against one is the parser.
//!
//! # The one invariant
//!
//! ⛔ **Decode then encode reproduces the input, byte for byte.** Every codec
//! here holds it and the suite asserts it over every fixture.
//!
//! It is not a tidiness rule. `docs/architecture.md` section 5 lists what a
//! parsed view has to retain: query and header order, duplicate fields,
//! percent-encoding hex case, all eight reserved bytes and early message order.
//! Every one of those is destroyed by the convenient implementation. A
//! round trip is the cheapest test that catches all of them at once, because a
//! decoder that dropped a detail has nothing to write back.
//!
//! So the codecs **observe and report** rather than impose. Unsorted bencode
//! keys, a non-canonical integer, a bare `\n` where the grammar says `\r\n`, an
//! unassigned message id, a non-standard handshake protocol string: each is a
//! difference between builds, so each is recorded and none is refused.
//!
//! # What this crate will not do
//!
//! ⛔ **It never maps a peer-ID prefix, a user agent or a BEP 10 `v` string to a
//! client name.** `docs/capture-methodology.md` lists a peer-ID registry or
//! decoder table among the inputs that may seed a hypothesis and may not
//! populate the catalogue. A codec that answered "this is client X" would put
//! that refused input inside the one component every observer trusts.
//!
//! # Where it sits
//!
//! `bit-ids` is the published record contract and a catalogue consumer depends
//! on it; nobody reading the catalogue needs a `BitTorrent` codec, so this is a
//! second crate rather than a module of that one. It depends on `bit-ids` for
//! the canonical value forms, and the arrow points that way only.

pub mod bencode;
pub mod dht;
pub mod error;
pub mod fixture;
pub mod peer_wire;
pub mod tracker_http;
pub mod tracker_udp;

pub use error::WireError;
pub use fixture::{FIXTURE_SCHEMA, Fixture, FixtureIndex};

#[cfg(test)]
mod tests {
    use super::{FIXTURE_SCHEMA, fixture::INDEX_SCHEMA};

    #[test]
    fn both_schema_identifiers_are_versioned() {
        assert_eq!(FIXTURE_SCHEMA, "bit-ids/wire-fixture/1");
        assert_eq!(INDEX_SCHEMA, "bit-ids/wire-fixture-index/1");
    }
}
