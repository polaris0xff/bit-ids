//! What two connectors each saw, and whether that amounts to agreement.
//!
//! Two independent observers can differ for reasons that are not the same
//! thing: one parser is wrong, a timing effect moved the bytes, or the build
//! genuinely varies. Choosing one of them silently corrupts the corpus, so the
//! model here keeps both observations, records which comparison was made, and
//! refuses to call anything agreement that was not compared.
//!
//! ⛔ **The trap this exists to close is a field only one connector could see.**
//! Calling that agreement is the easiest mistake available: nothing disagreed.
//! A connector that cannot observe a surface says so, in
//! [`SeenValue::OutOfScope`], and a field with fewer than two observers left is
//! `not_corroborated` however well the one observation reads.

use serde::{Deserialize, Serialize};

use crate::Agreement;
use crate::canonical::{HexBytes, Instant, Label, Slug};
use crate::record::Profile;
use crate::validate::{SchemaError, Violations};

/// What one connector saw for one field.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum SeenValue {
    /// The connector observed these bytes, after its projection.
    Bytes(HexBytes),
    /// The connector created the condition and the build produced nothing. An
    /// absence two connectors both saw is a corroborated absence.
    Absent,
    /// The connector cannot observe this surface at all. ⛔ Not a value and not
    /// an absence: it is the reason this connector's silence proves nothing.
    OutOfScope,
}

impl SeenValue {
    /// Whether this connector was in a position to see the field.
    ///
    /// Only in-scope observations can agree or disagree. Counting an
    /// out-of-scope one would turn a single observation into a corroborated
    /// pair, which is exactly the false agreement this model refuses.
    #[must_use]
    pub const fn in_scope(&self) -> bool {
        !matches!(self, Self::OutOfScope)
    }

    /// The canonical spelling of the kind.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Bytes(_) => "bytes",
            Self::Absent => "absent",
            Self::OutOfScope => "out_of_scope",
        }
    }
}

/// What was done to a raw observation before it was compared.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum Projection {
    /// Nothing. The bytes were compared as they arrived.
    Raw,
    /// A declared normalization was applied first.
    Normalized(Slug),
}

impl Projection {
    /// The normalization this projection applies, if any.
    #[must_use]
    pub const fn normalization(&self) -> Option<&Slug> {
        match self {
            Self::Raw => None,
            Self::Normalized(id) => Some(id),
        }
    }
}

/// A named transformation applied before comparing two observations.
///
/// ⛔ A normalization cannot discard order or unknown bytes merely to obtain
/// agreement. Both properties are declared rather than assumed, so that using
/// one is an assertion somebody made and a check can refuse.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Normalization {
    /// Identifier, cited by a [`Projection`].
    pub id: Slug,
    /// What it does, in one line.
    pub summary: Label,
    /// Whether the order of what it transforms survives it.
    pub preserves_order: bool,
    /// Whether bytes it does not understand survive it.
    pub preserves_unknown_bytes: bool,
}

impl Normalization {
    /// Whether this normalization may be used to reach agreement.
    #[must_use]
    pub const fn is_usable(&self) -> bool {
        self.preserves_order && self.preserves_unknown_bytes
    }
}

/// One connector's observation of one field.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectorObservation {
    /// Which connector.
    pub connector: Slug,
    /// The artifact the value was read out of. Kept so the comparison can be
    /// redone from bytes rather than trusted.
    pub evidence: Slug,
    /// What was applied before comparing.
    pub projection: Projection,
    /// What this connector saw.
    pub seen: SeenValue,
}

/// Why a disagreement was settled the way it was.
///
/// The four are the causes `SCHEMA-03` names: a parser can be wrong, timing can
/// move bytes, a build can genuinely vary, and an observer that terminates a
/// protocol can change what the target offers.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdjudicationReason {
    /// One connector parsed the bytes wrongly.
    ParserDefect,
    /// The difference was an artefact of when the observation happened.
    TimingEffect,
    /// The build genuinely emits different values.
    ClientVariability,
    /// Observing the surface changed what the target offered.
    ObserverPerturbation,
}

/// The record that settles a correction.
///
/// ⛔ Required on any record that supersedes another, and forbidden on an
/// original. A correction with no reason is a record that changed its mind
/// without saying why, and the append-only store keeps both forever.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Adjudication {
    /// When it was settled, UTC.
    pub decided_at: Instant,
    /// What kind of cause.
    pub reason: AdjudicationReason,
    /// What was decided, in one line.
    pub summary: Label,
    /// The evidence that settled it, cited into the record's evidence list.
    pub evidence: Vec<Slug>,
}

/// The outcome of comparing what the connectors saw for one field.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldCorroboration {
    /// The field this outcome is about.
    pub path: crate::observation::FieldPath,
    /// What each connector saw, sorted by connector.
    pub observations: Vec<ConnectorObservation>,
    /// The outcome of comparing the in-scope observations.
    pub agreement: Agreement,
    /// What differs, required when the outcome is a disagreement and forbidden
    /// otherwise. A conflict nobody described is a conflict nobody can act on.
    pub conflict: Option<Label>,
}

impl FieldCorroboration {
    /// The observations that could see the field.
    pub fn in_scope(&self) -> impl Iterator<Item = &ConnectorObservation> {
        self.observations
            .iter()
            .filter(|observation| observation.seen.in_scope())
    }

    /// How many connectors were in a position to see the field.
    #[must_use]
    pub fn overlap(&self) -> usize {
        self.in_scope().count()
    }

    /// Whether the in-scope observations all saw the same thing.
    ///
    /// Comparison is on the projected value, which is what a projection is for.
    /// An out-of-scope observation takes no part: it is not equal to anything
    /// and not unequal to anything.
    #[must_use]
    pub fn in_scope_values_match(&self) -> bool {
        let mut seen = self.in_scope().map(|observation| &observation.seen);
        let Some(first) = seen.next() else {
            return false;
        };
        seen.all(|value| value == first)
    }

    /// Whether any observation was compared through a normalization.
    #[must_use]
    pub fn uses_normalization(&self) -> bool {
        self.observations
            .iter()
            .any(|observation| observation.projection.normalization().is_some())
    }
}

// -- publishability ---------------------------------------------------------

/// Whether a record may be published as a corroborated measurement.
///
/// ⛔ This is a different question from whether the record is valid, and both
/// have to exist. A disagreement must be *recordable*, or the project has no
/// way to keep the evidence of one; `docs/architecture.md` section 8 says such
/// a run moves to `provisional` with its evidence retained. So
/// [`crate::validate`] accepts a record carrying a conflict, and this refuses
/// to publish it.
///
/// Two things block publication:
///
/// - a field whose independent observations disagree, which is the absolute in
///   `docs/AGENTS.md`: they agree or the profile stays unpublished with the
///   disagreement recorded;
/// - a field that asserts a measurement no second connector could see, which
///   `docs/capture-methodology.md` calls provisional until a second route
///   exists. A record carrying a provisional field is itself provisional, and
///   the state machine does not skip from there to published.
///
/// # Errors
///
/// Returns every field that blocks publication, each with a stable code.
pub fn publishable(profile: &Profile) -> Result<(), Violations> {
    let mut out = Vec::new();
    for entry in &profile.corroboration {
        let at = format!("corroboration {}", entry.path);
        if entry.agreement == Agreement::Disagrees {
            out.push(SchemaError::new(
                "E-PUB-01",
                &at,
                "the connectors disagree, so this record stays provisional with the \
                 disagreement recorded",
            ));
        }
        let measured = profile
            .field(&entry.path)
            .is_some_and(|field| field.state.asserts_a_measurement());
        if measured && entry.agreement == Agreement::NotCorroborated {
            out.push(SchemaError::new(
                "E-PUB-02",
                &at,
                format!(
                    "asserts a measurement that {} connector(s) could see; it stays provisional \
                     until a second one can",
                    entry.overlap()
                ),
            ));
        }
    }
    Violations::from_errors(out)
}
