//! The per-user runtime directory that holds every daemon's socket, locks, pid file and log
//! (spec §4.3), and the validation that the shim and the daemon both run on every use (§7).
//!
//! Validation hardens the directory against other local users. It has three parts:
//!
//! - the directory itself is a real directory (not a symlink), owned by the current user, with
//!   mode 0700;
//! - on macOS it carries no access control list, because a list can grant another user what
//!   the mode denies (on Linux a mode of 0700 masks every list entry, so the mode is enough);
//! - every directory its resolution passes through, symlinks included, belongs to this user
//!   or root, is either not writable by anyone else or sticky, and on macOS has no list entry
//!   that allows anything (`dir_chain`). Without this the first two would say who can use the
//!   directory that is there now, not who can put another one in its place.
//!
//! This is hardening, not the isolation guarantee. It covers what owner, mode bits and lists
//! express; it does not examine mount options, and it is checked by path, not through a held
//! descriptor (`bind` and `connect` take paths). What keeps a client from talking to another
//! user's process is the check of the socket peer's uid, which the daemon makes on every
//! connection (S4) and the shim must make too (S5a). None of it is a defense against a
//! process of the same user (§2) or against root.
use super::acl::{list_kind, ListKind};
use super::dir_chain::{check_location, check_resolution_chain, Exposure};
use std::ffi::OsString;
use std::fmt;
use std::io;
use std::os::unix::fs::{DirBuilderExt, MetadataExt};
use std::path::{Path, PathBuf};

/// Environment variable that replaces the base directory. Every launcher that should share a
/// daemon must resolve the same value (spec §4.3). The base and every directory above it must
/// be owned by this user or root and be writable by nobody else, or be sticky, and on macOS
/// carry no list entry that allows anything; validation refuses a base that is not, because
/// another user could swap the validated directory for their own.
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
    /// The filesystem refused: `op` is `"inspect"` (an `lstat`, a `readlink` or reading the
    /// access control list, of the directory or of a step on the way to it) or `"create"` (the
    /// `mkdir`). `path` is what the operation was applied to.
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
    /// The directory carries an access control list (macOS), which can grant another user
    /// what mode 0700 denies.
    HasAcl {
        path: PathBuf,
    },
    /// Another user could replace the directory or something on the way to it: `container`
    /// does not protect its entry `entry`.
    Replaceable {
        path: PathBuf,
        container: PathBuf,
        entry: PathBuf,
        exposure: Exposure,
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
            Self::HasAcl { path } => write!(
                f,
                "runtime directory {} has an access control list; a list can let other users in \
                 whatever the mode says, so none is accepted here; remove it (chmod -N)",
                path.display()
            )?,
            Self::Replaceable {
                path,
                container,
                entry,
                exposure,
            } => {
                write!(
                    f,
                    "runtime directory {} could be replaced by another user: {} ",
                    path.display(),
                    container.display()
                )?;
                match exposure {
                    Exposure::ForeignOwner { owner } => write!(f, "is owned by uid {owner}")?,
                    Exposure::WritableByOthers { mode } => write!(
                        f,
                        "has mode {mode:04o}, writable by others and not sticky"
                    )?,
                    Exposure::ListAllows => write!(
                        f,
                        "has an access control list with an allow entry, which can let other \
                         users add or remove what is in it whatever its mode"
                    )?,
                    Exposure::ForeignEntryInSticky { entry_owner } => write!(
                        f,
                        "is sticky and {} in it is owned by uid {entry_owner}",
                        entry.display()
                    )?,
                }
            }
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
/// already exist). Call this on every use, not once per process: `Ok` says that at the moment
/// of the call the directory was a real directory, owned by the current user, mode 0700, with
/// no access control list (macOS), and that every directory its path resolves through was
/// owned by this user or root, not writable by anyone else unless sticky, and without a list
/// entry that allows anything (macOS).
///
/// That refuses the ways another user can alter or replace the directory that owner, mode
/// bits and lists can express. It does not examine mount options (a volume mounted to ignore
/// ownership reports every file as the caller's), and it is a check by path. It is hardening:
/// what keeps another user's process from being talked to is the check of the socket peer's
/// uid, by the daemon and by the shim.
pub fn validate_runtime_dir(dir: &Path) -> Result<(), RuntimeDirError> {
    validate_runtime_dir_stepwise(dir, current_uid(), |_| {})
}

/// Points between the steps of validation, for a test to change the directory at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    /// The directory exists and its type, owner and mode have been accepted once; its list
    /// has not been read yet.
    BeforeListCheck,
}

pub(crate) fn validate_runtime_dir_stepwise(
    dir: &Path,
    uid: u32,
    between: impl Fn(Step),
) -> Result<(), RuntimeDirError> {
    if !dir.is_absolute() {
        return Err(RuntimeDirError::NotAbsolute {
            path: dir.to_path_buf(),
        });
    }
    // The spelling every check below inspects (no trailing slash).
    let dir: PathBuf = dir.components().collect();
    // First the place the directory is, or would be created: nothing is written to a location
    // another user could swap out.
    check_location(&dir, uid)?;
    validate_runtime_dir_with(&RealDirAccess, &dir, uid)?;
    between(Step::BeforeListCheck);
    let list = list_kind(&dir).map_err(|source| RuntimeDirError::Io {
        op: "inspect",
        path: dir.clone(),
        source,
    })?;
    if list != ListKind::None {
        return Err(RuntimeDirError::HasAcl { path: dir });
    }
    // The type, owner and mode again, now that there is known to be no list. A list entry can
    // let its grantee change the mode and then remove the list; facts read before the list was
    // found absent could be from before such a change.
    validate_runtime_dir_with(&RealDirAccess, &dir, uid)?;
    // Then the last step of the resolution: the directory itself in its parent.
    check_resolution_chain(&dir, uid)
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
