//! The peer exchange observer: which peers a build gossips, and how it says it.
//!
//! BEP 11 rides inside BEP 10's extension protocol, so ⭐ **this observer opens
//! no socket at all.** It reads a [`Stream`] the peer-wire observer already
//! recorded and reports what the `ut_pex` messages in it carried. That makes it
//! the cheapest of the adjacent surfaces to contain and the one with the least
//! to say about egress: nothing here can send anything.
//!
//! ⚠ **It is still behind a capability.** What `ut_pex` carries is a list of
//! addresses a client will then dial, so a lab that gossips is a lab that tells
//! the build under measurement about hosts outside it. The switch is on the
//! reading because the reading is what this entry adds; an observer that
//! *offered* peers would need the same switch and a great deal more argument.
//!
//! # What is identifying here
//!
//! The specification names six keys and requires none of them. So a build is
//! distinguished by which subset it sends, whether it ever sends `dropped`,
//! whether it fills in `added.f` at all, whether it bothers with the IPv6 keys,
//! the order it writes the keys in, and how often it sends anything. ⛔ **The
//! dictionary is kept as it arrived.** `bit_ids_wire::bencode::Value` preserves
//! key order and duplicates for exactly this reason, and a decoder that sorted
//! on the way in would erase two of those signals.
//!
//! ⚠ **An extension id of zero means the extension is off.** BEP 10 reserves 0
//! for the handshake itself and says a peer disables an extension by mapping it
//! to 0. A build that advertises `ut_pex` at 0 is saying it does not do peer
//! exchange, and reading that as "id 0" would attribute every extended
//! handshake in the stream to `ut_pex`.

use std::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};

use bit_ids_lab::adjacent::{Capability, NotEnabled, Surface, require};
use bit_ids_wire::bencode::Value;

use crate::peer_wire::Stream;

/// The extension name BEP 11 registers.
pub const NAME: &[u8] = b"ut_pex";

/// How many bytes one IPv4 peer occupies in a compact list.
pub const COMPACT_V4_LEN: usize = 6;

/// How many bytes one IPv6 peer occupies in a compact list.
pub const COMPACT_V6_LEN: usize = 18;

/// The keys BEP 11 names, in the order the specification lists them.
pub const KEYS: [&[u8]; 6] = [
    b"added",
    b"added.f",
    b"dropped",
    b"added6",
    b"added6.f",
    b"dropped6",
];

/// One peer's flag byte from `added.f`.
///
/// ⚠ The bits are read and reported, never acted on. Whether a peer "prefers
/// encryption" is a claim the sending build made about a third party, and this
/// project records claims rather than believing them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Flags(u8);

impl Flags {
    /// Bit 0: the peer prefers encryption.
    pub const PREFERS_ENCRYPTION: u8 = 0x01;
    /// Bit 1: the peer is a seed or is upload-only.
    pub const SEED: u8 = 0x02;
    /// Bit 2: the peer supports uTP.
    pub const SUPPORTS_UTP: u8 = 0x04;
    /// Bit 3: the peer supports the holepunch extension.
    pub const SUPPORTS_HOLEPUNCH: u8 = 0x08;
    /// Bit 4: the sender reached this peer by connecting out to it.
    pub const REACHABLE: u8 = 0x10;

    /// The byte as sent.
    #[must_use]
    pub const fn byte(self) -> u8 {
        self.0
    }

    /// Whether a named bit is set.
    #[must_use]
    pub const fn has(self, bit: u8) -> bool {
        self.0 & bit != 0
    }

    /// Whether any bit outside the five BEP 11 names is set.
    ///
    /// ⭐ A signal in its own right. A build using a bit the specification does
    /// not define is telling something about its lineage.
    #[must_use]
    pub const fn has_unspecified_bits(self) -> bool {
        self.0
            & !(Self::PREFERS_ENCRYPTION
                | Self::SEED
                | Self::SUPPORTS_UTP
                | Self::SUPPORTS_HOLEPUNCH
                | Self::REACHABLE)
            != 0
    }
}

/// Something a `ut_pex` message did that BEP 11 does not describe.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Refusal {
    /// The payload is not a dictionary.
    NotADictionary(&'static str),
    /// A key BEP 11 defines as a byte string arrived as something else.
    NotBytes {
        /// The key.
        key: Vec<u8>,
        /// What arrived instead.
        found: &'static str,
    },
    /// A compact peer list whose length is not a whole number of peers.
    NotCompact {
        /// The key.
        key: Vec<u8>,
        /// How many bytes it held.
        len: usize,
        /// How many bytes one peer occupies.
        stride: usize,
    },
    /// A flag list with a different number of entries from its peer list.
    FlagsDisagree {
        /// The flag key.
        key: Vec<u8>,
        /// How many peers the matching list held.
        peers: usize,
        /// How many flag bytes arrived.
        flags: usize,
    },
    /// The dictionary carried the same key twice.
    DuplicateKeys,
    /// A `ut_pex` message arrived before the extended handshake that offers it.
    BeforeHandshake {
        /// Which message in the stream.
        at: usize,
    },
    /// The peer advertised `ut_pex` mapped to zero, which BEP 10 reads as off.
    OfferedDisabled,
}

/// One `ut_pex` message.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Gossip {
    at: usize,
    raw: Vec<u8>,
    document: Value,
    refusals: Vec<Refusal>,
}

impl Gossip {
    /// Which message of the stream this was, counting from zero.
    #[must_use]
    pub const fn at(&self) -> usize {
        self.at
    }

    /// The bencoded payload as it arrived.
    #[must_use]
    pub fn raw(&self) -> &[u8] {
        &self.raw
    }

    /// The decoded dictionary, with key order and duplicates intact.
    #[must_use]
    pub const fn document(&self) -> &Value {
        &self.document
    }

    /// Everything this message did that BEP 11 does not describe.
    #[must_use]
    pub fn refusals(&self) -> &[Refusal] {
        &self.refusals
    }

    /// The keys in the order the build wrote them.
    #[must_use]
    pub fn key_order(&self) -> Vec<Vec<u8>> {
        match &self.document {
            Value::Dictionary(entries) => entries.iter().map(|(key, _)| key.clone()).collect(),
            _ => Vec::new(),
        }
    }

    /// Whether the keys arrived in the ascending order BEP 3 requires.
    #[must_use]
    pub fn keys_are_sorted(&self) -> Option<bool> {
        self.document.keys_are_sorted()
    }

    /// Whether a key is present at all, whatever it holds.
    #[must_use]
    pub fn carries(&self, key: &[u8]) -> bool {
        self.document.get(key).is_some()
    }

    /// The bytes stored under a key, when it is a byte string.
    #[must_use]
    pub fn bytes(&self, key: &[u8]) -> Option<&[u8]> {
        match self.document.get(key)? {
            Value::Bytes(bytes) => Some(bytes),
            _ => None,
        }
    }

    /// The IPv4 peers under a key, whole entries only.
    ///
    /// ⚠ A trailing partial entry is dropped here and reported as
    /// [`Refusal::NotCompact`]. Inventing an address out of four bytes and a
    /// missing port would put a peer in the record that was never sent.
    #[must_use]
    pub fn peers_v4(&self, key: &[u8]) -> Vec<SocketAddrV4> {
        let Some(bytes) = self.bytes(key) else {
            return Vec::new();
        };
        // ⚠ `as_chunks` rather than a manual stride: the remainder is returned
        // separately, so a partial trailing entry cannot be read as a peer by
        // an off-by-one. It is reported as `NotCompact` and dropped here.
        let (whole, _partial) = bytes.as_chunks::<COMPACT_V4_LEN>();
        whole
            .iter()
            .map(|entry| {
                SocketAddrV4::new(
                    Ipv4Addr::new(entry[0], entry[1], entry[2], entry[3]),
                    u16::from_be_bytes([entry[4], entry[5]]),
                )
            })
            .collect()
    }

    /// The IPv6 peers under a key, whole entries only.
    #[must_use]
    pub fn peers_v6(&self, key: &[u8]) -> Vec<SocketAddrV6> {
        let Some(bytes) = self.bytes(key) else {
            return Vec::new();
        };
        let (whole, _partial) = bytes.as_chunks::<COMPACT_V6_LEN>();
        whole
            .iter()
            .map(|entry| {
                let mut address = [0_u8; 16];
                address.copy_from_slice(&entry[..16]);
                SocketAddrV6::new(
                    Ipv6Addr::from(address),
                    u16::from_be_bytes([entry[16], entry[17]]),
                    0,
                    0,
                )
            })
            .collect()
    }

    /// The flag bytes under a key, one per peer.
    #[must_use]
    pub fn flags(&self, key: &[u8]) -> Vec<Flags> {
        self.bytes(key)
            .map(|bytes| bytes.iter().copied().map(Flags).collect())
            .unwrap_or_default()
    }
}

/// What one peer-wire stream carried on `ut_pex`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Exchange {
    offered_id: Option<i64>,
    handshake_at: Option<usize>,
    gossip: Vec<Gossip>,
    refusals: Vec<Refusal>,
}

impl Exchange {
    /// The extension id the peer advertised for `ut_pex`, if it advertised one.
    ///
    /// [`None`] means the build offered no peer exchange, which is a finding
    /// rather than an absence of data.
    #[must_use]
    pub const fn offered_id(&self) -> Option<i64> {
        self.offered_id
    }

    /// Whether the build offers peer exchange in a form it can actually use.
    #[must_use]
    pub fn offers_peer_exchange(&self) -> bool {
        self.offered_id.is_some_and(|id| id != 0)
    }

    /// Which message of the stream carried the extended handshake.
    #[must_use]
    pub const fn handshake_at(&self) -> Option<usize> {
        self.handshake_at
    }

    /// Every `ut_pex` message, in the order it arrived.
    #[must_use]
    pub fn gossip(&self) -> &[Gossip] {
        &self.gossip
    }

    /// Findings about the stream rather than about one message.
    #[must_use]
    pub fn refusals(&self) -> &[Refusal] {
        &self.refusals
    }

    /// Whether every message and the stream itself matched BEP 11.
    #[must_use]
    pub fn is_conforming(&self) -> bool {
        self.refusals.is_empty() && self.gossip.iter().all(|one| one.refusals.is_empty())
    }
}

/// Reads what one recorded peer-wire stream carried on `ut_pex`.
///
/// # Errors
///
/// Returns [`NotEnabled`] when `capability` enables a different surface.
pub fn read(capability: Capability, stream: &Stream) -> Result<Exchange, NotEnabled> {
    require(capability, Surface::Pex)?;
    let mut exchange = Exchange {
        offered_id: None,
        handshake_at: None,
        gossip: Vec::new(),
        refusals: Vec::new(),
    };

    // ⛔ **Two passes, because an extended message has no meaning until the
    // handshake names its id.** A single forward pass can only attribute
    // messages that arrive after the handshake, which makes a build that
    // gossips before announcing its extension map look like a build that never
    // gossips at all, and [`Refusal::BeforeHandshake`] would be a variant
    // nothing could ever produce.
    let mut candidates = Vec::new();
    for (at, message) in stream.messages().iter().enumerate() {
        let Some(Ok(extended)) = message.as_extended() else {
            continue;
        };
        if !extended.is_handshake() {
            candidates.push((at, extended));
            continue;
        }
        // The first one. A second extended handshake is a finding for `OBS-05`
        // to report, and taking the later map here would change what every
        // message before it was read as.
        if exchange.handshake_at.is_some() {
            continue;
        }
        exchange.handshake_at = Some(at);
        exchange.offered_id = extended
            .extension_ids()
            .into_iter()
            .find(|(name, _)| name.as_slice() == NAME)
            .map(|(_, id)| id);
        if exchange.offered_id == Some(0) {
            exchange.refusals.push(Refusal::OfferedDisabled);
        }
    }

    // ⚠ Matched against the id the peer offered, never against a fixed number.
    // `ut_pex` has no reserved id: the peer chooses one and says so, and a
    // hard-coded 1 would read whichever extension that build happened to put
    // first.
    //
    // ⛔ **`filter` here is the one guard in this entry that nothing refutes,
    // and it is kept.** Removing it survived a plant, because no candidate can
    // carry extended id 0: `is_handshake` is `extended_id == 0`, so an id-0
    // message always takes the handshake branch above and never reaches this
    // loop. What refutes the same mistake elsewhere is `offers_peer_exchange`,
    // which reports the build's own answer to a reader, and `OfferedDisabled`,
    // which reports it in the record; both are refuted. This line is defence
    // against a later change to how a handshake is recognised, and it says so
    // rather than looking like a check that does work.
    let Some(offered) = exchange.offered_id.filter(|id| *id != 0) else {
        return Ok(exchange);
    };
    for (at, extended) in candidates {
        if u8::try_from(offered) != Ok(extended.extended_id()) {
            continue;
        }
        if exchange.handshake_at.is_none_or(|handshake| at < handshake) {
            exchange.refusals.push(Refusal::BeforeHandshake { at });
        }
        exchange.gossip.push(gossip_of(at, &extended));
    }
    Ok(exchange)
}

/// Reads one extended message as a `ut_pex` payload.
fn gossip_of(at: usize, extended: &bit_ids_wire::peer_wire::ExtendedMessage) -> Gossip {
    let mut gossip = Gossip {
        at,
        raw: extended.raw().to_vec(),
        document: extended.document().clone(),
        refusals: Vec::new(),
    };
    gossip.refusals = refusals_of(&gossip);
    gossip
}

/// Every way one `ut_pex` payload departs from BEP 11.
///
/// ⚠ All of them, not the first. The set of things a build gets wrong is more
/// identifying than whichever one a short-circuiting reader stopped at.
fn refusals_of(gossip: &Gossip) -> Vec<Refusal> {
    let Value::Dictionary(_) = &gossip.document else {
        return vec![Refusal::NotADictionary(gossip.document.type_name())];
    };
    let mut refusals = Vec::new();
    if gossip.document.has_duplicate_keys() == Some(true) {
        refusals.push(Refusal::DuplicateKeys);
    }
    for key in KEYS {
        let Some(value) = gossip.document.get(key) else {
            continue;
        };
        let Value::Bytes(bytes) = value else {
            refusals.push(Refusal::NotBytes {
                key: key.to_vec(),
                found: value.type_name(),
            });
            continue;
        };
        let stride = if key.ends_with(b".f") {
            1
        } else if key.ends_with(b"6") {
            COMPACT_V6_LEN
        } else {
            COMPACT_V4_LEN
        };
        if bytes.len() % stride != 0 {
            refusals.push(Refusal::NotCompact {
                key: key.to_vec(),
                len: bytes.len(),
                stride,
            });
        }
    }
    refusals.extend(flag_disagreements(gossip));
    refusals
}

/// Flag lists whose length does not match the peer list they describe.
fn flag_disagreements(gossip: &Gossip) -> Vec<Refusal> {
    let mut refusals = Vec::new();
    for (flag_key, peer_key, stride) in [
        (&b"added.f"[..], &b"added"[..], COMPACT_V4_LEN),
        (&b"added6.f"[..], &b"added6"[..], COMPACT_V6_LEN),
    ] {
        let Some(flags) = gossip.bytes(flag_key) else {
            continue;
        };
        let peers = gossip.bytes(peer_key).map_or(0, <[u8]>::len) / stride;
        if flags.len() != peers {
            refusals.push(Refusal::FlagsDisagree {
                key: flag_key.to_vec(),
                peers,
                flags: flags.len(),
            });
        }
    }
    refusals
}

#[cfg(test)]
mod tests {
    use super::{Exchange, Flags, KEYS, Refusal, read};
    use bit_ids_lab::adjacent::{ALL_SURFACES as ALL, Capability, Surface};
    use bit_ids_wire::bencode;
    use bit_ids_wire::peer_wire::{
        EXTENDED_HANDSHAKE_ID, EXTENDED_MESSAGE_ID, Handshake, INFO_HASH_LEN, Message, PEER_ID_LEN,
    };

    use crate::peer_wire::{Role, Stream};
    use bit_ids_lab::endpoint::ConnectionId;

    const PEX_ID: u8 = 3;

    fn capability() -> Capability {
        Capability::enable(Surface::Pex)
    }

    /// The BEP 3 handshake every transcript opens with, offering BEP 10.
    fn opening() -> Vec<u8> {
        let mut bytes = vec![19];
        bytes.extend_from_slice(b"BitTorrent protocol");
        bytes.extend_from_slice(&[0, 0, 0, 0, 0, 0x10, 0, 0]);
        bytes.extend_from_slice(&[7_u8; INFO_HASH_LEN]);
        bytes.extend_from_slice(&[9_u8; PEER_ID_LEN]);
        Handshake::parse(&bytes)
            .expect("a well-formed handshake")
            .encode()
    }

    /// One BEP 10 extended message, framed.
    fn extended(id: u8, payload: &[u8]) -> Vec<u8> {
        let mut body = vec![id];
        body.extend_from_slice(payload);
        Message::Typed {
            id: EXTENDED_MESSAGE_ID,
            payload: body,
        }
        .encode()
    }

    /// The extended handshake, offering `ut_pex` at `offered` when there is one.
    fn hello(offered: Option<i64>) -> Vec<u8> {
        let map = offered.map_or_else(Vec::new, |id| {
            vec![(b"ut_pex".to_vec(), bencode::Value::integer(id))]
        });
        let document =
            bencode::Value::Dictionary(vec![(b"m".to_vec(), bencode::Value::Dictionary(map))]);
        extended(EXTENDED_HANDSHAKE_ID, &bencode::encode(&document))
    }

    /// Builds a peer-wire transcript: a handshake, an extended handshake
    /// offering `ut_pex` at `offered`, then one extended message per payload.
    fn transcript(offered: Option<i64>, payloads: &[(u8, Vec<u8>)]) -> Vec<u8> {
        let mut out = opening();
        out.extend_from_slice(&hello(offered));
        for (id, payload) in payloads {
            out.extend_from_slice(&extended(*id, payload));
        }
        out
    }

    fn stream_of(offered: Option<i64>, payloads: &[(u8, Vec<u8>)]) -> Stream {
        Stream::recorded(
            ConnectionId::recorded(1).expect("one is a real connection number"),
            Role::TargetDialled,
            &transcript(offered, payloads),
        )
    }

    fn read_exchange(offered: Option<i64>, payloads: &[(u8, Vec<u8>)]) -> Exchange {
        let stream = stream_of(offered, payloads);
        assert!(stream.error().is_none(), "the fixture transcript decodes");
        read(capability(), &stream).expect("the capability names this surface")
    }

    fn pex(entries: Vec<(&[u8], bencode::Value)>) -> Vec<u8> {
        bencode::encode(&bencode::Value::Dictionary(
            entries
                .into_iter()
                .map(|(key, value)| (key.to_vec(), value))
                .collect(),
        ))
    }

    fn peer(a: u8, b: u8, c: u8, d: u8, port: u16) -> Vec<u8> {
        let mut out = vec![a, b, c, d];
        out.extend_from_slice(&port.to_be_bytes());
        out
    }

    #[test]
    fn a_conforming_message_is_read_and_keeps_its_own_bytes() {
        let mut added = peer(10, 0, 0, 1, 6881);
        added.extend_from_slice(&peer(10, 0, 0, 2, 51413));
        let payload = pex(vec![
            (b"added", bencode::Value::Bytes(added)),
            (b"added.f", bencode::Value::Bytes(vec![0x01, 0x12])),
            (b"dropped", bencode::Value::Bytes(peer(10, 0, 0, 3, 1))),
        ]);
        let exchange = read_exchange(Some(i64::from(PEX_ID)), &[(PEX_ID, payload.clone())]);

        assert_eq!(exchange.offered_id(), Some(i64::from(PEX_ID)));
        assert!(exchange.offers_peer_exchange());
        assert_eq!(exchange.handshake_at(), Some(0));
        assert!(exchange.is_conforming(), "{:?}", exchange.refusals());

        let gossip = &exchange.gossip()[0];
        assert_eq!(gossip.at(), 1);
        assert_eq!(gossip.raw(), payload, "the payload bytes are the evidence");
        assert_eq!(gossip.peers_v4(b"added").len(), 2);
        assert_eq!(gossip.peers_v4(b"added")[1].port(), 51413);
        assert_eq!(gossip.peers_v4(b"dropped").len(), 1);

        let flags = gossip.flags(b"added.f");
        assert!(flags[0].has(Flags::PREFERS_ENCRYPTION));
        assert!(!flags[0].has(Flags::SEED));
        assert!(flags[1].has(Flags::REACHABLE));
        assert!(flags[1].has(Flags::SEED));
        assert!(!flags[1].has_unspecified_bits());
    }

    #[test]
    fn the_key_order_a_build_used_survives_the_reading() {
        // ⭐ Written out of BEP 3's order on purpose. `bencode::Value` keeps a
        // dictionary as a list of pairs so this can be seen at all.
        let payload = pex(vec![
            (b"dropped", bencode::Value::Bytes(Vec::new())),
            (b"added", bencode::Value::Bytes(peer(10, 0, 0, 1, 6881))),
        ]);
        let exchange = read_exchange(Some(1), &[(1, payload)]);
        let gossip = &exchange.gossip()[0];
        assert_eq!(
            gossip.key_order(),
            vec![b"dropped".to_vec(), b"added".to_vec()]
        );
        assert_eq!(gossip.keys_are_sorted(), Some(false));
        assert!(!gossip.carries(b"added.f"));
        assert!(gossip.carries(b"dropped"));
        // Unsorted keys are legible bencode and a difference between builds,
        // so they are reported rather than refused.
        assert!(exchange.is_conforming(), "{:?}", gossip.refusals());
    }

    #[test]
    fn a_build_that_offers_no_peer_exchange_gossips_nothing() {
        let payload = pex(vec![(b"added", bencode::Value::Bytes(Vec::new()))]);
        let exchange = read_exchange(None, &[(1, payload)]);
        assert_eq!(exchange.offered_id(), None);
        assert!(!exchange.offers_peer_exchange());
        assert!(exchange.gossip().is_empty());
    }

    #[test]
    fn an_extension_mapped_to_zero_is_off_rather_than_at_id_zero() {
        // ⛔ Reading `ut_pex: 0` as an id would attribute every extended
        // handshake in the stream to peer exchange.
        let payload = pex(vec![(b"added", bencode::Value::Bytes(Vec::new()))]);
        let exchange = read_exchange(Some(0), &[(0, payload)]);
        assert_eq!(exchange.offered_id(), Some(0));
        assert!(!exchange.offers_peer_exchange());
        assert!(exchange.gossip().is_empty());
        assert!(exchange.refusals().contains(&Refusal::OfferedDisabled));
    }

    #[test]
    fn a_message_on_another_extensions_id_is_not_read_as_peer_exchange() {
        let payload = pex(vec![(b"added", bencode::Value::Bytes(Vec::new()))]);
        let exchange = read_exchange(Some(i64::from(PEX_ID)), &[(PEX_ID + 1, payload)]);
        assert!(exchange.gossip().is_empty());
    }

    #[test]
    fn a_peer_list_that_is_not_a_whole_number_of_peers_is_refused() {
        let payload = pex(vec![(
            b"added",
            bencode::Value::Bytes(vec![10, 0, 0, 1, 26]),
        )]);
        let exchange = read_exchange(Some(1), &[(1, payload)]);
        let gossip = &exchange.gossip()[0];
        assert!(matches!(
            gossip.refusals()[0],
            Refusal::NotCompact {
                len: 5,
                stride: 6,
                ..
            }
        ));
        // ⛔ The whole entries are still read, and the partial one is not
        // invented into an address.
        assert!(gossip.peers_v4(b"added").is_empty());
    }

    #[test]
    fn a_flag_list_that_does_not_match_its_peer_list_is_refused() {
        let payload = pex(vec![
            (b"added", bencode::Value::Bytes(peer(10, 0, 0, 1, 6881))),
            (b"added.f", bencode::Value::Bytes(vec![0x01, 0x02, 0x04])),
        ]);
        let exchange = read_exchange(Some(1), &[(1, payload)]);
        assert!(matches!(
            exchange.gossip()[0].refusals()[0],
            Refusal::FlagsDisagree {
                peers: 1,
                flags: 3,
                ..
            }
        ));
    }

    #[test]
    fn a_key_of_the_wrong_type_and_a_payload_that_is_not_a_dictionary_are_refused() {
        let payload = pex(vec![(b"added", bencode::Value::integer(3))]);
        let exchange = read_exchange(Some(1), &[(1, payload)]);
        assert!(matches!(
            exchange.gossip()[0].refusals()[0],
            Refusal::NotBytes {
                found: "integer",
                ..
            }
        ));

        let list = bencode::encode(&bencode::Value::List(Vec::new()));
        let exchange = read_exchange(Some(1), &[(1, list)]);
        assert_eq!(
            exchange.gossip()[0].refusals(),
            [Refusal::NotADictionary("list")]
        );
    }

    #[test]
    fn every_key_the_specification_names_is_checked_for_its_own_stride() {
        // ⚠ Asserts the table rather than one entry: a stride picked by the
        // wrong rule would pass a v6 list at six bytes a peer.
        assert_eq!(KEYS.len(), 6);
        let payload = pex(vec![
            (b"added6", bencode::Value::Bytes(vec![0_u8; 17])),
            (b"dropped6", bencode::Value::Bytes(vec![0_u8; 36])),
        ]);
        let exchange = read_exchange(Some(1), &[(1, payload)]);
        let refusals = exchange.gossip()[0].refusals();
        assert_eq!(refusals.len(), 1, "{refusals:?}");
        assert!(matches!(
            refusals[0],
            Refusal::NotCompact {
                len: 17,
                stride: 18,
                ..
            }
        ));
        assert_eq!(exchange.gossip()[0].peers_v6(b"dropped6").len(), 2);
    }

    /// ⛔ The case a single forward pass cannot see. Nothing knows message 0 is
    /// `ut_pex` until message 1 says so, and a reader that decided as it went
    /// would report a build that gossiped early as one that never gossiped.
    #[test]
    fn gossip_sent_before_the_extended_handshake_is_attributed_and_reported() {
        let payload = pex(vec![(
            b"added",
            bencode::Value::Bytes(peer(10, 0, 0, 1, 81)),
        )]);
        let mut raw = opening();
        raw.extend_from_slice(&extended(PEX_ID, &payload));
        raw.extend_from_slice(&hello(Some(i64::from(PEX_ID))));
        let stream = Stream::recorded(
            ConnectionId::recorded(1).expect("one is a real connection number"),
            Role::TargetDialled,
            &raw,
        );
        assert!(stream.error().is_none(), "the fixture transcript decodes");
        let exchange = read(capability(), &stream).expect("this surface");

        assert_eq!(exchange.handshake_at(), Some(1));
        assert_eq!(exchange.gossip().len(), 1, "the message is still read");
        assert_eq!(exchange.gossip()[0].at(), 0);
        assert_eq!(exchange.gossip()[0].peers_v4(b"added").len(), 1);
        assert_eq!(exchange.refusals(), [Refusal::BeforeHandshake { at: 0 }]);
        assert!(!exchange.is_conforming());
    }

    #[test]
    fn the_reading_is_refused_without_a_capability_for_its_own_surface() {
        let stream = stream_of(Some(1), &[]);
        for other in ALL {
            if other == Surface::Pex {
                continue;
            }
            let refusal = read(Capability::enable(other), &stream).expect_err("another surface");
            assert_eq!(refusal.wanted, Surface::Pex);
            assert_eq!(refusal.offered, other);
        }
        assert!(read(capability(), &stream).is_ok());
    }
}
