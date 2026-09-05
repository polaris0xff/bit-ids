//! The HTTP tracker observer: what a build puts in an announce.
//!
//! `OBS-02`. An announce carries the peer ID, the user agent, the header set,
//! the order of the query and of the headers, the percent-encoding case, the
//! key, `numwant`, the compact and no-peer-ID flags, and the event sequence.
//! ⛔ **The ordinary HTTP server interface destroys every one of them**, which
//! is why this is not one. A handler is given a map of decoded strings, and a
//! map has no order, keeps one value per key and holds no record of how a value
//! was encoded, so all three are gone before any handler runs.
//!
//! So the observer keeps the exact head bytes, decodes them with
//! [`bit_ids_wire::tracker_http`], and answers. Nothing is normalised on the way
//! in and nothing is decoded until a caller asks for one field.
//!
//! # What it answers, and why that is not a detail
//!
//! ⚠ **The response shape is chosen from what the request asked for.** A client
//! that sent `compact=1` and receives a peer list, or the reverse, reports an
//! error and changes what it does next. That behaviour would be recorded as
//! identity, and it would be the observer's behaviour rather than the build's.
//! So `compact` and `no_peer_id` are read out of the announce and honoured.
//!
//! # What it is not
//!
//! ⛔ **Nothing here maps a peer-ID prefix or a user agent to a client name.**
//! `docs/capture-methodology.md` lists a decoder table among the inputs that may
//! seed a hypothesis and may not populate the catalogue.

use std::sync::{Arc, Mutex, PoisonError};

use bit_ids_lab::StreamReply;
use bit_ids_wire::WireError;
use bit_ids_wire::bencode::{self, Value};
use bit_ids_wire::tracker_http::{HttpRequest, PercentCase, head_end};

/// One request the observer saw, kept as the bytes that arrived.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Announce {
    raw: Vec<u8>,
    request: HttpRequest,
}

impl Announce {
    /// The head exactly as it arrived, terminator included.
    #[must_use]
    pub fn raw(&self) -> &[u8] {
        &self.raw
    }

    /// The decoded view, which retains everything the bytes carried.
    #[must_use]
    pub const fn request(&self) -> &HttpRequest {
        &self.request
    }

    /// The query keys in the order they arrived, duplicates included.
    ///
    /// ⭐ Order and multiplicity are the observation. Two builds that send the
    /// same parameters in a different order are distinguishable here and
    /// identical in any map.
    #[must_use]
    pub fn query_key_order(&self) -> Vec<Vec<u8>> {
        self.request
            .query_pairs()
            .iter()
            .map(|pair| pair.key().to_vec())
            .collect()
    }

    /// The header names in the order they arrived, duplicates included.
    #[must_use]
    pub fn header_name_order(&self) -> Vec<Vec<u8>> {
        self.request
            .headers()
            .iter()
            .map(|header| header.name().to_vec())
            .collect()
    }

    /// The first value of one header, as it arrived.
    #[must_use]
    pub fn header(&self, name: &[u8]) -> Option<&[u8]> {
        self.request
            .header_values(name)
            .first()
            .map(|header| header.value())
    }

    /// The percent-encoding case one query value used.
    ///
    /// `%1a` and `%1A` are the same byte and different evidence: a build is
    /// consistent within itself, so the case is an identity signal that a
    /// decode-on-arrival parser can never report.
    #[must_use]
    pub fn percent_case(&self, key: &[u8]) -> Option<PercentCase> {
        self.request
            .query_values(key)
            .first()
            .map(bit_ids_wire::tracker_http::QueryPair::percent_case)
    }

    /// The decoded bytes of the first occurrence of one query key.
    ///
    /// `None` means the key was absent. ⚠ The inner error means the value is not
    /// valid percent-encoding, which is an observation about the build and not a
    /// failure of the observer, so it is returned rather than swallowed.
    #[must_use]
    pub fn decoded(&self, key: &[u8]) -> Option<Result<Vec<u8>, WireError>> {
        let pairs = self.request.query_values(key);
        let pair = pairs.first()?;
        match pair.decoded_value() {
            Ok(Some(value)) => Some(Ok(value)),
            // A bare key with no `=` is present and carries nothing.
            Ok(None) => Some(Ok(Vec::new())),
            Err(error) => Some(Err(error)),
        }
    }

    /// The `peer_id` parameter, decoded.
    ///
    /// ⚠ The length is not checked here. BEP 3 fixes it at 20 bytes and
    /// `bit_ids::observation` refuses any other width in a record, but a build
    /// that sends 19 is exactly the measurement this project exists to take, so
    /// the observer reports what arrived.
    #[must_use]
    pub fn peer_id(&self) -> Option<Result<Vec<u8>, WireError>> {
        self.decoded(b"peer_id")
    }

    /// Whether this announce asked for a compact peer list.
    ///
    /// Absent is not the same as `0`: BEP 23 leaves the default to the tracker,
    /// and a build that omits the flag is making a different statement from one
    /// that sends `compact=0`.
    #[must_use]
    pub fn wants_compact(&self) -> Option<bool> {
        let value = self.decoded(b"compact")?.ok()?;
        Some(value == b"1")
    }

    /// Whether this announce asked for peer IDs to be omitted.
    #[must_use]
    pub fn wants_no_peer_id(&self) -> Option<bool> {
        let value = self.decoded(b"no_peer_id")?.ok()?;
        Some(value == b"1")
    }
}

/// One peer the tracker offers back.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OfferedPeer {
    /// The four bytes of an IPv4 address.
    pub address: [u8; 4],
    /// The port, in host order.
    pub port: u16,
    /// The 20 peer-ID bytes, used only by the non-compact form.
    pub peer_id: [u8; 20],
}

impl OfferedPeer {
    fn compact(&self) -> Vec<u8> {
        let mut out = self.address.to_vec();
        out.extend_from_slice(&self.port.to_be_bytes());
        out
    }

    fn dictionary(&self, with_peer_id: bool) -> Value {
        let mut entries = Vec::new();
        if with_peer_id {
            entries.push((b"peer id".to_vec(), Value::bytes(self.peer_id.to_vec())));
        }
        let address = format!(
            "{}.{}.{}.{}",
            self.address[0], self.address[1], self.address[2], self.address[3]
        );
        entries.push((b"ip".to_vec(), Value::bytes(address.into_bytes())));
        entries.push((b"port".to_vec(), Value::integer(i64::from(self.port))));
        // Sorted so the response is canonical bencode and re-encodes to
        // the bytes it was built from, which
        // `http_tracker_answers_a_body_that_decodes_and_re_encodes_unchanged`
        // asserts. An unsorted dictionary is legal bencode and this project has
        // measured nothing about how a client reads one.
        entries.sort_by(|left, right| left.0.cmp(&right.0));
        Value::Dictionary(entries)
    }
}

/// What the observer answers an announce with.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrackerResponse {
    /// Seconds the client is asked to wait before announcing again.
    pub interval: i64,
    /// Seeders.
    pub complete: i64,
    /// Leechers.
    pub incomplete: i64,
    /// The peers offered back.
    pub peers: Vec<OfferedPeer>,
}

impl Default for TrackerResponse {
    /// 1800 seconds, which is far longer than any capture deadline, so a
    /// client re-announces only because the run made it, and no peers.
    fn default() -> Self {
        Self {
            interval: 1800,
            complete: 0,
            incomplete: 0,
            peers: Vec::new(),
        }
    }
}

impl TrackerResponse {
    /// The bencoded body for one announce, in the shape that announce asked for.
    #[must_use]
    pub fn body_for(&self, announce: &Announce) -> Vec<u8> {
        // BEP 23 makes `compact` the client's request and leaves the choice
        // to the tracker when it is absent. This chooses the compact form, and
        // ⚠ that choice is part of the experiment rather than a default: an
        // announce with no flag is answered one way here and the observation is
        // of a build that was answered that way.
        let compact = announce.wants_compact().unwrap_or(true);
        let with_peer_id = !announce.wants_no_peer_id().unwrap_or(false);
        let peers = if compact {
            let mut bytes = Vec::new();
            for peer in &self.peers {
                bytes.extend_from_slice(&peer.compact());
            }
            Value::bytes(bytes)
        } else {
            Value::List(
                self.peers
                    .iter()
                    .map(|peer| peer.dictionary(with_peer_id))
                    .collect(),
            )
        };
        bencode::encode(&Value::Dictionary(vec![
            (b"complete".to_vec(), Value::integer(self.complete)),
            (b"incomplete".to_vec(), Value::integer(self.incomplete)),
            (b"interval".to_vec(), Value::integer(self.interval)),
            (b"peers".to_vec(), peers),
        ]))
    }
}

/// The bencoded body a tracker returns when it refuses a request.
#[must_use]
pub fn failure_body(reason: &str) -> Vec<u8> {
    bencode::encode(&Value::Dictionary(vec![(
        b"failure reason".to_vec(),
        Value::bytes(reason.as_bytes().to_vec()),
    )]))
}

/// Wraps a bencoded body in a minimal HTTP response.
///
/// ⚠ The header set is deliberately small. Every header a tracker sends is
/// something a client may react to, and a reaction to this code is not a
/// measurement of the build.
#[must_use]
pub fn http_response(status: &str, body: &[u8]) -> Vec<u8> {
    let mut out = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n",
        body.len()
    )
    .into_bytes();
    out.extend_from_slice(body);
    out
}

/// The HTTP tracker observer.
///
/// Hand [`HttpTracker::responder`] to a `bit-ids-lab` stream endpoint. The lab
/// records the bytes; this records what they decoded to.
#[derive(Clone, Debug)]
pub struct HttpTracker {
    seen: Arc<Mutex<Record>>,
    response: TrackerResponse,
    max_announces: usize,
}

/// What has been kept, and what was not.
#[derive(Clone, Debug, Default)]
struct Record {
    kept: Vec<Announce>,
    dropped: usize,
}

/// How many announces one observer keeps before it stops keeping them.
///
/// ⛔ The bound exists because the target is untrusted by construction. The
/// lab's deadline bounds how long a client can announce for and not how fast,
/// and a build that announces in a loop would otherwise grow this vector until
/// the host runs out of memory. A capture that needs more sets its own.
pub const DEFAULT_MAX_ANNOUNCES: usize = 4096;

impl HttpTracker {
    /// An observer that answers with `response`.
    #[must_use]
    pub fn new(response: TrackerResponse) -> Self {
        Self {
            seen: Arc::new(Mutex::new(Record::default())),
            response,
            max_announces: DEFAULT_MAX_ANNOUNCES,
        }
    }

    /// How many announces this observer keeps.
    #[must_use]
    pub const fn with_max_announces(mut self, max_announces: usize) -> Self {
        self.max_announces = max_announces;
        self
    }

    /// Every announce kept, in the order it arrived.
    ///
    /// ⚠ A poisoned lock is recovered rather than propagated, for the reason
    /// `bit_ids_lab` gives: observations taken before a panic are still
    /// observations.
    #[must_use]
    pub fn announces(&self) -> Vec<Announce> {
        self.seen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .kept
            .clone()
    }

    /// How many announces arrived after the cap and were answered but not kept.
    ///
    /// ⭐ Counted rather than silently discarded. A record saying it kept 4096
    /// announces, with nothing saying how many there were, is a measurement with
    /// no denominator.
    #[must_use]
    pub fn dropped(&self) -> usize {
        self.seen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .dropped
    }

    /// The responder to give a stream endpoint.
    pub fn responder(&self) -> impl Fn(&[u8]) -> StreamReply + Send + Sync + 'static {
        let seen = Arc::clone(&self.seen);
        let response = self.response.clone();
        let cap = self.max_announces;
        move |buffered: &[u8]| respond(&seen, &response, cap, buffered)
    }
}

impl Default for HttpTracker {
    fn default() -> Self {
        Self::new(TrackerResponse::default())
    }
}

/// How many bytes of one request this observer will hold before giving up.
///
/// The same bound the decoder uses, because a framer with a looser bound than
/// its decoder buffers bytes the decoder will refuse: the target is a binary
/// this project installed minutes earlier, and a head it never terminates is a
/// memory leak with a socket attached.
const MAX_HEAD: usize = HttpRequest::MAX_HEAD;

fn respond(
    seen: &Arc<Mutex<Record>>,
    response: &TrackerResponse,
    cap: usize,
    buffered: &[u8],
) -> StreamReply {
    let Some(end) = head_end(buffered) else {
        if buffered.len() > MAX_HEAD {
            return StreamReply::Close {
                send: http_response(
                    "400 Bad Request",
                    &failure_body("request head has no blank line"),
                ),
            };
        }
        return StreamReply::NeedMore;
    };

    let head = &buffered[..end];
    let Ok(request) = HttpRequest::parse(head) else {
        // ⚠ Refused and not kept as an announce: a head that did not decode is
        // not one. The bytes are not lost, because the lab recorded them before
        // this responder ever saw them, and
        // `http_tracker_leaves_the_bytes_of_a_refused_request_in_the_lab_journal`
        // is what holds the two halves to that.
        return StreamReply::Close {
            send: http_response(
                "400 Bad Request",
                &failure_body("request head did not decode"),
            ),
        };
    };

    // A body is consumed with its head, so a request carrying one does not
    // leave bytes that the next call reads as the start of another request.
    let declared = match content_length(&request) {
        Ok(declared) => declared,
        Err(reason) => {
            return StreamReply::Close {
                send: http_response("400 Bad Request", &failure_body(reason)),
            };
        }
    };
    let total = end.saturating_add(declared);
    if buffered.len() < total {
        return StreamReply::NeedMore;
    }

    let announce = Announce {
        raw: buffered[..total].to_vec(),
        request,
    };
    let body = response.body_for(&announce);
    {
        let mut record = seen.lock().unwrap_or_else(PoisonError::into_inner);
        if record.kept.len() < cap {
            record.kept.push(announce);
        } else {
            record.dropped += 1;
        }
    }
    StreamReply::Answer {
        consumed: total,
        send: http_response("200 OK", &body),
    }
}

/// The declared body length.
///
/// ⚠ A value that is not a number, or that is larger than the head cap, is read
/// as zero rather than believed. Trusting a declared length is how a framer is
/// made to wait for bytes that are never coming.
///
/// ⛔ **Two `Content-Length` headers refuse the request rather than picking
/// one.** They may disagree, and a framer that took the first would consume a
/// different number of bytes from the one the sender meant, so everything after
/// it on that connection is read at the wrong offset. There is no reading of two
/// lengths that is safe, and guessing is what makes the next announce look like
/// a build that sends malformed requests.
fn content_length(request: &HttpRequest) -> Result<usize, &'static str> {
    let headers = request.header_values(b"content-length");
    if headers.len() > 1 {
        return Err("more than one content-length header");
    }
    let Some(header) = headers.first() else {
        return Ok(0);
    };
    let text = core::str::from_utf8(header.value()).unwrap_or("").trim();
    match text.parse::<usize>() {
        Ok(length) if length <= MAX_HEAD => Ok(length),
        _ => Ok(0),
    }
}
