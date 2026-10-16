//! Finding the daemon for a key, and starting it when none is alive (spec §4.1 steps 4–7, §4.6),
//! all before one deadline.
//!
//! 1. Try the socket (`connect::reach`). `READY` for this key ends here.
//! 2. On anything transient, take the start lock (`<key16>.start`, retried until the deadline).
//!    Under it, try again; another shim may have started the daemon meanwhile.
//! 3. Still nothing: look at the owner lock. Held: a daemon is alive (starting, or wedged, or
//!    draining); try again shortly, never touching its files. Free: no daemon owns the key;
//!    holding the owner lock, remove the stale socket, pid file and build marker, release it, and
//!    spawn a daemon, which takes the owner lock itself. While that daemon is still running and
//!    has not answered, the owner lock is not looked at again (looking takes it for a moment, and
//!    a daemon claiming the key in that moment would leave believing another owns it).
//! 4. A daemon this shim spawned that exits unsuccessfully before `READY` ends the start: it will
//!    not do better on a second try. One that exits successfully lost the key to another daemon
//!    (S4: "exit 0 silently"); step 3 runs again.
//!
//! The build marker bounds restarts (S3b's SI-10): a key whose last daemon died during its index
//! build is not restarted for [`BUILD_CRASH_BACKOFF`] from the moment a shim first saw the dead
//! build, which that shim records in the marker; until then the shim fails closed naming the log.
//! The minute runs from the sighting, not from the build's start, so it bounds builds of any
//! length (review round 1, sol 3), and a refusal never renews it. A panic on the build thread is
//! fatal by the spec (§3.1), so a build that panics (or is killed) every time would otherwise make
//! every new shim start, and lose, another daemon.
use super::connect::{reach, Hook, PeerUid, Point, Reached, Refusal, Target, Transient};
use super::key_files::KeyDir;
use crate::mcp::daemon::files::FilesError;
use crate::mcp::shared::identity::IdentityKey;
use crate::mcp::shared::paths::DaemonPaths;
use crate::mcp::shared::preamble::Ready;
use crate::mcp::shared::runtime_dir::{current_uid, validate_runtime_dir};
use std::fmt;
use std::fs::TryLockError;
use std::io;
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// How long a shim waits between two looks at the socket and the locks.
pub(super) const POLL: Duration = Duration::from_millis(10);
/// How long a key whose last daemon died during its build stays closed, from the moment a shim
/// first saw the dead build.
pub(super) const BUILD_CRASH_BACKOFF: Duration = Duration::from_secs(60);

/// Starts a daemon process for the key, detached from the shim (spec §4.1 step 6). Called with
/// the start lock held and the owner lock released.
pub(super) trait Starter {
    fn start(&mut self, dir: &KeyDir<'_>) -> io::Result<Box<dyn Started>>;
}

/// A daemon process this shim started.
pub(super) trait Started: Send {
    fn pid(&self) -> u32;
    /// `None` while it runs; how it ended once it has (and it is then reaped).
    fn exit(&mut self) -> Option<Exit>;
    /// Wait for it to end, so it does not stay a zombie of the shim's. Blocks; the caller runs it
    /// on a thread of its own.
    fn reap(self: Box<Self>);
}

/// How a started daemon ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Exit {
    /// Status 0: it served and drained, or it found the key owned by another daemon.
    Success,
    /// Anything else, described.
    Failure(String),
}

/// A daemon reached.
pub(super) struct Connected {
    /// Blocking, no timeouts; the next byte on it is the daemon's first MCP byte.
    pub(super) stream: UnixStream,
    pub(super) ready: Ready,
    /// The daemon this shim started on the way, if it did.
    pub(super) started: Option<Box<dyn Started>>,
}

/// What a shim knows about the owner lock when it gives up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Owner {
    /// Not looked at.
    Unknown,
    /// A live daemon held it.
    Held,
    /// No daemon held it (a daemon may have been started since).
    Free,
}

/// Why no daemon was reached. Each is a fail-closed diagnostic (spec §4.1 step 7, §10).
#[derive(Debug)]
pub(super) enum Failure {
    /// The runtime directory failed validation, or a key file could not be opened or removed.
    Files(FilesError),
    /// The daemon at the socket answered something the shim fails closed on.
    Refused(Refusal),
    /// Another shim held the start lock until the deadline.
    StartLockTimeout { last: Transient },
    /// The deadline passed. `started` is the pid of the daemon this shim spawned, if any.
    NoReady {
        last: Transient,
        owner: Owner,
        started: Option<u32>,
    },
    /// The daemon could not be spawned.
    SpawnFailed(io::Error),
    /// The daemon this shim spawned ended unsuccessfully before `READY`.
    DaemonExited { pid: u32, exit: String },
    /// The key's last daemon (`pid`, when the marker says) died during its index build; the key
    /// stays closed for `left_secs` more (S3b's SI-10).
    RecentBuildCrash { pid: Option<u32>, left_secs: u64 },
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Files(error) => error.fmt(f),
            Self::Refused(refusal) => refusal.fmt(f),
            Self::StartLockTimeout { last } => write!(
                f,
                "another shim held the start lock until the deadline (last attempt: {last})"
            ),
            Self::NoReady {
                last,
                owner,
                started,
            } => {
                write!(
                    f,
                    "no daemon answered READY before the deadline (last attempt: {last}"
                )?;
                match owner {
                    Owner::Held => write!(f, "; a live daemon holds the owner lock")?,
                    // The last look came just before this shim's own spawn, and D82 stops the
                    // looks while that child runs, so it may hold the lock by now.
                    Owner::Free if started.is_some() => write!(
                        f,
                        "; the owner lock was free when this shim last looked, before it started \
                         its daemon"
                    )?,
                    Owner::Free => write!(f, "; no daemon held the owner lock")?,
                    Owner::Unknown => {}
                }
                if let Some(pid) = started {
                    write!(
                        f,
                        "; the daemon this shim started, pid {pid}, did not answer"
                    )?;
                }
                write!(f, ")")
            }
            Self::SpawnFailed(error) => write!(f, "cannot start the daemon: {error}"),
            Self::DaemonExited { pid, exit } => write!(
                f,
                "the daemon this shim started, pid {pid}, ended before READY: {exit}"
            ),
            Self::RecentBuildCrash { pid, left_secs } => {
                write!(f, "the last daemon for this key")?;
                if let Some(pid) = pid {
                    write!(f, ", pid {pid},")?;
                }
                write!(
                    f,
                    " died during its index build; the key is not restarted for {} s from when a \
                     shim first saw that ({left_secs} s left), so that a build that dies every \
                     time does not cost a daemon per shim",
                    BUILD_CRASH_BACKOFF.as_secs()
                )
            }
        }
    }
}

impl From<FilesError> for Failure {
    fn from(error: FilesError) -> Self {
        Self::Files(error)
    }
}

/// One shim's search for its daemon.
pub(super) struct Arbiter<'a> {
    pub(super) paths: &'a DaemonPaths,
    pub(super) key: IdentityKey,
    /// The `HELLO` line, newline included.
    pub(super) hello: &'a str,
    /// The one deadline (spec §4.1 step 4).
    pub(super) deadline: Instant,
    pub(super) peer_uid: PeerUid,
    pub(super) hook: Option<Hook>,
}

impl Arbiter<'_> {
    fn hook(&self, point: Point) {
        if let Some(hook) = &self.hook {
            hook(point);
        }
    }

    fn reach(&self) -> Reached {
        reach(&Target {
            socket: self.paths.socket(),
            key: self.key,
            hello: self.hello,
            deadline: self.deadline,
            peer_uid: self.peer_uid,
            uid: current_uid(),
            hook: self.hook.as_ref(),
        })
    }

    fn past_deadline(&self) -> bool {
        Instant::now() >= self.deadline
    }

    fn pause(&self) {
        let left = self.deadline.saturating_duration_since(Instant::now());
        std::thread::sleep(POLL.min(left));
    }

    /// Find the daemon, starting one with `starter` if none is alive.
    pub(super) fn arbitrate(&self, starter: &mut dyn Starter) -> Result<Connected, Failure> {
        validate_runtime_dir(self.paths.runtime_dir()).map_err(FilesError::from)?;
        let mut last = match self.reach() {
            Reached::Ready(stream, ready) => {
                return Ok(Connected {
                    stream,
                    ready,
                    started: None,
                })
            }
            Reached::Refused(refusal) => return Err(Failure::Refused(refusal)),
            Reached::Transient(transient) => transient,
        };
        if matches!(last, Transient::NoReply) {
            // A daemon accepted and did not answer before the deadline: live but wedged
            // (spec §4.4). It is reported, never replaced.
            return Err(Failure::NoReady {
                last,
                owner: Owner::Unknown,
                started: None,
            });
        }

        let dir = KeyDir::open(self.paths)?;
        let start_lock = dir.start_lock()?;
        let mut waited = false;
        loop {
            match start_lock.try_lock() {
                Ok(()) => break,
                Err(TryLockError::WouldBlock) if !self.past_deadline() => {
                    if !waited {
                        waited = true;
                        self.hook(Point::StartLockBusy);
                    }
                    self.pause();
                }
                Err(TryLockError::WouldBlock) => return Err(Failure::StartLockTimeout { last }),
                Err(TryLockError::Error(source)) => {
                    return Err(Failure::Files(FilesError::Io {
                        op: "lock",
                        path: self.paths.start_lock().to_path_buf(),
                        source,
                    }))
                }
            }
        }
        self.hook(Point::StartLockTaken);

        // The start lock is held from here until this function returns (spec §4.1 step 6:
        // "Keep the start lock until READY or the deadline").
        let mut child: Option<Box<dyn Started>> = None;
        let mut owner = Owner::Unknown;
        loop {
            match self.reach() {
                Reached::Ready(stream, ready) => {
                    return Ok(Connected {
                        stream,
                        ready,
                        started: child,
                    })
                }
                Reached::Refused(refusal) => return Err(Failure::Refused(refusal)),
                Reached::Transient(transient) => last = transient,
            }
            if self.past_deadline() {
                return Err(Failure::NoReady {
                    last,
                    owner,
                    started: child.as_ref().map(|child| child.pid()),
                });
            }
            let child_running = match child.as_mut().map(|child| (child.pid(), child.exit())) {
                Some((_, None)) => true,
                Some((pid, Some(Exit::Failure(exit)))) => {
                    return Err(Failure::DaemonExited { pid, exit })
                }
                Some((_, Some(Exit::Success))) | None => false,
            };
            if !child_running {
                match dir.try_own()? {
                    Some(owner_lock) => {
                        owner = Owner::Free;
                        if let Some(crash) = recent_build_crash(&dir)? {
                            return Err(crash);
                        }
                        dir.remove_stale()?;
                        drop(owner_lock);
                        self.hook(Point::BeforeSpawn);
                        child = Some(starter.start(&dir).map_err(Failure::SpawnFailed)?);
                        self.hook(Point::AfterSpawn);
                    }
                    None => {
                        owner = Owner::Held;
                        self.hook(Point::OwnerHeld);
                    }
                }
            }
            self.pause();
        }
    }
}

/// Whether the key stays closed because its last daemon died during its build. A marker with no
/// sighting is seen now: the sighting is recorded and the key closes for the full backoff. One
/// with a sighting keeps the key closed until the backoff has passed since then; a sighting dated
/// in the future (a clock set back) is not counted, or it could hold the key closed for as long
/// as the clock is off. Call it holding the start and owner locks.
fn recent_build_crash(dir: &KeyDir<'_>) -> Result<Option<Failure>, FilesError> {
    let Some(marker) = dir.build_marker() else {
        return Ok(None);
    };
    let backoff = BUILD_CRASH_BACKOFF.as_secs();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let left_secs = match marker.seen {
        None => {
            dir.record_sighting(&marker, now)?;
            backoff
        }
        Some(seen) => match now.checked_sub(seen) {
            Some(age) if age < backoff => backoff - age,
            _ => return Ok(None),
        },
    };
    Ok(Some(Failure::RecentBuildCrash {
        pid: marker.pid,
        left_secs,
    }))
}

#[cfg(test)]
#[path = "arbitrate_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "arbitrate_race_tests.rs"]
mod race_tests;
