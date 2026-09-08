//! `PUB-04`'s refusals, planted into a contract document one at a time.
//!
//! ⛔ **These exist because a mutation pass found three of the five refusals
//! unreachable.** [`bit_ids::access::contract`] derives a path's stability from
//! `is_canonical_path`, its integrity from its kind, and its order by sorting,
//! so the validator's checks on all three can never fire on anything the deriver
//! produced: the two halves agree by construction. Blanking each of them left
//! every case in the crate and the whole `check-access.sh` harness green.
//!
//! ⚠ **That is not an argument for deleting them.** A contract arrives from a
//! file as often as from a deriver, and `AccessContract::from_json` is the door a
//! consumer's copy comes through. What it is an argument for is testing them
//! where they are reachable, which is at the document level, which is what this
//! file does.

use bit_ids::access::{
    ACCESS_SCHEMA, AccessContract, AccessPath, DATA_BRANCH, Integrity, PathKind, Stability,
    contract, validate_contract,
};
use bit_ids::canonical::{RelPath, Sha256Digest};
use bit_ids::release::assemble;
use bit_ids::store::{Entry, ObjectRef, StoreTree};

fn path(text: &str) -> RelPath {
    RelPath::parse(text).expect("path")
}

fn object(bytes: &[u8]) -> Entry {
    Entry::Object(ObjectRef {
        bytes: bytes.len() as u64,
        sha256: Sha256Digest::of(bytes),
    })
}

/// A publication shaped like a real one: a record, its run, one evidence
/// artifact, and one file from each derived root.
fn published() -> AccessContract {
    let mut tree = StoreTree::new();
    for (at, bytes) in [
        (
            "profiles/v1/fixture-client/1.2.3/linux/x86-64/deb/cap-1.json",
            b"the measurement".as_slice(),
        ),
        (
            "raw/v1/fixture-client/1.2.3/linux/x86-64/deb/cap-1/manifest.json",
            b"the run".as_slice(),
        ),
        (
            "raw/v1/fixture-client/1.2.3/linux/x86-64/deb/cap-1/observer/events.jsonl",
            b"the bytes".as_slice(),
        ),
        ("indexes/v1/profiles.json", b"the lookups".as_slice()),
        ("formats/bit-ids-v1.json", b"the rendering".as_slice()),
        ("LICENSE", b"the licence".as_slice()),
    ] {
        tree.insert(path(at), object(bytes));
    }
    let release = assemble(&tree).expect("the tree assembles");
    let manifest = release.manifest_json();
    let checksums = release.checksums(manifest.as_bytes());
    contract(&release, manifest.as_bytes(), checksums.as_bytes()).expect("the contract derives")
}

fn refuses(document: &AccessContract, code: &str) {
    let violations = validate_contract(document).expect_err("the plant is refused");
    assert!(
        violations.has(code),
        "expected {code}, got {:?}",
        violations.codes().collect::<Vec<_>>()
    );
}

fn at<'a>(document: &'a mut AccessContract, text: &str) -> &'a mut AccessPath {
    document
        .paths
        .iter_mut()
        .find(|entry| entry.path.as_str() == text)
        .expect("the contract carries that path")
}

#[test]
fn access_a_derived_publication_describes_both_classes() {
    let document = published();
    // ⚠ Both non-empty, or every comparison below holds vacuously.
    assert_eq!(
        document.immutable().len(),
        3,
        "the record, its run, its bytes"
    );
    assert!(
        document.current().len() >= 4,
        "the derived files and the two roots"
    );
    assert_eq!(document.branch, DATA_BRANCH);
    validate_contract(&document).expect("a derived contract validates");
}

#[test]
fn access_refuses_a_measurement_path_declared_current() {
    // ⛔ The rule the whole document exists for, planted from the document side
    // where the deriver cannot reach it: a consumer told to refetch a record
    // gets the same bytes forever, and one told to cache an index serves a
    // measurement that has been retracted.
    let mut document = published();
    at(
        &mut document,
        "profiles/v1/fixture-client/1.2.3/linux/x86-64/deb/cap-1.json",
    )
    .stability = Stability::Current;
    refuses(&document, "E-ACC-04");
}

#[test]
fn access_refuses_a_derived_path_declared_immutable() {
    let mut document = published();
    at(&mut document, "indexes/v1/profiles.json").stability = Stability::Immutable;
    refuses(&document, "E-ACC-04");
}

#[test]
fn access_refuses_an_integrity_source_the_path_does_not_imply() {
    // The manifest cannot prove itself, and a contract saying it does sends a
    // consumer to a document for a digest that is not in it.
    let mut document = published();
    at(&mut document, "MANIFEST.json").integrity = Integrity::Manifest;
    refuses(&document, "E-ACC-05");

    let mut document = published();
    at(&mut document, "SHA256SUMS").integrity = Integrity::Manifest;
    refuses(&document, "E-ACC-05");
}

#[test]
fn access_refuses_a_contract_with_no_manifest_or_no_checksums() {
    for missing in ["MANIFEST.json", "SHA256SUMS"] {
        let mut document = published();
        document
            .paths
            .retain(|entry| entry.path.as_str() != missing);
        refuses(&document, "E-ACC-02");
    }
}

#[test]
fn access_refuses_a_contract_for_another_branch() {
    let mut document = published();
    document.branch = "main".to_owned();
    refuses(&document, "E-ACC-02");
}

#[test]
fn access_refuses_paths_out_of_order_or_carried_twice() {
    let mut document = published();
    document.paths.swap(0, 1);
    refuses(&document, "E-ACC-03");

    let mut document = published();
    let duplicate = document.paths[0].clone();
    document.paths.insert(1, duplicate);
    refuses(&document, "E-ACC-03");
}

#[test]
fn access_refuses_a_published_path_it_cannot_classify() {
    // ⛔ `routes/` is the live case. It was in `docs/publishing.md`'s layout,
    // nothing ever wrote it, and it cannot be written in that shape because the
    // path omits `<package>` while the routes differ per package. A default
    // stability would have published it with a caching rule nobody decided.
    let mut tree = StoreTree::new();
    tree.insert(
        path("profiles/v1/fixture-client/1.2.3/linux/x86-64/deb/cap-1.json"),
        object(b"the measurement"),
    );
    tree.insert(
        path("routes/v1/fixture-client/latest/linux/x86-64.json"),
        object(b"a path nothing writes"),
    );
    let release = assemble(&tree).expect("the tree assembles");
    let manifest = release.manifest_json();
    let checksums = release.checksums(manifest.as_bytes());
    let violations = contract(&release, manifest.as_bytes(), checksums.as_bytes())
        .expect_err("an unclassifiable path blocks the contract");
    assert!(violations.has("E-ACC-01"), "{violations}");
}

#[test]
fn access_documents_round_trip_through_their_canonical_form() {
    let document = published();
    let text = document.to_json().expect("document");
    let read = AccessContract::from_json(&text).expect("a written contract reads back");
    assert_eq!(read, document);
    assert_eq!(read.to_json().expect("document"), text);
}

#[test]
fn access_refuses_a_document_from_another_generation() {
    let text = published()
        .to_json()
        .expect("document")
        .replace(ACCESS_SCHEMA, "bit-ids/access/2");
    let error = AccessContract::from_json(&text).expect_err("another generation is refused");
    assert!(
        error.to_string().contains("bit-ids/access/2"),
        "the message names what was found: {error}"
    );
}

#[test]
fn access_writes_nothing_it_would_refuse_to_read() {
    let mut document = published();
    at(&mut document, "indexes/v1/profiles.json").stability = Stability::Immutable;
    // ⛔ The write path validates, so an unproven contract has no canonical form.
    assert!(document.to_json().is_err());
    // And the read path refuses the same bytes, which is what makes a contract
    // on disk one a consumer can trust.
    let text = serde_json::to_string(&document).expect("serde still serializes");
    assert!(AccessContract::from_json(&text).is_err());
}

#[test]
fn access_every_path_kind_is_reachable_from_a_publication() {
    // ⚠ A kind no publication produces is a vocabulary entry describing nothing,
    // and `PathKind::ALL` is what a spelling test iterates. Every kind except
    // the two root documents comes out of the tree; those two are added by
    // `contract` itself, and this asserts all eight arrive.
    let document = published();
    let mut seen: Vec<PathKind> = document.paths.iter().map(|entry| entry.kind).collect();
    seen.sort_by_key(|kind| kind.as_str());
    seen.dedup();
    let mut expected: Vec<PathKind> = PathKind::ALL.to_vec();
    expected.sort_by_key(|kind| kind.as_str());
    assert_eq!(seen, expected);
}
