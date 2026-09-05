//! `OBS-06`, driven: the adjacent surfaces in a running lab, and the proof that
//! nothing they do leaves the lab's allowed address set.
//!
//! ⛔ **The egress claim is made three ways, because no one of them is enough.**
//! A unit test on the guard proves the guard refuses; it does not prove the
//! guard is reached. A source sweep proves no module went around it; it does not
//! prove the guard is right. A driven run proves what actually crossed a socket;
//! it cannot prove what a different input would have done. The three together
//! are what the entry's Prove asks for, and each one is named where it sits.
//!
//! ⚠ **What is not proved here is that no packet left the host.** That needs a
//! capture on the interface, which needs privileges a test runner does not have
//! and `docs/capture-host.md` does not grant. `TODO/observer.md` carries it as a
//! residual rather than letting the three checks above read as more than they
//! are.

use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::time::Duration;

use bit_ids::canonical::Slug;
use bit_ids_lab::adjacent::{Capability, Surface, endpoint_name};
use bit_ids_lab::{Lab, bind};
use bit_ids_probe::local_discovery::{self, GROUP_V4, GROUP_V6, LocalDiscovery};

const ANNOUNCE: &[u8] = b"BT-SEARCH * HTTP/1.1\r\nHost: 239.192.152.143:6771\r\nPort: 51413\r\nInfohash: 0123456789abcdef0123456789abcdef01234567\r\ncookie: bit-ids\r\n\r\n\r\n";

/// Sends one datagram to a lab endpoint from a loopback socket.
fn announce_to(address: SocketAddr, payload: &[u8]) {
    let client = bind::datagram(IpAddr::V4(Ipv4Addr::LOCALHOST)).expect("loopback binds");
    bind::send_to(&client, payload, address).expect("the lab is on loopback");
}

#[test]
fn a_local_discovery_announce_is_recorded_and_nothing_is_answered() {
    let observer = LocalDiscovery::new(Capability::enable(Surface::LocalDiscovery))
        .expect("the capability names this surface");
    let lab = Lab::builder()
        .deadline(Duration::from_secs(5))
        .datagram(endpoint_name(Surface::LocalDiscovery), observer.observing())
        .expect("a canonical endpoint name")
        .start()
        .expect("loopback binds");

    let endpoint = lab
        .endpoint(endpoint_name(Surface::LocalDiscovery))
        .expect("it was added");
    assert!(endpoint.address().ip().is_loopback());
    announce_to(endpoint.address(), ANNOUNCE);

    // The lab records on arrival, so wait for the segment rather than for a
    // fixed sleep: a timing guess is a flake on a loaded runner.
    let name = Slug::parse(endpoint_name(Surface::LocalDiscovery)).expect("a canonical name");
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while observer.announces().is_empty() && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }

    let announces = observer.announces();
    assert_eq!(announces.len(), 1, "the announce was recorded");
    assert_eq!(
        announces[0].raw(),
        ANNOUNCE,
        "the bytes are the measurement"
    );
    assert!(
        announces[0].is_conforming(),
        "{:?}",
        announces[0].refusals()
    );
    assert_eq!(announces[0].port(), Some(51413));

    let journal = lab.shutdown();
    let segments = journal.for_endpoint(&name);
    // ⛔ **One segment, inbound.** BEP 14 defines no reply, so a second segment
    // would be this project inventing a protocol and then measuring what the
    // client did about it.
    assert_eq!(segments.len(), 1, "{segments:?}");
    assert_eq!(segments[0].bytes(), ANNOUNCE);
    assert_eq!(journal.received(&name), ANNOUNCE);
}

/// ⛔ The driven half of the egress claim: every byte that crossed a socket in
/// a real run went to an address inside the allowed set.
#[test]
fn every_endpoint_a_lab_hands_out_for_an_adjacent_surface_is_on_loopback() {
    let observer = LocalDiscovery::new(Capability::enable(Surface::LocalDiscovery))
        .expect("the capability names this surface");
    let lab = Lab::builder()
        .deadline(Duration::from_secs(2))
        .datagram(endpoint_name(Surface::LocalDiscovery), observer.observing())
        .expect("a canonical endpoint name")
        .start()
        .expect("loopback binds");

    for endpoint in lab.endpoints() {
        assert!(
            endpoint.address().ip().is_loopback(),
            "{} is at {}",
            endpoint.name(),
            endpoint.address()
        );
        assert_ne!(endpoint.address().port(), 0);
    }
    // ⚠ And the address the protocol itself names is not one of them. A lab
    // that had joined the group would be reachable from a real LAN.
    for endpoint in lab.endpoints() {
        assert_ne!(endpoint.address(), GROUP_V4);
        assert_ne!(endpoint.address(), GROUP_V6);
    }
    drop(lab.shutdown());
}

/// ⛔ The guard, driven from the module that would otherwise be the one to
/// reach the group: a socket the lab itself bound cannot be aimed at it.
#[test]
fn a_lab_socket_cannot_be_aimed_at_the_group_the_protocol_names() {
    let socket = bind::datagram(IpAddr::V4(Ipv4Addr::LOCALHOST)).expect("loopback binds");
    for group in [GROUP_V4, GROUP_V6] {
        let refusal = bind::send_to(&socket, ANNOUNCE, group).expect_err("outside loopback");
        assert!(
            matches!(refusal, bind::BindError::NotReachable { .. }),
            "{refusal}"
        );
        // The refusal names where it was aimed, so a reader of a failing run
        // does not have to guess which surface tried.
        assert!(refusal.to_string().contains(&group.ip().to_string()));
    }
}

/// ⭐ A refusal is not the same as being unable to send. Without this, the case
/// above passes over a `send_to` that fails for every destination.
#[test]
fn the_same_socket_reaches_a_loopback_destination() {
    let listener = bind::datagram(IpAddr::V4(Ipv4Addr::LOCALHOST)).expect("loopback binds");
    let address = listener.local_addr().expect("a bound socket has one");
    let socket = bind::datagram(IpAddr::V4(Ipv4Addr::LOCALHOST)).expect("loopback binds");
    assert_eq!(
        bind::send_to(&socket, ANNOUNCE, address).expect("loopback is allowed"),
        ANNOUNCE.len()
    );
    listener
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("a read timeout");
    let mut buffer = vec![0_u8; ANNOUNCE.len() + 16];
    let (read, _) = listener.recv_from(&mut buffer).expect("it arrives");
    assert_eq!(&buffer[..read], ANNOUNCE);
}

/// ⛔ The sweep, over the crate that holds the protocols.
///
/// `bit-ids-lab` greps its own source because it owns the sockets. This crate
/// constructs none, and that is the claim: an observer here that opened one
/// would bypass every guard the lab has, and `local_discovery` is exactly the
/// module that would be tempted. BEP 14 is a multicast protocol, and joining a
/// group means binding a socket the lab did not hand out.
///
/// ⚠ **Naming `std::net` is not opening a socket**, and the needles say so:
/// `local_discovery` holds the two multicast groups as `SocketAddr` values so
/// the lab can be shown refusing them, and `peer_exchange` reads compact peer
/// lists into `SocketAddrV4`. Those are addresses. What is forbidden is the
/// call that turns one into a socket, or that sends on one.
#[test]
fn no_observer_opens_a_socket_of_its_own() {
    const FORBIDDEN: [&str; 8] = [
        "TcpListener",
        "UdpSocket::bind",
        "TcpStream::connect",
        "UdpSocket::connect",
        ".send_to(",
        "join_multicast",
        "set_multicast",
        "std::net::UdpSocket::",
    ];

    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenders = Vec::new();
    let mut checked = Vec::new();
    for entry in std::fs::read_dir(&source).expect("the crate has a src directory") {
        let path = entry.expect("a readable directory entry").path();
        if path.extension().is_none_or(|suffix| suffix != "rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("a readable source file");
        checked.push(path.clone());
        for needle in FORBIDDEN {
            if text.contains(needle) {
                offenders.push(format!("{} names {needle}", path.display()));
            }
        }
    }
    // ⚠ A sweep that read nothing reports no offenders. The count is asserted
    // against the modules that exist, the way the lab's own sweep is.
    assert!(
        checked.len() >= 5,
        "only {} modules were read: {checked:?}",
        checked.len()
    );
    assert!(offenders.is_empty(), "{offenders:?}");
}

/// ⚠ The observer records what arrives and does not correct it.
///
/// A build that announces a port it is not listening on, or an info hash for a
/// torrent it does not have, is making a claim. The claim is the measurement.
#[test]
fn a_malformed_announce_is_recorded_with_what_is_wrong_with_it() {
    let observer = LocalDiscovery::new(Capability::enable(Surface::LocalDiscovery))
        .expect("the capability names this surface");
    let responder = observer.observing();
    assert!(responder(b"BT-SEARCH * HTTP/1.1\r\nPort: nope\r\n\r\n").is_none());

    let announces = observer.announces();
    assert_eq!(announces.len(), 1);
    assert!(!announces[0].is_conforming());
    let reasons: Vec<String> = announces[0]
        .refusals()
        .iter()
        .map(local_discovery::Refusal::describe)
        .collect();
    assert_eq!(reasons.len(), 2, "{reasons:?}");
    assert!(
        reasons.iter().any(|why| why.contains("no Infohash")),
        "{reasons:?}"
    );
    assert!(
        reasons.iter().any(|why| why.contains("not a number")),
        "{reasons:?}"
    );
}

/// A datagram endpoint answers through the lab and both directions are
/// recorded, whichever responder it carries.
///
/// ⚠ **This does NOT prove the reply went through the egress guard**, and it
/// was named as though it did until a plant showed otherwise: reverting
/// `serve_datagram` to `socket.send_to(..)` leaves every assertion here true,
/// because a loopback echo works either way. Proving the routing behaviourally
/// would need a datagram arriving with a forged source address, which needs a
/// raw socket. `no_module_outside_the_bind_guard_reaches_the_network` in
/// `bit-ids-lab` is what refuses that revert, by reading the source.
#[test]
fn a_datagram_endpoint_answers_and_both_directions_are_recorded() {
    let lab = Lab::builder()
        .deadline(Duration::from_secs(2))
        .datagram("echo", |received: &[u8]| Some(received.to_vec()))
        .expect("a canonical endpoint name")
        .start()
        .expect("loopback binds");
    let endpoint = lab.endpoint("echo").expect("it was added");

    let client = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).expect("a client socket binds");
    client
        .send_to(b"ping", endpoint.address())
        .expect("the endpoint is on loopback");
    client
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("a read timeout");
    let mut buffer = [0_u8; 16];
    let (read, _) = client.recv_from(&mut buffer).expect("the echo comes back");
    assert_eq!(&buffer[..read], b"ping");

    let journal = lab.shutdown();
    let name = Slug::parse("echo").expect("a canonical name");
    assert_eq!(journal.for_endpoint(&name).len(), 2, "in and back out");
}
