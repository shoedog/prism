use assert_cmd::Command;

#[test]
fn call_stats_reports_kind_counts_and_drops() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("a.rs"),
        "impl A {\n    fn poll(&self) {}\n}\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("b.rs"),
        "impl B {\n    fn poll(&self) {}\n}\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("m.rs"), "fn drive() {\n    x.poll();\n}\n").unwrap();
    let out = Command::cargo_bin("prism")
        .unwrap()
        .args(["nav", "--no-cache", "call-stats", "--repo"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["dropped_multi_owner"], 1);
    assert!(v["kinds"].is_object());
}

#[test]
fn call_stats_reports_parameter_slot_and_level3_telemetry() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("callbacks.js"),
        "function safe() {}\nfunction blocked(cb, cb) { cb(); }\nfunction invoke(a, cb) { cb(); }\nfunction outer() {\n  blocked(safe, safe);\n  invoke(0, safe);\n}\n",
    )
    .unwrap();

    let out = Command::cargo_bin("prism")
        .unwrap()
        .args(["nav", "--no-cache", "call-stats", "--repo"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["param_slots_unknown"]["JavaScript"], 1);
    assert_eq!(v["level3_indirect_resolved"], 1);
}

#[test]
fn call_stats_same_name_owner_collision_demotes_out_of_multi_target_exact() {
    // Two distinct structs both literally named `Foo`, each with an associated
    // `make`, in separate files. A qualified `Foo::make()` call keys the bare
    // owner index `("Foo","make")` to BOTH defs; because both share the primary
    // owner name "Foo", `primary_owners` does NOT exceed 1. The demote-not-drop
    // fix emits this same-name collision at NameOnly (not Exact), so it is no
    // longer a multi-target-Exact site and both edges land in
    // kind_nameonly[qualified_owner].
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("a.rs"),
        "struct Foo;\nimpl Foo {\n    fn make() -> Foo { Foo }\n}\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("b.rs"),
        "struct Foo;\nimpl Foo {\n    fn make() -> Foo { Foo }\n}\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("m.rs"),
        "fn drive() {\n    Foo::make();\n}\n",
    )
    .unwrap();
    let out = Command::cargo_bin("prism")
        .unwrap()
        .args(["nav", "--no-cache", "call-stats", "--repo"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    // After demote-not-drop: the same-name `Foo::make` collision resolves at
    // NameOnly, so it is no longer a multi-target-Exact site, and both edges land
    // in kind_nameonly[qualified_owner] (the unrelabeled `::`-split path).
    assert_eq!(v["multi_target_exact_sites"], 0);
    assert_eq!(v["kind_nameonly"]["qualified_owner"], 2);
    assert!(v["kind_exact"].get("qualified_owner").is_none());
}

#[test]
fn call_stats_demoted_collision_absent_from_shape_and_shadow() {
    // `a.rs` defines `Foo` AND calls `Foo::make()`; `b.rs` defines a colliding
    // `Foo::make`. After demote-not-drop the call resolves NameOnly, so it is no
    // longer multi-target-Exact: the shape/shadow stratification (gated on >=2
    // Exact edges) does not run, and both edges land in kind_nameonly[qualified_owner].
    // (The shape/shadow counters stay in the code — forward instrument for the
    // completeness-gate follow-on, where ruff gains a scope graph and the shadow
    // again measures live narrowability.)
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("a.rs"),
        "struct Foo;\nimpl Foo {\n    fn make() -> Foo { Foo }\n    fn run(&self) {\n        Foo::make();\n    }\n}\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("b.rs"),
        "struct Foo;\nimpl Foo {\n    fn make() -> Foo { Foo }\n}\n",
    )
    .unwrap();
    let out = Command::cargo_bin("prism")
        .unwrap()
        .args(["nav", "--no-cache", "call-stats", "--repo"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    // Demoted: no multi-target-Exact site, so the shape and shadow maps are empty,
    // and both edges are NameOnly qualified_owner.
    assert_eq!(v["multi_target_exact_sites"], 0);
    assert_eq!(v["multi_target_exact_shape"].as_object().unwrap().len(), 0);
    assert_eq!(v["shadow_typepath_narrow"].as_object().unwrap().len(), 0);
    assert_eq!(v["kind_nameonly"]["qualified_owner"], 2);
}

#[test]
fn call_stats_recovery_counter_moves_on_recovered_owner_site() {
    // A repo whose scope graph DOES build (all Rust -> complete) and where a
    // `T::m` owner site recovers to a single Exact. The new recovery counter
    // records that site under `singleton`. The same fixture without a graph
    // would sit at NameOnly (the #120 floor); with the graph the recovery
    // instrument shows the win.
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("a/src")).unwrap();
    std::fs::create_dir_all(dir.path().join("b/src")).unwrap();
    std::fs::write(
        dir.path().join("a/src/lib.rs"),
        "pub struct CliTest;\nimpl CliTest {\n    pub fn with_file(&self) {}\n}\npub fn drive() {\n    CliTest::with_file();\n}\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("b/src/lib.rs"),
        "pub struct CliTest;\nimpl CliTest {\n    pub fn with_file(&self) {}\n}\n",
    )
    .unwrap();
    let out = Command::cargo_bin("prism")
        .unwrap()
        .args(["nav", "--no-cache", "call-stats", "--repo"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    // The recovered owner site lands a single Exact qualified_owner edge AND is
    // counted under the new recovery instrument's `singleton` bucket.
    assert_eq!(v["kind_exact"]["qualified_owner"], 1);
    assert_eq!(
        v["recovery_typepath"]["singleton"], 1,
        "the recovered owner site is recorded as a singleton recovery"
    );
}

#[test]
fn call_stats_reports_embedded_promotion_and_ambiguity() {
    let dir = tempfile::tempdir().unwrap();
    // One resolved promotion (Wrap.Ping) + one equal-depth ambiguity (A.M via X,Y).
    std::fs::write(
        dir.path().join("main.go"),
        "package main\ntype Base struct{}\nfunc (b Base) Ping() {}\ntype Wrap struct {\n\tBase\n}\nfunc run(w Wrap) {\n\tw.Ping()\n}\ntype X struct{}\nfunc (x X) M() {}\ntype Y struct{}\nfunc (y Y) M() {}\ntype A struct {\n\tX\n\tY\n}\n",
    )
    .unwrap();
    let out = Command::cargo_bin("prism")
        .unwrap()
        .args(["nav", "--no-cache", "call-stats", "--repo"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["kinds"]["embedded_promotion"], 1);
    assert_eq!(v["embedding_gaps"]["ambiguous"], 1);
    assert!(v["interface_gaps"].is_object());
    assert!(v["interface_overapprox"].is_object());
    assert!(v["interface_fanout"].is_object());
}

#[test]
fn call_stats_counts_malformed_go_build_file_once_not_per_consult() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("bad.go"),
        "//go:build linux &&

package a
func f() {}
",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("other_windows.go"),
        "package a
func f() {}
",
    )
    .unwrap();
    let callers = (0..50)
        .map(|i| {
            format!(
                "func use{i}() {{ f() }}
"
            )
        })
        .collect::<String>();
    std::fs::write(
        dir.path().join("use_linux.go"),
        format!(
            "package a
{callers}"
        ),
    )
    .unwrap();
    let out = Command::cargo_bin("prism")
        .unwrap()
        .args(["nav", "--no-cache", "call-stats", "--repo"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["go_build_expr_unparsed"], 1);
}

#[test]
fn call_stats_reports_js_export_reexport_telemetry() {
    // P4: a 3-hop re-export chain exceeds js_exports::MAX_REEXPORT_DEPTH (2)
    // and fails closed -- js_export_chain_unresolved must count it (the chain
    // never emits an import_member edge, so there's no other trace of it).
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("impl.ts"),
        "export function process(): number { return 1; }\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("mid2.ts"),
        "export { process } from './impl';\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("mid.ts"),
        "export { process } from './mid2';\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("index.ts"),
        "export { process } from './mid';\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("app.ts"),
        "import { process } from './index';\nfunction run() { process(); }\n",
    )
    .unwrap();
    let out = Command::cargo_bin("prism")
        .unwrap()
        .args(["nav", "--no-cache", "call-stats", "--repo"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["js_export_chain_unresolved"], 1);
    assert_eq!(v["js_export_barrel_conflicts"], 0);
}

#[test]
fn call_stats_reports_js_export_star_only_reexport_telemetry() {
    // F5 (review-fix wave, codex MINOR = opus Minor 1): mirrors
    // `call_stats_reports_js_export_reexport_telemetry` above, but the 3-hop
    // chain is `export * from` barrels the whole way instead of named
    // re-export lists -- previously this star-only form escaped
    // `js_export_chain_unresolved` telemetry entirely.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("impl.ts"),
        "export function process(): number { return 1; }\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("mid2.ts"), "export * from './impl';\n").unwrap();
    std::fs::write(dir.path().join("mid.ts"), "export * from './mid2';\n").unwrap();
    std::fs::write(dir.path().join("index.ts"), "export * from './mid';\n").unwrap();
    std::fs::write(
        dir.path().join("app.ts"),
        "import { process } from './index';\nfunction run() { process(); }\n",
    )
    .unwrap();
    let out = Command::cargo_bin("prism")
        .unwrap()
        .args(["nav", "--no-cache", "call-stats", "--repo"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v["js_export_chain_unresolved"].as_u64().unwrap() > 0);
}

#[test]
fn call_stats_reports_js_export_skipped_exprs() {
    // F6 (opus Minor 2, controller-adjudicated: do it -- `skipped_expr_count`
    // becomes load-bearing once F1-F4 add more skip paths). Aggregated across
    // all files' `JsExportFacts::skipped_expr_count` and surfaced as
    // `js_export_skipped_exprs`, alongside the other two P4 counters.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("util.js"),
        "function helper() { return 1; }\nmodule.exports = helper();\n",
    )
    .unwrap();
    let out = Command::cargo_bin("prism")
        .unwrap()
        .args(["nav", "--no-cache", "call-stats", "--repo"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v["js_export_skipped_exprs"].as_u64().unwrap() > 0);
}

#[test]
fn call_stats_reports_macro_arg_telemetry() {
    // P8: rust_macro_args -- one allowlisted-macro-minted free call
    // (`check(1)` inside `assert!`), one non-allowlisted skipped macro
    // (`stringify!(check(x))`, call-shaped -> counted), and one
    // uppercase-constructor skip (`Foo(1)` inside `assert!`).
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("a.rs"),
        "fn check(x: i32) -> bool { x > 0 }\n\
         fn host() {\n    \
             assert!(check(1));\n    \
             stringify!(check(2));\n    \
             assert!(Foo(3));\n\
         }\n",
    )
    .unwrap();
    let out = Command::cargo_bin("prism")
        .unwrap()
        .args(["nav", "--no-cache", "call-stats", "--repo"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["macro_arg_calls_recorded"].as_u64().unwrap(), 1);
    assert_eq!(v["macro_arg_skipped_macros"].as_u64().unwrap(), 1);
    assert_eq!(v["macro_arg_ctor_skips"].as_u64().unwrap(), 1);
}
