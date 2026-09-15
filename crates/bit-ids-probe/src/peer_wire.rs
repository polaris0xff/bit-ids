//! The peer-wire handshake observer: what a build opens a peer connection with.
//!
//! `OBS-04`. The handshake carries the protocol string, all eight reserved
//! bytes, the info hash and the twenty peer-ID bytes, and the messages after it
//! carry their own order. ⛔ **Order and the reserved block are the two things a
//! convenient implementation destroys**, by decoding the reserved bytes into
//! named flags and by looking messages up by id, so neither is done here.
//!
//! # Both roles, because a client can differ by role
//!
//! ⭐ **A build that dialled and a build that accepted are two observations.**
//! `docs/architecture.md` section 5 asks for both incoming and outgoing
//! connections for that reason. The same responder serves each: what changes is
//! who sends the handshake first, which the lab's `dial` handles by writing an
//! opening before it reads.
//!
//! ⚠ **The observer's handshake is part of the experiment.** A peer that never
//! completes a handshake makes a client disconnect, and the disconnection would
//! be recorded as identity. So the observer answers with a handshake carrying
//! the info hash the target asked for, which is what a peer that has the torrent
//! does.
//!
//! # What it is not
//!
//! ⛔ **Nothing here maps a peer-ID prefix or a BEP 10 `v` string to a client
//! name.** The peer ID is twenty bytes; what build produced them is what a
//! capture measures.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use bit_ids_lab::{ConnectionId, StreamReply};
use bit_ids_wire::WireError;
use bit_ids_wire::bencode::{self, Value};
use bit_ids_wire::peer_wire::{
    EXTENDED_HANDSHAKE_ID, EXTENDED_MESSAGE_ID, ExtendedMessage, Handshake, INFO_HASH_LEN, Message,
    PEER_ID_LEN, RESERVED_LEN, Transcript,
};

/// How many connections one observer keeps before it stops keeping them.
///
/// ⛔ The lab's deadline bounds how long a target can talk and not how many
/// connections it opens.
pub const DEFAULT_MAX_STREAMS: usize = 512;

/// How many trailing bytes of the observer's peer ID carry the connection
/// ordinal.
///
/// ⚠ Four, because four decimal digits is what fits beside the sixteen-byte
/// name the rest of this repository's fixture identities already carry.
pub const ORDINAL_DIGITS: usize = 4;

/// One more than the largest ordinal four decimal digits can spell.
const ORDINAL_WRAP: u32 = 10_000;

// A peer ID shorter than its own ordinal has no prefix left to be a name, and
// the slice below would panic on it. `PEER_ID_LEN` is BEP 3's and does not move,
// so this stops a later edit at the build rather than at one suite.
const _: () = assert!(PEER_ID_LEN > ORDINAL_DIGITS);

/// What one peer connection carried.
#[derive(Clone, Debug)]
pub struct Stream {
    connection: ConnectionId,
    role: Role,
    raw: Vec<u8>,
    handshake: Option<Handshake>,
    messages: Vec<Message>,
    error: Option<WireError>,
    /// Whether the observer has already sent its handshake on this connection.
    ///
    /// ⛔ Per connection, which is why the responder takes a [`ConnectionId`].
    /// One responder serves every connection an endpoint accepts, and without
    /// this a second connection's handshake would go down the first.
    answered: bool,
    /// Whether the observer has already sent its extended handshake.
    extended_sent: bool,
    /// The twenty bytes the observer presented on this connection.
    ///
    /// ⚠ **A condition of the measurement rather than a measurement.** What the
    /// build answered is [`Stream::handshake`]; this is what it was answering,
    /// and the two are kept apart because a reader that confused them would
    /// report the observer's own identity as the build's.
    presented: Option<[u8; PEER_ID_LEN]>,
}

/// Which side opened the connection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Role {
    /// The build under measurement connected to the observer.
    TargetDialled,
    /// The observer connected to the build under measurement.
    ObserverDialled,
}

impl Stream {
    /// Reads a stream back from bytes that were already recorded.
    ///
    /// ⭐ **What a later pass over an evidence bundle gets.** `OBS-09` stores a
    /// run's segments as bytes; an analysis that wants the same reading the live
    /// observer made would otherwise re-implement the decode, and the copy would
    /// disagree first about the partial trailing message. This is that decode,
    /// called by a caller who has the bytes rather than the socket.
    ///
    /// ⚠ `connection` and `role` are passed in because they are capture facts.
    /// Which side dialled is not in the byte stream, and a reader that guessed
    /// would be inventing the one field `OBS-04` exists to distinguish.
    ///
    /// ⛔ **[`Stream::presented`] is `None` here and that is not an absence of
    /// one.** `raw` is what the TARGET sent; what the observer presented went
    /// the other way, and a bundle carries it as the connection's `to_target`
    /// segment. A reader that filled this in from `raw` would be reporting the
    /// build's own peer ID as the observer's offer.
    #[must_use]
    pub fn recorded(connection: ConnectionId, role: Role, raw: &[u8]) -> Self {
        let mut stream = Self {
            connection,
            role,
            raw: raw.to_vec(),
            handshake: None,
            messages: Vec::new(),
            error: None,
            answered: false,
            extended_sent: false,
            presented: None,
        };
        match Transcript::parse(raw) {
            Ok(transcript) => {
                stream.handshake = Some(transcript.handshake().clone());
                stream.messages = transcript.messages().to_vec();
            }
            Err(error) => {
                // A partial tail does not hide a handshake that already arrived
                // whole, which is what the live path does with the same bytes.
                stream.handshake = Handshake::parse_prefix(raw).ok().map(|(one, _)| one);
                stream.error = Some(error);
            }
        }
        stream
    }

    /// Which connection this was.
    #[must_use]
    pub const fn connection(&self) -> ConnectionId {
        self.connection
    }

    /// Which side opened it.
    #[must_use]
    pub const fn role(&self) -> Role {
        self.role
    }

    /// The twenty bytes the observer presented on this connection, if it has.
    ///
    /// `None` on a connection the observer has not answered yet, and on any
    /// stream rebuilt by [`Stream::recorded`], for the reason that method gives.
    #[must_use]
    pub const fn presented_peer_id(&self) -> Option<&[u8; PEER_ID_LEN]> {
        self.presented.as_ref()
    }

    /// Everything the target sent on this connection, in order.
    #[must_use]
    pub fn raw(&self) -> &[u8] {
        &self.raw
    }

    /// The handshake, once enough bytes had arrived to read one.
    #[must_use]
    pub const fn handshake(&self) -> Option<&Handshake> {
        self.handshake.as_ref()
    }

    /// The messages after the handshake, in the order they arrived.
    ///
    /// ⭐ A sequence, never a lookup by id. Early message order is an identity
    /// field, and a map would let a caller forget that.
    #[must_use]
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Why the bytes stopped decoding, when they did.
    ///
    /// ⚠ Kept rather than discarded. A build that sends something this codec
    /// cannot read has told us something, and the bytes are in `raw`.
    #[must_use]
    pub const fn error(&self) -> Option<&WireError> {
        self.error.as_ref()
    }

    /// The target's BEP 10 extended handshake, when it sent one.
    ///
    /// ⚠ The inner error means the extension dictionary did not decode, which
    /// is an observation about the build. The bytes are in [`Stream::raw`]
    /// either way.
    #[must_use]
    pub fn extended_handshake(&self) -> Option<Result<ExtendedMessage, WireError>> {
        match Transcript::parse(&self.raw) {
            Ok(transcript) => match transcript.extended_handshake() {
                Ok(found) => found.map(Ok),
                Err(error) => Some(Err(error)),
            },
            // A trailing partial message does not hide a handshake that already
            // arrived whole, so the messages decoded so far are searched.
            Err(_) => self
                .messages
                .iter()
                .filter_map(Message::as_extended)
                .find(|extended| extended.as_ref().is_ok_and(ExtendedMessage::is_handshake)),
        }
    }

    /// Whether the target offered BEP 10 in its own reserved block.
    ///
    /// ⭐ Distinct from whether it sent an extended handshake. A build that
    /// offers the protocol and never uses it, and one that does not offer it at
    /// all, are different measurements.
    #[must_use]
    pub fn offers_extension_protocol(&self) -> bool {
        self.handshake
            .as_ref()
            .is_some_and(Handshake::offers_extension_protocol)
    }

    /// Whether the whole of what arrived rebuilds byte for byte from what was
    /// decoded.
    ///
    /// ⛔ **This is the check the entry asks for.** A normalized handshake that
    /// cannot be rebuilt from the raw bytes means the decode lost something, and
    /// a published field derived from that decode would be describing the
    /// decoder rather than the build.
    #[must_use]
    pub fn rebuilds_from_raw(&self) -> bool {
        match Transcript::parse(&self.raw) {
            Ok(transcript) => transcript.encode() == self.raw,
            Err(_) => false,
        }
    }
}

/// What the observer offers in its reserved block, and what it says in BEP 10.
///
/// ⭐ **This is a condition of the measurement, not a setting.** A build answers
/// what it was offered: it sends an extended handshake because the observer
/// asked for one, and its extension map may differ with what the observer put in
/// its own. `OBS-05`'s approach is to vary allowed features one at a time, which
/// is only meaningful if what was offered is recorded beside what came back.
/// [`PeerWire::offer`] is where a record reads it from.
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct Offer {
    /// BEP 10, the extension protocol.
    pub extension_protocol: ExtensionProtocol,
    /// BEP 5, the DHT. Reserved byte 7, bit `0x01`.
    pub dht: bool,
    /// BEP 6, the fast extension. Reserved byte 7, bit `0x04`.
    pub fast: bool,
}

/// What this observer does about BEP 10.
///
/// ⛔ **Three states, and a bit plus an option would have been four.** The
/// fourth is "do not offer the protocol and send an extended handshake anyway",
/// which is an observer inventing a negotiation, and the guard-mutation pass
/// found that deleting the check for it changed no test result. It is not a
/// state this type can hold.
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub enum ExtensionProtocol {
    /// The reserved bit stays clear and nothing is sent.
    #[default]
    NotOffered,
    /// The reserved bit is set and no extended handshake follows.
    ///
    /// A real condition to run: a build that is offered the protocol and never
    /// answered is a different measurement from one that was never offered it.
    OfferedSilent,
    /// The reserved bit is set and this handshake follows, once the target has
    /// offered the protocol too.
    Offered(ExtendedOffer),
}

impl ExtensionProtocol {
    /// Whether the reserved bit is set.
    #[must_use]
    pub const fn is_offered(&self) -> bool {
        !matches!(self, Self::NotOffered)
    }

    /// The extended handshake to send, when there is one.
    #[must_use]
    pub const fn handshake(&self) -> Option<&ExtendedOffer> {
        match self {
            Self::Offered(offer) => Some(offer),
            Self::NotOffered | Self::OfferedSilent => None,
        }
    }
}

/// The BEP 10 extended handshake the observer sends.
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct ExtendedOffer {
    /// The `m` map: extension name to the id this observer will accept it on.
    pub extensions: Vec<(Vec<u8>, i64)>,
    /// The `v` string, which is what this observer says it is.
    pub client: Option<Vec<u8>>,
    /// `reqq`, the request queue depth this observer advertises.
    pub request_queue: Option<i64>,
    /// `metadata_size`, present only when `ut_metadata` is offered.
    pub metadata_size: Option<i64>,
}

impl Offer {
    /// The reserved block these flags produce.
    ///
    /// ⛔ Built from the flags rather than written as a literal, so what the
    /// record says was offered and what went on the wire cannot disagree.
    #[must_use]
    pub const fn reserved(&self) -> [u8; RESERVED_LEN] {
        let mut reserved = [0_u8; RESERVED_LEN];
        if self.extension_protocol.is_offered() {
            reserved[5] |= 0x10;
        }
        if self.dht {
            reserved[7] |= 0x01;
        }
        if self.fast {
            reserved[7] |= 0x04;
        }
        reserved
    }
}

impl ExtendedOffer {
    /// The bencoded extended handshake payload.
    ///
    /// Keys are sorted so the document is canonical bencode and re-encodes to
    /// the bytes it was built from, for the reason the tracker response is
    /// sorted: a deviation here would be this code's, not the build's.
    #[must_use]
    pub fn document(&self) -> Value {
        let mut map: Vec<(Vec<u8>, Value)> = self
            .extensions
            .iter()
            .map(|(name, id)| (name.clone(), Value::integer(*id)))
            .collect();
        map.sort_by(|left, right| left.0.cmp(&right.0));

        let mut entries: Vec<(Vec<u8>, Value)> = vec![(b"m".to_vec(), Value::Dictionary(map))];
        if let Some(size) = self.metadata_size {
            entries.push((b"metadata_size".to_vec(), Value::integer(size)));
        }
        if let Some(queue) = self.request_queue {
            entries.push((b"reqq".to_vec(), Value::integer(queue)));
        }
        if let Some(client) = &self.client {
            entries.push((b"v".to_vec(), Value::bytes(client.clone())));
        }
        entries.sort_by(|left, right| left.0.cmp(&right.0));
        Value::Dictionary(entries)
    }

    /// The whole extended-handshake message, length prefix and ids included.
    #[must_use]
    pub fn message(&self) -> Vec<u8> {
        let payload = bencode::encode(&self.document());
        Message::Typed {
            id: EXTENDED_MESSAGE_ID,
            payload: {
                let mut out = vec![EXTENDED_HANDSHAKE_ID];
                out.extend_from_slice(&payload);
                out
            },
        }
        .encode()
    }
}

/// What the observer sends when a target opens a connection to it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeerIdentity {
    /// The protocol string, `BitTorrent protocol` for BEP 3.
    pub protocol: Vec<u8>,
    /// All eight reserved bytes, sent as given.
    pub reserved: [u8; RESERVED_LEN],
    /// The twenty peer-ID bytes the observer presents.
    pub peer_id: [u8; PEER_ID_LEN],
}

impl Default for PeerIdentity {
    /// The observer identifies itself as a fixture, and offers nothing.
    ///
    /// ⚠ The reserved block is zero on purpose. Every bit set there asks the
    /// target to use an extension, and an extension the observer offered is a
    /// condition of the run rather than something the build chose.
    /// [`PeerIdentity::offering`] turns them on deliberately.
    fn default() -> Self {
        Self {
            protocol: b"BitTorrent protocol".to_vec(),
            reserved: [0; RESERVED_LEN],
            peer_id: *b"bit-ids-fixture-0001",
        }
    }
}

impl PeerIdentity {
    /// The default identity with the reserved block an [`Offer`] produces.
    #[must_use]
    pub fn offering(offer: &Offer) -> Self {
        Self {
            reserved: offer.reserved(),
            ..Self::default()
        }
    }
}

impl PeerIdentity {
    /// The twenty bytes this observer presents on connection `connection`.
    ///
    /// ⛔ **THE LAST FOUR BYTES ARE THE CONNECTION ORDINAL, IN DECIMAL**, so the
    /// first connection presents `...-0001`, the second `...-0002`, and no two
    /// connections of one run are offered the same peer. ⚠ Without that,
    /// `capture-client` run 18 measured a build answering the handshake on one
    /// of two connections and sending nothing on the other, which is exactly
    /// what a client that drops a duplicate peer does - so a value the build
    /// regenerates per connection stayed at one sample and the record stayed
    /// unpublishable.
    ///
    /// ⭐ **This is a fixed prefix and a moving tail, which is the shape this
    /// project MEASURES in a build.** It is here because it is OFFERED:
    /// `OBS-04` records what the observer sent beside what came back, so the
    /// variation is a declared condition of the run rather than noise.
    ///
    /// ⚠ **It wraps at 10000** and the wrap is recorded rather than refused. A
    /// capture that opened ten thousand connections would present its first
    /// identity again; the ordinal is in the transcript either way, and
    /// refusing would end a run over a condition no capture has come near.
    #[must_use]
    pub fn at(&self, connection: u32) -> [u8; PEER_ID_LEN] {
        let mut peer_id = self.peer_id;
        // `{:04}` over a value below 10000 is exactly ORDINAL_DIGITS bytes, so
        // the copy below cannot be short and cannot overflow the array.
        let digits = format!("{:04}", connection % ORDINAL_WRAP);
        peer_id[PEER_ID_LEN - ORDINAL_DIGITS..].copy_from_slice(digits.as_bytes());
        peer_id
    }

    /// The handshake bytes for connection `connection`, answering `info_hash`.
    ///
    /// ⚠ The info hash is echoed rather than chosen. A peer that answers with a
    /// different one is a peer that does not have the torrent, and every client
    /// drops that connection: the observation would then be of this code.
    ///
    /// ⛔ **There is no spelling of this that leaves the connection out.** A
    /// caller that could ask for "the handshake" would get one identity on every
    /// connection, which is the defect [`PeerIdentity::at`] exists to remove, so
    /// the ordinal is a parameter rather than a default.
    #[must_use]
    pub fn handshake(&self, info_hash: &[u8; INFO_HASH_LEN], connection: u32) -> Vec<u8> {
        let mut out = Vec::with_capacity(1 + self.protocol.len() + 48);
        // A protocol string longer than a byte can describe cannot be sent, and
        // truncating one would put a length on the wire that does not match.
        let length = u8::try_from(self.protocol.len()).unwrap_or(u8::MAX);
        out.push(length);
        out.extend_from_slice(&self.protocol[..usize::from(length)]);
        out.extend_from_slice(&self.reserved);
        out.extend_from_slice(info_hash);
        out.extend_from_slice(&self.at(connection));
        out
    }
}

/// One connection's worth of what this observer offers a build.
///
/// ⛔ **The only way to get one is [`PeerWire::present`], which ALLOCATES.** A
/// plain ordinal a caller chose would let two dials present the same identity by
/// typo, and that is the whole defect this type exists to remove - the same
/// argument `OfferedPeer::new` and `Capability::enable` make one layer down.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Presented {
    connection: u32,
    peer_id: [u8; PEER_ID_LEN],
    handshake: Vec<u8>,
}

impl Presented {
    /// Which connection of this run's peer surface it was offered on, from 1.
    #[must_use]
    pub const fn connection(&self) -> u32 {
        self.connection
    }

    /// The twenty bytes offered.
    #[must_use]
    pub const fn peer_id(&self) -> &[u8; PEER_ID_LEN] {
        &self.peer_id
    }

    /// The opening bytes to hand `bit_ids_lab::Lab::dial`.
    #[must_use]
    pub fn handshake(&self) -> &[u8] {
        &self.handshake
    }
}

#[derive(Debug)]
struct Record {
    streams: HashMap<u64, Stream>,
    order: Vec<u64>,
    dropped: usize,
    /// The ordinal the next connection will be offered.
    ///
    /// ⛔ **ONE counter for both roles.** A dial takes its ordinal before the
    /// connection exists and an accepted connection takes one when it is
    /// answered; two counters would hand the same identity to one of each, which
    /// is the defect on the surface where a build meets both.
    next: u32,
    /// Every identity offered, in the order it was allocated.
    presented: Vec<Presented>,
}

impl Default for Record {
    fn default() -> Self {
        Self {
            streams: HashMap::new(),
            order: Vec::new(),
            dropped: 0,
            // ⚠ From 1 rather than 0, so the first connection presents the
            // `...-0001` every fixture identity in this repository already spells.
            next: 1,
            presented: Vec::new(),
        }
    }
}

/// Allocates the next ordinal and records what it will be offered as.
///
/// ⚠ Takes the info hash because an accepted connection is answered with the one
/// the TARGET asked for, which is not always this observer's own.
fn allocate(
    record: &mut Record,
    identity: &PeerIdentity,
    info_hash: &[u8; INFO_HASH_LEN],
) -> Presented {
    let connection = record.next;
    // Saturating rather than wrapping: an ordinal that went back to zero would
    // re-offer an identity already on the wire, and `at` wraps its own digits.
    record.next = record.next.saturating_add(1);
    let presented = Presented {
        connection,
        peer_id: identity.at(connection),
        handshake: identity.handshake(info_hash, connection),
    };
    record.presented.push(presented.clone());
    presented
}

/// The peer-wire observer.
///
/// Hand [`PeerWire::responder`] to a `bit-ids-lab` stream endpoint for the
/// accept role, and to [`bit_ids_lab::Lab::dial`] with
/// [`PeerWire::opening`] for the dial role.
#[derive(Clone, Debug)]
pub struct PeerWire {
    seen: Arc<Mutex<Record>>,
    identity: PeerIdentity,
    offer: Offer,
    info_hash: [u8; INFO_HASH_LEN],
    max_streams: usize,
}

impl PeerWire {
    /// An observer presenting `identity` for the torrent `info_hash`.
    #[must_use]
    pub fn new(identity: PeerIdentity, info_hash: [u8; INFO_HASH_LEN]) -> Self {
        Self {
            seen: Arc::new(Mutex::new(Record::default())),
            identity,
            offer: Offer::default(),
            info_hash,
            max_streams: DEFAULT_MAX_STREAMS,
        }
    }

    /// An observer that offers `offer`, with a reserved block to match.
    ///
    /// ⭐ One constructor for both halves, because they must not disagree: the
    /// reserved block is derived from the same flags the extended handshake is,
    /// so a run that says it offered BEP 10 cannot have sent a zero reserved
    /// block, and one that offered nothing cannot send an extended handshake.
    #[must_use]
    pub fn offering(offer: Offer, info_hash: [u8; INFO_HASH_LEN]) -> Self {
        Self {
            seen: Arc::new(Mutex::new(Record::default())),
            identity: PeerIdentity::offering(&offer),
            offer,
            info_hash,
            max_streams: DEFAULT_MAX_STREAMS,
        }
    }

    /// What this observer offers, which a record cites as a run condition.
    #[must_use]
    pub const fn offer(&self) -> &Offer {
        &self.offer
    }

    /// How many connections this observer keeps.
    #[must_use]
    pub const fn with_max_streams(mut self, max_streams: usize) -> Self {
        self.max_streams = max_streams;
        self
    }

    /// Takes the next identity this observer will offer, for one dial.
    ///
    /// ⭐ The dialling side speaks first, so this cannot wait for a responder
    /// call: a responder is only invoked once bytes have arrived, and in this
    /// role none will until the observer has introduced itself.
    ///
    /// ⛔ **It ALLOCATES, so two dials cannot present one identity.** This
    /// replaced an `opening()` that returned the same bytes however often it was
    /// called, which is what `capture-client` run 18 dialled twice with. Pair
    /// each value with the [`PeerWire::dialling`] responder built from it, so
    /// the connection the bytes arrive on is the one the offer is recorded
    /// against.
    #[must_use]
    pub fn present(&self) -> Presented {
        let info_hash = self.info_hash;
        let identity = self.identity.clone();
        allocate(&mut self.locked(), &identity, &info_hash)
    }

    /// Every identity this observer has offered, in the order it allocated them.
    ///
    /// ⭐ What a record cites as a run condition beside
    /// [`PeerWire::offer`]: the build's answer means something only against what
    /// it was answering.
    #[must_use]
    pub fn presented(&self) -> Vec<Presented> {
        self.locked().presented.clone()
    }

    /// Every connection kept, in the order it was first seen.
    #[must_use]
    pub fn streams(&self) -> Vec<Stream> {
        let record = self.locked();
        record
            .order
            .iter()
            .filter_map(|id| record.streams.get(id).cloned())
            .collect()
    }

    /// One connection by identity.
    #[must_use]
    pub fn stream(&self, connection: ConnectionId) -> Option<Stream> {
        self.locked().streams.get(&connection.get()).cloned()
    }

    /// How many connections arrived after the cap and were served but not kept.
    #[must_use]
    pub fn dropped(&self) -> usize {
        self.locked().dropped
    }

    fn locked(&self) -> std::sync::MutexGuard<'_, Record> {
        self.seen.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The responder for an endpoint the target connects to.
    ///
    /// ⚠ It allocates its identity when it answers rather than now, because an
    /// accepting endpoint does not know how many connections will arrive and an
    /// ordinal taken per responder would be one ordinal for all of them.
    pub fn accepting(&self) -> impl Fn(ConnectionId, &[u8]) -> StreamReply + Send + Sync + 'static {
        self.responder(Role::TargetDialled, None)
    }

    /// The responder for a connection the observer dialled with `presented`.
    ///
    /// ⭐ Two constructors rather than one taking a [`Role`]. The role decides
    /// who sends the handshake first, so attaching the accepting role to a dial
    /// makes the observer wait for bytes it was supposed to send, and nothing in
    /// a single-constructor signature stops that.
    ///
    /// ⛔ **It takes the identity the dial actually presented**, so the offer is
    /// recorded against the connection the answer arrives on. Deriving it here
    /// instead would be a second allocation, and the bytes on the wire came from
    /// the first.
    pub fn dialling(
        &self,
        presented: &Presented,
    ) -> impl Fn(ConnectionId, &[u8]) -> StreamReply + Send + Sync + 'static {
        self.responder(Role::ObserverDialled, Some(presented.peer_id))
    }

    fn responder(
        &self,
        role: Role,
        presented: Option<[u8; PEER_ID_LEN]>,
    ) -> impl Fn(ConnectionId, &[u8]) -> StreamReply + Send + Sync + 'static {
        let seen = Arc::clone(&self.seen);
        let identity = self.identity.clone();
        let offer = self.offer.clone();
        let cap = self.max_streams;
        move |connection, buffered: &[u8]| {
            respond(
                &seen, &identity, &offer, role, presented, cap, connection, buffered,
            )
        }
    }
}

/// ⛔ Nothing is consumed, ever.
///
/// The codec reads a whole transcript from its first byte, so the buffer is the
/// transcript and draining it would leave the decoder without the handshake that
/// frames everything after it. The lab's per-connection byte cap is what bounds
/// the buffer instead, and `Message::MAX_LEN` bounds any single message inside
/// it.
#[expect(
    clippy::too_many_arguments,
    reason = "every one is a term of the experiment, and folding them into a \
              struct would hide which are per-observer and which per-connection"
)]
fn respond(
    seen: &Arc<Mutex<Record>>,
    identity: &PeerIdentity,
    offer: &Offer,
    role: Role,
    presented: Option<[u8; PEER_ID_LEN]>,
    cap: usize,
    connection: ConnectionId,
    buffered: &[u8],
) -> StreamReply {
    let parsed = Transcript::parse(buffered);
    let handshake_only = Handshake::parse_prefix(buffered).ok();

    let mut record = seen.lock().unwrap_or_else(PoisonError::into_inner);
    let known = record.streams.contains_key(&connection.get());
    // ⚠ Closed rather than left open. `NeedMore` would hold the connection
    // buffering until the lab's per-connection byte cap fired, which is a slower
    // and less legible version of the same refusal. ⛔ It does change what the
    // target sees, which is why `dropped` counts it: past the cap this observer
    // has stopped observing and says so.
    if !known && record.order.len() >= cap {
        record.dropped += 1;
        return StreamReply::Close { send: Vec::new() };
    }
    if !known {
        record.order.push(connection.get());
    }
    let stream = record.streams.entry(connection.get()).or_insert(Stream {
        connection,
        role,
        raw: Vec::new(),
        handshake: None,
        messages: Vec::new(),
        error: None,
        answered: false,
        extended_sent: false,
        // ⚠ The dialling role already put its opening on the wire, so the offer
        // is known before a byte comes back. The accepting role has offered
        // nothing yet and fills this in when it answers.
        presented,
    });
    stream.raw = buffered.to_vec();
    match &parsed {
        Ok(transcript) => {
            stream.handshake = Some(transcript.handshake().clone());
            stream.messages = transcript.messages().to_vec();
            stream.error = None;
        }
        Err(error) => {
            // A partial transcript still yields its handshake, and the tail is
            // an incomplete message rather than a defect. Both are kept.
            stream.handshake = handshake_only.as_ref().map(|(one, _)| one.clone());
            stream.error = Some(error.clone());
        }
    }
    let target_offers_bep10 = stream
        .handshake
        .as_ref()
        .is_some_and(Handshake::offers_extension_protocol);
    let info_hash = stream.handshake.as_ref().map(|one| *one.info_hash());
    let answering = info_hash.is_some() && !stream.answered && role == Role::TargetDialled;

    let mut send = Vec::new();
    // ⛔ The ordinal is allocated here, outside the stream's own borrow, because
    // it belongs to the OBSERVER rather than to this connection: the counter it
    // comes from is the one a dial takes from too.
    if let (true, Some(info_hash)) = (answering, info_hash) {
        let offered = allocate(&mut record, identity, &info_hash);
        send.extend_from_slice(offered.handshake());
        if let Some(stream) = record.streams.get_mut(&connection.get()) {
            stream.answered = true;
            stream.presented = Some(*offered.peer_id());
        }
    }
    // Unreachable: the entry above inserted it under this lock. Answering
    // `NeedMore` rather than unwrapping keeps a later refactor from turning a
    // lookup into a panic inside a lab worker thread.
    let Some(stream) = record.streams.get_mut(&connection.get()) else {
        return StreamReply::NeedMore;
    };
    // ⛔ Only when both sides offered it. Sending an extended handshake to a
    // peer that did not set the bit is this observer inventing a negotiation,
    // and whatever the build did about it would be recorded as identity.
    if let Some(extended) = offer.extension_protocol.handshake()
        && target_offers_bep10
        && stream.handshake.is_some()
        && !stream.extended_sent
    {
        stream.extended_sent = true;
        send.extend_from_slice(&extended.message());
    }
    if send.is_empty() {
        StreamReply::NeedMore
    } else {
        StreamReply::Answer { consumed: 0, send }
    }
}

#[cfg(test)]
mod tests {
    use super::{ORDINAL_DIGITS, ORDINAL_WRAP, PEER_ID_LEN, PeerIdentity, PeerWire};

    const INFO_HASH: [u8; 20] = [0x5a; 20];

    #[test]
    fn the_connection_ordinal_is_the_last_four_bytes_and_the_name_is_the_rest() {
        let identity = PeerIdentity::default();
        assert_eq!(&identity.at(1), b"bit-ids-fixture-0001");
        assert_eq!(&identity.at(2), b"bit-ids-fixture-0002");
        assert_eq!(&identity.at(4242), b"bit-ids-fixture-4242");
        // The name is what a reader recognises the observer by, so it must not
        // move with the ordinal.
        for connection in [1_u32, 2, 9999] {
            assert_eq!(
                &identity.at(connection)[..PEER_ID_LEN - ORDINAL_DIGITS],
                b"bit-ids-fixture-"
            );
        }
    }

    #[test]
    fn the_ordinal_wraps_at_four_digits_rather_than_growing_the_peer_id() {
        let identity = PeerIdentity::default();
        // ⛔ Every derived value is twenty bytes, whatever the ordinal. A
        // formatter that let the tail grow would put a handshake on the wire
        // whose peer ID is the wrong width, which every client refuses.
        for connection in [0_u32, 1, 9999, ORDINAL_WRAP, ORDINAL_WRAP + 1, u32::MAX] {
            assert_eq!(identity.at(connection).len(), PEER_ID_LEN);
        }
        assert_eq!(identity.at(ORDINAL_WRAP), identity.at(0));
        assert_eq!(identity.at(ORDINAL_WRAP + 1), identity.at(1));
    }

    #[test]
    fn a_handshake_carries_the_identity_of_the_connection_it_names() {
        let identity = PeerIdentity::default();
        let first = identity.handshake(&INFO_HASH, 1);
        let second = identity.handshake(&INFO_HASH, 2);
        assert_eq!(first.len(), 68);
        assert_eq!(first[..48], second[..48], "only the peer ID differs");
        assert_eq!(&first[48..68], b"bit-ids-fixture-0001");
        assert_eq!(&second[48..68], b"bit-ids-fixture-0002");
    }

    #[test]
    fn presenting_allocates_a_fresh_identity_every_time() {
        // ⛔ The defect this replaced: `opening()` answered the same bytes
        // however often it was called, so two dials offered one peer.
        let peer = PeerWire::new(PeerIdentity::default(), INFO_HASH);
        let first = peer.present();
        let second = peer.present();
        assert_eq!(first.connection(), 1);
        assert_eq!(second.connection(), 2);
        assert_ne!(first.peer_id(), second.peer_id());
        assert_ne!(first.handshake(), second.handshake());
        assert_eq!(&first.handshake()[48..68], first.peer_id());
        let presented = peer.presented();
        assert_eq!(presented, vec![first, second], "both, in the order taken");
    }

    #[test]
    fn a_second_observer_starts_its_own_ordinals() {
        // ⚠ The counter belongs to the observer rather than to the process, so
        // two labs in one test binary do not interleave their offers.
        let one = PeerWire::new(PeerIdentity::default(), INFO_HASH);
        let other = PeerWire::new(PeerIdentity::default(), INFO_HASH);
        let _ = one.present();
        assert_eq!(other.present().connection(), 1);
    }
}
