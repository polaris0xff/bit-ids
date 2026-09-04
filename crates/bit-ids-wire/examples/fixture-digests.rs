//! Prints the fixture corpus index derived from what is on disk.
//!
//! It does not write. Redirect it over `tests/fixtures/index.json` to
//! regenerate the committed index after adding or changing a fixture, and read
//! the diff: a digest that moved for a fixture you did not touch is a finding.
//!
//! Running it twice and comparing the output is the direct form of `FOUND-03`'s
//! acceptance, "identical fixture digests". The suite asserts the same thing
//! against the committed file, which is the form that survives a session.

use std::path::PathBuf;
use std::process::ExitCode;

use bit_ids_wire::FixtureIndex;
use bit_ids_wire::fixture::load_directory;

fn main() -> ExitCode {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    let loaded = match load_directory(&directory) {
        Ok(loaded) => loaded,
        Err(error) => {
            eprintln!("{}: {error}", directory.display());
            return ExitCode::FAILURE;
        }
    };
    if loaded.is_empty() {
        eprintln!("{}: no fixtures", directory.display());
        return ExitCode::FAILURE;
    }
    let fixtures: Vec<_> = loaded.into_iter().map(|(_, fixture)| fixture).collect();
    match FixtureIndex::of(&fixtures).and_then(|index| index.to_json()) {
        Ok(document) => {
            print!("{document}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
