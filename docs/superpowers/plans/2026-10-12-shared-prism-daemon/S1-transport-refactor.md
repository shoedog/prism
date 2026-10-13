# S1 — Transport and Framing Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the MCP transport servable over a Unix socket with per-connection output settings and a control path that needs no index, while the in-process (`--standalone`) server's stdout stays byte-for-byte what it is on `main` today.

**Architecture:** The line framing moves to `src/mcp/framing.rs` as one generic `LineTransport<R, W>`; stdio and Unix-socket transports are two instantiations of it. `handle_message` splits into `handle_control` (lifecycle, listing, call validation; no runtime) and three dispatch functions (`dispatch_tool_read` on shared access, `dispatch_tool_refresh` and `dispatch_tool_auto_refresh` on exclusive access). Output settings become an explicit `ConnectionSettings` value. The in-process server composes these exactly as today; the daemon (S3a onward) will compose them under its own locks.

**Tech Stack:** Rust 2021, `std::os::unix::net::UnixStream`, `serde_json`, `assert_cmd`/`tempfile` for tests. No new dependency.

**Spec:** `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` (§4.5, §5, §9 unit list, §12 row S1). Lane plan: `docs/superpowers/plans/2026-10-12-shared-prism-daemon/README.md`.

## Global Constraints

- All work is under `--features mcp`. Build and test with `cargo test --features mcp`; the default build must still compile (`cargo check`).
- **Standalone stdout is byte-for-byte unchanged** against the post-#356 implementation (spec §4.5). Proof has two parts, both mandatory: the golden transcripts recorded in Task 1 before any `src/` change stay unmodified, and `src/mcp/transport_tests.rs` plus `src/mcp/owner_transport_tests.rs` have a zero-line diff against the base.
- The one intentional observable change: an invalid `PRISM_MCP_MAX_RESULT_CHARS`, `PRISM_MCP_STRUCTURED_CONTENT` or `PRISM_MCP_CONCISE_SHAPE` value warns on **stderr once per server** instead of once per `tools/call`. Stdout is unaffected.
- Frame limit stays 1 MiB (`MAX_REQUEST_BYTES = 1 << 20`). The request-id limit, envelope reserve and every result-shaping constant are unchanged.
- The in-process server keeps `LazySessionProvider` and the `SessionRuntime` trait unchanged (spec §4.2). Do not add, remove or rename a `SessionRuntime` method.
- Unix-socket code is `#[cfg(unix)]`. Nothing outside `#[cfg(unix)]` may name `UnixStream`.
- Keep files under 600 lines where this slice creates them. `transport.rs` and `transport_tests.rs` are already over; do not grow `transport_tests.rs` at all, and put new tests in new files.
- `BTreeMap`/`BTreeSet` for anything that reaches output. Line numbers are 1-indexed.
- No new clippy warning in any file this slice touches (the base already prints 259 warning lines; CI does not deny warnings).

## Review Focus

Conditions the spec implies that a person will hit, each pinned by a test in the task named:

1. **A socket peer disconnects in the middle of a frame.** The connection must end; the process must not panic or die on `SIGPIPE`. → Task 4, `a_peer_that_disconnects_mid_frame_ends_the_connection_without_a_panic`.
2. **A client pipelines many requests before reading any response.** Every request is answered, in order, with the same bytes stdio framing produces. → Task 4, `a_pipelined_session_over_a_unix_socket_matches_stdio_framing_byte_for_byte`.
3. **A frame of exactly 1 MiB arriving in socket-sized chunks is accepted; one byte more is rejected and the stream resyncs.** → Task 3, cases `exactly the limit is accepted` and `one byte over is malformed and the stream resyncs`.
4. **An invalid output setting in the environment.** The server still serves with the default and says so once, not on every call. → Task 4, `invalid_output_env_value_warns_once_per_server`.
5. **Multi-byte UTF-8 split across socket reads.** The line is decoded intact, not reported malformed. → Task 3, case `multi-byte text split across reads`.

## File Structure

| File | Responsibility | Change |
|---|---|---|
| `src/mcp/framing.rs` | Newline-delimited JSON framing: `ReadOutcome`, `Transport`, `LineTransport<R, W>`, `UnixStreamTransport`, `InMemoryTransport`, `MAX_REQUEST_BYTES` | Create (Task 2 moves code in; Task 3 adds the socket type) |
| `src/mcp/framing_tests.rs` | One framing suite run over an in-memory reader and a Unix socket | Create (Task 3) |
| `src/mcp/settings.rs` | `ConnectionSettings` (result cap, structured-content mode, concise-shape mode) | Create (Task 4) |
| `src/mcp/transport.rs` | Serve loop, `handle_control`, `dispatch_tool_*`, `SessionRuntime` and its impls, result helpers | Modify (Tasks 2, 4) |
| `src/mcp/transport_split_tests.rs` | Tests for the split, the explicit settings, and a session over a Unix socket | Create (Task 4) |
| `src/mcp/mod.rs` | Module list | Modify (Tasks 2, 4) |
| `src/mcp/transport_tests.rs`, `src/mcp/owner_transport_tests.rs` | The existing corpus | **Must not change** |
| `tests/mcp/transcript.rs` | Line-interactive `prism-mcp` session driver | Create (Task 1) |
| `tests/mcp/golden_transcript_test.rs` | Golden comparison in five modes | Create (Task 1) |
| `tests/mcp/settings_test.rs` | Binary-level test for the stderr warning cadence | Create (Task 4) |
| `tests/mcp/main.rs` | Umbrella module list | Modify (Tasks 1, 4) |
| `tests/fixtures/mcp/golden/*.jsonl` | Five recorded transcripts | Create (Task 1), then frozen |
| `CLAUDE.md` | MCP Adapter notes | Modify (Task 5) |

Names the existing test corpus resolves through `use super::*` from `transport.rs`, which must stay nameable there in test builds: `StructuredContentMode`, `MAX_RESULT_CHARS_FLOOR`, `FreshnessProbe`, `FreshnessReport`, `FRESHNESS_RESERVE_BYTES`, `ToolRegistry`, `AutoRefreshSummary`, `RefreshSummary`, `RefreshVerification`, `StartupMode`, `MAX_REQUEST_BYTES`, `MAX_ID_BYTES`, `ENVELOPE_RESERVE`, `AUTO_REFRESH_RESERVE_BYTES`, `MIN_MUTATING_TOOL_CAP_BYTES`, `Lifecycle`, `Dispatch`, `SessionRuntime`, `StdioTransport`, `InMemoryTransport`, `serve_session`, `serve_runtime`, `handle_message`, `call_tool_response_with_cap`, `call_tool_response_with_cap_and_mode`, `build_failure_result`, `success_response`, `json!`, `Value`. When an edit makes one of these unused in non-test code, gate its import with `#[cfg(test)]`; never delete it and never edit the corpus to compensate.

---

### Task 1: Golden transcripts recorded on the base

Records what the in-process server writes today, before any `src/` change, so every later task is compared against the base in the same environment. This commit touches no file under `src/`.

**Files:**
- Create: `tests/mcp/transcript.rs`
- Create: `tests/mcp/golden_transcript_test.rs`
- Create: `tests/fixtures/mcp/golden/eager-default.jsonl`, `lazy-default.jsonl`, `eager-always-legacy.jsonl`, `eager-auto-full.jsonl`, `eager-auto-incremental-cap.jsonl` (generated in Step 4)
- Modify: `tests/mcp/main.rs`

**Interfaces:**
- Consumes: the `prism-mcp` binary through `assert_cmd::cargo::cargo_bin("prism-mcp")`.
- Produces (used again by S5a for standalone-versus-shared equivalence):
  - `pub enum Step { Request(String), Notify(String), Edit { file: &'static str, contents: &'static str } }`
  - `pub struct Mode { pub name: &'static str, pub flags: &'static [&'static str], pub env: &'static [(&'static str, &'static str)] }`
  - `pub fn write_fixture(root: &std::path::Path)`
  - `pub fn script() -> Vec<Step>`
  - `pub fn run_transcript(root: &std::path::Path, mode: &Mode, script: &[Step]) -> Vec<u8>`

- [ ] **Step 1: Record the base test totals**

Run each command to a log file and read the summary lines:

```bash
cargo test --features mcp --lib mcp:: > /tmp/s1-base-lib.log 2>&1; grep -E '^test result' /tmp/s1-base-lib.log
cargo test --features mcp --test mcp > /tmp/s1-base-mcp.log 2>&1; grep -E '^test result' /tmp/s1-base-mcp.log
```

Expected on `main` at `bab0dfbd` or later with no MCP change: `195 passed; 0 failed` and `4 passed; 0 failed`. Write the two numbers you actually got into the PR description draft; Task 5 compares against them.

- [ ] **Step 2: Write the transcript driver**

Create `tests/mcp/transcript.rs`:

```rust
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
```

- [ ] **Step 3: Write the golden test and its comparator**

Create `tests/mcp/golden_transcript_test.rs`:

```rust
#![cfg(feature = "mcp")]
//! The in-process server's stdout, recorded on the commit before the S1 transport refactor.
//! These goldens are the standalone byte-for-byte contract of the shared-daemon lane (spec
//! §4.5): slices S1–S5 must not change them. A later change that legitimately alters tool
//! output re-records them with `PRISM_MCP_BLESS_GOLDEN=1` and says why in its commit message.

use crate::transcript::{run_transcript, script, write_fixture, Mode};
use std::path::PathBuf;

const MODES: &[Mode] = &[
    Mode {
        name: "eager-default",
        flags: &["--eager"],
        env: &[],
    },
    Mode {
        name: "lazy-default",
        flags: &["--first-call-wait", "60"],
        env: &[],
    },
    Mode {
        name: "eager-always-legacy",
        flags: &["--eager"],
        env: &[
            ("PRISM_MCP_STRUCTURED_CONTENT", "always"),
            ("PRISM_MCP_CONCISE_SHAPE", "legacy"),
        ],
    },
    Mode {
        name: "eager-auto-full",
        flags: &["--eager", "--refresh-policy", "auto-full"],
        env: &[],
    },
    Mode {
        name: "eager-auto-incremental-cap",
        flags: &["--eager", "--refresh-policy", "auto-incremental"],
        env: &[("PRISM_MCP_MAX_RESULT_CHARS", "20000")],
    },
];

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/mcp/golden")
        .join(format!("{name}.jsonl"))
}

/// The package version is the only build-dependent text in a transcript; replace it so a
/// release bump does not invalidate the goldens.
fn normalize_version(transcript: Vec<u8>) -> Vec<u8> {
    let text = String::from_utf8(transcript).expect("transcript is UTF-8");
    let built = format!(
        r#""serverInfo":{{"name":"prism-mcp","version":"{}"}}"#,
        env!("CARGO_PKG_VERSION")
    );
    assert_eq!(
        text.matches(&built).count(),
        1,
        "expected exactly one serverInfo block in the transcript"
    );
    text.replace(
        &built,
        r#""serverInfo":{"name":"prism-mcp","version":"<VERSION>"}"#,
    )
    .into_bytes()
}

/// `None` when equal; otherwise where the first difference is, with a readable window.
fn first_difference(expected: &[u8], actual: &[u8]) -> Option<String> {
    if expected == actual {
        return None;
    }
    let expected_lines: Vec<&[u8]> = expected.split(|byte| *byte == b'\n').collect();
    let actual_lines: Vec<&[u8]> = actual.split(|byte| *byte == b'\n').collect();
    for (index, (want, got)) in expected_lines.iter().zip(&actual_lines).enumerate() {
        if want != got {
            let at = want
                .iter()
                .zip(got.iter())
                .position(|(a, b)| a != b)
                .unwrap_or_else(|| want.len().min(got.len()));
            let window = |line: &[u8]| {
                let start = at.saturating_sub(80);
                let end = line.len().min(at + 160);
                String::from_utf8_lossy(&line[start.min(end)..end]).into_owned()
            };
            return Some(format!(
                "response line {} differs at byte {at}\n  golden: ...{}...\n  actual: ...{}...",
                index + 1,
                window(want),
                window(got)
            ));
        }
    }
    Some(format!(
        "line count differs: golden has {}, actual has {}",
        expected_lines.len(),
        actual_lines.len()
    ))
}

#[test]
fn standalone_transcripts_match_the_goldens_recorded_before_the_transport_refactor() {
    let bless = std::env::var_os("PRISM_MCP_BLESS_GOLDEN").is_some();
    for mode in MODES {
        let repo = tempfile::tempdir().expect("temp repo");
        write_fixture(repo.path());
        let actual = normalize_version(run_transcript(repo.path(), mode, &script()));
        let golden = golden_path(mode.name);
        if bless {
            std::fs::create_dir_all(golden.parent().expect("golden dir")).expect("create dir");
            std::fs::write(&golden, &actual).expect("write golden");
            continue;
        }
        let expected = std::fs::read(&golden).unwrap_or_else(|error| {
            panic!(
                "missing golden {}: {error}; record it with PRISM_MCP_BLESS_GOLDEN=1",
                golden.display()
            )
        });
        if let Some(difference) = first_difference(&expected, &actual) {
            panic!("mode {}: {difference}", mode.name);
        }
    }
}

#[test]
fn transcript_comparison_names_the_first_differing_line_and_byte() {
    assert_eq!(first_difference(b"a\nb\n", b"a\nb\n"), None);
    let message = first_difference(b"same\nab-golden\n", b"same\nab-actual\n").expect("differs");
    assert!(
        message.starts_with("response line 2 differs at byte 3"),
        "{message}"
    );
    // An extra response shows up as a line the golden does not have.
    let message = first_difference(b"a\n", b"a\nb\n").expect("differs");
    assert!(
        message.starts_with("response line 2 differs at byte 0"),
        "{message}"
    );
    let message = first_difference(b"a\nb", b"a\nb\nc").expect("differs");
    assert!(
        message.starts_with("line count differs: golden has 2, actual has 3"),
        "{message}"
    );
}

#[test]
fn every_scripted_request_gets_exactly_one_response_line() {
    let repo = tempfile::tempdir().expect("temp repo");
    write_fixture(repo.path());
    let transcript = run_transcript(repo.path(), &MODES[0], &script());
    let requests = script()
        .iter()
        .filter(|step| matches!(step, crate::transcript::Step::Request(_)))
        .count();
    assert_eq!(requests, 25);
    assert_eq!(
        transcript.iter().filter(|byte| **byte == b'\n').count(),
        requests,
        "a notification was answered or a request was not"
    );
    let repo_path = repo.path().to_string_lossy().into_owned();
    assert!(
        !String::from_utf8_lossy(&transcript).contains(&repo_path),
        "the temp repository path leaked into a response"
    );
}
```

Replace `tests/mcp/main.rs` with:

```rust
mod golden_transcript_test;
mod smoke_test;
mod transcript;
```

- [ ] **Step 4: Run it and see the golden test fail for the right reason**

Run: `cargo test --features mcp --test mcp golden_transcript_test > /tmp/s1-t1-red.log 2>&1; grep -E 'missing golden|^test |^test result' /tmp/s1-t1-red.log`

Expected: `standalone_transcripts_match_the_goldens_recorded_before_the_transport_refactor` FAILS with `missing golden .../eager-default.jsonl`; the other two tests PASS. If the first test fails for any other reason, fix the driver before going on.

- [ ] **Step 5: Record the goldens on the base code, then prove they are stable**

```bash
git status --short src/            # must print nothing: no src/ change exists yet
PRISM_MCP_BLESS_GOLDEN=1 cargo test --features mcp --test mcp golden_transcript_test > /tmp/s1-t1-bless.log 2>&1
shasum -a 256 tests/fixtures/mcp/golden/*.jsonl > /tmp/s1-golden-1.sha
for i in 1 2 3; do cargo test --features mcp --test mcp golden_transcript_test > /tmp/s1-t1-green-$i.log 2>&1; grep -E '^test result' /tmp/s1-t1-green-$i.log; done
shasum -a 256 tests/fixtures/mcp/golden/*.jsonl | diff - /tmp/s1-golden-1.sha && echo GOLDENS-STABLE
wc -l tests/fixtures/mcp/golden/*.jsonl
```

Expected: three runs of `3 passed; 0 failed`; `GOLDENS-STABLE`; each golden has 25 lines. Then read one golden and confirm it holds what the script intends, not just that it is stable:

```bash
python3 - <<'EOF'
import json
rows = [json.loads(line) for line in open("tests/fixtures/mcp/golden/eager-default.jsonl")]
by_id = {row.get("id"): row for row in rows if row.get("id") is not None}
assert by_id[1]["result"]["serverInfo"]["version"] == "<VERSION>"
assert len(by_id[3]["result"]["tools"]) == 9
assert by_id[11]["result"]["isError"] is False, "taint_reaches must succeed"
assert by_id[18]["result"]["_meta"]["prism/index_freshness"] == "stale"
assert by_id[19]["result"]["_meta"]["prism/refresh_generation"] == 1
assert "prism/index_freshness" not in by_id[20]["result"]["_meta"]
auto = {r.get("id"): r for r in map(json.loads, open("tests/fixtures/mcp/golden/eager-auto-full.jsonl")) if r.get("id") is not None}
assert auto[18]["result"]["_meta"]["prism/auto_refresh"] == "refreshed"
print("GOLDEN-CONTENT-OK")
EOF
```

Expected: `GOLDEN-CONTENT-OK`. If an assertion fails, the script or fixture is wrong; fix it and re-record before committing.

- [ ] **Step 6: Commit, push, and wait for CI on this commit alone**

```bash
cargo fmt
git add tests/mcp/main.rs tests/mcp/transcript.rs tests/mcp/golden_transcript_test.rs tests/fixtures/mcp/golden
git diff --cached --stat -- src/ | wc -l     # must be 0
git commit -m "test(mcp): golden transcripts of the in-process server, recorded before the transport refactor"
git push -u origin HEAD
```

Open the PR as a draft and wait for CI. **Do not start Task 2 until CI is green on this commit.** The goldens were recorded on macOS; CI runs on Linux, and a green run here is the evidence that they are platform-independent. If CI fails on a golden, read the reported line and byte: that is a platform-dependent output on the base, to be reported to the controller before any refactor, not normalized away silently.

---

### Task 2: Move the framing into `src/mcp/framing.rs`

A pure move plus one rename. No behavior change; the existing tests are the check.

**Files:**
- Create: `src/mcp/framing.rs`
- Modify: `src/mcp/transport.rs` (remove the moved items; add imports and the test alias)
- Modify: `src/mcp/mod.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces:
  - `pub(super) const MAX_REQUEST_BYTES: usize`
  - `pub enum ReadOutcome { Message(String), Malformed }`
  - `pub trait Transport { fn read_message(&mut self) -> anyhow::Result<Option<ReadOutcome>>; fn write_message(&mut self, v: serde_json::Value) -> anyhow::Result<()>; }`
  - `pub(super) struct LineTransport<R, W>` with `pub(super) fn new(reader: R, writer: W) -> Self`, and `impl<R: BufRead, W: Write> Transport for LineTransport<R, W>`
  - `#[cfg(test)] pub struct InMemoryTransport` with `new(Vec<&str>)` and `responses(&self) -> &[Value]`
  - `transport.rs` keeps re-exporting `ReadOutcome` and `Transport` as `pub`, so `crate::mcp::transport::{Transport, ReadOutcome}` still resolves.

- [ ] **Step 1: Create `src/mcp/framing.rs` with the moved code**

Move these items out of `src/mcp/transport.rs` **verbatim, doc comments included**: the `MAX_REQUEST_BYTES` constant and its comment, `ReadOutcome`, `Transport`, `struct StdioTransport` with both `impl` blocks and the `Transport` impl, `finish_line`, and `InMemoryTransport` with its two `impl` blocks. Apply exactly these edits to the moved code:

- Rename the struct `StdioTransport` to `LineTransport` in all four places it is named.
- `MAX_REQUEST_BYTES` becomes `pub(super) const`.
- `struct LineTransport` and its `fn new` become `pub(super)`.

The file starts with:

```rust
//! Newline-delimited JSON framing shared by every byte transport (stdio today, a Unix socket for
//! the shared daemon). One implementation, generic over the reader and writer, so a socket
//! connection frames messages exactly as stdio does.
use serde_json::Value;
#[cfg(test)]
use std::collections::VecDeque;
use std::io::{BufRead, Write};

/// Max inbound JSON-RPC message size (one line). A nav request is tiny; this caps a hostile/buggy
/// local client's memory (holistic-review MAJOR — `read_line` was unbounded).
pub(super) const MAX_REQUEST_BYTES: usize = 1 << 20; // 1 MiB
```

and the struct header reads:

```rust
/// One newline-delimited JSON message per line, over any buffered reader and any writer.
pub(super) struct LineTransport<R, W> {
    reader: R,
    writer: W,
}

impl<R, W> LineTransport<R, W> {
    pub(super) fn new(reader: R, writer: W) -> Self {
        Self { reader, writer }
    }
}
```

Everything else in the moved bodies is byte-identical to what was in `transport.rs`.

- [ ] **Step 2: Rewire `src/mcp/transport.rs` and `src/mcp/mod.rs`**

In `src/mcp/mod.rs`, add the module in alphabetical position (after `evidence_view`):

```rust
pub mod framing;
```

In `src/mcp/transport.rs`:

- Delete the moved items and the now-unused `#[cfg(test)] use std::collections::VecDeque;`.
- Change `use std::io::{self, BufRead, Write};` to `use std::io;`.
- Add, with the other `use super::…` lines:

```rust
#[cfg(test)]
pub use super::framing::InMemoryTransport;
use super::framing::LineTransport;
#[cfg(test)]
use super::framing::MAX_REQUEST_BYTES;
pub use super::framing::{ReadOutcome, Transport};

/// The name the existing transport test corpus uses for the stdio instantiation.
#[cfg(test)]
type StdioTransport<R, W> = LineTransport<R, W>;
```

- In `serve_stdio_runtime`, change `StdioTransport::new(stdin.lock(), stdout.lock())` to `LineTransport::new(stdin.lock(), stdout.lock())`.

- [ ] **Step 3: Run the corpus and the goldens**

```bash
cargo fmt
cargo test --features mcp --lib mcp:: > /tmp/s1-t2-lib.log 2>&1; grep -E '^test result|error(\[|:)' /tmp/s1-t2-lib.log
cargo test --features mcp --test mcp > /tmp/s1-t2-mcp.log 2>&1; grep -E '^test result|error(\[|:)' /tmp/s1-t2-mcp.log
cargo check > /tmp/s1-t2-default.log 2>&1; tail -1 /tmp/s1-t2-default.log
git diff --stat origin/main -- src/mcp/transport_tests.rs src/mcp/owner_transport_tests.rs tests/fixtures/mcp/golden | wc -l
```

Expected: the lib total equals the Step 1 base number (195) with 0 failed; the `mcp` target shows `7 passed` (4 base + 3 from Task 1); `cargo check` finishes; the last command prints `0`.

- [ ] **Step 4: Commit**

```bash
git add src/mcp/framing.rs src/mcp/transport.rs src/mcp/mod.rs
git commit -m "refactor(mcp): move line framing to framing.rs as LineTransport (pure move)"
```

---

### Task 3: `UnixStreamTransport` and one framing suite over both transports

**Files:**
- Modify: `src/mcp/framing.rs`
- Create: `src/mcp/framing_tests.rs`

**Interfaces:**
- Consumes: `LineTransport`, `Transport`, `ReadOutcome`, `MAX_REQUEST_BYTES` from Task 2.
- Produces:
  - `#[cfg(unix)] pub(super) type UnixStreamTransport = LineTransport<std::io::BufReader<UnixStream>, std::io::BufWriter<UnixStream>>;`
  - `#[cfg(unix)] impl UnixStreamTransport { pub(super) fn from_stream(stream: UnixStream) -> std::io::Result<Self> }`

- [ ] **Step 1: Write the framing suite**

Create `src/mcp/framing_tests.rs`:

```rust
//! One framing suite, run through the `Transport` trait over an in-memory reader and, on Unix,
//! over a real socket pair. The socket run delivers bytes in kernel-sized chunks, so the same
//! cases also exercise frames split across reads.
use super::*;
use std::hash::{Hash, Hasher};

#[derive(Debug, PartialEq)]
enum Got {
    Message(String),
    Malformed,
}

fn drain(transport: &mut impl Transport) -> Vec<Got> {
    let mut got = Vec::new();
    while let Some(outcome) = transport.read_message().expect("read_message") {
        got.push(match outcome {
            ReadOutcome::Message(line) => Got::Message(line),
            ReadOutcome::Malformed => Got::Malformed,
        });
    }
    got
}

/// Short, comparable descriptions, so a failing megabyte-sized case prints one line.
fn describe(got: &[Got]) -> Vec<String> {
    got.iter()
        .map(|item| match item {
            Got::Malformed => "malformed".to_string(),
            Got::Message(text) if text.len() <= 64 => format!("message {text:?}"),
            Got::Message(text) => {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                text.hash(&mut hasher);
                format!("message of {} bytes, hash {:016x}", text.len(), hasher.finish())
            }
        })
        .collect()
}

fn message(text: impl Into<String>) -> Got {
    Got::Message(text.into())
}

fn cases() -> Vec<(&'static str, Vec<u8>, Vec<Got>)> {
    let limit = "x".repeat(MAX_REQUEST_BYTES);
    let over = "x".repeat(MAX_REQUEST_BYTES + 1);
    let accented = "é".repeat(20_000);
    vec![
        ("two lines", b"a\nb\n".to_vec(), vec![message("a"), message("b")]),
        (
            "crlf line endings",
            b"a\r\nb\r\n".to_vec(),
            vec![message("a"), message("b")],
        ),
        (
            "final line without a newline",
            b"a\nb".to_vec(),
            vec![message("a"), message("b")],
        ),
        ("an empty line is a message", b"\n".to_vec(), vec![message("")]),
        ("end of input only", Vec::new(), Vec::new()),
        (
            "invalid utf-8 is malformed and the next line is read",
            vec![0xff, 0xfe, b'\n', b'o', b'k', b'\n'],
            vec![Got::Malformed, message("ok")],
        ),
        (
            "exactly the limit is accepted",
            format!("{limit}\nok\n").into_bytes(),
            vec![message(limit.clone()), message("ok")],
        ),
        (
            "one byte over is malformed and the stream resyncs",
            format!("{over}\nok\n").into_bytes(),
            vec![Got::Malformed, message("ok")],
        ),
        (
            "an oversized final line without a newline is malformed",
            over.clone().into_bytes(),
            vec![Got::Malformed],
        ),
        (
            "multi-byte text split across reads",
            format!("{accented}\nok\n").into_bytes(),
            vec![message(accented.clone()), message("ok")],
        ),
    ]
}

#[test]
fn framing_cases_over_an_in_memory_reader() {
    for (name, input, expected) in cases() {
        let mut out = Vec::new();
        let mut transport = LineTransport::new(std::io::Cursor::new(input), &mut out);
        assert_eq!(describe(&drain(&mut transport)), describe(&expected), "case: {name}");
    }
}

#[cfg(unix)]
mod unix {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::Shutdown;
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    #[test]
    fn framing_cases_over_a_unix_socket() {
        for (name, input, expected) in cases() {
            let (mut client, server) = UnixStream::pair().expect("socket pair");
            let writer = std::thread::spawn(move || {
                client.write_all(&input).expect("client write");
                client.shutdown(Shutdown::Write).expect("client half-close");
                // Hold the read half open until the server is done reading.
                let mut rest = Vec::new();
                let _ = client.read_to_end(&mut rest);
            });
            let mut transport = UnixStreamTransport::from_stream(server).expect("transport");
            let got = drain(&mut transport);
            drop(transport);
            writer.join().expect("client thread");
            assert_eq!(describe(&got), describe(&expected), "case: {name}");
        }
    }

    #[test]
    fn unix_transport_flushes_one_json_line_per_message() {
        let (client, server) = UnixStream::pair().expect("socket pair");
        client
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("read timeout");
        let mut transport = UnixStreamTransport::from_stream(server).expect("transport");
        transport
            .write_message(serde_json::json!({"a": 1}))
            .expect("first write");
        // The transport is still alive: the line must already be on the wire, not parked in a
        // buffer until the connection closes.
        let mut reader = BufReader::new(client);
        let mut line = String::new();
        reader.read_line(&mut line).expect("first line arrives without closing");
        assert_eq!(line, "{\"a\":1}\n");
        transport
            .write_message(serde_json::json!({"b": [1, 2]}))
            .expect("second write");
        line.clear();
        reader.read_line(&mut line).expect("second line arrives");
        assert_eq!(line, "{\"b\":[1,2]}\n");
    }

    #[test]
    fn writing_to_a_closed_peer_is_an_error_not_a_panic() {
        let (client, server) = UnixStream::pair().expect("socket pair");
        let mut transport = UnixStreamTransport::from_stream(server).expect("transport");
        drop(client);
        assert!(transport.write_message(serde_json::json!({})).is_err());
    }
}
```

At the bottom of `src/mcp/framing.rs` add:

```rust
#[cfg(test)]
#[path = "framing_tests.rs"]
mod tests;
```

- [ ] **Step 2: Run it and see the socket tests fail to compile**

Run: `cargo test --features mcp --lib mcp::framing > /tmp/s1-t3-red.log 2>&1; grep -E 'error\[|cannot find|^test result' /tmp/s1-t3-red.log | head`

Expected: a compile error naming `UnixStreamTransport` (`cannot find type` or `failed to resolve`). `framing_cases_over_an_in_memory_reader` characterizes the existing framing and would pass on its own; the three socket tests are the failing ones.

- [ ] **Step 3: Add `UnixStreamTransport`**

In `src/mcp/framing.rs`, add to the imports:

```rust
#[cfg(unix)]
use std::os::unix::net::UnixStream;
```

and after the `Transport for LineTransport` impl:

```rust
/// The framing over one connected Unix socket. Reads and writes go through independent handles
/// to the same stream; the writer is buffered and `write_message` flushes once per message, so a
/// response is one write to the peer rather than one per JSON token.
// First production consumer: the daemon's connection loop (shared-daemon slice S3a).
#[cfg(unix)]
#[cfg_attr(not(test), allow(dead_code))]
pub(super) type UnixStreamTransport =
    LineTransport<std::io::BufReader<UnixStream>, std::io::BufWriter<UnixStream>>;

#[cfg(unix)]
#[cfg_attr(not(test), allow(dead_code))]
impl UnixStreamTransport {
    pub(super) fn from_stream(stream: UnixStream) -> std::io::Result<Self> {
        let reader = stream.try_clone()?;
        Ok(Self::new(
            std::io::BufReader::new(reader),
            std::io::BufWriter::new(stream),
        ))
    }
}
```

- [ ] **Step 4: Run the suite**

Run: `cargo test --features mcp --lib mcp::framing > /tmp/s1-t3-green.log 2>&1; grep -E '^test |^test result' /tmp/s1-t3-green.log`

Expected: 4 tests pass (`framing_cases_over_an_in_memory_reader` and the three under `unix::`).

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add src/mcp/framing.rs src/mcp/framing_tests.rs
git commit -m "feat(mcp): UnixStreamTransport sharing the stdio line framing"
```

- [ ] **Step 6: Prove the flush test bites**

With the task committed, comment out the flush and watch the test fail:

```bash
sed -i.bak 's|^        self.writer.flush()?;$|        // self.writer.flush()?;|' src/mcp/framing.rs && rm src/mcp/framing.rs.bak
git diff --stat src/mcp/framing.rs        # exactly 1 line changed
cargo test --features mcp --lib mcp::framing::tests::unix::unix_transport_flushes > /tmp/s1-t3-bite.log 2>&1; grep -E '^test |panicked' -A1 /tmp/s1-t3-bite.log
git checkout src/mcp/framing.rs && git status --short src/mcp/framing.rs
```

Expected: the diff shows one changed line; the test is FAILED with `first line arrives without closing` (a timed-out read); after the checkout `git status` prints nothing. Do this only after Step 5: `git checkout` restores the committed file, so an uncommitted Task 3 would be lost with the experiment.

---

### Task 4: Split control from dispatch and make output settings explicit

**Files:**
- Create: `src/mcp/settings.rs`
- Create: `src/mcp/transport_split_tests.rs`
- Create: `tests/mcp/settings_test.rs`
- Modify: `src/mcp/transport.rs`
- Modify: `src/mcp/mod.rs`
- Modify: `tests/mcp/main.rs`

**Interfaces:**
- Consumes: `LineTransport`, `UnixStreamTransport`, `InMemoryTransport`, `Transport` (Tasks 2–3); the existing `SessionRuntime` trait, `Readiness`, `ToolRegistry`, `ToolDescriptor`.
- Produces (all in `crate::mcp`; these are what S2–S4 build on):
  - `pub struct ConnectionSettings { pub result_cap: usize, pub structured_content: StructuredContentMode, pub concise_shape: ConciseShapeMode }` with `pub fn from_env() -> Self`; `Clone, Copy, Debug, PartialEq, Eq`.
  - `pub(super) enum Lifecycle { PreInit, InitializeReceived, Initialized }` (visibility raised; variants unchanged).
  - `pub(super) struct ToolCall<'r>` with `pub(super) fn is_refresh(&self) -> bool`.
  - `pub(super) enum Control<'r> { Response(Value), NoResponse, ToolCall(ToolCall<'r>) }`
  - `pub(super) fn handle_control<'r>(message: &Value, state: &mut Lifecycle, registry: &'r ToolRegistry, startup_mode: StartupMode, settings: &ConnectionSettings) -> Control<'r>`
  - `pub(super) fn readiness_response(call: &ToolCall<'_>, readiness: Readiness, admission: impl FnOnce() -> Option<Value>, settings: &ConnectionSettings) -> Option<Value>`
  - `pub(super) enum ReadDispatch { Response(Value), NeedsAutoRefresh { initial_report: Option<FreshnessReport> } }`
  - `pub(super) fn dispatch_tool_read(call: &ToolCall<'_>, view: &impl SessionRuntime, settings: &ConnectionSettings) -> ReadDispatch`
  - `pub(super) fn dispatch_tool_refresh(call: &ToolCall<'_>, provider: &mut impl SessionRuntime, settings: &ConnectionSettings) -> Value`
  - `pub(super) fn dispatch_tool_auto_refresh(call: &ToolCall<'_>, provider: &mut impl SessionRuntime, initial_report: Option<FreshnessReport>, settings: &ConnectionSettings) -> Value`
  - `pub(super) fn serve_connection(runtime: &mut impl SessionRuntime, registry: &ToolRegistry, transport: &mut impl Transport, settings: &ConnectionSettings) -> anyhow::Result<()>`

Design decisions, so they are not re-litigated mid-task:

- **The read-only facade is `&impl SessionRuntime`.** A shared reference cannot call `ensure_ready`, `refresh_index` or `auto_refresh_index` (all `&mut self`), and it sees the whole provider, including the sticky `known_stale_after_refresh` evidence, with today's precedence in `effective_stale_report`. No new trait is introduced; `&SessionProvider` is what the daemon will pass.
- **`handle_control` takes `startup_mode` by value**, because `initialize` advertises it and the control path has no runtime to ask. The in-process server passes `runtime.startup_mode()`.
- **Validation that needs no index stays in the control path**: params shape, unknown tool, and `refresh_index` with non-empty arguments are answered before readiness is consulted, exactly as today.
- **No trigger re-check in `dispatch_tool_auto_refresh`.** The in-process server has no interleaving between the read and the refresh, and adding a second freshness probe would change behavior. S3b adds the re-check on the daemon's write-locked path.
- **The test-only entry points `handle_message`, `call_tool_response_with_cap` and `call_tool_response_with_cap_and_mode` keep their exact signatures**, so the existing corpus compiles unmodified.

- [ ] **Step 1: Write the in-crate tests**

Create `src/mcp/transport_split_tests.rs`:

```rust
//! Tests for the control/dispatch split: `handle_control` needs no index, `dispatch_tool_read`
//! needs only shared access, and output settings are an explicit per-connection value.
use super::*;
use crate::mcp::concise_shape::ConciseShapeMode;
use crate::mcp::settings::ConnectionSettings;
use crate::mcp::{CacheMode, RefreshPolicy, ServerConfig, SessionProvider};

const INIT: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#;
const INITED: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
const LIST: &str = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#;
const CALLEES: &str = r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"nav_callees","arguments":{"seed":{"kind":"symbol","name":"run","file":"main.py"}}}}"#;
const REFRESH: &str =
    r#"{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"refresh_index","arguments":{}}}"#;

fn settings(structured_content: StructuredContentMode) -> ConnectionSettings {
    ConnectionSettings {
        result_cap: 80_000,
        structured_content,
        concise_shape: ConciseShapeMode::Slim,
    }
}

fn provider(refresh_policy: RefreshPolicy) -> (tempfile::TempDir, SessionProvider) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("util.py"), "def helper():\n    return 1\n").unwrap();
    std::fs::write(
        dir.path().join("main.py"),
        "from util import helper\n\ndef run():\n    return helper()\n",
    )
    .unwrap();
    let cfg = ServerConfig {
        repo_root: dir.path().to_path_buf(),
        cache: CacheMode::NoCache,
        refresh_policy,
        startup: StartupMode::Eager,
        first_call_wait: std::time::Duration::from_secs(20),
    };
    (dir, SessionProvider::bootstrap(&cfg).unwrap())
}

fn make_stale(dir: &tempfile::TempDir) {
    std::fs::write(
        dir.path().join("util.py"),
        "def helper():\n    return 1\n\n\ndef extra():\n    return helper()\n",
    )
    .unwrap();
}

fn parse(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

fn control<'r>(
    registry: &'r ToolRegistry,
    state: &mut Lifecycle,
    request: &str,
    startup_mode: StartupMode,
    settings: &ConnectionSettings,
) -> Control<'r> {
    handle_control(&parse(request), state, registry, startup_mode, settings)
}

fn response_of(control: Control<'_>) -> Value {
    match control {
        Control::Response(response) => response,
        Control::NoResponse => panic!("expected a response, got none"),
        Control::ToolCall(_) => panic!("expected a response, got a tool call"),
    }
}

fn tool_call<'r>(
    registry: &'r ToolRegistry,
    request: &str,
    settings: &ConnectionSettings,
) -> ToolCall<'r> {
    let mut state = Lifecycle::Initialized;
    match control(registry, &mut state, request, StartupMode::Eager, settings) {
        Control::ToolCall(call) => call,
        Control::Response(response) => panic!("expected a tool call, got {response}"),
        Control::NoResponse => panic!("expected a tool call, got no response"),
    }
}

fn read_response(dispatch: ReadDispatch) -> Value {
    match dispatch {
        ReadDispatch::Response(response) => response,
        ReadDispatch::NeedsAutoRefresh { .. } => panic!("expected a response, got a deferral"),
    }
}

#[test]
fn handle_control_serves_the_handshake_and_listing_with_no_index() {
    let registry = ToolRegistry::all_v1();
    let settings = settings(StructuredContentMode::OmitDefaultPath);
    let mut state = Lifecycle::PreInit;

    let early = response_of(control(&registry, &mut state, LIST, StartupMode::Lazy, &settings));
    assert_eq!(early["error"]["code"], -32600, "tools/list before initialize");

    let init = response_of(control(
        &registry,
        &mut state,
        INIT,
        StartupMode::Background,
        &settings,
    ));
    assert!(
        init["result"]["instructions"]
            .as_str()
            .unwrap()
            .contains("started at server startup"),
        "initialize advertises the startup mode it was given: {init}"
    );
    assert!(state == Lifecycle::InitializeReceived);

    assert!(matches!(
        control(&registry, &mut state, INITED, StartupMode::Background, &settings),
        Control::NoResponse
    ));
    assert!(state == Lifecycle::Initialized);

    let listed = response_of(control(
        &registry,
        &mut state,
        LIST,
        StartupMode::Background,
        &settings,
    ));
    assert_eq!(listed["result"]["tools"].as_array().unwrap().len(), 9);

    let ping = response_of(control(
        &registry,
        &mut state,
        r#"{"jsonrpc":"2.0","id":3,"method":"ping"}"#,
        StartupMode::Background,
        &settings,
    ));
    assert_eq!(ping["result"], json!({}));
}

#[test]
fn handle_control_rejects_what_needs_no_index_to_reject() {
    let registry = ToolRegistry::all_v1();
    let mut state = Lifecycle::Initialized;
    let always = settings(StructuredContentMode::Always);

    let unknown = response_of(control(
        &registry,
        &mut state,
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"no_such_tool","arguments":{}}}"#,
        StartupMode::Eager,
        &always,
    ));
    assert_eq!(unknown["result"]["isError"], true);
    assert!(unknown["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .starts_with("unknown tool 'no_such_tool'"));

    let refresh_with_args = response_of(control(
        &registry,
        &mut state,
        r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"refresh_index","arguments":{"x":1}}}"#,
        StartupMode::Eager,
        &always,
    ));
    assert_eq!(
        refresh_with_args["result"]["content"][0]["text"],
        "refresh_index takes no arguments"
    );

    let array_arguments = response_of(control(
        &registry,
        &mut state,
        r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"nav_callees","arguments":[]}}"#,
        StartupMode::Eager,
        &always,
    ));
    assert_eq!(array_arguments["error"]["code"], -32602);

    let missing_name = response_of(control(
        &registry,
        &mut state,
        r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{}}"#,
        StartupMode::Eager,
        &always,
    ));
    assert_eq!(missing_name["error"]["code"], -32602);
}

#[test]
fn handle_control_returns_a_validated_tool_call() {
    let registry = ToolRegistry::all_v1();
    let settings = settings(StructuredContentMode::Always);
    assert!(!tool_call(&registry, CALLEES, &settings).is_refresh());
    assert!(tool_call(&registry, REFRESH, &settings).is_refresh());
}

#[test]
fn dispatch_tool_read_answers_from_shared_access() {
    let registry = ToolRegistry::all_v1();
    let settings = settings(StructuredContentMode::Always);
    let (_dir, provider) = provider(RefreshPolicy::WarnOnly);
    let call = tool_call(&registry, CALLEES, &settings);

    // `provider` is not `mut`: the read path compiles against shared access only.
    let response = read_response(dispatch_tool_read(&call, &provider, &settings));
    assert_eq!(response["id"], 7);
    assert_eq!(response["result"]["isError"], false);
    assert_eq!(
        response["result"]["structuredContent"]["query"],
        "callees:run@main.py"
    );
}

#[test]
fn a_stale_warn_only_index_is_answered_with_the_stale_warning() {
    let registry = ToolRegistry::all_v1();
    let settings = settings(StructuredContentMode::Always);
    let (dir, provider) = provider(RefreshPolicy::WarnOnly);
    make_stale(&dir);
    let call = tool_call(&registry, CALLEES, &settings);

    let response = read_response(dispatch_tool_read(&call, &provider, &settings));
    assert_eq!(response["result"]["_meta"]["prism/index_freshness"], "stale");
}

#[test]
fn a_stale_auto_refresh_index_is_deferred_to_the_exclusive_path() {
    let registry = ToolRegistry::all_v1();
    let settings = settings(StructuredContentMode::Always);
    let (dir, mut provider) = provider(RefreshPolicy::AutoFull);
    make_stale(&dir);
    let call = tool_call(&registry, CALLEES, &settings);

    let initial_report = match dispatch_tool_read(&call, &provider, &settings) {
        ReadDispatch::NeedsAutoRefresh { initial_report } => initial_report,
        ReadDispatch::Response(response) => panic!("expected a deferral, got {response}"),
    };
    assert!(initial_report.as_ref().is_some_and(|report| report.stale));

    let response = dispatch_tool_auto_refresh(&call, &mut provider, initial_report, &settings);
    assert_eq!(response["result"]["_meta"]["prism/auto_refresh"], "refreshed");
    assert_eq!(response["result"]["_meta"]["prism/refresh_generation"], 1);

    // Fresh again: the next read is answered directly.
    let again = read_response(dispatch_tool_read(&call, &provider, &settings));
    assert!(again["result"]["_meta"].get("prism/index_freshness").is_none());
}

#[test]
fn dispatch_tool_refresh_rebuilds_and_reports_the_new_generation() {
    let registry = ToolRegistry::all_v1();
    let settings = settings(StructuredContentMode::Always);
    let (dir, mut provider) = provider(RefreshPolicy::WarnOnly);
    make_stale(&dir);
    let call = tool_call(&registry, REFRESH, &settings);

    let response = dispatch_tool_refresh(&call, &mut provider, &settings);
    assert_eq!(response["id"], 8);
    assert_eq!(response["result"]["structuredContent"]["generation"], 1);
    assert_eq!(
        response["result"]["structuredContent"]["stale_before_refresh"],
        true
    );
}

#[test]
fn output_settings_belong_to_the_call_site_not_the_process() {
    let registry = ToolRegistry::all_v1();
    let (_dir, provider) = provider(RefreshPolicy::WarnOnly);
    let always = settings(StructuredContentMode::Always);
    let omit = settings(StructuredContentMode::OmitDefaultPath);

    let with = read_response(dispatch_tool_read(
        &tool_call(&registry, CALLEES, &always),
        &provider,
        &always,
    ));
    let without = read_response(dispatch_tool_read(
        &tool_call(&registry, CALLEES, &omit),
        &provider,
        &omit,
    ));
    assert!(with["result"].get("structuredContent").is_some());
    assert!(without["result"].get("structuredContent").is_none());
}

#[test]
fn readiness_response_maps_each_state_and_asks_for_admission_only_on_failure() {
    let registry = ToolRegistry::all_v1();
    let settings = settings(StructuredContentMode::Always);
    let call = tool_call(&registry, CALLEES, &settings);
    let asked = std::cell::Cell::new(0);
    let admission = || {
        asked.set(asked.get() + 1);
        None
    };

    assert!(readiness_response(&call, Readiness::Ready, admission, &settings).is_none());
    let warming = readiness_response(
        &call,
        Readiness::Warming {
            elapsed: std::time::Duration::from_secs(3),
        },
        admission,
        &settings,
    )
    .expect("warming is answered");
    assert_eq!(warming["id"], 7);
    assert_eq!(warming["result"]["structuredContent"]["status"], "warming");
    assert_eq!(warming["result"]["structuredContent"]["elapsed_secs"], 3);
    assert_eq!(asked.get(), 0, "no admission lookup unless the build failed");

    let failed = readiness_response(
        &call,
        Readiness::Failed {
            error: "boom".into(),
        },
        admission,
        &settings,
    )
    .expect("failure is answered");
    assert_eq!(failed["result"]["structuredContent"]["status"], "build_failed");
    assert_eq!(failed["result"]["structuredContent"]["cause"], "boom");
    assert_eq!(asked.get(), 1);
}

#[test]
fn serve_connection_applies_the_settings_it_is_given() {
    let registry = ToolRegistry::all_v1();
    for (mode, expect_structured) in [
        (StructuredContentMode::Always, true),
        (StructuredContentMode::OmitDefaultPath, false),
    ] {
        let (_dir, mut provider) = provider(RefreshPolicy::WarnOnly);
        let mut transport = InMemoryTransport::new(vec![INIT, INITED, CALLEES]);
        serve_connection(&mut provider, &registry, &mut transport, &settings(mode)).unwrap();
        let responses = transport.responses();
        assert_eq!(responses.len(), 2);
        assert_eq!(
            responses[1]["result"].get("structuredContent").is_some(),
            expect_structured
        );
    }
}

#[cfg(unix)]
mod unix {
    use super::*;
    use crate::mcp::framing::{LineTransport, UnixStreamTransport};
    use std::io::{Read, Write};
    use std::net::Shutdown;
    use std::os::unix::net::UnixStream;

    #[test]
    fn a_pipelined_session_over_a_unix_socket_matches_stdio_framing_byte_for_byte() {
        let settings = settings(StructuredContentMode::OmitDefaultPath);
        let mut script = format!("{INIT}\n{INITED}\n{LIST}\n");
        for _ in 0..200 {
            script.push_str(CALLEES);
            script.push('\n');
        }

        let registry = ToolRegistry::all_v1();
        let (_dir_a, mut over_stdio) = provider(RefreshPolicy::WarnOnly);
        let mut stdio_out = Vec::new();
        {
            let mut transport =
                LineTransport::new(std::io::Cursor::new(script.clone().into_bytes()), &mut stdio_out);
            serve_connection(&mut over_stdio, &registry, &mut transport, &settings).unwrap();
        }

        let (_dir_b, mut over_socket) = provider(RefreshPolicy::WarnOnly);
        let (client, server) = UnixStream::pair().expect("socket pair");
        let server_thread = std::thread::spawn(move || {
            let registry = ToolRegistry::all_v1();
            let mut transport = UnixStreamTransport::from_stream(server).expect("transport");
            serve_connection(&mut over_socket, &registry, &mut transport, &settings)
        });
        // Every request is written before any response is consumed; the reader below runs
        // concurrently only so the two socket buffers cannot deadlock each other.
        let mut write_half = client.try_clone().expect("clone client");
        let writer = std::thread::spawn(move || {
            write_half.write_all(script.as_bytes()).expect("client write");
            write_half.shutdown(Shutdown::Write).expect("client half-close");
        });
        let mut socket_out = Vec::new();
        let mut read_half = client;
        read_half.read_to_end(&mut socket_out).expect("client read");
        writer.join().expect("client writer");
        server_thread
            .join()
            .expect("serve loop must not panic")
            .expect("serve loop ends cleanly at end of input");

        assert_eq!(
            socket_out.iter().filter(|byte| **byte == b'\n').count(),
            202,
            "initialize + tools/list + 200 calls"
        );
        assert!(socket_out == stdio_out, "socket and stdio responses differ");
    }

    #[test]
    fn a_peer_that_disconnects_mid_frame_ends_the_connection_without_a_panic() {
        let settings = settings(StructuredContentMode::OmitDefaultPath);
        let (_dir, mut provider) = provider(RefreshPolicy::WarnOnly);
        let (mut client, server) = UnixStream::pair().expect("socket pair");
        let server_thread = std::thread::spawn(move || {
            let registry = ToolRegistry::all_v1();
            let mut transport = UnixStreamTransport::from_stream(server).expect("transport");
            serve_connection(&mut provider, &registry, &mut transport, &settings)
        });
        client
            .write_all(br#"{"jsonrpc":"2.0","id":1,"me"#)
            .expect("partial frame");
        drop(client);
        // The truncated frame is answered with a parse error nobody is left to read. A clean end
        // or a write error are both acceptable outcomes; a panic is not.
        let _outcome = server_thread.join().expect("serve loop must not panic");
    }
}
```

In `src/mcp/transport.rs`, next to the existing test module declarations at the bottom, add:

```rust
#[cfg(test)]
#[path = "transport_split_tests.rs"]
mod split_tests;
```

- [ ] **Step 2: Write the binary-level test for the warning cadence**

Create `tests/mcp/settings_test.rs`:

```rust
#![cfg(feature = "mcp")]

use assert_cmd::Command;

#[test]
fn invalid_output_env_value_warns_once_per_server() {
    let repo = tempfile::tempdir().expect("temp repo");
    std::fs::write(repo.path().join("a.py"), "def f():\n    return 1\n").expect("write a.py");
    let call = |id: u32| {
        format!(
            r#"{{"jsonrpc":"2.0","id":{id},"method":"tools/call","params":{{"name":"nav_callees","arguments":{{"seed":{{"kind":"symbol","name":"f","file":"a.py"}}}}}}}}"#
        )
    };
    let stdin = [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#.to_string(),
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#.to_string(),
        call(2),
        call(3),
    ]
    .join("\n")
        + "\n";

    let output = Command::cargo_bin("prism-mcp")
        .expect("prism-mcp binary")
        .arg("--repo")
        .arg(repo.path())
        .arg("--no-cache")
        .arg("--eager")
        .env("PRISM_MCP_CONCISE_SHAPE", "bogus")
        .env_remove("PRISM_MCP_STRUCTURED_CONTENT")
        .env_remove("PRISM_MCP_MAX_RESULT_CHARS")
        .write_stdin(stdin)
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let responses: Vec<serde_json::Value> = stdout
        .lines()
        .map(|line| serde_json::from_str(line).expect("stdout line is JSON"))
        .collect();
    assert_eq!(responses.len(), 3, "initialize and two calls");
    assert_eq!(responses[1]["result"]["isError"], false, "the server still serves");
    assert_eq!(responses[2]["result"]["isError"], false);

    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert_eq!(
        stderr.matches("PRISM_MCP_CONCISE_SHAPE=").count(),
        1,
        "one warning for the whole server, not one per call: {stderr}"
    );
}
```

Replace `tests/mcp/main.rs` with:

```rust
mod golden_transcript_test;
mod settings_test;
mod smoke_test;
mod transcript;
```

- [ ] **Step 3: Run both and see them fail**

```bash
cargo test --features mcp --test mcp settings_test > /tmp/s1-t4-red-bin.log 2>&1; grep -E '^test |one warning|^test result' /tmp/s1-t4-red-bin.log
cargo test --features mcp --lib mcp::transport::split_tests > /tmp/s1-t4-red-lib.log 2>&1; grep -E 'error\[|cannot find|unresolved|^test result' /tmp/s1-t4-red-lib.log | head
```

Expected: the binary test FAILS on the count assertion (the base warns twice: left `2`, right `1`). The lib tests fail to compile, naming `settings`, `handle_control`, `Control`, `ToolCall`, `serve_connection` and the dispatch functions.

- [ ] **Step 4: Add `ConnectionSettings`**

Create `src/mcp/settings.rs`:

```rust
//! Output settings of one MCP connection. The in-process server resolves them once from its own
//! environment; the shared daemon builds one per connection from the shim's preamble, so a
//! client's output shape never depends on which client started the daemon.
use super::concise_shape::{resolve_concise_shape_mode, ConciseShapeMode};
use super::output::{resolve_cap, resolve_structured_content_mode, StructuredContentMode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConnectionSettings {
    /// Wire byte cap per tool result (`PRISM_MCP_MAX_RESULT_CHARS`).
    pub result_cap: usize,
    /// Whether the default path carries `structuredContent` (`PRISM_MCP_STRUCTURED_CONTENT`).
    pub structured_content: StructuredContentMode,
    /// Item shape of concise results (`PRISM_MCP_CONCISE_SHAPE`).
    pub concise_shape: ConciseShapeMode,
}

impl ConnectionSettings {
    /// Resolve from this process's environment. Each invalid value warns on stderr when it is
    /// resolved, so call this once per connection, not once per request.
    pub fn from_env() -> Self {
        Self {
            result_cap: resolve_cap(),
            structured_content: resolve_structured_content_mode(),
            concise_shape: resolve_concise_shape_mode(),
        }
    }
}
```

In `src/mcp/mod.rs` add, in alphabetical position (after `session`):

```rust
pub mod settings;
```

- [ ] **Step 5: Split `handle_message` in `src/mcp/transport.rs`**

Make these edits. Functions not mentioned (`initialize`'s `server_instructions`, `list_tools`, `effective_stale_report`, `post_refresh_stale_report`, `cap_after_reserve`, `apply_auto_refresh_metadata`, `apply_auto_refresh_failure_metadata`, `unknown_tool_result`, `warming_result`, `build_failure_result`, `retryable_status_result`, `success_response`, `error_response`, `is_safe_id`, `safe_id_from_message`, `payload_budget`, the `SessionRuntime` trait and all its impls, `serve_session`, `serve_session_with_freshness`, `serve_stdio`) stay exactly as they are.

**5a. Imports.** Replace the `concise_shape`, `output` and `registry` import lines with:

```rust
#[cfg(test)]
use super::concise_shape::ConciseShapeMode;
#[cfg(test)]
use super::output::StructuredContentMode;
use super::output::{clamp_user_text, McpToolResult, MAX_RESULT_CHARS_FLOOR, SCHEMA_VERSION};
use super::registry::{ToolContext, ToolDescriptor, ToolRegistry, ToolRuntimeBehavior};
use super::settings::ConnectionSettings;
```

**5b. `Lifecycle`.** Change `enum Lifecycle` to `pub(super) enum Lifecycle` (keep the derive and the doc comment).

**5c. Serve loop.** Replace `serve_runtime` with:

```rust
fn serve_runtime(
    runtime: &mut impl SessionRuntime,
    registry: &ToolRegistry,
    transport: &mut impl Transport,
) -> anyhow::Result<()> {
    serve_connection(runtime, registry, transport, &ConnectionSettings::from_env())
}

/// Serve one connection until its transport reaches end of input. `settings` are this
/// connection's output settings, fixed for its whole life.
pub(super) fn serve_connection(
    runtime: &mut impl SessionRuntime,
    registry: &ToolRegistry,
    transport: &mut impl Transport,
    settings: &ConnectionSettings,
) -> anyhow::Result<()> {
    let mut state = Lifecycle::PreInit;

    while let Some(outcome) = transport.read_message()? {
        let line = match outcome {
            ReadOutcome::Message(line) => line,
            ReadOutcome::Malformed => {
                // Recoverable framing/encoding fault: reply and keep the session alive.
                transport.write_message(error_response(Value::Null, -32700, "Parse error"))?;
                continue;
            }
        };
        let message = match serde_json::from_str::<Value>(&line) {
            Ok(message) => message,
            Err(_) => {
                transport.write_message(error_response(Value::Null, -32700, "Parse error"))?;
                continue;
            }
        };

        let response =
            match handle_message_with(&message, runtime, registry, &mut state, settings) {
                Dispatch::Response(response) => Some(response),
                Dispatch::NoResponse => None,
            };
        if let Some(response) = response {
            transport.write_message(response)?;
        }
    }

    Ok(())
}
```

**5d. Control path.** Replace `enum Dispatch` and `fn handle_message` with:

```rust
enum Dispatch {
    Response(Value),
    NoResponse,
}

/// A `tools/call` that passed every check that needs no index: params shape, a registered tool
/// and, for `refresh_index`, empty arguments. Produced by `handle_control`; executed by the
/// `dispatch_tool_*` functions against a provider.
pub(super) struct ToolCall<'r> {
    id: Value,
    tool: &'r ToolDescriptor,
    arguments: Value,
}

impl ToolCall<'_> {
    /// Whether this call is `refresh_index`, which needs exclusive access to the provider.
    pub(super) fn is_refresh(&self) -> bool {
        self.tool.runtime_behavior == Some(ToolRuntimeBehavior::RefreshIndex)
    }
}

/// What the control path made of one message.
pub(super) enum Control<'r> {
    Response(Value),
    NoResponse,
    /// A validated tool call, to be run once the index is ready.
    ToolCall(ToolCall<'r>),
}

/// The in-process composition of control and dispatch over one `SessionRuntime`.
fn handle_message_with(
    message: &Value,
    runtime: &mut impl SessionRuntime,
    registry: &ToolRegistry,
    state: &mut Lifecycle,
    settings: &ConnectionSettings,
) -> Dispatch {
    match handle_control(message, state, registry, runtime.startup_mode(), settings) {
        Control::Response(response) => Dispatch::Response(response),
        Control::NoResponse => Dispatch::NoResponse,
        Control::ToolCall(call) => Dispatch::Response(dispatch_tool(&call, runtime, settings)),
    }
}

/// Test entry point with the pre-split signature (used by `owner_transport_tests.rs`).
#[cfg(all(test, feature = "detached-owner-audit"))]
fn handle_message(
    message: &Value,
    runtime: &mut impl SessionRuntime,
    registry: &ToolRegistry,
    state: &mut Lifecycle,
) -> Dispatch {
    handle_message_with(
        message,
        runtime,
        registry,
        state,
        &ConnectionSettings::from_env(),
    )
}

/// Everything a message needs that does not touch the index: envelope validation, the lifecycle
/// gate, `initialize`, `ping`, `tools/list`, and validation of a `tools/call` up to the point
/// where it must run. Takes no runtime, so a caller can answer the handshake while another
/// client's build or refresh holds the provider.
pub(super) fn handle_control<'r>(
    message: &Value,
    state: &mut Lifecycle,
    registry: &'r ToolRegistry,
    startup_mode: StartupMode,
    settings: &ConnectionSettings,
) -> Control<'r> {
    if message.is_array() {
        return Control::Response(error_response(Value::Null, -32600, "Invalid Request"));
    }
    let Some(obj) = message.as_object() else {
        return Control::Response(error_response(
            safe_id_from_message(message),
            -32600,
            "Invalid Request",
        ));
    };
    if obj.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Control::Response(error_response(
            safe_id_from_message(message),
            -32600,
            "Invalid Request",
        ));
    }
    let Some(method) = obj.get("method").and_then(Value::as_str) else {
        return Control::Response(error_response(
            safe_id_from_message(message),
            -32600,
            "Invalid Request",
        ));
    };

    // Notifications are fire-and-forget. MCP §9: receipt of `notifications/initialized` transitions the
    // session "regardless of its body", so handle any `notifications/*` BEFORE the id/gating checks — a
    // non-conformant client that attaches an `id` must not hit the pre-init gate and permanently deadlock
    // the session (round-8 MINOR). A stray `notifications/initialized` before a valid `initialize` is
    // still ignored (no transition).
    if method.starts_with("notifications/") {
        if method == "notifications/initialized" && *state == Lifecycle::InitializeReceived {
            *state = Lifecycle::Initialized;
        }
        return Control::NoResponse;
    }

    let id = obj.get("id").cloned();
    if id.is_none() {
        // A non-notification request with no id is invalid.
        return Control::Response(error_response(Value::Null, -32600, "Invalid Request"));
    }
    let id = id.unwrap_or(Value::Null);

    // Validate the echoed id's shape AND size (re-review). Same rule as `is_safe_id`; reject a bad id
    // with a Null-id error (we won't echo a bad id).
    if !is_safe_id(&id) {
        return Control::Response(error_response(
            Value::Null,
            -32600,
            "Invalid Request: bad id",
        ));
    }

    // Pre-initialized gating: only `initialize` and `ping` are allowed before `Initialized`.
    if *state != Lifecycle::Initialized && method != "initialize" && method != "ping" {
        return Control::Response(error_response(id, -32600, "Invalid Request"));
    }

    match method {
        "initialize" => {
            let response = initialize_response(obj, id, startup_mode);
            // Advance ONLY from PreInit, and only on a *successful* initialize. A repeat initialize once
            // negotiated responds but never downgrades the state (lifecycle is monotonic — re-review MAJOR).
            if *state == Lifecycle::PreInit && response.get("result").is_some() {
                *state = Lifecycle::InitializeReceived;
            }
            Control::Response(response)
        }
        "ping" => Control::Response(success_response(id, json!({}))),
        "tools/list" => Control::Response(success_response(id, list_tools(registry))),
        "tools/call" => parse_tool_call(obj, id, registry, settings),
        _ => Control::Response(error_response(id, -32601, "Method not found")),
    }
}
```

**5e. `initialize_response` returns the response value.** Change its signature to `fn initialize_response(obj: &Map<String, Value>, id: Value, startup_mode: StartupMode) -> Value` and unwrap its three `Dispatch::Response(…)` returns: `return error_response(id, -32602, "Invalid params");` (twice) and the final expression becomes `success_response(id, json!({ … }))` with the JSON body unchanged. Keep both comments.

**5f. Call validation and dispatch.** Replace `call_tool_response`, `call_tool_response_with_cap`, `call_tool_response_with_cap_and_mode` and `auto_refresh_tool_response` with:

```rust
/// Test-only entry point that fixes the cap but not the S2/S3 env-gated modes (always
/// `StructuredContentMode::Always` and `ConciseShapeMode::Legacy`, i.e. today's unconditional
/// behavior) — existing `transport_tests.rs` callers inject a deterministic `cap` this way without
/// touching either gate.
#[cfg(test)]
fn call_tool_response_with_cap(
    obj: &Map<String, Value>,
    id: Value,
    runtime: &mut impl SessionRuntime,
    registry: &ToolRegistry,
    original_cap: usize,
) -> Dispatch {
    call_tool_response_with_cap_and_mode(
        obj,
        id,
        runtime,
        registry,
        original_cap,
        StructuredContentMode::Always,
        ConciseShapeMode::Legacy,
    )
}

/// Test-only entry point with the pre-split signature: one `tools/call` with every output setting
/// given explicitly.
#[cfg(test)]
fn call_tool_response_with_cap_and_mode(
    obj: &Map<String, Value>,
    id: Value,
    runtime: &mut impl SessionRuntime,
    registry: &ToolRegistry,
    original_cap: usize,
    structured_content_mode: StructuredContentMode,
    concise_shape_mode: ConciseShapeMode,
) -> Dispatch {
    let settings = ConnectionSettings {
        result_cap: original_cap,
        structured_content: structured_content_mode,
        concise_shape: concise_shape_mode,
    };
    match parse_tool_call(obj, id, registry, &settings) {
        Control::Response(response) => Dispatch::Response(response),
        Control::NoResponse => Dispatch::NoResponse,
        Control::ToolCall(call) => Dispatch::Response(dispatch_tool(&call, runtime, &settings)),
    }
}

/// Validate a `tools/call` as far as possible without the index.
fn parse_tool_call<'r>(
    obj: &Map<String, Value>,
    id: Value,
    registry: &'r ToolRegistry,
    settings: &ConnectionSettings,
) -> Control<'r> {
    let Some(params) = obj.get("params").and_then(Value::as_object) else {
        return Control::Response(error_response(id, -32602, "Invalid params"));
    };
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return Control::Response(error_response(id, -32602, "Invalid params"));
    };
    // Per MCP, `arguments` must be an object. A wrong-TYPE `arguments` is a protocol error (-32602),
    // distinct from a bad tool INPUT (which correctly returns a result with isError) — round-6 MAJOR.
    let arguments = match params.get("arguments") {
        None => json!({}),
        Some(v) if v.is_object() => v.clone(),
        Some(_) => {
            return Control::Response(error_response(
                id,
                -32602,
                "Invalid params: arguments must be an object",
            ));
        }
    };

    let Some(tool) = registry.get(name) else {
        return Control::Response(success_response(
            id,
            unknown_tool_result(name, registry)
                .to_call_tool_result_value(settings.structured_content),
        ));
    };

    if tool.runtime_behavior == Some(ToolRuntimeBehavior::RefreshIndex)
        && !arguments.as_object().is_some_and(|obj| obj.is_empty())
    {
        return Control::Response(success_response(
            id,
            tools_refresh::invalid_arguments_result()
                .to_call_tool_result_value(settings.structured_content),
        ));
    }

    Control::ToolCall(ToolCall {
        id,
        tool,
        arguments,
    })
}

/// Run one validated call against an in-process runtime: readiness first, then the refresh tool
/// or a read that may hand over to the automatic-refresh path.
fn dispatch_tool(
    call: &ToolCall<'_>,
    runtime: &mut impl SessionRuntime,
    settings: &ConnectionSettings,
) -> Value {
    let readiness = runtime.ensure_ready();
    if let Some(response) =
        readiness_response(call, readiness, || runtime.admission_diagnostic(), settings)
    {
        return response;
    }
    if call.is_refresh() {
        return dispatch_tool_refresh(call, runtime, settings);
    }
    match dispatch_tool_read(call, &*runtime, settings) {
        ReadDispatch::Response(response) => response,
        ReadDispatch::NeedsAutoRefresh { initial_report } => {
            dispatch_tool_auto_refresh(call, runtime, initial_report, settings)
        }
    }
}

/// The response for a call that cannot run yet; `None` when the index is ready. `admission` is
/// asked only when the build failed.
pub(super) fn readiness_response(
    call: &ToolCall<'_>,
    readiness: Readiness,
    admission: impl FnOnce() -> Option<Value>,
    settings: &ConnectionSettings,
) -> Option<Value> {
    let result = match readiness {
        Readiness::Ready => return None,
        Readiness::Warming { elapsed } => warming_result(elapsed),
        Readiness::Failed { error } => build_failure_result(&error, admission()),
    };
    Some(success_response(
        call.id.clone(),
        result.to_call_tool_result_value(settings.structured_content),
    ))
}

/// `refresh_index`: rebuild the snapshot. Needs exclusive access to the provider.
pub(super) fn dispatch_tool_refresh(
    call: &ToolCall<'_>,
    provider: &mut impl SessionRuntime,
    settings: &ConnectionSettings,
) -> Value {
    let result = match provider.refresh_index() {
        Ok(summary) => tools_refresh::refresh_result(&summary),
        Err(error) => tools_refresh::refresh_error_result(&error),
    };
    success_response(
        call.id.clone(),
        result.to_call_tool_result_value(settings.structured_content),
    )
}

/// Outcome of running a query with shared access.
pub(super) enum ReadDispatch {
    Response(Value),
    /// The index is stale and the refresh policy is automatic: the caller must take exclusive
    /// access and finish through `dispatch_tool_auto_refresh`, passing this report on.
    NeedsAutoRefresh {
        initial_report: Option<FreshnessReport>,
    },
}

/// Run a query against a ready provider through shared access only. `view` is the read-only
/// facade: the whole provider, including the sticky stale evidence a refresh recorded, with the
/// precedence `effective_stale_report` gives it.
pub(super) fn dispatch_tool_read(
    call: &ToolCall<'_>,
    view: &impl SessionRuntime,
    settings: &ConnectionSettings,
) -> ReadDispatch {
    let report = effective_stale_report(view);
    let stale = report.as_ref().is_some_and(|report| report.stale);
    if stale
        && matches!(
            view.refresh_policy(),
            RefreshPolicy::AutoFull | RefreshPolicy::AutoIncremental
        )
    {
        return ReadDispatch::NeedsAutoRefresh {
            initial_report: report,
        };
    }
    let cap = if stale {
        cap_after_reserve(settings.result_cap, FRESHNESS_RESERVE_BYTES)
    } else {
        settings.result_cap
    };
    let ctx = ToolContext::new(
        view.session(),
        cap,
        settings.concise_shape,
        settings.structured_content,
    );
    let mut result = (call.tool.handler)(&ctx, &call.arguments);
    if stale && !result.is_error && result.structured.is_some() {
        if let Some(report) = &report {
            apply_freshness_report(&mut result, report, settings.result_cap);
        }
    }

    ReadDispatch::Response(success_response(
        call.id.clone(),
        result.to_call_tool_result_value(settings.structured_content),
    ))
}

/// Refresh a stale index under an automatic policy, then run the query on the result. Needs
/// exclusive access to the provider.
pub(super) fn dispatch_tool_auto_refresh(
    call: &ToolCall<'_>,
    provider: &mut impl SessionRuntime,
    initial_report: Option<FreshnessReport>,
    settings: &ConnectionSettings,
) -> Value {
    let handler = call.tool.handler.as_ref();
    let original_cap = settings.result_cap;
    let mode = settings.structured_content;
    let respond = |result: McpToolResult| {
        success_response(call.id.clone(), result.to_call_tool_result_value(mode))
    };
    match provider.auto_refresh_index() {
        Ok(summary) => {
            let stale_after_refresh = post_refresh_stale_report(provider, &summary);
            let may_apply_stale = stale_after_refresh
                .as_ref()
                .is_some_and(|report| report.stale);
            let reserve = AUTO_REFRESH_RESERVE_BYTES
                + if may_apply_stale {
                    FRESHNESS_RESERVE_BYTES
                } else {
                    0
                };
            let ctx = ToolContext::new(
                provider.session(),
                cap_after_reserve(original_cap, reserve),
                settings.concise_shape,
                mode,
            );
            let mut result = handler(&ctx, &call.arguments);
            if result.is_error {
                return respond(result);
            }
            let status = match &summary.verification {
                RefreshVerification::Clean => "refreshed",
                RefreshVerification::Diverged(_) => "raced_stale",
            };
            apply_auto_refresh_metadata(&mut result, status, &summary, original_cap);
            if let Some(report) = stale_after_refresh.as_ref().filter(|report| report.stale) {
                if result.structured.is_some() {
                    apply_freshness_report(&mut result, report, original_cap);
                }
            }
            respond(result)
        }
        Err(error) => {
            let reserve = AUTO_REFRESH_RESERVE_BYTES + FRESHNESS_RESERVE_BYTES;
            let ctx = ToolContext::new(
                provider.session(),
                cap_after_reserve(original_cap, reserve),
                settings.concise_shape,
                mode,
            );
            let mut result = handler(&ctx, &call.arguments);
            if result.is_error {
                return respond(result);
            }
            if let Some(report) = initial_report.as_ref().filter(|report| report.stale) {
                if result.structured.is_some() {
                    apply_freshness_report(&mut result, report, original_cap);
                }
            }
            apply_auto_refresh_failure_metadata(&mut result, &error, original_cap);
            respond(result)
        }
    }
}
```

Compare `dispatch_tool_auto_refresh` against the removed `auto_refresh_tool_response` line by line before deleting the old one: the two arms must perform the same calls in the same order with the same caps. The only differences allowed are the parameter sources (`call`, `settings`) and the `respond` closure replacing the repeated `Dispatch::Response(success_response(…))`.

- [ ] **Step 6: Compile, fix imports the compiler names, and run the new tests**

```bash
cargo fmt
cargo test --features mcp --lib mcp::transport::split_tests > /tmp/s1-t4-green-lib.log 2>&1; grep -E '^test |^test result|error(\[|:)|warning: unused' /tmp/s1-t4-green-lib.log
cargo test --features mcp --test mcp settings_test > /tmp/s1-t4-green-bin.log 2>&1; grep -E '^test |^test result' /tmp/s1-t4-green-bin.log
```

Expected: 12 tests pass under `split_tests` (10 at the top level, 2 under `unix::`); `settings_test` passes with 1 test. If the compiler reports an unused import in `transport.rs`, gate it with `#[cfg(test)]` when it is on the corpus list in **File Structure**, and delete it otherwise. If it reports an unresolved name inside `transport_tests.rs`, restore that name in `transport.rs`; do not edit the corpus.

- [ ] **Step 7: Run the whole MCP surface, the goldens, and the zero-diff check**

```bash
cargo test --features mcp --lib mcp:: > /tmp/s1-t4-lib.log 2>&1; grep -E '^test result' /tmp/s1-t4-lib.log
cargo test --features mcp --test mcp > /tmp/s1-t4-mcp.log 2>&1; grep -E '^test result|differs' /tmp/s1-t4-mcp.log
cargo check --tests --features mcp,detached-owner-audit > /tmp/s1-t4-owner.log 2>&1; tail -1 /tmp/s1-t4-owner.log
git diff --stat origin/main -- src/mcp/transport_tests.rs src/mcp/owner_transport_tests.rs | wc -l
T1=$(git log --format=%H --diff-filter=A -- tests/fixtures/mcp/golden/eager-default.jsonl | tail -1)
git diff --stat "$T1" -- tests/fixtures/mcp/golden | wc -l
```

Expected: lib total = base 195 + 4 (Task 3) + 12 (this task) = 211 passed, 0 failed; the `mcp` target shows 8 passed (4 base + 3 Task 1 + 1 this task); the owner-audit check finishes (it compiles `owner_transport_tests.rs` against the kept `handle_message`); both diff counts are `0` (`$T1` is the Task 1 commit, the one that added the goldens; the second diff compares it with the working tree).

- [ ] **Step 8: Commit**

```bash
git add src/mcp/settings.rs src/mcp/transport.rs src/mcp/transport_split_tests.rs src/mcp/mod.rs tests/mcp/settings_test.rs tests/mcp/main.rs
git commit -m "refactor(mcp): split control from tool dispatch; explicit per-connection output settings"
```

---

### Task 5: Proof, docs, and the pull request

**Files:**
- Modify: `CLAUDE.md` (MCP Adapter section)
- Modify: `docs/MCP.md` only if Step 2 finds a statement this slice made false

**Interfaces:**
- Consumes: everything above.
- Produces: the PR, with the proof and the exclusions stated.

- [ ] **Step 1: Run the full verification set and keep the logs**

```bash
cargo fmt --check
cargo test --features mcp > /tmp/s1-full-mcp.log 2>&1; grep -E '^test result|FAILED|failed' /tmp/s1-full-mcp.log
cargo check > /tmp/s1-default-check.log 2>&1; tail -1 /tmp/s1-default-check.log
cargo test --doc --features mcp > /tmp/s1-doc.log 2>&1; grep -E '^test result' /tmp/s1-doc.log
cargo clippy --all-targets --features mcp -- -W clippy::all > /tmp/s1-clippy.log 2>&1
grep -E -A8 '^warning' /tmp/s1-clippy.log | grep -E 'src/mcp/(framing|settings|transport)\.rs|src/mcp/(framing_tests|transport_split_tests)\.rs|tests/mcp/' | sort -u
```

Expected: `cargo fmt --check` prints nothing; every `test result` line shows `0 failed`; sum the `passed` counts and report the total; the last command lists no warning in a file this slice created, and for `transport.rs` no warning that is absent on the base (if `transport.rs` shows any line, run the same two commands in a second worktree checked out at the Task 1 commit and compare; do not use `git stash`).

- [ ] **Step 2: State the proof**

```bash
BASE=$(git merge-base origin/main HEAD)
git diff --stat "$BASE" -- src/mcp/transport_tests.rs src/mcp/owner_transport_tests.rs | wc -l     # 0
T1=$(git log --format=%H --diff-filter=A -- tests/fixtures/mcp/golden/eager-default.jsonl | tail -1)
git diff --stat "$T1" HEAD -- tests/fixtures/mcp/golden | wc -l                                     # 0
git show --stat --format=%s "$T1" -- src/ | tail -n +2 | wc -l                                      # 0: Task 1 touched no src/
grep -rn 'per call\|every call\|each call' docs/MCP.md | grep -i 'PRISM_MCP_\|warn'
```

Expected: three zeros. The last command finds any documentation sentence about per-call warnings; correct it if one exists, otherwise leave `docs/MCP.md` alone.

- [ ] **Step 3: Update the architecture notes**

In `CLAUDE.md`, in the **MCP Adapter** section, after the paragraph that begins "`Evidence` also has an additive optional `reasoning` field", add:

```markdown
Transport layout: `src/mcp/framing.rs` holds the newline-delimited JSON framing as one generic
`LineTransport<R, W>`; stdio and the Unix-socket `UnixStreamTransport` are instantiations of it.
`src/mcp/transport.rs` splits a message into `handle_control` (lifecycle, `tools/list`, call
validation; needs no index) and `dispatch_tool_read` / `dispatch_tool_refresh` /
`dispatch_tool_auto_refresh` (shared versus exclusive access to the provider). Output settings are
an explicit `ConnectionSettings` (`src/mcp/settings.rs`), resolved from the environment once per
server, so an invalid `PRISM_MCP_*` output value warns once, not once per call.
`tests/fixtures/mcp/golden/` pins the in-process server's stdout; re-record with
`PRISM_MCP_BLESS_GOLDEN=1 cargo test --features mcp --test mcp golden_transcript_test` only for a
change that is meant to alter tool output, and say why in the commit.
```

- [ ] **Step 4: Run the pre-merge mutation gate and state what it covers**

Run: `python3 scripts/mutgate/mutgate.py > /tmp/s1-mutgate.log 2>&1; tail -5 /tmp/s1-mutgate.log`

Expected: exit 0 with every selected verdict KILLED. Report it as: "full gate green; it builds without `--features mcp`, so it selects no code from this slice and is evidence only that the rest of the crate is unaffected." Do not present it as mutation coverage of S1.

- [ ] **Step 5: Commit and open the PR for review**

```bash
git add CLAUDE.md docs/MCP.md
git commit -m "docs(mcp): transport layout and the golden-transcript contract"
git push
gh pr ready
```

PR title: `refactor(mcp): transport/framing split with explicit connection settings (shared daemon S1)`

The PR body states, in this order:

1. **Goal:** the lane goal from the lane plan, and that S1 changes no standalone stdout.
2. **Proof:** goldens recorded in the first commit (hash), which touches no `src/` file and was CI-green alone; goldens unchanged since; zero-line diff in `transport_tests.rs` and `owner_transport_tests.rs`.
3. **The one intended change:** invalid output env values warn once per server instead of once per call, with the test that pins it.
4. **Totals:** the full-suite `passed`/`failed` numbers from Step 1 next to the Task 1 base numbers.
5. **Not verified:** a non-Unix build (no Windows target is installed or in CI); the owner-audit tests were compiled but not run unless `PRISM_TYPESCRIPT` was set; the mutation gate does not cover MCP code.
6. **Tier-A harness:** not run; this slice touches none of `src/call_graph.rs`, `src/navigation/`, `src/cpg/`, `src/ast.rs`.

---

## Self-review record

- **Spec coverage (§12 row S1):** `UnixStreamTransport` → Task 3. `handle_control`/`dispatch_tool` split → Task 4 (the spec's single `dispatch_tool(message, view, settings)` is three functions, one per access mode, because the read path must be callable with shared access only). Read-only provider facade → `&impl SessionRuntime` in `dispatch_tool_read`, Task 4. `ConnectionSettings` explicit → Task 4. Standalone byte-for-byte proof against post-#356 → Task 1 (recorded on base) plus Tasks 2–5 (unchanged) and the zero-diff corpus. §9 unit item "`UnixStreamTransport` shares the stdio framing tests through the `Transport` trait" → Task 3.
- **Out of S1 by the spec's own slicing:** preamble and connection-local `first_call_wait`/`warm_at_startup` (S2); readiness coordinator and `RwLock` (S3a); auto-refresh trigger re-check and panic boundaries (S3b).
- **Disclosed deviation from the spec text:** §4.5 says the in-process server building settings once is "today's behavior". Today it resolves them on every `tools/call`. S1 makes the spec's description true and tests the resulting stderr change.
