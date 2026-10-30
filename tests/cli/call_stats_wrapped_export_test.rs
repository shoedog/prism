//! S1 (wrapped-export SPEC §5, T-O2): `nav call-stats` surfaces the three new keys.
use assert_cmd::Command;

#[test]
fn call_stats_reports_wrapped_export_counters() {
    // S1 T-O2 (wrapped-export SPEC §5): the three new keys, with exact values.
    let dir = tempfile::tempdir().unwrap();
    let lib = "import { memo } from 'react';\nexport const Island = memo((p: any) => null);\n\
        export const x = 1;\n";
    let app = "import { Island } from './lib';\nexport function App() {\n  Island({});\n  \
        return <Island/>;\n}\n";
    std::fs::write(dir.path().join("lib.tsx"), lib).unwrap();
    std::fs::write(dir.path().join("app.tsx"), app).unwrap();
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
    assert_eq!(v["js_export_spanned_admitted"], 1);
    let reasons = serde_json::json!({ "non_call_initializer": 1 });
    assert_eq!(v["js_export_skipped_decl_reasons"], reasons);
    assert_eq!(v["dropped_wrapped_export_non_jsx"], 1);
    assert_eq!(v["kind_exact"]["import_member"], 1);
}
