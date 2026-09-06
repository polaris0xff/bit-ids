//! The DHT wire surface: BEP 5's KRPC, read without losing what a build chose.
//!
//! A KRPC message is one bencoded dictionary in one datagram, so this module is
//! thin on top of [`crate::bencode`] and carries the whole decoded [`Value`]
//! rather than a struct of extracted fields. ⛔ **That is the round-trip
//! invariant, not a shortcut**: `docs/architecture.md` section 5 says a parsed
//! view retains what the sender chose, and a KRPC message's key order, its
//! transaction-id width, its integer spelling and the extra keys a build invents
//! are all exactly that. A struct of named fields could not write any of them
//! back.
//!
//! # What is identity here
//!
//! Almost everything, which is why the surface is worth observing at all:
//!
//! - the **node id** a build picks for itself, twenty bytes in `a.id` or `r.id`.
//!   BEP 42 asks that it be derived from the node's own address, and whether a
//!   build does that is a difference between builds;
//! - the **`v` string**, BEP 5's optional client-version field. ⛔ It is
//!   recorded as bytes and never decoded to a client name, for the reason
//!   `lib.rs` gives about peer-ID prefixes: this crate is the one component
//!   every observer trusts, and a decoder table inside it would put a refused
//!   input there;
//! - the **transaction id**, whose width and shape a build fixes for itself. Two
//!   bytes, four bytes and printable ASCII are all in the wild;
//! - the **key order** of the message and of its arguments, which BEP 3 says is
//!   sorted and implementations disagree about;
//! - which **queries** a build sends unprompted, and the optional arguments it
//!   attaches: `want`, `implied_port`, `ro` from BEP 43, `noseed` and `scrape`
//!   from BEP 33.
//!
//! # It observes and refuses nothing
//!
//! ⛔ **A message that is not what BEP 5 describes still decodes.** [`Message`]
//! reads whatever bencode it is given; [`Message::departures`] is where the
//! reading says what is unusual about it, and the caller keeps both. A codec
//! that refused a query with no `y` key would turn the observation *this build
//! omits `y`* into a parse failure, and the bytes that prove it would be
//! reported as a decode error rather than as a finding.
//!
//! The one thing that is an error is bencode that does not decode, and that
//! refusal is [`crate::bencode`]'s.
//!
//! ⚠ **A datagram is the frame.** There is no length prefix and no continuation:
//! one packet is one message, and trailing bytes after the dictionary are a
//! [`Departure::TrailingBytes`] rather than a second message.

use crate::bencode::{self, Value};
use crate::error::WireError;

/// The key BEP 5 gives the transaction identifier.
pub const KEY_TRANSACTION: &[u8] = b"t";

/// The key BEP 5 gives the message type.
pub const KEY_TYPE: &[u8] = b"y";

/// The key BEP 5 gives a query's method name.
pub const KEY_METHOD: &[u8] = b"q";

/// The key BEP 5 gives a query's arguments.
pub const KEY_ARGUMENTS: &[u8] = b"a";

/// The key BEP 5 gives a response's return values.
pub const KEY_RETURN: &[u8] = b"r";

/// The key BEP 5 gives an error's code and message.
pub const KEY_ERROR: &[u8] = b"e";

/// The key BEP 5 gives the optional client version.
pub const KEY_VERSION: &[u8] = b"v";

/// The key a node identifier is carried under, inside `a` or `r`.
pub const KEY_NODE_ID: &[u8] = b"id";

/// How many bytes BEP 5 gives a node identifier.
///
/// ⭐ Pinned to its literal rather than derived from anything, because a
/// constant every test reads is a constant no test can check: narrowing it would
/// re-measure the value and the comparison against it together. `OBS-08` found
/// two of that shape.
pub const NODE_ID_LEN: usize = 20;

/// How many bytes one IPv4 node in a compact `nodes` string occupies: a
/// twenty-byte identifier, four address bytes and a two-byte port.
pub const COMPACT_NODE_V4_LEN: usize = NODE_ID_LEN + 4 + 2;

/// How many bytes one IPv6 node in a compact `nodes6` string occupies.
pub const COMPACT_NODE_V6_LEN: usize = NODE_ID_LEN + 16 + 2;

/// What BEP 5's `y` key said this message is.
///
/// ⚠ [`Kind::Other`] and [`Kind::Absent`] are states rather than errors. A build
/// that spells the type something else, or omits it, has told us something.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Kind {
    /// `y` is `q`: a query.
    Query,
    /// `y` is `r`: a response.
    Response,
    /// `y` is `e`: an error.
    Error,
    /// `y` is present and is none of the three, kept as it arrived.
    Other(Vec<u8>),
    /// There is no `y` key at all.
    Absent,
}

impl Kind {
    /// Reads the `y` value of a decoded message.
    fn of(document: &Value) -> Self {
        match document.get(KEY_TYPE) {
            None => Self::Absent,
            Some(Value::Bytes(value)) => match value.as_slice() {
                b"q" => Self::Query,
                b"r" => Self::Response,
                b"e" => Self::Error,
                other => Self::Other(other.to_vec()),
            },
            // ⚠ A `y` that is not a byte string is still a `y`. Recording it as
            // absent would lose the difference between a build that omits the
            // key and one that puts an integer there.
            Some(other) => Self::Other(other.type_name().as_bytes().to_vec()),
        }
    }
}

/// Something about a message that BEP 5 does not describe.
///
/// ⚠ **Every one of these is a finding about the build, not a reason to drop
/// the message.** They are reported all at once rather than at the first,
/// because the set of things a build gets wrong is more identifying than
/// whichever one a short-circuiting reader stopped at. `OBS-06` established that
/// shape for local discovery.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Departure {
    /// The datagram is not a bencoded dictionary at the top level.
    NotADictionary(&'static str),
    /// Bytes followed the dictionary inside one datagram.
    TrailingBytes(Vec<u8>),
    /// No `t` key, so nothing can be matched to a reply.
    NoTransaction,
    /// A `t` that is not a byte string.
    TransactionNotBytes(&'static str),
    /// No `y` key, so the message does not say what it is.
    NoType,
    /// A `y` that is none of `q`, `r` or `e`.
    UnknownType(Vec<u8>),
    /// A query with no `q` key, so it names no method.
    NoMethod,
    /// A query with no `a` dictionary, or a response with no `r` dictionary.
    NoPayload,
    /// The payload key is present and is not a dictionary.
    PayloadNotADictionary(&'static str),
    /// No `id` inside the payload, so the message names no node.
    NoNodeId,
    /// An `id` that is not exactly [`NODE_ID_LEN`] bytes.
    NodeIdWrongLength(usize),
    /// An error message whose `e` is not a list of at least a code and a text.
    ErrorNotAPair,
    /// The message's own keys are not in the order BEP 3 fixes.
    KeysUnsorted,
    /// The message carries the same key twice.
    DuplicateKey,
}

impl Departure {
    /// The departure in one line.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::NotADictionary(found) => format!("the message is a {found}, not a dictionary"),
            Self::TrailingBytes(rest) => {
                format!("{} bytes followed the dictionary", rest.len())
            }
            Self::NoTransaction => "no t key".to_owned(),
            Self::TransactionNotBytes(found) => format!("t is a {found}, not a byte string"),
            Self::NoType => "no y key".to_owned(),
            Self::UnknownType(value) => {
                format!("y is {:?}, not q, r or e", String::from_utf8_lossy(value))
            }
            Self::NoMethod => "a query with no q key".to_owned(),
            Self::NoPayload => "no a or r dictionary".to_owned(),
            Self::PayloadNotADictionary(found) => {
                format!("the payload is a {found}, not a dictionary")
            }
            Self::NoNodeId => format!("no {} in the payload", String::from_utf8_lossy(KEY_NODE_ID)),
            Self::NodeIdWrongLength(len) => {
                format!("the node id is {len} bytes, not {NODE_ID_LEN}")
            }
            Self::ErrorNotAPair => "e is not a list of a code and a message".to_owned(),
            Self::KeysUnsorted => "the message keys are not sorted".to_owned(),
            Self::DuplicateKey => "the message carries a key twice".to_owned(),
        }
    }
}

/// One KRPC message, as it arrived.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Message {
    raw: Vec<u8>,
    document: Value,
    trailing: Vec<u8>,
}

impl Message {
    /// Reads one datagram as a KRPC message.
    ///
    /// ⚠ Only bencode that does not decode is an error. Everything else about
    /// the message is reported by [`Message::departures`], including a top-level
    /// value that is not a dictionary at all.
    ///
    /// # Errors
    ///
    /// Returns whatever [`crate::bencode::decode_prefix`] refused.
    pub fn parse(datagram: &[u8]) -> Result<Self, WireError> {
        let (document, used) = bencode::decode_prefix(datagram)?;
        Ok(Self {
            raw: datagram.to_vec(),
            document,
            trailing: datagram[used..].to_vec(),
        })
    }

    /// Writes the message back.
    ///
    /// ⛔ **This is the invariant the fixture corpus asserts**: the trailing
    /// bytes are appended because they were in the datagram, so a message whose
    /// sender put something after the dictionary re-encodes to what it sent
    /// rather than to a tidied version of it.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = bencode::encode(&self.document);
        out.extend_from_slice(&self.trailing);
        out
    }

    /// The datagram exactly as it arrived.
    #[must_use]
    pub fn raw(&self) -> &[u8] {
        &self.raw
    }

    /// The decoded document, with key order and integer spelling intact.
    #[must_use]
    pub const fn document(&self) -> &Value {
        &self.document
    }

    /// Anything that followed the dictionary in the same datagram.
    #[must_use]
    pub fn trailing(&self) -> &[u8] {
        &self.trailing
    }

    /// What `y` said this is.
    #[must_use]
    pub fn kind(&self) -> Kind {
        Kind::of(&self.document)
    }

    /// The transaction identifier, as bytes.
    ///
    /// ⚠ Bytes rather than a number. A build choosing two bytes, four bytes or
    /// printable ASCII is the observation, and a numeric reading would erase the
    /// width along with any value that is not a number.
    #[must_use]
    pub fn transaction_id(&self) -> Option<&[u8]> {
        match self.document.get(KEY_TRANSACTION) {
            Some(Value::Bytes(value)) => Some(value.as_slice()),
            _ => None,
        }
    }

    /// The method a query names.
    #[must_use]
    pub fn method(&self) -> Option<&[u8]> {
        match self.document.get(KEY_METHOD) {
            Some(Value::Bytes(value)) => Some(value.as_slice()),
            _ => None,
        }
    }

    /// BEP 5's optional client version string, as bytes.
    ///
    /// ⛔ **Never mapped to a client name.** `lib.rs` says why: this crate is
    /// the one component every observer trusts, and `capture-methodology.md`
    /// lists a decoder table among the inputs that may seed a hypothesis and may
    /// not populate the catalogue.
    #[must_use]
    pub fn version(&self) -> Option<&[u8]> {
        match self.document.get(KEY_VERSION) {
            Some(Value::Bytes(value)) => Some(value.as_slice()),
            _ => None,
        }
    }

    /// The arguments of a query, or the return values of a response.
    ///
    /// ⚠ Keyed on what the message actually carries rather than on its declared
    /// kind, so a message whose `y` and payload key disagree is still readable.
    /// That disagreement is itself a departure and is reported as one.
    #[must_use]
    pub fn payload(&self) -> Option<&Value> {
        self.document
            .get(KEY_ARGUMENTS)
            .or_else(|| self.document.get(KEY_RETURN))
    }

    /// The node identifier the message carries for its sender.
    #[must_use]
    pub fn node_id(&self) -> Option<&[u8]> {
        match self.payload()?.get(KEY_NODE_ID) {
            Some(Value::Bytes(value)) => Some(value.as_slice()),
            _ => None,
        }
    }

    /// One value out of the payload, by key.
    #[must_use]
    pub fn argument(&self, key: &[u8]) -> Option<&Value> {
        self.payload()?.get(key)
    }

    /// The payload's keys, in the order the build wrote them.
    ///
    /// ⭐ One of the stronger signals on this surface. BEP 5 lists a query's
    /// arguments and fixes no order for them, and a build is consistent with
    /// itself.
    #[must_use]
    pub fn argument_order(&self) -> Vec<Vec<u8>> {
        match self.payload() {
            Some(Value::Dictionary(entries)) => {
                entries.iter().map(|(key, _)| key.clone()).collect()
            }
            _ => Vec::new(),
        }
    }

    /// The message's own keys, in the order the build wrote them.
    #[must_use]
    pub fn key_order(&self) -> Vec<Vec<u8>> {
        match &self.document {
            Value::Dictionary(entries) => entries.iter().map(|(key, _)| key.clone()).collect(),
            _ => Vec::new(),
        }
    }

    /// Everything about this message BEP 5 does not describe.
    ///
    /// Empty for a message that matches the specification.
    #[must_use]
    pub fn departures(&self) -> Vec<Departure> {
        let mut found = Vec::new();
        let Value::Dictionary(_) = &self.document else {
            found.push(Departure::NotADictionary(self.document.type_name()));
            if !self.trailing.is_empty() {
                found.push(Departure::TrailingBytes(self.trailing.clone()));
            }
            return found;
        };
        if !self.trailing.is_empty() {
            found.push(Departure::TrailingBytes(self.trailing.clone()));
        }
        match self.document.get(KEY_TRANSACTION) {
            None => found.push(Departure::NoTransaction),
            Some(Value::Bytes(_)) => {}
            Some(other) => found.push(Departure::TransactionNotBytes(other.type_name())),
        }
        let kind = self.kind();
        match &kind {
            Kind::Absent => found.push(Departure::NoType),
            Kind::Other(value) => found.push(Departure::UnknownType(value.clone())),
            Kind::Query | Kind::Response | Kind::Error => {}
        }
        if kind == Kind::Query && self.method().is_none() {
            found.push(Departure::NoMethod);
        }
        self.check_payload(&kind, &mut found);
        if kind == Kind::Error {
            let pair = match self.document.get(KEY_ERROR) {
                Some(Value::List(items)) => items.len() >= 2,
                _ => false,
            };
            if !pair {
                found.push(Departure::ErrorNotAPair);
            }
        }
        if self.document.keys_are_sorted() == Some(false) {
            found.push(Departure::KeysUnsorted);
        }
        if self.document.has_duplicate_keys() == Some(true) {
            found.push(Departure::DuplicateKey);
        }
        found
    }

    /// The payload half of [`Message::departures`].
    ///
    /// ⚠ An error message carries neither `a` nor `r`, so it is exempt from the
    /// payload rules rather than reported as missing one.
    fn check_payload(&self, kind: &Kind, found: &mut Vec<Departure>) {
        if *kind == Kind::Error {
            return;
        }
        let key = if *kind == Kind::Response {
            KEY_RETURN
        } else {
            KEY_ARGUMENTS
        };
        match self.document.get(key) {
            None => {
                found.push(Departure::NoPayload);
                return;
            }
            Some(Value::Dictionary(_)) => {}
            Some(other) => {
                found.push(Departure::PayloadNotADictionary(other.type_name()));
                return;
            }
        }
        match self.node_id() {
            None => found.push(Departure::NoNodeId),
            Some(id) if id.len() != NODE_ID_LEN => {
                found.push(Departure::NodeIdWrongLength(id.len()));
            }
            Some(_) => {}
        }
    }

    /// Whether this message is one BEP 5 describes.
    #[must_use]
    pub fn is_conforming(&self) -> bool {
        self.departures().is_empty()
    }
}

/// How many whole nodes a compact node string holds, and the remainder.
///
/// ⚠ **The remainder is returned rather than swallowed.** A `nodes` value that
/// is not a whole number of entries is a finding about the build, and a reader
/// that divided and discarded would report a shorter list with nothing saying
/// bytes were left over. `peer_exchange` learned this on its own stride check.
#[must_use]
pub const fn compact_nodes(len: usize, stride: usize) -> (usize, usize) {
    if stride == 0 {
        return (0, len);
    }
    (len / stride, len % stride)
}

#[cfg(test)]
mod tests {
    use super::{
        COMPACT_NODE_V4_LEN, COMPACT_NODE_V6_LEN, Departure, Kind, Message, NODE_ID_LEN,
        compact_nodes,
    };
    use crate::bencode::{self, Value};

    /// Twenty bytes, which is what BEP 5 fixes a node id at.
    const ID: &[u8] = b"abcdefghij0123456789";

    /// A `find_node` query with sorted keys, exactly as BEP 5 describes one.
    fn find_node() -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"d1:ad2:id20:");
        out.extend_from_slice(ID);
        out.extend_from_slice(b"6:target20:");
        out.extend_from_slice(b"mnopqrstuvwxyz123456");
        out.extend_from_slice(b"e1:q9:find_node1:t2:aa1:y1:qe");
        out
    }

    #[test]
    fn a_conforming_query_reads_and_writes_back_byte_for_byte() {
        let bytes = find_node();
        let message = Message::parse(&bytes).expect("it is bencode");
        assert!(message.is_conforming(), "{:?}", message.departures());
        assert_eq!(message.kind(), Kind::Query);
        assert_eq!(message.method(), Some(&b"find_node"[..]));
        assert_eq!(message.transaction_id(), Some(&b"aa"[..]));
        assert_eq!(message.node_id(), Some(ID));
        assert_eq!(message.version(), None);
        assert_eq!(
            message.argument_order(),
            vec![b"id".to_vec(), b"target".to_vec()]
        );
        assert_eq!(
            message.key_order(),
            vec![b"a".to_vec(), b"q".to_vec(), b"t".to_vec(), b"y".to_vec()]
        );
        // ⛔ The invariant the whole crate rests on.
        assert_eq!(message.encode(), bytes);
        assert_eq!(message.raw(), bytes);
    }

    /// ⭐ The version string is bytes and stays bytes. A build that writes a
    /// four-byte tag with a control character in it is the ordinary case, and it
    /// is never turned into a client name.
    #[test]
    fn the_client_version_is_kept_as_bytes_and_never_named() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"d1:rd2:id20:");
        bytes.extend_from_slice(ID);
        bytes.extend_from_slice(b"e1:t2:aa1:v4:UT\x01\x021:y1:re");
        let message = Message::parse(&bytes).expect("it is bencode");
        assert!(message.is_conforming(), "{:?}", message.departures());
        assert_eq!(message.kind(), Kind::Response);
        assert_eq!(message.version(), Some(&b"UT\x01\x02"[..]));
        assert_eq!(message.encode(), bytes);
    }

    /// ⚠ Unsorted keys and a non-canonical integer are recorded, not refused.
    /// Both are what a build chose, and a codec that tidied them would erase
    /// the difference between two builds.
    #[test]
    fn a_build_that_writes_its_keys_out_of_order_is_recorded_and_rebuilt() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"d1:y1:q1:t2:zz1:q9:get_peers1:ad2:id20:");
        bytes.extend_from_slice(ID);
        bytes.extend_from_slice(b"9:info_hash20:");
        bytes.extend_from_slice(b"mnopqrstuvwxyz123456");
        bytes.extend_from_slice(b"12:implied_porti01eee");
        let message = Message::parse(&bytes).expect("it is bencode");
        assert_eq!(message.kind(), Kind::Query);
        assert_eq!(message.method(), Some(&b"get_peers"[..]));
        assert!(
            message.departures().contains(&Departure::KeysUnsorted),
            "{:?}",
            message.departures()
        );
        // ⛔ And `i01e` survives, which is what proves nothing normalised it.
        assert_eq!(message.encode(), bytes);
        let Some(Value::Integer(implied)) = message.argument(b"implied_port") else {
            panic!("implied_port is an integer");
        };
        assert_eq!(implied.as_str(), "01");
        assert!(!implied.is_canonical());
    }

    #[test]
    fn every_departure_is_reported_rather_than_the_first() {
        // No `t`, an unknown `y`, no payload, and a duplicate key, at once.
        let bytes = b"d1:y1:x1:y1:xe";
        let message = Message::parse(bytes).expect("it is bencode");
        let found = message.departures();
        assert!(found.contains(&Departure::NoTransaction), "{found:?}");
        assert!(
            found.iter().any(|d| matches!(d, Departure::UnknownType(_))),
            "{found:?}"
        );
        assert!(found.contains(&Departure::NoPayload), "{found:?}");
        assert!(found.contains(&Departure::DuplicateKey), "{found:?}");
        assert_eq!(message.encode(), bytes);
    }

    /// ⚠ A `y` that is not a byte string is not the same observation as a
    /// missing `y`, and collapsing them would lose the difference.
    #[test]
    fn a_type_key_of_the_wrong_shape_is_not_read_as_a_missing_one() {
        let present = Message::parse(b"d1:ti1e1:yi3ee").expect("it is bencode");
        assert!(matches!(present.kind(), Kind::Other(_)));
        assert!(
            !present.departures().contains(&Departure::NoType),
            "{:?}",
            present.departures()
        );
        let absent = Message::parse(b"d1:t2:aae").expect("it is bencode");
        assert_eq!(absent.kind(), Kind::Absent);
        assert!(absent.departures().contains(&Departure::NoType));
        // ⚠ And `t` of the wrong type is its own finding rather than a missing t.
        assert!(
            present
                .departures()
                .iter()
                .any(|d| matches!(d, Departure::TransactionNotBytes(_)))
        );
        assert!(!present.departures().contains(&Departure::NoTransaction));
    }

    #[test]
    fn a_node_id_of_the_wrong_width_is_reported_with_the_width() {
        let bytes = b"d1:ad2:id3:abce1:q4:ping1:t2:aa1:y1:qe";
        let message = Message::parse(bytes).expect("it is bencode");
        assert!(
            message
                .departures()
                .contains(&Departure::NodeIdWrongLength(3)),
            "{:?}",
            message.departures()
        );
        assert_eq!(message.encode(), bytes);
    }

    /// ⛔ One datagram is one message. Bytes after the dictionary are a finding
    /// and are written back, rather than being read as a second message or
    /// dropped.
    #[test]
    fn bytes_after_the_dictionary_are_kept_and_written_back() {
        let mut bytes = find_node();
        bytes.extend_from_slice(b"junk");
        let message = Message::parse(&bytes).expect("the prefix is bencode");
        assert_eq!(message.trailing(), b"junk");
        assert!(
            message
                .departures()
                .contains(&Departure::TrailingBytes(b"junk".to_vec()))
        );
        assert_eq!(message.encode(), bytes);
    }

    /// ⚠ A top-level value that is not a dictionary decodes. Refusing it would
    /// turn "this build sent a list" into a parse error.
    #[test]
    fn a_message_that_is_not_a_dictionary_still_decodes_and_says_so() {
        let message = Message::parse(b"li1ei2ee").expect("it is bencode");
        assert!(
            message
                .departures()
                .contains(&Departure::NotADictionary("list"))
        );
        assert_eq!(message.transaction_id(), None);
        assert_eq!(message.payload(), None);
        assert_eq!(message.key_order(), Vec::<Vec<u8>>::new());
        assert_eq!(message.encode(), b"li1ei2ee");
    }

    #[test]
    fn bencode_that_does_not_decode_is_the_one_refusal() {
        assert!(Message::parse(b"d1:t").is_err());
        assert!(Message::parse(b"").is_err());
    }

    #[test]
    fn an_error_message_needs_a_code_and_a_text_and_no_payload() {
        let good = Message::parse(b"d1:eli201e23:A Generic Error Ocurrede1:t2:aa1:y1:ee")
            .expect("it is bencode");
        assert_eq!(good.kind(), Kind::Error);
        assert!(good.is_conforming(), "{:?}", good.departures());

        let short = Message::parse(b"d1:eli201ee1:t2:aa1:y1:ee").expect("it is bencode");
        assert!(short.departures().contains(&Departure::ErrorNotAPair));
    }

    /// ⛔ The strides are pinned to their literals. Deriving one from the other
    /// would let a change to the node-id width move both and the comparison
    /// between them at once.
    #[test]
    fn the_compact_node_widths_are_the_ones_the_specification_fixes() {
        assert_eq!(NODE_ID_LEN, 20);
        assert_eq!(COMPACT_NODE_V4_LEN, 26);
        assert_eq!(COMPACT_NODE_V6_LEN, 38);
        // A whole number of nodes, and one that is not.
        assert_eq!(compact_nodes(52, COMPACT_NODE_V4_LEN), (2, 0));
        assert_eq!(compact_nodes(55, COMPACT_NODE_V4_LEN), (2, 3));
        assert_eq!(compact_nodes(0, COMPACT_NODE_V4_LEN), (0, 0));
        // ⚠ A zero stride reports nothing consumed rather than dividing by it.
        assert_eq!(compact_nodes(9, 0), (0, 9));
    }

    /// ⚠ The reading is over the decoded document, so it survives a re-encode
    /// and back. A reader built on byte offsets into the datagram would not.
    #[test]
    fn a_message_rebuilt_from_its_own_document_reads_identically() {
        let bytes = find_node();
        let first = Message::parse(&bytes).expect("it is bencode");
        let again = Message::parse(&bencode::encode(first.document())).expect("it is bencode");
        assert_eq!(first.kind(), again.kind());
        assert_eq!(first.method(), again.method());
        assert_eq!(first.node_id(), again.node_id());
        assert_eq!(first.key_order(), again.key_order());
        assert_eq!(first.argument_order(), again.argument_order());
    }
}
