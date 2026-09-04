//! Acceptance for `SCHEMA-02`.
//!
//! ```text
//! cargo test --workspace evidence_manifest
//! ```
//!
//! Every test name carries `evidence_manifest` so that command selects this
//! file rather than running the workspace and reporting a number about
//! something else.
//!
//! Two things are proved here. A complete run round-trips, meaning the manifest
//! can express one without losing anything. And the manifest and the profile
//! are held against each other, because they overlap on purpose and a value in
//! two places with nothing comparing them is the copy a reader trusts being the
//! wrong one.

use bit_ids::manifest::{PhaseName, ToolRole};
use bit_ids::{DocumentError, Profile, RunManifest, Violations, bind};
use serde_json::{Value, json};

const MANIFEST: &str = include_str!("fixtures/valid-manifest.json");
const PROFILE: &str = include_str!("fixtures/valid-profile.json");
const UNSUPPORTED: &str = include_str!("fixtures/unsupported-manifest-schema.json");

/// The manifest module's own source, read so the coverage test cannot drift.
const MANIFEST_SOURCE: &str = include_str!("../src/manifest.rs");

fn manifest_value() -> Value {
    serde_json::from_str(MANIFEST).expect("the manifest fixture is JSON")
}

fn profile_value() -> Value {
    serde_json::from_str(PROFILE).expect("the profile fixture is JSON")
}

fn refuse(document: &Value) -> Violations {
    let text = serde_json::to_string(document).expect("a mutated manifest serializes");
    match RunManifest::from_json(&text) {
        Err(DocumentError::Invalid(violations)) => violations,
        Ok(_) => panic!("the mutated manifest validated, so the guard did not fire"),
        Err(other) => panic!("expected a refused invariant, got: {other}"),
    }
}

/// Both documents must stay individually valid, so that what `bind` reports is
/// a disagreement between them and not a defect inside one.
fn refuse_binding(manifest: &Value, profile: &Value) -> Violations {
    let manifest_text = serde_json::to_string(manifest).expect("serializes");
    let profile_text = serde_json::to_string(profile).expect("serializes");
    let manifest = RunManifest::from_json(&manifest_text)
        .expect("the mutated manifest must stay valid on its own");
    let profile =
        Profile::from_json(&profile_text).expect("the mutated profile must stay valid on its own");
    match bind(&manifest, &profile) {
        Err(violations) => violations,
        Ok(()) => panic!("the two documents agreed, so the binding guard did not fire"),
    }
}

fn phase_mut<'a>(document: &'a mut Value, name: &str) -> &'a mut Value {
    document["phases"]
        .as_array_mut()
        .expect("phases is an array")
        .iter_mut()
        .find(|phase| phase["name"] == name)
        .unwrap_or_else(|| panic!("the golden run has a {name} phase"))
}

fn drop_phase(document: &mut Value, name: &str) {
    document["phases"]
        .as_array_mut()
        .expect("phases is an array")
        .retain(|phase| phase["name"] != name);
}

fn tool_mut<'a>(document: &'a mut Value, id: &str) -> &'a mut Value {
    document["tools"]
        .as_array_mut()
        .expect("tools is an array")
        .iter_mut()
        .find(|tool| tool["id"] == id)
        .unwrap_or_else(|| panic!("the golden run declares {id}"))
}

fn evidence_mut<'a>(document: &'a mut Value, id: &str) -> &'a mut Value {
    document["evidence"]
        .as_array_mut()
        .expect("evidence is an array")
        .iter_mut()
        .find(|record| record["id"] == id)
        .unwrap_or_else(|| panic!("the golden run keeps {id}"))
}

// -- the complete run -------------------------------------------------------

#[test]
fn evidence_manifest_accepts_a_complete_run() {
    let manifest = RunManifest::from_json(MANIFEST).expect("the golden manifest validates");
    assert_eq!(manifest.schema.as_str(), bit_ids::MANIFEST_SCHEMA);
    assert_eq!(manifest.phases.len(), 7);
    // Nine since `ACQ-01`: seven observation artifacts plus the version output
    // each route's installed build printed when it was asked.
    assert_eq!(manifest.evidence.len(), 9);
    assert_eq!(manifest.acquisition.len(), 2);
    assert_eq!(
        manifest.observer().map(|tool| tool.id.as_str()),
        Some("bit-ids-probe")
    );
}

#[test]
fn evidence_manifest_round_trips_a_complete_run_byte_for_byte() {
    let manifest = RunManifest::from_json(MANIFEST).expect("the golden manifest validates");
    let written = manifest
        .to_json()
        .expect("a valid manifest has a canonical form");
    assert_eq!(
        written, MANIFEST,
        "two assemblies of one run must produce identical bytes"
    );
    let reread = RunManifest::from_json(&written).expect("the written manifest reads back");
    assert_eq!(reread, manifest);
}

#[test]
fn evidence_manifest_walks_the_capture_state_machine_in_order() {
    let manifest = RunManifest::from_json(MANIFEST).expect("the golden manifest validates");
    let walked: Vec<PhaseName> = manifest.phases.iter().map(|phase| phase.name).collect();
    assert_eq!(
        walked,
        [
            PhaseName::Planned,
            PhaseName::Resolved,
            PhaseName::AcquiredTwice,
            PhaseName::Installed,
            PhaseName::Observed,
            PhaseName::Corroborated,
            PhaseName::Validated,
        ],
        "the golden run has to walk every step or it is not a complete run"
    );
}

#[test]
fn evidence_manifest_binds_to_the_profile_of_the_same_run() {
    let manifest = RunManifest::from_json(MANIFEST).expect("the golden manifest validates");
    let profile = Profile::from_json(PROFILE).expect("the golden profile validates");
    bind(&manifest, &profile).expect("the two documents describe one run");
}

#[test]
fn evidence_manifest_addresses_every_artifact_by_its_content() {
    let manifest = RunManifest::from_json(MANIFEST).expect("the golden manifest validates");
    for record in &manifest.evidence {
        let hex = record.sha256.to_string();
        let hex = hex.strip_prefix("sha256:").expect("a canonical digest");
        let path = record.object_path();
        assert_eq!(
            path,
            format!("objects/sha256/{}/{}/{}", &hex[0..2], &hex[2..4], &hex[4..]),
            "the store path is derived from the digest, never recorded beside it"
        );
        assert!(
            !path.contains(record.path.as_str()),
            "the content address must not be the readable path in disguise"
        );
    }
}

#[test]
fn evidence_manifest_declares_what_was_scrubbed_from_each_artifact() {
    let manifest = RunManifest::from_json(MANIFEST).expect("the golden manifest validates");
    let redacted: Vec<&str> = manifest
        .evidence
        .iter()
        .filter(|record| record.redacted)
        .map(|record| record.id.as_str())
        .collect();
    assert_eq!(redacted, ["ev-observer-stream"]);
    for id in &redacted {
        assert!(
            manifest
                .redactions
                .iter()
                .any(|redaction| redaction.evidence.as_str() == *id),
            "a redacted artifact says what was taken out of it"
        );
    }
}

#[test]
fn evidence_manifest_rejects_an_unknown_schema_version() {
    match RunManifest::from_json(UNSUPPORTED) {
        Err(DocumentError::UnsupportedSchema { found, expected }) => {
            assert_eq!(found, "bit-ids/manifest/2");
            assert_eq!(
                expected,
                bit_ids::MANIFEST_SCHEMA,
                "the message names the manifest schema, not the profile's"
            );
        }
        other => panic!("expected an unsupported schema, got: {other:?}"),
    }
}

#[test]
fn evidence_manifest_rejects_a_missing_digest() {
    let mut document = manifest_value();
    document["evidence"][0]
        .as_object_mut()
        .expect("an evidence record is an object")
        .remove("sha256");
    let text = serde_json::to_string(&document).expect("serializes");
    match RunManifest::from_json(&text) {
        Err(DocumentError::Malformed(error)) => {
            assert!(
                error.to_string().contains("sha256"),
                "the refusal names the missing digest: {error}"
            );
        }
        other => panic!("an artifact with no digest is not evidence, got: {other:?}"),
    }
}

#[test]
fn evidence_manifest_rejects_a_source_url_carrying_credentials() {
    // ⚠ ASSEMBLED, NOT WRITTEN OUT. A literal `name:secret@host` in a tracked
    // file is a credential shape, and a public repository carrying one trips
    // every secret scanner that ever reads it, ours included. The value under
    // test is identical; only the spelling in the tree differs.
    let userinfo = format!("{}:{}@", "user", "token");
    let mut document = manifest_value();
    document["acquisition"][0]["source"] = json!(format!(
        "https://{userinfo}packages.example.invalid/x.tar.gz"
    ));
    let text = serde_json::to_string(&document).expect("serializes");
    match RunManifest::from_json(&text) {
        Err(DocumentError::Malformed(error)) => {
            assert!(
                error.to_string().contains("url-credentials"),
                "a published location must not carry userinfo: {error}"
            );
        }
        other => panic!("expected a refused URL, got: {other:?}"),
    }
}

#[test]
fn evidence_manifest_validates_on_every_serde_route_not_just_from_json() {
    let mut document = manifest_value();
    document["host"]["disposable"] = json!(false);
    let text = serde_json::to_string(&document).expect("serializes");
    let error = serde_json::from_str::<RunManifest>(&text)
        .expect_err("the generic serde route must not be the loose one");
    assert!(
        error.to_string().contains("E-MAN-30"),
        "the refusal carries the code: {error}"
    );
    serde_json::from_str::<RunManifest>(MANIFEST)
        .expect("a valid manifest still reads through serde");
}

// -- every guard is planted against ----------------------------------------

type Mutation = fn(&mut Value);

const PLANTED: &[(&str, Mutation)] = &[
    ("E-MAN-01", |m| {
        m["phases"] = json!([]);
    }),
    ("E-MAN-02", |m| {
        drop_phase(m, "planned");
    }),
    ("E-MAN-03", |m| {
        drop_phase(m, "installed");
    }),
    ("E-MAN-04", |m| {
        let phases = m["phases"].as_array_mut().expect("an array");
        phases.insert(
            2,
            json!({
                "name": "provisional",
                "started_at": "2026-09-04T13:30:40Z",
                "ended_at": "2026-09-04T13:30:40Z",
                "detail": "planted"
            }),
        );
    }),
    ("E-MAN-05", |m| {
        phase_mut(m, "observed")["ended_at"] = json!("2026-09-04T13:31:00Z");
    }),
    ("E-MAN-06", |m| {
        phase_mut(m, "installed")["started_at"] = json!("2026-09-04T13:30:00Z");
    }),
    ("E-MAN-07", |m| {
        let last = m["phases"].as_array().expect("an array").last().cloned();
        m["phases"]
            .as_array_mut()
            .expect("an array")
            .push(last.expect("a phase"));
    }),
    ("E-MAN-10", |m| {
        m["acquisition"]
            .as_array_mut()
            .expect("an array")
            .truncate(1);
    }),
    ("E-MAN-11", |m| {
        m["acquisition"]
            .as_array_mut()
            .expect("an array")
            .swap(0, 1);
    }),
    ("E-MAN-12", |m| {
        m["acquisition"][0]["installed_version"] = json!("0.0.1-fixture");
    }),
    ("E-MAN-13", |m| {
        m["acquisition"][0]["bytes"] = json!(0);
    }),
    ("E-MAN-20", |m| {
        tool_mut(m, "bit-ids-probe")["role"] = json!("connector");
    }),
    ("E-MAN-21", |m| {
        tool_mut(m, "packet-oracle")["role"] = json!("harness");
    }),
    ("E-MAN-22", |m| {
        m["tools"].as_array_mut().expect("an array").swap(0, 1);
    }),
    ("E-MAN-30", |m| {
        m["host"]["disposable"] = json!(false);
    }),
    ("E-MAN-31", |m| {
        m["isolation"]["network"] = json!("host_routed");
    }),
    ("E-MAN-32", |m| {
        m["isolation"]["external_reason"] = json!("no reason is needed on loopback");
    }),
    ("E-MAN-40", |m| {
        m["clocks"]["wall_end"] = json!("2026-09-04T13:29:00Z");
    }),
    ("E-MAN-41", |m| {
        m["clocks"]["monotonic_elapsed_ns"] = json!(0);
    }),
    ("E-MAN-42", |m| {
        phase_mut(m, "planned")["started_at"] = json!("2026-09-04T13:00:00Z");
    }),
    ("E-MAN-50", |m| {
        m["evidence"][1]["id"] = m["evidence"][0]["id"].clone();
    }),
    ("E-MAN-51", |m| {
        m["evidence"][0]["bytes"] = json!(0);
    }),
    ("E-MAN-52", |m| {
        m["evidence"][0]["produced_by"] = json!("a-tool-that-did-not-run");
    }),
    ("E-MAN-53", |m| {
        m["evidence"][0]["phase"] = json!("provisional");
    }),
    ("E-MAN-54", |m| {
        m["evidence"][1]["path"] = m["evidence"][0]["path"].clone();
    }),
    ("E-MAN-60", |m| {
        m["redactions"][0]["evidence"] = json!("ev-that-was-never-collected");
    }),
    ("E-MAN-61", |m| {
        m["redactions"][0]["occurrences"] = json!(0);
    }),
    ("E-MAN-62", |m| {
        evidence_mut(m, "ev-peer-transcript")["redacted"] = json!(true);
    }),
    ("E-MAN-63", |m| {
        m["redactions"]
            .as_array_mut()
            .expect("an array")
            .push(json!({
                "evidence": "ev-peer-transcript",
                "rule": "hostname",
                "occurrences": 1
            }));
    }),
    ("E-MAN-64", |m| {
        let first = m["redactions"][0].clone();
        m["redactions"]
            .as_array_mut()
            .expect("an array")
            .push(first);
    }),
];

#[test]
fn evidence_manifest_refuses_every_planted_defect() {
    for (code, plant) in PLANTED {
        let mut document = manifest_value();
        plant(&mut document);
        let violations = refuse(&document);
        assert!(
            violations.has(code),
            "planting the defect for {code} produced {violations}"
        );
    }
}

// -- the two documents are held against each other -------------------------

type Pair = fn(&mut Value, &mut Value);

/// Each row leaves both documents individually valid and makes them disagree.
const PLANTED_BINDING: &[(&str, Pair)] = &[
    ("E-BND-01", |m, _| {
        m["capture"] = json!("fixture-capture-0002");
    }),
    ("E-BND-02", |m, _| {
        m["target"] = json!("another-fixture-client");
    }),
    ("E-BND-03", |m, _| {
        m["platform"] = json!("windows");
    }),
    ("E-BND-04", |m, _| {
        m["evidence"]
            .as_array_mut()
            .expect("an array")
            .retain(|record| record["id"] != "ev-connector-report");
    }),
    ("E-BND-05", |m, _| {
        evidence_mut(m, "ev-connector-report")["bytes"] = json!(4097);
    }),
    ("E-BND-06", |_, p| {
        let connectors = p["capture"]["connectors"].as_array_mut().expect("an array");
        connectors.insert(1, json!({ "id": "ghost-connector", "version": "0.0.0" }));
    }),
    ("E-BND-07", |m, _| {
        tool_mut(m, "packet-oracle")["version"] = json!("0.0.1");
    }),
    ("E-BND-08", |m, _| {
        tool_mut(m, "bit-ids-probe")["role"] = json!("connector");
        tool_mut(m, "fixture-forge")["role"] = json!("observer");
    }),
    ("E-BND-09", |m, _| {
        m["acquisition"][0]["route"] = json!("route-mirror");
    }),
    ("E-BND-11", |m, _| {
        m["acquisition"][1]["artifact"] =
            json!("sha256:0000000000000000000000000000000000000000000000000000000000000000");
    }),
    ("E-BND-12", |_, p| {
        p["capture"]["captured_at"] = json!("2026-09-04T20:00:00Z");
    }),
    // ⛔ The publishable half of a claim the run never made. The record says the
    // signature was verified; the run says it was not checked. Nothing else
    // forces the two into step, which is why this comparison can fail and
    // `E-BND-10` could not.
    ("E-BND-13", |_, p| {
        p["acquisition"][0]["signature"] = json!("verified");
    }),
    ("E-BND-20", |m, _| {
        // A run that restarted nothing and offered one torrent over one
        // connection cannot support a field that says the value changes.
        m["sampling"] = json!({ "sessions": 1, "torrents": 1, "connections": 1 });
    }),
    ("E-BND-21", |m, _| {
        m["sampling"] = json!({ "sessions": 2, "torrents": 1, "connections": 1 });
    }),
];

#[test]
fn evidence_manifest_refuses_every_planted_disagreement() {
    for (code, plant) in PLANTED_BINDING {
        let mut manifest = manifest_value();
        let mut profile = profile_value();
        plant(&mut manifest, &mut profile);
        let violations = refuse_binding(&manifest, &profile);
        assert!(
            violations.has(code),
            "planting the disagreement for {code} produced {violations}"
        );
    }
}

#[test]
fn evidence_manifest_binds_the_documents_it_plants_defects_in() {
    // The two tables above are only evidence if the unmutated pair agrees.
    let manifest = RunManifest::from_json(MANIFEST).expect("valid");
    let profile = Profile::from_json(PROFILE).expect("valid");
    assert!(bind(&manifest, &profile).is_ok());
}

#[test]
fn evidence_manifest_plants_a_defect_for_every_diagnostic_code() {
    let mut declared: Vec<&str> = MANIFEST_SOURCE
        .match_indices("\"E-")
        .filter_map(|(start, _)| {
            let rest = &MANIFEST_SOURCE[start + 1..];
            let end = rest.find('"')?;
            Some(&rest[..end])
        })
        .collect();
    declared.sort_unstable();
    declared.dedup();
    assert!(
        declared.len() > 30,
        "the code scan found only {} codes, so it is not reading the manifest module",
        declared.len()
    );

    let mut planted: Vec<&str> = PLANTED
        .iter()
        .map(|(code, _)| *code)
        .chain(PLANTED_BINDING.iter().map(|(code, _)| *code))
        .collect();
    planted.sort_unstable();
    planted.dedup();

    let uncovered: Vec<&&str> = declared
        .iter()
        .filter(|code| !planted.contains(code))
        .collect();
    assert!(
        uncovered.is_empty(),
        "these diagnostics have never been seen to refuse anything: {uncovered:?}"
    );
    let unknown: Vec<&&str> = planted
        .iter()
        .filter(|code| !declared.contains(code))
        .collect();
    assert!(
        unknown.is_empty(),
        "these planted codes are not in the manifest module: {unknown:?}"
    );
}

#[test]
fn evidence_manifest_names_every_tool_that_took_part() {
    let manifest = RunManifest::from_json(MANIFEST).expect("valid");
    let mut roles: Vec<ToolRole> = manifest.tools.iter().map(|tool| tool.role).collect();
    roles.dedup();
    assert_eq!(manifest.tools.len(), 4);
    assert!(roles.contains(&ToolRole::Observer));
    assert!(roles.contains(&ToolRole::Connector));
    for record in &manifest.evidence {
        assert!(
            manifest.tool(&record.produced_by).is_some(),
            "every artifact names a tool the run declares"
        );
    }
}
