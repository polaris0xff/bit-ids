//! Drive the artifact cache through a source that moved.
//!
//! ```text
//! cargo run -p bit-ids --example cache-scenario -- [--permitted TARGET]...
//! ```
//!
//! This is the driving surface for `ACQ-05`. It observes one artifact from one
//! location, observes the same bytes from a second location, and reports what
//! the cache did about it. ⛔ **Nothing here fetches anything.** The scenario is
//! about identity and permission, and a real download would make the run depend
//! on a vendor's uptime to answer a question about this project's rules.
//!
//! ⭐ **`--permitted` comes from `check-licences --permitted` and never from a
//! second reader of the register.** `catalogue/licences.toml` has one parser per
//! twin and this example is not a third; the caller asks that check what the
//! register permits and passes the answer in.
//!
//! Exit codes follow `docs/capture-methodology.md`: 0 the cache behaved as the
//! model says, 1 it did not, 2 the route could not run.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::process::ExitCode;

use bit_ids::acquisition::{RouteKind, SignatureStatus};
use bit_ids::cache::{Cache, CachedArtifact, Disposition, Retrieval, validate_cache};
use bit_ids::canonical::{Instant, Sha256Digest, Slug, Url, Version};

/// The artifact the scenario is about. Its bytes are generated here and are not
/// anybody's installer: the point is what the cache does with a digest.
fn artifact(target: &Slug) -> CachedArtifact {
    let bytes = b"bit-ids cache scenario artifact\n";
    CachedArtifact {
        target: target.clone(),
        version: Version::parse("1.37.0").expect("a reported version"),
        sha256: Sha256Digest::of(bytes),
        bytes: bytes.len() as u64,
        signature: SignatureStatus::Unsigned,
        stored: false,
        retrievals: Vec::new(),
    }
}

fn retrieval(origin: &str, at: &str) -> Retrieval {
    Retrieval {
        at: Instant::parse(at).expect("a canonical instant"),
        route: RouteKind::GithubRelease,
        origin: Url::parse(origin).expect("a retrieval location"),
    }
}

/// The disposition per target: refused for the scenario's own target, and
/// permitted for whatever the caller was told the register permits.
///
/// ⚠ The default is refused rather than absent, so the scenario exercises the
/// register's answer rather than `E-CAC-02`'s missing-row refusal.
fn register() -> Result<BTreeMap<Slug, Disposition>, ExitCode> {
    let target = Slug::parse("aria2").expect("a canonical slug");
    let mut register = BTreeMap::from([(target, Disposition::Refused)]);
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        if arg != *"--permitted" {
            let _ = writeln!(
                std::io::stderr(),
                "usage: cache-scenario [--permitted TARGET]...\n\
                 drives the artifact cache through a source that moved"
            );
            return Err(ExitCode::from(2));
        }
        let Some(text) = args.next().and_then(|value| value.into_string().ok()) else {
            let _ = writeln!(std::io::stderr(), "--permitted needs a target id");
            return Err(ExitCode::from(2));
        };
        match Slug::parse(&text) {
            Ok(slug) => {
                register.insert(slug, Disposition::Permitted);
            }
            Err(error) => {
                let _ = writeln!(std::io::stderr(), "--permitted {text}: {error}");
                return Err(ExitCode::from(2));
            }
        }
    }
    Ok(register)
}

fn main() -> ExitCode {
    let register = match register() {
        Ok(register) => register,
        Err(code) => return code,
    };
    let target = Slug::parse("aria2").expect("a canonical slug");
    let allows_storage = matches!(register.get(&target), Some(Disposition::Permitted));

    let mut stdout = std::io::stdout();
    let mut cache = Cache::default();
    let old = retrieval(
        "https://example.invalid/downloads/v1/aria2.tar.gz",
        "2026-01-04T09:00:00Z",
    );
    let new = retrieval(
        "https://example.invalid/archive/2026/aria2.tar.gz",
        "2026-06-18T09:00:00Z",
    );

    let first = match cache.observe(artifact(&target), old.clone()) {
        Ok(outcome) => outcome,
        Err(violations) => {
            let _ = writeln!(
                std::io::stderr(),
                "the first retrieval was refused: {violations}"
            );
            return ExitCode::from(1);
        }
    };
    // ⛔ THE SOURCE MOVED AND THE ARTIFACT DID NOT. Same bytes, new location.
    let second = match cache.observe(artifact(&target), new.clone()) {
        Ok(outcome) => outcome,
        Err(violations) => {
            let _ = writeln!(
                std::io::stderr(),
                "the second retrieval was refused: {violations}"
            );
            return ExitCode::from(1);
        }
    };

    let expected = artifact(&target).sha256;
    let Some(known) = cache.resolve(&expected) else {
        let _ = writeln!(std::io::stderr(), "the digest no longer names the artifact");
        return ExitCode::from(1);
    };

    let _ = writeln!(stdout, "first retrieval:  {first:?}");
    let _ = writeln!(stdout, "second retrieval: {second:?}");
    let _ = writeln!(stdout, "artifacts: {}", cache.artifacts.len());
    let _ = writeln!(stdout, "retrievals: {}", known.retrievals.len());
    for entry in &known.retrievals {
        let _ = writeln!(stdout, "  {} {}", entry.at, entry.origin);
    }

    if cache.artifacts.len() != 1 || known.retrievals.len() != 2 {
        let _ = writeln!(
            std::io::stderr(),
            "a moved source produced {} artifact(s) and {} retrieval(s)",
            cache.artifacts.len(),
            known.retrievals.len()
        );
        return ExitCode::from(1);
    }

    if let Err(violations) = validate_cache(&cache, &register) {
        let _ = writeln!(
            std::io::stderr(),
            "the cache that keeps nothing was refused: {violations}"
        );
        return ExitCode::from(1);
    }
    let _ = writeln!(stdout, "keeping nothing: accepted");

    // ⛔ NOW ASK FOR THE BYTES. This is the half the licence register decides,
    // and the expected answer today is a refusal for every target.
    cache.artifacts[0].stored = true;
    let verdict = validate_cache(&cache, &register);
    match (allows_storage, verdict) {
        (false, Err(violations)) if violations.has("E-CAC-01") => {
            let _ = writeln!(stdout, "keeping the bytes: refused as E-CAC-01");
        }
        (true, Ok(())) => {
            let _ = writeln!(stdout, "keeping the bytes: permitted by the register");
        }
        (false, Ok(())) => {
            let _ = writeln!(
                std::io::stderr(),
                "the register refuses this target and the cache kept the bytes anyway"
            );
            return ExitCode::from(1);
        }
        (_, Err(violations)) => {
            let _ = writeln!(std::io::stderr(), "unexpected refusal: {violations}");
            return ExitCode::from(1);
        }
    }

    ExitCode::SUCCESS
}
