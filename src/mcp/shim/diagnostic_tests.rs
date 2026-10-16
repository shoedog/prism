//! Tests for the fail-closed line (spec §4.1 step 7).
use super::super::shim_support::{key, runtime};
use super::*;
use std::path::PathBuf;

/// Spec §4.1 step 7: the line names the key, the runtime directory, the socket, the pid file's
/// contents (its pid and start time, spec §4.1 step 6), the log, and the standalone invocation,
/// on one line.
#[test]
fn the_line_names_everything_an_operator_needs_on_one_line() {
    let (_base, paths) = runtime();
    std::fs::write(paths.pid_file(), "4242\n1700000000\nabc\n").expect("pid file");
    let key = key();
    let at = Where {
        key: Some(&key),
        runtime_dir: Some(paths.runtime_dir()),
        paths: Some(&paths),
    };
    let line = fail_closed_line("the reason", &at, "prism-mcp --repo /r --standalone");
    assert!(!line.contains('\n'), "{line}");
    for part in [
        "the reason".to_string(),
        key.to_hex(),
        paths.runtime_dir().display().to_string(),
        paths.socket().display().to_string(),
        "pid 4242, start time 1700000000, build abc".to_string(),
        paths.log_file().display().to_string(),
        "prism-mcp --repo /r --standalone".to_string(),
    ] {
        assert!(line.contains(&part), "{part} in {line}");
    }
}

/// S4's interface: the pid file is rewritten in place, so a reader can find it empty or partial;
/// that is said, not misread. A missing one is said too.
#[test]
fn an_empty_partial_or_missing_pid_file_is_reported_as_such() {
    let (_base, paths) = runtime();
    let key = key();
    let at = Where {
        key: Some(&key),
        runtime_dir: Some(paths.runtime_dir()),
        paths: Some(&paths),
    };
    assert!(fail_closed_line("r", &at, "s").contains("absent"));
    std::fs::write(paths.pid_file(), "").expect("pid file");
    assert!(fail_closed_line("r", &at, "s").contains("empty or partial (\"\")"));
    std::fs::write(paths.pid_file(), "42\n17").expect("pid file");
    let line = fail_closed_line("r", &at, "s");
    assert!(line.contains("empty or partial (\"42\\n17\")"), "{line}");
    assert!(!line.contains('\n'));
}

/// Lane README, notes for S5a: a path with a newline or a terminal escape cannot break the line.
#[test]
fn control_characters_in_paths_cannot_end_the_line() {
    let key = key();
    let dir = PathBuf::from("/tmp/a\nb\u{1b}[2J");
    let at = Where {
        key: Some(&key),
        runtime_dir: Some(&dir),
        paths: None,
    };
    let line = fail_closed_line("r", &at, "s");
    assert!(!line.contains('\n') && !line.contains('\u{1b}'), "{line:?}");
    assert!(line.contains("/tmp/a\\nb\\u{1b}[2J"), "{line}");
}

/// Spec §4.1 step 7: "the exact `--standalone` invocation": the same repository, cache,
/// refresh policy, first-call wait and warm start, and words a shell reads back as they are.
#[test]
fn the_standalone_invocation_keeps_the_configuration() {
    let mut cfg = ServerConfig::new(PathBuf::from("/my repo"));
    assert_eq!(
        standalone_invocation(Path::new("/bin/prism-mcp"), &cfg),
        "/bin/prism-mcp --repo '/my repo' --standalone"
    );
    cfg.cache = CacheMode::Dir(PathBuf::from("/c/it's"));
    cfg.refresh_policy = RefreshPolicy::AutoFull;
    cfg.first_call_wait = Duration::from_secs(60);
    cfg.startup = StartupMode::Background;
    assert_eq!(
        standalone_invocation(Path::new("/bin/prism-mcp"), &cfg),
        "/bin/prism-mcp --repo '/my repo' --cache-dir '/c/it'\\''s' --refresh-policy auto-full \
         --first-call-wait 60 --warm-at-startup --standalone"
    );
}

/// Review round 1, sol 5: a failure before the key is known still makes the one line, with the
/// key, the runtime directory and the files unresolved and the standalone invocation named.
#[test]
fn a_failure_before_the_key_is_known_names_what_it_can() {
    let at = Where {
        key: None,
        runtime_dir: None,
        paths: None,
    };
    let line = fail_closed_line("no repository", &at, "prism-mcp --repo /r --standalone");
    assert_eq!(
        line,
        "prism-mcp: --shared failed closed, no index was built: no repository; key unresolved; \
         runtime dir unresolved; to run without the shared daemon: prism-mcp --repo /r --standalone"
    );
}
