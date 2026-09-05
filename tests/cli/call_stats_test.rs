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
fn call_stats_reports_parameter_slots_and_disabled_level3() {
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
    assert_eq!(v["level3_indirect_resolved"], 0);
}

#[test]
fn call_stats_reports_go_level3_b1_conservation() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("callbacks.go"),
        "package p\nfunc invoke(cb func()) { cb() }\nfunc safe() {}\nfunc accepted() { invoke(safe) }\nfunc dropped() { safe := func() {}; invoke(safe) }\n",
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
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["go_level3_b1_candidates"], 2);
    assert_eq!(value["go_level3_b1_exact_inbound_sites"], 2);
    assert_eq!(value["go_level3_b1_accepted_inbound_sites"], 1);
    assert_eq!(value["go_level3_b1_unique_targets"], 1);
    assert_eq!(value["go_level3_b1_edges"], 1);
    assert_eq!(value["level3_indirect_resolved"], 1);
    assert_eq!(value["go_level3_b1_drops"]["local_binding_or_mutation"], 1);
    let drop_total: u64 = value["go_level3_b1_drops"]
        .as_object()
        .unwrap()
        .values()
        .map(|count| count.as_u64().unwrap())
        .sum();
    assert_eq!(
        value["go_level3_b1_candidates"].as_u64().unwrap(),
        value["go_level3_b1_accepted_inbound_sites"]
            .as_u64()
            .unwrap()
            + drop_total
    );
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
fn call_stats_reports_existing_concrete_promotion_and_ambiguity() {
    let dir = tempfile::tempdir().unwrap();
    // One existing concrete promotion (Wrap.Ping) + one equal-depth ambiguity
    // (A.M via X,Y).
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
    assert_eq!(v["kinds"]["embedded_promotion"], 1, "{v:#}");
    assert_eq!(v["go_concrete_receiver_promoted_existing"], 1);
    assert_eq!(v["go_concrete_receiver_promoted_deferred"], 0);
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

#[test]
fn call_stats_reports_go_testdata_skip_and_excludes_its_exact_edge() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("testdata")).unwrap();
    std::fs::write(dir.path().join("go.mod"), "module example.com/root\n").unwrap();
    std::fs::write(
        dir.path().join("main.go"),
        "package root\n\
         type Doer interface { Act(string) }\n\
         type Holder struct { Doer }\n\
         func invoke(h Holder) { h.Act(\"ok\") }\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("testdata/impl.go"),
        "package testdata\n\
         type Impl struct{}\n\
         func (Impl) Act(string) {}\n",
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
    assert!(v["kind_exact"].get("interface_dispatch").is_none());
    assert_eq!(v["skipped_go_testdata_files"], 1);
}

#[test]
fn call_stats_omits_zero_go_testdata_skip_for_byte_compatibility() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("main.go"), "package main\nfunc main() {}\n").unwrap();

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
    assert!(v.get("skipped_go_testdata_files").is_none());
}

#[test]
fn call_stats_reports_go_module_graph_and_import_path_conservation() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    for subdir in ["nested", "inactive", "malformed", "linked", "versioned"] {
        std::fs::create_dir_all(root.join(subdir)).unwrap();
    }
    std::fs::write(
        root.join("go.mod"),
        "module example.com/root\nrequire original.example/a v1.0.0\nreplace original.example/a v1.0.0 => ./versioned\n",
    )
    .unwrap();
    std::fs::write(root.join("go.work"), "go 1.22\nuse (\n.\n./nested\n)\n").unwrap();
    std::fs::write(root.join("nested/go.mod"), "module example.com/nested\n").unwrap();
    std::fs::write(
        root.join("inactive/go.mod"),
        "module example.com/inactive\n",
    )
    .unwrap();
    std::fs::write(root.join("malformed/go.mod"), "module bad path\n").unwrap();
    std::fs::write(
        root.join("versioned/go.mod"),
        "module versioned.example/a\n",
    )
    .unwrap();
    std::fs::write(root.join("linked-target"), "module linked.example/a\n").unwrap();
    std::os::unix::fs::symlink("../linked-target", root.join("linked/go.mod")).unwrap();
    for file in [
        "root.go",
        "nested/nested.go",
        "inactive/inactive.go",
        "malformed/malformed.go",
        "linked/linked.go",
        "versioned/versioned.go",
    ] {
        std::fs::write(root.join(file), "package fixture\nfunc f() {}\n").unwrap();
    }

    let out = Command::cargo_bin("prism")
        .unwrap()
        .args(["nav", "--no-cache", "call-stats", "--repo"])
        .arg(root)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        v["go_module_graph"],
        serde_json::json!({
            "modules": 4,
            "active": 2,
            "replaces_parsed": 1,
            "replaces_applied": 0,
            "workspace_invalid": false,
        })
    );
    assert_eq!(v["go_import_path_proven_files"], 2);
    assert_eq!(v["go_import_path_unproven_files"], 4);
    assert_eq!(
        v["go_import_path_unproven_reasons"],
        serde_json::json!({
            "inactive_module": 1,
            "malformed": 1,
            "replace_unproven": 1,
            "symlink": 1,
        })
    );
    let proven = v["go_import_path_proven_files"].as_u64().unwrap();
    let unproven = v["go_import_path_unproven_files"].as_u64().unwrap();
    let reason_sum: u64 = v["go_import_path_unproven_reasons"]
        .as_object()
        .unwrap()
        .values()
        .map(|count| count.as_u64().unwrap())
        .sum();
    assert_eq!(proven + unproven, 6);
    assert_eq!(reason_sum, unproven);
}

#[test]
fn call_stats_omits_go_module_extension_for_non_go_byte_compatibility() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    let out = Command::cargo_bin("prism")
        .unwrap()
        .args(["nav", "--no-cache", "call-stats", "--repo"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    for key in [
        "go_module_graph",
        "go_import_path_proven_files",
        "go_import_path_unproven_files",
        "go_import_path_unproven_reasons",
    ] {
        assert!(v.get(key).is_none(), "unexpected non-Go field {key}");
    }
}

#[test]
fn call_stats_dump_sites_emits_no_synthetic_callback_custody() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("callbacks.js"),
        "function safe() {}\nfunction invoke(cb) { cb(); }\nfunction forward() { invoke(safe); }\n",
    )
    .unwrap();
    let out = Command::cargo_bin("prism")
        .unwrap()
        .args(["nav", "--no-cache", "call-stats", "--dump-sites", "--repo"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let sites: Vec<serde_json::Value> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(sites.len(), 2, "one JSONL record per source site");
    assert!(
        sites
            .iter()
            .all(|site| site["origin"] != "IndirectResolution"),
        "disabled Level-3 must emit no synthetic callback site"
    );
    assert!(sites.iter().all(|site| {
        site["resolved_targets"].as_array().map_or(true, |targets| {
            targets
                .iter()
                .all(|target| target["kind"] != "parameter_callback")
        })
    }));
}

#[test]
fn call_stats_dump_sites_classifies_accepted_and_dropped_go_level3_candidates() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("callbacks.go"),
        "package p\nfunc invoke(cb func()) { cb() }\nfunc safe() {}\nfunc accepted() { invoke(safe) }\nfunc dropped() { safe := func() {}; invoke(safe) }\n",
    )
    .unwrap();
    let out = Command::cargo_bin("prism")
        .unwrap()
        .args(["nav", "--no-cache", "call-stats", "--dump-sites", "--repo"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let records: Vec<serde_json::Value> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .filter(|record: &serde_json::Value| record["record_kind"] == "go_level3_b1_candidate")
        .collect();
    assert_eq!(records.len(), 2);
    let accepted = records
        .iter()
        .find(|record| record["decision"] == "accepted")
        .expect("accepted candidate record");
    assert_eq!(accepted["hof_name"], "invoke");
    assert_eq!(accepted["slot"], 0);
    assert_eq!(accepted["argument"], "safe");
    assert_eq!(accepted["hof"]["name"], "invoke");
    assert_eq!(accepted["exact_target"]["name"], "safe");
    assert_eq!(accepted["callback_parameter"], "cb");
    assert_eq!(accepted["invocation_spans"].as_array().unwrap().len(), 1);
    assert!(accepted["drop_reason"].is_null());

    let dropped = records
        .iter()
        .find(|record| record["decision"] == "dropped")
        .expect("dropped candidate record");
    assert_eq!(dropped["hof_name"], "invoke");
    assert_eq!(dropped["slot"], 0);
    assert_eq!(dropped["argument"], "safe");
    assert_eq!(dropped["hof"]["name"], "invoke");
    assert!(dropped["exact_target"].is_null());
    assert_eq!(dropped["callback_parameter"], "cb");
    assert_eq!(dropped["drop_reason"], "local_binding_or_mutation");
    for record in &records {
        assert_eq!(record["inbound_span"]["file"], "callbacks.go");
        assert!(record["inbound_span"]["start_byte"].as_u64().unwrap() > 0);
        assert!(
            record["inbound_span"]["end_byte"].as_u64().unwrap()
                > record["inbound_span"]["start_byte"].as_u64().unwrap()
        );
    }
}

#[test]
fn call_stats_emits_one_return_flow_subobject_with_all_custody_counters() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("app.py"),
        "def decorate(user):\n    return user + 'x'\n\ndef run(user):\n    value = decorate(user)\n    sink(value)\n",
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
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        value["return_flow"],
        serde_json::json!({
            "return_flow_edges": 1,
            "return_input_edges": 1,
            "return_flow_skipped_nameonly": 0,
            "return_flow_skipped_multi": 0,
            "return_flow_skipped_mixed": 0,
            "return_flow_skipped_non_simple_lhs": 0,
            "return_flow_skipped_arity_mismatch": 0,
            "return_flow_skipped_named_return": 0,
            "return_flow_skipped_forwarded_return": 0,
            "return_flow_suppression_certified": 1,
            "return_flow_suppression_void_incomplete_returns": 0,
            "return_flow_suppression_void_unbound_uses": 0,
        })
    );
}

#[test]
fn dfg_labels_are_additive_and_preserve_preexisting_call_stats_values() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("app.py"),
        "def run():\n    x = source()\n    sink(x)\n",
    )
    .unwrap();

    let mut options = prism::api::NavOptions::default();
    options.no_cache = true;
    let session = prism::api::nav_session(dir.path(), &options).unwrap();
    let mut expected = prism::navigation::queries::call_stats(session.index.call_graph());
    expected.as_object_mut().unwrap().insert(
        "return_flow".into(),
        serde_json::to_value(&session.index.cpg().return_flow_stats).unwrap(),
    );

    let output = Command::cargo_bin("prism")
        .unwrap()
        .args(["nav", "--no-cache", "call-stats", "--repo"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut actual: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let dfg_labels = actual
        .as_object_mut()
        .unwrap()
        .remove("dfg_labels")
        .expect("call-stats must add dfg_labels");
    assert!(dfg_labels.is_object(), "{dfg_labels:#}");
    assert_eq!(actual, expected, "a pre-existing call-stats leaf changed");
}
