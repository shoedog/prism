//! PR #404 (follow-up to PR #390): the DFG-level query behind LeftFlow's condition phase.
//! `dfg_use_reads_projection_only_def` is true only for a `Use` fed by a projection-only base
//! Def of a member-only formal, which is how the condition walk decides to keep its name
//! fallback. The slice-level LeftFlow/FullFlow tests of the same follow-up call the review layer
//! and live in `crates/prism-review/src/algorithms/member_only_param_leftflow_slice_tests.rs`.

use crate::languages::Language;
use std::collections::BTreeMap;

// PR #404 round-2 fold: the cut reproduces pre-#390's fallback decision.

#[test]
fn projection_read_query_is_true_only_for_a_use_fed_by_a_projection_only_def() {
    // Opus r2 S2: LeftFlow only ever passes a Use, so no slice input reaches the
    // `kind != Use` guard; exercise it through the query directly.
    let src = "function h(p) {\n  if (p.x) {\n    a();\n  }\n  let n = 1;\n  use(n);\n}\n";
    let files = BTreeMap::from([(
        "m.js".to_string(),
        crate::ast::ParsedFile::parse("m.js", src, Language::JavaScript).unwrap(),
    )]);
    let ctx = crate::cpg::CpgContext::build(&files, None);
    let loc_at = |line: usize, base: &str, use_kind: bool| {
        ctx.cpg
            .nodes_at("m.js", line)
            .into_iter()
            .filter_map(|idx| ctx.cpg.to_var_location(idx))
            .find(|l| {
                l.path.base == base && (l.kind == crate::data_flow::VarAccessKind::Use) == use_kind
            })
            .unwrap_or_else(|| panic!("no {base} at line {line}"))
    };
    let base_def = loc_at(1, "p", false);
    assert!(ctx.cpg.dfg_is_projection_only_def(&base_def));
    assert!(
        !ctx.cpg.dfg_use_reads_projection_only_def(&base_def),
        "a Def is never a projection read"
    );
    assert!(ctx
        .cpg
        .dfg_use_reads_projection_only_def(&loc_at(2, "p", true)));
    assert!(!ctx
        .cpg
        .dfg_use_reads_projection_only_def(&loc_at(6, "n", true)));
}
