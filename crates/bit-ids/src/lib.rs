//! Shared contract for the `BitTorrent` identity catalogue.
//!
//! The crate owns the published record shape and the rules a record must
//! satisfy before it can be published. It does not acquire, install, launch or
//! observe anything; `docs/architecture.md` section 3 says which component
//! owns each of those.
//!
//! # Reading a record
//!
//! ```
//! # let document = include_str!("../tests/fixtures/valid-profile.json");
//! let profile = bit_ids::Profile::from_json(document).expect("a published record validates");
//! assert_eq!(profile.schema.as_str(), bit_ids::PROFILE_SCHEMA);
//! ```
//!
//! No route from bytes to a [`Profile`] skips validation. [`Profile::from_json`]
//! is the one to prefer, because it answers the schema version first and
//! returns the refused invariants with their codes; `Profile` does not derive
//! `Deserialize`, so reaching for serde directly validates as well.
//! [`Profile::to_json`] validates too, which means an unproven record has no
//! canonical form to be stored under.
//!
//! Building a `Profile` in memory is deliberately open: a capture tool
//! assembles one field at a time and a test needs to plant a defect in one.
//! The write path is where that is caught.

pub mod agreement;
pub mod canonical;
pub mod identity;
mod json;
pub mod manifest;
pub mod observation;
pub mod record;
pub mod sampling;
pub mod validate;

pub use agreement::publishable;
pub use json::DocumentError;
pub use manifest::{MANIFEST_SCHEMA, RunManifest, bind, validate_manifest};
pub use record::Profile;
pub use validate::{SchemaError, Violations, validate};

use serde::{Deserialize, Serialize};

/// Identifier carried by every first-generation profile.
pub const PROFILE_SCHEMA: &str = "bit-ids/profile/1";

/// A publishable capture is always a stable release.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseChannel {
    /// The vendor or upstream project declares this build stable.
    Stable,
}

/// Result of comparing overlapping observations from independent connectors.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Agreement {
    /// The independently parsed values are byte-for-byte equal.
    Exact,
    /// A documented normalization makes the values equal.
    Normalized,
    /// The observations differ and the profile is not publishable.
    Disagrees,
    /// Only one connector could observe the field; it remains provisional.
    NotCorroborated,
}

/// A record may be published only when the overlapping observations agree.
#[must_use]
pub const fn is_publishable(agreement: Agreement) -> bool {
    matches!(agreement, Agreement::Exact | Agreement::Normalized)
}

#[cfg(test)]
mod tests {
    use super::{Agreement, PROFILE_SCHEMA, is_publishable};

    #[test]
    fn schema_identifier_is_versioned() {
        assert_eq!(PROFILE_SCHEMA, "bit-ids/profile/1");
    }

    #[test]
    fn disagreement_and_missing_corroboration_are_not_publishable() {
        assert!(is_publishable(Agreement::Exact));
        assert!(is_publishable(Agreement::Normalized));
        assert!(!is_publishable(Agreement::Disagrees));
        assert!(!is_publishable(Agreement::NotCorroborated));
    }
}
