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
