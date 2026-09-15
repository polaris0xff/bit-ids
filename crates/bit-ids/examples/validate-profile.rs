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
            // ⚠ Validity and publishability are different questions. A record
            // carrying a disagreement is a valid record, and has to be, or the
            // project has no way to keep the evidence of one. It is refused
            // here rather than never written.
            if let Err(blockers) = bit_ids::publishable(&profile) {
                let _ = writeln!(
                    std::io::stderr(),
                    "provisional, not publishable\n{blockers}"
                );
                // ⛔ THIS READS ONE DOCUMENT, AND `E-PUB-04` IS NOT A PROPERTY
                // OF ONE. Whether two byte-different installs behave alike is a
                // property of a PAIR, and the second capture is a different
                // record; `publishable_among` is the same gate asked where the
                // store is visible. Saying so is the difference between a
                // reader taking this for a permanent refusal and taking it for
                // the answer one file can give.
                if blockers.has("E-PUB-04") {
                    let _ = writeln!(
                        std::io::stderr(),
                        "⚠ E-PUB-04 is the answer ONE record can give. A capture \
                         of the other route, in the same store, is what settles \
                         it: validate-corpus asks that question."
                    );
                }
                return ExitCode::from(1);
            }
            let _ = writeln!(stdout, "publishable");
            ExitCode::SUCCESS
        }
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "refused {}", path.to_string_lossy());
            let _ = writeln!(std::io::stderr(), "{error}");
            ExitCode::from(1)
        }
    }
}
