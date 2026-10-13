//! The effective cache base directory of a server configuration, in one canonical spelling
//! (spec §4.1 step 1, §4.3). The navigation cache treats `--cache-dir` as a base and appends a
//! per-repository suffix, and it creates the directory lazily, so two spellings of one base
//! have to compare equal before anything exists there. Nothing here creates a directory.
use crate::mcp::CacheMode;
use std::fmt;
use std::io;
use std::path::{Component, Path, PathBuf};

#[derive(Debug)]
pub enum CacheBaseError {
    /// `CacheMode::NoCache`: a cache-free server is never shared (spec §4.1 step 2).
    NoCache,
    /// The base cannot be resolved: a component is unreadable, is not a directory, or is a
    /// symlink that leads nowhere. `path` is the component that failed.
    Io { path: PathBuf, source: io::Error },
}

impl fmt::Display for CacheBaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoCache => write!(f, "a server without a cache has no cache base directory"),
            Self::Io { path, source } => write!(
                f,
                "cannot resolve the cache base directory at {}: {source}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for CacheBaseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NoCache => None,
            Self::Io { source, .. } => Some(source),
        }
    }
}

/// The base `CacheMode::Default` caches under. Must stay equal to the default inside
/// `navigation::cache` (`nav_cache_dir`, `NavigationIndex::build_cached`); a test pins it.
pub fn default_cache_base() -> PathBuf {
    dirs::cache_dir().unwrap_or_else(std::env::temp_dir)
}

/// The canonical cache base of `cache` for this process: `Default` and an explicit directory
/// that names the same place give the same path, whether or not the place exists yet.
pub fn effective_cache_base(cache: &CacheMode) -> Result<PathBuf, CacheBaseError> {
    effective_cache_base_in(cache, &default_cache_base(), std::env::current_dir)
}

/// [`effective_cache_base`] with the default base and the working directory explicit.
pub(crate) fn effective_cache_base_in(
    cache: &CacheMode,
    default_base: &Path,
    working_dir: impl FnOnce() -> io::Result<PathBuf>,
) -> Result<PathBuf, CacheBaseError> {
    let base = match cache {
        CacheMode::NoCache => return Err(CacheBaseError::NoCache),
        CacheMode::Default => default_base,
        CacheMode::Dir(base) => base.as_path(),
    };
    if base.is_absolute() {
        return canonical_spelling(base);
    }
    // Only a relative base depends on the working directory, so only then is it read: a
    // process whose working directory has been removed can still name an absolute base.
    let working_dir = working_dir().map_err(|source| CacheBaseError::Io {
        path: base.to_path_buf(),
        source,
    })?;
    canonical_spelling(&working_dir.join(base))
}

/// Resolve `absolute` the way the kernel would, as far as it exists, and keep the rest as
/// written. Every component that exists is resolved through symlinks, so `resolved` is always
/// a real path or a real path plus names that do not exist yet; `..` can therefore drop the
/// last name: the parent of a real directory, or of a directory `create_dir_all` would make.
fn canonical_spelling(absolute: &Path) -> Result<PathBuf, CacheBaseError> {
    let mut resolved = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => resolved.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                resolved.pop();
            }
            Component::Normal(name) => {
                resolved.push(name);
                match std::fs::symlink_metadata(&resolved) {
                    Ok(_) => {
                        resolved = std::fs::canonicalize(&resolved).map_err(|source| {
                            CacheBaseError::Io {
                                path: resolved.clone(),
                                source,
                            }
                        })?;
                    }
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(source) => {
                        return Err(CacheBaseError::Io {
                            path: resolved,
                            source,
                        })
                    }
                }
            }
        }
    }
    Ok(resolved)
}

#[cfg(test)]
#[path = "cache_base_tests.rs"]
mod tests;
