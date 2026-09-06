//! Runs an HTTP tracker observer and prints what announced to it.
//!
//! ```text
//! cargo run -p bit-ids-probe --example http-tracker -- 20
//! ```
//!
//! It prints the announce URL, serves until the deadline, and then prints one
//! block per announce: the query keys in order, the header names in order, the
//! percent-encoding case of the info hash, and the peer ID as hexadecimal.
//!
//! ⛔ It never says which client sent an announce. `docs/capture-methodology.md`
//! lists a peer-ID table among the inputs that may seed a hypothesis and may not
//! populate the catalogue.

use core::fmt::Write as _;
use std::time::Duration;

use bit_ids_lab::Lab;
use bit_ids_probe::{HttpTracker, OfferedPeer, TrackerResponse};

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
        .join(", ")
}

fn main() {
    let seconds: u64 = std::env::args()
        .nth(1)
        .and_then(|argument| argument.parse().ok())
        .unwrap_or(20);

    let tracker = HttpTracker::new(TrackerResponse {
        interval: 60,
        complete: 1,
        incomplete: 0,
        peers: vec![
            OfferedPeer::new([127, 0, 0, 1], 6881, *b"bit-ids-fixture-0001")
                .expect("a loopback peer is inside the allowed set"),
        ],
    });

    let mut lab = Lab::builder()
        .deadline(Duration::from_secs(seconds))
        .stream("tracker-http", tracker.responder())
        .expect("a canonical endpoint name")
        .start()
        .expect("a loopback lab binds");

    let address = lab
        .endpoint("tracker-http")
        .expect("it was added")
        .address();
    println!("announce http://{address}/announce");
    println!("serving for {seconds}s");

    lab.wait();
    let journal = lab.shutdown();

    let announces = tracker.announces();
    println!("announces: {}", announces.len());
    println!("segments: {}", journal.segments().len());
    for (index, announce) in announces.iter().enumerate() {
        println!("--- announce {index}");
        println!("  head bytes    {}", announce.raw().len());
        println!("  query order   {}", names(&announce.query_key_order()));
        println!("  header order  {}", names(&announce.header_name_order()));
        println!("  compact       {:?}", announce.wants_compact());
        match announce.percent_case(b"info_hash") {
            Some(case) => println!("  info_hash esc {case:?}"),
            None => println!("  info_hash esc absent"),
        }
        match announce.peer_id() {
            Some(Ok(bytes)) => println!("  peer_id       {} ({} bytes)", hex(&bytes), bytes.len()),
            Some(Err(error)) => println!("  peer_id       did not decode: {error}"),
            None => println!("  peer_id       absent"),
        }
    }
}
