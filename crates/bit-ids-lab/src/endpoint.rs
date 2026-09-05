//! What an endpoint does with a connection, and the state every endpoint
//! shares with the supervisor.
//!
//! The loops here know nothing about any protocol. `OBS-02` through `OBS-05`
//! supply a responder per surface and this module moves bytes and records them.
//! Keeping the protocol out is what lets one deadline, one loopback guard and
//! one journal cover every surface instead of each observer growing its own.

use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use bit_ids::canonical::Slug;
use bit_ids_wire::tracker_udp::Direction;

use crate::journal::Segment;

/// How much of a stream is read in one call.
///
/// ⚠ The size is evidence, not a tuning knob. A segment is one read, and a
/// buffer smaller than what the kernel has waiting splits a run of bytes the
/// target may have written as one. 64 KiB is above any handshake, tracker
/// request or early message this project observes.
const STREAM_READ_BYTES: usize = 64 * 1024;

/// How much of a datagram is read in one call.
///
/// ⛔ Above the largest datagram an IPv4 host can deliver, which is 65507
/// bytes. `recv_from` truncates a datagram that does not fit and reports the
/// truncated length, so a smaller buffer would silently record a short packet
/// as a whole one. `docs/conventions/forbidden-patterns.md` calls that shape
/// out by name.
const DATAGRAM_READ_BYTES: usize = 65536;

/// The largest payload an IPv4 datagram can carry: 65535 less the 20-byte IP
/// header and the 8-byte UDP header.
///
/// ⭐ The constant above is a claim about this number and
/// `the_read_buffers_are_above_what_the_protocols_can_deliver` is what holds it.
/// The guard-mutation pass shrank the buffer to four bytes and every test still
/// passed, because no fixture datagram was larger than three: a corpus only
/// tests the defects it contains an example of.
const MAX_IPV4_UDP_PAYLOAD: usize = 65507;

// ⛔ A datagram larger than the buffer is truncated by `recv_from`, which
// reports the truncated length and nothing else, so the record would say a
// short packet arrived whole. A small stream buffer does not truncate, it
// splits, and a segment is meant to be one read.
//
// These are compile-time rather than a test on purpose: shrinking either
// constant then stops the build for every consumer instead of failing one
// suite that somebody could have skipped.
const _: () = assert!(
    DATAGRAM_READ_BYTES > MAX_IPV4_UDP_PAYLOAD,
    "the datagram buffer cannot hold every datagram a host can deliver"
);
const _: () = assert!(
    STREAM_READ_BYTES > MAX_IPV4_UDP_PAYLOAD,
    "the stream buffer would split messages the kernel delivered whole"
);

/// What a stream endpoint does with the bytes it has so far.
///
/// The responder sees everything read on the connection that it has not yet
/// consumed, so a surface framed by messages can answer one message at a time
/// rather than being handed a fresh buffer per read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StreamReply {
    /// Not a complete unit yet. Nothing is consumed and nothing is sent.
    NeedMore,
    /// The responder consumed `consumed` leading bytes and wants `send`
    /// written back. The connection stays open.
    Answer {
        /// How many leading bytes of the buffer the responder used.
        consumed: usize,
        /// What to write back. May be empty.
        send: Vec<u8>,
    },
    /// Write `send`, then close the connection.
    Close {
        /// What to write back before closing. May be empty.
        send: Vec<u8>,
    },
}

/// A stream endpoint's protocol behaviour.
pub type StreamResponder = dyn Fn(&[u8]) -> StreamReply + Send + Sync + 'static;

/// A datagram endpoint's protocol behaviour: one packet in, at most one out.
pub type DatagramResponder = dyn Fn(&[u8]) -> Option<Vec<u8>> + Send + Sync + 'static;

/// State every worker shares with the supervisor.
pub(crate) struct Shared {
    started: Instant,
    deadline: Duration,
    poll: Duration,
    max_connections: usize,
    stop: AtomicBool,
    expired: AtomicBool,
    journal: Mutex<Vec<Segment>>,
}

impl Shared {
    pub(crate) fn new(deadline: Duration, poll: Duration, max_connections: usize) -> Self {
        Self {
            started: Instant::now(),
            deadline,
            poll,
            max_connections,
            stop: AtomicBool::new(false),
            expired: AtomicBool::new(false),
            journal: Mutex::new(Vec::new()),
        }
    }

    pub(crate) fn request_stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }

    pub(crate) fn expired(&self) -> bool {
        self.expired.load(Ordering::SeqCst)
    }

    /// Whether the lab's deadline has passed, recording that it did.
    ///
    /// A lab that ran out of time and one that was told to stop are different
    /// outcomes for a capture, so the reason is kept rather than inferred from
    /// an empty journal.
    fn deadline_passed(&self) -> bool {
        if self.started.elapsed() >= self.deadline {
            self.expired.store(true, Ordering::SeqCst);
            true
        } else {
            false
        }
    }

    fn should_stop(&self) -> bool {
        self.stop.load(Ordering::SeqCst) || self.deadline_passed()
    }

    /// Appends one segment under the lock that defines the journal's order.
    ///
    /// ⚠ A poisoned lock is recovered rather than propagated. Poisoning means
    /// some worker panicked, and the bytes recorded before that are still
    /// evidence; throwing a whole run's transcript away because one connection
    /// handler died is a worse failure than the one that caused it.
    fn record(&self, endpoint: &Slug, direction: Direction, bytes: Vec<u8>) {
        if bytes.is_empty() {
            return;
        }
        let segment = Segment::new(endpoint.clone(), self.started.elapsed(), direction, bytes);
        self.journal
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(segment);
    }

    pub(crate) fn take_journal(&self) -> Vec<Segment> {
        core::mem::take(&mut *self.journal.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// A copy of what is recorded so far, leaving the journal in place.
    pub(crate) fn snapshot(&self) -> Vec<Segment> {
        self.journal
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// Whether a read returned nothing because there was nothing yet.
///
/// ⚠ Two kinds, and both are needed. A timed-out read on a Unix host reports
/// `WouldBlock` and on Windows reports `TimedOut`, so a loop that matched one
/// of them would treat a quiet moment as a broken socket on the other platform
/// and close the connection under a client that was still thinking.
fn is_quiet(kind: ErrorKind) -> bool {
    matches!(kind, ErrorKind::WouldBlock | ErrorKind::TimedOut)
}

/// Accepts connections until the lab stops, one handler thread each.
///
/// A handler per connection rather than a sequential loop: a client that opens
/// a peer connection, holds it, and then announces would otherwise wait on
/// itself. `max_connections` bounds the threads so a client that opens sockets
/// in a loop cannot exhaust the host.
pub(crate) fn serve_stream(
    shared: &Arc<Shared>,
    name: &Slug,
    listener: TcpListener,
    responder: &Arc<StreamResponder>,
) {
    // The accept call must not block, or the loop cannot notice the deadline or
    // a stop request until a client happens to connect.
    if listener.set_nonblocking(true).is_err() {
        return;
    }
    let mut handlers: Vec<JoinHandle<()>> = Vec::new();
    while !shared.should_stop() {
        handlers.retain(|handler| !handler.is_finished());
        match listener.accept() {
            Ok((stream, _)) => {
                if handlers.len() >= shared.max_connections {
                    let _ = stream.shutdown(Shutdown::Both);
                    continue;
                }
                let shared = Arc::clone(shared);
                let name = name.clone();
                let responder = Arc::clone(responder);
                // A refused thread leaves the listener up, so the next accept
                // can succeed once a handler finishes rather than the endpoint
                // going quiet for the rest of the run.
                if let Ok(handler) = std::thread::Builder::new()
                    .name(format!("bit-ids-lab-{name}"))
                    .spawn(move || serve_connection(&shared, &name, stream, responder.as_ref()))
                {
                    handlers.push(handler);
                }
            }
            Err(error) if is_quiet(error.kind()) => std::thread::sleep(shared.poll),
            // The listener itself failed. Nothing further can be accepted on
            // it, and a tight loop over a permanent error would spin a core.
            Err(_) => break,
        }
    }
    for handler in handlers {
        let _ = handler.join();
    }
    // ⭐ Explicit, and after the join. This is what closes the port, and
    // `shutting_a_lab_down_releases_every_port_it_held` is the test that says
    // so. Leaving it to the enclosing closure would put the release outside the
    // function that owns the loop, where a later edit could keep the listener
    // alive without anything reading differently.
    drop(listener);
}

fn serve_connection(
    shared: &Arc<Shared>,
    name: &Slug,
    mut stream: TcpStream,
    responder: &StreamResponder,
) {
    // Inherited from the listener on some platforms, so it is set explicitly.
    // The read timeout is what lets a silent client be noticed by the deadline.
    if stream.set_nonblocking(false).is_err() || stream.set_read_timeout(Some(shared.poll)).is_err()
    {
        return;
    }
    let mut pending: Vec<u8> = Vec::new();
    let mut buffer = vec![0_u8; STREAM_READ_BYTES];
    while !shared.should_stop() {
        let read = match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => read,
            Err(error) if is_quiet(error.kind()) => continue,
            Err(_) => break,
        };
        shared.record(name, Direction::FromTarget, buffer[..read].to_vec());
        pending.extend_from_slice(&buffer[..read]);
        // ⛔ The responder is offered the buffer until it stops consuming, not
        // once per read. One read can carry two messages, and answering only
        // the first left the second sitting in the buffer until more bytes
        // happened to arrive. A client that sends two and waits for two then
        // waits forever. The guard-mutation pass found it: replacing the drain
        // with a clear changed nothing, because no fixture ever put two units
        // in one write.
        let mut done = false;
        loop {
            match responder(&pending) {
                StreamReply::NeedMore => break,
                StreamReply::Answer { consumed, send } => {
                    assert!(
                        consumed <= pending.len(),
                        "endpoint {name}: responder consumed {consumed} of {} buffered bytes",
                        pending.len()
                    );
                    pending.drain(..consumed);
                    if !write_and_record(shared, name, &mut stream, &send) {
                        done = true;
                        break;
                    }
                    // ⚠ Consuming nothing is the termination condition. A
                    // responder may answer without consuming, and re-offering
                    // the same buffer to it forever is a spinning core rather
                    // than a stalled connection.
                    if consumed == 0 || pending.is_empty() {
                        break;
                    }
                }
                StreamReply::Close { send } => {
                    write_and_record(shared, name, &mut stream, &send);
                    done = true;
                    break;
                }
            }
        }
        if done {
            break;
        }
    }
    let _ = stream.shutdown(Shutdown::Both);
}

/// Writes a reply and records it, reporting whether the connection survived.
///
/// ⚠ The segment is recorded only after the write returns. Recording what was
/// meant to be sent would put bytes in a transcript that never reached the
/// target, and a replay of that transcript would not reproduce the run.
fn write_and_record(
    shared: &Arc<Shared>,
    name: &Slug,
    stream: &mut TcpStream,
    send: &[u8],
) -> bool {
    if send.is_empty() {
        return true;
    }
    if stream.write_all(send).is_err() || stream.flush().is_err() {
        return false;
    }
    shared.record(name, Direction::ToTarget, send.to_vec());
    true
}

/// Reads datagrams until the lab stops, answering each with the responder.
pub(crate) fn serve_datagram(
    shared: &Arc<Shared>,
    name: &Slug,
    socket: UdpSocket,
    responder: &Arc<DatagramResponder>,
) {
    if socket.set_read_timeout(Some(shared.poll)).is_err() {
        return;
    }
    let mut buffer = vec![0_u8; DATAGRAM_READ_BYTES];
    while !shared.should_stop() {
        let (read, from) = match socket.recv_from(&mut buffer) {
            Ok(pair) => pair,
            Err(error) if is_quiet(error.kind()) => continue,
            Err(_) => break,
        };
        shared.record(name, Direction::FromTarget, buffer[..read].to_vec());
        if let Some(send) = responder(&buffer[..read])
            && !send.is_empty()
            && socket.send_to(&send, from).is_ok()
        {
            shared.record(name, Direction::ToTarget, send);
        }
    }
    // Explicit for the reason `serve_stream` gives: this is what frees the port.
    drop(socket);
}

#[cfg(test)]
mod tests {
    use super::is_quiet;
    use std::io::ErrorKind;

    #[test]
    fn both_platforms_spellings_of_a_timed_out_read_count_as_quiet() {
        // Unix reports WouldBlock and Windows reports TimedOut for the same
        // event. Matching one of them closes a live connection on the other.
        assert!(is_quiet(ErrorKind::WouldBlock));
        assert!(is_quiet(ErrorKind::TimedOut));
        assert!(!is_quiet(ErrorKind::ConnectionReset));
        assert!(!is_quiet(ErrorKind::BrokenPipe));
    }
}
