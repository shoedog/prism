//! Shared test scaffolding for the S1b-3a unit-test files (`js_binding_walk_tests.rs`,
//! `js_binding_site_tests.rs`, `js_binding_checks_tests.rs`, `js_binding_values_tests.rs`,
//! `js_binding_recovery_tests.rs`, `js_binding_writes_tests.rs`), deduplicated out of each
//! (owner 2026-09-29). `js_binding_tests.rs` (S1b-2a's own file) keeps its own copies.
use crate::ast::ParsedFile;
use crate::languages::Language;
use tree_sitter::Node;

/// Parses `src` under the grammar `path`'s extension implies (`.tsx` -> Tsx, `.ts`/`.mts` ->
/// TypeScript, otherwise JavaScript).
pub(crate) fn parse(path: &str, src: &str) -> ParsedFile {
    let lang = match path.rsplit('.').next() {
        Some("tsx") => Language::Tsx,
        Some("ts" | "mts") => Language::TypeScript,
        _ => Language::JavaScript,
    };
    ParsedFile::parse(path, src, lang).unwrap()
}

/// The `n`-th (0-indexed) `identifier`/`type_identifier` node spelled `name`, in source order:
/// the exact node a real call or JSX site would hand the walk (never a container spanning
/// extra tokens). TS class names parse as `type_identifier`; both kinds are searched so
/// occurrence counts agree across the JS and TSX grammars.
pub(crate) fn ident<'a>(p: &'a ParsedFile, name: &str, n: usize) -> Node<'a> {
    fn walk<'a>(node: Node<'a>, name: &str, text: &str, out: &mut Vec<Node<'a>>) {
        if matches!(node.kind(), "identifier" | "type_identifier")
            && &text[node.start_byte()..node.end_byte()] == name
        {
            out.push(node);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            walk(child, name, text, out);
        }
    }
    let mut out = Vec::new();
    walk(p.tree.root_node(), name, &p.source, &mut out);
    out.into_iter()
        .nth(n)
        .unwrap_or_else(|| panic!("no occurrence {n} of {name:?}"))
}

/// The smallest node spanning the first occurrence of `at` (a distinctive substring chosen to
/// resolve to exactly one meaningful node, per `js_binding_tests.rs`'s convention).
pub(crate) fn node_at<'a>(p: &'a ParsedFile, at: &str) -> Node<'a> {
    let start = p.source.find(at).unwrap_or_else(|| panic!("no {at:?}"));
    let root = p.tree.root_node();
    root.descendant_for_byte_range(start, start + at.len())
        .unwrap()
}

/// The direct error children of the program root (recovered top-level fragments).
pub(crate) fn root_errors<'a>(p: &'a ParsedFile) -> Vec<Node<'a>> {
    let root = p.tree.root_node();
    let mut cursor = root.walk();
    root.children(&mut cursor)
        .filter(|n| n.is_error())
        .collect()
}
