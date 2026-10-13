//! Scripted, line-interactive `prism-mcp` sessions for transcript tests. A step sends one line
//! and reads exactly one response line back, sends a notification, or edits a fixture file
//! between requests, so stale-index and refresh behavior are reproducible byte for byte.
#![cfg(feature = "mcp")]

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

/// How long a due response may take before the run is declared hung.
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);

pub enum Step {
    /// Send this line and read exactly one response line.
    Request(String),
    /// Send this line; the server must not answer it.
    Notify(String),
    /// Overwrite a fixture file, path relative to the repository root.
    Edit {
        file: &'static str,
        contents: &'static str,
    },
}

pub struct Mode {
    pub name: &'static str,
    pub flags: &'static [&'static str],
    pub env: &'static [(&'static str, &'static str)],
}

const FIXTURE: &[(&str, &str)] = &[
    ("util.py", "def helper():\n    return 1\n"),
    (
        "main.py",
        "from util import helper\n\ndef run():\n    return helper()\n",
    ),
    (
        "app.py",
        "def f():\n    user = input()\n    value = user\n    sink(value)\n",
    ),
];

pub fn write_fixture(root: &Path) {
    for (name, contents) in FIXTURE {
        std::fs::write(root.join(name), contents).expect("write fixture file");
    }
}

fn call(id: u64, name: &str, arguments: Value) -> Step {
    Step::Request(
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": { "name": name, "arguments": arguments }
        })
        .to_string(),
    )
}

/// Twenty-five responses: framing and lifecycle errors, the handshake, every tool once, input
/// errors, both agent views, then an edit that makes the index stale, a refresh, and a re-query.
pub fn script() -> Vec<Step> {
    let run = || json!({"kind": "symbol", "name": "run", "file": "main.py"});
    let helper = || json!({"kind": "symbol", "name": "helper", "file": "util.py"});
    vec![
        Step::Request("this is not json".into()),
        Step::Request("[]".into()),
        Step::Request(r#"{"jsonrpc":"2.0","id":90,"method":"tools/list"}"#.into()),
        Step::Request(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"golden","version":"0"}}}"#
                .into(),
        ),
        Step::Notify(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#.into()),
        Step::Request(r#"{"jsonrpc":"2.0","id":2,"method":"ping"}"#.into()),
        Step::Request(r#"{"jsonrpc":"2.0","id":3,"method":"tools/list"}"#.into()),
        call(4, "nav_repo_map", json!({})),
        call(5, "nav_callees", json!({"seed": run()})),
        call(6, "nav_callers", json!({"seed": helper()})),
        call(7, "nav_ego_graph", json!({"seed": run()})),
        call(8, "nav_nodes_at", json!({"file": "main.py", "line": 4})),
        call(9, "nav_symbol_spans", json!({"seed": run()})),
        call(10, "nav_module_deps", json!({"file": "main.py"})),
        call(
            11,
            "taint_reaches",
            json!({
                "sources": [{"kind": "loc", "file": "app.py", "line": 2}],
                "sinks": [{"kind": "loc", "file": "app.py", "line": 4}],
                "verbosity": "detailed"
            }),
        ),
        call(12, "no_such_tool", json!({})),
        Step::Request(
            r#"{"jsonrpc":"2.0","id":13,"method":"tools/call","params":{"name":"nav_callees","arguments":[]}}"#
                .into(),
        ),
        call(14, "nav_callees", json!({"seed": {"kind": "bogus"}})),
        call(15, "refresh_index", json!({"x": 1})),
        call(
            16,
            "nav_callees",
            json!({"seed": run(), "format": "agent_json", "profile": "dependencies"}),
        ),
        call(
            17,
            "nav_callees",
            json!({"seed": run(), "format": "agent_markdown"}),
        ),
        Step::Edit {
            file: "util.py",
            contents: "def helper():\n    return 1\n\n\ndef extra():\n    return helper()\n",
        },
        call(18, "nav_callers", json!({"seed": helper()})),
        call(19, "refresh_index", json!({})),
        call(20, "nav_callers", json!({"seed": helper()})),
        Step::Request(r#"{"jsonrpc":"2.0","id":21,"method":"no/such/method"}"#.into()),
        Step::Request(r#"{"jsonrpc":"2.0","method":"tools/list"}"#.into()),
    ]
}

/// Runs `script` against a fresh `prism-mcp --repo <root> --no-cache <mode.flags>` and returns
/// everything it wrote to stdout. Panics, naming the step, when a due response does not arrive.
pub fn run_transcript(root: &Path, mode: &Mode, script: &[Step]) -> Vec<u8> {
    let stderr_dir = tempfile::tempdir().expect("stderr dir");
    let stderr_path = stderr_dir.path().join("stderr.txt");
    let stderr_file = std::fs::File::create(&stderr_path).expect("stderr file");
    let stderr_text = || std::fs::read_to_string(&stderr_path).unwrap_or_default();

    let mut command = Command::new(assert_cmd::cargo::cargo_bin("prism-mcp"));
    command
        .arg("--repo")
        .arg(root)
        .arg("--no-cache")
        .args(mode.flags)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(stderr_file));
    for key in [
        "PRISM_MCP_STRUCTURED_CONTENT",
        "PRISM_MCP_CONCISE_SHAPE",
        "PRISM_MCP_MAX_RESULT_CHARS",
    ] {
        command.env_remove(key);
    }
    for (key, value) in mode.env {
        command.env(key, value);
    }
    let mut child = command.spawn().expect("spawn prism-mcp");
    let mut stdin = child.stdin.take().expect("child stdin");
    let stdout = child.stdout.take().expect("child stdout");

    let (lines_tx, lines_rx) = mpsc::channel::<Vec<u8>>();
    let reader = std::thread::spawn(move || {
        let mut stdout = BufReader::new(stdout);
        loop {
            let mut line = Vec::new();
            match stdout.read_until(b'\n', &mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if lines_tx.send(line).is_err() {
                        break;
                    }
                }
            }
        }
    });

    let mut transcript = Vec::new();
    for (index, step) in script.iter().enumerate() {
        match step {
            Step::Request(line) => {
                writeln!(stdin, "{line}").expect("write request");
                stdin.flush().expect("flush request");
                let response = lines_rx.recv_timeout(RESPONSE_TIMEOUT).unwrap_or_else(|_| {
                    panic!(
                        "mode {}: no response to step {index}: {line}\nstderr:\n{}",
                        mode.name,
                        stderr_text()
                    )
                });
                transcript.extend_from_slice(&response);
            }
            Step::Notify(line) => {
                writeln!(stdin, "{line}").expect("write notification");
                stdin.flush().expect("flush notification");
            }
            Step::Edit { file, contents } => {
                std::fs::write(root.join(file), contents).expect("edit fixture file");
            }
        }
    }
    drop(stdin);
    // Anything no step asked for (an answer to a notification, a stray line) is kept, so it
    // shows up in the comparison instead of being silently dropped.
    while let Ok(extra) = lines_rx.recv_timeout(RESPONSE_TIMEOUT) {
        transcript.extend_from_slice(&extra);
    }
    reader.join().expect("stdout reader thread");
    let status = child.wait().expect("wait for prism-mcp");
    assert!(
        status.success(),
        "mode {}: prism-mcp exited with {status}\nstderr:\n{}",
        mode.name,
        stderr_text()
    );
    transcript
}
