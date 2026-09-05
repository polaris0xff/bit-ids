//! `OBS-09`'s acceptance: a bundle written to disk, read back off it, and bound
//! against the profile that cites it.
//!
//! ⚠ **The Prove was authored as `cargo test --workspace --locked --test
//! evidence_journal`, and `--test <target>` skips the library's own tests.**
//! `OBS-01` was corrected for the same shape and
//! `docs/conventions/forbidden-patterns.md` carries the class, so the
//! acceptance run is `cargo test -p bit-ids-lab --locked --all-targets`, which
//! is this file and the module's own unit tests together. `CI-05` is the check
//! that would stop it returning.
//!
//! ⭐ **The golden documents are included from `bit-ids` rather than copied.**
//! The manifest and profile fixtures are that crate's, and a second copy here
//! would be a value in two places with nothing comparing them: the copy would
//! keep validating against a schema that had moved.

use std::collections::BTreeMap;
use std::io::{Read as _, Write as _};
use std::net::TcpStream;
use std::time::Duration;

use bit_ids::canonical::{Sha256Digest, Slug};
use bit_ids::manifest::{PhaseName, RedactionRule};
use bit_ids::record::EvidenceKind;
use bit_ids::{Profile, RunManifest, bind};
use bit_ids_lab::evidence::{REDACTED, TRANSCRIPT_SCHEMA};
use bit_ids_lab::{Bundle, BundleError, Journal, Lab, Scrub, StreamReply, TranscriptOf};
use bit_ids_wire::tracker_udp::Direction;
use serde_json::{Value, json};

const MANIFEST: &str = include_str!("../../bit-ids/tests/fixtures/valid-manifest.json");
const PROFILE: &str = include_str!("../../bit-ids/tests/fixtures/valid-profile.json");

/// The bytes the metainfo artifact carries, named so a test can assert the
/// record describes what was handed over rather than only what is on disk.
const METAINFO: &[u8] = b"d4:infod4:name4:teseee";

fn slug(text: &str) -> Slug {
    Slug::parse(text).expect("a canonical identifier")
}

/// A directory this test owns, removed and recreated so a rerun starts clean.
///
/// ⚠ Under `target/`, not the system temporary directory. A bundle is compared
/// byte for byte against what a record says, and a shared temporary directory
/// is the one place another process can write into the middle of that.
fn scratch(name: &str) -> std::path::PathBuf {
    let root = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("the scratch directory is creatable");
    root
}

/// Runs a lab with one stream endpoint, sends `bytes`, and answers `reply`.
fn journal_of_one_exchange(endpoint: &str, bytes: &[u8], reply: &'static [u8]) -> Journal {
    let name = endpoint.to_owned();
    let lab = Lab::builder()
        .deadline(Duration::from_secs(60))
        .stream(&name, move |_connection, received: &[u8]| {
            StreamReply::Close {
                send: {
                    let _ = received;
                    reply.to_vec()
                },
            }
        })
        .expect("a canonical endpoint name")
        .start()
        .expect("loopback binds");

    let address = lab.endpoint(endpoint).expect("added").address();
    let mut client = TcpStream::connect(address).expect("the endpoint accepts");
    client
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("a read timeout is settable");
    client.write_all(bytes).expect("write");
    client.flush().expect("flush");
    let mut answer = Vec::new();
    let _ = client.read_to_end(&mut answer);
    drop(client);
    lab.shutdown()
}

fn peer_plan() -> BTreeMap<Slug, TranscriptOf> {
    BTreeMap::from([(
        slug("peer-wire"),
        TranscriptOf {
            id: slug("ev-peer-transcript"),
            kind: EvidenceKind::PeerTranscript,
        },
    )])
}

/// A bundle carrying the three artifacts a lab run produces, under the
/// identifiers the golden profile already cites.
fn bundle_of_a_run(root: &std::path::Path) -> Bundle {
    let journal = journal_of_one_exchange("peer-wire", b"\x13BitTorrent protocol", b"answered");
    let mut bundle = Bundle::create(root, slug("bit-ids-probe"), PhaseName::Observed)
        .expect("the bundle root is creatable");
    bundle
        .transcripts(&journal, &peer_plan())
        .expect("the plan names every endpoint");
    bundle
        .transcript(
            slug("ev-metainfo"),
            EvidenceKind::Metainfo,
            "fixture/generated.torrent",
            METAINFO,
        )
        .expect("the metainfo is written");
    bundle
        .scrubbed_text(
            slug("ev-observer-stream"),
            EvidenceKind::ObserverStream,
            "observer/events.jsonl",
            "{\"at\":\"/home/runner/work/run\",\"peer\":\"127.0.0.1:6881\"}\n\
             {\"at\":\"/Users/runner/cache\",\"peer\":\"127.0.0.1:6882\"}\n",
            &[
                Scrub::Ipv4Addresses,
                // ⭐ Two literals under one rule, so the writer has to aggregate:
                // `E-MAN-64` refuses a rule declared twice against one artifact,
                // and a writer that pushed a row per scrub would produce exactly
                // that and only on a run that scrubbed two paths.
                Scrub::Literal {
                    rule: RedactionRule::AbsolutePath,
                    value: "/home/runner/work/run".to_owned(),
                },
                Scrub::Literal {
                    rule: RedactionRule::AbsolutePath,
                    value: "/Users/runner/cache".to_owned(),
                },
            ],
        )
        .expect("the observer stream is written");
    bundle
}

/// Rewrites the two golden documents so their evidence is the bundle's.
///
/// Each document keeps its own shape: a manifest row carries the tool, the
/// phase and the redaction flag, and a profile row carries the connector. That
/// difference is the point of `bind` and is why the splice cannot be one
/// function over one value.
fn splice(bundle: &Bundle, into_manifest: bool, into_profile: bool) -> (Value, Value) {
    let mut manifest: Value = serde_json::from_str(MANIFEST).expect("the manifest fixture is JSON");
    let mut profile: Value = serde_json::from_str(PROFILE).expect("the profile fixture is JSON");

    for record in bundle.evidence() {
        let id = record.id.to_string();
        if into_manifest {
            let row = json!({
                "id": id,
                "kind": serde_json::to_value(record.kind).expect("a kind serializes"),
                "path": record.path.as_str(),
                "bytes": record.bytes,
                "sha256": record.sha256.to_string(),
                "produced_by": record.produced_by.to_string(),
                "phase": serde_json::to_value(record.phase).expect("a phase serializes"),
                "redacted": record.redacted,
            });
            replace_row(&mut manifest, &id, row);
        }
        if into_profile {
            let row = json!({
                "id": id,
                "kind": serde_json::to_value(record.kind).expect("a kind serializes"),
                "path": record.path.as_str(),
                "bytes": record.bytes,
                "sha256": record.sha256.to_string(),
                "connector": connector_of(&profile, &id),
            });
            replace_row(&mut profile, &id, row);
        }
    }

    if into_manifest {
        // The golden file declares a redaction against `ev-observer-stream`,
        // and `E-MAN-62` and `E-MAN-63` require the flag and the declarations to
        // agree. The bundle is the authority on both, so both come from it.
        manifest["redactions"] = Value::Array(
            bundle
                .redactions()
                .iter()
                .map(|redaction| {
                    json!({
                        "evidence": redaction.evidence.to_string(),
                        "rule": serde_json::to_value(redaction.rule).expect("a rule serializes"),
                        "occurrences": redaction.occurrences,
                    })
                })
                .collect(),
        );
    }
    (manifest, profile)
}

fn connector_of(profile: &Value, id: &str) -> Value {
    profile["evidence"]
        .as_array()
        .expect("evidence is a list")
        .iter()
        .find(|row| row["id"] == id)
        .map_or(Value::Null, |row| row["connector"].clone())
}

fn replace_row(document: &mut Value, id: &str, row: Value) {
    let rows = document["evidence"]
        .as_array_mut()
        .expect("evidence is a list");
    let at = rows
        .iter()
        .position(|existing| existing["id"] == id)
        .unwrap_or_else(|| panic!("the golden document does not cite {id}"));
    rows[at] = row;
}

// --- what the Prove names ---------------------------------------------------

#[test]
fn the_bundle_binds_against_the_profile_that_cites_it() {
    let root = scratch("bind");
    let bundle = bundle_of_a_run(&root);
    let (manifest, profile) = splice(&bundle, true, true);

    // ⛔ Both documents parse through the validating route, so what `bind`
    // reports is a disagreement between them rather than a defect inside one.
    let manifest = RunManifest::from_json(&manifest.to_string())
        .expect("the manifest carrying the bundle's rows is valid");
    let profile = Profile::from_json(&profile.to_string())
        .expect("the profile carrying the bundle's rows is valid");
    bind(&manifest, &profile).expect("the two documents agree about the bundle");
}

#[test]
fn a_row_spliced_into_one_document_and_not_the_other_is_refused_by_bind() {
    // ⭐ The control for the test above. Both documents start out agreeing
    // about artifacts that do not exist, so a `bind` that passed over the real
    // ones would also have passed had the writer produced nothing at all.
    let root = scratch("bind-control");
    let bundle = bundle_of_a_run(&root);
    let (manifest, profile) = splice(&bundle, true, false);
    let manifest =
        RunManifest::from_json(&manifest.to_string()).expect("the manifest stays valid alone");
    let profile = Profile::from_json(&profile.to_string()).expect("the profile stays valid alone");
    let violations =
        bind(&manifest, &profile).expect_err("a manifest describing other bytes must not bind");
    assert!(
        violations.to_string().contains("E-BND-"),
        "expected a binding violation, got {violations}"
    );
}

#[test]
fn every_artifact_reads_back_off_disk_as_its_record_describes_it() {
    let root = scratch("readback");
    let bundle = bundle_of_a_run(&root);
    assert_eq!(bundle.evidence().len(), 3);

    // ⛔ Against what the caller asked to write, not only against the disk. A
    // record compared with the file alone is self-consistent over a short
    // write: the truncated bytes digest to the truncated digest and nothing
    // says the artifact is missing half of itself.
    let metainfo = bundle
        .evidence()
        .iter()
        .find(|record| record.kind == EvidenceKind::Metainfo)
        .expect("the run wrote a metainfo");
    assert_eq!(
        metainfo.bytes,
        u64::try_from(METAINFO.len()).expect("a small artifact"),
        "the record does not describe the bytes the caller handed over"
    );
    assert_eq!(metainfo.sha256, Sha256Digest::of(METAINFO));

    for record in bundle.evidence() {
        // ⛔ The tool and the phase the bundle was opened with. `E-MAN-52` and
        // `E-MAN-53` only require these to name something the run declares, so
        // a writer that filed every artifact under another declared tool or a
        // later phase produces a manifest that validates and lies.
        assert_eq!(
            record.produced_by,
            slug("bit-ids-probe"),
            "{} names a tool that did not write it",
            record.id
        );
        assert_eq!(
            record.phase,
            PhaseName::Observed,
            "{} is filed under a phase it did not come out of",
            record.id
        );
        let found =
            std::fs::read(root.join(record.path.as_str())).expect("the artifact is on disk");
        assert_eq!(
            u64::try_from(found.len()).expect("a small artifact"),
            record.bytes,
            "{} is not the size its record claims",
            record.id
        );
        assert_eq!(
            Sha256Digest::of(&found),
            record.sha256,
            "{} does not digest to the value its record carries",
            record.id
        );
    }
    bundle.verify().expect("a bundle nobody touched verifies");
}

#[test]
fn a_truncated_artifact_is_refused_rather_than_verified() {
    let root = scratch("truncated");
    let bundle = bundle_of_a_run(&root);
    let victim = bundle
        .evidence()
        .iter()
        .find(|record| record.kind == EvidenceKind::PeerTranscript)
        .expect("the run wrote a peer transcript");
    let path = root.join(victim.path.as_str());
    let whole = std::fs::read(&path).expect("readable");
    std::fs::write(&path, &whole[..whole.len() / 2]).expect("writable");

    match bundle.verify() {
        Err(BundleError::ShortWrite { id, found, .. }) => {
            assert_eq!(id, victim.id);
            assert_eq!(found, whole.len() / 2);
        }
        other => panic!("a truncated artifact was not refused: {other:?}"),
    }

    // ⛔ And an edit that keeps the length, because a check on the size alone
    // passes over exactly the tampering that matters most.
    let mut bent = whole.clone();
    let last = bent.len() - 1;
    bent[last] ^= 0x01;
    std::fs::write(&path, &bent).expect("writable");
    assert!(
        matches!(bundle.verify(), Err(BundleError::ShortWrite { .. })),
        "an artifact edited without changing its length was accepted"
    );

    std::fs::write(&path, &whole).expect("writable");
    bundle.verify().expect("the restored bundle verifies again");
}

/// ⛔ A path that is spelled correctly and resolves somewhere else.
///
/// `RelPath` refuses `..`, a leading separator and a backslash, and every one of
/// those is a rule about the text. A symlink sitting in a reused bundle root
/// satisfies all of them and still lands the artifact outside, with the manifest
/// citing a path that reads as inside. Two gates on one action, and this proves
/// the second is there.
///
/// ⚠ Unix only, for the symlink.
#[cfg(target_family = "unix")]
#[test]
fn an_artifact_whose_directory_leads_out_of_the_bundle_is_refused() {
    let root = scratch("escape");
    let elsewhere = scratch("escape-target");
    std::os::unix::fs::symlink(&elsewhere, root.join("observer")).expect("a symlink is creatable");

    let mut bundle = Bundle::create(&root, slug("bit-ids-probe"), PhaseName::Observed)
        .expect("the root is creatable");
    match bundle.transcript(
        slug("ev-observer-stream"),
        EvidenceKind::ObserverStream,
        "observer/events.jsonl",
        METAINFO,
    ) {
        Err(BundleError::Outside(path)) => assert_eq!(path.as_str(), "observer/events.jsonl"),
        other => panic!("a path leading out of the bundle was not refused: {other:?}"),
    }
    assert!(
        !elsewhere.join("events.jsonl").exists(),
        "the artifact was written outside the bundle anyway"
    );
    assert!(bundle.evidence().is_empty());
}

/// ⭐ The other half of the escape check: the root itself reached through a
/// symlink.
///
/// The check compares where a write resolves against the root, so the root has
/// to be resolved too or the comparison refuses everything. That is not a
/// hypothetical shape: `/tmp` is a symlink on macOS, and a bind mount or a
/// symlinked home directory does the same on Linux. A bundle that refused every
/// artifact there would read as a broken filesystem rather than as this check.
///
/// ⚠ Unix only, for the symlink.
#[cfg(target_family = "unix")]
#[test]
fn a_bundle_root_reached_through_a_symlink_still_accepts_its_own_artifacts() {
    let real = scratch("symlinked-root-target");
    let links = scratch("symlinked-root");
    let root = links.join("bundle");
    std::os::unix::fs::symlink(&real, &root).expect("a symlink is creatable");

    let mut bundle = Bundle::create(&root, slug("bit-ids-probe"), PhaseName::Observed)
        .expect("the root is creatable");
    bundle
        .transcript(
            slug("ev-metainfo"),
            EvidenceKind::Metainfo,
            "fixture/generated.torrent",
            METAINFO,
        )
        .expect("a root behind a symlink is still this bundle's root");
    assert_eq!(
        std::fs::read(real.join("fixture/generated.torrent")).expect("on disk"),
        METAINFO
    );
    bundle.verify().expect("and it verifies");
}

/// ⛔ Something already at the artifact's path.
///
/// A rerun into a dirty root would overwrite the earlier artifact, and a file
/// planted there would be followed. Both are refused before anything is written.
#[test]
fn an_artifact_path_that_is_already_taken_is_refused_rather_than_overwritten() {
    let root = scratch("occupied");
    std::fs::write(root.join("planted.bin"), b"not this run's evidence").expect("writable");

    let mut bundle = Bundle::create(&root, slug("bit-ids-probe"), PhaseName::Observed)
        .expect("the root is creatable");
    match bundle.transcript(
        slug("ev-metainfo"),
        EvidenceKind::Metainfo,
        "planted.bin",
        METAINFO,
    ) {
        Err(BundleError::Occupied(path)) => assert_eq!(path.as_str(), "planted.bin"),
        other => panic!("an occupied path was not refused: {other:?}"),
    }
    assert_eq!(
        std::fs::read(root.join("planted.bin")).expect("still there"),
        b"not this run's evidence",
        "the file that was already there was overwritten"
    );
}

/// ⛔ The write path's own read-back, provoked rather than argued about.
///
/// The guard exists for a filesystem that accepts a write and stores something
/// else, and a test cannot make an ordinary file do that. The null device can:
/// it takes every byte and reads back empty, which is the same shape as a full
/// disk that reports success. Without this the guard is only refutable by
/// breaking the write itself, and a removed read-back reads as green.
///
/// ⚠ Unix only, because the null device is. `check-runner` is already
/// platform-gated for the same reason and the Windows lane skips it. Nothing is
/// written that could alter the host: the null device discards it.
#[cfg(target_family = "unix")]
#[test]
fn a_target_that_stores_something_other_than_what_was_written_is_refused() {
    let mut bundle = Bundle::create("/dev", slug("bit-ids-probe"), PhaseName::Observed)
        .expect("the null device's directory exists");
    match bundle.transcript(
        slug("ev-metainfo"),
        EvidenceKind::Metainfo,
        "null",
        METAINFO,
    ) {
        Err(BundleError::ShortWrite {
            id,
            intended,
            found,
        }) => {
            assert_eq!(id, slug("ev-metainfo"));
            assert_eq!(intended, METAINFO.len());
            assert_eq!(
                found, 0,
                "the device read back as something other than empty"
            );
        }
        other => panic!("a write that did not survive was recorded anyway: {other:?}"),
    }
    assert!(
        bundle.evidence().is_empty(),
        "a write that did not survive still produced a record"
    );
}

// --- the transcript document ------------------------------------------------

#[test]
fn the_transcript_keeps_the_order_direction_and_connection_of_every_segment() {
    let root = scratch("transcript");
    let journal = journal_of_one_exchange("peer-wire", b"first request", b"an answer");
    let mut bundle = Bundle::create(&root, slug("bit-ids-probe"), PhaseName::Observed)
        .expect("the root is creatable");
    bundle
        .transcripts(&journal, &peer_plan())
        .expect("the plan names the endpoint");

    let written = std::fs::read_to_string(root.join("peer-wire.transcript.json"))
        .expect("the transcript is on disk");
    let document: Value = serde_json::from_str(&written).expect("the transcript is JSON");
    // ⛔ The literal, not only the constant. `TRANSCRIPT_SCHEMA` moves with any
    // edit to it, so an assertion against it alone cannot see the version
    // disappear. `OBS-08` found the same shape in its own constants.
    assert_eq!(document["schema"], "bit-ids/transcript/1");
    assert_eq!(document["schema"], TRANSCRIPT_SCHEMA);
    assert_eq!(document["endpoint"], "peer-wire");

    let segments = document["segments"].as_array().expect("a list");
    let recorded = journal.for_endpoint(&slug("peer-wire"));
    assert_eq!(segments.len(), recorded.len());
    assert!(!recorded.is_empty(), "the run recorded nothing to write");
    for (written, segment) in segments.iter().zip(recorded) {
        assert_eq!(
            written["direction"],
            match segment.direction() {
                Direction::FromTarget => "from_target",
                Direction::ToTarget => "to_target",
            },
            "the direction a segment travelled was not kept"
        );
        assert_eq!(written["offset_ms"], segment.offset_ms());
        assert_eq!(
            written["connection"],
            segment
                .connection()
                .map_or(Value::Null, |id| json!(id.get())),
            "the connection a segment belonged to was not kept"
        );
        let hex = segment.bytes().iter().fold(String::new(), |mut out, byte| {
            use core::fmt::Write as _;
            write!(out, "{byte:02x}").expect("a String cannot fail");
            out
        });
        assert_eq!(written["bytes"], hex, "the bytes were not kept exactly");
    }

    // ⭐ Both directions are present, so a writer that kept only what the target
    // sent would fail here rather than passing over a one-sided transcript.
    let directions: Vec<&Value> = segments.iter().map(|one| &one["direction"]).collect();
    assert!(directions.contains(&&json!("from_target")));
    assert!(directions.contains(&&json!("to_target")));
}

#[test]
fn a_run_with_two_surfaces_writes_one_artifact_each_and_mixes_neither() {
    // ⛔ Every guard that separates the endpoints is invisible on a run with
    // one. A writer that put the whole journal in each file, or that wrote one
    // file twice, passes every single-endpoint test there is.
    let root = scratch("two-surfaces");
    let lab = Lab::builder()
        .deadline(Duration::from_secs(60))
        .stream("tracker-http", |_connection, _received: &[u8]| {
            StreamReply::Close {
                send: b"HTTP/1.1 200 OK\r\n\r\n".to_vec(),
            }
        })
        .expect("a canonical endpoint name")
        .stream("peer-wire", |_connection, _received: &[u8]| {
            StreamReply::Close {
                send: b"peer-answer".to_vec(),
            }
        })
        .expect("a canonical endpoint name")
        .start()
        .expect("loopback binds");

    // Interleaved on purpose: the endpoints run on their own threads and the
    // journal is one ordered list, so the writer has to group by endpoint
    // rather than by arrival.
    for (endpoint, request) in [
        ("tracker-http", &b"GET /announce HTTP/1.1\r\n\r\n"[..]),
        ("peer-wire", b"a peer request"),
        ("tracker-http", b"GET /again HTTP/1.1\r\n\r\n"),
    ] {
        let address = lab.endpoint(endpoint).expect("added").address();
        let mut client = TcpStream::connect(address).expect("the endpoint accepts");
        client
            .set_read_timeout(Some(Duration::from_secs(10)))
            .expect("a read timeout is settable");
        client.write_all(request).expect("write");
        client.flush().expect("flush");
        let mut answer = Vec::new();
        let _ = client.read_to_end(&mut answer);
    }
    let journal = lab.shutdown();

    let plan = BTreeMap::from([
        (
            slug("peer-wire"),
            TranscriptOf {
                id: slug("ev-peer-transcript"),
                kind: EvidenceKind::PeerTranscript,
            },
        ),
        (
            slug("tracker-http"),
            TranscriptOf {
                id: slug("ev-observer-stream"),
                kind: EvidenceKind::TrackerCapture,
            },
        ),
    ]);
    let mut bundle = Bundle::create(&root, slug("bit-ids-probe"), PhaseName::Observed)
        .expect("the root is creatable");
    bundle
        .transcripts(&journal, &plan)
        .expect("the plan names both endpoints");

    assert_eq!(bundle.evidence().len(), 2, "one artifact per endpoint");
    // ⛔ `E-MAN-50` requires ascending identifiers, and the endpoints were not
    // written in that order.
    let ids: Vec<String> = bundle
        .evidence()
        .iter()
        .map(|record| record.id.to_string())
        .collect();
    let mut ascending = ids.clone();
    ascending.sort();
    assert_eq!(ids, ascending, "the records are not in ascending order");

    for (file, mine, theirs) in [
        ("peer-wire.transcript.json", "peer-wire", "tracker-http"),
        ("tracker-http.transcript.json", "tracker-http", "peer-wire"),
    ] {
        let written = std::fs::read_to_string(root.join(file)).expect("the transcript is on disk");
        let document: Value = serde_json::from_str(&written).expect("the transcript is JSON");
        assert_eq!(document["endpoint"], mine);
        assert_eq!(
            document["segments"].as_array().expect("a list").len(),
            journal.for_endpoint(&slug(mine)).len(),
            "{mine} did not get exactly its own segments"
        );
        let other: String = journal
            .for_endpoint(&slug(theirs))
            .iter()
            .flat_map(|segment| segment.bytes().iter().map(|byte| format!("{byte:02x}")))
            .collect();
        assert!(
            !other.is_empty() && !written.contains(&other),
            "{mine}'s artifact carries {theirs}'s bytes"
        );
    }
}

#[test]
fn two_bundles_of_one_journal_are_byte_for_byte_the_same() {
    // ⚠ A capture that produced a different artifact each time it was written
    // would content-address to a new object per run, so a store could never say
    // two runs captured the same bytes.
    let journal = journal_of_one_exchange("peer-wire", b"a request", b"an answer");
    let mut digests = Vec::new();
    for name in ["deterministic-a", "deterministic-b"] {
        let root = scratch(name);
        let mut bundle = Bundle::create(&root, slug("bit-ids-probe"), PhaseName::Observed)
            .expect("the root is creatable");
        bundle
            .transcripts(&journal, &peer_plan())
            .expect("the plan names the endpoint");
        digests.push(
            bundle
                .evidence()
                .iter()
                .map(|record| record.sha256.to_string())
                .collect::<Vec<_>>(),
        );
    }
    assert_eq!(digests[0], digests[1]);
}

// --- the refusals -----------------------------------------------------------

#[test]
fn an_endpoint_the_plan_does_not_name_is_refused_rather_than_guessed_at() {
    let root = scratch("unplanned");
    let journal = journal_of_one_exchange("tracker-http", b"GET / HTTP/1.1\r\n\r\n", b"HTTP/1.1");
    let mut bundle = Bundle::create(&root, slug("bit-ids-probe"), PhaseName::Observed)
        .expect("the root is creatable");
    match bundle.transcripts(&journal, &peer_plan()) {
        Err(BundleError::Unplanned(endpoint)) => assert_eq!(endpoint, slug("tracker-http")),
        other => panic!("an unplanned endpoint was not refused: {other:?}"),
    }
    assert!(
        bundle.evidence().is_empty(),
        "a refused plan still wrote a record"
    );
}

#[test]
fn an_artifact_with_no_bytes_and_a_name_claimed_twice_are_both_refused() {
    let root = scratch("refusals");
    let mut bundle = Bundle::create(&root, slug("bit-ids-probe"), PhaseName::Observed)
        .expect("the root is creatable");

    // `E-MAN-51`: a parsed value with no recoverable bytes is not a
    // measurement, so the writer refuses before the manifest has to.
    match bundle.transcript(slug("ev-metainfo"), EvidenceKind::Metainfo, "a.bin", b"") {
        Err(BundleError::Empty(id)) => assert_eq!(id, slug("ev-metainfo")),
        other => panic!("an empty artifact was not refused: {other:?}"),
    }

    bundle
        .transcript(slug("ev-metainfo"), EvidenceKind::Metainfo, "a.bin", b"xy")
        .expect("the first write lands");
    assert!(matches!(
        bundle.transcript(slug("ev-metainfo"), EvidenceKind::Metainfo, "b.bin", b"xy"),
        Err(BundleError::Duplicate(_))
    ));
    assert!(matches!(
        bundle.transcript(
            slug("ev-packet-capture"),
            EvidenceKind::PacketCapture,
            "a.bin",
            b"xy"
        ),
        Err(BundleError::Duplicate(_))
    ));
    assert!(matches!(
        bundle.transcript(
            slug("ev-packet-capture"),
            EvidenceKind::PacketCapture,
            "../out",
            b"xy"
        ),
        Err(BundleError::Name(_))
    ));
    assert_eq!(
        bundle.evidence().len(),
        1,
        "a refusal wrote a record anyway"
    );
}

// --- what was scrubbed ------------------------------------------------------

#[test]
fn a_scrub_declares_what_it_removed_and_a_transcript_is_never_marked_redacted() {
    let root = scratch("scrubbed");
    let bundle = bundle_of_a_run(&root);

    let stream = bundle
        .evidence()
        .iter()
        .find(|record| record.id == slug("ev-observer-stream"))
        .expect("the run wrote an observer stream");
    assert!(stream.redacted, "a scrubbed artifact says so");

    // Two addresses and two paths, aggregated to one declaration per rule
    // because `E-MAN-64` refuses a rule declared twice against one artifact.
    let declared: Vec<(RedactionRule, u32)> = bundle
        .redactions()
        .iter()
        .filter(|redaction| redaction.evidence == stream.id)
        .map(|redaction| (redaction.rule, redaction.occurrences))
        .collect();
    // In the order the scrubs were declared, which is the order the writer
    // applies them and therefore the order it reports them.
    assert_eq!(
        declared,
        [
            (RedactionRule::IpAddress, 2),
            (RedactionRule::AbsolutePath, 2),
        ]
    );

    let text = std::fs::read_to_string(root.join(stream.path.as_str())).expect("on disk");
    assert!(!text.contains("127.0.0.1"), "an address survived the scrub");
    assert!(
        !text.contains("/home/runner/"),
        "an absolute path survived the scrub"
    );
    // ⛔ The literal and the constant. The literal is what a reader of a
    // published bundle recognises, so a drift in it is a format change; the
    // constant is the module's claim about that literal, and asserting only one
    // of the two leaves the other free to move. `REDACTED` had no reader
    // outside its own module until this line.
    assert_eq!(REDACTED, "[redacted]");
    assert_eq!(text.matches(REDACTED).count(), 4);

    // ⛔ And the transcripts are untouched: scrubbing the bytes a build put on
    // the wire would edit the measurement.
    for record in bundle.evidence() {
        if record.id != stream.id {
            assert!(
                !record.redacted,
                "{} is a measurement and was marked redacted",
                record.id
            );
            assert!(
                !bundle
                    .redactions()
                    .iter()
                    .any(|redaction| redaction.evidence == record.id),
                "{} had something taken out of it",
                record.id
            );
        }
    }
}

#[test]
fn a_rule_that_replaced_nothing_is_not_declared() {
    // `E-MAN-61` refuses a declaration of zero and is right to: it reads as a
    // scrub that ran, and it is a scrub that found nothing.
    let root = scratch("nothing-to-scrub");
    let mut bundle = Bundle::create(&root, slug("bit-ids-probe"), PhaseName::Observed)
        .expect("the root is creatable");
    bundle
        .scrubbed_text(
            slug("ev-installed-vendor-release"),
            EvidenceKind::ProcessOutput,
            "install/version.txt",
            "qBittorrent v4.6.2\n",
            &[
                Scrub::Ipv4Addresses,
                Scrub::Literal {
                    rule: RedactionRule::Hostname,
                    value: "not-in-the-text".to_owned(),
                },
            ],
        )
        .expect("the artifact is written");
    assert!(bundle.redactions().is_empty(), "nothing was replaced");
    assert!(!bundle.evidence()[0].redacted);
    assert_eq!(
        std::fs::read_to_string(root.join("install/version.txt")).expect("on disk"),
        "qBittorrent v4.6.2\n",
        "a version string is not an address"
    );
}
