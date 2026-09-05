//! The ordered record of every byte an endpoint moved.
//!
//! ⛔ **The order is the vector's order, and the vector is appended to under one
//! lock.** Endpoints run on their own threads, so two segments can share a
//! millisecond and `offset_ms` cannot order them. Reading the offsets as the
//! order would put a reply before the request that caused it, which is the one
//! thing a transcript exists to say.
//!
//! ⚠ **A stream segment is what the observer read, not provably what the target
//! wrote.** `docs/architecture.md` section 5 keeps write segmentation because a
//! handshake and a bitfield in one write is a different observation from the
//! same bytes in two, and TCP preserves no write boundaries to recover. One
//! segment per read call is the closest an observer can get. The read buffer is
//! larger than any message these surfaces carry, so no message is split by the
//! buffer size, but a burst larger than the buffer still spans two segments and
//! nothing in the bytes distinguishes that from two writes. A datagram has no
//! such gap: one segment is one packet, and the buffer is above the largest
//! datagram a host can deliver so none is truncated.

use std::time::Duration;

use bit_ids::canonical::Slug;
use bit_ids_wire::tracker_udp::Direction;

/// One contiguous run of bytes, in the direction it travelled.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Segment {
    endpoint: Slug,
    offset_ms: u64,
    direction: Direction,
    bytes: Vec<u8>,
}

impl Segment {
    /// Builds a segment. The lab is the only producer; the fields are read-only
    /// afterwards so a consumer cannot edit a transcript it was handed.
    pub(crate) fn new(
        endpoint: Slug,
        elapsed: Duration,
        direction: Direction,
        bytes: Vec<u8>,
    ) -> Self {
        Self {
            endpoint,
            // A run that outlives `u64::MAX` milliseconds is not a run. The
            // saturating form is here so the conversion cannot panic in a
            // worker thread, where a panic would poison the journal lock.
            offset_ms: u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
            direction,
            bytes,
        }
    }

    /// Which endpoint moved these bytes.
    #[must_use]
    pub const fn endpoint(&self) -> &Slug {
        &self.endpoint
    }

    /// Milliseconds from the lab starting to this segment being recorded.
    ///
    /// ⚠ Informational. [`Journal::segments`] is the order.
    #[must_use]
    pub const fn offset_ms(&self) -> u64 {
        self.offset_ms
    }

    /// Whether the target sent these bytes or the lab did.
    #[must_use]
    pub const fn direction(&self) -> Direction {
        self.direction
    }

    /// The bytes, exactly as they were read or written.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Everything a lab observed, in the order it observed it.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Journal {
    segments: Vec<Segment>,
}

impl Journal {
    pub(crate) const fn from_segments(segments: Vec<Segment>) -> Self {
        Self { segments }
    }

    /// Every segment, in the order it was recorded.
    #[must_use]
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    /// The segments one endpoint moved, in order.
    #[must_use]
    pub fn for_endpoint(&self, endpoint: &Slug) -> Vec<&Segment> {
        self.segments
            .iter()
            .filter(|segment| segment.endpoint() == endpoint)
            .collect()
    }

    /// The bytes the target sent to one endpoint, joined.
    ///
    /// The lab's own replies are excluded, because
    /// `crates/bit-ids-wire/src/fixture.rs` states the rule this follows: the
    /// observer's replies prove nothing about a build. The replies stay in
    /// [`Journal::segments`], where a replay needs them.
    #[must_use]
    pub fn received(&self, endpoint: &Slug) -> Vec<u8> {
        let mut out = Vec::new();
        for segment in self.for_endpoint(endpoint) {
            if segment.direction() == Direction::FromTarget {
                out.extend_from_slice(segment.bytes());
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::{Direction, Journal, Segment};
    use bit_ids::canonical::Slug;
    use std::time::Duration;

    fn slug(text: &str) -> Slug {
        Slug::parse(text).expect("a test name is a slug")
    }

    fn segment(endpoint: &str, ms: u64, direction: Direction, bytes: &[u8]) -> Segment {
        Segment::new(
            slug(endpoint),
            Duration::from_millis(ms),
            direction,
            bytes.to_vec(),
        )
    }

    #[test]
    fn segments_keep_the_order_they_were_appended_in_not_the_offset_order() {
        let journal = Journal::from_segments(vec![
            segment("http", 7, Direction::FromTarget, b"GET / "),
            segment("udp", 7, Direction::FromTarget, b"\x00\x00"),
            segment("http", 7, Direction::ToTarget, b"HTTP/1.1"),
        ]);
        let order: Vec<&[u8]> = journal
            .segments()
            .iter()
            .map(super::Segment::bytes)
            .collect();
        assert_eq!(
            order,
            vec![&b"GET / "[..], &b"\x00\x00"[..], &b"HTTP/1.1"[..]]
        );
    }

    #[test]
    fn received_joins_only_what_the_target_sent() {
        let journal = Journal::from_segments(vec![
            segment("http", 0, Direction::FromTarget, b"GET "),
            segment("http", 1, Direction::ToTarget, b"HTTP/1.1 200"),
            segment("http", 2, Direction::FromTarget, b"/announce"),
            segment("udp", 3, Direction::FromTarget, b"other"),
        ]);
        assert_eq!(journal.received(&slug("http")), b"GET /announce".to_vec());
        assert_eq!(journal.received(&slug("udp")), b"other".to_vec());
        assert_eq!(journal.for_endpoint(&slug("http")).len(), 3);
    }

    #[test]
    fn an_elapsed_time_past_the_offset_width_saturates_rather_than_panicking() {
        let long = Duration::from_millis(u64::MAX) + Duration::from_millis(1);
        let segment = Segment::new(slug("http"), long, Direction::FromTarget, Vec::new());
        assert_eq!(segment.offset_ms(), u64::MAX);
    }
}
