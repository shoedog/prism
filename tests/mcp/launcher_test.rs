//! `scripts/prism-mcp-launch.sh`, the Claude Code plugin's launcher, and its opt-in to the shared
//! daemon (`PRISM_MCP_SHARED`). The argument tests run it against a stand-in that prints its
//! arguments (`tests/fixtures/mcp/launcher/argv-stub.sh`); the last test runs it against the
//! built binary and reaches a shared daemon. The binary is reached through a symbolic link in a
//! plugin root of the test's own, so no test writes a file it then executes.
#![cfg(feature = "mcp")]

use crate::daemon_process_test::{setup, CALLEES, HANG, INIT, INITED};
use crate::shim_process_test::Daemons;
use crate::transcript::spawn_line_reader;
use prism::mcp::shared::runtime_dir::RUNTIME_DIR_ENV;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const SHARED_ENV: &str = "PRISM_MCP_SHARED";
const BIN_ENV: &str = "PRISM_MCP_BIN";

fn launcher() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/prism-mcp-launch.sh")
}

/// A plugin root whose `target/release/prism-mcp` is a symbolic link to `binary`.
fn plugin_root(binary: &Path) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("a plugin root");
    let release = root.path().join("target/release");
    std::fs::create_dir_all(&release).expect("target/release");
    std::os::unix::fs::symlink(binary, release.join("prism-mcp")).expect("link the binary");
    root
}

/// The launcher under `/bin/bash` (on macOS bash 3.2, the oldest a user has) with the stand-in
/// as the binary, `shared` as `PRISM_MCP_SHARED` (`None`: unset) and `args` after the
/// repository.
fn launch(shared: Option<&str>, args: &[&str]) -> Output {
    let stub =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mcp/launcher/argv-stub.sh");
    let root = plugin_root(&stub);
    let mut command = Command::new("/bin/bash");
    command
        .arg(launcher())
        .arg("/some/repo")
        .args(args)
        .env("CLAUDE_PLUGIN_ROOT", root.path())
        .env_remove(SHARED_ENV)
        .env_remove(BIN_ENV)
        .stdin(Stdio::null());
    if let Some(value) = shared {
        command.env(SHARED_ENV, value);
    }
    command.output().expect("run the launcher")
}

/// The arguments the stand-in was given, one per line.
fn argv(output: &Output) -> Vec<String> {
    assert!(
        output.status.success(),
        "the launcher failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone())
        .expect("UTF-8")
        .lines()
        .map(str::to_string)
        .collect()
}

#[test]
fn without_the_variable_the_arguments_pass_through_unchanged() {
    assert_eq!(
        argv(&launch(None, &["--cache-dir", "/a dir"])),
        ["--repo", "/some/repo", "--cache-dir", "/a dir"]
    );
}

#[test]
fn prism_mcp_shared_1_adds_shared_after_the_repository() {
    assert_eq!(
        argv(&launch(Some("1"), &["--cache-dir", "/a dir"])),
        ["--repo", "/some/repo", "--shared", "--cache-dir", "/a dir"]
    );
    assert_eq!(
        argv(&launch(Some("1"), &[])),
        ["--repo", "/some/repo", "--shared"]
    );
}

#[test]
fn zero_or_empty_changes_nothing() {
    for value in ["0", ""] {
        assert_eq!(
            argv(&launch(Some(value), &["--eager"])),
            ["--repo", "/some/repo", "--eager"],
            "PRISM_MCP_SHARED={value:?}"
        );
    }
}

#[test]
fn an_explicit_shared_or_standalone_argument_wins_over_the_variable() {
    assert_eq!(
        argv(&launch(Some("1"), &["--standalone"])),
        ["--repo", "/some/repo", "--standalone"]
    );
    assert_eq!(
        argv(&launch(Some("1"), &["--cache-dir", "/c", "--shared"])),
        ["--repo", "/some/repo", "--cache-dir", "/c", "--shared"]
    );
}

#[test]
fn any_other_value_is_refused_before_anything_runs() {
    for value in ["yes", "true", "2", " 1"] {
        let output = launch(Some(value), &[]);
        assert_eq!(output.status.code(), Some(2), "PRISM_MCP_SHARED={value:?}");
        assert!(output.stdout.is_empty(), "the binary ran for {value:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("PRISM_MCP_SHARED must be 1") && stderr.contains(&format!("'{value}'")),
            "{stderr}"
        );
    }
}

/// Every combination of the variable and an explicit mode argument (review round 1, sol S7): an
/// explicit `--shared` or `--standalone` wins over a valid value, and a value the launcher does
/// not accept is refused even then, because the variable is checked first.
#[test]
fn the_variable_and_an_explicit_argument_in_every_combination() {
    for value in [None, Some(""), Some("0"), Some("1"), Some("yes")] {
        for explicit in [None, Some("--shared"), Some("--standalone")] {
            let args: Vec<&str> = explicit.into_iter().collect();
            let output = launch(value, &args);
            let what = format!("PRISM_MCP_SHARED={value:?} with {explicit:?}");
            if value == Some("yes") {
                assert_eq!(output.status.code(), Some(2), "{what}");
                assert!(output.stdout.is_empty(), "{what}: the binary ran");
                continue;
            }
            let mut expected = vec!["--repo", "/some/repo"];
            match explicit {
                Some(flag) => expected.push(flag),
                None if value == Some("1") => expected.push("--shared"),
                None => {}
            }
            assert_eq!(argv(&output), expected, "{what}");
        }
    }
}

fn argv_stub() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mcp/launcher/argv-stub.sh")
}

/// `PRISM_MCP_BIN` names the binary ahead of the plugin root, whose binary here would fail.
#[test]
fn prism_mcp_bin_wins_over_the_plugin_root() {
    let root = plugin_root(Path::new("/usr/bin/false"));
    let output = Command::new("/bin/bash")
        .arg(launcher())
        .arg("/some/repo")
        .env("CLAUDE_PLUGIN_ROOT", root.path())
        .env(BIN_ENV, argv_stub())
        .env_remove(SHARED_ENV)
        .stdin(Stdio::null())
        .output()
        .expect("run the launcher");
    assert_eq!(argv(&output), ["--repo", "/some/repo"]);
}

/// A `PRISM_MCP_BIN` that is missing or not an executable file is refused, never skipped for
/// another binary.
#[test]
fn a_prism_mcp_bin_that_is_not_an_executable_file_is_refused() {
    let root = plugin_root(&argv_stub());
    let dir = tempfile::tempdir().expect("a directory");
    let plain = dir.path().join("plain");
    std::fs::write(&plain, "not a program").expect("a plain file");
    let missing = dir.path().join("missing");
    for bin in [missing.as_path(), dir.path(), plain.as_path()] {
        let output = Command::new("/bin/bash")
            .arg(launcher())
            .arg("/some/repo")
            .env("CLAUDE_PLUGIN_ROOT", root.path())
            .env(BIN_ENV, bin)
            .env_remove(SHARED_ENV)
            .stdin(Stdio::null())
            .output()
            .expect("run the launcher");
        assert_eq!(output.status.code(), Some(127), "{bin:?}");
        assert!(output.stdout.is_empty(), "a binary ran for {bin:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("PRISM_MCP_BIN is not an executable file"),
            "{stderr}"
        );
    }
}

fn parent(path: &Path) -> &Path {
    path.parent().expect("a parent directory")
}

/// This repo's tracked `.codex/config.toml` server, run as Codex runs it: its command and
/// arguments, started in the session's working directory. Git looks no higher than `ceiling`, so
/// a temporary directory inside some other checkout still counts as outside one.
fn run_codex_config_server(cwd: &Path, ceiling: &Path) -> Output {
    let text =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(".codex/config.toml"))
            .expect("read .codex/config.toml");
    let config: toml::Table = text.parse().expect("parse .codex/config.toml");
    let server = &config["mcp_servers"]["prism"];
    let env = server["env"].as_table().expect("an env table");
    assert!(
        env.contains_key(BIN_ENV) && env.contains_key(RUNTIME_DIR_ENV),
        "{env:?}"
    );
    let args: Vec<&str> = server["args"]
        .as_array()
        .expect("args")
        .iter()
        .map(|arg| arg.as_str().expect("a string argument"))
        .collect();
    Command::new(server["command"].as_str().expect("a command"))
        .args(args)
        .current_dir(cwd)
        .env("GIT_CEILING_DIRECTORIES", ceiling)
        .env(BIN_ENV, argv_stub())
        .env_remove(SHARED_ENV)
        .env_remove("CLAUDE_PLUGIN_ROOT")
        .stdin(Stdio::null())
        .output()
        .expect("run the configured server")
}

/// From a subdirectory of a checkout, the Codex config indexes that checkout's root through the
/// checkout's own launcher (linked here), in shared mode.
#[test]
fn the_codex_config_indexes_the_checkout_it_starts_in() {
    let checkout = tempfile::tempdir().expect("a checkout");
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(checkout.path())
        .status()
        .expect("git init");
    assert!(status.success());
    std::fs::create_dir_all(checkout.path().join("scripts")).expect("scripts");
    std::os::unix::fs::symlink(
        launcher(),
        checkout.path().join("scripts/prism-mcp-launch.sh"),
    )
    .expect("link the launcher");
    let nested = checkout.path().join("src/deep");
    std::fs::create_dir_all(&nested).expect("a subdirectory");
    let root = checkout.path().canonicalize().expect("canonical checkout");
    assert_eq!(
        argv(&run_codex_config_server(&nested, parent(checkout.path()))),
        ["--repo", root.to_str().expect("UTF-8 path"), "--shared"]
    );
}

/// Outside any checkout the Codex config fails before a binary runs.
#[test]
fn outside_a_checkout_the_codex_config_fails() {
    let dir = tempfile::tempdir().expect("a directory");
    let output = run_codex_config_server(dir.path(), parent(dir.path()));
    assert!(
        !output.status.success(),
        "the server started outside a checkout"
    );
    assert!(output.stdout.is_empty(), "a binary ran outside a checkout");
}

/// A launcher process, killed and reaped when the test ends however it ends.
struct Launched(std::process::Child);

impl Drop for Launched {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// The launcher run as the plugin runs it (directly, through its `#!` line) with the built
/// binary and `PRISM_MCP_SHARED=1` starts a shim that reaches a shared daemon and serves a tool
/// call through it.
#[test]
fn with_the_variable_set_the_plugin_s_server_is_a_shim_of_the_shared_daemon() {
    let setup = setup();
    let _daemons = Daemons(&setup.paths);
    let root = plugin_root(Path::new(env!("CARGO_BIN_EXE_prism-mcp")));
    let stderr_dir = tempfile::tempdir().expect("a directory for standard error");
    let stderr_path = stderr_dir.path().join("stderr.txt");
    let stderr = || std::fs::read_to_string(&stderr_path).unwrap_or_default();
    let mut launched = Launched(
        Command::new(launcher())
            .arg(setup.repo.path())
            .arg("--cache-dir")
            .arg(setup.cache.path())
            .args(["--daemon-start-timeout", "60"])
            .env("CLAUDE_PLUGIN_ROOT", root.path())
            .env(SHARED_ENV, "1")
            .env_remove(BIN_ENV)
            .env(RUNTIME_DIR_ENV, setup.base.path())
            .env_remove("PRISM_MCP_STRUCTURED_CONTENT")
            .env_remove("PRISM_MCP_CONCISE_SHAPE")
            .env_remove("PRISM_MCP_MAX_RESULT_CHARS")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(
                File::create(&stderr_path).expect("stderr file"),
            ))
            .spawn()
            .expect("start the launcher"),
    );
    let mut stdin = launched.0.stdin.take().expect("stdin");
    let (lines, _reader) = spawn_line_reader(launched.0.stdout.take().expect("stdout"));
    let request = |stdin: &mut std::process::ChildStdin, line: &str| {
        writeln!(stdin, "{line}").expect("write a request");
        stdin.flush().expect("flush");
        match lines.recv_timeout(HANG) {
            Ok(Ok(response)) => {
                serde_json::from_slice::<serde_json::Value>(&response).expect("a JSON response")
            }
            other => panic!("no response: {other:?}; stderr: {}", stderr()),
        }
    };
    assert!(request(&mut stdin, INIT)["result"].is_object());
    writeln!(stdin, "{INITED}").expect("write the notification");
    let answer = request(&mut stdin, CALLEES);
    assert_eq!(answer["result"]["isError"], false, "{answer}");
    drop(stdin);
    let status = launched.0.wait().expect("the shim's status");
    assert!(status.success(), "{status:?}: {}", stderr());
    assert!(
        stderr().contains("prism-mcp: shared daemon pid "),
        "no daemon was reached: {}",
        stderr()
    );
}
