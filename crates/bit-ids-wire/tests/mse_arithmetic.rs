//! The control on `OBS-11`'s MSE arithmetic: an implementation nobody here
//! wrote.
//!
//! ⛔ **A modular exponentiation that agrees with itself proves nothing.** The
//! double-and-add multiply in [`bit_ids_wire::mse`] is written out in this
//! project, so the suite around it is a check of that code against this
//! project's reading of the algorithm. What settles whether the reading is right
//! is a second implementation, and Python's arbitrary-precision `pow` is one
//! that predates this repository by decades.
//!
//! ⭐ **The vectors below were computed by that `pow`**, on 2026-09-06, over the
//! 768-bit MODP prime of RFC 2409 that MSE fixes. They are values this project
//! did not produce, which is what makes them a control rather than a snapshot of
//! its own output. Reproduce them with:
//!
//! ```text
//! python3 -c 'P=int("FFFF...",16); print(f"{pow(2,x,P):0192x}")'
//! ```
//!
//! ⚠ The same discipline as `PUB-03`'s `cbor2` check and `OBS-08`'s `torf` and
//! `libtorrent` runs: a third-party reader belongs in an entry's driven pass,
//! and where its answer is a fixed value rather than a running program the value
//! is committed with the provenance beside it.

use bit_ids_wire::mse::{PRIVATE_LEN, public_key, shared_secret};

/// The private exponent for a seed, matching the generator the module's own
/// tests use so the two sets of vectors describe the same inputs.
fn private(seed: u8) -> [u8; PRIVATE_LEN] {
    let mut out = [0_u8; PRIVATE_LEN];
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = seed
            .wrapping_mul(31)
            .wrapping_add(u8::try_from(index).expect("below 256"));
    }
    out
}

fn hex(bytes: &[u8]) -> String {
    use core::fmt::Write as _;

    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(out, "{byte:02x}").expect("writing to a String cannot fail");
    }
    out
}

/// Public keys Python computed as `pow(2, x, P)`.
///
/// ⚠ **One line each, exactly 192 hexadecimal digits, quoted at both ends.**
/// `check-no-secrets --public` refuses a long hex run because that is the shape
/// of a leaked key, and these are the opposite: a Diffie-Hellman *public* key
/// over a published group, derived from a private exponent spelled out in this
/// same file. The allowance in both twins is anchored on the `MSE_` name and on
/// both quotes and pinned to exactly 192, so it cannot blank the leading part of
/// a longer run the way an unanchored `{40}` once did.
const MSE_PUBLIC_KEY_SEED_1: &str = "fdab9fa53cec94f0a48477fbd9f10286fe4eb034c2e9a90a32545d4711a9a6cb3d883f950aeeb85a2dd8a9e88893226edf8ea55eb92433e6a951a1c6b2461a444862130bb70c8ff0e050d47fcf982a9771c9b4c418b25ece53685a8742bb91e2";
const MSE_PUBLIC_KEY_SEED_2: &str = "52fd19301c64aa53cf1adde99af1b222567ff87318e855a1df70fd4382d462b71a493d83b648e7c70fdc7f83c722a0916fac9e77af489508041cbaa26f8335cbd6a9bce614df0f74096167b75708a75bf9abb82ee3e2635a897302c52084b71a";
const MSE_PUBLIC_KEY_SEED_3: &str = "ff0172e82b03a187fc4fe1799fcc0210a0bdeb168d086331362c6afc48aa92a1de67a137d18fc59eb132b40528b5501eef1b1a4c16782175d5ed97b4e5bd3b6ed5b3ce48f0a3c8b88888379891d45e1823f3d01ff7f050f0d3c9cef9b9e5ba23";
const MSE_PUBLIC_KEY_SEED_255: &str = "743c162bb3c678edf805fa4ac8a713699db65fdac413e4428a306f5ebaa4555bc15a44b902e9be5d261dc9d63769d865dc4e4da8da2d2bce14a98b14528666b930b6c9121e96fe1ba113dcdbdc2b4be385e08f22c6c4314c05b1adc872d43376";

/// The shared secret Python computed for seeds 1 and 2.
const MSE_SHARED_SECRET_1_2: &str = "eed49055e4bdea8cb7cd6807adfd6d5bd9994efc11ff5055cae32e83d8c9744e636b02ec77a35ba3109c2e76c0d59e58bcd9a34162b7f1af1afe334ee7a3598beaa7c10ffb897079ee2ad07837b016144797f0586cf1d1dfef9f6307b357e95b";

/// The seeds above, with the value each one should produce.
const PUBLIC_KEYS: [(u8, &str); 4] = [
    (1, MSE_PUBLIC_KEY_SEED_1),
    (2, MSE_PUBLIC_KEY_SEED_2),
    (3, MSE_PUBLIC_KEY_SEED_3),
    (255, MSE_PUBLIC_KEY_SEED_255),
];

/// ⛔ The whole point. Four exponentiations, every digit compared against a
/// value this project did not compute.
#[test]
fn every_public_key_matches_the_one_python_computed() {
    for (seed, expected) in PUBLIC_KEYS {
        assert_eq!(
            expected.len(),
            192,
            "seed {seed}: a 768-bit value is 192 hex digits"
        );
        assert_eq!(
            hex(&public_key(&private(seed))),
            expected,
            "seed {seed}: the modular exponentiation disagrees with Python's"
        );
    }
}

#[test]
fn the_shared_secret_matches_the_one_python_computed() {
    let expected = MSE_SHARED_SECRET_1_2;
    let (a, b) = (private(1), private(2));
    let (ya, yb) = (public_key(&a), public_key(&b));
    assert_eq!(hex(&shared_secret(&yb, &a)), expected);
    // ⚠ And from the other side, which is the property the exchange rests on
    // and is checked here against the same external value rather than against
    // the first half of this test.
    assert_eq!(hex(&shared_secret(&ya, &b)), expected);
}

/// ⚠ **A control on the control.** If the vectors above were somehow this
/// project's own output rather than Python's, every assertion would still pass.
/// What cannot be faked that way is a value with a known closed form: `2^1 mod P`
/// is 2 for any modulus above 2, and no implementation that got the arithmetic
/// wrong lands on it by accident.
#[test]
fn an_exponent_of_one_returns_the_generator_itself() {
    let mut one = [0_u8; PRIVATE_LEN];
    one[PRIVATE_LEN - 1] = 1;
    let key = public_key(&one);
    assert_eq!(key[PRIVATE_LEN..].len(), 76);
    assert_eq!(&key[..95], &[0_u8; 95][..], "2^1 has 95 leading zero bytes");
    assert_eq!(key[95], 2);

    // ⚠ And an exponent of zero gives one, which is the other closed form.
    let zero = [0_u8; PRIVATE_LEN];
    let key = public_key(&zero);
    assert_eq!(&key[..95], &[0_u8; 95][..]);
    assert_eq!(key[95], 1);
}
