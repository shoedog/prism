//! Delta-slice candidate edges (moved from `src/data_flow/tests/outputs.rs` by #701 S5: it calls the review layer).

use crate::ast::ParsedFile;
use crate::cpg::CpgContext;
use crate::diff::{DiffInfo, DiffInput, ModifyType};
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn delta_candidate_edges_contribute() {
    let old_repo = tempfile::tempdir().unwrap();
    std::fs::write(old_repo.path().join("a.py"), "def f():\n    return 1\n").unwrap();
    let files = BTreeMap::from([(
        "a.py".into(),
        ParsedFile::parse(
            "a.py",
            "def f():\n    x = 1\n    y = x\n    return y\n",
            Language::Python,
        )
        .unwrap(),
    )]);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "a.py".into(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2]),
        }],
    };
    let new_graph = crate::cpg::CodePropertyGraph::build(&files);
    let ctx = CpgContext::build_with_cached_cpg(&files, new_graph, None);
    let run = crate::algorithms::delta_slice::slice(&ctx, &diff, old_repo.path()).unwrap();
    assert!(
        run.blocks.iter().any(|b| b
            .file_line_map
            .get("a.py")
            .is_some_and(|lines| lines.contains_key(&4))),
        "new-flow endpoint at return line 4 must contribute"
    );
    std::fs::write(old_repo.path().join("a.py"), &files["a.py"].source).unwrap();
    let same = crate::algorithms::delta_slice::slice(&ctx, &diff, old_repo.path()).unwrap();
    assert!(same.blocks.is_empty());
    std::fs::write(old_repo.path().join("a.py"), "def f():\n    return 1\n").unwrap();
}
