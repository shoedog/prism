//! S1b-2b (S1b SPEC §5): `nav call-stats` reports the D4 export route's refusals by reason
//! and the export occurrences kept at base behavior by E5.
use assert_cmd::Command;

#[test]
fn call_stats_reports_local_export_refusals_and_may_call() {
    // `g` (a ternary) refuses `not_callable`, `k` (no declaration) `unbound`, `broken`'s
    // file `parse_recovery` (a brace inside an error); `h` is written (may-call); `f` is
    // proven and counts nowhere.
    let dir = tempfile::tempdir().unwrap();
    let lib = "function f() {\n  return 1;\n}\nconst g = f ? 1 : 2;\nlet h = () => 1;\nh = f;\n\
        export { f, g, h, k };\n";
    std::fs::write(dir.path().join("lib.ts"), lib).unwrap();
    let broken =
        "export function f() {\n  return 1;\n}\nfunction b() {\n  return <div>{</div>;\n}\n";
    std::fs::write(dir.path().join("broken.jsx"), broken).unwrap();
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
    let want = serde_json::json!({"not_callable": 1, "parse_recovery": 1, "unbound": 1});
    assert_eq!(v["js_export_local_refusals"], want);
    assert_eq!(v["js_export_local_may_call"], 1);
}
