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

use std::path::Path;

use bit_ids::canonical::{Label, Slug};
use bit_ids::corpus::Corpus;
use bit_ids::resolution::VersionScheme;
use bit_ids::store::{is_manifest_path, is_profile_path};
use bit_ids::{Profile, RunManifest};

#[path = "walk.rs"]
mod walked;

/// Parses one `--scheme TARGET:PREFIX:MIN:MAX` argument.
///
/// `-` in the prefix position means the target publishes versions with no tag
/// prefix. ⛔ Nothing here has a default: a scheme this cannot parse is refused
/// rather than filled in, because a filled-in scheme orders versions under a
/// shape nobody declared.
pub fn scheme(text: &str) -> Result<(Slug, VersionScheme), String> {
    let parts: Vec<&str> = text.split(':').collect();
    let [target, prefix, min, max] = parts.as_slice() else {
        return Err(format!("{text:?}: expected TARGET:PREFIX:MIN:MAX"));
    };
    let target = Slug::parse(target).map_err(|error| format!("{text:?}: {error}"))?;
    let tag_prefix = if *prefix == "-" {
        None
    } else {
        Some(Label::parse(prefix).map_err(|error| format!("{text:?}: {error}"))?)
    };
    let min_components: u8 = min.parse().map_err(|_| format!("{text:?}: min"))?;
    let max_components: u8 = max.parse().map_err(|_| format!("{text:?}: max"))?;
    if min_components == 0 || min_components > max_components {
        return Err(format!("{text:?}: min must be 1 or more and at most max"));
    }
    Ok((
        target,
        VersionScheme {
            tag_prefix,
            min_components,
            max_components,
        },
    ))
}

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
