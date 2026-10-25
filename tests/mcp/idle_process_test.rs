//! Real shim spawn, first-spawner policy, control neutrality and unload/reload/serve.
#![cfg(all(feature = "mcp", unix))]
use crate::daemon_process_test::{setup, wait_until, Setup, CALLEES, HANG, INIT};
use crate::shim_process_test::{Daemons, ShimProcess};
use crate::transcript::spawn_line_reader;
use prism::mcp::shared::control::{ControlRequest, DaemonStatus};
use prism::mcp::shared::preamble::BuildState;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

fn status(setup: &Setup) -> DaemonStatus {
    let mut stream = UnixStream::connect(setup.paths.socket()).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    stream
        .write_all(ControlRequest::Status.encode().as_bytes())
        .unwrap();
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).unwrap();
    DaemonStatus::decode(line.trim_end().as_bytes()).unwrap()
}

fn operator(setup: &Setup, verb: &str) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_prism-mcp"))
        .args([verb, "--repo"])
        .arg(setup.repo.path())
        .arg("--cache-dir")
        .arg(setup.cache.path())
        .arg("--runtime-dir")
        .arg(setup.base.path())
        .env(
            "PRISM_MCP_INDEX_IDLE_SECS",
            "invalid-is-irrelevant-to-operator",
        )
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output);
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn shared_idle_spawn_reload_process() {
    let setup = setup();
    let _cleanup = Daemons(&setup.paths);
    let mut first = ShimProcess::start(
        &setup,
        &["--index-idle-secs", "1"],
        &[("PRISM_MCP_INDEX_IDLE_SECS", "bad")],
    );
    first.handshake();
    let pid = first.daemon_pid();
    let initial = status(&setup);
    eprintln!(
        "first READY: {}STATUS: {}",
        first.stderr(),
        initial.encode()
    );
    assert_eq!(initial.build, BuildState::Idle);
    assert_eq!(
        initial.index_idle_timeout_secs, 1,
        "actual spawned daemon policy"
    );
    assert_eq!(initial.instance.pid, pid);
    assert!(!first.stderr().contains("idle policy mismatch"));
    assert!(first
        .stderr()
        .contains("index idle; index_idle_timeout_secs=1; index_idle_pressure_relief=false"));
    let served = first.request(CALLEES);
    eprintln!("initial served: {served}");
    assert_eq!(served["result"]["isError"], false, "{served}");
    assert!(served.to_string().contains("helper"), "{served}");
    // A descheduled parent may observe a legitimate unload after the served response.
    assert!(matches!(
        status(&setup).build,
        BuildState::Ready | BuildState::Unloaded
    ));
    // Keep a second client connected and ask it to drain: neither operator resets tool-idle.
    let mut second = ShimProcess::start(
        &setup,
        &["--index-idle-secs", "7", "--index-idle-pressure-relief"],
        &[],
    );
    second.handshake();
    assert_eq!(second.daemon_pid(), pid);
    let said = second.stderr();
    eprintln!("second READY: {said}");
    let mismatch: Vec<_> = said
        .lines()
        .filter(|l| l.contains("idle policy mismatch"))
        .collect();
    assert_eq!(mismatch.len(), 1, "{said}");
    for part in [
        format!("pid {pid}"),
        "effective index_idle_timeout_secs=1".into(),
        "requested index_idle_timeout_secs=7".into(),
        "effective index_idle_pressure_relief=false; requested index_idle_pressure_relief=true"
            .into(),
    ] {
        assert!(mismatch[0].contains(&part), "{said}");
    }
    assert!(
        said.contains("index_idle_timeout_secs=1; index_idle_pressure_relief=false"),
        "{said}"
    );
    let shutdown = operator(&setup, "daemon-shutdown");
    eprintln!("shutdown: {shutdown}");
    assert!(shutdown.contains("drain"), "{shutdown}");
    wait_until(
        "unloaded while STATUS and shutdown do not reset idle",
        || {
            operator(&setup, "daemon-shutdown");
            let text = operator(&setup, "daemon-status");
            assert!(text.contains("connected; in use"), "{text}");
            assert!(
                text.contains("index_idle_timeout_secs=1; index_idle_pressure_relief=false"),
                "{text}"
            );
            if text.contains("index: unloaded ") {
                eprintln!("unloaded STATUS: {text}");
            }
            text.contains("index: unloaded ")
        },
    );
    let unloaded = status(&setup);
    assert_eq!(unloaded.generation, 0);
    assert_eq!(unloaded.instance, initial.instance);
    let served = second.request(CALLEES);
    eprintln!("reload served: {served}");
    assert_eq!(served["result"]["isError"], false, "{served}");
    assert!(served.to_string().contains("helper"), "{served}");
    assert!(matches!(
        status(&setup).build,
        BuildState::Ready | BuildState::Unloaded
    ));
    eprintln!(
        "daemon key-log:\n{}",
        std::fs::read_to_string(setup.paths.log_file()).unwrap()
    );
    second.close_input();
    assert!(second.ended().0.success());
    first.close_input();
    assert!(first.ended().0.success());
    assert_eq!(
        second
            .stderr()
            .lines()
            .filter(|l| l.contains("idle policy mismatch"))
            .count(),
        1
    );
}

struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn shim_command(setup: &Setup) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_prism-mcp"));
    c.arg("--repo")
        .arg(setup.repo.path())
        .arg("--shared")
        .arg("--cache-dir")
        .arg(setup.cache.path())
        .env("PRISM_MCP_RUNTIME_DIR", setup.base.path())
        .env_remove("PRISM_MCP_INDEX_IDLE_SECS");
    c
}

#[test]
fn shared_idle_cli_resolution_process() {
    for (flag, env, relief, expected) in [
        (None, None, false, 0),
        (None, Some("2"), false, 2),
        (Some("3"), Some("2"), false, 3),
        (Some("0"), Some("2"), false, 0),
        (Some("0"), Some("bad"), false, 0),
        (Some("1"), Some("0"), true, 1),
        (Some("0"), Some("0"), true, 0),
    ] {
        let setup = setup();
        let _cleanup = Daemons(&setup.paths);
        let mut c = shim_command(&setup);
        if let Some(flag) = flag {
            c.args(["--index-idle-secs", flag]);
        }
        if let Some(env) = env {
            c.env("PRISM_MCP_INDEX_IDLE_SECS", env);
        }
        if relief {
            c.arg("--index-idle-pressure-relief");
        }
        let mut shim = ChildGuard(
            c.stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap(),
        );
        let (lines, reader) = spawn_line_reader(shim.0.stdout.take().unwrap());
        writeln!(shim.0.stdin.as_mut().unwrap(), "{INIT}").unwrap();
        let response = lines.recv_timeout(HANG).unwrap().unwrap();
        let response: serde_json::Value = serde_json::from_slice(&response).unwrap();
        assert!(response["result"].is_object(), "{response}");
        let actual = status(&setup);
        assert_eq!(actual.index_idle_timeout_secs, expected, "{flag:?}/{env:?}");
        let effective_relief = relief && expected != 0 && cfg!(target_os = "macos");
        assert_eq!(actual.index_idle_pressure_relief, effective_relief);
        drop(shim.0.stdin.take());
        let mut exit = None;
        wait_until("shim exit", || {
            exit = shim.0.try_wait().unwrap();
            exit.is_some()
        });
        let mut text = String::new();
        let mut stderr = shim.0.stderr.take().unwrap();
        stderr.read_to_string(&mut text).unwrap();
        reader.join().unwrap();
        assert!(exit.unwrap().success(), "{text}");
        assert_eq!(
            text.lines()
                .filter(|l| l.contains("idle policy mismatch"))
                .count(),
            usize::from(relief != effective_relief),
            "{text}"
        );
    }
}

#[test]
fn shared_idle_invalid_settings_process() {
    for (flag, env) in [
        (Some("bad"), "0"),
        (None, "bad"),
        (Some("18446744073709551616"), "0"),
        (None, "18446744073709551616"),
    ] {
        let setup = setup();
        let _cleanup = Daemons(&setup.paths);
        let mut command = shim_command(&setup);
        command.env("PRISM_MCP_INDEX_IDLE_SECS", env);
        if let Some(flag) = flag {
            command.args(["--index-idle-secs", flag]);
        }
        let output = command.output().unwrap();
        let text = String::from_utf8(output.stderr).unwrap();
        assert!(!output.status.success(), "{flag:?}/{env}: {text}");
        assert!(text.contains("index idle seconds"), "{text}");
        assert!(!setup.paths.socket().exists(), "must not spawn");
    }
}
