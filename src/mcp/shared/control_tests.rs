//! Tests for the operator's control exchange on the wire (spec §4.7): one spelling per message,
//! a version of its own, and a status line that fits the preamble's limit whatever the root is.
use super::*;
use crate::mcp::shared::preamble::{BuildState, PREAMBLE_MAX_BYTES};
use crate::mcp::shared::SHARED_PROTOCOL_VERSION;

fn key() -> IdentityKey {
    IdentityKey::from_hex(&"ab".repeat(32)).expect("a key")
}

fn instance() -> Instance {
    Instance {
        identity: key(),
        pid: 4242,
        start_time: 1_760_600_000,
    }
}

fn status() -> DaemonStatus {
    DaemonStatus {
        instance: instance(),
        build_identity: "f".repeat(64),
        repo_root: "/work/repo".to_string(),
        admitted: 2,
        pending: 1,
        max_clients: 64,
        build: BuildState::Ready,
        index_idle_timeout_secs: 0,
        index_idle_pressure_relief: false,
        attempt: 1,
        generation: 3,
        idle_secs: None,
        idle_ttl_secs: 900,
        shutdown: false,
    }
}

/// The line without its newline, as a reader returns it.
fn line(encoded: &str) -> Vec<u8> {
    encoded
        .strip_suffix('\n')
        .expect("an encoded line ends in a newline")
        .as_bytes()
        .to_vec()
}

/// The exact text of each request: what a daemon of another build must be able to read.
#[test]
fn requests_have_one_spelling_and_round_trip() {
    assert_eq!(ControlRequest::Status.encode(), "STATUS {\"control\":2}\n");
    let shutdown = ControlRequest::Shutdown(instance());
    assert_eq!(
        shutdown.encode(),
        format!(
            "SHUTDOWN {{\"control\":2,\"identity\":\"{}\",\"pid\":4242,\"start_time\":1760600000}}\n",
            "ab".repeat(32)
        )
    );
    for request in [ControlRequest::Status, shutdown] {
        assert_eq!(
            ControlRequest::decode(&line(&request.encode())),
            Ok(request)
        );
    }
}

/// A request is refused unless it is spelled as `encode` spells it: another field order,
/// spaces, an unknown or a missing field, a malformed key. Rules out a lenient decoder, which
/// would let two spellings of one request exist.
#[test]
fn a_request_spelled_otherwise_is_refused() {
    let key = "ab".repeat(32);
    let refused = [
        "STATUS {\"control\": 2}".to_string(),
        "STATUS {}".to_string(),
        "STATUS {\"control\":2,\"extra\":0}".to_string(),
        format!("SHUTDOWN {{\"control\":2,\"pid\":4242,\"identity\":\"{key}\",\"start_time\":1}}"),
        format!("SHUTDOWN {{\"control\":2,\"identity\":\"{key}\",\"pid\":4242}}"),
        "SHUTDOWN {\"control\":2,\"identity\":\"AB\",\"pid\":1,\"start_time\":1}".to_string(),
        "STATUS [1]".to_string(),
    ];
    for text in refused {
        if text == "STATUS {\"control\": 2}" {
            assert!(
                matches!(
                    ControlRequest::decode(text.as_bytes()),
                    Err(PreambleError::Invalid { detail }) if detail.contains("not spelled")
                ),
                "whitespace fixture must exercise canonical spelling, not version rejection"
            );
        }
        assert!(
            ControlRequest::decode(text.as_bytes()).is_err(),
            "accepted: {text}"
        );
    }
}

/// Another version of the control exchange is refused as such, before its payload is read, so
/// a daemon can say why in its log. Rules out reading a future request as this version's.
#[test]
fn another_control_version_is_refused_as_unsupported() {
    assert_eq!(
        ControlRequest::decode(b"STATUS {\"control\":3}"),
        Err(PreambleError::UnsupportedControl { got: 3 })
    );
    assert_eq!(
        ControlRequest::decode(b"SHUTDOWN {\"control\":3,\"anything\":true}"),
        Err(PreambleError::UnsupportedControl { got: 3 })
    );
    // The daemon logs this error: it names the control exchange's version, not the preamble's
    // protocol version (review of #416, Opus S1).
    assert_eq!(
        PreambleError::UnsupportedControl { got: 3 }.to_string(),
        format!("control exchange version 3 is not {CONTROL_VERSION}")
    );
}

/// The control exchange has a version of its own: changing the preamble's protocol version
/// (which is part of every identity key) would not change these lines (D97). Rules out a
/// control line that carries the preamble's version.
#[test]
fn the_control_version_is_not_the_preamble_s() {
    assert!(ControlRequest::Status
        .encode()
        .contains(&format!("\"control\":{CONTROL_VERSION}")));
    assert!(!ControlRequest::Status.encode().contains("\"protocol\""));
    assert_eq!(CONTROL_VERSION, 2);
    assert_eq!(
        SHARED_PROTOCOL_VERSION, 2,
        "the preamble deliberately moved to v2 with control"
    );
}

/// Only a line whose verb is `STATUS` or `SHUTDOWN` is a control line; a `HELLO` and anything
/// that merely starts with those letters are not. Rules out taking a shim's `HELLO` down the
/// control path, or a verb with a longer name for a control verb.
#[test]
fn only_the_two_verbs_are_control_lines() {
    assert!(is_control_line(b"STATUS {\"control\":2}"));
    assert!(is_control_line(b"SHUTDOWN {}"));
    assert!(is_control_line(b"STATUS x"));
    assert!(!is_control_line(b"HELLO {\"protocol\":2}"));
    assert!(!is_control_line(b"STATUSES {}"));
    assert!(!is_control_line(b"STATUS"));
    assert!(!is_control_line(b"status {}"));
    assert!(!is_control_line(b""));
}

/// The status reply round-trips with every field, an idle time present or absent included.
#[test]
fn a_status_reply_round_trips() {
    let busy = status();
    let idle = DaemonStatus {
        admitted: 0,
        pending: 0,
        idle_secs: Some(12),
        shutdown: true,
        build: BuildState::Building,
        index_idle_timeout_secs: 0,
        index_idle_pressure_relief: false,
        ..status()
    };
    for reply in [busy, idle] {
        let encoded = reply.encode();
        assert!(encoded.starts_with("STATUS {\"control\":2,"), "{encoded}");
        assert_eq!(DaemonStatus::decode(&line(&encoded)), Ok(reply));
    }
}

/// A status reply spelled otherwise is refused too. Rules out a decoder that drops the
/// canonical check (a reply with a missing idle time would read as "in use").
#[test]
fn a_status_reply_spelled_otherwise_is_refused() {
    let encoded = status().encode();
    let without_idle = encoded.replace(",\"idle_secs\":null", "");
    assert_ne!(without_idle, encoded);
    assert!(DaemonStatus::decode(&line(&without_idle)).is_err());
    let spaced = encoded.replace("\"pid\":4242", "\"pid\": 4242");
    assert!(DaemonStatus::decode(&line(&spaced)).is_err());
    assert!(DaemonStatus::decode(b"STATUS {\"control\":3}").is_err());
}

/// A root that holds quotes, backslashes, control characters and non-ASCII text comes back as
/// it went: the reply is JSON, not a format string.
#[test]
fn a_root_with_any_characters_round_trips() {
    let reply = DaemonStatus {
        repo_root: "/w/\"q\"\\b\n\u{1b}[31m\u{e9}t\u{e9}".to_string(),
        ..status()
    };
    let encoded = reply.encode();
    assert_eq!(encoded.matches('\n').count(), 1, "one line: {encoded:?}");
    assert_eq!(DaemonStatus::decode(&line(&encoded)), Ok(reply));
}

/// The root a daemon reports is clipped to [`MAX_ROOT_CHARS`] characters, so the reply fits the
/// preamble's 4 KiB line even when every character is one JSON writes as six bytes. Rules out
/// an unclipped root, whose reply the operator's reader refuses as too long.
#[test]
fn a_long_root_is_clipped_so_the_reply_fits_one_preamble_line() {
    let worst = format!("/{}", "\u{1}".repeat(4000));
    let shown = display_root(std::path::Path::new(&worst));
    assert_eq!(shown.chars().count(), MAX_ROOT_CHARS + 1);
    assert!(shown.ends_with('\u{2026}'), "marked as clipped");
    let reply = DaemonStatus {
        repo_root: shown,
        ..status()
    };
    let encoded = reply.encode();
    assert!(
        encoded.len() <= PREAMBLE_MAX_BYTES,
        "{} bytes",
        encoded.len()
    );
    assert_eq!(DaemonStatus::decode(&line(&encoded)), Ok(reply));
    assert_eq!(
        display_root(std::path::Path::new("/short/root")),
        "/short/root",
        "a short root is not marked"
    );
}

/// A root that is not UTF-8 is shown with its undecodable bytes replaced: the status is for a
/// person to read, and the key, not the root, names the daemon.
#[test]
fn a_root_that_is_not_utf8_is_shown_lossily() {
    use std::os::unix::ffi::OsStrExt;
    let root = std::path::Path::new(std::ffi::OsStr::from_bytes(b"/w/\xffx"));
    assert_eq!(display_root(root), "/w/\u{fffd}x");
}
