//! One control exchange with a daemon (spec §4.7), for `daemon-status` and `daemon-shutdown`:
//! connect without waiting past the deadline (`socket.rs`), check that the peer is this user
//! before anything is sent (D78, as the shim does), send one control line, and read the one reply
//! line with every byte's read bounded by the time left (`connect::read_reply`). The daemon closes
//! the connection after its reply; the connection is never admitted.
use super::connect::{left, read_reply, PeerUid, Reached, Refusal, Transient};
use super::socket::connect_within;
use crate::mcp::shared::control::{ControlRequest, DaemonStatus};
use crate::mcp::shared::preamble::Reply;
use std::io::{self, Write};
use std::path::Path;
use std::time::Instant;

/// How a control exchange ended.
#[derive(Debug)]
pub(super) enum Exchanged {
    /// The daemon's status.
    Status(DaemonStatus),
    /// The daemon drains (spec §3.1).
    Retiring,
    /// The daemon is at its client bound; a control request counts against it until its line is
    /// read (spec §4.2).
    Busy { max_clients: u32 },
    /// Nothing accepted the connection: no socket, one that no daemon listens on, or a full
    /// backlog.
    NotListening(io::Error),
    /// The daemon accepted and sent no whole reply line: it closed (a daemon that does not know
    /// this exchange, or this version of it, closes the line it refuses), or the deadline passed.
    NoReply(Transient),
    /// A peer of another user, or whose uid could not be read (nothing was sent to it), or a
    /// reply this build cannot read.
    Refused(Refusal),
}

/// Make one control exchange with the daemon at `socket`; `uid` is this process's effective uid.
pub(super) fn exchange(
    socket: &Path,
    request: &ControlRequest,
    deadline: Instant,
    peer_uid: PeerUid,
    uid: u32,
) -> Exchanged {
    let mut stream = match connect_within(socket, deadline) {
        Ok(stream) => stream,
        Err(error) => return Exchanged::NotListening(error),
    };
    match peer_uid(&stream) {
        Ok(found) if found == uid => {}
        Ok(found) => return Exchanged::Refused(Refusal::ForeignPeer { uid: found }),
        Err(error) => return Exchanged::Refused(Refusal::PeerUnknown(error)),
    }
    let Some(time) = left(deadline) else {
        return Exchanged::NoReply(Transient::NoReply);
    };
    // A daemon at its bound answers `BUSY` and may close before the line is written (`EPIPE`);
    // its reply is read anyway, as the shim reads it (S4's interface for S5a).
    let _ = stream.set_write_timeout(Some(time));
    let _ = stream.write_all(request.encode().as_bytes());
    let line = match read_reply(&mut stream, deadline, None) {
        Ok(line) => line,
        Err(Reached::Transient(transient)) => return Exchanged::NoReply(transient),
        Err(Reached::Refused(refusal)) => return Exchanged::Refused(refusal),
        // `read_reply` reads a line; it never reaches a daemon.
        Err(Reached::Ready(..)) => return Exchanged::NoReply(Transient::Closed),
    };
    if line.starts_with(b"STATUS ") {
        return match DaemonStatus::decode(&line) {
            Ok(status) => Exchanged::Status(status),
            Err(error) => Exchanged::Refused(Refusal::BadReply(error.to_string())),
        };
    }
    match Reply::decode(&line) {
        Ok(Reply::Retiring) => Exchanged::Retiring,
        Ok(Reply::Busy { max_clients }) => Exchanged::Busy { max_clients },
        Ok(other) => Exchanged::Refused(Refusal::BadReply(format!(
            "{} does not answer a control request",
            other.encode().trim_end()
        ))),
        Err(error) => Exchanged::Refused(Refusal::BadReply(error.to_string())),
    }
}

#[cfg(test)]
#[path = "control_tests.rs"]
mod tests;
