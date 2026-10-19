use super::*;
use crate::mcp::lazy::SessionBuilder;
use crate::mcp::{CacheMode, SessionProvider, StartupMode};
use serde_json::json;
use std::io::{self, BufReader, Read};
use std::sync::Weak;
use std::time::Duration;

struct Chunks {
    rx: mpsc::Receiver<Vec<u8>>,
    current: std::io::Cursor<Vec<u8>>,
}
impl Read for Chunks {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        loop {
            let n = self.current.read(bytes)?;
            if n > 0 {
                return Ok(n);
            }
            match self.rx.recv() {
                Ok(chunk) => self.current = std::io::Cursor::new(chunk),
                Err(_) => return Ok(0),
            }
        }
    }
}
struct Responses {
    tx: mpsc::Sender<Vec<u8>>,
    bytes: Vec<u8>,
}
impl Write for Responses {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes.extend_from_slice(bytes);
        while let Some(end) = self.bytes.iter().position(|b| *b == b'\n') {
            let line: Vec<_> = self.bytes.drain(..=end).collect();
            self.tx.send(line).map_err(io::Error::other)?;
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
struct Client {
    input: Option<mpsc::Sender<Vec<u8>>>,
    output: mpsc::Receiver<Vec<u8>>,
    done: mpsc::Receiver<anyhow::Result<()>>,
}
impl Client {
    fn send(&self, v: Value) {
        let mut bytes = serde_json::to_vec(&v).unwrap();
        bytes.push(b'\n');
        self.input.as_ref().unwrap().send(bytes).unwrap();
    }
    fn response(&self) -> Value {
        serde_json::from_slice(&self.raw_response()).unwrap()
    }
    fn raw_response(&self) -> Vec<u8> {
        self.output.recv_timeout(Duration::from_secs(10)).unwrap()
    }
    fn init(&self) {
        self.send(json!({"jsonrpc":"2.0", "id":1, "method":"initialize", "params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"idle-test","version":"1"}}}));
        assert_eq!(self.response()["id"], 1);
        self.send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
    }
    fn query(&self, id: u32) {
        self.send(json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"a.py","line":1}}}));
    }
    fn finish(mut self) {
        self.input.take();
        self.done
            .recv_timeout(Duration::from_secs(10))
            .unwrap()
            .unwrap();
    }
}
fn start(cfg: ServerConfig, timeout: Duration, builder: Option<SessionBuilder>) -> Client {
    let (input, rx) = mpsc::channel();
    let (output, output_rx) = mpsc::channel();
    let (done, done_rx) = mpsc::channel();
    std::thread::spawn(move || {
        let rt = IdleRuntime::new(&cfg, IdlePolicy::enabled(timeout), builder).unwrap();
        let tx = rt.tx.clone();
        let read = std::thread::spawn(move || {
            reader(
                BufReader::new(Chunks {
                    rx,
                    current: std::io::Cursor::new(Vec::new()),
                }),
                tx,
            )
        });
        let result = serve_events(
            rt,
            Responses {
                tx: output,
                bytes: Vec::new(),
            },
            &ToolRegistry::all_v1(),
            &ConnectionSettings::from_env(),
        );
        read.join().unwrap();
        done.send(result).unwrap();
    });
    Client {
        input: Some(input),
        output: output_rx,
        done: done_rx,
    }
}
fn fixture() -> (tempfile::TempDir, ServerConfig) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), "def f():\n    return 1\n").unwrap();
    let mut cfg = ServerConfig::new(dir.path().to_owned());
    cfg.cache = CacheMode::NoCache;
    (dir, cfg)
}
fn wait_dropped_with_reason<T>(weak: &Weak<T>, reason: &str) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while weak.strong_count() != 0 {
        assert!(Instant::now() < deadline, "{reason}");
        std::thread::yield_now();
    }
}

fn wait_dropped<T>(weak: &Weak<T>) {
    wait_dropped_with_reason(
        weak,
        "silent open stdin must unload without another message",
    );
}

#[test]
fn idle_stdio_silent_partial_input_unloads() {
    for partial in [false, true] {
        let (_dir, mut cfg) = fixture();
        cfg.startup = StartupMode::Background;
        let builder_cfg = cfg.clone();
        let (built, rx) = mpsc::channel();
        let builder: SessionBuilder = Arc::new(move || {
            let p = SessionProvider::bootstrap(&builder_cfg)?;
            built
                .send((
                    Arc::downgrade(&p.session().repo),
                    Arc::downgrade(&p.session().index),
                ))
                .unwrap();
            Ok(p)
        });
        let client = start(cfg, Duration::from_millis(30), Some(builder));
        if partial {
            client
                .input
                .as_ref()
                .unwrap()
                .send(b"{\"jsonrpc\":".to_vec())
                .unwrap();
        }
        let (repo, index) = rx.recv_timeout(Duration::from_secs(10)).unwrap();
        wait_dropped(&repo);
        wait_dropped(&index);
        assert!(
            client.output.try_recv().is_err(),
            "no unsolicited tool stdout"
        );
        client.finish();
    }
}

#[test]
fn idle_stdio_readiness_preserves_input_eof_order() {
    let (_dir, cfg) = fixture();
    let builder_cfg = cfg.clone();
    let (entered, entered_rx) = mpsc::channel();
    let (release, rx) = mpsc::channel();
    let rx = Mutex::new(rx);
    let builder: SessionBuilder = Arc::new(move || {
        entered.send(()).unwrap();
        rx.lock().unwrap().recv().unwrap();
        SessionProvider::bootstrap(&builder_cfg)
    });
    let mut client = start(cfg, Duration::from_secs(10), Some(builder));
    client.init();
    client.query(2);
    entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    client.send(json!({"jsonrpc":"2.0","id":3,"method":"ping"}));
    client.input.take(); // EOF stays behind the ping, while readiness waits for completion
    release.send(()).unwrap();
    assert_eq!(client.response()["id"], 2);
    assert_eq!(client.response()["id"], 3);
    client.finish();
}

#[test]
fn idle_stdio_warming_retry_success() {
    let (_dir, mut cfg) = fixture();
    cfg.first_call_wait = Duration::ZERO;
    let builder_cfg = cfg.clone();
    let (built, built_rx) = mpsc::channel();
    let (release, rx) = mpsc::channel();
    let rx = Mutex::new(rx);
    let builder: SessionBuilder = Arc::new(move || {
        rx.lock().unwrap().recv().unwrap();
        let p = SessionProvider::bootstrap(&builder_cfg)?;
        built.send(()).unwrap();
        Ok(p)
    });
    let client = start(cfg, Duration::from_secs(10), Some(builder));
    client.init();
    client.query(2);
    assert!(client.response().to_string().contains("warming"));
    client.query(3);
    assert!(client.response().to_string().contains("warming"));
    release.send(()).unwrap();
    built_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    // Ping is an ordered channel barrier, and doesn't itself collect or reset idle.
    client.send(json!({"jsonrpc":"2.0","id":4,"method":"ping"}));
    assert_eq!(client.response()["id"], 4);
    for id in 5..50 {
        client.query(id);
        let value = client.response();
        if !value.to_string().contains("warming") {
            assert!(!value.to_string().contains("build_failed"));
            assert_eq!(value["result"]["isError"], false);
            client.finish();
            return;
        }
    }
    panic!("must serve after the one build completes");
}

#[test]
fn idle_stdio_eof_disposes_late_build() {
    let (_dir, mut cfg) = fixture();
    cfg.startup = StartupMode::Background;
    let builder_cfg = cfg.clone();
    let (entered, entered_rx) = mpsc::channel();
    let (release, rx) = mpsc::channel();
    let rx = Mutex::new(rx);
    let (built, built_rx) = mpsc::channel();
    let builder: SessionBuilder = Arc::new(move || {
        entered.send(()).unwrap();
        rx.lock().unwrap().recv().unwrap();
        let p = SessionProvider::bootstrap(&builder_cfg)?;
        built.send(Arc::downgrade(&p.session().repo)).unwrap();
        Ok(p)
    });
    let client = start(cfg, Duration::from_millis(30), Some(builder));
    entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    client.finish();
    release.send(()).unwrap();
    let repo = built_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    wait_dropped_with_reason(
        &repo,
        "late build must be disposed after EOF retires the owner",
    );
}

#[test]
fn idle_stdio_unheard_failure_reports_then_retries() {
    let (_dir, mut cfg) = fixture();
    cfg.startup = StartupMode::Background;
    let builder_cfg = cfg.clone();
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let count = calls.clone();
    let (failed, failure_rx) = mpsc::channel();
    let builder: SessionBuilder = Arc::new(move || {
        if count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
            failed.send(()).unwrap();
            anyhow::bail!("unheard startup failure");
        }
        SessionProvider::bootstrap(&builder_cfg)
    });
    let client = start(cfg, Duration::from_secs(10), Some(builder));
    failure_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    client.init();
    client.query(2);
    assert!(client.response().to_string().contains("build_failed"));
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    client.query(3);
    let served = client.response();
    assert_eq!(served["result"]["isError"], false);
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 2);
    client.finish();
}

#[test]
fn idle_stdio_same_snapshot_bytes_across_unload() {
    let (_dir, cfg) = fixture();
    let builder_cfg = cfg.clone();
    let (built, rx) = mpsc::channel();
    let builder: SessionBuilder = Arc::new(move || {
        let p = SessionProvider::bootstrap(&builder_cfg)?;
        built.send(Arc::downgrade(&p.session().repo)).unwrap();
        Ok(p)
    });
    let client = start(cfg, Duration::from_millis(30), Some(builder));
    client.init();
    client.query(2);
    let first = client.raw_response();
    for _ in 0..3 {
        let repo = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        wait_dropped(&repo);
        client.query(2);
        assert_eq!(
            first,
            client.raw_response(),
            "full same-build MCP bytes, including metadata"
        );
    }
    let repo = rx.recv_timeout(Duration::from_secs(5)).unwrap();
    wait_dropped(&repo);
    client.send(json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"refresh_index","arguments":{}}}));
    let refresh = client.response();
    assert_eq!(refresh["result"]["isError"], false);
    assert_eq!(refresh["result"]["_meta"]["prism/refresh_strategy"], "full");
    assert_eq!(refresh["result"]["_meta"]["prism/refresh_generation"], 1);
    client.finish();
}

#[test]
fn idle_stdio_on_off_transcript_parity() {
    let (_dir, cfg) = fixture();
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"idle-test","version":"1"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"a.py","line":1}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"refresh_index","arguments":{}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"a.py","line":1}}}),
    ];
    let script: String = messages
        .iter()
        .map(|message| format!("{message}\n"))
        .collect();
    let mut legacy = super::super::lazy::LazySessionProvider::new(&cfg).unwrap();
    let mut legacy_bytes = Vec::new();
    let mut transport = LineTransport::new(BufReader::new(script.as_bytes()), &mut legacy_bytes);
    transport::serve_connection(
        &mut legacy,
        &ToolRegistry::all_v1(),
        &mut transport,
        &ConnectionSettings::from_env(),
    )
    .unwrap();
    drop(transport);
    let client = start(cfg, Duration::from_secs(10), None);
    for message in messages {
        client.send(message);
    }
    let enabled_bytes: Vec<_> = (0..4).flat_map(|_| client.raw_response()).collect();
    assert_eq!(
        enabled_bytes, legacy_bytes,
        "literal framed transcript bytes"
    );
    client.finish();
}

#[test]
fn idle_stdio_warming_then_silence_releases_build() {
    let (_dir, mut cfg) = fixture();
    cfg.first_call_wait = Duration::ZERO;
    let builder_cfg = cfg.clone();
    let (release, rx) = mpsc::channel();
    let rx = Mutex::new(rx);
    let (built, observed) = mpsc::channel();
    let builder: SessionBuilder = Arc::new(move || {
        rx.lock().unwrap().recv().unwrap();
        let p = SessionProvider::bootstrap(&builder_cfg)?;
        built.send(Arc::downgrade(&p.session().repo)).unwrap();
        Ok(p)
    });
    let client = start(cfg, Duration::from_millis(30), Some(builder));
    client.init();
    client.query(2);
    assert!(client.response().to_string().contains("warming"));
    release.send(()).unwrap();
    let repo = observed.recv_timeout(Duration::from_secs(10)).unwrap();
    wait_dropped(&repo);
    assert!(client.output.try_recv().is_err());
    client.finish();
}

#[test]
fn idle_stdio_control_only_stays_unbuilt() {
    let (_dir, cfg) = fixture();
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let count = calls.clone();
    let builder: SessionBuilder = Arc::new(move || {
        count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        anyhow::bail!("control messages must not build");
    });
    let client = start(cfg, Duration::from_millis(1), Some(builder));
    client.init();
    for (id, method) in [(2, "ping"), (3, "tools/list")] {
        client.send(json!({"jsonrpc":"2.0", "id":id, "method":method}));
        assert_eq!(client.response()["id"], id);
    }
    client.finish();
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
}

#[test]
fn idle_stdio_background_instructions_explain_demand_reload() {
    let (_dir, mut cfg) = fixture();
    cfg.startup = StartupMode::Background;
    let client = start(cfg, Duration::from_secs(10), None);
    client.send(json!({"jsonrpc":"2.0", "id":1, "method":"initialize", "params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"idle-test","version":"1"}}}));
    let response = client.response();
    assert!(response["result"]["instructions"]
        .as_str()
        .unwrap()
        .contains("first tool call after an idle unload"));
    client.finish();
}
