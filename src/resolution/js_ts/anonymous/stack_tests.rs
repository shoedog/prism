//! Public constructors must dispatch recursive file work onto large-stack workers.
use crate::{ast::ParsedFile, call_graph::CallGraph, languages::Language};
use std::{collections::BTreeMap, process::Command};

fn check(route: &'static str, test: &str) {
    if let Ok(case) = std::env::var("PRISM_COLLECT_STACK_CASE") {
        std::thread::Builder::new()
            .stack_size(256 * 1024 * 1024)
            .spawn(move || {
                assert!(rayon::current_thread_index().is_none());
                let depth = if case == "deep" { 32_768 } else { 2 };
                let source = format!(
                    "const x = {}0{}; function get() {{ return x; }}",
                    "[".repeat(depth),
                    "]".repeat(depth)
                );
                let files: BTreeMap<_, _> = [
                    ("nested.ts", source.as_str()),
                    ("shallow.ts", "export function shallow() { return 1; }"),
                ]
                .into_iter()
                .map(|(path, source)| {
                    let parsed = ParsedFile::parse(path, source, Language::TypeScript).unwrap();
                    assert!(!parsed.tree.root_node().has_error());
                    (path.to_string(), parsed)
                })
                .collect();
                eprintln!("parsed {case}; outside-pool constructor");
                assert!(rayon::current_thread_index().is_none());
                let graph = if route == "proofs" {
                    CallGraph::build_skeleton(&files)
                } else {
                    CallGraph::build(&files)
                };
                for name in ["get", "shallow"] {
                    assert!(graph.functions.contains_key(name));
                }
                eprintln!("completed {case}");
            })
            .unwrap()
            .join()
            .unwrap();
        return;
    }
    for case in ["shallow", "deep"] {
        let output = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test, "--nocapture"])
            .env("PRISM_COLLECT_STACK_CASE", case)
            .env("RAYON_NUM_THREADS", "2")
            .env_remove("RAYON_RS_NUM_CPUS")
            .env_remove("RUST_MIN_STACK")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{route}/{case}: {}\n{}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains(&format!("completed {case}")));
    }
}

#[test]
fn proofs_public_constructor_stack_safety() {
    check(
        "proofs",
        "js_anonymous::stack_tests::proofs_public_constructor_stack_safety",
    );
}

#[test]
fn facts_public_constructor_stack_safety() {
    check(
        "facts",
        "js_anonymous::stack_tests::facts_public_constructor_stack_safety",
    );
}
