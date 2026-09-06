//! Runs a message stream encryption observer and prints what negotiated with it.
//!
//! ```text
//! cargo run -p bit-ids-probe --example mse -- 20
//! ```
//!
//! It prints the endpoint address, the observer's own public key and the info
//! hash it derives its stream keys from, completes exchanges until the deadline,
//! and then prints one block per exchange: what the build offered, the padding
//! it chose, and the `BitTorrent` handshake that came out of `IA`.
//!
//! ⭐ **The public key and the info hash are printed because a driver needs
//! them.** MSE's receiving side derives its keys from the shared secret and the
//! torrent, so anything driving this has to know which torrent is meant. In a
//! capture the build knows because it holds the `.torrent`.
//!
//! ⛔ It never says which client negotiated.
//!
//! ⭐ **Point something that is not this project at it.** MSE is Diffie-Hellman
//! over a published group, `RC4`, and `SHA-1`; a few dozen lines of Python using
//! its own `pow` is a complete and completely independent initiator.

use std::time::Duration;

use bit_ids_lab::Lab;
use bit_ids_lab::adjacent::{Capability, Surface, endpoint_name};
use bit_ids_probe::mse::{Mse, Selection};

fn hex(bytes: &[u8]) -> String {
    use core::fmt::Write as _;

    let mut out = String::with_capacity(bytes.len() * 2);
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

    // The torrent this observer is set up for. A capture takes it from the
    // generated fixture; here it is a declared constant so a driver can use it.
    let info_hash = [0x11_u8; 20];

    // ⛔ The switch, written out. Nothing defaults it on. The selection is a
    // condition of the measurement and is named here rather than defaulted.
    let observer = Mse::new(Capability::enable(Surface::Mse), info_hash, Selection::Rc4)
        .expect("the capability names this surface")
        .with_pad_b(vec![0xBB; 16]);

    let public_key = observer.public_key();

    let mut lab = Lab::builder()
        .deadline(Duration::from_secs(seconds))
        .stream(endpoint_name(Surface::Mse), observer.responder())
        .expect("a canonical endpoint name")
        .start()
        .expect("a loopback lab binds");

    let address = lab
        .endpoint(endpoint_name(Surface::Mse))
        .expect("it was added")
        .address();
    println!("mse tcp://{address}");
    println!("observer public key {}", hex(&public_key));
    println!("info hash {}", hex(&info_hash));
    println!("selecting rc4");
    println!("serving for {seconds}s");

    lab.wait();
    let journal = lab.shutdown();

    let exchanges = observer.exchanges();
    println!("exchanges: {}", exchanges.len());
    println!("dropped past the cap: {}", observer.dropped());
    println!("segments: {}", journal.segments().len());
    for (index, exchange) in exchanges.iter().enumerate() {
        println!("--- exchange {index}");
        println!("  their key:   {}", hex(exchange.their_key()));
        println!("  padA:        {} bytes", exchange.pad_a_len());
        println!("  selected:    {:?}", exchange.selected());
        match exchange.provide() {
            None => println!("  provide:     (did not decrypt)"),
            Some(provide) => {
                println!("  crypto:      {:#010x}", provide.crypto_provide);
                println!("    plaintext: {}", provide.offers_plaintext());
                println!("    rc4:       {}", provide.offers_rc4());
                println!("  verified:    {}", provide.verified());
                println!("  padC:        {} bytes", provide.pad.len());
                println!("  IA:          {} bytes", provide.initial_payload.len());
            }
        }
        // ⭐ `OBS-04`'s measurement through a different door. Printed as bytes;
        // nothing here turns it into a client name.
        if let Some(inside) = exchange.initial_payload()
            && let Ok(transcript) = bit_ids_wire::peer_wire::Transcript::parse(inside)
        {
            println!(
                "  peer id:     {:?}",
                String::from_utf8_lossy(transcript.handshake().peer_id())
            );
        }
        for refusal in exchange.refusals() {
            println!("  not MSE:     {}", refusal.describe());
        }
    }
}
