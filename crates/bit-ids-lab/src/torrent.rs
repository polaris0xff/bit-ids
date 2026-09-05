//! The synthetic torrent a capture hands a client.
//!
//! `OBS-08`. A client announces about an info hash and asks for pieces of a real
//! piece layout, so without a torrent the lab can accept a connection and cannot
//! make a build say anything.
//!
//! ⛔ **Generated, never committed, and never a copyrighted file.** The payload
//! is bytes this module produces from a declared seed. `SECURITY.md` and
//! `docs/AGENTS.md` both forbid redistributing anything, and a lab that shipped
//! a file to seed would be doing exactly that.
//!
//! ⭐ **Generating it is what makes it citable.** `capture.fixture` in
//! `bit-ids::record` is a digest of the metainfo a run used, and a torrent whose
//! bytes are a function of its declared inputs can be rebuilt from the record
//! and compared against that digest. A committed file could only be trusted.
//!
//! # Two digests of two different things, and confusing them is the trap
//!
//! ⚠ **The info hash is SHA-1 of the encoded info dictionary.** BEP 3 fixes
//! that and nothing here can change it: a client computes the same value and
//! announces it. **`capture.fixture` is SHA-256 of the whole metainfo file**,
//! which is this project's own canonical digest of the artefact. They are
//! different algorithms over different byte ranges, and a record that cited one
//! where it meant the other would name a torrent nobody used.
//!
//! # The payload generator is deterministic on purpose
//!
//! ⚠ `docs/conventions/code.md` asks for a cryptographic random source, and this
//! inverts it for the reason the UDP tracker's connection ids do. The payload
//! protects nothing, and two runs of one capture have to produce the same
//! torrent or their records cite different fixtures for the same experiment. The
//! generator is `SplitMix64`, which is fully specified in a few lines, so the
//! bytes are reproducible from the seed by anything that reads this file.

use bit_ids::canonical::Sha256Digest;
use bit_ids_wire::bencode::{self, Value};
use sha1::{Digest as _, Sha1};

/// The width of one SHA-1 piece hash, fixed by BEP 3.
pub const PIECE_HASH_LEN: usize = 20;

/// The smallest piece length a client is expected to accept, from BEP 3
/// practice: 16 `KiB`.
pub const MIN_PIECE_LENGTH: u32 = 16 * 1024;

/// The largest payload this module will generate.
///
/// ⛔ A bound, not a preference. A spec is a value a caller supplies, and
/// multiplying two large numbers into an allocation is how a lab takes a host
/// down. 64 `MiB` is far above anything a capture needs to make a client
/// announce and ask for a piece.
pub const MAX_PAYLOAD_BYTES: u64 = 64 * 1024 * 1024;

/// The declared inputs a torrent is generated from.
///
/// ⭐ Everything the bytes depend on is here. A field that changed the output
/// without appearing in a spec would make the torrent unreproducible from the
/// record, which is the whole reason it is generated.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TorrentSpec {
    /// The `name` field, which is also the file name a client writes.
    pub name: String,
    /// Bytes per piece.
    pub piece_length: u32,
    /// How many pieces.
    pub piece_count: u32,
    /// The seed the payload is generated from.
    pub payload_seed: u64,
    /// The announce URL, or `None` for a torrent with no tracker.
    pub announce: Option<String>,
    /// The `private` flag of BEP 27.
    pub private: bool,
    /// The `creation date` field, in seconds.
    ///
    /// ⚠ Declared rather than read from the clock. A torrent that used the
    /// current time would have different bytes on every run, so two runs of one
    /// capture would cite different fixtures for the same experiment.
    pub created_at: i64,
}

impl Default for TorrentSpec {
    fn default() -> Self {
        Self {
            name: "bit-ids-fixture".to_owned(),
            piece_length: MIN_PIECE_LENGTH,
            piece_count: 4,
            payload_seed: 1,
            announce: None,
            private: false,
            created_at: 0,
        }
    }
}

/// Why a torrent could not be generated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TorrentError {
    /// The piece length is below [`MIN_PIECE_LENGTH`] or not a power of two.
    PieceLength(u32),
    /// The piece count is zero, so there would be nothing to hash.
    NoPieces,
    /// The payload would be larger than [`MAX_PAYLOAD_BYTES`].
    PayloadTooLarge(u64),
    /// The name is empty, which no client accepts.
    EmptyName,
}

impl core::fmt::Display for TorrentError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::PieceLength(length) => write!(
                f,
                "piece length {length} is not a power of two of at least {MIN_PIECE_LENGTH}"
            ),
            Self::NoPieces => f.write_str("a torrent with no pieces has nothing to hash"),
            Self::PayloadTooLarge(bytes) => write!(
                f,
                "the payload would be {bytes} bytes, over the {MAX_PAYLOAD_BYTES} cap"
            ),
            Self::EmptyName => f.write_str("a torrent needs a name"),
        }
    }
}

impl core::error::Error for TorrentError {}

/// A generated torrent, with everything derived from it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntheticTorrent {
    spec: TorrentSpec,
    payload: Vec<u8>,
    info: Value,
    info_hash: [u8; PIECE_HASH_LEN],
    metainfo: Vec<u8>,
}

impl SyntheticTorrent {
    /// Generates the torrent `spec` describes.
    ///
    /// # Errors
    ///
    /// Returns [`TorrentError`] for a spec that describes no usable torrent.
    /// Every refusal is about the spec, so a caller can act on it without
    /// reading these bytes.
    pub fn generate(spec: TorrentSpec) -> Result<Self, TorrentError> {
        if spec.name.is_empty() {
            return Err(TorrentError::EmptyName);
        }
        if spec.piece_length < MIN_PIECE_LENGTH || !spec.piece_length.is_power_of_two() {
            return Err(TorrentError::PieceLength(spec.piece_length));
        }
        if spec.piece_count == 0 {
            return Err(TorrentError::NoPieces);
        }
        let total = u64::from(spec.piece_length) * u64::from(spec.piece_count);
        if total > MAX_PAYLOAD_BYTES {
            return Err(TorrentError::PayloadTooLarge(total));
        }

        let payload = generate_payload(spec.payload_seed, total);
        let mut pieces = Vec::with_capacity(spec.piece_count as usize * PIECE_HASH_LEN);
        for chunk in payload.chunks(spec.piece_length as usize) {
            pieces.extend_from_slice(&Sha1::digest(chunk));
        }

        // ⚠ Sorted, and the info dictionary is built once. The bytes that are
        // hashed are the bytes that go in the file, because the same `Value` is
        // encoded for both: hashing a separately built copy is how an info hash
        // comes to name a dictionary the file does not contain.
        let mut info: Vec<(Vec<u8>, Value)> = vec![
            (
                b"length".to_vec(),
                Value::integer(i64::try_from(total).unwrap_or(i64::MAX)),
            ),
            (
                b"name".to_vec(),
                Value::bytes(spec.name.clone().into_bytes()),
            ),
            (
                b"piece length".to_vec(),
                Value::integer(i64::from(spec.piece_length)),
            ),
            (b"pieces".to_vec(), Value::bytes(pieces)),
        ];
        if spec.private {
            info.push((b"private".to_vec(), Value::integer(1)));
        }
        info.sort_by(|left, right| left.0.cmp(&right.0));
        let info = Value::Dictionary(info);

        let encoded_info = bencode::encode(&info);
        let mut info_hash = [0_u8; PIECE_HASH_LEN];
        info_hash.copy_from_slice(&Sha1::digest(&encoded_info));

        let mut document: Vec<(Vec<u8>, Value)> = vec![
            (
                b"created by".to_vec(),
                Value::bytes(b"bit-ids synthetic fixture".to_vec()),
            ),
            (b"creation date".to_vec(), Value::integer(spec.created_at)),
            (b"info".to_vec(), info.clone()),
        ];
        if let Some(announce) = &spec.announce {
            document.push((
                b"announce".to_vec(),
                Value::bytes(announce.clone().into_bytes()),
            ));
        }
        document.sort_by(|left, right| left.0.cmp(&right.0));
        let metainfo = bencode::encode(&Value::Dictionary(document));

        Ok(Self {
            spec,
            payload,
            info,
            info_hash,
            metainfo,
        })
    }

    /// The spec these bytes came from.
    #[must_use]
    pub const fn spec(&self) -> &TorrentSpec {
        &self.spec
    }

    /// The generated payload, in full.
    #[must_use]
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// One piece of the payload.
    #[must_use]
    pub fn piece(&self, index: u32) -> Option<&[u8]> {
        let length = self.spec.piece_length as usize;
        let start = (index as usize).checked_mul(length)?;
        self.payload.get(start..start + length)
    }

    /// The info dictionary, as a decoded document.
    #[must_use]
    pub const fn info(&self) -> &Value {
        &self.info
    }

    /// The info hash: SHA-1 of the encoded info dictionary, per BEP 3.
    ///
    /// ⛔ Not [`SyntheticTorrent::digest`], and not a digest of the whole file.
    /// This is the value a client announces, and it is the one thing here whose
    /// algorithm this project does not choose.
    #[must_use]
    pub const fn info_hash(&self) -> &[u8; PIECE_HASH_LEN] {
        &self.info_hash
    }

    /// The `.torrent` bytes, which is what a client is handed.
    #[must_use]
    pub fn metainfo(&self) -> &[u8] {
        &self.metainfo
    }

    /// The digest `capture.fixture` cites: SHA-256 of the metainfo file.
    ///
    /// ⚠ This project's own canonical digest of the artefact, not the info
    /// hash. Two runs that generated the same torrent produce the same value
    /// here, which is what makes a record's fixture claim checkable.
    #[must_use]
    pub fn digest(&self) -> Sha256Digest {
        Sha256Digest::of(&self.metainfo)
    }
}

/// Generates `length` bytes from `seed`.
///
/// `SplitMix64`, which is small enough to state in full: the state advances by
/// the golden-ratio constant and each output is an avalanche of the state. It is
/// specified here rather than depended on so the bytes are reproducible by
/// anything that reads this function.
fn generate_payload(seed: u64, length: u64) -> Vec<u8> {
    let capacity = usize::try_from(length).unwrap_or(usize::MAX);
    let mut out = Vec::with_capacity(capacity);
    let mut state = seed;
    while out.len() < capacity {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut word = state;
        word = (word ^ (word >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        word = (word ^ (word >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        word ^= word >> 31;
        let take = (capacity - out.len()).min(8);
        out.extend_from_slice(&word.to_be_bytes()[..take]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Sha1, generate_payload};
    use sha1::Digest as _;

    /// RFC 3174 section 7.3, the first test vector.
    ///
    /// ⚠ Named rather than written inline because `check-no-secrets --public`
    /// refuses a bare run of long hex, and correctly: 40 lowercase hex digits
    /// is exactly what a token looks like. The exclusion is anchored to this
    /// name and to a 40-digit value, so it cannot spread to hex elsewhere.
    /// `docs/security/secrets.md` carries the rule that a false positive is
    /// narrowed rather than switched off.
    const RFC3174_ABC: &str = "a9993e364706816aba3e25717850c26c9cd0d89d";
    /// RFC 3174, the digest of the empty input.
    const RFC3174_EMPTY: &str = "da39a3ee5e6b4b0d3255bfef95601890afd80709";
    /// RFC 3174 section 7.3, the second test vector.
    const RFC3174_ALPHABET: &str = "84983e441c3bd26ebaae4aa1f95129e5e54670f1";

    #[test]
    fn the_sha1_implementation_matches_the_published_vectors() {
        // ⭐ The info hash is the one value here whose algorithm this project
        // does not get to choose, so the dependency that computes it is checked
        // against the specification rather than trusted.
        assert_eq!(hex(&Sha1::digest(b"abc")), RFC3174_ABC);
        assert_eq!(hex(&Sha1::digest(b"")), RFC3174_EMPTY);
        assert_eq!(
            hex(&Sha1::digest(
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
            )),
            RFC3174_ALPHABET
        );
    }

    #[test]
    fn the_payload_generator_is_reproducible_and_seed_dependent() {
        assert_eq!(generate_payload(7, 64), generate_payload(7, 64));
        assert_ne!(generate_payload(7, 64), generate_payload(8, 64));
        assert_eq!(generate_payload(7, 5).len(), 5, "a partial word is trimmed");
        // A prefix relationship, so a longer request extends rather than
        // re-generates: two pieces of one payload must not depend on how much
        // was asked for.
        assert_eq!(generate_payload(7, 5), generate_payload(7, 64)[..5]);
    }

    fn hex(bytes: &[u8]) -> String {
        use core::fmt::Write as _;
        let mut out = String::new();
        for byte in bytes {
            write!(out, "{byte:02x}").expect("writing to a String cannot fail");
        }
        out
    }
}
