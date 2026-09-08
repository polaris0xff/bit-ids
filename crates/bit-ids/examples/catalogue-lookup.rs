//! Open a published catalogue and answer one question against it.
//!
//! ```text
//! cargo run -p bit-ids --example catalogue-lookup -- TREE latest TARGET PLATFORM ARCH PACKAGE
//! cargo run -p bit-ids --example catalogue-lookup -- TREE lookup KIND KEY
//! cargo run -p bit-ids --example catalogue-lookup -- TREE plan
//! ```
//!
//! This is the driving surface for `LIB-01`, and it is what a consuming tool
//! looks like: it reads bytes, hands them to the library, and asks a typed
//! question. Every rule about what those bytes have to be is the library's.
//!
//! ⛔ **It opens no socket, and neither does the library.** A `TREE` is a
//! directory a caller already has, which on the real path is a checkout of the
//! `data` branch. `plan` prints what a caller would need in order to fetch each
//! path itself: the stability class, which document proves the bytes, and the
//! digest to check what comes back against.
//!
//! ⚠ **`--expect-checksums` is how a caller supplies the one digest a
//! publication cannot prove.** Without it the checksum file is trusted, and the
//! summary line says which of the two happened rather than leaving a reader to
//! assume the stronger one.
//!
//! Exit codes follow `docs/capture-methodology.md`: 0 the question was
//! answered, 1 the catalogue or the question was refused, 2 it could not run.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use bit_ids::canonical::{Sha256Digest, Slug};
use bit_ids::catalogue::{Bundle, Catalogue, plan};
use bit_ids::index::IndexKind;

/// Reads a directory into a bundle, without following a link out of it.
///
/// ⚠ The walk is here rather than in the library, for the reason
/// `docs/architecture.md` section 3 gives: the crate's rules are pure over
/// bytes a caller has already read.
fn read_tree(root: &Path, prefix: &str, bundle: &mut Bundle) -> Result<(), String> {
    let mut names: Vec<(std::ffi::OsString, PathBuf)> = std::fs::read_dir(root)
        .map_err(|error| format!("{}: {error}", root.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("{}: {error}", root.display()))?
        .into_iter()
        .map(|entry| (entry.file_name(), entry.path()))
        .collect();
    names.sort();

    for (name, path) in names {
        let Some(name) = name.to_str() else {
            return Err(format!("{}: a name that is not UTF-8", path.display()));
        };
        let at = if prefix.is_empty() {
            name.to_owned()
        } else {
            format!("{prefix}/{name}")
        };
        let meta = std::fs::symlink_metadata(&path).map_err(|error| format!("{at}: {error}"))?;
        if meta.is_symlink() {
            return Err(format!("{at}: a publication carries bytes, not links"));
        }
        if meta.is_dir() {
            read_tree(&path, &at, bundle)?;
        } else {
            let bytes = std::fs::read(&path).map_err(|error| format!("{at}: {error}"))?;
            bundle.insert(at, bytes);
        }
    }
    Ok(())
}

fn usage() -> ExitCode {
    let _ = writeln!(
        std::io::stderr(),
        "usage: catalogue-lookup TREE [--expect-checksums DIGEST] <question>\n\
         questions:\n\
         \x20 latest TARGET PLATFORM ARCH PACKAGE\n\
         \x20 lookup KIND KEY   (kind: target|peer_prefix|bep10_client|platform|version|captured_at)\n\
         \x20 plan"
    );
    ExitCode::from(2)
}

fn kind_of(text: &str) -> Option<IndexKind> {
    IndexKind::ALL
        .iter()
        .copied()
        .find(|kind| kind.as_str() == text)
}

fn answer(catalogue: &Catalogue, question: &[String]) -> Result<Vec<String>, ExitCode> {
    let mut stderr = std::io::stderr();
    match question {
        [verb, target, platform, arch, package] if verb == "latest" => {
            let parse = |text: &String| Slug::parse(text).map_err(|_| ());
            let (Ok(target), Ok(platform), Ok(arch), Ok(package)) =
                (parse(target), parse(platform), parse(arch), parse(package))
            else {
                let _ = writeln!(stderr, "a build line is four slugs");
                return Err(ExitCode::from(2));
            };
            Ok(catalogue
                .latest(&target, &platform, &arch, &package)
                .into_iter()
                .map(|profile| format!("{} {}", profile.id, profile.build.version))
                .collect())
        }
        [verb, kind, key] if verb == "lookup" => {
            let Some(kind) = kind_of(kind) else {
                let _ = writeln!(stderr, "{kind:?} is not an index kind");
                return Err(ExitCode::from(2));
            };
            Ok(catalogue
                .lookup(kind, key)
                .into_iter()
                .map(|profile| format!("{} {}", profile.id, profile.build.version))
                .collect())
        }
        [verb] if verb == "plan" => {
            let _ = writeln!(stderr, "plan is answered before the catalogue is opened");
            Err(ExitCode::from(2))
        }
        _ => Err(usage()),
    }
}

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut expected: Option<Sha256Digest> = None;
    if let Some(at) = args.iter().position(|arg| arg == "--expect-checksums") {
        let Some(text) = args.get(at + 1).cloned() else {
            return usage();
        };
        match Sha256Digest::parse(&text) {
            Ok(digest) => expected = Some(digest),
            Err(error) => {
                let _ = writeln!(std::io::stderr(), "--expect-checksums: {error}");
                return ExitCode::from(2);
            }
        }
        args.drain(at..=at + 1);
    }
    let Some((root, question)) = args.split_first() else {
        return usage();
    };
    if question.is_empty() {
        return usage();
    }

    let mut stderr = std::io::stderr();
    let mut bundle = Bundle::new();
    if let Err(error) = read_tree(Path::new(root), "", &mut bundle) {
        let _ = writeln!(stderr, "cannot read the publication: {error}");
        return ExitCode::from(2);
    }

    // ⚠ Answered before the catalogue is opened, because a caller planning
    // fetches has the manifest and not yet the records.
    if question == ["plan"] {
        return match plan(&bundle) {
            Ok(paths) => {
                let mut stdout = std::io::stdout();
                for (path, stability, integrity, digest) in &paths {
                    let _ = writeln!(stdout, "{stability} {integrity} {digest} {path}");
                }
                let _ = writeln!(stderr, "{} path(s)", paths.len());
                ExitCode::SUCCESS
            }
            Err(violations) => {
                let _ = writeln!(stderr, "refused: {} finding(s)", violations.len());
                for error in violations.errors() {
                    let _ = writeln!(stderr, "  {error}");
                }
                ExitCode::from(1)
            }
        };
    }

    let catalogue = match Catalogue::open(&bundle, expected.as_ref()) {
        Ok(catalogue) => catalogue,
        Err(violations) => {
            let _ = writeln!(stderr, "refused: {} finding(s)", violations.len());
            for error in violations.errors() {
                let _ = writeln!(stderr, "  {error}");
            }
            return ExitCode::from(1);
        }
    };

    let rows = match answer(&catalogue, question) {
        Ok(rows) => rows,
        Err(code) => return code,
    };
    let mut stdout = std::io::stdout();
    for row in &rows {
        let _ = writeln!(stdout, "{row}");
    }
    // ⛔ Which of the two verification modes ran is on the summary line. A run
    // that trusted the checksum file and one that was handed its digest are
    // different results, and a reader who cannot tell them apart will assume
    // the stronger one.
    let _ = writeln!(
        stderr,
        "{} record(s) published, {} answer(s), checksums {}",
        catalogue.records().len(),
        rows.len(),
        if expected.is_some() {
            "verified against the caller's digest"
        } else {
            "trusted"
        }
    );
    if rows.is_empty() {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
