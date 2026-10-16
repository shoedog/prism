//! One attempt to reach a running daemon (spec §4.1 step 5, §4.4): connect to its socket, check
//! that the peer is this user, send `HELLO`, and read the one reply line before the deadline.
//!
//! Every way an attempt can end is sorted into one of three: `READY` for this key (the daemon is
//! reached), transient (nothing usable answered and arbitration may start over: spec §4.1 step
//! 5's `RETIRING`, `ECONNREFUSED`, `ENOENT`, and EOF, reset or broken pipe before `READY`), or a
//! refusal the shim fails closed on (spec §10: `BUSY`, `MISMATCH`; and a peer of another user).
//! No MCP byte has been forwarded when an attempt ends, so starting over is always safe.
use super::socket::connect_within;
use crate::mcp::shared::identity::IdentityKey;
use crate::mcp::shared::preamble::{Ready, Reply};
use crate::mcp::shared::preamble_reader::{PreambleLineReader, PreambleReadError};
use std::fmt;
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// How a peer's uid is read. Production: `daemon::peer::peer_uid`. A test plays another user.
pub(super) type PeerUid = fn(&UnixStream) -> io::Result<u32>;

/// Points in the start protocol a test can act at (spec §9 tests 8, 13 and 15 pause a shim at
/// them). Production installs no hook.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Point {
    /// Connected and the peer checked; `HELLO` not sent yet.
    AfterConnect,
    /// A whole reply line was read; it has not been accepted yet.
    ReplyRead,
    /// The start lock is held by another shim; this one waits for it.
    StartLockBusy,
    /// The start lock is held; the attempt under it has not run yet.
    StartLockTaken,
    /// The owner lock was found held by a live daemon.
    OwnerHeld,
    /// The owner lock was taken, the stale files removed and the lock released; nothing spawned.
    BeforeSpawn,
    /// A daemon was spawned; it may not own the key yet.
    AfterSpawn,
}

pub(super) type Hook = Arc<dyn Fn(Point) + Send + Sync>;

/// What one attempt connects to and says.
pub(super) struct Target<'a> {
    pub(super) socket: &'a Path,
    pub(super) key: IdentityKey,
    /// The `HELLO` line, newline included.
    pub(super) hello: &'a str,
    /// The shim's one deadline (spec §4.1 step 4).
    pub(super) deadline: Instant,
    pub(super) peer_uid: PeerUid,
    /// This process's effective uid.
    pub(super) uid: u32,
    pub(super) hook: Option<&'a Hook>,
}

/// How an attempt ended.
pub(super) enum Reached {
    /// `READY` for this key. The stream is blocking with no timeouts, ready for the pump.
    Ready(UnixStream, Ready),
    Transient(Transient),
    Refused(Refusal),
}

/// Nothing usable answered; arbitration may start over within the deadline.
#[derive(Debug)]
pub(super) enum Transient {
    /// Nothing accepted the connection: no socket, a stale one, or a full backlog.
    Connect(io::Error),
    /// The daemon is draining (spec §3.1).
    Retiring,
    /// The connection ended, by end of stream or a reset, before a whole reply line.
    Closed,
    /// No whole reply line before the deadline: a daemon that accepts and does not answer.
    NoReply,
}

impl fmt::Display for Transient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connect(error) => write!(f, "connect failed: {error}"),
            Self::Retiring => write!(f, "the daemon answered RETIRING"),
            Self::Closed => write!(f, "the connection closed before a reply"),
            Self::NoReply => write!(f, "the daemon accepted but sent no reply"),
        }
    }
}

/// An answer the shim fails closed on (spec §10), or a peer it must not talk to.
#[derive(Debug)]
pub(super) enum Refusal {
    /// The daemon is at its connection bound.
    Busy { max_clients: u32 },
    /// The daemon serves another identity.
    Mismatch { expected: IdentityKey },
    /// `READY` for an identity other than the one asked for: no MCP byte may go to it.
    ReadyForAnother { identity: IdentityKey },
    /// The socket's peer is another user. The isolation guarantee between users is this check
    /// and the daemon's (lane README, notes for S5a); the runtime directory checks are hardening.
    ForeignPeer { uid: u32 },
    /// The peer's uid could not be read.
    PeerUnknown(io::Error),
    /// A reply line this build cannot read.
    BadReply(String),
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Busy { max_clients } => write!(
                f,
                "the daemon answered BUSY: it already has {max_clients} clients (--max-clients)"
            ),
            Self::Mismatch { expected } => write!(
                f,
                "the daemon answered MISMATCH: it serves identity {expected}, another build or \
                 configuration"
            ),
            Self::ReadyForAnother { identity } => {
                write!(
                    f,
                    "the daemon answered READY for another identity, {identity}"
                )
            }
            Self::ForeignPeer { uid } => write!(
                f,
                "the socket's peer is uid {uid}, not this user; nothing was sent to it"
            ),
            Self::PeerUnknown(error) => write!(f, "the socket's peer uid is unknown: {error}"),
            Self::BadReply(error) => write!(f, "the daemon's reply is unreadable: {error}"),
        }
    }
}

/// The time left before `deadline`, or `None` once it has passed. Never zero, which a socket
/// timeout does not accept.
fn left(deadline: Instant) -> Option<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|left| !left.is_zero())
}

/// Make one attempt.
pub(super) fn reach(target: &Target<'_>) -> Reached {
    let mut stream = match connect_within(target.socket, target.deadline) {
        Ok(stream) => stream,
        Err(error) => return Reached::Transient(Transient::Connect(error)),
    };
    match (target.peer_uid)(&stream) {
        Ok(uid) if uid == target.uid => {}
        Ok(uid) => return Reached::Refused(Refusal::ForeignPeer { uid }),
        Err(error) => return Reached::Refused(Refusal::PeerUnknown(error)),
    }
    if let Some(hook) = target.hook {
        hook(Point::AfterConnect);
    }
    let Some(time) = left(target.deadline) else {
        return Reached::Transient(Transient::NoReply);
    };
    // A daemon that answers `BUSY` or `RETIRING`, or drops the connection, may close before the
    // line is written (`EPIPE`); its reply is read anyway (S4's interface for S5a).
    let _ = stream.set_write_timeout(Some(time));
    let _ = stream.write_all(target.hello.as_bytes());
    let line = match read_reply(&mut stream, target.deadline, target.hook) {
        Ok(line) => line,
        Err(ended) => return ended,
    };
    match Reply::decode(&line) {
        Ok(Reply::Ready(ready)) if ready.identity == target.key => {
            // The preamble's timeouts must not outlive it: an MCP session may be quiet for hours.
            let cleared = stream
                .set_read_timeout(None)
                .and_then(|()| stream.set_write_timeout(None));
            match cleared {
                Ok(()) => Reached::Ready(stream, ready),
                Err(_) => Reached::Transient(Transient::Closed),
            }
        }
        Ok(Reply::Ready(ready)) => Reached::Refused(Refusal::ReadyForAnother {
            identity: ready.identity,
        }),
        Ok(Reply::Retiring) => Reached::Transient(Transient::Retiring),
        Ok(Reply::Busy { max_clients }) => Reached::Refused(Refusal::Busy { max_clients }),
        Ok(Reply::Mismatch { expected }) => Reached::Refused(Refusal::Mismatch { expected }),
        Err(error) => Reached::Refused(Refusal::BadReply(error.to_string())),
    }
}

/// A stream whose every read is bounded by the time left to one deadline and refused once it
/// has passed. `PreambleLineReader` reads one byte at a time through it, so the deadline bounds
/// the whole reply line, not each silence (review round 1, sol 1: a reply dripped one byte at a
/// time never let a per-call timeout expire).
struct Deadlined<'s> {
    stream: &'s mut UnixStream,
    deadline: Instant,
}

impl Read for Deadlined<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let Some(time) = left(self.deadline) else {
            return Err(io::Error::from(io::ErrorKind::TimedOut));
        };
        // On macOS this fails with `EINVAL` once the peer has closed; a read then returns at
        // once, so the previous bound stands.
        let _ = self.stream.set_read_timeout(Some(time));
        self.stream.read(buf)
    }
}

/// Read the reply line before the deadline: every read is bounded by the time left, and a line
/// whose last byte arrives at the deadline is not accepted.
fn read_reply(
    stream: &mut UnixStream,
    deadline: Instant,
    hook: Option<&Hook>,
) -> Result<Vec<u8>, Reached> {
    let mut reader = PreambleLineReader::new();
    let mut source = Deadlined { stream, deadline };
    loop {
        if left(deadline).is_none() {
            return Err(Reached::Transient(Transient::NoReply));
        }
        match reader.read_line(&mut source) {
            Ok(line) => {
                if let Some(hook) = hook {
                    hook(Point::ReplyRead);
                }
                if left(deadline).is_none() {
                    return Err(Reached::Transient(Transient::NoReply));
                }
                return Ok(line);
            }
            Err(PreambleReadError::TooLong) => {
                return Err(Reached::Refused(Refusal::BadReply(
                    PreambleReadError::TooLong.to_string(),
                )))
            }
            Err(PreambleReadError::Io(error))
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) => {}
            // End of stream, or a reset: on Linux a daemon that closes a connection with bytes
            // of this shim's unread is seen as a reset (S4's interface for S5a).
            Err(PreambleReadError::Eof { .. } | PreambleReadError::Io(_)) => {
                return Err(Reached::Transient(Transient::Closed))
            }
        }
    }
}

#[cfg(test)]
#[path = "connect_tests.rs"]
mod tests;
