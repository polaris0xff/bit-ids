//! The five candidate sets `ACQ-02` names, and a plant for every refusal.
//!
//! ⛔ The sets are the point. A resolver tested only on a tidy list of stable
//! semantic versions is tested on the one input that cannot distinguish it from
//! a lexical sort.

use bit_ids::canonical::{Instant, Label, Sha256Digest, Slug, Url, Version};
use bit_ids::resolution::{
    Candidate, Considered, RESOLUTION_SCHEMA, Resolution, SourceResponse, Verdict, VersionScheme,
    resolve, validate_resolution,
};
use serde_json::{Value, json};

fn slug(text: &str) -> Slug {
    Slug::parse(text).expect("slug")
}

fn label(text: &str) -> Label {
    Label::parse(text).expect("label")
}

fn instant(text: &str) -> Instant {
    Instant::parse(text).expect("instant")
}

fn source(id: &str) -> SourceResponse {
    SourceResponse {
        id: slug(id),
        url: Url::parse("https://example.invalid/releases").expect("url"),
        retrieved_at: instant("2026-09-04T12:00:00Z"),
        digest: Sha256Digest::of(id.as_bytes()),
        candidates: 0,
    }
}

fn candidate(source: &str, tag: &str, prerelease: bool, draft: bool) -> Candidate {
    Candidate {
        source: slug(source),
        tag: label(tag),
        prerelease,
        draft,
        published_at: Some(instant("2026-07-07T21:52:42Z")),
    }
}

/// The tag shape qBittorrent publishes.
fn prefixed_scheme() -> VersionScheme {
    VersionScheme {
        tag_prefix: Some(label("release-")),
        min_components: 3,
        max_components: 4,
    }
}

/// The tag shape Transmission publishes.
fn bare_scheme() -> VersionScheme {
    VersionScheme {
        tag_prefix: None,
        min_components: 3,
        max_components: 3,
    }
}

fn run(scheme: VersionScheme, candidates: Vec<Candidate>) -> Resolution {
    let mut sources: Vec<SourceResponse> = Vec::new();
    for candidate in &candidates {
        if !sources.iter().any(|s| s.id == candidate.source) {
            sources.push(source(candidate.source.as_str()));
        }
    }
    for entry in &mut sources {
        entry.candidates =
            u32::try_from(candidates.iter().filter(|c| c.source == entry.id).count())
                .expect("small");
    }
    sources.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));
    resolve(
        slug("fixture-client"),
        instant("2026-09-04T13:00:00Z"),
        scheme,
        sources,
        candidates,
    )
}

fn verdicts(resolution: &Resolution) -> Vec<(&str, Verdict)> {
    resolution
        .considered
        .iter()
        .map(|entry| (entry.candidate.tag.as_str(), entry.verdict))
        .collect()
}

/// Set one: an ordinary stable list. ⛔ Includes `.10` against `.9`, which is
/// the pair a lexical sort gets backwards and a tidy list never exercises.
#[test]
fn resolution_selects_the_newest_stable_release() {
    let found = run(
        prefixed_scheme(),
        vec![
            candidate("github-releases", "release-5.2.9", false, false),
            candidate("github-releases", "release-5.2.10", false, false),
            candidate("github-releases", "release-5.2.3", false, false),
        ],
    );
    assert_eq!(
        found.selected.as_ref().map(Version::as_str),
        Some("5.2.10"),
        "5.2.10 is newer than 5.2.9 and text order says otherwise"
    );
    assert_eq!(found.with_verdict(Verdict::Selected).len(), 1);
    assert_eq!(found.with_verdict(Verdict::Superseded).len(), 2);
    validate_resolution(&found).expect("a resolved decision validates");
}

/// Set two: prereleases, flagged and unflagged. Both signals are believed, and
/// a project that forgets the flag is still not selected.
#[test]
fn resolution_refuses_a_prerelease_by_either_signal() {
    let found = run(
        bare_scheme(),
        vec![
            candidate("github-releases", "4.1.3", false, false),
            candidate("github-releases", "4.2.0-beta.5", true, false),
            // ⚠ The dangerous one: a beta the source forgot to flag. Trusting
            // the label alone selects it and publishes a preview as stable.
            candidate("github-releases", "4.3.0-rc1", false, false),
            candidate("github-releases", "4.4.0", false, true),
        ],
    );
    assert_eq!(found.selected.as_ref().map(Version::as_str), Some("4.1.3"));
    assert_eq!(
        verdicts(&found),
        vec![
            ("4.1.3", Verdict::Selected),
            ("4.2.0-beta.5", Verdict::PrereleaseByLabel),
            ("4.3.0-rc1", Verdict::PrereleaseByVersion),
            ("4.4.0", Verdict::PrereleaseByLabel),
        ]
    );
    validate_resolution(&found).expect("validates");
}

/// Set three: nothing to select. An empty list and a list of only prereleases
/// both fail closed rather than answering.
#[test]
fn resolution_fails_closed_when_no_stable_release_exists() {
    // ⚠ A source that answered with nothing, not "no source was asked". The
    // second is a different failure and `E-RES-01` is what reports it.
    let empty = resolve(
        slug("fixture-client"),
        instant("2026-09-04T13:00:00Z"),
        bare_scheme(),
        vec![SourceResponse {
            candidates: 0,
            ..source("github-releases")
        }],
        Vec::new(),
    );
    assert!(!empty.resolved(), "nothing offered, nothing selected");
    validate_resolution(&empty).expect("failing closed is a valid document");

    let all_pre = run(
        bare_scheme(),
        vec![
            candidate("github-releases", "4.1.0-beta.1", true, false),
            candidate("github-releases", "4.1.0-beta.2", true, false),
        ],
    );
    assert!(!all_pre.resolved());
    assert_eq!(all_pre.with_verdict(Verdict::PrereleaseByLabel).len(), 2);
}

/// Set four: two sources that disagree about the newest stable release.
#[test]
fn resolution_fails_closed_when_two_sources_disagree() {
    let found = run(
        bare_scheme(),
        vec![
            candidate("github-releases", "4.1.3", false, false),
            candidate("distro-index", "4.1.2", false, false),
        ],
    );
    assert!(
        !found.resolved(),
        "a disagreement is not a tie to break by picking the larger"
    );
    assert_eq!(found.with_verdict(Verdict::Divergent).len(), 2);
    validate_resolution(&found).expect("validates");
}

/// ⭐ Set five, and the one that matters most: a tag nobody can order.
///
/// Skipping it silently produces an older version selected confidently, with
/// nothing saying a newer one was seen and not understood.
#[test]
fn resolution_fails_closed_on_a_version_it_cannot_order() {
    let found = run(
        bare_scheme(),
        vec![
            candidate("github-releases", "4.1.3", false, false),
            candidate("github-releases", "2026.spring", false, false),
        ],
    );
    assert!(
        !found.resolved(),
        "an unorderable candidate blocks rather than being skipped"
    );
    assert_eq!(found.with_verdict(Verdict::Unorderable).len(), 1);
    assert_eq!(found.with_verdict(Verdict::Superseded).len(), 1);
    validate_resolution(&found).expect("validates");
}

/// ⭐ An unorderable tag published before the winner cannot be the newest, so
/// publication order settles it without guessing.
///
/// This is what a live dry run made necessary: `transmission` publishes 51
/// two-component tags from a decade ago that the current scheme cannot read, and
/// refusing over them made the resolver correct and useless.
#[test]
fn resolution_lets_publication_order_settle_an_unorderable_older_tag() {
    let newest = Candidate {
        published_at: Some(instant("2026-06-30T03:24:03Z")),
        ..candidate("github-releases", "4.1.3", false, false)
    };
    let ancient = Candidate {
        published_at: Some(instant("2016-05-11T00:00:00Z")),
        ..candidate("github-releases", "2.92", false, false)
    };
    let found = run(bare_scheme(), vec![newest, ancient]);
    assert_eq!(found.selected.as_ref().map(Version::as_str), Some("4.1.3"));
    assert_eq!(found.with_verdict(Verdict::PredatesSelection).len(), 1);
    validate_resolution(&found).expect("validates");
}

/// ⛔ And it only settles it when there is a date to settle it with. An
/// unorderable candidate with no publication date still blocks, because nothing
/// then rules out that it is newer.
#[test]
fn resolution_still_fails_closed_when_an_unorderable_tag_has_no_date() {
    let newest = Candidate {
        published_at: Some(instant("2026-06-30T03:24:03Z")),
        ..candidate("github-releases", "4.1.3", false, false)
    };
    let undated = Candidate {
        published_at: None,
        ..candidate("github-releases", "2.92", false, false)
    };
    let found = run(bare_scheme(), vec![newest.clone(), undated]);
    assert!(!found.resolved(), "no date, no second signal, no answer");
    assert_eq!(found.with_verdict(Verdict::Unorderable).len(), 1);

    // Nor when the unorderable one is newer than the winner.
    let later = Candidate {
        published_at: Some(instant("2026-08-01T00:00:00Z")),
        ..candidate("github-releases", "2026.autumn", false, false)
    };
    let found = run(bare_scheme(), vec![newest, later]);
    assert!(
        !found.resolved(),
        "it could be the newest and cannot be read"
    );
    assert_eq!(found.with_verdict(Verdict::Unorderable).len(), 1);
}

/// A tag that is not this target's release at all does not block. A repository
/// carrying `nox-5.2.3` beside `release-5.2.3` is ordinary.
#[test]
fn resolution_ignores_a_tag_that_is_not_this_targets() {
    let found = run(
        prefixed_scheme(),
        vec![
            candidate("github-releases", "release-5.2.3", false, false),
            candidate("github-releases", "webui-2.0", false, false),
        ],
    );
    assert_eq!(found.selected.as_ref().map(Version::as_str), Some("5.2.3"));
    assert_eq!(found.with_verdict(Verdict::ForeignTag).len(), 1);
}

/// Two spellings of one version make "newest" ambiguous rather than equal.
#[test]
fn resolution_fails_closed_on_two_spellings_of_one_version() {
    let found = run(
        VersionScheme {
            tag_prefix: None,
            min_components: 2,
            max_components: 3,
        },
        vec![
            candidate("github-releases", "4.1.0", false, false),
            candidate("github-releases", "4.1", false, false),
        ],
    );
    assert!(!found.resolved());
    assert_eq!(found.with_verdict(Verdict::Ambiguous).len(), 2);
}

/// The four-component enhanced-edition shape, which a three-component scheme
/// would call unorderable and a lexical sort would get backwards.
#[test]
fn resolution_orders_a_four_component_version() {
    let found = run(
        prefixed_scheme(),
        vec![
            candidate("github-releases", "release-5.2.3.9", false, false),
            candidate("github-releases", "release-5.2.3.10", false, false),
        ],
    );
    assert_eq!(
        found.selected.as_ref().map(Version::as_str),
        Some("5.2.3.10")
    );
}

#[test]
fn resolution_round_trips_a_decision_byte_for_byte() {
    let found = run(
        bare_scheme(),
        vec![candidate("github-releases", "4.1.3", false, false)],
    );
    let document = found.to_json().expect("writes");
    let read = Resolution::from_json(&document).expect("reads");
    assert_eq!(read, found);
    assert_eq!(read.to_json().expect("writes"), document);
}

#[test]
fn resolution_reports_an_unknown_schema_version_before_any_field() {
    let found = run(
        bare_scheme(),
        vec![candidate("github-releases", "4.1.3", false, false)],
    );
    let mut document: Value =
        serde_json::from_str(&found.to_json().expect("writes")).expect("parses");
    document["schema"] = json!("bit-ids/resolution/2");
    document["sources"] = json!([]);
    let error = Resolution::from_json(&serde_json::to_string(&document).expect("writes"))
        .expect_err("another generation");
    assert!(
        error.to_string().contains("unsupported schema"),
        "the version is answered first: {error}"
    );
}

type Mutation = fn(&mut Value);

/// One planted defect per refusal. ⛔ A code nobody has seen fire is a code
/// nobody knows works.
const PLANTED: &[(&str, Mutation)] = &[
    ("E-RES-01", |document| {
        document["sources"] = json!([]);
        document["considered"] = json!([]);
        document["selected"] = Value::Null;
    }),
    ("E-RES-02", |document| {
        let first = document["sources"][0].clone();
        document["sources"] = json!([first.clone(), first]);
    }),
    ("E-RES-03", |document| {
        document["considered"][0]["candidate"]["source"] = json!("no-such-source");
    }),
    ("E-RES-04", |document| {
        document["sources"][0]["candidates"] = json!(9);
    }),
    ("E-RES-05", |document| {
        document["sources"][0]["retrieved_at"] = json!("2026-09-04T23:59:59Z");
    }),
    ("E-RES-06", |document| {
        document["selected"] = json!("9.9.9");
    }),
    ("E-RES-07", |document| {
        document["considered"][0]["verdict"] = json!("superseded");
    }),
    ("E-RES-08", |document| {
        document["considered"][1]["verdict"] = json!("unorderable");
    }),
];

fn planted_base() -> Value {
    // Two candidates so a plant can target the second without removing the
    // selection, and one source so the counts are simple to move.
    let found = run(
        bare_scheme(),
        vec![
            candidate("github-releases", "4.1.3", false, false),
            candidate("github-releases", "4.1.2", false, false),
        ],
    );
    serde_json::from_str(&found.to_json().expect("writes")).expect("parses")
}

#[test]
fn resolution_accepts_the_document_the_defects_are_planted_in() {
    // ⛔ The table is only evidence if the document it mutates is otherwise
    // clean; a base that already failed would make every row pass for the
    // wrong reason.
    let document = planted_base();
    let text = serde_json::to_string(&document).expect("writes");
    Resolution::from_json(&text).expect("the base document validates");
}

#[test]
fn resolution_refuses_every_planted_defect() {
    for (code, plant) in PLANTED {
        let mut document = planted_base();
        plant(&mut document);
        let text = serde_json::to_string(&document).expect("writes");
        let error = Resolution::from_json(&text)
            .expect_err(&format!("{code}: the planted defect was accepted"));
        assert!(
            error.to_string().contains(code),
            "planting the defect for {code} produced {error}"
        );
    }
}

#[test]
fn resolution_plants_a_defect_for_every_diagnostic_code() {
    const SOURCE: &str = include_str!("../src/resolution.rs");
    let mut declared: Vec<&str> = SOURCE
        .match_indices("\"E-")
        .filter_map(|(start, _)| {
            let rest = &SOURCE[start + 1..];
            let end = rest.find('"')?;
            Some(&rest[..end])
        })
        .collect();
    declared.sort_unstable();
    declared.dedup();
    assert!(
        declared.len() >= 8,
        "the code scan found only {} codes, so it is not reading the module",
        declared.len()
    );
    let planted: Vec<&str> = PLANTED.iter().map(|(code, _)| *code).collect();
    let uncovered: Vec<&&str> = declared
        .iter()
        .filter(|code| !planted.contains(code))
        .collect();
    assert!(
        uncovered.is_empty(),
        "these diagnostics have never been seen to refuse anything: {uncovered:?}"
    );
}

/// The document is the record of a decision, so a reader must be able to see
/// every candidate that was weighed and why each lost.
#[test]
fn resolution_keeps_every_candidate_it_considered() {
    let offered = vec![
        candidate("github-releases", "release-5.2.3", false, false),
        candidate("github-releases", "release-5.2.2", false, false),
        candidate("github-releases", "release-5.3.0-beta1", true, false),
        candidate("github-releases", "webui-2.0", false, false),
    ];
    let found = run(prefixed_scheme(), offered.clone());
    assert_eq!(found.considered.len(), offered.len());
    for (entry, original) in found.considered.iter().zip(&offered) {
        assert_eq!(&entry.candidate, original, "a candidate was rewritten");
    }
    assert_eq!(found.schema.as_str(), RESOLUTION_SCHEMA);
    let selected: Vec<&Considered> = found.with_verdict(Verdict::Selected);
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].candidate.tag.as_str(), "release-5.2.3");
}

/// ⛔ `Verdict::as_str` and serde's `rename_all` are two spellings of one
/// vocabulary. A door sweep found nothing comparing them.
#[test]
fn resolution_verdict_spellings_agree_with_the_serialized_form() {
    for verdict in Verdict::ALL {
        let serialized = serde_json::to_string(verdict).expect("serializes");
        assert_eq!(
            format!("\"{}\"", verdict.as_str()),
            serialized,
            "{verdict:?} spells itself two ways"
        );
    }
    assert_eq!(
        Verdict::ALL.len(),
        9,
        "a variant was added without a spelling"
    );
}

/// A release the reader cannot turn into a candidate is refused, never dropped.
/// A candidate list quietly missing an entry is the silent skip the resolver
/// exists to prevent, one layer earlier.
#[test]
fn resolution_refuses_a_release_it_cannot_read_rather_than_dropping_it() {
    let id = slug("github-releases");
    let good = br#"[{"tag_name":"4.1.3","prerelease":false,"draft":false,
                    "published_at":"2026-06-30T03:24:03Z"}]"#;
    let read = bit_ids::resolution::sources::github_releases(good, &id).expect("reads");
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].tag.as_str(), "4.1.3");

    // A tag with a control byte in it is not a Label, and the whole list is
    // refused rather than yielding one candidate and silently losing the other.
    let bad = b"[{\"tag_name\":\"4.1.3\"},{\"tag_name\":\"4.1\\u00014\"}]";
    let error = bit_ids::resolution::sources::github_releases(bad, &id)
        .expect_err("an unusable tag is a finding");
    assert!(error.contains("tag"), "the message names the tag: {error}");

    // A published_at that is not this schema's instant is refused too.
    let odd = br#"[{"tag_name":"4.1.3","published_at":"2026-06-30T03:24:03.123Z"}]"#;
    let error = bit_ids::resolution::sources::github_releases(odd, &id)
        .expect_err("fractional seconds are not this schema's instant");
    assert!(error.contains("published_at"), "{error}");
}
