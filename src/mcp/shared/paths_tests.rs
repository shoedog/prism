//! Tests for the daemon's file names and the socket path length limit.
use super::*;
use crate::mcp::shared::identity::IdentityKey;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

const KEY: &str = "0123456789abcdeffedcba98765432100123456789abcdeffedcba9876543210";

fn key(hex: &str) -> IdentityKey {
    IdentityKey::from_hex(hex).unwrap()
}

/// An absolute directory path of exactly `len` bytes (it need not exist).
fn dir_of_len(len: usize) -> PathBuf {
    let path = PathBuf::from(format!("/{}", "d".repeat(len - 1)));
    assert_eq!(path.as_os_str().len(), len);
    path
}

/// `/` + 16 hex digits + `.sock`.
const SOCKET_NAME_BYTES: usize = 1 + 16 + 5;

/// Spec §4.3 "Files".
#[test]
fn the_five_files_are_named_by_the_first_16_hex_digits_inside_the_runtime_dir() {
    let dir = Path::new("/run/user/501/prism-mcpd");
    let paths = DaemonPaths::new(dir, &key(KEY)).unwrap();
    assert_eq!(paths.runtime_dir(), dir);
    assert_eq!(paths.socket(), dir.join("0123456789abcdef.sock"));
    assert_eq!(paths.start_lock(), dir.join("0123456789abcdef.start"));
    assert_eq!(paths.owner_lock(), dir.join("0123456789abcdef.owner"));
    assert_eq!(paths.pid_file(), dir.join("0123456789abcdef.pid"));
    assert_eq!(paths.log_file(), dir.join("0123456789abcdef.log"));
}

/// The file names are a locator, not the identity: two keys that agree in their first 16
/// digits name the same files, which is why the preamble compares the full key (spec §4.4).
#[test]
fn keys_that_share_their_first_16_digits_share_their_files() {
    let dir = Path::new("/run/user/501/prism-mcpd");
    let other = format!("{}{}", &KEY[..16], "0".repeat(48));
    assert_ne!(key(KEY), key(&other));
    assert_eq!(
        DaemonPaths::new(dir, &key(KEY)).unwrap(),
        DaemonPaths::new(dir, &key(&other)).unwrap()
    );

    let different = format!("f{}", &KEY[1..]);
    assert_ne!(
        DaemonPaths::new(dir, &key(KEY)).unwrap().socket(),
        DaemonPaths::new(dir, &key(&different)).unwrap().socket()
    );
}

/// Spec §4.3: the socket path must be at most 103 bytes. Rules out an off-by-one in either
/// direction.
#[test]
fn a_socket_path_of_103_bytes_fits_and_104_does_not() {
    let fits = DaemonPaths::new(&dir_of_len(103 - SOCKET_NAME_BYTES), &key(KEY)).unwrap();
    assert_eq!(fits.socket().as_os_str().len(), 103);

    let dir = dir_of_len(104 - SOCKET_NAME_BYTES);
    let outcome = DaemonPaths::new(&dir, &key(KEY));
    match outcome {
        Err(SocketPathTooLong { path, len }) => {
            assert_eq!(len, 104);
            assert_eq!(path, dir.join("0123456789abcdef.sock"));
        }
        other => panic!("{other:?}"),
    }
}

/// Rules out counting characters: this path is 97 characters and 111 bytes.
#[test]
fn the_limit_counts_bytes_not_characters() {
    let dir = PathBuf::from(format!("/{}", "é".repeat(14) + &"d".repeat(60)));
    let socket_chars = dir.to_str().unwrap().chars().count() + SOCKET_NAME_BYTES;
    assert!(socket_chars <= 103, "{socket_chars} characters");
    let outcome = DaemonPaths::new(&dir, &key(KEY));
    assert!(
        matches!(&outcome, Err(SocketPathTooLong { len, .. }) if *len == socket_chars + 14),
        "{outcome:?}"
    );
}

/// The limit is the kernel's, not a preference: a path of exactly 103 bytes can be bound on
/// every supported platform (`sun_path` holds 104 bytes on Darwin, 108 on Linux, including
/// the terminating NUL). The directory is made under `/tmp` so its length does not depend on
/// `$TMPDIR`.
#[test]
fn a_socket_can_really_be_bound_at_the_limit() {
    let denied = |error: &std::io::Error| {
        matches!(
            error.raw_os_error(),
            Some(code) if code == libc::EACCES || code == libc::EPERM
        )
    };
    let base = match tempfile::Builder::new().prefix("s2").tempdir_in("/tmp") {
        Ok(base) => base,
        Err(error) if denied(&error) => {
            eprintln!("skipped: this environment does not allow a directory under /tmp ({error})");
            return;
        }
        Err(error) => panic!("tempdir under /tmp: {error}"),
    };
    let base_len = base.path().as_os_str().as_bytes().len();
    let padding = 103 - SOCKET_NAME_BYTES - base_len - 1;
    let dir = base.path().join("p".repeat(padding));
    std::fs::create_dir(&dir).unwrap();

    let paths = DaemonPaths::new(&dir, &key(KEY)).unwrap();
    assert_eq!(paths.socket().as_os_str().as_bytes().len(), 103);
    match std::os::unix::net::UnixListener::bind(paths.socket()) {
        Ok(listener) => drop(listener),
        Err(error) if denied(&error) => {
            eprintln!("skipped: this environment does not allow binding a socket ({error})")
        }
        Err(error) => panic!("bind at 103 bytes: {error}"),
    }
}

#[test]
fn the_error_states_the_length_the_limit_and_the_way_out() {
    let error = DaemonPaths::new(&dir_of_len(120), &key(KEY)).unwrap_err();
    let text = error.to_string();
    assert!(text.contains("142 bytes"), "{text}");
    assert!(text.contains("103"), "{text}");
    assert!(text.contains("0123456789abcdef.sock"), "{text}");
    assert!(text.contains("PRISM_MCP_RUNTIME_DIR"), "{text}");
    assert!(!text.contains('\n'), "one line: {text}");
}
