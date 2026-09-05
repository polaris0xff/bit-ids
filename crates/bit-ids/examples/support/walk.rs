//! Read a directory into a [`StoreTree`].
//!
//! ⛔ **One walk, included by every example that needs one.** It is not an
//! example target of its own: cargo discovers `examples/*.rs` and only those
//! subdirectories carrying a `main.rs`, so a file under `examples/support/` is
//! compiled where it is included and nowhere else. Copying this into a second
//! example is the divergent-copies row in
//! `docs/conventions/forbidden-patterns.md`, and the copies would differ in
//! exactly the place that matters, which is what a walk does about a symlink.
//!
//! ⚠ The filesystem stays out of the `bit-ids` crate, whose rules are pure over
//! a tree that is already in memory. This is the caller that reads one.

use std::path::Path;

use bit_ids::canonical::{RelPath, Sha256Digest};
use bit_ids::store::{Entry, ObjectRef, StoreTree};

/// What one walk produced: the tree, and every path it could not put in one.
pub struct Walk {
    /// The objects the walk could name canonically.
    pub tree: StoreTree,
    /// Paths a published tree cannot carry, reported rather than dropped.
    ///
    /// ⚠ Dropping one would make the append-only comparison see a deletion that
    /// is not one, which is a refusal pointing at the wrong file.
    pub refused: Vec<String>,
}

/// Reads one directory into a tree, without following a link out of it.
///
/// # Errors
///
/// Returns a message when a directory or a file cannot be read at all. A path
/// that reads but cannot be published is a refusal in [`Walk::refused`] instead,
/// because the walk succeeded and the tree is the thing that is wrong.
pub fn walk(root: &Path) -> Result<Walk, String> {
    let mut walk = Walk {
        tree: StoreTree::new(),
        refused: Vec::new(),
    };
    descend(root, "", &mut walk)?;
    Ok(walk)
}

fn descend(dir: &Path, prefix: &str, walk: &mut Walk) -> Result<(), String> {
    let mut names: Vec<_> = std::fs::read_dir(dir)
        .map_err(|error| format!("{}: {error}", dir.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("{}: {error}", dir.display()))?
        .into_iter()
        .map(|entry| (entry.file_name(), entry.path()))
        .collect();
    // Sorted, so two runs over one tree report in one order.
    names.sort();

    for (name, path) in names {
        let Some(name) = name.to_str() else {
            walk.refused.push(format!(
                "{}: a name that is not UTF-8 cannot be a published path",
                path.display()
            ));
            continue;
        };
        let relative = if prefix.is_empty() {
            name.to_owned()
        } else {
            format!("{prefix}/{name}")
        };

        // ⛔ symlink_metadata, never metadata. The latter follows the link and
        // would report a link to a directory as a directory, which is how a walk
        // leaves the tree it was given.
        let kind = std::fs::symlink_metadata(&path)
            .map_err(|error| format!("{}: {error}", path.display()))?
            .file_type();

        if kind.is_dir() {
            descend(&path, &relative, walk)?;
            continue;
        }

        let Ok(rel) = RelPath::parse(&relative) else {
            walk.refused.push(format!(
                "{relative}: not a canonical relative path, so it cannot be published"
            ));
            continue;
        };

        let entry = if kind.is_symlink() {
            Entry::Symlink
        } else if kind.is_file() {
            let bytes = std::fs::read(&path).map_err(|error| format!("{relative}: {error}"))?;
            Entry::Object(ObjectRef {
                bytes: bytes.len() as u64,
                sha256: Sha256Digest::of(&bytes),
            })
        } else {
            Entry::Other
        };

        if walk.tree.insert(rel, entry).is_some() {
            walk.refused
                .push(format!("{relative}: read twice from one walk"));
        }
    }
    Ok(())
}
