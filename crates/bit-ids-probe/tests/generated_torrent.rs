//! The seam between the torrent `OBS-08` generates and the observers that see
//! it announced.
//!
//! ⛔ **Nothing else joins these two.** The generator declares a twenty-byte
//! info hash as `bit_ids_lab::torrent::PIECE_HASH_LEN` and the peer wire
//! declares one as `bit_ids_wire::peer_wire::INFO_HASH_LEN`, in different
//! crates, and until something passes one to the other neither the widths nor
//! the bytes are checked against each other. `docs/methodology/gate.md` names
//! that class: each part correct, the assembly untested. Handing the array
//! across makes the compiler check the width, and driving it over a socket
//! checks the bytes.
//!
//! ⚠ **What this proves and what it does not.** The info hash on the wire here
//! is the one the generator computed, so this is not a second opinion on
//! whether that value is the SHA-1 a client would derive from the file. The
//! `OBS-08` acceptance suite and its driven pass answer that, the latter with
//! `libtorrent` computing the hash itself. What is proven here is that the value
//! survives the seam: the bytes a record will publish are the bytes an observer
//! reads back off a real connection, at both surfaces a capture uses to
//! identify a torrent.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use bit_ids::canonical::{Sha256Digest, Slug};
use bit_ids::manifest::PhaseName;
use bit_ids::record::EvidenceKind;
use bit_ids_lab::torrent::PIECE_HASH_LEN;
use bit_ids_lab::{Bundle, Lab, SyntheticTorrent, TorrentSpec, TranscriptOf};
use bit_ids_probe::HttpTracker;
use bit_ids_probe::peer_wire::{PeerIdentity, PeerWire};
use bit_ids_wire::peer_wire::{INFO_HASH_LEN, RESERVED_LEN};

/// The torrent a capture would hand a client.
fn generated() -> SyntheticTorrent {
    SyntheticTorrent::generate(TorrentSpec {
        name: "obs-08-seam".to_owned(),
        announce: Some("http://127.0.0.1:6969/announce".to_owned()),
        private: true,
        ..TorrentSpec::default()
    })
    .expect("the spec describes a usable torrent")
}

fn percent_encode(bytes: &[u8]) -> String {
    use core::fmt::Write as _;
    let mut out = String::with_capacity(bytes.len() * 3);
    for byte in bytes {
        write!(out, "%{byte:02x}").expect("writing to a String cannot fail");
    }
    out
}

#[test]
fn the_generated_info_hash_is_the_one_the_peer_observer_reads_off_the_wire() {
    let torrent = generated();

    // ⛔ The assignment is the width check. `PeerWire` takes
    // `[u8; INFO_HASH_LEN]`; the generator answers `[u8; PIECE_HASH_LEN]`. If
    // the two constants ever disagree this line stops compiling, which is the
    // only place in the workspace that says they must not.
    let on_the_wire: [u8; INFO_HASH_LEN] = *torrent.info_hash();
    let peer = PeerWire::new(PeerIdentity::default(), on_the_wire);

    let lab = Lab::builder()
        .deadline(Duration::from_secs(60))
        .stream("peer-wire", peer.accepting())
        .expect("a canonical endpoint name")
        .start()
        .expect("loopback binds");

    // A BEP 3 handshake written here rather than with the observer's own
    // encoder, carrying the generated info hash the way a client would.
    let mut sent = vec![19_u8];
    sent.extend_from_slice(b"BitTorrent protocol");
    sent.extend_from_slice(&[0_u8; RESERVED_LEN]);
    sent.extend_from_slice(&on_the_wire);
    sent.extend_from_slice(b"-qB5000-abcdefghijkl");

    let address = lab.endpoint("peer-wire").expect("added").address();
    let mut client = TcpStream::connect(address).expect("the endpoint accepts");
    client
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("a read timeout is settable");
    client.write_all(&sent).expect("write");
    client.flush().expect("flush");

    let mut answer = vec![0_u8; 68];
    client
        .read_exact(&mut answer)
        .expect("a handshake comes back");
    drop(client);
    drop(lab);

    // ⭐ A client checks the info hash in the reply and drops a peer that
    // answered about a different torrent, so the observer's own handshake has
    // to carry the generated value too.
    assert_eq!(
        &answer[1 + 19 + RESERVED_LEN..][..INFO_HASH_LEN],
        torrent.info_hash(),
        "the observer answered about a different torrent"
    );

    let streams = peer.streams();
    assert_eq!(streams.len(), 1);
    let observed = streams[0].handshake().expect("a handshake was read");
    assert_eq!(
        observed.info_hash(),
        torrent.info_hash(),
        "the info hash the observer recorded is not the generated one"
    );
    assert_eq!(observed.info_hash().len(), PIECE_HASH_LEN);
}

#[test]
fn the_generated_info_hash_survives_the_announce_the_tracker_observer_records() {
    // ⭐ The surface the entry's Problem statement names: a client announces
    // *about an info hash*. On the HTTP tracker that value is twenty raw bytes
    // percent-encoded into a query, so it passes through an encoding that
    // mangles it if either side gets the escaping wrong.
    let torrent = generated();
    let tracker = HttpTracker::default();
    let lab = Lab::builder()
        .deadline(Duration::from_secs(60))
        .stream("tracker-http", tracker.responder())
        .expect("a canonical endpoint name")
        .start()
        .expect("loopback binds");

    let head = format!(
        "GET /announce?info_hash={}&peer_id=-qB5000-abcdefghijkl&port=6881 HTTP/1.1\r\n\r\n",
        percent_encode(torrent.info_hash())
    );
    let address = lab.endpoint("tracker-http").expect("added").address();
    let mut client = TcpStream::connect(address).expect("the endpoint accepts");
    client
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("a read timeout is settable");
    client.write_all(head.as_bytes()).expect("write");
    client.flush().expect("flush");
    let mut answer = [0_u8; 16];
    let _ = client.read(&mut answer).expect("the endpoint answers");
    drop(client);
    drop(lab);

    let announces = tracker.announces();
    assert_eq!(announces.len(), 1);
    let decoded = announces[0]
        .decoded(b"info_hash")
        .expect("the announce carried one")
        .expect("it decodes");
    assert_eq!(
        decoded,
        torrent.info_hash(),
        "the info hash the tracker observer decoded is not the generated one"
    );
}

/// ⛔ The whole path a capture walks, end to end: a torrent, an observer that
/// records what a client said about it, and the evidence a manifest will cite.
///
/// Each leg is covered on its own. Nothing covered the assembly, and that is the
/// class `docs/methodology/gate.md` names: each part correct, the composition
/// wrong. This crate is the only one that depends on both halves, so it is the
/// only place the test can live.
#[test]
fn an_announce_about_the_generated_torrent_reaches_the_evidence_a_manifest_cites() {
    let torrent = generated();
    let tracker = HttpTracker::default();
    let lab = Lab::builder()
        .deadline(Duration::from_secs(60))
        .stream("tracker-http", tracker.responder())
        .expect("a canonical endpoint name")
        .start()
        .expect("loopback binds");

    let head = format!(
        "GET /announce?info_hash={}&peer_id=-qB5000-abcdefghijkl&port=6881 HTTP/1.1\r\n\r\n",
        percent_encode(torrent.info_hash())
    );
    let address = lab.endpoint("tracker-http").expect("added").address();
    let mut client = TcpStream::connect(address).expect("the endpoint accepts");
    client
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("a read timeout is settable");
    client.write_all(head.as_bytes()).expect("write");
    client.flush().expect("flush");
    let mut answer = [0_u8; 16];
    let _ = client.read(&mut answer).expect("the endpoint answers");
    drop(client);
    let journal = lab.shutdown();

    let root = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("capture-path");
    let _ = std::fs::remove_dir_all(&root);
    let mut bundle = Bundle::create(
        &root,
        Slug::parse("bit-ids-probe").expect("a slug"),
        PhaseName::Observed,
    )
    .expect("the bundle root is creatable");
    bundle
        .transcripts(
            &journal,
            &BTreeMap::from([(
                Slug::parse("tracker-http").expect("a slug"),
                TranscriptOf {
                    id: Slug::parse("ev-tracker-capture").expect("a slug"),
                    kind: EvidenceKind::TrackerCapture,
                },
            )]),
        )
        .expect("the plan names the endpoint");
    bundle
        .transcript(
            Slug::parse("ev-metainfo").expect("a slug"),
            EvidenceKind::Metainfo,
            "fixture/generated.torrent",
            torrent.metainfo(),
        )
        .expect("the metainfo is written");
    bundle.verify().expect("the bundle verifies");

    // ⭐ The identity the observer decoded is the identity the artifact carries.
    // A capture that recorded the announce and wrote a transcript of some other
    // exchange would pass every test either half has on its own.
    let capture = bundle
        .evidence()
        .iter()
        .find(|record| record.kind == EvidenceKind::TrackerCapture)
        .expect("the run wrote a tracker capture");
    let written = std::fs::read_to_string(root.join(capture.path.as_str()))
        .expect("the transcript is on disk");
    // ⚠ The transcript holds the raw wire bytes as lowercase hex, so what to
    // look for is the hex of the request's ASCII, not the request itself. The
    // two encodings are easy to conflate and the first version of this
    // assertion did.
    let on_the_wire = head.as_bytes().iter().fold(String::new(), |mut out, byte| {
        use core::fmt::Write as _;
        write!(out, "{byte:02x}").expect("a String cannot fail");
        out
    });
    assert!(
        written.contains(&on_the_wire),
        "the transcript does not carry the announce this client sent"
    );
    assert_eq!(
        tracker.announces()[0]
            .decoded(b"info_hash")
            .expect("present")
            .expect("decodes"),
        torrent.info_hash(),
        "the observer decoded an info hash the torrent does not have"
    );
    assert_eq!(torrent.info_hash().len(), PIECE_HASH_LEN);

    // And the metainfo the client was handed is in the bundle beside it, so a
    // reader can re-derive the info hash the announce carried.
    let metainfo = bundle
        .evidence()
        .iter()
        .find(|record| record.kind == EvidenceKind::Metainfo)
        .expect("the run wrote a metainfo");
    assert_eq!(metainfo.sha256, torrent.digest());
    assert_eq!(
        std::fs::read(root.join(metainfo.path.as_str())).expect("on disk"),
        torrent.metainfo()
    );
    assert_eq!(
        Sha256Digest::of(torrent.metainfo()),
        torrent.digest(),
        "capture.fixture does not name the file the bundle carries"
    );
}
