//! Compare a published store against the successor a run proposes.
//!
//! This is the driving surface for `CORPUS-01`: the path the data-branch
//! publisher in `docs/publishing.md` takes, run over two directories on a disk
//! rather than through the library's own harness.
//!
//! ```text
//! cargo run -p bit-ids --example check-store -- PRIOR NEXT
//! ```
//!
//! `PRIOR` is the tree that is already published and `NEXT` is the one about to
//! be. Both must exist; an empty directory is how a first publication says it
//! has no predecessor, because a missing directory and a typo are the same
//! bytes and one of them would pass everything.
//!
//! Three questions, in order, and all three are asked even when the first
//! answers badly, so one run reports every refusal rather than the first:
//!
//! 1. can the successor be checked out at all, on every platform in the matrix;
//! 2. is every record filed where its own contents say it belongs;
//! 3. does the successor leave every published path exactly as it was.
//!
//! Exit codes follow `docs/capture-methodology.md`: 0 the successor is
//! publishable, 1 it was read and refused, 2 the route could not run.
//!
//! ⛔ It does not follow a symbolic link. A link is reported as one and refused,
//! which is `E-STO-14`. Following one would walk a tree the store does not own
//! and digest bytes it cannot republish.

use std::io::Write as _;
use std::path::Path;
use std::process::ExitCode;

use bit_ids::canonical::{RelPath, Sha256Digest};
use bit_ids::store::{
    Entry, ObjectRef, StoreKey, StoreTree, append_only, check_manifest_placement,
    check_profile_placement, is_manifest_path, is_profile_path, validate_tree,
};
use bit_ids::{Profile, RunManifest};

/// What one walk produced: the tree, and every path it could not put in one.
struct Walk {
    tree: StoreTree,
    refused: Vec<String>,
}

/// Reads one directory into a tree, without following a link out of it.
fn walk(root: &Path) -> Result<Walk, String> {
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

/// Reads every record the successor carries and checks it is filed where its own
/// identity puts it.
///
/// ⛔ **It reads only what the walk called an object, and that gate is not
/// belt-and-braces.** A path is selected here by its name, and the walk is the
/// only thing that knows what is actually at it. Opening one by name reached a
/// named pipe on the first driven run of this example and blocked forever, with
/// `validate_tree` already carrying the refusal for it: one action, two doors,
/// and the second one had no gate. The pipe is `E-STO-15` and is reported by the
/// check that owns entry kinds.
fn check_placement(root: &Path, tree: &StoreTree) -> Vec<String> {
    let mut out = Vec::new();
    for (path, entry) in tree.iter() {
        if entry.object().is_none() {
            continue;
        }
        let profile = is_profile_path(path);
        let manifest = is_manifest_path(path);
        if !profile && !manifest {
            continue;
        }
        let on_disk = root.join(path.as_str());
        let document = match std::fs::read_to_string(&on_disk) {
            Ok(document) => document,
            Err(error) => {
                out.push(format!("{path}: {error}"));
                continue;
            }
        };
        if profile {
            match Profile::from_json(&document) {
                Ok(record) => {
                    if let Err(error) = check_profile_placement(path, &record) {
                        out.push(error.to_string());
                    }
                }
                Err(error) => out.push(format!("{path}: {error}")),
            }
        } else {
            match RunManifest::from_json(&document) {
                Ok(record) => {
                    if let Err(error) = check_manifest_placement(path, &record) {
                        out.push(error.to_string());
                    }
                }
                Err(error) => out.push(format!("{path}: {error}")),
            }
        }
    }
    out
}

fn usage() -> ExitCode {
    let _ = writeln!(
        std::io::stderr(),
        "usage: check-store PRIOR NEXT\n\
         compares a published store tree against the successor a run proposes\n\
         \n\
         usage: check-store --where FILE\n\
         prints the store path the record in FILE belongs at"
    );
    ExitCode::from(2)
}

/// Prints where one record belongs.
///
/// ⛔ It exists so that a caller which has to place a record does not derive the
/// path a second time. `scripts/corpus/check-store.sh` builds whole trees and
/// would otherwise carry its own copy of the layout, which is two
/// implementations of one rule and the drift `check-twins.sh` was written about.
fn locate(file: &Path) -> ExitCode {
    let document = match std::fs::read_to_string(file) {
        Ok(document) => document,
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "{}: {error}", file.display());
            return ExitCode::from(2);
        }
    };
    let derived = match Profile::from_json(&document) {
        Ok(record) => StoreKey::of_profile(&record).profile_path(),
        Err(profile_error) => match RunManifest::from_json(&document) {
            Ok(record) => StoreKey::of_manifest(&record).manifest_path(),
            Err(manifest_error) => {
                let _ = writeln!(
                    std::io::stderr(),
                    "{}: neither a profile nor a manifest\n  as a profile: {profile_error}\n  \
                     as a manifest: {manifest_error}",
                    file.display()
                );
                return ExitCode::from(1);
            }
        },
    };
    match derived {
        Ok(path) => {
            let _ = writeln!(std::io::stdout(), "{path}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "{}: {error}", file.display());
            ExitCode::from(1)
        }
    }
}

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    let first = args.next();
    if first.as_deref() == Some(std::ffi::OsStr::new("--where")) {
        let (Some(file), None) = (args.next(), args.next()) else {
            return usage();
        };
        return locate(Path::new(&file));
    }
    let (Some(prior_arg), Some(next_arg), None) = (first, args.next(), args.next()) else {
        return usage();
    };
    let prior_root = Path::new(&prior_arg).to_path_buf();
    let next_root = Path::new(&next_arg).to_path_buf();

    for root in [&prior_root, &next_root] {
        if !root.is_dir() {
            let _ = writeln!(
                std::io::stderr(),
                "{}: not a directory. An empty one is how a first publication says it has no \
                 predecessor",
                root.display()
            );
            return ExitCode::from(2);
        }
    }

    let prior = match walk(&prior_root) {
        Ok(walk) => walk,
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "cannot read the published tree: {error}");
            return ExitCode::from(2);
        }
    };
    let next = match walk(&next_root) {
        Ok(walk) => walk,
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "cannot read the proposed tree: {error}");
            return ExitCode::from(2);
        }
    };

    // ⛔ Every question is asked even after one has answered badly. A checker
    // that stopped at the first refusal would report one defect per run, and the
    // next run would find the next one.
    let mut refusals: Vec<String> = Vec::new();

    // ⚠ BOTH TREES ARE WALK-CHECKED AND ONLY THE SUCCESSOR IS RULE-CHECKED, and
    // the asymmetry is deliberate. A path the walk could not read on the
    // published side makes the comparison below see a deletion that is not one,
    // so it has to be loud. But the published tree is already published, and the
    // store is append-only: refusing it for a structural defect would block
    // every future publication over something nobody is now allowed to fix. The
    // successor is where a rule can still be met.
    for (side, walk) in [("published", &prior), ("proposed", &next)] {
        for problem in &walk.refused {
            refusals.push(format!("{side} tree: {problem}"));
        }
    }
    if let Err(violations) = validate_tree(&next.tree) {
        for error in violations.errors() {
            refusals.push(error.to_string());
        }
    }
    refusals.extend(check_placement(&next_root, &next.tree));
    if let Err(violations) = append_only(&prior.tree, &next.tree) {
        for error in violations.errors() {
            refusals.push(error.to_string());
        }
    }

    if refusals.is_empty() {
        let appended = next.tree.len() - prior.tree.len();
        let mut stdout = std::io::stdout();
        let _ = writeln!(
            stdout,
            "append-only {} -> {} object(s), {appended} appended, 0 changed, 0 removed",
            prior.tree.len(),
            next.tree.len()
        );
        return ExitCode::SUCCESS;
    }

    let mut stderr = std::io::stderr();
    let _ = writeln!(stderr, "refused: {} finding(s)", refusals.len());
    for refusal in &refusals {
        let _ = writeln!(stderr, "  {refusal}");
    }
    ExitCode::from(1)
}
