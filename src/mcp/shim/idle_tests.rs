//! Behaviour witnesses for shim argv, READY diagnostics and the operator's text.
use super::control_support::{always_line, fake_status, ControlFake, FakeReply};
use super::run::{daemon_args, ready_lines, ShimOptions};
use super::shim_support::{runtime, this_user, HANG};
use super::status::{status_report, StatusOptions};
use crate::mcp::shared::identity::DaemonIdentity;
use crate::mcp::shared::preamble::{BuildState, Ready};
use crate::mcp::{CacheMode, ServerConfig};
use std::path::Path;

fn options() -> (tempfile::TempDir, ShimOptions, DaemonIdentity) {
    let root = tempfile::tempdir().unwrap();
    let mut config = ServerConfig::new(root.path().to_path_buf());
    config.cache = CacheMode::Dir(root.path().join("cache"));
    let identity = DaemonIdentity::resolve(&config).unwrap();
    let options = ShimOptions {
        config,
        start_timeout: HANG,
        idle_ttl: None,
        max_clients: None,
        index_idle_secs: None,
        index_idle_pressure_relief: false,
    };
    (root, options, identity)
}

#[test]
fn shim_idle_flag_env_forwarding() {
    let (_root, mut options, identity) = options();
    for (flag, env, expected) in [
        (None, None, "0"),
        (None, Some("7"), "7"),
        (Some("3"), Some("7"), "3"),
        (Some("0"), Some("7"), "0"),
        (Some("0"), Some("bad"), "0"),
        (Some("0"), Some("18446744073709551616"), "0"),
    ] {
        options.index_idle_secs = flag.map(str::to_owned);
        let args = daemon_args(&identity, &options, env).unwrap();
        let log = tempfile::tempfile().unwrap();
        let command = super::spawn::daemon_command(
            Path::new("/bin/prism-mcp"),
            &args,
            Path::new("/runtime/prism-mcpd"),
            log,
        )
        .unwrap();
        let argv: Vec<_> = command
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        let expected = ["--index-idle-secs", expected].map(str::to_owned);
        assert!(
            argv.windows(2).any(|w| w == expected),
            "{flag:?}/{env:?}: {argv:?}"
        );
        assert_eq!(argv.iter().filter(|a| *a == "--index-idle-secs").count(), 1);
        assert!(!argv.iter().any(|a| a == "--index-idle-pressure-relief"));
        assert!(argv
            .windows(2)
            .any(|w| w == ["--key", &identity.key().to_hex()]));
    }
    shim_idle_invalid_values();
}

#[test]
fn shim_idle_invalid_values() {
    let (_root, mut options, identity) = options();
    for bad in [
        "",
        "bad",
        "-1",
        "+1",
        " 1",
        "1.5",
        "18446744073709551616",
        "18446744073709551615",
    ] {
        options.index_idle_secs = Some(bad.into());
        assert!(
            daemon_args(&identity, &options, Some("0")).is_err(),
            "flag {bad}"
        );
        options.index_idle_secs = None;
        assert!(
            daemon_args(&identity, &options, Some(bad)).is_err(),
            "env {bad}"
        );
    }
}

#[test]
fn shim_idle_relief_forwarding_and_applicability() {
    let (_root, mut options, identity) = options();
    options.index_idle_secs = Some("1".into());
    options.index_idle_pressure_relief = true;
    let argv = daemon_args(&identity, &options, None).unwrap().to_args();
    assert_eq!(
        argv.iter()
            .filter(|a| *a == "--index-idle-pressure-relief")
            .count(),
        1
    );
}

#[test]
fn shim_idle_first_spawner_mismatch_once() {
    let (_root, mut options, identity) = options();
    let mut ready = Ready {
        identity: identity.key(),
        pid: 4242,
        start_time: 123,
        generation: 0,
        build: BuildState::Unloaded,
        index_idle_timeout_secs: 7,
        index_idle_pressure_relief: false,
    };
    options.index_idle_secs = Some("3".into());
    let requested = daemon_args(&identity, &options, None).unwrap();
    let lines = ready_lines(&ready, &requested, Path::new("/socket\ninjected"));
    assert_eq!(
        lines
            .iter()
            .filter(|s| s.contains("idle policy mismatch"))
            .count(),
        1,
        "{lines:?}"
    );
    let mismatch = lines
        .iter()
        .find(|s| s.contains("idle policy mismatch"))
        .unwrap();
    for part in [
        "pid 4242",
        "effective index_idle_timeout_secs=7",
        "requested index_idle_timeout_secs=3",
    ] {
        assert!(mismatch.contains(part), "{lines:?}");
    }
    assert!(!mismatch.contains("pressure_relief"));
    assert!(
        lines[0].contains(
            "index unloaded; index_idle_timeout_secs=7; index_idle_pressure_relief=false"
        ),
        "{lines:?}"
    );
    assert!(
        lines.iter().all(|s| !s.contains('\n')),
        "sanitized single lines"
    );
    options.index_idle_secs = Some("7".into());
    let same = daemon_args(&identity, &options, None).unwrap();
    let lines = ready_lines(&ready, &same, Path::new("/socket"));
    assert_eq!(lines.len(), 1, "identical policy has no mismatch");
    assert_eq!(ready.identity, same.key);
    ready.index_idle_pressure_relief = true;
    let lines = ready_lines(&ready, &same, Path::new("/socket"));
    assert_eq!(lines.len(), 2);
    assert!(
        lines[1].contains(
            "effective index_idle_pressure_relief=true; requested index_idle_pressure_relief=false"
        ),
        "{lines:?}"
    );
}

#[test]
fn operator_idle_unloaded_status() {
    for build in [
        BuildState::Idle,
        BuildState::Building,
        BuildState::Ready,
        BuildState::Failed,
        BuildState::Unloaded,
    ] {
        for (clients, pending) in [(0, 0), (1, 0), (0, 1)] {
            for seconds in [0, 7] {
                let (base, paths) = runtime();
                let mut status = fake_status(clients, false);
                status.pending = pending;
                if pending != 0 {
                    status.idle_secs = None;
                }
                status.build = build;
                status.index_idle_timeout_secs = seconds;
                status.index_idle_pressure_relief = seconds != 0;
                let fake = ControlFake::start(&paths, true, always_line(status.encode()));
                let report = status_report(
                    &StatusOptions {
                        config: None,
                        runtime_base: Some(base.path().into()),
                        timeout: HANG,
                    },
                    this_user,
                )
                .unwrap();
                assert!(
                    report.contains(&format!("index: {} ", build.word())),
                    "{report}"
                );
                assert!(
                    report.contains(&format!(
                        "index_idle_timeout_secs={seconds}; index_idle_pressure_relief={}",
                        seconds != 0
                    )),
                    "{report}"
                );
                assert!(
                    report.contains(if clients + pending == 0 {
                        "clientless; idle 7 s"
                    } else {
                        "connected; in use"
                    }),
                    "{report}"
                );
                assert_eq!(
                    fake.lines(),
                    [crate::mcp::shared::control::ControlRequest::Status.encode()]
                );
            }
        }
    }
}

#[test]
fn operator_idle_old_control_unavailable() {
    let old = fake_status(0, false)
        .encode()
        .replacen("\"control\":2", "\"control\":1", 1);
    for reply in [Some(old), None] {
        let (base, paths) = runtime();
        let _fake = ControlFake::start(
            &paths,
            true,
            std::sync::Arc::new(move |_| match &reply {
                Some(line) => FakeReply::Line(line.clone()),
                None => FakeReply::Close,
            }),
        );
        let report = status_report(
            &StatusOptions {
                config: None,
                runtime_base: Some(base.path().into()),
                timeout: HANG,
            },
            this_user,
        )
        .unwrap();
        assert!(
            report.contains("incompatible") && report.contains("matching binary"),
            "{report}"
        );
        assert!(
            !report.contains("index_idle_timeout_secs="),
            "unknown policy must remain unavailable: {report}"
        );
    }
}
