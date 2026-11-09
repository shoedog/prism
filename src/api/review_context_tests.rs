use super::*;
use crate::api::{load_review_inputs, ReviewOptions};
use crate::call_graph::CallGraph;

fn inputs(files: &[(&str, &str)], loaded: &[&str]) -> (tempfile::TempDir, ReviewInputs) {
    let dir = tempfile::tempdir().unwrap();
    for (name, source) in files {
        let path = dir.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, source).unwrap();
    }
    let diff = serde_json::json!({"files": loaded.iter().map(|file| serde_json::json!({
        "file_path": file, "modify_type": "Modified", "diff_lines": []
    })).collect::<Vec<_>>()});
    let inputs = load_review_inputs(&ReviewOptions::new(dir.path()), &diff.to_string()).unwrap();
    (dir, inputs)
}

#[test]
fn review_hints_bind_captured_listing_and_exact_argument_spans() {
    let source = "import { f } from './lib';\nfunction h(x) { f('safe'); f(\n wrap(x)); }\n";
    let (dir, inputs) = inputs(&[("app.js", source), ("lib.js", "")], &["app.js"]);
    // Removing a listed file cannot cause a rewalk to erase the JS hint.
    std::fs::remove_file(dir.path().join("lib.js")).unwrap();
    let graph = CallGraph::build(&inputs.files);
    let calls: Vec<_> = graph
        .calls
        .values()
        .flatten()
        .filter(|c| c.callee_name == "f")
        .collect();
    assert_eq!(calls.len(), 2);
    let mut hints = ReviewBoundaryHints::new(&inputs).unwrap();
    for call in &calls {
        assert!(
            matches!(hints.hint_for_call(call), Some(ReviewImportHint::InRepo { files, basis: ReviewHintBasis::Bound, .. }) if files == ["lib.js"])
        );
    }
    let texts: Vec<_> = calls
        .iter()
        .map(|call| {
            hints
                .argument_spans(call)
                .into_iter()
                .map(|r| &source[r])
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(texts, [vec!["'safe'"], vec!["wrap(x)"]]);
    let mut forged = calls[0].clone();
    forged.start_byte += 1;
    assert!(hints.argument_spans(&forged).is_empty());
}

#[test]
fn review_hints_keep_unknown_loaded_and_external_silent_and_missing_visible() {
    let (_dir, inputs) = inputs(&[("app.js", "import { f } from './lib';\nimport { g } from './late';\nimport { exec } from 'child_process';\nfunction h(x) { f(x); g(x); exec(x); unknown(x); }"), ("lib.js", "export function f(x) {}")], &["app.js", "lib.js"]);
    let graph = CallGraph::build(&inputs.files);
    let mut hints = ReviewBoundaryHints::new(&inputs).unwrap();
    let mut count = 0;
    for call in graph.calls.values().flatten() {
        if call.callee_name == "g" {
            assert!(matches!(
                hints.hint_for_call(call),
                Some(ReviewImportHint::Unresolvable {
                    why: ReviewUnresolvedReason::NoFile,
                    ..
                })
            ));
            count += 1;
        } else {
            assert_eq!(hints.hint_for_call(call), None);
        }
    }
    assert_eq!(count, 1);
}

#[test]
fn review_hints_go_declarations_are_live_but_candidate_listing_is_captured() {
    let (dir, inputs) = inputs(
        &[
            ("go.mod", "module example.com/app\n"),
            (
                "main.go",
                "package main\nimport \"example.com/app/util\"\nfunc h(x string) { util.Run(x) }",
            ),
            ("util/a.go", "package util\nfunc Run(x string) {}"),
            ("util/b.go", "package util\nfunc Other() {}"),
        ],
        &["main.go"],
    );
    std::fs::write(
        dir.path().join("util/a.go"),
        "package util\nfunc Other() {}",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("util/b.go"),
        "package util\nfunc Run(x string) {}",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("util/late.go"),
        "package util\nfunc Run(x string) {}",
    )
    .unwrap();
    let graph = CallGraph::build(&inputs.files);
    let call = graph
        .calls
        .values()
        .flatten()
        .find(|c| c.callee_name == "Run")
        .unwrap();
    let mut hints = ReviewBoundaryHints::new(&inputs).unwrap();
    assert!(
        matches!(hints.hint_for_call(call), Some(ReviewImportHint::InRepo { files, declaring, .. }) if files == ["util/a.go", "util/b.go"] && declaring == ["util/b.go"])
    );
}

#[test]
fn review_hints_refuse_an_unavailable_listing() {
    let (_dir, mut inputs) = inputs(&[("app.js", "")], &["app.js"]);
    inputs.source_listing = Err(crate::repo_loader::ListingError {
        dir: String::new(),
        message: "unavailable".into(),
    });
    assert!(ReviewBoundaryHints::new(&inputs).is_err());
}

#[test]
fn review_hints_bind_relative_root_before_cwd_changes() {
    const CHILD: &str = "PRISM_REVIEW_HINT_ROOT_CHILD";
    if let Some(alternate) = std::env::var_os(CHILD) {
        let inputs = load_review_inputs(
            &ReviewOptions::new("original"),
            r#"{"files":[{"file_path":"main.go","modify_type":"Modified","diff_lines":[] }]}"#,
        )
        .unwrap();
        // This branch is a single selected test in its own process. The main
        // test runner never changes its working directory.
        std::env::set_current_dir(alternate).unwrap();
        let graph = CallGraph::build(&inputs.files);
        let call = graph
            .calls
            .values()
            .flatten()
            .find(|c| c.callee_name == "Run")
            .unwrap();
        let found = ReviewBoundaryHints::new(&inputs)
            .unwrap()
            .hint_for_call(call);
        assert!(
            matches!(&found, Some(ReviewImportHint::InRepo { declaring, .. }) if declaring == &["util/a.go"]),
            "original Run declaration required, got {found:?}"
        );
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    for (path, source) in [
        ("original/go.mod", "module example.com/app\n"),
        (
            "original/main.go",
            "package main\nimport \"example.com/app/util\"\nfunc h(x string) { util.Run(x) }",
        ),
        ("original/util/a.go", "package util\nfunc Run(x string) {}"),
        (
            "alternate/original/util/a.go",
            "package util\nfunc Other(x string) {}",
        ),
    ] {
        let file = dir.path().join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, source).unwrap();
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "api::review_context::tests::review_hints_bind_relative_root_before_cwd_changes",
            "--nocapture",
        ])
        .current_dir(dir.path())
        .env(CHILD, dir.path().join("alternate"))
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("running 1 test"), "{stdout}");
    assert!(
        output.status.success(),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
