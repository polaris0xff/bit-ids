//! `OBS-03`'s acceptance: connect, announce, timeout, retry, key, event,
//! `num_want`, and the rejection cases.
//!
//! ⚠ Every datagram here is written by this test, byte for byte, and driven over
//! a real UDP socket. No client is installed, so nothing here is evidence about
//! any build.

use std::net::{Ipv4Addr, UdpSocket};
use std::time::Duration;

use bit_ids_lab::Lab;
use bit_ids_probe::tracker_udp::{FIRST_CONNECTION_ID, UdpTracker, UdpTrackerResponse};
use bit_ids_probe::{OfferedPeer, Refusal};
use bit_ids_wire::tracker_udp::{ANNOUNCE_REQUEST_LEN, Action, PROTOCOL_ID};

fn lab_with(tracker: &UdpTracker) -> Lab {
    Lab::builder()
        .deadline(Duration::from_secs(60))
        .datagram("tracker-udp", tracker.responder())
        .expect("a canonical endpoint name")
        .start()
        .expect("loopback binds")
}

/// A client socket bound on loopback with a read timeout.
fn client() -> UdpSocket {
    client_waiting(Duration::from_secs(10))
}

/// A client socket for a case that expects no answer.
///
/// ⚠ The absence of a datagram is the one condition that cannot be waited on,
/// so this is the one place a duration is unavoidable. A loopback round trip
/// here is measured in microseconds, so a second is five orders of magnitude of
/// margin, and every one of these cases also asserts a positive fact about what
/// the observer recorded rather than resting on the silence alone.
fn client_expecting_silence() -> UdpSocket {
    client_waiting(Duration::from_secs(1))
}

fn client_waiting(timeout: Duration) -> UdpSocket {
    let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).expect("a client socket binds");
    socket
        .set_read_timeout(Some(timeout))
        .expect("a read timeout is settable");
    socket
}

/// Sends one datagram and returns the answer, or `None` when none came.
fn exchange(lab: &Lab, socket: &UdpSocket, packet: &[u8]) -> Option<Vec<u8>> {
    let address = lab.endpoint("tracker-udp").expect("added").address();
    socket.send_to(packet, address).expect("the endpoint is up");
    let mut buffer = vec![0_u8; 2048];
    match socket.recv_from(&mut buffer) {
        Ok((read, _)) => {
            buffer.truncate(read);
            Some(buffer)
        }
        Err(_) => None,
    }
}

fn connect_request(transaction_id: u32) -> Vec<u8> {
    let mut out = PROTOCOL_ID.to_be_bytes().to_vec();
    out.extend_from_slice(&Action::Connect.code().to_be_bytes());
    out.extend_from_slice(&transaction_id.to_be_bytes());
    out
}

/// A BEP 15 announce request, all 98 bytes of it.
#[expect(clippy::too_many_arguments, reason = "BEP 15 fixes the field list")]
fn announce_request(
    connection_id: u64,
    transaction_id: u32,
    info_hash: [u8; 20],
    peer_id: [u8; 20],
    event: u32,
    key: u32,
    num_want: i32,
    port: u16,
) -> Vec<u8> {
    let mut out = connection_id.to_be_bytes().to_vec();
    out.extend_from_slice(&Action::Announce.code().to_be_bytes());
    out.extend_from_slice(&transaction_id.to_be_bytes());
    out.extend_from_slice(&info_hash);
    out.extend_from_slice(&peer_id);
    out.extend_from_slice(&0_u64.to_be_bytes()); // downloaded
    out.extend_from_slice(&0_u64.to_be_bytes()); // left
    out.extend_from_slice(&0_u64.to_be_bytes()); // uploaded
    out.extend_from_slice(&event.to_be_bytes());
    out.extend_from_slice(&0_u32.to_be_bytes()); // ip
    out.extend_from_slice(&key.to_be_bytes());
    out.extend_from_slice(&num_want.to_be_bytes());
    out.extend_from_slice(&port.to_be_bytes());
    assert_eq!(out.len(), ANNOUNCE_REQUEST_LEN, "BEP 15 fixes the width");
    out
}

/// Connects, and returns the connection id the tracker issued.
fn connect(lab: &Lab, socket: &UdpSocket, transaction_id: u32) -> u64 {
    let answer = exchange(lab, socket, &connect_request(transaction_id)).expect("an answer");
    assert_eq!(answer.len(), 16, "a connect response is sixteen bytes");
    assert_eq!(&answer[..4], &Action::Connect.code().to_be_bytes());
    assert_eq!(&answer[4..8], &transaction_id.to_be_bytes());
    u64::from_be_bytes(answer[8..16].try_into().expect("eight bytes"))
}

#[test]
fn udp_tracker_answers_a_connect_with_the_transaction_id_that_asked() {
    let tracker = UdpTracker::default();
    let lab = lab_with(&tracker);
    let socket = client();
    let id = connect(&lab, &socket, 0xdead_beef);
    drop(lab);

    assert_eq!(
        id, FIRST_CONNECTION_ID,
        "the first id is the base of the range"
    );
    assert_eq!(tracker.issued_connection_ids(), 1);
    let seen = tracker.datagrams();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].action(), Some(Action::Connect));
    assert!(tracker.refusals().is_empty());
}

#[test]
fn udp_tracker_keeps_every_announce_field_as_it_arrived() {
    let tracker = UdpTracker::default();
    let lab = lab_with(&tracker);
    let socket = client();
    let id = connect(&lab, &socket, 1);

    let info_hash = [0xab_u8; 20];
    let peer_id = *b"-qB5000-abcdefghijkl";
    let request = announce_request(id, 2, info_hash, peer_id, 2, 0x1234_5678, -1, 6881);
    let answer = exchange(&lab, &socket, &request).expect("an answer");
    drop(lab);

    assert_eq!(&answer[..4], &Action::Announce.code().to_be_bytes());
    assert_eq!(answer.len(), 20, "no peers were offered");

    let seen = tracker.datagrams();
    assert_eq!(seen.len(), 2);
    assert_eq!(
        seen[1].raw(),
        request.as_slice(),
        "the bytes are kept verbatim"
    );
    let announce = seen[1]
        .announce()
        .expect("it is an announce")
        .expect("it decodes");
    assert_eq!(announce.info_hash, info_hash);
    assert_eq!(announce.peer_id, peer_id);
    assert_eq!(
        announce.key, 0x1234_5678,
        "the key is a per-client identity value"
    );
    assert_eq!(announce.event, 2, "two is started");
    assert_eq!(announce.port, 6881);
    // ⛔ Signed. Read as unsigned, `-1` becomes 4294967295 and a record says a
    // client asked for four billion peers when it asked for the default.
    assert_eq!(announce.num_want, -1);
    assert!(announce.options.is_empty());
}

#[test]
fn udp_tracker_keeps_the_bep_41_options_a_client_appends() {
    // ⭐ An announce is 98 bytes and a client that sends more is sending BEP 41
    // request-string options. Ignoring the surplus discards an identity signal;
    // reading it as part of a fixed field would corrupt one.
    let tracker = UdpTracker::default();
    let lab = lab_with(&tracker);
    let socket = client();
    let id = connect(&lab, &socket, 1);

    let mut request = announce_request(id, 2, [1; 20], [2; 20], 0, 7, 50, 51413);
    request.extend_from_slice(&[
        0x02, 0x09, b'/', b'a', b'n', b'n', b'o', b'u', b'n', b'c', b'e',
    ]);
    exchange(&lab, &socket, &request).expect("an answer");
    drop(lab);

    let announce = tracker.datagrams()[1]
        .announce()
        .expect("an announce")
        .expect("it decodes");
    assert_eq!(announce.num_want, 50);
    assert_eq!(
        announce.options,
        vec![
            0x02, 0x09, b'/', b'a', b'n', b'n', b'o', b'u', b'n', b'c', b'e'
        ]
    );
}

#[test]
fn udp_tracker_answers_six_bytes_per_peer_and_nothing_else() {
    let tracker = UdpTracker::new(UdpTrackerResponse {
        interval: 900,
        leechers: 3,
        seeders: 4,
        peers: vec![
            OfferedPeer {
                address: [127, 0, 0, 1],
                port: 6881,
                peer_id: *b"bit-ids-fixture-0001",
            },
            OfferedPeer {
                address: [127, 0, 0, 2],
                port: 51413,
                peer_id: *b"bit-ids-fixture-0002",
            },
        ],
    });
    let lab = lab_with(&tracker);
    let socket = client();
    let id = connect(&lab, &socket, 1);
    let answer = exchange(
        &lab,
        &socket,
        &announce_request(id, 2, [1; 20], [2; 20], 0, 0, -1, 6881),
    )
    .expect("an answer");
    drop(lab);

    // BEP 15 has no non-compact form: twenty bytes of header and six per peer.
    assert_eq!(answer.len(), 20 + 6 * 2);
    assert_eq!(i32::from_be_bytes(answer[8..12].try_into().unwrap()), 900);
    assert_eq!(i32::from_be_bytes(answer[12..16].try_into().unwrap()), 3);
    assert_eq!(i32::from_be_bytes(answer[16..20].try_into().unwrap()), 4);
    assert_eq!(&answer[20..26], &[127, 0, 0, 1, 0x1a, 0xe1]);
    assert_eq!(&answer[26..32], &[127, 0, 0, 2, 0xc8, 0xd5]);
}

#[test]
fn udp_tracker_refuses_an_announce_carrying_an_id_it_never_issued() {
    // ⭐ Not an error to hide. A build that announces with an id this tracker
    // never handed out reused a stale one, invented one, or skipped the connect,
    // and each is an observation.
    let tracker = UdpTracker::default();
    let lab = lab_with(&tracker);
    let socket = client();
    let answer = exchange(
        &lab,
        &socket,
        &announce_request(0xffff_ffff_ffff_ffff, 7, [1; 20], [2; 20], 0, 0, -1, 6881),
    )
    .expect("an answer");
    drop(lab);

    assert_eq!(&answer[..4], &Action::Error.code().to_be_bytes());
    assert_eq!(
        &answer[4..8],
        &7_u32.to_be_bytes(),
        "the error echoes the transaction"
    );
    assert_eq!(tracker.refusals(), vec![Refusal::UnknownConnectionId]);
    assert_eq!(
        tracker.datagrams().len(),
        1,
        "a refused announce is still recorded"
    );
}

#[test]
fn udp_tracker_refuses_a_connect_that_does_not_open_with_the_protocol_id() {
    let tracker = UdpTracker::default();
    let lab = lab_with(&tracker);
    let socket = client();
    let mut wrong = 0x0000_0000_0000_0001_u64.to_be_bytes().to_vec();
    wrong.extend_from_slice(&Action::Connect.code().to_be_bytes());
    wrong.extend_from_slice(&9_u32.to_be_bytes());
    let answer = exchange(&lab, &socket, &wrong).expect("an answer");
    drop(lab);

    assert_eq!(&answer[..4], &Action::Error.code().to_be_bytes());
    assert_eq!(tracker.refusals(), vec![Refusal::WrongProtocolId]);
    assert_eq!(
        tracker.issued_connection_ids(),
        0,
        "a refused connect hands out nothing"
    );
}

#[test]
fn udp_tracker_refuses_an_action_bep_15_does_not_define() {
    let tracker = UdpTracker::default();
    let lab = lab_with(&tracker);
    let socket = client();
    let mut unknown = 0_u64.to_be_bytes().to_vec();
    unknown.extend_from_slice(&99_u32.to_be_bytes());
    unknown.extend_from_slice(&11_u32.to_be_bytes());
    let answer = exchange(&lab, &socket, &unknown).expect("an answer");
    drop(lab);

    assert_eq!(&answer[..4], &Action::Error.code().to_be_bytes());
    assert_eq!(tracker.refusals(), vec![Refusal::UnknownAction]);
    assert_eq!(tracker.datagrams()[0].action(), Some(Action::Other(99)));
}

#[test]
fn udp_tracker_records_a_datagram_it_cannot_decode_and_does_not_answer_it() {
    // ⛔ The error action needs a transaction id, and a datagram this short does
    // not carry one this observer can trust. Answering with a guess would put
    // bytes on the wire that no request asked for.
    let tracker = UdpTracker::default();
    let lab = lab_with(&tracker);
    let socket = client_expecting_silence();
    let answer = exchange(&lab, &socket, &[0x00, 0x01, 0x02]);
    let journal = lab.shutdown();

    assert!(answer.is_none(), "a truncated datagram gets no answer");
    let seen = tracker.datagrams();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].raw(), &[0x00, 0x01, 0x02]);
    assert!(
        seen[0].decoded().is_err(),
        "the reason it could not be read is kept with it"
    );
    // The bytes are in the lab's transcript too, which is the evidence.
    let endpoint = bit_ids::canonical::Slug::parse("tracker-udp").expect("a slug");
    assert_eq!(journal.received(&endpoint), vec![0x00, 0x01, 0x02]);
}

#[test]
fn udp_tracker_refuses_an_announce_shorter_than_bep_15_fixes() {
    let tracker = UdpTracker::default();
    let lab = lab_with(&tracker);
    let socket = client_expecting_silence();
    let id = connect(&lab, &socket, 1);
    let mut short = announce_request(id, 2, [1; 20], [2; 20], 0, 0, -1, 6881);
    short.truncate(ANNOUNCE_REQUEST_LEN - 1);
    let answer = exchange(&lab, &socket, &short);
    drop(lab);

    // The codec refuses it, so there is no transaction id to answer with.
    assert!(answer.is_none());
    assert!(tracker.datagrams()[1].decoded().is_err());
}

#[test]
fn udp_tracker_gives_a_retried_connect_a_new_id_and_honours_both() {
    // A BEP 15 client retries a connect it thinks was lost, with the same
    // transaction id. Both ids are live, and an announce with either is
    // answered: refusing the earlier one would record a build as sending a
    // stale id when this tracker had reissued behind it.
    let tracker = UdpTracker::default();
    let lab = lab_with(&tracker);
    let socket = client();
    let first = connect(&lab, &socket, 42);
    let second = connect(&lab, &socket, 42);
    assert_ne!(first, second, "each connect hands out its own id");

    for id in [first, second] {
        let answer = exchange(
            &lab,
            &socket,
            &announce_request(id, 43, [1; 20], [2; 20], 0, 0, -1, 6881),
        )
        .expect("an answer");
        assert_eq!(&answer[..4], &Action::Announce.code().to_be_bytes());
    }
    drop(lab);

    assert_eq!(tracker.issued_connection_ids(), 2);
    assert!(tracker.refusals().is_empty());
    assert_eq!(tracker.datagrams().len(), 4);
}

#[test]
fn udp_tracker_answers_a_scrape_with_twelve_bytes_per_hash() {
    let tracker = UdpTracker::default();
    let lab = lab_with(&tracker);
    let socket = client();
    let id = connect(&lab, &socket, 1);

    let mut scrape = id.to_be_bytes().to_vec();
    scrape.extend_from_slice(&Action::Scrape.code().to_be_bytes());
    scrape.extend_from_slice(&3_u32.to_be_bytes());
    scrape.extend_from_slice(&[7_u8; 20]);
    scrape.extend_from_slice(&[8_u8; 20]);
    let answer = exchange(&lab, &socket, &scrape).expect("an answer");
    drop(lab);

    assert_eq!(&answer[..4], &Action::Scrape.code().to_be_bytes());
    assert_eq!(answer.len(), 8 + 12 * 2, "the width follows what arrived");
    assert!(tracker.refusals().is_empty());
}

#[test]
fn udp_tracker_records_a_client_that_connects_and_never_announces() {
    // The timeout case. A build that connects and stops is a build that decided
    // not to announce, and the record has to be able to say so: an empty record
    // and a connect-only record are different measurements.
    let tracker = UdpTracker::default();
    let mut lab = Lab::builder()
        .deadline(Duration::from_millis(300))
        .poll(Duration::from_millis(5))
        .datagram("tracker-udp", tracker.responder())
        .expect("a canonical endpoint name")
        .start()
        .expect("loopback binds");

    let socket = client();
    connect(&lab, &socket, 5);
    lab.wait();
    assert!(lab.deadline_expired(), "the deadline ended the run");
    drop(lab.shutdown());

    let seen = tracker.datagrams();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].action(), Some(Action::Connect));
    assert!(
        seen.iter().all(|one| one.announce().is_none()),
        "nothing announced"
    );
}

#[test]
fn udp_tracker_counts_the_datagrams_it_stopped_keeping() {
    let tracker = UdpTracker::default().with_max_datagrams(2);
    let lab = lab_with(&tracker);
    let socket = client();
    for transaction in 0..5_u32 {
        let answer = exchange(&lab, &socket, &connect_request(transaction))
            .expect("a datagram past the cap is still answered");
        assert_eq!(&answer[..4], &Action::Connect.code().to_be_bytes());
    }
    drop(lab);

    assert_eq!(tracker.datagrams().len(), 2);
    assert_eq!(tracker.dropped(), 3);
    assert_eq!(
        tracker.issued_connection_ids(),
        5,
        "answering is not the same as keeping"
    );
}

/// ⭐ The observer is driven by the committed corpus, not only by datagrams this
/// file writes.
#[test]
fn udp_tracker_reads_every_committed_udp_fixture_frame_over_a_real_socket() {
    use bit_ids::observation::Surface;
    use bit_ids_wire::fixture::load_directory;

    let corpus =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../bit-ids-wire/tests/fixtures");
    let loaded = load_directory(&corpus).expect("the corpus loads");
    let frames: Vec<(String, Vec<u8>)> = loaded
        .iter()
        .filter(|(_, fixture)| fixture.surface == Surface::TrackerUdp)
        .flat_map(|(_, fixture)| {
            fixture.frames.iter().map(|frame| {
                (
                    fixture.id.as_str().to_owned(),
                    frame.bytes.as_slice().to_vec(),
                )
            })
        })
        .collect();
    assert!(
        frames.len() >= 3,
        "the corpus should carry more than one UDP frame, found {}",
        frames.len()
    );

    let tracker = UdpTracker::default();
    let lab = lab_with(&tracker);
    let socket = client_expecting_silence();
    for (id, frame) in &frames {
        // ⚠ A fixture frame is a datagram the target emitted, which may be an
        // announce carrying a connection id this observer never issued. What is
        // asserted is that every frame is recorded with its bytes intact, not
        // that every one is answered: refusing one is itself correct.
        let _ = exchange(&lab, &socket, frame);
        let seen = tracker.datagrams();
        assert_eq!(
            seen.last().expect("something was recorded").raw(),
            frame.as_slice(),
            "{id} was not kept verbatim"
        );
    }
    drop(lab);
    assert_eq!(tracker.datagrams().len(), frames.len());
}

#[test]
fn udp_tracker_bounds_the_refusal_list_as_well_as_the_datagram_list() {
    // ⭐ The door sweep found the bound on one of two lists that grow together.
    // A build sending garbage in a loop is answered every time, so the refusals
    // grew without limit while the datagrams were capped.
    let tracker = UdpTracker::default().with_max_datagrams(2);
    let lab = lab_with(&tracker);
    let socket = client();
    for transaction in 0..6_u32 {
        let mut unknown = 0_u64.to_be_bytes().to_vec();
        unknown.extend_from_slice(&99_u32.to_be_bytes());
        unknown.extend_from_slice(&transaction.to_be_bytes());
        let answer = exchange(&lab, &socket, &unknown).expect("every one is answered");
        assert_eq!(&answer[..4], &Action::Error.code().to_be_bytes());
    }
    drop(lab);

    assert_eq!(tracker.datagrams().len(), 2);
    assert_eq!(tracker.dropped(), 4);
    assert_eq!(
        tracker.refusals().len(),
        2,
        "the refusals are bounded by the same cap"
    );
}

#[test]
fn udp_tracker_reads_the_connection_id_through_one_path_for_every_action() {
    // The announce arm took the id from the decoded request and the scrape arm
    // read the same eight bytes itself. Two spans for one field is the shape a
    // fix in one copy never reaches, so both now go through the codec.
    let tracker = UdpTracker::default();
    let lab = lab_with(&tracker);
    let socket = client();
    let id = connect(&lab, &socket, 1);

    let mut scrape = 0xdead_beef_dead_beef_u64.to_be_bytes().to_vec();
    scrape.extend_from_slice(&Action::Scrape.code().to_be_bytes());
    scrape.extend_from_slice(&2_u32.to_be_bytes());
    scrape.extend_from_slice(&[7_u8; 20]);
    let refused = exchange(&lab, &socket, &scrape).expect("an answer");
    assert_eq!(&refused[..4], &Action::Error.code().to_be_bytes());

    let mut good = id.to_be_bytes().to_vec();
    good.extend_from_slice(&Action::Scrape.code().to_be_bytes());
    good.extend_from_slice(&3_u32.to_be_bytes());
    good.extend_from_slice(&[7_u8; 20]);
    let accepted = exchange(&lab, &socket, &good).expect("an answer");
    assert_eq!(&accepted[..4], &Action::Scrape.code().to_be_bytes());
    drop(lab);

    assert_eq!(tracker.refusals(), vec![Refusal::UnknownConnectionId]);

    // ⛔ A connect request has no connection id. Its first eight bytes are the
    // BEP 15 protocol id, and reporting that as a connection id hands a caller
    // a value that looks like one: this assertion was written the other way
    // round and the failure is what found it.
    let seen = tracker.datagrams();
    let connect = seen[0].decoded().expect("the connect decoded");
    assert_eq!(connect.connection_id(), None);
    assert!(connect.opens_with_protocol_id());

    let scrape = seen[1].decoded().expect("the scrape decoded");
    assert_eq!(scrape.connection_id(), Some(0xdead_beef_dead_beef));
}
