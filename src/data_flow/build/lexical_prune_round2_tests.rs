//! Focused witnesses for the child window and the independent U operator guard.
use super::{lexical_prune::LexicalFacts, FileRefs, Producer};
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::cpg::RdFileStats;
use crate::languages::Language;
use std::collections::BTreeSet;

fn has_pair(parts: &FileRefs, path: &str, def: usize, use_line: usize) -> bool {
    parts.edges.iter().any(|edge| {
        edge.from.path.to_string() == path && edge.from.line == def && edge.to.line == use_line
    })
}

#[test]
fn go_prune_loop_carried_pair_inside_dominator_child_window() {
    let source =
        include_str!("../../../tests/fixtures/dfg_prune/go/19_loop_carried_child_window.go");
    let parsed = ParsedFile::parse("loop.go", source, Language::Go).unwrap();
    let before = Producer::file_refs_frozen(&parsed.path, &parsed);
    let after = Producer::file_refs(&parsed.path, &parsed);
    assert!(has_pair(&before, "x", 6, 5), "frozen loop-carried witness");
    assert!(
        has_pair(&after, "x", 6, 5),
        "D must retain x = 2 -> use(x) inside the containing loop child"
    );
    assert!(has_pair(&after, "x", 3, 5), "first-iteration write");
    assert!(has_pair(&before, "x", 2, 5), "frozen older parameter");
    assert!(
        !has_pair(&after, "x", 2, 5),
        "the preceding x = 1 must establish a D window, not keep by uncertainty"
    );
    for (key, label) in &after.labels {
        assert_eq!(before.labels.get(key), Some(label));
    }
}

#[test]
fn go_prune_compound_operator_guard_without_rvalue_facts() {
    for operator in [
        "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>=", "&^=", "=",
    ] {
        let source = format!("package p\nfunc f(x int) {{\nx = 1\nx {operator} 1\n}}\n");
        let parsed = ParsedFile::parse("operator.go", &source, Language::Go).unwrap();
        let owner = parsed.all_functions()[0];
        assert!(!owner.has_error(), "{operator}: valid Go syntax");
        let lines: BTreeSet<_> = (2..=5).collect();
        let lvalues = parsed.assignment_lvalue_spans_on_lines(&owner, &lines);
        let span = lvalues.iter().find(|span| span.line == 3).unwrap();
        // Deliberately omit rvalue facts to isolate classify_write's operator
        // certificate. The producer assertion below separately uses real facts.
        let facts = LexicalFacts::new(
            &parsed,
            owner,
            &lvalues,
            &[],
            &crate::call_graph::rust::macro_args::RustMacroCensus::of_file(&parsed),
            &crate::call_graph::c::macro_census::CMacroCensus::of_file(&parsed),
        );
        let mut stats = RdFileStats::default();
        let drop = facts.drop_write_only(&AccessPath::simple("x"), 4, &mut stats);
        assert_eq!(
            drop,
            operator == "=",
            "{operator}: independent U operator guard"
        );
        assert_eq!(stats.dfg_pruned_write_only, usize::from(operator == "="));
        assert!(facts.keep_def(&span.path, span.start_byte, span.end_byte, 4, &mut stats));
        let before = Producer::file_refs_frozen(&parsed.path, &parsed);
        let after = Producer::file_refs(&parsed.path, &parsed);
        assert!(
            has_pair(&before, "x", 3, 4),
            "{operator}: frozen incoming pair"
        );
        assert_eq!(
            has_pair(&after, "x", 3, 4),
            operator != "=",
            "{operator}: incoming pair"
        );
        assert_eq!(
            after
                .uses
                .values()
                .flatten()
                .any(|loc| loc.path == span.path && loc.line == 4),
            operator != "=",
            "{operator}: compound reads retain their Use; plain writes mint none"
        );
    }
}
