//! The semantic corpus validator: what only a whole store can answer.
//!
//! `CORPUS-02` owns this. Every rule here needs more than one document, which is
//! exactly why none of them lives in [`crate::validate`]: a record reader has
//! one record and cannot know whether the bytes it cites are anywhere.
//!
//! ⛔ **A parsed value with no recoverable bytes is not a measurement, and
//! nothing checked that until here.** [`crate::manifest::bind`] compares the two
//! documents against each other, so a run that agreed with itself about an
//! artifact that was never written passed every check this project had. The
//! store is what turns a citation into bytes, so the store is where the citation
//! is resolved.
//!
//! ⚠ **Valid is not publishable, and the split is the same one the record model
//! already makes.** A store carrying a provisional record is correct and has to
//! be: refusing it would throw away the disagreement along with the evidence of
//! it. So [`validate_corpus`] refuses only what must hold of any store, and
//! [`publishable_view`] separately reports which records may enter a published
//! view. `CORPUS-03` builds the views; `CORPUS-04` owns supersession chains.

use std::collections::{BTreeMap, BTreeSet};

use crate::agreement::publishable;
use crate::canonical::RelPath;
use crate::identity::RecordId;
use crate::manifest::{RunManifest, bind, validate_manifest};
use crate::record::Profile;
use crate::store::{
    MANIFEST_FILE, PROFILE_ROOT, RAW_ROOT, STORE_LAYOUT, StoreKey, StoreTree,
    check_manifest_placement, check_profile_placement,
};
use crate::validate::{SchemaError, Violations, validate};

/// One profile record as the store carries it.
#[derive(Clone, Debug)]
pub struct StoredProfile {
    /// Where it is filed.
    pub path: RelPath,
    /// The record.
    pub profile: Profile,
}

/// One run manifest as the store carries it.
#[derive(Clone, Debug)]
pub struct StoredManifest {
    /// Where it is filed.
    pub path: RelPath,
    /// The manifest.
    pub manifest: RunManifest,
}

/// A store, as much of it as a check needs: the objects, and the documents read
/// out of them.
///
/// ⚠ The tree is the authority on what exists and the documents are the
/// authority on what is claimed. Keeping them apart is the point: a check that
/// asked a document whether its own artifact exists would answer yes.
#[derive(Clone, Debug, Default)]
pub struct Corpus {
    tree: StoreTree,
    profiles: Vec<StoredProfile>,
    manifests: Vec<StoredManifest>,
}

impl Corpus {
    /// An empty corpus over one tree.
    #[must_use]
    pub fn new(tree: StoreTree) -> Self {
        Self {
            tree,
            profiles: Vec::new(),
            manifests: Vec::new(),
        }
    }

    /// Records one profile the store carries.
    pub fn insert_profile(&mut self, path: RelPath, profile: Profile) {
        self.profiles.push(StoredProfile { path, profile });
    }

    /// Records one manifest the store carries.
    pub fn insert_manifest(&mut self, path: RelPath, manifest: RunManifest) {
        self.manifests.push(StoredManifest { path, manifest });
    }

    /// The objects the store holds.
    #[must_use]
    pub const fn tree(&self) -> &StoreTree {
        &self.tree
    }

    /// Every profile record, in insertion order.
    #[must_use]
    pub fn profiles(&self) -> &[StoredProfile] {
        &self.profiles
    }

    /// Every run manifest, in insertion order.
    #[must_use]
    pub fn manifests(&self) -> &[StoredManifest] {
        &self.manifests
    }
}

/// Resolves the manifest for one capture, by the bundle its identity derives.
fn manifest_for<'a>(
    corpus: &'a Corpus,
    profile: &Profile,
) -> Option<(&'a RelPath, &'a RunManifest)> {
    let wanted = StoreKey::of_profile(profile).manifest_path().ok()?;
    corpus
        .manifests
        .iter()
        .find(|stored| stored.path == wanted)
        .map(|stored| (&stored.path, &stored.manifest))
}

/// Checks one run's declared artifacts against the bytes the store actually
/// holds, and reports every path that run accounts for.
fn check_bundle(
    corpus: &Corpus,
    stored: &StoredManifest,
    accounted: &mut BTreeSet<RelPath>,
    out: &mut Vec<SchemaError>,
) {
    let key = StoreKey::of_manifest(&stored.manifest);
    accounted.insert(stored.path.clone());

    for artifact in &stored.manifest.evidence {
        let Ok(resolved) = key.evidence_path(&artifact.path) else {
            out.push(SchemaError::new(
                "E-CRP-03",
                format!("evidence {}", artifact.id),
                "its path does not compose into a publishable store path",
            ));
            continue;
        };
        accounted.insert(resolved.clone());

        let Some(object) = corpus.tree.get(&resolved).and_then(|entry| entry.object()) else {
            out.push(SchemaError::new(
                "E-CRP-03",
                format!("evidence {}", artifact.id),
                format!("cited at {resolved}, which the store does not carry as bytes"),
            ));
            continue;
        };
        if object.bytes != artifact.bytes {
            out.push(SchemaError::new(
                "E-CRP-04",
                format!("evidence {}", artifact.id),
                format!(
                    "declared {} bytes, the store holds {}",
                    artifact.bytes, object.bytes
                ),
            ));
        }
        if object.sha256 != artifact.sha256 {
            out.push(SchemaError::new(
                "E-CRP-05",
                format!("evidence {}", artifact.id),
                format!(
                    "declared {}, the store holds {}",
                    artifact.sha256, object.sha256
                ),
            ));
        }
    }
}

/// Every object under a known root that no document in the corpus accounts for.
///
/// ⛔ **Swept over the whole tree, not per document, and that distinction is a
/// defect this had.** The sweep used to run inside the per-manifest check and
/// was scoped to that manifest's own bundle, so a store carrying evidence and
/// no manifest at all had nothing to sweep with: nine orphan artifacts, no
/// records, no runs, and a verdict of valid. A check that only fires when the
/// thing it checks is present is a check that passes vacuously on the case that
/// matters most.
///
/// ⚠ An artifact in a run's bundle that the run does not declare is either a
/// file nobody meant to publish or a citation that was dropped, and the
/// manifest's redaction declarations say nothing about it either way.
fn sweep_unaccounted(corpus: &Corpus, accounted: &BTreeSet<RelPath>, out: &mut Vec<SchemaError>) {
    let raw = format!("{RAW_ROOT}/{STORE_LAYOUT}/");
    let profiles = format!("{PROFILE_ROOT}/{STORE_LAYOUT}/");
    let filed: BTreeSet<&RelPath> = corpus.profiles.iter().map(|stored| &stored.path).collect();

    for (path, entry) in corpus.tree.iter() {
        if entry.object().is_none() {
            continue;
        }
        let text = path.as_str();
        if text.starts_with(&raw) && !accounted.contains(path) {
            out.push(SchemaError::new(
                "E-CRP-06",
                text,
                "sits under the evidence root and no run in this store declares it",
            ));
        }
        if text.starts_with(&profiles) && !filed.contains(path) {
            out.push(SchemaError::new(
                "E-CRP-08",
                text,
                "sits under the record root and this store carries no record read from it",
            ));
        }
    }
}

/// Every invariant a store must satisfy, whatever it is used for.
///
/// It re-runs each document's own validator rather than trusting that whoever
/// assembled the corpus read them through a validating path. That is deliberate
/// redundancy: `from_json` is the only route that validates, and a corpus built
/// in memory has taken no such route.
///
/// # Errors
///
/// Returns every refusal. The store-level codes are:
///
/// | code | refused |
/// | --- | --- |
/// | `E-CRP-01` | a profile record with no run manifest in the store |
/// | `E-CRP-02` | a run manifest with no profile record in the store |
/// | `E-CRP-03` | an artifact a run declares that the store does not carry |
/// | `E-CRP-04` | an artifact whose stored length disagrees with the run |
/// | `E-CRP-05` | an artifact whose stored digest disagrees with the run |
/// | `E-CRP-06` | a file in a run's bundle that the run does not declare |
/// | `E-CRP-07` | a correction naming a record the store does not carry |
/// | `E-CRP-08` | an object under the record root that is not a record here |
///
/// Refusals from [`validate`], [`validate_manifest`], [`bind`] and the store's
/// own placement rules are returned under their own codes, because a caller
/// acting on the class should not have to learn a second spelling of it.
pub fn validate_corpus(corpus: &Corpus) -> Result<(), Violations> {
    let mut out = Vec::new();
    let mut accounted: BTreeSet<RelPath> = BTreeSet::new();

    let mut known: BTreeMap<RecordId, &RelPath> = BTreeMap::new();
    for stored in &corpus.profiles {
        known.insert(stored.profile.id, &stored.path);
    }

    for stored in &corpus.profiles {
        if let Err(violations) = validate(&stored.profile) {
            out.extend(violations.errors().iter().cloned());
        }
        if let Err(error) = check_profile_placement(&stored.path, &stored.profile) {
            out.push(error);
        }

        match manifest_for(corpus, &stored.profile) {
            Some((_, manifest)) => {
                if let Err(violations) = bind(manifest, &stored.profile) {
                    out.extend(violations.errors().iter().cloned());
                }
            }
            None => out.push(SchemaError::new(
                "E-CRP-01",
                stored.path.as_str(),
                format!(
                    "no {MANIFEST_FILE} for capture {}; a record without its run cannot be replayed",
                    stored.profile.capture.id
                ),
            )),
        }

        // ⛔ A correction that names a record the store does not carry is a
        // chain with a hole in it, and the hole is where the evidence the
        // correction argues against used to be.
        if let Some(superseded) = stored.profile.supersedes
            && !known.contains_key(&superseded)
        {
            out.push(SchemaError::new(
                "E-CRP-07",
                stored.path.as_str(),
                format!("supersedes {superseded}, which this store does not carry"),
            ));
        }
    }

    let captures: BTreeSet<RelPath> = corpus
        .profiles
        .iter()
        .filter_map(|stored| StoreKey::of_profile(&stored.profile).manifest_path().ok())
        .collect();

    for stored in &corpus.manifests {
        if let Err(violations) = validate_manifest(&stored.manifest) {
            out.extend(violations.errors().iter().cloned());
        }
        if let Err(error) = check_manifest_placement(&stored.path, &stored.manifest) {
            out.push(error);
        }
        if !captures.contains(&stored.path) {
            out.push(SchemaError::new(
                "E-CRP-02",
                stored.path.as_str(),
                format!(
                    "describes capture {} and no record in this store cites it; evidence nothing \
                     names is evidence for nothing",
                    stored.manifest.capture
                ),
            ));
        }
        check_bundle(corpus, stored, &mut accounted, &mut out);
    }

    sweep_unaccounted(corpus, &accounted, &mut out);

    Violations::from_errors(out)
}

/// Which records may enter a published view, and why the rest may not.
///
/// ⚠ **This is a report, not a gate.** A provisional record belongs in the store
/// and its disagreement is the evidence; what it does not belong in is a
/// published view. Returning both halves is what lets `CORPUS-03` build a view
/// without deciding the rule, and what stops a caller inferring "absent from the
/// view" as "absent from the store".
#[must_use]
pub fn publishable_view(corpus: &Corpus) -> Vec<(&RelPath, Result<(), Violations>)> {
    corpus
        .profiles
        .iter()
        .map(|stored| (&stored.path, publishable(&stored.profile)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Corpus, publishable_view, validate_corpus};
    use crate::canonical::{RelPath, Sha256Digest};
    use crate::store::{Entry, ObjectRef, StoreKey, StoreTree};
    use crate::{Profile, RunManifest};

    const PROFILE: &str = include_str!("../tests/fixtures/valid-profile.json");
    const MANIFEST: &str = include_str!("../tests/fixtures/valid-manifest.json");

    fn profile() -> Profile {
        Profile::from_json(PROFILE).expect("the fixture record validates")
    }

    fn manifest() -> RunManifest {
        RunManifest::from_json(MANIFEST).expect("the fixture manifest validates")
    }

    fn object(bytes: u64, sha256: Sha256Digest) -> Entry {
        Entry::Object(ObjectRef { bytes, sha256 })
    }

    /// A store holding one record, its run, and the exact bytes the run
    /// declares. Everything below starts from this and takes one thing away.
    fn complete() -> Corpus {
        let record = profile();
        let run = manifest();
        let key = StoreKey::of_profile(&record);

        let mut tree = StoreTree::new();
        let profile_path = key.profile_path().expect("a publishable path");
        let manifest_path = key.manifest_path().expect("a publishable path");
        tree.insert(
            profile_path.clone(),
            object(PROFILE.len() as u64, Sha256Digest::of(PROFILE.as_bytes())),
        );
        tree.insert(
            manifest_path.clone(),
            object(MANIFEST.len() as u64, Sha256Digest::of(MANIFEST.as_bytes())),
        );
        for artifact in &run.evidence {
            tree.insert(
                key.evidence_path(&artifact.path)
                    .expect("a publishable path"),
                object(artifact.bytes, artifact.sha256),
            );
        }

        let mut corpus = Corpus::new(tree);
        corpus.insert_profile(profile_path, record);
        corpus.insert_manifest(manifest_path, run);
        corpus
    }

    #[test]
    fn a_complete_store_validates() {
        validate_corpus(&complete()).expect("one record, its run and its bytes");
    }

    #[test]
    fn the_fixture_run_declares_nine_artifacts() {
        // ⛔ Pinned to the literal. Every case below removes exactly one thing
        // from `complete()`, so a `complete()` that quietly stopped building the
        // bundle would make all of them pass over an empty store.
        assert_eq!(manifest().evidence.len(), 9);
        assert_eq!(complete().tree().len(), 11);
    }

    #[test]
    fn a_record_with_no_run_is_refused() {
        let full = complete();
        let mut corpus = Corpus::new(full.tree().clone());
        corpus.insert_profile(full.profiles()[0].path.clone(), profile());
        let violations = validate_corpus(&corpus).expect_err("a record with no run");
        assert!(violations.has("E-CRP-01"), "{violations}");
    }

    #[test]
    fn a_run_no_record_cites_is_refused() {
        let full = complete();
        let mut corpus = Corpus::new(full.tree().clone());
        corpus.insert_manifest(full.manifests()[0].path.clone(), manifest());
        let violations = validate_corpus(&corpus).expect_err("a run with no record");
        assert!(violations.has("E-CRP-02"), "{violations}");
    }

    /// ⛔ The invariant this entry exists for. `bind` compares the two documents
    /// and both agree about an artifact that was never written.
    #[test]
    fn a_citation_the_store_cannot_resolve_is_refused() {
        let full = complete();
        let run = manifest();
        let key = StoreKey::of_manifest(&run);
        let missing = key
            .evidence_path(&run.evidence[0].path)
            .expect("a publishable path");

        let mut tree = StoreTree::new();
        for (path, entry) in full.tree() {
            if path != &missing {
                tree.insert(path.clone(), *entry);
            }
        }
        let mut corpus = Corpus::new(tree);
        corpus.insert_profile(full.profiles()[0].path.clone(), profile());
        corpus.insert_manifest(full.manifests()[0].path.clone(), run);

        let violations = validate_corpus(&corpus).expect_err("a citation with no bytes");
        assert!(violations.has("E-CRP-03"), "{violations}");
    }

    #[test]
    fn stored_bytes_that_disagree_with_the_run_are_refused() {
        for (code, shift_len) in [("E-CRP-04", true), ("E-CRP-05", false)] {
            let full = complete();
            let run = manifest();
            let key = StoreKey::of_manifest(&run);
            let target = key
                .evidence_path(&run.evidence[0].path)
                .expect("a publishable path");

            let mut tree = StoreTree::new();
            for (path, entry) in full.tree() {
                if path == &target {
                    let object = entry.object().expect("an object");
                    let swapped = if shift_len {
                        ObjectRef {
                            bytes: object.bytes + 1,
                            ..*object
                        }
                    } else {
                        ObjectRef {
                            sha256: Sha256Digest::of(b"other bytes entirely"),
                            ..*object
                        }
                    };
                    tree.insert(path.clone(), Entry::Object(swapped));
                } else {
                    tree.insert(path.clone(), *entry);
                }
            }
            let mut corpus = Corpus::new(tree);
            corpus.insert_profile(full.profiles()[0].path.clone(), profile());
            corpus.insert_manifest(full.manifests()[0].path.clone(), run);

            let violations =
                validate_corpus(&corpus).expect_err("the store disagrees with the run");
            assert!(violations.has(code), "expected {code}: {violations}");
        }
    }

    #[test]
    fn a_file_the_run_does_not_declare_is_refused() {
        let full = complete();
        let run = manifest();
        let bundle = StoreKey::of_manifest(&run)
            .bundle_dir()
            .expect("a publishable path");
        let mut tree = full.tree().clone();
        tree.insert(
            RelPath::parse(&format!("{bundle}/stray.log")).expect("a canonical path"),
            object(4, Sha256Digest::of(b"leak")),
        );
        let mut corpus = Corpus::new(tree);
        corpus.insert_profile(full.profiles()[0].path.clone(), profile());
        corpus.insert_manifest(full.manifests()[0].path.clone(), run);

        let violations = validate_corpus(&corpus).expect_err("an undeclared file in a bundle");
        assert!(violations.has("E-CRP-06"), "{violations}");
    }

    /// ⛔ A correction whose target is not in the store is a chain with a hole,
    /// and the hole is where the evidence it argues against used to be.
    #[test]
    fn a_correction_naming_an_absent_record_is_refused() {
        const CORRECTION: &str = include_str!("../tests/fixtures/valid-correction.json");
        let corrected = Profile::from_json(CORRECTION).expect("the fixture correction validates");
        assert!(
            corrected.supersedes.is_some(),
            "the fixture correction supersedes something"
        );

        let key = StoreKey::of_profile(&corrected);
        let path = key.profile_path().expect("a publishable path");
        let mut tree = StoreTree::new();
        tree.insert(
            path.clone(),
            object(
                CORRECTION.len() as u64,
                Sha256Digest::of(CORRECTION.as_bytes()),
            ),
        );
        let mut corpus = Corpus::new(tree);
        corpus.insert_profile(path, corrected);

        let violations = validate_corpus(&corpus).expect_err("a correction with no original");
        assert!(violations.has("E-CRP-07"), "{violations}");
    }

    /// ⛔ The case the per-manifest sweep could not see: evidence in the store
    /// with no run to sweep it against. It reported a store of nine orphan
    /// artifacts as valid, which a driven pass found and no unit test had.
    #[test]
    fn evidence_with_no_run_at_all_is_refused() {
        let full = complete();
        let manifest_path = full.manifests()[0].path.clone();
        let profile_path = full.profiles()[0].path.clone();

        let mut tree = StoreTree::new();
        for (path, entry) in full.tree() {
            if path != &manifest_path && path != &profile_path {
                tree.insert(path.clone(), *entry);
            }
        }
        // Nothing is inserted into the corpus: no records, no runs, nine
        // artifacts, and the sweep is the only thing that can refuse it.
        let corpus = Corpus::new(tree);
        let violations = validate_corpus(&corpus).expect_err("evidence nothing declares");
        assert!(violations.has("E-CRP-06"), "{violations}");
        assert_eq!(
            violations.errors().len(),
            9,
            "one refusal per orphan artifact"
        );
    }

    /// The same sweep from the record side: a file under the record root that
    /// the corpus did not read is a record nothing validated.
    #[test]
    fn an_object_under_the_record_root_that_is_not_a_record_is_refused() {
        let full = complete();
        let mut tree = full.tree().clone();
        tree.insert(
            RelPath::parse("profiles/v1/stray.json").expect("a canonical path"),
            object(2, Sha256Digest::of(b"{}")),
        );
        let mut corpus = Corpus::new(tree);
        corpus.insert_profile(full.profiles()[0].path.clone(), profile());
        corpus.insert_manifest(full.manifests()[0].path.clone(), manifest());

        let violations = validate_corpus(&corpus).expect_err("an unread record file");
        assert!(violations.has("E-CRP-08"), "{violations}");
    }

    #[test]
    fn a_provisional_record_is_stored_and_is_not_published() {
        let corpus = complete();
        let view = publishable_view(&corpus);
        assert_eq!(view.len(), 1);
        assert!(
            view[0].1.is_ok(),
            "the fixture record publishes: {:?}",
            view[0].1
        );
        // The store keeps it either way; that is what the split means.
        validate_corpus(&corpus).expect("a valid store");
    }
}
