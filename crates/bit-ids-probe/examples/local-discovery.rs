//! Runs a BEP 14 local discovery observer and prints what announced to it.
//!
//! ```text
//! cargo run -p bit-ids-probe --example local-discovery -- 20
//! ```
//!
//! It prints the endpoint address, serves until the deadline, and then prints
//! one block per announce: the field names in the case and order the build sent
//! them, the torrents it named, and anything BEP 14 does not describe.
//!
//! ⛔ **It answers nothing and it joins no group.** A client is pointed at the
//! printed loopback address instead of finding this by multicast, which is what
//! makes the run a lab run rather than something on somebody's LAN. The group
//! addresses are printed as what the lab refuses, not as where it listens.
//!
//! ⛔ It never says which client sent an announce.

use std::time::Duration;

use bit_ids_lab::adjacent::{Capability, Surface, endpoint_name};
use bit_ids_lab::{Lab, bind};
use bit_ids_probe::local_discovery::{GROUP_V4, GROUP_V6, LocalDiscovery};

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn main() {
    let seconds: u64 = std::env::args()
        .nth(1)
        .and_then(|argument| argument.parse().ok())
        .unwrap_or(20);

    // ⛔ The switch, written out. Nothing defaults it on.
    let observer = LocalDiscovery::new(Capability::enable(Surface::LocalDiscovery))
        .expect("the capability names this surface");

    let mut lab = Lab::builder()
        .deadline(Duration::from_secs(seconds))
        .datagram(endpoint_name(Surface::LocalDiscovery), observer.observing())
        .expect("a canonical endpoint name")
        .start()
        .expect("a loopback lab binds");

    let address = lab
        .endpoint(endpoint_name(Surface::LocalDiscovery))
        .expect("it was added")
        .address();
    println!("local discovery udp://{address}");
    println!("refused destinations: {GROUP_V4} {GROUP_V6}");
    println!("serving for {seconds}s, answering nothing");

    lab.wait();
    let journal = lab.shutdown();

    let announces = observer.announces();
    println!("announces: {}", announces.len());
    println!("dropped past the cap: {}", observer.dropped());
    println!("segments: {}", journal.segments().len());
    for (index, announce) in announces.iter().enumerate() {
        println!(
            "  {index} {} bytes, {}",
            announce.raw().len(),
            if announce.is_conforming() {
                "as BEP 14 describes"
            } else {
                "with findings"
            }
        );
        let order: Vec<String> = announce.field_order().iter().map(|one| text(one)).collect();
        println!("      fields    {}", order.join(", "));
        println!("      trailer   {:?}", announce.trailer());
        println!("      rebuilds  {}", announce.rebuilds_from_raw());
        if let Some(port) = announce.port() {
            println!("      port      {port}");
        }
        if let Some(cookie) = announce.cookie() {
            println!("      cookie    {:?}", text(cookie));
        }
        for hash in announce.info_hashes() {
            println!("      infohash  {}", text(hash));
        }
        for refusal in announce.refusals() {
            println!("      finding   {}", refusal.describe());
        }
    }

    // ⭐ Printed rather than assumed: the run ends by showing the guard still
    // refuses the group this protocol names.
    let socket = bind::datagram(address.ip()).expect("loopback binds");
    match bind::send_to(&socket, b"BT-SEARCH * HTTP/1.1\r\n\r\n", GROUP_V4) {
        Ok(sent) => println!("egress: {sent} bytes reached {GROUP_V4}, which is a defect"),
        Err(refusal) => println!("egress: {refusal}"),
    }
}
