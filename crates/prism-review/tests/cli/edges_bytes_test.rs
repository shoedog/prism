//! `PRISM_EDGES_BYTES=1` adds occurrence bytes to both endpoints of `nav dfg-stats --edges`
//! rows (the alias-twin gate's byte-level edge identity); the default output is unchanged.
use serde_json::Value;

fn edge_rows(repo: &std::path::Path, bytes: bool) -> Vec<Value> {
    let mut command = crate::prism_bin().unwrap();
    command.args(["nav", "--no-cache", "dfg-stats", "--repo"]);
    command.arg(repo);
    command.arg("--edges");
    if bytes {
        command.env("PRISM_EDGES_BYTES", "1");
    } else {
        command.env_remove("PRISM_EDGES_BYTES");
    }
    let out = command.output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|row| row.get("from").is_some())
        .collect()
}

fn strip_bytes(mut row: Value) -> Value {
    for end in ["from", "to"] {
        let obj = row[end].as_object_mut().unwrap();
        obj.remove("start_byte");
        obj.remove("end_byte");
    }
    row
}

#[test]
fn edges_carry_bytes_only_when_asked_and_are_otherwise_identical() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("a.py"),
        "def f(x):\n    y = x\n    z = y\n    return z\n",
    )
    .unwrap();
    let plain = edge_rows(dir.path(), false);
    let with_bytes = edge_rows(dir.path(), true);
    assert!(!plain.is_empty(), "fixture must produce DFG edges");
    for row in &plain {
        for end in ["from", "to"] {
            assert!(row[end].get("start_byte").is_none() && row[end].get("end_byte").is_none());
        }
    }
    assert_eq!(plain.len(), with_bytes.len());
    for row in &with_bytes {
        for end in ["from", "to"] {
            let (s, e) = (&row[end]["start_byte"], &row[end]["end_byte"]);
            assert!(
                s.is_u64() && e.is_u64() && s.as_u64() <= e.as_u64(),
                "{row}"
            );
        }
    }
    let stripped: Vec<Value> = with_bytes.into_iter().map(strip_bytes).collect();
    assert_eq!(plain, stripped, "bytes must be the only difference");
}
