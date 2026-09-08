//! Where a consumer fetches a published byte from, and how long it is good for.
//!
//! `PUB-04` owns this. A consumer that has to scrape a repository's web UI to
//! find a record has no contract at all: the layout can move, a path can be
//! reused, and nothing says which of two fetches may be cached. This is the
//! contract, derived from an assembled release rather than written beside it.
//!
//! ⛔ **The stability class is read from `CANONICAL_ROOTS`, never declared per
//! path.** `docs/architecture.md` section 9 already decided which roots carry
//! measurements and never change: `profiles/` and `raw/`. A second list here
//! would be the same rule spelled twice, and the copy that drifted would be the
//! one telling a consumer it may cache a file that moves.
//!
//! ⛔ **Which document proves a path's bytes is a function of the path.**
//! `MANIFEST.json` describes every file except itself and `SHA256SUMS`, and
//! `SHA256SUMS` covers every file except itself. So between them each published
//! byte is covered once, and exactly one file is covered by neither: the
//! checksum file. ⚠ A consumer that assumed either document covered everything
//! would find the gap precisely where the other one is, which is why
//! [`Integrity`] is on the record for every path rather than left to be worked
//! out.
//!
//! ⚠ **No host and no URL appear here.** A public URL built from a hardcoded
//! host is dead everywhere except the machine that made it, so the contract
//! carries paths and a caller composes a URL from a base it was given.
//! `docs/publishing.md` carries the forms.

use core::fmt;
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::canonical::{CanonicalError, RelPath, Sha256Digest};
use crate::json::DocumentError;
use crate::release::{CHECKSUMS_FILE, LICENSE_FILE, RELEASE_MANIFEST_FILE, Release};
use crate::store::is_canonical_path;
use crate::validate::{SchemaError, Violations, strictly_ascending};

/// Identifier carried by every first-generation access contract.
pub const ACCESS_SCHEMA: &str = "bit-ids/access/1";

/// The branch a publication is appended to.
///
/// ⚠ Named here because the contract is about that branch and a consumer has to
/// spell it in a URL. It is the same name `publish-data.sh` defaults to, and
/// `the_published_branch_has_one_spelling` holds the two together.
pub const DATA_BRANCH: &str = "data";

/// The schema identifier a contract declares.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct AccessSchema(&'static str);

impl AccessSchema {
    /// The schema this build reads and writes.
    #[must_use]
    pub const fn current() -> Self {
        Self(ACCESS_SCHEMA)
    }

    /// Parses a declared schema identifier.
    ///
    /// # Errors
    ///
    /// Returns an error for any identifier other than [`ACCESS_SCHEMA`].
    pub fn parse(text: &str) -> Result<Self, CanonicalError> {
        if text == ACCESS_SCHEMA {
            Ok(Self(ACCESS_SCHEMA))
        } else {
            Err(CanonicalError::new(
                "access-schema-version",
                format!("unsupported schema {text:?}, this build reads {ACCESS_SCHEMA:?}"),
            ))
        }
    }

    /// The declared identifier.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

impl Serialize for AccessSchema {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0)
    }
}

impl<'de> Deserialize<'de> for AccessSchema {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(D::Error::custom)
    }
}

/// How long the bytes at a path are good for.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stability {
    /// The bytes never change and the path never disappears. A consumer may
    /// cache it without revalidating, and a correction appends a new path rather
    /// than editing this one.
    Immutable,
    /// The path is stable and its bytes change when the catalogue does. A
    /// consumer refetches it and compares the digest.
    Current,
}

impl Stability {
    /// The published spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Immutable => "immutable",
            Self::Current => "current",
        }
    }
}

impl fmt::Display for Stability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Which published document proves a path's bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Integrity {
    /// `MANIFEST.json` carries this path's digest, and `SHA256SUMS` does too.
    Manifest,
    /// `SHA256SUMS` carries it and the manifest cannot: this is the manifest.
    Checksums,
    /// ⛔ Neither document covers this file, because it is the one that covers
    /// the others and no document states its own digest. A consumer verifies it
    /// out of band, from the release asset listing or from a digest it recorded
    /// on a previous fetch.
    OutOfBand,
}

impl Integrity {
    /// The published spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Manifest => "manifest",
            Self::Checksums => "checksums",
            Self::OutOfBand => "out_of_band",
        }
    }
}

impl fmt::Display for Integrity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What a published path carries.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PathKind {
    /// One measured record.
    Record,
    /// One capture run's manifest.
    Run,
    /// One content-addressed evidence artifact.
    Evidence,
    /// A derived lookup document.
    Index,
    /// A derived consumer-facing rendering.
    Format,
    /// The release manifest.
    Manifest,
    /// The checksum file.
    Checksums,
    /// The licence.
    Licence,
}

impl PathKind {
    /// Every kind, so a test can hold the two spellings in step.
    pub const ALL: &'static [Self] = &[
        Self::Record,
        Self::Run,
        Self::Evidence,
        Self::Index,
        Self::Format,
        Self::Manifest,
        Self::Checksums,
        Self::Licence,
    ];

    /// The published spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Record => "record",
            Self::Run => "run",
            Self::Evidence => "evidence",
            Self::Index => "index",
            Self::Format => "format",
            Self::Manifest => "manifest",
            Self::Checksums => "checksums",
            Self::Licence => "licence",
        }
    }
}

impl fmt::Display for PathKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The roots a derived document is published under.
///
/// ⚠ Recognised rather than enumerated as whole paths, because `PUB-03` and
/// `CORPUS-03` decide what sits under each and this decides only what a consumer
/// may assume about it.
const INDEX_ROOT: &str = "indexes/";
const FORMAT_ROOT: &str = "formats/";

/// One published path, and everything a consumer needs to fetch it safely.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccessPath {
    /// Where it sits, relative to the branch root.
    pub path: RelPath,
    /// What it carries.
    pub kind: PathKind,
    /// How long its bytes are good for.
    pub stability: Stability,
    /// Which published document proves them.
    pub integrity: Integrity,
    /// The digest a consumer compares against.
    pub sha256: Sha256Digest,
}

/// The whole contract for one publication.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AccessContract {
    /// The schema identifier, read before anything else.
    pub schema: AccessSchema,
    /// The branch the paths are relative to.
    pub branch: String,
    /// Every published path, ascending.
    pub paths: Vec<AccessPath>,
}

/// The same shape with the derive on it, so [`AccessContract`]'s own
/// `Deserialize` validates.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AccessContractFields {
    schema: AccessSchema,
    branch: String,
    paths: Vec<AccessPath>,
}

impl From<AccessContractFields> for AccessContract {
    fn from(fields: AccessContractFields) -> Self {
        Self {
            schema: fields.schema,
            branch: fields.branch,
            paths: fields.paths,
        }
    }
}

impl<'de> Deserialize<'de> for AccessContract {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;
        let contract = Self::from(AccessContractFields::deserialize(deserializer)?);
        validate_contract(&contract).map_err(D::Error::custom)?;
        Ok(contract)
    }
}

impl AccessContract {
    /// Reads and validates a contract document.
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
        if probe.schema != ACCESS_SCHEMA {
            return Err(DocumentError::UnsupportedSchema {
                found: probe.schema,
                expected: ACCESS_SCHEMA,
            });
        }
        let fields: AccessContractFields =
            serde_json::from_str(document).map_err(DocumentError::Malformed)?;
        let contract = Self::from(fields);
        validate_contract(&contract).map_err(DocumentError::Invalid)?;
        Ok(contract)
    }

    /// Writes the contract in the canonical form.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError::Invalid`] with the refused invariants.
    pub fn to_json(&self) -> Result<String, DocumentError> {
        validate_contract(self).map_err(DocumentError::Invalid)?;
        let mut out = serde_json::to_string_pretty(self).map_err(DocumentError::Malformed)?;
        out.push('\n');
        Ok(out)
    }

    /// Every path a consumer may cache without revalidating.
    #[must_use]
    pub fn immutable(&self) -> Vec<&AccessPath> {
        self.with_stability(Stability::Immutable)
    }

    /// Every path whose bytes move when the catalogue does.
    #[must_use]
    pub fn current(&self) -> Vec<&AccessPath> {
        self.with_stability(Stability::Current)
    }

    fn with_stability(&self, stability: Stability) -> Vec<&AccessPath> {
        self.paths
            .iter()
            .filter(|entry| entry.stability == stability)
            .collect()
    }
}

/// Classifies one published path.
///
/// ⛔ **It fails closed.** A path this build cannot classify blocks the contract
/// rather than being published with a guessed stability, for
/// [`crate::release`]'s reason about a media type: a consumer told it may cache
/// a file that moves will serve a stale measurement, and the failure lands on
/// the reader.
fn classify(path: &RelPath) -> Option<(PathKind, Stability)> {
    let text = path.as_str();
    if text == RELEASE_MANIFEST_FILE {
        return Some((PathKind::Manifest, Stability::Current));
    }
    if text == CHECKSUMS_FILE {
        return Some((PathKind::Checksums, Stability::Current));
    }
    if text == LICENSE_FILE {
        return Some((PathKind::Licence, Stability::Current));
    }
    if text.starts_with(INDEX_ROOT) {
        return Some((PathKind::Index, Stability::Current));
    }
    if text.starts_with(FORMAT_ROOT) {
        return Some((PathKind::Format, Stability::Current));
    }
    // ⛔ The measurement roots, and the stability comes from the same predicate
    // the append-only rule uses rather than from a second reading of the layout.
    if crate::store::is_profile_path(path) {
        return Some((PathKind::Record, Stability::Immutable));
    }
    if crate::store::is_manifest_path(path) {
        return Some((PathKind::Run, Stability::Immutable));
    }
    if is_canonical_path(path) {
        return Some((PathKind::Evidence, Stability::Immutable));
    }
    None
}

/// Which document proves a path, which is a function of the path alone.
const fn integrity_of(kind: PathKind) -> Integrity {
    match kind {
        // The manifest cannot state its own digest, so the checksum file does.
        PathKind::Manifest => Integrity::Checksums,
        // And nothing states the checksum file's.
        PathKind::Checksums => Integrity::OutOfBand,
        _ => Integrity::Manifest,
    }
}

/// Derives the contract for an assembled release.
///
/// `manifest` and `checksums` are the two documents [`crate::release`] produces
/// after assembly, passed in as bytes because their digests are what a consumer
/// checks and neither is in [`Release::entries`].
///
/// ⛔ **The two root documents are added here rather than assumed present.**
/// `assemble` describes everything except them, so a contract built from the
/// entries alone would omit exactly the two files a consumer needs first, and
/// omit them silently.
///
/// # Errors
///
/// | code | refused |
/// | --- | --- |
/// | `E-ACC-01` | a published path this build cannot classify |
/// | `E-ACC-02` | a contract missing one of the two root documents |
/// | `E-ACC-03` | paths out of order, or one carried twice |
/// | `E-ACC-04` | a stability class that disagrees with `CANONICAL_ROOTS` |
/// | `E-ACC-05` | an integrity source that is not the one the path implies |
pub fn contract(
    release: &Release,
    manifest: &[u8],
    checksums: &[u8],
) -> Result<AccessContract, Violations> {
    let mut errors = Vec::new();
    let mut paths: Vec<AccessPath> = Vec::new();

    for entry in release.entries() {
        let Some((kind, stability)) = classify(&entry.path) else {
            errors.push(SchemaError::new(
                "E-ACC-01",
                entry.path.as_str(),
                "this build cannot say how long a consumer may cache this path",
            ));
            continue;
        };
        paths.push(AccessPath {
            path: entry.path.clone(),
            kind,
            stability,
            integrity: integrity_of(kind),
            sha256: entry.sha256,
        });
    }

    for (name, bytes) in [
        (RELEASE_MANIFEST_FILE, manifest),
        (CHECKSUMS_FILE, checksums),
    ] {
        let Ok(path) = RelPath::parse(name) else {
            errors.push(SchemaError::new(
                "E-ACC-02",
                name,
                "the root document's name is not a relative path",
            ));
            continue;
        };
        let Some((kind, stability)) = classify(&path) else {
            errors.push(SchemaError::new(
                "E-ACC-02",
                name,
                "the root document is not classified",
            ));
            continue;
        };
        paths.push(AccessPath {
            path,
            kind,
            stability,
            integrity: integrity_of(kind),
            sha256: Sha256Digest::of(bytes),
        });
    }

    paths.sort_by(|left, right| left.path.as_str().cmp(right.path.as_str()));
    let contract = AccessContract {
        schema: AccessSchema::current(),
        branch: DATA_BRANCH.to_owned(),
        paths,
    };
    if !errors.is_empty() {
        return Violations::from_errors(errors).map(|()| unreachable!());
    }
    validate_contract(&contract)?;
    Ok(contract)
}

/// The invariants a contract document must satisfy.
///
/// # Errors
///
/// Returns the refusals, each with a stable code. The table is on [`contract`].
pub fn validate_contract(contract: &AccessContract) -> Result<(), Violations> {
    let mut out = Vec::new();

    if contract.branch != DATA_BRANCH {
        out.push(SchemaError::new(
            "E-ACC-02",
            "branch",
            format!(
                "the contract is for {DATA_BRANCH:?} and declares {:?}",
                contract.branch
            ),
        ));
    }
    if let Some(index) = strictly_ascending(&contract.paths, |entry| entry.path.to_string()) {
        out.push(SchemaError::new(
            "E-ACC-03",
            format!("paths[{index}]"),
            format!(
                "paths must be unique and ascending, found {}",
                contract.paths[index].path
            ),
        ));
    }

    let names: BTreeSet<&str> = contract
        .paths
        .iter()
        .map(|entry| entry.path.as_str())
        .collect();
    // ⛔ A consumer needs both root documents to check anything at all, so a
    // contract without them describes a publication nobody can verify.
    for required in [RELEASE_MANIFEST_FILE, CHECKSUMS_FILE] {
        if !names.contains(required) {
            out.push(SchemaError::new(
                "E-ACC-02",
                "paths",
                format!("the contract carries no {required}"),
            ));
        }
    }

    for (index, entry) in contract.paths.iter().enumerate() {
        // ⛔ THE ONE RULE THIS DOCUMENT EXISTS FOR. A path under a canonical root
        // carries a measurement and never changes; anything else is derived and
        // exists in order to change. A contract that said otherwise would tell a
        // consumer to cache a file that moves, or to refetch one that cannot.
        let canonical = is_canonical_path(&entry.path);
        let immutable = entry.stability == Stability::Immutable;
        if canonical != immutable {
            out.push(SchemaError::new(
                "E-ACC-04",
                format!("paths[{index}].stability"),
                format!(
                    "{} is {} a canonical root and is declared {}",
                    entry.path,
                    if canonical { "under" } else { "outside" },
                    entry.stability
                ),
            ));
        }
        let expected = integrity_of(entry.kind);
        if entry.integrity != expected {
            out.push(SchemaError::new(
                "E-ACC-05",
                format!("paths[{index}].integrity"),
                format!(
                    "a {} is proved by {expected} and this declares {}",
                    entry.kind, entry.integrity
                ),
            ));
        }
    }

    Violations::from_errors(out)
}

#[cfg(test)]
mod tests {
    use super::{
        ACCESS_SCHEMA, DATA_BRANCH, Integrity, PathKind, Stability, classify, integrity_of,
    };
    use crate::canonical::RelPath;

    fn path(text: &str) -> RelPath {
        RelPath::parse(text).expect("path")
    }

    #[test]
    fn a_measurement_path_is_immutable_and_a_derived_one_is_not() {
        // ⛔ The two roots the append-only rule names, and one of each of the
        // derived kinds. A path that moved between these two answers is the
        // defect the whole document exists to prevent.
        let immutable = [
            "profiles/v1/t/1.2.3/linux/x86-64/deb/cap-1.json",
            "raw/v1/t/1.2.3/linux/x86-64/deb/cap-1/manifest.json",
            "raw/v1/t/1.2.3/linux/x86-64/deb/cap-1/objects/sha256/ab/cd.bin",
        ];
        for text in immutable {
            let (_, stability) = classify(&path(text)).expect("classified");
            assert_eq!(stability, Stability::Immutable, "{text}");
        }
        let current = [
            "MANIFEST.json",
            "SHA256SUMS",
            "LICENSE",
            "indexes/v1/profiles.json",
            "formats/bit-ids-v1.json",
        ];
        for text in current {
            let (_, stability) = classify(&path(text)).expect("classified");
            assert_eq!(stability, Stability::Current, "{text}");
        }
    }

    #[test]
    fn a_path_this_build_cannot_classify_blocks_rather_than_defaulting() {
        // ⚠ `routes/` is the live example: `docs/publishing.md` documented it and
        // nothing writes it. A default would have published it with a stability
        // nobody decided.
        assert!(classify(&path("routes/v1/t/latest/linux/x86-64.json")).is_none());
        assert!(classify(&path("README.md")).is_none());
    }

    #[test]
    fn exactly_one_published_file_is_covered_by_neither_document() {
        // ⛔ The manifest cannot state its own digest and the checksum file
        // cannot state its own, so the checksum file is the one a consumer has to
        // verify from outside the publication. A second `out_of_band` would be a
        // second unverifiable file.
        let out_of_band: Vec<PathKind> = PathKind::ALL
            .iter()
            .copied()
            .filter(|kind| integrity_of(*kind) == Integrity::OutOfBand)
            .collect();
        assert_eq!(out_of_band, vec![PathKind::Checksums]);
        assert_eq!(integrity_of(PathKind::Manifest), Integrity::Checksums);
        assert_eq!(integrity_of(PathKind::Record), Integrity::Manifest);
    }

    #[test]
    fn the_vocabularies_agree_with_their_serialized_forms() {
        for kind in PathKind::ALL {
            let json = serde_json::to_string(kind).expect("serialize");
            assert_eq!(json, format!("\"{}\"", kind.as_str()));
        }
        for stability in [Stability::Immutable, Stability::Current] {
            let json = serde_json::to_string(&stability).expect("serialize");
            assert_eq!(json, format!("\"{}\"", stability.as_str()));
        }
        for integrity in [
            Integrity::Manifest,
            Integrity::Checksums,
            Integrity::OutOfBand,
        ] {
            let json = serde_json::to_string(&integrity).expect("serialize");
            assert_eq!(json, format!("\"{}\"", integrity.as_str()));
        }
    }

    #[test]
    fn the_schema_and_the_branch_have_one_spelling_each() {
        assert_eq!(ACCESS_SCHEMA, "bit-ids/access/1");
        assert_eq!(DATA_BRANCH, "data");
    }
}
