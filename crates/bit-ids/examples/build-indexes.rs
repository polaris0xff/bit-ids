//! Build the deterministic indexes and the latest view over a store.
//!
//! ```text
//! cargo run -p bit-ids --example build-indexes -- STORE [OUT]
//! ```
//!
//! This is the driving surface for `CORPUS-03`. With `OUT` it writes the index
//! document there; without one it prints the digest and the row counts, which is
//! what a determinism check compares between two builds.
//!
//! ⛔ **It builds the indexes and then checks every row against the store they
//! came from.** A derived file is read instead of the records, so a row naming a
//! record nobody can open is an answer with nothing behind it. Checking the
//! builder against its own output would agree with itself.
//!
//! ⚠ **A version scheme is given, never defaulted.** A target declares how it
//! spells versions, and this project has no general rule that reads every
//! target's spelling; `resolve-stable` takes one on the command line for the
//! same reason. `--scheme TARGET:PREFIX:MIN:MAX` declares one, with `-` for no
//! prefix. A record whose target has no scheme blocks the latest view and this
//! exits 1 naming it, rather than being ordered under an assumed three-component
//! shape.
//!
//! Exit codes follow `docs/capture-methodology.md`: 0 the indexes were built,
//! 1 the store or a row was refused, 2 the route could not run.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::Path;
use std::process::ExitCode;

use bit_ids::canonical::{Label, Slug};
use bit_ids::corpus::Corpus;
use bit_ids::index::{build, rows_resolve};
use bit_ids::resolution::VersionScheme;
use bit_ids::store::{is_manifest_path, is_profile_path};
use bit_ids::{Profile, RunManifest};

#[path = "support/walk.rs"]
mod support;

use support::walk;

/// Parses one `--scheme TARGET:PREFIX:MIN:MAX` argument.
///
/// `-` in the prefix position means the target publishes versions with no tag
/// prefix. ⛔ Nothing here has a default: a scheme this cannot parse is refused
/// rather than filled in, because a filled-in scheme orders versions under a
/// shape nobody declared.
fn scheme(text: &str) -> Result<(Slug, VersionScheme), String> {
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

/// What the command line asked for.
struct Request {
    schemes: BTreeMap<Slug, VersionScheme>,
    store: std::ffi::OsString,
    out: Option<std::ffi::OsString>,
}

fn parse_args() -> Result<Request, ExitCode> {
    let mut schemes: BTreeMap<Slug, VersionScheme> = BTreeMap::new();
    let mut positional: Vec<std::ffi::OsString> = Vec::new();
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        if arg != *"--scheme" {
            positional.push(arg);
            continue;
        }
        let Some(text) = args.next().and_then(|value| value.into_string().ok()) else {
            let _ = writeln!(std::io::stderr(), "--scheme needs TARGET:PREFIX:MIN:MAX");
            return Err(ExitCode::from(2));
        };
        match scheme(&text) {
            Ok((target, parsed)) => {
                schemes.insert(target, parsed);
            }
            Err(error) => {
                let _ = writeln!(std::io::stderr(), "{error}");
                return Err(ExitCode::from(2));
            }
        }
    }

    let (store, out) = match positional.as_slice() {
        [store] => (store.clone(), None),
        [store, out] => (store.clone(), Some(out.clone())),
        _ => {
            let _ = writeln!(
                std::io::stderr(),
                "usage: build-indexes [--scheme TARGET:PREFIX:MIN:MAX]... STORE [OUT]\n\
                 builds the lookup indexes and the latest view over a store"
            );
            return Err(ExitCode::from(2));
        }
    };
    Ok(Request {
        schemes,
        store,
        out,
    })
}

/// Reads every record and run the store carries into a corpus, reporting what
/// it could not read rather than dropping it.
fn read_store(root: &Path, refusals: &mut Vec<String>) -> Result<Corpus, String> {
    let walked = walk(root)?;
    refusals.extend(walked.refused.iter().cloned());
    let mut corpus = Corpus::new(walked.tree.clone());
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

fn report(refusals: &[String]) -> ExitCode {
    let mut stderr = std::io::stderr();
    let _ = writeln!(stderr, "refused: {} finding(s)", refusals.len());
    for refusal in refusals {
        let _ = writeln!(stderr, "  {refusal}");
    }
    ExitCode::from(1)
}

fn main() -> ExitCode {
    let request = match parse_args() {
        Ok(request) => request,
        Err(code) => return code,
    };
    let root = Path::new(&request.store);
    if !root.is_dir() {
        let _ = writeln!(std::io::stderr(), "{}: not a directory", root.display());
        return ExitCode::from(2);
    }

    let mut refusals: Vec<String> = Vec::new();
    let corpus = match read_store(root, &mut refusals) {
        Ok(corpus) => corpus,
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "cannot read the store: {error}");
            return ExitCode::from(2);
        }
    };

    let indexes = match build(&corpus, &request.schemes) {
        Ok(indexes) => indexes,
        Err(violations) => {
            for error in violations.errors() {
                refusals.push(error.to_string());
            }
            return report(&refusals);
        }
    };

    if let Err(violations) = rows_resolve(&indexes, &corpus) {
        for error in violations.errors() {
            refusals.push(error.to_string());
        }
    }
    if !refusals.is_empty() {
        return report(&refusals);
    }

    let document = indexes.to_json();
    if let Some(out) = request.out
        && let Err(error) = std::fs::write(Path::new(&out), document.as_bytes())
    {
        let _ = writeln!(std::io::stderr(), "cannot write the index: {error}");
        return ExitCode::from(2);
    }

    let mut stdout = std::io::stdout();
    let _ = writeln!(
        stdout,
        "{} {} row(s), {} latest, {} excluded",
        indexes.digest(),
        indexes.rows() - indexes.latest.len(),
        indexes.latest.len(),
        indexes.excluded
    );
    ExitCode::SUCCESS
}
