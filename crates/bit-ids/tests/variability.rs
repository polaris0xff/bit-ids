//! Acceptance for `SCHEMA-04`.
//!
//! ```text
//! cargo test --workspace variability
//! ```
//!
//! Every test name carries `variability` so that command selects this file.
//!
//! The classifications here are a function of the samples, never a judgement
//! about them. A dimension the run did not vary produces `unknown`, and one
//! sample produces `unknown` for everything, because a value that looked the
//! same twice inside one process is not a value shown to survive a restart.

use core::num::NonZeroU32;

use bit_ids::canonical::HexBytes;
use bit_ids::observation::{FieldState, PatternRun};
use bit_ids::sampling::{Lifetime, Sample, SamplingPlan, classify, field_state};

fn plan(sessions: u32, torrents: u32, connections: u32) -> SamplingPlan {
    SamplingPlan {
        sessions: NonZeroU32::new(sessions).expect("a plan exercises at least one"),
        torrents: NonZeroU32::new(torrents).expect("a plan exercises at least one"),
        connections: NonZeroU32::new(connections).expect("a plan exercises at least one"),
    }
}

fn sample(session: u32, torrent: u32, connection: u32, hex: &str) -> Sample {
    Sample {
        session,
        torrent,
        connection,
        value: HexBytes::parse(hex).expect("a canonical byte string"),
    }
}

/// A 20-byte peer ID: the eight-byte client prefix `-XX0000-`, then the twelve
/// bytes the caller supplies as 24 hex digits.
fn peer_id(suffix: &str) -> String {
    assert_eq!(
        suffix.len(),
        24,
        "a peer ID is 20 bytes, so the suffix is 12 of them"
    );
    format!("2d5858303030302d{suffix}")
}

// -- the shape a peer ID actually has --------------------------------------

#[test]
fn variability_separates_a_fixed_prefix_from_a_changing_suffix() {
    let plan = plan(4, 1, 1);
    let samples: Vec<Sample> = (0..4)
        .map(|session| {
            sample(
                session,
                0,
                0,
                &peer_id(&format!("aabbccddeeff0011223344{session:02x}")),
            )
        })
        .collect();

    let report = classify(&samples, &plan).expect("four samples classify");
    assert_eq!(report.length, Some(20));
    assert_eq!(report.samples, 4);

    // The eight-byte client prefix never moved, across four separate process
    // starts, so it is a stored value rather than one that happened to repeat.
    for offset in 0..8 {
        assert_eq!(
            report.span_at(offset).map(|span| span.lifetime),
            Some(Lifetime::Persistent),
            "byte {offset} is part of the fixed prefix"
        );
    }
    // The last byte is the one that moved, and it moved with the session.
    assert_eq!(
        report.span_at(19).map(|span| span.lifetime),
        Some(Lifetime::PerSession)
    );
    assert!(report.any_variation());
}

#[test]
fn variability_reports_adjacent_bytes_that_behaved_alike_as_one_span() {
    let plan = plan(2, 1, 1);
    let samples = vec![
        sample(0, 0, 0, &peer_id("111111110000000000000000")),
        sample(1, 0, 0, &peer_id("222222220000000000000000")),
    ];
    let report = classify(&samples, &plan).expect("two samples classify");
    let spans: Vec<(usize, usize, Lifetime)> = report
        .spans
        .iter()
        .map(|span| (span.offset, span.length, span.lifetime))
        .collect();
    assert_eq!(
        spans,
        vec![
            (0, 8, Lifetime::Persistent),
            (8, 4, Lifetime::PerSession),
            (12, 8, Lifetime::Persistent),
        ],
        "the value is three runs, not twenty independent bytes"
    );
}

// -- one sample proves nothing about a lifetime ----------------------------

#[test]
fn variability_refuses_to_call_anything_stable_from_one_sample() {
    let plan = plan(1, 1, 1);
    let samples = vec![sample(0, 0, 0, &peer_id("aabbccddeeff001122334455"))];
    let report = classify(&samples, &plan).expect("one sample still reports");
    assert_eq!(report.samples, 1);
    for span in &report.spans {
        assert_eq!(
            span.lifetime,
            Lifetime::Unknown,
            "one observation establishes bytes for one connection and nothing more"
        );
    }
    assert!(!report.any_variation());
}

#[test]
fn variability_refuses_persistence_when_the_run_never_restarted() {
    // Four identical samples, all from one process. Nothing distinguishes a
    // value stored on disk from one generated once per process start.
    let plan = plan(1, 2, 2);
    let samples: Vec<Sample> = (0..2)
        .flat_map(|torrent| {
            (0..2).map(move |connection| {
                sample(0, torrent, connection, &peer_id("aabbccddeeff001122334455"))
            })
        })
        .collect();
    let report = classify(&samples, &plan).expect("four samples classify");
    assert_eq!(report.samples, 4);
    for span in &report.spans {
        assert_eq!(
            span.lifetime,
            Lifetime::Unknown,
            "a value that never changed inside one process is not a persistent value"
        );
    }
}

#[test]
fn variability_calls_it_persistent_only_once_a_restart_has_been_exercised() {
    let plan = plan(2, 1, 1);
    let samples = vec![
        sample(0, 0, 0, &peer_id("aabbccddeeff001122334455")),
        sample(1, 0, 0, &peer_id("aabbccddeeff001122334455")),
    ];
    let report = classify(&samples, &plan).expect("two samples classify");
    for span in &report.spans {
        assert_eq!(span.lifetime, Lifetime::Persistent);
    }
}

// -- each dimension is separated by the one that varied --------------------

#[test]
fn variability_names_a_value_that_changes_per_connection() {
    let plan = plan(1, 1, 3);
    let samples: Vec<Sample> = (0..3)
        .map(|connection| {
            sample(
                0,
                0,
                connection,
                &peer_id(&format!("aabbccddeeff0011223344{connection:02x}")),
            )
        })
        .collect();
    let report = classify(&samples, &plan).expect("three samples classify");
    assert_eq!(
        report.span_at(19).map(|span| span.lifetime),
        Some(Lifetime::PerConnection),
        "it changed between connections inside one session and one torrent"
    );
}

#[test]
fn variability_names_a_value_that_changes_per_torrent() {
    let plan = plan(1, 3, 2);
    let samples: Vec<Sample> = (0..3)
        .flat_map(|torrent| {
            (0..2).map(move |connection| {
                sample(
                    0,
                    torrent,
                    connection,
                    &peer_id(&format!("aabbccddeeff0011223344{torrent:02x}")),
                )
            })
        })
        .collect();
    let report = classify(&samples, &plan).expect("six samples classify");
    assert_eq!(
        report.span_at(19).map(|span| span.lifetime),
        Some(Lifetime::PerTorrent),
        "it held across connections and moved with the torrent"
    );
}

#[test]
fn variability_names_a_value_that_changes_per_session() {
    let plan = plan(3, 2, 1);
    let samples: Vec<Sample> = (0..3)
        .flat_map(|session| {
            (0..2).map(move |torrent| {
                sample(
                    session,
                    torrent,
                    0,
                    &peer_id(&format!("aabbccddeeff0011223344{session:02x}")),
                )
            })
        })
        .collect();
    let report = classify(&samples, &plan).expect("six samples classify");
    assert_eq!(
        report.span_at(19).map(|span| span.lifetime),
        Some(Lifetime::PerSession),
        "it held across torrents and moved with the process"
    );
}

#[test]
fn variability_will_not_name_a_dimension_the_run_did_not_vary() {
    // The value changes with the session, but the plan only ran one session, so
    // whatever produced the difference, this run did not separate it.
    let plan = plan(1, 1, 2);
    let samples = vec![
        sample(0, 0, 0, &peer_id("aabbccddeeff001122334455")),
        sample(0, 0, 1, &peer_id("aabbccddeeff001122334466")),
    ];
    let report = classify(&samples, &plan).expect("two samples classify");
    assert_eq!(
        report.span_at(19).map(|span| span.lifetime),
        Some(Lifetime::PerConnection)
    );
    // And the bytes that held still cannot be called persistent.
    assert_eq!(
        report.span_at(0).map(|span| span.lifetime),
        Some(Lifetime::Unknown)
    );
}

// -- edges -----------------------------------------------------------------

#[test]
fn variability_reports_a_changing_width_rather_than_guessing_at_offsets() {
    let plan = plan(2, 1, 1);
    let samples = vec![sample(0, 0, 0, "aabbcc"), sample(1, 0, 0, "aabbccdd")];
    let report = classify(&samples, &plan).expect("two samples classify");
    assert_eq!(
        report.length, None,
        "offsets do not line up, so nothing can be said per offset"
    );
    assert!(report.spans.is_empty());
    assert_eq!(report.samples, 2);
}

#[test]
fn variability_has_nothing_to_report_with_no_samples_at_all() {
    assert!(classify(&[], &plan(4, 2, 2)).is_none());
}

#[test]
fn variability_counts_what_a_plan_can_produce() {
    assert_eq!(plan(4, 2, 3).observations(), 24);
    assert_eq!(plan(1, 1, 1).observations(), 1);
    assert!(!plan(1, 1, 1).varies_anything());
    assert!(!plan(1, 1, 1).restarts());
    assert!(plan(1, 2, 1).varies_anything());
    assert!(
        !plan(1, 2, 1).restarts(),
        "varying torrents is not restarting the process"
    );
    assert!(plan(2, 1, 1).restarts());
}

// -- the state a record carries, derived from those samples ----------------
//
// `field_state` is the join this entry was closed without: the classifier said
// what the samples prove and nothing turned that into the `FieldState` a record
// holds. These assert the mapping, and the one that matters most is the third,
// which is the case where reading the lifetime rather than the bytes would
// report a value that changed as fixed.

fn runs_of(state: &FieldState) -> Vec<(&'static str, usize)> {
    let FieldState::Patterned(patterned) = state else {
        panic!("expected a patterned state, got {}", state.as_str());
    };
    patterned
        .pattern
        .runs
        .iter()
        .map(|run| match run {
            PatternRun::Fixed { bytes } => ("fixed", bytes.len()),
            PatternRun::Varying { length, .. } => ("varying", *length),
        })
        .collect()
}

/// Four restarts of a build whose eight-byte prefix holds and whose twelve-byte
/// tail is regenerated, which is the shape a real peer ID has.
fn restarted_peer_ids() -> Vec<Sample> {
    (0..4u32)
        .map(|session| {
            let mut bytes = b"-XX0000-".to_vec();
            bytes.extend((0..12u32).map(|index| {
                u8::try_from((session * 13 + index * 7 + 1) & 0xff).expect("masked to one byte")
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

#[test]
fn variability_turns_a_restarted_peer_id_into_the_patterned_state_a_record_carries() {
    let state =
        field_state(&restarted_peer_ids(), &plan(4, 1, 1)).expect("four samples support a state");
    assert_eq!(
        runs_of(&state),
        vec![("fixed", 8), ("varying", 12)],
        "the client prefix is the identifying half and it is kept"
    );
    let FieldState::Patterned(patterned) = &state else {
        unreachable!("runs_of already refused anything else");
    };
    assert_eq!(patterned.pattern.length, 20);
    assert_eq!(patterned.samples.get(), 4);
}

#[test]
fn variability_reports_a_value_that_never_moved_as_constant() {
    let plan = plan(3, 1, 1);
    let samples: Vec<Sample> = (0..3)
        .map(|session| sample(session, 0, 0, &peer_id("aabbccddeeff001122334455")))
        .collect();
    let state = field_state(&samples, &plan).expect("three samples support a state");
    let FieldState::Constant(constant) = &state else {
        panic!("expected constant, got {}", state.as_str());
    };
    assert_eq!(constant.samples.get(), 3);
    assert!(
        !state.claims_variation(),
        "nothing moved, so nothing may claim it did"
    );
}

#[test]
fn variability_will_not_call_a_span_fixed_because_its_lifetime_does_not_vary() {
    // ⛔ `Lifetime::Unknown` covers two facts: a value that never changed, and a
    // value that changed where no dimension the plan varied separates the
    // change. A state derived from the lifetime would report the second as
    // fixed bytes. These samples are labelled across sessions under a plan that
    // declares one, which is the combination that reaches it.
    let plan = plan(1, 2, 1);
    let samples = vec![
        sample(0, 0, 0, &peer_id("aabbccddeeff001122334455")),
        sample(1, 0, 0, &peer_id("aabbccddeeff00112233ffff")),
    ];
    let offset = 18;
    let report = classify(&samples, &plan).expect("two samples classify");
    assert_eq!(
        report.span_at(offset).map(|span| span.lifetime),
        Some(Lifetime::Unknown),
        "this is the case the guard exists for; if it stops being unknown the \
         test is no longer testing anything"
    );

    let state = field_state(&samples, &plan).expect("two samples support a state");
    assert_eq!(
        runs_of(&state),
        vec![("fixed", 18), ("varying", 2)],
        "the bytes differ over those two, whatever the lifetime could not say"
    );
}

#[test]
fn variability_merges_touching_spans_the_record_cannot_tell_apart() {
    // `classify` splits on lifetime and a record's pattern splits on bytes, so
    // a per-connection byte beside a per-session one is two spans there and one
    // varying run here. Two shapes for one measurement would be two records.
    let plan = plan(2, 1, 2);
    let samples: Vec<Sample> = [(0, 0), (0, 1), (1, 0), (1, 1)]
        .into_iter()
        .map(|(session, connection): (u32, u32)| {
            let tail = format!("{connection:02x}{session:02x}");
            sample(
                session,
                0,
                connection,
                &peer_id(&format!("aabbccddeeff00112233{tail}")),
            )
        })
        .collect();
    let report = classify(&samples, &plan).expect("four samples classify");
    assert_eq!(
        report.span_at(18).map(|span| span.lifetime),
        Some(Lifetime::PerConnection)
    );
    assert_eq!(
        report.span_at(19).map(|span| span.lifetime),
        Some(Lifetime::PerSession),
        "two different lifetimes, which is what makes them two spans"
    );

    let state = field_state(&samples, &plan).expect("four samples support a state");
    assert_eq!(runs_of(&state), vec![("fixed", 18), ("varying", 2)]);
}

#[test]
fn variability_reports_a_value_whose_every_byte_moved_as_variable() {
    // ⛔ A pattern needs a fixed run and a varying one, so a value with no
    // stable span is a variable rather than a pattern of one run. `E-OBS-10`
    // refuses the other spelling.
    let plan = plan(2, 1, 1);
    let samples = vec![sample(0, 0, 0, "aabbcc"), sample(1, 0, 0, "ddeeff")];
    let state = field_state(&samples, &plan).expect("two samples support a state");
    let FieldState::Variable(variable) = &state else {
        panic!("expected variable, got {}", state.as_str());
    };
    assert_eq!(variable.length, Some(3));
    assert_eq!(variable.samples.get(), 2);
    assert_eq!(variable.distinct.get(), 2);
}

#[test]
fn variability_reports_a_changing_width_as_a_variable_with_no_length() {
    let plan = plan(2, 1, 1);
    let samples = vec![sample(0, 0, 0, "aabbcc"), sample(1, 0, 0, "aabbccdd")];
    let state = field_state(&samples, &plan).expect("two samples support a state");
    let FieldState::Variable(variable) = &state else {
        panic!("expected variable, got {}", state.as_str());
    };
    assert_eq!(
        variable.length, None,
        "the width itself moved, so there is no common length to state"
    );
    assert_eq!(variable.distinct.get(), 2);
}

#[test]
fn variability_supports_no_state_at_all_over_no_samples() {
    assert!(
        field_state(&[], &plan(4, 2, 2)).is_none(),
        "a state over nothing is a measurement nobody took"
    );
}

#[test]
fn variability_states_the_bytes_even_when_the_plan_varied_nothing() {
    // ⚠ The plan is the caller's claim and the samples are the measurement, so
    // this reports what it saw. The pair is contradictory and `E-BND-20` is
    // what refuses it, against the run manifest - not this function, which
    // would otherwise be a second gate answering a different way.
    let plan = plan(1, 1, 1);
    let samples = vec![sample(0, 0, 0, "aabbcc"), sample(0, 0, 0, "aabbdd")];
    let state = field_state(&samples, &plan).expect("two samples support a state");
    assert!(
        state.claims_variation(),
        "the bytes moved; calling that constant would publish a value no sample carried"
    );
}
