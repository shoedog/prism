//! S1b-1 (S1b SPEC §5, §7 A-4): `nav call-stats` counts the two new drop reasons.
use assert_cmd::Command;

#[test]
fn call_stats_reports_jsx_intrinsic_and_local_binding_counters() {
    // `div` (a same-file function) and `my-el` are intrinsic; `missing` and `Card` stay
    // `UnknownName`; nothing in S1b-1 drops `LocalBindingUnproven`.
    let dir = tempfile::tempdir().unwrap();
    let src = "function div() {\n  return 1;\n}\nexport function App() {\n  missing();\n  \
        return <div><my-el/><Card/></div>;\n}\n";
    std::fs::write(dir.path().join("a.tsx"), src).unwrap();
    let out = Command::cargo_bin("prism")
        .unwrap()
        .args(["nav", "--no-cache", "call-stats", "--repo"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["dropped_jsx_intrinsic"], 2);
    assert_eq!(v["unresolved_unknown_name"], 2);
    assert_eq!(v["dropped_local_binding_unproven"], 0);
    assert_eq!(v["total_call_sites"], 4);
}
