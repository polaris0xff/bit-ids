//! Read a run manifest and the profile it produced, and report whether they
//! describe the same run.
//!
//! This is the driving surface for `SCHEMA-02`. Validating either document
//! alone is what `validate-profile` does; what only shows up here is the pair
//! disagreeing, which is the failure the two-document design exists to make
//! visible.
//!
//! ```text
//! cargo run --example validate-run -- MANIFEST_PATH PROFILE_PATH
//! ```
//!
//! Exit codes follow `docs/capture-methodology.md`: 0 the pair agrees, 1 a
//! document was refused or the two disagree, 2 the route could not run.

use std::io::Write as _;
use std::process::ExitCode;

fn read(path: &std::ffi::OsStr) -> Result<String, ExitCode> {
    std::fs::read_to_string(path).map_err(|error| {
        let _ = writeln!(
            std::io::stderr(),
            "cannot read {}: {error}",
            path.to_string_lossy()
        );
        ExitCode::from(2)
    })
}

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    let (Some(manifest_path), Some(profile_path), None) = (args.next(), args.next(), args.next())
    else {
        let _ = writeln!(
            std::io::stderr(),
            "usage: validate-run MANIFEST_PATH PROFILE_PATH\n\
             reads one run manifest and one profile and checks they describe one run"
        );
        return ExitCode::from(2);
    };

    let (manifest_text, profile_text) = match (read(&manifest_path), read(&profile_path)) {
        (Ok(manifest), Ok(profile)) => (manifest, profile),
        (Err(code), _) | (_, Err(code)) => return code,
    };

    let manifest = match bit_ids::RunManifest::from_json(&manifest_text) {
        Ok(manifest) => manifest,
        Err(error) => {
            let _ = writeln!(
                std::io::stderr(),
                "refused manifest {}\n{error}",
                manifest_path.to_string_lossy()
            );
            return ExitCode::from(1);
        }
    };
    let profile = match bit_ids::Profile::from_json(&profile_text) {
        Ok(profile) => profile,
        Err(error) => {
            let _ = writeln!(
                std::io::stderr(),
                "refused profile {}\n{error}",
                profile_path.to_string_lossy()
            );
            return ExitCode::from(1);
        }
    };

    if let Err(violations) = bit_ids::bind(&manifest, &profile) {
        let _ = writeln!(
            std::io::stderr(),
            "the two documents do not describe the same run\n{violations}"
        );
        return ExitCode::from(1);
    }

    let redacted = manifest
        .evidence
        .iter()
        .filter(|record| record.redacted)
        .count();
    let mut stdout = std::io::stdout();
    let _ = writeln!(
        stdout,
        "bound {} {} {} {}\n{} phase(s), {} tool(s), {} route(s), {} artifact(s), {} redacted",
        manifest.capture,
        manifest.target,
        manifest.version,
        profile.id,
        manifest.phases.len(),
        manifest.tools.len(),
        manifest.acquisition.len(),
        manifest.evidence.len(),
        redacted,
    );
    ExitCode::SUCCESS
}
