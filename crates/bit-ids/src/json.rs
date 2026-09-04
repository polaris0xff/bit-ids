//! The one way a profile record is read from, and written to, JSON.
//!
//! There is no route that produces a [`Profile`] from bytes without validating
//! it, and none that writes one out without validating it either. A second,
//! looser path is how an unproven record reaches a store that trusted the
//! first one.

use core::fmt;

use crate::PROFILE_SCHEMA;
use crate::record::{Profile, ProfileFields};
use crate::validate::{Violations, validate};

/// Why a record could not be read or written.
#[derive(Debug)]
pub enum ProfileError {
    /// The document declares a schema this build does not read. It is reported
    /// before any other parsing so that a later generation of the record shape
    /// produces this rather than a confusing complaint about a field.
    UnsupportedSchema {
        /// The identifier the document declared.
        found: String,
    },
    /// The document is not JSON, or not this schema's shape.
    Malformed(serde_json::Error),
    /// The document parsed and then refused an invariant.
    Invalid(Violations),
}

impl fmt::Display for ProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchema { found } => write!(
                f,
                "unsupported schema {found:?}, this build reads {PROFILE_SCHEMA:?}"
            ),
            Self::Malformed(error) => write!(f, "malformed record: {error}"),
            Self::Invalid(violations) => write!(f, "invalid record:\n{violations}"),
        }
    }
}

impl core::error::Error for ProfileError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::UnsupportedSchema { .. } => None,
            Self::Malformed(error) => Some(error),
            Self::Invalid(violations) => Some(violations),
        }
    }
}

/// Just enough of the document to learn which schema wrote it.
///
/// Unknown fields are ignored here on purpose. This read has one job, and a
/// document from a later generation must reach the version check rather than
/// failing on a field this build has never heard of.
#[derive(serde::Deserialize)]
struct VersionProbe {
    schema: String,
}

impl Profile {
    /// Reads and validates a record.
    ///
    /// # Errors
    ///
    /// Returns [`ProfileError::UnsupportedSchema`] when the document was
    /// written against another schema generation, [`ProfileError::Malformed`]
    /// when it is not this schema's shape, and [`ProfileError::Invalid`] when
    /// it parsed but refused an invariant.
    pub fn from_json(document: &str) -> Result<Self, ProfileError> {
        let probe: VersionProbe =
            serde_json::from_str(document).map_err(ProfileError::Malformed)?;
        if probe.schema != PROFILE_SCHEMA {
            return Err(ProfileError::UnsupportedSchema {
                found: probe.schema,
            });
        }
        // ⚠ Through the field mirror rather than through `Profile`, whose own
        // `Deserialize` validates and can only report a string. Going the long
        // way here is what lets a caller act on the codes.
        let fields: ProfileFields =
            serde_json::from_str(document).map_err(ProfileError::Malformed)?;
        let profile = Self::from(fields);
        validate(&profile).map_err(ProfileError::Invalid)?;
        Ok(profile)
    }

    /// Writes a record in the canonical published form.
    ///
    /// The output is two-space indented, key order is the declaration order of
    /// the schema, and it ends with a newline. Two assemblies of one record
    /// produce identical bytes, which is what makes a rebuilt publication tree
    /// comparable to the one already on the data branch.
    ///
    /// # Errors
    ///
    /// Returns [`ProfileError::Invalid`] with the refused invariants: an
    /// invalid record has no canonical form, so it is not written. A
    /// [`ProfileError::Malformed`] here would mean a `Serialize` implementation
    /// in this crate refused a value it built.
    pub fn to_json(&self) -> Result<String, ProfileError> {
        validate(self).map_err(ProfileError::Invalid)?;
        let mut out = serde_json::to_string_pretty(self).map_err(ProfileError::Malformed)?;
        out.push('\n');
        Ok(out)
    }
}
