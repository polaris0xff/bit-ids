//! Acceptance for `SCHEMA-03`.
//!
//! ```text
//! cargo test --workspace agreement
//! ```
//!
//! Every test name carries `agreement` so that command selects this file.
//!
//! Two claims are under test, and they pull in opposite directions. A conflict
//! must be *recordable*, because a disagreement whose evidence cannot be
//! written down is a disagreement the project loses. And a conflict must be
//! *unpublishable*, because publishing one would put a value in the catalogue
//! that two observers could not agree on. So a conflicted record reads and
//! validates, and is refused at the point of publication.

use bit_ids::agreement::{Projection, SeenValue};
use bit_ids::{Agreement, Profile, Violations, is_publishable, publishable, validate};
use serde_json::{Value, json};

const GOLDEN: &str = include_str!("fixtures/valid-profile.json");
const CORRECTION: &str = include_str!("fixtures/valid-correction.json");

/// The agreement module's own source, read so the coverage test cannot drift.
const AGREEMENT_SOURCE: &str = include_str!("../src/agreement.rs");

fn golden_value() -> Value {
    serde_json::from_str(GOLDEN).expect("the golden fixture is JSON")
}

fn read(document: &Value) -> Profile {
    let text = serde_json::to_string(document).expect("serializes");
    Profile::from_json(&text).unwrap_or_else(|error| {
        panic!("the mutated record must stay readable, otherwise nothing recorded it: {error}")
    })
}

fn corroboration_mut<'a>(document: &'a mut Value, path: &str) -> &'a mut Value {
    document["corroboration"]
        .as_array_mut()
        .expect("corroboration is an array")
        .iter_mut()
        .find(|entry| entry["path"] == path)
        .unwrap_or_else(|| panic!("the golden record corroborates {path}"))
}

/// Makes one field's connectors genuinely disagree, and say so.
fn plant_conflict(document: &mut Value) {
    let entry = corroboration_mut(document, "peer_wire/reserved");
    entry["observations"][1]["seen"] = json!({ "kind": "bytes", "detail": "0000000000100006" });
    entry["agreement"] = json!("disagrees");
    entry["conflict"] = json!("the packet oracle read one bit differently");
}

fn refuse_publication(document: &Value) -> Violations {
    let profile = read(document);
    match publishable(&profile) {
        Err(violations) => violations,
        Ok(()) => panic!("the record published, so the publication guard did not fire"),
    }
}

// -- the agreeing record ----------------------------------------------------

#[test]
fn agreement_publishes_a_record_whose_connectors_agree() {
    let profile = Profile::from_json(GOLDEN).expect("the golden record validates");
    publishable(&profile).expect("every measured field is corroborated");
}

#[test]
fn agreement_keeps_both_observations_rather_than_choosing_one() {
    let profile = Profile::from_json(GOLDEN).expect("the golden record validates");
    for entry in &profile.corroboration {
        assert!(
            entry.observations.len() >= 2,
            "{} records what each connector saw, not a verdict",
            entry.path
        );
        for observation in &entry.observations {
            assert!(
                profile.evidence_entry(&observation.evidence).is_some(),
                "{} cites the artifact its value came out of, so the comparison can be redone",
                entry.path
            );
        }
    }
}

#[test]
fn agreement_names_the_normalization_it_compared_through() {
    let profile = Profile::from_json(GOLDEN).expect("the golden record validates");
    let normalized: Vec<_> = profile
        .corroboration
        .iter()
        .flat_map(|entry| &entry.observations)
        .filter_map(|observation| observation.projection.normalization())
        .collect();
    assert!(
        !normalized.is_empty(),
        "the golden record has to exercise a normalized comparison"
    );
    for id in normalized {
        let declared = profile
            .normalization(id)
            .unwrap_or_else(|| panic!("{id} is declared"));
        assert!(
            declared.is_usable(),
            "a normalization used to reach agreement preserves order and unknown bytes"
        );
    }
}

// -- a conflict is recorded, and then refused ------------------------------

#[test]
fn agreement_records_a_conflict_rather_than_losing_it() {
    let mut document = golden_value();
    plant_conflict(&mut document);
    let profile = read(&document);
    validate(&profile).expect("a record carrying a disagreement is still a valid record");

    let entry = profile
        .corroboration
        .iter()
        .find(|entry| entry.path.to_string() == "peer_wire/reserved")
        .expect("the field is corroborated");
    assert_eq!(entry.overlap(), 2, "both connectors saw it");
    assert!(
        !entry.in_scope_values_match(),
        "and they saw different things"
    );
    assert!(
        entry.conflict.is_some(),
        "a disagreement nobody described is one nobody can adjudicate"
    );
}

#[test]
fn agreement_refuses_to_publish_a_conflict() {
    let mut document = golden_value();
    plant_conflict(&mut document);
    let violations = refuse_publication(&document);
    assert!(
        violations.has("E-PUB-01"),
        "the connectors disagree, so the record stays provisional: {violations}"
    );
}

#[test]
fn agreement_refuses_to_publish_a_measurement_no_second_connector_saw() {
    let mut document = golden_value();
    let entry = corroboration_mut(&mut document, "peer_wire/peer_id");
    entry["observations"][1]["seen"] = json!({ "kind": "out_of_scope" });
    entry["agreement"] = json!("not_corroborated");
    let violations = refuse_publication(&document);
    assert!(
        violations.has("E-PUB-02"),
        "one observer is not corroboration: {violations}"
    );
}

// -- the trap: nothing disagreed, because nothing else looked --------------

#[test]
fn agreement_is_not_claimed_over_a_field_one_connector_could_not_see() {
    let mut document = golden_value();
    // The packet oracle cannot see this surface at all. Nothing disagrees with
    // the probe, and that is precisely not agreement.
    corroboration_mut(&mut document, "peer_wire/peer_id")["observations"][1]["seen"] =
        json!({ "kind": "out_of_scope" });
    let text = serde_json::to_string(&document).expect("serializes");
    let error =
        Profile::from_json(&text).expect_err("an overlap of one cannot be called agreement");
    assert!(
        error.to_string().contains("E-COR-05"),
        "the refusal names the missing overlap: {error}"
    );
}

#[test]
fn agreement_counts_only_the_connectors_that_could_see_the_field() {
    let mut document = golden_value();
    corroboration_mut(&mut document, "peer_wire/peer_id")["observations"][1]["seen"] =
        json!({ "kind": "out_of_scope" });
    corroboration_mut(&mut document, "peer_wire/peer_id")["agreement"] = json!("not_corroborated");
    let profile = read(&document);
    let entry = profile
        .corroboration
        .iter()
        .find(|entry| entry.path.to_string() == "peer_wire/peer_id")
        .expect("corroborated");
    assert_eq!(entry.observations.len(), 2, "two connectors are listed");
    assert_eq!(entry.overlap(), 1, "one of them could actually see it");
    assert!(
        !matches!(entry.observations[1].seen, SeenValue::Bytes(_)),
        "an out-of-scope observation carries no value to compare"
    );
}

#[test]
fn agreement_over_an_absence_still_needs_two_connectors_to_have_looked() {
    let profile = Profile::from_json(GOLDEN).expect("the golden record validates");
    let entry = profile
        .corroboration
        .iter()
        .find(|entry| entry.path.to_string() == "dht/node_id")
        .expect("the absence is corroborated");
    assert_eq!(entry.overlap(), 2);
    for observation in &entry.observations {
        assert_eq!(
            observation.seen,
            SeenValue::Absent,
            "both connectors created the condition and saw nothing"
        );
        assert_eq!(observation.projection, Projection::Raw);
    }
}

// -- a correction says why -------------------------------------------------

#[test]
fn agreement_requires_a_correction_to_adjudicate_itself() {
    let correction = Profile::from_json(CORRECTION).expect("the correction validates");
    let adjudication = correction
        .adjudication
        .as_ref()
        .expect("a correction says why it corrects");
    assert!(
        !adjudication.evidence.is_empty(),
        "a decision with no evidence behind it is an opinion"
    );
    for id in &adjudication.evidence {
        assert!(
            correction.evidence_entry(id).is_some(),
            "the adjudication cites artifacts the record carries"
        );
    }

    let mut document: Value = serde_json::from_str(CORRECTION).expect("JSON");
    document["adjudication"] = json!(null);
    let text = serde_json::to_string(&document).expect("serializes");
    let error = Profile::from_json(&text).expect_err("a correction with no reason is refused");
    assert!(
        error.to_string().contains("E-ADJ-01"),
        "the refusal names the missing adjudication: {error}"
    );
}

// -- every publication guard is planted against ----------------------------

type Mutation = fn(&mut Value);

const PLANTED: &[(&str, Mutation)] = &[
    ("E-PUB-01", plant_conflict),
    ("E-PUB-02", |document| {
        let entry = corroboration_mut(document, "peer_wire/peer_id");
        entry["observations"][1]["seen"] = json!({ "kind": "out_of_scope" });
        entry["agreement"] = json!("not_corroborated");
    }),
];

#[test]
fn agreement_refuses_to_publish_every_planted_blocker() {
    for (code, plant) in PLANTED {
        let mut document = golden_value();
        plant(&mut document);
        let violations = refuse_publication(&document);
        assert!(
            violations.has(code),
            "planting the blocker for {code} produced {violations}"
        );
    }
}

#[test]
fn agreement_plants_a_blocker_for_every_publication_code() {
    let mut declared: Vec<&str> = AGREEMENT_SOURCE
        .match_indices("\"E-PUB-")
        .filter_map(|(start, _)| {
            let rest = &AGREEMENT_SOURCE[start + 1..];
            let end = rest.find('"')?;
            Some(&rest[..end])
        })
        .collect();
    declared.sort_unstable();
    declared.dedup();
    assert!(
        !declared.is_empty(),
        "the code scan found nothing, so it is not reading the agreement module"
    );

    let planted: Vec<&str> = PLANTED.iter().map(|(code, _)| *code).collect();
    let uncovered: Vec<&&str> = declared
        .iter()
        .filter(|code| !planted.contains(code))
        .collect();
    assert!(
        uncovered.is_empty(),
        "these publication guards have never been seen to refuse anything: {uncovered:?}"
    );
}

#[test]
fn agreement_publishes_the_record_the_blockers_are_planted_in() {
    // The table above is only evidence if the unmutated record publishes.
    let profile = Profile::from_json(GOLDEN).expect("valid");
    assert!(publishable(&profile).is_ok());
}

#[test]
fn agreement_keeps_the_per_outcome_and_per_record_rules_in_step() {
    // ⛔ Two public functions answer "may this be published": `is_publishable`
    // for one outcome, `publishable` for a whole record. A door sweep found
    // nothing holding them together, which is the same value in two places
    // with no check between them. This is that check.
    for outcome in [
        Agreement::Exact,
        Agreement::Normalized,
        Agreement::Disagrees,
        Agreement::NotCorroborated,
    ] {
        let mut document = golden_value();
        let entry = corroboration_mut(&mut document, "peer_wire/reserved");
        entry["agreement"] = json!(match outcome {
            Agreement::Exact => "exact",
            Agreement::Normalized => "normalized",
            Agreement::Disagrees => "disagrees",
            Agreement::NotCorroborated => "not_corroborated",
        });
        // Shape the observations so the record stays valid for this outcome,
        // since an invalid record would never reach the publication question.
        match outcome {
            Agreement::Disagrees => {
                entry["observations"][1]["seen"] =
                    json!({ "kind": "bytes", "detail": "0000000000100006" });
                entry["conflict"] = json!("planted");
            }
            Agreement::NotCorroborated => {
                entry["observations"][1]["seen"] = json!({ "kind": "out_of_scope" });
            }
            Agreement::Normalized => {
                entry["observations"][0]["projection"] =
                    json!({ "kind": "normalized", "detail": "bep10-value-unwrap" });
            }
            Agreement::Exact => {}
        }

        let profile = read(&document);
        let published = publishable(&profile).is_ok();
        assert_eq!(
            published,
            is_publishable(outcome),
            "the record-level answer for {outcome:?} disagrees with the per-outcome one"
        );
    }
}
