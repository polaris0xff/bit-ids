//! Shared contract for the `BitTorrent` identity catalogue.
//!
//! The crate deliberately exposes only the invariants established during the
//! repository bootstrap. Profile types, validation and embedded catalogue data
//! belong to the open `SCHEMA-*`, `CORPUS-*` and `LIB-*` work items.

/// Identifier carried by every first-generation profile.
pub const PROFILE_SCHEMA: &str = "bit-ids/profile/1";

/// A publishable capture is always a stable release.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseChannel {
    /// The vendor or upstream project declares this build stable.
    Stable,
}

/// Result of comparing overlapping observations from independent connectors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
