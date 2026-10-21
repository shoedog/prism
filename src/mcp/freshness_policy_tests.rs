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
