//! The web-seed observer: what a build's HTTP client looks like when the
//! torrent tells it to fetch pieces over HTTP.
//!
//! BEP 19 puts a URL in the torrent's `url-list` and the build fetches the
//! payload from it with ordinary HTTP `GET`s. ⭐ **That makes this the one
//! surface where a build's identity comes from a component it did not write**:
//! the `User-Agent` is usually the HTTP library's rather than the client's, the
//! header order and capitalisation are the library's, and whether the build
//! sends `Accept-Encoding`, `Connection: keep-alive` or a conditional header at
//! all is the library's default. Two clients built on the same library look
//! alike here and different everywhere else, which is a distinction no other
//! surface can draw.
//!
//! # Read by the codec that already exists
//!
//! A BEP 19 fetch is an HTTP request, so
//! [`bit_ids_wire::tracker_http::HttpRequest`] decodes it, the same way
//! [`local_discovery`](crate::local_discovery) reads a BEP 14 announce. ⚠ A
//! second head parser here would be two readings of one grammar that disagree
//! first about exactly the things this observer exists to record.
//!
//! # What it answers, and why the payload is real
//!
//! ⛔ **It serves the bytes the torrent's own pieces hash to.** A web seed that
//! answered with anything else would make every piece fail its hash check, and
//! the build would blacklist the seed and stop fetching, so the run would
//! measure a build reacting to a broken server rather than a build using a web
//! seed. The payload comes from [`bit_ids_lab::SyntheticTorrent`], so the bytes
//! served and the bytes hashed have one source.
//!
//! ⚠ **A `Range` request is answered with `206` and the exact span asked for.**
//! Answering `200` with the whole file is legal HTTP and changes what the build
//! does next, which would be recorded as identity when it is this observer's
//! doing. An unsatisfiable range gets `416`, which is the protocol's own answer
//! rather than an invented one.
//!
//! # Where it sits relative to the containment
//!
//! ⚠ The torrent is what names this endpoint, and
//! [`bit_ids_lab::torrent::WebSeed`] is where that address is checked. This
//! module answers on a socket the lab bound and hands out no addresses at all,
//! so the third door does not open here.

use std::sync::{Arc, Mutex, PoisonError};

use bit_ids_lab::adjacent::{Capability, NotEnabled, Surface, require};
use bit_ids_lab::endpoint::{ConnectionId, StreamReply};
use bit_ids_wire::tracker_http::{HttpRequest, head_end};

/// How many fetches one observer keeps before it stops keeping them.
///
/// ⛔ Bounded for the reason every other observer's record is: a build that
/// fetches in a loop would otherwise grow this vector until the host runs out of
/// memory.
pub const DEFAULT_MAX_FETCHES: usize = 4096;

/// How many bytes of one request head this observer holds before giving up.
///
/// The same bound the decoder uses, because a framer with a looser bound than
/// its decoder buffers bytes the decoder will refuse.
const MAX_HEAD: usize = HttpRequest::MAX_HEAD;

/// The byte span a `Range` header asked for.
///
/// ⚠ Kept as what was asked rather than as what was served, because a build that
/// asks for a whole piece and one that asks for a fixed window are
/// distinguishable and only the request says which.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Requested {
    /// No `Range` header: the build asked for the whole file.
    Whole,
    /// `bytes=first-last`, inclusive at both ends as HTTP defines it.
    Span {
        /// The first byte offset asked for.
        first: u64,
        /// The last byte offset asked for, inclusive.
        last: u64,
    },
    /// `bytes=first-` with no end, meaning everything from `first`.
    From {
        /// The first byte offset asked for.
        first: u64,
    },
    /// A `Range` header this observer could not read, kept as it arrived.
    ///
    /// ⚠ A finding rather than an error. A build sending a range unit that is
    /// not `bytes`, or a multi-range request, has told us something, and the
    /// bytes are what say what.
    Unreadable(
        /// The header value, as sent.
        &'static str,
    ),
}

/// Why a fetch is not what BEP 19 describes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Refusal {
    /// The head does not decode as an HTTP request.
    NotAMessage(String),
    /// The method is not `GET`.
    ///
    /// ⚠ Recorded rather than refused. A build that sends `HEAD` first to learn
    /// the length is doing something a specification permits and a great many
    /// builds do not.
    NotAGet(Vec<u8>),
    /// A `Range` header whose value this observer could not read.
    RangeUnreadable(Vec<u8>),
    /// A range that starts past the end of the payload.
    RangeUnsatisfiable {
        /// Where the build asked to start.
        first: u64,
        /// How many bytes there are.
        length: u64,
    },
}

impl Refusal {
    /// The refusal in one line.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::NotAMessage(why) => format!("not an HTTP request: {why}"),
            Self::NotAGet(method) => format!(
                "method is {:?} rather than GET",
                String::from_utf8_lossy(method)
            ),
            Self::RangeUnreadable(value) => format!(
                "Range {:?} is not a single byte range",
                String::from_utf8_lossy(value)
            ),
            Self::RangeUnsatisfiable { first, length } => {
                format!("a range starting at {first} over {length} bytes")
            }
        }
    }
}

/// One fetch, as it arrived, with what the observer answered.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fetch {
    raw: Vec<u8>,
    request: Option<HttpRequest>,
    status: Option<u16>,
    served: usize,
    refusals: Vec<Refusal>,
}

impl Fetch {
    /// Every byte of the request head, in the order it arrived.
    #[must_use]
    pub fn raw(&self) -> &[u8] {
        &self.raw
    }

    /// The decoded request, when the head decoded at all.
    #[must_use]
    pub const fn request(&self) -> Option<&HttpRequest> {
        self.request.as_ref()
    }

    /// The status the observer answered with.
    ///
    /// ⭐ Recorded because it is a condition of the measurement: what a build
    /// does next depends on what it heard.
    #[must_use]
    pub const fn status(&self) -> Option<u16> {
        self.status
    }

    /// How many payload bytes were served.
    #[must_use]
    pub const fn served(&self) -> usize {
        self.served
    }

    /// Everything about this fetch BEP 19 does not describe.
    #[must_use]
    pub fn refusals(&self) -> &[Refusal] {
        &self.refusals
    }

    /// Whether this fetch is one BEP 19 describes.
    #[must_use]
    pub fn is_conforming(&self) -> bool {
        self.refusals.is_empty()
    }

    /// The header names in the case and order the build sent them.
    ///
    /// ⭐ The strongest signal on this surface. HTTP fixes neither the order of
    /// headers nor their capitalisation, and a build's HTTP library is
    /// consistent with itself.
    #[must_use]
    pub fn header_order(&self) -> Vec<Vec<u8>> {
        self.request.as_ref().map_or_else(Vec::new, |request| {
            request
                .headers()
                .iter()
                .map(|header| header.name().to_vec())
                .collect()
        })
    }

    /// The values of one header, matched without regard to case, in order.
    #[must_use]
    pub fn header(&self, name: &[u8]) -> Vec<&[u8]> {
        self.request.as_ref().map_or_else(Vec::new, |request| {
            request
                .headers()
                .iter()
                .filter(|header| header.name().eq_ignore_ascii_case(name))
                .map(bit_ids_wire::tracker_http::Header::value)
                .collect()
        })
    }

    /// The `User-Agent` the build's HTTP client sent.
    ///
    /// ⛔ **Kept as bytes and never resolved to a client name.** It is usually
    /// the HTTP library's own string rather than the client's, which is
    /// precisely why it is worth recording and precisely why turning it into a
    /// name would be wrong twice.
    #[must_use]
    pub fn user_agent(&self) -> Option<&[u8]> {
        self.header(b"User-Agent").first().copied()
    }

    /// What the `Range` header asked for.
    #[must_use]
    pub fn requested(&self) -> Requested {
        match self.header(b"Range").first() {
            None => Requested::Whole,
            Some(value) => read_range(value).unwrap_or(Requested::Unreadable("not a byte range")),
        }
    }
}

/// Reads a `Range` header value as a single byte range.
///
/// ⚠ Single only. A multi-range request is legal HTTP and would need a
/// multipart response; a build that sends one is recorded as
/// [`Requested::Unreadable`] rather than answered with a shape this observer
/// would then be measuring the build's reaction to.
fn read_range(value: &[u8]) -> Option<Requested> {
    let text = core::str::from_utf8(value).ok()?;
    let spec = text.trim().strip_prefix("bytes=")?;
    if spec.contains(',') {
        return None;
    }
    let (first, last) = spec.split_once('-')?;
    let first: u64 = first.trim().parse().ok()?;
    let last = last.trim();
    if last.is_empty() {
        return Some(Requested::From { first });
    }
    let last: u64 = last.parse().ok()?;
    if last < first {
        return None;
    }
    Some(Requested::Span { first, last })
}

#[derive(Debug, Default)]
struct Record {
    kept: Vec<Fetch>,
    dropped: usize,
}

/// The BEP 19 observer.
///
/// ⛔ Built only from a [`Capability`] for [`Surface::WebSeed`].
#[derive(Debug)]
pub struct WebSeedServer {
    seen: Arc<Mutex<Record>>,
    payload: Arc<Vec<u8>>,
    max_fetches: usize,
}

impl WebSeedServer {
    /// An observer serving `payload`, if web seeding was turned on.
    ///
    /// ⛔ `payload` is the torrent's own payload. See the module documentation
    /// for why serving anything else measures this code rather than the build.
    ///
    /// # Errors
    ///
    /// Returns [`NotEnabled`] when `capability` enables a different surface.
    pub fn new(capability: Capability, payload: Vec<u8>) -> Result<Self, NotEnabled> {
        require(capability, Surface::WebSeed)?;
        Ok(Self {
            seen: Arc::new(Mutex::new(Record::default())),
            payload: Arc::new(payload),
            max_fetches: DEFAULT_MAX_FETCHES,
        })
    }

    /// How many fetches this observer keeps.
    #[must_use]
    pub const fn with_max_fetches(mut self, max_fetches: usize) -> Self {
        self.max_fetches = max_fetches;
        self
    }

    /// Every fetch kept, in the order it arrived.
    #[must_use]
    pub fn fetches(&self) -> Vec<Fetch> {
        self.locked().kept.clone()
    }

    /// How many fetches arrived after the cap and were answered but not kept.
    #[must_use]
    pub fn dropped(&self) -> usize {
        self.locked().dropped
    }

    /// ⚠ A poisoned lock is recovered rather than propagated.
    fn locked(&self) -> std::sync::MutexGuard<'_, Record> {
        self.seen.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The responder to give a stream endpoint.
    pub fn responder(&self) -> impl Fn(ConnectionId, &[u8]) -> StreamReply + Send + Sync + 'static {
        let seen = Arc::clone(&self.seen);
        let payload = Arc::clone(&self.payload);
        let cap = self.max_fetches;
        move |_connection, buffered: &[u8]| respond(&seen, &payload, cap, buffered)
    }
}

/// Frames one request out of the buffer and answers it.
fn respond(
    seen: &Arc<Mutex<Record>>,
    payload: &Arc<Vec<u8>>,
    cap: usize,
    buffered: &[u8],
) -> StreamReply {
    let Some(end) = head_end(buffered) else {
        if buffered.len() > MAX_HEAD {
            return StreamReply::Close {
                send: head(400, "Bad Request", 0, None, payload.len()),
            };
        }
        return StreamReply::NeedMore;
    };
    let head_bytes = &buffered[..end];
    let (status, body, refusals, request) = answer(payload, head_bytes);
    let mut send = head(
        status,
        reason(status),
        body.len(),
        content_range(status, payload, head_bytes),
        payload.len(),
    );
    send.extend_from_slice(&body);
    keep(
        seen,
        cap,
        Fetch {
            raw: head_bytes.to_vec(),
            request,
            status: Some(status),
            served: body.len(),
            refusals,
        },
    );
    // ⚠ The connection stays open. BEP 19 fetches are usually several requests
    // on one connection, and closing after each would make every build look like
    // one that does not reuse connections.
    StreamReply::Answer {
        consumed: end,
        send,
    }
}

/// What to answer one decoded head with.
fn answer(
    payload: &Arc<Vec<u8>>,
    head_bytes: &[u8],
) -> (u16, Vec<u8>, Vec<Refusal>, Option<HttpRequest>) {
    let request = match HttpRequest::parse(head_bytes) {
        Ok(request) => request,
        Err(error) => {
            return (
                400,
                Vec::new(),
                vec![Refusal::NotAMessage(error.to_string())],
                None,
            );
        }
    };
    let mut refusals = Vec::new();
    if request.method() != b"GET" {
        refusals.push(Refusal::NotAGet(request.method().to_vec()));
    }
    let fetch = Fetch {
        raw: head_bytes.to_vec(),
        request: Some(request),
        status: None,
        served: 0,
        refusals: Vec::new(),
    };
    let length = payload.len() as u64;
    match fetch.requested() {
        Requested::Whole => (200, payload.as_ref().clone(), refusals, fetch.request),
        Requested::Unreadable(_) => {
            let value = fetch
                .header(b"Range")
                .first()
                .map_or_else(Vec::new, |value| (*value).to_vec());
            refusals.push(Refusal::RangeUnreadable(value));
            // ⚠ Answered as if no range were sent, which is what HTTP says to do
            // with a Range header a server cannot use, rather than an error the
            // build would then be measured reacting to.
            (200, payload.as_ref().clone(), refusals, fetch.request)
        }
        Requested::From { first } | Requested::Span { first, .. } if first >= length => {
            refusals.push(Refusal::RangeUnsatisfiable { first, length });
            (416, Vec::new(), refusals, fetch.request)
        }
        Requested::From { first } => {
            let body = payload[usize_of(first)..].to_vec();
            (206, body, refusals, fetch.request)
        }
        Requested::Span { first, last } => {
            // ⚠ Clamped to the end, which is what HTTP requires of a range whose
            // last byte is past the payload. Refusing it would be a shape a
            // build's reaction gets recorded as identity.
            let end = usize_of(last.min(length - 1)) + 1;
            let body = payload[usize_of(first)..end].to_vec();
            (206, body, refusals, fetch.request)
        }
    }
}

/// The `Content-Range` header a `206` needs, and nothing for any other status.
fn content_range(status: u16, payload: &Arc<Vec<u8>>, head_bytes: &[u8]) -> Option<String> {
    if status != 206 {
        return None;
    }
    let request = HttpRequest::parse(head_bytes).ok()?;
    let value = request
        .headers()
        .iter()
        .find(|header| header.name().eq_ignore_ascii_case(b"Range"))
        .map(bit_ids_wire::tracker_http::Header::value)?;
    let length = payload.len() as u64;
    match read_range(value)? {
        Requested::From { first } => Some(format!("bytes {first}-{}/{length}", length - 1)),
        Requested::Span { first, last } => {
            Some(format!("bytes {first}-{}/{length}", last.min(length - 1)))
        }
        Requested::Whole | Requested::Unreadable(_) => None,
    }
}

/// ⚠ A cast that cannot lose bytes on any host this runs on, written once.
/// The payload is bounded by `MAX_PAYLOAD_BYTES`, far below `usize::MAX` on a
/// 32-bit host, and a range past the end is refused before reaching here.
fn usize_of(value: u64) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

/// The reason phrase for a status this observer sends.
const fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        206 => "Partial Content",
        416 => "Range Not Satisfiable",
        _ => "Bad Request",
    }
}

/// A response head.
///
/// ⚠ `Accept-Ranges` is sent on every response, because a build that learns the
/// server does not support ranges stops sending them, and the shape of a build's
/// range requests is most of what this surface measures.
fn head(status: u16, phrase: &str, length: usize, range: Option<String>, total: usize) -> Vec<u8> {
    use core::fmt::Write as _;

    let mut out = format!(
        "HTTP/1.1 {status} {phrase}\r\nContent-Length: {length}\r\nAccept-Ranges: bytes\r\n"
    );
    if let Some(range) = range {
        write!(out, "Content-Range: {range}\r\n").expect("writing to a String cannot fail");
    }
    if status == 416 {
        write!(out, "Content-Range: bytes */{total}\r\n").expect("writing to a String cannot fail");
    }
    out.push_str("\r\n");
    out.into_bytes()
}

/// Records one fetch, counting it instead once the cap is reached.
fn keep(seen: &Arc<Mutex<Record>>, cap: usize, fetch: Fetch) {
    let mut record = seen.lock().unwrap_or_else(PoisonError::into_inner);
    if record.kept.len() >= cap {
        record.dropped += 1;
        return;
    }
    record.kept.push(fetch);
}

#[cfg(test)]
mod tests {
    use super::{Refusal, Requested, WebSeedServer};
    use bit_ids_lab::adjacent::{ALL_SURFACES as ALL, Capability, Surface};
    use bit_ids_lab::endpoint::{ConnectionId, StreamReply};

    fn payload() -> Vec<u8> {
        (0..=u8::MAX).collect()
    }

    fn observer() -> WebSeedServer {
        WebSeedServer::new(Capability::enable(Surface::WebSeed), payload())
            .expect("the capability names this surface")
    }

    fn connection() -> ConnectionId {
        ConnectionId::recorded(1).expect("one is a real connection")
    }

    /// Splits a reply into its head and its body.
    fn split(reply: &[u8]) -> (String, Vec<u8>) {
        let end = bit_ids_wire::tracker_http::head_end(reply).expect("a reply has a head");
        (
            String::from_utf8_lossy(&reply[..end]).into_owned(),
            reply[end..].to_vec(),
        )
    }

    fn ask(observer: &WebSeedServer, head: &str) -> (u16, Vec<u8>) {
        let responder = observer.responder();
        let StreamReply::Answer { consumed, send } = responder(connection(), head.as_bytes())
        else {
            panic!("a complete head is answered");
        };
        assert_eq!(consumed, head.len(), "the whole head was consumed");
        let (head, body) = split(&send);
        let status: u16 = head
            .split_whitespace()
            .nth(1)
            .expect("a status line")
            .parse()
            .expect("a numeric status");
        (status, body)
    }

    #[test]
    fn the_observer_is_refused_without_a_capability_for_its_own_surface() {
        assert!(WebSeedServer::new(Capability::enable(Surface::WebSeed), payload()).is_ok());
        for other in ALL {
            if other == Surface::WebSeed {
                continue;
            }
            let refusal = WebSeedServer::new(Capability::enable(other), payload())
                .expect_err("a different surface");
            assert_eq!(refusal.wanted, Surface::WebSeed);
            assert_eq!(refusal.offered, other);
        }
    }

    /// ⛔ The bytes served are the torrent's own, so every piece hashes. A seed
    /// answering anything else would be blacklisted by the build and the run
    /// would measure a build reacting to a broken server.
    #[test]
    fn a_whole_file_fetch_is_answered_with_the_payload_itself() {
        let observer = observer();
        let (status, body) = ask(
            &observer,
            "GET /payload HTTP/1.1\r\nHost: 127.0.0.1:8080\r\nUser-Agent: some-http-lib/1.0\r\n\r\n",
        );
        assert_eq!(status, 200);
        assert_eq!(body, payload());

        let kept = observer.fetches();
        assert_eq!(kept.len(), 1);
        assert!(kept[0].is_conforming(), "{:?}", kept[0].refusals());
        assert_eq!(kept[0].requested(), Requested::Whole);
        assert_eq!(kept[0].user_agent(), Some(&b"some-http-lib/1.0"[..]));
        assert_eq!(kept[0].status(), Some(200));
        assert_eq!(kept[0].served(), payload().len());
    }

    /// ⚠ The exact span asked for, and a `206`. Answering `200` with the whole
    /// file is legal HTTP and changes what the build does next.
    #[test]
    fn a_range_request_gets_exactly_the_span_it_asked_for() {
        let observer = observer();
        let (status, body) = ask(
            &observer,
            "GET /payload HTTP/1.1\r\nHost: x\r\nRange: bytes=16-31\r\n\r\n",
        );
        assert_eq!(status, 206);
        assert_eq!(body, payload()[16..=31]);
        assert_eq!(
            observer.fetches()[0].requested(),
            Requested::Span {
                first: 16,
                last: 31
            }
        );

        // An open-ended range.
        let (status, body) = ask(
            &observer,
            "GET /payload HTTP/1.1\r\nHost: x\r\nRange: bytes=240-\r\n\r\n",
        );
        assert_eq!(status, 206);
        assert_eq!(body, payload()[240..]);
        assert_eq!(
            observer.fetches()[1].requested(),
            Requested::From { first: 240 }
        );
    }

    /// ⚠ A last byte past the end is clamped, which is what HTTP requires. A
    /// build asking for a whole piece at the end of a file does exactly this.
    #[test]
    fn a_range_running_past_the_end_is_clamped_rather_than_refused() {
        let observer = observer();
        let (status, body) = ask(
            &observer,
            "GET /payload HTTP/1.1\r\nHost: x\r\nRange: bytes=250-9999\r\n\r\n",
        );
        assert_eq!(status, 206);
        assert_eq!(body, payload()[250..]);
        assert!(
            observer.fetches()[0].is_conforming(),
            "{:?}",
            observer.fetches()[0].refusals()
        );
    }

    #[test]
    fn a_range_starting_past_the_end_gets_the_protocols_own_refusal() {
        let observer = observer();
        let (status, body) = ask(
            &observer,
            "GET /payload HTTP/1.1\r\nHost: x\r\nRange: bytes=9999-\r\n\r\n",
        );
        assert_eq!(status, 416);
        assert!(body.is_empty());
        assert!(
            observer.fetches()[0]
                .refusals()
                .contains(&Refusal::RangeUnsatisfiable {
                    first: 9999,
                    length: 256
                })
        );
    }

    /// ⚠ A multi-range request and a non-byte unit are findings, and each is
    /// answered as though no range had been sent rather than with an error.
    #[test]
    fn a_range_this_observer_cannot_read_is_recorded_and_the_whole_file_is_sent() {
        for value in ["bytes=0-15,32-47", "pieces=0-1", "bytes=nonsense"] {
            let observer = observer();
            let (status, body) = ask(
                &observer,
                &format!("GET /p HTTP/1.1\r\nHost: x\r\nRange: {value}\r\n\r\n"),
            );
            assert_eq!(status, 200, "{value}");
            assert_eq!(body, payload(), "{value}");
            assert!(
                observer.fetches()[0]
                    .refusals()
                    .iter()
                    .any(|why| matches!(why, Refusal::RangeUnreadable(_))),
                "{value}: {:?}",
                observer.fetches()[0].refusals()
            );
        }
    }

    /// ⭐ The header order and case a build's HTTP library used survive the
    /// reading. Two clients on one library look alike here, which is the
    /// distinction this surface exists to draw.
    #[test]
    fn the_header_order_and_case_a_build_used_survive_the_reading() {
        let observer = observer();
        ask(
            &observer,
            "GET /p HTTP/1.1\r\nhost: x\r\nAccept-Encoding: gzip\r\nUSER-AGENT: odd/2\r\nRange: bytes=0-0\r\n\r\n",
        );
        let kept = observer.fetches();
        assert_eq!(
            kept[0].header_order(),
            vec![
                b"host".to_vec(),
                b"Accept-Encoding".to_vec(),
                b"USER-AGENT".to_vec(),
                b"Range".to_vec(),
            ]
        );
        // ⚠ Matched without regard to case, so an odd spelling is still found.
        assert_eq!(kept[0].user_agent(), Some(&b"odd/2"[..]));
    }

    /// ⚠ A `HEAD` is a finding rather than an error: a build that asks for the
    /// length first is doing something many do not.
    #[test]
    fn a_method_that_is_not_get_is_recorded_and_still_answered() {
        let observer = observer();
        let (status, _) = ask(&observer, "HEAD /p HTTP/1.1\r\nHost: x\r\n\r\n");
        assert_eq!(status, 200);
        assert!(
            observer.fetches()[0]
                .refusals()
                .contains(&Refusal::NotAGet(b"HEAD".to_vec()))
        );
    }

    /// ⛔ The connection stays open. Closing after each request would make every
    /// build look like one that does not reuse connections.
    #[test]
    fn two_fetches_on_one_connection_are_both_answered() {
        let observer = observer();
        let responder = observer.responder();
        let first = "GET /a HTTP/1.1\r\nHost: x\r\nRange: bytes=0-3\r\n\r\n";
        let second = "GET /b HTTP/1.1\r\nHost: x\r\nRange: bytes=4-7\r\n\r\n";
        let both = format!("{first}{second}");

        let StreamReply::Answer { consumed, .. } = responder(connection(), both.as_bytes()) else {
            panic!("the first request is answered");
        };
        assert_eq!(consumed, first.len(), "only the first head was consumed");
        let StreamReply::Answer { consumed, .. } =
            responder(connection(), &both.as_bytes()[consumed..])
        else {
            panic!("the second request is answered");
        };
        assert_eq!(consumed, second.len());
        assert_eq!(observer.fetches().len(), 2);
    }

    #[test]
    fn a_head_with_no_blank_line_yet_asks_for_more_and_is_bounded() {
        let observer = observer();
        let responder = observer.responder();
        assert_eq!(
            responder(connection(), b"GET /p HTTP/1.1\r\nHost: x\r\n"),
            StreamReply::NeedMore
        );
        assert!(observer.fetches().is_empty(), "nothing complete arrived");

        // ⛔ And it is bounded: a build that never terminates its head is a
        // memory leak with a socket attached.
        let huge = vec![b'a'; super::MAX_HEAD + 1];
        assert!(matches!(
            responder(connection(), &huge),
            StreamReply::Close { .. }
        ));
    }

    #[test]
    fn fetches_past_the_cap_are_counted_rather_than_kept() {
        let observer = observer().with_max_fetches(2);
        for _ in 0..5 {
            ask(&observer, "GET /p HTTP/1.1\r\nHost: x\r\n\r\n");
        }
        assert_eq!(observer.fetches().len(), 2);
        assert_eq!(observer.dropped(), 3);
    }
}
