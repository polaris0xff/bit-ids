//! `OBS-02`'s acceptance: raw ordering, binary query values, repeated requests,
//! and malformed input.
//!
//! ⚠ Every announce here is written by this test, byte for byte, and driven
//! over a real socket. That is what an observer suite can prove: no client is
//! installed, so nothing here is evidence about any build.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use bit_ids_lab::Lab;
use bit_ids_probe::{HttpTracker, OfferedPeer, TrackerResponse};
use bit_ids_wire::bencode::{self, Value};
use bit_ids_wire::tracker_http::PercentCase;

/// Starts a lab with one HTTP tracker endpoint and returns both.
fn lab_with(tracker: &HttpTracker) -> Lab {
    Lab::builder()
        .deadline(Duration::from_secs(60))
        .stream("tracker-http", tracker.responder())
        .expect("a canonical endpoint name")
        .start()
        .expect("loopback binds")
}

/// Sends one request head and reads the whole response.
fn announce(lab: &Lab, head: &[u8]) -> Vec<u8> {
    let address = lab.endpoint("tracker-http").expect("added").address();
    let mut client = TcpStream::connect(address).expect("the endpoint accepts");
    client
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("a read timeout is settable");
    client.write_all(head).expect("write");
    client.flush().expect("flush");
    let mut answer = Vec::new();
    // The observer keeps the connection open, so the read stops at the declared
    // body rather than at end of stream.
    read_one_response(&mut client, &mut answer);
    answer
}

fn read_one_response(client: &mut TcpStream, answer: &mut Vec<u8>) {
    let mut chunk = [0_u8; 1024];
    loop {
        let read = client.read(&mut chunk).expect("the endpoint answers");
        if read == 0 {
            return;
        }
        answer.extend_from_slice(&chunk[..read]);
        if let Some(end) = bit_ids_wire::tracker_http::head_end(answer) {
            let declared = declared_length(&answer[..end]);
            if answer.len() >= end + declared {
                answer.truncate(end + declared);
                return;
            }
        }
    }
}

fn declared_length(head: &[u8]) -> usize {
    let text = String::from_utf8_lossy(head).to_ascii_lowercase();
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("content-length:") {
            return value.trim().parse().unwrap_or(0);
        }
    }
    0
}

/// The bencoded body of a response.
fn body_of(answer: &[u8]) -> Vec<u8> {
    let end = bit_ids_wire::tracker_http::head_end(answer).expect("a response has a head");
    answer[end..].to_vec()
}

#[test]
fn http_tracker_keeps_the_query_order_and_the_duplicates_that_arrived() {
    let tracker = HttpTracker::default();
    let lab = lab_with(&tracker);
    announce(
        &lab,
        b"GET /announce?port=6881&info_hash=%01%02&port=6882&peer_id=-QB5000-abcdefghijkl HTTP/1.1\r\n\
          Host: 127.0.0.1\r\n\r\n",
    );
    drop(lab);

    let seen = tracker.announces();
    assert_eq!(seen.len(), 1);
    // ⭐ `port` twice, and `port` before `info_hash`. A map keyed by name loses
    // both, and both are differences between builds.
    assert_eq!(
        seen[0].query_key_order(),
        vec![
            b"port".to_vec(),
            b"info_hash".to_vec(),
            b"port".to_vec(),
            b"peer_id".to_vec(),
        ]
    );
}

#[test]
fn http_tracker_keeps_the_header_order_and_the_case_that_arrived() {
    let tracker = HttpTracker::default();
    let lab = lab_with(&tracker);
    announce(
        &lab,
        b"GET /announce?info_hash=%01 HTTP/1.1\r\n\
          User-Agent: qBittorrent/5.0.0\r\n\
          host: 127.0.0.1\r\n\
          Accept-Encoding: gzip\r\n\
          ACCEPT: */*\r\n\r\n",
    );
    drop(lab);

    let seen = tracker.announces();
    assert_eq!(
        seen[0].header_name_order(),
        vec![
            b"User-Agent".to_vec(),
            b"host".to_vec(),
            b"Accept-Encoding".to_vec(),
            b"ACCEPT".to_vec(),
        ],
        "the spelling of a header name is a difference between builds"
    );
    assert_eq!(
        seen[0].header(b"user-agent"),
        Some(&b"qBittorrent/5.0.0"[..])
    );
}

#[test]
fn http_tracker_keeps_a_binary_query_value_and_the_case_of_its_escapes() {
    let tracker = HttpTracker::default();
    let lab = lab_with(&tracker);
    // Every byte class an info hash can carry: a null, a high byte, a literal
    // `+`, and one escape in each hexadecimal case.
    announce(
        &lab,
        b"GET /announce?info_hash=%00%FF%2b%1a&peer_id=%2Dqb%2D HTTP/1.1\r\n\r\n",
    );
    drop(lab);

    let seen = tracker.announces();
    let info_hash = seen[0]
        .decoded(b"info_hash")
        .expect("the key is present")
        .expect("it decodes");
    // ⛔ `+` stays a plus. Folding it to a space is an HTML form convention and
    // would corrupt one byte in 256 of the field this catalogue is about.
    assert_eq!(info_hash, vec![0x00, 0xff, b'+', 0x1a]);
    assert_eq!(seen[0].percent_case(b"info_hash"), Some(PercentCase::Mixed));
    assert_eq!(seen[0].percent_case(b"peer_id"), Some(PercentCase::Upper));
    assert_eq!(
        seen[0].peer_id().expect("present").expect("decodes"),
        b"-qb-".to_vec()
    );
}

#[test]
fn http_tracker_records_every_announce_of_a_lifecycle_in_order() {
    let tracker = HttpTracker::default();
    let lab = lab_with(&tracker);
    for event in ["started", "completed", "stopped"] {
        let head = format!("GET /announce?info_hash=%01&event={event} HTTP/1.1\r\n\r\n");
        announce(&lab, head.as_bytes());
    }
    drop(lab);

    let seen = tracker.announces();
    assert_eq!(
        seen.len(),
        3,
        "one record per announce, not one per connection"
    );
    let events: Vec<Vec<u8>> = seen
        .iter()
        .map(|one| one.decoded(b"event").expect("present").expect("decodes"))
        .collect();
    assert_eq!(
        events,
        vec![
            b"started".to_vec(),
            b"completed".to_vec(),
            b"stopped".to_vec()
        ]
    );
}

#[test]
fn http_tracker_answers_two_announces_on_one_connection() {
    // A client that reuses a connection is a different observation from one
    // that opens a new one per announce, and both have to work or the
    // difference reads as the observer's.
    let tracker = HttpTracker::default();
    let lab = lab_with(&tracker);
    let address = lab.endpoint("tracker-http").expect("added").address();
    let mut client = TcpStream::connect(address).expect("the endpoint accepts");
    client
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("a read timeout is settable");

    for event in ["started", "stopped"] {
        let head = format!("GET /announce?info_hash=%01&event={event} HTTP/1.1\r\n\r\n");
        client.write_all(head.as_bytes()).expect("write");
        client.flush().expect("flush");
        let mut answer = Vec::new();
        read_one_response(&mut client, &mut answer);
        assert!(answer.starts_with(b"HTTP/1.1 200 OK"));
    }
    drop(client);
    drop(lab);

    assert_eq!(tracker.announces().len(), 2);
}

#[test]
fn http_tracker_answers_a_body_that_decodes_and_re_encodes_unchanged() {
    let tracker = HttpTracker::new(TrackerResponse {
        interval: 900,
        complete: 2,
        incomplete: 3,
        peers: vec![OfferedPeer {
            address: [127, 0, 0, 1],
            port: 6881,
            peer_id: *b"bit-ids-fixture-0001",
        }],
    });
    let lab = lab_with(&tracker);
    let answer = announce(
        &lab,
        b"GET /announce?info_hash=%01&compact=1 HTTP/1.1\r\n\r\n",
    );
    drop(lab);

    let body = body_of(&answer);
    let decoded = bencode::decode(&body).expect("the answer is bencode");
    // ⭐ The round trip is the same invariant the fixture corpus holds. A
    // response this project cannot re-encode is one a client may read
    // differently from how it was meant.
    assert_eq!(bencode::encode(&decoded), body);

    let Value::Dictionary(entries) = &decoded else {
        panic!("a tracker response is a dictionary");
    };
    assert_eq!(decoded.keys_are_sorted(), Some(true));
    let peers = entries
        .iter()
        .find(|(key, _)| key == b"peers")
        .map(|(_, value)| value)
        .expect("peers is present");
    assert_eq!(
        peers,
        &Value::bytes(vec![127, 0, 0, 1, 0x1a, 0xe1]),
        "compact=1 asked for six bytes per peer"
    );
}

#[test]
fn http_tracker_answers_the_shape_the_announce_asked_for() {
    // ⛔ A client that asked for a peer list and got a compact string reports an
    // error and changes what it does next. That behaviour would be recorded as
    // identity and it would be this code's, not the build's.
    let tracker = HttpTracker::new(TrackerResponse {
        peers: vec![OfferedPeer {
            address: [127, 0, 0, 1],
            port: 6881,
            peer_id: *b"bit-ids-fixture-0001",
        }],
        ..TrackerResponse::default()
    });
    let lab = lab_with(&tracker);
    let listed = announce(
        &lab,
        b"GET /announce?info_hash=%01&compact=0 HTTP/1.1\r\n\r\n",
    );
    let anonymous = announce(
        &lab,
        b"GET /announce?info_hash=%01&compact=0&no_peer_id=1 HTTP/1.1\r\n\r\n",
    );
    drop(lab);

    let decoded = bencode::decode(&body_of(&listed)).expect("bencode");
    let peers = decoded.get(b"peers").expect("peers is present");
    let Value::List(entries) = peers else {
        panic!("compact=0 asked for a list, got {peers:?}");
    };
    assert_eq!(entries.len(), 1);
    assert!(entries[0].get(b"peer id").is_some());

    let decoded = bencode::decode(&body_of(&anonymous)).expect("bencode");
    let Value::List(entries) = decoded.get(b"peers").expect("present") else {
        panic!("compact=0 asked for a list");
    };
    assert!(
        entries[0].get(b"peer id").is_none(),
        "no_peer_id=1 asked for the peer id to be left out"
    );
}

#[test]
fn http_tracker_refuses_a_head_it_cannot_decode_and_says_why_in_bencode() {
    let tracker = HttpTracker::default();
    let lab = lab_with(&tracker);
    // A header line with no colon: the decoder's own refusal.
    let answer = announce(&lab, b"GET /announce HTTP/1.1\r\nnot-a-header\r\n\r\n");
    drop(lab);

    assert!(answer.starts_with(b"HTTP/1.1 400 Bad Request"));
    let decoded = bencode::decode(&body_of(&answer)).expect("even a refusal is bencode");
    assert!(
        decoded.get(b"failure reason").is_some(),
        "a client reads the reason out of the body, not the status line"
    );
    assert!(
        tracker.announces().is_empty(),
        "a head that did not decode is not an announce"
    );
}

#[test]
fn http_tracker_refuses_a_head_that_never_ends() {
    use bit_ids_wire::tracker_http::HttpRequest;

    let tracker = HttpTracker::default();
    let lab = lab_with(&tracker);
    let address = lab.endpoint("tracker-http").expect("added").address();
    let mut client = TcpStream::connect(address).expect("the endpoint accepts");
    client
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("a read timeout is settable");

    // ⛔ The target is a binary this project installed minutes earlier. A head
    // it never terminates has to be bounded or it is a memory leak with a
    // socket attached.
    let flood = vec![b'a'; HttpRequest::MAX_HEAD + 1024];
    client
        .write_all(b"GET /announce HTTP/1.1\r\n")
        .expect("write");
    let _ = client.write_all(&flood);
    let _ = client.flush();

    let mut answer = Vec::new();
    read_one_response(&mut client, &mut answer);
    drop(lab);

    assert!(
        answer.starts_with(b"HTTP/1.1 400 Bad Request"),
        "got {:?}",
        String::from_utf8_lossy(&answer[..answer.len().min(64)])
    );
    assert!(tracker.announces().is_empty());
}

#[test]
fn http_tracker_frames_a_head_whose_terminators_are_mixed() {
    // ⚠ A bare newline where the grammar says CRLF is tolerated by trackers and
    // is a difference between builds. A framer that could not find the end of
    // such a head would record the announce and never answer it, which reads as
    // a client that announced and hung.
    let tracker = HttpTracker::default();
    let lab = lab_with(&tracker);
    let answer = announce(&lab, b"GET /announce?info_hash=%01 HTTP/1.1\nHost: h\n\r\n");
    drop(lab);

    assert!(answer.starts_with(b"HTTP/1.1 200 OK"));
    let seen = tracker.announces();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].header_name_order(), vec![b"Host".to_vec()]);
}

#[test]
fn http_tracker_keeps_the_head_bytes_exactly_as_they_arrived() {
    let head = b"GET /announce?info_hash=%00%ff&numwant=200&key=%7A1 HTTP/1.1\r\n\
                 User-Agent: Transmission/4.0.6\r\n\r\n";
    let tracker = HttpTracker::default();
    let lab = lab_with(&tracker);
    announce(&lab, head);
    drop(lab);

    let seen = tracker.announces();
    // ⭐ Not "equivalent to": the same bytes. Everything else this observer
    // reports is derived from these, so a raw record that drifted would make
    // every derived claim unverifiable.
    assert_eq!(seen[0].raw(), head);
    assert_eq!(seen[0].request().encode(), head.to_vec());
    assert_eq!(
        seen[0]
            .decoded(b"numwant")
            .expect("present")
            .expect("decodes"),
        b"200".to_vec()
    );
    assert_eq!(
        seen[0].decoded(b"key").expect("present").expect("decodes"),
        b"z1".to_vec()
    );
}

#[test]
fn http_tracker_reports_an_undecodable_value_rather_than_dropping_it() {
    let tracker = HttpTracker::default();
    let lab = lab_with(&tracker);
    // A truncated escape. The request decodes; this one value does not.
    announce(&lab, b"GET /announce?info_hash=%0 HTTP/1.1\r\n\r\n");
    drop(lab);

    let seen = tracker.announces();
    assert_eq!(seen.len(), 1, "the announce is still an announce");
    assert!(
        seen[0].decoded(b"info_hash").expect("present").is_err(),
        "a value that is not valid percent-encoding is an observation, not an omission"
    );
    assert_eq!(seen[0].decoded(b"absent"), None);
}

/// ⭐ The observer is driven by the committed corpus, not only by heads this
/// file writes.
///
/// `TODO/PROGRESS.md` puts the observers against the `bit-ids-wire` fixture
/// corpus rather than a live client, and the reason is in
/// `crates/bit-ids-wire/src/fixture.rs`: a live capture cannot separate an
/// observer regression from a client behaviour change, because both of its
/// inputs moved. These bytes provably did not. A parse that changes against one
/// is this code.
///
/// ⚠ The fixtures are synthetic and are not evidence about any build. What this
/// asserts is that the observer answers them and records them unchanged.
#[test]
fn http_tracker_answers_every_committed_tracker_fixture_over_a_real_socket() {
    use bit_ids::observation::Surface;
    use bit_ids_wire::fixture::load_directory;

    let corpus =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../bit-ids-wire/tests/fixtures");
    let loaded = load_directory(&corpus).expect("the corpus loads");
    let heads: Vec<(String, Vec<u8>)> = loaded
        .iter()
        .filter(|(_, fixture)| fixture.surface == Surface::TrackerHttp)
        .map(|(_, fixture)| (fixture.id.as_str().to_owned(), fixture.joined_bytes()))
        .collect();
    // A sweep that found no fixtures would pass having read nothing.
    assert!(
        heads.len() >= 2,
        "the corpus should carry more than one HTTP fixture, found {}",
        heads.len()
    );

    let tracker = HttpTracker::default();
    let lab = lab_with(&tracker);
    for (id, head) in &heads {
        let answer = announce(&lab, head);
        assert!(
            answer.starts_with(b"HTTP/1.1 200 OK"),
            "{id} was not answered: {:?}",
            String::from_utf8_lossy(&answer[..answer.len().min(64)])
        );
        assert_eq!(
            bencode::encode(&bencode::decode(&body_of(&answer)).expect("bencode")),
            body_of(&answer),
            "{id} drew an answer that does not re-encode"
        );
    }
    drop(lab);

    let seen = tracker.announces();
    assert_eq!(seen.len(), heads.len());
    for (recorded, (id, head)) in seen.iter().zip(&heads) {
        assert_eq!(
            recorded.raw(),
            head.as_slice(),
            "{id} was not kept verbatim"
        );
        assert_eq!(
            recorded.request().encode(),
            head.as_slice(),
            "{id} did not re-encode from what the observer decoded"
        );
    }
}

#[test]
fn http_tracker_distinguishes_an_absent_flag_from_one_set_to_zero() {
    // BEP 23 leaves the default to the tracker, so a build that omits `compact`
    // is making a different statement from one that sends `compact=0`.
    let tracker = HttpTracker::default();
    let lab = lab_with(&tracker);
    announce(&lab, b"GET /announce?info_hash=%01 HTTP/1.1\r\n\r\n");
    announce(
        &lab,
        b"GET /announce?info_hash=%01&compact=0 HTTP/1.1\r\n\r\n",
    );
    announce(
        &lab,
        b"GET /announce?info_hash=%01&compact=1 HTTP/1.1\r\n\r\n",
    );
    drop(lab);

    let seen = tracker.announces();
    assert_eq!(seen[0].wants_compact(), None);
    assert_eq!(seen[1].wants_compact(), Some(false));
    assert_eq!(seen[2].wants_compact(), Some(true));
}

#[test]
fn http_tracker_consumes_a_body_so_the_next_request_is_not_read_at_the_wrong_offset() {
    // ⛔ A tracker announce is a GET, so this is not a shape a client sends. It
    // is the shape a framer gets wrong: leaving a body in the buffer makes the
    // next request start mid-stream, and the misframed bytes are then recorded
    // as a build that sends malformed requests.
    let tracker = HttpTracker::default();
    let lab = lab_with(&tracker);
    let address = lab.endpoint("tracker-http").expect("added").address();
    let mut client = TcpStream::connect(address).expect("the endpoint accepts");
    client
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("a read timeout is settable");

    client
        .write_all(b"POST /announce?info_hash=%01 HTTP/1.1\r\nContent-Length: 5\r\n\r\nhello")
        .expect("write");
    client.flush().expect("flush");
    let mut first = Vec::new();
    read_one_response(&mut client, &mut first);
    assert!(first.starts_with(b"HTTP/1.1 200 OK"));

    client
        .write_all(b"GET /announce?info_hash=%02&event=stopped HTTP/1.1\r\n\r\n")
        .expect("write");
    client.flush().expect("flush");
    let mut second = Vec::new();
    read_one_response(&mut client, &mut second);
    assert!(second.starts_with(b"HTTP/1.1 200 OK"));
    drop(client);
    drop(lab);

    let seen = tracker.announces();
    assert_eq!(seen.len(), 2);
    assert_eq!(
        seen[0].raw().len(),
        65,
        "the body was consumed with its head"
    );
    assert_eq!(
        seen[1]
            .decoded(b"event")
            .expect("present")
            .expect("decodes"),
        b"stopped".to_vec(),
        "the second request was read from its own first byte"
    );
}

#[test]
fn http_tracker_refuses_a_request_declaring_its_length_twice() {
    // Two lengths may disagree, and there is no reading of them that frames the
    // rest of the connection correctly.
    let tracker = HttpTracker::default();
    let lab = lab_with(&tracker);
    let answer = announce(
        &lab,
        b"POST /announce HTTP/1.1\r\nContent-Length: 5\r\nContent-Length: 9\r\n\r\nhello",
    );
    drop(lab);

    assert!(answer.starts_with(b"HTTP/1.1 400 Bad Request"));
    let decoded = bencode::decode(&body_of(&answer)).expect("bencode");
    assert!(decoded.get(b"failure reason").is_some());
    assert!(tracker.announces().is_empty());
}

#[test]
fn http_tracker_counts_the_announces_it_stopped_keeping() {
    // ⭐ A cap that silently discards leaves a record with no denominator: a
    // reader cannot tell "there were two" from "there were two thousand".
    let tracker = HttpTracker::default().with_max_announces(2);
    let lab = lab_with(&tracker);
    for index in 0..5 {
        let head = format!("GET /announce?info_hash=%0{index} HTTP/1.1\r\n\r\n");
        let answer = announce(&lab, head.as_bytes());
        assert!(
            answer.starts_with(b"HTTP/1.1 200 OK"),
            "an announce past the cap is still answered, or the client changes what it does next"
        );
    }
    drop(lab);

    assert_eq!(tracker.announces().len(), 2);
    assert_eq!(tracker.dropped(), 3);
}

#[test]
fn http_tracker_frames_a_head_terminated_entirely_with_bare_newlines() {
    // ⭐ The guard-mutation pass found this gap. Every other head here ends
    // `\r\n\r\n` or mixes the two, so the branch of the framer that handles a
    // blank line of one bare `\n` was never reached: shortening its answer by a
    // byte changed no result. A corpus only tests the defects it contains an
    // example of.
    let tracker = HttpTracker::default();
    let lab = lab_with(&tracker);
    let answer = announce(&lab, b"GET /announce?info_hash=%01 HTTP/1.1\nHost: h\n\n");
    drop(lab);

    assert!(
        answer.starts_with(b"HTTP/1.1 200 OK"),
        "got {:?}",
        String::from_utf8_lossy(&answer[..answer.len().min(64)])
    );
    let seen = tracker.announces();
    assert_eq!(seen.len(), 1);
    assert_eq!(
        seen[0].raw(),
        b"GET /announce?info_hash=%01 HTTP/1.1\nHost: h\n\n",
        "a head short by its last byte is a head that did not decode"
    );
}

#[test]
fn http_tracker_leaves_the_bytes_of_a_refused_request_in_the_lab_journal() {
    // ⛔ The observer refuses a head it cannot decode and keeps no announce for
    // it. The bytes are still the observation, and losing them would leave a
    // record saying a build sent something unreadable with nothing to read.
    // The lab records what arrived; this asserts the two halves agree on that.
    use bit_ids::canonical::Slug;

    let tracker = HttpTracker::default();
    let lab = lab_with(&tracker);
    let head = b"GET /announce HTTP/1.1\r\nnot-a-header\r\n\r\n";
    announce(&lab, head);
    let journal = lab.shutdown();

    assert!(tracker.announces().is_empty(), "it did not decode");
    let endpoint = Slug::parse("tracker-http").expect("a slug");
    assert_eq!(
        journal.received(&endpoint),
        head.to_vec(),
        "the evidence survives the refusal"
    );
}
