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
use bit_ids_wire::peer_wire::{
    Handshake, INFO_HASH_LEN, Message, PEER_ID_LEN, RESERVED_LEN, Transcript,
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
    /// `OBS-05` turns on BEP 10 deliberately and records that it did.
    fn default() -> Self {
        Self {
            protocol: b"BitTorrent protocol".to_vec(),
            reserved: [0; RESERVED_LEN],
            peer_id: *b"bit-ids-fixture-0001",
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
            info_hash,
            max_streams: DEFAULT_MAX_STREAMS,
        }
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
        let cap = self.max_streams;
        move |connection, buffered: &[u8]| {
            respond(&seen, &identity, role, cap, connection, buffered)
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
    let answered = stream.answered;
    let info_hash = stream.handshake.as_ref().map(|one| *one.info_hash());
    if let Some(info_hash) = info_hash
        && !answered
        && role == Role::TargetDialled
    {
        stream.answered = true;
        return StreamReply::Answer {
            consumed: 0,
            send: identity.handshake(&info_hash),
        };
    }
    StreamReply::NeedMore
}
