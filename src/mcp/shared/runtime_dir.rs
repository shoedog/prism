//! The per-user runtime directory that holds every daemon's socket, locks, pid file and log
//! (spec §4.3), and the validation that the shim and the daemon both run on every use (§7).
//!
//! Validation is what keeps another local user out: the directory must be a real directory
//! (not a symlink), owned by the current user, with mode 0700. It is checked by path with
//! `lstat`, not through a held descriptor, so it is a check against other users who cannot
//! write the parent directory; it is not a defense against a process of the same user (§2).
use std::ffi::OsString;
use std::fmt;
use std::io;
use std::os::unix::fs::{DirBuilderExt, MetadataExt};
use std::path::{Path, PathBuf};

/// Environment variable that replaces the base directory. Every launcher that should share a
/// daemon must resolve the same value (spec §4.3).
pub const RUNTIME_DIR_ENV: &str = "PRISM_MCP_RUNTIME_DIR";
/// The directory joined to the base. This, not the base, is what gets validated and created.
pub const RUNTIME_DIR_NAME: &str = "prism-mcpd";

/// The nine permission bits a runtime directory must have, exactly.
const PRIVATE_DIR_MODE: u32 = 0o700;
const PERMISSION_BITS: u32 = 0o777;

/// Why the runtime directory cannot be used. Every variant means "fail closed" (spec §4.1 step
/// 7, §4.6 last row); none is retried.
#[derive(Debug)]
pub enum RuntimeDirError {
    /// The base is not an absolute path, so two processes could resolve it differently.
    NotAbsolute {
        path: PathBuf,
    },
    /// The filesystem refused: `op` is `"inspect"` (the `lstat`) or `"create"` (the `mkdir`).
    Io {
        op: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    Symlink {
        path: PathBuf,
    },
    NotADirectory {
        path: PathBuf,
    },
    NotOwned {
        path: PathBuf,
        owner: u32,
        current: u32,
    },
    /// `mode` is the directory's nine permission bits.
    InsecureMode {
        path: PathBuf,
        mode: u32,
    },
}

impl fmt::Display for RuntimeDirError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAbsolute { path } => write!(
                f,
                "runtime directory base {} is not an absolute path",
                path.display()
            )?,
            Self::Io { op, path, source } => write!(
                f,
                "cannot {op} runtime directory {}: {source}",
                path.display()
            )?,
            Self::Symlink { path } => write!(
                f,
                "runtime directory {} is a symlink; it must be a real directory",
                path.display()
            )?,
            Self::NotADirectory { path } => {
                write!(f, "runtime directory {} is not a directory", path.display())?
            }
            Self::NotOwned {
                path,
                owner,
                current,
            } => write!(
                f,
                "runtime directory {} is owned by uid {owner}, not by the current user (uid {current})",
                path.display()
            )?,
            Self::InsecureMode { path, mode } => write!(
                f,
                "runtime directory {} has mode {mode:04o}; it must be 0700",
                path.display()
            )?,
        }
        write!(
            f,
            " (set {RUNTIME_DIR_ENV} to an absolute directory you own to use another location)"
        )
    }
}

impl std::error::Error for RuntimeDirError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// The runtime directory for this process: `$PRISM_MCP_RUNTIME_DIR` if set and non-empty, else
/// the user runtime directory (Linux `$XDG_RUNTIME_DIR`), else the temp directory (macOS
/// `$TMPDIR`), with `prism-mcpd` joined to whichever base applied. Nothing is created or
/// checked here; call [`validate_runtime_dir`] on the result before every use.
pub fn runtime_dir() -> Result<PathBuf, RuntimeDirError> {
    runtime_dir_from(
        std::env::var_os(RUNTIME_DIR_ENV),
        dirs::runtime_dir(),
        std::env::temp_dir(),
    )
}

/// [`runtime_dir`] with its three inputs explicit.
pub(crate) fn runtime_dir_from(
    override_base: Option<OsString>,
    user_runtime_dir: Option<PathBuf>,
    temp_dir: PathBuf,
) -> Result<PathBuf, RuntimeDirError> {
    let base = match override_base.filter(|value| !value.is_empty()) {
        Some(value) => PathBuf::from(value),
        None => user_runtime_dir.unwrap_or(temp_dir),
    };
    if !base.is_absolute() {
        return Err(RuntimeDirError::NotAbsolute { path: base });
    }
    Ok(base.join(RUNTIME_DIR_NAME))
}

/// What validation needs to know about a path, as `lstat` reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DirFacts {
    pub(crate) is_symlink: bool,
    pub(crate) is_dir: bool,
    pub(crate) owner: u32,
    /// `st_mode`, file-type bits included.
    pub(crate) mode: u32,
}

/// The two filesystem operations validation performs, so tests can inject failures and races.
pub(crate) trait DirAccess {
    /// `lstat`: never follows a symlink at `path`. `Ok(None)` when nothing is there.
    fn inspect(&self, path: &Path) -> io::Result<Option<DirFacts>>;
    /// Create exactly `path`, not its parents, with mode 0700.
    fn create_private(&self, path: &Path) -> io::Result<()>;
}

pub(crate) struct RealDirAccess;

impl DirAccess for RealDirAccess {
    fn inspect(&self, path: &Path) -> io::Result<Option<DirFacts>> {
        match std::fs::symlink_metadata(path) {
            Ok(metadata) => Ok(Some(DirFacts {
                is_symlink: metadata.file_type().is_symlink(),
                is_dir: metadata.is_dir(),
                owner: metadata.uid(),
                mode: metadata.mode(),
            })),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn create_private(&self, path: &Path) -> io::Result<()> {
        std::fs::DirBuilder::new()
            .mode(PRIVATE_DIR_MODE)
            .create(path)
    }
}

/// The effective user id: the owner of files this process creates, and the uid a peer
/// credential check compares against.
pub fn current_uid() -> u32 {
    // SAFETY: `geteuid` takes no arguments, cannot fail and touches no memory.
    unsafe { libc::geteuid() }
}

/// Validate the runtime directory, creating it with mode 0700 if it is absent (its parent must
/// already exist). Call this on every use, not once per process: `Ok` says the directory was a
/// real directory, owned by the current user, mode 0700, at the moment of the call.
pub fn validate_runtime_dir(dir: &Path) -> Result<(), RuntimeDirError> {
    validate_runtime_dir_with(&RealDirAccess, dir, current_uid())
}

pub(crate) fn validate_runtime_dir_with(
    access: &impl DirAccess,
    dir: &Path,
    uid: u32,
) -> Result<(), RuntimeDirError> {
    if !dir.is_absolute() {
        return Err(RuntimeDirError::NotAbsolute {
            path: dir.to_path_buf(),
        });
    }
    // `lstat("dir/")` follows a symlink named `dir`. Rebuilding the path from its components
    // drops a trailing slash, so the final component itself is what gets inspected.
    let dir: PathBuf = dir.components().collect();
    let facts = match inspect(access, &dir)? {
        Some(facts) => facts,
        None => {
            match access.create_private(&dir) {
                Ok(()) => {}
                // Another process created it first; what is there now is checked like any
                // other existing directory.
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(source) => {
                    return Err(RuntimeDirError::Io {
                        op: "create",
                        path: dir,
                        source,
                    })
                }
            }
            // A directory this process just created is inspected like one it found: creating
            // it proves nothing about what is at the path now.
            match inspect(access, &dir)? {
                Some(facts) => facts,
                None => {
                    return Err(RuntimeDirError::Io {
                        op: "inspect",
                        path: dir,
                        source: io::Error::new(
                            io::ErrorKind::NotFound,
                            "the directory disappeared right after it was created",
                        ),
                    })
                }
            }
        }
    };

    if facts.is_symlink {
        return Err(RuntimeDirError::Symlink { path: dir });
    }
    if !facts.is_dir {
        return Err(RuntimeDirError::NotADirectory { path: dir });
    }
    if facts.owner != uid {
        return Err(RuntimeDirError::NotOwned {
            path: dir,
            owner: facts.owner,
            current: uid,
        });
    }
    let permissions = facts.mode & PERMISSION_BITS;
    if permissions != PRIVATE_DIR_MODE {
        return Err(RuntimeDirError::InsecureMode {
            path: dir,
            mode: permissions,
        });
    }
    Ok(())
}

fn inspect(access: &impl DirAccess, dir: &Path) -> Result<Option<DirFacts>, RuntimeDirError> {
    access.inspect(dir).map_err(|source| RuntimeDirError::Io {
        op: "inspect",
        path: dir.to_path_buf(),
        source,
    })
}

#[cfg(test)]
#[path = "runtime_dir_tests.rs"]
mod tests;
