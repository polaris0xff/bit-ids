//! Message stream encryption: the handshake a build performs before any
//! observer can read the peer wire.
//!
//! MSE has no BEP. It is the de-facto "protocol encryption" every major client
//! implements, and it is obfuscation rather than security: the shared secret is
//! unauthenticated, so anything on the path can complete the exchange. ⭐ **That
//! is exactly why a lab can observe it.** The lab is the peer, so it performs
//! the exchange as the receiving side and reads what the build offered.
//!
//! # Why it comes first, and what that costs
//!
//! ⛔ **A lab that offers MSE changes what `OBS-04` and `OBS-05` see.** The
//! encrypted handshake wraps the `BitTorrent` one, so a build that negotiates
//! encryption sends its peer ID inside `IA` rather than in the clear. The offer
//! is therefore a condition of the measurement and is recorded beside the
//! result, the way [`crate::peer_wire`]'s BEP 10 offer already is.
//!
//! # The arithmetic, and why it is written out here
//!
//! MSE fixes the 768-bit MODP group of RFC 2409 with generator 2, so the
//! exchange needs modular exponentiation over a 768-bit modulus. ⭐ **The
//! modular multiply is double-and-add rather than schoolbook-with-division**,
//! which is slower and very much simpler: every step is a shift by one, a
//! comparison and a conditional subtraction, and there is no long division to
//! get subtly wrong. `tests/mse_arithmetic.rs` compares it against Python's own
//! arbitrary-precision `pow`, which is an implementation nobody here wrote.
//!
//! ⚠ **Nothing here is a security primitive and none of it should be reused as
//! one.** There is no constant-time discipline, because there is nothing to
//! protect: the only party on the far side is a binary this project installed
//! minutes earlier, on a loopback socket, and MSE's own threat model does not
//! include the peer.
//!
//! # What it observes
//!
//! The identity is in `crypto_provide`, which says whether a build offers
//! plaintext, `RC4`, or both, and in the padding lengths it chooses, and in
//! whether it initiates MSE at all before falling back. ⛔ **Every one of those
//! is recorded and none is refused**, the way every other codec here works.

use sha1::{Digest as _, Sha1};

use crate::error::WireError;

/// How many bytes the MSE public key occupies: the 768-bit modulus.
pub const KEY_LEN: usize = 96;

/// How many bytes of private exponent MSE uses.
///
/// ⚠ The specification says the private keys are 160-bit random numbers, so the
/// exponent is a fifth of the modulus width and the exponentiation is a fifth of
/// the work. Pinned to its literal for the reason `OBS-08` gives about a
/// constant every test reads.
pub const PRIVATE_LEN: usize = 20;

/// The largest padding MSE permits at each step.
pub const MAX_PAD: usize = 512;

/// The verification constant: eight zero bytes, encrypted, which is how the
/// receiving side recognises that it has the right key.
pub const VC: [u8; 8] = [0; 8];

/// `crypto_provide` bit for plaintext after the handshake.
pub const CRYPTO_PLAINTEXT: u32 = 0x01;

/// `crypto_provide` bit for `RC4` after the handshake.
pub const CRYPTO_RC4: u32 = 0x02;

/// How many `RC4` keystream bytes MSE discards before use.
///
/// ⚠ 1024, which is the specification's own defence against the early-keystream
/// bias in `RC4`. A build that discarded a different number would produce a
/// stream this observer cannot read, which is itself a finding.
pub const RC4_DISCARD: usize = 1024;

/// How many limbs a 768-bit number occupies at 64 bits each.
const LIMBS: usize = 12;

/// The 768-bit MODP prime of RFC 2409, which MSE fixes as its modulus.
///
/// ⚠ Most significant limb first, matching the byte order the wire uses.
const P: [u64; LIMBS] = [
    0xFFFF_FFFF_FFFF_FFFF,
    0xC90F_DAA2_2168_C234,
    0xC4C6_628B_80DC_1CD1,
    0x2902_4E08_8A67_CC74,
    0x020B_BEA6_3B13_9B22,
    0x514A_0879_8E34_04DD,
    0xEF95_19B3_CD3A_431B,
    0x302B_0A6D_F25F_1437,
    0x4FE1_356D_6D51_C245,
    0xE485_B576_625E_7EC6,
    0xF44C_42E9_A63A_3620,
    0xFFFF_FFFF_FFFF_FFFF,
];

/// A 768-bit unsigned integer, most significant limb first.
type Big = [u64; LIMBS];

/// The prime, as the bytes it occupies on the wire.
#[must_use]
pub fn modulus_bytes() -> [u8; KEY_LEN] {
    to_bytes(&P)
}

/// `a` compared with `b`.
fn cmp(a: &Big, b: &Big) -> core::cmp::Ordering {
    for (left, right) in a.iter().zip(b.iter()) {
        match left.cmp(right) {
            core::cmp::Ordering::Equal => {}
            other => return other,
        }
    }
    core::cmp::Ordering::Equal
}

/// `a - b`, which the callers only ever use when `a >= b` after accounting for
/// the carry they pass separately.
fn sub_assign(a: &mut Big, b: &Big) {
    let mut borrow = 0_u64;
    for index in (0..LIMBS).rev() {
        let (first, one) = a[index].overflowing_sub(b[index]);
        let (second, two) = first.overflowing_sub(borrow);
        a[index] = second;
        borrow = u64::from(one) + u64::from(two);
    }
}

/// `a + b`, reporting the carry out of the top limb.
fn add_assign(a: &mut Big, b: &Big) -> u64 {
    let mut carry = 0_u64;
    for index in (0..LIMBS).rev() {
        let (first, one) = a[index].overflowing_add(b[index]);
        let (second, two) = first.overflowing_add(carry);
        a[index] = second;
        carry = u64::from(one) + u64::from(two);
    }
    carry
}

/// `a << 1`, reporting the bit shifted out of the top limb.
fn shl1_assign(a: &mut Big) -> u64 {
    let mut carry = 0_u64;
    for index in (0..LIMBS).rev() {
        let out = a[index] >> 63;
        a[index] = (a[index] << 1) | carry;
        carry = out;
    }
    carry
}

/// Reduces a value that is at most `2P - 1`, given the carry out of its top.
///
/// ⚠ One conditional subtraction is enough and that is a property of the
/// callers: both of them produce a sum of two values each below `P`, so the
/// result is below `2P`.
fn reduce_once(value: &mut Big, carry: u64) {
    if carry != 0 || cmp(value, &P) != core::cmp::Ordering::Less {
        sub_assign(value, &P);
    }
}

/// `(a * b) mod P`, by double-and-add.
///
/// ⭐ **No division anywhere.** The schoolbook alternative needs a 1536-bit by
/// 768-bit long division, which is the part of a bignum implementation that is
/// hardest to get right and hardest to test. This is 768 iterations of a shift,
/// a compare and a subtract, and the test compares the whole thing against
/// Python's `pow`.
fn mul_mod(a: &Big, b: &Big) -> Big {
    let mut result = [0_u64; LIMBS];
    for limb in b {
        for bit in (0..64).rev() {
            let carry = shl1_assign(&mut result);
            reduce_once(&mut result, carry);
            if (limb >> bit) & 1 == 1 {
                let carry = add_assign(&mut result, a);
                reduce_once(&mut result, carry);
            }
        }
    }
    result
}

/// `base^exponent mod P`, with the exponent taken from big-endian bytes.
fn pow_mod(base: &Big, exponent: &[u8]) -> Big {
    let mut result = [0_u64; LIMBS];
    result[LIMBS - 1] = 1;
    for byte in exponent {
        for bit in (0..8).rev() {
            result = mul_mod(&result, &result);
            if (byte >> bit) & 1 == 1 {
                result = mul_mod(&result, base);
            }
        }
    }
    result
}

/// Reads 96 big-endian bytes as a number.
fn from_bytes(bytes: &[u8; KEY_LEN]) -> Big {
    let mut value = [0_u64; LIMBS];
    for (index, limb) in value.iter_mut().enumerate() {
        let mut eight = [0_u8; 8];
        eight.copy_from_slice(&bytes[index * 8..index * 8 + 8]);
        *limb = u64::from_be_bytes(eight);
    }
    value
}

/// Writes a number as 96 big-endian bytes.
fn to_bytes(value: &Big) -> [u8; KEY_LEN] {
    let mut bytes = [0_u8; KEY_LEN];
    for (index, limb) in value.iter().enumerate() {
        bytes[index * 8..index * 8 + 8].copy_from_slice(&limb.to_be_bytes());
    }
    bytes
}

/// The public key for a private exponent: `2^private mod P`.
///
/// ⚠ The generator is 2, which MSE fixes along with the modulus.
#[must_use]
pub fn public_key(private: &[u8; PRIVATE_LEN]) -> [u8; KEY_LEN] {
    let mut generator = [0_u64; LIMBS];
    generator[LIMBS - 1] = 2;
    to_bytes(&pow_mod(&generator, private))
}

/// The shared secret `S`: their public key raised to our private exponent.
#[must_use]
pub fn shared_secret(theirs: &[u8; KEY_LEN], private: &[u8; PRIVATE_LEN]) -> [u8; KEY_LEN] {
    to_bytes(&pow_mod(&from_bytes(theirs), private))
}

/// The `RC4` state MSE uses, with its mandatory discard already applied.
///
/// ⚠ **Not a security primitive.** See the module documentation: there is
/// nothing here to protect, and this is written for readability rather than for
/// resistance to anything.
#[derive(Clone, Debug)]
pub struct Rc4 {
    state: [u8; 256],
    i: u8,
    j: u8,
}

impl Rc4 {
    /// Keys the cipher and discards [`RC4_DISCARD`] bytes.
    ///
    /// # Panics
    ///
    /// Panics on an empty key, which is a caller error rather than an input:
    /// every key here is a 20-byte `SHA-1` digest.
    #[must_use]
    pub fn new(key: &[u8]) -> Self {
        let mut state = [0_u8; 256];
        for (index, slot) in state.iter_mut().enumerate() {
            *slot = u8::try_from(index).expect("an index below 256");
        }
        let mut j = 0_u8;
        for index in 0..256 {
            j = j
                .wrapping_add(state[index])
                .wrapping_add(key[index % key.len()]);
            state.swap(index, usize::from(j));
        }
        let mut cipher = Self { state, i: 0, j: 0 };
        // ⛔ The discard is part of the protocol, not a hardening choice. A
        // build that skipped it produces a stream this observer cannot read.
        let mut sink = [0_u8; RC4_DISCARD];
        cipher.apply(&mut sink);
        cipher
    }

    /// Exclusive-ors `data` with the keystream, in place.
    pub fn apply(&mut self, data: &mut [u8]) {
        for byte in data {
            self.i = self.i.wrapping_add(1);
            self.j = self.j.wrapping_add(self.state[usize::from(self.i)]);
            self.state.swap(usize::from(self.i), usize::from(self.j));
            let index =
                self.state[usize::from(self.i)].wrapping_add(self.state[usize::from(self.j)]);
            *byte ^= self.state[usize::from(index)];
        }
    }
}

/// `SHA-1` of the concatenation of its arguments.
fn hash(parts: &[&[u8]]) -> [u8; 20] {
    let mut digest = Sha1::new();
    for part in parts {
        digest.update(part);
    }
    let mut out = [0_u8; 20];
    out.copy_from_slice(&Sha1::finalize(digest));
    out
}

/// `HASH('req1', S)`, which the initiator sends so the receiver can find the
/// start of the encrypted section without decrypting anything.
#[must_use]
pub fn req1(secret: &[u8; KEY_LEN]) -> [u8; 20] {
    hash(&[b"req1", secret])
}

/// `HASH('req2', SKEY) xor HASH('req3', S)`, which names the torrent without
/// putting its info hash on the wire.
#[must_use]
pub fn req2_xor_req3(info_hash: &[u8; 20], secret: &[u8; KEY_LEN]) -> [u8; 20] {
    let two = hash(&[b"req2", info_hash]);
    let three = hash(&[b"req3", secret]);
    let mut out = [0_u8; 20];
    for (slot, (left, right)) in out.iter_mut().zip(two.iter().zip(three.iter())) {
        *slot = left ^ right;
    }
    out
}

/// The initiator's keystream key, `HASH('keyA', S, SKEY)`.
#[must_use]
pub fn key_a(secret: &[u8; KEY_LEN], info_hash: &[u8; 20]) -> [u8; 20] {
    hash(&[b"keyA", secret, info_hash])
}

/// The receiver's keystream key, `HASH('keyB', S, SKEY)`.
#[must_use]
pub fn key_b(secret: &[u8; KEY_LEN], info_hash: &[u8; 20]) -> [u8; 20] {
    hash(&[b"keyB", secret, info_hash])
}

/// What the initiator put in the third message, once it has been decrypted.
///
/// ⚠ Every field is kept as it arrived. The padding is kept rather than
/// discarded, because its length and its content are choices a build makes and
/// a reader that dropped it could not say what those were.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Provide {
    /// The verification constant as it decrypted, which must be [`VC`].
    pub vc: [u8; 8],
    /// The methods the initiator offers.
    pub crypto_provide: u32,
    /// The padding it chose, in full.
    pub pad: Vec<u8>,
    /// The initial payload, which is the `BitTorrent` handshake.
    pub initial_payload: Vec<u8>,
}

impl Provide {
    /// Whether the initiator offered plaintext after the handshake.
    #[must_use]
    pub const fn offers_plaintext(&self) -> bool {
        self.crypto_provide & CRYPTO_PLAINTEXT != 0
    }

    /// Whether the initiator offered `RC4` after the handshake.
    #[must_use]
    pub const fn offers_rc4(&self) -> bool {
        self.crypto_provide & CRYPTO_RC4 != 0
    }

    /// Whether the verification constant decrypted to what MSE fixes.
    #[must_use]
    pub fn verified(&self) -> bool {
        self.vc == VC
    }
}

/// Reads the encrypted remainder of the initiator's third message.
///
/// `body` starts at the encrypted `VC` and runs to the end of what arrived. The
/// caller has already matched [`req1`] and located this offset.
///
/// # Errors
///
/// Returns `mse-truncated` when the message stops inside a field, and
/// `mse-pad-length` when a declared padding length is beyond what MSE permits.
/// ⚠ Nothing else is refused: a bad `VC` and an unknown `crypto_provide` bit are
/// both recorded rather than rejected, because each is a finding about the
/// build.
pub fn read_provide(cipher: &mut Rc4, body: &[u8]) -> Result<Provide, WireError> {
    let mut plain = body.to_vec();
    cipher.apply(&mut plain);
    let mut at = 0_usize;
    let vc = take_array::<8>(&plain, &mut at, "vc")?;
    let crypto_provide = u32::from_be_bytes(take_array::<4>(&plain, &mut at, "crypto_provide")?);
    let pad_len = usize::from(u16::from_be_bytes(take_array::<2>(
        &plain,
        &mut at,
        "len(padC)",
    )?));
    if pad_len > MAX_PAD {
        return Err(WireError::new(
            "mse-pad-length",
            at,
            format!("padC is {pad_len} bytes, over the {MAX_PAD} MSE permits"),
        ));
    }
    let pad = take_slice(&plain, &mut at, pad_len, "padC")?.to_vec();
    let ia_len = usize::from(u16::from_be_bytes(take_array::<2>(
        &plain, &mut at, "len(IA)",
    )?));
    let initial_payload = take_slice(&plain, &mut at, ia_len, "IA")?.to_vec();
    Ok(Provide {
        vc,
        crypto_provide,
        pad,
        initial_payload,
    })
}

/// Writes the receiver's fourth message: `VC`, the selection, and its padding.
///
/// # Panics
///
/// Panics when `pad` is longer than 65535 bytes, which MSE's own two-byte
/// length field cannot express and no caller here offers: [`MAX_PAD`] is 512.
#[must_use]
pub fn write_select(cipher: &mut Rc4, crypto_select: u32, pad: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + 4 + 2 + pad.len());
    out.extend_from_slice(&VC);
    out.extend_from_slice(&crypto_select.to_be_bytes());
    out.extend_from_slice(
        &u16::try_from(pad.len())
            .expect("a caller does not offer padding above 65535")
            .to_be_bytes(),
    );
    out.extend_from_slice(pad);
    cipher.apply(&mut out);
    out
}

/// Where `needle` starts in `haystack`, or [`None`].
///
/// ⚠ The receiving side has to find `HASH('req1', S)` in a stream that begins
/// with up to [`MAX_PAD`] bytes of padding it cannot predict, so a scan is what
/// MSE requires rather than a shortcut.
#[must_use]
pub fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    (0..=haystack.len() - needle.len()).find(|&at| &haystack[at..at + needle.len()] == needle)
}

fn take_slice<'a>(
    from: &'a [u8],
    at: &mut usize,
    len: usize,
    what: &str,
) -> Result<&'a [u8], WireError> {
    let end = at
        .checked_add(len)
        .ok_or_else(|| WireError::new("mse-truncated", *at, format!("{what} length overflows")))?;
    if end > from.len() {
        return Err(WireError::new(
            "mse-truncated",
            *at,
            format!("{what} wants {len} bytes and {} remain", from.len() - *at),
        ));
    }
    let slice = &from[*at..end];
    *at = end;
    Ok(slice)
}

fn take_array<const N: usize>(
    from: &[u8],
    at: &mut usize,
    what: &str,
) -> Result<[u8; N], WireError> {
    let slice = take_slice(from, at, N, what)?;
    let mut out = [0_u8; N];
    out.copy_from_slice(slice);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{
        CRYPTO_PLAINTEXT, CRYPTO_RC4, KEY_LEN, MAX_PAD, PRIVATE_LEN, Rc4, VC, find, key_a, key_b,
        modulus_bytes, public_key, read_provide, req1, req2_xor_req3, shared_secret, write_select,
    };

    fn private(seed: u8) -> [u8; PRIVATE_LEN] {
        let mut out = [0_u8; PRIVATE_LEN];
        for (index, byte) in out.iter_mut().enumerate() {
            *byte = seed
                .wrapping_mul(31)
                .wrapping_add(u8::try_from(index).expect("below 256"));
        }
        out
    }

    /// ⭐ The property the whole exchange rests on: both sides compute the same
    /// secret from different halves.
    #[test]
    fn both_sides_of_the_exchange_reach_one_shared_secret() {
        let (a, b) = (private(1), private(2));
        let (ya, yb) = (public_key(&a), public_key(&b));
        assert_ne!(ya, yb, "two private keys gave one public key");
        assert_eq!(shared_secret(&yb, &a), shared_secret(&ya, &b));
        assert_eq!(ya.len(), KEY_LEN);
    }

    /// ⚠ The modulus is the one MSE names, checked by its width and its ends
    /// rather than trusted. `tests/mse_arithmetic.rs` is what compares the
    /// arithmetic itself against an implementation nobody here wrote.
    #[test]
    fn the_modulus_is_the_768_bit_group_the_specification_fixes() {
        let bytes = modulus_bytes();
        assert_eq!(bytes.len(), 96);
        assert_eq!(&bytes[..8], &[0xFF; 8]);
        assert_eq!(&bytes[88..], &[0xFF; 8]);
        assert_eq!(bytes[8], 0xC9);
        assert_eq!(bytes[9], 0x0F);
    }

    /// ⭐ `RC4` against a value this project did not compute.
    ///
    /// ⛔ **The offset is the whole subtlety and the first draft's comment got
    /// it wrong.** RFC 6229 publishes the keystream for key `0102030405` at
    /// several offsets. [`Rc4::new`] applies MSE's mandatory 1024-byte discard,
    /// so what a freshly keyed cipher produces here is the stream at **offset
    /// 1024**, not the offset-zero block. Asserting the offset-zero value would
    /// have failed; asserting this one over a cipher that had skipped the
    /// discard would have failed too, so the constant pins the discard as well
    /// as the cipher.
    ///
    /// ⚠ Confirmed against an independent Python `RC4` on 2026-09-06, which
    /// agreed with RFC 6229 at offset zero and with the value below at 1024.
    /// The offset-zero block is not spelled here: it is a 32-digit hex run, and
    /// `check-no-secrets --public` reads one as the shape of a leaked key. The
    /// value is in RFC 6229 for anyone comparing.
    #[test]
    fn the_cipher_matches_the_published_test_vector_at_the_offset_mse_starts_from() {
        let mut plain = [0_u8; 16];
        let mut cipher = Rc4::new(&[0x01, 0x02, 0x03, 0x04, 0x05]);
        cipher.apply(&mut plain);
        assert_eq!(
            plain,
            [
                0x30, 0xAB, 0xBC, 0xC7, 0xC2, 0x0B, 0x01, 0x60, 0x9F, 0x23, 0xEE, 0x2D, 0x5F, 0x6B,
                0xB7, 0xDF
            ]
        );
    }

    /// ⚠ Two keys give two streams, which is the property a shared-secret
    /// derivation needs and a constant one would not have.
    #[test]
    fn the_two_directions_use_different_keys() {
        let secret = [7_u8; KEY_LEN];
        let info_hash = [0x11_u8; 20];
        assert_ne!(key_a(&secret, &info_hash), key_b(&secret, &info_hash));
        // ⚠ And both depend on the torrent as well as on the secret.
        assert_ne!(key_a(&secret, &info_hash), key_a(&secret, &[0x22; 20]));
    }

    /// ⛔ `req2 xor req3` names the torrent without putting its info hash on the
    /// wire, and that is the property to check: the value moves with the info
    /// hash and with the secret, and it is not either of them.
    #[test]
    fn the_torrent_is_named_without_its_info_hash_appearing() {
        let secret = [7_u8; KEY_LEN];
        let one = req2_xor_req3(&[0x11; 20], &secret);
        let two = req2_xor_req3(&[0x22; 20], &secret);
        assert_ne!(one, two);
        assert_ne!(one, req2_xor_req3(&[0x11; 20], &[8; KEY_LEN]));
        assert_ne!(&one[..], &[0x11_u8; 20][..]);
        // ⚠ And `req1` is of the secret alone, so a receiver can match it before
        // it knows which torrent is meant.
        assert_ne!(req1(&secret), req1(&[8; KEY_LEN]));
    }

    /// The round trip a lab performs: the initiator writes the third message and
    /// the receiver reads it back.
    #[test]
    fn a_provide_written_with_one_key_reads_back_with_the_same_key() {
        let secret = [3_u8; KEY_LEN];
        let info_hash = [0x11_u8; 20];
        let key = key_a(&secret, &info_hash);

        let mut body = Vec::new();
        body.extend_from_slice(&VC);
        body.extend_from_slice(&(CRYPTO_PLAINTEXT | CRYPTO_RC4).to_be_bytes());
        body.extend_from_slice(&3_u16.to_be_bytes());
        body.extend_from_slice(b"abc");
        body.extend_from_slice(&4_u16.to_be_bytes());
        body.extend_from_slice(b"\x13Bit");
        let mut sending = Rc4::new(&key);
        sending.apply(&mut body);

        let mut receiving = Rc4::new(&key);
        let provide = read_provide(&mut receiving, &body).expect("it reads");
        assert!(provide.verified());
        assert!(provide.offers_plaintext());
        assert!(provide.offers_rc4());
        assert_eq!(provide.pad, b"abc");
        assert_eq!(provide.initial_payload, b"\x13Bit");
    }

    /// ⚠ A wrong key is not an error: the fields decrypt to something and the
    /// verification constant is what says the key was wrong. Refusing here would
    /// turn "this build used a different secret" into a parse failure.
    #[test]
    fn a_wrong_key_leaves_the_verification_constant_to_report_it() {
        let secret = [3_u8; KEY_LEN];
        let mut body = Vec::new();
        body.extend_from_slice(&VC);
        body.extend_from_slice(&CRYPTO_RC4.to_be_bytes());
        body.extend_from_slice(&0_u16.to_be_bytes());
        body.extend_from_slice(&0_u16.to_be_bytes());
        let mut sending = Rc4::new(&key_a(&secret, &[0x11; 20]));
        sending.apply(&mut body);

        let mut wrong = Rc4::new(&key_a(&secret, &[0x99; 20]));
        // ⚠ It may or may not parse; what it must not do is claim verification.
        if let Ok(provide) = read_provide(&mut wrong, &body) {
            assert!(!provide.verified(), "a wrong key verified");
        }
    }

    #[test]
    fn a_truncated_third_message_says_where_it_stopped() {
        let key = key_a(&[3; KEY_LEN], &[0x11; 20]);
        let mut body = vec![0_u8; 6];
        Rc4::new(&key).apply(&mut body);
        let error = read_provide(&mut Rc4::new(&key), &body).expect_err("six bytes is not enough");
        assert_eq!(error.kind(), "mse-truncated");
    }

    /// ⛔ A declared padding length beyond what MSE permits is refused before it
    /// is allocated, which is the bound every other codec here carries: the
    /// sender is a binary this project installed minutes ago.
    #[test]
    fn a_padding_length_above_the_maximum_is_refused_before_it_is_allocated() {
        let key = key_a(&[3; KEY_LEN], &[0x11; 20]);
        let mut body = Vec::new();
        body.extend_from_slice(&VC);
        body.extend_from_slice(&CRYPTO_RC4.to_be_bytes());
        let over = u16::try_from(MAX_PAD + 1).expect("fits");
        body.extend_from_slice(&over.to_be_bytes());
        let mut sending = Rc4::new(&key);
        sending.apply(&mut body);
        let error = read_provide(&mut Rc4::new(&key), &body).expect_err("over the maximum");
        assert_eq!(error.kind(), "mse-pad-length");
    }

    #[test]
    fn the_selection_written_back_is_what_the_initiator_reads() {
        let key = key_b(&[3; KEY_LEN], &[0x11; 20]);
        let written = write_select(&mut Rc4::new(&key), CRYPTO_RC4, b"pad");
        let mut plain = written.clone();
        Rc4::new(&key).apply(&mut plain);
        assert_eq!(&plain[..8], &VC);
        assert_eq!(&plain[8..12], &CRYPTO_RC4.to_be_bytes());
        assert_eq!(&plain[12..14], &3_u16.to_be_bytes());
        assert_eq!(&plain[14..], b"pad");
    }

    /// ⚠ The receiver scans for `req1` because the stream opens with padding it
    /// cannot predict. An empty needle finds nothing rather than matching at
    /// zero, which would report a match in every stream.
    #[test]
    fn the_scan_finds_the_marker_after_padding_and_refuses_an_empty_needle() {
        let mut stream = vec![0xAA_u8; 300];
        stream.extend_from_slice(b"MARKER");
        stream.extend_from_slice(&[0xBB; 10]);
        assert_eq!(find(&stream, b"MARKER"), Some(300));
        assert_eq!(find(&stream, b"ABSENT"), None);
        assert_eq!(find(&stream, b""), None);
        assert_eq!(find(b"ab", b"abc"), None);
    }
}
