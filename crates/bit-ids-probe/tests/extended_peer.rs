//! `OBS-05`'s acceptance: the BEP 10 handshake, unknown extension keys,
//! ordering, and size limits.
//!
//! ⚠ Every message here is written by this test, byte for byte, and driven over
//! a real socket. No client is installed, so nothing here is evidence about any
//! build.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use bit_ids_lab::Lab;
use bit_ids_probe::peer_wire::{ExtendedOffer, ExtensionProtocol, Offer, PeerWire};
use bit_ids_wire::bencode::{self, Value};
use bit_ids_wire::peer_wire::{
    EXTENDED_HANDSHAKE_ID, EXTENDED_MESSAGE_ID, INFO_HASH_LEN, Message, RESERVED_LEN,
};

const INFO_HASH: [u8; INFO_HASH_LEN] = [0x5a; INFO_HASH_LEN];

/// The offer this suite makes unless a test varies it.
fn full_offer() -> Offer {
    Offer {
        extension_protocol: ExtensionProtocol::Offered(ExtendedOffer {
            extensions: vec![(b"ut_metadata".to_vec(), 1), (b"ut_pex".to_vec(), 2)],
            client: Some(b"bit-ids-fixture/0".to_vec()),
            request_queue: Some(250),
            metadata_size: None,
        }),
        dht: false,
        fast: false,
    }
}

fn lab_with(peer: &PeerWire) -> Lab {
    Lab::builder()
        .deadline(Duration::from_secs(60))
        .stream("peer-wire", peer.accepting())
        .expect("a canonical endpoint name")
        .start()
        .expect("loopback binds")
}

fn handshake(reserved: [u8; RESERVED_LEN], peer_id: &[u8; 20]) -> Vec<u8> {
    let protocol = b"BitTorrent protocol";
    let mut out = vec![u8::try_from(protocol.len()).expect("short")];
    out.extend_from_slice(protocol);
    out.extend_from_slice(&reserved);
    out.extend_from_slice(&INFO_HASH);
    out.extend_from_slice(peer_id);
    out
}

/// A reserved block offering the extension protocol.
const OFFERS_BEP10: [u8; RESERVED_LEN] = [0, 0, 0, 0, 0, 0x10, 0, 0];
/// A reserved block offering nothing.
const OFFERS_NOTHING: [u8; RESERVED_LEN] = [0; RESERVED_LEN];

/// An extended message, built here rather than with the code under test.
fn extended(extended_id: u8, document: &Value) -> Vec<u8> {
    let mut payload = vec![extended_id];
    payload.extend_from_slice(&bencode::encode(document));
    let length = u32::try_from(payload.len() + 1).expect("short");
    let mut out = length.to_be_bytes().to_vec();
    out.push(EXTENDED_MESSAGE_ID);
    out.extend_from_slice(&payload);
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

fn read_exactly(client: &mut TcpStream, count: usize) -> Vec<u8> {
    let mut out = vec![0_u8; count];
    client.read_exact(&mut out).expect("the endpoint answers");
    out
}

/// Waits until the observer has decoded the target's extended handshake.
///
/// ⚠ Waits on the condition. The bound turns a hang into a failure and is not a
/// measurement of how long the observer took.
fn wait_for_extended(peer: &PeerWire) {
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while peer
        .streams()
        .first()
        .is_none_or(|one| one.extended_handshake().is_none())
    {
        assert!(
            std::time::Instant::now() < deadline,
            "the observer never decoded an extended handshake"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn extended_peer_offers_bep10_in_the_reserved_block_it_says_it_offers() {
    // ⛔ The two halves are derived from one set of flags, so a run that says it
    // offered BEP 10 cannot have sent a zero reserved block.
    let peer = PeerWire::offering(full_offer(), INFO_HASH);
    assert!(peer.offer().extension_protocol.is_offered());

    let lab = lab_with(&peer);
    let mut client = connect(&lab);
    client
        .write_all(&handshake(OFFERS_BEP10, b"-qB5000-abcdefghijkl"))
        .expect("write");
    client.flush().expect("flush");
    let answer = read_exactly(&mut client, 68);
    drop(client);
    drop(lab);

    assert_eq!(
        &answer[20..28],
        &OFFERS_BEP10,
        "the observer offered the protocol it says it did"
    );
}

#[test]
fn extended_peer_sends_its_extended_handshake_and_keeps_what_came_back() {
    let peer = PeerWire::offering(full_offer(), INFO_HASH);
    let lab = lab_with(&peer);
    let mut client = connect(&lab);
    client
        .write_all(&handshake(OFFERS_BEP10, b"-qB5000-abcdefghijkl"))
        .expect("write");
    client.flush().expect("flush");
    read_exactly(&mut client, 68);

    // The observer's own extended handshake follows its handshake.
    let length_bytes = read_exactly(&mut client, 4);
    let length = u32::from_be_bytes(length_bytes.try_into().expect("four bytes"));
    let body = read_exactly(
        &mut client,
        usize::try_from(length).expect("a short message"),
    );
    assert_eq!(body[0], EXTENDED_MESSAGE_ID);
    assert_eq!(body[1], EXTENDED_HANDSHAKE_ID);
    let document = bencode::decode(&body[2..]).expect("the observer sends bencode");
    // ⭐ Canonical, so it re-encodes to what it was built from.
    assert_eq!(bencode::encode(&document), body[2..].to_vec());
    assert_eq!(document.keys_are_sorted(), Some(true));
    let Some(Value::Dictionary(map)) = document.get(b"m") else {
        panic!("an extended handshake carries an m map");
    };
    let names: Vec<&[u8]> = map.iter().map(|(name, _)| name.as_slice()).collect();
    assert_eq!(names, vec![&b"ut_metadata"[..], b"ut_pex"]);
    // ⭐ The rest of what was offered, because it is a run condition a record
    // cites: the guard-mutation pass dropped `v` and `reqq` from the outgoing
    // handshake and every test still passed.
    assert_eq!(
        document.get(b"v"),
        Some(&Value::bytes(b"bit-ids-fixture/0".to_vec()))
    );
    assert_eq!(document.get(b"reqq"), Some(&Value::integer(250)));
    assert_eq!(
        document.get(b"metadata_size"),
        None,
        "nothing was offered for it, so nothing claims one"
    );

    // Now the target answers with its own.
    let theirs = Value::Dictionary(vec![
        (
            b"m".to_vec(),
            Value::Dictionary(vec![
                (b"ut_pex".to_vec(), Value::integer(1)),
                (b"ut_metadata".to_vec(), Value::integer(3)),
                (b"lt_donthave".to_vec(), Value::integer(7)),
            ]),
        ),
        (b"metadata_size".to_vec(), Value::integer(1234)),
        (b"reqq".to_vec(), Value::integer(500)),
        (b"v".to_vec(), Value::bytes(b"qBittorrent/5.0.0".to_vec())),
        (b"yourip".to_vec(), Value::bytes(vec![127, 0, 0, 1])),
    ]);
    client.write_all(&extended(0, &theirs)).expect("write");
    client.flush().expect("flush");
    wait_for_extended(&peer);
    drop(client);
    drop(lab);

    let streams = peer.streams();
    let observed = streams[0]
        .extended_handshake()
        .expect("one arrived")
        .expect("it decoded");
    // ⭐ Order as sent. The target's map is deliberately unsorted, and a decoder
    // that sorted it would erase a difference between builds.
    assert_eq!(
        observed.extension_ids(),
        vec![
            (b"ut_pex".to_vec(), 1),
            (b"ut_metadata".to_vec(), 3),
            (b"lt_donthave".to_vec(), 7),
        ]
    );
    assert_eq!(
        observed.advertised_client(),
        Some(&b"qBittorrent/5.0.0"[..])
    );
    assert_eq!(observed.integer(b"reqq"), Some(500));
    assert_eq!(observed.integer(b"metadata_size"), Some(1234));
    // ⛔ The undecoded bytes are kept beside the decode, and re-encode to them.
    assert_eq!(
        bencode::encode(observed.document()),
        observed.raw().to_vec()
    );
}

#[test]
fn extended_peer_keeps_an_extension_key_nobody_has_registered() {
    // ⭐ An unknown key is one of the more informative things a build can send,
    // and a parser that only reads the keys it knows discards it.
    let peer = PeerWire::offering(full_offer(), INFO_HASH);
    let lab = lab_with(&peer);
    let mut client = connect(&lab);
    client
        .write_all(&handshake(OFFERS_BEP10, b"-XX0000-000000000000"))
        .expect("write");
    client.flush().expect("flush");
    read_exactly(&mut client, 68);
    let length_bytes = read_exactly(&mut client, 4);
    let length = u32::from_be_bytes(length_bytes.try_into().expect("four bytes"));
    read_exactly(&mut client, usize::try_from(length).expect("short"));

    let theirs = Value::Dictionary(vec![
        (
            b"m".to_vec(),
            Value::Dictionary(vec![(b"x_private_thing".to_vec(), Value::integer(42))]),
        ),
        (
            b"an_unregistered_key".to_vec(),
            Value::bytes(b"kept".to_vec()),
        ),
    ]);
    client.write_all(&extended(0, &theirs)).expect("write");
    client.flush().expect("flush");
    wait_for_extended(&peer);
    drop(client);
    drop(lab);

    let streams = peer.streams();
    let observed = streams[0]
        .extended_handshake()
        .expect("one arrived")
        .expect("it decoded");
    assert_eq!(
        observed.extension_ids(),
        vec![(b"x_private_thing".to_vec(), 42)]
    );
    assert!(
        observed.document().get(b"an_unregistered_key").is_some(),
        "a top-level key nobody registered is kept too"
    );
}

#[test]
fn extended_peer_does_not_negotiate_with_a_target_that_did_not_offer() {
    // ⛔ Sending an extended handshake to a peer that never set the bit is this
    // observer inventing a negotiation, and whatever the build did about it
    // would be recorded as identity.
    let peer = PeerWire::offering(full_offer(), INFO_HASH);
    let lab = lab_with(&peer);
    let mut client = connect(&lab);
    client
        .write_all(&handshake(OFFERS_NOTHING, b"-XX0000-000000000000"))
        .expect("write");
    client.flush().expect("flush");
    let answer = read_exactly(&mut client, 68);
    assert_eq!(answer.len(), 68);
    drop(client);
    let journal = lab.shutdown();

    let streams = peer.streams();
    assert!(!streams[0].offers_extension_protocol());
    let connection = streams[0].connection();
    let sent: usize = journal
        .for_connection(connection)
        .iter()
        .filter(|segment| segment.direction() == bit_ids_wire::tracker_udp::Direction::ToTarget)
        .map(|segment| segment.bytes().len())
        .sum();
    assert_eq!(sent, 68, "the handshake and nothing after it");
}

#[test]
fn extended_peer_offers_nothing_when_the_run_offers_nothing() {
    // The other half of varying one feature at a time: an observer that offers
    // no extension protocol sends a zero reserved block and no extended
    // handshake, whatever the target offered.
    let peer = PeerWire::offering(Offer::default(), INFO_HASH);
    let lab = lab_with(&peer);
    let mut client = connect(&lab);
    client
        .write_all(&handshake(OFFERS_BEP10, b"-qB5000-abcdefghijkl"))
        .expect("write");
    client.flush().expect("flush");
    let answer = read_exactly(&mut client, 68);
    drop(client);
    let journal = lab.shutdown();

    assert_eq!(&answer[20..28], &OFFERS_NOTHING);
    let connection = peer.streams()[0].connection();
    let sent: usize = journal
        .for_connection(connection)
        .iter()
        .filter(|segment| segment.direction() == bit_ids_wire::tracker_udp::Direction::ToTarget)
        .map(|segment| segment.bytes().len())
        .sum();
    assert_eq!(sent, 68);
    // ⭐ And the target still offered it, which is the measurement: what a build
    // offers does not depend on what it was offered.
    assert!(peer.streams()[0].offers_extension_protocol());
}

#[test]
fn extended_peer_refuses_a_message_past_the_size_limit_and_keeps_the_bytes() {
    let peer = PeerWire::offering(full_offer(), INFO_HASH);
    let lab = lab_with(&peer);
    let mut client = connect(&lab);
    client
        .write_all(&handshake(OFFERS_BEP10, b"-qB5000-abcdefghijkl"))
        .expect("write");
    client.flush().expect("flush");
    read_exactly(&mut client, 68);
    let length_bytes = read_exactly(&mut client, 4);
    let length = u32::from_be_bytes(length_bytes.try_into().expect("four bytes"));
    read_exactly(&mut client, usize::try_from(length).expect("short"));

    // ⛔ A length past the cap is refused rather than allocated. A peer that
    // sends 0xffffffff as a length is a peer this observer must survive.
    let over = u32::try_from(Message::MAX_LEN + 1).expect("the cap fits a u32");
    let mut frame = over.to_be_bytes().to_vec();
    frame.push(EXTENDED_MESSAGE_ID);
    client.write_all(&frame).expect("write");
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
            "the observer never reported the over-long message"
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    drop(client);
    drop(lab);

    let error = stream.error().expect("a reason is kept");
    assert_eq!(error.kind(), "message-too-long", "{error}");
    assert!(stream.raw().ends_with(&frame), "the bytes are kept");
    assert!(!stream.rebuilds_from_raw());
}

#[test]
fn extended_peer_keeps_an_extension_dictionary_that_did_not_decode() {
    let peer = PeerWire::offering(full_offer(), INFO_HASH);
    let lab = lab_with(&peer);
    let mut client = connect(&lab);
    client
        .write_all(&handshake(OFFERS_BEP10, b"-qB5000-abcdefghijkl"))
        .expect("write");
    client.flush().expect("flush");
    read_exactly(&mut client, 68);
    let length_bytes = read_exactly(&mut client, 4);
    let length = u32::from_be_bytes(length_bytes.try_into().expect("four bytes"));
    read_exactly(&mut client, usize::try_from(length).expect("short"));

    // An extended handshake whose payload is not bencode at all.
    let payload = b"\x00not-bencode";
    let mut frame = u32::try_from(payload.len() + 1)
        .expect("short")
        .to_be_bytes()
        .to_vec();
    frame.push(EXTENDED_MESSAGE_ID);
    frame.extend_from_slice(payload);
    client.write_all(&frame).expect("write");
    client.flush().expect("flush");

    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    let stream = loop {
        let streams = peer.streams();
        if let Some(one) = streams.first()
            && one.extended_handshake().is_some_and(|found| found.is_err())
        {
            break one.clone();
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the observer never reported the malformed dictionary"
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    drop(client);
    drop(lab);

    assert!(
        stream.extended_handshake().expect("one arrived").is_err(),
        "a dictionary that did not decode is reported, not dropped"
    );
    assert!(
        stream.raw().ends_with(&frame),
        "and the bytes that did not decode are kept"
    );
}

/// ⭐ Driven by the committed corpus, which is what `FOUND-03` wrote it for.
#[test]
fn extended_peer_reads_the_committed_extended_handshake_fixture() {
    use bit_ids::observation::Surface;
    use bit_ids_wire::fixture::load_directory;

    let corpus =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../bit-ids-wire/tests/fixtures");
    let loaded = load_directory(&corpus).expect("the corpus loads");
    let (_, fixture) = loaded
        .iter()
        .find(|(_, one)| {
            one.surface == Surface::PeerWire && one.id.as_str().contains("extended-handshake")
        })
        .expect("the corpus carries an extended-handshake fixture");
    let bytes = fixture.joined_bytes();

    let mut info_hash = [0_u8; INFO_HASH_LEN];
    let protocol_len = usize::from(bytes[0]);
    info_hash.copy_from_slice(&bytes[1 + protocol_len + RESERVED_LEN..][..INFO_HASH_LEN]);

    let peer = PeerWire::offering(full_offer(), info_hash);
    let lab = Lab::builder()
        .deadline(Duration::from_secs(60))
        .stream("peer-wire", peer.accepting())
        .expect("a canonical endpoint name")
        .start()
        .expect("loopback binds");
    let mut client = connect(&lab);
    client.write_all(&bytes).expect("write");
    client.flush().expect("flush");
    read_exactly(&mut client, 68);
    wait_for_extended(&peer);
    drop(client);
    drop(lab);

    let streams = peer.streams();
    assert_eq!(streams[0].raw(), bytes.as_slice(), "kept verbatim");
    assert!(streams[0].rebuilds_from_raw());
    let observed = streams[0]
        .extended_handshake()
        .expect("the fixture carries one")
        .expect("it decodes");
    assert!(
        !observed.extension_ids().is_empty(),
        "the fixture advertises extensions"
    );
    assert_eq!(
        bencode::encode(observed.document()),
        observed.raw().to_vec()
    );
}

#[test]
fn extended_peer_sends_its_extended_handshake_once_and_not_once_per_read() {
    // ⭐ The same shape as the plain handshake, and the guard-mutation pass
    // found it the same way: clearing the sent flag made the observer
    // re-negotiate after every message, and nothing read again to notice.
    let peer = PeerWire::offering(full_offer(), INFO_HASH);
    let lab = lab_with(&peer);
    let mut client = connect(&lab);
    client
        .write_all(&handshake(OFFERS_BEP10, b"-qB5000-abcdefghijkl"))
        .expect("write");
    client.flush().expect("flush");
    read_exactly(&mut client, 68);
    let length_bytes = read_exactly(&mut client, 4);
    let length = u32::from_be_bytes(length_bytes.try_into().expect("four bytes"));
    let ours = read_exactly(&mut client, usize::try_from(length).expect("short"));

    let theirs = Value::Dictionary(vec![(
        b"m".to_vec(),
        Value::Dictionary(vec![(b"ut_pex".to_vec(), Value::integer(1))]),
    )]);
    client.write_all(&extended(0, &theirs)).expect("write");
    client.flush().expect("flush");
    wait_for_extended(&peer);
    for id in [0_u8, 1, 2] {
        let mut frame = 1_u32.to_be_bytes().to_vec();
        frame.push(id);
        client.write_all(&frame).expect("write");
        client.flush().expect("flush");
    }
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while peer
        .streams()
        .first()
        .is_none_or(|one| one.messages().len() < 4)
    {
        assert!(
            std::time::Instant::now() < deadline,
            "the observer never decoded the messages after the handshake"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    let connection = peer.streams()[0].connection();
    drop(client);
    let journal = lab.shutdown();

    // ⛔ Counted from the transcript rather than by reading the socket: an
    // assertion that nothing more arrived is an assertion about silence.
    let sent: Vec<usize> = journal
        .for_connection(connection)
        .iter()
        .filter(|segment| segment.direction() == bit_ids_wire::tracker_udp::Direction::ToTarget)
        .map(|segment| segment.bytes().len())
        .collect();
    assert_eq!(
        sent.iter().sum::<usize>(),
        68 + 4 + ours.len(),
        "one handshake and one extended handshake, whatever arrived after"
    );
}

#[test]
fn extended_peer_can_offer_the_protocol_and_say_nothing_in_it() {
    // ⭐ The third state, and a real condition to run: a build that is offered
    // the protocol and never answered is a different measurement from one that
    // was never offered it. A bit plus an option would have made a fourth state
    // that means nothing, which is why this is an enum.
    let peer = PeerWire::offering(
        Offer {
            extension_protocol: ExtensionProtocol::OfferedSilent,
            dht: false,
            fast: false,
        },
        INFO_HASH,
    );
    let lab = lab_with(&peer);
    let mut client = connect(&lab);
    client
        .write_all(&handshake(OFFERS_BEP10, b"-qB5000-abcdefghijkl"))
        .expect("write");
    client.flush().expect("flush");
    let answer = read_exactly(&mut client, 68);
    assert_eq!(&answer[20..28], &OFFERS_BEP10, "the bit is set");

    // The target answers with its own, which is what the offer asked for.
    let theirs = Value::Dictionary(vec![(
        b"m".to_vec(),
        Value::Dictionary(vec![(b"ut_pex".to_vec(), Value::integer(1))]),
    )]);
    client.write_all(&extended(0, &theirs)).expect("write");
    client.flush().expect("flush");
    wait_for_extended(&peer);
    let connection = peer.streams()[0].connection();
    drop(client);
    let journal = lab.shutdown();

    let sent: usize = journal
        .for_connection(connection)
        .iter()
        .filter(|segment| segment.direction() == bit_ids_wire::tracker_udp::Direction::ToTarget)
        .map(|segment| segment.bytes().len())
        .sum();
    assert_eq!(sent, 68, "the bit was offered and nothing was said in it");
    assert!(
        peer.streams()[0].extended_handshake().is_some(),
        "and the target's own handshake is still observed"
    );
}
