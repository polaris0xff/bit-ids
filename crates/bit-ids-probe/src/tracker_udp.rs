//! The UDP tracker observer: BEP 15, and what a build puts in a datagram.
//!
//! `OBS-03`. The UDP announce carries fields the HTTP one does not expose in the
//! same way: a binary layout with no names, a 32-bit `key`, a signed `num_want`
//! where `-1` means the tracker's default, an event code, and whatever a client
//! appends past byte 98 as BEP 41 options. ⛔ **Every field is positional**, so
//! the guard against reading the wrong span is a length check on every frame,
//! which `bit_ids_wire::tracker_udp` holds.
//!
//! # The exchange is stateful, and that is the point
//!
//! BEP 15 makes a client connect before it announces, and the tracker's
//! connection id is what ties the two together. ⭐ **An announce carrying an id
//! this observer never issued is an observation**, not an error to hide: it
//! means the build reused a stale id, invented one, or skipped the connect. So
//! the id is checked, and a mismatch is answered with the protocol's own error
//! action rather than ignored.
//!
//! ⚠ **The connection ids are a contiguous deterministic range and that inverts
//! this project's usual rule.** `docs/conventions/code.md` asks for identifiers
//! from a cryptographic random source. Here the value protects nothing, the only
//! party who can see it is the build under measurement on a loopback socket, and
//! two runs of one capture have to produce comparable transcripts. A random id
//! would make every recorded exchange differ in bytes for no measurement. The
//! range also means membership is an arithmetic test rather than a set that
//! grows for as long as a client keeps connecting.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex, PoisonError};

use bit_ids_wire::WireError;
use bit_ids_wire::tracker_udp::{Action, AnnounceRequest, Datagram, Direction};

use crate::tracker_http::OfferedPeer;

/// The first connection id this observer hands out.
///
/// Non-zero, so a client that sends an all-zero id is distinguishable from one
/// that echoed the first id issued.
pub const FIRST_CONNECTION_ID: u64 = 0x6269_745f_6964_0001;

/// How many datagrams one observer keeps before it stops keeping them.
///
/// ⛔ The lab's deadline bounds how long a target can talk and not how fast. A
/// build that announces in a loop would otherwise grow this record until the
/// host ran out of memory.
pub const DEFAULT_MAX_DATAGRAMS: usize = 8192;

/// One datagram the observer saw, kept as the bytes that arrived.
///
/// ⭐ The decode is kept as a `Result`. A datagram this codec cannot read is an
/// observation about the build, so it is recorded with the reason rather than
/// dropped for being unreadable.
#[derive(Clone, Debug)]
pub struct Observed {
    raw: Vec<u8>,
    decoded: Result<Datagram, WireError>,
}

impl Observed {
    /// The datagram exactly as it arrived.
    #[must_use]
    pub fn raw(&self) -> &[u8] {
        &self.raw
    }

    /// The decoded view, or why it could not be decoded.
    ///
    /// # Errors
    ///
    /// Returns what `bit_ids_wire::tracker_udp` refused.
    pub fn decoded(&self) -> Result<&Datagram, &WireError> {
        self.decoded.as_ref()
    }

    /// The action code, when the datagram decoded.
    #[must_use]
    pub fn action(&self) -> Option<Action> {
        self.decoded.as_ref().ok().map(Datagram::action)
    }

    /// The announce fields, when this is a decodable announce request.
    #[must_use]
    pub fn announce(&self) -> Option<Result<AnnounceRequest, WireError>> {
        self.decoded.as_ref().ok()?.as_announce_request()
    }
}

/// What the observer answers an announce with.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UdpTrackerResponse {
    /// Seconds the client is asked to wait before announcing again.
    pub interval: i32,
    /// Leechers reported back.
    pub leechers: i32,
    /// Seeders reported back.
    pub seeders: i32,
    /// The peers offered back. BEP 15 has only the six-byte compact form.
    pub peers: Vec<OfferedPeer>,
}

impl Default for UdpTrackerResponse {
    /// 1800 seconds, which is far longer than any capture deadline, so a client
    /// re-announces only because the run made it, and no peers.
    fn default() -> Self {
        Self {
            interval: 1800,
            leechers: 0,
            seeders: 0,
            peers: Vec::new(),
        }
    }
}

/// Why the observer answered with the protocol's error action.
///
/// ⭐ Each variant is a thing a build did, kept so a record can say which.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Refusal {
    /// A connect request that did not open with BEP 15's magic value.
    WrongProtocolId,
    /// An announce or scrape carrying a connection id this observer never
    /// issued.
    UnknownConnectionId,
    /// An action code BEP 15 does not define for a request.
    UnknownAction,
    /// An announce whose fields could not be read.
    UndecodableAnnounce,
}

impl Refusal {
    /// The message put in the error datagram.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::WrongProtocolId => "connect did not open with the BEP 15 protocol id",
            Self::UnknownConnectionId => "connection id was never issued by this tracker",
            Self::UnknownAction => "action is not connect, announce or scrape",
            Self::UndecodableAnnounce => "announce fields did not decode",
        }
    }
}

#[derive(Debug, Default)]
struct Record {
    kept: Vec<Observed>,
    dropped: usize,
    refusals: Vec<Refusal>,
    issued: u64,
}

impl Record {
    /// Hands out the next connection id.
    fn issue(&mut self) -> u64 {
        let id = FIRST_CONNECTION_ID.wrapping_add(self.issued);
        self.issued += 1;
        id
    }

    /// Whether an id is one this observer handed out.
    ///
    /// Arithmetic rather than a lookup, which is what the contiguous range in
    /// the module documentation buys.
    const fn issued_by_us(&self, id: u64) -> bool {
        id >= FIRST_CONNECTION_ID && id < FIRST_CONNECTION_ID.wrapping_add(self.issued)
    }
}

/// The UDP tracker observer.
///
/// Hand [`UdpTracker::responder`] to a `bit-ids-lab` datagram endpoint.
#[derive(Clone, Debug)]
pub struct UdpTracker {
    seen: Arc<Mutex<Record>>,
    response: UdpTrackerResponse,
    max_datagrams: usize,
}

impl UdpTracker {
    /// An observer that answers with `response`.
    #[must_use]
    pub fn new(response: UdpTrackerResponse) -> Self {
        Self {
            seen: Arc::new(Mutex::new(Record::default())),
            response,
            max_datagrams: DEFAULT_MAX_DATAGRAMS,
        }
    }

    /// How many datagrams this observer keeps.
    #[must_use]
    pub const fn with_max_datagrams(mut self, max_datagrams: usize) -> Self {
        self.max_datagrams = max_datagrams;
        self
    }

    /// Every datagram kept, in the order it arrived.
    #[must_use]
    pub fn datagrams(&self) -> Vec<Observed> {
        self.locked().kept.clone()
    }

    /// How many datagrams arrived after the cap and were answered but not kept.
    #[must_use]
    pub fn dropped(&self) -> usize {
        self.locked().dropped
    }

    /// Every refusal, in order, with why the observer answered with an error.
    #[must_use]
    pub fn refusals(&self) -> Vec<Refusal> {
        self.locked().refusals.clone()
    }

    /// How many connection ids have been handed out.
    #[must_use]
    pub fn issued_connection_ids(&self) -> u64 {
        self.locked().issued
    }

    /// ⚠ A poisoned lock is recovered rather than propagated: observations taken
    /// before a panic are still observations.
    fn locked(&self) -> std::sync::MutexGuard<'_, Record> {
        self.seen.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The responder to give a datagram endpoint.
    ///
    /// ⚠ **The source address is taken and deliberately unused here.** BEP 15's
    /// announce carries an `IP address` field whose zero value means *use the
    /// source address of this packet*, so comparing the two is a real
    /// observation and it is `OBS-03`'s to make, not a line to slip into
    /// `OBS-11`'s prerequisite. Recording it would widen `Announce` and its
    /// acceptance; naming why it is skipped is what keeps the omission from
    /// reading as an oversight.
    pub fn responder(
        &self,
    ) -> impl Fn(SocketAddr, &[u8]) -> Option<Vec<u8>> + Send + Sync + 'static {
        let seen = Arc::clone(&self.seen);
        let response = self.response.clone();
        let cap = self.max_datagrams;
        move |_source: SocketAddr, packet: &[u8]| respond(&seen, &response, cap, packet)
    }
}

impl Default for UdpTracker {
    fn default() -> Self {
        Self::new(UdpTrackerResponse::default())
    }
}

/// The connect response of BEP 15: action, transaction id, connection id.
///
/// ⚠ Private, with the three below it, for the reason `tracker_http` gives.
fn connect_response(transaction_id: u32, connection_id: u64) -> Vec<u8> {
    let mut out = Action::Connect.code().to_be_bytes().to_vec();
    out.extend_from_slice(&transaction_id.to_be_bytes());
    out.extend_from_slice(&connection_id.to_be_bytes());
    out
}

/// The announce response of BEP 15, with six bytes per peer.
fn announce_response(transaction_id: u32, response: &UdpTrackerResponse) -> Vec<u8> {
    let mut out = Action::Announce.code().to_be_bytes().to_vec();
    out.extend_from_slice(&transaction_id.to_be_bytes());
    out.extend_from_slice(&response.interval.to_be_bytes());
    out.extend_from_slice(&response.leechers.to_be_bytes());
    out.extend_from_slice(&response.seeders.to_be_bytes());
    for peer in &response.peers {
        out.extend_from_slice(&peer.address);
        out.extend_from_slice(&peer.port.to_be_bytes());
    }
    out
}

/// The scrape response of BEP 15, with twelve bytes per info hash.
fn scrape_response(transaction_id: u32, response: &UdpTrackerResponse, hashes: usize) -> Vec<u8> {
    let mut out = Action::Scrape.code().to_be_bytes().to_vec();
    out.extend_from_slice(&transaction_id.to_be_bytes());
    for _ in 0..hashes {
        out.extend_from_slice(&response.seeders.to_be_bytes());
        out.extend_from_slice(&0_i32.to_be_bytes());
        out.extend_from_slice(&response.leechers.to_be_bytes());
    }
    out
}

/// The error response of BEP 15: action three, the transaction id, a message.
fn error_response(transaction_id: u32, message: &str) -> Vec<u8> {
    let mut out = Action::Error.code().to_be_bytes().to_vec();
    out.extend_from_slice(&transaction_id.to_be_bytes());
    out.extend_from_slice(message.as_bytes());
    out
}

/// How many info hashes a scrape request carries, twenty bytes each.
const SCRAPE_HASH_LEN: usize = 20;
/// Where a scrape request's info hashes begin: after the connection id, the
/// action and the transaction id.
const SCRAPE_HASHES_AT: usize = 16;

fn respond(
    seen: &Arc<Mutex<Record>>,
    response: &UdpTrackerResponse,
    cap: usize,
    packet: &[u8],
) -> Option<Vec<u8>> {
    let decoded = Datagram::parse(Direction::FromTarget, packet);
    let mut record = seen.lock().unwrap_or_else(PoisonError::into_inner);
    if record.kept.len() < cap {
        record.kept.push(Observed {
            raw: packet.to_vec(),
            decoded: decoded.clone(),
        });
    } else {
        record.dropped += 1;
    }

    // ⛔ A datagram that did not decode is not answered. The error action needs
    // a transaction id, and this observer does not have one it can trust:
    // answering with a guess would put bytes on the wire that no request asked
    // for, and a replay would not reproduce them.
    let datagram = decoded.ok()?;
    let transaction_id = datagram.transaction_id();

    // ⚠ Bounded by the same cap as the record. The datagram list was capped and
    // this list was not, so a build sending garbage in a loop would have grown
    // it until the host ran out of memory: the bound was on one of two lists
    // that grow together.
    let refuse = |record: &mut Record, refusal: Refusal| -> Option<Vec<u8>> {
        if record.refusals.len() < cap {
            record.refusals.push(refusal);
        }
        Some(error_response(transaction_id, refusal.message()))
    };

    match datagram.action() {
        Action::Connect => {
            if !datagram.opens_with_protocol_id() {
                return refuse(&mut record, Refusal::WrongProtocolId);
            }
            let connection_id = record.issue();
            Some(connect_response(transaction_id, connection_id))
        }
        Action::Announce => {
            let Some(Ok(request)) = datagram.as_announce_request() else {
                return refuse(&mut record, Refusal::UndecodableAnnounce);
            };
            if !record.issued_by_us(request.connection_id) {
                return refuse(&mut record, Refusal::UnknownConnectionId);
            }
            Some(announce_response(transaction_id, response))
        }
        Action::Scrape => {
            // ⚠ Defensive, and it says the accurate thing. A decoded request
            // always carries eight readable bytes here, so this arm should not
            // be reachable; labelling it `UndecodableAnnounce` would have put
            // the word announce on a scrape. A request whose id cannot be read
            // is certainly not one this tracker issued.
            let Some(id) = datagram.connection_id() else {
                return refuse(&mut record, Refusal::UnknownConnectionId);
            };
            if !record.issued_by_us(id) {
                return refuse(&mut record, Refusal::UnknownConnectionId);
            }
            // A short trailing run is not a whole info hash, so it is not
            // counted as one: the response width follows what arrived.
            let hashes = packet
                .len()
                .saturating_sub(SCRAPE_HASHES_AT)
                .checked_div(SCRAPE_HASH_LEN)
                .unwrap_or(0);
            Some(scrape_response(transaction_id, response, hashes))
        }
        // Action three is a response code. A client sending one is talking to
        // itself, and answering an error with an error is a loop.
        Action::Error => None,
        Action::Other(_) => refuse(&mut record, Refusal::UnknownAction),
    }
}
