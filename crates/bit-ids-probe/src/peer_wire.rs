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
    /// The handshake bytes, answering `info_hash`.
    ///
    /// ⚠ The info hash is echoed rather than chosen. A peer that answers with a
    /// different one is a peer that does not have the torrent, and every client
    /// drops that connection: the observation would then be of this code.
    #[must_use]
    pub fn handshake(&self, info_hash: &[u8; INFO_HASH_LEN]) -> Vec<u8> {
        let mut out = Vec::with_capacity(1 + self.protocol.len() + 48);
        // A protocol string longer than a byte can describe cannot be sent, and
        // truncating one would put a length on the wire that does not match.
        let length = u8::try_from(self.protocol.len()).unwrap_or(u8::MAX);
        out.push(length);
        out.extend_from_slice(&self.protocol[..usize::from(length)]);
        out.extend_from_slice(&self.reserved);
        out.extend_from_slice(info_hash);
        out.extend_from_slice(&self.peer_id);
        out
    }
}

#[derive(Debug, Default)]
struct Record {
    streams: HashMap<u64, Stream>,
    order: Vec<u64>,
    dropped: usize,
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

    /// The handshake to write when the observer is the side that dials.
    ///
    /// ⭐ The dialling side speaks first, so this cannot wait for a responder
    /// call: a responder is only invoked once bytes have arrived, and in this
    /// role none will until the observer has introduced itself.
    #[must_use]
    pub fn opening(&self) -> Vec<u8> {
        self.identity.handshake(&self.info_hash)
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
    pub fn accepting(&self) -> impl Fn(ConnectionId, &[u8]) -> StreamReply + Send + Sync + 'static {
        self.responder(Role::TargetDialled)
    }

    /// The responder for a connection the observer dialled.
    ///
    /// ⭐ Two constructors rather than one taking a [`Role`]. The role decides
    /// who sends the handshake first, so attaching the accepting role to a dial
    /// makes the observer wait for bytes it was supposed to send, and nothing in
    /// a single-constructor signature stops that.
    pub fn dialling(&self) -> impl Fn(ConnectionId, &[u8]) -> StreamReply + Send + Sync + 'static {
        self.responder(Role::ObserverDialled)
    }

    fn responder(
        &self,
        role: Role,
    ) -> impl Fn(ConnectionId, &[u8]) -> StreamReply + Send + Sync + 'static {
        let seen = Arc::clone(&self.seen);
        let identity = self.identity.clone();
        let offer = self.offer.clone();
        let cap = self.max_streams;
        move |connection, buffered: &[u8]| {
            respond(&seen, &identity, &offer, role, cap, connection, buffered)
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
fn respond(
    seen: &Arc<Mutex<Record>>,
    identity: &PeerIdentity,
    offer: &Offer,
    role: Role,
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

    let mut send = Vec::new();
    if let Some(info_hash) = info_hash
        && !stream.answered
        && role == Role::TargetDialled
    {
        stream.answered = true;
        send.extend_from_slice(&identity.handshake(&info_hash));
    }
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
