//! The append-only store: where a record is filed, and what may change once it
//! has been.
//!
//! `CORPUS-01` owns this. The published tree in `docs/publishing.md` is the
//! layout; this module is the part of it that is checkable, being where a
//! record's path comes from and what a successor tree may do to a path that is
//! already published.
//!
//! Two rules carry the whole thing.
//!
//! ⛔ **A path is derived from the record's identity tuple, never chosen.** It
//! is the same tuple [`crate::identity::RecordId`] digests, in full. A path
//! built from fewer components files two different measurements at one name and
//! one of them silently wins; a path built from more files one measurement under
//! two names, which the append-only rule then protects both of. Neither is
//! recoverable after the fact, because the loser was never written.
//!
//! ⛔ **A published path never changes and never disappears.** That is the
//! whole product: a catalogue that regenerated a latest-only view would erase
//! the older stable releases it exists to have measured. Corrections append a
//! record carrying `supersedes`; they do not edit the record they correct.
//!
//! The rules here are pure functions over a tree that is already in memory.
//! Walking a directory to build one is the caller's, which keeps this crate
//! free of the filesystem the way the rest of it is; `examples/check-store.rs`
//! is that caller and is the driving surface for this entry.

use std::collections::{BTreeMap, BTreeSet};

use crate::canonical::{RelPath, Sha256Digest, Slug, Version};
use crate::manifest::RunManifest;
use crate::record::Profile;
use crate::validate::{SchemaError, Violations};

/// The layout generation every store path carries.
///
/// ⚠ It stands for the schema generation rather than repeating it.
/// [`crate::PROFILE_SCHEMA`] parses to exactly one value, so the two cannot
/// disagree today, and a second record generation gets its own layout root
/// rather than mixing two shapes under one. `layout_generation_tracks_schema`
/// pins the pair to their literals so a bumped schema cannot leave this behind.
pub const STORE_LAYOUT: &str = "v1";

/// Where a profile record is filed.
pub const PROFILE_ROOT: &str = "profiles";

/// Where a run's manifest and raw evidence are filed.
pub const RAW_ROOT: &str = "raw";

/// The manifest's name inside a run's bundle directory.
///
/// The manifest sits beside the bytes it describes rather than inside the
/// profile, per `docs/architecture.md` section 4: a replay needs the whole run
/// and a consumer of the catalogue needs only the record.
pub const MANIFEST_FILE: &str = "manifest.json";

/// The extension a profile record is written under.
pub const PROFILE_EXT: &str = ".json";

/// Names Windows resolves to a device rather than to a file, whatever the
/// extension, and whatever case they are spelled in.
///
/// A directory called `nul` cannot be created there at all, so a store carrying
/// one is a store that half of the capture matrix cannot check out. The list is
/// the classic set; `com0` and `lpt0` are included because the shell resolves
/// them the same way even though the hardware never existed.
const RESERVED_STEMS: [&str; 24] = [
    "con", "prn", "aux", "nul", "com0", "com1", "com2", "com3", "com4", "com5", "com6", "com7",
    "com8", "com9", "lpt0", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Why one path segment cannot be published, or `None` when it can be.
///
/// ⛔ **One rule, two callers, on purpose.** [`StoreKey::profile_path`] runs it
/// over the version before pasting it into a path, and [`validate_tree`] runs it
/// over every segment of every path a tree already carries. A rule enforced at
/// one of two doors into the same action is the most recurring hole there is,
/// and here the second door is real: a tree is read off a disk somebody else
/// wrote, so a segment can arrive without this crate ever having derived it.
fn segment_hazard(segment: &str) -> Option<String> {
    if segment.is_empty() {
        return Some("an empty segment".to_owned());
    }
    if segment.starts_with('.') {
        return Some(format!(
            "{segment:?} begins with a dot, which is a traversal or a hidden name"
        ));
    }
    if segment.ends_with('.') {
        return Some(format!(
            "{segment:?} ends with a dot, which Windows strips, so two segments become one"
        ));
    }
    if !segment
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_' || b == b'-')
    {
        return Some(format!("{segment:?} carries a byte outside A-Za-z0-9._-"));
    }
    let stem = segment.split('.').next().unwrap_or(segment);
    if RESERVED_STEMS
        .iter()
        .any(|reserved| stem.eq_ignore_ascii_case(reserved))
    {
        return Some(format!(
            "{segment:?} resolves to a Windows device rather than to a file"
        ));
    }
    None
}

/// The identity tuple a store path is derived from.
///
/// ⚠ It is [`crate::identity::RecordKey`] without the schema, which the layout
/// generation carries instead. Every other component is present because the
/// record identifier digests it: two records the identifier tells apart must be
/// two paths, and two paths the store tells apart must be two records.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StoreKey<'a> {
    /// The catalogue target.
    pub target: &'a Slug,
    /// The version the installed build reported.
    pub version: &'a Version,
    /// Host family.
    pub platform: &'a Slug,
    /// Machine architecture.
    pub arch: &'a Slug,
    /// Package format the artifact was delivered in.
    pub package: &'a Slug,
    /// The capture run.
    pub capture: &'a Slug,
}

impl<'a> StoreKey<'a> {
    /// The tuple a profile record is filed under.
    #[must_use]
    pub const fn of_profile(profile: &'a Profile) -> Self {
        Self {
            target: &profile.target.id,
            version: &profile.build.version,
            platform: &profile.build.platform,
            arch: &profile.build.arch,
            package: &profile.build.package,
            capture: &profile.capture.id,
        }
    }

    /// The tuple a run manifest is filed under.
    #[must_use]
    pub const fn of_manifest(manifest: &'a RunManifest) -> Self {
        Self {
            target: &manifest.target,
            version: &manifest.version,
            platform: &manifest.platform,
            arch: &manifest.arch,
            package: &manifest.package,
            capture: &manifest.capture,
        }
    }

    /// The version as a path segment.
    ///
    /// ⛔ **A version string is a measurement, not an identifier.**
    /// [`Version`] accepts whatever the installed build printed, because
    /// imposing a grammar on it would refuse builds that number themselves some
    /// other way, and `version_is_not_a_path_segment` pins that `../../etc`
    /// parses as one. Pasting that into a path is a traversal with the measured
    /// build choosing where it lands.
    ///
    /// So a version that cannot be a segment blocks publication rather than
    /// being mangled into one. Mangling was the alternative and it loses on the
    /// same rule the canonical forms are built on: an escape maps two versions
    /// onto one directory unless it is injective, an injective one needs bytes
    /// [`RelPath`] refuses, and a lossy one silently merges two measurements.
    fn version_segment(&self) -> Result<&'a str, SchemaError> {
        let text = self.version.as_str();
        match segment_hazard(text) {
            None => Ok(text),
            Some(detail) => Err(SchemaError::new(
                "E-STO-01",
                "build.version",
                format!("{detail}; it cannot be a store path segment"),
            )),
        }
    }

    /// The `<target>/<version>/<platform>/<arch>/<package>/<capture>` tail every
    /// path in this run's store shares.
    fn tail(&self) -> Result<String, SchemaError> {
        Ok(format!(
            "{}/{}/{}/{}/{}/{}",
            self.target,
            self.version_segment()?,
            self.platform,
            self.arch,
            self.package,
            self.capture,
        ))
    }

    /// Turns a composed path into a [`RelPath`], reporting the refusal under a
    /// store code.
    ///
    /// ⚠ The tuple's components are each bounded well below [`RelPath::MAX_LEN`]
    /// and their sum is not, so a legal tuple can compose an over-long path.
    /// That is a refusal with a name rather than a surprise: `E-STO-04` says the
    /// path is unpublishable and how long it was.
    fn compose(text: &str) -> Result<RelPath, SchemaError> {
        let length = text.len();
        RelPath::parse(text).map_err(|error| {
            SchemaError::new(
                "E-STO-04",
                "store.path",
                format!("{length} characters, refused as a store path: {error}"),
            )
        })
    }

    /// Where this run's profile record is filed.
    ///
    /// # Errors
    ///
    /// Returns `E-STO-01` when the version cannot be a path segment, or
    /// `E-STO-04` when the composed path is not a canonical relative path.
    pub fn profile_path(&self) -> Result<RelPath, SchemaError> {
        Self::compose(&format!(
            "{PROFILE_ROOT}/{STORE_LAYOUT}/{}{PROFILE_EXT}",
            self.tail()?
        ))
    }

    /// The directory this run's raw evidence bundle is rooted at.
    ///
    /// Evidence paths in a record are relative to it, per
    /// [`crate::record::EvidenceRef::path`].
    ///
    /// # Errors
    ///
    /// As [`StoreKey::profile_path`].
    pub fn bundle_dir(&self) -> Result<RelPath, SchemaError> {
        Self::compose(&format!("{RAW_ROOT}/{STORE_LAYOUT}/{}", self.tail()?))
    }

    /// Where this run's manifest is filed.
    ///
    /// # Errors
    ///
    /// As [`StoreKey::profile_path`].
    pub fn manifest_path(&self) -> Result<RelPath, SchemaError> {
        Self::compose(&format!(
            "{RAW_ROOT}/{STORE_LAYOUT}/{}/{MANIFEST_FILE}",
            self.tail()?
        ))
    }

    /// Where one evidence artifact of this run is filed.
    ///
    /// # Errors
    ///
    /// As [`StoreKey::profile_path`]. `inside` is already a canonical relative
    /// path, so it contributes length rather than a new class of refusal.
    pub fn evidence_path(&self, inside: &RelPath) -> Result<RelPath, SchemaError> {
        Self::compose(&format!(
            "{RAW_ROOT}/{STORE_LAYOUT}/{}/{inside}",
            self.tail()?
        ))
    }
}

/// The roots whose published paths never change once they are published.
///
/// ⛔ **The append rule is about measurements, not about every byte in the
/// tree.** A record and the evidence it cites cannot be rewritten, because the
/// bytes that go are not anywhere else. The derived files exist in order to
/// change: an index or a checksum manifest that did not move when a record was
/// appended would be one that had stopped describing the store.
///
/// ⚠ **This distinction was missing and a driven run found it.** `PUB-02`
/// appended a second version to a branch and was refused, because
/// `MANIFEST.json`, `SHA256SUMS` and the generated index had all changed, which
/// is exactly what they are for. Applying the rule to every path made a correct
/// second publication impossible.
///
/// ⚠ A derived path may also disappear, since the whole derived set is rebuilt
/// from the canonical one. Whether a consumer-facing path is allowed to vanish
/// is a different question and `PUB-04` owns it.
pub const CANONICAL_ROOTS: [&str; 2] = [PROFILE_ROOT, RAW_ROOT];

/// Whether a path carries a measurement rather than something derived from one.
#[must_use]
pub fn is_canonical_path(path: &RelPath) -> bool {
    let text = path.as_str();
    CANONICAL_ROOTS.iter().any(|root| {
        text.starts_with(root) && text[root.len()..].starts_with(&format!("/{STORE_LAYOUT}/"))
    })
}

/// Whether a path is where this build files a profile record.
///
/// ⛔ **The recogniser lives beside the composer because it decides whether the
/// placement check runs at all.** A caller that spelled the layout a second time
/// to decide what to read would, on the day the layout moves, quietly recognise
/// nothing: every record would skip its placement check and the suite would stay
/// green, which is a gate that stopped applying rather than one that failed.
/// `every_derived_path_is_recognised` closes the loop.
#[must_use]
pub fn is_profile_path(path: &RelPath) -> bool {
    let text = path.as_str();
    text.starts_with(PROFILE_ROOT)
        && text[PROFILE_ROOT.len()..].starts_with(&format!("/{STORE_LAYOUT}/"))
        && text.ends_with(PROFILE_EXT)
}

/// Whether a path is where this build files a run manifest.
#[must_use]
pub fn is_manifest_path(path: &RelPath) -> bool {
    let text = path.as_str();
    text.starts_with(RAW_ROOT)
        && text[RAW_ROOT.len()..].starts_with(&format!("/{STORE_LAYOUT}/"))
        && text.ends_with(&format!("/{MANIFEST_FILE}"))
}

/// A regular file in a published tree.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObjectRef {
    /// Its exact size in bytes.
    pub bytes: u64,
    /// Its digest.
    pub sha256: Sha256Digest,
}

/// What sits at one path in a tree.
///
/// ⚠ A published tree is bytes. The two variants that are not bytes exist so a
/// walker can report what it found rather than dropping it: a path a check
/// never heard of is a path a check never guarded.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Entry {
    /// A regular file.
    Object(ObjectRef),
    /// A symbolic link. Refused: the store publishes bytes, and a link is a
    /// pointer whose target the store does not control and cannot digest.
    Symlink,
    /// Anything else a filesystem carries.
    Other,
}

impl Entry {
    /// The object at this path, when it is one.
    #[must_use]
    pub const fn object(&self) -> Option<&ObjectRef> {
        match self {
            Self::Object(object) => Some(object),
            Self::Symlink | Self::Other => None,
        }
    }

    /// A stable one-word name for a diagnostic.
    const fn kind(&self) -> &'static str {
        match self {
            Self::Object(_) => "object",
            Self::Symlink => "symlink",
            Self::Other => "other",
        }
    }
}

/// A published tree: every path in it, with what sits there.
///
/// Ordered, because two runs assembling the same store must produce the same
/// comparison and the same diagnostics in the same order.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StoreTree {
    entries: BTreeMap<RelPath, Entry>,
}

impl StoreTree {
    /// An empty tree, which is what a first publication appends to.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records what sits at one path, reporting what was there before.
    ///
    /// ⚠ A caller that ignores a returned `Some` has read one path twice and is
    /// building a tree that cannot exist on a disk.
    pub fn insert(&mut self, path: RelPath, entry: Entry) -> Option<Entry> {
        self.entries.insert(path, entry)
    }

    /// What sits at one path.
    #[must_use]
    pub fn get(&self, path: &RelPath) -> Option<&Entry> {
        self.entries.get(path)
    }

    /// How many paths the tree carries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the tree carries nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Every path, in order.
    pub fn paths(&self) -> impl Iterator<Item = &RelPath> {
        self.entries.keys()
    }

    /// Every path with what sits there, in order.
    pub fn iter(&self) -> impl Iterator<Item = (&RelPath, &Entry)> {
        self.entries.iter()
    }
}

impl<'a> IntoIterator for &'a StoreTree {
    type Item = (&'a RelPath, &'a Entry);
    type IntoIter = std::collections::btree_map::Iter<'a, RelPath, Entry>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}

impl FromIterator<(RelPath, Entry)> for StoreTree {
    fn from_iter<T: IntoIterator<Item = (RelPath, Entry)>>(iter: T) -> Self {
        Self {
            entries: iter.into_iter().collect(),
        }
    }
}

/// What one tree must satisfy on its own, before anything is compared to it.
///
/// These are the rules that make a tree checkable out on the machines that have
/// to check it out. Every one of them is a way for two paths to become one, or
/// for a path to name something that is not the bytes it claims.
///
/// # Errors
///
/// Returns every refusal, in path order:
///
/// | code | refused |
/// | --- | --- |
/// | `E-STO-10` | two paths that differ only in ASCII case |
/// | `E-STO-11` | a segment that cannot be a published path segment |
/// | `E-STO-12` | a path that is also a directory holding other paths |
/// | `E-STO-13` | a zero-length object |
/// | `E-STO-14` | a symbolic link |
/// | `E-STO-15` | an entry that is neither a file nor a link |
pub fn validate_tree(tree: &StoreTree) -> Result<(), Violations> {
    let mut out = Vec::new();

    let mut directories: BTreeSet<&str> = BTreeSet::new();
    for path in tree.paths() {
        let text = path.as_str();
        for (index, byte) in text.bytes().enumerate() {
            if byte == b'/' {
                directories.insert(&text[..index]);
            }
        }
    }

    let mut folded: BTreeMap<String, &RelPath> = BTreeMap::new();
    for (path, entry) in tree.iter() {
        let text = path.as_str();

        if let Some(first) = folded.insert(text.to_ascii_lowercase(), path)
            && first != path
        {
            out.push(SchemaError::new(
                "E-STO-10",
                text,
                format!(
                    "collides with {first} on a case-insensitive filesystem, where one \
                     silently overwrites the other"
                ),
            ));
        }

        for segment in text.split('/') {
            if let Some(detail) = segment_hazard(segment) {
                out.push(SchemaError::new("E-STO-11", text, detail));
            }
        }

        if directories.contains(text) {
            out.push(SchemaError::new(
                "E-STO-12",
                text,
                "is a file here and a directory for other paths in the same tree",
            ));
        }

        match entry {
            Entry::Object(object) => {
                if object.bytes == 0 {
                    out.push(SchemaError::new(
                        "E-STO-13",
                        text,
                        "is empty; a parsed value with no recoverable bytes is not a measurement",
                    ));
                }
            }
            Entry::Symlink => {
                out.push(SchemaError::new(
                    "E-STO-14",
                    text,
                    "is a symbolic link; a published tree carries bytes, not pointers",
                ));
            }
            Entry::Other => {
                out.push(SchemaError::new(
                    "E-STO-15",
                    text,
                    format!("is a {}, which a published tree cannot carry", entry.kind()),
                ));
            }
        }
    }

    Violations::from_errors(out)
}

/// The append-only rule, between a published tree and the successor a run
/// proposes.
///
/// Additions pass, which is the whole point: a new stable release appends a new
/// version directory and the old one is untouched. Everything else about an
/// already-published **canonical** path is refused; a derived one is expected to
/// move and [`CANONICAL_ROOTS`] says why.
///
/// # Errors
///
/// Returns every refusal, in the published tree's path order:
///
/// | code | refused |
/// | --- | --- |
/// | `E-STO-20` | a published path is absent from the successor |
/// | `E-STO-21` | a published path's content changed |
/// | `E-STO-22` | a published path's size changed while its digest did not |
pub fn append_only(prior: &StoreTree, next: &StoreTree) -> Result<(), Violations> {
    let mut out = Vec::new();

    for (path, was) in prior.iter() {
        // ⛔ Only a canonical path is immutable. See [`CANONICAL_ROOTS`]: a
        // derived file that did not change when a record was appended would be
        // one that had stopped describing the store.
        if !is_canonical_path(path) {
            continue;
        }
        let Some(now) = next.get(path) else {
            out.push(SchemaError::new(
                "E-STO-20",
                path.as_str(),
                "is published and absent from the successor; the store never deletes",
            ));
            continue;
        };

        match (was.object(), now.object()) {
            (Some(before), Some(after)) => {
                if before.sha256 != after.sha256 {
                    out.push(SchemaError::new(
                        "E-STO-21",
                        path.as_str(),
                        format!(
                            "published as {}, proposed as {}; correct by superseding, never by \
                             rewriting",
                            before.sha256, after.sha256
                        ),
                    ));
                } else if before.bytes != after.bytes {
                    // ⚠ NOT REACHABLE FROM A TREE BUILT BY DIGESTING FILES, and
                    // kept anyway. There the size and the digest come off the
                    // same bytes, so an equal digest forces an equal size. It is
                    // reachable from a tree built out of a manifest, whose
                    // declared length is a second copy of a fact its digest
                    // already carries; `PUB-01` reads one of those.
                    out.push(SchemaError::new(
                        "E-STO-22",
                        path.as_str(),
                        format!(
                            "published at {} bytes, proposed at {} bytes, under one digest; one \
                             of the two lengths is not the bytes it names",
                            before.bytes, after.bytes
                        ),
                    ));
                }
            }
            _ => {
                out.push(SchemaError::new(
                    "E-STO-21",
                    path.as_str(),
                    format!(
                        "published as a {}, proposed as a {}",
                        was.kind(),
                        now.kind()
                    ),
                ));
            }
        }
    }

    Violations::from_errors(out)
}

/// Whether a profile record is filed where its own contents say it belongs.
///
/// ⛔ **The path and the record are two copies of one fact.** A record read out
/// of a path nobody compared against it is the copy a reader trusts being the
/// wrong one, and here the wrong one is a measurement attributed to a build that
/// did not produce it.
///
/// # Errors
///
/// Returns `E-STO-30` when the record belongs elsewhere, or the derivation's own
/// refusal when the record has no publishable path at all.
pub fn check_profile_placement(path: &RelPath, profile: &Profile) -> Result<(), SchemaError> {
    let derived = StoreKey::of_profile(profile).profile_path()?;
    if &derived == path {
        Ok(())
    } else {
        Err(SchemaError::new(
            "E-STO-30",
            path.as_str(),
            format!("carries a record whose identity derives {derived}"),
        ))
    }
}

/// Whether a run manifest is filed where its own contents say it belongs.
///
/// # Errors
///
/// As [`check_profile_placement`].
pub fn check_manifest_placement(path: &RelPath, manifest: &RunManifest) -> Result<(), SchemaError> {
    let derived = StoreKey::of_manifest(manifest).manifest_path()?;
    if &derived == path {
        Ok(())
    } else {
        Err(SchemaError::new(
            "E-STO-30",
            path.as_str(),
            format!("carries a manifest whose identity derives {derived}"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Entry, MANIFEST_FILE, ObjectRef, PROFILE_EXT, PROFILE_ROOT, RAW_ROOT, STORE_LAYOUT,
        StoreKey, StoreTree, append_only, is_canonical_path, is_manifest_path, is_profile_path,
        segment_hazard, validate_tree,
    };
    use crate::PROFILE_SCHEMA;
    use crate::canonical::{RelPath, Sha256Digest, Slug, Version};

    fn slug(text: &str) -> Slug {
        Slug::parse(text).expect("a canonical slug")
    }

    fn version(text: &str) -> Version {
        Version::parse(text).expect("a reported version")
    }

    fn path(text: &str) -> RelPath {
        RelPath::parse(text).expect("a canonical relative path")
    }

    fn object(body: &str) -> Entry {
        Entry::Object(ObjectRef {
            bytes: body.len() as u64,
            sha256: Sha256Digest::of(body.as_bytes()),
        })
    }

    struct Tuple {
        target: Slug,
        version: Version,
        platform: Slug,
        arch: Slug,
        package: Slug,
        capture: Slug,
    }

    impl Tuple {
        fn sample() -> Self {
            Self {
                target: slug("qbittorrent"),
                version: version("5.2.3"),
                platform: slug("linux"),
                arch: slug("x86-64"),
                package: slug("appimage"),
                capture: slug("cap-0001"),
            }
        }

        fn key(&self) -> StoreKey<'_> {
            StoreKey {
                target: &self.target,
                version: &self.version,
                platform: &self.platform,
                arch: &self.arch,
                package: &self.package,
                capture: &self.capture,
            }
        }
    }

    /// ⛔ The literals, not the constants. A path asserted against the constant
    /// that spells it moves with the constant and pins nothing, which is the
    /// shape this project found twice in one session.
    #[test]
    fn layout_generation_tracks_schema() {
        assert_eq!(STORE_LAYOUT, "v1");
        assert_eq!(PROFILE_SCHEMA, "bit-ids/profile/1");
        assert_eq!(PROFILE_ROOT, "profiles");
        assert_eq!(RAW_ROOT, "raw");
        assert_eq!(MANIFEST_FILE, "manifest.json");
        assert_eq!(PROFILE_EXT, ".json");
        assert!(
            PROFILE_SCHEMA.ends_with(STORE_LAYOUT.trim_start_matches('v')),
            "the layout generation stands for the schema generation"
        );
    }

    #[test]
    fn a_record_is_filed_under_its_whole_identity_tuple() {
        let tuple = Tuple::sample();
        assert_eq!(
            tuple.key().profile_path().expect("a publishable path"),
            path("profiles/v1/qbittorrent/5.2.3/linux/x86-64/appimage/cap-0001.json")
        );
        assert_eq!(
            tuple.key().manifest_path().expect("a publishable path"),
            path("raw/v1/qbittorrent/5.2.3/linux/x86-64/appimage/cap-0001/manifest.json")
        );
        assert_eq!(
            tuple
                .key()
                .evidence_path(&path("peer/handshake.json"))
                .expect("a publishable path"),
            path("raw/v1/qbittorrent/5.2.3/linux/x86-64/appimage/cap-0001/peer/handshake.json")
        );
    }

    #[test]
    fn every_component_of_the_tuple_moves_the_path() {
        let base = Tuple::sample();
        let mut seen = std::collections::BTreeSet::new();
        seen.insert(base.key().profile_path().expect("a publishable path"));

        let variants = [
            Tuple {
                target: slug("transmission"),
                ..Tuple::sample()
            },
            Tuple {
                version: version("5.2.2"),
                ..Tuple::sample()
            },
            Tuple {
                platform: slug("windows"),
                ..Tuple::sample()
            },
            Tuple {
                arch: slug("aarch64"),
                ..Tuple::sample()
            },
            Tuple {
                package: slug("deb"),
                ..Tuple::sample()
            },
            Tuple {
                capture: slug("cap-0002"),
                ..Tuple::sample()
            },
        ];
        for variant in &variants {
            let derived = variant.key().profile_path().expect("a publishable path");
            assert!(
                seen.insert(derived.clone()),
                "{derived} collides with a tuple it differs from"
            );
        }
        assert_eq!(seen.len(), 1 + variants.len());
    }

    /// ⛔ The one the published layout got wrong. `package` is in the identity
    /// tuple and was not in the path, so two records the identifier tells apart
    /// were one file.
    /// ⛔ The recogniser and the composer, closed into a loop. Each alone is a
    /// spelling of the layout; the pair is a rule.
    #[test]
    fn every_derived_path_is_recognised() {
        let tuple = Tuple::sample();
        let profile = tuple.key().profile_path().expect("a publishable path");
        let manifest = tuple.key().manifest_path().expect("a publishable path");
        let evidence = tuple
            .key()
            .evidence_path(&path("observer/events.jsonl"))
            .expect("a publishable path");

        assert!(is_profile_path(&profile));
        assert!(!is_manifest_path(&profile));
        assert!(is_manifest_path(&manifest));
        assert!(!is_profile_path(&manifest));
        assert!(!is_profile_path(&evidence));
        assert!(!is_manifest_path(&evidence));

        // A near miss on each side, so the recogniser is not simply true.
        assert!(!is_profile_path(&path("profiles/v2/a/1/l/x/p/c.json")));
        assert!(!is_profile_path(&path("profilesx/v1/a/1/l/x/p/c.json")));
        assert!(!is_profile_path(&path("profiles/v1/a/1/l/x/p/c.txt")));
        assert!(!is_manifest_path(&path("raw/v1/a/1/l/x/p/c/other.json")));
    }

    #[test]
    fn two_packages_of_one_build_are_two_paths() {
        let appimage = Tuple::sample();
        let deb = Tuple {
            package: slug("deb"),
            ..Tuple::sample()
        };
        assert_ne!(
            appimage.key().profile_path().expect("a publishable path"),
            deb.key().profile_path().expect("a publishable path")
        );
    }

    #[test]
    fn version_is_not_a_path_segment() {
        // ⚠ The premise, measured rather than assumed: `Version` accepts this,
        // because a version string is what the build printed and imposing a
        // grammar on it would refuse builds that number themselves some other
        // way. The store is what has to refuse it.
        let escape = version("../../etc");
        let tuple = Tuple {
            version: escape,
            ..Tuple::sample()
        };
        let error = tuple
            .key()
            .profile_path()
            .expect_err("a traversal is not a publishable path");
        assert_eq!(error.code(), "E-STO-01");
    }

    #[test]
    fn a_composed_path_over_the_canonical_ceiling_is_refused() {
        let long = "a".repeat(Version::MAX_LEN);
        let tuple = Tuple {
            target: slug(&"t".repeat(Slug::MAX_LEN)),
            version: version(&long),
            platform: slug(&"p".repeat(Slug::MAX_LEN)),
            ..Tuple::sample()
        };
        let error = tuple
            .key()
            .profile_path()
            .expect_err("an over-long path is not publishable");
        assert_eq!(error.code(), "E-STO-04");
    }

    #[test]
    fn segment_hazards_are_named_one_by_one() {
        assert!(segment_hazard("5.2.3").is_none());
        assert!(segment_hazard("1.0.0-rc1").is_none());
        assert!(segment_hazard("").is_some());
        assert!(segment_hazard(".").is_some());
        assert!(segment_hazard("..").is_some());
        assert!(segment_hazard(".hidden").is_some());
        assert!(segment_hazard("trailing.").is_some());
        assert!(segment_hazard("has space").is_some());
        assert!(segment_hazard("a/b").is_some());
        assert!(segment_hazard("nul").is_some());
        assert!(segment_hazard("NUL").is_some());
        assert!(segment_hazard("com9.json").is_some());
        assert!(segment_hazard("common").is_none());
    }

    #[test]
    fn a_clean_tree_passes_every_structural_rule() {
        let tree: StoreTree = [
            (
                path("profiles/v1/a/1.0/linux/x86-64/deb/c1.json"),
                object("{}"),
            ),
            (
                path("raw/v1/a/1.0/linux/x86-64/deb/c1/manifest.json"),
                object("{}"),
            ),
        ]
        .into_iter()
        .collect();
        validate_tree(&tree).expect("a clean tree");
    }

    #[test]
    fn structural_hazards_are_refused_by_code() {
        let cases: [(RelPath, Entry, &str); 6] = [
            (path("profiles/v1/A.json"), object("{}"), "E-STO-10"),
            (path("profiles/v1/.hidden/a.json"), object("{}"), "E-STO-11"),
            (path("profiles/v1/a.json/inner"), object("{}"), "E-STO-12"),
            (path("profiles/v1/empty.json"), object(""), "E-STO-13"),
            (path("profiles/v1/link.json"), Entry::Symlink, "E-STO-14"),
            (path("profiles/v1/other.json"), Entry::Other, "E-STO-15"),
        ];
        for (target, entry, code) in cases {
            let mut tree = StoreTree::new();
            tree.insert(path("profiles/v1/a.json"), object("{}"));
            tree.insert(target.clone(), entry);
            let violations =
                validate_tree(&tree).expect_err(&format!("{target} should be refused as {code}"));
            assert!(
                violations.has(code),
                "{target} was refused, but not as {code}: {violations}"
            );
        }
    }

    #[test]
    fn appending_a_new_version_directory_is_accepted() {
        let prior: StoreTree = [(
            path("profiles/v1/a/1.0/linux/x86-64/deb/c1.json"),
            object("first"),
        )]
        .into_iter()
        .collect();
        let mut next = prior.clone();
        next.insert(
            path("profiles/v1/a/1.1/linux/x86-64/deb/c2.json"),
            object("second"),
        );
        append_only(&prior, &next).expect("an append is what the store is for");
    }

    #[test]
    fn a_deletion_and_a_rewrite_are_each_refused_by_code() {
        let published = path("profiles/v1/a/1.0/linux/x86-64/deb/c1.json");
        let prior: StoreTree = [(published.clone(), object("first"))].into_iter().collect();

        let empty = StoreTree::new();
        let deleted = append_only(&prior, &empty).expect_err("a deletion is refused");
        assert!(deleted.has("E-STO-20"), "{deleted}");

        let rewritten: StoreTree = [(published.clone(), object("edited"))]
            .into_iter()
            .collect();
        let changed = append_only(&prior, &rewritten).expect_err("a byte change is refused");
        assert!(changed.has("E-STO-21"), "{changed}");

        let relinked: StoreTree = [(published, Entry::Symlink)].into_iter().collect();
        let swapped = append_only(&prior, &relinked).expect_err("a kind change is refused");
        assert!(swapped.has("E-STO-21"), "{swapped}");
    }

    /// ⛔ The distinction a driven publication found. A second bundle changes
    /// every derived file by design, and applying the append rule to those made
    /// a correct second publication impossible.
    #[test]
    fn a_derived_file_may_change_and_a_record_may_not() {
        let record = path("profiles/v1/a/1.0/linux/x86-64/deb/c1.json");
        let evidence = path("raw/v1/a/1.0/linux/x86-64/deb/c1/observer/events.jsonl");
        let derived = [
            path("MANIFEST.json"),
            path("SHA256SUMS"),
            path("indexes/v1/profiles.json"),
            path("routes/v1/a/latest/linux/x86-64.json"),
            path("formats/bit-ids-v1.csv"),
        ];

        assert!(is_canonical_path(&record));
        assert!(is_canonical_path(&evidence));
        for at in &derived {
            assert!(!is_canonical_path(at), "{at} is derived");
        }

        let mut prior = StoreTree::new();
        prior.insert(record.clone(), object("the measurement"));
        prior.insert(evidence.clone(), object("the bytes"));
        for at in &derived {
            prior.insert(at.clone(), object("built from the first publication"));
        }

        // Every derived file changes and one record is appended: a correct
        // second publication.
        let mut next = StoreTree::new();
        next.insert(record.clone(), object("the measurement"));
        next.insert(evidence.clone(), object("the bytes"));
        next.insert(
            path("profiles/v1/a/1.1/linux/x86-64/deb/c2.json"),
            object("a second measurement"),
        );
        for at in &derived {
            next.insert(at.clone(), object("built from the second publication"));
        }
        append_only(&prior, &next).expect("a derived file is meant to move");

        // ⛔ And the rule still holds where it matters. Same tree, one record
        // rewritten.
        let mut rewritten = next.clone();
        rewritten.insert(record, object("an edited measurement"));
        let violations = append_only(&prior, &rewritten).expect_err("a record is immutable");
        assert!(violations.has("E-STO-21"), "{violations}");

        // A derived file may also disappear; a record may not.
        let mut dropped = next.clone();
        for at in &derived {
            dropped = dropped
                .iter()
                .filter(|(had, _)| *had != at)
                .map(|(had, entry)| (had.clone(), *entry))
                .collect();
        }
        append_only(&prior, &dropped).expect("the derived set is rebuilt whole");

        let without_evidence: StoreTree = next
            .iter()
            .filter(|(had, _)| **had != evidence)
            .map(|(had, entry)| (had.clone(), *entry))
            .collect();
        let violations =
            append_only(&prior, &without_evidence).expect_err("evidence is not deletable");
        assert!(violations.has("E-STO-20"), "{violations}");
    }

    /// The one guard [`append_only`] carries that a walked tree cannot reach.
    /// A tree built from a manifest can, and that is where it fires.
    #[test]
    fn one_digest_over_two_lengths_is_refused() {
        let published = path("profiles/v1/a/1.0/linux/x86-64/deb/c1.json");
        let body = "first";
        let honest = ObjectRef {
            bytes: body.len() as u64,
            sha256: Sha256Digest::of(body.as_bytes()),
        };
        let lying = ObjectRef {
            bytes: honest.bytes + 1,
            ..honest
        };
        let prior: StoreTree = [(published.clone(), Entry::Object(honest))]
            .into_iter()
            .collect();
        let next: StoreTree = [(published, Entry::Object(lying))].into_iter().collect();
        let violations = append_only(&prior, &next).expect_err("two lengths, one digest");
        assert!(violations.has("E-STO-22"), "{violations}");
    }
}
