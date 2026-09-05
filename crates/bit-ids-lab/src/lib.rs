//! The isolated loopback observation lab: what a client is pointed at, and
//! what records every byte it sends.
//!
//! # Why a lab rather than a socket per observer
//!
//! A capture runs a binary this project downloaded minutes earlier and points
//! it at a network. `docs/capture-host.md` contains the blast radius at the
//! host, before anything is installed. This crate contains it at the socket:
//! ⛔ **every endpoint an observer gets is bound by
//! [`bind`], which refuses any address that is not loopback and
//! then reads back the address the socket actually got.** A guard on one of
//! several ways to open a socket is the shape
//! `docs/methodology/reviews.md` names as the most recurring hole there is, so
//! there is one way.
//!
//! # What it is not
//!
//! ⛔ **Nothing here speaks a protocol.** `OBS-02` through `OBS-05` own the
//! tracker and peer surfaces and supply a responder each; this crate moves
//! bytes, records them in order, and stops on time. Keeping the protocols out
//! is what lets one deadline, one loopback guard and one journal serve every
//! surface rather than each observer growing its own.
//!
//! What is here is the supervisor the observers plug into, the synthetic torrent
//! a capture hands a client, and the writer that turns a run into the
//! content-addressed evidence a manifest cites.
//!
//! [`torrent`] is `OBS-08`. ⚠ Its bytes are a function of its declared spec,
//! which is what lets a record cite the fixture it used and have that be
//! checkable, so the payload's byte stream is part of the contract rather than
//! an implementation detail.
//!
//! [`evidence`] is `OBS-09`. ⛔ It writes to a contract that already exists
//! rather than inventing one, and a transcript it writes is never scrubbed: the
//! bytes a build put on the wire are the measurement.
//!
//! # Starting one
//!
//! ```
//! use bit_ids_lab::{Lab, StreamReply};
//!
//! let lab = Lab::builder()
//!     .stream("tracker-http", |_connection, received: &[u8]| {
//!         if received.ends_with(b"\r\n\r\n") {
//!             StreamReply::Close {
//!                 send: b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n".to_vec(),
//!             }
//!         } else {
//!             StreamReply::NeedMore
//!         }
//!     })
//!     .expect("a canonical endpoint name")
//!     .start()
//!     .expect("loopback binds");
//!
//! let address = lab.endpoint("tracker-http").expect("it was added").address();
//! assert!(address.ip().is_loopback());
//! assert_ne!(address.port(), 0);
//!
//! let journal = lab.shutdown();
//! assert!(journal.segments().is_empty(), "nothing connected");
//! ```

pub mod bind;
pub mod endpoint;
pub mod evidence;
pub mod journal;
mod lab;
pub mod torrent;

pub use bind::BindError;
pub use endpoint::{
    ConnectionId, DEFAULT_MAX_PENDING_BYTES, DatagramResponder, StreamReply, StreamResponder,
};
pub use evidence::{Bundle, BundleError, Scrub, TranscriptOf};
pub use journal::{Journal, Segment};
pub use lab::{
    DEFAULT_DEADLINE, DEFAULT_DIAL_TIMEOUT, DEFAULT_MAX_CONNECTIONS, DEFAULT_POLL, Endpoint, Lab,
    LabBuilder, LabError, Transport,
};
pub use torrent::{SyntheticTorrent, TorrentError, TorrentSpec};
