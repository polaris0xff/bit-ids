//! The schema version and the record identifier.
//!
//! Both exist so that a stored record can say what it is without a reader
//! guessing from its filename. A published path can be renamed, mirrored or
//! rewritten by a consumer; the bytes inside the record cannot.

use core::fmt;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::PROFILE_SCHEMA;
use crate::canonical::{CanonicalError, Sha256Digest, Slug, Version};

/// The schema identifier a record declares.
///
/// Only [`PROFILE_SCHEMA`] parses. A later generation of the record shape gets
/// its own identifier and its own type, and this one keeps reading exactly the
/// records it was written for.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SchemaVersion(&'static str);

impl SchemaVersion {
    /// The schema this build of the crate reads and writes.
    #[must_use]
    pub const fn current() -> Self {
        Self(PROFILE_SCHEMA)
    }

    /// Parses a declared schema identifier.
    ///
    /// # Errors
    ///
    /// Returns an error for any identifier other than [`PROFILE_SCHEMA`].
    pub fn parse(text: &str) -> Result<Self, CanonicalError> {
        if text == PROFILE_SCHEMA {
            Ok(Self(PROFILE_SCHEMA))
        } else {
            Err(CanonicalError::new(
                "schema-version",
                format!("unsupported schema {text:?}, this build reads {PROFILE_SCHEMA:?}"),
            ))
        }
    }

    /// The declared identifier.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

impl fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl Serialize for SchemaVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0)
    }
}

impl<'de> Deserialize<'de> for SchemaVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(D::Error::custom)
    }
}

/// The tuple that decides which record a measurement is.
///
/// Two runs with the same tuple are the same record and one of them is a
/// duplicate. Two runs differing anywhere in it are different records, and the
/// store keeps both.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecordKey<'a> {
    /// The schema the record is written against.
    pub schema: &'a SchemaVersion,
    /// The catalogue target identifier.
    pub target: &'a Slug,
    /// The version the installed build reported.
    pub version: &'a Version,
    /// The host family.
    pub platform: &'a Slug,
    /// The machine architecture.
    pub arch: &'a Slug,
    /// The package format.
    pub package: &'a Slug,
    /// The capture run identifier.
    pub capture: &'a Slug,
}

impl RecordKey<'_> {
    /// Domain separator, so this digest can never collide with a digest of the
    /// same bytes taken for another purpose.
    const DOMAIN: &'static str = "bit-ids/record-id/1";

    /// The exact bytes [`RecordId::derive`] hashes.
    ///
    /// Every component is length-prefixed. Joining them with a separator
    /// character would let two different tuples encode to one string the moment
    /// a component contained the separator, and a record identifier that can
    /// collide is a store that silently overwrites a measurement.
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let parts = [
            Self::DOMAIN,
            self.schema.as_str(),
            self.target.as_str(),
            self.version.as_str(),
            self.platform.as_str(),
            self.arch.as_str(),
            self.package.as_str(),
            self.capture.as_str(),
        ];
        let mut out = Vec::new();
        for part in parts {
            let len = u32::try_from(part.len()).unwrap_or(u32::MAX);
            out.extend_from_slice(&len.to_be_bytes());
            out.extend_from_slice(part.as_bytes());
        }
        out
    }
}

/// A record's deterministic identifier.
///
/// It is derived from [`RecordKey`], not stored independently of it, so a
/// record whose identifier disagrees with its own contents is refused rather
/// than filed under a name that describes a different measurement.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RecordId(Sha256Digest);

impl RecordId {
    /// The prefix that distinguishes a record identifier from a content digest.
    /// Both are SHA-256; only one of them digests a file.
    pub const PREFIX: &'static str = "record:";

    /// Derives the identifier for an identity tuple.
    #[must_use]
    pub fn derive(key: &RecordKey<'_>) -> Self {
        Self(Sha256Digest::of(&key.canonical_bytes()))
    }

    /// Parses the canonical `record:sha256:<64 lowercase hex>` form.
    ///
    /// # Errors
    ///
    /// Returns an error when the record prefix is missing or the remainder is
    /// not a canonical SHA-256 digest.
    pub fn parse(text: &str) -> Result<Self, CanonicalError> {
        let Some(rest) = text.strip_prefix(Self::PREFIX) else {
            return Err(CanonicalError::new(
                "record-id",
                format!("expected the {} prefix", Self::PREFIX),
            ));
        };
        Ok(Self(Sha256Digest::parse(rest)?))
    }
}

impl fmt::Display for RecordId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", Self::PREFIX, self.0)
    }
}

impl Serialize for RecordId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for RecordId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(D::Error::custom)
    }
}
