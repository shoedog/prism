//! The accept loop and the bounded handshake stage of a shared daemon (spec §4.2 "Per
//! connection", "Admission and retirement", §4.4), and how it drains (§3.1).
//!
//! One thread runs the loop. It accepts connections, drops any whose peer is another user,
//! reads each pending connection's `HELLO` from a non-blocking socket against that
//! connection's own deadline, and answers it: `READY` (and a worker thread from then on),
//! `MISMATCH`, `BUSY`, `RETIRING`, or nothing for a line it refuses. It never reads or writes an
//! admitted connection again. It also takes the retirement decision and notices a lost root or
//! replaced files, and then drains: the socket is unlinked under the control lock, every
//! connection still pending or queued is answered `RETIRING`, the pid file is removed and the
//! owner lock released.
//!
//! The loop is outside both of the daemon's domains, like a connection's framing: nothing in it
//! waits on the provider slot. A nonblocking idle tick may detach it, with destruction on a
//! worker. What the loop does under the control lock it does through the
//! guarded functions of `admission.rs`. A poisoned control lock ends the daemon there.
//!
//! A connection whose line is an operator's `STATUS` or `SHUTDOWN` (spec §4.7) is answered with
//! the daemon's state and closed; it is never admitted, and it leaves the idle time as it found
//! it, also when the read of its line completes across its deadline. A connection that ends or times out
//! before its line is whole, or whose line is neither a control line nor a `HELLO`, cannot be told
//! from a client's, and restarts the idle time when it goes.
use super::admission::{Admit, Control, Entry};
use super::connection::serve_admitted;
use super::control::StopReason;
use super::coordinator::DaemonState;
use super::files::DaemonFiles;
use super::log;
use super::peer::peer_uid;
use crate::mcp::registry::ToolRegistry;
use crate::mcp::settings::ClientSettings;
use crate::mcp::shared::control::{
    display_root, is_control_line, ControlRequest, DaemonStatus, Instance,
};
use crate::mcp::shared::identity::{FsIdentity, IdentityKey};
use crate::mcp::shared::preamble::{Hello, PreambleError, Ready, Reply};
use crate::mcp::shared::preamble_reader::{PreambleLineReader, PreambleReadError};
use crate::mcp::shared::runtime_dir::current_uid;
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// How long a connection may take to send its `HELLO` line, from its acceptance. The shim's
/// whole start deadline defaults to the same 10 s (spec §4.1 step 4); a shim whose handshake is
/// dropped sees EOF and starts over within its own deadline (§4.1 step 5).
pub(super) const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
/// How often the loop checks the repository root's identity and its own files when nothing
/// else wakes it (spec §4.3: "on the TTL tick").
pub(super) const IDENTITY_CHECK_INTERVAL: Duration = Duration::from_secs(5);
/// How many connections one turn of the loop accepts before it reads the pending ones and checks
/// their deadlines, and before the next turn's stop and root checks: a backlog that keeps
/// refilling must not starve them (review round 1, sol F3). A burst of `--max-clients` (64)
/// shims takes four turns, each of which starts at once while the listener stays readable.
pub(super) const ACCEPTS_PER_TURN: usize = 16;
/// Pause only listener admission when the process or system descriptor table is full.
const ACCEPT_BACKOFF: Duration = Duration::from_millis(100);
/// Name of each admitted connection's thread. Linux keeps 15 bytes of it.
const WORKER_THREAD_NAME: &str = "prism-mcp-conn";

/// The bounds a daemon serves within (spec §8 step 4: `--max-clients`, `--idle-ttl`).
#[derive(Clone, Copy, Debug)]
pub(super) struct ServeOptions {
    /// Pending and admitted connections together (spec §4.2).
    pub(super) max_clients: u32,
    pub(super) idle_ttl: Duration,
    pub(super) handshake_timeout: Duration,
}

/// What this daemon is, as `READY` reports it and its checks need it.
#[derive(Clone, Debug)]
pub(super) struct Served {
    pub(super) key: IdentityKey,
    /// The canonical repository root and its device and inode when the daemon started.
    pub(super) repo_root: PathBuf,
    pub(super) root_fs: FsIdentity,
    pub(super) pid: u32,
    /// Seconds since the Unix epoch.
    pub(super) start_time: u64,
    /// `PRISM_CACHE_BUILD_IDENTITY`, as the pid file's third line has it.
    pub(super) build_identity: String,
}

impl Served {
    /// This daemon instance, as a shutdown request must name it (spec §11 item 3).
    fn instance(&self) -> Instance {
        Instance {
            identity: self.key,
            pid: self.pid,
            start_time: self.start_time,
        }
    }
}

/// Wakes the loop from another thread: a connection ended, a test moved the clock, or the
/// daemon is asked to stop.
#[derive(Clone)]
pub(super) struct Waker {
    stream: Arc<UnixStream>,
    stop: Arc<AtomicBool>,
}

impl Waker {
    pub(super) fn wake(&self) {
        // A full socket means a wake is already pending; nothing else can fail that matters.
        let _ = (&*self.stream).write(&[1]);
    }

    /// Drain the daemon whatever its connections, as a signal does (spec §3.1: SIGTERM and
    /// SIGINT go through Draining). Tests only; production reads its signal flag.
    #[cfg(test)]
    pub(super) fn request_stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        self.wake();
    }

    /// A descriptor that wakes the loop when one byte is written to it, for a signal handler.
    /// It is a duplicate that is never closed, so it stays valid for the life of the process.
    pub(super) fn leak_raw_fd(&self) -> io::Result<i32> {
        use std::os::fd::IntoRawFd;
        Ok(self.stream.try_clone()?.into_raw_fd())
    }
}

/// How a peer's uid is read. Production: `peer::peer_uid`. A test can play another user.
pub(super) type PeerUid = Box<dyn Fn(&UnixStream) -> io::Result<u32> + Send>;

/// A test seam: called at the top of each turn of the loop, with no lock held.
#[cfg(test)]
pub(super) type TurnHook = Arc<dyn Fn() + Send + Sync>;

#[cfg(test)]
type AcceptHook = Box<dyn FnMut() -> Option<io::Result<UnixStream>> + Send>;
#[cfg(test)]
type AcceptLog = Box<dyn Fn(&str) + Send>;

/// A connection in the handshake stage.
struct Pending {
    stream: UnixStream,
    reader: PreambleLineReader,
    deadline: Instant,
}

pub(super) struct Server {
    state: Arc<DaemonState>,
    registry: Arc<ToolRegistry>,
    files: DaemonFiles,
    listener: UnixListener,
    served: Served,
    options: ServeOptions,
    wakes: UnixStream,
    waker: Waker,
    peer_uid: PeerUid,
    pending: Vec<Pending>,
    next_identity_check: Instant,
    /// Kept after expiry until accept makes progress, so repeated exhaustion logs once.
    accept_retry_at: Option<Instant>,
    /// Whether a termination signal has arrived (`run.rs` installs the handlers).
    signalled: fn() -> bool,
    #[cfg(test)]
    turn: Option<TurnHook>,
    /// A test seam: called after a pending connection's whole line has been read, before its
    /// deadline is checked again.
    #[cfg(test)]
    line_read: Option<TurnHook>,
    #[cfg(test)]
    accept_hook: Option<AcceptHook>,
    #[cfg(test)]
    accept_log: Option<AcceptLog>,
}

impl Server {
    pub(super) fn new(
        state: Arc<DaemonState>,
        registry: Arc<ToolRegistry>,
        files: DaemonFiles,
        listener: UnixListener,
        served: Served,
        options: ServeOptions,
    ) -> io::Result<Self> {
        listener.set_nonblocking(true)?;
        let (wakes, wake_end) = UnixStream::pair()?;
        wakes.set_nonblocking(true)?;
        wake_end.set_nonblocking(true)?;
        let next_identity_check = state.now();
        let waker = Waker {
            stream: Arc::new(wake_end),
            stop: Arc::new(AtomicBool::new(false)),
        };
        let idle_waker = waker.clone();
        state.set_idle_waker(Arc::new(move || idle_waker.wake()));
        Ok(Self {
            state,
            registry,
            files,
            listener,
            served,
            options,
            wakes,
            waker,
            peer_uid: Box::new(peer_uid),
            pending: Vec::new(),
            next_identity_check,
            accept_retry_at: None,
            signalled: || false,
            #[cfg(test)]
            turn: None,
            #[cfg(test)]
            line_read: None,
            #[cfg(test)]
            accept_hook: None,
            #[cfg(test)]
            accept_log: None,
        })
    }

    pub(super) fn waker(&self) -> Waker {
        self.waker.clone()
    }

    /// Where the loop learns that a termination signal arrived.
    pub(super) fn set_signalled(&mut self, signalled: fn() -> bool) {
        self.signalled = signalled;
    }

    #[cfg(test)]
    pub(super) fn set_peer_uid(&mut self, peer_uid: PeerUid) {
        self.peer_uid = peer_uid;
    }

    #[cfg(test)]
    pub(super) fn set_turn_hook(&mut self, turn: TurnHook) {
        self.turn = Some(turn);
    }

    #[cfg(test)]
    pub(super) fn set_line_read_hook(&mut self, line_read: TurnHook) {
        self.line_read = Some(line_read);
    }

    /// Serve until the daemon drains, then drain and say why. An error means a poisoned
    /// control lock (which has already ended a production daemon) or a failed `poll`.
    pub(super) fn run(mut self) -> anyhow::Result<StopReason> {
        loop {
            #[cfg(test)]
            if let Some(turn) = &self.turn {
                turn();
            }
            if let Some(reason) = self.check()? {
                return self.drain(reason);
            }
            self.state.unload_tick()?;
            let timeout = self.poll_timeout()?;
            let accept = self.wait(timeout)?;
            self.empty_wakes();
            if accept {
                self.accept_all()?;
            }
            self.advance_handshakes()?;
        }
    }

    /// Everything that can start Draining, in one place: a signal or a stop request, the
    /// periodic root and files check, and the idle retirement decision.
    fn check(&mut self) -> anyhow::Result<Option<StopReason>> {
        let files = &self.files;
        if self.waker.stop.load(Ordering::SeqCst) || (self.signalled)() {
            return Ok(Some(
                self.state
                    .drain(StopReason::Signal, || files.unlink_socket())?,
            ));
        }
        let now = self.state.now();
        if now >= self.next_identity_check {
            self.next_identity_check = now.checked_add(IDENTITY_CHECK_INTERVAL).unwrap_or(now);
            if let Some(reason) = self.lost_identity() {
                return Ok(Some(self.state.drain(reason, || files.unlink_socket())?));
            }
        }
        Ok(self
            .state
            .retire_if_idle(self.options.idle_ttl, || files.unlink_socket())?)
    }

    /// Spec §4.3: the root's device and inode are checked on the TTL tick and on every
    /// admission; loss or change drains the daemon. Its own files are checked with them.
    fn lost_identity(&self) -> Option<StopReason> {
        let root = FsIdentity::of(&self.served.repo_root);
        if root.ok() != Some(self.served.root_fs) {
            return Some(StopReason::RootChanged);
        }
        if !self.files.still_ours() {
            return Some(StopReason::FilesReplaced);
        }
        None
    }

    /// How long the next `poll` may sleep: until the first handshake deadline, the idle
    /// deadline, listener retry or the next identity check, whichever is first.
    fn poll_timeout(&self) -> anyhow::Result<Duration> {
        let now = self.state.now();
        let idle = self
            .state
            .idle_deadline(self.options.idle_ttl)?
            .filter(|deadline| *deadline > now);
        let first = self
            .pending
            .iter()
            .map(|pending| pending.deadline)
            .chain(idle)
            .chain(self.state.unload_deadline()?)
            .chain(Some(self.next_identity_check))
            .chain(self.accept_retry_at.filter(|deadline| *deadline > now))
            .min()
            .unwrap_or(now);
        Ok(first
            .saturating_duration_since(now)
            .min(IDENTITY_CHECK_INTERVAL))
    }

    /// Sleep in `poll` until the listener or a pending connection is readable, a wake byte
    /// arrives, or `timeout` passes. Returns whether the listener is readable.
    fn wait(&self, timeout: Duration) -> anyhow::Result<bool> {
        let mut fds = self.poll_fds();
        // Round up, so that a deadline a fraction of a millisecond away is not polled for 0 ms
        // again and again.
        let millis = timeout.as_micros().div_ceil(1000);
        let millis = libc::c_int::try_from(millis).unwrap_or(libc::c_int::MAX);
        let count = libc::nfds_t::try_from(fds.len()).unwrap_or(libc::nfds_t::MAX);
        // SAFETY: `fds` is a valid array of `count` initialized `pollfd`s, alive and not
        // otherwise borrowed for the call.
        let ready = unsafe { libc::poll(fds.as_mut_ptr(), count, millis) };
        if ready < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                return Ok(false);
            }
            return Err(error.into());
        }
        Ok(fds
            .iter()
            .any(|fd| fd.fd == self.listener.as_raw_fd() && fd.revents != 0))
    }

    fn poll_fds(&self) -> Vec<libc::pollfd> {
        std::iter::once(self.listener.as_raw_fd())
            .filter(|_| !self.accept_paused())
            .chain(std::iter::once(self.wakes.as_raw_fd()))
            .chain(
                self.pending
                    .iter()
                    .map(|pending| pending.stream.as_raw_fd()),
            )
            .map(|fd| libc::pollfd {
                fd,
                events: libc::POLLIN,
                revents: 0,
            })
            .collect()
    }

    fn accept_paused(&self) -> bool {
        self.accept_retry_at
            .is_some_and(|deadline| self.state.now() < deadline)
    }

    fn empty_wakes(&self) {
        let mut buffer = [0u8; 64];
        while matches!((&self.wakes).read(&mut buffer), Ok(n) if n > 0) {}
    }

    /// Accept the connections that are waiting, at most `ACCEPTS_PER_TURN` of them; the rest
    /// wait for the next turn, which starts at once because the listener is still readable.
    fn accept_all(&mut self) -> anyhow::Result<()> {
        if self.accept_paused() {
            return Ok(());
        }
        for _ in 0..ACCEPTS_PER_TURN {
            match self.accept() {
                Ok(stream) => {
                    self.accept_retry_at = None;
                    self.enter(stream)?;
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    self.accept_retry_at = None;
                    return Ok(());
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) if matches!(error.raw_os_error(), Some(libc::EMFILE | libc::ENFILE)) => {
                    if self.accept_retry_at.is_none() {
                        self.log_accept_error(&error);
                    }
                    let now = self.state.now();
                    self.accept_retry_at = Some(now.checked_add(ACCEPT_BACKOFF).unwrap_or(now));
                    return Ok(());
                }
                // A connection that went away before it was accepted: retry next turn.
                Err(error) => {
                    self.accept_retry_at = None;
                    self.log_accept_error(&error);
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    fn accept(&mut self) -> io::Result<UnixStream> {
        #[cfg(test)]
        if let Some(result) = self.accept_hook.as_mut().and_then(|accept| accept()) {
            return result;
        }
        self.listener.accept().map(|(stream, _address)| stream)
    }

    fn log_accept_error(&self, error: &io::Error) {
        let message = format!("accepting a connection failed: {error}");
        #[cfg(test)]
        if let Some(log) = &self.accept_log {
            log(&message);
        }
        log::line(&message);
    }

    /// A connection just accepted: check its peer, then count it as pending or answer it.
    fn enter(&mut self, stream: UnixStream) -> anyhow::Result<()> {
        // Spec §4.4, §10: a peer of another user is dropped before any reply and is never
        // counted.
        match (self.peer_uid)(&stream) {
            Ok(uid) if uid == current_uid() => {}
            Ok(uid) => {
                log::line(&format!("refused a connection from uid {uid}"));
                return Ok(());
            }
            Err(error) => {
                log::line(&format!(
                    "refused a connection whose peer is unknown: {error}"
                ));
                return Ok(());
            }
        }
        if stream.set_nonblocking(true).is_err() {
            return Ok(());
        }
        match self.state.enter_handshake(self.options.max_clients)? {
            Entry::Pending => {
                let now = self.state.now();
                self.pending.push(Pending {
                    stream,
                    reader: PreambleLineReader::new(),
                    deadline: now
                        .checked_add(self.options.handshake_timeout)
                        .unwrap_or(now),
                });
            }
            Entry::Busy { max_clients } => send(&stream, &Reply::Busy { max_clients }),
            Entry::Retiring => send(&stream, &Reply::Retiring),
        }
        Ok(())
    }

    /// Read what each pending connection has sent. A whole line is answered; a connection
    /// past its deadline, closed, or sending more than a preamble line is dropped. The deadline
    /// is checked against the clock as it is when the connection is read, before the read and
    /// again before a whole line is answered, so it bounds a line that completes after it too
    /// (review round 1).
    fn advance_handshakes(&mut self) -> anyhow::Result<()> {
        for mut pending in std::mem::take(&mut self.pending) {
            if self.state.now() >= pending.deadline {
                self.state.leave_handshake()?;
                continue;
            }
            match pending.reader.read_line(&mut pending.stream) {
                Ok(line) => {
                    #[cfg(test)]
                    if let Some(line_read) = &self.line_read {
                        line_read();
                    }
                    if self.state.now() < pending.deadline {
                        self.answer(pending.stream, &line)?
                    } else if is_control_line(&line) {
                        // An operator's line whose read completed across its deadline is not
                        // answered, records nothing, and leaves the idle time as it found it
                        // (review round 1, sol 3). One still incomplete when the deadline is
                        // checked before a read is a client's (review round 2, Opus W1).
                        self.state.control_exchange(false)?;
                    } else {
                        self.state.leave_handshake()?
                    }
                }
                Err(PreambleReadError::Io(error))
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                    ) =>
                {
                    self.pending.push(pending);
                }
                Err(_) => self.state.leave_handshake()?,
            }
        }
        Ok(())
    }

    /// Answer a whole `HELLO` line, or an operator's control line.
    fn answer(&mut self, stream: UnixStream, line: &[u8]) -> anyhow::Result<()> {
        if is_control_line(line) {
            return self.answer_control(&stream, line);
        }
        let settings = match admit_hello(Hello::decode(line), &self.served.key) {
            Ok(settings) => settings,
            Err(Some(reply)) => {
                send(&stream, &reply);
                return Ok(self.state.leave_handshake()?);
            }
            Err(None) => return Ok(self.state.leave_handshake()?),
        };
        // Spec §4.3: the root is checked on every admission. A root that changed drains the
        // daemon, and this connection is told so.
        if let Some(reason) = self.lost_identity() {
            let files = &self.files;
            self.state.drain(reason, || files.unlink_socket())?;
        }
        match self.state.admit()? {
            Admit::Retiring => send(&stream, &Reply::Retiring),
            Admit::Ready(status) => {
                let ready = Reply::Ready(Ready {
                    identity: self.served.key,
                    pid: self.served.pid,
                    start_time: self.served.start_time,
                    generation: status.generation,
                    build: status.state,
                    index_idle_timeout_secs: self.state.idle_timeout.map_or(0, |n| n.as_secs()),
                    index_idle_pressure_relief: self.state.lifecycle.relief,
                });
                if (&stream).write_all(ready.encode().as_bytes()).is_err() {
                    return Ok(self.state.release()?);
                }
                self.start_worker(stream, settings)?;
            }
        }
        Ok(())
    }

    /// Answer an operator's control line (spec §4.7) and close the connection: it is never
    /// admitted, and it leaves the idle time as it found it. A shutdown is recorded only when it
    /// names this very instance (spec §11 item 3); every request is answered with the status, so
    /// the operator sees which instance answered. A line this build refuses is closed without a
    /// reply, as a refused `HELLO` is (D54).
    fn answer_control(&self, stream: &UnixStream, line: &[u8]) -> anyhow::Result<()> {
        let request = match ControlRequest::decode(line) {
            Ok(request) => request,
            Err(error) => {
                log::line(&format!("refused a control line: {error}"));
                self.state.control_exchange(false)?;
                return Ok(());
            }
        };
        let shutdown = match request {
            ControlRequest::Status => false,
            ControlRequest::Shutdown(instance) => instance == self.served.instance(),
        };
        let snapshot = match self.state.control_exchange(shutdown)? {
            Control::Retiring => {
                send(stream, &Reply::Retiring);
                return Ok(());
            }
            Control::Status(snapshot) => snapshot,
        };
        if shutdown {
            log::line(
                "an operator asked this daemon to shut down; it drains once no client is left",
            );
        }
        let status = DaemonStatus {
            instance: self.served.instance(),
            build_identity: self.served.build_identity.clone(),
            repo_root: display_root(&self.served.repo_root),
            admitted: snapshot.admitted,
            pending: snapshot.pending,
            max_clients: self.options.max_clients,
            build: snapshot.build.state,
            attempt: snapshot.build.attempt,
            generation: snapshot.build.generation,
            idle_secs: snapshot.idle_for.map(|idle| idle.as_secs()),
            idle_ttl_secs: self.options.idle_ttl.as_secs(),
            index_idle_timeout_secs: self.state.idle_timeout.map_or(0, |n| n.as_secs()),
            index_idle_pressure_relief: self.state.lifecycle.relief,
            shutdown: snapshot.shutdown,
        };
        let _ = (&*stream).write_all(status.encode().as_bytes());
        Ok(())
    }

    /// The admitted connection's own thread, from `READY` on. Its admission is released when
    /// it ends, on every path.
    fn start_worker(&self, stream: UnixStream, settings: ClientSettings) -> anyhow::Result<()> {
        let released = Released {
            state: Arc::clone(&self.state),
            waker: self.waker(),
        };
        let registry = Arc::clone(&self.registry);
        let spawned = std::thread::Builder::new()
            .name(WORKER_THREAD_NAME.to_string())
            .spawn(move || {
                if let Err(error) = serve_admitted(&released.state, &registry, stream, &settings) {
                    log::line(&format!("a connection ended with an error: {error:#}"));
                }
                drop(released);
            });
        if let Err(error) = spawned {
            // The closure, with the stream and the release, was dropped with the error.
            log::line(&format!("could not start a connection's thread: {error}"));
        }
        Ok(())
    }

    /// Drain (spec §3.1): the socket is unlinked under the control lock (already, for an idle
    /// retirement), every connection still pending or queued on the listener is answered
    /// `RETIRING`, then the pid file is removed and the owner lock released. Admitted
    /// connections are not touched here; they end with the process.
    fn drain(self, reason: StopReason) -> anyhow::Result<StopReason> {
        let Self {
            state,
            files,
            listener,
            pending,
            peer_uid,
            ..
        } = self;
        let reason = state.drain(reason, || files.unlink_socket())?;
        for pending in pending {
            send(&pending.stream, &Reply::Retiring);
            state.leave_handshake()?;
        }
        // The socket's name is gone, so whatever queued before the unlink is answered here. A
        // `connect` that resolved the name just before the unlink can still queue after this
        // loop has emptied the backlog; it sees a reset when the listener is dropped, which a
        // shim treats as transient (spec §4.1 step 5).
        loop {
            match listener.accept() {
                Ok((stream, _address)) => {
                    if peer_uid(&stream).is_ok_and(|uid| uid == current_uid()) {
                        send(&stream, &Reply::Retiring);
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(_) => break,
            }
        }
        drop(listener);
        files.release();
        Ok(reason)
    }
}

/// The one place a decoded `HELLO` becomes an admission (lane README, notes for S4: admission
/// must not read the settings of a `HELLO` without comparing its identity). `Ok` is the
/// client's settings; `Err(Some(reply))` is the reply it is owed; `Err(None)` means the line is
/// refused without a reply (SI-3, D54).
fn admit_hello(
    decoded: Result<Hello, PreambleError>,
    served: &IdentityKey,
) -> Result<ClientSettings, Option<Reply>> {
    match decoded {
        Ok(hello) => match hello.mismatch_against(served) {
            Some(reply) => Err(Some(reply)),
            None => Ok(hello.settings),
        },
        // Another protocol version is another identity (the version is part of the key), so
        // `MISMATCH` is the truth (S2's SI-3 note).
        Err(PreambleError::UnsupportedProtocol { .. }) => {
            Err(Some(Reply::Mismatch { expected: *served }))
        }
        Err(error) => {
            log::line(&format!("refused a preamble line: {error}"));
            Err(None)
        }
    }
}

/// Write one reply on a connection that is about to be closed. A failure is ignored: the peer
/// has gone, and it would have been told nothing else.
fn send(stream: &UnixStream, reply: &Reply) {
    let _ = (&*stream).write_all(reply.encode().as_bytes());
}

/// Releases an admission when the connection's thread ends.
struct Released {
    state: Arc<DaemonState>,
    waker: Waker,
}

impl Drop for Released {
    fn drop(&mut self) {
        // A poisoned control lock has already ended the daemon (`on_fatal`).
        let _ = self.state.release();
        self.waker.wake();
    }
}

#[cfg(test)]
#[path = "server_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "server_drain_tests.rs"]
mod drain_tests;

#[cfg(test)]
#[path = "server_control_tests.rs"]
mod control_tests;

#[cfg(test)]
#[path = "server_accept_tests.rs"]
mod accept_tests;
