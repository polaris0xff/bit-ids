//! Assemble a release: describe every file once, then read the result back.
//!
//! ```text
//! cargo run -p bit-ids --example assemble-release -- DIR
//! ```
//!
//! This is the driving surface for `PUB-01`. `DIR` is a tree that already holds
//! everything a release carries except its own two descriptions; this writes
//! `MANIFEST.json` and `SHA256SUMS` into it and prints the manifest's digest,
//! which is what two runs compare.
//!
//! ⛔ **It re-walks the directory afterwards and checks the manifest against
//! it.** A writer that reports the digest of what it meant to write cannot
//! detect a short write, and a manifest is exactly the document a reader trusts
//! instead of looking. Reading it back is the only thing that turns "assembled"
//! into a fact.
//!
//! ⚠ It writes into the directory it is given, which is why it refuses one that
//! already carries either document: those describe another run.
//!
//! Exit codes follow `docs/capture-methodology.md`: 0 the release was
//! assembled and read back, 1 it was refused, 2 the route could not run.

use std::io::Write as _;
use std::path::Path;
use std::process::ExitCode;

use bit_ids::release::{CHECKSUMS_FILE, RELEASE_MANIFEST_FILE, assemble, manifest_covers};

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

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    let (Some(root_arg), None) = (args.next(), args.next()) else {
        let _ = writeln!(
            std::io::stderr(),
            "usage: assemble-release DIR\n\
             writes MANIFEST.json and SHA256SUMS describing everything else in DIR"
        );
        return ExitCode::from(2);
    };
    let root = Path::new(&root_arg);
    if !root.is_dir() {
        let _ = writeln!(std::io::stderr(), "{}: not a directory", root.display());
        return ExitCode::from(2);
    }

    let walked = match walk(root) {
        Ok(walked) => walked,
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "cannot read the tree: {error}");
            return ExitCode::from(2);
        }
    };
    let mut refusals = walked.refused.clone();

    let release = match assemble(&walked.tree) {
        Ok(release) => release,
        Err(violations) => {
            for error in violations.errors() {
                refusals.push(error.to_string());
            }
            return report(&refusals);
        }
    };
    if !refusals.is_empty() {
        return report(&refusals);
    }

    let manifest = release.manifest_json();
    let sums = release.checksums(manifest.as_bytes());
    for (name, body) in [
        (RELEASE_MANIFEST_FILE, manifest.as_str()),
        (CHECKSUMS_FILE, sums.as_str()),
    ] {
        if let Err(error) = std::fs::write(root.join(name), body.as_bytes()) {
            let _ = writeln!(std::io::stderr(), "cannot write {name}: {error}");
            return ExitCode::from(2);
        }
    }

    // ⛔ Read back from the disk, not from the buffer that was just written.
    let after = match walk(root) {
        Ok(after) => after,
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "cannot re-read the tree: {error}");
            return ExitCode::from(2);
        }
    };
    if let Err(violations) = manifest_covers(&release, &after.tree) {
        for error in violations.errors() {
            refusals.push(error.to_string());
        }
        return report(&refusals);
    }

    let mut stdout = std::io::stdout();
    let _ = writeln!(
        stdout,
        "{} {} file(s) described, {} checksum row(s)",
        bit_ids::canonical::Sha256Digest::of(manifest.as_bytes()),
        release.len(),
        sums.lines().count()
    );
    ExitCode::SUCCESS
}
