use crate::mcp::{
    idle::IdlePolicy,
    idle_runtime::{Event, IdleRuntime},
    lazy::Readiness,
    transport::SessionRuntime,
    CacheMode, ServerConfig,
};
use std::sync::{Arc, Mutex};
use std::time::Duration;
pub(super) type Lines = Arc<Mutex<Vec<String>>>;
pub(super) use super::parity_tests::capture;
pub(super) fn fixture() -> (tempfile::TempDir, ServerConfig) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("a.py"),
        "def helper():\n    return 1\n\ndef run():\n    return helper()\n",
    )
    .unwrap();
    let mut cfg = ServerConfig::new(dir.path().join("."));
    cfg.cache = CacheMode::Dir(dir.path().join("cache"));
    (dir, cfg)
}
pub(super) fn runtime(cfg: &ServerConfig) -> (IdleRuntime, Lines) {
    capture(|| IdleRuntime::new(cfg, IdlePolicy::enabled(Duration::from_secs(10)), None).unwrap())
}
pub(super) fn ready(rt: &mut IdleRuntime) {
    assert!(matches!(rt.ensure_ready(), Readiness::Ready));
    rt.join_build_for_tests();
}
pub(super) fn unload(rt: &mut IdleRuntime) {
    rt.tick(rt.idle_deadline().unwrap());
    loop {
        let e = rt.rx.recv_timeout(Duration::from_secs(10)).unwrap();
        let done = matches!(e, Event::Disposed);
        rt.accept(e);
        if done {
            break;
        }
    }
}
pub(super) fn line(lines: &Lines, event: &str) -> String {
    lines
        .lock()
        .unwrap()
        .iter()
        .rev()
        .find(|s| s.contains(&format!("event={event} ")))
        .unwrap_or_else(|| panic!("missing event={event}"))
        .clone()
}
pub(crate) fn value<'a>(s: &'a str, key: &str) -> &'a str {
    s.split_whitespace()
        .find_map(|t| t.strip_prefix(&format!("{key}=")))
        .unwrap_or_else(|| panic!("missing {key}: {s}"))
}
pub(crate) fn number(s: &str, key: &str) -> u128 {
    value(s, key).parse().unwrap()
}
pub(crate) fn sidecar_scope(s: &str) {
    if !matches!(value(s, "event"), "publication" | "unload") {
        assert_eq!(
            value(s, "sidecar"),
            "not_applicable",
            "unrelated event: {s}"
        );
    }
}
pub(crate) fn publication_facts(s: &str, expected: &str) {
    assert_eq!(value(s, "cpg"), expected, "actual publication cache branch");
    let stages: u128 = ["load_parse_ns", "f_validation_ns", "index_ns"]
        .iter()
        .map(|k| {
            let n = number(s, k);
            assert!(n > 0, "missing actual publication stage {k}");
            n
        })
        .sum();
    assert!(number(s, "bootstrap_ns") >= stages);
}

pub(super) fn edges(rt: &IdleRuntime) {
    crate::navigation::queries::callees(rt.session(), Some("run"), None, None, 1).unwrap();
}
#[test]
fn o_actual_cache_and_stages() {
    let (_dir, mut cfg) = fixture();
    for expected in ["miss", "hit", "partial_hit", "disabled"] {
        if expected == "partial_hit" {
            std::fs::write(
                cfg.repo_root.join("a.py"),
                "def run():\n    return 123456\n",
            )
            .unwrap();
        }
        if expected == "disabled" {
            cfg.cache = CacheMode::NoCache;
        }
        let (mut rt, lines) = runtime(&cfg);
        ready(&mut rt);
        let s = line(&lines, "publication");
        assert_eq!(
            value(&line(&lines, "startup"), "identity"),
            value(&s, "identity")
        );
        assert_eq!(
            value(&s, "cpg"),
            expected,
            "actual cache branch, never speed"
        );
        let stages: u128 = ["load_parse_ns", "f_validation_ns", "index_ns"]
            .iter()
            .map(|k| {
                let n = number(&s, k);
                assert!(n > 0, "missing actual stage {k}");
                n
            })
            .sum();
        assert!(number(&s, "bootstrap_ns") >= stages);
        assert_eq!(value(&s, "sidecar"), "not_attempted");
        assert_eq!(rt.session().index.call_edges_loaded_from_cache(), None);
        unload(&mut rt);
    }
    let (mut lazy, lines) = capture(|| crate::mcp::lazy::LazySessionProvider::new(&cfg).unwrap());
    assert!(matches!(lazy.ensure_ready(), Readiness::Ready));
    lazy.join_build_for_tests();
    assert_eq!(value(&line(&lines, "publication"), "cpg"), "disabled");
    assert!(!lines
        .lock()
        .unwrap()
        .iter()
        .any(|s| s.contains("event=unload ")));
    let bad = Arc::new(|| anyhow::bail!("injected failed bootstrap"));
    let (mut rt, lines) = capture(|| {
        IdleRuntime::new(
            &cfg,
            IdlePolicy::enabled(Duration::from_secs(10)),
            Some(bad),
        )
        .unwrap()
    });
    assert!(matches!(rt.ensure_ready(), Readiness::Failed { .. }));
    rt.join_build_for_tests();
    assert_eq!(value(&line(&lines, "build"), "cpg"), "unknown");
}

#[test]
fn o_resident_publication_facts() {
    for startup in [
        crate::mcp::StartupMode::Lazy,
        crate::mcp::StartupMode::Background,
    ] {
        let (_dir, mut cfg) = fixture();
        cfg.startup = startup;
        for expected in ["miss", "hit"] {
            let (mut lazy, lines) =
                capture(|| crate::mcp::lazy::LazySessionProvider::new(&cfg).unwrap());
            assert!(matches!(lazy.ensure_ready(), Readiness::Ready));
            lazy.join_build_for_tests();
            publication_facts(&line(&lines, "publication"), expected);
            for s in lines.lock().unwrap().iter() {
                sidecar_scope(s);
            }
        }
        let bad = Arc::new(|| anyhow::bail!("injected resident failure"));
        let (mut lazy, lines) = capture(|| {
            crate::mcp::lazy::LazySessionProvider::with_builder(&cfg, Duration::from_secs(10), bad)
                .unwrap()
        });
        assert!(matches!(lazy.ensure_ready(), Readiness::Failed { .. }));
        lazy.join_build_for_tests();
        assert_eq!(
            value(&line(&lines, "build_finish"), "reason"),
            "demand_or_startup"
        );
        assert_eq!(value(&line(&lines, "build"), "reason"), "build_failed");
        assert_eq!(value(&line(&lines, "build"), "cpg"), "unknown");
    }
}
