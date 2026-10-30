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
    let mixed = mixed_width_line();
    // 4 bytes each: 300_000 of them is 1_200_000 bytes, past the limit by a wide margin.
    let oversized_wide = "𝄞".repeat(300_000);
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
        (
            "two-, three- and four-byte characters at every alignment",
            format!("{mixed}\nok\n").into_bytes(),
            vec![message(mixed.clone()), message("ok")],
        ),
        (
            "a truncated multi-byte sequence is malformed and the next line is read",
            vec![b'a', 0xe2, 0x82, b'\n', b'o', b'k', b'\n'],
            vec![Got::Malformed, message("ok")],
        ),
        (
            "an oversized multi-byte line is malformed and the stream resyncs",
            format!("{oversized_wide}\n{mixed}\n").into_bytes(),
            vec![Got::Malformed, message(mixed.clone())],
        ),
    ]
}

/// A line holding `é` (2 bytes), `€` (3) and `𝄞` (4), each placed at every byte offset modulo 42
/// (the least common multiple of the chunk sizes 2, 3 and 7), so a chunk boundary of any size in
/// the tests falls inside each kind of character.
fn mixed_width_line() -> String {
    let mut line = String::new();
    for ch in ['é', '€', '𝄞'] {
        for offset in 0..42 {
            while line.len() % 42 != offset {
                line.push('a');
            }
            line.push(ch);
        }
    }
    line
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

/// A reader whose `fill_buf` yields at most `chunk` bytes, whatever is buffered behind it. Written
/// out rather than `BufReader::with_capacity(chunk, ..)`: the guarantee is then this adapter's
/// own three lines, not an implementation detail of std's buffer refill.
struct Chunked {
    data: Vec<u8>,
    at: usize,
    chunk: usize,
}

impl Chunked {
    fn new(data: Vec<u8>, chunk: usize) -> Self {
        assert!(chunk > 0);
        Self { data, at: 0, chunk }
    }
}

impl std::io::Read for Chunked {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        let available = std::io::BufRead::fill_buf(self)?;
        let n = available.len().min(out.len());
        out[..n].copy_from_slice(&available[..n]);
        std::io::BufRead::consume(self, n);
        Ok(n)
    }
}

impl std::io::BufRead for Chunked {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        let end = (self.at + self.chunk).min(self.data.len());
        Ok(&self.data[self.at..end])
    }

    fn consume(&mut self, amount: usize) {
        self.at = (self.at + amount).min(self.data.len());
    }
}

const CHUNK_SIZES: [usize; 4] = [1, 2, 3, 7];

#[test]
fn framing_cases_when_every_read_is_a_few_bytes() {
    for chunk in CHUNK_SIZES {
        for (name, input, expected) in cases() {
            let mut out = Vec::new();
            let mut transport = LineTransport::new(Chunked::new(input, chunk), &mut out);
            assert_eq!(
                describe(&drain(&mut transport)),
                describe(&expected),
                "case: {name}, at most {chunk} byte(s) per read"
            );
        }
    }
}

#[test]
fn the_chunk_sizes_put_a_boundary_inside_every_character_width() {
    let line = mixed_width_line().into_bytes();
    for chunk in CHUNK_SIZES {
        // Where a reader that consumes each chunk whole sees its read boundaries.
        let boundaries: Vec<usize> = (1..)
            .map(|k| k * chunk)
            .take_while(|at| *at < line.len())
            .collect();
        for ch in ['é', '€', '𝄞'] {
            let mut encoded = [0u8; 4];
            let width = ch.encode_utf8(&mut encoded).len();
            let split = line
                .windows(width)
                .enumerate()
                .filter(|(_, window)| *window == &encoded[..width])
                .any(|(start, _)| {
                    boundaries
                        .iter()
                        .any(|at| *at > start && *at < start + width)
                });
            // A one-byte chunk splits every multi-byte character; larger ones need an alignment
            // that puts the boundary inside, which the varying prefixes provide.
            assert!(split, "chunk {chunk}: no boundary inside {ch:?}");
        }
    }
}

#[test]
fn the_limit_is_one_mebibyte_of_line_content_whatever_the_constant_says() {
    const LIMIT: usize = 1 << 20;
    let at_limit = "x".repeat(LIMIT);
    let over = "x".repeat(LIMIT + 1);
    let wide_at_limit = "𝄞".repeat(LIMIT / 4);
    assert_eq!(wide_at_limit.len(), LIMIT);
    let cases: Vec<(&str, Vec<u8>, Vec<Got>)> = vec![
        (
            "exactly 1 << 20 bytes, newline not counted, is accepted",
            format!("{at_limit}\nok\n").into_bytes(),
            vec![message(at_limit.clone()), message("ok")],
        ),
        (
            "1 << 20 bytes ending the input is accepted",
            at_limit.clone().into_bytes(),
            vec![message(at_limit.clone())],
        ),
        (
            "1 << 20 bytes of 4-byte characters is accepted",
            format!("{wide_at_limit}\nok\n").into_bytes(),
            vec![message(wide_at_limit.clone()), message("ok")],
        ),
        (
            "(1 << 20) + 1 bytes is rejected and the stream resyncs",
            format!("{over}\nok\n").into_bytes(),
            vec![Got::Malformed, message("ok")],
        ),
        (
            "(1 << 20) + 1 bytes ending the input is rejected",
            over.clone().into_bytes(),
            vec![Got::Malformed],
        ),
    ];
    for chunk in [usize::MAX, 7] {
        for (name, input, expected) in &cases {
            let mut out = Vec::new();
            let got = if chunk == usize::MAX {
                let mut transport =
                    LineTransport::new(std::io::Cursor::new(input.clone()), &mut out);
                drain(&mut transport)
            } else {
                let mut transport =
                    LineTransport::new(Chunked::new(input.clone(), chunk), &mut out);
                drain(&mut transport)
            };
            assert_eq!(
                describe(&got),
                describe(expected),
                "case: {name}, read size {chunk}"
            );
        }
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
