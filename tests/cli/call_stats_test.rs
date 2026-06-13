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
