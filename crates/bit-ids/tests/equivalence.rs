//! `ACQ-03`: two routes accepted when they really are one build, and refused
//! when equal version labels stand over evidence that conflicts.
//!
//! ⛔ Equal labels are the question, not the answer. `E-ACQ-04` already forces
//! every route to report the version the record declares, so a test that only
//! checked labels would pass over every case this entry exists for.

use core::num::NonZeroU32;

use bit_ids::agreement::publishable_among;
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
    rederive_id(document);
}

/// Re-derives the record identifier from the identity tuple the document now
/// carries. ⚠ Separate from [`recapture`] because a byte-different pair sets the
/// installs itself, and a helper that also rewrote them would undo it.
fn rederive_id(document: &mut Value) {
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

// -- the pair the per-record gate cannot see --------------------------------
//
// ⛔ `classify` reaches `byte_identical` or `unresolved` and never
// `build_equivalent`, so a record whose routes installed different bytes was
// refused by `E-PUB-04` however many captures existed. The second capture
// through the other route is a different record, and `publishable_among` is the
// same gate asked where that record is visible.

/// What the second route delivers, which is not what the first one does.
const OTHER_BYTES: &str = "sha256:4444444444444444444444444444444444444444444444444444444444444444";

/// The golden record with the two routes installing different bytes, which is
/// every release-against-source pair there is, observed through `observed`.
///
/// ⛔ `E-CAP-06` binds `build.executable` to what the **observed** route
/// installed, so the two records of such a pair legitimately declare different
/// builds. That is the shape, not a workaround: each record describes the bytes
/// its own capture put on the wire.
fn byte_different(capture: &str, observed: &str) -> Value {
    let mut document = golden_value();
    document["acquisition"][1]["installed_executable"] = json!(OTHER_BYTES);
    document["capture"]["id"] = json!(capture);
    document["capture"]["observed_route"] = json!(observed);
    let installed = document["acquisition"]
        .as_array()
        .expect("an array")
        .iter()
        .find(|route| route["id"] == observed)
        .expect("the record carries the route it says it watched")["installed_executable"]
        .clone();
    document["build"]["executable"] = installed;
    rederive_id(&mut document);
    assert_ne!(
        document["acquisition"][0]["installed_executable"],
        document["acquisition"][1]["installed_executable"],
        "the whole point of this helper is that the routes differ"
    );
    document
}

fn release_lane() -> Value {
    byte_different("fixture-capture-0001", "route-package-manager")
}

fn source_lane() -> Value {
    byte_different("fixture-capture-0002", "route-vendor-release")
}

#[test]
fn equivalence_among_settles_a_pair_the_per_record_gate_cannot() {
    let first = read(&release_lane());
    let second = read(&source_lane());

    // The record alone is refused, and it is refused for a reason no amount of
    // capturing this record could address.
    let alone = publishable(&first).expect_err("one record cannot settle a pair");
    assert!(alone.has("E-PUB-04"), "{alone}");
    assert_eq!(
        classify(&first).outcome,
        Equivalence::Unresolved,
        "which is what the per-record classifier can say and no more"
    );

    // With the other route's capture in the store, the pair settles it.
    assert_eq!(
        classify_across(&[&first, &second]).outcome,
        Equivalence::BuildEquivalent
    );
    publishable_among(&first, &[&second]).expect("the sibling capture settles the routes");
    publishable_among(&second, &[&first]).expect("and it settles them both ways");
}

#[test]
fn equivalence_among_still_refuses_when_nothing_in_the_store_is_a_pair() {
    let first = read(&release_lane());

    // A record of a different build says nothing about this one. ⚠ `E-ACQ-04`
    // makes every route report the version the record declares, so moving the
    // version means moving it everywhere; a record with one of them changed is
    // refused before it can be compared.
    let mut other = source_lane();
    other["build"]["version"] = json!("9.9.9");
    for route in other["acquisition"].as_array_mut().expect("an array") {
        route["installed_version"] = json!("9.9.9");
    }
    rederive_id(&mut other);
    let other = read(&other);
    assert_eq!(
        classify_across(&[&first, &other]).outcome,
        Equivalence::Unresolved,
        "not a pair, which is what makes this the no-sibling case"
    );

    let refused =
        publishable_among(&first, &[&other]).expect_err("nothing here settles the routes");
    assert!(refused.has("E-PUB-04"), "{refused}");
    assert!(
        refused
            .errors()
            .iter()
            .any(|error| error.detail().contains("no capture of another route")),
        "the message says the store held no pair: {refused}"
    );
}

#[test]
fn equivalence_among_refuses_a_record_whose_sibling_disagrees() {
    let first = read(&release_lane());
    let mut second = source_lane();
    let mut changed = 0;
    for field in second["observations"]
        .as_array_mut()
        .expect("an array")
        .iter_mut()
    {
        if field["path"] == "peer_wire/reserved" {
            assert_ne!(field["state"]["detail"]["value"], json!("0000000000000000"));
            field["state"]["detail"]["value"] = json!("0000000000000000");
            changed += 1;
        }
    }
    assert_eq!(changed, 1, "the fixture must carry the field being changed");
    let second = read(&second);

    let refused = publishable_among(&first, &[&second]).expect_err("the two captures disagree");
    assert!(refused.has("E-PUB-03"), "{refused}");
    assert!(
        !refused.has("E-PUB-04"),
        "a conflict is a different finding from an absence of evidence: {refused}"
    );
}

/// ⛔ The rule that a first-match search would get wrong. One sibling agrees and
/// one conflicts, so the store holds evidence against this record; publishing on
/// the agreeable half would bury it.
#[test]
fn equivalence_among_refuses_when_one_sibling_agrees_and_another_conflicts() {
    let first = read(&release_lane());
    let agrees = read(&source_lane());

    let mut conflicts = byte_different("fixture-capture-0003", "route-vendor-release");
    let mut changed = 0;
    for field in conflicts["observations"]
        .as_array_mut()
        .expect("an array")
        .iter_mut()
    {
        if field["path"] == "peer_wire/reserved" {
            field["state"]["detail"]["value"] = json!("0000000000000000");
            changed += 1;
        }
    }
    assert_eq!(changed, 1);
    let conflicts = read(&conflicts);

    assert_eq!(
        classify_across(&[&first, &agrees]).outcome,
        Equivalence::BuildEquivalent,
        "the first sibling really does agree, or this tests nothing"
    );
    let refused = publishable_among(&first, &[&agrees, &conflicts])
        .expect_err("a conflict anywhere refuses the record");
    assert!(refused.has("E-PUB-03"), "{refused}");

    // And the order of the store must not decide the verdict.
    let reversed = publishable_among(&first, &[&conflicts, &agrees])
        .expect_err("whichever order the store is read in");
    assert!(reversed.has("E-PUB-03"), "{reversed}");
}

#[test]
fn equivalence_among_leaves_a_byte_identical_record_exactly_as_it_was() {
    let profile = read(&golden_value());
    assert_eq!(classify(&profile).outcome, Equivalence::ByteIdentical);
    publishable(&profile).expect("it published before");
    publishable_among(&profile, &[]).expect("and an empty store changes nothing");
    let other = read(&source_lane());
    publishable_among(&profile, &[&other]).expect("nor does an unrelated record");
}
