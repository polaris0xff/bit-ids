//! The peer handshake and the message transcript that follows it.
//!
//! ⛔ **Nothing here maps a peer-ID prefix to a client name, and nothing ever
//! will.** `docs/capture-methodology.md` lists a peer-ID registry or decoder
//! table among the inputs that may seed a hypothesis and may not populate the
//! catalogue. A codec that answered "this is qBittorrent" would be that refused
//! input, arriving through the one component every observer trusts. The peer ID
//! is twenty bytes; what build produced them is what a capture measures.
//!
//! The same rule governs the BEP 10 `v` key. It is a string the peer chose to
//! send about itself, so it is evidence of what the peer *says*, kept as bytes
//! beside everything else it did.

use crate::bencode::{self, Value};
use crate::error::{WireError, be_bytes};

/// The reserved block width, fixed by BEP 3.
pub const RESERVED_LEN: usize = 8;
/// The info-hash width, fixed by BEP 3.
pub const INFO_HASH_LEN: usize = 20;
/// The peer-ID width, fixed by BEP 3.
pub const PEER_ID_LEN: usize = 20;
/// The message id BEP 10 assigns to the extension protocol.
pub const EXTENDED_MESSAGE_ID: u8 = 20;
/// The extended sub-id BEP 10 assigns to the extended handshake.
pub const EXTENDED_HANDSHAKE_ID: u8 = 0;

/// The opening handshake of a peer connection.
///
/// `protocol` is kept as bytes rather than checked against
/// `BitTorrent protocol`. A build that sends a different string has told us
/// something, and a decoder that refused it would turn that observation into a
/// parse failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Handshake {
    protocol: Vec<u8>,
    reserved: [u8; RESERVED_LEN],
    info_hash: [u8; INFO_HASH_LEN],
    peer_id: [u8; PEER_ID_LEN],
}

impl Handshake {
    /// Decodes a handshake and reports how many bytes it consumed.
    ///
    /// # Errors
    ///
    /// Returns `truncated` when the frame is shorter than its own length byte
    /// requires.
    pub fn parse_prefix(input: &[u8]) -> Result<(Self, usize), WireError> {
        let &length = input
            .first()
            .ok_or_else(|| WireError::new("truncated", 0, "handshake has no length byte"))?;
        let length = usize::from(length);
        let protocol_end = 1 + length;
        let protocol = input
            .get(1..protocol_end)
            .ok_or_else(|| {
                WireError::new(
                    "truncated",
                    1,
                    format!("protocol string claims {length} bytes"),
                )
            })?
            .to_vec();
        let reserved = be_bytes::<RESERVED_LEN>(input, protocol_end, "reserved")?;
        let info_hash = be_bytes::<INFO_HASH_LEN>(input, protocol_end + RESERVED_LEN, "info hash")?;
        let peer_id = be_bytes::<PEER_ID_LEN>(
            input,
            protocol_end + RESERVED_LEN + INFO_HASH_LEN,
            "peer id",
        )?;
        let used = protocol_end + RESERVED_LEN + INFO_HASH_LEN + PEER_ID_LEN;
        Ok((
            Self {
                protocol,
                reserved,
                info_hash,
                peer_id,
            },
            used,
        ))
    }

    /// Decodes a handshake that must be the whole input.
    ///
    /// # Errors
    ///
    /// Returns `truncated` for a short frame and `trailing-bytes` when anything
    /// follows the handshake.
    pub fn parse(input: &[u8]) -> Result<Self, WireError> {
        let (handshake, used) = Self::parse_prefix(input)?;
        if used != input.len() {
            return Err(WireError::new(
                "trailing-bytes",
                used,
                format!("{} bytes after the handshake", input.len() - used),
            ));
        }
        Ok(handshake)
    }

    /// The protocol identifier, as sent.
    #[must_use]
    pub fn protocol(&self) -> &[u8] {
        &self.protocol
    }

    /// All eight reserved bytes.
    ///
    /// Whole, never as a set of decoded flags. The bits nobody has assigned are
    /// as much a part of a build's identity as the ones that are named, and a
    /// flags struct would drop every one of them.
    #[must_use]
    pub const fn reserved(&self) -> &[u8; RESERVED_LEN] {
        &self.reserved
    }

    /// The info hash the peer is asking about.
    #[must_use]
    pub const fn info_hash(&self) -> &[u8; INFO_HASH_LEN] {
        &self.info_hash
    }

    /// The twenty peer-ID bytes, undecoded.
    #[must_use]
    pub const fn peer_id(&self) -> &[u8; PEER_ID_LEN] {
        &self.peer_id
    }

    /// Whether the reserved block advertises the BEP 10 extension protocol.
    #[must_use]
    pub const fn offers_extension_protocol(&self) -> bool {
        self.reserved[5] & 0x10 != 0
    }

    /// Whether the reserved block advertises the BEP 5 DHT port message.
    #[must_use]
    pub const fn offers_dht(&self) -> bool {
        self.reserved[7] & 0x01 != 0
    }

    /// Whether the reserved block advertises the BEP 6 fast extension.
    #[must_use]
    pub const fn offers_fast_extension(&self) -> bool {
        self.reserved[7] & 0x04 != 0
    }

    /// Writes the handshake back.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(1 + self.protocol.len() + 48);
        // A protocol string longer than 255 bytes cannot have been decoded,
        // because the length arrived in one byte. Truncation here is therefore
        // unreachable for a parsed handshake, and wrong for a constructed one:
        // the cast is guarded rather than assumed.
        let length = u8::try_from(self.protocol.len()).unwrap_or(u8::MAX);
        out.push(length);
        out.extend_from_slice(&self.protocol[..usize::from(length)]);
        out.extend_from_slice(&self.reserved);
        out.extend_from_slice(&self.info_hash);
        out.extend_from_slice(&self.peer_id);
        out
    }
}

/// One length-prefixed peer message.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Message {
    /// A zero-length message, the keep-alive.
    KeepAlive,
    /// Any other message: an id byte and the payload after it.
    ///
    /// An id nobody has assigned is kept with its payload rather than refused.
    /// An unknown message is one of the more informative things a build can do.
    Typed {
        /// The message id.
        id: u8,
        /// Everything after the id byte.
        payload: Vec<u8>,
    },
}

impl Message {
    /// The largest message this will decode, in bytes.
    ///
    /// A `piece` message for a 16 `KiB` block plus its header is the largest a
    /// well-behaved peer sends. The cap is generous against that and finite
    /// against a peer that sends `0xffffffff` as a length.
    pub const MAX_LEN: usize = 1 << 20;

    /// The id, or `None` for a keep-alive.
    #[must_use]
    pub const fn id(&self) -> Option<u8> {
        match self {
            Self::KeepAlive => None,
            Self::Typed { id, .. } => Some(*id),
        }
    }

    /// The payload, empty for a keep-alive or an id-only message.
    #[must_use]
    pub fn payload(&self) -> &[u8] {
        match self {
            Self::KeepAlive => &[],
            Self::Typed { payload, .. } => payload,
        }
    }

    /// Decodes one message and reports how many bytes it consumed.
    ///
    /// # Errors
    ///
    /// Returns `truncated` for a short frame and `message-too-long` for a
    /// length past [`Message::MAX_LEN`].
    pub fn parse_prefix(input: &[u8]) -> Result<(Self, usize), WireError> {
        let length = u32::from_be_bytes(be_bytes::<4>(input, 0, "message length")?) as usize;
        if length > Self::MAX_LEN {
            return Err(WireError::new(
                "message-too-long",
                0,
                format!("length {length} exceeds the {}-byte cap", Self::MAX_LEN),
            ));
        }
        if length == 0 {
            return Ok((Self::KeepAlive, 4));
        }
        let end = 4 + length;
        let body = input.get(4..end).ok_or_else(|| {
            WireError::new(
                "truncated",
                4,
                format!(
                    "message claims {length} bytes, {} remain",
                    input.len().saturating_sub(4)
                ),
            )
        })?;
        Ok((
            Self::Typed {
                id: body[0],
                payload: body[1..].to_vec(),
            },
            end,
        ))
    }

    /// Writes the message back.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        match self {
            Self::KeepAlive => out.extend_from_slice(&0_u32.to_be_bytes()),
            Self::Typed { id, payload } => {
                // Guarded rather than assumed: a payload wider than u32 cannot
                // have been decoded, and saturating is the honest answer for a
                // value constructed in memory.
                let length = u32::try_from(payload.len() + 1).unwrap_or(u32::MAX);
                out.extend_from_slice(&length.to_be_bytes());
                out.push(*id);
                out.extend_from_slice(payload);
            }
        }
        out
    }

    /// Reads the message as a BEP 10 extended message.
    ///
    /// Returns `None` when this is not message id [`EXTENDED_MESSAGE_ID`].
    ///
    /// # Errors
    ///
    /// Returns `truncated` when the extended sub-id is missing, or the bencode
    /// error from the payload with its offset re-based onto this message.
    #[must_use]
    pub fn as_extended(&self) -> Option<Result<ExtendedMessage, WireError>> {
        let Self::Typed { id, payload } = self else {
            return None;
        };
        if *id != EXTENDED_MESSAGE_ID {
            return None;
        }
        let Some((&extended_id, rest)) = payload.split_first() else {
            return Some(Err(WireError::new(
                "truncated",
                5,
                "extended message has no sub-id",
            )));
        };
        // ⚠ Offsets are re-based onto the transcript frame. `byte 3` of a
        // payload names nothing a reader can find in an evidence bundle.
        let document = match bencode::decode(rest) {
            Ok(document) => document,
            Err(error) => return Some(Err(error.at_base(6))),
        };
        Some(Ok(ExtendedMessage {
            extended_id,
            document,
            raw: rest.to_vec(),
        }))
    }
}

/// A decoded BEP 10 extended message.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtendedMessage {
    extended_id: u8,
    document: Value,
    raw: Vec<u8>,
}

impl ExtendedMessage {
    /// The extension id, `0` for the extended handshake.
    #[must_use]
    pub const fn extended_id(&self) -> u8 {
        self.extended_id
    }

    /// Whether this is the extended handshake.
    #[must_use]
    pub const fn is_handshake(&self) -> bool {
        self.extended_id == EXTENDED_HANDSHAKE_ID
    }

    /// The decoded payload.
    #[must_use]
    pub const fn document(&self) -> &Value {
        &self.document
    }

    /// The undecoded payload bytes.
    ///
    /// Kept beside the decoded document because the dictionary is the evidence
    /// and the decode is an interpretation of it.
    #[must_use]
    pub fn raw(&self) -> &[u8] {
        &self.raw
    }

    /// The extension name to id map the peer advertised, in the order sent.
    ///
    /// Empty when there is no `m` dictionary. Order is a real difference
    /// between builds, so this is a list of pairs rather than a map.
    #[must_use]
    pub fn extension_ids(&self) -> Vec<(Vec<u8>, i64)> {
        let Some(Value::Dictionary(entries)) = self.document.get(b"m") else {
            return Vec::new();
        };
        entries
            .iter()
            .filter_map(|(name, value)| match value {
                Value::Integer(number) => Some((name.clone(), number.to_i64()?)),
                _ => None,
            })
            .collect()
    }

    /// The `v` string, which is what the peer says it is.
    ///
    /// ⚠ Evidence of a claim, not a measured identity. Nothing in this crate
    /// treats it as authoritative about the build.
    #[must_use]
    pub fn advertised_client(&self) -> Option<&[u8]> {
        match self.document.get(b"v") {
            Some(Value::Bytes(bytes)) => Some(bytes),
            _ => None,
        }
    }

    /// An integer the handshake carried at the top level, such as `reqq` or `p`.
    #[must_use]
    pub fn integer(&self, key: &[u8]) -> Option<i64> {
        match self.document.get(key) {
            Some(Value::Integer(number)) => number.to_i64(),
            _ => None,
        }
    }
}

/// A handshake followed by the messages that came after it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transcript {
    handshake: Handshake,
    messages: Vec<Message>,
}

impl Transcript {
    /// Decodes a handshake and every whole message after it.
    ///
    /// # Errors
    ///
    /// Returns the first [`WireError`] any part produced, with its offset
    /// re-based onto the whole transcript.
    pub fn parse(input: &[u8]) -> Result<Self, WireError> {
        let (handshake, mut cursor) = Handshake::parse_prefix(input)?;
        let mut messages = Vec::new();
        while cursor < input.len() {
            let (message, used) =
                Message::parse_prefix(&input[cursor..]).map_err(|error| error.at_base(cursor))?;
            messages.push(message);
            cursor += used;
        }
        Ok(Self {
            handshake,
            messages,
        })
    }

    /// The opening handshake.
    #[must_use]
    pub const fn handshake(&self) -> &Handshake {
        &self.handshake
    }

    /// The messages, in the order they arrived.
    ///
    /// ⭐ Order is the measurement here. `docs/architecture.md` section 5 names
    /// early message order as an identity field, so this is a sequence and
    /// there is no by-id lookup that would let a caller forget that.
    #[must_use]
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// The first extended handshake in the transcript, when there is one.
    ///
    /// # Errors
    ///
    /// Returns the decode error of the first extended message that fails to
    /// decode, even when a later one would have succeeded. A transcript with a
    /// malformed extension dictionary is a finding.
    pub fn extended_handshake(&self) -> Result<Option<ExtendedMessage>, WireError> {
        let mut offset = self.handshake.encode().len();
        for message in &self.messages {
            if let Some(extended) = message.as_extended() {
                let extended = extended.map_err(|error| error.at_base(offset))?;
                if extended.is_handshake() {
                    return Ok(Some(extended));
                }
            }
            offset += message.encode().len();
        }
        Ok(None)
    }

    /// Writes the transcript back.
    ///
    /// ⛔ Byte for byte what [`Transcript::parse`] read.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = self.handshake.encode();
        for message in &self.messages {
            out.extend_from_slice(&message.encode());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::{EXTENDED_MESSAGE_ID, Handshake, Message, Transcript};

    fn handshake_bytes(reserved: [u8; 8]) -> Vec<u8> {
        let mut out = vec![19];
        out.extend_from_slice(b"BitTorrent protocol");
        out.extend_from_slice(&reserved);
        out.extend_from_slice(&[0x11; 20]);
        out.extend_from_slice(b"bit-ids-fixture-0001");
        out
    }

    #[test]
    fn every_reserved_byte_survives_including_the_unassigned_ones() {
        let reserved = [0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x05];
        let raw = handshake_bytes(reserved);
        let handshake = Handshake::parse(&raw).expect("decodes");
        assert_eq!(handshake.reserved(), &reserved);
        assert!(handshake.offers_extension_protocol());
        assert!(handshake.offers_dht());
        assert!(handshake.offers_fast_extension());
        assert_eq!(handshake.peer_id(), b"bit-ids-fixture-0001");
        assert_eq!(handshake.encode(), raw);
    }

    #[test]
    fn a_non_standard_protocol_string_is_recorded_rather_than_refused() {
        let mut raw = vec![7];
        raw.extend_from_slice(b"OtherPr");
        raw.extend_from_slice(&[0_u8; 8]);
        raw.extend_from_slice(&[0x22; 20]);
        raw.extend_from_slice(b"bit-ids-fixture-0002");
        let handshake = Handshake::parse(&raw).expect("the string is evidence, not a rule");
        assert_eq!(handshake.protocol(), b"OtherPr");
        assert_eq!(handshake.encode(), raw);
    }

    #[test]
    fn an_unknown_message_id_keeps_its_payload() {
        let raw = [0, 0, 0, 3, 200, 1, 2];
        let (message, used) = Message::parse_prefix(&raw).expect("decodes");
        assert_eq!((message.id(), message.payload()), (Some(200), &[1, 2][..]));
        assert_eq!(used, 7);
        assert_eq!(message.encode(), raw);
    }

    #[test]
    fn an_absurd_message_length_is_refused_before_it_is_allocated() {
        let raw = [0xff, 0xff, 0xff, 0xff, 0x00];
        assert_eq!(
            Message::parse_prefix(&raw)
                .expect_err("nothing is that long")
                .kind(),
            "message-too-long"
        );
    }

    #[test]
    fn an_extended_handshake_reports_its_map_in_the_order_it_arrived() {
        let payload = b"d1:md11:ut_metadatai2e6:ut_pexi1ee1:pi6881e4:reqqi500e1:v11:fixture/0.0e";
        let mut body = vec![EXTENDED_MESSAGE_ID, 0];
        body.extend_from_slice(payload);
        let mut raw = handshake_bytes([0, 0, 0, 0, 0, 0x10, 0, 0]);
        let length = u32::try_from(body.len()).expect("small");
        raw.extend_from_slice(&length.to_be_bytes());
        raw.extend_from_slice(&body);

        let transcript = Transcript::parse(&raw).expect("decodes");
        let extended = transcript
            .extended_handshake()
            .expect("the dictionary decodes")
            .expect("there is one");
        let names: Vec<Vec<u8>> = extended
            .extension_ids()
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        assert_eq!(names, vec![b"ut_metadata".to_vec(), b"ut_pex".to_vec()]);
        assert_eq!(extended.advertised_client(), Some(&b"fixture/0.0"[..]));
        assert_eq!(extended.integer(b"reqq"), Some(500));
        assert_eq!(extended.integer(b"p"), Some(6881));
        assert_eq!(transcript.encode(), raw);
    }

    #[test]
    fn a_keep_alive_is_a_message_and_not_an_absence() {
        let mut raw = handshake_bytes([0_u8; 8]);
        raw.extend_from_slice(&[0, 0, 0, 0]);
        raw.extend_from_slice(&[0, 0, 0, 1, 1]);
        let transcript = Transcript::parse(&raw).expect("decodes");
        assert_eq!(transcript.messages().len(), 2);
        assert_eq!(transcript.messages()[0], Message::KeepAlive);
        assert_eq!(transcript.messages()[1].id(), Some(1));
        assert_eq!(transcript.encode(), raw);
    }

    #[test]
    fn a_truncated_message_names_its_offset_in_the_whole_transcript() {
        let mut raw = handshake_bytes([0_u8; 8]);
        raw.extend_from_slice(&[0, 0, 0, 9, 1]);
        let error = Transcript::parse(&raw).expect_err("the last message is cut off");
        assert_eq!(error.kind(), "truncated");
        assert_eq!(error.offset(), 68 + 4);
    }

    #[test]
    fn anything_after_a_lone_handshake_is_a_finding() {
        let mut raw = handshake_bytes([0_u8; 8]);
        raw.push(0);
        assert_eq!(
            Handshake::parse(&raw)
                .expect_err("one handshake only")
                .kind(),
            "trailing-bytes"
        );
    }
}
