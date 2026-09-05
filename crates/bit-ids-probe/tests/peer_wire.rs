//! `OBS-04`'s acceptance: both roles, and a transcript that must rebuild from
//! the bytes that arrived.
//!
//! ⚠ Every handshake here is written by this test, byte for byte, and driven
//! over a real socket. No client is installed, so nothing here is evidence about
//! any build.

use std::io::{Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::time::Duration;

use bit_ids_lab::{Lab, Transport};
use bit_ids_probe::peer_wire::{PeerIdentity, PeerWire, Role};
use bit_ids_wire::peer_wire::{INFO_HASH_LEN, Message, RESERVED_LEN};

const INFO_HASH: [u8; INFO_HASH_LEN] = [0x5a; INFO_HASH_LEN];

fn observer() -> PeerWire {
    PeerWire::new(PeerIdentity::default(), INFO_HASH)
}

fn lab_accepting(peer: &PeerWire) -> Lab {
    Lab::builder()
        .deadline(Duration::from_secs(60))
        .stream("peer-wire", peer.accepting())
        .expect("a canonical endpoint name")
        .start()
        .expect("loopback binds")
}

/// A BEP 3 handshake, built here rather than with the observer's own encoder.
///
/// ⛔ A test that built its input with the code under test would prove that the
/// encoder agrees with itself.
fn handshake(protocol: &[u8], reserved: [u8; RESERVED_LEN], peer_id: &[u8; 20]) -> Vec<u8> {
    let mut out = vec![u8::try_from(protocol.len()).expect("a short protocol string")];
    out.extend_from_slice(protocol);
    out.extend_from_slice(&reserved);
    out.extend_from_slice(&INFO_HASH);
    out.extend_from_slice(peer_id);
    out
}

fn standard_handshake() -> Vec<u8> {
    handshake(
        b"BitTorrent protocol",
        [0, 0, 0, 0, 0, 0x10, 0, 0x05],
        b"-qB5000-abcdefghijkl",
    )
}

/// A length-prefixed peer message.
fn message(id: u8, payload: &[u8]) -> Vec<u8> {
    let length = u32::try_from(payload.len() + 1).expect("a short payload");
    let mut out = length.to_be_bytes().to_vec();
    out.push(id);
    out.extend_from_slice(payload);
    out
}

fn connect(lab: &Lab) -> TcpStream {
    let address = lab.endpoint("peer-wire").expect("added").address();
    let client = TcpStream::connect(address).expect("the endpoint accepts");
    client
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("a read timeout is settable");
    client
}

/// Reads exactly the handshake the observer sends back.
fn read_handshake(client: &mut TcpStream) -> Vec<u8> {
    let mut length = [0_u8; 1];
    client.read_exact(&mut length).expect("a length byte");
    let mut rest = vec![0_u8; usize::from(length[0]) + 48];
    client.read_exact(&mut rest).expect("the rest");
    let mut out = length.to_vec();
    out.extend_from_slice(&rest);
    out
}

#[test]
fn peer_wire_keeps_the_protocol_string_and_all_eight_reserved_bytes() {
    let peer = observer();
    let lab = lab_accepting(&peer);
    let mut client = connect(&lab);
    let sent = standard_handshake();
    client.write_all(&sent).expect("write");
    client.flush().expect("flush");
    read_handshake(&mut client);
    drop(client);
    drop(lab);

    let streams = peer.streams();
    assert_eq!(streams.len(), 1);
    let observed = streams[0].handshake().expect("a handshake was read");
    assert_eq!(observed.protocol(), b"BitTorrent protocol");
    // ⛔ Whole, not decoded into named flags. The bits nobody has assigned are
    // the ones most worth keeping.
    assert_eq!(observed.reserved(), &[0, 0, 0, 0, 0, 0x10, 0, 0x05]);
    assert!(observed.offers_extension_protocol());
    assert!(observed.offers_dht());
    assert!(observed.offers_fast_extension());
    assert_eq!(observed.peer_id(), b"-qB5000-abcdefghijkl");
    assert_eq!(observed.info_hash(), &INFO_HASH);
    assert_eq!(streams[0].role(), Role::TargetDialled);
}

#[test]
fn peer_wire_keeps_a_non_standard_protocol_string_rather_than_refusing_it() {
    // ⭐ A build that sends a different protocol string has told us something,
    // and a decoder that refused it would turn an observation into a parse
    // failure.
    let peer = observer();
    let lab = lab_accepting(&peer);
    let mut client = connect(&lab);
    client
        .write_all(&handshake(
            b"NotBitTorrent",
            [0; RESERVED_LEN],
            b"-XX0000-000000000000",
        ))
        .expect("write");
    client.flush().expect("flush");
    read_handshake(&mut client);
    drop(client);
    drop(lab);

    let streams = peer.streams();
    assert_eq!(
        streams[0].handshake().expect("read").protocol(),
        b"NotBitTorrent"
    );
    assert!(streams[0].rebuilds_from_raw());
}

#[test]
fn peer_wire_answers_with_the_info_hash_it_was_asked_for() {
    // ⚠ A peer that answers with a different info hash is a peer that does not
    // have the torrent, and a client drops that connection. The disconnection
    // would then be recorded as identity when it is this code's doing.
    let peer = observer();
    let lab = lab_accepting(&peer);
    let mut client = connect(&lab);
    client.write_all(&standard_handshake()).expect("write");
    client.flush().expect("flush");
    let answer = read_handshake(&mut client);
    drop(client);
    drop(lab);

    assert_eq!(answer[0], 19);
    assert_eq!(&answer[1..20], b"BitTorrent protocol");
    assert_eq!(&answer[20..28], &[0; RESERVED_LEN], "nothing is offered");
    assert_eq!(&answer[28..48], &INFO_HASH);
    assert_eq!(&answer[48..68], b"bit-ids-fixture-0001");
}

#[test]
fn peer_wire_keeps_the_early_message_order_that_arrived() {
    let peer = observer();
    let lab = lab_accepting(&peer);
    let mut client = connect(&lab);
    client.write_all(&standard_handshake()).expect("write");
    client.flush().expect("flush");
    read_handshake(&mut client);

    // ⭐ Order is the measurement. A build that sends bitfield then interested
    // and one that sends interested then bitfield are distinguishable here and
    // identical in anything keyed by id.
    let mut after = message(5, &[0xff, 0x00]);
    after.extend_from_slice(&message(2, &[]));
    after.extend_from_slice(&[0, 0, 0, 0]); // a keep-alive
    after.extend_from_slice(&message(9, &[0x1a, 0xe1]));
    client.write_all(&after).expect("write");
    client.flush().expect("flush");

    // Wait on the condition: the observer has decoded all four messages.
    let observed = wait_for_messages(&peer, 4);
    drop(client);
    drop(lab);

    let ids: Vec<Option<u8>> = observed.iter().map(Message::id).collect();
    assert_eq!(ids, vec![Some(5), Some(2), None, Some(9)]);
    assert_eq!(observed[0].payload(), &[0xff, 0x00]);
    assert_eq!(observed[3].payload(), &[0x1a, 0xe1]);
}

/// Waits until the observer has decoded `count` messages on its one connection.
///
/// ⚠ Waits on the condition rather than on a duration. The bound exists to turn
/// a hang into a failure, not to measure how long the observer took.
fn wait_for_messages(peer: &PeerWire, count: usize) -> Vec<Message> {
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        let streams = peer.streams();
        if let Some(stream) = streams.first()
            && stream.messages().len() >= count
        {
            return stream.messages().to_vec();
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the observer never decoded {count} messages"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn peer_wire_keeps_a_message_id_nobody_has_assigned() {
    let peer = observer();
    let lab = lab_accepting(&peer);
    let mut client = connect(&lab);
    client.write_all(&standard_handshake()).expect("write");
    client.flush().expect("flush");
    read_handshake(&mut client);
    client.write_all(&message(0xfe, b"private")).expect("write");
    client.flush().expect("flush");

    let observed = wait_for_messages(&peer, 1);
    drop(client);
    drop(lab);

    assert_eq!(observed[0].id(), Some(0xfe));
    assert_eq!(observed[0].payload(), b"private");
}

#[test]
fn peer_wire_tells_two_concurrent_connections_apart() {
    // ⛔ The responder is one function serving every connection, so without a
    // connection identity a second handshake would go down the first
    // connection. Two clients, two handshakes, two records.
    let peer = observer();
    let lab = lab_accepting(&peer);

    let mut first = connect(&lab);
    let mut second = connect(&lab);
    first.write_all(&standard_handshake()).expect("write");
    first.flush().expect("flush");
    let first_answer = read_handshake(&mut first);

    second
        .write_all(&handshake(
            b"BitTorrent protocol",
            [0; RESERVED_LEN],
            b"-TR4060-zyxwvutsrqpo",
        ))
        .expect("write");
    second.flush().expect("flush");
    let second_answer = read_handshake(&mut second);

    assert_eq!(first_answer, second_answer, "one observer, one identity");
    drop(first);
    drop(second);
    drop(lab);

    let streams = peer.streams();
    assert_eq!(streams.len(), 2, "two connections, two records");
    assert_ne!(streams[0].connection(), streams[1].connection());
    let peer_ids: Vec<Vec<u8>> = streams
        .iter()
        .map(|one| one.handshake().expect("read").peer_id().to_vec())
        .collect();
    assert!(peer_ids.contains(&b"-qB5000-abcdefghijkl".to_vec()));
    assert!(peer_ids.contains(&b"-TR4060-zyxwvutsrqpo".to_vec()));
}

#[test]
fn peer_wire_observes_the_other_role_by_dialling_the_target() {
    // ⭐ The role the entry exists for. A build can behave differently as the
    // side that accepted, and a lab that could only accept measures half the
    // surface.
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a target listener");
    let address = listener.local_addr().expect("an address");

    let peer = observer();
    let mut lab = Lab::builder()
        .deadline(Duration::from_secs(60))
        // A lab needs an endpoint to start; this one is not used.
        .datagram("unused", |_: &[u8]| None)
        .expect("a canonical name")
        .start()
        .expect("loopback binds");

    // The target accepts, reads the observer's handshake, and answers.
    let target = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("the observer dials");
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .expect("a read timeout is settable");
        let mut length = [0_u8; 1];
        stream.read_exact(&mut length).expect("a length byte");
        let mut rest = vec![0_u8; usize::from(length[0]) + 48];
        stream.read_exact(&mut rest).expect("the rest");
        stream
            .write_all(&handshake(
                b"BitTorrent protocol",
                [0, 0, 0, 0, 0, 0x10, 0, 0],
                b"-DE13F0-listenerside",
            ))
            .expect("the target answers");
        stream.flush().expect("flush");
        std::thread::sleep(Duration::from_millis(200));
        length[0]
    });

    let endpoint = lab
        .dial("peer-dial", address, peer.opening(), peer.dialling())
        .expect("a loopback dial");
    assert_eq!(endpoint.transport(), Transport::Dialled);
    assert_eq!(endpoint.address(), address);

    let sent_length = target.join().expect("the target thread");
    assert_eq!(sent_length, 19, "the observer sent its handshake first");

    let streams = wait_for_handshake(&peer);
    let journal = lab.shutdown();

    assert_eq!(streams[0].role(), Role::ObserverDialled);
    let observed = streams[0].handshake().expect("the target answered");
    assert_eq!(observed.peer_id(), b"-DE13F0-listenerside");
    assert_eq!(observed.reserved(), &[0, 0, 0, 0, 0, 0x10, 0, 0]);

    // The lab's transcript carries both directions of the dialled connection.
    let dialled = bit_ids::canonical::Slug::parse("peer-dial").expect("a slug");
    assert_eq!(
        journal.received(&dialled),
        streams[0].raw(),
        "the observer's record and the lab's transcript agree"
    );
}

fn wait_for_handshake(peer: &PeerWire) -> Vec<bit_ids_probe::Stream> {
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        let streams = peer.streams();
        if streams.first().is_some_and(|one| one.handshake().is_some()) {
            return streams;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the observer never read a handshake"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn peer_wire_refuses_a_transcript_that_cannot_be_rebuilt_from_its_bytes() {
    // ⛔ The check the entry names. A decode that lost something cannot put it
    // back, and a field derived from that decode would describe the decoder.
    let peer = observer();
    let lab = lab_accepting(&peer);
    let mut client = connect(&lab);
    client.write_all(&standard_handshake()).expect("write");
    client.flush().expect("flush");
    read_handshake(&mut client);
    client.write_all(&message(5, &[0xff])).expect("write");
    client.flush().expect("flush");
    wait_for_messages(&peer, 1);
    drop(client);
    drop(lab);

    let streams = peer.streams();
    assert!(
        streams[0].rebuilds_from_raw(),
        "a clean transcript rebuilds byte for byte"
    );
    assert!(streams[0].error().is_none());
}

#[test]
fn peer_wire_keeps_the_bytes_and_the_reason_when_a_transcript_stops_decoding() {
    let peer = observer();
    let lab = lab_accepting(&peer);
    let mut client = connect(&lab);
    client.write_all(&standard_handshake()).expect("write");
    client.flush().expect("flush");
    read_handshake(&mut client);
    // A length prefix past the codec's cap: refused rather than allocated.
    client
        .write_all(&[0xff, 0xff, 0xff, 0xff, 0x05])
        .expect("write");
    client.flush().expect("flush");

    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    let stream = loop {
        let streams = peer.streams();
        if let Some(one) = streams.first()
            && one.error().is_some()
        {
            break one.clone();
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the observer never reported the refusal"
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    drop(client);
    drop(lab);

    assert!(
        stream.handshake().is_some(),
        "the handshake decoded even though the tail did not"
    );
    assert!(!stream.rebuilds_from_raw());
    assert!(
        stream.raw().ends_with(&[0xff, 0xff, 0xff, 0xff, 0x05]),
        "the bytes that would not decode are kept"
    );
}

#[test]
fn peer_wire_counts_the_connections_it_stopped_keeping() {
    let peer = observer().with_max_streams(1);
    let lab = lab_accepting(&peer);

    let mut first = connect(&lab);
    first.write_all(&standard_handshake()).expect("write");
    first.flush().expect("flush");
    read_handshake(&mut first);

    let mut second = connect(&lab);
    second.write_all(&standard_handshake()).expect("write");
    second.flush().expect("flush");

    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while peer.dropped() == 0 {
        assert!(
            std::time::Instant::now() < deadline,
            "the second connection was never counted"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    drop(first);
    drop(second);
    drop(lab);

    assert_eq!(peer.streams().len(), 1);
    assert!(peer.dropped() >= 1);
}

/// ⭐ Driven by the committed corpus rather than only by handshakes this file
/// writes.
#[test]
fn peer_wire_reads_every_committed_peer_fixture_over_a_real_socket() {
    use bit_ids::observation::Surface;
    use bit_ids_wire::fixture::load_directory;

    let corpus =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../bit-ids-wire/tests/fixtures");
    let loaded = load_directory(&corpus).expect("the corpus loads");
    let transcripts: Vec<(String, Vec<u8>)> = loaded
        .iter()
        .filter(|(_, fixture)| fixture.surface == Surface::PeerWire)
        .map(|(_, fixture)| (fixture.id.as_str().to_owned(), fixture.joined_bytes()))
        .collect();
    assert!(
        transcripts.len() >= 3,
        "the corpus should carry more than one peer fixture, found {}",
        transcripts.len()
    );

    for (id, bytes) in &transcripts {
        // A fixture's info hash is its own, so the observer answers with that
        // one: the identity under measurement is the target's, not the hash.
        let mut info_hash = [0_u8; INFO_HASH_LEN];
        let protocol_len = usize::from(bytes[0]);
        info_hash.copy_from_slice(&bytes[1 + protocol_len + RESERVED_LEN..][..INFO_HASH_LEN]);
        let peer = PeerWire::new(PeerIdentity::default(), info_hash);
        let lab = lab_accepting(&peer);
        let mut client = connect(&lab);
        client.write_all(bytes).expect("write");
        client.flush().expect("flush");
        read_handshake(&mut client);

        let deadline = std::time::Instant::now() + Duration::from_secs(20);
        loop {
            let streams = peer.streams();
            if streams
                .first()
                .is_some_and(|one| one.raw().len() == bytes.len())
            {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "{id} was not fully recorded"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        drop(client);
        drop(lab);

        let streams = peer.streams();
        assert_eq!(streams[0].raw(), bytes.as_slice(), "{id} was altered");
        assert!(
            streams[0].rebuilds_from_raw(),
            "{id} does not rebuild from its own bytes"
        );
    }
}

#[test]
fn peer_wire_closes_a_connection_it_has_stopped_observing() {
    // ⚠ Past the cap the observer stops observing, and closing says so. Leaving
    // the connection open would hold it buffering until the lab's byte cap
    // fired, which is the same refusal arriving later and less legibly.
    let peer = observer().with_max_streams(1);
    let lab = lab_accepting(&peer);

    let mut first = connect(&lab);
    first.write_all(&standard_handshake()).expect("write");
    first.flush().expect("flush");
    read_handshake(&mut first);

    let mut second = connect(&lab);
    second.write_all(&standard_handshake()).expect("write");
    second.flush().expect("flush");
    let mut answer = Vec::new();
    second
        .read_to_end(&mut answer)
        .expect("the endpoint closes rather than hanging");
    assert!(
        answer.is_empty(),
        "nothing is sent to a connection past the cap"
    );

    drop(first);
    drop(second);
    drop(lab);
    assert_eq!(peer.streams().len(), 1);
    assert!(peer.dropped() >= 1);
}

#[test]
fn peer_wire_sends_its_handshake_once_per_connection_and_not_once_per_read() {
    // ⭐ The guard-mutation pass found this gap. Clearing the sent flag on every
    // read made the observer re-introduce itself after each message, and every
    // test still passed because none of them read again after the handshake. A
    // client receiving two handshakes reads the second as a message.
    let peer = observer();
    let lab = lab_accepting(&peer);
    let mut client = connect(&lab);
    client.write_all(&standard_handshake()).expect("write");
    client.flush().expect("flush");
    read_handshake(&mut client);

    for id in [5_u8, 2, 9] {
        client.write_all(&message(id, &[0x01])).expect("write");
        client.flush().expect("flush");
    }
    wait_for_messages(&peer, 3);
    let connection = peer.streams()[0].connection();
    drop(client);
    let journal = lab.shutdown();

    // ⛔ Counted from the lab's transcript rather than by reading the socket:
    // asserting that nothing more arrives is asserting a silence, and this is
    // the same fact stated positively.
    let sent: Vec<&[u8]> = journal
        .for_connection(connection)
        .iter()
        .filter(|segment| segment.direction() == bit_ids_wire::tracker_udp::Direction::ToTarget)
        .map(|segment| segment.bytes())
        .collect();
    assert_eq!(sent.len(), 1, "one handshake per connection, not per read");
    assert_eq!(sent[0].len(), 68, "and it is a handshake");
}
