#![cfg(feature = "mcp")]
//! The golden-transcript driver itself: output that arrives late or fails to read must fail the
//! run, never shorten the transcript a golden is compared with.

use crate::transcript::{drain_tail, drive, spawn_line_reader, Step, TailError};
use std::io::Read;
use std::process::Command;
use std::time::{Duration, Instant};

/// A shell that prints one line, then becomes `sleep` (exec, so killing the shell closes the
/// pipe) with stdout still open.
fn child_that_keeps_stdout_open() -> Command {
    let mut command = Command::new("/bin/sh");
    command.arg("-c").arg("printf 'early\\n'; exec sleep 30");
    command
}

#[test]
fn a_timeout_draining_trailing_output_is_an_error_not_a_shorter_transcript() {
    let root = tempfile::tempdir().expect("root");
    let started = Instant::now();
    let outcome = drive(
        child_that_keeps_stdout_open(),
        "fake",
        root.path(),
        &[],
        Duration::from_millis(500),
    );
    let failure =
        outcome.expect_err("stdout never closed: the run must fail, not return a transcript");
    assert!(
        failure.contains("timed out after 500ms"),
        "the error names the timeout: {failure}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(20),
        "the child was killed rather than waited for: {:?}",
        started.elapsed()
    );
}

#[test]
fn a_child_that_closes_stdout_ends_the_drain_cleanly_with_everything_it_wrote() {
    let root = tempfile::tempdir().expect("root");
    let mut command = Command::new("/bin/sh");
    command.arg("-c").arg("printf 'one\\ntwo\\n'");
    let transcript = drive(command, "fake", root.path(), &[], Duration::from_secs(20))
        .expect("a child that writes two lines and exits is a clean run");
    assert_eq!(transcript, b"one\ntwo\n");
}

#[test]
fn a_late_line_after_the_script_is_kept_not_dropped() {
    let root = tempfile::tempdir().expect("root");
    let mut command = Command::new("/bin/sh");
    command
        .arg("-c")
        .arg("read request; printf 'reply\\n'; sleep 1; printf 'late\\n'");
    let transcript = drive(
        command,
        "fake",
        root.path(),
        &[Step::Request("ping".into())],
        Duration::from_secs(20),
    )
    .expect("clean run");
    assert_eq!(transcript, b"reply\nlate\n");
}

#[test]
fn a_stdout_read_error_is_propagated_not_treated_as_the_end() {
    struct FailsAfterOneLine(bool);
    impl Read for FailsAfterOneLine {
        fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
            if self.0 {
                return Err(std::io::Error::other("pipe went bad"));
            }
            self.0 = true;
            out[..5].copy_from_slice(b"line\n");
            Ok(5)
        }
    }
    let (lines, reader) = spawn_line_reader(FailsAfterOneLine(false));
    let mut transcript = Vec::new();
    let outcome = drain_tail(&lines, Duration::from_secs(20), &mut transcript);
    reader.join().expect("reader thread");
    match outcome {
        Err(TailError::ReadFailed(error)) => assert!(error.to_string().contains("pipe went bad")),
        other => panic!("a read error must surface as ReadFailed, got {other:?}"),
    }
    assert_eq!(
        transcript, b"line\n",
        "what arrived before the failure is kept"
    );
}
