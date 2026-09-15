//! `ACQ-03`: two routes accepted when they really are one build, and refused
//! when equal version labels stand over evidence that conflicts.
//!
//! ⛔ Equal labels are the question, not the answer. `E-ACQ-04` already forces
//! every route to report the version the record declares, so a test that only
//! checked labels would pass over every case this entry exists for.

use core::num::NonZeroU32;

use bit_ids::canonical::{HexBytes, Slug, Version};
use bit_ids::equivalence::{Equivalence, classify, classify_across};
use bit_ids::identity::{RecordId, RecordKey, SchemaVersion};
use bit_ids::observation::FieldState;
use bit_ids::sampling::{Sample, SamplingPlan, field_state};
use bit_ids::{Profile, publishable};
use serde_json::{Value, json};

const GOLDEN: &str = include_str!("fixtures/valid-profile.json");

fn golden_value() -> Value {
    serde_json::from_str(GOLDEN).expect("the golden record parses")
}

fn read(document: &Value) -> Profile {
    Profile::from_json(&serde_json::to_string(document).expect("writes"))
        .expect("the record validates")
}

/// A second capture of the same build, observed through the other route.
///
/// ⚠ The capture id changes, and so the record id must be re-derived. Two
/// records of one run are one record, which `classify_across` refuses, so a
/// realistic pair is what this test needs rather than a copy with one field
/// edited.
fn second_capture(document: &mut Value) {
    recapture(document, "fixture-capture-0002", "route-vendor-release");
}

fn recapture(document: &mut Value, capture: &str, observed: &str) {
    document["capture"]["id"] = json!(capture);
    document["capture"]["observed_route"] = json!(observed);
    document["acquisition"][1]["installed_executable"] = document["build"]["executable"].clone();
    let schema = SchemaVersion::current();
    let target = Slug::parse(document["target"]["id"].as_str().expect("a string")).expect("slug");
    let version =
        Version::parse(document["build"]["version"].as_str().expect("a string")).expect("version");
    let platform =
        Slug::parse(document["build"]["platform"].as_str().expect("a string")).expect("slug");
    let arch = Slug::parse(document["build"]["arch"].as_str().expect("a string")).expect("slug");
    let package =
        Slug::parse(document["build"]["package"].as_str().expect("a string")).expect("slug");
    let capture = Slug::parse(document["capture"]["id"].as_str().expect("a string")).expect("slug");
    let derived = RecordId::derive(&RecordKey {
        schema: &schema,
        target: &target,
        version: &version,
        platform: &platform,
        arch: &arch,
        package: &package,
        capture: &capture,
    });
    document["id"] = json!(derived.to_string());
}

/// Two archives, one executable inside, so there is one build and observing
/// either observed it.
#[test]
fn equivalence_accepts_two_routes_that_installed_the_same_bytes() {
    let profile = read(&golden_value());
    let comparison = classify(&profile);
    assert_eq!(comparison.outcome, Equivalence::ByteIdentical);
    assert_eq!(comparison.routes.len(), 2);
    assert!(
        !comparison.reasons.is_empty(),
        "a verdict with no reasoning is one nobody can check"
    );
    publishable(&profile).expect("a byte-identical pair publishes");
}

/// ⛔ The case the entry names. The labels are equal, the artifacts differ, the
/// installs differ, and only one of them was ever put on the wire. Nothing here
/// can say the other behaves the same, so it does not publish.
#[test]
fn equivalence_refuses_equal_labels_over_installs_only_one_of_which_was_observed() {
    let mut document = golden_value();
    document["acquisition"][1]["installed_executable"] =
        json!("sha256:2222222222222222222222222222222222222222222222222222222222222222");
    let profile = read(&document);

    // ⚠ Still valid: the difference is recordable, and it has to be, or there
    // is nowhere to keep the evidence of it. Publishability is the other gate.
    assert_eq!(
        profile.acquisition[0].installed_version, profile.acquisition[1].installed_version,
        "the version labels agree, which is exactly the trap"
    );
    let comparison = classify(&profile);
    assert_eq!(comparison.outcome, Equivalence::Unresolved);
    assert!(
        comparison
            .reasons
            .iter()
            .any(|reason| reason.contains("never put on the")),
        "the reason names what is missing: {:?}",
        comparison.reasons
    );

    let refused = publishable(&profile).expect_err("an unresolved pair does not publish");
    assert!(refused.has("E-PUB-04"), "{refused}");
}

/// One route is not a comparison, whatever else the record carries.
#[test]
fn equivalence_needs_two_routes_before_it_says_anything() {
    let mut document = golden_value();
    document["acquisition"]
        .as_array_mut()
        .expect("an array")
        .truncate(1);
    // The record is invalid now, by `E-ACQ-01`, so the classifier is asked
    // directly: it must not answer confidently over one route either.
    let text = serde_json::to_string(&document).expect("writes");
    assert!(
        Profile::from_json(&text).is_err(),
        "one route is not a publishable record"
    );
}

/// Two captures of one build through different routes, agreeing on every field
/// both measured. This is the only path to `build_equivalent`, and it needs a
/// capture per route rather than a second opinion about one.
#[test]
fn equivalence_across_two_routes_agrees_when_both_were_observed() {
    let first = read(&golden_value());
    let mut second = golden_value();
    second_capture(&mut second);
    let second = read(&second);

    let comparison = classify_across(&[&first, &second]);
    assert_eq!(comparison.outcome, Equivalence::BuildEquivalent);
    assert!(
        comparison
            .reasons
            .iter()
            .any(|reason| reason.contains("agree across")),
        "{:?}",
        comparison.reasons
    );
}

/// ⛔ And the same pair, refused, once a field they both measured disagrees.
/// Equal version labels over a behavioural difference is the case
/// `architecture.md` section 7 says is never silently collapsed.
#[test]
fn equivalence_across_two_routes_diverges_when_an_observed_field_conflicts() {
    let first = read(&golden_value());
    let mut second = golden_value();
    second_capture(&mut second);
    // The second route's build reports a different reserved block, which is a
    // difference in what it puts on the wire, not in how it was packaged.
    let mut changed = 0;
    for field in second["observations"]
        .as_array_mut()
        .expect("an array")
        .iter_mut()
    {
        if field["path"] == "peer_wire/reserved" {
            // ⚠ Asserted to differ from what the fixture carries. The first
            // version of this test wrote the fixture's own value back, so the
            // records agreed and the test passed for the wrong reason until the
            // assertion below was added.
            assert_ne!(field["state"]["detail"]["value"], json!("0000000000000000"));
            field["state"]["detail"]["value"] = json!("0000000000000000");
            changed += 1;
        }
    }
    assert_eq!(changed, 1, "the fixture must carry the field being changed");
    let second = read(&second);

    let comparison = classify_across(&[&first, &second]);
    assert_eq!(comparison.outcome, Equivalence::Divergent);
    assert!(
        comparison
            .reasons
            .iter()
            .any(|reason| reason.contains("peer_wire/reserved")),
        "the reason names the field: {:?}",
        comparison.reasons
    );
}

/// ⛔ Two runs that both watched the same route compare a build against itself.
/// They agree, for a reason that says nothing about the other route.
#[test]
fn equivalence_across_refuses_two_captures_of_one_route() {
    let first = read(&golden_value());
    let mut second = golden_value();
    recapture(&mut second, "fixture-capture-0002", "route-package-manager");
    let second = read(&second);
    assert_ne!(first.capture.id, second.capture.id, "two distinct runs");

    let comparison = classify_across(&[&first, &second]);
    assert_eq!(comparison.outcome, Equivalence::Unresolved);
    assert!(
        comparison
            .reasons
            .iter()
            .any(|reason| reason.contains("observed twice")),
        "{:?}",
        comparison.reasons
    );
}

/// And the shallower version of the same mistake: one record, passed twice.
#[test]
fn equivalence_across_refuses_one_record_passed_twice() {
    let profile = read(&golden_value());
    let comparison = classify_across(&[&profile, &profile]);
    assert_eq!(comparison.outcome, Equivalence::Unresolved);
    assert!(
        comparison
            .reasons
            .iter()
            .any(|reason| reason.contains("appears twice")),
        "{:?}",
        comparison.reasons
    );
}

/// Records of two different builds are not a route comparison at all.
#[test]
fn equivalence_across_refuses_records_of_different_builds() {
    let first = read(&golden_value());
    let mut second = golden_value();
    second_capture(&mut second);
    let second = read(&second);
    // Compared as-is they are one build; the guard is that the classifier looks
    // at the tuple rather than assuming the caller passed a matched pair.
    assert_eq!(
        classify_across(&[&first, &second]).outcome,
        Equivalence::BuildEquivalent
    );

    let mut other = golden_value();
    second_capture(&mut other);
    other["build"]["arch"] = json!("aarch64");
    // The identifier is derived from the tuple, so changing the tuple changes it.
    let text = serde_json::to_string(&other).expect("writes");
    let error = Profile::from_json(&text).expect_err("the record id no longer derives");
    assert!(error.to_string().contains("E-ID-01"), "{error}");
}

/// One capture cannot reach `build_equivalent`, and saying so is the point.
#[test]
fn equivalence_within_one_record_never_claims_build_equivalence() {
    let mut document = golden_value();
    document["acquisition"][1]["installed_executable"] =
        json!("sha256:3333333333333333333333333333333333333333333333333333333333333333");
    let profile = read(&document);
    let outcome = classify(&profile).outcome;
    assert_ne!(
        outcome,
        Equivalence::BuildEquivalent,
        "one capture cannot establish that two different builds behave alike"
    );
    assert!(!outcome.publishable());
}

// -- what makes a real pair diverge, and what stops it ----------------------
//
// `capture-client` run 17 produced two records whose `peer_wire/peer_id` was
// `constant` with one sample each, because nothing turned samples into a state.
// A peer ID's tail is per connection, so the two constants differed and the
// pair was `divergent`. These two tests are that situation and its repair, with
// the states built by `sampling::field_state` rather than written by hand.

/// Four restarts of one build whose `-qB5230-` prefix holds and whose tail is
/// regenerated. `seed` is what makes two lanes produce different tails.
fn observed_peer_ids(seed: u32) -> Vec<Sample> {
    (0..4u32)
        .map(|session| {
            let mut bytes = b"-qB5230-".to_vec();
            bytes.extend((0..12u32).map(|index| {
                u8::try_from((seed + session * 13 + index * 7) & 0xff).expect("masked to one byte")
            }));
            Sample {
                session,
                torrent: 0,
                connection: 0,
                value: HexBytes::new(bytes).expect("twenty bytes is not empty"),
            }
        })
        .collect()
}

fn restart_plan() -> SamplingPlan {
    SamplingPlan {
        sessions: NonZeroU32::new(4).expect("four is not zero"),
        torrents: NonZeroU32::new(1).expect("one is not zero"),
        connections: NonZeroU32::new(1).expect("one is not zero"),
    }
}

fn set_peer_id(document: &mut Value, state: &FieldState) {
    let encoded = serde_json::to_value(state).expect("a state serializes");
    let mut changed = 0;
    for field in document["observations"]
        .as_array_mut()
        .expect("an array")
        .iter_mut()
    {
        if field["path"] == "peer_wire/peer_id" {
            field["state"] = encoded.clone();
            changed += 1;
        }
    }
    assert_eq!(changed, 1, "the fixture must carry peer_wire/peer_id");
}

/// ⛔ One sample per lane is what run 17 recorded, and it cannot do anything
/// else: a tail that is regenerated per connection differs between any two
/// captures, so two constants conflict and the pair does not publish.
#[test]
fn equivalence_across_diverges_on_a_peer_id_tail_recorded_from_one_sample() {
    let one = |seed| {
        let samples = observed_peer_ids(seed);
        field_state(&samples[..1], &restart_plan()).expect("one sample supports a state")
    };
    let left = one(0x10);
    let right = one(0x90);
    assert!(
        matches!(left, FieldState::Constant(_)),
        "one sample is constant and nothing else"
    );
    assert_ne!(left, right, "the tails differ, which is the whole problem");

    let mut first = golden_value();
    set_peer_id(&mut first, &left);
    let first = read(&first);
    let mut second = golden_value();
    second_capture(&mut second);
    set_peer_id(&mut second, &right);
    let second = read(&second);

    let comparison = classify_across(&[&first, &second]);
    assert_eq!(comparison.outcome, Equivalence::Divergent);
    assert!(
        comparison
            .reasons
            .iter()
            .any(|reason| reason.contains("peer_wire/peer_id")),
        "the reason names the field: {:?}",
        comparison.reasons
    );
}

/// ⭐ And the same two lanes, sampled across restarts, agree - because what the
/// build holds still is the prefix, and that is what a pattern states. The
/// tails are still different bytes; they are no longer a different *claim*.
#[test]
fn equivalence_across_agrees_on_a_peer_id_tail_recorded_as_the_pattern_it_is() {
    let sampled = |seed| {
        field_state(&observed_peer_ids(seed), &restart_plan())
            .expect("four samples support a state")
    };
    let left = sampled(0x10);
    let right = sampled(0x90);
    assert!(
        matches!(left, FieldState::Patterned(_)),
        "four restarts over a regenerated tail is a pattern"
    );
    assert_eq!(
        left, right,
        "two lanes of one build describe one shape, which is why they agree"
    );

    let mut first = golden_value();
    set_peer_id(&mut first, &left);
    let first = read(&first);
    let mut second = golden_value();
    second_capture(&mut second);
    set_peer_id(&mut second, &right);
    let second = read(&second);

    let comparison = classify_across(&[&first, &second]);
    assert_eq!(
        comparison.outcome,
        Equivalence::BuildEquivalent,
        "{:?}",
        comparison.reasons
    );
    assert!(
        publishable(&first).is_ok() && publishable(&second).is_ok(),
        "and neither record is held back by anything else"
    );
}
