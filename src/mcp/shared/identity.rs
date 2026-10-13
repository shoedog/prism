//! The identity of a shared daemon: everything that changes the index content or the analyzer's
//! answers (spec §4.3). Its SHA-256 key names the daemon's socket, locks, pid file and log, and
//! is compared in full in the preamble (§4.4), so one daemon never answers for another
//! checkout, another analyzer build, another cache or another refresh policy.
//!
//! Connection-local settings (output shape, result cap, `--first-call-wait`,
//! `--warm-at-startup`) are deliberately not part of it; they travel in the preamble.
use super::cache_base::{effective_cache_base, CacheBaseError};
use super::SHARED_PROTOCOL_VERSION;
use crate::mcp::{RefreshPolicy, ServerConfig};
use sha2::{Digest, Sha256};
use std::fmt;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

/// First field of the encoding, so the digest cannot be mistaken for a hash of something else.
const ENCODING_DOMAIN: &[u8] = b"prism-mcpd identity";

/// The build of the analyzer, as far as it can change an answer or an on-disk cache.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildIdentity {
    /// `CARGO_PKG_VERSION`.
    pub package_version: String,
    /// `GRAMMAR_FINGERPRINT`: the tree-sitter grammar versions.
    pub grammar_fingerprint: String,
    /// `PRISM_CACHE_BUILD_IDENTITY`: the analyzer build identity the CPG cache checks.
    pub cache_build_identity: String,
}

impl BuildIdentity {
    /// The build of the running binary.
    pub fn current() -> Self {
        let info = crate::api::build_info();
        Self {
            package_version: info.package_version.to_string(),
            grammar_fingerprint: info.grammar_fingerprint.to_string(),
            cache_build_identity: info.build_identity.to_string(),
        }
    }
}

/// Which directory a pathname names right now: the mounted device number and the inode. A
/// directory that is renamed away and replaced keeps its pathname and changes this.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FsIdentity {
    pub dev: u64,
    pub ino: u64,
}

impl FsIdentity {
    /// The identity of what `path` names now. An error means there is nothing usable there.
    pub fn of(path: &Path) -> io::Result<Self> {
        let metadata = std::fs::metadata(path)?;
        Ok(Self {
            dev: metadata.dev(),
            ino: metadata.ino(),
        })
    }
}

/// The name a refresh policy has on the command line (`--refresh-policy`); also its spelling
/// in the key.
pub fn refresh_policy_name(policy: RefreshPolicy) -> &'static str {
    match policy {
        RefreshPolicy::WarnOnly => "warn-only",
        RefreshPolicy::AutoFull => "auto-full",
        RefreshPolicy::AutoIncremental => "auto-incremental",
    }
}

#[derive(Debug)]
pub enum IdentityError {
    /// The repository root cannot be resolved or inspected.
    RepoRoot {
        path: PathBuf,
        source: io::Error,
    },
    CacheBase(CacheBaseError),
}

impl fmt::Display for IdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RepoRoot { path, source } => write!(
                f,
                "cannot resolve the repository root {}: {source}",
                path.display()
            ),
            Self::CacheBase(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for IdentityError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::RepoRoot { source, .. } => Some(source),
            Self::CacheBase(error) => Some(error),
        }
    }
}

impl From<CacheBaseError> for IdentityError {
    fn from(error: CacheBaseError) -> Self {
        Self::CacheBase(error)
    }
}

/// Everything one daemon is for. Two configurations share a daemon exactly when these are
/// equal; the fields are private so that an identity always comes from [`Self::resolve`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DaemonIdentity {
    repo_root: PathBuf,
    root_fs: FsIdentity,
    build: BuildIdentity,
    protocol: u32,
    cache_base: PathBuf,
    refresh_policy: RefreshPolicy,
}

impl DaemonIdentity {
    /// The identity `cfg` has for the running binary. Reads the filesystem and creates nothing.
    /// Only `repo_root`, `cache` and `refresh_policy` are read; `startup` and
    /// `first_call_wait` are connection-local and do not take part.
    pub fn resolve(cfg: &ServerConfig) -> Result<Self, IdentityError> {
        Self::resolve_with(cfg, BuildIdentity::current(), SHARED_PROTOCOL_VERSION)
    }

    /// [`Self::resolve`] for a given build and protocol version (what a different binary would
    /// compute for the same configuration).
    pub fn resolve_with(
        cfg: &ServerConfig,
        build: BuildIdentity,
        protocol: u32,
    ) -> Result<Self, IdentityError> {
        let root_error = |source| IdentityError::RepoRoot {
            path: cfg.repo_root.clone(),
            source,
        };
        // The same canonicalization `canonical_config` applies for the in-process server.
        let repo_root = std::fs::canonicalize(&cfg.repo_root).map_err(root_error)?;
        let root_fs = FsIdentity::of(&repo_root).map_err(root_error)?;
        let cache_base = effective_cache_base(&cfg.cache)?;
        Ok(Self {
            repo_root,
            root_fs,
            build,
            protocol,
            cache_base,
            refresh_policy: cfg.refresh_policy,
        })
    }

    /// The canonical repository root.
    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    /// The root's device and inode when the identity was resolved. A daemon compares this with
    /// [`FsIdentity::of`] of the same path to notice that its root was lost or replaced.
    pub fn root_fs(&self) -> FsIdentity {
        self.root_fs
    }

    pub fn build(&self) -> &BuildIdentity {
        &self.build
    }

    pub fn protocol(&self) -> u32 {
        self.protocol
    }

    /// The canonical cache base directory (the value for the daemon's `--cache-dir`).
    pub fn cache_base(&self) -> &Path {
        &self.cache_base
    }

    pub fn refresh_policy(&self) -> RefreshPolicy {
        self.refresh_policy
    }

    /// The identity key: SHA-256 of [`Self::encode`].
    pub fn key(&self) -> IdentityKey {
        IdentityKey(Sha256::digest(self.encode()).into())
    }

    /// The lossless encoding the key is a digest of: ten fields in a fixed order, each written
    /// as its length (8 bytes, little-endian) followed by its bytes. Paths are their raw bytes;
    /// the device, inode and protocol numbers are fixed-width little-endian.
    fn encode(&self) -> Vec<u8> {
        let mut encoded = Vec::new();
        let mut field = |bytes: &[u8]| {
            encoded.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
            encoded.extend_from_slice(bytes);
        };
        field(ENCODING_DOMAIN);
        field(self.repo_root.as_os_str().as_bytes());
        field(&self.root_fs.dev.to_le_bytes());
        field(&self.root_fs.ino.to_le_bytes());
        field(self.build.package_version.as_bytes());
        field(self.build.grammar_fingerprint.as_bytes());
        field(self.build.cache_build_identity.as_bytes());
        field(&self.protocol.to_le_bytes());
        field(self.cache_base.as_os_str().as_bytes());
        field(refresh_policy_name(self.refresh_policy).as_bytes());
        encoded
    }
}

/// The 256-bit identity key. Its text form is 64 lowercase hexadecimal digits; the first 16
/// name the daemon's files. The 16 digits locate a daemon; only the full key proves it is the
/// right one (spec §4.4).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IdentityKey([u8; 32]);

/// Text that is not exactly 64 lowercase hexadecimal digits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidIdentityKey;

impl fmt::Display for InvalidIdentityKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "an identity key is 64 lowercase hexadecimal digits")
    }
}

impl std::error::Error for InvalidIdentityKey {}

const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

fn hex_value(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        _ => None,
    }
}

impl IdentityKey {
    /// All 64 hexadecimal digits.
    pub fn to_hex(&self) -> String {
        let mut hex = String::with_capacity(64);
        for byte in self.0 {
            hex.push(HEX_DIGITS[usize::from(byte >> 4)] as char);
            hex.push(HEX_DIGITS[usize::from(byte & 0x0f)] as char);
        }
        hex
    }

    /// The first 16 hexadecimal digits: the stem of the daemon's file names (`key16`).
    pub fn locator(&self) -> String {
        let mut hex = self.to_hex();
        hex.truncate(16);
        hex
    }

    /// Parse the text form. Accepts exactly what [`Self::to_hex`] produces.
    pub fn from_hex(text: &str) -> Result<Self, InvalidIdentityKey> {
        let digits = text.as_bytes();
        if digits.len() != 64 {
            return Err(InvalidIdentityKey);
        }
        let mut key = [0u8; 32];
        for (byte, pair) in key.iter_mut().zip(digits.chunks_exact(2)) {
            let high = hex_value(pair[0]).ok_or(InvalidIdentityKey)?;
            let low = hex_value(pair[1]).ok_or(InvalidIdentityKey)?;
            *byte = high << 4 | low;
        }
        Ok(Self(key))
    }
}

impl fmt::Display for IdentityKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl fmt::Debug for IdentityKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IdentityKey({})", self.to_hex())
    }
}

#[cfg(test)]
#[path = "identity_tests.rs"]
mod tests;
