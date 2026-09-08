//! Serves a real client the torrent it is about to announce about, and records
//! what it emitted.
//!
//! ```text
//! cargo run -p bit-ids-probe --example client-capture -- <bundle> <seconds> <torrent> [peer-port]
//! ```
//!
//! ⭐ **This is the observer half of a client capture, and the difference from
//! `evidence-bundle` is the torrent.** That example generates one with a
//! hard-coded announce URL, which no client can be pointed at, so the only thing
//! that could drive it was something told the endpoint separately. Here the
//! announce URL is **this lab's own tracker endpoint**, written into a
//! `.torrent` on disk, so the driver is `<client> <torrent>` and the client
//! chooses the address out of the file. A stock build needs no argument it does
//! not already take.
//!
//! ⛔ **It never says which client announced.** `docs/capture-methodology.md`
//! lists a peer-ID table among the inputs that may seed a hypothesis and may not
//! populate the catalogue, and this prints the bytes rather than a name.
//!
//! ## The two roles, and why the peer one is dialled
//!
//! The tracker endpoint accepts, because the client is told where it is. The
//! peer endpoint **dials**, because it cannot be told: `Lab` binds port zero so
//! the operating system chooses, and `TrackerResponse` is cloned into the
//! responder while the lab is still being built, which is before any port
//! exists. A tracker answer naming this lab's peer port would have to predict
//! it.
//!
//! ⭐ So the client's own listen port is the argument instead, and the lab dials
//! it once an announce has arrived. That is `OBS-04`'s dialling role, it needs
//! no prediction, and it is the stronger measurement of the two: the side that
//! dials sends its handshake first, so the bytes that come back are the build
//! answering rather than the build opening.
//!
//! ⚠ The dial waits for an announce rather than happening at once, because a
//! client that has not yet read the torrent is not yet listening for peers on
//! it, and a refused connection would be recorded as a build that declined.
//!
//! ## What it prints
//!
//! One `key value` line per fact, because `capture-run` reads them with `awk`
//! and a shell script parsing prose is a second grammar to keep in step.

use core::fmt::Write as _;
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use bit_ids::canonical::Slug;
use bit_ids::manifest::PhaseName;
use bit_ids::record::EvidenceKind;
use bit_ids_lab::{Bundle, Lab, SyntheticTorrent, TorrentSpec, TranscriptOf};
use bit_ids_probe::peer_wire::{ExtendedOffer, ExtensionProtocol, Offer, PeerWire};
use bit_ids_probe::{HttpTracker, TrackerResponse};

/// The name every fixture peer ID in this repository carries.
///
/// ⚠ It is what **this observer** calls itself, and never what the build under
/// measurement is called. A capture whose transcript held only this value
/// measured nothing.
const OBSERVER_PEER_ID: &[u8] = b"bit-ids-fixture-0001";

fn slug(text: &str) -> Slug {
    Slug::parse(text).expect("a canonical identifier")
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::new();
    for byte in bytes {
        write!(out, "{byte:02x}").expect("writing to a String cannot fail");
    }
    out
}

fn names(values: &[Vec<u8>]) -> String {
    values
        .iter()
        .map(|value| String::from_utf8_lossy(value).into_owned())
        .collect::<Vec<_>>()
        .join(",")
}

fn fail(message: &str) -> ! {
    eprintln!("client-capture: {message}");
    std::process::exit(1)
}

fn main() {
    let mut arguments = std::env::args().skip(1);
    let root = arguments.next().unwrap_or_else(|| "bundle".to_owned());
    let seconds: u64 = arguments
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or(20);
    let torrent_path = arguments
        .next()
        .unwrap_or_else(|| "fixture.torrent".to_owned());
    // ⚠ Absent means "do not dial", which is a different run from a dial that
    // failed. A capture of a client with no peer port measures the tracker
    // surface alone and says so, rather than reporting a peer surface nobody
    // could reach.
    let peer_port: Option<u16> = arguments.next().and_then(|value| value.parse().ok());

    // ⚠ What the tracker answers is a condition of the run. It offers no peer
    // at all: the peer surface is reached by dialling below, and a compact
    // peer list naming an address this lab does not serve would put a
    // connection refusal in the build's log and change what it does next.
    let tracker = HttpTracker::new(TrackerResponse {
        interval: 60,
        complete: 0,
        incomplete: 1,
        peers: Vec::new(),
    });

    // ⚠ Offered, and recorded as offered. A build's extension map may differ
    // with what it was shown, so the offer is part of the measurement.
    let offer = Offer {
        extension_protocol: ExtensionProtocol::Offered(ExtendedOffer {
            extensions: vec![(b"ut_metadata".to_vec(), 1), (b"ut_pex".to_vec(), 2)],
            client: Some(b"bit-ids-observer/1".to_vec()),
            request_queue: Some(250),
            metadata_size: None,
        }),
        dht: false,
        fast: false,
    };

    let mut lab = match Lab::builder()
        .deadline(Duration::from_secs(seconds))
        .stream("tracker-http", tracker.responder())
        .and_then(|builder| {
            builder.stream("peer-wire", |_connection, received: &[u8]| {
                // ⚠ The accepting side is present so a build that finds this lab by
                // some other route is still recorded. It answers nothing, because a
                // client cannot learn this port from the torrent and anything that
                // arrives here is unexplained rather than expected.
                bit_ids_lab::StreamReply::Answer {
                    consumed: received.len(),
                    send: Vec::new(),
                }
            })
        })
        .and_then(bit_ids_lab::LabBuilder::start)
    {
        Ok(lab) => lab,
        Err(error) => fail(&format!("the lab would not start: {error}")),
    };

    let announce_to = match lab.endpoint("tracker-http") {
        Some(endpoint) => endpoint.address(),
        None => fail("the lab reported no tracker-http endpoint"),
    };
    let announce_url = format!("http://{announce_to}/announce");

    // ⛔ The torrent is generated from a declared spec, so `capture.fixture` can
    // be re-derived from the record and compared against these bytes. Only the
    // announce URL varies per run, and it is the one field a client must read
    // out of the file for any of this to happen at all.
    let torrent = match SyntheticTorrent::generate(TorrentSpec {
        name: "bit-ids-client-capture".to_owned(),
        announce: Some(announce_url.clone()),
        ..TorrentSpec::default()
    }) {
        Ok(torrent) => torrent,
        Err(error) => fail(&format!("the spec describes no usable torrent: {error}")),
    };
    if let Err(error) = std::fs::write(&torrent_path, torrent.metainfo()) {
        fail(&format!(
            "the torrent could not be written to {torrent_path}: {error}"
        ));
    }

    for endpoint in lab.endpoints() {
        println!("endpoint {} {}", endpoint.name(), endpoint.address());
    }
    println!("announce-url {announce_url}");
    println!("torrent {torrent_path}");
    println!("info-hash {}", hex(torrent.info_hash()));
    println!("fixture-sha256 {}", torrent.digest());
    println!("offered-reserved {}", hex(&offer.reserved()));
    match peer_port {
        Some(port) => println!("peer-dial 127.0.0.1:{port}"),
        None => println!("peer-dial none"),
    }
    // ⛔ LAST, and the line `capture-run` waits on. Everything a driver needs to
    // start a client is printed above it, so a driver that saw this line and
    // then read the others cannot race them.
    println!("serving for {seconds}s");

    let peer_wire = PeerWire::offering(offer, *torrent.info_hash());
    println!(
        "peer-dialled {}",
        dial_when_announced(&mut lab, &tracker, &peer_wire, peer_port, seconds)
    );

    lab.wait();
    let expired = lab.deadline_expired();
    let journal = lab.shutdown();
    println!("deadline expired: {expired}");
    println!("segments: {}", journal.segments().len());

    report_announces(&tracker);
    report_peers(&peer_wire);
    write_bundle(&root, &journal, &torrent);
}

/// Dials the client's own listen port, once an announce says it is listening.
///
/// ⚠ Polling the observer rather than the socket. An announce is the first
/// evidence that the build read the torrent, and it is the earliest moment its
/// own listener is up for this info hash.
fn dial_when_announced(
    lab: &mut Lab,
    tracker: &HttpTracker,
    peer_wire: &PeerWire,
    peer_port: Option<u16>,
    seconds: u64,
) -> String {
    let Some(port) = peer_port else {
        return "none".to_owned();
    };
    let address: SocketAddr = ([127, 0, 0, 1], port).into();
    let waiting_since = Instant::now();
    let wait_for = Duration::from_secs(seconds).min(Duration::from_secs(60));
    loop {
        if !tracker.announces().is_empty() {
            return match lab.dial(
                "peer-wire-dialled",
                address,
                peer_wire.opening(),
                peer_wire.dialling(),
            ) {
                Ok(_) => format!("{address}"),
                // ⚠ A refused dial is recorded and does not end the run. The
                // tracker surface is already measured by this point, and
                // throwing it away because a peer port was wrong would lose a
                // measurement to a detail of the driver.
                Err(error) => format!("refused: {error}"),
            };
        }
        if waiting_since.elapsed() >= wait_for {
            return "no announce arrived".to_owned();
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// What the build put in its announces.
///
/// ⛔ Bytes, in hexadecimal, and never a client name. The peer ID is the
/// measurement; naming its prefix would put the one refused input inside the
/// component every observer trusts.
fn report_announces(tracker: &HttpTracker) {
    let announces = tracker.announces();
    println!("announces {}", announces.len());
    println!("announces-dropped {}", tracker.dropped());
    for (index, announce) in announces.iter().enumerate() {
        let peer_id = match announce.peer_id() {
            Some(Ok(bytes)) => hex(&bytes),
            Some(Err(error)) => format!("undecodable: {error}"),
            None => "absent".to_owned(),
        };
        println!("announce {index} peer-id {peer_id}");
        println!(
            "announce {index} query {}",
            names(&announce.query_key_order())
        );
        println!(
            "announce {index} headers {}",
            names(&announce.header_name_order())
        );
        let agent = announce
            .header(b"user-agent")
            .map_or_else(|| "absent".to_owned(), hex);
        println!("announce {index} user-agent {agent}");
        // ⚠ Whether the observer's own peer ID came back is how a reader tells a
        // build's announce from this example's own traffic in a shared log.
        let mine = announce
            .peer_id()
            .and_then(Result::ok)
            .is_some_and(|bytes| bytes == OBSERVER_PEER_ID);
        println!("announce {index} is-observer-peer-id {mine}");
    }
}

/// What the build put on the peer wire.
fn report_peers(peer_wire: &PeerWire) {
    let streams = peer_wire.streams();
    println!("peer-streams {}", streams.len());
    println!("peer-dropped {}", peer_wire.dropped());
    for (index, stream) in streams.iter().enumerate() {
        println!("peer {index} role {:?}", stream.role());
        println!("peer {index} messages {}", stream.messages().len());
        println!("peer {index} rebuilds {}", stream.rebuilds_from_raw());
        match stream.extended_handshake() {
            Some(Ok(message)) => {
                // ⛔ The undecoded dictionary is the evidence and the decode is
                // an interpretation of it, so both are printed and neither
                // replaces the other. The extension names a build invents are
                // identity as much as the ones it was offered.
                println!("peer {index} extended {}", hex(message.raw()));
                let offered: Vec<String> = message
                    .extension_ids()
                    .into_iter()
                    .map(|(name, id)| format!("{}={id}", String::from_utf8_lossy(&name)))
                    .collect();
                println!("peer {index} extensions {}", offered.join(","));
            }
            Some(Err(error)) => println!("peer {index} extended undecodable: {error}"),
            None => println!("peer {index} extended absent"),
        }
    }
}

/// Writes the run out as evidence and prints the rows a manifest would carry.
fn write_bundle(root: &str, journal: &bit_ids_lab::Journal, torrent: &SyntheticTorrent) {
    let mut bundle = match Bundle::create(root, slug("bit-ids-probe"), PhaseName::Observed) {
        Ok(bundle) => bundle,
        Err(error) => fail(&format!("could not open the bundle at {root}: {error}")),
    };

    // ⛔ Every endpoint the lab may have is named. `transcripts` refuses an
    // endpoint the plan does not carry rather than filing it under a guessed
    // kind, and the dialled peer endpoint only exists on some runs, so a plan
    // written from the endpoints that happen to be present would refuse the run
    // that dialled.
    let plan = BTreeMap::from([
        (
            slug("tracker-http"),
            TranscriptOf {
                id: slug("ev-tracker-capture"),
                kind: EvidenceKind::TrackerCapture,
            },
        ),
        (
            slug("peer-wire"),
            TranscriptOf {
                id: slug("ev-peer-transcript"),
                kind: EvidenceKind::PeerTranscript,
            },
        ),
        (
            slug("peer-wire-dialled"),
            TranscriptOf {
                id: slug("ev-peer-dialled"),
                kind: EvidenceKind::PeerTranscript,
            },
        ),
    ]);
    if let Err(error) = bundle.transcripts(journal, &plan) {
        fail(&format!("the run could not be written: {error}"));
    }
    if let Err(error) = bundle.transcript(
        slug("ev-metainfo"),
        EvidenceKind::Metainfo,
        "fixture/generated.torrent",
        torrent.metainfo(),
    ) {
        fail(&format!("the metainfo could not be written: {error}"));
    }

    // ⛔ Read back before it is described.
    if let Err(error) = bundle.verify() {
        fail(&format!("the bundle does not verify: {error}"));
    }

    println!("root {}", bundle.root().display());
    for record in bundle.evidence() {
        println!(
            "evidence {} {} {} {} {} {}",
            record.id,
            record.path,
            record.bytes,
            record.sha256,
            record.redacted,
            record.object_path()
        );
    }
    println!("verified: true");
}
