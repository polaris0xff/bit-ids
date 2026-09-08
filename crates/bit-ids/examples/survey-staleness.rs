//! Compare what the resolvers selected against what the store publishes, and
//! print the capture work outstanding.
//!
//! ```text
//! cargo run -p bit-ids --example survey-staleness -- \
//!     STORE (--resolution FILE --platforms LIST)... [--open FILE] [OUT]
//! ```
//!
//! This is the driving surface for `CI-02`. `--platforms` applies to the
//! `--resolution` before it, and is the comma-separated list of host families a
//! capture is wanted on, which the catalogue declares per target.
//!
//! ⛔ **It re-derives the views rather than parsing the published index.** The
//! index document is a function of the store and
//! [`bit_ids::index::build`](../src/index.rs) is the one derivation, so reading
//! the store here means the monitor compares against exactly the view that gets
//! published. A second parser for that document would be a second reading of it,
//! and the two would disagree in the direction that opens a request for a
//! capture already taken. ⚠ The `data` branch **is** the store: it carries
//! `profiles/v1/` and `raw/v1/`, so pointing this at a checkout of that branch
//! is the real path rather than a stand-in for it.
//!
//! ⚠ **The version schemes come out of the resolutions.** A resolution carries
//! the target it resolved and the scheme it ordered under, so nothing is
//! declared twice; `build-indexes` takes `--scheme` because it has no
//! resolutions to read them from.
//!
//! ⛔ **`--open` is what makes a repeated run safe**, and its absence means an
//! empty tracker rather than an unknown one. A run given no open set reports
//! every request as newly opened, which is correct for a first run and is why
//! the flag is named in the summary line.
//!
//! Exit codes: 0 the survey ran and opened nothing new, 1 it opened at least one
//! request, 2 it could not run. ⚠ A blocked line is **not** in the exit code:
//! `unresolved`, `regressed`, `ambiguous` and `unorderable` are reported in the
//! summary and carried in the document, because the code answers the entry's own
//! question, which is whether a capture was asked for. The alternative, a third
//! code for a blocked line, was rejected: it would make a caller that only wants
//! to know about new work read a code it has to special-case.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::Path;
use std::process::ExitCode;

use bit_ids::canonical::{Instant, Slug};
use bit_ids::index::build;
use bit_ids::resolution::{Resolution, VersionScheme};
use bit_ids::staleness::{OpenRequest, Staleness, Watch, survey};

#[path = "support/reader.rs"]
mod support;

use support::read_store;

/// What the command line asked for.
struct Request {
    store: std::ffi::OsString,
    watches: Vec<(std::ffi::OsString, Vec<Slug>)>,
    open: Option<std::ffi::OsString>,
    out: Option<std::ffi::OsString>,
}

fn usage() -> ExitCode {
    let _ = writeln!(
        std::io::stderr(),
        "usage: survey-staleness STORE (--resolution FILE --platforms LIST)... \
         [--open FILE] [OUT]\n\
         compares each resolution's selection against what the store publishes"
    );
    ExitCode::from(2)
}

fn parse_args() -> Result<Request, ExitCode> {
    let mut watches: Vec<(std::ffi::OsString, Vec<Slug>)> = Vec::new();
    let mut positional: Vec<std::ffi::OsString> = Vec::new();
    let mut open: Option<std::ffi::OsString> = None;
    let mut args = std::env::args_os().skip(1);

    while let Some(arg) = args.next() {
        if arg == *"--resolution" {
            let Some(path) = args.next() else {
                return Err(usage());
            };
            watches.push((path, Vec::new()));
            continue;
        }
        if arg == *"--platforms" {
            let Some(text) = args.next().and_then(|value| value.into_string().ok()) else {
                return Err(usage());
            };
            // ⛔ Attached to the resolution before it rather than to the run.
            // A flag that applied to every target would give one target's
            // platform list to another, and a request for a platform a target
            // does not ship on is work no route can satisfy.
            let Some(last) = watches.last_mut() else {
                let _ = writeln!(
                    std::io::stderr(),
                    "--platforms applies to the --resolution before it"
                );
                return Err(ExitCode::from(2));
            };
            for part in text.split(',').filter(|part| !part.is_empty()) {
                match Slug::parse(part) {
                    Ok(platform) => last.1.push(platform),
                    Err(error) => {
                        let _ = writeln!(std::io::stderr(), "--platforms {part:?}: {error}");
                        return Err(ExitCode::from(2));
                    }
                }
            }
            continue;
        }
        if arg == *"--open" {
            let Some(path) = args.next() else {
                return Err(usage());
            };
            open = Some(path);
            continue;
        }
        positional.push(arg);
    }

    if watches.is_empty() || watches.iter().any(|(_, platforms)| platforms.is_empty()) {
        let _ = writeln!(
            std::io::stderr(),
            "every --resolution needs a --platforms after it"
        );
        return Err(ExitCode::from(2));
    }
    let (store, out) = match positional.as_slice() {
        [store] => (store.clone(), None),
        [store, out] => (store.clone(), Some(out.clone())),
        _ => return Err(usage()),
    };
    Ok(Request {
        store,
        watches,
        open,
        out,
    })
}

/// Reads the tracker's open set.
///
/// The file is a JSON array of `{target, version, channel, platform}`. ⚠ A file
/// that is present and unreadable is exit 2 rather than an empty set: a tracker
/// whose state could not be read is not a tracker holding nothing, and treating
/// it as one opens every request again.
fn read_open(path: &Path) -> Result<Vec<OpenRequest>, String> {
    let document = std::fs::read_to_string(path).map_err(|error| format!("{error}"))?;
    serde_json::from_str(&document).map_err(|error| format!("{error}"))
}

fn read_resolution(path: &Path) -> Result<Resolution, String> {
    let document = std::fs::read_to_string(path).map_err(|error| format!("{error}"))?;
    Resolution::from_json(&document).map_err(|error| format!("{error}"))
}

fn now_utc() -> Result<Instant, ()> {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| ())?
        .as_secs();
    Instant::parse(&format_epoch(seconds)).map_err(|_| ())
}

/// Formats seconds since the epoch as the one instant spelling this schema
/// accepts. The same function `resolve-stable` carries, and for the same reason:
/// a date crate for one call is a dependency this project would have to argue
/// for.
fn format_epoch(seconds: u64) -> String {
    let days = seconds / 86_400;
    let rest = seconds % 86_400;
    let (hour, minute, second) = (rest / 3600, (rest % 3600) / 60, rest % 60);
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Days since 1970-01-01 to a civil date, by Howard Hinnant's `civil_from_days`.
fn civil_from_days(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z % 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Reads every resolution named on the command line, with its platform list.
fn load_watches(
    request: &Request,
    stderr: &mut std::io::Stderr,
) -> Result<Vec<(Resolution, Vec<Slug>)>, ExitCode> {
    let mut out = Vec::new();
    for (path, platforms) in &request.watches {
        match read_resolution(Path::new(path)) {
            Ok(resolution) => out.push((resolution, platforms.clone())),
            Err(error) => {
                let _ = writeln!(stderr, "{}: {error}", Path::new(path).display());
                return Err(ExitCode::from(2));
            }
        }
    }
    Ok(out)
}

/// Reads the store and derives the views the monitor compares against.
///
/// ⚠ The schemes are the resolutions' own, so a store record whose target no
/// resolution covers blocks the view under `E-VIW-01`. That is the right answer
/// rather than an inconvenience: a monitor that indexed a target it was not
/// resolving would compare against a view built under a scheme nobody declared
/// for it.
fn load_latest(
    root: &Path,
    resolutions: &[(Resolution, Vec<Slug>)],
    stderr: &mut std::io::Stderr,
) -> Result<Vec<bit_ids::index::LatestRow>, ExitCode> {
    if !root.is_dir() {
        let _ = writeln!(stderr, "{}: not a directory", root.display());
        return Err(ExitCode::from(2));
    }
    let mut refusals: Vec<String> = Vec::new();
    let corpus = match read_store(root, &mut refusals) {
        Ok(corpus) => corpus,
        Err(error) => {
            let _ = writeln!(stderr, "cannot read the store: {error}");
            return Err(ExitCode::from(2));
        }
    };
    let schemes: BTreeMap<Slug, VersionScheme> = resolutions
        .iter()
        .map(|(resolution, _)| (resolution.target.clone(), resolution.scheme.clone()))
        .collect();
    let indexes = match build(&corpus, &schemes) {
        Ok(indexes) => indexes,
        Err(violations) => {
            let _ = writeln!(
                stderr,
                "the store does not index: {} finding(s)",
                violations.len()
            );
            for error in violations.errors() {
                let _ = writeln!(stderr, "  {error}");
            }
            return Err(ExitCode::from(1));
        }
    };
    if !refusals.is_empty() {
        let _ = writeln!(
            stderr,
            "the store has {} unreadable path(s)",
            refusals.len()
        );
        for refusal in &refusals {
            let _ = writeln!(stderr, "  {refusal}");
        }
        return Err(ExitCode::from(1));
    }
    Ok(indexes.latest)
}

fn main() -> ExitCode {
    let request = match parse_args() {
        Ok(request) => request,
        Err(code) => return code,
    };
    let mut stderr = std::io::stderr();

    let resolutions = match load_watches(&request, &mut stderr) {
        Ok(resolutions) => resolutions,
        Err(code) => return code,
    };
    let open = match &request.open {
        None => Vec::new(),
        Some(path) => match read_open(Path::new(path)) {
            Ok(open) => open,
            Err(error) => {
                let _ = writeln!(stderr, "{}: {error}", Path::new(path).display());
                return ExitCode::from(2);
            }
        },
    };
    let latest = match load_latest(Path::new(&request.store), &resolutions, &mut stderr) {
        Ok(latest) => latest,
        Err(code) => return code,
    };

    let Ok(now) = now_utc() else {
        let _ = writeln!(stderr, "the host clock is not readable as UTC seconds");
        return ExitCode::from(2);
    };
    let watches: Vec<Watch<'_>> = resolutions
        .iter()
        .map(|(resolution, platforms)| Watch {
            resolution,
            platforms,
        })
        .collect();
    let set = survey(now, &watches, &latest, &open);

    let document = match set.to_json() {
        Ok(document) => document,
        Err(error) => {
            let _ = writeln!(stderr, "the survey is not a valid document: {error}");
            return ExitCode::from(2);
        }
    };
    if let Some(out) = &request.out {
        if let Err(error) = std::fs::write(Path::new(out), document.as_bytes()) {
            let _ = writeln!(stderr, "cannot write the survey: {error}");
            return ExitCode::from(2);
        }
    } else {
        let mut stdout = std::io::stdout();
        let _ = write!(stdout, "{document}");
    }

    // ⛔ Every verdict that declined to answer is named, not summed into an
    // "other" count. A monitor whose blocked lines are invisible is one whose
    // targets stop being monitored quietly.
    let blocked: Vec<String> = [
        Staleness::Unresolved,
        Staleness::Regressed,
        Staleness::Ambiguous,
        Staleness::Unorderable,
    ]
    .iter()
    .filter_map(|staleness| {
        let count = set.with_staleness(*staleness).len();
        (count > 0).then(|| format!("{count} {staleness}"))
    })
    .collect();

    let opened = set.opened().len();
    let _ = writeln!(
        stderr,
        "{} line(s), {opened} opened, {} already open, {} retired, tracker {}{}",
        set.assessments.len(),
        set.requests.len() - opened,
        set.superseded.len(),
        if request.open.is_some() {
            "read"
        } else {
            "not given"
        },
        if blocked.is_empty() {
            String::new()
        } else {
            format!("; blocked: {}", blocked.join(", "))
        }
    );
    if opened > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
