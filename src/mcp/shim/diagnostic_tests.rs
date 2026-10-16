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
        dir_validated: true,
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
        dir_validated: true,
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
        dir_validated: false,
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
        dir_validated: false,
    };
    let line = fail_closed_line("no repository", &at, "prism-mcp --repo /r --standalone");
    assert_eq!(
        line,
        "prism-mcp: --shared failed closed, no index was built: no repository; key unresolved; \
         runtime dir unresolved; to run without the shared daemon: prism-mcp --repo /r --standalone"
    );
}

/// The fail-closed line for `paths`, computed on a thread of its own: `None` when it is not
/// made within `limit` (the read it hangs in is left behind).
fn line_within(paths: &DaemonPaths, limit: Duration) -> Option<String> {
    let (paths, key) = (paths.clone(), key());
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let at = Where {
            key: Some(&key),
            runtime_dir: Some(paths.runtime_dir()),
            paths: Some(&paths),
            dir_validated: true,
        };
        let _ = tx.send(fail_closed_line("r", &at, "s"));
    });
    rx.recv_timeout(limit).ok()
}

/// Review round 1 (Opus F1, sol 1): a FIFO with no writer at the pid file's path cannot hold
/// the line back: it is named as not a regular file, at once.
#[test]
fn a_pid_file_that_is_a_fifo_does_not_hold_the_line_back() {
    let (_base, paths) = runtime();
    let name =
        std::ffi::CString::new(paths.pid_file().as_os_str().as_encoded_bytes()).expect("a C path");
    // SAFETY: `name` is a valid NUL-terminated string that outlives the call.
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0, "mkfifo");
    let line = line_within(&paths, Duration::from_secs(5))
        .expect("the line within 5 s, though the pid file is a FIFO with no writer");
    assert!(line.contains("not a regular file"), "{line}");
}

/// Review round 1 (Opus F1, sol 1): the pid file is read without following a symlink, so a
/// symlink in its place (to a terminal, say) is named, not followed.
#[test]
fn a_pid_file_that_is_a_symlink_is_not_followed() {
    let (base, paths) = runtime();
    let target = base.path().join("elsewhere");
    std::fs::write(&target, "4242\n1700000000\nabc\n").expect("a pid-like file");
    std::os::unix::fs::symlink(&target, paths.pid_file()).expect("a symlink");
    let line = line_within(&paths, Duration::from_secs(5)).expect("the line");
    assert!(!line.contains("pid 4242"), "{line}");
    assert!(line.contains("unreadable"), "{line}");
}

/// The words `bash` reads from `invocation`, as bytes.
fn words_bash_reads(invocation: &str) -> Vec<Vec<u8>> {
    let output = std::process::Command::new("bash")
        .arg("-c")
        .arg(format!("printf '%s\\0' {invocation}"))
        .output()
        .expect("bash");
    assert!(output.status.success(), "{output:?}");
    let mut words: Vec<Vec<u8>> = output
        .stdout
        .split(|b| *b == 0)
        .map(<[u8]>::to_vec)
        .collect();
    assert_eq!(words.pop(), Some(Vec::new()), "NUL-terminated words");
    words
}

/// Review round 1, sol 2: the printed invocation is the exact command for any path, a newline,
/// a tab, a C1 control, a quote, a backslash or a byte that is not UTF-8 in it included: a shell
/// reads back the very paths, and the line stays one line.
#[test]
fn the_printed_invocation_names_the_very_paths_whatever_bytes_they_hold() {
    use std::os::unix::ffi::OsStrExt;
    let repo = PathBuf::from(std::ffi::OsStr::from_bytes(
        b"/tmp/a\nb\tc'd \\e\xff\x1b[2J\xc2\x85f",
    ));
    let exe = Path::new("/opt/it's\nprism-mcp");
    let invocation = standalone_invocation(exe, &ServerConfig::new(repo.clone()));
    let at = Where {
        key: None,
        runtime_dir: None,
        paths: None,
        dir_validated: false,
    };
    let line = fail_closed_line("r", &at, &invocation);
    assert!(!line.contains(|c: char| c.is_control()), "{line:?}");
    let printed = line
        .split_once("; to run without the shared daemon: ")
        .expect("the invocation")
        .1;
    assert_eq!(
        words_bash_reads(printed),
        [
            exe.as_os_str().as_bytes(),
            b"--repo",
            repo.as_os_str().as_bytes(),
            b"--standalone",
        ]
    );
}
