//! What ends a shared daemon (spec §3.1 "Fatal", §4.2 "Panic policy", §10).
//!
//! A panic in provider dispatch (a query or a refresh, run under the provider slot's lock), on
//! the build thread, or in the control plane (under the control lock) ends the daemon, and so
//! does a lock found poisoned. The process aborts: every attached client sees its connection
//! end, as a client of a crashed in-process server does, and the next client starts a new
//! daemon. Nothing is served from state a panic may have left half-changed, and no lock is read
//! through past its poisoning.
//!
//! A panic anywhere else on a connection's thread (framing, the readiness answers, writing a
//! response) ends that connection only; that boundary is in `connection.rs`.
use std::fmt;
use std::io::Write;
use std::sync::Arc;

/// Where a panic that ends the daemon was raised.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Domain {
    /// A query or a refresh, under the provider slot's read or write lock.
    Dispatch,
    /// The build thread: the builder, then publication.
    Build,
    /// The readiness coordinator, under the control lock.
    Control,
    /// Destruction of the whole detached provider on its disposal worker.
    Disposal,
}

impl fmt::Display for Domain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Dispatch => "provider dispatch",
            Self::Build => "the index build",
            Self::Control => "the control plane",
            Self::Disposal => "provider disposal",
        })
    }
}

/// Why the daemon ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Fatal {
    /// A thread panicked in this domain.
    Panicked(Domain),
    /// Code that needed this lock found it poisoned: a thread panicked while holding it.
    Poisoned { lock: &'static str },
}

impl fmt::Display for Fatal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Panicked(domain) => write!(f, "prism-mcp daemon: a panic in {domain}"),
            Self::Poisoned { lock } => {
                write!(f, "prism-mcp daemon: the {lock} lock is poisoned")
            }
        }?;
        f.write_str("; the daemon ends so that no client is served from what the panic left")
    }
}

/// What the daemon does when it cannot go on. Production: [`abort_daemon`]. A test replaces it
/// with a record and then sees the code after the call run, which a running daemon never does.
pub(super) type OnFatal = Arc<dyn Fn(Fatal) + Send + Sync>;

/// One line on standard error (the daemon's log; the panic message, if there was one, is the
/// line before it), then abort.
pub(super) fn abort_daemon(fatal: Fatal) {
    // A failed write must not panic here: this runs while a thread is unwinding.
    let _ = writeln!(std::io::stderr(), "{fatal}");
    std::process::abort();
}

/// Calls `on_fatal` with `Fatal::Panicked(domain)` if the thread unwinds while this is alive.
///
/// Made right after each piece of code in a domain has taken its lock (D47), and dropped at its
/// exit. It does nothing if the thread was already unwinding when it was made (code that runs in
/// a destructor during another panic belongs to that panic, which has its own boundary).
pub(super) struct FatalOnUnwind<'a> {
    on_fatal: &'a OnFatal,
    domain: Domain,
    armed: bool,
}

impl<'a> FatalOnUnwind<'a> {
    pub(super) fn new(on_fatal: &'a OnFatal, domain: Domain) -> Self {
        Self {
            on_fatal,
            domain,
            armed: !std::thread::panicking(),
        }
    }
}

impl Drop for FatalOnUnwind<'_> {
    fn drop(&mut self) {
        if self.armed && std::thread::panicking() {
            (self.on_fatal)(Fatal::Panicked(self.domain));
        }
    }
}

#[cfg(test)]
#[path = "fatal_tests.rs"]
mod tests;
