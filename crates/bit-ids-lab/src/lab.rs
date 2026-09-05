//! The supervisor: what binds, what stops it, and what it hands back.

use core::fmt;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, UdpSocket};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use bit_ids::canonical::{CanonicalError, Slug};

use crate::bind::{self, BindError};
use crate::endpoint::{DatagramResponder, Shared, StreamResponder, serve_datagram, serve_stream};
use crate::journal::Journal;

/// How long a lab serves before it stops itself.
///
/// A capture that has not finished by then has a client that is not going to
/// answer, and a run that hangs is a CI job that hangs. The value is a default
/// and every lab may set its own.
pub const DEFAULT_DEADLINE: Duration = Duration::from_secs(30);

/// How long a worker waits on a quiet socket before re-checking the deadline.
///
/// It bounds how late a stop is noticed, not how fast the lab answers: a socket
/// with bytes waiting returns immediately.
pub const DEFAULT_POLL: Duration = Duration::from_millis(10);

/// How many connections one stream endpoint serves at once.
pub const DEFAULT_MAX_CONNECTIONS: usize = 64;

/// Which kind of socket an endpoint is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Transport {
    /// TCP.
    Stream,
    /// UDP.
    Datagram,
}

/// One bound endpoint, and where a client reaches it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Endpoint {
    name: Slug,
    address: SocketAddr,
    transport: Transport,
}

impl Endpoint {
    /// The name the lab knows this endpoint by, which is also its name in the
    /// journal.
    #[must_use]
    pub const fn name(&self) -> &Slug {
        &self.name
    }

    /// The address the socket reported after binding.
    ///
    /// ⭐ Read back from the socket, never the address that was requested. The
    /// port is one the operating system chose, and a lab that printed what it
    /// asked for would print `:0`.
    #[must_use]
    pub const fn address(&self) -> SocketAddr {
        self.address
    }

    /// TCP or UDP.
    #[must_use]
    pub const fn transport(&self) -> Transport {
        self.transport
    }
}

/// Why a lab did not start.
#[derive(Debug)]
pub enum LabError {
    /// An endpoint name is not a canonical identifier.
    Name(CanonicalError),
    /// Two endpoints were given the same name.
    DuplicateEndpoint(Slug),
    /// A lab with no endpoint is a lab nothing can reach.
    NoEndpoints,
    /// A stream endpoint was capped at zero connections, which accepts every
    /// connection and closes it at once. That reads to a client as a server
    /// that is up and broken, so it is refused rather than served.
    NoConnectionsAllowed,
    /// A socket was refused. [`BindError::NotLoopback`] is the interesting one.
    Bind(BindError),
    /// The host refused a worker thread.
    Thread(io::Error),
}

impl fmt::Display for LabError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Name(error) => write!(f, "endpoint name: {error}"),
            Self::DuplicateEndpoint(name) => {
                write!(f, "two endpoints are both called {name:?}")
            }
            Self::NoEndpoints => f.write_str("a lab with no endpoint observes nothing"),
            Self::NoConnectionsAllowed => {
                f.write_str("max_connections is zero, so every connection would be closed at once")
            }
            Self::Bind(error) => write!(f, "{error}"),
            Self::Thread(error) => write!(f, "could not start a worker: {error}"),
        }
    }
}

impl core::error::Error for LabError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Name(error) => Some(error),
            Self::Bind(error) => Some(error),
            Self::Thread(error) => Some(error),
            _ => None,
        }
    }
}

impl From<BindError> for LabError {
    fn from(error: BindError) -> Self {
        Self::Bind(error)
    }
}

enum Spec {
    Stream(Slug, Arc<StreamResponder>),
    Datagram(Slug, Arc<DatagramResponder>),
}

impl Spec {
    const fn name(&self) -> &Slug {
        match self {
            Self::Stream(name, _) | Self::Datagram(name, _) => name,
        }
    }
}

enum Bound {
    Stream(Slug, TcpListener, Arc<StreamResponder>),
    Datagram(Slug, UdpSocket, Arc<DatagramResponder>),
}

/// Collects endpoints, then starts them together.
///
/// Nothing binds until [`LabBuilder::start`], so a lab either has every
/// endpoint it was asked for or none of them. A half-started lab is a client
/// pointed at an address that answers for one surface and refuses on another,
/// which reads as a client defect.
pub struct LabBuilder {
    host: IpAddr,
    deadline: Duration,
    poll: Duration,
    max_connections: usize,
    specs: Vec<Spec>,
}

impl Default for LabBuilder {
    fn default() -> Self {
        Self {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            deadline: DEFAULT_DEADLINE,
            poll: DEFAULT_POLL,
            max_connections: DEFAULT_MAX_CONNECTIONS,
            specs: Vec::new(),
        }
    }
}

impl LabBuilder {
    /// The address every endpoint binds.
    ///
    /// Anything but loopback is refused by [`LabBuilder::start`] rather than
    /// here, so one guard answers for every socket instead of one per entry
    /// point. `crates/bit-ids-lab/src/bind.rs` is that guard.
    #[must_use]
    pub const fn host(mut self, host: IpAddr) -> Self {
        self.host = host;
        self
    }

    /// How long the lab serves before stopping itself.
    #[must_use]
    pub const fn deadline(mut self, deadline: Duration) -> Self {
        self.deadline = deadline;
        self
    }

    /// How long a worker waits on a quiet socket before re-checking.
    #[must_use]
    pub const fn poll(mut self, poll: Duration) -> Self {
        self.poll = poll;
        self
    }

    /// How many connections one stream endpoint serves at once.
    ///
    /// Zero is refused by [`LabBuilder::start`] when the lab has a stream
    /// endpoint, because a zero cap accepts every connection and closes it
    /// immediately, which a client reads as a server that is up and broken.
    #[must_use]
    pub const fn max_connections(mut self, max_connections: usize) -> Self {
        self.max_connections = max_connections;
        self
    }

    /// Adds a TCP endpoint.
    ///
    /// # Errors
    ///
    /// Returns [`LabError::Name`] when `name` is not a canonical identifier.
    pub fn stream<R>(mut self, name: &str, responder: R) -> Result<Self, LabError>
    where
        R: Fn(&[u8]) -> crate::StreamReply + Send + Sync + 'static,
    {
        let name = Slug::parse(name).map_err(LabError::Name)?;
        self.specs.push(Spec::Stream(name, Arc::new(responder)));
        Ok(self)
    }

    /// Adds a UDP endpoint.
    ///
    /// # Errors
    ///
    /// Returns [`LabError::Name`] when `name` is not a canonical identifier.
    pub fn datagram<R>(mut self, name: &str, responder: R) -> Result<Self, LabError>
    where
        R: Fn(&[u8]) -> Option<Vec<u8>> + Send + Sync + 'static,
    {
        let name = Slug::parse(name).map_err(LabError::Name)?;
        self.specs.push(Spec::Datagram(name, Arc::new(responder)));
        Ok(self)
    }

    /// Binds every endpoint and starts serving.
    ///
    /// # Errors
    ///
    /// Returns [`LabError::NoEndpoints`] for an empty lab,
    /// [`LabError::DuplicateEndpoint`] when two endpoints share a name,
    /// [`LabError::Bind`] when a socket is refused, and [`LabError::Thread`]
    /// when the host will not start a worker. On any of them nothing is left
    /// running.
    pub fn start(self) -> Result<Lab, LabError> {
        if self.specs.is_empty() {
            return Err(LabError::NoEndpoints);
        }
        if self.max_connections == 0
            && self
                .specs
                .iter()
                .any(|spec| matches!(spec, Spec::Stream(_, _)))
        {
            return Err(LabError::NoConnectionsAllowed);
        }
        for (index, spec) in self.specs.iter().enumerate() {
            if self.specs[..index]
                .iter()
                .any(|earlier| earlier.name() == spec.name())
            {
                return Err(LabError::DuplicateEndpoint(spec.name().clone()));
            }
        }

        // Every socket is bound before any thread starts, so a refused bind
        // leaves nothing to unwind.
        let mut bound = Vec::with_capacity(self.specs.len());
        let mut endpoints = Vec::with_capacity(self.specs.len());
        for spec in self.specs {
            match spec {
                Spec::Stream(name, responder) => {
                    let listener = bind::stream(self.host)?;
                    let address = listener
                        .local_addr()
                        .map_err(|error| LabError::Bind(BindError::Io(error)))?;
                    endpoints.push(Endpoint {
                        name: name.clone(),
                        address,
                        transport: Transport::Stream,
                    });
                    bound.push(Bound::Stream(name, listener, responder));
                }
                Spec::Datagram(name, responder) => {
                    let socket = bind::datagram(self.host)?;
                    let address = socket
                        .local_addr()
                        .map_err(|error| LabError::Bind(BindError::Io(error)))?;
                    endpoints.push(Endpoint {
                        name: name.clone(),
                        address,
                        transport: Transport::Datagram,
                    });
                    bound.push(Bound::Datagram(name, socket, responder));
                }
            }
        }

        // The clock starts here, so an offset in the journal is measured from
        // the moment the lab began serving rather than from the first bind.
        let shared = Arc::new(Shared::new(self.deadline, self.poll, self.max_connections));
        let mut workers: Vec<JoinHandle<()>> = Vec::with_capacity(bound.len());
        for one in bound {
            let worker_shared = Arc::clone(&shared);
            let spawned = match one {
                Bound::Stream(name, listener, responder) => std::thread::Builder::new()
                    .name(format!("bit-ids-lab-accept-{name}"))
                    .spawn(move || serve_stream(&worker_shared, &name, listener, &responder)),
                Bound::Datagram(name, socket, responder) => std::thread::Builder::new()
                    .name(format!("bit-ids-lab-datagram-{name}"))
                    .spawn(move || serve_datagram(&worker_shared, &name, socket, &responder)),
            };
            match spawned {
                Ok(worker) => workers.push(worker),
                Err(error) => {
                    // Anything already serving is stopped before the error is
                    // returned, so a failed start leaves no bound port behind.
                    shared.request_stop();
                    for worker in workers {
                        let _ = worker.join();
                    }
                    return Err(LabError::Thread(error));
                }
            }
        }

        Ok(Lab {
            shared,
            endpoints,
            workers,
        })
    }
}

/// A running lab.
///
/// Dropping one stops every endpoint and releases every port. A test or a
/// capture that returns early therefore cannot leave a listener behind for the
/// next run to collide with.
pub struct Lab {
    shared: Arc<Shared>,
    endpoints: Vec<Endpoint>,
    workers: Vec<JoinHandle<()>>,
}

impl Lab {
    /// A builder with the defaults applied.
    #[must_use]
    pub fn builder() -> LabBuilder {
        LabBuilder::default()
    }

    /// Every endpoint, in the order they were added.
    #[must_use]
    pub fn endpoints(&self) -> &[Endpoint] {
        &self.endpoints
    }

    /// One endpoint by name.
    #[must_use]
    pub fn endpoint(&self, name: &str) -> Option<&Endpoint> {
        self.endpoints
            .iter()
            .find(|endpoint| endpoint.name().as_str() == name)
    }

    /// Whether the lab stopped because its deadline passed.
    #[must_use]
    pub fn deadline_expired(&self) -> bool {
        self.shared.expired()
    }

    /// A copy of what has been recorded so far, without stopping the lab.
    #[must_use]
    pub fn journal(&self) -> Journal {
        Journal::from_segments(self.shared.snapshot())
    }

    /// Waits until every endpoint has stopped on its own.
    ///
    /// The only thing that stops a lab on its own is the deadline, so this is
    /// how a capture runs one for its whole allotted time. ⚠ It does not ask
    /// the endpoints to stop: calling it on a lab with a long deadline waits
    /// for that deadline.
    pub fn wait(&mut self) {
        self.join_workers();
    }

    /// Asks every endpoint to stop and waits for it, leaving the journal
    /// readable.
    ///
    /// Idempotent, and what [`Lab::shutdown`] and [`Drop`] both use.
    pub fn stop(&mut self) {
        self.shared.request_stop();
        self.join_workers();
    }

    /// Stops every endpoint and returns everything observed.
    #[must_use]
    pub fn shutdown(mut self) -> Journal {
        self.stop();
        Journal::from_segments(self.shared.take_journal())
    }

    fn join_workers(&mut self) {
        for worker in core::mem::take(&mut self.workers) {
            // A worker that panicked has already poisoned nothing that matters:
            // the journal lock is recovered rather than propagated. Joining is
            // still what releases the port.
            let _ = worker.join();
        }
    }
}

impl Drop for Lab {
    fn drop(&mut self) {
        self.stop();
    }
}
