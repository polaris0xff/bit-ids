//! Runs a peer-wire observer in both roles and prints what each side sent.
//!
//! ```text
//! cargo run -p bit-ids-probe --example peer-wire -- 20 [DIAL_ADDRESS]
//! ```
//!
//! It always accepts, printing the address a peer should connect to. Given a
//! second argument it also dials that loopback address, which is the other role:
//! a build can behave differently as the side that accepted.
//!
//! ⛔ It never says which client sent a handshake.

use core::fmt::Write as _;
use std::time::Duration;

use bit_ids_lab::Lab;
use bit_ids_probe::peer_wire::{ExtendedOffer, ExtensionProtocol, Offer, PeerWire};
use bit_ids_wire::peer_wire::INFO_HASH_LEN;

/// The torrent this observer claims to have. `OBS-08` will generate a real one.
const INFO_HASH: [u8; INFO_HASH_LEN] = [0x5a; INFO_HASH_LEN];

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
    let dial_to: Option<std::net::SocketAddr> =
        std::env::args().nth(2).and_then(|one| one.parse().ok());

    // ⚠ What this offers is a condition of the run, printed with the result.
    let offer = Offer {
        extension_protocol: ExtensionProtocol::Offered(ExtendedOffer {
            extensions: vec![(b"ut_metadata".to_vec(), 1), (b"ut_pex".to_vec(), 2)],
            client: Some(b"bit-ids-fixture/0".to_vec()),
            request_queue: Some(250),
            metadata_size: None,
        }),
        dht: false,
        fast: false,
    };
    println!("offered reserved {}", hex(&offer.reserved()));
    let peer = PeerWire::offering(offer, INFO_HASH);
    let mut lab = Lab::builder()
        .deadline(Duration::from_secs(seconds))
        .stream("peer-wire", peer.accepting())
        .expect("a canonical endpoint name")
        .start()
        .expect("a loopback lab binds");

    let address = lab.endpoint("peer-wire").expect("it was added").address();
    println!("peer {address}");
    println!("info_hash {}", hex(&INFO_HASH));
    if let Some(target) = dial_to {
        match lab.dial("peer-dial", target, peer.opening(), peer.dialling()) {
            Ok(endpoint) => println!("dialled {}", endpoint.address()),
            Err(error) => println!("dial refused: {error}"),
        }
    }
    println!("serving for {seconds}s");

    lab.wait();
    let journal = lab.shutdown();

    println!("streams: {}", peer.streams().len());
    println!("segments: {}", journal.segments().len());
    for stream in peer.streams() {
        println!("--- connection {} {:?}", stream.connection(), stream.role());
        println!("  bytes         {}", stream.raw().len());
        println!("  rebuilds      {}", stream.rebuilds_from_raw());
        match stream.handshake() {
            Some(handshake) => {
                println!(
                    "  protocol      {:?}",
                    String::from_utf8_lossy(handshake.protocol())
                );
                println!("  reserved      {}", hex(handshake.reserved()));
                println!("  info_hash     {}", hex(handshake.info_hash()));
                println!(
                    "  peer_id       {} ({:?})",
                    hex(handshake.peer_id()),
                    String::from_utf8_lossy(handshake.peer_id())
                );
            }
            None => println!("  no handshake arrived"),
        }
        let ids: Vec<String> = stream
            .messages()
            .iter()
            .map(|message| match message.id() {
                Some(id) => format!("{id}({})", message.payload().len()),
                None => "keep-alive".to_owned(),
            })
            .collect();
        println!("  messages      {}", ids.join(", "));
        match stream.extended_handshake() {
            Some(Ok(extended)) => {
                let names: Vec<String> = extended
                    .extension_ids()
                    .iter()
                    .map(|(name, id)| format!("{}={id}", String::from_utf8_lossy(name)))
                    .collect();
                println!("  bep10 m       {}", names.join(", "));
                println!(
                    "  bep10 v       {:?}",
                    extended.advertised_client().map(String::from_utf8_lossy)
                );
                println!("  bep10 reqq    {:?}", extended.integer(b"reqq"));
            }
            Some(Err(error)) => println!("  bep10 did not decode: {error}"),
            None => println!("  no extended handshake"),
        }
        if let Some(error) = stream.error() {
            println!("  stopped at    {error}");
        }
    }
}
