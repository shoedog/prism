//! PR-C R1 through the binary: a field seed never reaches a sibling field; a whole-parameter
//! seed reaches every field read (accepted cost); `--algorithm taint` never links one request
//! field's source line to another field's sink line.

use assert_cmd::Command;

const HANDLER: &str = "function handler(req) {\n  sinkBody(req.body);\n  sinkHeaders(req.headers);\n  sinkQuery(req.query);\n}\n";

fn nav_reach(dir: &std::path::Path, source: &str, sink: &str) -> serde_json::Value {
    let out = Command::cargo_bin("prism")
        .unwrap()
        .args(["nav", "--no-cache", "taint-reaches", "--repo"])
        .arg(dir)
        .args(["--source", source, "--sink", sink, "--format", "json"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn nav_taint_reaches_field_seed_stays_on_its_field_and_parameter_seed_fans_out() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("h.js"), HANDLER).unwrap();

    // Field seed: line 2 (`req.body`) reaches line 2 but not the headers/query lines.
    assert_eq!(
        nav_reach(dir.path(), "h.js:2", "h.js:2")["reasoning"]["reachability"],
        "Reached"
    );
    for sink in ["h.js:3", "h.js:4"] {
        assert_ne!(
            nav_reach(dir.path(), "h.js:2", sink)["reasoning"]["reachability"],
            "Reached",
            "req.body seed must not reach {sink}"
        );
    }
    // Whole-parameter seed (the signature line): reaches every field read.
    for sink in ["h.js:2", "h.js:3", "h.js:4"] {
        assert_eq!(
            nav_reach(dir.path(), "h.js:1", sink)["reasoning"]["reachability"],
            "Reached",
            "parameter seed reaches {sink}"
        );
    }
}

#[test]
fn algorithm_taint_never_links_one_request_field_source_to_another_fields_sink() {
    let dir = tempfile::tempdir().unwrap();
    let source = "const express = require('express');\nconst { exec } = require('child_process');\nconst app = express();\nfunction handler(req, res) {\n  exec(req.query.cmd);\n  res.send(req.headers.agent);\n}\napp.get('/run', handler);\n";
    std::fs::write(dir.path().join("app.js"), source).unwrap();
    let diff = format!(
        "--- a/app.js\n+++ b/app.js\n@@ -0,0 +1,{} @@\n{}",
        source.lines().count(),
        source
            .lines()
            .map(|l| format!("+{l}\n"))
            .collect::<String>()
    );
    std::fs::write(dir.path().join("d.patch"), diff).unwrap();
    let out = Command::cargo_bin("prism")
        .unwrap()
        .arg("--repo")
        .arg(dir.path())
        .arg("--diff")
        .arg(dir.path().join("d.patch"))
        .args(["--algorithm", "taint", "--format", "json"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let descriptions: Vec<String> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["description"].as_str().unwrap().to_string())
        .collect();
    assert!(
        descriptions.contains(&"tainted value from line 5 reaches sink at line 5".to_string())
            && descriptions
                .contains(&"tainted value from line 6 reaches sink at line 6".to_string()),
        "{descriptions:?}"
    );
    for cross in [
        "tainted value from line 5 reaches sink at line 6",
        "tainted value from line 6 reaches sink at line 5",
    ] {
        assert!(
            !descriptions.iter().any(|d| d == cross),
            "cross-field finding {cross}: {descriptions:?}"
        );
    }
}
