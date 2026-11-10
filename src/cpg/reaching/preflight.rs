//! The solver's edge-independent refusal predicates, in their original order.
use super::{
    deduplicate_definitions, DefSite, FileCfgEdgeIndex, RdUnavailable, RD_MAX_DEFS, RD_MAX_LINES,
};
use crate::ast::ParsedFile;
use std::collections::BTreeMap;
use tree_sitter::Node;

/// The universe an owner's RD availability is decided on (#693 fold 2, one place for both seams). A refusal
/// is a label decision only: an owner whose loop tree the plan refused (option O1) is available exactly when
/// it would be with every O1-refused tree admitted (874ce2c3's build): that universe's statement-line count
/// against `RD_MAX_LINES`, its CFG's edge check and its one-node test. Solving and labelling keep the
/// standing universe and CFG. Any other owner is decided on its standing universe, as on main.
pub(super) struct Availability {
    pub(super) statement_lines: usize,
    pub(super) lines: BTreeMap<usize, usize>,
    /// `Some`: whether the all-admitted CFG has an edge between two of `lines` (an O1-refused owner).
    admitted_edges: Option<bool>,
}

impl Availability {
    pub(super) fn of(parsed: &ParsedFile, owner: &Node<'_>, standing: &[(usize, String)]) -> Self {
        let index = |s: &[(usize, String)]| -> BTreeMap<usize, usize> {
            s.iter()
                .enumerate()
                .map(|(i, (l, _))| (*l, i + 1))
                .collect()
        };
        if parsed.language.is_js_family() && !crate::cfg::js_loops::main_loop_cfg() {
            let plan = parsed.js_loop_plan();
            if plan.refuses_owner((owner.start_byte(), owner.end_byte())) {
                let (all, _) =
                    plan.with_all_admitted(parsed, || super::rd_statements(parsed, owner));
                let lines = index(&all);
                let edges = plan.admitted_cfg_has_edges(parsed, &lines);
                return Self {
                    statement_lines: all.len(),
                    lines,
                    admitted_edges: Some(edges),
                };
            }
        }
        Self {
            statement_lines: standing.len(),
            lines: index(standing),
            admitted_edges: None,
        }
    }

    /// Whether the universe has no CFG edge; `standing_has_edges` answers for the standing CFG.
    pub(super) fn no_edges(
        &self,
        standing_has_edges: impl FnOnce(&BTreeMap<usize, usize>) -> bool,
    ) -> bool {
        !self
            .admitted_edges
            .unwrap_or_else(|| standing_has_edges(&self.lines))
    }
}

pub(crate) fn unavailable_before_edges(
    parsed: &ParsedFile,
    owner: &Node<'_>,
    defs: &[DefSite],
    cfg: &FileCfgEdgeIndex<'_>,
) -> Option<RdUnavailable> {
    // The census never changes availability (#660 S1b-1): this is main's preflight on recorded defs.
    let defs = deduplicate_definitions(defs);
    let count = defs.len();
    if count > RD_MAX_DEFS {
        return Some(RdUnavailable::DefinitionsCapExceeded { actual: count });
    }
    let (statements, _) = super::rd_statements(parsed, owner);
    let avail = Availability::of(parsed, owner, &statements);
    if avail.statement_lines > RD_MAX_LINES {
        return Some(RdUnavailable::StatementLinesCapExceeded {
            actual: avail.statement_lines,
        });
    }
    // Join classification changes labels only; it never removes selected edges.
    if avail.no_edges(|lines| cfg.has_owner_edges(lines))
        && !crate::cfg::js_single::single_node_cfg_is_complete(
            parsed,
            avail.lines.len(),
            &defs
                .iter()
                .map(|def| def.path.base.as_str())
                .collect::<Vec<_>>(),
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

    #[test]
    fn preflight_matches_solver_for_one_node_js_owners() {
        let cases = [
            (
                "available",
                "function f(c) {\n  exec(c);\n}\n",
                &["c"][..],
                None,
            ),
            (
                "expression arrow",
                "const f = (c) =>\n  exec(c);\n",
                &["c"],
                None,
            ),
            (
                "duplicate base",
                "function f(c) {\n  exec(c);\n}\n",
                &["c", "c"],
                Some(RdUnavailable::NoCfgEdges),
            ),
            (
                // #660 S1d-2: the update is the census's to label, not an availability refusal.
                "update is not a hazard",
                "function f(c) {\n  return (c++, exec(c));\n}\n",
                &["c"],
                None,
            ),
            (
                // #693 fold 1: the plan refuses this loop tree (SharedBoundaryLine) and main's CFG has no
                // edge, but refusal is a label decision: availability is the admitted tree's (a self-loop).
                "refused loop tree keeps availability",
                "function f(xs) { let y = 0; for (const x of xs) { y = g(x, y); continue; } }\n",
                &["y", "y"],
                None,
            ),
            (
                "empty body",
                "function f(c) { }\n",
                &["c"],
                Some(RdUnavailable::NoCfgEdges),
            ),
        ];
        for (name, source, bases, expected) in cases {
            let parsed = ParsedFile::parse("a.js", source, Language::JavaScript).unwrap();
            let cfg = build_cfg_edges_with_arms(&parsed);
            let index = FileCfgEdgeIndex::new(&cfg);
            let defs: Vec<_> = bases
                .iter()
                .enumerate()
                .map(|(i, base)| DefSite {
                    id: DefId(i as u32),
                    path: AccessPath::simple(*base),
                    line: 1,
                    start_byte: i,
                    alias_derived: false,
                })
                .collect();
            let owner = parsed.all_functions().into_iter().next().unwrap();
            let outcome = reaching_definitions_with_exact(&parsed, &owner, &defs, &[], &[], &index);
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
