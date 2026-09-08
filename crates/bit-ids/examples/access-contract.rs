//! Derive the access contract for a published tree, and check the tree against
//! its own two descriptions first.
//!
//! ```text
//! cargo run -p bit-ids --example access-contract -- TREE [OUT]
//! ```
//!
//! This is the driving surface for `PUB-04`. `TREE` is a checkout of the `data`
//! branch, or any directory holding one publication.
//!
//! ⛔ **It re-derives `MANIFEST.json` and `SHA256SUMS` from the published bytes
//! and compares them against the published ones before saying anything about
//! access.** A contract listing digests copied out of the manifest would agree
//! with the manifest whatever the bytes on disk are, which is the whole failure
//! mode a consumer trusts the manifest to prevent. Re-assembling is what turns
//! "the manifest describes this tree" into a fact.
//!
//! ⚠ The two root documents are taken out of the tree before assembly, because
//! `assemble` describes everything it is given and refuses a tree that already
//! carries them: those describe another run. They are put back into the contract
//! by [`bit_ids::access::contract`], which is where the reason lives.
//!
//! ⛔ **A path this build cannot classify blocks the contract.** There is no
//! default stability, for the reason `PUB-01` gives about a media type: a
//! consumer told it may cache a file that moves serves a stale measurement, and
//! the failure lands on the reader.
//!
//! Exit codes follow `docs/capture-methodology.md`: 0 the contract was derived,
//! 1 the tree or a path was refused, 2 the route could not run.

use std::io::Write as _;
use std::path::Path;
use std::process::ExitCode;

use bit_ids::access::contract;
use bit_ids::release::{CHECKSUMS_FILE, RELEASE_MANIFEST_FILE, Release, assemble, manifest_covers};
use bit_ids::store::StoreTree;

#[path = "support/walk.rs"]
mod support;

use support::walk;

fn report(refusals: &[String]) -> ExitCode {
    let mut stderr = std::io::stderr();
    let _ = writeln!(stderr, "refused: {} finding(s)", refusals.len());
    for refusal in refusals {
        let _ = writeln!(stderr, "  {refusal}");
    }
    ExitCode::from(1)
}

/// The three things a published tree owes before anything is said about access:
/// it reads, its manifest describes the bytes on disk, and its checksum file
/// covers them.
///
/// Returns the release the tree re-assembles to, and the two published documents
/// as bytes, because those are what a consumer fetches and what their digests
/// are taken over.
fn read_publication(root: &Path) -> Result<(Release, Vec<u8>, Vec<u8>), ExitCode> {
    let walked = match walk(root) {
        Ok(walked) => walked,
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "cannot read the tree: {error}");
            return Err(ExitCode::from(2));
        }
    };
    let mut refusals = walked.refused.clone();

    // ⚠ Read from disk rather than from the walk, because what is compared is
    // the bytes a consumer would fetch.
    let published_manifest = match std::fs::read(root.join(RELEASE_MANIFEST_FILE)) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "{RELEASE_MANIFEST_FILE}: {error}");
            return Err(ExitCode::from(1));
        }
    };
    let published_checksums = match std::fs::read(root.join(CHECKSUMS_FILE)) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "{CHECKSUMS_FILE}: {error}");
            return Err(ExitCode::from(1));
        }
    };

    let mut described = StoreTree::new();
    for (path, entry) in walked.tree.iter() {
        if path.as_str() == RELEASE_MANIFEST_FILE || path.as_str() == CHECKSUMS_FILE {
            continue;
        }
        described.insert(path.clone(), *entry);
    }

    let release = match assemble(&described) {
        Ok(release) => release,
        Err(violations) => {
            for error in violations.errors() {
                refusals.push(error.to_string());
            }
            return Err(report(&refusals));
        }
    };
    if let Err(violations) = manifest_covers(&release, &described) {
        for error in violations.errors() {
            refusals.push(error.to_string());
        }
    }

    // ⛔ THE COMPARISON THAT MAKES EVERY DIGEST BELOW MEAN SOMETHING. A manifest
    // re-derived from the bytes on disk and the manifest that was published are
    // two facts, and a contract quoting the second without checking the first
    // describes a tree it never read.
    let rederived_manifest = release.manifest_json();
    if rederived_manifest.as_bytes() != published_manifest.as_slice() {
        refusals.push(format!(
            "{RELEASE_MANIFEST_FILE} does not describe the bytes in this tree"
        ));
    }
    let rederived_checksums = release.checksums(published_manifest.as_slice());
    if rederived_checksums.as_bytes() != published_checksums.as_slice() {
        refusals.push(format!(
            "{CHECKSUMS_FILE} does not cover the bytes in this tree"
        ));
    }
    if !refusals.is_empty() {
        return Err(report(&refusals));
    }
    Ok((release, published_manifest, published_checksums))
}

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    let (Some(root_arg), out_arg) = (args.next(), args.next()) else {
        let _ = writeln!(
            std::io::stderr(),
            "usage: access-contract TREE [OUT]\n\
             derives the documented access paths for one published tree"
        );
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        let _ = writeln!(std::io::stderr(), "usage: access-contract TREE [OUT]");
        return ExitCode::from(2);
    }
    let root = Path::new(&root_arg);
    if !root.is_dir() {
        let _ = writeln!(std::io::stderr(), "{}: not a directory", root.display());
        return ExitCode::from(2);
    }
    let (release, published_manifest, published_checksums) = match read_publication(root) {
        Ok(parts) => parts,
        Err(code) => return code,
    };

    let mut refusals: Vec<String> = Vec::new();
    let access = match contract(&release, &published_manifest, &published_checksums) {
        Ok(access) => access,
        Err(violations) => {
            for error in violations.errors() {
                refusals.push(error.to_string());
            }
            return report(&refusals);
        }
    };
    let document = match access.to_json() {
        Ok(document) => document,
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "the contract is not valid: {error}");
            return ExitCode::from(2);
        }
    };

    if let Some(out) = out_arg {
        if let Err(error) = std::fs::write(Path::new(&out), document.as_bytes()) {
            let _ = writeln!(std::io::stderr(), "cannot write the contract: {error}");
            return ExitCode::from(2);
        }
    } else {
        let mut stdout = std::io::stdout();
        let _ = write!(stdout, "{document}");
    }

    // ⚠ Both counts, because a contract that is all one class is a contract that
    // classified nothing. A publication always has some of each: measurements
    // never move and the documents describing them always do.
    let mut stderr = std::io::stderr();
    let _ = writeln!(
        stderr,
        "{} path(s): {} immutable, {} current, on branch {}",
        access.paths.len(),
        access.immutable().len(),
        access.current().len(),
        access.branch
    );
    ExitCode::SUCCESS
}
