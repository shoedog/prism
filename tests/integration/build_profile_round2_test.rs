use serde_json::Value;
use std::path::Path;
use std::process::Command;

fn run(repo: &Path, threads: &str) -> (Vec<Value>, Vec<Value>) {
    let path = repo.join("profile.jsonl");
    let out = Command::new(env!("CARGO_BIN_EXE_prism"))
        .env("PRISM_PROFILE", &path)
        .env("RAYON_NUM_THREADS", threads)
        .args(["nav", "--no-cache", "dfg-stats", "--edges", "--repo"])
        .arg(repo)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stderr.is_empty());
    let rows = std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    (
        rows,
        String::from_utf8(out.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect(),
    )
}

#[test]
fn captured_write_admission_counts_admitted_and_refused_fences() {
    let dir = tempfile::tempdir().unwrap();
    for (write, fences, definitions, doubt) in
        [("x = 2", 4, 2, "killed"), ("let x = 2", 3, 1, "exact")]
    {
        std::fs::write(
            dir.path().join("a.js"),
            format!("consume(() => {{\n let x = 1;\n consume(() => {{ {write}; }});\n return x;\n}});\n"),
        ).unwrap();
        for threads in ["1", "4"] {
            let (rows, edges) = run(dir.path(), threads);
            let owner = rows.iter().find(|r| r["owner"] == "<anon@1:9>").unwrap();
            // Read the observable before checking the instrumentation.
            let edge = edges
                .iter()
                .find(|e| e["from"]["line"] == 2 && e["to"]["line"] == 4)
                .unwrap();
            if doubt == "killed" {
                assert_eq!(edge["doubt"], doubt);
                assert_eq!(edge["kill_line"], 3);
            } else {
                assert_eq!(edge["confidence"], "exact");
                assert!(edge.get("doubt").is_none_or(Value::is_null));
                assert!(edge.get("kill_line").is_none_or(Value::is_null));
            }
            assert_eq!(owner["definitions"], definitions);
            assert_eq!(owner["counters"]["simple_searches"], 1);
            assert_eq!(owner["counters"]["shadow_check_calls"], 3);
            assert_eq!(
                owner["counters"]["binding_fence_calls"], fences,
                "write={write} threads={threads}"
            );
        }
    }
}

#[test]
fn rss_intervals_exclude_cfg_in_both_owner_orders() {
    let dir = tempfile::tempdir().unwrap();
    let tiny = "function tiny() { return 0; }\n";
    let big: String = (0..10)
        .map(|i| {
            format!(
                "function big{i}() {{\n{}\n}}\n",
                (0..300)
                    .map(|n| format!("let v{n} = {n}; consume(v{n});\n"))
                    .collect::<String>()
            )
        })
        .collect();
    let mut rises = Vec::new();
    let mut first_cfg_rise = 0;
    for first in [true, false] {
        std::fs::write(
            dir.path().join("a.js"),
            if first {
                format!("{tiny}{big}")
            } else {
                format!("{big}{tiny}")
            },
        )
        .unwrap();
        let (rows, _) = run(dir.path(), "1");
        let tiny = rows.iter().find(|r| r["owner"] == "tiny").unwrap();
        let cfg = rows.iter().find(|r| r["name"] == "dfg.file_cfg").unwrap();
        if cfg["rss_rise_bytes"].is_null() {
            continue;
        } // Unsupported sampler.
        rises.push(tiny["rss_rise_bytes"].as_u64().unwrap());
        if first {
            let rise = cfg["rss_rise_bytes"].as_u64().unwrap();
            first_cfg_rise = rise;
            assert!(rise > 262_144, "fixture must raise CFG HWM: {cfg}");
            assert!(
                tiny["rss_rise_bytes"].as_u64().unwrap() < rise,
                "CFG charged to tiny: owner={tiny} cfg={cfg}"
            );
            assert_eq!(tiny["file_cfg_rss_rise_bytes"], rise);
        } else {
            assert_eq!(tiny["file_cfg_init_ns"], 0);
            assert_eq!(tiny["file_cfg_rss_rise_bytes"], 0);
        }
        let rd = rows.iter().find(|r| r["name"] == "dfg.file_rd").unwrap();
        let owners: Vec<_> = rows.iter().filter(|r| r["record"] == "owner").collect();
        let sum: u64 = owners
            .iter()
            .map(|r| r["rd_rss_rise_bytes"].as_u64().expect("RD-only sample"))
            .sum();
        assert_eq!(rd["rss_rise_bytes"], sum);
        for owner in owners {
            assert!(
                owner["rd_rss_rise_bytes"].as_u64().unwrap()
                    <= owner["rss_rise_bytes"].as_u64().unwrap()
            );
        }
    }
    if rises.len() == 2 {
        // Both orders remain below the file-CFG rise; moving the tiny owner
        // must never transfer that file-sized charge onto it.
        assert!(
            rises[0].max(rises[1]) < first_cfg_rise,
            "tiny first/last rises {rises:?}, CFG rise {first_cfg_rise}"
        );
    }
}

#[test]
fn call_graph_initializers_have_individual_rows() {
    let dir = tempfile::tempdir().unwrap();
    // Empty/non-JS builds still measure each initializer's actual invocation.
    for source in ["", "consume(() => { return 1; });\n"] {
        std::fs::write(dir.path().join("a.js"), source).unwrap();
        let (rows, _) = run(dir.path(), "1");
        for name in [
            "call_graph.js_ts_props",
            "call_graph.populate_scope_graph",
            "call_graph.js_anonymous_proofs",
            "call_graph.rust_macro_shadow_intersection",
        ] {
            let matching: Vec<_> = rows.iter().filter(|r| r["name"] == name).collect();
            assert_eq!(matching.len(), 1, "{name}");
            assert_eq!(matching[0]["status"], "measured");
        }
    }
}
