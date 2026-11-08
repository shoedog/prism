//! Enabled-I0 stdio only. Socket framing and legacy/Eager/owner loops are unchanged.
use super::framing::{LineTransport, ReadOutcome, Transport};
use super::idle::IdlePolicy;
use super::idle_runtime::{Event, IdleRuntime, Input};
use super::registry::ToolRegistry;
use super::settings::ConnectionSettings;
use super::transport::{self, Control, Lifecycle};
use super::ServerConfig;
use serde_json::Value;
use std::io::{BufRead, Write};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::time::Instant;

type TimerState = Arc<(Mutex<(bool, Option<Instant>)>, Condvar)>;

/// One outstanding timer notification. Re-arming never queues a stream of timer ticks.
struct Timer {
    state: TimerState,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl Timer {
    fn new(tx: mpsc::Sender<Event>) -> Self {
        let state = Arc::new((Mutex::new((false, None::<Instant>)), Condvar::new()));
        let shared = state.clone();
        let worker = std::thread::spawn(move || {
            let (lock, wake) = &*shared;
            let mut state = lock.lock().unwrap();
            loop {
                if state.0 {
                    return;
                }
                match state.1 {
                    None => state = wake.wait(state).unwrap(),
                    Some(deadline) => {
                        let now = Instant::now();
                        if now >= deadline {
                            state.1 = None;
                            if tx.send(Event::Tick(now)).is_err() {
                                return;
                            }
                        } else {
                            state = wake.wait_timeout(state, deadline - now).unwrap().0;
                        }
                    }
                }
            }
        });
        Self {
            state,
            worker: Some(worker),
        }
    }
    fn arm(&self, deadline: Option<Instant>) {
        let (lock, wake) = &*self.state;
        lock.lock().unwrap().1 = deadline;
        wake.notify_one();
    }
}
impl Drop for Timer {
    fn drop(&mut self) {
        let (lock, wake) = &*self.state;
        lock.lock().unwrap().0 = true;
        wake.notify_one();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn reader(reader: impl BufRead, tx: mpsc::Sender<Event>) {
    let mut transport = LineTransport::new(reader, std::io::sink());
    loop {
        let outcome = transport.read_message();
        let terminal = !matches!(&outcome, Ok(Some(_)));
        let (next, acknowledged) = mpsc::channel();
        if tx
            .send(Event::Input(Input {
                outcome,
                next: Some(next),
            }))
            .is_err()
        {
            return;
        }
        if terminal || acknowledged.recv().is_err() {
            return;
        }
    }
}

pub(super) fn serve(
    cfg: &ServerConfig,
    policy: IdlePolicy,
    registry: &ToolRegistry,
) -> anyhow::Result<()> {
    let rt = IdleRuntime::new(cfg, policy, None)?;
    let tx = rt.tx.clone();
    // Stdin can be held open with an incomplete line. The reader owns that blocking read,
    // never the provider. On fatal output failure this thread is detached until process exit.
    std::thread::spawn(move || reader(std::io::stdin().lock(), tx));
    serve_events(
        rt,
        std::io::stdout().lock(),
        registry,
        &ConnectionSettings::from_env(),
    )
}

pub(super) fn serve_events(
    mut rt: IdleRuntime,
    writer: impl Write,
    registry: &ToolRegistry,
    settings: &ConnectionSettings,
) -> anyhow::Result<()> {
    let timer = Timer::new(rt.tx.clone());
    let mut output = LineTransport::new(std::io::empty(), writer);
    let mut lifecycle = Lifecycle::PreInit;
    loop {
        if let Some(input) = rt.pending.pop_front() {
            if let Some(next) = input.next {
                let _ = next.send(());
            }
            let Some(outcome) = input.outcome? else {
                return Ok(());
            };
            let message = match outcome {
                ReadOutcome::Message(line) => serde_json::from_str::<Value>(&line).ok(),
                ReadOutcome::Malformed => None,
            };
            let response = if let Some(message) = message {
                match transport::handle_control(
                    &message,
                    &mut lifecycle,
                    registry,
                    rt.startup_mode(),
                    true,
                    settings,
                ) {
                    Control::Response(value) => Some(value),
                    Control::NoResponse => None,
                    Control::ToolCall(call) => {
                        let lease = rt.activity();
                        let response = transport::dispatch_tool(&call, &mut rt, settings);
                        drop(lease); // release after the owned response and before stdout
                        Some(response)
                    }
                }
            } else {
                Some(transport::error_response(
                    Value::Null,
                    -32700,
                    "Parse error",
                ))
            };
            if let Some(response) = response {
                output.write_message(response)?;
            }
            rt.flush_events();
        } else {
            timer.arm(rt.idle_deadline());
            // Runtime retains a sender for build retries, so disconnection isn't the EOF signal.
            let event = rt.rx.recv()?;
            rt.accept(event);
        }
    }
}
use super::transport::SessionRuntime;

#[cfg(test)]
#[path = "idle_stdio_tests.rs"]
mod tests;

/// Runs real framing/dispatch with a held lifecycle sink; joins before reporting artifacts.
#[cfg(test)]
pub(super) fn parity_witness(cfg: ServerConfig, write_error: bool) -> (Vec<u8>, bool, bool) {
    use super::lazy::SessionBuilder;
    use super::SessionProvider;
    use std::time::Duration;
    struct Bytes(Arc<Mutex<Vec<u8>>>);
    impl Write for Bytes {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    struct Broken;
    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("injected lifecycle write failure"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let bytes = Arc::new(Mutex::new(Vec::new()));
    let out = bytes.clone();
    let (entered, reached) = mpsc::channel();
    let (release, resume) = mpsc::channel();
    let (weak_tx, weak_rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let build_cfg = cfg.clone();
        let builder: SessionBuilder = Arc::new(move || {
            let mut p = SessionProvider::bootstrap(&build_cfg)?;
            p.force_next_refresh_error_for_tests("EOF scope witness");
            let _ = weak_tx.send(Arc::downgrade(&p.session().repo));
            Ok(p)
        });
        let mut rt = IdleRuntime::new(
            &cfg,
            IdlePolicy::enabled(Duration::from_secs(10)),
            Some(builder),
        )
        .unwrap();
        if write_error {
            rt.log_context()
                .emit_lines_to(&["injected".into()], &mut Broken);
        }
        let resume = Mutex::new(resume);
        let check = out.clone();
        rt.log_context().observe = Some(Arc::new(move |s| {
            if s.contains("event=refresh ") {
                let complete = String::from_utf8_lossy(&check.lock().unwrap()).contains("\"id\":8");
                let _ = entered.send(complete);
                let _ = resume.lock().unwrap().recv_timeout(Duration::from_secs(10));
            }
        }));
        for raw in [
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"o","version":"1"}}}"#,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"a.py","line":1}}}"#,
            r#"{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"refresh_index","arguments":{}}}"#,
        ] {
            rt.tx
                .send(Event::Input(Input {
                    outcome: Ok(Some(ReadOutcome::Message(raw.into()))),
                    next: None,
                }))
                .unwrap();
        }
        rt.tx
            .send(Event::Input(Input {
                outcome: Ok(None),
                next: None,
            }))
            .unwrap();
        serve_events(
            rt,
            Bytes(out),
            &ToolRegistry::all_v1(),
            &ConnectionSettings::from_env(),
        )
        .unwrap();
    });
    let complete = reached
        .recv_timeout(Duration::from_secs(10))
        .unwrap_or(false);
    let _ = release.send(());
    worker.join().unwrap();
    let dead = weak_rx
        .recv_timeout(Duration::from_secs(10))
        .unwrap()
        .upgrade()
        .is_none();
    let result = bytes.lock().unwrap().clone();
    (result, complete, dead)
}
