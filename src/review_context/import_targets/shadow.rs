//! Python and Go: is the call's root name bound in a nearer scope than the
//! file's imports? Built on the parser's function-local binding collector, which
//! over-collects (any binding anywhere in the function counts) and refuses
//! (`None`) on a function with parse errors; both read as "bound", so a
//! doubtful site is `Unknown` rather than a misattributed import.
//! JS/TS uses `ParsedFile::js_ts_call_root_scope` instead.
use crate::ast::ParsedFile;
use crate::call_graph::CallSite;
use std::collections::BTreeSet;
use tree_sitter::Node;

fn call_node<'a>(parsed: &'a ParsedFile, call: &CallSite) -> Option<Node<'a>> {
    parsed
        .tree
        .root_node()
        .descendant_for_byte_range(call.start_byte, call.end_byte)
}

/// True when a function or lambda enclosing the call binds
/// `name` (a parameter, an assignment target, a loop or comprehension variable, a
/// nested definition, a function-level import). Class bodies are not searched: a
/// class scope is invisible to the functions inside it.
pub(super) fn py_scope_binds(parsed: &ParsedFile, call: &CallSite, name: &str) -> bool {
    let Some(mut node) = call_node(parsed, call) else {
        return true;
    };
    loop {
        match node.kind() {
            "function_definition" | "lambda"
                if parsed
                    .function_local_value_bindings(&node)
                    .is_none_or(|names| names.contains(name)) =>
            {
                return true;
            }
            _ => {}
        }
        match node.parent() {
            Some(parent) => node = parent,
            None => return false,
        }
    }
}

/// True when module scope binds `name` other than by an import: an assignment,
/// a loop or `with`/`except` target, a walrus, a `def` or `class`. Flow-
/// insensitive; function bodies are not entered.
pub(super) fn py_module_rebinds(parsed: &ParsedFile, name: &str) -> bool {
    module_binds(parsed, parsed.tree.root_node(), name)
}

fn module_binds(parsed: &ParsedFile, node: Node<'_>, name: &str) -> bool {
    let binds = |field: &str| {
        node.child_by_field_name(field)
            .is_some_and(|target| pattern_binds(parsed, target, name))
    };
    match node.kind() {
        "import_statement" | "import_from_statement" | "lambda" => return false,
        "function_definition" | "class_definition" => {
            return node
                .child_by_field_name("name")
                .is_some_and(|n| parsed.node_text(&n) == name);
        }
        "assignment" | "augmented_assignment" | "for_statement" if binds("left") => return true,
        "named_expression" if binds("name") => return true,
        "as_pattern" if binds("alias") => return true,
        "except_clause" | "except_group_clause" => {
            let mut cursor = node.walk();
            let as_target = node.named_children(&mut cursor).any(|c| {
                c.prev_sibling().is_some_and(|p| p.kind() == "as") && pattern_binds(parsed, c, name)
            });
            if as_target {
                return true;
            }
        }
        _ => {}
    }
    let mut cursor = node.walk();
    let found = node
        .children(&mut cursor)
        .any(|child| module_binds(parsed, child, name));
    found
}

/// Identifiers a binding target introduces: plain names and the usual unpacking
/// shapes; attribute and subscript targets bind nothing.
fn pattern_binds(parsed: &ParsedFile, node: Node<'_>, name: &str) -> bool {
    match node.kind() {
        "identifier" => parsed.node_text(&node) == name,
        "pattern_list"
        | "tuple_pattern"
        | "list_pattern"
        | "list_splat_pattern"
        | "as_pattern_target"
        | "expression_list"
        | "parenthesized_expression" => {
            let mut cursor = node.walk();
            let found = node
                .named_children(&mut cursor)
                .any(|child| pattern_binds(parsed, child, name));
            found
        }
        _ => false,
    }
}

/// True when a function, method or function literal enclosing the call binds
/// `name` (a parameter, receiver, `:=`, `var`, or range variable).
pub(super) fn go_scope_binds(parsed: &ParsedFile, call: &CallSite, name: &str) -> bool {
    let Some(mut node) = call_node(parsed, call) else {
        return true;
    };
    loop {
        if matches!(
            node.kind(),
            "function_declaration" | "method_declaration" | "func_literal"
        ) && parsed
            .function_local_value_bindings(&node)
            .is_none_or(|names: BTreeSet<String>| names.contains(name))
        {
            return true;
        }
        match node.parent() {
            Some(parent) => node = parent,
            None => return false,
        }
    }
}
