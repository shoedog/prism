use super::*;
use crate::mcp::{registry::ToolRegistry, settings::ConnectionSettings, transport};
use serde_json::{json, Value};
use std::os::unix::fs::PermissionsExt;

fn fixture(policy: RefreshPolicy) -> (tempfile::TempDir, ServerConfig) {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("a.py"), "def old():\n    return 1\n").unwrap();
    let mut cfg = ServerConfig::new(d.path().to_path_buf());
    cfg.cache = CacheMode::NoCache;
    cfg.refresh_policy = policy;
    (d, cfg)
}
fn query(p: &mut SessionProvider) -> Value {
    let registry = ToolRegistry::all_v1();
    let settings = ConnectionSettings {
        result_cap: 80_000,
        structured_content: Default::default(),
        concise_shape: Default::default(),
    };
    let message = json!({"jsonrpc":"2.0", "id":1, "method":"tools/call",
        "params":{"name":"nav_nodes_at", "arguments":{"file":"a.py", "line":1}}});
    let mut lifecycle = transport::Lifecycle::Initialized;
    let transport::Control::ToolCall(call) = transport::handle_control(
        &message,
        &mut lifecycle,
        &registry,
        StartupMode::Eager,
        &settings,
    ) else {
        panic!("valid fixture must produce a tool call")
    };
    transport::dispatch_tool(&call, p, &settings)["result"].clone()
}
fn raced(cfg: &ServerConfig, f: impl FnOnce()) -> SessionProvider {
    let mut f = Some(f);
    SessionProvider::bootstrap_observed(cfg, &mut |stage| {
        if stage == BuildStage::RepoLoaded {
            f.take().unwrap()();
        }
    })
    .unwrap()
}
#[test]
fn freshness_read_a_edit_b_stamp_b() {
    let (d, cfg) = fixture(RefreshPolicy::WarnOnly);
    let meta = d.path().join("meta");
    std::fs::create_dir(&meta).unwrap();
    write_source(&meta, "blocked.py", "x=1");
    std::fs::set_permissions(&meta, std::fs::Permissions::from_mode(0o400)).unwrap();
    assert!(std::fs::metadata(meta.join("blocked.py")).is_err());
    let mut p = raced(&cfg, || {
        std::fs::write(
            d.path().join("a.py"),
            "def replacement():\n    return 222\n",
        )
        .unwrap()
    });
    assert!(p.session().repo.files["a.py"].source.contains("def old()"));
    let result = query(&mut p);
    std::fs::set_permissions(&meta, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(result.to_string().contains("old"), "index still contains A");
    assert_eq!(
        result["_meta"]["prism/index_freshness"], "stale",
        "read A/edit B/stamp B must never report A clean"
    );
    assert_eq!(result["_meta"]["prism/stale_index_total"], 1);
    assert_eq!(result["_meta"]["prism/stale_index_paths"], json!(["a.py"]));
    assert!(
        p.idle_publication().diverged,
        "idle publication must carry bootstrap divergence"
    );
}
#[test]
fn freshness_warn_auto_policy_metadata() {
    for policy in [
        RefreshPolicy::WarnOnly,
        RefreshPolicy::AutoFull,
        RefreshPolicy::AutoIncremental,
    ] {
        let (d, cfg) = fixture(policy);
        let mut p = raced(&cfg, || {
            std::fs::write(
                d.path().join("a.py"),
                "def replacement():\n    return 222\n",
            )
            .unwrap()
        });
        let result = query(&mut p);
        if policy == RefreshPolicy::WarnOnly {
            assert_eq!(result["_meta"]["prism/index_freshness"], "stale");
            assert_eq!(p.generation(), 0);
        } else {
            assert_eq!(result["_meta"]["prism/auto_refresh"], "refreshed");
            assert!(result.to_string().contains("replacement"));
            assert!(result["_meta"].get("prism/index_freshness").is_none());
            assert_eq!(p.generation(), 1);
            assert!(!p.idle_publication().diverged);
        }
    }
}
#[test]
fn freshness_policy_race_failure_and_clean_clearing() {
    for policy in [RefreshPolicy::AutoFull, RefreshPolicy::AutoIncremental] {
        let (d, cfg) = fixture(policy);
        let mut p = SessionProvider::bootstrap(&cfg).unwrap();
        std::fs::write(
            d.path().join("a.py"),
            "def replacement():\n    return 222\n",
        )
        .unwrap();
        let report = FreshnessReport::from_changed_paths(["a.py".into()]);
        p.force_next_verification_for_tests(RefreshVerification::Diverged(report));
        assert_eq!(query(&mut p)["_meta"]["prism/auto_refresh"], "raced_stale");
        p.force_next_refresh_error_for_tests("fixture refresh refused");
        let before = p.generation();
        let result = query(&mut p);
        assert_eq!(result["_meta"]["prism/index_freshness"], "stale");
        assert_eq!(result["_meta"]["prism/auto_refresh"], "failed");
        assert_eq!(
            p.generation(),
            before,
            "failure preserves resident generation"
        );
        assert!(p.known_stale_after_refresh().is_some());
        assert_eq!(query(&mut p)["_meta"]["prism/auto_refresh"], "refreshed");
        assert!(p.known_stale_after_refresh().is_none());
    }
}
#[test]
fn freshness_uncertifiable_refresh_once_and_rearm() {
    for policy in [RefreshPolicy::AutoFull, RefreshPolicy::AutoIncremental] {
        let (d, cfg) = fixture(policy);
        let path = d.path().join("a.py");
        let mut p = raced(&cfg, || {
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o0)).unwrap()
        });
        p.force_next_refresh_error_for_tests("fixture refresh refused");
        assert_eq!(query(&mut p)["_meta"]["prism/auto_refresh"], "failed");
        let result = query(&mut p);
        assert!(
            result["_meta"].get("prism/auto_refresh").is_none(),
            "identical uncertifiable evidence cannot refresh-loop"
        );
        assert_eq!(result["_meta"]["prism/index_freshness"], "stale");
        assert_eq!(
            p.cfg.refresh_policy, policy,
            "configured policy is unchanged"
        );
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        std::fs::write(&path, "def changed_evidence():\n    return 33333\n").unwrap();
        assert_eq!(
            query(&mut p)["_meta"]["prism/auto_refresh"],
            "refreshed",
            "changed evidence re-arms auto policy"
        );
        assert!(p.known_stale_after_refresh().is_none());
    }
}

fn permission_only_recovery(policy: RefreshPolicy) {
    let (d, cfg) = fixture(policy);
    let path = d.path().join("a.py");
    let bytes = std::fs::read(&path).unwrap();
    let identity = |path: &std::path::Path| {
        let metadata = std::fs::metadata(path).unwrap();
        (metadata.len(), metadata.modified().unwrap())
    };
    let file_identity = identity(&path);
    let root_identity = identity(d.path());
    let mut p = raced(&cfg, || {
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0)).unwrap();
    });
    assert_eq!(
        std::fs::read(&path).unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied,
        "requires owner-mode permission denial"
    );
    std::fs::set_permissions(d.path(), std::fs::Permissions::from_mode(0o100)).unwrap();
    assert_eq!(
        std::fs::read_dir(d.path()).unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied
    );
    assert_eq!(identity(&path), file_identity, "lookup/stat still works");
    let failed = query(&mut p);
    // Restore root access before assertions so even a failed fixture cleans up.
    let still_denied = std::fs::read_dir(d.path()).is_err();
    let suppressed = query(&mut p);
    std::fs::set_permissions(d.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert!(still_denied);
    assert_eq!(failed["_meta"]["prism/auto_refresh"], "failed");
    assert!(failed["_meta"]["prism/auto_refresh_error"]
        .as_str()
        .unwrap()
        .contains("failed to read repository root"));
    assert_eq!(failed["_meta"]["prism/index_freshness"], "stale");
    assert!(failed.to_string().contains("old"));
    assert!(
        suppressed["_meta"].get("prism/auto_refresh").is_none(),
        "unchanged unavailability allows only one failed attempt"
    );
    assert_eq!(suppressed["_meta"]["prism/index_freshness"], "stale");
    assert_eq!(p.generation(), 0);
    assert_eq!(p.cfg.refresh_policy, policy);
    assert_eq!(
        identity(&path),
        file_identity,
        "no file content/mtime change"
    );
    assert_eq!(
        identity(d.path()),
        root_identity,
        "no directory membership/mtime change"
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let recovered = query(&mut p);
    assert_eq!(
        recovered["_meta"]["prism/auto_refresh"], "refreshed",
        "permission-only recovery must re-arm the automatic policy"
    );
    assert!(recovered["_meta"].get("prism/index_freshness").is_none());
    assert!(recovered.to_string().contains("old"));
    assert_eq!(p.generation(), 1);
    assert!(p.known_stale_after_refresh().is_none());
    assert!(!p.idle_publication().diverged);
    assert!(query(&mut p)["_meta"].get("prism/auto_refresh").is_none());
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o400)).unwrap();
    let mode_only = query(&mut p);
    assert!(
        mode_only["_meta"].get("prism/index_freshness").is_none(),
        "access evidence must not affect ordinary freshness comparisons"
    );
    assert_eq!(p.generation(), 1);
}

#[test]
fn freshness_permission_recovery_auto_full() {
    permission_only_recovery(RefreshPolicy::AutoFull);
}

#[test]
fn freshness_permission_recovery_auto_incremental() {
    permission_only_recovery(RefreshPolicy::AutoIncremental);
}

#[test]
fn freshness_certain_sticky_does_no_stat_work() {
    let (d, cfg) = fixture(RefreshPolicy::WarnOnly);
    let mut p = raced(&cfg, || {
        std::fs::write(d.path().join("a.py"), "def replacement():\n    pass\n").unwrap();
    });
    crate::mcp::freshness::capture_tests::on("path-stat", || {
        panic!("certain sticky report must do no per-query stat work");
    });
    let result = query(&mut p);
    assert_eq!(result["_meta"]["prism/index_freshness"], "stale");
    assert!(result.to_string().contains("old"));
    // Exercise the direct check path as well as SessionProvider's sticky accessor.
    assert!(p.freshness().check().stale);
    crate::mcp::freshness::capture_tests::on("unused", || {});

    // Verification can diverge after an otherwise clean candidate capture.
    let (d, cfg) = fixture(RefreshPolicy::WarnOnly);
    let mut p = SessionProvider::bootstrap(&cfg).unwrap();
    std::fs::write(d.path().join("a.py"), "def replacement():\n    pass\n").unwrap();
    let report = FreshnessReport::from_changed_paths(["a.py".into()]);
    p.force_next_verification_for_tests(RefreshVerification::Diverged(report));
    assert_eq!(p.refresh().unwrap().status, "raced_stale");
    assert!(p.freshness().bootstrap_report().is_none());
    crate::mcp::freshness::capture_tests::on("path-stat", || {
        panic!("certain verification-only sticky report must do no per-query stat work");
    });
    let result = query(&mut p);
    assert_eq!(result["_meta"]["prism/index_freshness"], "stale");
    assert!(result.to_string().contains("replacement"));
    crate::mcp::freshness::capture_tests::on("unused", || {});
}
#[test]
fn freshness_service_recheck_after_validation() {
    let (d, cfg) = fixture(RefreshPolicy::WarnOnly);
    let mut p = SessionProvider::bootstrap_observed(&cfg, &mut |stage| {
        if stage == BuildStage::IndexBuilt {
            std::fs::write(d.path().join("a.py"), "def after_validation():\n    pass\n").unwrap();
        }
    })
    .unwrap();
    assert_eq!(
        query(&mut p)["_meta"]["prism/index_freshness"],
        "stale",
        "service recheck must see post-validation edit"
    );
}

#[test]
fn freshness_diverged_commit_preserves_uncertifiable_attempt() {
    for policy in [RefreshPolicy::AutoFull, RefreshPolicy::AutoIncremental] {
        let (d, cfg) = fixture(policy);
        let mut p = SessionProvider::bootstrap(&cfg).unwrap();
        let path = d.path().join("a.py");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o0)).unwrap();
        let first = FreshnessProbe::from_loaded_repo(&p.session.repo);
        let replacement = FreshnessProbe::from_loaded_repo(&p.session.repo);
        let first_report = first.check();
        let replacement_report = replacement.check();
        let key = first.uncertifiable_key();
        let replacement_key = replacement.uncertifiable_key();
        // Restore access before assertions and candidate loading; no unlock/relock
        // occurs between the two captures of identical uncertain evidence.
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(first_report.stale);
        assert_eq!(first_report, replacement_report);
        assert!(key.is_some(), "fixture must be uncertifiable");
        assert_eq!(
            key, replacement_key,
            "equal evidence must suppress replacement"
        );

        // Isolate the commit invariant from loader access/ctime changes. The
        // automatic attempt records this key before committing a candidate.
        p.auto_attempt = key;
        p.force_next_verification_for_tests(RefreshVerification::Diverged(first_report.clone()));
        let committed = p.refresh_verified(RefreshStrategyPolicy::FullOnly).unwrap();
        assert_eq!(
            committed.verification,
            RefreshVerification::Diverged(first_report)
        );
        assert_eq!(p.generation(), 1, "a replacement was committed");
        assert_eq!(
            p.auto_attempt, key,
            "Diverged commit must preserve auto_attempt"
        );
        // With equally uncertain replacement evidence, the preserved attempt
        // suppresses the next query without changing the configured policy.
        p.freshness = replacement;
        assert_eq!(p.refresh_policy(), RefreshPolicy::WarnOnly);
        assert_eq!(p.cfg.refresh_policy, policy);

        p.force_next_verification_for_tests(RefreshVerification::Clean);
        p.refresh_verified(RefreshStrategyPolicy::FullOnly).unwrap();
        assert_eq!(p.auto_attempt, None, "Clean commit must clear auto_attempt");
        assert_eq!(p.refresh_policy(), policy);
        assert!(p.known_stale_after_refresh().is_none());
    }
}

#[test]
fn freshness_equally_uncertifiable_replacement_and_explicit_refresh() {
    for policy in [RefreshPolicy::AutoFull, RefreshPolicy::AutoIncremental] {
        let (d, cfg) = fixture(policy);
        let path = d.path().join("a.py");
        let lock = |p| std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o0)).unwrap();
        let unlock =
            |p| std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o600)).unwrap();
        let mut p = raced(&cfg, || lock(&path));
        unlock(&path);
        let next = path.clone();
        crate::mcp::freshness::capture_tests::on("before-validation", move || {
            std::fs::set_permissions(next, std::fs::Permissions::from_mode(0o0)).unwrap();
        });
        let report = FreshnessReport::from_changed_paths(["a.py".into()]);
        p.force_next_verification_for_tests(RefreshVerification::Diverged(report));
        assert_eq!(query(&mut p)["_meta"]["prism/auto_refresh"], "raced_stale");
        // The unlock/relock during candidate construction changed access evidence.
        // It permits one new attempt; unchanged failure must then suppress again.
        p.force_next_refresh_error_for_tests("changed-access refresh refused");
        assert_eq!(query(&mut p)["_meta"]["prism/auto_refresh"], "failed");
        assert!(query(&mut p)["_meta"].get("prism/auto_refresh").is_none());
        p.force_next_refresh_error_for_tests("explicit refresh refused");
        let generation = p.generation();
        assert!(p.refresh().is_err());
        assert_eq!(p.generation(), generation);
        assert!(p.known_stale_after_refresh().is_some());
        unlock(&path);
        assert_eq!(p.refresh().unwrap().status, "refreshed");
        assert!(p.known_stale_after_refresh().is_none());
    }
}

fn write_source(root: &std::path::Path, rel: &str, text: &str) {
    std::fs::write(root.join(rel), text).unwrap();
}
#[test]
fn freshness_logical_keys_are_not_paths() {
    let (d, _) = fixture(RefreshPolicy::WarnOnly);
    write_source(
        d.path(),
        "tsconfig.json",
        r#"{"compilerOptions":{"baseUrl":"."}}"#,
    );
    write_source(d.path(), "js_paths_occupancy", "fake input");
    write_source(d.path(), "js_paths_config:user.py", "x=1");
    let probe = FreshnessProbe::from_loaded_repo(&load_repo(d.path()).unwrap());
    write_source(d.path(), "js_paths_occupancy", "changed fake input");
    assert!(!probe.check().stale, "logical label");
    write_source(d.path(), "js_paths_config:user.py", "x=12345");
    assert!(probe.check().stale, "real source");
}
