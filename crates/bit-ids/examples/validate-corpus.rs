//! Validate a whole store: the records, their runs, and the bytes they cite.
//!
//! This is the driving surface for `CORPUS-02`. `check-store` answers whether a
//! successor may replace a predecessor; this one answers whether a tree is a
//! coherent corpus at all, which is a different question and needs the documents
//! read rather than only digested.
//!
//! ```text
//! cargo run -p bit-ids --example validate-corpus -- STORE
//! ```
//!
//! ⛔ **A citation is resolved against the store, not against the other
//! document.** `bind` compares the profile and the manifest, so a run that
//! agreed with itself about an artifact nobody wrote satisfied every check this
//! project had before this one.
//!
//! It also reports which records may enter a published view. ⚠ That is a report
//! and not a verdict: a provisional record belongs in the store, because
//! refusing it would throw away the disagreement along with the evidence of it.
//! An exit code of 0 over a store whose records are all provisional is correct.
//!
//! Exit codes follow `docs/capture-methodology.md`: 0 the store is coherent,
//! 1 it was read and refused, 2 the route could not run.

use std::io::Write as _;
use std::path::Path;
use std::process::ExitCode;

use bit_ids::corpus::{Corpus, publishable_view, validate_corpus};
use bit_ids::store::{is_manifest_path, is_profile_path, validate_tree};
use bit_ids::{Profile, RunManifest};

#[path = "support/walk.rs"]
mod support;

use support::walk;

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    let (Some(root_arg), None) = (args.next(), args.next()) else {
        let _ = writeln!(
            std::io::stderr(),
            "usage: validate-corpus STORE\n\
             reads every record and run in a store and checks what only the whole store can answer"
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
            let _ = writeln!(std::io::stderr(), "cannot read the store: {error}");
            return ExitCode::from(2);
        }
    };

    let mut refusals: Vec<String> = walked.refused.clone();
    if let Err(violations) = validate_tree(&walked.tree) {
        for error in violations.errors() {
            refusals.push(error.to_string());
        }
    }

    // ⛔ Only what the walk called an object is opened. Selecting a path by its
    // name and opening it reached a named pipe and blocked forever the first
    // time `check-store` did that; the walk is the only thing that knows what is
    // actually at a path.
    let mut corpus = Corpus::new(walked.tree.clone());
    let mut records = 0_usize;
    let mut runs = 0_usize;
    for (path, entry) in walked.tree.iter() {
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
                Ok(record) => {
                    corpus.insert_profile(path.clone(), record);
                    records += 1;
                }
                Err(error) => refusals.push(format!("{path}: {error}")),
            }
        } else {
            match RunManifest::from_json(&document) {
                Ok(record) => {
                    corpus.insert_manifest(path.clone(), record);
                    runs += 1;
                }
                Err(error) => refusals.push(format!("{path}: {error}")),
            }
        }
    }

    if let Err(violations) = validate_corpus(&corpus) {
        for error in violations.errors() {
            refusals.push(error.to_string());
        }
    }

    if !refusals.is_empty() {
        let mut stderr = std::io::stderr();
        let _ = writeln!(stderr, "refused: {} finding(s)", refusals.len());
        for refusal in &refusals {
            let _ = writeln!(stderr, "  {refusal}");
        }
        return ExitCode::from(1);
    }

    // ⚠ Reported after the verdict, never folded into it. A store of provisional
    // records is a valid store, and a reader who saw one number would take
    // "publishable: 0" for a failure.
    let view = publishable_view(&corpus);
    let publishable = view.iter().filter(|(_, verdict)| verdict.is_ok()).count();
    let mut stdout = std::io::stdout();
    let _ = writeln!(
        stdout,
        "valid store: {records} record(s), {runs} run(s), {} object(s)",
        walked.tree.len()
    );
    let _ = writeln!(
        stdout,
        "{publishable} of {} record(s) may enter a published view",
        view.len()
    );
    for (path, verdict) in &view {
        if let Err(violations) = verdict {
            let _ = writeln!(stdout, "  provisional {path}");
            for error in violations.errors() {
                let _ = writeln!(stdout, "    {error}");
            }
        }
    }
    ExitCode::SUCCESS
}
