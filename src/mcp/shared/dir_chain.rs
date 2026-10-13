//! Whether another user could replace the runtime directory, or anything on the way to it.
//!
//! Checking the directory itself (owner, mode, not a symlink) says who can use what is there
//! now. It does not say who can put something else there: whoever may rename entries in a
//! directory on the path can move the validated directory aside and create their own in its
//! place, between the check and the `bind` or `connect` that follows, and no number of repeated
//! checks closes that. So the whole resolution is walked, symlinks included, and every
//! directory that holds a step of it must be one where only this user and root can rename
//! that step.
//!
//! Access control lists on those directories are not examined. Each of them is owned by this
//! user or by root, so a list that lets someone else in was put there by one of the two.
use super::runtime_dir::{DirFacts, RuntimeDirError};
use std::collections::VecDeque;
use std::ffi::OsString;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

/// More symlinks than this in one resolution is treated as a loop (the kernel's own limit is
/// 32 on Darwin and 40 on Linux).
const MAX_SYMLINK_HOPS: usize = 40;

const GROUP_OR_OTHER_WRITE: u32 = 0o022;
const STICKY: u32 = 0o1000;
const ROOT: u32 = 0;

/// Why a directory does not protect one of its entries from another user.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exposure {
    /// The directory belongs to someone other than this user or root; its owner can rename
    /// anything in it.
    ForeignOwner { owner: u32 },
    /// The directory is writable by its group or by everyone and is not sticky, so those
    /// users can rename anything in it.
    WritableByOthers { mode: u32 },
    /// The directory is sticky, which lets only an entry's owner rename it, and this entry
    /// belongs to someone else.
    ForeignEntryInSticky { entry_owner: u32 },
}

/// Whether `container` lets a user other than `uid` or root rename an entry owned by
/// `entry_owner`. `None` when it does not.
pub(crate) fn exposure(container: DirFacts, entry_owner: u32, uid: u32) -> Option<Exposure> {
    let trusted = |owner: u32| owner == uid || owner == ROOT;
    if !trusted(container.owner) {
        return Some(Exposure::ForeignOwner {
            owner: container.owner,
        });
    }
    if container.mode & GROUP_OR_OTHER_WRITE == 0 {
        return None;
    }
    if container.mode & STICKY == 0 {
        return Some(Exposure::WritableByOthers {
            mode: container.mode & 0o7777,
        });
    }
    if trusted(entry_owner) {
        None
    } else {
        Some(Exposure::ForeignEntryInSticky { entry_owner })
    }
}

fn facts(path: &Path) -> io::Result<DirFacts> {
    let metadata = std::fs::symlink_metadata(path)?;
    Ok(DirFacts {
        is_symlink: metadata.file_type().is_symlink(),
        is_dir: metadata.is_dir(),
        owner: metadata.uid(),
        mode: metadata.mode(),
    })
}

fn steps_of(path: &Path) -> impl Iterator<Item = OsString> + '_ {
    path.components().filter_map(|component| match component {
        Component::Normal(name) => Some(name.to_os_string()),
        Component::ParentDir => Some(OsString::from("..")),
        Component::Prefix(_) | Component::RootDir | Component::CurDir => None,
    })
}

/// Walk the resolution of `dir` (absolute, and existing) from the root and refuse it if any
/// directory on the way lets another user rename the step taken through it.
pub(crate) fn check_resolution_chain(dir: &Path, uid: u32) -> Result<(), RuntimeDirError> {
    let inspect_error = |path: &Path, source: io::Error| RuntimeDirError::Io {
        op: "inspect",
        path: path.to_path_buf(),
        source,
    };
    let mut pending: VecDeque<OsString> = steps_of(dir).collect();
    let mut current = PathBuf::from("/");
    let mut hops = 0;
    while let Some(name) = pending.pop_front() {
        if name == ".." {
            current.pop();
            continue;
        }
        let container = facts(&current).map_err(|source| inspect_error(&current, source))?;
        let entry_path = current.join(&name);
        let entry = facts(&entry_path).map_err(|source| inspect_error(&entry_path, source))?;
        if let Some(exposure) = exposure(container, entry.owner, uid) {
            return Err(RuntimeDirError::Replaceable {
                path: dir.to_path_buf(),
                container: current,
                entry: entry_path,
                exposure,
            });
        }
        if !entry.is_symlink {
            current = entry_path;
            continue;
        }
        hops += 1;
        if hops > MAX_SYMLINK_HOPS {
            return Err(inspect_error(
                &entry_path,
                io::Error::from_raw_os_error(libc::ELOOP),
            ));
        }
        let target =
            std::fs::read_link(&entry_path).map_err(|source| inspect_error(&entry_path, source))?;
        if target.is_absolute() {
            current = PathBuf::from("/");
        }
        for step in steps_of(&target).collect::<Vec<_>>().into_iter().rev() {
            pending.push_front(step);
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "dir_chain_tests.rs"]
mod tests;
