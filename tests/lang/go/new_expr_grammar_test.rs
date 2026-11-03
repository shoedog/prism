//! Go 1.26 operands and shadowed generic builtins; fixtures are parsed, never run.
use crate::common::*;
use prism::finding_confidence::ParseQuality;

const EXPRESSIONS: &str = include_str!("../../fixtures/go_new_expr/new_expr.go");
const SHADOWED: &str = include_str!("../../fixtures/go_new_expr/shadowed.go");

fn clean(source: &str) {
    let parsed = ParsedFile::parse("new.go", source, Language::Go).unwrap();
    assert_eq!(parsed.source, source);
    let mut pending = vec![parsed.tree.root_node()];
    while let Some(node) = pending.pop() {
        assert!(!node.is_error() && !node.is_missing(), "{}", node.to_sexp());
        let mut cursor = node.walk();
        pending.extend(node.children(&mut cursor));
    }
    assert_eq!(parsed.parse_error_count, 0);
    let files = BTreeMap::from([("new.go".to_owned(), parsed)]);
    let (warnings, quality) = algorithms::check_parse_quality(&files);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(algorithms::check_parse_warnings(&files).is_empty());
    assert_eq!(
        ParseQuality::min_over(&["new.go"], &quality, &files),
        ParseQuality::Clean
    );
}

#[test]
fn go126_expression_operands_have_clean_parse_quality() {
    clean(EXPRESSIONS);
}

#[test]
fn generic_alias_preserves_name_type_and_new_parameter_field() {
    let source = "package p\ntype Alias[T any] = []T\n";
    clean(source);
    let parsed = ParsedFile::parse("alias.go", source, Language::Go).unwrap();
    let mut pending = vec![parsed.tree.root_node()];
    let mut aliases = 0;
    while let Some(node) = pending.pop() {
        if node.kind() == "type_alias" {
            aliases += 1;
            for (field, expected) in [
                ("name", "Alias"),
                ("type", "[]T"),
                ("type_parameters", "[T any]"),
            ] {
                assert_eq!(
                    parsed.node_text(&node.child_by_field_name(field).unwrap()),
                    expected
                );
            }
        }
        let mut cursor = node.walk();
        pending.extend(node.children(&mut cursor));
    }
    assert_eq!(aliases, 1);
}

#[test]
fn shadowed_generic_new_and_make_are_documented_unsupported() {
    // PR #193's special-call type_arguments hunk breaks ordinary new[i].
    // The indexed fallback may parse, but the special builtin branch has no
    // type_arguments field. Do not mistake a clean fallback for #189 support.
    let parsed = ParsedFile::parse("new.go", SHADOWED, Language::Go).unwrap();
    assert_eq!(parsed.parse_error_count, 0);
    let mut pending = vec![parsed.tree.root_node()];
    let mut indexed_new = 0;
    while let Some(node) = pending.pop() {
        if node.kind() == "call_expression" {
            let function = node.child_by_field_name("function").unwrap();
            if parsed.node_text(&function) == "new[int]" {
                assert_eq!(function.kind(), "index_expression");
                assert!(node.child_by_field_name("type_arguments").is_none());
                indexed_new += 1;
            }
        }
        let mut cursor = node.walk();
        pending.extend(node.named_children(&mut cursor));
    }
    assert_eq!(indexed_new, 1);
}

#[test]
fn malformed_new_operand_remains_a_parse_error() {
    let parsed =
        ParsedFile::parse("bad.go", "package p\nfunc f() { _ = new( }", Language::Go).unwrap();
    assert!(parsed.tree.root_node().has_error());
    assert!(parsed.parse_error_count > 0);
}

#[test]
fn new_expression_owner_gets_write_only_and_dominated_pruning() {
    let parsed = ParsedFile::parse("new.go", EXPRESSIONS, Language::Go).unwrap();
    let dfg = DataFlowGraph::build(&BTreeMap::from([("new.go".to_owned(), parsed)]));
    let line = |marker: &str| {
        EXPRESSIONS
            .lines()
            .position(|l| l.contains(marker))
            .unwrap()
            + 1
    };
    let outer = line("// outer definition");
    let old = line("// overwritten definition");
    let current = line("// dominating definition");
    let read = line("// dominated read");
    let other = line("// outer read");
    let pairs: BTreeSet<_> = dfg
        .edges()
        .iter()
        .filter(|e| e.from.function == "parse" && e.from.path.to_string() == "yyLOCAL")
        .map(|e| (e.from.line, e.to.line))
        .collect();
    println!("yyLOCAL pairs: {pairs:?}");
    assert!(!pairs.contains(&(outer, old)), "U must drop write-only use");
    assert!(
        !pairs.contains(&(outer, current)),
        "U must drop write-only use"
    );
    assert!(
        !pairs.contains(&(outer, read)),
        "D must drop outer definition"
    );
    assert!(
        !pairs.contains(&(old, read)),
        "D must drop overwritten definition"
    );
    assert!(
        pairs.contains(&(current, read)),
        "keep dominating definition"
    );
    assert!(pairs.contains(&(outer, other)), "keep read in other case");
}
