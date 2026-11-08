//! Follow-up to PR #390: LeftFlow's condition phase (shared with FullFlow through
//! `left_flow_core`) walked backward from `Use(p.x)` to a member-only formal's base Def
//! and then forward through every projection, so a diff on `if (p.x)` pulled in lines
//! that use a local derived from a sibling field (`const html = p.z; ... sink(html)`).
//! Pre-#390 the formal had no Def, so the condition variable took the name fallback
//! (every textual `p` line, nothing derived). The forward leg now seeds only from
//! locations reached before a projection-only Def, and a condition whose walk reaches
//! one keeps that name fallback.

use crate::languages::Language;
use std::collections::BTreeMap;

// 1. Leak shapes: each fails on d293009c (line 6 / 7 present) ------------------------

// 2. Controls: main's output is unchanged ----------------------------------------------

// 3. PR #404 round-1 fold ------------------------------------------------------------

// 4. PR #404 round-2 fold: the cut reproduces pre-#390's fallback decision -------------

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
