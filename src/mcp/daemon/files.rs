//! A daemon's files in the runtime directory (spec §3.1 "Starting", §4.3 "Files"): the owner
//! lock, held for the daemon's whole life; the socket; the pid file.
//!
//! Who owns a key is decided by one lock: `<key16>.owner`, taken with `flock(LOCK_EX|LOCK_NB)`.
//! A daemon that cannot take it is not the owner and touches nothing (spec §3.1: "failure =
//! another live daemon owns this key → exit 0 silently"). The owner, and only the owner,
//! replaces a stale socket, writes the pid file and removes both. Lock files are created 0600
//! and never removed: removing one is how two holders of one lock come to exist.
//!
//! The runtime directory is validated (`validate_runtime_dir`) and then opened once; every file
//! but the socket is opened, examined and removed through that descriptor, with `O_NOFOLLOW`, so
//! a directory swapped under the daemon after the check is not the one it writes into or
//! removes from. `bind` takes a path, so the socket is bound by path (S2's SI-5); it is removed
//! through the descriptor, and only while the entry is still the one this daemon bound. Like
//! the directory checks, this is hardening; the isolation guarantee between users is the peer
//! uid check on every connection.
use crate::mcp::shared::identity::IdentityKey;
use crate::mcp::shared::paths::{DaemonPaths, SocketPathTooLong};
use crate::mcp::shared::runtime_dir::{current_uid, validate_runtime_dir, RuntimeDirError};
use std::ffi::CString;
use std::fmt;
use std::fs::{File, TryLockError};
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};

/// The mode lock files and the pid file are created with.
const PRIVATE_FILE_MODE: libc::c_uint = 0o600;

/// Why a daemon could not take or use its files. Each ends the daemon's start with a message;
/// none is retried.
#[derive(Debug)]
pub(super) enum FilesError {
    RuntimeDir(RuntimeDirError),
    SocketPath(SocketPathTooLong),
    /// The filesystem refused `op` on `path`.
    Io {
        op: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    /// What is at `path` is not what this daemon may use: `why` says what it is instead.
    Unusable {
        path: PathBuf,
        why: &'static str,
    },
}

impl fmt::Display for FilesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RuntimeDir(error) => error.fmt(f),
            Self::SocketPath(error) => error.fmt(f),
            Self::Io { op, path, source } => {
                write!(f, "cannot {op} {}: {source}", path.display())
            }
            Self::Unusable { path, why } => write!(f, "{} {why}", path.display()),
        }
    }
}

impl std::error::Error for FilesError {}

impl From<RuntimeDirError> for FilesError {
    fn from(error: RuntimeDirError) -> Self {
        Self::RuntimeDir(error)
    }
}

impl From<SocketPathTooLong> for FilesError {
    fn from(error: SocketPathTooLong) -> Self {
        Self::SocketPath(error)
    }
}

/// The outcome of trying to own a key.
pub(super) enum Claim {
    /// This process holds the owner lock until the `DaemonFiles` is released or dropped.
    Owned(Box<DaemonFiles>),
    /// Another process holds it: a daemon for this key is alive (starting, serving or
    /// draining). Nothing was created but the lock file, if it was missing.
    Taken,
}

/// Which file a path or a descriptor names: device and inode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileId {
    dev: u64,
    ino: u64,
}

impl FileId {
    fn of(metadata: &std::fs::Metadata) -> Self {
        Self {
            dev: metadata.dev(),
            ino: metadata.ino(),
        }
    }
}

/// The files of the daemon that owns a key. Dropping it releases the owner lock; `release`
/// first removes the pid file.
pub(super) struct DaemonFiles {
    paths: DaemonPaths,
    dir: OwnedFd,
    owner: File,
    owner_id: FileId,
    socket_id: Option<FileId>,
    pid_contents: Option<String>,
}

impl DaemonFiles {
    /// Validate `runtime_dir`, then try to take the owner lock of `key` in it.
    pub(super) fn claim(runtime_dir: &Path, key: &IdentityKey) -> Result<Claim, FilesError> {
        validate_runtime_dir(runtime_dir)?;
        let paths = DaemonPaths::new(runtime_dir, key)?;
        let dir = open_runtime_dir(paths.runtime_dir())?;
        let owner = open_private(&dir, paths.owner_lock(), libc::O_RDWR, Create::Yes)?;
        let owner_id = FileId::of(&owner.metadata().map_err(|source| FilesError::Io {
            op: "inspect",
            path: paths.owner_lock().to_path_buf(),
            source,
        })?);
        match owner.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => return Ok(Claim::Taken),
            Err(TryLockError::Error(source)) => {
                return Err(FilesError::Io {
                    op: "lock",
                    path: paths.owner_lock().to_path_buf(),
                    source,
                })
            }
        }
        Ok(Claim::Owned(Box::new(Self {
            paths,
            dir,
            owner,
            owner_id,
            socket_id: None,
            pid_contents: None,
        })))
    }

    pub(super) fn paths(&self) -> &DaemonPaths {
        &self.paths
    }

    /// Bind the daemon's socket. Whatever is at its path is removed first: a socket there was
    /// left by a daemon that died (the owner lock this daemon holds says no other is alive), and
    /// only the owner removes it (spec §4.6 "Daemon crashes").
    pub(super) fn bind(&mut self) -> Result<UnixListener, FilesError> {
        let socket = self.paths.socket().to_path_buf();
        match unlink_at(&self.dir, &socket) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(FilesError::Io {
                    op: "remove the stale socket",
                    path: socket,
                    source,
                })
            }
        }
        let listener = UnixListener::bind(&socket).map_err(|source| FilesError::Io {
            op: "bind",
            path: socket.clone(),
            source,
        })?;
        let bound = stat_at(&self.dir, &socket).map_err(|source| FilesError::Io {
            op: "inspect",
            path: socket.clone(),
            source,
        })?;
        self.socket_id = Some(bound);
        Ok(listener)
    }

    /// Write the pid file: `pid\nstart_time\nbuild_identity\n` (spec §3.1).
    pub(super) fn write_pid(
        &mut self,
        pid: u32,
        start_time: u64,
        build_identity: &str,
    ) -> Result<(), FilesError> {
        let path = self.paths.pid_file().to_path_buf();
        let contents = format!("{pid}\n{start_time}\n{build_identity}\n");
        let mut file = open_private(
            &self.dir,
            &path,
            libc::O_WRONLY | libc::O_TRUNC,
            Create::Yes,
        )?;
        file.write_all(contents.as_bytes())
            .map_err(|source| FilesError::Io {
                op: "write",
                path,
                source,
            })?;
        self.pid_contents = Some(contents);
        Ok(())
    }

    /// Whether the owner lock and the socket that the paths name are still the ones this
    /// daemon holds and bound. Not, when the runtime directory was removed or replaced under
    /// it: another daemon could then take the key's lock in the new directory (lane README,
    /// notes for S4).
    pub(super) fn still_ours(&self) -> bool {
        let same = |path: &Path, id: FileId| {
            std::fs::symlink_metadata(path).is_ok_and(|metadata| FileId::of(&metadata) == id)
        };
        same(self.paths.owner_lock(), self.owner_id)
            && self
                .socket_id
                .is_none_or(|id| same(self.paths.socket(), id))
    }

    /// Remove the socket, if the entry of its name in this daemon's directory is still the one
    /// this daemon bound. Errors are ignored: the daemon is stopping, and a socket left behind
    /// is replaced by the next owner.
    pub(super) fn unlink_socket(&self) {
        let socket = self.paths.socket();
        if self
            .socket_id
            .is_some_and(|id| stat_at(&self.dir, socket).is_ok_and(|found| found == id))
        {
            let _ = unlink_at(&self.dir, socket);
        }
    }

    /// Remove the pid file if it still holds what this daemon wrote, then release the owner
    /// lock (spec §3.1: "the pid file is removed, the owner lock released, exit 0").
    pub(super) fn release(self) {
        if let Some(written) = &self.pid_contents {
            let pid_file = self.paths.pid_file();
            let mut found = String::new();
            let ours = open_private(&self.dir, pid_file, libc::O_RDONLY, Create::No)
                .and_then(|mut file| {
                    file.read_to_string(&mut found)
                        .map_err(|source| FilesError::Io {
                            op: "read",
                            path: pid_file.to_path_buf(),
                            source,
                        })
                })
                .is_ok_and(|_| found == *written);
            if ours {
                let _ = unlink_at(&self.dir, pid_file);
            }
        }
        // Closing the only descriptor of the lock file releases the lock.
        drop(self.owner);
    }
}

/// The last component of `path`, as a C string, for the `*at` calls.
fn entry_name(path: &Path) -> io::Result<CString> {
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no file name"))?;
    CString::new(name.as_bytes()).map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))
}

/// Open the runtime directory itself, not a symlink in its place, and check that what was
/// opened is a directory this user owns with mode 0700 (the validation's facts, now about the
/// descriptor every later file operation goes through).
fn open_runtime_dir(path: &Path) -> Result<OwnedFd, FilesError> {
    let io_error = |source| FilesError::Io {
        op: "open",
        path: path.to_path_buf(),
        source,
    };
    let name = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io_error(io::Error::from(io::ErrorKind::InvalidInput)))?;
    // SAFETY: `name` is a valid NUL-terminated string that outlives the call.
    let fd = unsafe {
        libc::open(
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(io_error(io::Error::last_os_error()));
    }
    // SAFETY: `fd` was just returned by `open` and nothing else owns it.
    let dir = unsafe { OwnedFd::from_raw_fd(fd) };
    let file = File::from(dir);
    let metadata = file.metadata().map_err(io_error)?;
    if !metadata.is_dir() || metadata.uid() != current_uid() || metadata.mode() & 0o777 != 0o700 {
        return Err(FilesError::Unusable {
            path: path.to_path_buf(),
            why: "changed after it was validated: it is no longer a 0700 directory of this user",
        });
    }
    Ok(OwnedFd::from(file))
}

/// Whether `open_private` creates the file when it is missing.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Create {
    Yes,
    No,
}

/// Open the entry of `path`'s name in `dir`, never through a symlink, creating it 0600 if
/// `create` says so; refuse anything but a regular file this user owns.
///
/// A missing file is created with `O_EXCL`, and an existing one opened without `O_CREAT`. A
/// plain `O_CREAT` is not used: on macOS it fails now and then with `ENOENT` when another
/// process creates the same name at the same moment, which is exactly what daemons racing for
/// one key do (D53; measured in validation, about 1 in 2,000 eight-way races).
fn open_private(
    dir: &OwnedFd,
    path: &Path,
    flags: libc::c_int,
    create: Create,
) -> Result<File, FilesError> {
    let io_error = |op, source| FilesError::Io {
        op,
        path: path.to_path_buf(),
        source,
    };
    let name = entry_name(path).map_err(|source| io_error("open", source))?;
    let open = |flags: libc::c_int| {
        // SAFETY: `dir` is an open directory descriptor and `name` a valid NUL-terminated
        // string, both alive for the call; the mode is passed as the variadic `c_uint` `openat`
        // reads.
        let fd = unsafe {
            libc::openat(
                dir.as_raw_fd(),
                name.as_ptr(),
                flags | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                PRIVATE_FILE_MODE,
            )
        };
        if fd < 0 {
            Err(io::Error::last_os_error())
        } else {
            // SAFETY: `fd` was just returned by `openat` and nothing else owns it.
            Ok(unsafe { File::from_raw_fd(fd) })
        }
    };
    let opened = match create {
        Create::Yes => match open(flags | libc::O_CREAT | libc::O_EXCL) {
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => open(flags),
            created => created,
        },
        Create::No => open(flags),
    };
    let file = opened.map_err(|source| io_error("open", source))?;
    let metadata = file
        .metadata()
        .map_err(|source| io_error("inspect", source))?;
    if !metadata.file_type().is_file() || metadata.uid() != current_uid() {
        return Err(FilesError::Unusable {
            path: path.to_path_buf(),
            why: "is not a regular file owned by this user",
        });
    }
    Ok(file)
}

/// `fstatat` of `path`'s name in `dir`, not following a symlink.
fn stat_at(dir: &OwnedFd, path: &Path) -> io::Result<FileId> {
    let name = entry_name(path)?;
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: `dir` is an open directory descriptor, `name` a valid NUL-terminated string and
    // `stat` points to writable memory of the right size, all alive for the call.
    let result = unsafe {
        libc::fstatat(
            dir.as_raw_fd(),
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `fstatat` returned 0, so it filled `stat`.
    let stat = unsafe { stat.assume_init() };
    // The field types differ between platforms (`dev_t` is `i32` on macOS); both are
    // reinterpreted the way `std`'s `MetadataExt` does, so ids from both sources compare.
    #[allow(clippy::unnecessary_cast)]
    Ok(FileId {
        dev: stat.st_dev as u64,
        ino: stat.st_ino as u64,
    })
}

/// `unlinkat` of `path`'s name in `dir`.
fn unlink_at(dir: &OwnedFd, path: &Path) -> io::Result<()> {
    let name = entry_name(path)?;
    // SAFETY: `dir` is an open directory descriptor and `name` a valid NUL-terminated string,
    // both alive for the call.
    let result = unsafe { libc::unlinkat(dir.as_raw_fd(), name.as_ptr(), 0) };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Whether `path` names a socket. Tests only.
#[cfg(test)]
pub(super) fn is_socket(path: &Path) -> bool {
    use std::os::unix::fs::FileTypeExt;
    std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_socket())
}

#[cfg(test)]
#[path = "files_tests.rs"]
mod tests;
