//! The solver's edge-independent refusal predicates, in their original order.
use super::{
    deduplicate_definitions, DefSite, FileCfgEdgeIndex, RdUnavailable, RD_MAX_DEFS, RD_MAX_LINES,
};
use crate::ast::ParsedFile;
use std::collections::BTreeMap;
use tree_sitter::Node;

pub(crate) fn unavailable_before_edges(
    parsed: &ParsedFile,
    owner: &Node<'_>,
    defs: &[DefSite],
    cfg: &FileCfgEdgeIndex<'_>,
) -> Option<RdUnavailable> {
    let defs = deduplicate_definitions(defs);
    let count = defs.len();
    if count > RD_MAX_DEFS {
        return Some(RdUnavailable::DefinitionsCapExceeded { actual: count });
    }
    let (statements, _) = super::rd_statements(parsed, owner);
    if statements.len() > RD_MAX_LINES {
        return Some(RdUnavailable::StatementLinesCapExceeded {
            actual: statements.len(),
        });
    }
    let lines: BTreeMap<_, _> = statements
        .iter()
        .enumerate()
        .map(|(i, (line, _))| (*line, i + 1))
        .collect();
    // Join classification changes labels only; it never removes selected edges.
    if !cfg.has_owner_edges(&lines)
        && !crate::cfg::js_single::single_node_cfg_is_complete(
            parsed.language,
            lines.len(),
            defs.iter().map(|def| def.path.base.as_str()),
        )
    {
        return Some(RdUnavailable::NoCfgEdges);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::access_path::AccessPath;
    use crate::cfg::build_cfg_edges_with_arms;
    use crate::cpg::reaching::{reaching_definitions_with_exact, DefId, RdOutcome};
    use crate::languages::Language;

    #[test]
    fn preflight_matches_solver_refusal_order_and_nested_owners() {
        let short = "def f():\n    first()\n    second()\n".to_string();
        let long = format!("def f():\n{}", "    g()\n".repeat(RD_MAX_LINES + 1));
        let over_defs = Some(RdUnavailable::DefinitionsCapExceeded {
            actual: RD_MAX_DEFS + 1,
        });
        let over_lines = Some(RdUnavailable::StatementLinesCapExceeded {
            actual: RD_MAX_LINES + 1,
        });
        let cases = [
            ("available", short.clone(), 0, false, None),
            ("no cfg", short.clone(), 0, true, Some(RdUnavailable::NoCfgEdges)),
            ("defs before cfg", short, RD_MAX_DEFS + 1, true, over_defs),
            ("lines before cfg", long.clone(), 0, true, over_lines),
            ("defs before lines", long, RD_MAX_DEFS + 1, true, over_defs),
            ("nested", "def f():\n    first()\n    def g():\n        inner()\n        again()\n    last()\n".into(), 0, false, None),
        ];
        for (name, source, count, empty_cfg, expected) in cases {
            let parsed = ParsedFile::parse("a.py", &source, Language::Python).unwrap();
            let cfg = if empty_cfg {
                vec![]
            } else {
                build_cfg_edges_with_arms(&parsed)
            };
            let index = FileCfgEdgeIndex::new(&cfg);
            let defs: Vec<_> = (0..count)
                .map(|i| DefSite {
                    id: DefId(i as u32),
                    path: AccessPath::simple(format!("v{i}")),
                    line: 2,
                    start_byte: i,
                    alias_derived: false,
                })
                .collect();
            for owner in parsed.all_functions() {
                let outcome =
                    reaching_definitions_with_exact(&parsed, &owner, &defs, &[], &[], &index);
                let reason = match outcome {
                    RdOutcome::Available(_) => None,
                    RdOutcome::Unavailable(reason) => Some(reason),
                };
                assert_eq!(reason, expected, "solver: {name}");
                assert_eq!(
                    unavailable_before_edges(&parsed, &owner, &defs, &index),
                    reason,
                    "preflight: {name}"
                );
            }
        }
    }
}
