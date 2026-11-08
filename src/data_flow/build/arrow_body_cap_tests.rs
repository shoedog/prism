//! #759: an owner the one-node belt newly refuses (two defs of one name in the nested body of an
//! expression-bodied arrow) is RD-unavailable on head, so the fallback edge cap applies to it as it does
//! to every RD-unavailable owner; on main its multi-statement universe was available and exempt. "Edge
//! sets identical" therefore holds below the cap (1,000,000 surviving raw emissions). Small-cap seam as
//! in `fallback_cap_tests`.
use super::{FileRefs, Producer};
use crate::ast::ParsedFile;
use crate::languages::Language;

/// The nested body defines `t` twice, so the outer owner's one node trips the distinct-bases belt.
const BELT: &str =
    "const outer = (k) =>\n  () => {\n    var t = src();\n    t = clean(t);\n    sink(t);\n  };\n";
/// Distinct names only: the one node is complete, so the outer owner stays RD-available.
const AVAILABLE: &str = "const outer = (k) =>\n  () => {\n    var t = src();\n    sink(t);\n  };\n";

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
fn a_belt_refused_arrow_owner_is_cap_eligible() {
    let old = refs(BELT, None);
    let outer = owner_edges(&old, "outer");
    assert!(outer > 1, "{outer}");
    // At the cap and above it, nothing moves.
    for cap in [outer, outer + 1] {
        let at = refs(BELT, Some(cap));
        assert_eq!(at.edges, old.edges, "cap {cap}");
        assert_eq!(at.rd_function_stats.fallback_functions_capped, 0);
    }
    // One below: the owner's emissions are dropped, the nested callable's own are kept.
    let capped = refs(BELT, Some(outer - 1));
    assert_eq!(owner_edges(&capped, "outer"), 0);
    assert_eq!(
        owner_edges(&capped, "<anon@2:3>"),
        owner_edges(&old, "<anon@2:3>")
    );
    assert_eq!(capped.rd_function_stats.fallback_functions_capped, 1);
    assert_eq!(capped.rd_function_stats.fallback_edges_capped, outer);
    // The refused owner is a `functions_without_cfg` owner with or without the cap (main's multi-statement
    // universe made it RD-available: 0); the cap does not count it twice.
    assert_eq!(old.rd_function_stats.functions_without_cfg, 1);
    assert_eq!(capped.rd_function_stats.functions_without_cfg, 1);
}

#[test]
fn an_available_arrow_owner_stays_exempt_from_the_cap() {
    let old = refs(AVAILABLE, None);
    assert!(owner_edges(&old, "outer") > 0);
    assert_eq!(old.rd_function_stats.functions_without_cfg, 0);
    let capped = refs(AVAILABLE, Some(0));
    assert_eq!(capped.edges, old.edges);
    assert_eq!(capped.rd_function_stats.fallback_functions_capped, 0);
}
