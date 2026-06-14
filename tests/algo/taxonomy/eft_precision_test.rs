use crate::common::*;
use prism::resolution::ResolutionConfidence;

fn result_includes_line(result: &SliceResult, file: &str, line: usize) -> bool {
    result.blocks.iter().any(|block| {
        block
            .file_line_map
            .get(file)
            .map_or(false, |lines| lines.contains_key(&line))
    })
}

#[test]
fn barrier_does_not_cross_report_same_name_methods() {
    let mut files = BTreeMap::new();
    files.insert(
        "src/a.rs".to_string(),
        ParsedFile::parse(
            "src/a.rs",
            "pub struct A;\nimpl A {\n    pub fn handle(&self) {\n        let _x = 1;\n    }\n}\n",
            Language::Rust,
        )
        .unwrap(),
    );
    files.insert(
        "src/b.rs".to_string(),
        ParsedFile::parse(
            "src/b.rs",
            "pub struct B;\nimpl B {\n    pub fn handle(&self) {\n        let _x = 2;\n    }\n}\n",
            Language::Rust,
        )
        .unwrap(),
    );
    files.insert(
        "src/use_b.rs".to_string(),
        ParsedFile::parse(
            "src/use_b.rs",
            "use crate::b::B;\nfn calls_b(b: B) {\n    b.handle();\n}\n",
            Language::Rust,
        )
        .unwrap(),
    );

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/a.rs".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::BarrierSlice),
        None,
    )
    .unwrap();

    assert!(
        !result_includes_line(&result, "src/use_b.rs", 3),
        "B::handle's caller must not appear in A::handle's barrier slice"
    );
}

#[test]
fn membrane_keeps_single_owner_nameonly_callback_caller() {
    let api_source = r#"
#include <stdlib.h>

int process(int *data, int len) {
    for (int i = 0; i < len; i++) {
        data[i] *= 2;
    }
    return 0;
}
"#;

    let caller_source = r#"
#include "api.h"

struct operations {
    int (*process)(int *data, int len);
};

int run_pipeline(struct operations *ops, int *data, int len) {
    int ret = ops->process(data, len);
    if (ret < 0) {
        return -1;
    }
    return 0;
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/api.c".to_string(),
        ParsedFile::parse("src/api.c", api_source, Language::C).unwrap(),
    );
    files.insert(
        "src/driver.c".to_string(),
        ParsedFile::parse("src/driver.c", caller_source, Language::C).unwrap(),
    );

    let call_graph = CallGraph::build(&files);
    let process = call_graph.functions.get("process").unwrap()[0].clone();
    let edges = call_graph.resolved_caller_edges(&process);
    assert!(
        edges.iter().any(|edge| {
            edge.caller.name == "run_pipeline"
                && edge.confidence == ResolutionConfidence::NameOnly
                && edge.call_site_line == 9
        }),
        "expected run_pipeline -> process to resolve as a single-owner NameOnly edge at the \
         ops->process call site, got {edges:?}"
    );

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/api.c".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5, 6]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
        None,
    )
    .unwrap();

    assert!(
        result_includes_line(&result, "src/driver.c", 9),
        "membrane (All) must keep the NameOnly callback call site"
    );
}
