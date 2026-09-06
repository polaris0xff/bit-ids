//! Runs a UDP tracker observer and prints what announced to it.
//!
//! ```text
//! cargo run -p bit-ids-probe --example udp-tracker -- 20
//! ```
//!
//! It prints the tracker address, serves until the deadline, and then prints one
//! line per datagram with its action, and the fields of every announce.
//!
//! ⛔ It never says which client sent an announce.

use core::fmt::Write as _;
use std::time::Duration;

use bit_ids_lab::Lab;
use bit_ids_probe::OfferedPeer;
use bit_ids_probe::tracker_udp::{UdpTracker, UdpTrackerResponse};

fn hex(bytes: &[u8]) -> String {
    let mut out = String::new();
    for byte in bytes {
        write!(out, "{byte:02x}").expect("writing to a String cannot fail");
    }
    out
}

fn main() {
    let seconds: u64 = std::env::args()
        .nth(1)
        .and_then(|argument| argument.parse().ok())
        .unwrap_or(20);

    let tracker = UdpTracker::new(UdpTrackerResponse {
        interval: 60,
        leechers: 0,
        seeders: 1,
        peers: vec![
            OfferedPeer::new([127, 0, 0, 1], 6881, *b"bit-ids-fixture-0001")
                .expect("a loopback peer is inside the allowed set"),
        ],
    });

    let mut lab = Lab::builder()
        .deadline(Duration::from_secs(seconds))
        .datagram("tracker-udp", tracker.responder())
        .expect("a canonical endpoint name")
        .start()
        .expect("a loopback lab binds");

    let address = lab.endpoint("tracker-udp").expect("it was added").address();
    println!("tracker udp://{address}/announce");
    println!("serving for {seconds}s");

    lab.wait();
    let journal = lab.shutdown();

    println!("datagrams: {}", tracker.datagrams().len());
    println!("segments: {}", journal.segments().len());
    println!("connection ids issued: {}", tracker.issued_connection_ids());
    println!("refusals: {:?}", tracker.refusals());
    for (index, observed) in tracker.datagrams().iter().enumerate() {
        match observed.decoded() {
            Ok(datagram) => println!(
                "  {index} {} bytes action {:?} transaction {:#010x}",
                observed.raw().len(),
                datagram.action(),
                datagram.transaction_id()
            ),
            Err(error) => println!(
                "  {index} {} bytes did not decode: {error}",
                observed.raw().len()
            ),
        }
        match observed.announce() {
            Some(Ok(announce)) => {
                println!("      info_hash {}", hex(&announce.info_hash));
                println!(
                    "      peer_id   {} ({:?})",
                    hex(&announce.peer_id),
                    String::from_utf8_lossy(&announce.peer_id)
                );
                println!(
                    "      key {:#010x} event {} num_want {} port {}",
                    announce.key, announce.event, announce.num_want, announce.port
                );
                if !announce.options.is_empty() {
                    println!("      bep41     {}", hex(&announce.options));
                }
            }
            Some(Err(error)) => println!("      announce did not decode: {error}"),
            None => {}
        }
    }
}
