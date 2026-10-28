//! Allocator policy and startup observability through the real serving binary.
#![cfg(feature = "mcp")]

use assert_cmd::Command;
use std::process::Output;
use std::time::Duration;

const INIT: &str = concat!(
    r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"malloc-test","version":"0"}}}"#,
    "\n"
);

fn command() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_prism-mcp"));
    for name in [
        "MallocSpaceEfficient",
        "PRISM_MCP_MALLOC_SPACE_EFFICIENT",
        "PRISM_MCP_MEM_REPORT",
        "PRISM_MCP_INDEX_IDLE_SECS",
        "PRISM_MCP_CONCISE_SHAPE",
        "PRISM_MCP_STRUCTURED_CONTENT",
        "PRISM_MCP_MAX_RESULT_CHARS",
    ] {
        command.env_remove(name);
    }
    command
}

fn serve(space: Option<&str>, opt_out: Option<&str>) -> Output {
    let repo = tempfile::tempdir().unwrap();
    let mut command = Command::from_std(command());
    command
        .timeout(Duration::from_secs(30))
        .arg("--repo")
        .arg(repo.path())
        .args(["--no-cache", "--index-idle-secs", "60"]);
    if let Some(value) = space {
        command.env("MallocSpaceEfficient", value);
    }
    if let Some(value) = opt_out {
        command.env("PRISM_MCP_MALLOC_SPACE_EFFICIENT", value);
    }
    command
        .write_stdin(INIT)
        .assert()
        .success()
        .get_output()
        .clone()
}

fn assert_startup(output: &Output, expected: &str) {
    let stderr = std::str::from_utf8(&output.stderr).unwrap();
    let startup: Vec<_> = stderr
        .lines()
        .filter(|line| line.contains(" event=startup "))
        .collect();
    assert_eq!(startup.len(), 1, "one startup event: {stderr}");
    let field = format!("malloc_space_efficient={expected}");
    assert!(
        startup[0].split_whitespace().any(|token| token == field),
        "expected {field}: {stderr}"
    );
}

#[test]
fn startup_reports_caller_allocator_setting() {
    for value in ["0", "1"] {
        let output = serve(Some(value), None);
        assert_startup(&output, value);
        assert_eq!(
            std::str::from_utf8(&output.stderr).unwrap().lines().count(),
            1
        );
    }
}

#[test]
fn startup_bounds_empty_and_arbitrary_allocator_values() {
    for value in ["", "unexpected\nvalue", &"x".repeat(2000)] {
        let output = serve(Some(value), Some("invalid"));
        assert_startup(&output, "other");
        assert_eq!(
            std::str::from_utf8(&output.stderr).unwrap().lines().count(),
            1
        );
    }
}

#[test]
fn startup_reports_unset_allocator_with_opt_out() {
    let output = serve(None, Some("0"));
    assert_startup(&output, "unset");
    assert_eq!(
        std::str::from_utf8(&output.stderr).unwrap().lines().count(),
        1
    );
}

#[cfg(target_os = "macos")]
#[test]
fn macos_reexec_enables_allocator_by_default_and_with_empty_or_one_opt_out() {
    for opt_out in [None, Some(""), Some("1")] {
        let output = serve(None, opt_out);
        assert_startup(&output, "1");
        assert_eq!(
            std::str::from_utf8(&output.stderr).unwrap().lines().count(),
            1
        );
    }
}

#[cfg(target_os = "macos")]
#[test]
fn macos_invalid_opt_out_warns_once_and_keeps_allocator_unset() {
    let output = serve(None, Some("bogus\nvalue"));
    assert_startup(&output, "unset");
    let stderr = std::str::from_utf8(&output.stderr).unwrap();
    assert_eq!(stderr.lines().count(), 2, "{stderr}");
    assert_eq!(
        stderr.matches("PRISM_MCP_MALLOC_SPACE_EFFICIENT").count(),
        1
    );
    assert!(stderr.contains("unset, empty, 1, or 0"), "{stderr}");
}

#[cfg(target_os = "macos")]
#[test]
fn macos_reexec_preserves_stdio_initialize_and_pid() {
    use crate::transcript::{spawn_line_reader, RESPONSE_TIMEOUT};
    use std::io::Write;
    use std::process::Stdio;

    struct ChildGuard(std::process::Child);
    impl Drop for ChildGuard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let repo = tempfile::tempdir().unwrap();
    let mut command = command();
    let mut child = ChildGuard(
        command
            .arg("--repo")
            .arg(repo.path())
            .args(["--no-cache", "--index-idle-secs", "60"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let pid = child.0.id();
    let (stdout, stdout_reader) = spawn_line_reader(child.0.stdout.take().unwrap());
    let (stderr, stderr_reader) = spawn_line_reader(child.0.stderr.take().unwrap());
    let startup = stderr.recv_timeout(RESPONSE_TIMEOUT).unwrap().unwrap();
    let startup = String::from_utf8(startup).unwrap();
    assert!(startup.contains(" malloc_space_efficient=1 "), "{startup}");
    assert!(startup.contains(&format!(" pid={pid} ")), "{startup}");
    child
        .0
        .stdin
        .as_mut()
        .unwrap()
        .write_all(INIT.as_bytes())
        .unwrap();
    let response = stdout.recv_timeout(RESPONSE_TIMEOUT).unwrap().unwrap();
    let response: serde_json::Value = serde_json::from_slice(&response).unwrap();
    assert_eq!(response["id"], 1);
    assert_eq!(response["result"]["protocolVersion"], "2025-11-25");
    assert!(response["error"].is_null(), "{response}");
    drop(child.0.stdin.take());
    crate::daemon_process_test::wait_until("stdio server to exit", || {
        child.0.try_wait().unwrap().is_some()
    });
    assert!(child.0.wait().unwrap().success());
    stdout_reader.join().unwrap();
    stderr_reader.join().unwrap();
    assert!(stdout.try_recv().is_err(), "no extra stdout");
    assert!(stderr.try_recv().is_err(), "no warning on normal path");
}

#[cfg(unix)]
#[test]
fn daemon_startup_reports_allocator_setting() {
    let setup = crate::daemon_process_test::setup();
    let output = Command::from_std(command())
        .timeout(Duration::from_secs(30))
        .args(["daemon", "--repo"])
        .arg(setup.repo.path())
        .arg("--cache-dir")
        .arg(setup.cache.path())
        .args(["--idle-ttl", "1"])
        .env("PRISM_MCP_RUNTIME_DIR", setup.base.path())
        .env("MallocSpaceEfficient", "0")
        .assert()
        .success()
        .get_output()
        .clone();
    assert_startup(&output, "0");
    assert!(output.stdout.is_empty());
}
