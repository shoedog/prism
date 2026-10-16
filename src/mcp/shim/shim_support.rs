//! Pieces the shim's tests share: a runtime directory of the test's own, keys, preamble lines,
//! and a stand-in for one daemon connection.
//!
//! No test here sleeps to order two events: a fake daemon answers when it has read what it waits
//! for, and a test gives up on a result after [`HANG`].
use crate::mcp::concise_shape::ConciseShapeMode;
use crate::mcp::output::StructuredContentMode;
use crate::mcp::settings::{ClientSettings, ConnectionSettings};
use crate::mcp::shared::identity::IdentityKey;
use crate::mcp::shared::paths::DaemonPaths;
use crate::mcp::shared::preamble::{BuildState, Hello, Ready, Reply};
use crate::mcp::shared::runtime_dir::{current_uid, validate_runtime_dir};
use std::io::Read;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

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
