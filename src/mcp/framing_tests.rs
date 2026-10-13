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
                format!(
                    "message of {} bytes, hash {:016x}",
                    text.len(),
                    hasher.finish()
                )
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
        (
            "two lines",
            b"a\nb\n".to_vec(),
            vec![message("a"), message("b")],
        ),
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
        (
            "an empty line is a message",
            b"\n".to_vec(),
            vec![message("")],
        ),
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
        assert_eq!(
            describe(&drain(&mut transport)),
            describe(&expected),
            "case: {name}"
        );
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
        reader
            .read_line(&mut line)
            .expect("first line arrives without closing");
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
