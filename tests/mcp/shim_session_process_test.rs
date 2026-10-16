//! Several `prism-mcp --shared` processes and one daemon (spec §4.1 step 6, §4.6, §5; the shim's
//! halves of §9 tests 8 and 15; tests 7 and 9 through real shims), and the spec §5 equivalence:
//! one scripted session, run standalone and through a shim, gives the same bytes.
#![cfg(feature = "mcp")]

use crate::daemon_process_test::{setup, Daemon, CALLEES};
use crate::shim_process_test::{end_daemons, owner_is_free, Daemons, ShimProcess};
use crate::transcript::{drive, script, write_fixture, RESPONSE_TIMEOUT};
use prism::mcp::shared::runtime_dir::RUNTIME_DIR_ENV;
use std::process::Command;

/// Spec §9 tests 8 and 15, the shim's halves: eight shims start at once while a daemon started on
/// its own (as a dead starter's child would be) starts too. All eight are admitted by one daemon
/// and each is served; every other daemon process ends having served nothing: the orphan, if it
/// lost, left silently, and the key's log, where every daemon a shim started writes, has one
/// `serves` line at most, the winner's (review round 1, Opus F5).
#[test]
fn shims_started_together_with_an_orphan_daemon_share_one_daemon() {
    let setup = setup();
    let _daemons = Daemons(&setup.paths);
    let key = setup.key.to_hex();
    let mut orphan = Daemon::start(&setup, &["--key", &key]);
    let mut shims: Vec<ShimProcess> = (0..8)
        .map(|_| ShimProcess::start(&setup, &[], &[]))
        .collect();
    for shim in &mut shims {
        shim.handshake();
        let answer = shim.request(CALLEES);
        assert_eq!(answer["result"]["isError"], false, "{answer}");
    }
    let pids: Vec<u32> = shims.iter().map(ShimProcess::daemon_pid).collect();
    assert!(pids.iter().all(|pid| *pid == pids[0]), "{pids:?}");
    if orphan.pid() != pids[0] {
        let (status, stderr) = orphan.ended();
        assert!(status.success(), "{status:?} {stderr}");
        assert_eq!(stderr, "", "the orphan lost the key and left silently");
    }
    let log = std::fs::read_to_string(setup.paths.log_file()).unwrap_or_default();
    let serves: Vec<&str> = log.lines().filter(|l| l.contains(" serves ")).collect();
    let expected: Vec<String> = if orphan.pid() == pids[0] {
        Vec::new()
    } else {
        vec![format!("pid {} serves", pids[0])]
    };
    assert_eq!(serves.len(), expected.len(), "{log}");
    for (line, start) in serves.iter().zip(&expected) {
        assert!(line.contains(start.as_str()), "{line} in {log}");
    }
    drop(shims);
    end_daemons(&setup.paths);
    assert!(owner_is_free(&setup.paths));
}

/// Spec §9 test 9 through real shims: two shims whose environments ask for different output
/// settings are served by one daemon, each in its own shape (the default path omits
/// `structuredContent`; `always` keeps it).
#[test]
fn two_shims_with_different_output_settings_get_their_own_shapes_from_one_daemon() {
    let setup = setup();
    let _daemons = Daemons(&setup.paths);
    let mut always = ShimProcess::start(&setup, &[], &[("PRISM_MCP_STRUCTURED_CONTENT", "always")]);
    let mut default = ShimProcess::start(&setup, &[], &[]);
    always.handshake();
    default.handshake();
    let with = always.request(CALLEES);
    let without = default.request(CALLEES);
    assert_eq!(always.daemon_pid(), default.daemon_pid());
    assert!(with["result"]["structuredContent"].is_object(), "{with}");
    assert!(
        without["result"].get("structuredContent").is_none(),
        "{without}"
    );
}

/// Spec §5 ("preserved, for the same indexed snapshot, the same connection settings and the same
/// readiness state") and the lane README's note for S5a: the transcript script (framing errors,
/// the handshake, every tool, an edit, a refresh) run through a shim gives byte for byte what the
/// same script gives a standalone lazy server with a cache, for the default settings and for
/// `always`/`legacy`.
#[test]
fn a_scripted_session_through_a_shim_matches_the_standalone_server_byte_for_byte() {
    for env in [
        &[][..],
        &[
            ("PRISM_MCP_STRUCTURED_CONTENT", "always"),
            ("PRISM_MCP_CONCISE_SHAPE", "legacy"),
        ][..],
    ] {
        let transcript = |shared: bool| {
            let repo = tempfile::tempdir().expect("a repository");
            write_fixture(repo.path());
            let cache = tempfile::tempdir().expect("a cache");
            let base = tempfile::Builder::new()
                .prefix("s5a")
                .tempdir_in("/tmp")
                .expect("a runtime base");
            let mut command = Command::new(env!("CARGO_BIN_EXE_prism-mcp"));
            command
                .arg("--repo")
                .arg(repo.path())
                .arg("--cache-dir")
                .arg(cache.path())
                .args(["--first-call-wait", "60"])
                .args(if shared {
                    &["--daemon-start-timeout", "60"][..]
                } else {
                    &[][..]
                })
                .arg(if shared { "--shared" } else { "--standalone" })
                .env(RUNTIME_DIR_ENV, base.path());
            for name in [
                "PRISM_MCP_STRUCTURED_CONTENT",
                "PRISM_MCP_CONCISE_SHAPE",
                "PRISM_MCP_MAX_RESULT_CHARS",
            ] {
                command.env_remove(name);
            }
            for (name, value) in env {
                command.env(name, value);
            }
            let label = if shared { "shared" } else { "standalone" };
            let output = drive(command, label, repo.path(), &script(), RESPONSE_TIMEOUT);
            // The daemon of a shared run is ended with its runtime directory.
            let key = {
                let mut cfg = prism::mcp::ServerConfig::new(repo.path().to_path_buf());
                cfg.cache = prism::mcp::CacheMode::Dir(cache.path().to_path_buf());
                prism::mcp::shared::identity::DaemonIdentity::resolve(&cfg)
                    .expect("an identity")
                    .key()
            };
            let paths = prism::mcp::shared::paths::DaemonPaths::new(
                &base
                    .path()
                    .join(prism::mcp::shared::runtime_dir::RUNTIME_DIR_NAME),
                &key,
            )
            .expect("paths");
            end_daemons(&paths);
            output.unwrap_or_else(|failure| panic!("{failure}"))
        };
        let standalone = transcript(false);
        let shared = transcript(true);
        assert!(
            standalone == shared,
            "{env:?}: the transcripts differ\nstandalone:\n{}\nshared:\n{}",
            String::from_utf8_lossy(&standalone),
            String::from_utf8_lossy(&shared)
        );
    }
}
