//! The one door to a bound socket, and the guard on it.
//!
//! ⛔ **Every socket this crate creates is created here.** A capture points a
//! binary this project downloaded minutes earlier at a network, and the whole
//! containment argument in `docs/capture-host.md` assumes the lab is reachable
//! from the host and from nowhere else. A second module calling
//! `TcpListener::bind` would be a gate on one of two doors into the same
//! action, which `docs/methodology/reviews.md` names as the most recurring hole
//! there is. `tests/lab_supervisor.rs` greps this crate's own source for that,
//! because the rule is otherwise only a comment.
//!
//! The guard runs twice on purpose. A requested address outside loopback is
//! refused before the syscall, and the address the socket actually got is read
//! back and refused again. Those are different facts: a bind request is what
//! was asked for and `local_addr` is what the kernel gave, and only the second
//! one describes where traffic can reach.
//!
//! ⛔ **A bound socket is half the containment.** Where a socket listens and
//! where it sends are different questions, and [`send_to`] is the answer to the
//! second: every outbound datagram this crate emits is addressed here, and a
//! destination outside loopback is refused before the packet exists. `OBS-06`
//! is what needed it, because the adjacent surfaces are precisely the ones
//! whose protocols carry a destination of their own.
//!
//! ⚠ **Writing to a TCP stream is not a third door, and the reason is worth
//! stating.** A `TcpStream` has one peer, fixed when it was accepted or
//! connected, and both of those came through this module. So a write can only
//! reach an address a guard already approved. A datagram is the exception
//! because its destination is chosen per packet, which is why it needs a guard
//! per packet.

use core::fmt;
use std::io;
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::time::Duration;

/// Why a socket was not created.
#[derive(Debug)]
pub enum BindError {
    /// The requested address is not a loopback address.
    NotLoopback {
        /// What was asked for.
        requested: IpAddr,
    },
    /// The socket bound, and the address it got is not on loopback.
    BoundElsewhere {
        /// What was asked for.
        requested: IpAddr,
        /// What the kernel reported after binding.
        bound: SocketAddr,
    },
    /// An outbound datagram was addressed outside the lab's allowed set.
    ///
    /// ⛔ **This is the one `OBS-06` needs and the bind guards could not
    /// give.** A bind says where a socket listens; it says nothing about
    /// where that socket sends. The adjacent surfaces are exactly the ones
    /// whose protocols name a destination. Local discovery multicasts to a
    /// group fixed by BEP 14, and a DHT's first act is to reach a bootstrap
    /// node, so a lab that guarded only its binds would have contained
    /// every surface except the ones that leave.
    NotReachable {
        /// Where the datagram was addressed.
        destination: SocketAddr,
    },
    /// The operating system refused the bind.
    Io(io::Error),
}

impl fmt::Display for BindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotLoopback { requested } => write!(
                f,
                "refusing to bind {requested}: a lab endpoint is on loopback and nowhere else"
            ),
            Self::BoundElsewhere { requested, bound } => write!(
                f,
                "asked for {requested} and the socket reports {bound}, which is not loopback"
            ),
            Self::NotReachable { destination } => write!(
                f,
                "refusing to send to {destination}: a lab packet goes to loopback and nowhere else"
            ),
            Self::Io(error) => write!(f, "the operating system refused the bind: {error}"),
        }
    }
}

impl core::error::Error for BindError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for BindError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Refuses a requested address that is not on loopback.
///
/// ⚠ An IPv4-mapped IPv6 address such as `::ffff:127.0.0.1` is refused, because
/// [`std::net::Ipv6Addr::is_loopback`] answers for `::1` alone. That is the
/// pessimistic direction and it costs a spelling nobody needs here; the
/// permissive reading would have to unwrap the mapping correctly to stay safe,
/// and being wrong there puts a capture on a routable address.
///
/// # Errors
///
/// Returns [`BindError::NotLoopback`] for anything else, including
/// `0.0.0.0` and `::`, which listen on every interface the host has.
pub const fn check_requested(requested: IpAddr) -> Result<(), BindError> {
    if requested.is_loopback() {
        Ok(())
    } else {
        Err(BindError::NotLoopback { requested })
    }
}

/// Refuses an address a socket actually got that is not on loopback.
///
/// Split out from the bind itself so it can be driven with an address a real
/// socket on this host cannot be made to produce on demand. The bind path calls
/// it with what `local_addr` reported.
///
/// # Errors
///
/// Returns [`BindError::BoundElsewhere`] when `bound` is not on loopback.
pub const fn check_bound(requested: IpAddr, bound: SocketAddr) -> Result<(), BindError> {
    if bound.ip().is_loopback() {
        Ok(())
    } else {
        Err(BindError::BoundElsewhere { requested, bound })
    }
}

/// Binds a TCP listener on loopback, on a port the operating system chooses.
///
/// Port zero is not a convenience. Two labs on one host that both named a port
/// collide, and the second one fails in whichever of them happens to start
/// second, which is a flake rather than a defect report.
///
/// # Errors
///
/// Returns [`BindError::NotLoopback`] before the syscall,
/// [`BindError::BoundElsewhere`] after it, and [`BindError::Io`] when the
/// operating system refuses.
pub fn stream(requested: IpAddr) -> Result<TcpListener, BindError> {
    check_requested(requested)?;
    let listener = TcpListener::bind(SocketAddr::new(requested, 0))?;
    check_bound(requested, listener.local_addr()?)?;
    Ok(listener)
}

/// Dials a loopback address, with a bounded wait.
///
/// ⛔ **A dial goes through this guard for the reason a bind does.** The module
/// documentation says every socket this crate creates is created here, and an
/// outbound connection is a socket: an observer that dialled wherever it was
/// told would reach off the host from inside a lab whose whole argument is that
/// it cannot. `tests/lab_supervisor.rs` greps for it.
///
/// ⚠ The timeout is not a convenience. `TcpStream::connect` to an address that
/// drops rather than refuses waits on the operating system's own retry
/// schedule, which outlasts any capture deadline, and the lab's deadline cannot
/// interrupt a thread blocked in a syscall.
///
/// # Errors
///
/// Returns [`BindError::NotLoopback`] before the syscall,
/// [`BindError::BoundElsewhere`] when the connected peer is not on loopback,
/// and [`BindError::Io`] when the connection is refused or times out.
pub fn dial(address: SocketAddr, timeout: Duration) -> Result<TcpStream, BindError> {
    check_requested(address.ip())?;
    let stream = TcpStream::connect_timeout(&address, timeout)?;
    // Read back, for the reason the bind path reads back: what was asked for
    // and what the kernel connected are different facts.
    check_bound(address.ip(), stream.peer_addr()?)?;
    Ok(stream)
}

/// Sends one datagram, to a destination inside the lab's allowed address set.
///
/// ⛔ **The door sweep found this one open.** Every socket went through this
/// module and the reply path did not: `endpoint::serve_datagram` called
/// `send_to` on the bound socket directly, with the source address of whatever
/// had just arrived. A UDP source address is a claim by the sender rather than
/// a fact the kernel checked, so a local process able to forge one could have
/// aimed a lab's replies off the host, from a socket the loopback guard had
/// already approved. That is the shape `docs/methodology/reviews.md` names: a
/// gate on one of several doors into the same action.
///
/// ⚠ **The guard here is the check before the syscall, and that is all it can
/// be.** A bind and a dial each read back what the kernel actually did, because
/// a listener has a local address and a connected stream has a peer. A datagram
/// socket has neither for the packet it just sent: there is no
/// after-the-fact reading of where a datagram went. So the destination must be
/// correct before it is passed, and this is the only place that decides.
///
/// # Errors
///
/// Returns [`BindError::NotReachable`] before the syscall for any destination
/// outside loopback, and [`BindError::Io`] when the operating system refuses
/// the send.
pub fn send_to(
    socket: &UdpSocket,
    payload: &[u8],
    destination: SocketAddr,
) -> Result<usize, BindError> {
    if !destination.ip().is_loopback() {
        return Err(BindError::NotReachable { destination });
    }
    Ok(socket.send_to(payload, destination)?)
}

/// Binds a UDP socket on loopback, on a port the operating system chooses.
///
/// # Errors
///
/// The same three as [`stream`].
pub fn datagram(requested: IpAddr) -> Result<UdpSocket, BindError> {
    check_requested(requested)?;
    let socket = UdpSocket::bind(SocketAddr::new(requested, 0))?;
    check_bound(requested, socket.local_addr()?)?;
    Ok(socket)
}

#[cfg(test)]
mod tests {
    use super::{BindError, check_bound, check_requested, datagram, stream};
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
    use std::time::Duration;

    fn v4(a: u8, b: u8, c: u8, d: u8) -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(a, b, c, d))
    }

    #[test]
    fn loopback_is_accepted_in_both_families() {
        assert!(check_requested(v4(127, 0, 0, 1)).is_ok());
        assert!(check_requested(v4(127, 9, 9, 9)).is_ok());
        assert!(check_requested(IpAddr::V6(Ipv6Addr::LOCALHOST)).is_ok());
    }

    #[test]
    fn the_wildcard_and_a_routable_address_are_refused() {
        for requested in [
            v4(0, 0, 0, 0),
            v4(192, 168, 1, 10),
            v4(8, 8, 8, 8),
            IpAddr::V6(Ipv6Addr::UNSPECIFIED),
            // ⚠ The mapped form of a loopback address. Refused, and the doc
            // comment says why that is the direction to be wrong in.
            IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0x7f00, 1)),
        ] {
            assert!(
                matches!(
                    check_requested(requested),
                    Err(BindError::NotLoopback { .. })
                ),
                "{requested} was not refused"
            );
        }
    }

    #[test]
    fn an_address_the_socket_reports_off_loopback_is_refused_after_binding() {
        let bound = SocketAddr::new(v4(192, 168, 1, 10), 6881);
        assert!(matches!(
            check_bound(v4(127, 0, 0, 1), bound),
            Err(BindError::BoundElsewhere { .. })
        ));
        let ok = SocketAddr::new(v4(127, 0, 0, 1), 6881);
        assert!(check_bound(v4(127, 0, 0, 1), ok).is_ok());
    }

    #[test]
    fn a_bound_socket_reports_a_loopback_address_and_a_chosen_port() {
        let listener = stream(v4(127, 0, 0, 1)).expect("loopback binds");
        let address = listener
            .local_addr()
            .expect("a bound listener has an address");
        assert!(address.ip().is_loopback());
        assert_ne!(address.port(), 0, "port zero means the kernel chose one");

        let socket = datagram(v4(127, 0, 0, 1)).expect("loopback binds");
        let address = socket.local_addr().expect("a bound socket has an address");
        assert!(address.ip().is_loopback());
        assert_ne!(address.port(), 0);
    }

    #[test]
    fn neither_binder_reaches_the_syscall_for_a_non_loopback_address() {
        assert!(matches!(
            stream(v4(0, 0, 0, 0)),
            Err(BindError::NotLoopback { .. })
        ));
        assert!(matches!(
            datagram(v4(0, 0, 0, 0)),
            Err(BindError::NotLoopback { .. })
        ));
    }

    #[test]
    fn dialling_off_loopback_is_refused_before_a_packet_leaves() {
        // ⛔ 198.51.100.0/24 is TEST-NET-2, reserved for documentation. If this
        // guard ever stopped firing the test would try to reach it, which is
        // the failure being prevented, so it must be an address that goes
        // nowhere rather than one that answers.
        let elsewhere = SocketAddr::new(v4(198, 51, 100, 7), 6881);
        assert!(matches!(
            super::dial(elsewhere, Duration::from_millis(50)),
            Err(BindError::NotLoopback { .. })
        ));
    }

    #[test]
    fn dialling_a_loopback_listener_connects_and_reads_the_peer_back() {
        let listener = stream(v4(127, 0, 0, 1)).expect("loopback binds");
        let address = listener.local_addr().expect("a bound listener has one");
        let dialled = super::dial(address, Duration::from_secs(5)).expect("it connects");
        assert!(
            dialled
                .peer_addr()
                .expect("a connected stream has a peer")
                .ip()
                .is_loopback()
        );
    }

    /// ⛔ The egress-negative case `OBS-06` asks for, over the addresses the
    /// adjacent protocols actually name.
    #[test]
    fn a_datagram_addressed_outside_loopback_never_leaves() {
        let socket = datagram(v4(127, 0, 0, 1)).expect("loopback binds");
        // The BEP 14 groups, the ones a local-discovery observer would be
        // handed by its own specification; 198.51.100.0/24 is TEST-NET-2,
        // standing in for a DHT bootstrap node without naming somebody's host.
        let elsewhere = [
            SocketAddr::new(v4(239, 192, 152, 143), 6771),
            SocketAddr::new(
                IpAddr::V6(Ipv6Addr::new(0xff15, 0, 0, 0, 0, 0, 0xefc0, 0x988f)),
                6771,
            ),
            SocketAddr::new(v4(198, 51, 100, 7), 6881),
            SocketAddr::new(v4(255, 255, 255, 255), 6771),
            SocketAddr::new(v4(0, 0, 0, 0), 6881),
        ];
        for destination in elsewhere {
            assert!(
                matches!(
                    super::send_to(&socket, b"BT-SEARCH * HTTP/1.1\r\n", destination),
                    Err(BindError::NotReachable { .. })
                ),
                "{destination} was not refused"
            );
        }
    }

    #[test]
    fn a_datagram_addressed_to_loopback_arrives_and_reports_its_length() {
        let listener = datagram(v4(127, 0, 0, 1)).expect("loopback binds");
        let address = listener.local_addr().expect("a bound socket has one");
        let sender = datagram(v4(127, 0, 0, 1)).expect("loopback binds");
        let payload = b"BT-SEARCH * HTTP/1.1\r\n\r\n\r\n";
        let sent = super::send_to(&sender, payload, address).expect("loopback is allowed");
        assert_eq!(sent, payload.len());

        listener
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("a read timeout");
        let mut buffer = [0_u8; 64];
        let (read, from) = listener
            .recv_from(&mut buffer)
            .expect("the datagram arrives");
        assert_eq!(&buffer[..read], payload);
        assert!(from.ip().is_loopback());
    }

    #[test]
    fn dialling_a_closed_loopback_port_fails_rather_than_hanging() {
        // The port is released before the dial, so nothing is listening on it.
        let port = {
            let listener = stream(v4(127, 0, 0, 1)).expect("loopback binds");
            listener.local_addr().expect("an address").port()
        };
        let address = SocketAddr::new(v4(127, 0, 0, 1), port);
        assert!(matches!(
            super::dial(address, Duration::from_secs(5)),
            Err(BindError::Io(_))
        ));
    }
}
