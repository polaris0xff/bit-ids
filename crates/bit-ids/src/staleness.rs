//! What a new stable release creates, and what it must not create twice.
//!
//! `CI-02` owns this. A resolver says what the newest stable release of a target
//! is; the published views say what has been measured. This is the comparison
//! between them, and its answer is the capture work outstanding.
//!
//! ⛔ **The comparison is against the published views, not against the store.**
//! `docs/architecture.md` section 4 makes an index a derived file a consumer
//! reads *instead of* the records, and a monitor is a consumer. A record sitting
//! in the store and in no view is a measurement nobody can look up, so counting
//! it as coverage would close work a consumer cannot see was done. The corollary
//! is worth stating: `CORPUS-04` drops a superseded record from every view, so a
//! retracted measurement re-opens its capture here with nothing added.
//!
//! ⛔ **A request identifier is derived from its key and never allocated.** That
//! is the whole of "no duplicate after repeated runs": two runs over the same
//! facts derive the same identifier, so a tracker keyed on it cannot hold two.
//! A counter, a timestamp or a random token would each make a second run's
//! request a different request, which is the defect this entry names.
//!
//! ⚠ **Work is bounded per build line rather than per record.** A request says
//! capture target T at version V on platform P. Architecture and package are
//! outcomes of the acquisition and are not known when the request is opened, so
//! they are not in the key; a request that carried them would multiply one
//! release into a request per packaging, most of which no route can satisfy.
//!
//! ⭐ **Nothing here judges stability.** A preview never reaches a request
//! because [`Resolution`] already refused it, and the survey takes the whole
//! resolution rather than a version so that it cannot be handed one that came
//! from somewhere else. A second stability rule here would be a second place for
//! the answer to be different.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::ReleaseChannel;
use crate::canonical::{CanonicalError, Instant, Sha256Digest, Slug, Version};
use crate::index::LatestRow;
use crate::json::DocumentError;
use crate::resolution::{Resolution, VersionScheme};
use crate::validate::{SchemaError, Violations, strictly_ascending};

/// Identifier carried by every first-generation survey.
pub const REQUEST_SCHEMA: &str = "bit-ids/capture-requests/1";

/// The schema identifier a survey declares.
///
/// A type rather than a validated `String`, the way [`crate::resolution`] and
/// [`crate::manifest`] already do it, so a wrong identifier is unrepresentable
/// rather than refused.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RequestSchema(&'static str);

impl RequestSchema {
    /// The schema this build reads and writes.
    #[must_use]
    pub const fn current() -> Self {
        Self(REQUEST_SCHEMA)
    }

    /// Parses a declared schema identifier.
    ///
    /// # Errors
    ///
    /// Returns an error for any identifier other than [`REQUEST_SCHEMA`].
    pub fn parse(text: &str) -> Result<Self, CanonicalError> {
        if text == REQUEST_SCHEMA {
            Ok(Self(REQUEST_SCHEMA))
        } else {
            Err(CanonicalError::new(
                "request-schema-version",
                format!("unsupported schema {text:?}, this build reads {REQUEST_SCHEMA:?}"),
            ))
        }
    }

    /// The declared identifier.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

impl Serialize for RequestSchema {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0)
    }
}

impl<'de> Deserialize<'de> for RequestSchema {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(D::Error::custom)
    }
}

/// The tuple that decides which request a piece of work is.
///
/// Target, version, channel and platform, and nothing else. Two runs agreeing
/// on all four are asking for one capture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RequestKey<'a> {
    /// The catalogue target.
    pub target: &'a Slug,
    /// The version the resolver selected.
    pub version: &'a Version,
    /// The release channel. Only stable is published, and only stable is asked
    /// for; the component is in the key so that a later channel cannot silently
    /// collide with today's requests.
    pub channel: ReleaseChannel,
    /// The host family the capture is wanted on.
    pub platform: &'a Slug,
}

impl RequestKey<'_> {
    /// Domain separator, so this digest can never collide with a digest of the
    /// same bytes taken for another purpose.
    const DOMAIN: &'static str = "bit-ids/capture-request/1";

    /// The exact bytes [`RequestId::derive`] hashes.
    ///
    /// Length-prefixed per component, for [`crate::identity::RecordKey`]'s
    /// reason: joining with a separator lets two different tuples encode to one
    /// string the moment a component contains the separator, and an identifier
    /// that can collide is a tracker that silently merges two pieces of work.
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let parts = [
            Self::DOMAIN,
            REQUEST_SCHEMA,
            self.target.as_str(),
            self.version.as_str(),
            self.channel.as_str(),
            self.platform.as_str(),
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

/// A capture request's deterministic identifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RequestId(Sha256Digest);

impl RequestId {
    /// The prefix that distinguishes a request identifier from a content digest
    /// and from a record identifier. All three are SHA-256; only one digests a
    /// file and only one names a measurement.
    pub const PREFIX: &'static str = "request:";

    /// Derives the identifier for a request key.
    #[must_use]
    pub fn derive(key: &RequestKey<'_>) -> Self {
        Self(Sha256Digest::of(&key.canonical_bytes()))
    }

    /// Parses the canonical `request:sha256:<64 lowercase hex>` form.
    ///
    /// # Errors
    ///
    /// Returns an error when the prefix is missing or the remainder is not a
    /// canonical SHA-256 digest.
    pub fn parse(text: &str) -> Result<Self, CanonicalError> {
        let Some(rest) = text.strip_prefix(Self::PREFIX) else {
            return Err(CanonicalError::new(
                "request-id",
                format!("expected the {} prefix", Self::PREFIX),
            ));
        };
        Ok(Self(Sha256Digest::parse(rest)?))
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", Self::PREFIX, self.0)
    }
}

impl Serialize for RequestId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for RequestId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(D::Error::custom)
    }
}

/// What a survey decided about one target on one platform.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Staleness {
    /// Nothing is measured on this platform at all. Opens the first request.
    Uncaptured,
    /// A measurement exists and the resolver selected something newer. Opens a
    /// request.
    Stale,
    /// The newest measurement is the selected release. Nothing to do.
    Current,
    /// ⛔ The newest measurement is **newer** than the selection. Opens nothing:
    /// a request here would ask a runner to capture a downgrade, and the fault
    /// is upstream of this comparison. A source that dropped a release and a
    /// resolver reading the wrong source both land here.
    Regressed,
    /// The measured and selected versions compare equal under the scheme and
    /// are spelled differently. Opens nothing, for [`crate::resolution`]'s
    /// reason: neither answer is safe, so neither is given.
    Ambiguous,
    /// The resolver failed closed, so there is no selection to compare against.
    /// Opens nothing, and is reported rather than skipped: a target whose
    /// resolution has been blocked for a month otherwise looks like a target
    /// with no work.
    Unresolved,
    /// A measured version this target's own scheme cannot order. Opens nothing,
    /// because nothing can rule out that it is newer than the selection.
    Unorderable,
}

impl Staleness {
    /// Every variant, so a test can hold the two spellings in step.
    pub const ALL: &'static [Self] = &[
        Self::Uncaptured,
        Self::Stale,
        Self::Current,
        Self::Regressed,
        Self::Ambiguous,
        Self::Unresolved,
        Self::Unorderable,
    ];

    /// The canonical spelling, for a message a person reads.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Uncaptured => "uncaptured",
            Self::Stale => "stale",
            Self::Current => "current",
            Self::Regressed => "regressed",
            Self::Ambiguous => "ambiguous",
            Self::Unresolved => "unresolved",
            Self::Unorderable => "unorderable",
        }
    }

    /// Whether this verdict is one that asks for a capture.
    ///
    /// ⛔ **The one answer to "does this open work".** `survey` uses it and
    /// `validate_requests` checks against it, so a verdict added later cannot be
    /// wired into one and forgotten in the other. Every variant that returns
    /// `false` returns it because acting on the comparison would be acting on a
    /// comparison that did not hold.
    #[must_use]
    pub const fn opens_work(self) -> bool {
        matches!(self, Self::Uncaptured | Self::Stale)
    }
}

impl fmt::Display for Staleness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Whether a request is new to this run.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestState {
    /// The tracker did not hold this request before this run.
    Opened,
    /// The tracker already holds it, unchanged.
    ///
    /// ⭐ **This variant is the entry's third acceptance made visible.** A
    /// second run over the same facts produces the same request in this state
    /// rather than a second request, and a caller that acts only on `Opened`
    /// does nothing.
    AlreadyOpen,
}

impl RequestState {
    /// The canonical spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Opened => "opened",
            Self::AlreadyOpen => "already_open",
        }
    }
}

impl fmt::Display for RequestState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One capture a survey asks for.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureRequest {
    /// Derived from the four fields below and checked against them.
    pub id: RequestId,
    /// The catalogue target.
    pub target: Slug,
    /// The version to capture.
    pub version: Version,
    /// The release channel.
    pub channel: ReleaseChannel,
    /// The host family.
    pub platform: Slug,
    /// Whether this run opened it.
    pub state: RequestState,
}

impl CaptureRequest {
    /// The identity tuple this request is filed under.
    #[must_use]
    pub fn key(&self) -> RequestKey<'_> {
        RequestKey {
            target: &self.target,
            version: &self.version,
            channel: self.channel,
            platform: &self.platform,
        }
    }
}

/// A request a tracker already holds, as the survey is told about it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenRequest {
    /// The catalogue target.
    pub target: Slug,
    /// The version it asks for.
    pub version: Version,
    /// The release channel.
    pub channel: ReleaseChannel,
    /// The host family.
    pub platform: Slug,
}

impl OpenRequest {
    /// The identity tuple, so a caller never spells an identifier by hand.
    #[must_use]
    pub fn key(&self) -> RequestKey<'_> {
        RequestKey {
            target: &self.target,
            version: &self.version,
            channel: self.channel,
            platform: &self.platform,
        }
    }

    /// Its derived identifier.
    #[must_use]
    pub fn id(&self) -> RequestId {
        RequestId::derive(&self.key())
    }
}

/// What the survey decided about one target on one platform, and why.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assessment {
    /// The catalogue target.
    pub target: Slug,
    /// The host family.
    pub platform: Slug,
    /// The release channel this is about.
    pub channel: ReleaseChannel,
    /// The verdict.
    pub staleness: Staleness,
    /// What the resolver selected, absent when it failed closed.
    pub selected: Option<Version>,
    /// The newest version the published views carry for this line, absent when
    /// nothing is published for it.
    pub measured: Option<Version>,
    /// The request this line is covered by, absent when it opens none.
    pub request: Option<RequestId>,
}

/// One monitor run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RequestSet {
    /// The schema identifier, read before anything else.
    pub schema: RequestSchema,
    /// When the survey was taken, UTC.
    pub surveyed_at: Instant,
    /// One per target and platform asked about, ascending.
    pub assessments: Vec<Assessment>,
    /// The requests that should be open after this run, ascending by
    /// identifier.
    pub requests: Vec<CaptureRequest>,
    /// Requests the tracker holds for a line this run assessed, at a version it
    /// no longer asks for.
    ///
    /// ⛔ **Retiring these is the "or update" half of the entry's Approach.**
    /// Left alone they accumulate one request per release that came and went
    /// before a capture happened, which is the unbounded work the Problem
    /// names. ⚠ A request for a line this run did **not** assess is never
    /// listed: a survey that retired work it was not asked about would delete a
    /// capture request because a target was left out of one run's input.
    pub superseded: Vec<RequestId>,
}

/// The same shape with the derive on it, so [`RequestSet`]'s own `Deserialize`
/// validates. Same construction as `Profile` and `Resolution`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestSetFields {
    schema: RequestSchema,
    surveyed_at: Instant,
    assessments: Vec<Assessment>,
    requests: Vec<CaptureRequest>,
    superseded: Vec<RequestId>,
}

impl From<RequestSetFields> for RequestSet {
    fn from(fields: RequestSetFields) -> Self {
        Self {
            schema: fields.schema,
            surveyed_at: fields.surveyed_at,
            assessments: fields.assessments,
            requests: fields.requests,
            superseded: fields.superseded,
        }
    }
}

impl<'de> Deserialize<'de> for RequestSet {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;
        let set = Self::from(RequestSetFields::deserialize(deserializer)?);
        validate_requests(&set).map_err(D::Error::custom)?;
        Ok(set)
    }
}

impl RequestSet {
    /// Reads and validates a survey document.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError::UnsupportedSchema`] for another generation,
    /// [`DocumentError::Malformed`] when it is not this shape, and
    /// [`DocumentError::Invalid`] when it parsed and refused an invariant.
    pub fn from_json(document: &str) -> Result<Self, DocumentError> {
        #[derive(Deserialize)]
        struct VersionProbe {
            schema: String,
        }

        let probe: VersionProbe =
            serde_json::from_str(document).map_err(DocumentError::Malformed)?;
        if probe.schema != REQUEST_SCHEMA {
            return Err(DocumentError::UnsupportedSchema {
                found: probe.schema,
                expected: REQUEST_SCHEMA,
            });
        }
        let fields: RequestSetFields =
            serde_json::from_str(document).map_err(DocumentError::Malformed)?;
        let set = Self::from(fields);
        validate_requests(&set).map_err(DocumentError::Invalid)?;
        Ok(set)
    }

    /// Writes the survey in the canonical form.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError::Invalid`] with the refused invariants. An
    /// invalid survey has no canonical form and is not written.
    pub fn to_json(&self) -> Result<String, DocumentError> {
        validate_requests(self).map_err(DocumentError::Invalid)?;
        let mut out = serde_json::to_string_pretty(self).map_err(DocumentError::Malformed)?;
        out.push('\n');
        Ok(out)
    }

    /// The requests this run is the first to ask for.
    ///
    /// ⭐ The number a caller acts on. Everything else in `requests` is work a
    /// tracker already holds.
    #[must_use]
    pub fn opened(&self) -> Vec<&CaptureRequest> {
        self.requests
            .iter()
            .filter(|request| request.state == RequestState::Opened)
            .collect()
    }

    /// Every assessment that reached one verdict.
    #[must_use]
    pub fn with_staleness(&self, staleness: Staleness) -> Vec<&Assessment> {
        self.assessments
            .iter()
            .filter(|entry| entry.staleness == staleness)
            .collect()
    }
}

/// One target the monitor watches, and the platforms it is wanted on.
#[derive(Clone, Copy, Debug)]
pub struct Watch<'a> {
    /// What the resolver decided, whole.
    ///
    /// ⛔ **The resolution rather than a version.** The selection, the scheme
    /// and the target all come out of one document that the resolver validated,
    /// so a survey cannot be handed a version that no resolution produced, and
    /// the reasoning behind a `Unresolved` verdict is a file a reader can open.
    pub resolution: &'a Resolution,
    /// The host families a capture is wanted on, from the catalogue.
    pub platforms: &'a [Slug],
}

/// Compares what the resolver selected against what the views carry, and reports
/// the capture work outstanding.
///
/// `latest` is [`crate::index::Indexes::latest`], and `open` is what the tracker
/// already holds. The answer is deterministic in all three inputs: two runs over
/// the same facts produce the same document, which is what makes a repeated run
/// safe.
///
/// ⚠ **A platform listed twice for one target is assessed once.** The catalogue
/// is a file a person edits, and a duplicated entry there would otherwise
/// produce two assessments of one line, which `E-REQ-01` then refuses. Removing
/// the duplicate here rather than refusing keeps a monitor running over a
/// catalogue typo instead of stopping every target because of one.
#[must_use]
pub fn survey(
    surveyed_at: Instant,
    watches: &[Watch<'_>],
    latest: &[LatestRow],
    open: &[OpenRequest],
) -> RequestSet {
    let mut assessments: Vec<Assessment> = Vec::new();
    let mut requests: BTreeMap<RequestId, CaptureRequest> = BTreeMap::new();
    let mut kept: BTreeSet<RequestId> = BTreeSet::new();
    let mut seen: BTreeSet<(Slug, Slug)> = BTreeSet::new();

    for watch in watches {
        let target = &watch.resolution.target;
        let scheme = &watch.resolution.scheme;
        for platform in watch.platforms {
            if !seen.insert((target.clone(), platform.clone())) {
                continue;
            }
            let measured = newest_measured(latest, target, platform, scheme);
            let (staleness, measured_version) =
                judge(scheme, watch.resolution.selected.as_ref(), measured);

            // ⛔ **The selection is read rather than unwrapped.** Only
            // `Uncaptured` and `Stale` reach here, and `judge` produces neither
            // without a selection to compare against, so the `filter` never
            // discards one in practice. Writing it as an `expect` would turn a
            // future verdict wired into `opens_work` and not into `judge` into a
            // panic; written this way that defect is an assessment asking for
            // work and naming no request, which `E-REQ-07` refuses with the
            // verdict in the message.
            let request = watch
                .resolution
                .selected
                .as_ref()
                .filter(|_| staleness.opens_work())
                .map(|version| {
                    let key = RequestKey {
                        target,
                        version,
                        channel: ReleaseChannel::Stable,
                        platform,
                    };
                    let id = RequestId::derive(&key);
                    let held = open.iter().any(|entry| entry.id() == id);
                    if held {
                        kept.insert(id);
                    }
                    requests.entry(id).or_insert_with(|| CaptureRequest {
                        id,
                        target: target.clone(),
                        version: version.clone(),
                        channel: ReleaseChannel::Stable,
                        platform: platform.clone(),
                        state: if held {
                            RequestState::AlreadyOpen
                        } else {
                            RequestState::Opened
                        },
                    });
                    id
                });

            assessments.push(Assessment {
                target: target.clone(),
                platform: platform.clone(),
                channel: ReleaseChannel::Stable,
                staleness,
                selected: watch.resolution.selected.clone(),
                measured: measured_version,
                request,
            });
        }
    }

    // ⛔ Only a line this run assessed. An open request for a target left out of
    // the input is somebody else's work, and retiring it would delete a capture
    // request because of what a caller did not ask about.
    let superseded: Vec<RequestId> = open
        .iter()
        .filter(|entry| entry.channel == ReleaseChannel::Stable)
        .filter(|entry| seen.contains(&(entry.target.clone(), entry.platform.clone())))
        .map(OpenRequest::id)
        .filter(|id| !kept.contains(id))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    assessments.sort_by(|left, right| assessment_order(left).cmp(&assessment_order(right)));
    RequestSet {
        schema: RequestSchema::current(),
        surveyed_at,
        assessments,
        requests: requests.into_values().collect(),
        superseded,
    }
}

/// The order an assessment is filed in: target, then platform, then channel.
fn assessment_order(entry: &Assessment) -> (String, String, &'static str) {
    (
        entry.target.to_string(),
        entry.platform.to_string(),
        entry.channel.as_str(),
    )
}

/// The newest version the views carry for one target on one platform.
///
/// ⚠ **Newest across every architecture and package of that platform.** A
/// request asks for a version on a platform, so the question is whether that
/// version has a measurement there at all. A package variant lagging behind is a
/// coverage question rather than a staleness one, and answering it here would
/// open a request whose completion condition was already met.
///
/// Returns the version and whether the scheme could order it. An unorderable
/// version is carried out rather than dropped, because a caller that dropped it
/// would compare against the next one down and answer confidently.
fn newest_measured<'a>(
    latest: &'a [LatestRow],
    target: &Slug,
    platform: &Slug,
    scheme: &VersionScheme,
) -> Option<(&'a Version, Option<Vec<u64>>)> {
    let rows: Vec<&LatestRow> = latest
        .iter()
        .filter(|row| row.target == *target && row.platform == *platform)
        .collect();
    if rows
        .iter()
        .any(|row| scheme.components(row.version.as_str()).is_none())
    {
        let row = rows
            .iter()
            .find(|row| scheme.components(row.version.as_str()).is_none())?;
        return Some((&row.version, None));
    }
    rows.into_iter()
        .filter_map(|row| Some((&row.version, scheme.components(row.version.as_str())?)))
        .max_by(|left, right| left.1.cmp(&right.1))
        .map(|(version, order)| (version, Some(order)))
}

/// The comparison itself, and every way it declines to answer.
fn judge(
    scheme: &VersionScheme,
    selected: Option<&Version>,
    measured: Option<(&Version, Option<Vec<u64>>)>,
) -> (Staleness, Option<Version>) {
    let measured_version = measured.as_ref().map(|(version, _)| (*version).clone());
    let Some(selected) = selected else {
        return (Staleness::Unresolved, measured_version);
    };
    // A selection the target's own scheme cannot order is the same blindness as
    // an unorderable measurement, and the resolver cannot produce one: it only
    // selects a candidate `components` read. Checked rather than assumed,
    // because a caller may build a `Resolution` in memory.
    let Some(selected_order) = scheme.components(selected.as_str()) else {
        return (Staleness::Unorderable, measured_version);
    };
    let Some((measured_version_ref, measured_order)) = measured else {
        return (Staleness::Uncaptured, None);
    };
    let Some(measured_order) = measured_order else {
        return (Staleness::Unorderable, Some(measured_version_ref.clone()));
    };
    match measured_order.cmp(&selected_order) {
        core::cmp::Ordering::Less => (Staleness::Stale, measured_version),
        core::cmp::Ordering::Greater => (Staleness::Regressed, measured_version),
        // ⛔ Equal components and different text is two spellings of one
        // release. `components` pads to the scheme's width, so `5.2` and `5.2.0`
        // land here; calling it current would leave a differently spelled
        // measurement standing for the selection, and calling it stale would
        // open a request for a version already captured under another name.
        core::cmp::Ordering::Equal => {
            if measured_version_ref.as_str() == selected.as_str() {
                (Staleness::Current, measured_version)
            } else {
                (Staleness::Ambiguous, measured_version)
            }
        }
    }
}

/// The invariants a survey document must satisfy.
///
/// # Errors
///
/// | code | refused |
/// | --- | --- |
/// | `E-REQ-01` | two assessments of one target, platform and channel |
/// | `E-REQ-02` | assessments out of canonical order |
/// | `E-REQ-03` | a request whose identifier disagrees with its own key |
/// | `E-REQ-04` | requests out of order, or one identifier carried twice |
/// | `E-REQ-05` | two requests for one target, platform and channel |
/// | `E-REQ-06` | an assessment and the request list that do not name each other |
/// | `E-REQ-07` | a verdict and a request that disagree about whether there is work |
/// | `E-REQ-08` | a request for a version its assessment did not select |
/// | `E-REQ-09` | superseded identifiers out of order, or one carried twice |
/// | `E-REQ-10` | an identifier both retired and asked for |
pub fn validate_requests(set: &RequestSet) -> Result<(), Violations> {
    let mut out = Vec::new();
    check_assessments(set, &mut out);
    check_requests(set, &mut out);
    check_pairing(set, &mut out);
    check_superseded(set, &mut out);
    Violations::from_errors(out)
}

fn check_assessments(set: &RequestSet, out: &mut Vec<SchemaError>) {
    if let Some(index) = strictly_ascending(&set.assessments, |entry| {
        let (target, platform, channel) = assessment_order(entry);
        format!("{target}\u{1f}{platform}\u{1f}{channel}")
    }) {
        let previous = &set.assessments[index - 1];
        let entry = &set.assessments[index];
        // ⛔ THE BOUND, and it is the Problem's own words. One line, one
        // assessment, so one release cannot grow work per run.
        let code = if entry.target == previous.target
            && entry.platform == previous.platform
            && entry.channel == previous.channel
        {
            "E-REQ-01"
        } else {
            "E-REQ-02"
        };
        out.push(SchemaError::new(
            code,
            format!("assessments[{index}]"),
            format!(
                "{} on {} must be unique and ascending",
                entry.target, entry.platform
            ),
        ));
    }
}

fn check_requests(set: &RequestSet, out: &mut Vec<SchemaError>) {
    for (index, request) in set.requests.iter().enumerate() {
        // ⛔ The derived identifier re-derived. A request filed under a name
        // that describes different work is the collision `RequestKey` exists to
        // prevent, arriving through a document instead of through a caller.
        if RequestId::derive(&request.key()) != request.id {
            out.push(SchemaError::new(
                "E-REQ-03",
                format!("requests[{index}].id"),
                format!(
                    "{} does not derive from {} {} {} on {}",
                    request.id,
                    request.target,
                    request.version,
                    request.channel.as_str(),
                    request.platform
                ),
            ));
        }
    }
    if let Some(index) = strictly_ascending(&set.requests, |request| request.id.to_string()) {
        out.push(SchemaError::new(
            "E-REQ-04",
            format!("requests[{index}]"),
            format!(
                "request ids must be unique and ascending, found {}",
                set.requests[index].id
            ),
        ));
    }
    let mut lines: BTreeMap<(&Slug, &Slug, &'static str), usize> = BTreeMap::new();
    for (index, request) in set.requests.iter().enumerate() {
        let line = (&request.target, &request.platform, request.channel.as_str());
        if let Some(first) = lines.insert(line, index) {
            out.push(SchemaError::new(
                "E-REQ-05",
                format!("requests[{index}]"),
                format!(
                    "{} on {} already asked for at requests[{first}]; work is one request per line",
                    request.target, request.platform
                ),
            ));
        }
    }
}

fn check_pairing(set: &RequestSet, out: &mut Vec<SchemaError>) {
    let known: BTreeMap<RequestId, &CaptureRequest> = set
        .requests
        .iter()
        .map(|request| (request.id, request))
        .collect();
    let mut named: BTreeSet<RequestId> = BTreeSet::new();

    for (index, entry) in set.assessments.iter().enumerate() {
        // ⛔ One test for both directions. `opens_work` is the single answer to
        // whether a verdict asks for a capture, so a verdict that grew a request
        // in `survey` and not here is not expressible.
        match (entry.staleness.opens_work(), entry.request) {
            (true, None) => out.push(SchemaError::new(
                "E-REQ-07",
                format!("assessments[{index}].request"),
                format!(
                    "{} asks for a capture and names no request",
                    entry.staleness
                ),
            )),
            (false, Some(id)) => out.push(SchemaError::new(
                "E-REQ-07",
                format!("assessments[{index}].request"),
                format!("{} asks for no capture and names {id}", entry.staleness),
            )),
            (_, None) => {}
            (_, Some(id)) => {
                named.insert(id);
                let Some(request) = known.get(&id) else {
                    out.push(SchemaError::new(
                        "E-REQ-06",
                        format!("assessments[{index}].request"),
                        format!("{id} is not among the requests"),
                    ));
                    continue;
                };
                if entry.selected.as_ref() != Some(&request.version) {
                    out.push(SchemaError::new(
                        "E-REQ-08",
                        format!("assessments[{index}].request"),
                        format!(
                            "{id} asks for {} and the assessment selected {}",
                            request.version,
                            entry
                                .selected
                                .as_ref()
                                .map_or("nothing", crate::canonical::Version::as_str)
                        ),
                    ));
                }
            }
        }
    }

    // ⛔ And the other end of the same question. A request nothing assessed is
    // work with no reason on the record, which is exactly the shape a duplicate
    // takes once the assessment it came from has been rewritten.
    for (index, request) in set.requests.iter().enumerate() {
        if !named.contains(&request.id) {
            out.push(SchemaError::new(
                "E-REQ-06",
                format!("requests[{index}].id"),
                format!("{} is named by no assessment", request.id),
            ));
        }
    }
}

fn check_superseded(set: &RequestSet, out: &mut Vec<SchemaError>) {
    if let Some(index) = strictly_ascending(&set.superseded, ToString::to_string) {
        out.push(SchemaError::new(
            "E-REQ-09",
            format!("superseded[{index}]"),
            format!(
                "retired ids must be unique and ascending, found {}",
                set.superseded[index]
            ),
        ));
    }
    for (index, id) in set.superseded.iter().enumerate() {
        if set.requests.iter().any(|request| request.id == *id) {
            out.push(SchemaError::new(
                "E-REQ-10",
                format!("superseded[{index}]"),
                format!("{id} is retired and asked for by the same run"),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{REQUEST_SCHEMA, ReleaseChannel, RequestId, RequestKey, Staleness};
    use crate::canonical::{Slug, Version};

    fn slug(text: &str) -> Slug {
        Slug::parse(text).expect("slug")
    }

    fn version(text: &str) -> Version {
        Version::parse(text).expect("version")
    }

    #[test]
    fn a_request_id_is_a_function_of_its_key_and_of_nothing_else() {
        let target = slug("fixture-client");
        let platform = slug("linux");
        let one = version("1.2.3");
        let key = RequestKey {
            target: &target,
            version: &one,
            channel: ReleaseChannel::Stable,
            platform: &platform,
        };
        // ⛔ The whole of "no duplicate after repeated runs": derived twice from
        // the same facts, equal both times.
        assert_eq!(RequestId::derive(&key), RequestId::derive(&key));

        let other = version("1.2.4");
        let moved = RequestKey {
            version: &other,
            ..key
        };
        assert_ne!(RequestId::derive(&key), RequestId::derive(&moved));
    }

    #[test]
    fn request_key_components_are_length_prefixed_rather_than_joined() {
        // ⚠ The collision a separator would allow: a target ending in the
        // separator and a version starting with the rest of one encode to the
        // same string when joined and to different bytes when length-prefixed.
        // `-` is the realistic separator, because it is the one character a
        // `Slug` and a `Version` can both carry.
        let left_target = slug("a-b");
        let left_version = version("1");
        let right_target = slug("a");
        let right_version = version("b-1");
        let platform = slug("linux");
        let left = RequestId::derive(&RequestKey {
            target: &left_target,
            version: &left_version,
            channel: ReleaseChannel::Stable,
            platform: &platform,
        });
        let right = RequestId::derive(&RequestKey {
            target: &right_target,
            version: &right_version,
            channel: ReleaseChannel::Stable,
            platform: &platform,
        });
        assert_ne!(left, right);
    }

    #[test]
    fn request_key_bytes_are_the_encoding_restated_independently() {
        // ⛔ **THIS TEST EXISTS BECAUSE THE ONE ABOVE CLAIMED MORE THAN IT
        // CHECKED.** A mutation pass replaced the length prefix with a single
        // separator byte and with the whole platform component removed, and both
        // plants survived every case in this module: the pair above collides
        // only under a `-` join, and nothing varied the platform at all. A
        // collision test proves injectivity for the one pair it names and says
        // nothing about the format.
        //
        // So the format is pinned instead, restated here from the doc comment on
        // `canonical_bytes` rather than read off it: each component's length as a
        // big-endian `u32`, then the component. Any dropped component, any
        // reordering and any join changes these bytes.
        let target = slug("fixture-client");
        let platform = slug("linux");
        let value = version("1.2.3");
        let key = RequestKey {
            target: &target,
            version: &value,
            channel: ReleaseChannel::Stable,
            platform: &platform,
        };
        let mut expected: Vec<u8> = Vec::new();
        for part in [
            "bit-ids/capture-request/1",
            "bit-ids/capture-requests/1",
            "fixture-client",
            "1.2.3",
            "stable",
            "linux",
        ] {
            let len = u32::try_from(part.len()).expect("a short component");
            expected.extend_from_slice(&len.to_be_bytes());
            expected.extend_from_slice(part.as_bytes());
        }
        assert_eq!(key.canonical_bytes(), expected);
    }

    #[test]
    fn a_request_id_moves_with_every_component_of_its_key() {
        // ⚠ One component at a time, because a test that varies only the version
        // proves nothing about the other three. Measured: the platform had no
        // case and dropping it from the key survived the whole module.
        let target = slug("fixture-client");
        let platform = slug("linux");
        let value = version("1.2.3");
        let base = RequestKey {
            target: &target,
            version: &value,
            channel: ReleaseChannel::Stable,
            platform: &platform,
        };
        let other_target = slug("other-client");
        let other_platform = slug("windows");
        let other_version = version("1.2.4");
        for moved in [
            RequestKey {
                target: &other_target,
                ..base
            },
            RequestKey {
                version: &other_version,
                ..base
            },
            RequestKey {
                platform: &other_platform,
                ..base
            },
        ] {
            assert_ne!(
                RequestId::derive(&base),
                RequestId::derive(&moved),
                "the identifier did not move with a component of its key"
            );
        }
    }

    #[test]
    fn a_request_id_carries_its_own_prefix_and_round_trips() {
        let target = slug("fixture-client");
        let platform = slug("linux");
        let one = version("1.2.3");
        let id = RequestId::derive(&RequestKey {
            target: &target,
            version: &one,
            channel: ReleaseChannel::Stable,
            platform: &platform,
        });
        let text = id.to_string();
        assert!(text.starts_with(RequestId::PREFIX));
        assert_eq!(RequestId::parse(&text).expect("round trip"), id);
        // ⚠ A record identifier is not a request identifier, and the prefix is
        // what says so. Both are SHA-256 and neither may be read as the other.
        let swapped = text.replace(RequestId::PREFIX, "record:");
        assert!(RequestId::parse(&swapped).is_err());
    }

    #[test]
    fn only_the_two_verdicts_that_compared_against_a_selection_open_work() {
        for staleness in Staleness::ALL {
            let expected = matches!(staleness, Staleness::Uncaptured | Staleness::Stale);
            assert_eq!(
                staleness.opens_work(),
                expected,
                "{staleness} opens_work disagrees"
            );
        }
    }

    #[test]
    fn staleness_spellings_agree_with_the_serialized_form() {
        for staleness in Staleness::ALL {
            let json = serde_json::to_string(staleness).expect("serialize");
            assert_eq!(json, format!("\"{}\"", staleness.as_str()));
        }
    }

    #[test]
    fn channel_spelling_agrees_with_the_serialized_form() {
        let json = serde_json::to_string(&ReleaseChannel::Stable).expect("serialize");
        assert_eq!(json, format!("\"{}\"", ReleaseChannel::Stable.as_str()));
    }

    #[test]
    fn the_request_schema_is_versioned_and_domain_separated() {
        assert_eq!(REQUEST_SCHEMA, "bit-ids/capture-requests/1");
        // ⚠ The digest domain and the document schema are two identifiers and
        // are not the same string. A digest keyed on the document's own schema
        // would change every request identifier the day the document shape does.
        assert_ne!(RequestKey::DOMAIN, REQUEST_SCHEMA);
    }
}
