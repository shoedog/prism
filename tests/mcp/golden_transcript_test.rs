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
        name: "lazy-idle-off",
        flags: &["--first-call-wait", "60", "--index-idle-secs", "0"],
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
    let name = if name == "lazy-idle-off" {
        "lazy-default"
    } else {
        name
    };
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
    let bless = std::env::var("PRISM_MCP_BLESS_GOLDEN").is_ok_and(|value| value == "1");
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

#[test]
fn stdio_background_instructions_follow_effective_idle_policy() {
    for (name, flags, idle) in [
        ("background-idle", &["--warm-at-startup"][..], true),
        (
            "background-idle-off",
            &["--warm-at-startup", "--index-idle-secs", "0"][..],
            false,
        ),
    ] {
        let repo = tempfile::tempdir().unwrap();
        write_fixture(repo.path());
        let mode = Mode {
            name,
            flags,
            env: &[],
        };
        let bytes = run_transcript(repo.path(), &mode, &script()[3..4]);
        let response: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let text = response["result"]["instructions"].as_str().unwrap();
        assert!(text.contains("started at server startup"));
        assert_eq!(
            text.contains("first tool call after an idle unload"),
            idle,
            "{text}"
        );
    }
}
