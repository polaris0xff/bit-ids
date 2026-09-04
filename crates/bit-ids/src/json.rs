//! The one way each published document is read from, and written to, JSON.
//!
//! There is no route that produces a [`Profile`] or a [`RunManifest`] from
//! bytes without validating it, and none that writes one out without
//! validating it either. A second, looser path is how an unproven record
//! reaches a store that trusted the first one.

use core::fmt;

use crate::PROFILE_SCHEMA;
use crate::manifest::{MANIFEST_SCHEMA, RunManifest, RunManifestFields, validate_manifest};
use crate::record::{Profile, ProfileFields};
use crate::validate::{Violations, validate};

/// Why a document could not be read or written.
///
/// One type for both documents, because the three ways a document can be
/// refused are the same for each and a second copy of them would drift.
#[derive(Debug)]
pub enum DocumentError {
    /// The document declares a schema this build does not read. It is reported
    /// before any other parsing so that a later generation of the record shape
    /// produces this rather than a confusing complaint about a field.
    UnsupportedSchema {
        /// The identifier the document declared.
        found: String,
        /// The identifier this build reads for that kind of document. ⚠ Carried
        /// rather than assumed: the profile and the manifest version
        /// independently, and a message naming the other one sends a reader to
        /// the wrong file.
        expected: &'static str,
    },
    /// The document is not JSON, or not this schema's shape.
    Malformed(serde_json::Error),
    /// The document parsed and then refused an invariant.
    Invalid(Violations),
}

impl fmt::Display for DocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchema { found, expected } => write!(
                f,
                "unsupported schema {found:?}, this build reads {expected:?}"
            ),
            Self::Malformed(error) => write!(f, "malformed document: {error}"),
            Self::Invalid(violations) => write!(f, "invalid document:\n{violations}"),
        }
    }
}

impl core::error::Error for DocumentError {
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
    /// Returns [`DocumentError::UnsupportedSchema`] when the document was
    /// written against another schema generation, [`DocumentError::Malformed`]
    /// when it is not this schema's shape, and [`DocumentError::Invalid`] when
    /// it parsed but refused an invariant.
    pub fn from_json(document: &str) -> Result<Self, DocumentError> {
        let probe: VersionProbe =
            serde_json::from_str(document).map_err(DocumentError::Malformed)?;
        if probe.schema != PROFILE_SCHEMA {
            return Err(DocumentError::UnsupportedSchema {
                found: probe.schema,
                expected: PROFILE_SCHEMA,
            });
        }
        // ⚠ Through the field mirror rather than through `Profile`, whose own
        // `Deserialize` validates and can only report a string. Going the long
        // way here is what lets a caller act on the codes.
        let fields: ProfileFields =
            serde_json::from_str(document).map_err(DocumentError::Malformed)?;
        let profile = Self::from(fields);
        validate(&profile).map_err(DocumentError::Invalid)?;
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
    /// Returns [`DocumentError::Invalid`] with the refused invariants: an
    /// invalid record has no canonical form, so it is not written. A
    /// [`DocumentError::Malformed`] here would mean a `Serialize` implementation
    /// in this crate refused a value it built.
    pub fn to_json(&self) -> Result<String, DocumentError> {
        validate(self).map_err(DocumentError::Invalid)?;
        let mut out = serde_json::to_string_pretty(self).map_err(DocumentError::Malformed)?;
        out.push('\n');
        Ok(out)
    }
}

/// Just enough of a manifest document to learn which schema wrote it.
#[derive(serde::Deserialize)]
struct ManifestVersionProbe {
    schema: String,
}

impl RunManifest {
    /// Reads and validates a run manifest.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError::UnsupportedSchema`] when the document was
    /// written against another manifest generation,
    /// [`DocumentError::Malformed`] when it is not this schema's shape, and
    /// [`DocumentError::Invalid`] when it parsed but refused an invariant.
    pub fn from_json(document: &str) -> Result<Self, DocumentError> {
        let probe: ManifestVersionProbe =
            serde_json::from_str(document).map_err(DocumentError::Malformed)?;
        if probe.schema != MANIFEST_SCHEMA {
            return Err(DocumentError::UnsupportedSchema {
                found: probe.schema,
                expected: MANIFEST_SCHEMA,
            });
        }
        // Through the field mirror, so a caller gets the codes rather than a
        // string. Same reason as the profile above.
        let fields: RunManifestFields =
            serde_json::from_str(document).map_err(DocumentError::Malformed)?;
        let manifest = Self::from(fields);
        validate_manifest(&manifest).map_err(DocumentError::Invalid)?;
        Ok(manifest)
    }

    /// Writes a run manifest in the canonical published form.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError::Invalid`] with the refused invariants. An
    /// invalid manifest has no canonical form, so it is not written.
    pub fn to_json(&self) -> Result<String, DocumentError> {
        validate_manifest(self).map_err(DocumentError::Invalid)?;
        let mut out = serde_json::to_string_pretty(self).map_err(DocumentError::Malformed)?;
        out.push('\n');
        Ok(out)
    }
}
