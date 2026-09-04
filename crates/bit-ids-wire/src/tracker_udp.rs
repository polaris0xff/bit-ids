//! The UDP tracker exchange of BEP 15.
//!
//! Every field here is fixed-width and positional, which is the one place this
//! project's usual advice inverts: there is no name to select by, so the guard
//! against reading the wrong span is a length check on every frame and a
//! refusal when it does not hold.
//!
//! ⭐ **The trailing bytes are the interesting part.** An announce request is 98
//! bytes; a client that sends more is sending BEP 41 request-string options,
//! and one that sends fewer is malformed. Both are identity signals, so the
//! surplus is kept as `options` rather than ignored, and the shortfall is an
//! error rather than a zero-filled read.

use crate::error::{WireError, be_bytes};

/// The connection-id every client must open with, fixed by BEP 15.
pub const PROTOCOL_ID: u64 = 0x0000_0417_2710_1980;
/// The width of an announce request before any BEP 41 options.
pub const ANNOUNCE_REQUEST_LEN: usize = 98;

/// Which side of the exchange a datagram travelled.
///
/// Named for the target under measurement rather than for a client and a
/// server, because the observer is the tracker: `FromTarget` is the datagram
/// the build emitted, which is the only kind that is evidence about it.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Direction {
    /// The build under measurement sent it.
    FromTarget,
    /// The observer sent it.
    ToTarget,
}

/// The action code of a datagram.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    /// `0`, the connect exchange.
    Connect,
    /// `1`, the announce exchange.
    Announce,
    /// `2`, the scrape exchange.
    Scrape,
    /// `3`, an error response.
    Error,
    /// Anything else, kept as the number that arrived.
    Other(u32),
}

impl Action {
    /// Reads an action code.
    #[must_use]
    pub const fn from_code(code: u32) -> Self {
        match code {
            0 => Self::Connect,
            1 => Self::Announce,
            2 => Self::Scrape,
            3 => Self::Error,
            other => Self::Other(other),
        }
    }

    /// The code as it travels on the wire.
    #[must_use]
    pub const fn code(self) -> u32 {
        match self {
            Self::Connect => 0,
            Self::Announce => 1,
            Self::Scrape => 2,
            Self::Error => 3,
            Self::Other(code) => code,
        }
    }
}

/// A decoded announce request, the datagram that carries the peer ID.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnnounceRequest {
    /// The connection id the tracker handed out.
    pub connection_id: u64,
    /// The transaction id the client chose for this exchange.
    pub transaction_id: u32,
    /// The torrent being announced.
    pub info_hash: [u8; 20],
    /// The twenty peer-ID bytes, undecoded.
    pub peer_id: [u8; 20],
    /// Bytes downloaded so far, as the client reports them.
    pub downloaded: u64,
    /// Bytes left, as the client reports them.
    pub left: u64,
    /// Bytes uploaded so far, as the client reports them.
    pub uploaded: u64,
    /// The event code: 0 none, 1 completed, 2 started, 3 stopped.
    pub event: u32,
    /// The address the client asked the tracker to use, usually zero.
    pub ip: u32,
    /// The client's announce key, a per-client identity value in its own right.
    pub key: u32,
    /// How many peers the client wants. Signed: `-1` means "the default".
    pub num_want: i32,
    /// The port the client says it listens on.
    pub port: u16,
    /// Anything after byte 98, which BEP 41 defines as request-string options.
    pub options: Vec<u8>,
}

/// One datagram of the exchange, with its parsed view beside its bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Datagram {
    direction: Direction,
    raw: Vec<u8>,
    action: Action,
    transaction_id: u32,
}

impl Datagram {
    /// The narrowest legible datagram: an action and a transaction id.
    pub const MIN_LEN: usize = 8;

    /// Decodes a datagram sent to the observer's tracker.
    ///
    /// A request carries its connection id first, so the action and transaction
    /// id sit eight bytes further in than they do in a response. Passing the
    /// wrong direction is therefore a parse that succeeds and reads the wrong
    /// span, which is why the direction is a parameter and not a guess.
    ///
    /// # Errors
    ///
    /// Returns `truncated` when the datagram is shorter than the header its
    /// direction requires, and `announce-length` when an announce request is
    /// shorter than [`ANNOUNCE_REQUEST_LEN`].
    pub fn parse(direction: Direction, input: &[u8]) -> Result<Self, WireError> {
        let header_at = match direction {
            Direction::FromTarget => 8,
            Direction::ToTarget => 0,
        };
        let action = Action::from_code(u32::from_be_bytes(be_bytes::<4>(
            input, header_at, "action",
        )?));
        let transaction_id =
            u32::from_be_bytes(be_bytes::<4>(input, header_at + 4, "transaction id")?);
        if direction == Direction::FromTarget
            && action == Action::Announce
            && input.len() < ANNOUNCE_REQUEST_LEN
        {
            return Err(WireError::new(
                "announce-length",
                0,
                format!(
                    "announce request is {} bytes, BEP 15 fixes {ANNOUNCE_REQUEST_LEN}",
                    input.len()
                ),
            ));
        }
        Ok(Self {
            direction,
            raw: input.to_vec(),
            action,
            transaction_id,
        })
    }

    /// Which side sent it.
    #[must_use]
    pub const fn direction(&self) -> Direction {
        self.direction
    }

    /// The action code.
    #[must_use]
    pub const fn action(&self) -> Action {
        self.action
    }

    /// The transaction id tying a request to its response.
    #[must_use]
    pub const fn transaction_id(&self) -> u32 {
        self.transaction_id
    }

    /// The datagram exactly as it arrived.
    #[must_use]
    pub fn raw(&self) -> &[u8] {
        &self.raw
    }

    /// Whether this is a connect request opening with [`PROTOCOL_ID`].
    ///
    /// A client that opens with a different magic value is talking a protocol
    /// this tracker does not implement, and saying so is more useful than
    /// answering it.
    #[must_use]
    pub fn opens_with_protocol_id(&self) -> bool {
        self.direction == Direction::FromTarget
            && self.action == Action::Connect
            && be_bytes::<8>(&self.raw, 0, "protocol id")
                .is_ok_and(|bytes| u64::from_be_bytes(bytes) == PROTOCOL_ID)
    }

    /// Reads the datagram as an announce request.
    ///
    /// Returns `None` unless this is an announce sent by the target.
    ///
    /// # Errors
    ///
    /// Returns `truncated` if the frame is short, which [`Datagram::parse`]
    /// already refuses; the check is repeated rather than assumed because this
    /// is a separate read path over the same bytes.
    #[must_use]
    pub fn as_announce_request(&self) -> Option<Result<AnnounceRequest, WireError>> {
        if self.direction != Direction::FromTarget || self.action != Action::Announce {
            return None;
        }
        Some(self.decode_announce_request())
    }

    fn decode_announce_request(&self) -> Result<AnnounceRequest, WireError> {
        let raw = &self.raw;
        let u64_at = |at: usize, what: &'static str| -> Result<u64, WireError> {
            Ok(u64::from_be_bytes(be_bytes::<8>(raw, at, what)?))
        };
        let u32_at = |at: usize, what: &'static str| -> Result<u32, WireError> {
            Ok(u32::from_be_bytes(be_bytes::<4>(raw, at, what)?))
        };
        Ok(AnnounceRequest {
            connection_id: u64_at(0, "connection id")?,
            transaction_id: u32_at(12, "transaction id")?,
            info_hash: be_bytes::<20>(raw, 16, "info hash")?,
            peer_id: be_bytes::<20>(raw, 36, "peer id")?,
            downloaded: u64_at(56, "downloaded")?,
            left: u64_at(64, "left")?,
            uploaded: u64_at(72, "uploaded")?,
            event: u32_at(80, "event")?,
            ip: u32_at(84, "ip")?,
            key: u32_at(88, "key")?,
            // ⚠ Signed on the wire. Read as u32 and printed, `-1` becomes
            // 4294967295 and a record says a client asked for four billion
            // peers when it asked for the tracker's default.
            num_want: u32_at(92, "num want")?.cast_signed(),
            port: u16::from_be_bytes(be_bytes::<2>(raw, 96, "port")?),
            options: raw.get(ANNOUNCE_REQUEST_LEN..).unwrap_or_default().to_vec(),
        })
    }

    /// Writes the datagram back.
    ///
    /// ⛔ Byte for byte what [`Datagram::parse`] read.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        self.raw.clone()
    }
}

/// The ordered datagrams of one exchange, with the observer's monotonic clock.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Exchange {
    frames: Vec<(u64, Datagram)>,
}

impl Exchange {
    /// Builds an exchange from datagrams already in arrival order.
    ///
    /// # Errors
    ///
    /// Returns `clock-order` when an offset is not greater than or equal to the
    /// one before it. Retry cadence is one of the fields BEP 15 observation is
    /// for, and a transcript whose clock runs backwards cannot measure it.
    pub fn new(frames: Vec<(u64, Datagram)>) -> Result<Self, WireError> {
        for (index, pair) in frames.windows(2).enumerate() {
            if pair[1].0 < pair[0].0 {
                return Err(WireError::new(
                    "clock-order",
                    index + 1,
                    format!("offset {} follows {}", pair[1].0, pair[0].0),
                ));
            }
        }
        Ok(Self { frames })
    }

    /// The datagrams with their monotonic offsets, in arrival order.
    #[must_use]
    pub fn frames(&self) -> &[(u64, Datagram)] {
        &self.frames
    }

    /// The gaps between consecutive datagrams the target sent.
    ///
    /// This is the retry cadence `docs/architecture.md` section 5 asks for,
    /// computed rather than asserted.
    #[must_use]
    pub fn target_send_gaps_ms(&self) -> Vec<u64> {
        let sent: Vec<u64> = self
            .frames
            .iter()
            .filter(|(_, frame)| frame.direction() == Direction::FromTarget)
            .map(|(offset, _)| *offset)
            .collect();
        sent.windows(2).map(|pair| pair[1] - pair[0]).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{ANNOUNCE_REQUEST_LEN, Action, Datagram, Direction, Exchange, PROTOCOL_ID};

    fn announce_request(num_want: i32, options: &[u8]) -> Vec<u8> {
        let mut raw = Vec::new();
        raw.extend_from_slice(&0x1122_3344_5566_7788_u64.to_be_bytes());
        raw.extend_from_slice(&1_u32.to_be_bytes());
        raw.extend_from_slice(&0xdead_beef_u32.to_be_bytes());
        raw.extend_from_slice(&[0x33; 20]);
        raw.extend_from_slice(b"bit-ids-fixture-0001");
        raw.extend_from_slice(&0_u64.to_be_bytes());
        raw.extend_from_slice(&1024_u64.to_be_bytes());
        raw.extend_from_slice(&0_u64.to_be_bytes());
        raw.extend_from_slice(&2_u32.to_be_bytes());
        raw.extend_from_slice(&0_u32.to_be_bytes());
        raw.extend_from_slice(&0x0a0b_0c0d_u32.to_be_bytes());
        raw.extend_from_slice(&num_want.to_be_bytes());
        raw.extend_from_slice(&6881_u16.to_be_bytes());
        raw.extend_from_slice(options);
        raw
    }

    #[test]
    fn a_default_num_want_reads_as_minus_one_rather_than_four_billion() {
        let raw = announce_request(-1, &[]);
        let datagram = Datagram::parse(Direction::FromTarget, &raw).expect("decodes");
        let announce = datagram
            .as_announce_request()
            .expect("it is an announce")
            .expect("it decodes");
        assert_eq!(announce.num_want, -1);
        assert_eq!(announce.peer_id, *b"bit-ids-fixture-0001");
        assert_eq!(announce.key, 0x0a0b_0c0d);
        assert_eq!(announce.port, 6881);
        assert_eq!(datagram.encode(), raw);
    }

    #[test]
    fn bytes_past_the_fixed_width_are_kept_as_bep_41_options() {
        let raw = announce_request(50, b"\x02\x09/announce");
        let datagram = Datagram::parse(Direction::FromTarget, &raw).expect("decodes");
        let announce = datagram
            .as_announce_request()
            .expect("it is an announce")
            .expect("it decodes");
        assert_eq!(announce.options, b"\x02\x09/announce");
        assert_eq!(raw.len(), ANNOUNCE_REQUEST_LEN + 11);
        assert_eq!(datagram.encode(), raw);
    }

    #[test]
    fn a_short_announce_is_refused_rather_than_read_as_zeroes() {
        let mut raw = announce_request(50, &[]);
        raw.truncate(ANNOUNCE_REQUEST_LEN - 1);
        assert_eq!(
            Datagram::parse(Direction::FromTarget, &raw)
                .expect_err("BEP 15 fixes the width")
                .kind(),
            "announce-length"
        );
    }

    #[test]
    fn a_connect_request_is_checked_against_the_protocol_magic() {
        let mut raw = PROTOCOL_ID.to_be_bytes().to_vec();
        raw.extend_from_slice(&0_u32.to_be_bytes());
        raw.extend_from_slice(&7_u32.to_be_bytes());
        let datagram = Datagram::parse(Direction::FromTarget, &raw).expect("decodes");
        assert_eq!(datagram.action(), Action::Connect);
        assert_eq!(datagram.transaction_id(), 7);
        assert!(datagram.opens_with_protocol_id());

        raw[0] = 0xff;
        let wrong = Datagram::parse(Direction::FromTarget, &raw).expect("still a datagram");
        assert!(!wrong.opens_with_protocol_id());
    }

    #[test]
    fn a_response_header_is_read_eight_bytes_earlier_than_a_request_header() {
        let mut raw = 0_u32.to_be_bytes().to_vec();
        raw.extend_from_slice(&7_u32.to_be_bytes());
        raw.extend_from_slice(&0x4242_4242_4242_4242_u64.to_be_bytes());
        let response = Datagram::parse(Direction::ToTarget, &raw).expect("decodes");
        assert_eq!(response.action(), Action::Connect);
        assert_eq!(response.transaction_id(), 7);
        assert_eq!(response.encode(), raw);
    }

    #[test]
    fn an_unassigned_action_keeps_its_code() {
        let mut raw = 0_u64.to_be_bytes().to_vec();
        raw.extend_from_slice(&99_u32.to_be_bytes());
        raw.extend_from_slice(&1_u32.to_be_bytes());
        let datagram = Datagram::parse(Direction::FromTarget, &raw).expect("decodes");
        assert_eq!(datagram.action(), Action::Other(99));
        assert_eq!(datagram.action().code(), 99);
        assert!(datagram.as_announce_request().is_none());
    }

    #[test]
    fn a_transcript_whose_clock_runs_backwards_is_refused() {
        let raw = announce_request(-1, &[]);
        let frame = || Datagram::parse(Direction::FromTarget, &raw).expect("decodes");
        assert_eq!(
            Exchange::new(vec![(10, frame()), (4, frame())])
                .expect_err("time does not go backwards")
                .kind(),
            "clock-order"
        );
        let exchange =
            Exchange::new(vec![(0, frame()), (15, frame()), (45, frame())]).expect("in order");
        assert_eq!(exchange.target_send_gaps_ms(), vec![15, 30]);
    }
}
