//! `prism-mcp daemon-status` and `prism-mcp daemon-shutdown` as processes, against a real daemon
//! process and a real shim (spec §4.7, §11 item 3). Each test has a runtime directory of its own.
#![cfg(feature = "mcp")]

use crate::daemon_process_test::{gone, setup, wait_until, Daemon, Setup};
use crate::shim_process_test::{Daemons, ShimProcess};
use prism::mcp::shared::runtime_dir::RUNTIME_DIR_ENV;
use std::process::{Command, Output};

const LIST: &str = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#;

/// `prism-mcp <verb> <args>`, with the setup's runtime base given as `--runtime-dir`.
fn operator(setup: &Setup, verb: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_prism-mcp"))
        .arg(verb)
        .arg("--runtime-dir")
        .arg(setup.base.path())
        .args(args)
        .env_remove(RUNTIME_DIR_ENV)
        .output()
        .expect("run the operator command")
}

/// `prism-mcp daemon-shutdown` for the setup's configuration.
fn shutdown(setup: &Setup, extra: &[&str]) -> Output {
    let repo = setup.repo.path().to_str().expect("a UTF-8 path");
    let cache = setup.cache.path().to_str().expect("a UTF-8 path");
    let mut args = vec!["--repo", repo, "--cache-dir", cache];
    args.extend_from_slice(extra);
    operator(setup, "daemon-shutdown", &args)
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn started(setup: &Setup, extra: &[&str]) -> Daemon {
    let daemon = Daemon::start(setup, extra);
    wait_until("the daemon's socket", || setup.paths.socket().exists());
    daemon
}

/// S4's interface for S5b: an operator who keeps looking does not keep an idle daemon up. With
/// probes every 100 ms the daemon still retires after its 2 s idle TTL; with a probe that
/// restarted the idle time it would never retire, and the wait below would give up at its hang
/// limit. Nothing here depends on when the first probe runs (review round 1, Opus S1: the
/// listing is asserted on a daemon with the default TTL, below).
#[test]
fn daemon_status_does_not_keep_an_idle_daemon_up() {
    let setup = setup();
    let mut daemon = started(&setup, &["--idle-ttl", "2"]);
    wait_until("the daemon to retire while it is being looked at", || {
        let output = operator(&setup, "daemon-status", &[]);
        assert!(output.status.success(), "{}", stderr(&output));
        std::thread::sleep(std::time::Duration::from_millis(100));
        daemon.exited().is_some()
    });
    let (status, log) = daemon.ended();
    assert!(status.success(), "{status:?} {log}");
    assert!(log.contains("stopped: idle"), "{log}");
}

/// Spec §4.7: `daemon-status` lists a running daemon; `daemon-shutdown` drains a daemon with no
/// client at once, waits for the drain, and the daemon exits 0 having removed its socket and pid
/// file.
#[test]
fn daemon_shutdown_drains_an_idle_daemon() {
    let setup = setup();
    let mut daemon = started(&setup, &[]);
    // Spec §4.7: `daemon-status` lists the daemon with its pid, root, key and idle TTL, found
    // through `PRISM_MCP_RUNTIME_DIR` as a shim finds it.
    let listed = Command::new(env!("CARGO_BIN_EXE_prism-mcp"))
        .arg("daemon-status")
        .env(RUNTIME_DIR_ENV, setup.base.path())
        .output()
        .expect("run daemon-status");
    assert!(listed.status.success(), "{}", stderr(&listed));
    let said = stdout(&listed);
    let root = std::fs::canonicalize(setup.repo.path()).expect("the canonical root");
    for expected in [
        format!("{} pid {}, started ", setup.key.locator(), daemon.pid()),
        format!("serving {}\n", root.display()),
        format!("  key {}; build ", setup.key),
        "retires after 900 s idle".to_string(),
    ] {
        assert!(said.contains(&expected), "{expected:?} in {said}");
    }
    let output = shutdown(&setup, &[]);
    assert!(output.status.success(), "{}", stderr(&output));
    let said = stdout(&output);
    assert!(
        said.starts_with(&format!(
            "prism-mcp daemon-shutdown: pid {} (started ",
            daemon.pid()
        )),
        "{said}"
    );
    assert!(said.ends_with(") drained\n"), "{said}");
    let (status, log) = daemon.ended();
    assert!(status.success(), "{status:?} {log}");
    assert!(
        log.contains("stopped: an operator's daemon-shutdown"),
        "{log}"
    );
    assert!(gone(setup.paths.socket()));
    assert!(gone(setup.paths.pid_file()));
}

/// Spec §4.7 and §11 item 3: `daemon-shutdown` never disconnects a client. The shim's session
/// goes on after the request, `daemon-status` shows the request, and the daemon exits once the
/// client has closed its session.
#[test]
fn daemon_shutdown_never_disconnects_a_client() {
    let setup = setup();
    let mut daemon = started(&setup, &[]);
    let _daemons = Daemons(&setup.paths);
    let mut shim = ShimProcess::start(&setup, &[], &[]);
    shim.handshake();
    assert_eq!(shim.daemon_pid(), daemon.pid());

    let output = shutdown(&setup, &[]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("drains once its 1 admitted clients have disconnected"),
        "{}",
        stdout(&output)
    );
    let listed = shim.request(LIST);
    assert!(listed["result"]["tools"].is_array(), "{listed}");
    let status = stdout(&operator(&setup, "daemon-status", &[]));
    assert!(
        status.contains("  shutdown requested: it drains once no client is left\n"),
        "{status}"
    );
    assert!(daemon.exited().is_none(), "the daemon serves its client");

    shim.close_input();
    let (shim_status, shim_log) = shim.ended();
    assert!(shim_status.success(), "{shim_status:?} {shim_log}");
    let (status, log) = daemon.ended();
    assert!(status.success(), "{status:?} {log}");
    assert!(
        log.contains("stopped: an operator's daemon-shutdown"),
        "{log}"
    );
}

/// Spec §11 item 3: `--pid` and `--start-time` bind the request to one instance; a daemon that
/// is another instance is left alone, and the command still exits 0 (the instance it named does
/// not serve the key).
#[test]
fn daemon_shutdown_of_another_instance_leaves_the_daemon_alone() {
    let setup = setup();
    let mut daemon = started(&setup, &[]);
    let pid = daemon.pid().to_string();
    let output = shutdown(&setup, &["--pid", &pid, "--start-time", "1"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("which was left alone"),
        "{}",
        stdout(&output)
    );
    let status = stdout(&operator(&setup, "daemon-status", &[]));
    assert!(!status.contains("shutdown requested"), "{status}");
    assert!(daemon.exited().is_none());
}

/// With no daemon for the configuration there is nothing to do: exit 0, and it says so.
#[test]
fn daemon_shutdown_with_no_daemon_says_so() {
    let setup = setup();
    let output = shutdown(&setup, &[]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("nothing to shut down"),
        "{}",
        stdout(&output)
    );
}

/// The flags: `daemon-shutdown` needs `--repo`; `--pid` and `--start-time` go together; the
/// configuration flags of `daemon-status` need `--repo`. Each is a usage error (status 2).
#[test]
fn the_operator_commands_check_their_arguments() {
    let setup = setup();
    let cache = setup.cache.path().to_str().expect("a UTF-8 path");
    for (verb, args, names) in [
        ("daemon-shutdown", vec![], "--repo"),
        (
            "daemon-shutdown",
            vec!["--repo", "/", "--pid", "1"],
            "--start-time",
        ),
        (
            "daemon-shutdown",
            vec!["--repo", "/", "--start-time", "1"],
            "--pid",
        ),
        ("daemon-status", vec!["--cache-dir", cache], "--repo"),
        ("daemon-status", vec!["--timeout", "0"], "--timeout"),
    ] {
        let output = operator(&setup, verb, &args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{verb} {args:?}: {}",
            stderr(&output)
        );
        assert!(
            stderr(&output).contains(names),
            "{verb} {args:?}: {}",
            stderr(&output)
        );
    }
}
