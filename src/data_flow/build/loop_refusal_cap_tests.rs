//! #693/#697 final refusal policy: a loop owner the plan refuses keeps EXACTLY main's availability and fallback cap
//! (it used to keep the admitted tree's, 874ce2c3, under the interim O1). Main's CFG of a one-line owner has no edge
//! (`NoCfgEdges`, 1 function without CFG, checked on main 8cc4985d's binary), and the owner is capped like any other
//! owner without a CFG. Small-cap seam as in `fallback_cap_tests`.
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
fn a_refused_loop_owner_has_mains_availability() {
    let parsed = ParsedFile::parse("a.js", REFUSED, Language::JavaScript).unwrap();
    assert!(parsed
        .js_loop_plan()
        .counters()
        .contains_key("owners_refused"));
    let r = refs(REFUSED, None);
    // Main: the one-line owner has no CFG edge (without_cfg 1), its fallback edges are drawn as for any such owner.
    assert!(owner_edges(&r, "p15") > 0);
    assert_eq!(r.rd_function_stats.functions_without_cfg, 1);
}

#[test]
fn a_refused_loop_owner_is_capped_like_main() {
    let old = refs(REFUSED, None);
    let n = owner_edges(&old, "p15");
    let capped = refs(REFUSED, Some(n - 1));
    assert_eq!(owner_edges(&capped, "p15"), 0);
    assert_eq!(capped.rd_function_stats.fallback_functions_capped, 1);
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
