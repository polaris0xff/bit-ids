//! Runs a lab, writes the run out as an evidence bundle, and prints the
//! manifest rows, so `OBS-09` can be driven by something that is not this
//! project's test harness.
//!
//! ```text
//! cargo run -p bit-ids-lab --example evidence-bundle -- /tmp/bundle 20
//! ```
//!
//! It prints each endpoint's address, serves until the deadline, then writes one
//! artifact per endpoint plus the generated torrent and prints what a manifest
//! would carry for each. ⭐ **The point is that an outside reader can check
//! every line of that against the files on disk**: the digest, the size, the
//! content-addressed store path, and whether the transcript holds the bytes that
//! reader actually sent.

use std::collections::BTreeMap;
use std::time::Duration;

use bit_ids::canonical::Slug;
use bit_ids::manifest::PhaseName;
use bit_ids::record::EvidenceKind;
use bit_ids_lab::{Bundle, Lab, StreamReply, SyntheticTorrent, TorrentSpec, TranscriptOf};

fn slug(text: &str) -> Slug {
    Slug::parse(text).expect("a canonical identifier")
}

fn main() {
    let mut arguments = std::env::args().skip(1);
    let root = arguments.next().unwrap_or_else(|| "bundle".to_owned());
    let seconds: u64 = arguments
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or(10);

    let mut lab = Lab::builder()
        .deadline(Duration::from_secs(seconds))
        .stream("tracker-http", |_connection, received: &[u8]| {
            if received.windows(4).any(|window| window == b"\r\n\r\n") {
                StreamReply::Close {
                    send: b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nde".to_vec(),
                }
            } else {
                StreamReply::NeedMore
            }
        })
        .expect("a canonical endpoint name")
        .stream("peer-wire", |_connection, received: &[u8]| {
            StreamReply::Answer {
                consumed: received.len(),
                send: b"observed".to_vec(),
            }
        })
        .expect("a canonical endpoint name")
        .start()
        .expect("a loopback lab binds");

    for endpoint in lab.endpoints() {
        println!("endpoint {} {}", endpoint.name(), endpoint.address());
    }
    println!("serving for {seconds}s");

    lab.wait();
    let expired = lab.deadline_expired();
    let journal = lab.shutdown();
    println!("deadline expired: {expired}");
    println!("segments: {}", journal.segments().len());

    let torrent = SyntheticTorrent::generate(TorrentSpec {
        name: "obs-09-run".to_owned(),
        announce: Some("http://127.0.0.1:6969/announce".to_owned()),
        ..TorrentSpec::default()
    })
    .expect("the spec describes a usable torrent");

    let mut bundle = match Bundle::create(&root, slug("bit-ids-probe"), PhaseName::Observed) {
        Ok(bundle) => bundle,
        Err(error) => {
            eprintln!("could not open the bundle at {root}: {error}");
            std::process::exit(1);
        }
    };

    let plan = BTreeMap::from([
        (
            slug("peer-wire"),
            TranscriptOf {
                id: slug("ev-peer-transcript"),
                kind: EvidenceKind::PeerTranscript,
            },
        ),
        (
            slug("tracker-http"),
            TranscriptOf {
                id: slug("ev-tracker-capture"),
                kind: EvidenceKind::TrackerCapture,
            },
        ),
    ]);
    if let Err(error) = bundle.transcripts(&journal, &plan) {
        eprintln!("the run could not be written: {error}");
        std::process::exit(1);
    }
    if let Err(error) = bundle.transcript(
        slug("ev-metainfo"),
        EvidenceKind::Metainfo,
        "fixture/generated.torrent",
        torrent.metainfo(),
    ) {
        eprintln!("the metainfo could not be written: {error}");
        std::process::exit(1);
    }

    // ⛔ Verified before it is described. A bundle printed without being read
    // back is a report about what this process believes it wrote.
    if let Err(error) = bundle.verify() {
        eprintln!("the bundle does not verify: {error}");
        std::process::exit(1);
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
    for redaction in bundle.redactions() {
        println!("redaction {} {}", redaction.evidence, redaction.occurrences);
    }
    println!("verified: true");
}
