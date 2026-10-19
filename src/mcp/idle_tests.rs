use super::*;
use crate::mcp::{CacheMode, RefreshPolicy};
use std::sync::{Arc, Mutex};

fn fixture() -> (tempfile::TempDir, ServerConfig) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), "def f():\n    return 1\n").unwrap();
    let mut cfg = ServerConfig::new(dir.path().to_owned());
    cfg.cache = CacheMode::Dir(dir.path().join("cache"));
    (dir, cfg)
}

fn runtime(cfg: &ServerConfig) -> IdleRuntime {
    IdleRuntime::new(cfg, IdlePolicy::enabled(Duration::from_secs(10)), None).unwrap()
}

fn ready(rt: &mut IdleRuntime) {
    assert!(matches!(rt.ensure_ready(), Readiness::Ready));
}

fn unload(rt: &mut IdleRuntime) {
    let now = rt.idle_deadline().unwrap();
    rt.accept(Event::Tick(now));
    assert!(matches!(rt.phase, Phase::Unloaded { .. }));
}

#[test]
fn idle_releases_whole_session() {
    let (_dir, cfg) = fixture();
    for initialize_edges in [false, true] {
        let mut rt = runtime(&cfg);
        assert!(rt.provider.is_none()); // handshake-only never builds
        ready(&mut rt);
        if initialize_edges {
            let s = rt.session();
            let _ = s.index.resolved_call_edges();
        }
        let repo = Arc::downgrade(&rt.session().repo);
        let index = Arc::downgrade(&rt.session().index);
        rt.tick(rt.idle_deadline().unwrap() - Duration::from_nanos(1));
        assert!(repo.upgrade().is_some());
        unload(&mut rt);
        assert!(repo.upgrade().is_none(), "repository must be released");
        assert!(
            index.upgrade().is_none(),
            "index and lazy call edges must be released"
        );
        assert!(cfg.repo_root.join("cache").exists(), "disk caches stay");
        ready(&mut rt);
        unload(&mut rt);
    }
}

#[test]
fn idle_never_interrupts_request() {
    let (_dir, cfg) = fixture();
    let mut rt = runtime(&cfg);
    ready(&mut rt);
    let clock = Rc::new(Cell::new(Instant::now()));
    let test_clock = clock.clone();
    rt.clock = Rc::new(move || test_clock.get());
    let old_deadline = rt.idle_deadline().unwrap();
    clock.set(old_deadline);
    let lease = rt.activity(); // readiness -> dispatch seam
    let response_lease = rt.activity(); // dispatch -> owned response seam
    clock.set(old_deadline + Duration::from_secs(100));
    rt.tick(old_deadline + Duration::from_secs(100));
    assert!(rt.provider.is_some(), "request pins the whole session");
    drop(lease);
    rt.tick(old_deadline + Duration::from_secs(100));
    assert!(rt.provider.is_some(), "response construction still pins");
    drop(response_lease);
    assert!(rt.idle_deadline().unwrap() > old_deadline);
    // Activity at exactly the deadline wins before disposal.
    let lease = rt.activity();
    rt.tick(rt.idle_deadline().unwrap_or(old_deadline));
    assert!(rt.provider.is_some());
    drop(lease);
    for auto in [false, true] {
        let lease = rt.activity();
        clock.set(clock.get() + Duration::from_secs(100));
        if auto {
            rt.auto_refresh_index().unwrap();
        } else {
            rt.refresh_index().unwrap();
        }
        rt.tick(clock.get());
        assert!(
            rt.provider.is_some(),
            "long explicit/auto refresh is pinned"
        );
        drop(lease);
    }
    let lease = rt.activity();
    rt.provider
        .as_mut()
        .unwrap()
        .force_next_refresh_error_for_tests("refresh error");
    assert!(rt.refresh_index().is_err());
    drop(lease);
    // Unwind releases admission too.
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _lease = rt.activity();
        panic!("simulated dispatch unwind");
    }));
    assert_eq!(rt.active.get(), 0);
    unload(&mut rt);
}

#[test]
fn idle_releases_uncollected_build() {
    for startup in [StartupMode::Background, StartupMode::Lazy] {
        let (_dir, mut cfg) = fixture();
        cfg.startup = startup;
        cfg.first_call_wait = Duration::ZERO;
        let mut rt = runtime(&cfg);
        if startup == StartupMode::Lazy {
            assert!(matches!(rt.ensure_ready(), Readiness::Warming { .. }));
        }
        // No readiness/tool call collects this result.
        let event = rt.rx.recv_timeout(Duration::from_secs(10)).unwrap();
        let finished = match &event {
            Event::Built { finished_at, .. } => *finished_at,
            _ => panic!("expected autonomous completion"),
        };
        rt.accept(event);
        assert!(matches!(rt.phase, Phase::Ready));
        let repo = Arc::downgrade(&rt.session().repo);
        assert_eq!(rt.published_at, Some(finished));
        rt.tick(finished + Duration::from_secs(100));
        assert!(
            repo.upgrade().is_none(),
            "completed build must not stay busy/pinned"
        );
    }
}

#[test]
fn idle_reload_rechecks_freshness() {
    let (dir, cfg) = fixture();
    let mut rt = runtime(&cfg);
    ready(&mut rt);
    rt.provider.as_mut().unwrap().refresh().unwrap();
    let g = rt.provider.as_ref().unwrap().generation();
    unload(&mut rt);
    ready(&mut rt);
    assert_eq!(rt.provider.as_ref().unwrap().generation(), g);
    for change in ["edit", "add", "delete", "touch"] {
        let previous = rt.provider.as_ref().unwrap().generation();
        unload(&mut rt);
        match change {
            "edit" => {
                std::fs::write(dir.path().join("a.py"), "def renamed():\n    return 2\n").unwrap()
            }
            "add" => {
                std::fs::write(dir.path().join("b.py"), "def added():\n    return 3\n").unwrap()
            }
            "delete" => std::fs::remove_file(dir.path().join("b.py")).unwrap(),
            "touch" => {
                let path = dir.path().join("a.py");
                let bytes = std::fs::read(&path).unwrap();
                std::fs::write(path, bytes).unwrap();
            }
            _ => unreachable!(),
        }
        ready(&mut rt);
        let provider = rt.provider.as_ref().unwrap();
        assert!(!provider.freshness().check().stale);
        assert_eq!(
            provider.generation(),
            previous + u64::from(change != "touch")
        );
        let s = provider.session();
        assert!(s.repo.files.values().any(|f| f.source.contains("renamed")));
        assert_eq!(s.repo.files.len(), if change == "add" { 2 } else { 1 });
    }
}

#[test]
fn idle_unheard_failure() {
    let (_dir, mut cfg) = fixture();
    cfg.startup = StartupMode::Background;
    let builder: SessionBuilder = Arc::new(|| anyhow::bail!("autonomous failure"));
    let mut rt = IdleRuntime::new(
        &cfg,
        IdlePolicy::enabled(Duration::from_secs(10)),
        Some(builder),
    )
    .unwrap();
    let event = rt.rx.recv_timeout(Duration::from_secs(5)).unwrap();
    rt.accept(event);
    assert!(matches!(rt.phase, Phase::Failed { heard: false, .. }));
    assert!(
        matches!(rt.ensure_ready(), Readiness::Failed { error } if error == "autonomous failure")
    );
    assert_eq!(rt.attempt, 1, "first call reports, does not retry");
    assert!(matches!(rt.ensure_ready(), Readiness::Failed { .. }));
    assert_eq!(rt.attempt, 2, "following call retries");
}

#[test]
fn idle_warming_retry_success() {
    let (_dir, mut cfg) = fixture();
    cfg.first_call_wait = Duration::ZERO;
    let (release, rx) = mpsc::channel();
    let rx = Mutex::new(rx);
    let builder_cfg = cfg.clone();
    let builder: SessionBuilder = Arc::new(move || {
        rx.lock().unwrap().recv().unwrap();
        SessionProvider::bootstrap(&builder_cfg)
    });
    let mut rt = IdleRuntime::new(
        &cfg,
        IdlePolicy::enabled(Duration::from_secs(10)),
        Some(builder),
    )
    .unwrap();
    assert!(matches!(rt.ensure_ready(), Readiness::Warming { .. }));
    assert!(matches!(rt.ensure_ready(), Readiness::Warming { .. }));
    assert_eq!(rt.attempt, 1);
    rt.tick(Instant::now() + Duration::from_secs(100));
    assert!(
        matches!(rt.phase, Phase::Building { .. }),
        "build >N is pinned"
    );
    release.send(()).unwrap();
    let event = rt.rx.recv_timeout(Duration::from_secs(10)).unwrap();
    rt.accept(event);
    ready(&mut rt);
    unload(&mut rt);
}

#[test]
fn idle_drops_sticky_diagnostics_and_keeps_generation() {
    let (_dir, mut cfg) = fixture();
    cfg.refresh_policy = RefreshPolicy::AutoFull;
    let mut rt = runtime(&cfg);
    ready(&mut rt);
    let provider = rt.provider.as_mut().unwrap();
    provider.force_next_verification_for_tests(
        super::super::session::RefreshVerification::Diverged(
            super::super::freshness::FreshnessReport::from_changed_paths(vec!["a.py".to_owned()]),
        ),
    );
    provider.refresh().unwrap();
    assert!(provider.known_stale_after_refresh().is_some());
    let generation = provider.generation();
    unload(&mut rt);
    ready(&mut rt);
    assert!(rt
        .provider
        .as_ref()
        .unwrap()
        .known_stale_after_refresh()
        .is_none());
    assert_eq!(rt.provider.as_ref().unwrap().generation(), generation);
}

#[test]
fn idle_pending_input_and_finished_at_clock() {
    let (_dir, cfg) = fixture();
    let mut rt = runtime(&cfg);
    ready(&mut rt);
    let deadline = rt.idle_deadline().unwrap();
    rt.tx
        .send(Event::Input(Input {
            outcome: Ok(Some(ReadOutcome::Malformed)),
            next: None,
        }))
        .unwrap();
    rt.accept(Event::Tick(deadline));
    assert!(rt.provider.is_some(), "queued input wins over timer");
    rt.pending.pop_front();
    // Collection may be arbitrarily later than completion. Its clock must not slide.
    let finished_at = Instant::now() - Duration::from_secs(100);
    rt.last_release.set(finished_at - Duration::from_secs(1));
    let provider = rt.provider.take().unwrap();
    rt.phase = Phase::Building {
        started: finished_at - Duration::from_secs(1),
        deadline: None,
    };
    rt.accept(Event::Built {
        result: Ok(provider),
        finished_at,
    });
    assert!(rt.idle_deadline().unwrap() < Instant::now());
    rt.accept(Event::Tick(Instant::now()));
    assert!(rt.provider.is_none());
}

#[test]
fn idle_reload_missing_corrupt_cache_and_failure_recovery() {
    let (dir, cfg) = fixture();
    let mut rt = runtime(&cfg);
    ready(&mut rt);
    let cache =
        crate::navigation::cache::nav_cache_subdir(&dir.path().join("cache"), &rt.session().repo);
    unload(&mut rt);
    std::fs::remove_dir_all(&cache).unwrap();
    ready(&mut rt);
    unload(&mut rt);
    // Corrupt only fixture-owned cache artifacts, never any user cache.
    fn corrupt(path: &std::path::Path) {
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                corrupt(&entry.path());
            } else {
                std::fs::write(entry.path(), b"invalid cache").unwrap();
            }
        }
    }
    corrupt(&cache);
    ready(&mut rt);
    unload(&mut rt);
    std::fs::remove_file(dir.path().join("a.py")).unwrap();
    std::fs::remove_dir_all(dir.path().join("cache")).unwrap();
    // A missing checkout fails, then the following call retries with the original config.
    std::fs::remove_dir(dir.path()).unwrap();
    assert!(matches!(rt.ensure_ready(), Readiness::Failed { .. }));
    std::fs::create_dir(dir.path()).unwrap();
    std::fs::write(dir.path().join("a.py"), "def recovered():\n    return 1\n").unwrap();
    ready(&mut rt);
    assert!(rt
        .session()
        .repo
        .files
        .values()
        .any(|f| f.source.contains("recovered")));
}

#[test]
fn idle_reload_manifest_and_resolver_changes() {
    let (dir, cfg) = fixture();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"name":"old","type":"module"}"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("tsconfig.json"),
        r#"{"compilerOptions":{"baseUrl":".","paths":{"@/*":["src/*"]}}}"#,
    )
    .unwrap();
    let mut rt = runtime(&cfg);
    ready(&mut rt);
    for (path, content) in [
        ("package.json", r#"{"name":"new","type":"commonjs"}"#),
        (
            "tsconfig.json",
            r#"{"compilerOptions":{"baseUrl":".","paths":{"@/*":["lib/*"]}}}"#,
        ),
    ] {
        let generation = rt.provider.as_ref().unwrap().generation();
        unload(&mut rt);
        std::fs::write(dir.path().join(path), content).unwrap();
        ready(&mut rt);
        assert!(!rt.freshness().unwrap().check().stale);
        assert_eq!(rt.provider.as_ref().unwrap().generation(), generation + 1);
        let reference = SessionProvider::bootstrap(&cfg).unwrap();
        assert_eq!(
            reference.idle_publication().fingerprint,
            rt.provider.as_ref().unwrap().idle_publication().fingerprint
        );
    }
}
