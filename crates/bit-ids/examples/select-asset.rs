//! Picks the one release artifact a `release` route should fetch.
//!
//! ⛔ **`resolve-stable` orders versions and does not choose an artifact**, and
//! for a while nothing did: every `release` route in this tree took its URL from
//! `BIT_IDS_RELEASE_URL` and nothing produced one. Which asset of a release is
//! the installable one is target knowledge, so the pattern comes from the
//! adapter and the parsing happens here - which is the line
//! [`../../../docs/architecture.md`](../../../docs/architecture.md) section 3
//! draws between shell and Rust.
//!
//! ⭐ **It reads the resolution rather than re-deriving one.** The tag comes
//! from the candidate the resolver actually selected, so a version chosen here
//! and a version chosen there cannot differ; and the assets come out of the same
//! response body the resolution already digested, so one recorded digest covers
//! both decisions and nothing is fetched twice.
//!
//! ```text
//! select-asset <resolution-file> <listing-file> <asset-pattern>
//! ```
//!
//! Prints `key=value` lines on stdout and its reasoning on stderr.
//!
//! Exit codes: 0 one asset was selected, 1 refused, 2 could not run. ⛔ An
//! ambiguous match is exit 1 and not a first-match answer: choosing between two
//! assets by the order the source listed them is choosing by a property of the
//! source.

use std::process::ExitCode;

use bit_ids::resolution::{AssetPattern, Resolution, Verdict, select_asset, sources};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [resolution_path, listing_path, pattern_text] = args.as_slice() else {
        eprintln!("usage: select-asset <resolution-file> <listing-file> <asset-pattern>");
        return ExitCode::from(2);
    };

    let resolution_text = match std::fs::read_to_string(resolution_path) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("select-asset: {resolution_path}: {error}");
            return ExitCode::from(2);
        }
    };
    let resolution = match Resolution::from_json(&resolution_text) {
        Ok(resolution) => resolution,
        Err(error) => {
            eprintln!("select-asset: {resolution_path} is not a resolution: {error}");
            return ExitCode::from(2);
        }
    };

    // ⛔ THE SELECTED CANDIDATE, NOT THE FIRST ONE. `selected` carries the
    // version and the tag is on the candidate that won, so both come from the
    // one decision. A resolution that failed closed selects nothing, and asking
    // for an asset of a release nobody chose is a refusal rather than a guess.
    let Some(version) = resolution.selected.as_ref() else {
        eprintln!(
            "select-asset: the resolution for {} failed closed over {} candidate(s); \
             there is no release to take an asset from",
            resolution.target,
            resolution.considered.len()
        );
        return ExitCode::FAILURE;
    };
    let Some(chosen) = resolution
        .considered
        .iter()
        .find(|considered| considered.verdict == Verdict::Selected)
    else {
        eprintln!(
            "select-asset: the resolution selected {version} and no candidate carries the \
             selected verdict"
        );
        return ExitCode::from(2);
    };
    let tag = &chosen.candidate.tag;

    let body = match std::fs::read(listing_path) {
        Ok(body) => body,
        Err(error) => {
            eprintln!("select-asset: {listing_path}: {error}");
            return ExitCode::from(2);
        }
    };
    let assets = match sources::github_release_assets(&body, tag) {
        Ok(Some(assets)) => assets,
        Ok(None) => {
            eprintln!("select-asset: {listing_path} carries no release tagged {tag}");
            return ExitCode::FAILURE;
        }
        Err(message) => {
            eprintln!("select-asset: {listing_path}: {message}");
            return ExitCode::from(2);
        }
    };

    let pattern = match AssetPattern::parse(pattern_text, version.as_str()) {
        Ok(pattern) => pattern,
        Err(message) => {
            eprintln!("select-asset: {message}");
            return ExitCode::from(2);
        }
    };

    // ⚠ THE WHOLE OFFER IS PRINTED BEFORE THE VERDICT, on stderr, so a refusal
    // says what was actually there. A pattern that stops matching because a
    // vendor renamed an artifact is otherwise a bare "no match" against a list
    // nobody can see.
    eprintln!(
        "select-asset: {} {version} ({tag}) offers {} asset(s), pattern {pattern_text:?}:",
        resolution.target,
        assets.len()
    );
    for asset in &assets {
        let mark = if pattern.matches(asset.name.as_str()) {
            "match  "
        } else {
            "       "
        };
        eprintln!("  {mark} {} ({} bytes)", asset.name, asset.size);
    }

    match select_asset(&assets, &pattern) {
        Ok(asset) => {
            print_field("target", resolution.target.as_str());
            print_field("version", version.as_str());
            print_field("tag", tag.as_str());
            print_field("asset", asset.name.as_str());
            print_field("url", asset.url.as_str());
            println!("size={}", asset.size);
            eprintln!("select-asset: selected {}", asset.name);
            ExitCode::SUCCESS
        }
        Err(refusal) => {
            eprintln!("select-asset: {refusal}");
            ExitCode::FAILURE
        }
    }
}

/// Writes one `key=value` line.
///
/// ⚠ A value carrying a newline would make one field read as two, and every
/// value here is a `Slug`, `Version`, `Label` or `Url`, none of which can hold
/// one. The assertion is against a future value that can.
fn print_field(key: &str, value: &str) {
    debug_assert!(
        !value.contains('\n'),
        "a key=value record cannot carry a newline in {key}"
    );
    println!("{key}={value}");
}
