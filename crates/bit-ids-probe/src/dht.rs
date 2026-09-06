//! The DHT observer: what a build says when it goes looking for the swarm on
//! its own.
//!
//! ⭐ **A client queries the DHT unprompted**, which is what makes this surface
//! observable in a lab that answers no tracker: a build with a torrent and no
//! peers reaches for whichever bootstrap node it was compiled with. What it says
//! on the way is dense with identity. BEP 5 fixes the method names and the key
//! spellings and fixes almost nothing else, so the node id a build picks for
//! itself, the width of its transaction ids, the order it writes its arguments,
//! the optional fields it attaches and the version tag it volunteers are each
//! the build's own choice.
//!
//! # It answers, and that is the difference from local discovery
//!
//! ⛔ **BEP 14 defines no reply and BEP 5 defines several**, so
//! [`local_discovery`](crate::local_discovery) is silent on every input and this
//! module is not. A build that queries and hears nothing back retries, backs off
//! and eventually stops, so a silent observer would measure a build talking to a
//! black hole rather than a build talking to a DHT. What is answered is part of
//! the experiment and is recorded beside what arrived, the way `OBS-05` records
//! a BEP 10 offer.
//!
//! # The door that is not a socket
//!
//! ⛔ **A `find_node` response hands the build addresses it will then dial
//! itself.** `nodes` and `values` are lists of places to go, and the packets that
//! follow leave the *build's* socket, so no guard on this crate's sockets can see
//! them: `bind::send_to` would never be called. A routable address offered that
//! way reaches the network exactly as surely as one the lab sent.
//!
//! [`bit_ids_lab::bind::check_offered`] is the guard, and every address this
//! module puts in a response goes through it. ⚠ **`adjacent::reaches` had already
//! named this hazard and named it on the wrong surface**, saying that `pex`
//! "hands out peer addresses a client will then dial". A DHT response does the
//! same thing through a different field, and a hazard recorded against one
//! surface but not the sibling that shares it is the one-gated-door defect
//! `docs/methodology/reviews.md` calls the most recurring hole there is.
//!
//! ⚠ The guard refuses rather than substitutes. Quietly replacing a routable
//! address with loopback would put bytes in a transcript that the observer chose
//! and the record would read as though the build had been offered them.
//!
//! # What it refuses to invent
//!
//! ⛔ **The token is a value the observer issues and then checks**, the way the
//! UDP tracker's connection id is. BEP 5 makes a build ask for a token with
//! `get_peers` and present it with `announce_peer`, so an announce carrying a
//! token this observer never issued means the build reused a stale one, invented
//! one, or skipped the `get_peers`. Each is a finding about the build and each is
//! answered with BEP 5's own protocol error rather than dropped.
//!
//! ⚠ The tokens are a deterministic sequence, which inverts
//! `docs/conventions/code.md`'s rule that identifiers come from a cryptographic
//! source, for the reason `tracker_udp` gives about connection ids: the value
//! protects nothing, the only party who can see it is the build under
//! measurement on a loopback socket, and two runs of one capture have to produce
//! comparable transcripts.

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::{Arc, Mutex, PoisonError};

use bit_ids_lab::adjacent::{Capability, NotEnabled, Surface, require};
use bit_ids_lab::bind;
use bit_ids_wire::bencode::{self, Value};
use bit_ids_wire::dht::{
    KEY_ERROR, KEY_NODE_ID, KEY_RETURN, KEY_TRANSACTION, KEY_TYPE, Kind, Message, NODE_ID_LEN,
};

/// The methods BEP 5 defines.
pub const METHOD_PING: &[u8] = b"ping";
/// The `find_node` method of BEP 5.
pub const METHOD_FIND_NODE: &[u8] = b"find_node";
/// The `get_peers` method of BEP 5.
pub const METHOD_GET_PEERS: &[u8] = b"get_peers";
/// The `announce_peer` method of BEP 5.
pub const METHOD_ANNOUNCE_PEER: &[u8] = b"announce_peer";

/// BEP 5's generic error code.
pub const ERROR_GENERIC: i64 = 201;
/// BEP 5's protocol error code, which a bad token is.
pub const ERROR_PROTOCOL: i64 = 203;
/// BEP 5's unknown-method error code.
pub const ERROR_METHOD_UNKNOWN: i64 = 204;

/// How many messages one observer keeps before it stops keeping them.
///
/// ⛔ Bounded for the reason every other observer's record is: the target is a
/// binary this project installed minutes ago, and a build that queries in a loop
/// would otherwise grow this vector until the host runs out of memory.
pub const DEFAULT_MAX_MESSAGES: usize = 4096;

/// The first token this observer issues. Tokens count up from it.
///
/// ⚠ Not zero, so a build that sends an all-zero token by mistake is
/// distinguishable from one presenting the first token actually issued.
pub const FIRST_TOKEN: u32 = 0x6269_0001;

/// The node identifier this observer answers with.
///
/// ⛔ **Exactly [`NODE_ID_LEN`] bytes, and the compiler is what says so.** The
/// first spelling of this was twenty-one bytes and read as though it were
/// twenty; a length that is a byte wrong produces a node id every build would
/// report as malformed, and the run would measure builds reacting to this
/// observer. The assertion below is compile-time so a later edit stops the build
/// rather than one suite somebody could have skipped.
pub const OBSERVER_NODE_ID: &[u8; NODE_ID_LEN] = b"bit-ids-observer-001";

const _: () = assert!(
    OBSERVER_NODE_ID.len() == NODE_ID_LEN,
    "the observer's node id is not the width BEP 5 fixes"
);

/// A well-known bootstrap node a real build reaches for, kept here to be
/// refused rather than used.
///
/// ⛔ **This address is never contacted.** It is what a build's own default
/// points at, and the acceptance drives `bind::send_to` and
/// `bind::check_offered` with it so the guards are shown refusing the address
/// the protocol actually names rather than one chosen to pass. That is the same
/// discipline `local_discovery` applies to the BEP 14 multicast groups.
///
/// ⚠ The port is BEP 5's conventional one. The address is `router.utorrent.com`
/// as an address literal, so nothing here resolves a name.
pub const A_REAL_BOOTSTRAP_NODE: SocketAddr =
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(82, 221, 103, 244), 6881));

/// Why a message is not what BEP 5 describes, beyond what the codec already
/// reports.
///
/// ⚠ These are the observer's findings rather than the codec's.
/// [`bit_ids_wire::dht::Departure`] carries the shape of the message; these
/// carry what it meant in this exchange.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Refusal {
    /// The datagram is not bencode at all.
    NotBencode(String),
    /// A query naming a method BEP 5 does not define.
    UnknownMethod(Vec<u8>),
    /// An `announce_peer` with no `token`.
    NoToken,
    /// An `announce_peer` presenting a token this observer never issued.
    ///
    /// ⭐ The finding this exchange exists to produce. A build that reused a
    /// stale token, invented one, or skipped `get_peers` entirely all land here,
    /// and the bytes it presented are kept.
    UnknownToken(Vec<u8>),
    /// A message that is a response or an error, which a build should not be
    /// sending to a node it queried.
    ///
    /// ⚠ Recorded rather than refused: a build that answers an observer that
    /// never queried it is telling us something about how it keeps state.
    NotAQuery(String),
}

impl Refusal {
    /// The refusal in one line.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::NotBencode(why) => format!("not bencode: {why}"),
            Self::UnknownMethod(method) => format!(
                "method {:?} is not one BEP 5 defines",
                String::from_utf8_lossy(method)
            ),
            Self::NoToken => "an announce_peer with no token".to_owned(),
            Self::UnknownToken(token) => format!(
                "token {:?} was never issued by this observer",
                String::from_utf8_lossy(token)
            ),
            Self::NotAQuery(kind) => format!("a {kind} arrived where a query was expected"),
        }
    }
}

/// One message, as it arrived, with what the observer answered.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Observed {
    raw: Vec<u8>,
    source: SocketAddr,
    message: Option<Message>,
    answered: Option<Vec<u8>>,
    refusals: Vec<Refusal>,
}

impl Observed {
    /// Every byte of the datagram, in the order it arrived.
    #[must_use]
    pub fn raw(&self) -> &[u8] {
        &self.raw
    }

    /// Where the datagram said it came from.
    ///
    /// ⚠ A claim by the sender rather than a fact the kernel checked, and it is
    /// what `implied_port` refers to. See
    /// [`bit_ids_lab::endpoint::DatagramResponder`].
    #[must_use]
    pub const fn source(&self) -> SocketAddr {
        self.source
    }

    /// The decoded message, when the datagram was bencode at all.
    #[must_use]
    pub const fn message(&self) -> Option<&Message> {
        self.message.as_ref()
    }

    /// What the observer answered, if it answered.
    ///
    /// ⭐ Kept because it is a condition of the measurement. What a build does
    /// next depends on what it heard, so a transcript that recorded only the
    /// build's side would attribute the observer's choices to the build.
    #[must_use]
    pub fn answered(&self) -> Option<&[u8]> {
        self.answered.as_deref()
    }

    /// Everything the observer found wrong with this message.
    #[must_use]
    pub fn refusals(&self) -> &[Refusal] {
        &self.refusals
    }

    /// The port this message says the build listens on, and where it got it.
    ///
    /// ⭐ **`implied_port` is why a datagram responder needs the source
    /// address.** BEP 5 says that when `implied_port` is present and non-zero
    /// the `port` argument is ignored and the source port of the packet is used
    /// instead, so the announced port is a value that exists only in the packet
    /// header. A reading that took `port` regardless would record a number the
    /// build explicitly told it to disregard.
    #[must_use]
    pub fn announced_port(&self) -> Option<AnnouncedPort> {
        let message = self.message.as_ref()?;
        if message.method() != Some(METHOD_ANNOUNCE_PEER) {
            return None;
        }
        let stated = match message.argument(b"port") {
            Some(Value::Integer(text)) => text.to_i64().and_then(|n| u16::try_from(n).ok()),
            _ => None,
        };
        let implied = match message.argument(b"implied_port") {
            Some(Value::Integer(text)) => text.to_i64().unwrap_or(0) != 0,
            _ => false,
        };
        Some(if implied {
            AnnouncedPort::Implied {
                observed: self.source.port(),
                stated,
            }
        } else {
            match stated {
                Some(port) => AnnouncedPort::Stated(port),
                None => AnnouncedPort::Neither,
            }
        })
    }
}

/// Which port an `announce_peer` actually announced.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnnouncedPort {
    /// `implied_port` was absent or zero, so the `port` argument is the answer.
    Stated(u16),
    /// `implied_port` was set, so the source port of the packet is the answer.
    Implied {
        /// The port the datagram arrived from, which is the announced one.
        observed: u16,
        /// What `port` said, kept because a build that sets `implied_port` and
        /// also sends a `port` is distinguishable from one that sends neither.
        stated: Option<u16>,
    },
    /// Neither a usable `port` nor an `implied_port`.
    Neither,
}

impl AnnouncedPort {
    /// The port the build announced, whichever way it said so.
    #[must_use]
    pub const fn port(self) -> Option<u16> {
        match self {
            Self::Stated(port) | Self::Implied { observed: port, .. } => Some(port),
            Self::Neither => None,
        }
    }
}

#[derive(Debug, Default)]
struct Record {
    kept: Vec<Observed>,
    dropped: usize,
    issued: Vec<Vec<u8>>,
}

/// What the observer offers a build that asks where to go.
///
/// ⛔ **Every address here has been through
/// [`bit_ids_lab::bind::check_offered`]**, which is why the constructor can
/// fail. A `Vec<SocketAddrV4>` field would let a caller append a routable
/// address after construction; a type whose only constructor checks cannot.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OfferedPeers {
    peers: Vec<SocketAddrV4>,
}

impl OfferedPeers {
    /// Offers nothing, which is what an observer with no peers to name does.
    #[must_use]
    pub const fn none() -> Self {
        Self { peers: Vec::new() }
    }

    /// Offers these addresses, if every one is inside the lab's allowed set.
    ///
    /// # Errors
    ///
    /// Returns [`bind::BindError::NotReachable`] naming the first address
    /// outside loopback, and offers nothing at all in that case.
    pub fn of(peers: &[SocketAddrV4]) -> Result<Self, bind::BindError> {
        for peer in peers {
            bind::check_offered(SocketAddr::V4(*peer))?;
        }
        Ok(Self {
            peers: peers.to_vec(),
        })
    }

    /// The addresses, every one already checked.
    #[must_use]
    pub fn addresses(&self) -> &[SocketAddrV4] {
        &self.peers
    }

    /// The BEP 5 compact form: four address bytes and a two-byte port each.
    #[must_use]
    pub fn compact(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.peers.len() * 6);
        for peer in &self.peers {
            out.extend_from_slice(&peer.ip().octets());
            out.extend_from_slice(&peer.port().to_be_bytes());
        }
        out
    }
}

/// The BEP 5 observer.
///
/// ⛔ Built only from a [`Capability`] for [`Surface::Dht`]. See
/// `bit_ids_lab::adjacent` for what that switch is and is not.
#[derive(Debug)]
pub struct Dht {
    seen: Arc<Mutex<Record>>,
    node_id: [u8; NODE_ID_LEN],
    offered: OfferedPeers,
    max_messages: usize,
}

impl Dht {
    /// An observer, if the DHT was turned on.
    ///
    /// # Errors
    ///
    /// Returns [`NotEnabled`] when `capability` enables a different surface.
    pub fn new(capability: Capability) -> Result<Self, NotEnabled> {
        require(capability, Surface::Dht)?;
        Ok(Self {
            seen: Arc::new(Mutex::new(Record::default())),
            node_id: *OBSERVER_NODE_ID,
            offered: OfferedPeers::none(),
            max_messages: DEFAULT_MAX_MESSAGES,
        })
    }

    /// The node identifier this observer answers with.
    ///
    /// ⚠ Fixed rather than derived from an address, which is what BEP 42 asks a
    /// real node to do. The observer is not a DHT node and a build has no reason
    /// to check; a value that changed per run would make two transcripts of one
    /// capture differ in bytes for no measurement, which is the argument
    /// `tracker_udp` makes about connection ids.
    #[must_use]
    pub const fn node_id(&self) -> &[u8; NODE_ID_LEN] {
        &self.node_id
    }

    /// What this observer offers when a build asks where to go.
    #[must_use]
    pub const fn offered(&self) -> &OfferedPeers {
        &self.offered
    }

    /// Offers a set of peers, every one already checked by [`OfferedPeers::of`].
    #[must_use]
    pub fn offering(mut self, offered: OfferedPeers) -> Self {
        self.offered = offered;
        self
    }

    /// How many messages this observer keeps.
    #[must_use]
    pub const fn with_max_messages(mut self, max_messages: usize) -> Self {
        self.max_messages = max_messages;
        self
    }

    /// Every message kept, in the order it arrived.
    #[must_use]
    pub fn messages(&self) -> Vec<Observed> {
        self.locked().kept.clone()
    }

    /// How many messages arrived after the cap and were recorded nowhere.
    #[must_use]
    pub fn dropped(&self) -> usize {
        self.locked().dropped
    }

    /// Every token this observer has issued, in the order it issued them.
    #[must_use]
    pub fn issued_tokens(&self) -> Vec<Vec<u8>> {
        self.locked().issued.clone()
    }

    /// ⚠ A poisoned lock is recovered rather than propagated: observations taken
    /// before a panic are still observations.
    fn locked(&self) -> std::sync::MutexGuard<'_, Record> {
        self.seen.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The responder to give a datagram endpoint.
    pub fn responder(
        &self,
    ) -> impl Fn(SocketAddr, &[u8]) -> Option<Vec<u8>> + Send + Sync + 'static {
        let seen = Arc::clone(&self.seen);
        let node_id = self.node_id;
        let offered = self.offered.clone();
        let cap = self.max_messages;
        move |source: SocketAddr, datagram: &[u8]| {
            respond(&seen, &node_id, &offered, cap, source, datagram)
        }
    }
}

/// Reads one datagram, records it, and returns what to answer with.
fn respond(
    seen: &Arc<Mutex<Record>>,
    node_id: &[u8; NODE_ID_LEN],
    offered: &OfferedPeers,
    cap: usize,
    source: SocketAddr,
    datagram: &[u8],
) -> Option<Vec<u8>> {
    let message = match Message::parse(datagram) {
        Ok(message) => message,
        Err(error) => {
            // ⛔ Kept, and unanswered. A datagram that is not bencode carries no
            // transaction id, so any reply would have to invent one, and a build
            // receiving a reply it cannot match is being measured on this
            // observer's behaviour rather than its own.
            keep(
                seen,
                cap,
                Observed {
                    raw: datagram.to_vec(),
                    source,
                    message: None,
                    answered: None,
                    refusals: vec![Refusal::NotBencode(error.to_string())],
                },
            );
            return None;
        }
    };
    let mut refusals = Vec::new();
    let answer = answer_for(seen, node_id, offered, &message, &mut refusals);
    keep(
        seen,
        cap,
        Observed {
            raw: datagram.to_vec(),
            source,
            message: Some(message),
            answered: answer.clone(),
            refusals,
        },
    );
    answer
}

/// What BEP 5 says to answer this message with.
fn answer_for(
    seen: &Arc<Mutex<Record>>,
    node_id: &[u8; NODE_ID_LEN],
    offered: &OfferedPeers,
    message: &Message,
    refusals: &mut Vec<Refusal>,
) -> Option<Vec<u8>> {
    let transaction = message.transaction_id()?.to_vec();
    match message.kind() {
        Kind::Query => {}
        // ⚠ Recorded and unanswered. Answering a response would be inventing an
        // exchange BEP 5 does not have.
        other => {
            refusals.push(Refusal::NotAQuery(format!("{other:?}").to_lowercase()));
            return None;
        }
    }
    let method = message.method()?;
    match method {
        METHOD_PING => Some(response(
            &transaction,
            vec![(KEY_NODE_ID.to_vec(), id(node_id))],
        )),
        METHOD_FIND_NODE => Some(response(
            &transaction,
            vec![
                (KEY_NODE_ID.to_vec(), id(node_id)),
                // ⛔ Empty, and empty is a decision. The observer names no other
                // node, so a build following this answer has nowhere else to go,
                // which is the containment holding rather than the protocol
                // failing. Naming one would need an address through
                // `bind::check_offered`.
                (b"nodes".to_vec(), Value::bytes(Vec::new())),
            ],
        )),
        METHOD_GET_PEERS => {
            let token = issue_token(seen);
            Some(response(
                &transaction,
                vec![
                    (KEY_NODE_ID.to_vec(), id(node_id)),
                    (b"token".to_vec(), Value::bytes(token)),
                    // ⛔ Every address in here was checked by `OfferedPeers::of`,
                    // which is why this cannot be the door that leaks.
                    (
                        b"values".to_vec(),
                        Value::List(
                            offered
                                .addresses()
                                .iter()
                                .map(|peer| {
                                    let mut one = peer.ip().octets().to_vec();
                                    one.extend_from_slice(&peer.port().to_be_bytes());
                                    Value::Bytes(one)
                                })
                                .collect(),
                        ),
                    ),
                ],
            ))
        }
        METHOD_ANNOUNCE_PEER => {
            let Some(Value::Bytes(presented)) = message.argument(b"token") else {
                refusals.push(Refusal::NoToken);
                return Some(error(&transaction, ERROR_PROTOCOL, "no token"));
            };
            let presented = presented.clone();
            if seen
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .issued
                .contains(&presented)
            {
                Some(response(
                    &transaction,
                    vec![(KEY_NODE_ID.to_vec(), id(node_id))],
                ))
            } else {
                refusals.push(Refusal::UnknownToken(presented));
                Some(error(&transaction, ERROR_PROTOCOL, "unknown token"))
            }
        }
        other => {
            refusals.push(Refusal::UnknownMethod(other.to_vec()));
            Some(error(
                &transaction,
                ERROR_METHOD_UNKNOWN,
                "method not recognised",
            ))
        }
    }
}

/// The observer's node id as a bencode value.
fn id(node_id: &[u8; NODE_ID_LEN]) -> Value {
    Value::bytes(node_id.to_vec())
}

/// The next token, recorded as issued.
///
/// ⚠ Big-endian bytes of a counter, so a transcript shows which query in the
/// run a token came from.
fn issue_token(seen: &Arc<Mutex<Record>>) -> Vec<u8> {
    let mut record = seen.lock().unwrap_or_else(PoisonError::into_inner);
    let next = FIRST_TOKEN.wrapping_add(
        u32::try_from(record.issued.len()).expect("a run cannot issue four billion tokens"),
    );
    let token = next.to_be_bytes().to_vec();
    record.issued.push(token.clone());
    token
}

/// A BEP 5 response: `{"r": {...}, "t": ..., "y": "r"}`, keys sorted.
///
/// ⚠ Sorted because BEP 3 says so and the observer is the one side of this
/// exchange that has no reason to be unusual. What a build does with an unsorted
/// response is a measurement of this code rather than of the build.
fn response(transaction: &[u8], mut values: Vec<(Vec<u8>, Value)>) -> Vec<u8> {
    values.sort_by(|left, right| left.0.cmp(&right.0));
    bencode::encode(&Value::Dictionary(vec![
        (KEY_RETURN.to_vec(), Value::Dictionary(values)),
        (KEY_TRANSACTION.to_vec(), Value::bytes(transaction.to_vec())),
        (KEY_TYPE.to_vec(), Value::bytes(b"r".to_vec())),
    ]))
}

/// A BEP 5 error: `{"e": [code, text], "t": ..., "y": "e"}`.
fn error(transaction: &[u8], code: i64, text: &str) -> Vec<u8> {
    bencode::encode(&Value::Dictionary(vec![
        (
            KEY_ERROR.to_vec(),
            Value::List(vec![
                Value::integer(code),
                Value::bytes(text.as_bytes().to_vec()),
            ]),
        ),
        (KEY_TRANSACTION.to_vec(), Value::bytes(transaction.to_vec())),
        (KEY_TYPE.to_vec(), Value::bytes(b"e".to_vec())),
    ]))
}

/// Records one observation, counting it instead once the cap is reached.
fn keep(seen: &Arc<Mutex<Record>>, cap: usize, observed: Observed) {
    let mut record = seen.lock().unwrap_or_else(PoisonError::into_inner);
    if record.kept.len() >= cap {
        record.dropped += 1;
        return;
    }
    record.kept.push(observed);
}

#[cfg(test)]
mod tests {
    use super::{A_REAL_BOOTSTRAP_NODE, AnnouncedPort, Dht, ERROR_PROTOCOL, OfferedPeers, Refusal};
    use bit_ids_lab::adjacent::{ALL_SURFACES as ALL, Capability, Surface};
    use bit_ids_lab::bind;
    use bit_ids_wire::bencode::{self, Value};
    use bit_ids_wire::dht::{KEY_ARGUMENTS, Kind, Message};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};

    const ID: &[u8] = b"a-build-under-measur";

    fn from_port(port: u16) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)
    }

    /// Builds a BEP 5 query with sorted keys.
    fn query(method: &[u8], transaction: &[u8], mut args: Vec<(Vec<u8>, Value)>) -> Vec<u8> {
        args.insert(0, (b"id".to_vec(), Value::bytes(ID.to_vec())));
        args.sort_by(|left, right| left.0.cmp(&right.0));
        bencode::encode(&Value::Dictionary(vec![
            (KEY_ARGUMENTS.to_vec(), Value::Dictionary(args)),
            (b"q".to_vec(), Value::bytes(method.to_vec())),
            (b"t".to_vec(), Value::bytes(transaction.to_vec())),
            (b"y".to_vec(), Value::bytes(b"q".to_vec())),
        ]))
    }

    fn observer() -> Dht {
        Dht::new(Capability::enable(Surface::Dht)).expect("the capability names this surface")
    }

    #[test]
    fn the_observer_is_refused_without_a_capability_for_its_own_surface() {
        assert!(Dht::new(Capability::enable(Surface::Dht)).is_ok());
        for other in ALL {
            if other == Surface::Dht {
                continue;
            }
            let refusal = Dht::new(Capability::enable(other)).expect_err("a different surface");
            assert_eq!(refusal.wanted, Surface::Dht);
            assert_eq!(refusal.offered, other);
        }
    }

    #[test]
    fn a_ping_is_answered_with_a_response_carrying_the_same_transaction() {
        let observer = observer();
        let responder = observer.responder();
        let sent = query(b"ping", b"\xaa\x01", Vec::new());
        let reply = responder(from_port(40000), &sent).expect("a ping is answered");

        let answer = Message::parse(&reply).expect("the answer is bencode");
        assert!(answer.is_conforming(), "{:?}", answer.departures());
        assert_eq!(answer.kind(), Kind::Response);
        assert_eq!(answer.transaction_id(), Some(&b"\xaa\x01"[..]));
        assert_eq!(answer.node_id(), Some(&observer.node_id()[..]));

        let kept = observer.messages();
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].raw(), sent);
        assert_eq!(kept[0].answered(), Some(&reply[..]));
        assert!(kept[0].refusals().is_empty(), "{:?}", kept[0].refusals());
    }

    /// ⛔ The whole reason a datagram responder needs a source address. A build
    /// setting `implied_port` announces the port the packet came from, and a
    /// reading that took `port` would record a number the build told it to
    /// ignore.
    #[test]
    fn implied_port_announces_the_source_port_and_stated_port_announces_the_argument() {
        let observer = observer();
        let responder = observer.responder();
        // A token has to be issued before an announce can be accepted.
        responder(
            from_port(45000),
            &query(b"get_peers", b"\xaa\x01", Vec::new()),
        )
        .expect("get_peers is answered");
        let token = observer.issued_tokens()[0].clone();

        let implied = query(
            b"announce_peer",
            b"\xaa\x02",
            vec![
                (b"implied_port".to_vec(), Value::integer(1)),
                (b"port".to_vec(), Value::integer(6881)),
                (b"token".to_vec(), Value::bytes(token.clone())),
            ],
        );
        responder(from_port(45000), &implied).expect("an announce is answered");
        let kept = observer.messages();
        assert_eq!(
            kept[1].announced_port(),
            Some(AnnouncedPort::Implied {
                observed: 45000,
                stated: Some(6881),
            })
        );
        // ⛔ 45000, not 6881. The build said to ignore the argument.
        assert_eq!(
            kept[1].announced_port().and_then(AnnouncedPort::port),
            Some(45000)
        );

        let stated = query(
            b"announce_peer",
            b"\xaa\x03",
            vec![
                (b"port".to_vec(), Value::integer(6881)),
                (b"token".to_vec(), Value::bytes(token)),
            ],
        );
        responder(from_port(45000), &stated).expect("an announce is answered");
        let kept = observer.messages();
        assert_eq!(kept[2].announced_port(), Some(AnnouncedPort::Stated(6881)));
        assert_eq!(
            kept[2].announced_port().and_then(AnnouncedPort::port),
            Some(6881)
        );
    }

    /// ⭐ The token is issued and then checked, the way the UDP tracker's
    /// connection id is.
    #[test]
    fn an_announce_with_a_token_this_observer_never_issued_gets_a_protocol_error() {
        let observer = observer();
        let responder = observer.responder();
        let forged = query(
            b"announce_peer",
            b"\xaa\x09",
            vec![
                (b"port".to_vec(), Value::integer(6881)),
                (b"token".to_vec(), Value::bytes(b"nope".to_vec())),
            ],
        );
        let reply = responder(from_port(45000), &forged).expect("an error is still an answer");
        let answer = Message::parse(&reply).expect("bencode");
        assert_eq!(answer.kind(), Kind::Error);
        let Some(Value::List(pair)) = answer.document().get(b"e") else {
            panic!("an error carries a list");
        };
        assert_eq!(pair[0], Value::integer(ERROR_PROTOCOL));

        let kept = observer.messages();
        assert!(
            kept[0]
                .refusals()
                .contains(&Refusal::UnknownToken(b"nope".to_vec())),
            "{:?}",
            kept[0].refusals()
        );
        // ⛔ Refused and kept. The bytes are the finding.
        assert_eq!(kept[0].raw(), forged);

        let none = query(b"announce_peer", b"\xaa\x0a", Vec::new());
        responder(from_port(45000), &none).expect("an error is still an answer");
        assert!(
            observer.messages()[1]
                .refusals()
                .contains(&Refusal::NoToken)
        );
    }

    #[test]
    fn an_unknown_method_is_reported_and_answered_with_the_protocol_s_own_error() {
        let observer = observer();
        let responder = observer.responder();
        let reply = responder(from_port(40000), &query(b"vote", b"\xaa\x01", Vec::new()))
            .expect("answered");
        assert_eq!(Message::parse(&reply).expect("bencode").kind(), Kind::Error);
        assert!(
            observer.messages()[0]
                .refusals()
                .contains(&Refusal::UnknownMethod(b"vote".to_vec()))
        );
    }

    #[test]
    fn a_datagram_that_is_not_bencode_is_kept_and_never_answered() {
        let observer = observer();
        let responder = observer.responder();
        assert!(responder(from_port(40000), b"\xff\xff\xff").is_none());
        let kept = observer.messages();
        assert_eq!(kept[0].raw(), b"\xff\xff\xff");
        assert!(kept[0].message().is_none());
        assert!(kept[0].answered().is_none());
        assert!(matches!(kept[0].refusals()[0], Refusal::NotBencode(_)));
    }

    /// ⚠ A response arriving where a query was expected is recorded and not
    /// answered. Answering it would invent an exchange BEP 5 does not have.
    #[test]
    fn a_response_arriving_unprompted_is_recorded_and_not_answered() {
        let observer = observer();
        let responder = observer.responder();
        let unexpected = bencode::encode(&Value::Dictionary(vec![
            (
                b"r".to_vec(),
                Value::Dictionary(vec![(b"id".to_vec(), Value::bytes(ID.to_vec()))]),
            ),
            (b"t".to_vec(), Value::bytes(b"zz".to_vec())),
            (b"y".to_vec(), Value::bytes(b"r".to_vec())),
        ]));
        assert!(responder(from_port(40000), &unexpected).is_none());
        assert!(matches!(
            observer.messages()[0].refusals()[0],
            Refusal::NotAQuery(_)
        ));
    }

    /// ⛔ The door that is not a socket. An address outside the allowed set
    /// cannot be put in a response, and the constructor is where that is
    /// decided rather than the send.
    #[test]
    fn a_peer_outside_the_allowed_set_cannot_be_offered_at_all() {
        let routable = SocketAddrV4::new(Ipv4Addr::new(198, 51, 100, 7), 6881);
        let refusal = OfferedPeers::of(&[routable]).expect_err("outside loopback");
        assert!(matches!(refusal, bind::BindError::NotReachable { .. }));

        // ⚠ And a list with one good address and one bad offers nothing, rather
        // than offering the good one and dropping the other silently.
        let mixed = OfferedPeers::of(&[SocketAddrV4::new(Ipv4Addr::LOCALHOST, 6881), routable]);
        assert!(mixed.is_err());

        let allowed = OfferedPeers::of(&[SocketAddrV4::new(Ipv4Addr::LOCALHOST, 6881)])
            .expect("loopback is allowed");
        assert_eq!(allowed.compact(), vec![127, 0, 0, 1, 0x1a, 0xe1]);
    }

    /// ⛔ The guards, driven with the address a real build's default actually
    /// names rather than one chosen to pass.
    #[test]
    fn the_bootstrap_node_a_build_reaches_for_is_refused_by_both_doors() {
        assert!(!A_REAL_BOOTSTRAP_NODE.ip().is_loopback());
        assert!(matches!(
            bind::check_offered(A_REAL_BOOTSTRAP_NODE),
            Err(bind::BindError::NotReachable { .. })
        ));
        let socket = bind::datagram(IpAddr::V4(Ipv4Addr::LOCALHOST)).expect("loopback binds");
        assert!(matches!(
            bind::send_to(&socket, b"d1:y1:qe", A_REAL_BOOTSTRAP_NODE),
            Err(bind::BindError::NotReachable { .. })
        ));
        // ⚠ And a refusal is not an inability to send: the same socket reaches a
        // loopback destination.
        let listener = bind::datagram(IpAddr::V4(Ipv4Addr::LOCALHOST)).expect("loopback binds");
        let address = listener.local_addr().expect("a bound socket has one");
        assert!(bind::send_to(&socket, b"d1:y1:qe", address).is_ok());
    }

    #[test]
    fn a_get_peers_answer_carries_a_token_and_only_addresses_that_were_checked() {
        let offered =
            OfferedPeers::of(&[SocketAddrV4::new(Ipv4Addr::LOCALHOST, 6881)]).expect("loopback");
        let observer = observer().offering(offered);
        let responder = observer.responder();
        let reply = responder(
            from_port(45000),
            &query(b"get_peers", b"\xaa\x01", Vec::new()),
        )
        .expect("answered");
        let answer = Message::parse(&reply).expect("bencode");
        assert!(answer.is_conforming(), "{:?}", answer.departures());
        assert_eq!(
            answer.argument(b"token"),
            Some(&Value::bytes(observer.issued_tokens()[0].clone()))
        );
        let Some(Value::List(values)) = answer.argument(b"values") else {
            panic!("values is a list");
        };
        assert_eq!(values.len(), 1);
        assert_eq!(values[0], Value::Bytes(vec![127, 0, 0, 1, 0x1a, 0xe1]));
    }

    /// ⛔ Empty is a decision. A build following a `find_node` answer has nowhere
    /// else to go, which is the containment holding rather than a protocol
    /// failure.
    #[test]
    fn a_find_node_answer_names_no_other_node() {
        let observer = observer();
        let responder = observer.responder();
        let reply = responder(
            from_port(40000),
            &query(b"find_node", b"\xaa\x01", Vec::new()),
        )
        .expect("answered");
        let answer = Message::parse(&reply).expect("bencode");
        assert_eq!(answer.argument(b"nodes"), Some(&Value::bytes(Vec::new())));
    }

    #[test]
    fn messages_past_the_cap_are_counted_rather_than_kept() {
        let observer = observer().with_max_messages(2);
        let responder = observer.responder();
        for _ in 0..5 {
            responder(from_port(40000), &query(b"ping", b"\xaa\x01", Vec::new()));
        }
        assert_eq!(observer.messages().len(), 2);
        assert_eq!(observer.dropped(), 3);
    }

    /// ⚠ Every answer this observer writes is a message its own codec reads
    /// back as conforming. An observer that emitted a message it would itself
    /// report a departure on would be measuring the build against a shape no
    /// specification describes.
    #[test]
    fn every_answer_this_observer_writes_is_one_bep_5_describes() {
        let observer = observer();
        let responder = observer.responder();
        let mut answers = 0;
        for method in [
            &b"ping"[..],
            b"find_node",
            b"get_peers",
            b"announce_peer",
            b"vote",
        ] {
            if let Some(reply) =
                responder(from_port(40000), &query(method, b"\xaa\x01", Vec::new()))
            {
                let answer = Message::parse(&reply).expect("bencode");
                assert!(
                    answer.is_conforming(),
                    "{}: {:?}",
                    String::from_utf8_lossy(method),
                    answer.departures()
                );
                answers += 1;
            }
        }
        assert_eq!(answers, 5, "every one of those is answered");
    }
}
