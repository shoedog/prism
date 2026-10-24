use super::build_tests::*;
use crate::mcp::transport::SessionRuntime;
use crate::mcp::SessionProvider;
use crate::navigation::queries::nodes_at;
#[test]
fn o_sidecar_attempt_only() {
    for query in [false, true] {
        for state in ["hit", "miss", "io_failure", "ineligible"] {
            let (_dir, cfg) = fixture();
            let p = SessionProvider::bootstrap(&cfg).unwrap();
            crate::navigation::queries::callees(p.session(), Some("run"), None, None, 1).unwrap();
            let cache = crate::navigation::cache::nav_cache_subdir(
                &cfg.repo_root.join("cache"),
                &p.session().repo,
            );
            drop(p);
            let path = cache.join("resolved-call-edge-index.bin");
            if state == "miss" {
                std::fs::remove_file(&path).unwrap();
            }
            if state == "io_failure" {
                std::fs::remove_file(&path).unwrap();
                std::fs::create_dir(&path).unwrap();
            }
            if state == "ineligible" {
                std::fs::remove_dir_all(&cache).unwrap();
            }
            let (mut rt, lines) = runtime(&cfg);
            ready(&mut rt);
            assert_eq!(
                value(&line(&lines, "publication"), "sidecar"),
                "not_attempted"
            );
            assert_eq!(rt.session().index.call_edges_loaded_from_cache(), None);
            if query {
                edges(&rt);
            } else {
                assert!(!nodes_at(rt.session(), "a.py", 1).items.is_empty());
            }
            let forced = rt.session().index.call_edges_loaded_from_cache();
            unload(&mut rt);
            let expected = if !query || state == "ineligible" {
                "not_attempted"
            } else if crate::cpg_cache::binary_input_dirty() {
                "refused_dirty"
            } else {
                state
            };
            assert_eq!(
                value(&line(&lines, "unload"), "sidecar"),
                expected,
                "late outcome exports in stdio"
            );
            for event in ["disposal_start", "disposal_end", "relief"] {
                assert_eq!(
                    value(&line(&lines, event), "sidecar"),
                    "not_applicable",
                    "{event} must not contradict unload {expected}"
                );
                assert_eq!(value(&line(&lines, event), "reason"), "idle");
            }
            assert_eq!(forced.is_some(), query);
        }
    }
}
#[test]
fn o_lifecycle_events() {
    let (_dir, cfg) = fixture();
    let (mut rt, lines) = runtime(&cfg);
    ready(&mut rt);
    drop(rt.activity());
    rt.refresh_index().unwrap();
    rt.auto_refresh_index().unwrap();
    rt.fail_refresh_for_tests();
    assert!(rt.refresh_index().is_err());
    assert!(line(&lines, "refresh").contains("outcome=failed"));
    rt.fail_refresh_for_tests();
    assert!(rt.auto_refresh_index().is_err());
    assert!(line(&lines, "auto_refresh").contains("outcome=failed"));
    unload(&mut rt);
    ready(&mut rt);
    unload(&mut rt);
    for name in "build_start build_finish publication build reload refresh auto_refresh last_release unload disposal_start disposal_end relief".split_whitespace() {
        line(&lines, name);
    }
    super::parity_tests::delayed_publication(&cfg);
    for attempt in [1, 2] {
        let logs = lines.lock().unwrap();
        let find = |event| {
            logs.iter()
                .find(|s| value(s, "event") == event && number(s, "attempt") == attempt)
                .unwrap()
        };
        assert!(number(find("build_finish"), "mono_ns") <= number(find("publication"), "mono_ns"));
    }
    let logs = lines.lock().unwrap();
    let names: Vec<_> = logs.iter().map(|s| value(s, "event")).collect();
    assert_eq!(names.iter().filter(|n| **n == "last_release").count(), 1);
    assert!(names.windows(2).any(|p| p == ["last_release", "unload"]));
}
#[test]
fn o_event_identity_and_clock() {
    let (_dir, cfg) = fixture();
    let (mut rt, lines) = runtime(&cfg);
    ready(&mut rt);
    unload(&mut rt);
    let logs = lines.lock().unwrap();
    let mut seq = 0;
    for s in logs.iter() {
        sidecar_scope(s);
        assert_eq!(number(s, "pid"), std::process::id() as u128);
        assert!(number(s, "epoch_ns") > 0 && number(s, "instance_start") > 0);
        assert_ne!(value(s, "identity"), "unbound");
        assert!(number(s, "sequence") > seq);
        seq = number(s, "sequence");
        assert!(number(s, "mono_ns") < number(s, "epoch_ns"));
        assert!(s.len() <= 1000 && !s.chars().any(char::is_control));
    }
}
#[test]
fn o_relief_pair_after_drop() {
    let (_dir, mut cfg) = fixture();
    cfg.cache = crate::mcp::CacheMode::NoCache;
    let (mut rt, lines) = runtime(&cfg);
    ready(&mut rt);
    let weak = std::sync::Arc::downgrade(&rt.session().repo);
    let check = weak.clone();
    let calls = std::sync::atomic::AtomicU64::new(0);
    rt.log_context().sampler = Some(std::sync::Arc::new(move || {
        assert!(
            check.upgrade().is_none(),
            "sample/relief must follow whole-provider drop"
        );
        crate::mcp::mem_sample::MemSample {
            heap_in_use_bytes: (calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst) != 0)
                .then_some(42),
            ..Default::default()
        }
    }));
    unload(&mut rt);
    assert!(weak.upgrade().is_none());
    let s = line(&lines, "relief");
    assert_eq!(value(&s, "heap_before"), "unknown");
    assert_eq!(
        value(&s, "heap_after"),
        "42",
        "second sample is independent"
    );
    assert!(number(&s, "sample_before_ns") <= number(&s, "sample_after_ns"));
    for k in "heap_before heap_after footprint_before footprint_after".split_whitespace() {
        assert!(!value(&s, k).is_empty());
    }
    assert_eq!(value(&s, "released_bytes"), "0");
    assert!(number(&line(&lines, "disposal_end"), "mono_ns") <= number(&s, "sample_before_ns"));
}
