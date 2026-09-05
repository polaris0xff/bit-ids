//! Write a coherent synthetic store into a directory.
//!
//! ```text
//! cargo run -p bit-ids --example build-store -- [--version V]... [--correct V]... DIR
//! ```
//!
//! ⚠ **`--version` writes the record at a version of your choosing**, with each
//! route's reported version moved with it and the identifier re-derived, because
//! it digests the version. The fixture's own `0.0.0-fixture` is deliberately not
//! a version any scheme can order, so a store meant for `CORPUS-03`'s latest
//! view has to be written at one that is.
//!
//! ⚠ **`--correct V` writes a second record at that version, on its own
//! capture, superseding the first.** It is what `CORPUS-04` needs a store for,
//! and it is written beside the record it corrects rather than over it: the
//! append-only rule keeps both, and the views are what stop naming the earlier
//! one. Pass `--version V` first, or the correction supersedes a record the
//! store does not carry and `E-CRP-07` refuses the result, which is that check
//! doing its job.
//!
//! `CORPUS-02` needs a store whose citations resolve, and the schema fixtures
//! are not one: they declare digests for artifacts that were never written,
//! which is exactly the shape `validate-corpus` refuses. So this writes the
//! artifacts, then rewrites each document's length and digest to the bytes it
//! actually put on the disk, and files everything where its own identity puts
//! it.
//!
//! ⛔ **The documents go out through `to_json`, which validates.** A fixture
//! builder that emitted a record its own reader would refuse is a harness that
//! tests the harness.
//!
//! ⛔ **Nothing here is evidence.** The target is `fixture-client`, which does
//! not exist, and the artifact bytes are generated from each artifact's
//! identifier rather than observed from anything. `docs/architecture.md`
//! section 5 says why a fixture is never evidence.
//!
//! Exit codes: 0 the store was written, 1 a document was refused, 2 the route
//! could not run.

use std::io::Write as _;
use std::path::Path;
use std::process::ExitCode;

use bit_ids::canonical::{Sha256Digest, Slug, Version};
use bit_ids::identity::{RecordId, RecordKey};
use bit_ids::store::StoreKey;
use bit_ids::{Profile, RunManifest};

const PROFILE: &str = include_str!("../tests/fixtures/valid-profile.json");
const MANIFEST: &str = include_str!("../tests/fixtures/valid-manifest.json");
const CORRECTION: &str = include_str!("../tests/fixtures/valid-correction.json");

/// Bytes for one artifact, a function of its identifier so two runs of this
/// example write the same store.
///
/// ⚠ Never empty. A zero-length artifact is refused by the record schema and by
/// the store, and a builder that emitted one would be exercising those refusals
/// rather than the citations this store exists to resolve.
fn payload(id: &str) -> Vec<u8> {
    let mut out = format!("bit-ids synthetic artifact: {id}\n").into_bytes();
    let seed = Sha256Digest::of(id.as_bytes());
    out.extend_from_slice(seed.as_bytes());
    out.push(b'\n');
    out
}

fn write_file(root: &Path, relative: &str, bytes: &[u8]) -> Result<(), String> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    std::fs::write(&path, bytes).map_err(|error| format!("{}: {error}", path.display()))
}

/// Moves a record and its run onto one version.
///
/// ⛔ Every route's reported version moves with it, because `E-ACQ-04` refuses a
/// route that installed a version the record does not declare, and the
/// identifier is re-derived, because it digests the version. A record whose
/// version was edited and whose identifier was not is refused by `E-ID-01`,
/// which is the check doing its job rather than an obstacle.
fn set_version(
    profile: &mut Profile,
    manifest: &mut RunManifest,
    version_text: &str,
) -> Result<(), String> {
    let version = Version::parse(version_text).map_err(|error| error.to_string())?;
    let capture = Slug::parse(&format!("cap-{}", version_text.replace('.', "-")))
        .map_err(|error| error.to_string())?;

    profile.build.version = version.clone();
    for route in &mut profile.acquisition {
        route.installed_version = version.clone();
        route.resolved_version = version.clone();
    }
    profile.capture.id = capture.clone();
    profile.id = RecordId::derive(&RecordKey {
        schema: &profile.schema,
        target: &profile.target.id,
        version: &profile.build.version,
        platform: &profile.build.platform,
        arch: &profile.build.arch,
        package: &profile.build.package,
        capture: &profile.capture.id,
    });

    manifest.version = version.clone();
    manifest.capture = capture;
    // ⛔ The run's own acquisition rows carry the installed version too, and
    // E-MAN-12 compares them against the run's. Moving one document's version
    // and not the other's produces a manifest that refuses itself, which is the
    // value-in-two-places row doing its job.
    for route in &mut manifest.acquisition {
        route.installed_version = version.clone();
    }
    Ok(())
}

/// Turns the record `set_version` just produced into a correction of itself.
///
/// ⛔ The prior identifier is read off the record rather than recomputed, so the
/// two can never disagree about what is being corrected. The capture moves in
/// both documents, because a profile and its run are paired by capture and a
/// correction is a second run of the same build.
fn set_correction(
    profile: &mut Profile,
    manifest: &mut RunManifest,
    version_text: &str,
) -> Result<(), String> {
    let prior = profile.id;
    let capture = Slug::parse(&format!("cap-{}-fix", version_text.replace('.', "-")))
        .map_err(|error| error.to_string())?;
    profile.capture.id = capture.clone();
    manifest.capture = capture;
    profile.id = RecordId::derive(&RecordKey {
        schema: &profile.schema,
        target: &profile.target.id,
        version: &profile.build.version,
        platform: &profile.build.platform,
        arch: &profile.build.arch,
        package: &profile.build.package,
        capture: &profile.capture.id,
    });
    profile.supersedes = Some(prior);
    // ⚠ Taken from the correction fixture, whose cited evidence the base record
    // also carries. `E-ADJ-04` refuses an adjudication citing evidence the
    // record does not have, so inventing one here would be refused.
    profile.adjudication = Profile::from_json(CORRECTION)
        .map_err(|error| error.to_string())?
        .adjudication;
    Ok(())
}

fn build(root: &Path, version: Option<&str>, correct: bool) -> Result<(usize, usize), String> {
    let mut profile = Profile::from_json(PROFILE).map_err(|error| error.to_string())?;
    let mut manifest = RunManifest::from_json(MANIFEST).map_err(|error| error.to_string())?;
    if let Some(text) = version {
        set_version(&mut profile, &mut manifest, text)?;
        if correct {
            set_correction(&mut profile, &mut manifest, text)?;
        }
    } else if correct {
        return Err("--correct needs a version to correct at".to_owned());
    }

    // The paths are resolved before the documents are touched, because the key
    // borrows the manifest that the loop below rewrites.
    let (profile_path, manifest_path, artifact_paths) = {
        let key = StoreKey::of_manifest(&manifest);
        let mut paths = Vec::with_capacity(manifest.evidence.len());
        for artifact in &manifest.evidence {
            paths.push(
                key.evidence_path(&artifact.path)
                    .map_err(|error| error.to_string())?,
            );
        }
        (
            key.profile_path().map_err(|error| error.to_string())?,
            key.manifest_path().map_err(|error| error.to_string())?,
            paths,
        )
    };

    // ⛔ The bytes are written first and the documents describe what was
    // written. Deriving the other way round is how a manifest comes to describe
    // an artifact nobody has.
    let mut artifacts = 0_usize;
    for (artifact, relative) in manifest.evidence.iter_mut().zip(&artifact_paths) {
        let bytes = payload(artifact.id.as_str());
        write_file(root, relative.as_str(), &bytes)?;
        artifact.bytes = bytes.len() as u64;
        artifact.sha256 = Sha256Digest::of(&bytes);
        artifacts += 1;
    }
    for cited in &mut profile.evidence {
        let bytes = payload(cited.id.as_str());
        cited.bytes = bytes.len() as u64;
        cited.sha256 = Sha256Digest::of(&bytes);
    }

    let manifest_document = manifest.to_json().map_err(|error| error.to_string())?;
    let profile_document = profile.to_json().map_err(|error| error.to_string())?;
    write_file(root, manifest_path.as_str(), manifest_document.as_bytes())?;
    write_file(root, profile_path.as_str(), profile_document.as_bytes())?;

    Ok((artifacts, 2))
}

fn main() -> ExitCode {
    // ⚠ Order is preserved across both flags, because a correction has to be
    // written after the record it supersedes for the result to be a store the
    // corpus validator accepts.
    let mut jobs: Vec<(String, bool)> = Vec::new();
    let mut positional: Vec<std::ffi::OsString> = Vec::new();
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        let correcting = arg == *"--correct";
        if arg != *"--version" && !correcting {
            positional.push(arg);
            continue;
        }
        let Some(text) = args.next().and_then(|value| value.into_string().ok()) else {
            let _ = writeln!(
                std::io::stderr(),
                "{} needs a version string",
                if correcting { "--correct" } else { "--version" }
            );
            return ExitCode::from(2);
        };
        jobs.push((text, correcting));
    }
    let [root_arg] = positional.as_slice() else {
        let _ = writeln!(
            std::io::stderr(),
            "usage: build-store [--version V]... [--correct V]... DIR\n\
             writes one coherent synthetic store: a record, its run, and the artifacts it cites"
        );
        return ExitCode::from(2);
    };
    let root = Path::new(&root_arg);
    if !root.is_dir() {
        let _ = writeln!(
            std::io::stderr(),
            "{}: not a directory. Create it first, so this never writes into a path that was a \
             typo",
            root.display()
        );
        return ExitCode::from(2);
    }

    let wanted: Vec<(Option<&str>, bool)> = if jobs.is_empty() {
        vec![(None, false)]
    } else {
        jobs.iter()
            .map(|(text, correct)| (Some(text.as_str()), *correct))
            .collect()
    };
    let mut artifacts = 0_usize;
    let mut documents = 0_usize;
    for (version, correct) in wanted {
        match build(root, version, correct) {
            Ok((wrote_artifacts, wrote_documents)) => {
                artifacts += wrote_artifacts;
                documents += wrote_documents;
            }
            Err(error) => {
                let _ = writeln!(std::io::stderr(), "cannot build the store: {error}");
                return ExitCode::from(1);
            }
        }
    }
    let _ = writeln!(
        std::io::stdout(),
        "wrote {documents} document(s) and {artifacts} artifact(s) into {}",
        root.display()
    );
    ExitCode::SUCCESS
}
