//! Pieces the shim's tests share: a runtime directory of the test's own, keys, preamble lines,
//! and a stand-in for one daemon connection.
//!
//! No test here sleeps to order two events: a fake daemon answers when it has read what it waits
//! for, a test waits for a condition that must become true (`wait_until`), and gives up after
//! [`HANG`].
use super::arbitrate::{Exit, Started, Starter};
use super::key_files::KeyDir;
use crate::mcp::concise_shape::ConciseShapeMode;
use crate::mcp::output::StructuredContentMode;
use crate::mcp::settings::{ClientSettings, ConnectionSettings};
use crate::mcp::shared::identity::IdentityKey;
use crate::mcp::shared::paths::DaemonPaths;
use crate::mcp::shared::preamble::{BuildState, Hello, Ready, Reply};
use crate::mcp::shared::runtime_dir::{current_uid, validate_runtime_dir};
use std::fs::{File, TryLockError};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// How long a test waits for something that must happen before it calls the run hung.
pub(super) const HANG: Duration = Duration::from_secs(30);

/// The key the tests' daemons serve.
pub(super) fn key() -> IdentityKey {
    IdentityKey::from_hex(&"cd".repeat(32)).expect("a key")
}

pub(super) fn other_key() -> IdentityKey {
    IdentityKey::from_hex(&"ef".repeat(32)).expect("a key")
}

/// A validated runtime directory of the test's own under `/tmp` (short enough for a socket
/// path), and the paths of [`key`] in it.
pub(super) fn runtime() -> (tempfile::TempDir, DaemonPaths) {
    let base = tempfile::Builder::new()
        .prefix("s5a")
        .tempdir_in("/tmp")
        .expect("a temporary directory under /tmp");
    let dir: PathBuf = base.path().join("prism-mcpd");
    validate_runtime_dir(&dir).expect("a valid runtime directory");
    let paths = DaemonPaths::new(&dir, &key()).expect("paths");
    (base, paths)
}

/// The `HELLO` line a shim sends for `key`, newline included.
pub(super) fn hello_line(key: IdentityKey) -> String {
    let output = ConnectionSettings {
        result_cap: 80_000,
        structured_content: StructuredContentMode::Always,
        concise_shape: ConciseShapeMode::Slim,
    };
    let settings = ClientSettings::new(output, 20, false).expect("settings");
    Hello {
        identity: key,
        settings,
    }
    .encode()
}

/// The `READY` line of a daemon with `pid` serving `key`, newline included.
pub(super) fn ready_line(key: IdentityKey, pid: u32) -> String {
    Reply::Ready(Ready {
        identity: key,
        pid,
        start_time: 1_700_000_000,
        generation: 0,
        build: BuildState::Idle,
    })
    .encode()
}

/// This user's uid, as the peer check of a connection this process made reads it.
pub(super) fn this_user(stream: &UnixStream) -> std::io::Result<u32> {
    crate::mcp::daemon::peer::peer_uid(stream)
}

/// A peer of another user, as no test can be without root.
pub(super) fn another_user(_stream: &UnixStream) -> std::io::Result<u32> {
    Ok(current_uid().wrapping_add(1))
}

/// Read one line from a daemon-side stream, newline included, one byte at a time so nothing
/// past it is taken; what arrived before the end of stream if no newline came.
pub(super) fn read_line(mut stream: &UnixStream) -> String {
    let mut line = Vec::new();
    let mut byte = [0u8; 1];
    while let Ok(1) = stream.read(&mut byte) {
        line.push(byte[0]);
        if byte[0] == b'\n' {
            break;
        }
    }
    String::from_utf8_lossy(&line).into_owned()
}

/// Everything the peer sends until it closes.
pub(super) fn read_to_end(mut stream: &UnixStream) -> Vec<u8> {
    let mut bytes = Vec::new();
    let _ = stream.read_to_end(&mut bytes);
    bytes
}

/// Poll `condition` until it holds; panic, naming `what`, after [`HANG`].
pub(super) fn wait_until(what: &str, mut condition: impl FnMut() -> bool) {
    let give_up = Instant::now() + HANG;
    while !condition() {
        assert!(
            Instant::now() < give_up,
            "gave up after {HANG:?} waiting for: {what}"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// What a fake daemon does with a connection, after reading its `HELLO`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Answer {
    /// `READY` with the daemon's pid, then hold the connection until the shim closes it.
    Ready,
    /// `RETIRING`, then close.
    Retiring,
    /// Close without a reply.
    Close,
    /// Hold the connection and never answer.
    Silent,
    /// `MISMATCH`, naming [`other_key`].
    Mismatch,
}

/// What a fake daemon answers each connection, by its number (0 first).
pub(super) type Script = Arc<dyn Fn(u32) -> Answer + Send + Sync>;

pub(super) fn always(answer: Answer) -> Script {
    Arc::new(move |_| answer)
}

/// A daemon on a thread of the test: it owns the key's owner lock (taken as a daemon process
/// takes it, on a descriptor of its own) and answers on the key's socket by its script. Dropping
/// it stops it and releases the lock; it leaves its socket file, as a daemon that died does.
pub(super) struct FakeDaemon {
    owner: Option<File>,
    stop: Arc<AtomicBool>,
    socket: PathBuf,
    thread: Option<JoinHandle<()>>,
}

impl FakeDaemon {
    /// Take the owner lock of `paths` and serve: `None`, touching nothing, when another holds it.
    pub(super) fn claim(paths: &DaemonPaths, pid: u32, script: Script) -> Option<Self> {
        let owner = open_lock(paths.owner_lock());
        match owner.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => return None,
            Err(TryLockError::Error(error)) => panic!("lock: {error}"),
        }
        let _ = std::fs::remove_file(paths.socket());
        let mut daemon = Self::listen(paths, pid, script);
        daemon.owner = Some(owner);
        Some(daemon)
    }

    /// Serve on the key's socket without the owner lock (a listener nobody owns).
    pub(super) fn listen(paths: &DaemonPaths, pid: u32, script: Script) -> Self {
        let listener = UnixListener::bind(paths.socket()).expect("bind the fake daemon");
        listener.set_nonblocking(true).expect("nonblocking");
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let stop = Arc::clone(&stop);
            std::thread::spawn(move || {
                let mut accepted = 0;
                while !stop.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            let answer = script(accepted);
                            accepted += 1;
                            let stop = Arc::clone(&stop);
                            std::thread::spawn(move || serve(stream, pid, answer, &stop));
                        }
                        Err(_) => std::thread::sleep(Duration::from_millis(2)),
                    }
                }
            })
        };
        Self {
            owner: None,
            stop,
            socket: paths.socket().to_path_buf(),
            thread: Some(thread),
        }
    }

    /// Retire as a daemon does: remove the socket, stop, release the owner lock.
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

impl Drop for FakeDaemon {
    fn drop(&mut self) {
        self.stop_thread();
    }
}

fn serve(mut stream: UnixStream, pid: u32, answer: Answer, stop: &AtomicBool) {
    let _ = stream.set_nonblocking(false);
    let hello = read_line(&stream);
    let identity = Hello::decode(hello.trim_end_matches('\n').as_bytes())
        .map(|hello| hello.identity)
        .unwrap_or_else(|_| key());
    match answer {
        Answer::Ready => {
            let _ = stream.write_all(ready_line(identity, pid).as_bytes());
            hold(&stream, stop);
        }
        Answer::Retiring => {
            let _ = stream.write_all(Reply::Retiring.encode().as_bytes());
        }
        Answer::Close => {}
        Answer::Silent => hold(&stream, stop),
        Answer::Mismatch => {
            let reply = Reply::Mismatch {
                expected: other_key(),
            };
            let _ = stream.write_all(reply.encode().as_bytes());
        }
    }
}

/// Keep a connection until its peer closes it or the daemon stops.
fn hold(mut stream: &UnixStream, stop: &AtomicBool) {
    let _ = stream.set_read_timeout(Some(Duration::from_millis(20)));
    let mut buf = [0u8; 256];
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

/// A started fake daemon process: it claims the key when started (or when its gate opens), and
/// exits successfully if the key was taken, as a daemon process does.
pub(super) struct FakeChild {
    pub(super) pid: u32,
    pub(super) state: Arc<Mutex<ChildState>>,
}

#[derive(Default)]
pub(super) struct ChildState {
    pub(super) exit: Option<Exit>,
    pub(super) daemon: Option<FakeDaemon>,
}

impl Started for FakeChild {
    fn pid(&self) -> u32 {
        self.pid
    }

    fn exit(&mut self) -> Option<Exit> {
        self.state.lock().expect("child state").exit.clone()
    }

    fn reap(self: Box<Self>) {}
}

/// Starts fake daemons: the n-th start (0 first) is `make(n)`. Counts its starts.
pub(super) struct FakeStarter {
    pub(super) starts: Arc<AtomicU32>,
    make: Box<dyn FnMut(u32) -> std::io::Result<Box<dyn Started>> + Send>,
}

impl FakeStarter {
    pub(super) fn new(
        make: impl FnMut(u32) -> std::io::Result<Box<dyn Started>> + Send + 'static,
    ) -> Self {
        Self {
            starts: Arc::new(AtomicU32::new(0)),
            make: Box::new(make),
        }
    }

    /// Each start claims the key at once and serves `script`; the children's states are
    /// collected in `children` for the test to inspect.
    pub(super) fn claiming(
        paths: &DaemonPaths,
        script: Script,
        children: Arc<Mutex<Vec<Arc<Mutex<ChildState>>>>>,
    ) -> Self {
        let paths = paths.clone();
        Self::new(move |n| {
            let pid = 1000 + n;
            let state = Arc::new(Mutex::new(ChildState::default()));
            {
                let mut child = state.lock().expect("child state");
                match FakeDaemon::claim(&paths, pid, Arc::clone(&script)) {
                    Some(daemon) => child.daemon = Some(daemon),
                    None => child.exit = Some(Exit::Success),
                }
            }
            children.lock().expect("children").push(Arc::clone(&state));
            Ok(Box::new(FakeChild { pid, state }))
        })
    }

    /// Each start returns a child that has already ended with `exit` and served nothing.
    pub(super) fn exiting(exit: Exit) -> Self {
        Self::new(move |n| {
            let state = ChildState {
                exit: Some(exit.clone()),
                daemon: None,
            };
            Ok(Box::new(FakeChild {
                pid: 1000 + n,
                state: Arc::new(Mutex::new(state)),
            }))
        })
    }
}

impl FakeStarter {
    /// Start one fake daemon, as [`Starter::start`] does.
    pub(super) fn start_fake(&mut self) -> std::io::Result<Box<dyn Started>> {
        let n = self.starts.fetch_add(1, Ordering::SeqCst);
        (self.make)(n)
    }
}

impl Starter for FakeStarter {
    fn start(&mut self, _dir: &KeyDir<'_>) -> std::io::Result<Box<dyn Started>> {
        self.start_fake()
    }
}

/// Open (creating) a lock file on a descriptor of its own, as another process would. On macOS a
/// create that races another process's create of the same name can fail with `ENOENT` (S4's
/// D53); it is retried.
pub(super) fn open_lock(path: &std::path::Path) -> File {
    let mut tries = 0;
    loop {
        let opened = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(path);
        match opened {
            Ok(file) => return file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && tries < 100 => {
                tries += 1;
            }
            Err(error) => panic!("open {}: {error}", path.display()),
        }
    }
}

/// A gate a thread waits at until the test opens it.
#[derive(Default)]
pub(super) struct Gate {
    open: Mutex<bool>,
    changed: Condvar,
}

impl Gate {
    pub(super) fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Wait until the gate is open; panic after [`HANG`].
    pub(super) fn pass(&self) {
        let open = self.open.lock().expect("gate");
        let (open, _) = self
            .changed
            .wait_timeout_while(open, HANG, |open| !*open)
            .expect("gate");
        assert!(*open, "the gate was never opened");
    }

    pub(super) fn open(&self) {
        *self.open.lock().expect("gate") = true;
        self.changed.notify_all();
    }
}

/// The current time in seconds since the Unix epoch, as a build marker writes it.
pub(super) fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("after the epoch")
        .as_secs()
}

/// A listening socket at `path` whose backlog takes no more connections, as a live daemon's does
/// once it stops accepting: bound and listening with a backlog of 0 (raw calls: `std` chooses its
/// own backlog), then filled with non-blocking clients until the kernel refuses another. The
/// caller holds both. On Linux a blocking `connect` to it waits for room; on macOS it is refused.
pub(super) fn full_backlog(path: &std::path::Path) -> (OwnedFd, Vec<OwnedFd>) {
    use std::os::unix::ffi::OsStrExt;
    let socket = || {
        // SAFETY: `socket` takes plain integers and returns a new descriptor or -1.
        let fd = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0) };
        assert!(fd >= 0, "socket: {}", std::io::Error::last_os_error());
        // SAFETY: `fd` was just returned by `socket` and nothing else owns it.
        unsafe { OwnedFd::from_raw_fd(fd) }
    };
    // SAFETY: an all-zero `sockaddr_un` is a valid value of the type.
    let mut addr: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    addr.sun_family = libc::AF_UNIX as libc::sa_family_t;
    let bytes = path.as_os_str().as_bytes();
    assert!(bytes.len() < addr.sun_path.len());
    for (slot, byte) in addr.sun_path.iter_mut().zip(bytes) {
        *slot = *byte as libc::c_char;
    }
    let len = std::mem::size_of::<libc::sockaddr_un>() as libc::socklen_t;
    let as_sockaddr = std::ptr::addr_of!(addr).cast::<libc::sockaddr>();
    let listener = socket();
    // SAFETY: the descriptor is a socket and `addr` a valid address of `len` bytes.
    let bound = unsafe { libc::bind(listener.as_raw_fd(), as_sockaddr, len) };
    assert_eq!(bound, 0, "bind: {}", std::io::Error::last_os_error());
    // SAFETY: the descriptor is a bound socket.
    let listening = unsafe { libc::listen(listener.as_raw_fd(), 0) };
    assert_eq!(listening, 0, "listen: {}", std::io::Error::last_os_error());
    let mut clients = Vec::new();
    for _ in 0..256 {
        let client = socket();
        // SAFETY: `F_SETFL` on an open descriptor with plain flags.
        unsafe { libc::fcntl(client.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK) };
        // SAFETY: the descriptor is a socket and `addr` a valid address of `len` bytes.
        let connected = unsafe { libc::connect(client.as_raw_fd(), as_sockaddr, len) };
        let accepted = connected == 0
            || std::io::Error::last_os_error().raw_os_error() == Some(libc::EINPROGRESS);
        if !accepted {
            break;
        }
        clients.push(client);
    }
    (listener, clients)
}

/// Whether these tests run as root, who searches any directory whatever its mode: a test that
/// needs `EACCES` from a directory it made unsearchable skips there (the Linux container runs as
/// root).
pub(super) fn as_root() -> bool {
    current_uid() == 0
}

/// `dir` made unsearchable (mode 0) until the guard is dropped, which gives it 0700 back, so a
/// temporary directory under it can still be removed however the test ends.
pub(super) struct Unsearchable(PathBuf);

impl Unsearchable {
    pub(super) fn new(dir: &std::path::Path) -> Self {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0)).expect("chmod 0");
        Self(dir.to_path_buf())
    }
}

impl Drop for Unsearchable {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o700));
    }
}
