//! A key's files as the shim uses them (spec §4.1 step 6): it takes the start lock, looks at the
//! owner lock, and, only while it holds the owner lock of a key no daemon owns, reads the build
//! marker and removes what a dead daemon left. Every file is opened through the runtime
//! directory's descriptor, with the daemon's own functions (`daemon::files`): lock files are
//! created with `O_EXCL` and then opened (S4's D53), never through a symlink, and never removed.
use crate::mcp::daemon::files::{
    open_private, open_runtime_dir, parse_build_marker, unlink_at, Create, FilesError,
    MarkerContents,
};
use crate::mcp::shared::paths::DaemonPaths;
use std::fs::{File, TryLockError};
use std::io::{self, Read, Write};
use std::os::fd::OwnedFd;
use std::path::Path;

/// Most bytes of a pid file read: three short lines.
const PID_TEXT_BYTES: u64 = 200;
/// Most bytes of a build marker read: three short lines.
const MARKER_BYTES: u64 = 256;

/// What is at a key's build marker path.
pub(super) enum Marker {
    Absent,
    /// Something a daemon did not write there (not a regular file of this user), or a marker
    /// that cannot be opened; why, in words.
    Unusable(String),
    Read(MarkerContents),
}

/// The runtime directory of one key, opened once.
pub(super) struct KeyDir<'a> {
    dir: OwnedFd,
    paths: &'a DaemonPaths,
}

impl<'a> KeyDir<'a> {
    /// Open the runtime directory of `paths`, which the caller has validated.
    pub(super) fn open(paths: &'a DaemonPaths) -> Result<Self, FilesError> {
        Ok(Self {
            dir: open_runtime_dir(paths.runtime_dir())?,
            paths,
        })
    }

    /// The start lock's file (spec §4.1 step 6), not locked yet.
    pub(super) fn start_lock(&self) -> Result<File, FilesError> {
        open_private(
            &self.dir,
            self.paths.start_lock(),
            libc::O_RDWR,
            Create::Yes,
        )
    }

    /// Take the owner lock if no daemon holds it: `Some` holds it until dropped, `None` means a
    /// daemon is alive (starting, serving or draining).
    pub(super) fn try_own(&self) -> Result<Option<File>, FilesError> {
        let owner = open_private(
            &self.dir,
            self.paths.owner_lock(),
            libc::O_RDWR,
            Create::Yes,
        )?;
        match owner.try_lock() {
            Ok(()) => Ok(Some(owner)),
            Err(TryLockError::WouldBlock) => Ok(None),
            Err(TryLockError::Error(source)) => Err(FilesError::Io {
                op: "lock",
                path: self.paths.owner_lock().to_path_buf(),
                source,
            }),
        }
    }

    /// What the build marker holds, when there is one that a daemon could have written (a
    /// regular file of this user). Call it holding the owner lock: then the marker is a dead
    /// daemon's. Anything else at its path is no marker, and the stale files' removal removes it.
    pub(super) fn build_marker(&self) -> Option<MarkerContents> {
        match self.read_build_marker() {
            Marker::Read(marker) => Some(marker),
            Marker::Absent | Marker::Unusable(_) => None,
        }
    }

    /// Read the build marker without waiting and only so far (S5b review round 1, sol 2 and
    /// Opus S6): through the directory's descriptor, never through a symlink, without waiting for
    /// a writer (`O_NONBLOCK`: a FIFO in its place would otherwise hold the caller, and the locks
    /// it holds, for ever), only a regular file of this user, and at most [`MARKER_BYTES`] of it,
    /// far more than the three short lines a daemon and a shim write.
    pub(super) fn read_build_marker(&self) -> Marker {
        let path = self.paths.build_marker();
        match open_private(
            &self.dir,
            path,
            libc::O_RDONLY | libc::O_NONBLOCK,
            Create::No,
        ) {
            Err(FilesError::Io { source, .. }) if source.kind() == io::ErrorKind::NotFound => {
                Marker::Absent
            }
            Err(error) => Marker::Unusable(error.to_string()),
            Ok(file) => {
                let mut text = String::new();
                // A marker that cannot be read whole is still a dead build's: its fields are
                // unknown.
                let _ = file.take(MARKER_BYTES).read_to_string(&mut text);
                Marker::Read(parse_build_marker(&text))
            }
        }
    }

    /// The pid file's text, at most [`PID_TEXT_BYTES`] of it, when there is one: read through the
    /// directory's descriptor, never through a symlink, without waiting for a writer, and only
    /// from a regular file of this user. The daemon rewrites it in place, so it can be empty or
    /// partial (S4's interface for S5a).
    pub(super) fn pid_text(&self) -> Option<String> {
        let file = open_private(
            &self.dir,
            self.paths.pid_file(),
            libc::O_RDONLY | libc::O_NONBLOCK,
            Create::No,
        )
        .ok()?;
        let mut text = String::new();
        file.take(PID_TEXT_BYTES).read_to_string(&mut text).ok()?;
        Some(text)
    }

    /// Record in the build marker that a shim first saw its daemon dead at `now` (seconds since
    /// the Unix epoch), keeping the pid and the build's start (an empty line when unknown, which
    /// reads back as unknown, so no later shim names a pid 0). Call it holding the start and
    /// owner locks, and only for a marker with no sighting yet.
    pub(super) fn record_sighting(
        &self,
        marker: &MarkerContents,
        now: u64,
    ) -> Result<(), FilesError> {
        let path = self.paths.build_marker();
        let mut file = open_private(&self.dir, path, libc::O_WRONLY | libc::O_TRUNC, Create::Yes)?;
        let known = |field: Option<u64>| field.map(|value| value.to_string()).unwrap_or_default();
        let text = format!(
            "{}\n{}\n{now}\n",
            known(marker.pid.map(u64::from)),
            known(marker.started)
        );
        file.write_all(text.as_bytes())
            .map_err(|source| FilesError::Io {
                op: "write",
                path: path.to_path_buf(),
                source,
            })
    }

    /// The key's log, opened for appending, for the daemon this shim starts (its standard output
    /// and standard error). Appending matters: the daemon that wins the key truncates its log
    /// when it is over 1 MiB (S4's SI-16), and a descriptor without `O_APPEND` would then go on
    /// writing at its old offset, leaving a hole, or overwrite the start of a shorter log.
    pub(super) fn open_log(&self) -> Result<File, FilesError> {
        open_private(
            &self.dir,
            self.paths.log_file(),
            libc::O_WRONLY | libc::O_APPEND,
            Create::Yes,
        )
    }

    /// Remove what a daemon that died left: its socket, pid file and build marker (spec §4.1
    /// step 6: "Holding it, unlink any stale socket and pid file"). Call it holding the owner
    /// lock, so that a daemon about to bind can never have its live socket removed.
    pub(super) fn remove_stale(&self) -> Result<(), FilesError> {
        for path in [
            self.paths.socket(),
            self.paths.pid_file(),
            self.paths.build_marker(),
        ] {
            remove(&self.dir, path)?;
        }
        Ok(())
    }
}

fn remove(dir: &OwnedFd, path: &Path) -> Result<(), FilesError> {
    match unlink_at(dir, path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(FilesError::Io {
            op: "remove",
            path: path.to_path_buf(),
            source,
        }),
    }
}
