//! Reduces a release listing to the fields this project's readers actually read.
//!
//! ⛔ **A recorded vendor listing is too large to keep and the whole of it is
//! not the measurement.** aria2's is 420 kilobytes, transmission's is a
//! megabyte, and almost all of both is release-note prose belonging to somebody
//! else. What
//! [`sources::github_releases`](../src/resolution.rs) and
//! `sources::github_release_assets` read is a handful of fields, so that is what
//! a recorded response is kept as.
//!
//! ⭐ **The projection is by construction everything the readers read**, because
//! it is produced by deserializing into their own shapes and re-emitting. A
//! hand-transcribed listing would risk exactly the transcription error that the
//! asset names under test are.
//!
//! ⚠ It is a reduction and says so: the fixture it writes carries the source URL
//! and the digest of the FULL response it came from, so anyone can re-fetch, re-
//! project and compare rather than take this file's word for it.
//!
//! ```text
//! project-listing <listing-file> <source-url> [tag]
//! ```
//!
//! Writes the projection to stdout. With a tag, only that release is kept; with
//! none, every release is. Exit codes: 0 written, 1 the tag is not in the
//! listing, 2 could not run.

use std::process::ExitCode;

use bit_ids::canonical::Sha256Digest;
use serde::{Deserialize, Serialize};

/// Exactly the release fields the two readers in `sources` deserialize.
#[derive(Deserialize, Serialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    draft: bool,
    published_at: Option<String>,
    #[serde(default)]
    assets: Vec<Asset>,
}

/// Exactly the asset fields `github_release_assets` deserializes.
#[derive(Deserialize, Serialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (path, source, tag) = match args.as_slice() {
        [path, source] => (path, source, None),
        [path, source, tag] => (path, source, Some(tag)),
        _ => {
            eprintln!("usage: project-listing <listing-file> <source-url> [tag]");
            return ExitCode::from(2);
        }
    };

    let body = match std::fs::read(path) {
        Ok(body) => body,
        Err(error) => {
            eprintln!("project-listing: {path}: {error}");
            return ExitCode::from(2);
        }
    };
    let digest = Sha256Digest::of(&body);
    let releases: Vec<Release> = match serde_json::from_slice(&body) {
        Ok(releases) => releases,
        Err(error) => {
            eprintln!("project-listing: {path} is not a release list: {error}");
            return ExitCode::from(2);
        }
    };
    let kept: Vec<Release> = match tag {
        Some(tag) => releases
            .into_iter()
            .filter(|release| &release.tag_name == tag)
            .collect(),
        None => releases,
    };
    if kept.is_empty() {
        eprintln!("project-listing: {path} carries no release to keep");
        return ExitCode::FAILURE;
    }

    match serde_json::to_string_pretty(&kept) {
        Ok(text) => {
            println!("{text}");
            eprintln!(
                "project-listing: {} release(s) from {source}, whose full response is {digest}",
                kept.len()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("project-listing: the projection could not be written: {error}");
            ExitCode::from(2)
        }
    }
}
