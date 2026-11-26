//! The solver's edge-independent refusal predicates, in their original order.
use super::{
    deduplicate_definitions, DefSite, FileCfgEdgeIndex, RdUnavailable, RD_MAX_DEFS, RD_MAX_LINES,
};
use crate::ast::ParsedFile;
use std::collections::BTreeMap;
use tree_sitter::Node;

#[cfg(test)]
thread_local! {
    static TEST_LIMITS: std::cell::Cell<(usize, usize)> =
        const { std::cell::Cell::new((RD_MAX_DEFS, RD_MAX_LINES)) };
}

#[cfg(test)]
pub(crate) fn with_availability_limits<T>(defs: usize, lines: usize, f: impl FnOnce() -> T) -> T {
    struct Restore((usize, usize));
    impl Drop for Restore {
        fn drop(&mut self) {
            TEST_LIMITS.set(self.0);
        }
    }
    let _restore = Restore(TEST_LIMITS.replace((defs, lines)));
    f()
}

fn limits() -> (usize, usize) {
    #[cfg(test)]
    return TEST_LIMITS.get();
    #[cfg(not(test))]
    (RD_MAX_DEFS, RD_MAX_LINES)
}

/// The universe an owner's RD availability is decided on, in one place for both seams (the producer preflight and
/// the solver). Since the final refusal policy a refused loop owner keeps EXACTLY main's geometry (no rewrite, no
/// admitted `do` condition), so every owner is decided on its standing universe and CFG, as on main.
pub(super) struct Availability {
    pub(super) statement_lines: usize,
    pub(super) lines: BTreeMap<usize, usize>,
}

impl Availability {
    pub(super) fn of(
        _parsed: &ParsedFile,
        _owner: &Node<'_>,
        standing: &[(usize, String)],
    ) -> Self {
        Self {
            statement_lines: standing.len(),
            lines: standing
                .iter()
                .enumerate()
                .map(|(i, (l, _))| (*l, i + 1))
                .collect(),
        }
    }

    /// Whether the universe has no CFG edge; `standing_has_edges` answers for the standing CFG.
    pub(super) fn no_edges(
        &self,
        standing_has_edges: impl FnOnce(&BTreeMap<usize, usize>) -> bool,
    ) -> bool {
        !standing_has_edges(&self.lines)
    }
}

/// The one-node census refusal counts against the S1 pair increment.
/// The ordinary preflight and solver retain their existing policies.
pub(crate) fn pattern_one_node_refusal(
    parsed: &ParsedFile,
    owner: &Node<'_>,
    defs: &[DefSite],
    cfg: &FileCfgEdgeIndex<'_>,
) -> bool {
    let (statements, _) = super::rd_statements(parsed, owner);
    let availability = Availability::of(parsed, owner, &statements);
    if availability.statement_lines != 1
        || !availability.no_edges(|lines| cfg.has_owner_edges(lines))
    {
        return false;
    }
    let (mut completed, mut census) =
        super::census_defs::complete_for(true, false, parsed, owner, deduplicate_definitions(defs));
    census.cap_defs(&mut completed, limits().0);
    let count = completed.len();
    census.one_node_admits(&mut completed, |bases| {
        crate::cfg::js_single::single_node_cfg_is_complete(parsed, 1, bases)
    }) && completed.len() != count
}

/// Whether the census-completed definition list overflows the cap although the recorded list fits:
/// the solver then forces every edge of the owner to `cfg_incomplete` (`owner-refuse:def-cap`).
pub(crate) fn census_def_cap_overflow(
    parsed: &ParsedFile,
    owner: &Node<'_>,
    defs: &[DefSite],
) -> bool {
    let (mut completed, mut census) =
        super::census_defs::complete_for(true, false, parsed, owner, deduplicate_definitions(defs));
    census.cap_defs(&mut completed, limits().0);
    census.def_cap_forced()
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
    let (max_defs, max_lines) = limits();
    if count > max_defs {
        return Some(RdUnavailable::DefinitionsCapExceeded { actual: count });
    }
    let (statements, _) = super::rd_statements(parsed, owner);
    let avail = Availability::of(parsed, owner, &statements);
    if avail.statement_lines > max_lines {
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
                    s1_entry: false,
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
                // Final refusal policy: the plan refuses this loop tree (SharedBoundaryLine), so the owner keeps
                // main's availability: main's CFG of a one-line owner has no edge (main 8cc4985d: 1 function
                // without CFG), no longer the admitted tree's self-loop.
                "refused loop tree has mains availability",
                "function f(xs) { let y = 0; for (const x of xs) { y = g(x, y); continue; } }\n",
                &["y", "y"],
                Some(RdUnavailable::NoCfgEdges),
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
                    s1_entry: false,
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
