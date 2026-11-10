//! #693/#697 slice 2 fold 1: a loop tree the plan refuses (option O1) is a LABEL decision. The owner's
//! reaching-definitions availability, and with it the fallback edge cap, are those of the owner had the tree
//! been admitted (874ce2c3), not main's: main's CFG of a one-line owner has no edge (`NoCfgEdges`), the
//! admitted tree has a self-loop. Small-cap seam as in `fallback_cap_tests`.
use super::{FileRefs, Producer};
use crate::ast::ParsedFile;
use crate::languages::Language;

/// One line; the loop shares every line with its header, body and the statements around it
/// (`SharedBoundaryLine`), so the plan refuses it; the admitted tree would hold a self-loop edge.
const REFUSED: &str =
    "function p15(xs, z) { let y = 0; z = h(); for (const x of xs) { y = g(x, y, z); continue; } }\n";
/// The same one-line owner without a loop: main's own `NoCfgEdges` (two defs of `y` in one node).
const NO_LOOP: &str = "function q(xs) { let y = 0; y = g(xs, y); }\n";

fn refs(source: &str, cap: Option<usize>) -> FileRefs {
    let parsed = ParsedFile::parse("a.js", source, Language::JavaScript).unwrap();
    Producer::file_refs_with_cap(&"a.js".to_string(), &parsed, cap)
}

fn owner_edges(refs: &FileRefs, owner: &str) -> usize {
    refs.edges
        .iter()
        .filter(|e| e.from.function == owner)
        .count()
}

#[test]
fn a_refused_loop_tree_keeps_its_owner_rd_available() {
    let parsed = ParsedFile::parse("a.js", REFUSED, Language::JavaScript).unwrap();
    assert!(parsed
        .js_loop_plan()
        .counters()
        .contains_key("owners_refused"));
    let r = refs(REFUSED, None);
    assert!(owner_edges(&r, "p15") > 0);
    assert_eq!(r.rd_function_stats.functions_without_cfg, 0);
}

#[test]
fn a_refused_loop_tree_keeps_its_owner_exempt_from_the_fallback_cap() {
    let old = refs(REFUSED, None);
    let capped = refs(REFUSED, Some(0));
    assert_eq!(capped.edges, old.edges);
    assert_eq!(capped.rd_function_stats.fallback_functions_capped, 0);
    assert_eq!(capped.rd_function_stats.fallback_edges_capped, 0);
}

#[test]
fn an_owner_without_a_loop_keeps_main_no_cfg_refusal_and_its_cap() {
    let old = refs(NO_LOOP, None);
    let n = owner_edges(&old, "q");
    assert!(n > 0);
    assert_eq!(old.rd_function_stats.functions_without_cfg, 1);
    let capped = refs(NO_LOOP, Some(n - 1));
    assert_eq!(owner_edges(&capped, "q"), 0);
    assert_eq!(capped.rd_function_stats.fallback_functions_capped, 1);
}
