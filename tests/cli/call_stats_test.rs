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
fn call_stats_reports_multi_target_exact_same_name_owner_collision() {
    // Two distinct structs both literally named `Foo`, each with an associated
    // `make`, in separate files. A qualified `Foo::make()` call keys the bare
    // owner index `("Foo","make")` to BOTH defs; because both share the primary
    // owner name "Foo", `primary_owners` does NOT exceed 1, so the TraitCha demote
    // never fires and the site resolves to two callees at Exact (1.0) — the
    // same-bare-name owner-key over-attribution the counter must surface.
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
    assert_eq!(v["multi_target_exact_sites"], 1);
    // Fanout-2 bucket holds exactly this site; keyed by stringified fanout.
    assert_eq!(v["multi_target_exact_fanout"]["2"], 1);
    // Attributed to the qualified-owner kind that minted the colliding pool.
    assert_eq!(v["multi_target_exact_by_kind"]["qualified_owner"], 1);
}

#[test]
fn call_stats_shadow_stratifies_type_path_collision_and_runs_narrowing() {
    // Pre-gate shadow: `a.rs` defines `Foo` AND calls `Foo::make()`; `b.rs` defines
    // a colliding `Foo::make`. The call's `callee_name` is "Foo::make" -> `type_path`
    // shape, and the narrowing shadow runs over it. A flat two-file repo gives the
    // scope graph no module structure to disambiguate the two `Foo`s, so the owner
    // type does not resolve to a single in-repo scope and the site is classified
    // `failopen_type_unresolved` (the residual FP shape the lever cannot reclaim
    // without a richer scope graph).
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
    assert_eq!(v["multi_target_exact_sites"], 1);
    // The colliding site is recognized as a type-path (`T::m`) shape...
    assert_eq!(v["multi_target_exact_shape"]["type_path"], 1);
    // ...and the narrowing shadow runs over it and classifies it (here the owner
    // type cannot be resolved to a single in-repo scope -> fail-open).
    assert_eq!(v["shadow_typepath_narrow"]["failopen_type_unresolved"], 1);
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
