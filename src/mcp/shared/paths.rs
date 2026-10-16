//! The files of one daemon inside the runtime directory (spec §4.3): the socket, the two lock
//! files, the pid file and the log, all named by the first 16 hex digits of the identity key.
//! Only names are computed here; nothing is created, opened or removed.
use super::identity::IdentityKey;
use super::runtime_dir::RUNTIME_DIR_ENV;
use std::fmt;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

/// Longest socket path, in bytes, that every supported platform can bind: `sun_path` holds 104
/// bytes on Darwin including the terminating NUL (spec §4.3).
pub const MAX_SOCKET_PATH_BYTES: usize = 103;

/// The socket path for this runtime directory does not fit in a socket address. The shim fails
/// closed (spec §4.1 step 4); a shorter `PRISM_MCP_RUNTIME_DIR` is the way out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SocketPathTooLong {
    pub path: PathBuf,
    /// Length of `path` in bytes.
    pub len: usize,
}

impl fmt::Display for SocketPathTooLong {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "daemon socket path {} is {} bytes; a Unix socket path can be at most \
             {MAX_SOCKET_PATH_BYTES} (set {RUNTIME_DIR_ENV} to a shorter absolute directory)",
            self.path.display(),
            self.len
        )
    }
}

impl std::error::Error for SocketPathTooLong {}

/// Where one daemon's files are. Two keys with the same first 16 hex digits have the same
/// paths: the paths locate a daemon, and the preamble's full-key comparison identifies it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DaemonPaths {
    runtime_dir: PathBuf,
    socket: PathBuf,
    start_lock: PathBuf,
    owner_lock: PathBuf,
    pid_file: PathBuf,
    log_file: PathBuf,
    build_marker: PathBuf,
}

impl DaemonPaths {
    /// The paths for `key` under `runtime_dir`, or an error when the socket path is longer
    /// than [`MAX_SOCKET_PATH_BYTES`]. `runtime_dir` is used as given; validating it is the
    /// caller's separate step (`validate_runtime_dir`).
    pub fn new(runtime_dir: &Path, key: &IdentityKey) -> Result<Self, SocketPathTooLong> {
        let locator = key.locator();
        let file = |extension: &str| runtime_dir.join(format!("{locator}.{extension}"));
        let socket = file("sock");
        let len = socket.as_os_str().as_bytes().len();
        if len > MAX_SOCKET_PATH_BYTES {
            return Err(SocketPathTooLong { path: socket, len });
        }
        Ok(Self {
            runtime_dir: runtime_dir.to_path_buf(),
            socket,
            start_lock: file("start"),
            owner_lock: file("owner"),
            pid_file: file("pid"),
            log_file: file("log"),
            build_marker: file("build"),
        })
    }

    pub fn runtime_dir(&self) -> &Path {
        &self.runtime_dir
    }

    /// `<key16>.sock`: the daemon's listening socket.
    pub fn socket(&self) -> &Path {
        &self.socket
    }

    /// `<key16>.start`: held by a shim while it arbitrates who starts the daemon.
    pub fn start_lock(&self) -> &Path {
        &self.start_lock
    }

    /// `<key16>.owner`: held by the daemon for its whole life.
    pub fn owner_lock(&self) -> &Path {
        &self.owner_lock
    }

    /// `<key16>.pid`: pid, start time and build identity of the daemon.
    pub fn pid_file(&self) -> &Path {
        &self.pid_file
    }

    /// `<key16>.log`: the daemon's stdout and stderr.
    pub fn log_file(&self) -> &Path {
        &self.log_file
    }

    /// `<key16>.build`: present while the daemon's index build runs. A daemon that dies during
    /// its build leaves it, and the next shim reads it to bound how often it restarts a daemon
    /// whose build keeps dying (S5a).
    pub fn build_marker(&self) -> &Path {
        &self.build_marker
    }
}

#[cfg(test)]
#[path = "paths_tests.rs"]
mod tests;
