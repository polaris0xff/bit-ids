//! Reading a published catalogue, without a network and without reimplementing
//! anything.
//!
//! `LIB-01` owns this. A Rust tool that wants the measured identity of a build
//! should not carry its own copy of the schema, the digest checks, the index
//! shape or the selection rule; every one of those is already in this crate, and
//! a second copy in a consumer is the one that answers wrongly the day it
//! drifts.
//!
//! ⛔ **NOTHING HERE OPENS A SOCKET, AND THAT IS STRUCTURAL RATHER THAN A
//! PROMISE.** A [`Catalogue`] is opened over bytes the caller already holds:
//! from a directory, from an embedded blob, or from a fetch the caller
//! performed. Opt-in retrieval is [`Retrieval`], a trait the caller implements,
//! so the fetching lives in the consumer and the *verifying* lives here. A
//! library that fetched would need an HTTP client in a workspace that has argued
//! for every dependency it has, and "no network by default" would be a flag
//! somebody could set rather than a thing the code cannot do.
//!
//! ⛔ **Every byte is verified before any question is answered.** `MANIFEST.json`
//! describes the bundle, so a catalogue is opened by checking every file against
//! it and refusing on the first mismatch. A consumer that could read a record
//! out of an unverified bundle is a consumer that can be handed a measurement
//! nobody made.
//!
//! ⚠ **The manifest itself is checked against `SHA256SUMS`**, and `SHA256SUMS`
//! is checked against a digest the caller supplies or against nothing. `PUB-04`
//! says why: no document states its own digest, so exactly one file in a
//! publication has to be trusted from outside it, and this is the one.
//!
//! ⛔ **Which records are published is the index document's answer, not a
//! filter here.** `PUB-03` found that shape once already: a renderer that
//! filtered a store on its own kept publishing a record the lookups had stopped
//! naming.

use std::collections::BTreeMap;

use crate::access::{Integrity, Stability, contract};
use crate::canonical::{RelPath, Sha256Digest, Slug, Version};
use crate::identity::RecordId;
use crate::index::{Index, IndexKind, Indexes};
use crate::record::Profile;
use crate::release::{CHECKSUMS_FILE, RELEASE_MANIFEST_FILE, Release, assemble};
use crate::store::{Entry, ObjectRef, StoreTree, is_profile_path};
use crate::validate::{SchemaError, Violations};

/// The published index document's path inside a bundle.
pub const INDEX_PATH: &str = "indexes/v1/profiles.json";

/// The bytes of one published bundle, as a caller assembled them.
///
/// ⚠ A map rather than a directory, because the same catalogue may arrive from
/// disk, from an archive, from an embedded blob or from a caller's own fetch,
/// and the verification is identical for all four. The filesystem stays out of
/// this crate, which is the rule `docs/architecture.md` section 3 states.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Bundle {
    files: BTreeMap<String, Vec<u8>>,
}

impl Bundle {
    /// An empty bundle.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one file. A path added twice keeps the later bytes.
    pub fn insert(&mut self, path: impl Into<String>, bytes: impl Into<Vec<u8>>) {
        self.files.insert(path.into(), bytes.into());
    }

    /// The bytes at one path.
    #[must_use]
    pub fn get(&self, path: &str) -> Option<&[u8]> {
        self.files.get(path).map(Vec::as_slice)
    }

    /// Every path, ascending.
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(String::as_str)
    }

    /// How many files it carries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Whether it carries nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

/// How a caller retrieves a published path, when it chooses to.
///
/// ⭐ **The whole of "opt-in verified retrieval".** The caller performs the
/// fetch, by whatever route it already trusts, and [`retrieve`] verifies what
/// came back against the digest the contract states. So this crate never learns
/// a host, never opens a socket, and still refuses bytes that do not match.
pub trait Retrieval {
    /// The error a caller's transport reports.
    type Error: core::fmt::Display;

    /// Fetches one published path, relative to the branch root.
    ///
    /// # Errors
    ///
    /// Whatever the caller's transport reports.
    fn fetch(&self, path: &str) -> Result<Vec<u8>, Self::Error>;
}

/// A verified, in-memory view of one published catalogue.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Catalogue {
    records: BTreeMap<RecordId, (RelPath, Profile)>,
    indexes: Indexes,
}

impl Catalogue {
    /// Opens a bundle, verifying every byte before answering anything.
    ///
    /// `checksums_digest` is the digest of `SHA256SUMS` if the caller has one
    /// from outside the publication. ⚠ Passing `None` is the honest default and
    /// not a weaker mode: nothing in a publication can prove that file, so a
    /// caller with no out-of-band digest is trusting it, and saying so in the
    /// signature is better than a flag that reads as optional strictness.
    ///
    /// # Errors
    ///
    /// | code | refused |
    /// | --- | --- |
    /// | `E-LIB-01` | a file the manifest describes that the bundle does not carry |
    /// | `E-LIB-02` | a file whose bytes disagree with the manifest |
    /// | `E-LIB-03` | a file the bundle carries that the manifest does not describe |
    /// | `E-LIB-04` | a manifest that disagrees with `SHA256SUMS` |
    /// | `E-LIB-05` | a `SHA256SUMS` that disagrees with the caller's digest |
    /// | `E-LIB-06` | a record, run or index document that will not parse |
    /// | `E-LIB-07` | an index row naming a record the bundle does not carry |
    /// | `E-LIB-08` | a bundle with no manifest, no checksums or no index |
    pub fn open(
        bundle: &Bundle,
        checksums_digest: Option<&Sha256Digest>,
    ) -> Result<Self, Violations> {
        let mut errors = Vec::new();
        let Some((manifest, checksums)) = roots(bundle, &mut errors) else {
            return Violations::from_errors(errors).map(|()| unreachable!());
        };

        // ⚠ Out of band first, because it is the only thing that can establish
        // the checksum file, and everything below leans on it.
        if let Some(expected) = checksums_digest {
            let found = Sha256Digest::of(checksums);
            if found != *expected {
                errors.push(SchemaError::new(
                    "E-LIB-05",
                    CHECKSUMS_FILE,
                    format!("the caller expects {expected} and the bundle carries {found}"),
                ));
            }
        }

        let release = describe(bundle, &mut errors);
        if let Some(release) = &release {
            check_files(bundle, release, &mut errors);
            // ⛔ The manifest is proved by the checksum file and by nothing
            // else, because no document states its own digest. `PUB-04`'s
            // `Integrity::Checksums` is the same fact typed.
            let rebuilt = release.checksums(manifest);
            if rebuilt.as_bytes() != checksums {
                errors.push(SchemaError::new(
                    "E-LIB-04",
                    CHECKSUMS_FILE,
                    "the checksum file does not cover this manifest and these bytes",
                ));
            }
        }
        if !errors.is_empty() {
            return Violations::from_errors(errors).map(|()| unreachable!());
        }

        let records = read_records(bundle, &mut errors);
        let indexes = read_indexes(bundle, &mut errors);
        if !errors.is_empty() {
            return Violations::from_errors(errors).map(|()| unreachable!());
        }
        let indexes = indexes.unwrap_or_else(|| unreachable!("checked above"));

        // ⛔ Every row is resolved against the records the bundle actually
        // carries. An index is read *instead of* the records, so a row naming
        // one nobody can open is an answer with nothing behind it, and a
        // consumer is exactly the reader who would never notice.
        for index in &indexes.indexes {
            for row in &index.rows {
                if !records.contains_key(&row.record) {
                    errors.push(SchemaError::new(
                        "E-LIB-07",
                        format!("{}[{}]", index.kind.as_str(), row.key),
                        format!("names {}, which this bundle does not carry", row.record),
                    ));
                }
            }
        }
        for row in &indexes.latest {
            if !records.contains_key(&row.record) {
                errors.push(SchemaError::new(
                    "E-LIB-07",
                    format!("latest[{} {}]", row.target, row.platform),
                    format!("names {}, which this bundle does not carry", row.record),
                ));
            }
        }
        Violations::from_errors(errors)?;
        Ok(Self { records, indexes })
    }

    /// Every record the catalogue publishes, ascending by identifier.
    #[must_use]
    pub fn records(&self) -> Vec<&Profile> {
        self.records.values().map(|(_, profile)| profile).collect()
    }

    /// One record by identifier.
    #[must_use]
    pub fn record(&self, id: RecordId) -> Option<&Profile> {
        self.records.get(&id).map(|(_, profile)| profile)
    }

    /// The published views, read from the bundle rather than re-derived.
    #[must_use]
    pub const fn indexes(&self) -> &Indexes {
        &self.indexes
    }

    /// The newest measured record for one build line.
    ///
    /// ⛔ **This is the latest view's answer and not a second selection.** The
    /// ordering a latest row rests on is `ACQ-02`'s version scheme, which a
    /// bundle does not carry, so a consumer re-deriving it would need a scheme
    /// nobody published and would answer differently the day it guessed one.
    #[must_use]
    pub fn latest(
        &self,
        target: &Slug,
        platform: &Slug,
        arch: &Slug,
        package: &Slug,
    ) -> Option<&Profile> {
        let row = self.indexes.latest.iter().find(|row| {
            row.target == *target
                && row.platform == *platform
                && row.arch == *arch
                && row.package == *package
        })?;
        self.record(row.record)
    }

    /// Every record measured at one version of one target.
    #[must_use]
    pub fn at_version(&self, target: &Slug, version: &Version) -> Vec<&Profile> {
        self.records()
            .into_iter()
            .filter(|profile| profile.target.id == *target && profile.build.version == *version)
            .collect()
    }

    /// Every record one lookup key resolves to.
    ///
    /// ⛔ **The peer-prefix lookup is `CORPUS-03`'s inversion of the decoder
    /// table, not an exception to it.** The key is the fixed span of a peer ID
    /// this project measured, and it resolves to the record that measured it.
    #[must_use]
    pub fn lookup(&self, kind: IndexKind, key: &str) -> Vec<&Profile> {
        self.index(kind)
            .map(|index| {
                index
                    .rows
                    .iter()
                    .filter(|row| row.key == key)
                    .filter_map(|row| self.record(row.record))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// One index, when the bundle carries it.
    #[must_use]
    pub fn index(&self, kind: IndexKind) -> Option<&Index> {
        self.indexes.indexes.iter().find(|index| index.kind == kind)
    }

    /// What answers now for a record a consumer still holds the identifier of.
    ///
    /// Returns the identifier itself when nothing corrects it, and the end of
    /// the correction chain when something does. ⚠ `CORPUS-04` publishes
    /// `current` rather than only `by`, so this is one lookup rather than a walk
    /// a consumer would have to fetch record by record.
    #[must_use]
    pub fn current(&self, id: RecordId) -> Option<RecordId> {
        self.indexes
            .corrections
            .iter()
            .find(|row| row.superseded == id)
            .map_or_else(
                || self.records.contains_key(&id).then_some(id),
                |row| Some(row.current),
            )
    }
}

/// Fetches one published path through a caller's transport and verifies it.
///
/// ⭐ **The verification is the library's half and the fetching is the
/// caller's.** `stability` and `sha256` come from `PUB-04`'s contract, so a
/// caller is told before the fetch whether the bytes may be cached, and after it
/// whether they are the ones the publication describes.
///
/// # Errors
///
/// Returns the transport's message, or a mismatch naming both digests.
pub fn retrieve<R: Retrieval>(
    transport: &R,
    path: &str,
    expected: &Sha256Digest,
) -> Result<Vec<u8>, String> {
    let bytes = transport
        .fetch(path)
        .map_err(|error| format!("{path}: {error}"))?;
    let found = Sha256Digest::of(&bytes);
    if found == *expected {
        Ok(bytes)
    } else {
        Err(format!("{path}: expected {expected}, retrieved {found}"))
    }
}

/// What a consumer is told about a path before it fetches one.
///
/// ⚠ Derived from the bundle's own manifest, so a caller planning fetches from
/// a publication it already holds gets the same answer `PUB-04` publishes.
///
/// # Errors
///
/// Returns the contract's refusals.
pub fn plan(
    bundle: &Bundle,
) -> Result<Vec<(String, Stability, Integrity, Sha256Digest)>, Violations> {
    let mut errors = Vec::new();
    let release = describe(bundle, &mut errors);
    let Some((manifest, checksums)) = roots(bundle, &mut errors) else {
        return Violations::from_errors(errors).map(|()| unreachable!());
    };
    let Some(release) = release else {
        return Violations::from_errors(errors).map(|()| unreachable!());
    };
    let access = contract(&release, manifest, checksums)?;
    Ok(access
        .paths
        .into_iter()
        .map(|entry| {
            (
                entry.path.as_str().to_owned(),
                entry.stability,
                entry.integrity,
                entry.sha256,
            )
        })
        .collect())
}

/// The two root documents, or the refusal that they are missing.
fn roots<'a>(bundle: &'a Bundle, errors: &mut Vec<SchemaError>) -> Option<(&'a [u8], &'a [u8])> {
    let manifest = bundle.get(RELEASE_MANIFEST_FILE);
    let checksums = bundle.get(CHECKSUMS_FILE);
    for (name, found) in [
        (RELEASE_MANIFEST_FILE, manifest.is_some()),
        (CHECKSUMS_FILE, checksums.is_some()),
        (INDEX_PATH, bundle.get(INDEX_PATH).is_some()),
    ] {
        if !found {
            errors.push(SchemaError::new(
                "E-LIB-08",
                name,
                "a publication without it cannot be read",
            ));
        }
    }
    manifest.zip(checksums)
}

/// Re-describes the bundle from its own bytes, which is what every digest below
/// is compared against.
///
/// ⛔ **Re-derived rather than parsed out of the manifest.** A consumer that
/// read the manifest's digests and compared them against the manifest's digests
/// would agree with itself over any bytes at all.
fn describe(bundle: &Bundle, errors: &mut Vec<SchemaError>) -> Option<Release> {
    let mut tree = StoreTree::new();
    for path in bundle.paths() {
        if path == RELEASE_MANIFEST_FILE || path == CHECKSUMS_FILE {
            continue;
        }
        let Ok(relative) = RelPath::parse(path) else {
            errors.push(SchemaError::new(
                "E-LIB-03",
                path,
                "not a relative path a publication can carry",
            ));
            continue;
        };
        let bytes = bundle.get(path).unwrap_or_default();
        tree.insert(
            relative,
            Entry::Object(ObjectRef {
                bytes: bytes.len() as u64,
                sha256: Sha256Digest::of(bytes),
            }),
        );
    }
    match assemble(&tree) {
        Ok(release) => Some(release),
        Err(violations) => {
            for error in violations.errors() {
                errors.push(SchemaError::new("E-LIB-03", "bundle", error.to_string()));
            }
            None
        }
    }
}

/// Compares the bundle against what the manifest describes, in both directions.
///
/// ⛔ **The published manifest is read, not re-derived.** A consumer comparing a
/// bundle against a manifest it built *from that bundle* agrees with itself over
/// any bytes at all: a described file that is missing would not be described
/// either, and a carried file nobody described would be described. Both
/// refusals existed and neither could fire until a mutation pass planted them.
/// The re-derivation is still made and compared against the published document,
/// because that is what catches a manifest describing another publication.
fn check_files(bundle: &Bundle, release: &Release, errors: &mut Vec<SchemaError>) {
    let manifest = bundle.get(RELEASE_MANIFEST_FILE).unwrap_or_default();
    let published = match core::str::from_utf8(manifest).map(Release::from_json) {
        Ok(Ok(published)) => published,
        Ok(Err(error)) => {
            errors.push(SchemaError::new(
                "E-LIB-06",
                RELEASE_MANIFEST_FILE,
                error.to_string(),
            ));
            return;
        }
        Err(_) => {
            errors.push(SchemaError::new(
                "E-LIB-06",
                RELEASE_MANIFEST_FILE,
                "not UTF-8",
            ));
            return;
        }
    };
    // ⛔ The published manifest against one re-derived from the bytes. This is
    // what refuses a manifest that describes a different publication, and it
    // fires on every mismatch below as well; the per-file codes are what say
    // *which* mismatch it was.
    if release.manifest_json().as_bytes() != manifest {
        errors.push(SchemaError::new(
            "E-LIB-02",
            RELEASE_MANIFEST_FILE,
            "does not describe the bytes in this bundle",
        ));
    }
    let described: BTreeMap<&str, &Sha256Digest> = published
        .entries()
        .iter()
        .map(|entry| (entry.path.as_str(), &entry.sha256))
        .collect();
    for (path, digest) in &described {
        match bundle.get(path) {
            None => errors.push(SchemaError::new(
                "E-LIB-01",
                *path,
                "described by the manifest and not carried",
            )),
            Some(bytes) => {
                let found = Sha256Digest::of(bytes);
                if found != **digest {
                    errors.push(SchemaError::new(
                        "E-LIB-02",
                        *path,
                        format!("described as {digest} and carries {found}"),
                    ));
                }
            }
        }
    }
    // ⛔ And the other direction. A file nobody described is bytes a consumer
    // would have no digest for, which is the half a one-way comparison misses.
    for path in bundle.paths() {
        if path == RELEASE_MANIFEST_FILE || path == CHECKSUMS_FILE {
            continue;
        }
        if !described.contains_key(path) {
            errors.push(SchemaError::new(
                "E-LIB-03",
                path,
                "carried and described by nothing",
            ));
        }
    }
}

fn read_records(
    bundle: &Bundle,
    errors: &mut Vec<SchemaError>,
) -> BTreeMap<RecordId, (RelPath, Profile)> {
    let mut records = BTreeMap::new();
    for path in bundle.paths() {
        let Ok(relative) = RelPath::parse(path) else {
            continue;
        };
        if !is_profile_path(&relative) {
            continue;
        }
        let bytes = bundle.get(path).unwrap_or_default();
        let Ok(document) = core::str::from_utf8(bytes) else {
            errors.push(SchemaError::new("E-LIB-06", path, "not UTF-8"));
            continue;
        };
        // ⛔ Through `Profile::from_json`, so a consumer cannot hold a record
        // this project would refuse to publish. There is no faster path here
        // and there should not be one.
        match Profile::from_json(document) {
            Ok(profile) => {
                records.insert(profile.id, (relative, profile));
            }
            Err(error) => errors.push(SchemaError::new("E-LIB-06", path, error.to_string())),
        }
    }
    records
}

fn read_indexes(bundle: &Bundle, errors: &mut Vec<SchemaError>) -> Option<Indexes> {
    let bytes = bundle.get(INDEX_PATH)?;
    let Ok(document) = core::str::from_utf8(bytes) else {
        errors.push(SchemaError::new("E-LIB-06", INDEX_PATH, "not UTF-8"));
        return None;
    };
    // ⚠ One code for both an unreadable document and one from another
    // generation. They are the same thing to a consumer: this build cannot read
    // this bundle's index, and `DocumentError`'s own message says which.
    match Indexes::from_json(document) {
        Ok(indexes) => Some(indexes),
        Err(error) => {
            errors.push(SchemaError::new("E-LIB-06", INDEX_PATH, error.to_string()));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Bundle, INDEX_PATH, Retrieval, retrieve};
    use crate::canonical::Sha256Digest;

    struct Canned(Vec<u8>);

    impl Retrieval for Canned {
        type Error = String;
        fn fetch(&self, _path: &str) -> Result<Vec<u8>, String> {
            Ok(self.0.clone())
        }
    }

    struct Broken;

    impl Retrieval for Broken {
        type Error = String;
        fn fetch(&self, _path: &str) -> Result<Vec<u8>, String> {
            Err("the transport refused".to_owned())
        }
    }

    #[test]
    fn a_retrieval_that_returns_other_bytes_is_refused() {
        let wanted = b"the published bytes";
        let digest = Sha256Digest::of(wanted);
        assert_eq!(
            retrieve(&Canned(wanted.to_vec()), "x", &digest).expect("matching bytes"),
            wanted
        );
        // ⛔ The whole point of opt-in retrieval: the caller fetched, and this
        // is what decides whether what came back is the publication's.
        let error = retrieve(&Canned(b"something else".to_vec()), "x", &digest)
            .expect_err("different bytes are refused");
        assert!(error.contains("expected"), "{error}");
    }

    #[test]
    fn a_transport_error_is_reported_and_never_treated_as_empty_bytes() {
        // ⚠ Empty bytes have a digest, so a retrieval that swallowed an error
        // and returned nothing would be refused for the wrong reason, or
        // accepted if the publication happened to carry an empty file.
        let digest = Sha256Digest::of(b"");
        let error = retrieve(&Broken, "x", &digest).expect_err("a transport error is an error");
        assert!(error.contains("the transport refused"), "{error}");
    }

    #[test]
    fn a_bundle_is_a_map_and_keeps_the_later_bytes() {
        let mut bundle = Bundle::new();
        assert!(bundle.is_empty());
        bundle.insert(INDEX_PATH, b"first".to_vec());
        bundle.insert(INDEX_PATH, b"second".to_vec());
        assert_eq!(bundle.len(), 1);
        assert_eq!(bundle.get(INDEX_PATH), Some(b"second".as_slice()));
    }
}
