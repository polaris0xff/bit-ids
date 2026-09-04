//! Canonical text forms for the values a profile record carries.
//!
//! Every type here has exactly one accepted spelling for a given value. That is
//! not tidiness: two spellings of one value make two records that differ in
//! bytes and agree in meaning, and the append-only store cannot then tell a
//! correction from a duplicate. Parsing rejects a non-canonical spelling rather
//! than normalizing it, so the refusal names the defect at its source.

use core::fmt;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A value that is not in the one canonical form this schema accepts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalError {
    kind: &'static str,
    detail: String,
}

impl CanonicalError {
    pub(crate) fn new(kind: &'static str, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }

    /// The stable machine-readable name of the rejected form.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        self.kind
    }
}

impl fmt::Display for CanonicalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.detail)
    }
}

impl core::error::Error for CanonicalError {}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn write_hex(bytes: &[u8], out: &mut String) {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    for byte in bytes {
        out.push(char::from(DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
}

/// A non-empty byte string, written as lowercase hexadecimal.
///
/// Uppercase hexadecimal is refused rather than folded. Two records that spell
/// one peer ID two ways would hash differently while describing the same
/// measurement.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct HexBytes(Vec<u8>);

impl HexBytes {
    /// Wraps observed bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when `bytes` is empty. No field in this schema carries
    /// a zero-length value, so an empty one is a dropped measurement rather
    /// than a short one.
    pub fn new(bytes: Vec<u8>) -> Result<Self, CanonicalError> {
        if bytes.is_empty() {
            return Err(CanonicalError::new("empty-bytes", "byte string is empty"));
        }
        Ok(Self(bytes))
    }

    /// Parses the canonical lowercase hexadecimal form.
    ///
    /// # Errors
    ///
    /// Returns an error for an empty string, an odd digit count, or any
    /// character outside `0-9a-f`, which includes uppercase hexadecimal.
    pub fn parse(text: &str) -> Result<Self, CanonicalError> {
        let raw = text.as_bytes();
        if raw.is_empty() {
            return Err(CanonicalError::new("empty-bytes", "byte string is empty"));
        }
        if !raw.len().is_multiple_of(2) {
            return Err(CanonicalError::new(
                "odd-hex-length",
                format!("{} hex digits", raw.len()),
            ));
        }
        let mut bytes = Vec::with_capacity(raw.len() / 2);
        for pair in raw.as_chunks::<2>().0 {
            let (Some(high), Some(low)) = (hex_value(pair[0]), hex_value(pair[1])) else {
                return Err(CanonicalError::new(
                    "not-lowercase-hex",
                    format!("{:?}", &text[..text.len().min(32)]),
                ));
            };
            bytes.push((high << 4) | low);
        }
        Ok(Self(bytes))
    }

    /// The observed bytes.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// How many bytes were observed.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Always false. Present because clippy pairs it with [`HexBytes::len`];
    /// the constructors refuse an empty value.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        false
    }

    /// The canonical lowercase hexadecimal spelling.
    #[must_use]
    pub fn to_hex(&self) -> String {
        let mut out = String::with_capacity(self.0.len() * 2);
        write_hex(&self.0, &mut out);
        out
    }
}

impl fmt::Display for HexBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl Serialize for HexBytes {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for HexBytes {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(D::Error::custom)
    }
}

/// A SHA-256 digest, written as `sha256:` followed by 64 lowercase hex digits.
///
/// The algorithm travels with the value. A bare digest string is a format that
/// cannot say what produced it, and this record set is meant to outlive
/// SHA-256.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Sha256Digest([u8; 32]);

impl Sha256Digest {
    /// The prefix that names the algorithm.
    pub const PREFIX: &'static str = "sha256:";

    /// Digests a byte string.
    #[must_use]
    pub fn of(data: &[u8]) -> Self {
        use sha2::{Digest as _, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(data);
        Self(hasher.finalize().into())
    }

    /// Parses the canonical `sha256:<64 lowercase hex>` form.
    ///
    /// # Errors
    ///
    /// Returns an error when the algorithm prefix is missing or the digest is
    /// not exactly 32 bytes of lowercase hexadecimal.
    pub fn parse(text: &str) -> Result<Self, CanonicalError> {
        let Some(hex) = text.strip_prefix(Self::PREFIX) else {
            return Err(CanonicalError::new(
                "digest-algorithm",
                format!("expected the {} prefix", Self::PREFIX),
            ));
        };
        let bytes = HexBytes::parse(hex)?;
        let raw: [u8; 32] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| CanonicalError::new("digest-length", format!("{} bytes", bytes.len())))?;
        Ok(Self(raw))
    }

    /// The 32 digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for Sha256Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = String::with_capacity(Self::PREFIX.len() + 64);
        out.push_str(Self::PREFIX);
        write_hex(&self.0, &mut out);
        f.write_str(&out)
    }
}

impl Serialize for Sha256Digest {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Sha256Digest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(D::Error::custom)
    }
}

const fn is_leap_year(year: u32) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

const fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// A UTC instant written as `YYYY-MM-DDTHH:MM:SSZ`.
///
/// One spelling only. No offset other than `Z`, no fractional seconds, no
/// leap second. A capture instant that sorts as text and as time is what makes
/// the published record set orderable without parsing it.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Instant(String);

impl Instant {
    /// Parses the canonical form.
    ///
    /// # Errors
    ///
    /// Returns an error for any other length, separator, offset, or a date that
    /// does not exist. Second 60 is refused: a leap second cannot be compared
    /// against a monotonic capture clock.
    pub fn parse(text: &str) -> Result<Self, CanonicalError> {
        let raw = text.as_bytes();
        if raw.len() != 20 {
            return Err(CanonicalError::new(
                "instant-shape",
                format!("expected 20 characters, found {}", raw.len()),
            ));
        }
        let shape = |index: usize, expected: u8| raw[index] == expected;
        if !(shape(4, b'-')
            && shape(7, b'-')
            && shape(10, b'T')
            && shape(13, b':')
            && shape(16, b':')
            && shape(19, b'Z'))
        {
            return Err(CanonicalError::new(
                "instant-shape",
                "expected YYYY-MM-DDTHH:MM:SSZ",
            ));
        }
        let digits = |range: core::ops::Range<usize>| -> Option<u32> {
            let slice = &text[range];
            if slice.bytes().all(|b| b.is_ascii_digit()) {
                slice.parse().ok()
            } else {
                None
            }
        };
        let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
            digits(0..4),
            digits(5..7),
            digits(8..10),
            digits(11..13),
            digits(14..16),
            digits(17..19),
        ) else {
            return Err(CanonicalError::new(
                "instant-shape",
                "a date or time component is not a decimal number",
            ));
        };
        if !(1..=12).contains(&month) {
            return Err(CanonicalError::new(
                "instant-range",
                format!("month {month}"),
            ));
        }
        if day < 1 || day > days_in_month(year, month) {
            return Err(CanonicalError::new("instant-range", format!("day {day}")));
        }
        if hour > 23 || minute > 59 || second > 59 {
            return Err(CanonicalError::new(
                "instant-range",
                format!("time {hour:02}:{minute:02}:{second:02}"),
            ));
        }
        Ok(Self(text.to_owned()))
    }

    /// The canonical spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Instant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for Instant {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Instant {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(D::Error::custom)
    }
}

/// A lowercase identifier used for a target, route, connector, capture or
/// evidence entry.
///
/// The shape is `a-z0-9` separated by single hyphens. It has to be safe as a
/// path segment on every host the capture matrix runs on, which rules out
/// case-only distinctions: two records differing only in case collide on a
/// case-insensitive filesystem and one silently overwrites the other.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Slug(String);

impl Slug {
    /// The longest accepted identifier.
    pub const MAX_LEN: usize = 64;

    /// Parses the canonical form.
    ///
    /// # Errors
    ///
    /// Returns an error when the value is empty, longer than
    /// [`Slug::MAX_LEN`], contains a character outside `a-z0-9-`, or begins,
    /// ends or doubles a hyphen.
    pub fn parse(text: &str) -> Result<Self, CanonicalError> {
        if text.is_empty() || text.len() > Self::MAX_LEN {
            return Err(CanonicalError::new(
                "slug-length",
                format!("{} characters", text.len()),
            ));
        }
        let ok = text
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        if !ok || text.starts_with('-') || text.ends_with('-') || text.contains("--") {
            return Err(CanonicalError::new("slug-shape", text.to_owned()));
        }
        Ok(Self(text.to_owned()))
    }

    /// The canonical spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Slug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for Slug {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Slug {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(D::Error::custom)
    }
}

/// A version string exactly as the installed build reports it.
///
/// It is not parsed into components and it is not compared for order. The
/// installed executable is the authority on its own version, and imposing a
/// semantic version grammar on it would refuse builds that number themselves
/// some other way.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Version(String);

impl Version {
    /// The longest accepted version string.
    pub const MAX_LEN: usize = 96;

    /// Accepts a reported version string.
    ///
    /// # Errors
    ///
    /// Returns an error when the value is empty, longer than
    /// [`Version::MAX_LEN`], or carries whitespace or a non-printable byte.
    /// Surrounding whitespace is refused rather than trimmed: a version read
    /// with a stray newline is a parsing defect in the acquisition route, and
    /// trimming it here would hide that.
    pub fn parse(text: &str) -> Result<Self, CanonicalError> {
        if text.is_empty() || text.len() > Self::MAX_LEN {
            return Err(CanonicalError::new(
                "version-length",
                format!("{} characters", text.len()),
            ));
        }
        if !text
            .bytes()
            .all(|b| b.is_ascii_graphic() && b != b'"' && b != b'\\')
        {
            return Err(CanonicalError::new("version-shape", text.to_owned()));
        }
        Ok(Self(text.to_owned()))
    }

    /// The reported spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for Version {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(D::Error::custom)
    }
}

/// A path inside an evidence bundle, relative to the bundle root.
///
/// Absolute paths, drive letters, backslashes, `.` and `..` segments are all
/// refused. An evidence path is resolved against a directory a publisher
/// controls, and a traversal segment in a record is a write outside it.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RelPath(String);

impl RelPath {
    /// The longest accepted path.
    pub const MAX_LEN: usize = 200;

    /// Parses the canonical form.
    ///
    /// # Errors
    ///
    /// Returns an error for an empty or over-long path, a leading separator, a
    /// backslash, an empty, `.` or `..` segment, or a character outside
    /// `A-Za-z0-9._-` and the `/` separator.
    pub fn parse(text: &str) -> Result<Self, CanonicalError> {
        if text.is_empty() || text.len() > Self::MAX_LEN {
            return Err(CanonicalError::new(
                "path-length",
                format!("{} characters", text.len()),
            ));
        }
        if text.contains('\\') {
            return Err(CanonicalError::new("path-separator", text.to_owned()));
        }
        for segment in text.split('/') {
            if segment.is_empty() || segment == "." || segment == ".." {
                return Err(CanonicalError::new("path-segment", text.to_owned()));
            }
            let ok = segment
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_' || b == b'-');
            if !ok {
                return Err(CanonicalError::new("path-character", text.to_owned()));
            }
        }
        Ok(Self(text.to_owned()))
    }

    /// The canonical spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RelPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for RelPath {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for RelPath {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(D::Error::custom)
    }
}
