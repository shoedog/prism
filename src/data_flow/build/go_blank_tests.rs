//! Blank removal may restore RD or bring fallback emissions back under its cap.
use super::{FileRefs, Producer};
use crate::ast::ParsedFile;
use crate::cpg::{FlowConfidence, RD_MAX_DEFS, RD_MAX_LINES};
use crate::languages::Language;

fn refs(source: &str, cap: Option<usize>) -> FileRefs {
    let parsed = ParsedFile::parse("boundary.go", source, Language::Go).unwrap();
    Producer::file_refs_with_cap(&parsed.path, &parsed, cap)
}

fn fingerprint(parts: &FileRefs, include_labels: bool) -> String {
    use sha2::{Digest, Sha256};
    let keep = |loc: &&crate::data_flow::VarLocation| loc.path.base != "_";
    let defs: Vec<_> = parts.defs.values().flatten().filter(keep).collect();
    let uses: Vec<_> = parts.uses.values().flatten().filter(keep).collect();
    let edges: Vec<_> = parts
        .edges
        .iter()
        .filter(|e| e.from.path.base != "_" && e.to.path.base != "_")
        .collect();
    let labels: Vec<_> = parts
        .labels
        .iter()
        .filter(|((f, t), _)| include_labels && f.path.base != "_" && t.path.base != "_")
        .collect();
    format!(
        "{:x}",
        Sha256::digest(bincode::serialize(&(defs, uses, edges, labels)).unwrap())
    )
}

fn assert_base_fingerprint(parts: &FileRefs, name: &str, labels: bool) {
    let baseline: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/go_blank/boundaries.json"
    ))
    .unwrap();
    assert_eq!(fingerprint(parts, labels), baseline[name].as_str().unwrap());
}

#[test]
fn go_blank_rd_definition_boundary() {
    let mut source = String::from("package p\nfunc sample(x int) {\n");
    for i in 0..RD_MAX_DEFS - 1 {
        source += &format!("v{i} := {i}\n");
    }
    source += "_ = 0\nsink(x)\n}\n";
    let actual = refs(&source, None);
    let defs = actual
        .defs
        .values()
        .flatten()
        .map(|loc| (&loc.path, loc.line, loc.start_byte))
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    println!(
        "RD_BOUNDARY defs={defs} edges={} stats={:?} labels={:?}",
        actual.edges.len(),
        actual.rd_function_stats,
        actual.labels.values().collect::<Vec<_>>()
    );
    println!("BOUNDARY_FINGERPRINT rd {}", fingerprint(&actual, false));
    assert_eq!(defs, RD_MAX_DEFS);
    assert_eq!(actual.rd_function_stats.functions_over_cap, 0);
    assert_eq!(actual.edges.len(), 1);
    assert_base_fingerprint(&actual, "rd", false);
    assert_eq!(
        actual.labels.values().copied().collect::<Vec<_>>(),
        vec![FlowConfidence::Exact]
    );
    // One additional real definition must still refuse RD; filtering is exactly `_`.
    let over = refs(&source.replace("_ = 0", "_value := 0"), None);
    assert_eq!(over.rd_function_stats.functions_over_cap, 1);
}

#[test]
fn go_blank_fallback_survivor_boundary() {
    let mut source = String::from("package p\nfunc sample(x int) {\n_ = 0\n");
    // A blank declaration in a nested callable is seen by the legacy outer owner
    // reference walk, so even valid Go can have blank def->use candidates.
    source += "g := func(_ int) {}\n";
    for _ in 0..RD_MAX_LINES + 1 {
        source += "sink(x)\n";
    }
    source += "sink(g)\n}\n";
    let actual = refs(&source, None);
    let count = actual.edges.len();
    let admitted = refs(&source, Some(RD_MAX_LINES + 3));
    println!(
        "FALLBACK_FIXED_CAP capped={} edges={}",
        admitted.rd_function_stats.fallback_functions_capped,
        admitted.edges.len()
    );
    println!(
        "FALLBACK_BOUNDARY edges={count} blank_edges={} stats={:?}",
        actual
            .edges
            .iter()
            .filter(|e| e.from.path.base == "_" || e.to.path.base == "_")
            .count(),
        actual.rd_function_stats
    );
    println!(
        "BOUNDARY_FINGERPRINT fallback {}",
        fingerprint(&actual, true)
    );
    assert!(!actual.edges.is_empty());
    assert_eq!(actual.rd_function_stats.functions_over_cap, 1);
    assert_eq!(count, RD_MAX_LINES + 3);
    assert!(actual
        .defs
        .values()
        .flatten()
        .all(|loc| loc.path.base != "_"));
    assert!(actual
        .uses
        .values()
        .flatten()
        .all(|loc| loc.path.base != "_"));
    // Exactly the surviving count is admitted, one below it is capped. Counts
    // include duplicates; count/replay must keep every nonblank occurrence byte.
    let capped = refs(&source, Some(count - 1));
    assert_base_fingerprint(&actual, "fallback", true);
    assert_eq!(admitted.rd_function_stats.fallback_functions_capped, 0);
    assert_eq!(capped.rd_function_stats.fallback_functions_capped, 1);
    assert_eq!(capped.rd_function_stats.fallback_edges_capped, count);
    assert!(capped.edges.is_empty());
    for candidate in [&admitted, &capped] {
        assert_eq!(
            bincode::serialize(&candidate.defs).unwrap(),
            bincode::serialize(&actual.defs).unwrap()
        );
        assert_eq!(
            bincode::serialize(&candidate.uses).unwrap(),
            bincode::serialize(&actual.uses).unwrap()
        );
    }
    assert_eq!(
        bincode::serialize(&admitted.edges).unwrap(),
        bincode::serialize(&actual.edges).unwrap()
    );
    assert_eq!(admitted.labels, actual.labels);
}

/// A removed blank-write alias twin of `err` had put a second RD def on the line that
/// defines `err`, collapsing that line to `NameOnly(SameLine)`; the kept edge is Exact now.
#[test]
fn go_blank_twin_no_longer_collapses_defining_line() {
    let source = "package p\nfunc f() (int, int, error) { return 0, 0, nil }\n\
func g() {\n r, _, err := f()\n _ = err\n use(r)\n use(err)\n}\n";
    let parts = refs(source, None);
    let mut seen = 0;
    for ((from, to), label) in &parts.labels {
        if from.path.base == "err" && from.line == 4 && to.line == 7 {
            seen += 1;
            assert_eq!(*label, FlowConfidence::Exact, "{from:?} {to:?}");
        }
        if from.path.base == "r" && from.line == 4 {
            assert_eq!(*label, FlowConfidence::Exact, "{from:?} {to:?}");
        }
    }
    assert!(seen >= 1, "expected an err def->use edge");
}
