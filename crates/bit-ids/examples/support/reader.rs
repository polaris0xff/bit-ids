//! Read a store directory into a [`Corpus`], and parse a declared version
//! scheme.
//!
//! ⛔ **One reader, included by every example that needs one**, for the reason
//! [`walk.rs`](walk.rs) gives about the walk it wraps. Which paths are records,
//! what a refusal is, and whether an unreadable document is dropped or reported
//! are decisions two copies would answer differently within a session, and the
//! copy that drifted would be the one nobody ran.
//!
//! ⚠ It includes the walk itself, so an example that includes this must not
//! also include `walk.rs`: the file would be compiled twice into one crate and
//! the two `StoreTree` walks would be different types.
//!
//! ⚠ The `--scheme` parser lives in [`scheme.rs`](scheme.rs) rather than here.
//! It was here until `survey-staleness` needed a store reader and no scheme
//! parser, and compiled one nothing called.

use std::path::Path;

use bit_ids::corpus::Corpus;
use bit_ids::store::{is_manifest_path, is_profile_path};
use bit_ids::{Profile, RunManifest};

#[path = "walk.rs"]
mod walked;

/// Reads every record and run the store carries into a corpus, reporting what
/// it could not read rather than dropping it.
pub fn read_store(root: &Path, refusals: &mut Vec<String>) -> Result<Corpus, String> {
    let walk = walked::walk(root)?;
    refusals.extend(walk.refused.iter().cloned());
    let mut corpus = Corpus::new(walk.tree.clone());
    for (path, entry) in walk.tree.iter() {
        if entry.object().is_none() {
            continue;
        }
        let profile = is_profile_path(path);
        let manifest = is_manifest_path(path);
        if !profile && !manifest {
            continue;
        }
        let document = match std::fs::read_to_string(root.join(path.as_str())) {
            Ok(document) => document,
            Err(error) => {
                refusals.push(format!("{path}: {error}"));
                continue;
            }
        };
        if profile {
            match Profile::from_json(&document) {
                Ok(record) => corpus.insert_profile(path.clone(), record),
                Err(error) => refusals.push(format!("{path}: {error}")),
            }
        } else {
            match RunManifest::from_json(&document) {
                Ok(record) => corpus.insert_manifest(path.clone(), record),
                Err(error) => refusals.push(format!("{path}: {error}")),
            }
        }
    }
    Ok(corpus)
}
