//! The message stream encryption observer: what a build offers before any other
//! observer can read a byte.
//!
//! ⛔ **MSE comes first or not at all**, which makes it unlike every other
//! surface here. A build that encrypts sends its `BitTorrent` handshake inside the
//! third MSE message, so its peer ID is not on the wire in the clear and
//! `OBS-04` sees nothing it recognises. Whether the lab offers MSE therefore
//! changes what the peer-wire observers measure, and the offer is a **condition
//! of the measurement** recorded beside the result, the way `OBS-05` records a
//! BEP 10 offer.
//!
//! # What is identity here
//!
//! - `crypto_provide`: whether a build offers plaintext, `RC4`, or both. A build
//!   that offers only `RC4` and one that offers both behave identically until
//!   something declines, and only this field tells them apart;
//! - the **padding lengths** a build picks at each step, which are its own
//!   choice out of a range the specification leaves open;
//! - whether it initiates MSE **at all** before falling back to plaintext, and
//!   how long it waits before doing so;
//! - the `BitTorrent` handshake inside `IA`, which is `OBS-04`'s measurement
//!   arriving through a different door. ⭐ **That overlap is the point**: the
//!   same peer ID observed encrypted and in the clear is two observations of one
//!   value, which is what `SCHEMA-03` calls corroboration.
//!
//! # What it answers
//!
//! ⚠ The lab is the receiving side, so it answers with its own public key and
//! then with a `crypto_select`. **What it selects is a condition of the
//! measurement**: selecting plaintext and selecting `RC4` produce different
//! subsequent streams, and a build's reaction to each is a different experiment.
//! [`Selection`] is the value, and it is recorded.
//!
//! ⛔ **Nothing here is a security primitive.** [`bit_ids_wire::mse`] says why in
//! full: MSE is obfuscation, the secret is unauthenticated, and the only party on
//! the far side is a binary this project installed minutes earlier on a loopback
//! socket.

use std::sync::{Arc, Mutex, PoisonError};

use bit_ids_lab::adjacent::{Capability, NotEnabled, Surface, require};
use bit_ids_lab::endpoint::{ConnectionId, StreamReply};
use bit_ids_wire::mse::{
    self, CRYPTO_PLAINTEXT, CRYPTO_RC4, KEY_LEN, MAX_PAD, PRIVATE_LEN, Provide, Rc4,
};

/// How many exchanges one observer keeps before it stops keeping them.
pub const DEFAULT_MAX_EXCHANGES: usize = 256;

/// The largest opening this observer will buffer before giving up on it.
///
/// ⛔ A bound for the reason every other observer carries one. The first MSE
/// message is a 96-byte key and up to [`MAX_PAD`] bytes of padding, and the third
/// is bounded by its own declared lengths; a peer that streams past this without
/// completing the exchange is a memory leak with a socket attached.
pub const MAX_OPENING: usize = KEY_LEN + MAX_PAD + 20 + 20 + 8 + 4 + 2 + MAX_PAD + 2 + 4096;

/// What the observer selects in its fourth message.
///
/// ⭐ **A value the caller writes, not a default.** Selecting plaintext and
/// selecting `RC4` are different experiments, and a flag that defaulted to one
/// of them would make the choice invisible in the record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Selection {
    /// Continue in plaintext after the handshake.
    Plaintext,
    /// Continue under `RC4` after the handshake.
    Rc4,
}

impl Selection {
    /// The `crypto_select` value MSE puts on the wire.
    #[must_use]
    pub const fn code(self) -> u32 {
        match self {
            Self::Plaintext => CRYPTO_PLAINTEXT,
            Self::Rc4 => CRYPTO_RC4,
        }
    }

    /// Whether the initiator offered this method.
    #[must_use]
    pub const fn offered_by(self, provide: &Provide) -> bool {
        match self {
            Self::Plaintext => provide.offers_plaintext(),
            Self::Rc4 => provide.offers_rc4(),
        }
    }
}

/// Why an exchange is not what MSE describes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Refusal {
    /// The third message never appeared: no `HASH('req1', S)` in the stream.
    ///
    /// ⚠ Bounded by [`MAX_OPENING`] rather than waited on forever.
    NoRequestMarker,
    /// The encrypted section did not read.
    Unreadable(String),
    /// The verification constant did not decrypt to eight zero bytes.
    ///
    /// ⭐ Which means the build derived a different secret than the observer
    /// did, and that is a finding about the build rather than a reason to drop
    /// the exchange.
    VerificationFailed([u8; 8]),
    /// The build offered no method this observer recognises.
    NothingOffered(u32),
    /// The observer selected a method the build did not offer.
    ///
    /// ⚠ A finding about the **run** rather than about the build, kept in the
    /// same list because a reader of the record needs to know the observer did
    /// something the build had not agreed to.
    SelectionNotOffered {
        /// What the observer selected.
        selected: u32,
        /// What the build offered.
        offered: u32,
    },
}

impl Refusal {
    /// The refusal in one line.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::NoRequestMarker => "no req1 marker in the opening".to_owned(),
            Self::Unreadable(why) => format!("the encrypted section did not read: {why}"),
            Self::VerificationFailed(vc) => {
                format!("the verification constant decrypted to {vc:02x?}")
            }
            Self::NothingOffered(bits) => format!("crypto_provide is {bits:#010x}"),
            Self::SelectionNotOffered { selected, offered } => format!(
                "the observer selected {selected:#010x} and the build offered {offered:#010x}"
            ),
        }
    }
}

/// One MSE exchange, as it happened.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Exchange {
    their_key: Vec<u8>,
    pad_a: usize,
    provide: Option<Provide>,
    selected: Selection,
    refusals: Vec<Refusal>,
}

impl Exchange {
    /// The public key the build sent, as bytes.
    #[must_use]
    pub fn their_key(&self) -> &[u8] {
        &self.their_key
    }

    /// How many bytes of padding the build put after its key.
    ///
    /// ⭐ A build's own choice out of `0..=512`, and one of the few numbers on
    /// this surface that is not fixed by anything.
    #[must_use]
    pub const fn pad_a_len(&self) -> usize {
        self.pad_a
    }

    /// What the build offered, once decrypted.
    #[must_use]
    pub const fn provide(&self) -> Option<&Provide> {
        self.provide.as_ref()
    }

    /// What the observer selected, which is a condition of the measurement.
    #[must_use]
    pub const fn selected(&self) -> Selection {
        self.selected
    }

    /// Everything about this exchange MSE does not describe.
    #[must_use]
    pub fn refusals(&self) -> &[Refusal] {
        &self.refusals
    }

    /// Whether this exchange is one MSE describes.
    #[must_use]
    pub fn is_conforming(&self) -> bool {
        self.refusals.is_empty()
    }

    /// The `BitTorrent` handshake the build sent inside `IA`, if it sent one.
    ///
    /// ⭐ **`OBS-04`'s measurement, arriving through a different door.** A peer
    /// ID read here and the same peer ID read in the clear are two observations
    /// of one value.
    #[must_use]
    pub fn initial_payload(&self) -> Option<&[u8]> {
        Some(self.provide.as_ref()?.initial_payload.as_slice())
    }
}

#[derive(Debug, Default)]
struct Record {
    kept: Vec<Exchange>,
    dropped: usize,
}

/// The MSE observer.
///
/// ⛔ Built only from a [`Capability`] for [`Surface::Mse`].
#[derive(Debug)]
pub struct Mse {
    seen: Arc<Mutex<Record>>,
    private: [u8; PRIVATE_LEN],
    info_hash: [u8; 20],
    selection: Selection,
    pad_b: Vec<u8>,
    max_exchanges: usize,
}

impl Mse {
    /// An observer for one torrent, if MSE was turned on.
    ///
    /// ⚠ `info_hash` is needed because MSE's key derivation includes it: the
    /// initiator names the torrent as `HASH('req2', SKEY) xor HASH('req3', S)`,
    /// and a receiver that did not already know the torrent could not derive the
    /// stream keys at all.
    ///
    /// # Errors
    ///
    /// Returns [`NotEnabled`] when `capability` enables a different surface.
    pub fn new(
        capability: Capability,
        info_hash: [u8; 20],
        selection: Selection,
    ) -> Result<Self, NotEnabled> {
        require(capability, Surface::Mse)?;
        Ok(Self {
            seen: Arc::new(Mutex::new(Record::default())),
            // ⚠ Fixed rather than drawn from a random source, for the reason
            // `bit_ids_wire::mse` gives: two runs of one capture have to produce
            // comparable transcripts, and the value protects nothing.
            private: *b"bit-ids-observer-key",
            info_hash,
            selection,
            pad_b: Vec::new(),
            max_exchanges: DEFAULT_MAX_EXCHANGES,
        })
    }

    /// The padding the observer puts after its own key.
    #[must_use]
    pub fn with_pad_b(mut self, pad: Vec<u8>) -> Self {
        self.pad_b = pad;
        self
    }

    /// How many exchanges this observer keeps.
    #[must_use]
    pub const fn with_max_exchanges(mut self, max_exchanges: usize) -> Self {
        self.max_exchanges = max_exchanges;
        self
    }

    /// The public key this observer answers with.
    #[must_use]
    pub fn public_key(&self) -> [u8; KEY_LEN] {
        mse::public_key(&self.private)
    }

    /// Every exchange kept, in the order it happened.
    #[must_use]
    pub fn exchanges(&self) -> Vec<Exchange> {
        self.locked().kept.clone()
    }

    /// How many exchanges arrived after the cap and were recorded nowhere.
    #[must_use]
    pub fn dropped(&self) -> usize {
        self.locked().dropped
    }

    /// ⚠ A poisoned lock is recovered rather than propagated.
    fn locked(&self) -> std::sync::MutexGuard<'_, Record> {
        self.seen.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Reads one complete opening and produces the observer's two replies.
    ///
    /// Split out from the responder so the whole exchange can be driven without
    /// a socket, which is what the unit cases do.
    ///
    /// # Errors
    ///
    /// Returns [`None`] when `opening` does not yet hold a complete exchange.
    #[must_use]
    pub fn read_opening(&self, opening: &[u8]) -> Option<(Exchange, Vec<u8>)> {
        if opening.len() < KEY_LEN {
            return None;
        }
        let mut their_key = [0_u8; KEY_LEN];
        their_key.copy_from_slice(&opening[..KEY_LEN]);
        let secret = mse::shared_secret(&their_key, &self.private);

        // ⚠ The scan MSE requires: the initiator's padding is unpredictable, so
        // the marker is found rather than computed to an offset.
        let marker = mse::req1(&secret);
        let at = mse::find(&opening[KEY_LEN..], &marker).map(|found| KEY_LEN + found);
        let Some(at) = at else {
            if opening.len() > MAX_OPENING {
                return Some((
                    Exchange {
                        their_key: their_key.to_vec(),
                        pad_a: opening.len() - KEY_LEN,
                        provide: None,
                        selected: self.selection,
                        refusals: vec![Refusal::NoRequestMarker],
                    },
                    Vec::new(),
                ));
            }
            return None;
        };

        // req1 is 20 bytes, then the 20-byte torrent name, then the encrypted
        // section.
        let body_at = at + 20 + 20;
        if opening.len() <= body_at {
            return None;
        }
        let mut refusals = Vec::new();
        let mut reading = Rc4::new(&mse::key_a(&secret, &self.info_hash));
        let provide = match mse::read_provide(&mut reading, &opening[body_at..]) {
            Ok(provide) => provide,
            Err(error) if error.kind() == "mse-truncated" => return None,
            Err(error) => {
                refusals.push(Refusal::Unreadable(error.to_string()));
                return Some((
                    Exchange {
                        their_key: their_key.to_vec(),
                        pad_a: at - KEY_LEN,
                        provide: None,
                        selected: self.selection,
                        refusals,
                    },
                    Vec::new(),
                ));
            }
        };
        if !provide.verified() {
            refusals.push(Refusal::VerificationFailed(provide.vc));
        }
        if provide.crypto_provide & (CRYPTO_PLAINTEXT | CRYPTO_RC4) == 0 {
            refusals.push(Refusal::NothingOffered(provide.crypto_provide));
        } else if !self.selection.offered_by(&provide) {
            // ⚠ Recorded rather than corrected. Silently selecting whatever the
            // build offered would hide that the run asked for something else.
            refusals.push(Refusal::SelectionNotOffered {
                selected: self.selection.code(),
                offered: provide.crypto_provide,
            });
        }

        let mut reply = self.public_key().to_vec();
        reply.extend_from_slice(&self.pad_b);
        let mut writing = Rc4::new(&mse::key_b(&secret, &self.info_hash));
        reply.extend_from_slice(&mse::write_select(
            &mut writing,
            self.selection.code(),
            &self.pad_b,
        ));

        Some((
            Exchange {
                their_key: their_key.to_vec(),
                pad_a: at - KEY_LEN,
                provide: Some(provide),
                selected: self.selection,
                refusals,
            },
            reply,
        ))
    }

    /// The responder to give a stream endpoint.
    pub fn responder(&self) -> impl Fn(ConnectionId, &[u8]) -> StreamReply + Send + Sync + 'static {
        let seen = Arc::clone(&self.seen);
        let private = self.private;
        let info_hash = self.info_hash;
        let selection = self.selection;
        let pad_b = self.pad_b.clone();
        let cap = self.max_exchanges;
        move |_connection, buffered: &[u8]| {
            let observer = Self {
                seen: Arc::clone(&seen),
                private,
                info_hash,
                selection,
                pad_b: pad_b.clone(),
                max_exchanges: cap,
            };
            match observer.read_opening(buffered) {
                None if buffered.len() > MAX_OPENING => StreamReply::Close { send: Vec::new() },
                None => StreamReply::NeedMore,
                Some((exchange, reply)) => {
                    keep(&seen, cap, exchange);
                    // ⛔ The whole buffer is consumed. What follows the handshake
                    // is the encrypted payload stream, which this observer does
                    // not attempt to read: `OBS-04` reads a peer wire and this
                    // module reads a negotiation, and one of them decoding the
                    // other's bytes is how two readings of one stream disagree.
                    StreamReply::Answer {
                        consumed: buffered.len(),
                        send: reply,
                    }
                }
            }
        }
    }
}

/// Records one exchange, counting it instead once the cap is reached.
fn keep(seen: &Arc<Mutex<Record>>, cap: usize, exchange: Exchange) {
    let mut record = seen.lock().unwrap_or_else(PoisonError::into_inner);
    if record.kept.len() >= cap {
        record.dropped += 1;
        return;
    }
    record.kept.push(exchange);
}

/// Builds the initiator's side of an exchange, for driving the observer.
///
/// ⚠ **Here rather than in a test, because the driven pass needs it too**, and a
/// second copy in a test file would be two readings of one specification. It is
/// the only initiator this project has: a stock client is `OBS-07`'s, and needs a
/// capture host.
///
/// # Panics
///
/// Panics when `pad_c` or `initial_payload` is longer than 65535 bytes, which
/// MSE's own two-byte length fields cannot express. Both are values a caller
/// here chooses rather than input from a build.
#[must_use]
pub fn initiate(
    private: &[u8; PRIVATE_LEN],
    their_key: &[u8; KEY_LEN],
    info_hash: &[u8; 20],
    crypto_provide: u32,
    pad_a: &[u8],
    pad_c: &[u8],
    initial_payload: &[u8],
) -> Vec<u8> {
    let secret = mse::shared_secret(their_key, private);
    let mut out = mse::public_key(private).to_vec();
    out.extend_from_slice(pad_a);
    out.extend_from_slice(&mse::req1(&secret));
    out.extend_from_slice(&mse::req2_xor_req3(info_hash, &secret));

    let mut body = Vec::new();
    body.extend_from_slice(&mse::VC);
    body.extend_from_slice(&crypto_provide.to_be_bytes());
    body.extend_from_slice(
        &u16::try_from(pad_c.len())
            .expect("padding above 65535 is not something a caller offers")
            .to_be_bytes(),
    );
    body.extend_from_slice(pad_c);
    body.extend_from_slice(
        &u16::try_from(initial_payload.len())
            .expect("a payload above 65535 is not something a caller offers")
            .to_be_bytes(),
    );
    body.extend_from_slice(initial_payload);
    Rc4::new(&mse::key_a(&secret, info_hash)).apply(&mut body);
    out.extend_from_slice(&body);
    out
}

#[cfg(test)]
mod tests {
    use super::{
        CRYPTO_PLAINTEXT, CRYPTO_RC4, KEY_LEN, Mse, PRIVATE_LEN, Refusal, Selection, initiate,
    };
    use bit_ids_lab::adjacent::{ALL_SURFACES as ALL, Capability, Surface};
    use bit_ids_wire::mse;

    const INFO_HASH: [u8; 20] = [0x11; 20];
    /// A `BitTorrent` handshake, which is what a build puts in `IA`.
    const IA: &[u8] = b"\x13BitTorrent protocol\x00\x00\x00\x00\x00\x10\x00\x05\x11\x11\x11\x11\x11\x11\x11\x11\x11\x11\x11\x11\x11\x11\x11\x11\x11\x11\x11\x11a-build-under-measur";

    fn observer(selection: Selection) -> Mse {
        Mse::new(Capability::enable(Surface::Mse), INFO_HASH, selection)
            .expect("the capability names this surface")
    }

    fn their_private() -> [u8; PRIVATE_LEN] {
        *b"a-build-private-key0"
    }

    #[test]
    fn the_observer_is_refused_without_a_capability_for_its_own_surface() {
        assert!(Mse::new(Capability::enable(Surface::Mse), INFO_HASH, Selection::Rc4).is_ok());
        for other in ALL {
            if other == Surface::Mse {
                continue;
            }
            let refusal = Mse::new(Capability::enable(other), INFO_HASH, Selection::Rc4)
                .expect_err("a different surface");
            assert_eq!(refusal.wanted, Surface::Mse);
            assert_eq!(refusal.offered, other);
        }
    }

    /// ⭐ The whole exchange: a build initiates, the observer completes it and
    /// reads what was offered, and the `BitTorrent` handshake comes out of `IA`.
    #[test]
    fn a_complete_exchange_yields_the_offer_and_the_handshake_inside_it() {
        let observer = observer(Selection::Rc4);
        let opening = initiate(
            &their_private(),
            &observer.public_key(),
            &INFO_HASH,
            CRYPTO_PLAINTEXT | CRYPTO_RC4,
            &[0xAA; 37],
            &[0xCC; 11],
            IA,
        );
        let (exchange, reply) = observer.read_opening(&opening).expect("it completes");
        assert!(exchange.is_conforming(), "{:?}", exchange.refusals());
        assert_eq!(exchange.pad_a_len(), 37, "the padding length is a choice");
        let provide = exchange.provide().expect("it decrypted");
        assert!(provide.verified());
        assert!(provide.offers_plaintext());
        assert!(provide.offers_rc4());
        assert_eq!(provide.pad, vec![0xCC; 11]);

        // ⛔ `OBS-04`'s measurement, through a different door.
        assert_eq!(exchange.initial_payload(), Some(IA));
        assert_eq!(&IA[48..68], b"a-build-under-measur");

        // The reply is the observer's key, its padding, and the selection.
        assert_eq!(&reply[..KEY_LEN], &observer.public_key()[..]);
        assert!(reply.len() > KEY_LEN);
        assert_eq!(exchange.selected(), Selection::Rc4);
    }

    /// ⚠ The selection is a condition of the measurement, so it is recorded and
    /// it is what the caller asked for rather than whatever the build offered.
    #[test]
    fn the_observer_selects_what_it_was_built_to_select_and_says_so() {
        for selection in [Selection::Plaintext, Selection::Rc4] {
            let observer = observer(selection);
            let opening = initiate(
                &their_private(),
                &observer.public_key(),
                &INFO_HASH,
                CRYPTO_PLAINTEXT | CRYPTO_RC4,
                &[],
                &[],
                IA,
            );
            let (exchange, reply) = observer.read_opening(&opening).expect("it completes");
            assert_eq!(exchange.selected(), selection);
            assert!(exchange.is_conforming(), "{:?}", exchange.refusals());

            // ⭐ Read the selection back out of the reply with the key the build
            // would use, rather than trusting the value the observer holds.
            let secret = mse::shared_secret(&observer.public_key(), &their_private());
            let mut plain = reply[KEY_LEN..].to_vec();
            mse::Rc4::new(&mse::key_b(&secret, &INFO_HASH)).apply(&mut plain);
            assert_eq!(&plain[..8], &mse::VC);
            assert_eq!(&plain[8..12], &selection.code().to_be_bytes());
        }
    }

    /// ⛔ A build offering only plaintext against an observer selecting `RC4` is
    /// a finding about the run, and it is recorded rather than corrected.
    #[test]
    fn selecting_a_method_the_build_did_not_offer_is_recorded() {
        let observer = observer(Selection::Rc4);
        let opening = initiate(
            &their_private(),
            &observer.public_key(),
            &INFO_HASH,
            CRYPTO_PLAINTEXT,
            &[],
            &[],
            IA,
        );
        let (exchange, _) = observer.read_opening(&opening).expect("it completes");
        assert!(
            exchange.refusals().contains(&Refusal::SelectionNotOffered {
                selected: CRYPTO_RC4,
                offered: CRYPTO_PLAINTEXT,
            }),
            "{:?}",
            exchange.refusals()
        );
        // ⚠ And it still completed, with the offer recorded. A refusal is a
        // finding, not a reason to drop the evidence.
        assert!(exchange.provide().expect("decrypted").offers_plaintext());
    }

    #[test]
    fn a_build_offering_nothing_recognised_is_reported_with_the_bits_it_sent() {
        let observer = observer(Selection::Rc4);
        let opening = initiate(
            &their_private(),
            &observer.public_key(),
            &INFO_HASH,
            0x0000_0080,
            &[],
            &[],
            IA,
        );
        let (exchange, _) = observer.read_opening(&opening).expect("it completes");
        assert!(
            exchange
                .refusals()
                .contains(&Refusal::NothingOffered(0x0000_0080))
        );
    }

    /// ⛔ **A build whose verification constant is wrong, and nothing else.**
    ///
    /// ⚠ **This case exists because a mutation pass found the variant
    /// unreachable.** Deleting the `verified()` check entirely left every test
    /// passing: the only case that exercised a wrong key relies on random
    /// plaintext, and random plaintext trips the pad-length check first and
    /// reports [`Refusal::Unreadable`] instead. So
    /// [`Refusal::VerificationFailed`] was a refusal nothing could produce,
    /// which is the same shape `OBS-06` found in `peer_exchange`'s
    /// `BeforeHandshake`.
    ///
    /// The case that reaches it is a build that keys its stream correctly and
    /// writes the wrong constant, so every length is structurally valid and the
    /// constant is the only thing wrong. Assembled by hand rather than through
    /// [`initiate`], which writes the correct constant by construction.
    #[test]
    fn a_build_that_writes_the_wrong_verification_constant_is_reported_for_that_alone() {
        let observer = observer(Selection::Rc4);
        let private = their_private();
        let secret = mse::shared_secret(&observer.public_key(), &private);

        let mut opening = mse::public_key(&private).to_vec();
        opening.extend_from_slice(&mse::req1(&secret));
        opening.extend_from_slice(&mse::req2_xor_req3(&INFO_HASH, &secret));
        let mut body = Vec::new();
        // ⛔ The one thing wrong: eight bytes that are not zero.
        body.extend_from_slice(&[0xFF; 8]);
        body.extend_from_slice(&CRYPTO_RC4.to_be_bytes());
        body.extend_from_slice(&0_u16.to_be_bytes());
        body.extend_from_slice(
            &u16::try_from(IA.len())
                .expect("the handshake fits")
                .to_be_bytes(),
        );
        body.extend_from_slice(IA);
        mse::Rc4::new(&mse::key_a(&secret, &INFO_HASH)).apply(&mut body);
        opening.extend_from_slice(&body);

        let (exchange, reply) = observer.read_opening(&opening).expect("it completes");
        assert_eq!(
            exchange.refusals(),
            &[Refusal::VerificationFailed([0xFF; 8])],
            "the constant should be the only finding"
        );
        // ⛔ And the exchange is still read and answered: a refusal is a finding
        // about the build, not a reason to drop the evidence.
        let provide = exchange.provide().expect("everything else decrypted");
        assert!(provide.offers_rc4());
        assert!(!provide.verified());
        assert_eq!(exchange.initial_payload(), Some(IA));
        assert!(!reply.is_empty(), "the observer still answered");
    }

    /// ⚠ An opening that is not complete asks for more rather than guessing.
    #[test]
    fn a_partial_opening_is_not_read_as_an_exchange() {
        let observer = observer(Selection::Rc4);
        let opening = initiate(
            &their_private(),
            &observer.public_key(),
            &INFO_HASH,
            CRYPTO_RC4,
            &[0xAA; 20],
            &[],
            IA,
        );
        assert!(observer.read_opening(&opening[..KEY_LEN - 1]).is_none());
        assert!(observer.read_opening(&opening[..KEY_LEN + 10]).is_none());
        // ⛔ And one byte short of the whole thing is still not an exchange.
        assert!(
            observer
                .read_opening(&opening[..opening.len() - 1])
                .is_none()
        );
        assert!(observer.read_opening(&opening).is_some());
    }

    /// ⛔ A build that derived a different secret is a finding, not an error.
    ///
    /// ⚠ **Which finding is not fixed, and asserting one of them was wrong.**
    /// The first draft required [`Refusal::VerificationFailed`]; the observer
    /// answered [`Refusal::Unreadable`] with `padC is 64448 bytes`, and the
    /// observer was right. Decrypting with the wrong key yields random
    /// plaintext, so the exchange fails at whichever structural check bites
    /// first: the pad length, the payload length, or the verification constant.
    /// The guarantee worth holding is the one below, that a wrong key never
    /// reads as a conforming exchange, and it is asserted rather than a
    /// particular route to it.
    ///
    /// ⚠ Random plaintext can also declare a length longer than what arrived,
    /// which reads as `mse-truncated` and leaves [`Mse::read_opening`] asking for
    /// more. That is bounded: the responder closes the connection past
    /// [`MAX_OPENING`], so a build that names a torrent this observer does not
    /// have stalls and is dropped rather than hanging the run.
    #[test]
    fn a_build_that_names_a_different_torrent_never_reads_as_a_conforming_exchange() {
        let observer = observer(Selection::Rc4);
        // The initiator keys its stream with a different torrent, which is what
        // a build connecting about another info hash would do.
        let opening = initiate(
            &their_private(),
            &observer.public_key(),
            &[0x99; 20],
            CRYPTO_RC4,
            &[],
            &[],
            IA,
        );
        // ⚠ The marker is of the secret alone, so the observer still finds it;
        // only the stream keys differ.
        if let Some((exchange, _)) = observer.read_opening(&opening) {
            assert!(
                !exchange.is_conforming(),
                "a mismatched torrent read as conforming: {:?}",
                exchange.refusals()
            );
            assert!(
                exchange.refusals().iter().any(|why| matches!(
                    why,
                    Refusal::VerificationFailed(_) | Refusal::Unreadable(_)
                )),
                "{:?}",
                exchange.refusals()
            );
        }

        // ⛔ The control, so this cannot pass over an observer that refuses
        // every exchange: the same initiator naming the right torrent conforms.
        let right = initiate(
            &their_private(),
            &observer.public_key(),
            &INFO_HASH,
            CRYPTO_RC4,
            &[],
            &[],
            IA,
        );
        let (exchange, _) = observer.read_opening(&right).expect("it completes");
        assert!(exchange.is_conforming(), "{:?}", exchange.refusals());
    }

    #[test]
    fn exchanges_past_the_cap_are_counted_rather_than_kept() {
        let observer = observer(Selection::Rc4).with_max_exchanges(2);
        let responder = observer.responder();
        let opening = initiate(
            &their_private(),
            &observer.public_key(),
            &INFO_HASH,
            CRYPTO_RC4,
            &[],
            &[],
            IA,
        );
        let connection =
            bit_ids_lab::endpoint::ConnectionId::recorded(1).expect("one is a connection");
        for _ in 0..5 {
            let _ = responder(connection, &opening);
        }
        assert_eq!(observer.exchanges().len(), 2);
        assert_eq!(observer.dropped(), 3);
    }

    /// ⚠ The observer's own key is stable across runs, for the reason the
    /// module gives: two runs of one capture have to be comparable.
    #[test]
    fn the_observers_key_is_the_same_on_every_run() {
        assert_eq!(
            observer(Selection::Rc4).public_key(),
            observer(Selection::Plaintext).public_key()
        );
    }
}
