//! Write a coherent synthetic store into a directory.
//!
//! ```text
//! cargo run -p bit-ids --example build-store -- DIR
//! ```
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

use bit_ids::canonical::Sha256Digest;
use bit_ids::store::StoreKey;
use bit_ids::{Profile, RunManifest};

const PROFILE: &str = include_str!("../tests/fixtures/valid-profile.json");
const MANIFEST: &str = include_str!("../tests/fixtures/valid-manifest.json");

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

fn build(root: &Path) -> Result<(usize, usize), String> {
    let mut profile = Profile::from_json(PROFILE).map_err(|error| error.to_string())?;
    let mut manifest = RunManifest::from_json(MANIFEST).map_err(|error| error.to_string())?;

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
    let mut args = std::env::args_os().skip(1);
    let (Some(root_arg), None) = (args.next(), args.next()) else {
        let _ = writeln!(
            std::io::stderr(),
            "usage: build-store DIR\n\
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

    match build(root) {
        Ok((artifacts, documents)) => {
            let _ = writeln!(
                std::io::stdout(),
                "wrote {documents} document(s) and {artifacts} artifact(s) into {}",
                root.display()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "cannot build the store: {error}");
            ExitCode::from(1)
        }
    }
}
