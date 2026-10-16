//! After `READY`, the shim is a byte pipe (spec §4.1 steps 8 and 9): its client's standard input
//! to the daemon's socket on one thread, the socket to standard output on the other, byte for
//! byte. When the client closes its input the socket is half-closed and the daemon's remaining
//! responses are still delivered; a daemon that closes the connection while the client is still
//! there ends the session, and the shim never reconnects: once a byte has been forwarded, a lost
//! daemon is a lost session.
use std::fmt;
use std::io::{self, Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const BUFFER_BYTES: usize = 64 * 1024;

/// How a session ended.
#[derive(Debug)]
pub(super) enum PumpEnd {
    /// The client closed its input and the daemon then finished: exit 0.
    ClientClosed,
    /// The daemon closed the connection while the client was still there.
    DaemonClosed,
    /// Reading from the daemon failed (a reset, on Linux, when it died).
    DaemonFailed(io::Error),
    /// Writing to the client failed: it is gone.
    ClientGone(io::Error),
}

impl fmt::Display for PumpEnd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ClientClosed => write!(f, "the client closed the session"),
            Self::DaemonClosed => write!(f, "the shared daemon closed the connection"),
            Self::DaemonFailed(error) => {
                write!(f, "the shared daemon's connection failed: {error}")
            }
            Self::ClientGone(error) => write!(f, "writing to the client failed: {error}"),
        }
    }
}

/// Copy `input` to `daemon` and `daemon` to `output` until the session ends.
pub(super) fn pump(
    mut input: impl Read + Send + 'static,
    output: &mut impl Write,
    daemon: UnixStream,
) -> PumpEnd {
    let input_closed = Arc::new(AtomicBool::new(false));
    match daemon.try_clone() {
        Ok(mut to_daemon) => {
            let input_closed = Arc::clone(&input_closed);
            std::thread::spawn(move || {
                let mut buffer = vec![0u8; BUFFER_BYTES];
                loop {
                    match input.read(&mut buffer) {
                        Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            if to_daemon.write_all(&buffer[..n]).is_err() {
                                // The daemon is gone; the other direction sees it too.
                                return;
                            }
                        }
                    }
                }
                // Recorded before the half-close: the daemon can end the connection in answer to
                // it only after this is visible.
                input_closed.store(true, Ordering::SeqCst);
                let _ = to_daemon.shutdown(Shutdown::Write);
            });
        }
        Err(error) => return PumpEnd::DaemonFailed(error),
    }
    let mut from_daemon = daemon;
    let mut buffer = vec![0u8; BUFFER_BYTES];
    loop {
        match from_daemon.read(&mut buffer) {
            Ok(0) if input_closed.load(Ordering::SeqCst) => return PumpEnd::ClientClosed,
            Ok(0) => return PumpEnd::DaemonClosed,
            Ok(n) => {
                if let Err(error) = output.write_all(&buffer[..n]).and_then(|()| output.flush()) {
                    return PumpEnd::ClientGone(error);
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return PumpEnd::DaemonFailed(error),
        }
    }
}

#[cfg(test)]
#[path = "pump_tests.rs"]
mod tests;
