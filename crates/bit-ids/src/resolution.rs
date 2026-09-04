//! Choosing the newest stable release, and keeping the reasoning.
//!
//! ⛔ **Version strings are not comparable in general, and a resolver that
//! pretends otherwise picks the wrong build silently.** Sorting tags as text
//! puts `4.1.10` before `4.1.9` and `release-5.2.3` before `release-5.2.10`. A
//! channel label is no better: a project can publish a preview without setting
//! the flag, and one that does set it can still tag a release the label calls
//! stable and the version string calls a beta.
//!
//! So nothing here guesses. A target declares how it spells versions, the
//! resolver compares only what that scheme can order, and anything it cannot
//! order **blocks the resolution** rather than being skipped.
//!
//! ⭐ **The skip is the dangerous case and it is worth being explicit about.**
//! An unrecognised tag that is quietly ignored does not produce an error; it
//! produces an older version, selected confidently, with nothing in the record
//! saying a newer one was seen and not understood. Every candidate is kept with
//! the verdict it got, so the resolution is a decision anyone can re-derive.
//!
//! ⚠ **Stability is judged pessimistically, by both signals.** A candidate that
//! either source or version text calls a prerelease is not stable. Being wrong
//! that way costs a skipped release; being wrong the other way publishes a
//! preview build as the stable one, which is a measurement about a build no
//! user runs.

use core::fmt;
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::canonical::{Instant, Label, Sha256Digest, Slug, Url, Version};
use crate::json::DocumentError;
use crate::validate::{SchemaError, Violations, strictly_ascending};

/// Identifier carried by every first-generation resolution.
pub const RESOLUTION_SCHEMA: &str = "bit-ids/resolution/1";

/// The schema identifier a resolution declares.
///
/// ⭐ A type rather than a validated `String`, which is what `ManifestSchema`
/// already does. A wrong identifier is then unrepresentable rather than
/// refused, and the invariant that checked for one was a guard `from_json`
/// could never reach: the version probe answers first. It was written, found
/// unreachable while planting a defect for it, and deleted, which is the same
/// path `E-BND-10` took in `SCHEMA-02`.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ResolutionSchema(&'static str);

impl ResolutionSchema {
    /// The schema this build reads and writes.
    #[must_use]
    pub const fn current() -> Self {
        Self(RESOLUTION_SCHEMA)
    }

    /// Parses a declared schema identifier.
    ///
    /// # Errors
    ///
    /// Returns an error for any identifier other than [`RESOLUTION_SCHEMA`].
    pub fn parse(text: &str) -> Result<Self, crate::canonical::CanonicalError> {
        if text == RESOLUTION_SCHEMA {
            Ok(Self(RESOLUTION_SCHEMA))
        } else {
            Err(crate::canonical::CanonicalError::new(
                "resolution-schema-version",
                format!("unsupported schema {text:?}, this build reads {RESOLUTION_SCHEMA:?}"),
            ))
        }
    }

    /// The declared identifier.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

impl Serialize for ResolutionSchema {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0)
    }
}

impl<'de> Deserialize<'de> for ResolutionSchema {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(D::Error::custom)
    }
}

/// Tokens that mark a version as something other than a stable release.
///
/// ⚠ Matched as whole tokens, not as substrings. `rc` inside a longer word is
/// not a release candidate, and a rule that matched anywhere would call a
/// version carrying `march` a prerelease.
const PRERELEASE_TOKENS: &[&str] = &[
    "alpha", "beta", "rc", "pre", "preview", "dev", "nightly", "snapshot", "test", "canary",
    "insider", "eap", "unstable",
];

/// How a target spells the versions it publishes.
///
/// Product-specific by construction. `qbittorrent` tags `release-5.2.3` and
/// `transmission` tags `4.1.3`, and there is no general rule that reads both
/// without also reading things that are not versions at all.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VersionScheme {
    /// A literal prefix stripped before parsing, such as `release-`.
    pub tag_prefix: Option<Label>,
    /// The fewest dot-separated numeric components a version may have.
    pub min_components: u8,
    /// The most it may have. Enhanced editions publish four; most publish three.
    pub max_components: u8,
}

impl VersionScheme {
    /// Strips the prefix, or reports that the tag is not this target's.
    fn strip<'a>(&self, tag: &'a str) -> Option<&'a str> {
        match &self.tag_prefix {
            Some(prefix) => tag.strip_prefix(prefix.as_str()),
            None => Some(tag),
        }
    }

    /// The numeric components, when the text is orderable under this scheme.
    fn components(&self, text: &str) -> Option<Vec<u64>> {
        let parts: Vec<&str> = text.split('.').collect();
        let count = u8::try_from(parts.len()).ok()?;
        if count < self.min_components || count > self.max_components {
            return None;
        }
        let mut components: Vec<u64> = parts
            .iter()
            .map(|part| {
                // ⚠ A component with a leading zero is refused rather than
                // parsed. `1.01` and `1.1` are two spellings that compare equal,
                // and an append-only store cannot tell those apart later.
                if part.is_empty() || (part.len() > 1 && part.starts_with('0')) {
                    return None;
                }
                part.parse::<u64>().ok()
            })
            .collect::<Option<Vec<u64>>>()?;
        // ⛔ Padded to the scheme's width before comparing. Without this, `4.1`
        // and `4.1.0` compare as different versions with the longer one newer,
        // because a shorter vector sorts first. They are the same release, and
        // treating them as an ordering rather than an ambiguity is the silent
        // wrong answer this module exists to refuse. Found by writing the test
        // for the ambiguous case and watching it select instead.
        components.resize(usize::from(self.max_components), 0);
        Some(components)
    }
}

/// Whether a version's own text marks it as a prerelease.
///
/// Case-insensitive and token-bounded.
#[must_use]
pub fn text_marks_prerelease(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    let tokens = lowered.split(|c: char| !c.is_ascii_alphanumeric());
    tokens.filter(|token| !token.is_empty()).any(|token| {
        // A token that is a marker, or a marker followed by digits such as
        // `rc1` or `beta5`, which is how most projects spell them.
        let letters: String = token
            .chars()
            .take_while(char::is_ascii_alphabetic)
            .collect();
        let rest = &token[letters.len()..];
        PRERELEASE_TOKENS.contains(&letters.as_str())
            && (rest.is_empty() || rest.bytes().all(|b| b.is_ascii_digit()))
    })
}

/// What the resolver decided about one candidate, and why.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// The newest stable release, and the resolution's answer.
    Selected,
    /// The tag does not carry this target's prefix, so it is not this
    /// target's release.
    ForeignTag,
    /// The publishing source flagged it a prerelease or a draft.
    PrereleaseByLabel,
    /// The version text carries a prerelease token.
    PrereleaseByVersion,
    /// ⛔ The version cannot be ordered under this target's scheme. It blocks
    /// the resolution rather than being skipped, because nothing here can rule
    /// out that it is newer than the candidate that would otherwise win.
    Unorderable,
    /// Stable, orderable, and older than the selection.
    Superseded,
    /// ⭐ Could not be ordered by version, and was published before the
    /// candidate that won, so it cannot be the newest.
    ///
    /// This is what keeps a project's own history from blocking every run. A
    /// live dry run against `transmission` found 51 candidates in exactly this
    /// position: two-component tags from a decade ago that the current scheme
    /// cannot read. Refusing over them is correct only if the resolver has no
    /// second signal, and publication order is one, so nothing is guessed and
    /// the reason is on the record.
    PredatesSelection,
    /// Compares equal to another candidate and is spelled differently, so
    /// "newest" has no single answer.
    Ambiguous,
    /// Its source's newest stable disagrees with another source's.
    Divergent,
}

impl Verdict {
    /// Every variant, so a test can hold the two spellings in step.
    pub const ALL: &'static [Self] = &[
        Self::Selected,
        Self::ForeignTag,
        Self::PrereleaseByLabel,
        Self::PrereleaseByVersion,
        Self::Unorderable,
        Self::Superseded,
        Self::PredatesSelection,
        Self::Ambiguous,
        Self::Divergent,
    ];

    /// The canonical spelling, for a message a person reads.
    ///
    /// ⚠ This is a second spelling of the vocabulary serde already derives, so
    /// `resolution_verdict_spellings_agree_with_the_serialized_form` holds the
    /// two together. A name in two places with nothing comparing them is the
    /// drift this repository refuses everywhere else, and a door sweep found
    /// this pair unchecked.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Selected => "selected",
            Self::ForeignTag => "foreign_tag",
            Self::PrereleaseByLabel => "prerelease_by_label",
            Self::PrereleaseByVersion => "prerelease_by_version",
            Self::Unorderable => "unorderable",
            Self::Superseded => "superseded",
            Self::PredatesSelection => "predates_selection",
            Self::Ambiguous => "ambiguous",
            Self::Divergent => "divergent",
        }
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One release a source offered, before any judgement.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    /// Which source offered it.
    pub source: Slug,
    /// The tag exactly as published.
    pub tag: Label,
    /// The source's own prerelease flag.
    pub prerelease: bool,
    /// The source's own draft flag.
    pub draft: bool,
    /// When the source says it was published, when it says.
    pub published_at: Option<Instant>,
}

/// A candidate with the verdict it got and the version that was read out of it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Considered {
    /// What was offered.
    pub candidate: Candidate,
    /// What was decided.
    pub verdict: Verdict,
    /// The version extracted from the tag, absent when none could be.
    pub version: Option<Version>,
}

/// The exact bytes one source answered with.
///
/// ⛔ A selection is only re-derivable if the input is recorded. The digest is
/// of the response as it arrived, before any parsing, so a later reader can
/// tell a resolver defect from a source that changed its answer.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceResponse {
    /// Source identifier, unique within the resolution.
    pub id: Slug,
    /// Where it was asked.
    pub url: Url,
    /// When it answered, UTC.
    pub retrieved_at: Instant,
    /// Digest of the bytes that arrived.
    pub digest: Sha256Digest,
    /// How many candidates were read out of it.
    pub candidates: u32,
}

/// One run of the resolver, with everything it looked at.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Resolution {
    /// The schema identifier, read before anything else.
    pub schema: ResolutionSchema,
    /// Which target this resolves.
    pub target: Slug,
    /// When the decision was made, UTC.
    pub resolved_at: Instant,
    /// How this target spells its versions.
    pub scheme: VersionScheme,
    /// The sources asked, and what they answered with.
    pub sources: Vec<SourceResponse>,
    /// The newest stable version, or absent when the resolver failed closed.
    pub selected: Option<Version>,
    /// Every candidate considered, in the order the sources offered them.
    pub considered: Vec<Considered>,
}

/// The same shape with the derive on it, so [`Resolution`]'s own `Deserialize`
/// validates. Same construction as `Profile`, and for the same reason.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResolutionFields {
    schema: ResolutionSchema,
    target: Slug,
    resolved_at: Instant,
    scheme: VersionScheme,
    sources: Vec<SourceResponse>,
    selected: Option<Version>,
    considered: Vec<Considered>,
}

impl From<ResolutionFields> for Resolution {
    fn from(fields: ResolutionFields) -> Self {
        Self {
            schema: fields.schema,
            target: fields.target,
            resolved_at: fields.resolved_at,
            scheme: fields.scheme,
            sources: fields.sources,
            selected: fields.selected,
            considered: fields.considered,
        }
    }
}

impl<'de> Deserialize<'de> for Resolution {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;
        let resolution = Self::from(ResolutionFields::deserialize(deserializer)?);
        validate_resolution(&resolution).map_err(D::Error::custom)?;
        Ok(resolution)
    }
}

impl Resolution {
    /// Reads and validates a resolution document.
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
        if probe.schema != RESOLUTION_SCHEMA {
            return Err(DocumentError::UnsupportedSchema {
                found: probe.schema,
                expected: RESOLUTION_SCHEMA,
            });
        }
        let fields: ResolutionFields =
            serde_json::from_str(document).map_err(DocumentError::Malformed)?;
        let resolution = Self::from(fields);
        validate_resolution(&resolution).map_err(DocumentError::Invalid)?;
        Ok(resolution)
    }

    /// Writes the resolution in the canonical form.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError::Invalid`] with the refused invariants. An
    /// invalid resolution has no canonical form and is not written.
    pub fn to_json(&self) -> Result<String, DocumentError> {
        validate_resolution(self).map_err(DocumentError::Invalid)?;
        let mut out = serde_json::to_string_pretty(self).map_err(DocumentError::Malformed)?;
        out.push('\n');
        Ok(out)
    }

    /// Whether the resolver reached an answer.
    #[must_use]
    pub const fn resolved(&self) -> bool {
        self.selected.is_some()
    }

    /// Every candidate that got one verdict.
    #[must_use]
    pub fn with_verdict(&self, verdict: Verdict) -> Vec<&Considered> {
        self.considered
            .iter()
            .filter(|entry| entry.verdict == verdict)
            .collect()
    }
}

/// Decides the newest stable release from what the sources offered.
///
/// ⛔ **It fails closed.** `selected` is absent whenever the answer is not
/// forced: no stable candidate at all, a candidate that could not be ordered,
/// two spellings that compare equal, or two sources that disagree. Every one of
/// those keeps its reason in the trace.
#[must_use]
pub fn resolve(
    target: Slug,
    resolved_at: Instant,
    scheme: VersionScheme,
    sources: Vec<SourceResponse>,
    candidates: Vec<Candidate>,
) -> Resolution {
    let mut considered: Vec<Considered> = candidates
        .into_iter()
        .map(|candidate| {
            let (verdict, version) = judge(&scheme, &candidate);
            Considered {
                candidate,
                verdict,
                version,
            }
        })
        .collect();
    // ⭐ One construction site. An earlier version returned a whole
    // `Resolution` from each fail-closed branch, and five copies of the same
    // struct literal is five places for a field to be forgotten.
    let selected = decide(&scheme, &sources, &mut considered);
    Resolution {
        schema: ResolutionSchema::current(),
        target,
        resolved_at,
        scheme,
        sources,
        selected,
        considered,
    }
}

/// The decision itself. `None` is the fail-closed answer, and every path to it
/// leaves the reason in a verdict.
fn decide(
    scheme: &VersionScheme,
    sources: &[SourceResponse],
    considered: &mut [Considered],
) -> Option<Version> {
    let (best_components, best_version) = considered
        .iter()
        .filter(|entry| entry.verdict == Verdict::Superseded)
        .filter_map(|entry| Some((components(scheme, entry)?, entry.version.clone()?)))
        .max_by(|left, right| left.0.cmp(&right.0))?;

    let equal = equal_indices(considered, scheme, &best_components);

    // Two spellings that compare equal make "newest" ambiguous. Order is
    // checked, never imposed, which is the same rule one layer up.
    let spellings: BTreeSet<&str> = equal
        .iter()
        .filter_map(|index| considered[*index].version.as_ref().map(Version::as_str))
        .collect();
    if spellings.len() > 1 {
        mark(considered, &equal, Verdict::Ambiguous);
        return None;
    }

    // Every source that offered a stable candidate must have offered this one.
    // A source whose own newest differs is a divergence, not a tie to break.
    let divergent: Vec<usize> = sources
        .iter()
        .filter_map(|source| {
            let (found, index) = considered
                .iter()
                .enumerate()
                .filter(|(_, entry)| {
                    entry.candidate.source == source.id && entry.verdict == Verdict::Superseded
                })
                .filter_map(|(index, entry)| Some((components(scheme, entry)?, index)))
                .max_by(|left, right| left.0.cmp(&right.0))?;
            (found != best_components).then_some(index)
        })
        .collect();
    if !divergent.is_empty() {
        mark(considered, &equal, Verdict::Divergent);
        mark(considered, &divergent, Verdict::Divergent);
        return None;
    }

    // ⛔ An unorderable candidate blocks unless a second signal rules it out.
    // Publication order is that signal: one published strictly before the
    // winner cannot be newer than it, whatever its tag says. One with no date,
    // or a date at or after the winner's, still blocks.
    let Some(best_published) = considered
        .iter()
        .filter(|entry| entry.verdict == Verdict::Superseded)
        .filter(|entry| components(scheme, entry).as_deref() == Some(&best_components))
        .find_map(|entry| entry.candidate.published_at.clone())
    else {
        return blocked_by_unorderable(considered)
            .then_some(())
            .map_or_else(
                || {
                    mark(considered, &equal, Verdict::Selected);
                    Some(best_version.clone())
                },
                |()| None,
            );
    };
    let predating: Vec<usize> = considered
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.verdict == Verdict::Unorderable)
        .filter(|(_, entry)| {
            entry
                .candidate
                .published_at
                .as_ref()
                .is_some_and(|published| *published < best_published)
        })
        .map(|(index, _)| index)
        .collect();
    mark(considered, &predating, Verdict::PredatesSelection);

    if blocked_by_unorderable(considered) {
        return None;
    }
    mark(considered, &equal, Verdict::Selected);
    Some(best_version)
}

fn blocked_by_unorderable(considered: &[Considered]) -> bool {
    considered
        .iter()
        .any(|entry| entry.verdict == Verdict::Unorderable)
}

fn mark(considered: &mut [Considered], indices: &[usize], verdict: Verdict) {
    for index in indices {
        considered[*index].verdict = verdict;
    }
}

fn equal_indices(considered: &[Considered], scheme: &VersionScheme, target: &[u64]) -> Vec<usize> {
    considered
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.verdict == Verdict::Superseded)
        .filter(|(_, entry)| components(scheme, entry).as_deref() == Some(target))
        .map(|(index, _)| index)
        .collect()
}

fn components(scheme: &VersionScheme, entry: &Considered) -> Option<Vec<u64>> {
    scheme.components(entry.version.as_ref()?.as_str())
}

/// The per-candidate rules, in the order they are asked.
///
/// ⚠ Stability before orderability. A long tail of `x.y.z-beta.n` tags would
/// otherwise be unorderable and block every resolution, when what they actually
/// are is not-stable and irrelevant.
fn judge(scheme: &VersionScheme, candidate: &Candidate) -> (Verdict, Option<Version>) {
    let Some(text) = scheme.strip(candidate.tag.as_str()) else {
        return (Verdict::ForeignTag, None);
    };
    let version = Version::parse(text).ok();
    if candidate.prerelease || candidate.draft {
        return (Verdict::PrereleaseByLabel, version);
    }
    if text_marks_prerelease(text) {
        return (Verdict::PrereleaseByVersion, version);
    }
    if scheme.components(text).is_none() {
        return (Verdict::Unorderable, version);
    }
    // `Superseded` is the provisional verdict for every orderable stable
    // candidate; `resolve` promotes the winner. Starting from "selected" and
    // demoting would leave a record claiming several selections if a later rule
    // returned early.
    (Verdict::Superseded, version)
}

/// The invariants a resolution document must satisfy.
///
/// # Errors
///
/// Returns the refusals, each with a stable code.
pub fn validate_resolution(resolution: &Resolution) -> Result<(), Violations> {
    let mut out = Vec::new();
    check_sources(resolution, &mut out);
    check_selection(resolution, &mut out);
    Violations::from_errors(out)
}

fn check_sources(resolution: &Resolution, out: &mut Vec<SchemaError>) {
    if resolution.sources.is_empty() {
        out.push(SchemaError::new(
            "E-RES-01",
            "sources",
            "a resolution with no source asked nothing",
        ));
    }
    if let Some(index) = strictly_ascending(&resolution.sources, |source| source.id.to_string()) {
        out.push(SchemaError::new(
            "E-RES-02",
            format!("sources[{index}]"),
            format!(
                "source ids must be unique and ascending, found {}",
                resolution.sources[index].id
            ),
        ));
    }
    for (index, entry) in resolution.considered.iter().enumerate() {
        if !resolution
            .sources
            .iter()
            .any(|source| source.id == entry.candidate.source)
        {
            out.push(SchemaError::new(
                "E-RES-03",
                format!("considered[{index}].candidate.source"),
                format!("{} is not among the sources asked", entry.candidate.source),
            ));
        }
    }
    for (index, source) in resolution.sources.iter().enumerate() {
        let counted = resolution
            .considered
            .iter()
            .filter(|entry| entry.candidate.source == source.id)
            .count();
        if u64::from(source.candidates) != counted as u64 {
            out.push(SchemaError::new(
                "E-RES-04",
                format!("sources[{index}].candidates"),
                format!(
                    "{} declares {} candidate(s) and {counted} are carried",
                    source.id, source.candidates
                ),
            ));
        }
        // A decision cannot predate the answer it was made from.
        if resolution.resolved_at < source.retrieved_at {
            out.push(SchemaError::new(
                "E-RES-05",
                format!("sources[{index}].retrieved_at"),
                format!(
                    "{} answered at {} and the decision is stamped {}",
                    source.id, source.retrieved_at, resolution.resolved_at
                ),
            ));
        }
    }
}

fn check_selection(resolution: &Resolution, out: &mut Vec<SchemaError>) {
    let selected: Vec<&Considered> = resolution
        .considered
        .iter()
        .filter(|entry| entry.verdict == Verdict::Selected)
        .collect();
    match (&resolution.selected, selected.as_slice()) {
        (None, []) => {}
        (Some(version), []) => out.push(SchemaError::new(
            "E-RES-07",
            "selected",
            format!("selects {version} and no candidate is marked selected"),
        )),
        (None, entries) => out.push(SchemaError::new(
            "E-RES-07",
            "selected",
            format!(
                "selects nothing and {} candidate(s) are marked selected",
                entries.len()
            ),
        )),
        (Some(version), entries) => {
            for entry in entries {
                if entry.version.as_ref() != Some(version) {
                    out.push(SchemaError::new(
                        "E-RES-06",
                        "selected",
                        format!(
                            "the document selects {version} and {} is marked selected",
                            entry.candidate.tag
                        ),
                    ));
                }
            }
        }
    }
    // ⛔ The rule the whole module exists for. A selection standing beside a
    // candidate nobody could order is a version chosen without ruling out a
    // newer one, which is the silent-skip defect this fails closed on.
    if resolution.selected.is_some()
        && resolution
            .considered
            .iter()
            .any(|entry| entry.verdict == Verdict::Unorderable)
    {
        out.push(SchemaError::new(
            "E-RES-08",
            "selected",
            "a candidate could not be ordered, so nothing may be selected",
        ));
    }
}

/// Reading a source's own answer into candidates.
///
/// ⛔ One reader per source shape, and each one is the only place that knows
/// that shape. A resolver that accepted a pre-digested list would move the
/// parsing somewhere with no test and no record of what arrived.
pub mod sources {
    use super::Candidate;
    use crate::canonical::{Instant, Label, Slug};

    /// Reads the GitHub releases list.
    ///
    /// # Errors
    ///
    /// Returns a message naming the entry that could not be read. A release
    /// with an unusable tag is refused rather than dropped: a candidate list
    /// quietly missing an entry is the silent skip this module exists to
    /// prevent, one layer earlier.
    pub fn github_releases(body: &[u8], source: &Slug) -> Result<Vec<Candidate>, String> {
        #[derive(serde::Deserialize)]
        struct Release {
            tag_name: String,
            #[serde(default)]
            prerelease: bool,
            #[serde(default)]
            draft: bool,
            published_at: Option<String>,
        }

        let releases: Vec<Release> =
            serde_json::from_slice(body).map_err(|error| format!("not a release list: {error}"))?;
        releases
            .into_iter()
            .map(|release| {
                let tag = Label::parse(&release.tag_name)
                    .map_err(|error| format!("tag {:?}: {error}", release.tag_name))?;
                let published_at = release
                    .published_at
                    .as_deref()
                    .map(Instant::parse)
                    .transpose()
                    .map_err(|error| format!("tag {tag}: published_at: {error}"))?;
                Ok(Candidate {
                    source: source.clone(),
                    tag,
                    prerelease: release.prerelease,
                    draft: release.draft,
                    published_at,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{VersionScheme, text_marks_prerelease};
    use crate::canonical::Label;

    fn scheme(prefix: Option<&str>, min: u8, max: u8) -> VersionScheme {
        VersionScheme {
            tag_prefix: prefix.map(|p| Label::parse(p).expect("label")),
            min_components: min,
            max_components: max,
        }
    }

    #[test]
    fn dotted_components_order_numerically_not_lexically() {
        let s = scheme(None, 2, 4);
        // ⛔ The headline defect: as text, "4.1.10" sorts before "4.1.9".
        assert!(s.components("4.1.10") > s.components("4.1.9"));
        assert!(s.components("5.2.10") > s.components("5.2.3"));
        assert!(s.components("5.2.3.10") > s.components("5.2.3.9"));
        assert!("4.1.10" < "4.1.9", "text order really is the wrong answer");
    }

    #[test]
    fn a_component_count_outside_the_scheme_is_not_orderable() {
        let s = scheme(None, 3, 3);
        assert!(s.components("4.1.3").is_some());
        assert!(s.components("4.1").is_none());
        assert!(s.components("4.1.3.1").is_none());
    }

    #[test]
    fn a_padded_component_is_refused_rather_than_parsed() {
        let s = scheme(None, 2, 4);
        assert!(s.components("1.01").is_none(), "1.01 and 1.1 compare equal");
        assert!(s.components("1.0").is_some());
    }

    #[test]
    fn a_tag_prefix_is_stripped_and_a_foreign_tag_is_not_this_target() {
        let s = scheme(Some("release-"), 3, 3);
        assert_eq!(s.strip("release-5.2.3"), Some("5.2.3"));
        assert_eq!(s.strip("v5.2.3"), None);
        assert_eq!(scheme(None, 3, 3).strip("5.2.3"), Some("5.2.3"));
    }

    #[test]
    fn prerelease_tokens_match_whole_tokens_and_their_numbered_forms() {
        for text in ["4.1.0-beta.5", "1.0.0-rc1", "2.0-ALPHA", "3.1.0.nightly"] {
            assert!(text_marks_prerelease(text), "{text} is a prerelease");
        }
        for text in ["4.1.3", "5.2.3.10", "1.2.3-final", "2.0.0-march"] {
            assert!(
                !text_marks_prerelease(text),
                "{text} carries no prerelease token"
            );
        }
    }
}
