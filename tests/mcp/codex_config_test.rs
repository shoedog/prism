//! This repo's tracked `.codex/config.toml` prism server, run as Codex runs it: its command and
//! arguments, started in the session's working directory, with only a minimal environment plus
//! the configured `env` table (Codex filters the environment it gives MCP servers). The binary is
//! the argument-printing stand-in `tests/fixtures/mcp/launcher/argv-stub.sh`.
#![cfg(feature = "mcp")]

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const BIN_ENV: &str = "PRISM_MCP_BIN";

fn argv_stub() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mcp/launcher/argv-stub.sh")
}

fn server() -> toml::Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".codex/config.toml");
    let text = std::fs::read_to_string(path).expect("read .codex/config.toml");
    let config: toml::Table = text.parse().expect("parse .codex/config.toml");
    config["mcp_servers"]["prism"].clone()
}

/// `git` with a minimal environment, so an inherited `GIT_DIR` or user config cannot redirect it.
fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .stdin(Stdio::null())
        .status()
        .expect("run git");
    assert!(status.success(), "git {args:?}");
}

/// Runs the configured server in `cwd`. Git looks no higher than `ceiling`, so a temporary
/// directory inside some other checkout still counts as outside one. The configured binary is
/// replaced by the stand-in; every other configured variable is passed as configured.
fn run_server(cwd: &Path, ceiling: &Path) -> Output {
    let server = server();
    let args: Vec<&str> = server["args"]
        .as_array()
        .expect("args")
        .iter()
        .map(|arg| arg.as_str().expect("a string argument"))
        .collect();
    let mut command = Command::new(server["command"].as_str().expect("a command"));
    command
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", ceiling)
        .env("GIT_CEILING_DIRECTORIES", ceiling)
        .stdin(Stdio::null());
    for (key, value) in server["env"].as_table().expect("an env table") {
        command.env(key, value.as_str().expect("a string value"));
    }
    command.env(BIN_ENV, argv_stub());
    command.output().expect("run the configured server")
}

fn argv(output: &Output) -> Vec<String> {
    assert!(
        output.status.success(),
        "the server failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone())
        .expect("UTF-8")
        .lines()
        .map(str::to_string)
        .collect()
}

fn parent(path: &Path) -> &Path {
    path.parent().expect("a parent directory")
}

fn canonical(path: &Path) -> String {
    path.canonicalize()
        .expect("canonical path")
        .to_str()
        .expect("UTF-8 path")
        .to_string()
}

/// The configured binary is an absolute path, and the shared daemon's runtime directory is set.
#[test]
fn the_config_names_an_absolute_binary_and_a_runtime_directory() {
    let server = server();
    let env = server["env"].as_table().expect("an env table");
    let bin = env[BIN_ENV].as_str().expect("PRISM_MCP_BIN");
    assert!(Path::new(bin).is_absolute(), "{bin}");
    assert!(env.contains_key("PRISM_MCP_RUNTIME_DIR"), "{env:?}");
}

/// From a subdirectory of a checkout, the server indexes that checkout's root, in shared mode.
#[test]
fn the_server_indexes_the_checkout_it_starts_in() {
    let checkout = tempfile::tempdir().expect("a checkout");
    git(checkout.path(), &["init", "-q"]);
    let nested = checkout.path().join("src/deep");
    std::fs::create_dir_all(&nested).expect("a subdirectory");
    assert_eq!(
        argv(&run_server(&nested, parent(checkout.path()))),
        ["--repo", &canonical(checkout.path()), "--shared"]
    );
}

/// In a linked worktree, the server indexes the worktree, not the checkout it was added from.
#[test]
fn a_linked_worktree_gets_its_own_index() {
    let dir = tempfile::tempdir().expect("a directory");
    let main = dir.path().join("main");
    std::fs::create_dir_all(&main).expect("a main checkout");
    git(&main, &["init", "-q"]);
    git(
        &main,
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@example.invalid",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "init",
        ],
    );
    let worktree = dir.path().join("wt");
    git(
        &main,
        &[
            "worktree",
            "add",
            "-q",
            "--detach",
            worktree.to_str().unwrap(),
        ],
    );
    let nested = worktree.join("sub");
    std::fs::create_dir_all(&nested).expect("a subdirectory");
    assert_eq!(
        argv(&run_server(&nested, dir.path())),
        ["--repo", &canonical(&worktree), "--shared"]
    );
}

/// Outside any checkout the server fails with git's diagnostic before any binary runs.
#[test]
fn outside_a_checkout_the_server_fails() {
    let dir = tempfile::tempdir().expect("a directory");
    let output = run_server(dir.path(), parent(dir.path()));
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty(), "a binary ran outside a checkout");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not a git repository"), "{stderr}");
}
