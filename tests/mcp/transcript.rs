//! Scripted, line-interactive `prism-mcp` sessions for transcript tests. A step sends one line
//! and reads exactly one response line back, sends a notification, or edits a fixture file
//! between requests, so stale-index and refresh behavior are reproducible byte for byte.
#![cfg(feature = "mcp")]

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

/// How long a due response, or the tail of output after the last step, may take before the run
/// is declared hung.
pub const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);

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

/// Why the output after the last step could not be collected.
#[derive(Debug)]
pub enum TailError {
    /// The channel stayed open, the child's stdout did not close, and no line arrived in time.
    TimedOut(Duration),
    /// Reading the child's stdout failed; that is not the same as it ending.
    ReadFailed(std::io::Error),
}

impl std::fmt::Display for TailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TailError::TimedOut(after) => write!(
                f,
                "timed out after {after:?} waiting for the child's stdout to close"
            ),
            TailError::ReadFailed(error) => write!(f, "reading the child's stdout failed: {error}"),
        }
    }
}

/// Reads `source` line by line on a thread. A clean end drops the sender (the receiver sees a
/// disconnect); a read error is sent as an `Err` so it cannot be mistaken for the end.
pub fn spawn_line_reader<R: Read + Send + 'static>(
    source: R,
) -> (
    mpsc::Receiver<std::io::Result<Vec<u8>>>,
    std::thread::JoinHandle<()>,
) {
    let (lines_tx, lines_rx) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut source = BufReader::new(source);
        loop {
            let mut line = Vec::new();
            match source.read_until(b'\n', &mut line) {
                Ok(0) => break,
                Err(error) => {
                    let _ = lines_tx.send(Err(error));
                    break;
                }
                Ok(_) => {
                    if lines_tx.send(Ok(line)).is_err() {
                        break;
                    }
                }
            }
        }
    });
    (lines_rx, reader)
}

/// Collects every line still to come after the last step. Anything no step asked for (an answer
/// to a notification, a stray line) is kept, so it shows up in the comparison instead of being
/// silently dropped. Only the channel disconnecting (the reader saw end of file) is a clean end.
pub fn drain_tail(
    lines: &mpsc::Receiver<std::io::Result<Vec<u8>>>,
    timeout: Duration,
    transcript: &mut Vec<u8>,
) -> Result<(), TailError> {
    loop {
        match lines.recv_timeout(timeout) {
            Ok(Ok(extra)) => transcript.extend_from_slice(&extra),
            Ok(Err(error)) => return Err(TailError::ReadFailed(error)),
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
            Err(mpsc::RecvTimeoutError::Timeout) => return Err(TailError::TimedOut(timeout)),
        }
    }
}

/// Kill the child, reap it, and let its reader thread finish (its stdout closes with it).
fn abort(child: &mut Child, reader: std::thread::JoinHandle<()>) {
    let _ = child.kill();
    let _ = child.wait();
    let _ = reader.join();
}

/// Runs `script` against a fresh `prism-mcp --repo <root> --no-cache <mode.flags>` and returns
/// everything it wrote to stdout. Panics, naming the step, when a due response does not arrive.
pub fn run_transcript(root: &Path, mode: &Mode, script: &[Step]) -> Vec<u8> {
    run_transcript_with_timeout(root, mode, script, RESPONSE_TIMEOUT)
        .unwrap_or_else(|failure| panic!("{failure}"))
}

/// `run_transcript` with the hang budget as a parameter. Returns the failure instead of panicking.
pub fn run_transcript_with_timeout(
    root: &Path,
    mode: &Mode,
    script: &[Step],
    timeout: Duration,
) -> Result<Vec<u8>, String> {
    let mut command = Command::new(assert_cmd::cargo::cargo_bin("prism-mcp"));
    command
        .arg("--repo")
        .arg(root)
        .arg("--no-cache")
        .args(mode.flags);
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
    drive(command, mode.name, root, script, timeout)
}

/// Spawns `command` with piped stdin and stdout, plays `script` against it, and collects stdout.
/// The child is killed and reaped on every failure path.
pub fn drive(
    mut command: Command,
    label: &str,
    root: &Path,
    script: &[Step],
    timeout: Duration,
) -> Result<Vec<u8>, String> {
    let stderr_dir = tempfile::tempdir().expect("stderr dir");
    let stderr_path = stderr_dir.path().join("stderr.txt");
    let stderr_file = std::fs::File::create(&stderr_path).expect("stderr file");
    let stderr_text = || std::fs::read_to_string(&stderr_path).unwrap_or_default();

    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(stderr_file));
    let mut child = command.spawn().expect("spawn child");
    let mut stdin = child.stdin.take().expect("child stdin");
    let stdout = child.stdout.take().expect("child stdout");
    let (lines_rx, reader) = spawn_line_reader(stdout);

    let mut transcript = Vec::new();
    for (index, step) in script.iter().enumerate() {
        match step {
            Step::Request(line) => {
                writeln!(stdin, "{line}").expect("write request");
                stdin.flush().expect("flush request");
                match lines_rx.recv_timeout(timeout) {
                    Ok(Ok(response)) => transcript.extend_from_slice(&response),
                    outcome => {
                        let why = match outcome {
                            Ok(Err(error)) => format!("stdout read failed: {error}"),
                            _ => "no response".to_string(),
                        };
                        let stderr = stderr_text();
                        abort(&mut child, reader);
                        return Err(format!(
                            "mode {label}: {why} to step {index}: {line}\nstderr:\n{stderr}"
                        ));
                    }
                }
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
    if let Err(failure) = drain_tail(&lines_rx, timeout, &mut transcript) {
        let stderr = stderr_text();
        abort(&mut child, reader);
        return Err(format!(
            "mode {label}: after the last step, {failure}; output so far was {} bytes\nstderr:\n{stderr}",
            transcript.len()
        ));
    }
    reader.join().expect("stdout reader thread");
    let status = child.wait().expect("wait for child");
    if !status.success() {
        return Err(format!(
            "mode {label}: child exited with {status}\nstderr:\n{}",
            stderr_text()
        ));
    }
    Ok(transcript)
}
