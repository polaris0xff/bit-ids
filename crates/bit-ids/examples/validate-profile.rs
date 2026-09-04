//! Read a profile record from a file and report whether it is publishable.
//!
//! This is the driving surface for `SCHEMA-01`: the path a consumer of the
//! published corpus actually takes, exercised end to end rather than through
//! the library's own test harness. `CORPUS-02` owns the validator that runs
//! over a whole store; this one answers for a single record.
//!
//! ```text
//! cargo run --example validate-profile -- PATH
//! ```
//!
//! Exit codes follow `docs/capture-methodology.md`: 0 the record validates,
//! 1 the record was read and refused, 2 the route could not run.

use std::io::Write as _;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    let (Some(path), None) = (args.next(), args.next()) else {
        let _ = writeln!(
            std::io::stderr(),
            "usage: validate-profile PATH\n\
             reads one bit-ids profile record and validates it"
        );
        return ExitCode::from(2);
    };

    let document = match std::fs::read_to_string(&path) {
        Ok(document) => document,
        Err(error) => {
            let _ = writeln!(
                std::io::stderr(),
                "cannot read {}: {error}",
                path.to_string_lossy()
            );
            return ExitCode::from(2);
        }
    };

    match bit_ids::Profile::from_json(&document) {
        Ok(profile) => {
            let measured = profile
                .observations
                .iter()
                .filter(|field| field.state.asserts_a_measurement())
                .count();
            let mut stdout = std::io::stdout();
            let _ = writeln!(
                stdout,
                "valid {} {} {} {} {}\n{} field(s), {} of them measured, {} evidence artifact(s)",
                profile.schema,
                profile.target.id,
                profile.build.version,
                profile.build.platform,
                profile.id,
                profile.observations.len(),
                measured,
                profile.evidence.len(),
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "refused {}", path.to_string_lossy());
            let _ = writeln!(std::io::stderr(), "{error}");
            ExitCode::from(1)
        }
    }
}
