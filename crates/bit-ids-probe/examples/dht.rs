//! Runs a BEP 5 DHT observer and prints what queried it.
//!
//! ```text
//! cargo run -p bit-ids-probe --example dht -- 20
//! ```
//!
//! It prints the endpoint address, answers queries until the deadline, and then
//! prints one block per message: the method, the node id the build chose for
//! itself, the version tag it volunteered, the order it wrote its arguments, and
//! anything BEP 5 does not describe.
//!
//! ⛔ **It never says which client queried.** A `v` string is recorded as bytes
//! and is never resolved to a name.
//!
//! ⛔ **The only address it offers is the loopback one it was given**, through
//! `bind::check_offered`. A `get_peers` answer names places the build will then
//! dial on its own socket, which no guard on this project's sockets can see.
//!
//! ⭐ **Point something that is not this project at it.** A `libtorrent`
//! `bencode` in a few lines of Python builds a real KRPC query and a raw socket
//! sends it.

use std::net::{Ipv4Addr, SocketAddrV4};
use std::time::Duration;

use bit_ids_lab::adjacent::{Capability, Surface, endpoint_name};
use bit_ids_lab::{Lab, bind};
use bit_ids_probe::dht::{A_REAL_BOOTSTRAP_NODE, Dht, OfferedPeers};

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn main() {
    let seconds: u64 = std::env::args()
        .nth(1)
        .and_then(|argument| argument.parse().ok())
        .unwrap_or(20);

    let offered = OfferedPeers::of(&[SocketAddrV4::new(Ipv4Addr::LOCALHOST, 6881)])
        .expect("loopback is inside the allowed set");

    // ⛔ The switch, written out. Nothing defaults it on.
    let observer = Dht::new(Capability::enable(Surface::Dht))
        .expect("the capability names this surface")
        .offering(offered);

    let mut lab = Lab::builder()
        .deadline(Duration::from_secs(seconds))
        .datagram(endpoint_name(Surface::Dht), observer.responder())
        .expect("a canonical endpoint name")
        .start()
        .expect("a loopback lab binds");

    let address = lab
        .endpoint(endpoint_name(Surface::Dht))
        .expect("it was added")
        .address();
    println!("dht udp://{address}");
    println!("observer node id: {}", text(observer.node_id()));
    println!("refused destination: {A_REAL_BOOTSTRAP_NODE}");
    // ⚠ Driven rather than asserted in prose: the address a build's own default
    // names is refused by the same socket that reaches the lab.
    let socket = bind::datagram(std::net::IpAddr::V4(Ipv4Addr::LOCALHOST)).expect("loopback binds");
    match bind::send_to(&socket, b"d1:y1:qe", A_REAL_BOOTSTRAP_NODE) {
        Ok(_) => println!("  THE GUARD DID NOT FIRE, which is a defect"),
        Err(error) => println!("  refused: {error}"),
    }
    println!("serving for {seconds}s");

    lab.wait();
    let journal = lab.shutdown();

    let messages = observer.messages();
    println!("messages: {}", messages.len());
    println!("dropped past the cap: {}", observer.dropped());
    println!("tokens issued: {}", observer.issued_tokens().len());
    println!("segments: {}", journal.segments().len());
    for (index, observed) in messages.iter().enumerate() {
        println!("--- message {index} from {}", observed.source());
        let Some(message) = observed.message() else {
            println!("  not bencode, {} bytes kept", observed.raw().len());
            continue;
        };
        println!("  kind:        {:?}", message.kind());
        println!(
            "  method:      {}",
            message.method().map_or_else(|| "(none)".to_owned(), text)
        );
        println!(
            "  node id:     {}",
            message.node_id().map_or_else(|| "(none)".to_owned(), text)
        );
        println!(
            "  version:     {}",
            message
                .version()
                .map_or_else(|| "(none)".to_owned(), |value| format!("{:?}", text(value)))
        );
        let order: Vec<String> = message
            .argument_order()
            .iter()
            .map(|key| text(key))
            .collect();
        println!("  arguments:   {}", order.join(", "));
        if let Some(port) = observed.announced_port() {
            println!("  announced:   {port:?}");
        }
        println!(
            "  answered:    {} bytes",
            observed.answered().unwrap_or(&[]).len()
        );
        for departure in message.departures() {
            println!("  not BEP 5:   {}", departure.describe());
        }
        for refusal in observed.refusals() {
            println!("  observer:    {}", refusal.describe());
        }
    }
}
