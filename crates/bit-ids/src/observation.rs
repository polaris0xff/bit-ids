//! What one identity field of one build was measured to be.
//!
//! The states are deliberately not collapsible. `unknown` says nobody looked,
//! `not_observed` says the observer created the condition and the build emitted
//! nothing, and `not_supported` says the build cannot expose the surface at
//! all. Folding the three into a null would publish "we did not look" and "it
//! does not do this" as the same fact, and only one of those is a measurement.

use core::fmt;
use core::num::NonZeroU32;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::canonical::{CanonicalError, HexBytes, Slug};

/// A protocol surface a field can be observed on.
///
/// The set is closed on purpose. [`crate::PROFILE_SCHEMA`] names the record
/// shape, so a surface nobody has modelled is a schema change and a version
/// bump, not a free-text value a reader has to guess at.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Surface {
    /// The HTTP tracker request line and header block.
    TrackerHttp,
    /// The UDP tracker datagram exchange.
    TrackerUdp,
    /// The peer handshake and the bounded initial message transcript.
    PeerWire,
    /// The distributed hash table.
    Dht,
    /// Peer exchange.
    Pex,
    /// BEP 14 local service discovery.
    ///
    /// ⚠ Added by `OBS-06`, which is the entry that modelled it. Nothing had
    /// ever been published when it was added, so no consumer held a record
    /// whose vocabulary this widens; a later addition is not free.
    LocalDiscovery,
    /// Message stream encryption.
    Mse,
    /// HTTP and HTTPS web seeding.
    WebSeed,
}

impl Surface {
    /// The canonical spelling used in a [`FieldPath`].
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TrackerHttp => "tracker_http",
            Self::TrackerUdp => "tracker_udp",
            Self::PeerWire => "peer_wire",
            Self::Dht => "dht",
            Self::Pex => "pex",
            Self::LocalDiscovery => "local_discovery",
            Self::Mse => "mse",
            Self::WebSeed => "web_seed",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "tracker_http" => Self::TrackerHttp,
            "tracker_udp" => Self::TrackerUdp,
            "peer_wire" => Self::PeerWire,
            "dht" => Self::Dht,
            "pex" => Self::Pex,
            "local_discovery" => Self::LocalDiscovery,
            "mse" => Self::Mse,
            "web_seed" => Self::WebSeed,
            _ => return None,
        })
    }
}

impl fmt::Display for Surface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Byte widths the protocol fixes, by field path.
///
/// A constant claiming some other width is a parser that read the wrong span,
/// which is exactly the defect that reaches the corpus looking plausible. BEP 3
/// fixes the peer ID at 20 bytes and the reserved block at 8.
const FIXED_WIDTHS: &[(&str, usize)] = &[
    ("peer_wire/peer_id", 20),
    ("peer_wire/reserved", 8),
    ("tracker_http/peer_id", 20),
    ("tracker_udp/peer_id", 20),
];

/// The address of one identity field, written as `surface/name`.
///
/// The name is dotted lower snake case so a nested protocol value keeps its
/// structure in one sortable token: `peer_wire/bep10.client` is one field, not
/// a path into a document a reader has to assemble.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct FieldPath {
    surface: Surface,
    name: String,
}

impl FieldPath {
    /// The longest accepted field name, excluding the surface.
    pub const MAX_NAME_LEN: usize = 96;

    /// Parses the canonical `surface/name` form.
    ///
    /// # Errors
    ///
    /// Returns an error when the surface is not one of [`Surface`], the name is
    /// empty or over-long, or a dotted segment is not lower snake case starting
    /// with a letter.
    pub fn parse(text: &str) -> Result<Self, CanonicalError> {
        let Some((surface, name)) = text.split_once('/') else {
            return Err(CanonicalError::new(
                "field-path",
                format!("expected surface/name: {text}"),
            ));
        };
        let Some(surface) = Surface::parse(surface) else {
            return Err(CanonicalError::new(
                "field-path",
                format!("unknown surface: {text}"),
            ));
        };
        if name.is_empty() || name.len() > Self::MAX_NAME_LEN {
            return Err(CanonicalError::new(
                "field-path",
                format!("name length: {text}"),
            ));
        }
        for segment in name.split('.') {
            let mut bytes = segment.bytes();
            let starts = bytes.next().is_some_and(|b| b.is_ascii_lowercase());
            let rest = bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
            if !starts || !rest || segment.ends_with('_') || segment.contains("__") {
                return Err(CanonicalError::new(
                    "field-path",
                    format!("name segment: {text}"),
                ));
            }
        }
        Ok(Self {
            surface,
            name: name.to_owned(),
        })
    }

    /// The surface this field was observed on.
    #[must_use]
    pub const fn surface(&self) -> Surface {
        self.surface
    }

    /// The field name within the surface.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The width the protocol fixes for this field, when it fixes one.
    #[must_use]
    pub fn fixed_width(&self) -> Option<usize> {
        let key = self.to_string();
        FIXED_WIDTHS
            .iter()
            .find(|(path, _)| *path == key)
            .map(|(_, width)| *width)
    }
}

impl fmt::Display for FieldPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.surface.as_str(), self.name)
    }
}

impl Serialize for FieldPath {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for FieldPath {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(D::Error::custom)
    }
}

/// One span of a [`BytePattern`].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum PatternRun {
    /// Every sample carried these bytes at this offset.
    Fixed {
        /// The bytes, identical in every sample.
        bytes: HexBytes,
    },
    /// Samples differed over this span.
    Varying {
        /// How many bytes the span covers.
        length: usize,
        /// The distinct byte values observed in the span, ascending, or `null`
        /// when the observer did not constrain them. An alphabet is a claim
        /// about what the build can emit and it needs its own sampling
        /// argument, so it is optional rather than inferred from a few runs.
        alphabet: Option<HexBytes>,
    },
}

impl PatternRun {
    /// How many bytes this run covers.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Fixed { bytes } => bytes.len(),
            Self::Varying { length, .. } => *length,
        }
    }

    /// Whether the run covers no bytes at all, which validation refuses.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A value with a fixed part and a part that changed between samples.
///
/// This is the shape a peer ID normally has: a stable client prefix followed by
/// bytes the build regenerates. Recording it as one opaque "varies" would throw
/// away the prefix, which is the identifying half.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BytePattern {
    /// The total value width in bytes, identical in every sample.
    pub length: usize,
    /// The spans, in offset order, tiling `0..length` exactly.
    pub runs: Vec<PatternRun>,
}

/// A value observed to be the same in every sample.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConstantValue {
    /// The observed bytes.
    pub value: HexBytes,
    /// How many separately initialized samples produced them.
    pub samples: NonZeroU32,
}

/// A value observed to have a fixed part and a varying part.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatternedValue {
    /// The tiling of fixed and varying spans.
    pub pattern: BytePattern,
    /// How many separately initialized samples produced it.
    pub samples: NonZeroU32,
}

/// A value that changed between samples with no fixed span to report.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VariableValue {
    /// The common width in bytes, or `null` when the width itself varied.
    pub length: Option<usize>,
    /// How many separately initialized samples were taken.
    pub samples: NonZeroU32,
    /// How many distinct values those samples produced.
    pub distinct: NonZeroU32,
}

/// What one field was measured to be.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum FieldState {
    /// Nobody has measured this field yet. It carries no value and no evidence,
    /// and it is the one state that asserts nothing about the build.
    Unknown,
    /// The observer created the condition and the build emitted nothing. It
    /// needs a positive control proving the observer would have seen it.
    NotObserved,
    /// The build cannot expose this surface. It needs the same positive
    /// control: an observer that cannot see a surface reports it identically.
    NotSupported,
    /// Every sample carried the same bytes.
    Constant(ConstantValue),
    /// Samples shared a fixed part and differed over a described span.
    Patterned(PatternedValue),
    /// Samples differed with no fixed span to report.
    Variable(VariableValue),
}

impl FieldState {
    /// Whether this state makes a claim about the build.
    ///
    /// Every state except `unknown` does, and every state that does needs
    /// evidence behind it. That is the whole of the unproven-field rule.
    #[must_use]
    pub const fn asserts_a_measurement(&self) -> bool {
        !matches!(self, Self::Unknown)
    }

    /// Whether this state claims the build did not produce the field.
    ///
    /// Absence is only publishable behind a positive control, per the observer
    /// contract in `docs/architecture.md` section 5.
    #[must_use]
    pub const fn claims_absence(&self) -> bool {
        matches!(self, Self::NotObserved | Self::NotSupported)
    }

    /// How many separately initialized samples the state rests on.
    #[must_use]
    pub const fn samples(&self) -> Option<NonZeroU32> {
        match self {
            Self::Constant(value) => Some(value.samples),
            Self::Patterned(value) => Some(value.samples),
            Self::Variable(value) => Some(value.samples),
            _ => None,
        }
    }

    /// Whether the state claims the value changes between samples.
    #[must_use]
    pub const fn claims_variation(&self) -> bool {
        matches!(self, Self::Patterned(_) | Self::Variable(_))
    }

    /// The canonical spelling of the state name.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::NotObserved => "not_observed",
            Self::NotSupported => "not_supported",
            Self::Constant(_) => "constant",
            Self::Patterned(_) => "patterned",
            Self::Variable(_) => "variable",
        }
    }
}

/// One identity field of one build, with the evidence that proves it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedField {
    /// Which field this is.
    pub path: FieldPath,
    /// What it was measured to be.
    pub state: FieldState,
    /// Evidence entry identifiers, resolving into the record's evidence list.
    pub evidence: Vec<Slug>,
}
