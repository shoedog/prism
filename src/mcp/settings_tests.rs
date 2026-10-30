//! Tests for the settings a client may ask a shared daemon for. They arrive from another
//! process, so every bound is enforced where the value is constructed.
use super::*;
use crate::mcp::concise_shape::ConciseShapeMode;
use crate::mcp::output::{
    resolve_cap_from, StructuredContentMode, MAX_RESULT_CHARS, MAX_RESULT_CHARS_FLOOR,
};
use crate::mcp::FIRST_CALL_WAIT_MAX;
use std::time::{Duration, Instant};

fn output(result_cap: usize) -> ConnectionSettings {
    ConnectionSettings {
        result_cap,
        structured_content: StructuredContentMode::OmitDefaultPath,
        concise_shape: ConciseShapeMode::Slim,
    }
}

/// A cap below the floor reaching dispatch trips `cap_after_reserve`'s debug assertion (a
/// panic in dispatch is fatal to a daemon) and silently saturates in release. Rules out
/// carrying the client's number through unchecked.
#[test]
fn a_result_cap_below_the_floor_is_refused() {
    for cap in [0, 1, 100, MAX_RESULT_CHARS_FLOOR - 1] {
        assert_eq!(
            ClientSettings::new(output(cap), 20, false),
            Err(SettingsError::ResultCapBelowFloor {
                cap,
                floor: MAX_RESULT_CHARS_FLOOR
            }),
            "cap {cap}"
        );
    }
}

#[test]
fn a_result_cap_at_or_above_the_floor_is_kept_exactly() {
    for cap in [
        MAX_RESULT_CHARS_FLOOR,
        MAX_RESULT_CHARS_FLOOR + 1,
        MAX_RESULT_CHARS,
        usize::MAX,
    ] {
        let settings = ClientSettings::new(output(cap), 20, false).expect("accepted");
        assert_eq!(settings.output().result_cap, cap);
    }
}

/// One acceptance rule for a cap, wherever it comes from: this constructor accepts exactly
/// the caps `resolve_cap_from` keeps (the environment path replaces the others with the
/// default and warns; a decoded cap is refused instead, because a daemon has no stderr the
/// client can see).
#[test]
fn the_constructor_accepts_exactly_the_caps_the_environment_resolver_keeps() {
    for cap in [
        0,
        1,
        MAX_RESULT_CHARS_FLOOR - 1,
        MAX_RESULT_CHARS_FLOOR,
        MAX_RESULT_CHARS_FLOOR + 1,
        MAX_RESULT_CHARS - 1,
        MAX_RESULT_CHARS + 1,
    ] {
        let kept_by_environment = resolve_cap_from(Some(&cap.to_string())) == cap;
        assert_eq!(
            ClientSettings::new(output(cap), 20, false).is_ok(),
            kept_by_environment,
            "cap {cap}"
        );
    }
}

/// The smallest cap the constructor accepts is usable where dispatch subtracts the most from
/// it: a query on a stale index under an automatic refresh policy pays both the auto-refresh
/// and the freshness reserve. In a debug build this is the path whose assertion a smaller cap
/// trips; here it must answer, within the cap.
#[test]
fn the_smallest_accepted_cap_serves_the_query_that_reserves_the_most() {
    use crate::mcp::framing::InMemoryTransport;
    use crate::mcp::registry::ToolRegistry;
    use crate::mcp::{CacheMode, RefreshPolicy, ServerConfig, SessionProvider, StartupMode};

    let repo = tempfile::tempdir().unwrap();
    std::fs::write(repo.path().join("util.py"), "def helper():\n    return 1\n").unwrap();
    std::fs::write(
        repo.path().join("main.py"),
        "from util import helper\n\ndef run():\n    return helper()\n",
    )
    .unwrap();
    let mut cfg = ServerConfig::new(repo.path().to_path_buf());
    cfg.cache = CacheMode::NoCache;
    cfg.refresh_policy = RefreshPolicy::AutoFull;
    cfg.startup = StartupMode::Eager;
    let mut provider = SessionProvider::bootstrap(&cfg).unwrap();
    // Make the index stale, so the call takes the refresh path.
    std::fs::write(
        repo.path().join("util.py"),
        "def helper():\n    return 1\n\n\ndef extra():\n    return helper()\n",
    )
    .unwrap();

    let settings = ClientSettings::new(output(MAX_RESULT_CHARS_FLOOR), 20, false).unwrap();
    let mut transport = InMemoryTransport::new(vec![
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_callers","arguments":{"seed":{"kind":"symbol","name":"helper","file":"util.py"}}}}"#,
    ]);
    crate::mcp::transport::serve_connection(
        &mut provider,
        &ToolRegistry::all_v1(),
        &mut transport,
        settings.output(),
    )
    .unwrap();

    let response = &transport.responses()[1];
    assert_eq!(response["result"]["isError"], false, "{response}");
    assert_eq!(
        response["result"]["_meta"]["prism/auto_refresh"], "refreshed",
        "the call went through the refresh path: {response}"
    );
    assert!(response.to_string().len() <= MAX_RESULT_CHARS_FLOOR);
}

/// Rules out an unbounded wait: a daemon adds it to an `Instant`, and an overflow there is a
/// panic.
#[test]
fn a_first_call_wait_above_the_command_line_maximum_is_refused() {
    let max_secs = FIRST_CALL_WAIT_MAX.as_secs();
    assert_eq!(max_secs, 600, "the `--first-call-wait` range is 0..=600");
    for secs in [max_secs + 1, 86_400, u64::MAX] {
        assert_eq!(
            ClientSettings::new(output(MAX_RESULT_CHARS), secs, false),
            Err(SettingsError::FirstCallWaitTooLong { secs, max_secs }),
            "{secs} s"
        );
    }
    for secs in [0, 1, 20, max_secs] {
        let settings = ClientSettings::new(output(MAX_RESULT_CHARS), secs, false).expect("ok");
        assert_eq!(settings.first_call_wait_secs(), secs);
        assert_eq!(settings.first_call_wait(), Duration::from_secs(secs));
        assert!(Instant::now()
            .checked_add(settings.first_call_wait())
            .is_some());
    }
}

#[test]
fn the_settings_are_returned_as_given() {
    let always_legacy = ConnectionSettings {
        result_cap: 20_000,
        structured_content: StructuredContentMode::Always,
        concise_shape: ConciseShapeMode::Legacy,
    };
    let settings = ClientSettings::new(always_legacy, 7, true).unwrap();
    assert_eq!(settings.output(), &always_legacy);
    assert_eq!(settings.first_call_wait_secs(), 7);
    assert!(settings.warm_at_startup());
    assert!(!ClientSettings::new(always_legacy, 7, false)
        .unwrap()
        .warm_at_startup());
}

#[test]
fn each_refusal_says_what_was_asked_and_what_is_allowed() {
    let cap = SettingsError::ResultCapBelowFloor {
        cap: 100,
        floor: MAX_RESULT_CHARS_FLOOR,
    }
    .to_string();
    assert!(cap.contains("100") && cap.contains("12000"), "{cap}");
    let wait = SettingsError::FirstCallWaitTooLong {
        secs: 601,
        max_secs: 600,
    }
    .to_string();
    assert!(wait.contains("601") && wait.contains("600"), "{wait}");
}
