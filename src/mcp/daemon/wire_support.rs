//! A raw MCP client on one end of a connected socket pair, and a daemon connection serving the
//! other end on its own thread. This is what stands in for a shim and an admitted connection
//! until the accept loop (S4) and the shim (S5a) exist.
use super::connection::serve_admitted;
use super::coordinator::DaemonState;
use super::test_support::{wait_until, HANG};
use crate::mcp::registry::ToolRegistry;
use crate::mcp::settings::ClientSettings;
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::thread::JoinHandle;

pub(super) const INIT: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#;
pub(super) const INITED: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
pub(super) const LIST: &str = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#;
pub(super) const PING: &str = r#"{"jsonrpc":"2.0","id":3,"method":"ping"}"#;
pub(super) const CALLEES: &str = r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"nav_callees","arguments":{"seed":{"kind":"symbol","name":"run","file":"main.py"}}}}"#;
pub(super) const NO_SUCH_TOOL: &str = r#"{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"no_such_tool","arguments":{}}}"#;

/// One client's end of a connection. Every read gives up after [`HANG`], so a server that
/// stops answering fails the test instead of hanging it.
pub(super) struct Client {
    writer: UnixStream,
    reader: BufReader<UnixStream>,
}

impl Client {
    pub(super) fn new(stream: UnixStream) -> Self {
        stream
            .set_read_timeout(Some(HANG))
            .expect("set the client's read timeout");
        let reader = BufReader::new(stream.try_clone().expect("clone the client socket"));
        Self {
            writer: stream,
            reader,
        }
    }

    /// Send one frame and do not wait for an answer.
    pub(super) fn send(&mut self, line: &str) {
        self.writer
            .write_all(line.as_bytes())
            .expect("write a frame");
        self.writer.write_all(b"\n").expect("end the frame");
    }

    /// The next response line, newline included.
    pub(super) fn recv_line(&mut self) -> String {
        let mut line = String::new();
        let read = self
            .reader
            .read_line(&mut line)
            .expect("a response within the hang limit");
        assert!(
            read > 0,
            "the server closed the connection instead of answering"
        );
        line
    }

    pub(super) fn recv(&mut self) -> Value {
        serde_json::from_str(&self.recv_line()).expect("a JSON response")
    }

    pub(super) fn request(&mut self, line: &str) -> Value {
        self.send(line);
        self.recv()
    }

    pub(super) fn handshake(&mut self) {
        let initialized = self.request(INIT);
        assert!(
            initialized["result"].is_object(),
            "initialize failed: {initialized}"
        );
        self.send(INITED);
    }
}

/// The thread serving the other end, and what its serve loop returned.
pub(super) type Worker = JoinHandle<anyhow::Result<()>>;

/// Open a connection to `state` as a client with these settings.
pub(super) fn connect(state: &Arc<DaemonState>, client: ClientSettings) -> (Client, Worker) {
    let (near, far) = UnixStream::pair().expect("a connected socket pair");
    let state = Arc::clone(state);
    let worker =
        std::thread::spawn(move || serve_admitted(&state, &ToolRegistry::all_v1(), far, &client));
    (Client::new(near), worker)
}

/// Wait for a worker to end and return what its serve loop returned. Panics if the worker
/// panicked or has not ended within [`HANG`].
pub(super) fn ended(worker: Worker) -> anyhow::Result<()> {
    wait_until("the connection's worker to end", || worker.is_finished());
    worker.join().expect("the worker must not panic")
}

/// Close the client's end and require the worker to end cleanly at end of input.
pub(super) fn close(client: Client, worker: Worker) {
    drop(client);
    ended(worker).expect("a connection ends cleanly at end of input");
}

pub(super) const REFRESH: &str = r#"{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"refresh_index","arguments":{}}}"#;
pub(super) const CALLERS: &str = r#"{"jsonrpc":"2.0","id":18,"method":"tools/call","params":{"name":"nav_callers","arguments":{"seed":{"kind":"symbol","name":"helper","file":"util.py"}}}}"#;

impl Client {
    /// Require that the server closes the connection without sending anything more.
    pub(super) fn expect_closed(&mut self) {
        let mut line = String::new();
        let read = self
            .reader
            .read_line(&mut line)
            .expect("end of stream within the hang limit");
        assert_eq!(read, 0, "expected the connection to be closed, got: {line}");
    }
}
