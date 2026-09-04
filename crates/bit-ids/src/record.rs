//! The profile record: one target, one exact stable build, one capture run.
//!
//! The section list is the one in `docs/architecture.md` section 4, and that
//! document is the authority when this file disagrees with it.
//!
//! Three sections are deliberately shallow here. `acquisition` carries only
//! what a profile-level invariant needs, and the route record itself belongs to
//! `ACQ-01`. `capture` carries the run identity and the connector list, and the
//! full run manifest belongs to `SCHEMA-02`. `corroboration` carries the
//! per-field outcome, and the agreement and conflict model belongs to
//! `SCHEMA-03`. Every field declared here is read by
//! [`crate::validate`]; none of them is a placeholder.

use serde::{Deserialize, Serialize};

use crate::ReleaseChannel;
use crate::acquisition::AcquisitionRoute;
use crate::agreement::{Adjudication, FieldCorroboration, Normalization};
use crate::canonical::{Instant, RelPath, Sha256Digest, Slug, Version};
use crate::identity::{RecordId, SchemaVersion};
use crate::observation::{FieldPath, ObservedField};

/// Whether the target is a product a user installs or a library harness.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    /// An application distributed to users.
    Client,
    /// A committed reference harness built against a library release.
    Library,
}

/// What was measured.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    /// Catalogue identifier, matching an `id` in `catalogue/clients.toml`.
    pub id: Slug,
    /// The name the project publishes the target under.
    pub display_name: String,
    /// Application or library harness.
    pub kind: TargetKind,
    /// The engine this build is known to embed, when the relationship was
    /// itself measured. `null` is the honest value until it is.
    pub engine: Option<Slug>,
}

/// The exact build the measurement came from.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Build {
    /// The version the installed executable or harness reported for itself.
    pub version: Version,
    /// Always stable. The type carries one variant so a prerelease cannot be
    /// spelled at all, rather than being spelled and refused later.
    pub channel: ReleaseChannel,
    /// Host family, as named in `catalogue/clients.toml`.
    pub platform: Slug,
    /// Machine architecture.
    pub arch: Slug,
    /// Package format the artifact was delivered in.
    pub package: Slug,
    /// Digest of the installed executable or harness binary.
    pub executable: Sha256Digest,
}

/// An implementation that observed the run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Connector {
    /// Connector identifier, unique within the record.
    pub id: Slug,
    /// The connector build that produced the evidence.
    pub version: Version,
}

/// The run that produced the observations.
///
/// `SCHEMA-02` owns the full run manifest: runner image, kernel, isolation
/// mode, ordered phases, redaction declarations and host facts.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capture {
    /// Capture identifier, unique for this target, version, platform and arch.
    pub id: Slug,
    /// When the run started, UTC.
    pub captured_at: Instant,
    /// Digest of the generated torrent metainfo the run used.
    pub fixture: Sha256Digest,
    /// Which acquisition route's installed build was put on the wire.
    ///
    /// ⛔ A run observes **one** installed build. The second route proves the
    /// version resolves the same; it does not prove the other bytes behave the
    /// same, and a record that did not say which install was watched let a
    /// reader assume both were. `ACQ-03` is what acts on the distinction.
    pub observed_route: Slug,
    /// The connector that is the project's own active observer. It must appear
    /// in `connectors`, because it is one of the two, not a third thing beside
    /// them.
    pub observer: Slug,
    /// Every connector that observed the run, the observer included.
    pub connectors: Vec<Connector>,
}

/// What kind of raw artifact an evidence entry is.
///
/// The set mirrors the raw bundle in `docs/capture-methodology.md`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    /// The generated torrent metainfo and payload digest.
    Metainfo,
    /// The observer's own event stream.
    ObserverStream,
    /// Raw tracker requests or datagrams, in order.
    TrackerCapture,
    /// The peer handshake and initial message transcript.
    PeerTranscript,
    /// An independent connector's machine-readable report.
    ConnectorOutput,
    /// A raw packet capture.
    PacketCapture,
    /// The target's stdout and stderr after secret scanning.
    ProcessOutput,
    /// The host and tool manifest for the run.
    EnvironmentManifest,
    /// A control run proving a connector can see the surface at all. Absence is
    /// only publishable behind one of these.
    PositiveControl,
}

/// One raw artifact in the evidence bundle.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRef {
    /// Evidence identifier, unique within the record and cited by fields.
    pub id: Slug,
    /// What kind of artifact it is.
    pub kind: EvidenceKind,
    /// Where it sits, relative to the bundle root.
    pub path: RelPath,
    /// Its exact size. A parsed value whose bytes are not recoverable is not a
    /// measurement, so a zero-length artifact is refused.
    pub bytes: u64,
    /// Its digest.
    pub sha256: Sha256Digest,
    /// The connector that produced it, or `null` for an artifact the run
    /// generated rather than observed, such as the fixture metainfo.
    pub connector: Option<Slug>,
}

/// The record's fields, as they sit in a document.
///
/// ⛔ This exists so that [`Profile`] does not derive `Deserialize`. A derived
/// one is a public door: `serde_json::from_str::<Profile>` would hand a caller
/// an unvalidated record, and a control enforced on one path into an operation
/// and not its siblings is the most recurring hole there is. It was a live
/// door here until a door sweep found it.
///
/// The duplication is compile-checked in both directions: a field added to one
/// and not the other fails to build at the conversion below.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProfileFields {
    pub(crate) schema: SchemaVersion,
    pub(crate) id: RecordId,
    pub(crate) target: Target,
    pub(crate) build: Build,
    pub(crate) acquisition: Vec<AcquisitionRoute>,
    pub(crate) capture: Capture,
    pub(crate) observations: Vec<ObservedField>,
    pub(crate) corroboration: Vec<FieldCorroboration>,
    pub(crate) normalizations: Vec<Normalization>,
    pub(crate) evidence: Vec<EvidenceRef>,
    pub(crate) supersedes: Option<RecordId>,
    pub(crate) adjudication: Option<Adjudication>,
}

impl From<ProfileFields> for Profile {
    fn from(fields: ProfileFields) -> Self {
        let ProfileFields {
            schema,
            id,
            target,
            build,
            acquisition,
            capture,
            observations,
            corroboration,
            normalizations,
            evidence,
            supersedes,
            adjudication,
        } = fields;
        Self {
            schema,
            id,
            target,
            build,
            acquisition,
            capture,
            observations,
            corroboration,
            normalizations,
            evidence,
            supersedes,
            adjudication,
        }
    }
}

impl<'de> Deserialize<'de> for Profile {
    /// Every serde route into a record validates.
    ///
    /// [`Profile::from_json`](crate::Profile::from_json) is the route to
    /// prefer, because it reports the schema version first and returns the
    /// refused invariants with their codes. This one exists so that the
    /// generic route cannot be the loose one; it can only say that the record
    /// was refused, and what.
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let profile = Self::from(ProfileFields::deserialize(deserializer)?);
        crate::validate::validate(&profile).map_err(serde::de::Error::custom)?;
        Ok(profile)
    }
}

/// An immutable measurement of one build's observable identity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Profile {
    /// The schema this record is written against.
    pub schema: SchemaVersion,
    /// The record's deterministic identifier, derived from the identity tuple.
    pub id: RecordId,
    /// What was measured.
    pub target: Target,
    /// The exact build.
    pub build: Build,
    /// The independent routes the build was obtained through.
    pub acquisition: Vec<AcquisitionRoute>,
    /// The run.
    pub capture: Capture,
    /// The identity fields, sorted by path.
    pub observations: Vec<ObservedField>,
    /// The per-field corroboration outcomes, sorted by path.
    pub corroboration: Vec<FieldCorroboration>,
    /// Every normalization a projection cites, sorted by identifier. Declared
    /// once here rather than repeated at each use, so two fields cannot claim
    /// the same name for different transformations.
    pub normalizations: Vec<Normalization>,
    /// The raw artifacts, sorted by identifier.
    pub evidence: Vec<EvidenceRef>,
    /// The record this one corrects, or `null` for an original record. A
    /// correction never edits the record it supersedes.
    pub supersedes: Option<RecordId>,
    /// Why this record corrects the one it supersedes. Required on a
    /// correction, forbidden on an original.
    pub adjudication: Option<Adjudication>,
}

impl Profile {
    /// The observation for one field path, when the record carries it.
    #[must_use]
    pub fn field(&self, path: &FieldPath) -> Option<&ObservedField> {
        self.observations.iter().find(|field| &field.path == path)
    }

    /// The normalization with one identifier, when the record declares it.
    #[must_use]
    pub fn normalization(&self, id: &Slug) -> Option<&Normalization> {
        self.normalizations.iter().find(|entry| &entry.id == id)
    }

    /// The evidence entry with one identifier, when the record carries it.
    #[must_use]
    pub fn evidence_entry(&self, id: &Slug) -> Option<&EvidenceRef> {
        self.evidence.iter().find(|entry| &entry.id == id)
    }
}
