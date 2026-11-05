//! S2 syntax scope facts for the contribution lane: names declared in a
//! callable, and the file-level facts that gate receiver calls.
use crate::ast::ParsedFile;
use std::collections::BTreeSet;
use tree_sitter::Node;

/// Names a callable declares for itself: parameters, `:=`, `var`/`const`,
/// range and receive clauses and type-switch aliases. Nested function
/// literals declare for themselves and are not entered (the literal passed in
/// is the root of the scan). Flow-insensitive, so a use before a later
/// declaration of the same name also counts as shadowed: fewer edges only.
pub(super) fn declared_names(parsed: &ParsedFile, callable: Node<'_>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    scan(parsed, callable, callable, &mut out);
    out
}

fn identifiers(parsed: &ParsedFile, node: Node<'_>, out: &mut BTreeSet<String>) {
    if node.kind() == "identifier" {
        out.insert(parsed.node_text(&node).to_owned());
        return;
    }
    if matches!(node.kind(), "expression_list" | "identifier_list") {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            identifiers(parsed, child, out);
        }
    }
}

fn scan(parsed: &ParsedFile, root: Node<'_>, node: Node<'_>, out: &mut BTreeSet<String>) {
    if node.id() != root.id() && matches!(node.kind(), "func_literal" | "function_declaration") {
        return;
    }
    match node.kind() {
        "parameter_declaration" | "variadic_parameter_declaration" | "var_spec" | "const_spec" => {
            let mut cursor = node.walk();
            for name in node.children_by_field_name("name", &mut cursor) {
                identifiers(parsed, name, out);
            }
        }
        "short_var_declaration" | "range_clause" | "receive_statement" => {
            let mut cursor = node.walk();
            let declares = node.children(&mut cursor).any(|c| c.kind() == ":=");
            if declares {
                if let Some(left) = node.child_by_field_name("left") {
                    identifiers(parsed, left, out);
                }
            }
        }
        "type_switch_statement" => {
            if let Some(alias) = node.child_by_field_name("alias") {
                identifiers(parsed, alias, out);
            }
        }
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        scan(parsed, root, child, out);
    }
}

/// File-level facts that refuse a receiver call: its import qualifiers.
/// Struct fields are not consulted here: a per-file producer cannot see the
/// package's other files (and a cached file would go stale when they change),
/// so func-typed fields are refused later over the whole repository.
#[derive(Default)]
pub(super) struct FileScope {
    pub imports: BTreeSet<String>,
}

pub(super) fn file_scope(parsed: &ParsedFile) -> FileScope {
    let mut scope = FileScope::default();
    let root = parsed.tree.root_node();
    let mut cursor = root.walk();
    for decl in root.named_children(&mut cursor) {
        if decl.kind() == "import_declaration" {
            imports(parsed, decl, &mut scope.imports);
        }
    }
    scope
}

fn imports(parsed: &ParsedFile, node: Node<'_>, out: &mut BTreeSet<String>) {
    if node.kind() == "import_spec" {
        let name = node
            .child_by_field_name("name")
            .map(|n| parsed.node_text(&n));
        let path = node
            .child_by_field_name("path")
            .map(|n| parsed.node_text(&n));
        let alias = match (name, path) {
            (Some(n), _) if n != "_" && n != "." => Some(n.to_owned()),
            (None, Some(p)) => p
                .trim_matches(|c| c == '"' || c == '`')
                .rsplit('/')
                .next()
                .map(str::to_owned),
            _ => None,
        };
        out.extend(alias);
        return;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        imports(parsed, child, out);
    }
}
