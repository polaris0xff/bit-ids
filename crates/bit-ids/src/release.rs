//! The release bundle: assembled once, described by itself, and byte-identical
//! between runs.
//!
//! `PUB-01` owns this. The defect it exists to prevent is two jobs rebuilding a
//! release and publishing different bytes under one label, so everything here is
//! a function of the tree it is given: no clock is read, nothing is ordered by a
//! map's iteration, and every file is listed once.
//!
//! ⛔ **Two documents describe the bundle and they describe different sets, on
//! purpose.** `MANIFEST.json` carries a media type, a schema and a digest for
//! every file except itself and `SHA256SUMS`, because a document cannot state
//! its own digest. `SHA256SUMS` covers everything except itself, `MANIFEST.json`
//! included, so an ordinary transport check reaches the manifest too. A reader
//! who assumed either covered everything would find a gap exactly where the
//! other one is.
//!
//! ⚠ **A media type is looked up, never guessed.** An extension this does not
//! know blocks the assembly rather than being published as an opaque blob: a
//! consumer that receives the wrong type mis-parses silently, and
//! `application/octet-stream` for a JSON document is the wrong type.
//!
//! ⚠ **`PUB-03` owns the formats.** This assembles the tree, its manifest and
//! its checksums once; the JSON, JSONL, CSV, SQLite and CBOR renderings and the
//! archives are derived from this bundle rather than beside it.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use crate::canonical::{RelPath, Sha256Digest};
use crate::store::{Entry, ObjectRef, StoreTree};
use crate::validate::{SchemaError, Violations};

/// Identifier carried by every first-generation release manifest.
pub const RELEASE_SCHEMA: &str = "bit-ids/release/1";

/// The manifest's name at the root of a release.
pub const RELEASE_MANIFEST_FILE: &str = "MANIFEST.json";

/// The checksum file's name at the root of a release.
pub const CHECKSUMS_FILE: &str = "SHA256SUMS";

/// The licence's name at the root of a release.
pub const LICENSE_FILE: &str = "LICENSE";

/// Every extension this build knows how to publish, and what it is.
///
/// ⛔ **A closed list, checked rather than defaulted.** A file whose extension is
/// not here blocks the assembly under `E-REL-01`. The alternative is publishing
/// it as `application/octet-stream`, which a consumer receives as "some bytes"
/// and mis-parses, and the failure lands on the reader rather than on the
/// publisher who added the file.
const MEDIA_TYPES: [(&str, &str); 10] = [
    ("json", "application/json"),
    ("jsonl", "application/jsonl"),
    ("csv", "text/csv"),
    ("cbor", "application/cbor"),
    ("sqlite3", "application/vnd.sqlite3"),
    ("txt", "text/plain"),
    ("log", "text/plain"),
    ("bin", "application/octet-stream"),
    ("pcapng", "application/vnd.tcpdump.pcap"),
    // ⭐ Added because the guard refused a real evidence bundle on its first
    // driven run. Every capture writes a generated metainfo file, and the
    // fail-closed rule named it rather than publishing it as opaque bytes,
    // which is the rule doing exactly what it is for.
    ("torrent", "application/x-bittorrent"),
];

/// The names a release carries with no extension, and what they are.
const NAMED_TYPES: [(&str, &str); 2] =
    [(LICENSE_FILE, "text/plain"), (CHECKSUMS_FILE, "text/plain")];

/// The media type for one published path.
fn media_type(path: &RelPath) -> Option<&'static str> {
    let text = path.as_str();
    let name = text.rsplit('/').next().unwrap_or(text);
    if let Some((_, kind)) = NAMED_TYPES.iter().find(|(named, _)| *named == name) {
        return Some(kind);
    }
    // ⚠ The last dotted segment, so `bit-ids-v1.tar.gz` asks about `gz` rather
    // than about `tar.gz`. A name with no dot has no extension and is refused,
    // which is what keeps a stray `README` out of a published tree.
    let extension = name.rsplit_once('.')?.1;
    MEDIA_TYPES
        .iter()
        .find(|(known, _)| *known == extension)
        .map(|(_, kind)| *kind)
}

/// The schema a published path declares, when this build knows one.
///
/// ⚠ Read from where the file sits rather than from inside it. The assembler is
/// handed digests and does not re-read every document, and a schema taken from
/// the path is a claim the corpus validator has already checked against the
/// bytes.
fn schema_of(path: &RelPath) -> Option<&'static str> {
    let text = path.as_str();
    if crate::store::is_profile_path(path) {
        Some(crate::PROFILE_SCHEMA)
    } else if crate::store::is_manifest_path(path) {
        Some(crate::MANIFEST_SCHEMA)
    } else if text == RELEASE_MANIFEST_FILE {
        Some(RELEASE_SCHEMA)
    } else {
        None
    }
}

/// One file in a release.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ReleaseEntry {
    /// Where it sits, relative to the release root.
    pub path: RelPath,
    /// What it is.
    pub media_type: &'static str,
    /// The document schema it declares, when it declares one.
    pub schema: Option<&'static str>,
    /// Its exact size.
    pub bytes: u64,
    /// Its digest.
    pub sha256: Sha256Digest,
}

/// An assembled release: every file, described once, in path order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Release {
    entries: Vec<ReleaseEntry>,
}

impl Release {
    /// Every file, in path order.
    #[must_use]
    pub fn entries(&self) -> &[ReleaseEntry] {
        &self.entries
    }

    /// How many files the release carries, excluding the two it describes
    /// itself with.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the release describes nothing.
    ///
    /// ⛔ An empty release is refused by [`assemble`], so this is always false
    /// for one that exists. It is here because clippy pairs it with
    /// [`Release::len`], and a reader should know which of the two can happen.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The `MANIFEST.json` bytes.
    ///
    /// ⛔ **Written by hand and reading no clock.** The field order, the spacing
    /// and the escaping are the bytes a consumer digests, so a derive that
    /// changed any of them between versions would move a digest every consumer
    /// had recorded. A timestamp would move it on every run, which is the whole
    /// defect this entry is about.
    #[must_use]
    pub fn manifest_json(&self) -> String {
        let mut out = String::new();
        out.push_str("{\n  \"schema\": \"");
        out.push_str(RELEASE_SCHEMA);
        out.push_str("\",\n  \"files\": [\n");
        for (index, entry) in self.entries.iter().enumerate() {
            out.push_str("    {\"path\": \"");
            out.push_str(entry.path.as_str());
            out.push_str("\", \"media_type\": \"");
            out.push_str(entry.media_type);
            out.push_str("\", \"schema\": ");
            match entry.schema {
                Some(schema) => {
                    out.push('"');
                    out.push_str(schema);
                    out.push('"');
                }
                None => out.push_str("null"),
            }
            out.push_str(", \"bytes\": ");
            let _ = write!(out, "{}", entry.bytes);
            out.push_str(", \"sha256\": \"");
            let _ = write!(out, "{}", entry.sha256);
            out.push_str("\"}");
            if index + 1 < self.entries.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("  ]\n}\n");
        out
    }

    /// The `SHA256SUMS` bytes, in the shape `sha256sum -c` reads.
    ///
    /// ⚠ It covers `MANIFEST.json` and the manifest does not cover it. The two
    /// documents describe different sets so that neither has to state its own
    /// digest, and between them every published byte is covered once.
    #[must_use]
    pub fn checksums(&self, manifest: &[u8]) -> String {
        let mut rows: Vec<(String, String)> = self
            .entries
            .iter()
            .map(|entry| {
                (
                    entry.path.to_string(),
                    entry.sha256.to_string().replace(Sha256Digest::PREFIX, ""),
                )
            })
            .collect();
        rows.push((
            RELEASE_MANIFEST_FILE.to_owned(),
            Sha256Digest::of(manifest)
                .to_string()
                .replace(Sha256Digest::PREFIX, ""),
        ));
        rows.sort();

        let mut out = String::new();
        for (path, digest) in rows {
            // Two spaces, which is what coreutils writes for a binary read.
            let _ = writeln!(out, "{digest}  {path}");
        }
        out
    }
}

/// Assembles a release from a tree of objects.
///
/// The tree is everything the release will carry except its own two
/// descriptions: the records, the evidence, the generated indexes and the
/// licence. `MANIFEST.json` and `SHA256SUMS` are produced from the result.
///
/// # Errors
///
/// | code | refused |
/// | --- | --- |
/// | `E-REL-01` | a path whose media type this build does not know |
/// | `E-REL-02` | an entry that is not bytes |
/// | `E-REL-03` | a zero-length file |
/// | `E-REL-04` | a tree carrying one of the two documents it will be given |
/// | `E-REL-05` | an empty release |
pub fn assemble(tree: &StoreTree) -> Result<Release, Violations> {
    let mut errors = Vec::new();
    let mut entries = Vec::new();

    for (path, entry) in tree.iter() {
        let text = path.as_str();
        if text == RELEASE_MANIFEST_FILE || text == CHECKSUMS_FILE {
            // ⛔ The assembler writes these, so a tree already holding one is a
            // second copy of a fact this is about to derive. Publishing over it
            // would make the manifest describe a manifest from another run.
            errors.push(SchemaError::new(
                "E-REL-04",
                text,
                "is produced by the assembler; a tree carrying one describes another run",
            ));
            continue;
        }
        let Entry::Object(ObjectRef { bytes, sha256 }) = entry else {
            errors.push(SchemaError::new(
                "E-REL-02",
                text,
                "is not bytes; a release publishes files and nothing else",
            ));
            continue;
        };
        if *bytes == 0 {
            errors.push(SchemaError::new(
                "E-REL-03",
                text,
                "is empty; a published file with no bytes is a citation with nothing behind it",
            ));
            continue;
        }
        let Some(media_type) = media_type(path) else {
            errors.push(SchemaError::new(
                "E-REL-01",
                text,
                "has no media type this build knows; add one rather than publishing it as opaque \
                 bytes",
            ));
            continue;
        };
        entries.push(ReleaseEntry {
            path: path.clone(),
            media_type,
            schema: schema_of(path),
            bytes: *bytes,
            sha256: *sha256,
        });
    }

    if entries.is_empty() && errors.is_empty() {
        // ⛔ Zero files is not a release. It is the shape a pipeline produces
        // when its input directory was a typo, and it would publish cleanly.
        errors.push(SchemaError::new(
            "E-REL-05",
            "release",
            "carries no files; an empty release publishes nothing and reports success",
        ));
    }

    Violations::from_errors(errors)?;
    // ⚠ NOT REFUTED, AND KEPT. The entries come out of a `StoreTree`, which is
    // an ordered map, and `ReleaseEntry` orders on its path first, so removing
    // this changes no output that can be produced today. It is what makes the
    // manifest a function of the *set* of files rather than of the container a
    // caller happened to hand over, and it would matter the moment `assemble`
    // took anything but an ordered map. A reader should not take an unreached
    // guard for a proven one.
    entries.sort();
    Ok(Release { entries })
}

/// Whether a manifest describes exactly the files a release carries.
///
/// ⛔ **Both directions, because each catches a different mistake.** A described
/// file that is absent is a consumer's failed download; a present file nobody
/// described is a byte published with no digest and no type, which is how
/// something ships that was never meant to.
///
/// # Errors
///
/// | code | refused |
/// | --- | --- |
/// | `E-REL-10` | a manifest row naming a file the release does not carry |
/// | `E-REL-11` | a file the manifest does not describe |
/// | `E-REL-12` | a manifest row disagreeing with the file it names |
pub fn manifest_covers(release: &Release, tree: &StoreTree) -> Result<(), Violations> {
    let mut errors = Vec::new();
    let described: BTreeMap<&RelPath, &ReleaseEntry> = release
        .entries
        .iter()
        .map(|entry| (&entry.path, entry))
        .collect();

    for entry in &release.entries {
        match tree.get(&entry.path).and_then(Entry::object) {
            None => errors.push(SchemaError::new(
                "E-REL-10",
                entry.path.as_str(),
                "is described and the release does not carry it",
            )),
            Some(object) => {
                if object.bytes != entry.bytes || object.sha256 != entry.sha256 {
                    errors.push(SchemaError::new(
                        "E-REL-12",
                        entry.path.as_str(),
                        format!(
                            "described as {} bytes {}, carried as {} bytes {}",
                            entry.bytes, entry.sha256, object.bytes, object.sha256
                        ),
                    ));
                }
            }
        }
    }

    let produced: BTreeSet<&str> = [RELEASE_MANIFEST_FILE, CHECKSUMS_FILE]
        .into_iter()
        .collect();
    for path in tree.paths() {
        if !described.contains_key(path) && !produced.contains(path.as_str()) {
            errors.push(SchemaError::new(
                "E-REL-11",
                path.as_str(),
                "is in the release and no manifest row describes it",
            ));
        }
    }

    Violations::from_errors(errors)
}

#[cfg(test)]
mod tests {
    use super::{
        CHECKSUMS_FILE, LICENSE_FILE, RELEASE_MANIFEST_FILE, RELEASE_SCHEMA, assemble,
        manifest_covers, media_type,
    };
    use crate::canonical::{RelPath, Sha256Digest};
    use crate::store::{Entry, ObjectRef, StoreTree};

    fn path(text: &str) -> RelPath {
        RelPath::parse(text).expect("a canonical relative path")
    }

    fn object(body: &str) -> Entry {
        Entry::Object(ObjectRef {
            bytes: body.len() as u64,
            sha256: Sha256Digest::of(body.as_bytes()),
        })
    }

    fn tree() -> StoreTree {
        [
            (path(LICENSE_FILE), object("0BSD")),
            (
                path("profiles/v1/a/1.2.3/linux/x86-64/deb/c1.json"),
                object("{\"record\": true}"),
            ),
            (
                path("raw/v1/a/1.2.3/linux/x86-64/deb/c1/manifest.json"),
                object("{\"run\": true}"),
            ),
            (
                path("raw/v1/a/1.2.3/linux/x86-64/deb/c1/observer/events.jsonl"),
                object("{}\n"),
            ),
            (path("indexes/v1/profiles.json"), object("{\"rows\": []}")),
        ]
        .into_iter()
        .collect()
    }

    /// ⛔ Literals, not the constants that spell them. These names are the
    /// published contract; a rename is a schema change.
    #[test]
    fn the_release_names_have_one_spelling() {
        assert_eq!(RELEASE_SCHEMA, "bit-ids/release/1");
        assert_eq!(RELEASE_MANIFEST_FILE, "MANIFEST.json");
        assert_eq!(CHECKSUMS_FILE, "SHA256SUMS");
        assert_eq!(LICENSE_FILE, "LICENSE");
    }

    #[test]
    fn two_assemblies_of_one_tree_are_byte_identical() {
        let first = assemble(&tree()).expect("a publishable tree");
        let second = assemble(&tree()).expect("a publishable tree");
        assert_eq!(first.manifest_json(), second.manifest_json());
        assert_eq!(
            first.checksums(first.manifest_json().as_bytes()),
            second.checksums(second.manifest_json().as_bytes())
        );
        assert_eq!(first.len(), 5);
    }

    /// ⛔ The two documents cover different sets, and the gap in each is exactly
    /// where the other one is.
    #[test]
    fn the_manifest_omits_itself_and_the_checksums_cover_it() {
        let release = assemble(&tree()).expect("a publishable tree");
        let manifest = release.manifest_json();
        assert!(
            !manifest.contains(RELEASE_MANIFEST_FILE),
            "a document cannot state its own digest"
        );
        assert!(!manifest.contains(CHECKSUMS_FILE));

        let sums = release.checksums(manifest.as_bytes());
        assert!(
            sums.contains(RELEASE_MANIFEST_FILE),
            "the checksums are what cover the manifest"
        );
        assert!(!sums.contains(CHECKSUMS_FILE), "nor can this one");
        assert_eq!(
            sums.lines().count(),
            release.len() + 1,
            "every file plus the manifest"
        );
        // Two spaces, which is the shape `sha256sum -c` reads.
        for line in sums.lines() {
            assert_eq!(&line[64..66], "  ", "{line}");
        }
    }

    #[test]
    fn a_schema_is_recorded_for_the_documents_that_declare_one() {
        let release = assemble(&tree()).expect("a publishable tree");
        let manifest = release.manifest_json();
        assert!(manifest.contains(crate::PROFILE_SCHEMA));
        assert!(manifest.contains(crate::MANIFEST_SCHEMA));
        // The licence declares no schema and says so rather than omitting it.
        assert!(manifest.contains("\"schema\": null"));
    }

    #[test]
    fn an_unknown_media_type_blocks_the_assembly() {
        assert_eq!(media_type(&path("a/b.json")), Some("application/json"));
        assert_eq!(media_type(&path(LICENSE_FILE)), Some("text/plain"));
        assert_eq!(
            media_type(&path("a/fixture/generated.torrent")),
            Some("application/x-bittorrent"),
            "every capture writes one, and the guard refused it on its first driven run"
        );
        assert_eq!(media_type(&path("a/b.unknown")), None);
        assert_eq!(media_type(&path("a/README")), None);

        let mut tree = tree();
        tree.insert(path("notes.unknown"), object("x"));
        let violations = assemble(&tree).expect_err("an unknown type");
        assert!(violations.has("E-REL-01"), "{violations}");
    }

    #[test]
    fn the_release_refuses_what_it_cannot_publish() {
        let cases: [(RelPath, Entry, &str); 3] = [
            (path("link.json"), Entry::Symlink, "E-REL-02"),
            (path("empty.json"), object(""), "E-REL-03"),
            (
                path(RELEASE_MANIFEST_FILE),
                object("{\"stale\": true}"),
                "E-REL-04",
            ),
        ];
        for (target, entry, code) in cases {
            let mut tree = tree();
            tree.insert(target.clone(), entry);
            let violations =
                assemble(&tree).expect_err(&format!("{target} should be refused as {code}"));
            assert!(violations.has(code), "{target}: {violations}");
        }

        let violations = assemble(&StoreTree::new()).expect_err("an empty release");
        assert!(violations.has("E-REL-05"), "{violations}");
    }

    /// ⛔ Both directions. A described file that is absent and a present file
    /// nobody described are different mistakes with different victims.
    #[test]
    fn the_manifest_and_the_release_describe_each_other() {
        let tree = tree();
        let release = assemble(&tree).expect("a publishable tree");
        manifest_covers(&release, &tree).expect("the manifest describes the release");

        let mut fewer = tree.clone();
        fewer.insert(path("indexes/v1/extra.json"), object("{}"));
        let violations = manifest_covers(&release, &fewer).expect_err("a file nobody described");
        assert!(violations.has("E-REL-11"), "{violations}");

        let mut changed = StoreTree::new();
        for (at, entry) in &tree {
            if at.as_str() == LICENSE_FILE {
                changed.insert(at.clone(), object("a different licence"));
            } else {
                changed.insert(at.clone(), *entry);
            }
        }
        let violations =
            manifest_covers(&release, &changed).expect_err("a row that does not match");
        assert!(violations.has("E-REL-12"), "{violations}");

        let mut missing = StoreTree::new();
        for (at, entry) in &tree {
            if at.as_str() != LICENSE_FILE {
                missing.insert(at.clone(), *entry);
            }
        }
        let violations = manifest_covers(&release, &missing).expect_err("a described file absent");
        assert!(violations.has("E-REL-10"), "{violations}");
    }
}
