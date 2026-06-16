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
