use super::build_tests::*;
use crate::mcp::transport::SessionRuntime;
use crate::mcp::SessionProvider;
use std::sync::{Arc, Mutex};
use std::time::Duration;
#[test]
fn o_event_fields_preserve_serving() {
    let (_dir, cfg) = fixture();
    let p = SessionProvider::bootstrap(&cfg).unwrap();
    let (normal, written, dead) = crate::mcp::idle_stdio::parity_witness(cfg.clone(), false);
    assert!(
        written && dead,
        "sink follows response write; EOF releases provider: written={written} dead={dead}"
    );
    let (errors, written, dead) = crate::mcp::idle_stdio::parity_witness(cfg.clone(), true);
    assert!(written && dead);
    assert_eq!(
        normal, errors,
        "ignored lifecycle write error preserves MCP bytes"
    );
    let (mut rt, lines) = runtime(&cfg);
    ready(&mut rt);
    let query = |s: &crate::navigation::NavigationSession| {
        serde_json::to_vec(
            &crate::navigation::queries::callees(s, Some("run"), None, None, 1).unwrap(),
        )
        .unwrap()
    };
    assert_eq!(
        rt.session().index.call_edges_loaded_from_cache(),
        None,
        "observation never forces edges"
    );
    assert_eq!(
        query(p.session()),
        query(rt.session()),
        "identical served bytes"
    );
    let weak = Arc::downgrade(&rt.session().repo);
    let deadline = rt.idle_deadline().unwrap();
    rt.flush_events();
    rt.tick(deadline - std::time::Duration::from_nanos(1));
    assert_eq!(
        rt.idle_deadline(),
        Some(deadline),
        "observation cannot reset idle"
    );
    unload(&mut rt);
    assert!(weak.upgrade().is_none());
    assert_eq!(
        value(&line(&lines, "unload"), "sidecar"),
        if crate::cpg_cache::binary_input_dirty() {
            "refused_dirty"
        } else {
            "hit"
        }
    );
}

pub(super) type Observer = std::sync::Arc<dyn Fn(&str) + Send + Sync>;
thread_local! { static OBSERVER: std::cell::RefCell<Option<Observer>> = const { std::cell::RefCell::new(None) }; }
pub(super) fn observer() -> Option<Observer> {
    OBSERVER.with_borrow(Clone::clone)
}

pub(super) fn capture<T>(f: impl FnOnce() -> T) -> (T, Lines) {
    let lines = Arc::new(Mutex::new(Vec::new()));
    let out = lines.clone();
    OBSERVER
        .with_borrow_mut(|o| *o = Some(Arc::new(move |s| out.lock().unwrap().push(s.to_owned()))));
    let result = f();
    OBSERVER.with_borrow_mut(|o| *o = None);
    (result, lines)
}

pub(super) fn delayed_publication(cfg: &crate::mcp::ServerConfig) {
    let ((mut delayed, send), seen) =
        capture(|| crate::mcp::idle_runtime::IdleRuntime::deferred_for_tests(cfg));
    for success in [true, false, true] {
        delayed.ensure_ready();
        delayed.ensure_ready();
        send.send(success).unwrap();
        let built = delayed.rx.recv_timeout(Duration::from_secs(10)).unwrap();
        let before = super::Stamp::now();
        delayed.accept(built);
        delayed.join_build_for_tests();
        if success {
            assert!(number(&line(&seen, "publication"), "mono_ns") >= before.mono());
            delayed.refresh_index().unwrap();
            unload(&mut delayed);
            let release = line(&seen, "last_release");
            assert_eq!(value(&release, "attempt"), "1");
            assert_eq!(value(&release, "generation"), "0");
            drop(delayed.activity());
        } else {
            let failure = line(&seen, "reload");
            assert_eq!(value(&failure, "attempt"), "2");
            assert_eq!(value(&failure, "generation"), "0");
        }
    }
}
