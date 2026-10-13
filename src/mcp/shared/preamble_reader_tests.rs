//! Tests for the bounded preamble line reader.
use super::*;
use crate::mcp::framing::{ReadOutcome, Transport, UnixStreamTransport};
use crate::mcp::shared::preamble::PREAMBLE_MAX_BYTES;
use std::collections::VecDeque;
use std::io::{Cursor, ErrorKind, Read, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

/// A reader that hands out scripted chunks and errors, one per `read` call, and counts the
/// bytes it has handed out.
struct Scripted {
    steps: VecDeque<Result<Vec<u8>, ErrorKind>>,
    handed_out: usize,
}

impl Scripted {
    fn new(steps: Vec<Result<&[u8], ErrorKind>>) -> Self {
        Self {
            steps: steps
                .into_iter()
                .map(|step| step.map(<[u8]>::to_vec))
                .collect(),
            handed_out: 0,
        }
    }
}

impl Read for Scripted {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.steps.pop_front() {
            None => Ok(0),
            Some(Err(kind)) => Err(kind.into()),
            Some(Ok(mut chunk)) => {
                let n = chunk.len().min(buf.len());
                buf[..n].copy_from_slice(&chunk[..n]);
                if n < chunk.len() {
                    self.steps.push_front(Ok(chunk.split_off(n)));
                }
                self.handed_out += n;
                Ok(n)
            }
        }
    }
}

#[test]
fn a_line_is_returned_without_its_newline_and_the_next_byte_is_left_unread() {
    let mut source = Cursor::new(b"READY {}\nrest of the stream".to_vec());
    let line = PreambleLineReader::new().read_line(&mut source).unwrap();
    assert_eq!(line, b"READY {}");
    assert_eq!(
        source.position(),
        9,
        "only the line and its newline were taken"
    );
    let mut rest = String::new();
    source.read_to_string(&mut rest).unwrap();
    assert_eq!(rest, "rest of the stream");
}

/// The reason for reading a byte at a time: after the preamble the same socket carries MCP
/// frames, and the first of them may already be in the socket buffer. Rules out reading the
/// preamble through a `BufReader` that is then dropped with the MCP bytes inside it.
#[test]
fn the_mcp_frame_that_follows_the_preamble_on_a_socket_is_intact() {
    let (mut writer, reader) = UnixStream::pair().unwrap();
    let mcp = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
    writer
        .write_all(format!("RETIRING {{}}\n{mcp}\n").as_bytes())
        .unwrap();
    // Closed, so a reader that lost the MCP bytes meets end of stream instead of waiting.
    drop(writer);

    let mut stream = reader;
    let line = PreambleLineReader::new().read_line(&mut stream).unwrap();
    assert_eq!(line, b"RETIRING {}");

    let mut transport = UnixStreamTransport::from_stream(stream).unwrap();
    match transport.read_message().unwrap() {
        Some(ReadOutcome::Message(message)) => assert_eq!(message, mcp),
        _ => panic!("the MCP frame after the preamble was lost"),
    }
}

/// Spec §4.2 and §7: a preamble line is at most 4 KiB.
#[test]
fn a_line_of_exactly_4_kib_is_read_and_one_byte_more_is_too_long() {
    let mut at_limit = vec![b'a'; PREAMBLE_MAX_BYTES];
    at_limit.push(b'\n');
    let line = PreambleLineReader::new()
        .read_line(&mut Cursor::new(at_limit))
        .unwrap();
    assert_eq!(line.len(), PREAMBLE_MAX_BYTES);

    let mut over = vec![b'a'; PREAMBLE_MAX_BYTES + 1];
    over.push(b'\n');
    let outcome = PreambleLineReader::new().read_line(&mut Cursor::new(over));
    assert!(
        matches!(outcome, Err(PreambleReadError::TooLong)),
        "{outcome:?}"
    );
}

/// Rules out reading the whole line and measuring it afterwards: a megabyte with no newline
/// is offered, and the reader must give up having taken 4 KiB and one byte.
#[test]
fn an_endless_line_is_given_up_on_after_one_byte_past_the_limit() {
    let megabyte = vec![b'a'; 1 << 20];
    let mut source = Scripted::new(vec![Ok(megabyte.as_slice())]);
    let mut reader = PreambleLineReader::new();
    let outcome = reader.read_line(&mut source);
    assert!(
        matches!(outcome, Err(PreambleReadError::TooLong)),
        "{outcome:?}"
    );
    assert_eq!(source.handed_out, PREAMBLE_MAX_BYTES + 1);
}

/// Once a line was too long the byte that proved it is gone, so nothing later on this
/// connection can be trusted to be a line.
#[test]
fn a_reader_that_overflowed_stays_overflowed() {
    let mut bytes = vec![b'a'; PREAMBLE_MAX_BYTES + 1];
    bytes.extend_from_slice(b"\nHELLO {}\n");
    let mut source = Cursor::new(bytes);
    let mut reader = PreambleLineReader::new();
    assert!(matches!(
        reader.read_line(&mut source),
        Err(PreambleReadError::TooLong)
    ));
    let again = reader.read_line(&mut source);
    assert!(
        matches!(again, Err(PreambleReadError::TooLong)),
        "{again:?}"
    );
}

/// Spec §4.1 step 5: end of stream before the reply is a transient condition the shim acts on,
/// so it is its own outcome, not an I/O error and not an empty line.
#[test]
fn end_of_stream_before_the_newline_is_reported_with_what_had_arrived() {
    let outcome = PreambleLineReader::new().read_line(&mut Cursor::new(b"READ".to_vec()));
    assert!(
        matches!(outcome, Err(PreambleReadError::Eof { received: 4 })),
        "{outcome:?}"
    );
    let outcome = PreambleLineReader::new().read_line(&mut Cursor::new(Vec::new()));
    assert!(
        matches!(outcome, Err(PreambleReadError::Eof { received: 0 })),
        "{outcome:?}"
    );
}

/// A non-blocking socket or a read timeout interrupts a line part-way. Rules out keeping the
/// partial line in a local variable: the second call must finish the same line.
#[test]
fn a_line_interrupted_by_would_block_continues_on_the_next_call() {
    let mut source = Scripted::new(vec![
        Ok(b"HEL"),
        Err(ErrorKind::WouldBlock),
        Ok(b"LO {}"),
        Err(ErrorKind::TimedOut),
        Ok(b"\nnext"),
    ]);
    let mut reader = PreambleLineReader::new();
    let first = reader.read_line(&mut source);
    assert!(
        matches!(&first, Err(PreambleReadError::Io(error)) if error.kind() == ErrorKind::WouldBlock),
        "{first:?}"
    );
    let second = reader.read_line(&mut source);
    assert!(
        matches!(&second, Err(PreambleReadError::Io(error)) if error.kind() == ErrorKind::TimedOut),
        "{second:?}"
    );
    assert_eq!(reader.read_line(&mut source).unwrap(), b"HELLO {}");
    assert_eq!(source.handed_out, 9, "nothing after the newline was taken");
}

#[test]
fn an_interrupted_read_is_retried_and_other_errors_are_passed_on() {
    let mut source = Scripted::new(vec![Ok(b"BU"), Err(ErrorKind::Interrupted), Ok(b"SY {}\n")]);
    let line = PreambleLineReader::new().read_line(&mut source).unwrap();
    assert_eq!(line, b"BUSY {}");

    let mut source = Scripted::new(vec![Ok(b"BU"), Err(ErrorKind::ConnectionReset)]);
    let outcome = PreambleLineReader::new().read_line(&mut source);
    assert!(
        matches!(&outcome, Err(PreambleReadError::Io(error)) if error.kind() == ErrorKind::ConnectionReset),
        "{outcome:?}"
    );
}

/// The same continuation against a real socket with a read timeout, which is how a shim
/// bounds its wait for the reply.
#[test]
fn a_socket_read_timeout_is_an_io_outcome_and_the_line_continues_afterwards() {
    let (mut writer, mut reader_end) = UnixStream::pair().unwrap();
    reader_end
        .set_read_timeout(Some(Duration::from_millis(50)))
        .unwrap();
    writer.write_all(b"MISMATCH {\"expec").unwrap();

    // The rest is written once the first call has returned. Should that call never return (a
    // reader that waits out the timeout itself), the rest is written anyway after two
    // seconds, so that mistake fails this test instead of hanging it.
    let (first_call_returned, released) = std::sync::mpsc::channel::<()>();
    let rest = std::thread::spawn(move || {
        let _ = released.recv_timeout(Duration::from_secs(2));
        writer.write_all(b"ted\":\"k\"}\n").unwrap();
        // `writer` is dropped here, which ends the stream for the reader.
    });
    let timed_out = |error: &std::io::Error| {
        matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut)
    };

    let mut reader = PreambleLineReader::new();
    let first = reader.read_line(&mut reader_end);
    let _ = first_call_returned.send(());
    assert!(
        matches!(&first, Err(PreambleReadError::Io(error)) if timed_out(error)),
        "{first:?}"
    );

    let line = loop {
        match reader.read_line(&mut reader_end) {
            Ok(line) => break line,
            Err(PreambleReadError::Io(error)) if timed_out(&error) => {}
            Err(other) => panic!("{other:?}"),
        }
    };
    assert_eq!(line, b"MISMATCH {\"expected\":\"k\"}");

    rest.join().unwrap();
    let closed = reader.read_line(&mut reader_end);
    assert!(
        matches!(closed, Err(PreambleReadError::Eof { received: 0 })),
        "{closed:?}"
    );
}

#[test]
fn what_one_side_encodes_the_other_reads_and_decodes() {
    use crate::mcp::shared::identity::IdentityKey;
    use crate::mcp::shared::preamble::Reply;
    let reply = Reply::Mismatch {
        expected: IdentityKey::from_hex(&"ab".repeat(32)).unwrap(),
    };
    let (mut daemon, mut shim) = UnixStream::pair().unwrap();
    daemon.write_all(reply.encode().as_bytes()).unwrap();
    drop(daemon);
    let line = PreambleLineReader::new().read_line(&mut shim).unwrap();
    assert_eq!(Reply::decode(&line), Ok(reply));
}

#[test]
fn every_outcome_prints_on_one_line() {
    for error in [
        PreambleReadError::TooLong,
        PreambleReadError::Eof { received: 12 },
        PreambleReadError::Io(ErrorKind::TimedOut.into()),
    ] {
        let text = error.to_string();
        assert!(!text.is_empty() && !text.contains('\n'), "{text}");
    }
    assert!(PreambleReadError::TooLong.to_string().contains("4096"));
}
