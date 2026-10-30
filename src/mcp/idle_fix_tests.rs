//! Round-1 regression witnesses; no parent-source swaps or wall-clock idle sleeps.
use super::*;
use crate::mcp::framing::ReadOutcome;
use crate::mcp::registry::{ToolAnnotations, ToolDescriptor, ToolRegistry};
use crate::mcp::settings::ConnectionSettings;
use serde_json::json;
use std::io::Write;
use std::sync::Mutex;

fn fixture() -> (tempfile::TempDir, ServerConfig) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), "def f():\n    return 1\n").unwrap();
    let mut cfg = ServerConfig::new(dir.path().to_owned());
    cfg.cache = crate::mcp::CacheMode::Dir(dir.path().join("cache"));
    (dir, cfg)
}

#[test]
fn idle_warming_elapsed_includes_blocked_wait() {
    for wait in [Duration::ZERO, Duration::from_millis(40)] {
        let (_dir, mut cfg) = fixture();
        cfg.first_call_wait = wait;
        let (release, held) = mpsc::channel();
        let held = Mutex::new(held);
        let builder: SessionBuilder = Arc::new(move || {
            let _ = held.lock().unwrap().recv_timeout(Duration::from_secs(5));
            anyhow::bail!("released blocked builder");
        });
        let mut rt = IdleRuntime::new(
            &cfg,
            IdlePolicy::enabled(Duration::from_secs(10)),
            Some(builder),
        )
        .unwrap();
        let Readiness::Warming { elapsed } = rt.ensure_ready() else {
            panic!("held build must return warming");
        };
        release.send(()).unwrap();
        assert!(
            elapsed >= wait,
            "elapsed {elapsed:?} must include readiness wait {wait:?}"
        );
        rt.accept(rt.rx.recv_timeout(Duration::from_secs(5)).unwrap());
        assert!(matches!(rt.ensure_ready(), Readiness::Failed { .. }));
    }
}

#[test]
fn idle_disposal_timeout_gives_next_call_full_wait() {
    let (_dir, mut cfg) = fixture();
    cfg.first_call_wait = Duration::from_secs(5);
    let mut rt =
        IdleRuntime::new(&cfg, IdlePolicy::enabled(Duration::from_secs(10)), None).unwrap();
    assert!(matches!(rt.ensure_ready(), Readiness::Ready));
    cfg.first_call_wait = Duration::from_millis(40);
    rt.wait = cfg.first_call_wait;
    let (entered, observed) = mpsc::channel();
    let (release, held) = mpsc::channel();
    let held = Mutex::new(held);
    rt.before_dispose = Some(Arc::new(move || {
        entered.send(()).unwrap();
        let _ = held.lock().unwrap().recv_timeout(Duration::from_secs(5));
    }));
    rt.tick(rt.idle_deadline().unwrap());
    observed.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(matches!(rt.ensure_ready(), Readiness::Warming { elapsed } if elapsed.is_zero()));
    assert_eq!(rt.attempt, 1, "no build while disposal owns the session");
    release.send(()).unwrap();
    while rt.disposing {
        rt.accept(rt.rx.recv_timeout(Duration::from_secs(5)).unwrap());
    }
    let (release, held) = mpsc::channel();
    let held = Mutex::new(held);
    rt.builder = Arc::new(move || {
        let _ = held.lock().unwrap().recv_timeout(Duration::from_secs(5));
        anyhow::bail!("released next build");
    });
    let started = Instant::now();
    let readiness = rt.ensure_ready();
    let waited = started.elapsed();
    release.send(()).unwrap();
    assert!(matches!(readiness, Readiness::Warming { .. }));
    assert!(
        waited >= cfg.first_call_wait,
        "next reload gets full wait, got {waited:?}"
    );
    rt.accept(rt.rx.recv_timeout(Duration::from_secs(5)).unwrap());
}

struct ReleaseWriter {
    active: Rc<Cell<usize>>,
    released: Rc<Cell<Instant>>,
    clock: Arc<Mutex<Instant>>,
    writes: Rc<Cell<usize>>,
    bytes: Vec<u8>,
}

impl Write for ReleaseWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.bytes.extend_from_slice(bytes);
        if self.bytes.ends_with(b"\n") {
            let response: serde_json::Value = serde_json::from_slice(&self.bytes).unwrap();
            if response["id"] == 2 {
                assert_eq!(self.active.get(), 0, "release must precede stdout");
                assert_eq!(
                    self.released.get(),
                    *self.clock.lock().unwrap(),
                    "release clock must follow owned response construction"
                );
                self.writes.set(self.writes.get() + 1);
            }
            self.bytes.clear();
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn idle_stdio_production_lease_and_release_clock() {
    for outcome in ["ready", "warming", "failed"] {
        let (_dir, mut cfg) = fixture();
        let (release, held) = mpsc::channel();
        let held = Mutex::new(held);
        cfg.first_call_wait = if outcome == "warming" {
            Duration::ZERO
        } else {
            Duration::from_secs(5)
        };
        let builder_cfg = cfg.clone();
        let builder: SessionBuilder = Arc::new(move || match outcome {
            "ready" => SessionProvider::bootstrap(&builder_cfg),
            "failed" => anyhow::bail!("seam build failure"),
            _ => {
                let _ = held.lock().unwrap().recv_timeout(Duration::from_secs(5));
                anyhow::bail!("seam held build");
            }
        });
        let mut rt = IdleRuntime::new(
            &cfg,
            IdlePolicy::enabled(Duration::from_secs(10)),
            Some(builder),
        )
        .unwrap();
        let clock = Arc::new(Mutex::new(Instant::now() + Duration::from_secs(1000)));
        let test_clock = clock.clone();
        let active = rt.active.clone();
        let admission_observed = Rc::new(Cell::new(false));
        let observed = admission_observed.clone();
        rt.clock = Rc::new(move || {
            if !observed.replace(true) {
                assert_eq!(
                    active.get(),
                    1,
                    "production lease must start before readiness"
                );
            }
            *test_clock.lock().unwrap()
        });
        let mut registry = ToolRegistry::all_v1();
        let handler_clock = clock.clone();
        registry.register(ToolDescriptor {
            name: "lease_probe",
            description: "advance time during response construction".into(),
            input_schema: json!({"type":"object"}),
            annotations: ToolAnnotations::read_only("lease probe"),
            runtime_behavior: None,
            handler: Box::new(move |_, _| {
                *handler_clock.lock().unwrap() += Duration::from_secs(100);
                crate::mcp::output::McpToolResult {
                    content_text: "owned response".into(),
                    structured: None,
                    is_error: false,
                    meta: Default::default(),
                }
            }),
        });
        for message in [
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"seam","version":"1"}}}),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"lease_probe","arguments":{}}}),
        ] {
            rt.pending.push_back(Input {
                outcome: Ok(Some(ReadOutcome::Message(message.to_string()))),
                next: None,
            });
        }
        rt.pending.push_back(Input {
            outcome: Ok(None),
            next: None,
        });
        let writes = Rc::new(Cell::new(0));
        let released = rt.last_release.clone();
        let writer = ReleaseWriter {
            active: rt.active.clone(),
            released: released.clone(),
            clock: clock.clone(),
            writes: writes.clone(),
            bytes: Vec::new(),
        };
        crate::mcp::idle_stdio::serve_events(
            rt,
            writer,
            &registry,
            &ConnectionSettings::from_env(),
        )
        .unwrap();
        let _ = release.send(());
        assert!(admission_observed.get());
        assert_eq!(
            writes.get(),
            1,
            "owned success/warming/failure response observed"
        );
        assert_eq!(released.get(), *clock.lock().unwrap());
    }
}

#[test]
fn idle_pressure_relief_unused_note() {
    let off = IdlePolicy::default();
    let on = IdlePolicy::enabled(Duration::from_secs(600));
    assert!(off.pressure_relief_note(false, false).is_none());
    assert!(off.pressure_relief_note(true, false).is_some());
    assert_eq!(
        on.pressure_relief_note(true, true).is_none(),
        cfg!(target_os = "macos")
    );
    assert_eq!(
        on.pressure_relief_note(true, false).is_none(),
        cfg!(target_os = "macos")
    );
}

#[test]
fn idle_lifecycle_log_names_build_and_configured_timeout() {
    let (_dir, cfg) = fixture();
    let mut rt =
        IdleRuntime::new(&cfg, IdlePolicy::enabled(Duration::from_secs(90)), None).unwrap();
    assert_eq!(rt.completion_event(), "build");
    let line = rt
        .log
        .line(rt.completion_event(), "ready", Duration::ZERO, None, 0);
    assert!(line.contains("event=build "));
    assert!(line.contains("configured_idle_secs=90.000 "));
    assert!(!line.contains("idle_minutes="));
    rt.previous = Some(Published {
        fingerprint: [0; 32],
        generation: 3,
        diverged: false,
    });
    assert_eq!(rt.completion_event(), "reload");
}

#[test]
fn idle_disposal_and_build_share_current_call_budget() {
    let (_dir, cfg) = fixture();
    let mut rt =
        IdleRuntime::new(&cfg, IdlePolicy::enabled(Duration::from_secs(10)), None).unwrap();
    assert!(matches!(rt.ensure_ready(), Readiness::Ready));
    rt.tick(rt.idle_deadline().unwrap());
    // Keep the actual Disposed event queued for ensure_ready, so this call starts
    // in disposal and then crosses into Building within its original budget.
    let disposed = rt.rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(matches!(disposed, Event::Disposed));
    rt.tx.send(disposed).unwrap();
    rt.wait = Duration::from_millis(40);
    let first = Instant::now();
    let stepped = first + Duration::from_millis(30);
    let sampled = Cell::new(false);
    rt.clock = Rc::new(move || {
        if sampled.replace(true) {
            stepped
        } else {
            first
        }
    });
    let (release, held) = mpsc::channel();
    let held = Mutex::new(held);
    rt.builder = Arc::new(move || {
        let _ = held.lock().unwrap().recv_timeout(Duration::from_secs(5));
        anyhow::bail!("released cumulative-budget build");
    });
    let readiness = rt.ensure_ready();
    release.send(()).unwrap();
    assert!(matches!(readiness, Readiness::Warming { .. }));
    let Phase::Building { deadline, .. } = rt.phase else {
        panic!("held reload must be building");
    };
    assert_eq!(
        deadline,
        Some(first + rt.wait),
        "same-call disposal must not reset the build deadline"
    );
    rt.accept(rt.rx.recv_timeout(Duration::from_secs(5)).unwrap());
}
