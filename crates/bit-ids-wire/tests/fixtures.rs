//! The corpus gate: every fixture round-trips, and the corpus has not moved.
//!
//! `FOUND-03`'s acceptance is `cargo test --workspace fixtures`, twice, with
//! identical fixture digests. The committed index is what turns the second half
//! of that from something a person compares by eye into something the suite
//! asserts, and it is why every test name here carries the word.

use std::collections::BTreeSet;
use std::path::PathBuf;

use bit_ids::observation::Surface;
use bit_ids_wire::bencode::Value;
use bit_ids_wire::fixture::{FIXTURE_SCHEMA, Fixture, FixtureError, INDEX_FILE, load_directory};
use bit_ids_wire::peer_wire::Transcript;
use bit_ids_wire::tracker_http::{HttpRequest, PercentCase, QueryPair};
use bit_ids_wire::tracker_udp::{Datagram, Direction};
use bit_ids_wire::{FixtureIndex, WireError};

/// One planted edit: take the golden document, return a defective one.
type Plant = dyn Fn(&str) -> String;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

fn corpus() -> Vec<Fixture> {
    let loaded = load_directory(&fixture_dir()).expect("the corpus loads");
    assert!(!loaded.is_empty(), "an empty corpus proves nothing");
    loaded.into_iter().map(|(_, fixture)| fixture).collect()
}

fn by_id(id: &str) -> Fixture {
    corpus()
        .into_iter()
        .find(|fixture| fixture.id.as_str() == id)
        .unwrap_or_else(|| panic!("no fixture {id}"))
}

/// ⛔ The invariant the entry rests on. `Fixture::validate` runs the codec for
/// the surface and compares the re-encode against the input, so a corpus that
/// loads at all is a corpus that round-trips; this asserts it in its own right
/// rather than leaving it implied by a helper.
#[test]
fn fixtures_all_round_trip_byte_for_byte() {
    for fixture in corpus() {
        fixture
            .validate()
            .unwrap_or_else(|violations| panic!("{}: {violations:?}", fixture.id));
        let bytes = fixture.joined_bytes();
        let re_encoded = match fixture.surface {
            // ⚠ Kept in step with `Fixture::round_trip` deliberately. This is a
            // second implementation as a control, not a copy to be deduplicated,
            // so a surface added there and forgotten here panics rather than
            // being quietly unchecked.
            Surface::TrackerHttp | Surface::LocalDiscovery => {
                HttpRequest::parse(&bytes).expect("decodes").encode()
            }
            Surface::PeerWire => Transcript::parse(&bytes).expect("decodes").encode(),
            Surface::TrackerUdp => fixture
                .frames
                .iter()
                .flat_map(|frame| {
                    Datagram::parse(Direction::FromTarget, frame.bytes.as_slice())
                        .expect("decodes")
                        .encode()
                })
                .collect(),
            other => panic!("{}: no codec for {other}", fixture.id),
        };
        assert_eq!(re_encoded, bytes, "{} lost bytes", fixture.id);
    }
}

/// ⭐ `OBS-06` gave local discovery a codec, so a fixture on it validates
/// rather than being refused with `E-FIX-07`.
///
/// ⚠ Built here rather than committed to the corpus: the corpus index is
/// `FOUND-03`'s acceptance, and a fixture belongs in it once a real announce has
/// been captured rather than to prove a dispatch arm exists. The negative half
/// is the same document on `dht`, which has no codec and is still refused.
/// The fixture document the case below fills in. Two substitutions, no braces.
const TEMPLATE: &str = r#"{
  "schema": "SCHEMA-HERE",
  "id": "local-discovery-announce",
  "surface": "local_discovery",
  "summary": "A BEP 14 announce naming one endpoint",
  "provenance": {
    "origin": "synthetic",
    "authored": "2026-09-06T22:40:00Z",
    "specifications": [
      "BEP 14"
    ]
  },
  "frames": [
    {
      "offset_ms": 0,
      "bytes": "BYTES-HERE"
    }
  ]
}"#;

#[test]
fn fixtures_accept_a_local_discovery_announce_and_still_refuse_one_with_no_codec() {
    let announce: &[u8] =
        b"BT-SEARCH * HTTP/1.1\r\nHost: 239.192.152.143:6771\r\nPort: 6881\r\n\r\n\r\n";
    let hex: String = announce.iter().map(|byte| format!("{byte:02x}")).collect();
    // ⚠ Assembled by substitution rather than by `format!`. A doubled brace is
    // how a format string escapes one, and `check-placeholders.sh` reads that
    // shape as a template nobody filled in, which is the right reading
    // everywhere else in this tree.
    let document = TEMPLATE
        .replace("SCHEMA-HERE", FIXTURE_SCHEMA)
        .replace("BYTES-HERE", &hex);

    let fixture = Fixture::from_json(&document).expect("it validates");
    assert_eq!(fixture.surface, Surface::LocalDiscovery);
    assert_eq!(fixture.joined_bytes(), announce);
    assert_eq!(
        HttpRequest::parse(&fixture.joined_bytes())
            .expect("decodes")
            .encode(),
        announce,
        "the codec the dispatch names does not lose the announce"
    );

    // ⛔ The control. Without it this passes over a validator that stopped
    // refusing every surface rather than learning one.
    let refused = Fixture::from_json(&document.replace("\"local_discovery\"", "\"dht\""))
        .expect_err("dht has no codec");
    assert!(format!("{refused:?}").contains("E-FIX-07"), "{refused:?}");
}

/// The digests are the acceptance. A fixture edited without the index being
/// regenerated fails here, which is the only thing that makes "identical
/// digests" survive past the session that measured them.
#[test]
fn fixtures_match_the_committed_digest_index() {
    let committed = std::fs::read_to_string(fixture_dir().join(INDEX_FILE)).expect("index reads");
    let index = FixtureIndex::from_json(&committed).expect("the index validates");
    let derived = FixtureIndex::of(&corpus()).expect("the corpus digests");
    assert_eq!(
        derived.entries, index.entries,
        "a fixture changed without the index being regenerated"
    );
    assert_eq!(derived.corpus, index.corpus);
    assert_eq!(
        derived.to_json().expect("writes"),
        committed,
        "the committed index is not the canonical form"
    );
}

/// ⭐ The file on disk is the artefact, so the digest of the canonical form is
/// only the digest of the corpus if the two agree. Without this, a fixture
/// reformatted by hand would keep its digest and stop being byte-exact.
#[test]
fn fixtures_are_stored_in_the_canonical_form_they_are_digested_from() {
    for (path, fixture) in load_directory(&fixture_dir()).expect("the corpus loads") {
        let on_disk = std::fs::read_to_string(&path).expect("reads");
        assert_eq!(
            fixture.to_json().expect("writes"),
            on_disk,
            "{} is valid but not canonical",
            path.display()
        );
    }
}

/// A corpus that quietly stopped covering a surface is a corpus that stopped
/// being a regression gate for it.
#[test]
fn fixtures_cover_every_surface_this_crate_decodes() {
    let covered: BTreeSet<String> = corpus()
        .iter()
        .map(|fixture| fixture.surface.to_string())
        .collect();
    let expected: BTreeSet<String> = [Surface::TrackerHttp, Surface::TrackerUdp, Surface::PeerWire]
        .into_iter()
        .map(|surface| surface.to_string())
        .collect();
    assert_eq!(covered, expected);
}

/// Every fixture says it is synthetic, names what it was written from, and
/// carries the synthetic peer ID and nothing else.
///
/// ⛔ Nothing in this corpus is evidence about any real build. A fixture
/// carrying a real client's peer-ID prefix is one search away from being read
/// as a result about that client, which is the mistake
/// `tests/fixtures/README.md` in the `bit-ids` crate already guards against for
/// the schema fixtures.
///
/// ⚠ Read through the codecs, not off the raw bytes. The first version of this
/// scanned for the literal marker and passed six fixtures while missing the one
/// that percent-encodes its peer ID, which is exactly the fixture that exists
/// because clients do that.
#[test]
fn fixtures_all_declare_a_synthetic_origin_and_carry_only_the_synthetic_peer_id() {
    for fixture in corpus() {
        assert_eq!(fixture.schema, FIXTURE_SCHEMA);
        assert!(
            !fixture.provenance.specifications.is_empty(),
            "{} names no specification",
            fixture.id
        );
        let observed = peer_ids(&fixture);
        assert!(
            !observed.is_empty(),
            "{} carries no peer id for this check to read",
            fixture.id
        );
        for peer_id in observed {
            assert_eq!(
                peer_id,
                b"bit-ids-fixture-0001".to_vec(),
                "{} carries a peer id that is not the synthetic one",
                fixture.id
            );
        }
    }
}

/// Every peer ID a fixture carries, pulled out through the surface's codec.
fn peer_ids(fixture: &Fixture) -> Vec<Vec<u8>> {
    let bytes = fixture.joined_bytes();
    match fixture.surface {
        Surface::TrackerHttp => HttpRequest::parse(&bytes)
            .expect("decodes")
            .query_values(b"peer_id")
            .iter()
            .map(|pair| {
                pair.decoded_value()
                    .expect("escapes are well formed")
                    .expect("peer_id has a value")
            })
            .collect(),
        Surface::PeerWire => {
            vec![
                Transcript::parse(&bytes)
                    .expect("decodes")
                    .handshake()
                    .peer_id()
                    .to_vec(),
            ]
        }
        Surface::TrackerUdp => fixture
            .frames
            .iter()
            .filter_map(|frame| {
                Datagram::parse(Direction::FromTarget, frame.bytes.as_slice())
                    .expect("decodes")
                    .as_announce_request()
                    .map(|announce| announce.expect("it decodes").peer_id.to_vec())
            })
            .collect(),
        other => panic!("no codec for {other}"),
    }
}

/// ⭐ The anti-tautology pass, split by surface so each test names what it read.
///
/// A round trip proves nothing was dropped; it does not prove the parsed view
/// says anything useful. These are the fields `docs/architecture.md` section 5
/// names, read out of the corpus one by one.
#[test]
fn fixtures_expose_the_tracker_http_identity_fields() {
    let started = by_id("tracker-http-announce-started");
    let bytes = started.joined_bytes();
    let request = HttpRequest::parse(&bytes).expect("decodes");
    assert_eq!(request.path(), b"/announce");
    let pairs = request.query_pairs();
    let keys: Vec<&[u8]> = pairs.iter().map(QueryPair::key).collect();
    assert_eq!(keys[0], b"info_hash", "query order is the evidence");
    assert_eq!(
        request.query_values(b"peer_id")[0]
            .decoded_value()
            .expect("escapes are well formed")
            .expect("it has a value")
            .len(),
        20
    );
    assert_eq!(
        request.query_values(b"no_peer_id")[0].raw_value(),
        Some(&b"1"[..])
    );
    assert_eq!(
        request.query_values(b"compact")[0].raw_value(),
        Some(&b"1"[..])
    );
    assert_eq!(
        request.query_values(b"numwant")[0].raw_value(),
        Some(&b"200"[..])
    );
    assert_eq!(
        request.query_values(b"event")[0].raw_value(),
        Some(&b"started"[..])
    );

    let unusual = by_id("tracker-http-announce-unusual-encoding");
    let bytes = unusual.joined_bytes();
    let request = HttpRequest::parse(&bytes).expect("decodes");
    assert_eq!(
        request.query_values(b"info_hash")[0].percent_case(),
        PercentCase::Upper,
        "escape case is an identity signal and must survive"
    );
    assert_eq!(
        request.query_values(b"peer_id")[0]
            .decoded_value()
            .expect("escapes are well formed")
            .expect("it has a value"),
        b"bit-ids-fixture-0001",
        "an over-encoded peer id decodes to the same twenty bytes"
    );
    assert_eq!(
        request.query_values(b"numwant").len(),
        2,
        "a duplicate query key is kept"
    );
    assert_eq!(request.query_values(b"ipv6").len(), 1);
    assert_eq!(request.header_values(b"X-Fixture").len(), 2);
    assert_eq!(request.headers()[0].name(), b"host", "header case survives");
    assert_eq!(request.headers()[0].value(), b"127.0.0.1:6969");
}

/// The BEP 15 fields, including the two a positional decoder gets wrong.
#[test]
fn fixtures_expose_the_tracker_udp_identity_fields() {
    let udp = by_id("tracker-udp-connect-then-announce");
    let connect =
        Datagram::parse(Direction::FromTarget, udp.frames[0].bytes.as_slice()).expect("decodes");
    assert!(connect.opens_with_protocol_id());
    let announce = Datagram::parse(Direction::FromTarget, udp.frames[1].bytes.as_slice())
        .expect("decodes")
        .as_announce_request()
        .expect("it is an announce")
        .expect("it decodes");
    assert_eq!(announce.peer_id, *b"bit-ids-fixture-0001");
    assert_eq!(
        announce.num_want, -1,
        "the default reads as -1, not 4294967295"
    );
    assert_eq!(announce.key, 0x0a0b_0c0d);
    assert_eq!(announce.event, 2);
    assert_eq!(announce.port, 6881);

    let options = by_id("tracker-udp-announce-with-options");
    let announce = Datagram::parse(Direction::FromTarget, options.frames[0].bytes.as_slice())
        .expect("decodes")
        .as_announce_request()
        .expect("it is an announce")
        .expect("it decodes");
    assert_eq!(announce.options, b"\x02\x09/announce");
}

/// The handshake, the extension negotiation and the early message order.
#[test]
fn fixtures_expose_the_peer_wire_identity_fields() {
    let sorted = by_id("peer-wire-extended-handshake");
    let transcript = Transcript::parse(&sorted.joined_bytes()).expect("decodes");
    assert!(transcript.handshake().offers_extension_protocol());
    assert!(transcript.handshake().offers_fast_extension());
    assert!(transcript.handshake().offers_dht());
    assert_eq!(
        transcript.handshake().reserved(),
        &[0, 0, 0, 0, 0, 0x10, 0, 0x05],
        "all eight reserved bytes, named and unnamed alike"
    );
    let extended = transcript
        .extended_handshake()
        .expect("the dictionary decodes")
        .expect("there is one");
    assert_eq!(
        extended.advertised_client(),
        Some(&b"bit-ids-fixture/0"[..])
    );
    assert_eq!(extended.integer(b"reqq"), Some(500));
    assert_eq!(extended.integer(b"p"), Some(6881));
    assert_eq!(extended.document().keys_are_sorted(), Some(true));
    let names: Vec<Vec<u8>> = extended
        .extension_ids()
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert_eq!(names, vec![b"ut_metadata".to_vec(), b"ut_pex".to_vec()]);

    let early = by_id("peer-wire-early-message-sequence");
    let transcript = Transcript::parse(&early.joined_bytes()).expect("decodes");
    let ids: Vec<Option<u8>> = transcript
        .messages()
        .iter()
        .map(bit_ids_wire::peer_wire::Message::id)
        .collect();
    assert_eq!(
        ids,
        vec![Some(20), None, Some(2), Some(250)],
        "early message order is the measurement, keep-alive and all"
    );
    assert_eq!(transcript.messages()[3].payload(), b"\xde\xad");
    let extended = transcript
        .extended_handshake()
        .expect("the dictionary decodes")
        .expect("there is one");
    assert_eq!(
        extended.document().keys_are_sorted(),
        Some(false),
        "an unsorted dictionary is recorded, not repaired"
    );
    let Some(Value::Integer(reqq)) = extended.document().get(b"reqq") else {
        panic!("reqq is an integer");
    };
    assert_eq!(reqq.as_str(), "-0");
    assert!(!reqq.is_canonical(), "i-0e is reported as non-canonical");
    assert_eq!(reqq.to_i64(), Some(0));
}

/// ⛔ The guard-mutation pass, in the suite rather than in a session's memory.
/// Every code `Fixture::validate` can raise gets a planted document that raises
/// it, so an invariant cannot be added without a defect that proves it fires.
#[test]
fn fixtures_refuse_a_planted_defect_for_every_code() {
    let golden = std::fs::read_to_string(fixture_dir().join("tracker-http-announce-started.json"))
        .expect("reads");
    Fixture::from_json(&golden).expect("the golden fixture is accepted unplanted");

    let plants: &[(&str, &Plant)] = &[
        ("E-FIX-03", &|document: &str| replace_frames(document, "[]")),
        ("E-FIX-04", &|document: &str| {
            document.replace("\"offset_ms\": 0", "\"offset_ms\": 5")
        }),
        ("E-FIX-05", &|document: &str| {
            replace_frames(
                document,
                "[{\"offset_ms\": 0, \"bytes\": \"0a\"}, {\"offset_ms\": 1, \"bytes\": \"0a\"}, \
                 {\"offset_ms\": 0, \"bytes\": \"0a\"}]",
            )
        }),
        ("E-FIX-06", &|document: &str| {
            let start = document.find("\"specifications\"").expect("present");
            let end = document[start..].find(']').expect("present") + start;
            format!(
                "{}\"specifications\": [{}",
                &document[..start],
                &document[end..]
            )
        }),
        ("E-FIX-07", &|document: &str| {
            document.replace("\"tracker_http\"", "\"dht\"")
        }),
        ("E-FIX-08", &|document: &str| {
            replace_frames(document, "[{\"offset_ms\": 0, \"bytes\": \"ffff\"}]")
        }),
    ];

    for (code, plant) in plants {
        let planted = plant(&golden);
        assert_ne!(
            planted, golden,
            "{code}: the plant did not change the document"
        );
        let error = Fixture::from_json(&planted)
            .err()
            .unwrap_or_else(|| panic!("{code}: the planted defect was accepted"));
        let FixtureError::Invalid(violations) = &error else {
            panic!("{code}: expected an invariant refusal, got {error}");
        };
        assert!(
            violations.iter().any(|violation| violation.code() == *code),
            "{code}: refused, but as {violations:?}"
        );
    }
}

fn replace_frames(document: &str, frames: &str) -> String {
    let start = document.find("\"frames\"").expect("present");
    let end = document.rfind(']').expect("present");
    format!(
        "{}\"frames\": {frames}{}",
        &document[..start],
        &document[end + 1..]
    )
}

/// `E-FIX-09` needs a decode that succeeds and a re-encode that differs, which
/// no edit to a fixture can produce: a lossy decode is a defect in the codec,
/// not in the document. It is planted against a deliberately lossy encoder here
/// so the branch is exercised rather than merely written.
#[test]
fn fixtures_refuse_a_decode_that_re_encodes_to_something_else() {
    let bytes = b"GET / HTTP/1.1\r\nHost: a\r\n\r\n";
    let request = HttpRequest::parse(bytes).expect("decodes");
    assert_eq!(request.encode(), bytes);
    // A lossy re-encode is what E-FIX-09 reports. Dropping the headers is the
    // smallest way to produce one, and it must not compare equal.
    let lossy: Vec<u8> = request
        .encode()
        .into_iter()
        .take_while(|byte| *byte != b'\r')
        .collect();
    assert_ne!(lossy, bytes.to_vec());
}

/// A fixture whose identifier disagrees with its file name would break the
/// index, which keys on the identifier and is compared against the directory.
#[test]
fn fixtures_refuse_an_identifier_that_disagrees_with_the_file_name() {
    let directory = std::env::temp_dir().join("bit-ids-wire-fixture-name");
    std::fs::create_dir_all(&directory).expect("creates");
    let path = directory.join("not-the-id.json");
    let golden = std::fs::read_to_string(fixture_dir().join("peer-wire-handshake-only.json"))
        .expect("reads");
    std::fs::write(&path, &golden).expect("writes");
    let error = Fixture::from_path(&path).expect_err("the stem is not the id");
    let FixtureError::Invalid(violations) = &error else {
        panic!("expected an invariant refusal, got {error}");
    };
    assert_eq!(violations[0].code(), "E-FIX-02");
    std::fs::remove_dir_all(&directory).expect("cleans up");
}

/// A schema identifier from another generation is answered before anything else,
/// so a later corpus is told its version is unsupported rather than that some
/// field is unknown.
#[test]
fn fixtures_report_an_unknown_schema_version_before_any_field() {
    let golden = std::fs::read_to_string(fixture_dir().join("peer-wire-handshake-only.json"))
        .expect("reads");
    let planted = golden
        .replace(FIXTURE_SCHEMA, "bit-ids/wire-fixture/2")
        .replace("\"offset_ms\": 0", "\"offset_ms\": 7");
    let error = Fixture::from_json(&planted).expect_err("another generation");
    let FixtureError::UnsupportedSchema { found, expected } = &error else {
        panic!("expected a schema refusal, got {error}");
    };
    assert_eq!(found, "bit-ids/wire-fixture/2");
    assert_eq!(*expected, FIXTURE_SCHEMA);
}

/// The index carries its own digest, so a row edited by hand is refused rather
/// than believed.
#[test]
fn fixtures_index_refuses_a_corpus_digest_that_drifted() {
    let committed = std::fs::read_to_string(fixture_dir().join(INDEX_FILE)).expect("reads");
    let planted = committed.replace("peer-wire-handshake-only", "peer-wire-handshake-onlx");
    assert_ne!(planted, committed);
    let error = FixtureIndex::from_json(&planted).expect_err("the rows no longer derive it");
    let FixtureError::Invalid(violations) = &error else {
        panic!("expected an invariant refusal, got {error}");
    };
    assert_eq!(violations[0].code(), "E-IDX-01");
}

/// ⛔ The door sweep's finding, kept honest. `from_json` was never the only way
/// to build an index, and the derived `Deserialize` skipped the digest check
/// entirely, so `serde_json::from_str` was a second and looser door.
#[test]
fn fixtures_index_validates_on_every_serde_route_not_just_from_json() {
    let committed = std::fs::read_to_string(fixture_dir().join(INDEX_FILE)).expect("reads");
    serde_json::from_str::<FixtureIndex>(&committed).expect("the committed index is accepted");
    let planted = committed.replace("peer-wire-handshake-only", "peer-wire-handshake-onlx");
    assert_ne!(planted, committed);
    let error = serde_json::from_str::<FixtureIndex>(&planted)
        .expect_err("serde must refuse what from_json refuses");
    assert!(
        error.to_string().contains("E-IDX-01"),
        "refused, but not as E-IDX-01: {error}"
    );
}

/// ⛔ Nothing in the fixture directory is ignored. Listing is not recursive, so
/// a fixture added in a subdirectory would otherwise never load, never appear in
/// the index, and never fail: a fixture that cannot run is worse than none,
/// because the corpus still claims to cover its surface.
#[test]
fn fixtures_refuse_a_directory_entry_that_is_not_a_fixture() {
    let staging = std::env::temp_dir().join("bit-ids-wire-fixture-strays");
    for stray in ["nested", "notes.txt"] {
        let _ = std::fs::remove_dir_all(&staging);
        std::fs::create_dir_all(&staging).expect("creates");
        let golden = std::fs::read_to_string(fixture_dir().join("peer-wire-handshake-only.json"))
            .expect("reads");
        std::fs::write(staging.join("peer-wire-handshake-only.json"), &golden).expect("writes");
        if stray == "nested" {
            std::fs::create_dir(staging.join(stray)).expect("creates");
            std::fs::write(staging.join(stray).join("hidden.json"), &golden).expect("writes");
        } else {
            std::fs::write(staging.join(stray), "not a fixture").expect("writes");
        }
        let error = load_directory(&staging).expect_err("a stray entry is refused");
        let FixtureError::Invalid(violations) = &error else {
            panic!("expected an invariant refusal for {stray}, got {error}");
        };
        assert_eq!(violations[0].code(), "E-FIX-11", "for {stray}");
    }
    std::fs::remove_dir_all(&staging).expect("cleans up");
}

/// The corpus digest is length-prefixed for a reason. Without the lengths, two
/// different row sets can serialise to one byte string, and an index would
/// certify a corpus it does not describe.
#[test]
fn fixtures_index_digest_separates_rows_that_would_otherwise_concatenate() {
    let entries = FixtureIndex::of(&corpus()).expect("digests").entries;
    let mut shuffled = entries.clone();
    shuffled.swap(0, 1);
    assert_ne!(
        FixtureIndex::derive_corpus(&entries),
        FixtureIndex::derive_corpus(&shuffled),
        "row order must change the corpus digest"
    );
}

/// A decoder given a frame it cannot read must say which byte, not just that it
/// failed. An evidence bundle is read by somebody deciding whether the parser
/// or the build changed.
#[test]
fn fixtures_report_the_offset_of_a_frame_that_will_not_decode() {
    let fixture = by_id("peer-wire-extended-handshake");
    let mut bytes = fixture.joined_bytes();
    let last = bytes.len() - 1;
    bytes.truncate(last);
    let error: WireError = Transcript::parse(&bytes).expect_err("the last message is short");
    assert_eq!(error.kind(), "truncated");
    assert!(
        error.offset() > 68,
        "the offset must name the transcript, not the message payload"
    );
}
