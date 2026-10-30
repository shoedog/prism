//! CLI → production options → actual daemon socket. Child processes isolate ambient settings.
use super::*;
use clap::Parser;
use prism::mcp::settings::{ClientSettings, ConnectionSettings};
use prism::mcp::shared::{
    control::ControlRequest,
    identity::DaemonIdentity,
    preamble::{Hello, Reply},
};
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn parsed(extra: &[&str]) -> DaemonCli {
    let mut args = vec!["daemon", "--repo", "."];
    args.extend_from_slice(extra);
    DaemonCli::try_parse_from(args).expect("daemon accepts explicit idle options")
}

#[test]
fn daemon_idle_cli_validation_and_identity() {
    let original = daemon_options(parsed(&[])).unwrap();
    let key = DaemonIdentity::resolve(&original.config).unwrap().key();
    for secs in ["0", "600"] {
        let options = daemon_options(parsed(&[
            "--index-idle-secs",
            secs,
            "--index-idle-pressure-relief",
        ]))
        .unwrap();
        assert_eq!(
            DaemonIdentity::resolve(&options.config).unwrap().key(),
            key,
            "idle policy must stay outside identity"
        );
    }
    for bad in [
        "",
        "-1",
        "+1",
        "1.5",
        " 1",
        "18446744073709551616",
        "18446744073709551615",
    ] {
        let args = ["daemon", "--repo", ".", "--index-idle-secs", bad];
        let invalid = DaemonCli::try_parse_from(args).map(daemon_options);
        assert!(
            !matches!(invalid, Ok(Ok(_))),
            "invalid timeout accepted: {bad:?}"
        );
    }
    use clap::CommandFactory;
    let help = DaemonCli::command().render_long_help().to_string();
    assert!(help.contains("--index-idle-secs"));
    assert!(!help.contains("--index-idle-pressure-relief"));
}

#[test]
fn child_daemon() {
    let Ok(args) = std::env::var("I0B_TEST_ARGS") else {
        return;
    };
    let args: Vec<String> = serde_json::from_str(&args).unwrap();
    prism::mcp::run_daemon(daemon_options(DaemonCli::try_parse_from(args).unwrap()).unwrap())
        .unwrap();
}

struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn line(stream: &UnixStream, text: &str) -> String {
    (&*stream).write_all(text.as_bytes()).unwrap();
    let mut reply = String::new();
    BufReader::new(stream).read_line(&mut reply).unwrap();
    reply
}
fn json(line: &str) -> Value {
    serde_json::from_str(line.split_once(' ').unwrap().1).unwrap()
}

fn socket_policy(extra: &[&str], expected: u64, relief: bool) {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("main.py"), "def run():\n    return 1\n").unwrap();
    let runtime = tempfile::Builder::new()
        .prefix("i0b2")
        .tempdir_in("/tmp")
        .unwrap();
    let mut args = vec![
        "daemon".to_string(),
        "--repo".into(),
        root.path().display().to_string(),
        "--cache-dir".into(),
        root.path().join("cache").display().to_string(),
    ];
    args.extend(extra.iter().map(|s| s.to_string()));
    let options = daemon_options(DaemonCli::try_parse_from(&args).unwrap()).unwrap();
    let key = DaemonIdentity::resolve(&options.config).unwrap().key();
    let _child = ChildGuard(
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "daemon_idle_tests::child_daemon", "--nocapture"])
            .env("I0B_TEST_ARGS", serde_json::to_string(&args).unwrap())
            .env("PRISM_MCP_RUNTIME_DIR", runtime.path())
            .env(
                "PRISM_MCP_INDEX_IDLE_SECS",
                "invalid-ambient-must-be-ignored",
            )
            .stdout(Stdio::null())
            // Never the test's own stderr: the daemon empties a regular-file stderr over 1 MiB at
            // start (log.rs), which would wipe a `cargo test > log 2>&1` caller's log.
            .stderr(Stdio::from(
                std::fs::File::create(runtime.path().join("daemon.log")).unwrap(),
            ))
            .spawn()
            .unwrap(),
    );
    let path = runtime
        .path()
        .join("prism-mcpd")
        .join(format!("{}.sock", key.locator()));
    let deadline = Instant::now() + Duration::from_secs(20);
    let stream = loop {
        if let Ok(s) = UnixStream::connect(&path) {
            break s;
        }
        assert!(Instant::now() < deadline, "daemon did not bind its socket");
        std::thread::sleep(Duration::from_millis(5));
    };
    stream
        .set_read_timeout(Some(Duration::from_secs(20)))
        .unwrap();
    let settings = ClientSettings::new(ConnectionSettings::from_env(), 20, false).unwrap();
    let ready = line(
        &stream,
        &Hello {
            identity: key,
            settings,
        }
        .encode(),
    );
    assert!(matches!(
        Reply::decode(ready.trim_end().as_bytes()),
        Ok(Reply::Ready(_))
    ));
    let ready = json(&ready);
    assert_eq!(ready["build"], "idle");
    assert_eq!(ready["generation"], 0);
    assert_eq!(
        ready["index_idle_timeout_secs"], expected,
        "READY reports effective timeout"
    );
    assert_eq!(ready["index_idle_pressure_relief"], relief);
    let status = || {
        let s = UnixStream::connect(&path).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
        json(&line(&s, &ControlRequest::Status.encode()))
    };
    let initial = status();
    assert_eq!(initial["build"], "idle");
    assert_eq!(initial["index_idle_timeout_secs"], expected);
    assert_eq!(initial["index_idle_pressure_relief"], relief);
    // A v1 control peer is closed with no reply, preserving the existing refusal contract.
    let old = UnixStream::connect(&path).unwrap();
    old.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
    assert!(line(&old, "STATUS {\"control\":1}\n").is_empty());
    // Exercise timed unload only for the one-second cases, not the 600-second default.
    if expected == 1 {
        line(&stream, "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2025-11-25\",\"capabilities\":{},\"clientInfo\":{\"name\":\"t\",\"version\":\"0\"}}}\n");
        (&stream)
            .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
            .unwrap();
        line(&stream, "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"nav_callees\",\"arguments\":{\"seed\":{\"kind\":\"symbol\",\"name\":\"run\",\"file\":\"main.py\"}}}}\n");
        let refresh: Value = serde_json::from_str(&line(&stream, "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{\"name\":\"refresh_index\",\"arguments\":{}}}\n")).unwrap();
        assert_eq!(refresh["result"]["isError"], false);
        assert_eq!(
            refresh["result"]["_meta"]["prism/refresh_generation"], 1,
            "resident G is captured in the refresh response"
        );
        // Deliberately deschedule the parent beyond the child's legitimate unload deadline.
        std::thread::sleep(Duration::from_millis(1100));
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let s = status();
            if s["build"] == "unloaded" {
                assert_eq!(s["generation"], 0);
                break;
            }
            assert!(Instant::now() < deadline, "enabled daemon must unload: {s}");
            std::thread::sleep(Duration::from_millis(10));
        }
        let s = UnixStream::connect(&path).unwrap();
        let ready = json(&line(
            &s,
            &Hello {
                identity: key,
                settings,
            }
            .encode(),
        ));
        assert_eq!(ready["build"], "unloaded");
        assert_eq!(ready["generation"], 0);
    }
}

#[test]
fn daemon_idle_wire_policy_default() {
    socket_policy(&[], 600, false);
}
#[test]
fn daemon_idle_wire_policy_zero() {
    socket_policy(
        &["--index-idle-secs", "0", "--index-idle-pressure-relief"],
        0,
        false,
    );
}
#[test]
fn daemon_idle_wire_policy_nonzero() {
    socket_policy(&["--index-idle-secs", "1"], 1, false);
}
#[test]
fn daemon_idle_wire_policy_relief() {
    socket_policy(
        &["--index-idle-secs", "1", "--index-idle-pressure-relief"],
        1,
        cfg!(target_os = "macos"),
    );
}
