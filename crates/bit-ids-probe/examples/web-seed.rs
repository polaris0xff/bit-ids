//! Runs a BEP 19 web-seed observer and prints what fetched from it.
//!
//! ```text
//! cargo run -p bit-ids-probe --example web-seed -- 20
//! ```
//!
//! It generates the torrent, prints the endpoint address and the `url-list`
//! entry that names it, serves the torrent's own payload until the deadline, and
//! then prints one block per fetch: the header names in the case and order the
//! build's HTTP library sent them, what it asked for, and what it was answered
//! with.
//!
//! ⭐ **The point of this surface is that the identity is not the client's.** A
//! `User-Agent` here is usually the HTTP library's own string, so two clients
//! built on one library look alike where every other surface tells them apart.
//!
//! ⛔ **It serves the torrent's own payload**, so a build's piece hashes pass. A
//! seed answering anything else gets blacklisted and the run measures a build
//! reacting to a broken server.
//!
//! ⛔ It never says which client fetched.
//!
//! ⭐ **Point something that is not this project at it.** `curl` is a complete
//! HTTP client nobody here wrote:
//!
//! ```text
//! curl -sS -D - -r 64-127 -o /dev/null http://127.0.0.1:PORT/payload
//! ```

use std::time::Duration;

use bit_ids_lab::adjacent::{Capability, Surface, endpoint_name};
use bit_ids_lab::torrent::WebSeed;
use bit_ids_lab::{Lab, SyntheticTorrent, TorrentSpec};
use bit_ids_probe::web_seed::WebSeedServer;

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn main() {
    let seconds: u64 = std::env::args()
        .nth(1)
        .and_then(|argument| argument.parse().ok())
        .unwrap_or(20);

    let torrent = SyntheticTorrent::generate(TorrentSpec::default())
        .expect("the default spec describes a usable torrent");

    // ⛔ The switch, written out. Nothing defaults it on.
    let observer = WebSeedServer::new(
        Capability::enable(Surface::WebSeed),
        torrent.payload().to_vec(),
    )
    .expect("the capability names this surface");

    let mut lab = Lab::builder()
        .deadline(Duration::from_secs(seconds))
        .stream(endpoint_name(Surface::WebSeed), observer.responder())
        .expect("a canonical endpoint name")
        .start()
        .expect("a loopback lab binds");

    let address = lab
        .endpoint(endpoint_name(Surface::WebSeed))
        .expect("it was added")
        .address();
    println!("web seed http://{address}/payload");
    println!("payload: {} bytes", torrent.payload().len());

    // ⚠ The URL a torrent would carry, built through the guard that refuses an
    // address outside the lab. A `url-list` entry is where a build is told to
    // go, and it is checked at construction rather than at fetch time.
    if let std::net::SocketAddr::V4(v4) = address {
        match WebSeed::new(v4, "/payload") {
            Ok(seed) => println!("url-list entry: {}", seed.url()),
            Err(error) => println!("the lab endpoint was refused as a web seed: {error}"),
        }
    }
    println!("serving for {seconds}s");

    lab.wait();
    let journal = lab.shutdown();

    let fetches = observer.fetches();
    println!("fetches: {}", fetches.len());
    println!("dropped past the cap: {}", observer.dropped());
    println!("segments: {}", journal.segments().len());
    for (index, fetch) in fetches.iter().enumerate() {
        println!("--- fetch {index}");
        let order: Vec<String> = fetch.header_order().iter().map(|name| text(name)).collect();
        println!("  header order: {}", order.join(", "));
        println!(
            "  user agent:   {}",
            fetch
                .user_agent()
                .map_or_else(|| "(none)".to_owned(), |value| format!("{:?}", text(value)))
        );
        println!("  requested:    {:?}", fetch.requested());
        println!(
            "  answered:     {} with {} bytes",
            fetch.status().unwrap_or(0),
            fetch.served()
        );
        for refusal in fetch.refusals() {
            println!("  not BEP 19:   {}", refusal.describe());
        }
    }
}
