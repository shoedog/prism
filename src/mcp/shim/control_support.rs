//! A stand-in for a daemon's answers to the operator's control exchange (S5b): it owns the key's
//! owner lock (or not), records every line it reads, and answers each by a script of the test's.
use super::shim_support::{key, open_lock, read_line, HANG};
use crate::mcp::shared::control::{DaemonStatus, Instance};
use crate::mcp::shared::paths::DaemonPaths;
use crate::mcp::shared::preamble::BuildState;
use std::fs::{File, TryLockError};
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

/// What the fake does with a connection after reading its line.
pub(super) enum FakeReply {
    /// Write this line (newline included), then close.
    Line(String),
    /// Close without a reply, as a daemon that refuses the line does.
    Close,
    /// Never answer; hold the connection until the peer closes it or the fake stops.
    Silent,
}

pub(super) type ControlScript = Arc<dyn Fn(&str) -> FakeReply + Send + Sync>;

/// The pid the fake reports.
pub(super) const FAKE_PID: u32 = 4242;
/// The start time the fake reports.
pub(super) const FAKE_START: u64 = 1_760_600_000;

/// The instance the fake is.
pub(super) fn fake_instance() -> Instance {
    Instance {
        identity: key(),
        pid: FAKE_PID,
        start_time: FAKE_START,
    }
}

/// A status of the fake's, with `admitted` clients and the shutdown flag as given.
pub(super) fn fake_status(admitted: u32, shutdown: bool) -> DaemonStatus {
    DaemonStatus {
        instance: fake_instance(),
        build_identity: "b".repeat(64),
        repo_root: "/work/repo".to_string(),
        admitted,
        pending: 0,
        max_clients: 64,
        build: BuildState::Ready,
        index_idle_timeout_secs: 0,
        index_idle_pressure_relief: false,
        attempt: 1,
        generation: 2,
        idle_secs: (admitted == 0).then_some(7),
        idle_ttl_secs: 900,
        shutdown,
    }
}

/// A daemon on a thread of the test that answers control lines by its script.
pub(super) struct ControlFake {
    owner: Option<File>,
    lines: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    socket: PathBuf,
}

impl ControlFake {
    /// Bind the key's socket and answer; with `owned`, hold the key's owner lock as a daemon
    /// process does.
    pub(super) fn start(paths: &DaemonPaths, owned: bool, script: ControlScript) -> Self {
        let owner = owned.then(|| {
            let owner = open_lock(paths.owner_lock());
            match owner.try_lock() {
                Ok(()) => owner,
                Err(TryLockError::WouldBlock) => panic!("the owner lock is held"),
                Err(TryLockError::Error(error)) => panic!("lock: {error}"),
            }
        });
        let listener = UnixListener::bind(paths.socket()).expect("bind the fake daemon");
        listener.set_nonblocking(true).expect("nonblocking");
        let lines: Arc<Mutex<Vec<String>>> = Arc::default();
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let (lines, stop) = (Arc::clone(&lines), Arc::clone(&stop));
            std::thread::spawn(move || {
                while !stop.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((stream, _)) => answer(stream, &script, &lines, &stop),
                        Err(_) => std::thread::sleep(Duration::from_millis(2)),
                    }
                }
            })
        };
        Self {
            owner,
            lines,
            stop,
            thread: Some(thread),
            socket: paths.socket().to_path_buf(),
        }
    }

    /// Every line read so far, newlines included.
    pub(super) fn lines(&self) -> Vec<String> {
        self.lines.lock().expect("lines").clone()
    }

    /// Stop answering, remove the socket and release the owner lock, as a drained daemon does.
    pub(super) fn retire(mut self) {
        let _ = std::fs::remove_file(&self.socket);
        self.stop_thread();
    }

    fn stop_thread(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        self.owner = None;
    }
}

impl Drop for ControlFake {
    fn drop(&mut self) {
        self.stop_thread();
    }
}

fn answer(
    mut stream: UnixStream,
    script: &ControlScript,
    lines: &Mutex<Vec<String>>,
    stop: &AtomicBool,
) {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(HANG));
    let line = read_line(&stream);
    lines.lock().expect("lines").push(line.clone());
    match script(&line) {
        FakeReply::Line(reply) => {
            let _ = stream.write_all(reply.as_bytes());
        }
        FakeReply::Close => {}
        FakeReply::Silent => {
            let _ = stream.set_read_timeout(Some(Duration::from_millis(20)));
            let mut buf = [0u8; 64];
            while !stop.load(Ordering::SeqCst) {
                match stream.read(&mut buf) {
                    Ok(0) => return,
                    Ok(_) => {}
                    Err(error)
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                        ) => {}
                    Err(_) => return,
                }
            }
        }
    }
}

/// A script that answers every line with `reply`.
pub(super) fn always_line(reply: String) -> ControlScript {
    Arc::new(move |_| FakeReply::Line(reply.clone()))
}

/// Make a FIFO at `path`, as a process of this user could plant one in place of a key's file.
pub(super) fn mkfifo(path: &std::path::Path) {
    use std::os::unix::ffi::OsStrExt;
    let name = std::ffi::CString::new(path.as_os_str().as_bytes()).expect("a path without NUL");
    // SAFETY: `name` is a valid NUL-terminated string that outlives the call.
    let made = unsafe { libc::mkfifo(name.as_ptr(), 0o600) };
    assert_eq!(made, 0, "mkfifo: {}", std::io::Error::last_os_error());
}

/// Run `work` on a thread of its own and return its result, or fail the test if it has not
/// returned within [`HANG`]: an independent watchdog for code that must not wait on anything.
pub(super) fn within_hang<T: Send + 'static>(
    what: &str,
    work: impl FnOnce() -> T + Send + 'static,
) -> T {
    let (sent, received) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = sent.send(work());
    });
    received
        .recv_timeout(HANG)
        .unwrap_or_else(|_| panic!("{what} did not return within {HANG:?}"))
}
