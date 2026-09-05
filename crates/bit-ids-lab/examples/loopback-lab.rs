//! Runs a lab and prints what reached it, so the supervisor can be driven by
//! something that is not this project's own test harness.
//!
//! ```text
//! cargo run -p bit-ids-lab --example loopback-lab -- 20
//! ```
//!
//! It prints the address of each endpoint, serves until the deadline, and then
//! prints the journal. Point `curl` at the HTTP endpoint or send a datagram to
//! the UDP one and the transcript comes back with the bytes that arrived.
//!
//! ⚠ It answers HTTP with a fixed 200 and answers a datagram by reversing it.
//! Neither is a tracker. `OBS-02` and `OBS-03` own the protocols; this exists to
//! drive the part that binds, records and stops.

use core::fmt::Write as _;
use std::time::Duration;

use bit_ids_lab::{Lab, StreamReply};

fn main() {
    let seconds: u64 = std::env::args()
        .nth(1)
        .and_then(|argument| argument.parse().ok())
        .unwrap_or(10);

    let mut lab = Lab::builder()
        .deadline(Duration::from_secs(seconds))
        .stream("tracker-http", |_connection, received: &[u8]| {
            // A request head ends at a blank line. Anything less is not a
            // request yet, which is what `NeedMore` is for.
            if received.windows(4).any(|window| window == b"\r\n\r\n") {
                StreamReply::Close {
                    send: b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n".to_vec(),
                }
            } else {
                StreamReply::NeedMore
            }
        })
        .expect("a canonical endpoint name")
        .datagram("tracker-udp", |received: &[u8]| {
            Some(received.iter().rev().copied().collect())
        })
        .expect("a canonical endpoint name")
        .start()
        .expect("a loopback lab binds");

    for endpoint in lab.endpoints() {
        println!(
            "{} {:?} {}",
            endpoint.name(),
            endpoint.transport(),
            endpoint.address()
        );
    }
    println!("serving for {seconds}s");

    lab.wait();
    let expired = lab.deadline_expired();
    let journal = lab.shutdown();

    println!("deadline expired: {expired}");
    println!("segments: {}", journal.segments().len());
    for segment in journal.segments() {
        let mut preview = String::new();
        for byte in segment.bytes().iter().take(48) {
            write!(preview, "{byte:02x}").expect("writing to a String cannot fail");
        }
        println!(
            "  {:>6}ms {} {:?} {} bytes {preview}",
            segment.offset_ms(),
            segment.endpoint(),
            segment.direction(),
            segment.bytes().len()
        );
    }
}
