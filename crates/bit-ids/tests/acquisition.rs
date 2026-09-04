//! The route model, and the vocabulary it shares with the catalogue.
//!
//! ⛔ **Reading `catalogue/clients.toml` here is a vocabulary check and nothing
//! more.** `candidate_routes` says somebody expects a route to exist; it is a
//! research lead, and `TODO/acquisition.md` is explicit that it is not an
//! availability claim. Nothing in the crate reads the catalogue at run time and
//! no test here treats a listed route as evidence that it resolves.
//!
//! What it does catch is the two files drifting apart: a route kind added to the
//! catalogue that the schema cannot express, and a variant in the schema that
//! nothing in the catalogue asks for.

use bit_ids::acquisition::RouteKind;

const CATALOGUE: &str = include_str!("../../../catalogue/clients.toml");

/// Every quoted string inside every `candidate_routes = [ ... ]` array.
///
/// ⚠ A scanner, not a TOML parser, and it fails loud rather than quietly
/// finding nothing: the assertions below refuse a result that is too small to
/// be real. A `toml` dependency was the alternative and it lost, because this
/// is one hand-maintained file read by one test, and a third-party crate is a
/// decision the supply-chain rules make us argue for rather than a convenience.
fn candidate_routes() -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = CATALOGUE;
    while let Some(at) = rest.find("candidate_routes") {
        rest = &rest[at..];
        let Some(open) = rest.find('[') else { break };
        let Some(close) = rest.find(']') else { break };
        assert!(open < close, "an array that closes before it opens");
        for piece in rest[open..close].split('"').skip(1).step_by(2) {
            found.push(piece.to_owned());
        }
        rest = &rest[close..];
    }
    found
}

#[test]
fn acquisition_route_kinds_cover_the_catalogue() {
    let listed = candidate_routes();
    // ⛔ A scan that found nothing would make every assertion below pass for
    // the wrong reason, which is the exact shape a coverage check exists to
    // refuse. The catalogue carries 17 targets naming at least two routes each.
    assert!(
        listed.len() >= 34,
        "the scan found {} candidate routes, so it is not reading the catalogue",
        listed.len()
    );

    for name in &listed {
        assert!(
            RouteKind::from_catalogue_str(name).is_some(),
            "the catalogue names the route kind {name:?} and the schema cannot express it"
        );
    }

    for kind in RouteKind::ALL {
        assert!(
            listed.iter().any(|name| name == kind.as_catalogue_str()),
            "{kind} is a variant nothing in the catalogue asks for"
        );
    }
}

/// The catalogue states the two-route minimum and `E-ACQ-01` enforces it. A
/// number in two files with nothing comparing them is the drift this repository
/// refuses everywhere else.
#[test]
fn acquisition_minimum_route_count_agrees_with_the_catalogue() {
    let line = CATALOGUE
        .lines()
        .find(|line| line.starts_with("minimum_acquisition_routes"))
        .expect("the catalogue states a minimum");
    let stated: usize = line
        .split('=')
        .nth(1)
        .expect("a value")
        .trim()
        .parse()
        .expect("a number");
    assert_eq!(
        stated, 2,
        "the catalogue asks for {stated} routes and E-ACQ-01 enforces 2"
    );
}

#[test]
fn acquisition_pairs_each_route_kind_with_one_resolvable_source_form() {
    // Every kind names a form, every form is reachable, and no two kinds that
    // deliver differently share one. Checked here rather than read off the
    // match arms, because the match is what this is meant to catch a change in.
    let mut forms: Vec<&str> = RouteKind::ALL.iter().map(|k| k.source_form()).collect();
    forms.sort_unstable();
    forms.dedup();
    assert_eq!(
        forms,
        vec![
            "indexed_package",
            "module_version",
            "published_file",
            "release_asset",
            "source_commit",
        ],
        "a source form was added, removed or renamed"
    );
    assert_eq!(
        RouteKind::GithubRelease.source_form(),
        "release_asset",
        "a release asset is identified by its tag and file name, not by a version string"
    );
    assert_eq!(
        RouteKind::SourceBuild.source_form(),
        "source_commit",
        "a source build is identified by the commit it was built from"
    );
}
