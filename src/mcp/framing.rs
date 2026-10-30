//! Newline-delimited JSON framing shared by every byte transport (stdio today, a Unix socket for
//! the shared daemon). One implementation, generic over the reader and writer, so a socket
//! connection frames messages exactly as stdio does.
use serde_json::Value;
#[cfg(test)]
use std::collections::VecDeque;
use std::io::{BufRead, Write};
#[cfg(unix)]
use std::os::unix::net::UnixStream;

/// Max inbound JSON-RPC message size (one line). A nav request is tiny; this caps a hostile/buggy
/// local client's memory (holistic-review MAJOR — `read_line` was unbounded).
pub(super) const MAX_REQUEST_BYTES: usize = 1 << 20; // 1 MiB

/// Outcome of reading one inbound frame. `Malformed` is a *recoverable* framing/encoding fault
/// (bad UTF-8, or an oversized line we drained to resync) — the caller replies `-32700` and keeps the
/// session (and its warm index) alive, instead of letting one stray byte from a buggy/hostile local
/// client kill the server (codex MAJOR — these used to propagate as fatal errors).
pub enum ReadOutcome {
    Message(String),
    Malformed,
}

pub trait Transport {
    fn read_message(&mut self) -> anyhow::Result<Option<ReadOutcome>>;
    fn write_message(&mut self, v: Value) -> anyhow::Result<()>;
}

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

/// Strip a trailing `\r`, then UTF-8 validate the whole accumulated line. Returns `None` on a UTF-8
/// failure so the caller can map it to a recoverable `ReadOutcome::Malformed` instead of a fatal
/// error (codex MAJOR — one stray non-UTF-8 byte must not kill the server).
fn finish_line(mut bytes: Vec<u8>) -> Option<String> {
    if bytes.last() == Some(&b'\r') {
        bytes.pop();
    }
    String::from_utf8(bytes).ok()
}

impl<R: BufRead, W: Write> LineTransport<R, W> {
    /// Consume bytes from the reader up to and including the next `\n` (or until EOF) so the stream
    /// resyncs to a line boundary after an oversized frame. Bounded only by where the next newline
    /// is; the oversized payload is dropped, not buffered.
    fn drain_to_newline(&mut self) -> anyhow::Result<()> {
        loop {
            let available = self.reader.fill_buf()?;
            if available.is_empty() {
                return Ok(()); // EOF
            }
            match available.iter().position(|&b| b == b'\n') {
                Some(pos) => {
                    self.reader.consume(pos + 1);
                    return Ok(());
                }
                None => {
                    let len = available.len();
                    self.reader.consume(len);
                }
            }
        }
    }
}

impl<R: BufRead, W: Write> Transport for LineTransport<R, W> {
    /// Read one newline-delimited message, **bounded to `MAX_REQUEST_BYTES`**. A genuine IO error
    /// (from `fill_buf`) still propagates (fatal); clean EOF returns `Ok(None)`. A bad-UTF-8 line or
    /// an oversized line is a *recoverable* fault: it returns `ReadOutcome::Malformed` (after draining
    /// the rest of the oversized line so the stream resyncs) instead of killing the server (codex
    /// MAJOR). Accumulates bytes and UTF-8-validates once at the end (correct across multi-byte
    /// boundaries).
    fn read_message(&mut self) -> anyhow::Result<Option<ReadOutcome>> {
        let mut bytes: Vec<u8> = Vec::new();
        loop {
            let available = self.reader.fill_buf()?;
            if available.is_empty() {
                return if bytes.is_empty() {
                    Ok(None) // EOF, clean
                } else {
                    // final line without trailing newline
                    Ok(Some(match finish_line(bytes) {
                        Some(line) => ReadOutcome::Message(line),
                        None => ReadOutcome::Malformed,
                    }))
                };
            }
            match available.iter().position(|&b| b == b'\n') {
                Some(pos) => {
                    if bytes.len() + pos > MAX_REQUEST_BYTES {
                        self.drain_to_newline()?;
                        return Ok(Some(ReadOutcome::Malformed));
                    }
                    bytes.extend_from_slice(&available[..pos]);
                    self.reader.consume(pos + 1);
                    return Ok(Some(match finish_line(bytes) {
                        Some(line) => ReadOutcome::Message(line),
                        None => ReadOutcome::Malformed,
                    }));
                }
                None => {
                    if bytes.len() + available.len() > MAX_REQUEST_BYTES {
                        let len = available.len();
                        self.reader.consume(len);
                        self.drain_to_newline()?;
                        return Ok(Some(ReadOutcome::Malformed));
                    }
                    let len = available.len();
                    bytes.extend_from_slice(available);
                    self.reader.consume(len);
                }
            }
        }
    }

    fn write_message(&mut self, v: Value) -> anyhow::Result<()> {
        serde_json::to_writer(&mut self.writer, &v)?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()?;
        Ok(())
    }
}

/// The framing over one connected Unix socket. Reads and writes go through independent handles
/// to the same stream; the writer is buffered and `write_message` flushes once per message, so a
/// response is one buffered line followed by a flush rather than one write per JSON token. A
/// response larger than the `BufWriter` buffer still reaches the peer in several writes.
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

#[cfg(test)]
pub struct InMemoryTransport {
    inputs: VecDeque<String>,
    responses: Vec<Value>,
}

#[cfg(test)]
impl InMemoryTransport {
    pub fn new(inputs: Vec<&str>) -> Self {
        Self {
            inputs: inputs.into_iter().map(str::to_owned).collect(),
            responses: Vec::new(),
        }
    }

    pub fn responses(&self) -> &[Value] {
        &self.responses
    }
}

#[cfg(test)]
impl Transport for InMemoryTransport {
    fn read_message(&mut self) -> anyhow::Result<Option<ReadOutcome>> {
        Ok(self.inputs.pop_front().map(ReadOutcome::Message))
    }

    fn write_message(&mut self, v: Value) -> anyhow::Result<()> {
        self.responses.push(v);
        Ok(())
    }
}

#[cfg(test)]
#[path = "framing_tests.rs"]
mod tests;
