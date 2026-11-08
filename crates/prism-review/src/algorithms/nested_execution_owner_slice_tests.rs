//! FullFlow control for nested execution owners (moved from `src/cpg/nested_execution_owner_audit_tests.rs`
//! by #701 S5: it calls the review layer).

use crate::algorithms::full_flow;
use prism::ast::ParsedFile;
use prism::cpg::CpgContext;
use prism::data_flow::DataFlowGraph;
use prism::diff::{DiffInfo, DiffInput, ModifyType};
use prism::languages::Language;
use prism::slice::{SliceConfig, SlicingAlgorithm};
use std::collections::{BTreeMap, BTreeSet};

fn normalized_slice(result: &prism::slice::SliceResult) -> Vec<String> {
    let mut rows = Vec::new();
    for block in &result.blocks {
        for (file, lines) in &block.file_line_map {
            for (line, is_diff) in lines {
                rows.push(format!("{file}:{line}:{is_diff}"));
            }
        }
    }
    rows.sort();
    rows
}
#[test]
fn nested_execution_owner_full_flow_controls() {
    let source = "function outer(seed) {\n  let before=seed;\n  register(eager, function inner(p) {\n    sink(nestedRead);\n  });\n  return before;\n}\n";
    let files = BTreeMap::from([(
        "app.js".to_string(),
        ParsedFile::parse("app.js", source, Language::JavaScript).unwrap(),
    )]);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "app.js".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow);
    let with = CpgContext::build(&files, None);
    let with_rows = normalized_slice(&full_flow::slice(&with, &diff, &config).unwrap());
    let mut without = CpgContext::build(&files, None);
    without.cpg.dfg = DataFlowGraph::empty();
    let without_rows = normalized_slice(&full_flow::slice(&without, &diff, &config).unwrap());
    println!("OWNER_FULL_FLOW with={with_rows:?} without={without_rows:?}");
    assert_eq!(with_rows, without_rows);
    assert_eq!(
        with_rows,
        vec!["app.js:3:true".to_string(), "app.js:5:false".to_string()]
    );
}
