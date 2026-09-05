//! `CORPUS-01`: the append-only store, driven with the records the tree
//! actually carries rather than with tuples invented for the test.
//!
//! The unit tests beside the module cover the derivation and the rules over
//! synthetic trees. These run the same rules over the schema fixtures, which is
//! the only place a real `Profile`, its `RunManifest` and a correction that
//! supersedes it exist together, so it is the only place the store's central
//! claim can be checked: a correction is an append.

use bit_ids::canonical::RelPath;
use bit_ids::canonical::Sha256Digest;
use bit_ids::store::{
    Entry, ObjectRef, StoreKey, StoreTree, append_only, check_manifest_placement,
    check_profile_placement, validate_tree,
};
use bit_ids::{Profile, RunManifest};

const PROFILE: &str = include_str!("fixtures/valid-profile.json");
const CORRECTION: &str = include_str!("fixtures/valid-correction.json");
const MANIFEST: &str = include_str!("fixtures/valid-manifest.json");

fn profile() -> Profile {
    Profile::from_json(PROFILE).expect("the fixture record validates")
}

fn correction() -> Profile {
    Profile::from_json(CORRECTION).expect("the fixture correction validates")
}

fn manifest() -> RunManifest {
    RunManifest::from_json(MANIFEST).expect("the fixture manifest validates")
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

/// ⛔ The whole path, spelled out. A path asserted by rebuilding it from the
/// same components that built it is a test of `format!`.
#[test]
fn the_fixture_record_files_at_one_named_path() {
    let record = profile();
    assert_eq!(
        StoreKey::of_profile(&record)
            .profile_path()
            .expect("a publishable path"),
        path(
            "profiles/v1/fixture-client/0.0.0-fixture/linux/x86-64/tar-gz/fixture-capture-0001.json"
        )
    );
    assert_eq!(
        StoreKey::of_manifest(&manifest())
            .manifest_path()
            .expect("a publishable path"),
        path(
            "raw/v1/fixture-client/0.0.0-fixture/linux/x86-64/tar-gz/fixture-capture-0001/manifest.json"
        )
    );
}

/// The profile and the manifest are two documents about one run, so they file
/// under one bundle. A store that derived two tails from them would put a
/// record's evidence somewhere the record does not cite.
#[test]
fn a_record_and_its_manifest_share_one_bundle() {
    let record = profile();
    let run = manifest();
    let from_record = StoreKey::of_profile(&record)
        .bundle_dir()
        .expect("a publishable path");
    let from_run = StoreKey::of_manifest(&run)
        .bundle_dir()
        .expect("a publishable path");
    assert_eq!(from_record, from_run);

    let prefix = format!("{from_record}/");
    for artifact in &record.evidence {
        let resolved = StoreKey::of_profile(&record)
            .evidence_path(&artifact.path)
            .expect("a publishable path");
        assert!(
            resolved.as_str().starts_with(&prefix),
            "{} resolves outside its own bundle",
            artifact.id
        );
    }
    assert_eq!(
        record.evidence.len(),
        9,
        "the fixture bundle's artifact count"
    );
}

#[test]
fn a_record_filed_anywhere_else_is_refused() {
    let record = profile();
    let derived = StoreKey::of_profile(&record)
        .profile_path()
        .expect("a publishable path");
    check_profile_placement(&derived, &record).expect("its own path");

    // The same record one directory over: the platform a reader would take from
    // the path disagrees with the platform the record measured.
    let moved = path(
        "profiles/v1/fixture-client/0.0.0-fixture/windows/x86-64/tar-gz/fixture-capture-0001.json",
    );
    let error = check_profile_placement(&moved, &record).expect_err("a moved record is refused");
    assert_eq!(error.code(), "E-STO-30");

    let run = manifest();
    let manifest_path = StoreKey::of_manifest(&run)
        .manifest_path()
        .expect("a publishable path");
    check_manifest_placement(&manifest_path, &run).expect("its own path");
    let misfiled = check_manifest_placement(&derived, &run).expect_err("a misfiled manifest");
    assert_eq!(misfiled.code(), "E-STO-30");
}

/// ⭐ The store's central claim, driven with the real records: a correction is
/// an append. `valid-correction.json` supersedes `valid-profile.json` and is a
/// second capture, so it lands beside the record it corrects rather than on it.
#[test]
fn a_correction_appends_and_leaves_the_record_it_corrects_alone() {
    let original = profile();
    let corrected = correction();
    assert_eq!(
        corrected.supersedes,
        Some(original.id),
        "the fixture correction supersedes the fixture record"
    );

    let original_path = StoreKey::of_profile(&original)
        .profile_path()
        .expect("a publishable path");
    let correction_path = StoreKey::of_profile(&corrected)
        .profile_path()
        .expect("a publishable path");
    assert_ne!(original_path, correction_path);

    let published: StoreTree = [(original_path.clone(), object(PROFILE))]
        .into_iter()
        .collect();
    let mut proposed = published.clone();
    proposed.insert(correction_path, object(CORRECTION));

    validate_tree(&proposed).expect("both records are publishable paths");
    append_only(&published, &proposed).expect("a correction is an append");
}

#[test]
fn rewriting_or_removing_the_corrected_record_is_refused() {
    let original = profile();
    let original_path = StoreKey::of_profile(&original)
        .profile_path()
        .expect("a publishable path");
    let published: StoreTree = [(original_path.clone(), object(PROFILE))]
        .into_iter()
        .collect();

    // ⛔ The realistic rewrite: the corrected record replaced in place by the
    // correction's bytes, which is exactly what `supersedes` exists to avoid.
    let rewritten: StoreTree = [(original_path.clone(), object(CORRECTION))]
        .into_iter()
        .collect();
    let changed = append_only(&published, &rewritten).expect_err("a rewrite is refused");
    assert!(changed.has("E-STO-21"), "{changed}");

    let regenerated: StoreTree = [(
        path("profiles/v1/fixture-client/0.0.0-fixture/linux/x86-64/tar-gz/fixture-capture-0002.json"),
        object(CORRECTION),
    )]
    .into_iter()
    .collect();
    let removed = append_only(&published, &regenerated)
        .expect_err("a latest-only regeneration drops the older record");
    assert!(removed.has("E-STO-20"), "{removed}");
}

/// The bundle a real run leaves, checked as a whole tree rather than a path at
/// a time. Every artifact the fixture record cites, at the place the store puts
/// it, has to survive its own structural rules.
#[test]
fn the_fixture_bundle_is_a_publishable_tree() {
    let record = profile();
    let key = StoreKey::of_profile(&record);
    let mut tree = StoreTree::new();
    tree.insert(
        key.profile_path().expect("a publishable path"),
        object(PROFILE),
    );
    tree.insert(
        StoreKey::of_manifest(&manifest())
            .manifest_path()
            .expect("a publishable path"),
        object(MANIFEST),
    );
    for artifact in &record.evidence {
        tree.insert(
            key.evidence_path(&artifact.path)
                .expect("a publishable path"),
            Entry::Object(ObjectRef {
                bytes: artifact.bytes,
                sha256: artifact.sha256,
            }),
        );
    }
    assert_eq!(tree.len(), 11, "one record, one manifest, nine artifacts");
    validate_tree(&tree).expect("the bundle a real run leaves");
}
