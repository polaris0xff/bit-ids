//! The local service discovery observer: what a build shouts at the LAN before
//! anyone has told it about a peer.
//!
//! BEP 14 sends one HTTP-shaped announce to a fixed multicast group. ⭐ **A
//! client sends it unprompted**, so it is observable in a lab that never
//! answers a tracker, and its shape is almost entirely convention: the
//! specification fixes the request line and the field names and says nothing
//! about their order, their case, whether a cookie is present, or how the
//! message ends. Every one of those is a build's own choice.
//!
//! ⛔ **This observer answers nothing, ever.** BEP 14 defines no reply. A
//! response would be this project inventing a protocol and then recording what
//! a client did when it received one, which is a measurement of this code. The
//! responder returns [`None`] on every datagram and
//! `answers_nothing_on_any_input` is what holds it to that.
//!
//! ⛔ **The addresses in [`GROUP_V4`] and [`GROUP_V6`] are here to be refused,
//! not used.** They are what a real client's announce is addressed to, and the
//! lab must never send there: a lab that joined the group would put a synthetic
//! announce on somebody's LAN. `bind::send_to` refuses them, and
//! `the_group_this_protocol_names_is_refused_by_the_lab` drives it with these
//! constants rather than with an address chosen to pass.
//!
//! # Parsed by the codec that already exists
//!
//! An LSD announce is an HTTP request with a different method, so
//! [`bit_ids_wire::tracker_http::HttpRequest`] decodes it. ⚠ A second head
//! parser here would be the divergent-copies defect `docs/methodology/reviews.md`
//! names, and the two copies would disagree first about exactly the things this
//! observer exists to record: header case, field order and line terminators.
//!
//! # What a refusal is
//!
//! ⚠ **A refused announce is kept.** [`Refusal`] says a build sent something
//! BEP 14 does not describe, which is a finding about the build rather than a
//! reason to drop evidence. The raw bytes are recorded either way.

use std::sync::{Arc, Mutex, PoisonError};

use bit_ids_lab::adjacent::{Capability, NotEnabled, Surface, require};
use bit_ids_wire::tracker_http::HttpRequest;

use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

/// The IPv4 multicast group and port BEP 14 fixes.
pub const GROUP_V4: SocketAddr =
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(239, 192, 152, 143), 6771));

/// The IPv6 multicast group and port BEP 14 fixes.
pub const GROUP_V6: SocketAddr = SocketAddr::V6(SocketAddrV6::new(
    Ipv6Addr::new(0xff15, 0, 0, 0, 0, 0, 0xefc0, 0x988f),
    6771,
    0,
    0,
));

/// The method BEP 14 fixes.
pub const METHOD: &[u8] = b"BT-SEARCH";

/// How many hexadecimal characters an info hash is written with.
pub const INFO_HASH_HEX_LEN: usize = 40;

/// How many announces one observer keeps before it stops keeping them.
///
/// ⛔ Bounded for the reason every other observer's record is: the target is a
/// binary this project installed minutes ago, and a build that announces in a
/// loop would otherwise grow this vector until the host runs out of memory.
pub const DEFAULT_MAX_ANNOUNCES: usize = 4096;

/// How an announce ended, after the blank line that closes its head.
///
/// ⭐ BEP 14's own example ends with an extra empty line, and implementations
/// disagree about whether to send it. It costs nothing to record and it cannot
/// be recovered once a parser has normalised the message.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Trailer {
    /// Nothing after the head's blank line.
    None,
    /// One further empty line, as BEP 14's example shows.
    Blank(Vec<u8>),
    /// Something else, kept as it arrived.
    Other(Vec<u8>),
}

/// Why an announce is not what BEP 14 describes.
///
/// ⚠ Every variant is a finding about the build under measurement. None of them
/// discards the bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Refusal {
    /// The head does not decode as an HTTP-shaped message at all.
    NotAMessage(String),
    /// The method is not `BT-SEARCH`.
    NotBtSearch(Vec<u8>),
    /// No `Infohash` field, so the announce names no torrent.
    NoInfoHash,
    /// An `Infohash` field that is not forty hexadecimal characters.
    InfoHashNotHex(Vec<u8>),
    /// No `Port` field, so the announce names no peer to dial.
    NoPort,
    /// A `Port` field that is not a number in range.
    PortNotANumber(Vec<u8>),
}

impl Refusal {
    /// The refusal in one line.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::NotAMessage(why) => format!("not an HTTP-shaped message: {why}"),
            Self::NotBtSearch(method) => {
                format!(
                    "method is {:?} rather than BT-SEARCH",
                    String::from_utf8_lossy(method)
                )
            }
            Self::NoInfoHash => "no Infohash field".to_owned(),
            Self::InfoHashNotHex(value) => format!(
                "Infohash {:?} is not {INFO_HASH_HEX_LEN} hexadecimal characters",
                String::from_utf8_lossy(value)
            ),
            Self::NoPort => "no Port field".to_owned(),
            Self::PortNotANumber(value) => {
                format!(
                    "Port {:?} is not a number in range",
                    String::from_utf8_lossy(value)
                )
            }
        }
    }
}

/// One announce, as it arrived.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Announce {
    raw: Vec<u8>,
    request: Option<HttpRequest>,
    refusals: Vec<Refusal>,
}

impl Announce {
    /// Every byte of the datagram, in the order it arrived.
    ///
    /// ⛔ The measurement. Everything else on this type is a reading of these
    /// bytes and can be re-derived from them.
    #[must_use]
    pub fn raw(&self) -> &[u8] {
        &self.raw
    }

    /// The decoded message, when the head decoded at all.
    #[must_use]
    pub const fn request(&self) -> Option<&HttpRequest> {
        self.request.as_ref()
    }

    /// Everything about this announce that BEP 14 does not describe.
    ///
    /// Empty for an announce that matches the specification.
    #[must_use]
    pub fn refusals(&self) -> &[Refusal] {
        &self.refusals
    }

    /// Whether this announce is one BEP 14 describes.
    #[must_use]
    pub fn is_conforming(&self) -> bool {
        self.refusals.is_empty()
    }

    /// The field names in the case and order the build sent them.
    ///
    /// ⭐ The strongest single signal here. BEP 14 lists `Host`, `Port`,
    /// `Infohash` and `cookie` and fixes neither their order nor their case,
    /// and a build is consistent with itself.
    #[must_use]
    pub fn field_order(&self) -> Vec<Vec<u8>> {
        self.request.as_ref().map_or_else(Vec::new, |request| {
            request
                .headers()
                .iter()
                .map(|header| header.name().to_vec())
                .collect()
        })
    }

    /// The values of one field, matched without regard to case, in order.
    #[must_use]
    pub fn field(&self, name: &[u8]) -> Vec<&[u8]> {
        self.request.as_ref().map_or_else(Vec::new, |request| {
            request
                .headers()
                .iter()
                .filter(|header| header.name().eq_ignore_ascii_case(name))
                .map(bit_ids_wire::tracker_http::Header::value)
                .collect()
        })
    }

    /// Every info hash this announce names, in order.
    ///
    /// ⚠ A list rather than one value. BEP 14 allows an announce to carry a
    /// field per torrent, and a build that announces several at once is
    /// distinguishable from one that sends a datagram each.
    #[must_use]
    pub fn info_hashes(&self) -> Vec<&[u8]> {
        self.field(b"Infohash")
    }

    /// The port the build says it listens on.
    #[must_use]
    pub fn port(&self) -> Option<u16> {
        let value = *self.field(b"Port").first()?;
        core::str::from_utf8(value).ok()?.parse().ok()
    }

    /// The cookie a build uses to recognise its own announces, if it sent one.
    #[must_use]
    pub fn cookie(&self) -> Option<&[u8]> {
        self.field(b"cookie").first().copied()
    }

    /// The `Host` field as sent.
    #[must_use]
    pub fn host(&self) -> Option<&[u8]> {
        self.field(b"Host").first().copied()
    }

    /// How the message ended after its head.
    #[must_use]
    pub fn trailer(&self) -> Trailer {
        let Some(request) = self.request.as_ref() else {
            return Trailer::None;
        };
        let body = request.body();
        if body.is_empty() {
            Trailer::None
        } else if body == b"\r\n" || body == b"\n" {
            Trailer::Blank(body.to_vec())
        } else {
            Trailer::Other(body.to_vec())
        }
    }

    /// Whether re-encoding the decoded message gives the bytes back exactly.
    ///
    /// ⭐ The lossless-decode invariant `bit-ids-wire` is built on, asserted per
    /// announce rather than assumed. A reading that cannot reproduce its input
    /// has dropped something, and what it dropped is the part no reader knew to
    /// look at.
    #[must_use]
    pub fn rebuilds_from_raw(&self) -> bool {
        self.request
            .as_ref()
            .is_some_and(|request| request.encode() == self.raw)
    }
}

#[derive(Debug, Default)]
struct Record {
    kept: Vec<Announce>,
    dropped: usize,
}

/// The BEP 14 observer.
///
/// ⛔ Built only from a [`Capability`] for [`Surface::LocalDiscovery`]. See
/// `bit_ids_lab::adjacent` for what that switch is and is not.
#[derive(Debug)]
pub struct LocalDiscovery {
    seen: Arc<Mutex<Record>>,
    max_announces: usize,
}

impl LocalDiscovery {
    /// An observer, if local discovery was turned on.
    ///
    /// # Errors
    ///
    /// Returns [`NotEnabled`] when `capability` enables a different surface.
    pub fn new(capability: Capability) -> Result<Self, NotEnabled> {
        require(capability, Surface::LocalDiscovery)?;
        Ok(Self {
            seen: Arc::new(Mutex::new(Record::default())),
            max_announces: DEFAULT_MAX_ANNOUNCES,
        })
    }

    /// How many announces this observer keeps.
    #[must_use]
    pub const fn with_max_announces(mut self, max_announces: usize) -> Self {
        self.max_announces = max_announces;
        self
    }

    /// Every announce kept, in the order it arrived.
    #[must_use]
    pub fn announces(&self) -> Vec<Announce> {
        self.seen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .kept
            .clone()
    }

    /// How many announces arrived after the cap and were recorded nowhere.
    #[must_use]
    pub fn dropped(&self) -> usize {
        self.seen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .dropped
    }

    /// The responder to give a datagram endpoint.
    ///
    /// ⛔ It returns [`None`] for every input. BEP 14 has no reply, and one
    /// invented here would be measured as the client's behaviour.
    pub fn observing(&self) -> impl Fn(&[u8]) -> Option<Vec<u8>> + Send + Sync + 'static {
        let seen = Arc::clone(&self.seen);
        let cap = self.max_announces;
        move |datagram: &[u8]| {
            observe(&seen, cap, datagram);
            None
        }
    }
}

/// Decodes one datagram and records it.
fn observe(seen: &Arc<Mutex<Record>>, cap: usize, datagram: &[u8]) {
    let announce = read(datagram);
    let mut record = seen.lock().unwrap_or_else(PoisonError::into_inner);
    if record.kept.len() >= cap {
        record.dropped += 1;
        return;
    }
    record.kept.push(announce);
}

/// Reads one datagram as a BEP 14 announce, keeping every refusal.
///
/// ⚠ Every check runs. Stopping at the first refusal would report a build's
/// first divergence and hide the rest, and the set of things a build gets wrong
/// is more identifying than the first one.
#[must_use]
pub fn read(datagram: &[u8]) -> Announce {
    let request = match HttpRequest::parse(datagram) {
        Ok(request) => request,
        Err(error) => {
            return Announce {
                raw: datagram.to_vec(),
                request: None,
                refusals: vec![Refusal::NotAMessage(error.to_string())],
            };
        }
    };
    let mut announce = Announce {
        raw: datagram.to_vec(),
        request: Some(request),
        refusals: Vec::new(),
    };
    announce.refusals = refusals_of(&announce);
    announce
}

/// Every way one decoded announce departs from BEP 14.
fn refusals_of(announce: &Announce) -> Vec<Refusal> {
    let mut refusals = Vec::new();
    let request = announce
        .request
        .as_ref()
        .expect("refusals_of is called only with a decoded request");
    if request.method() != METHOD {
        refusals.push(Refusal::NotBtSearch(request.method().to_vec()));
    }
    let hashes = announce.info_hashes();
    if hashes.is_empty() {
        refusals.push(Refusal::NoInfoHash);
    }
    for hash in hashes {
        if hash.len() != INFO_HASH_HEX_LEN || !hash.iter().all(u8::is_ascii_hexdigit) {
            refusals.push(Refusal::InfoHashNotHex(hash.to_vec()));
        }
    }
    match announce.field(b"Port").first() {
        None => refusals.push(Refusal::NoPort),
        Some(value) => {
            let parsed = core::str::from_utf8(value)
                .ok()
                .and_then(|text| text.parse::<u16>().ok());
            if parsed.is_none_or(|port| port == 0) {
                refusals.push(Refusal::PortNotANumber((*value).to_vec()));
            }
        }
    }
    refusals
}

#[cfg(test)]
mod tests {
    use super::{GROUP_V4, GROUP_V6, LocalDiscovery, Refusal, Trailer, read};
    use bit_ids_lab::adjacent::{ALL_SURFACES as ALL, Capability, Surface};

    const CONFORMING: &[u8] = b"BT-SEARCH * HTTP/1.1\r\nHost: 239.192.152.143:6771\r\nPort: 6881\r\nInfohash: 0123456789abcdef0123456789abcdef01234567\r\ncookie: bit-ids-1\r\n\r\n\r\n";

    #[test]
    fn a_conforming_announce_is_read_and_rebuilds_from_its_own_bytes() {
        let announce = read(CONFORMING);
        assert!(announce.is_conforming(), "{:?}", announce.refusals());
        assert_eq!(announce.port(), Some(6881));
        assert_eq!(announce.cookie(), Some(&b"bit-ids-1"[..]));
        assert_eq!(announce.host(), Some(&b"239.192.152.143:6771"[..]));
        assert_eq!(announce.info_hashes().len(), 1);
        assert_eq!(announce.trailer(), Trailer::Blank(b"\r\n".to_vec()));
        assert!(announce.rebuilds_from_raw());
        assert_eq!(announce.raw(), CONFORMING);
    }

    #[test]
    fn the_field_order_and_case_a_build_used_survive_the_reading() {
        // ⭐ Lowercase `infohash`, a reordered head and a bare-newline
        // terminator: three things a normalising parser would erase and three
        // things that tell two builds apart.
        let odd = b"BT-SEARCH * HTTP/1.1\ninfohash: 0123456789abcdef0123456789abcdef01234567\nPort: 51413\nHost: 239.192.152.143:6771\n\n";
        let announce = read(odd);
        assert!(announce.is_conforming(), "{:?}", announce.refusals());
        assert_eq!(
            announce.field_order(),
            vec![b"infohash".to_vec(), b"Port".to_vec(), b"Host".to_vec()]
        );
        assert_eq!(announce.cookie(), None);
        assert_eq!(announce.trailer(), Trailer::None);
        assert!(announce.rebuilds_from_raw());
    }

    #[test]
    fn several_info_hashes_in_one_announce_are_all_kept() {
        let many = b"BT-SEARCH * HTTP/1.1\r\nHost: 239.192.152.143:6771\r\nPort: 6881\r\nInfohash: 0123456789abcdef0123456789abcdef01234567\r\nInfohash: 89abcdef0123456789abcdef0123456789abcdef\r\n\r\n";
        let announce = read(many);
        assert!(announce.is_conforming(), "{:?}", announce.refusals());
        assert_eq!(announce.info_hashes().len(), 2);
    }

    #[test]
    fn every_departure_from_the_specification_is_reported_rather_than_the_first() {
        // Wrong method, one bad hash, and a port of zero, in one datagram.
        let bad = b"BT-SEARCH2 * HTTP/1.1\r\nHost: x\r\nPort: 0\r\nInfohash: nothex\r\n\r\n";
        let announce = read(bad);
        let refusals = announce.refusals();
        assert_eq!(refusals.len(), 3, "{refusals:?}");
        assert!(matches!(refusals[0], Refusal::NotBtSearch(_)));
        assert!(matches!(refusals[1], Refusal::InfoHashNotHex(_)));
        assert!(matches!(refusals[2], Refusal::PortNotANumber(_)));
        // ⛔ Refused and kept. The bytes are the finding.
        assert_eq!(announce.raw(), bad);
        assert!(announce.rebuilds_from_raw());
    }

    #[test]
    fn a_datagram_that_is_not_a_message_at_all_is_kept_as_bytes() {
        let announce = read(b"\x00\x01\x02");
        assert!(announce.request().is_none());
        assert!(matches!(announce.refusals()[0], Refusal::NotAMessage(_)));
        assert_eq!(announce.raw(), b"\x00\x01\x02");
        assert!(!announce.rebuilds_from_raw());
    }

    #[test]
    fn an_announce_naming_no_torrent_is_refused() {
        let none = b"BT-SEARCH * HTTP/1.1\r\nHost: x\r\nPort: 6881\r\n\r\n";
        assert!(read(none).refusals().contains(&Refusal::NoInfoHash));
        let no_port = b"BT-SEARCH * HTTP/1.1\r\nHost: x\r\nInfohash: 0123456789abcdef0123456789abcdef01234567\r\n\r\n";
        assert!(read(no_port).refusals().contains(&Refusal::NoPort));
    }

    #[test]
    fn the_observer_is_refused_without_a_capability_for_its_own_surface() {
        assert!(LocalDiscovery::new(Capability::enable(Surface::LocalDiscovery)).is_ok());
        for other in ALL {
            if other == Surface::LocalDiscovery {
                continue;
            }
            let refusal =
                LocalDiscovery::new(Capability::enable(other)).expect_err("a different surface");
            assert_eq!(refusal.wanted, Surface::LocalDiscovery);
            assert_eq!(refusal.offered, other);
        }
    }

    #[test]
    fn answers_nothing_on_any_input() {
        let observer = LocalDiscovery::new(Capability::enable(Surface::LocalDiscovery))
            .expect("the capability names this surface");
        let responder = observer.observing();
        // ⛔ Conforming, malformed and empty alike. BEP 14 has no reply, so
        // there is no input for which one would be correct.
        for datagram in [CONFORMING, b"\x00\x01\x02", b""] {
            assert!(responder(datagram).is_none());
        }
        assert_eq!(observer.announces().len(), 3);
    }

    #[test]
    fn announces_past_the_cap_are_counted_rather_than_kept() {
        let observer = LocalDiscovery::new(Capability::enable(Surface::LocalDiscovery))
            .expect("the capability names this surface")
            .with_max_announces(2);
        let responder = observer.observing();
        for _ in 0..5 {
            assert!(responder(CONFORMING).is_none());
        }
        assert_eq!(observer.announces().len(), 2);
        assert_eq!(observer.dropped(), 3);
    }

    /// ⛔ The egress-negative case. The addresses are the ones BEP 14 itself
    /// names, so a guard that stopped firing would put a synthetic announce on
    /// a real LAN.
    #[test]
    fn the_group_this_protocol_names_is_refused_by_the_lab() {
        use bit_ids_lab::bind;
        use std::net::{IpAddr, Ipv4Addr};

        let socket = bind::datagram(IpAddr::V4(Ipv4Addr::LOCALHOST)).expect("loopback binds");
        for group in [GROUP_V4, GROUP_V6] {
            assert!(
                matches!(
                    bind::send_to(&socket, CONFORMING, group),
                    Err(bind::BindError::NotReachable { .. })
                ),
                "{group} was not refused"
            );
        }
        assert!(!GROUP_V4.ip().is_loopback());
        assert!(!GROUP_V6.ip().is_loopback());
    }
}
