use super::*;

#[test]
fn repeated_phase_instances_are_additive_and_distinct() {
    const CHILD: &str = "PRISM_PHASE_INSTANCE_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let session = Session::start();
        for _ in 0..2 {
            drop(Phase::serial("call_graph"));
            drop(Phase::file("dfg.file", "a.js"));
        }
        drop(Phase::file("dfg.file", "b.js"));
        drop(session);
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("profile.jsonl");
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "profile::deferred_tests::repeated_phase_instances_are_additive_and_distinct",
        ])
        .env(CHILD, "1")
        .env("PRISM_PROFILE", &path)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let text = std::fs::read_to_string(path).unwrap();
    let rows: Vec<serde_json::Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    for (line, row) in text.lines().zip(&rows) {
        if row["record"] == "phase" {
            assert!(
                line.starts_with("{\"record\":\"phase\","),
                "phase field order: {line}"
            );
            assert!(line.ends_with(&format!(",\"instance\":{}}}", row["instance"])));
        }
    }
    assert_eq!(rows[0]["schema_version"], 1);
    for (name, file, expected) in [
        ("call_graph", None, vec![1, 2]),
        ("dfg.file", Some("a.js"), vec![1, 2]),
        ("dfg.file", Some("b.js"), vec![1]),
        ("cache.load", None, vec![1]),
    ] {
        let instances: Vec<_> = rows
            .iter()
            .filter(|r| r["name"] == name && r["file"].as_str() == file)
            .map(|r| r["instance"].as_u64().expect("additive instance field"))
            .collect();
        assert_eq!(instances, expected);
    }
}

#[test]
fn initializer_stage_boundaries_precede_builders() {
    let source = include_str!("../call_graph_profile_initializers.rs");
    let mut previous = 0;
    for (name, builder) in [
        ("js_ts_props", "let props = crate::js_ts_props::collect(files);"),
        ("populate_scope_graph", "let scope = CallGraph::populate_scope_graph(files, scope_inputs);"),
        ("js_anonymous_proofs", "let proofs = crate::js_anonymous::ProofIndex::build(files);"),
        ("rust_macro_shadow_intersection", "let macro_shadow = crate::rust_macro_args::transparent_shadow_intersection(macro_shadow);"),
        ("struct_init", "    Initializers {"),
    ] {
        let stage = source.find(&format!("profile.next(\"call_graph.{name}\");")).unwrap();
        let work = source.find(builder).unwrap();
        assert!(previous < stage && stage < work, "{name} must start after previous work and before its own work");
        previous = work;
    }
    assert!(
        !source[previous..].contains("profile.next("),
        "struct_init covers only the literal"
    );
}

#[test]
fn renderer_regressions() {
    let output = std::process::Command::new("python3")
        .args([
            "-m",
            "unittest",
            "discover",
            "-s",
            "scripts/perf",
            "-p",
            "test_summarize_profile.py",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let count = stderr
        .lines()
        .find_map(|line| {
            let words: Vec<_> = line.split_whitespace().collect();
            (words.first() == Some(&"Ran") && words.get(2) == Some(&"tests"))
                .then(|| words.get(1)?.parse::<usize>().ok())
                .flatten()
        })
        .expect("unittest must report its executed test count");
    assert!(
        count >= 4,
        "expected at least four renderer tests: {stderr}"
    );
}
