//! Conservative negative filter, and exact semantic-scope identities.
//! No positive binding verdict is derived from this inventory.
use super::super::{is_js_ts_function_like, ParsedFile};
use std::collections::{BTreeSet, HashMap};
use tree_sitter::Node;

pub(super) struct Inventory {
    pub scopes: HashMap<usize, usize>,
    pub names: BTreeSet<String>,
    pub recovery: bool,
}

fn boundary(p: &ParsedFile, node: Node<'_>) -> bool {
    is_js_ts_function_like(node.kind())
        || p.language.is_scope_block(node.kind())
        || matches!(
            node.kind(),
            "for_statement"
                | "for_in_statement"
                | "for_of_statement"
                | "for_await_statement"
                | "switch_statement"
                | "switch_body"
                | "catch_clause"
                | "class"
                | "class_declaration"
                | "abstract_class_declaration"
                | "internal_module"
                | "module"
        )
}

impl Inventory {
    pub(super) fn new(p: &ParsedFile) -> Self {
        let root = p.tree.root_node();
        let mut out = Self {
            scopes: HashMap::new(),
            names: BTreeSet::new(),
            recovery: false,
        };
        let mut stack = vec![(root, root.id())];
        while let Some((node, enclosing)) = stack.pop() {
            let scope = if boundary(p, node) {
                node.id()
            } else {
                enclosing
            };
            out.recovery |= node.is_error() || node.is_missing();
            if node.kind() == "identifier" {
                // These are the only spellings consumed by the ambient guard.
                // Unknown/foreign nodes retain the uncached predicate.
                if matches!(p.node_text(&node), "module" | "exports" | "require") {
                    out.scopes.insert(node.id(), scope);
                }
            }
            if is_js_ts_function_like(node.kind()) {
                p.collect_js_ts_parameter_bindings(node, &mut out.names);
            }
            // Only declarations use this accessor in the old reaching walk.
            // Avoid inferred display-name parent searches for anonymous callables.
            if matches!(
                node.kind(),
                "function_declaration" | "generator_function_declaration"
            ) {
                if let Some(name) = p.language.function_name(&node) {
                    out.names.insert(p.node_text(&name).to_owned());
                }
            }
            match node.kind() {
                "variable_declarator" => {
                    if let Some(name) = node.child_by_field_name("name") {
                        p.collect_js_ts_binding_pattern_names(name, &mut out.names);
                    }
                }
                "catch_clause" => p.collect_js_ts_catch_binding_names(node, &mut out.names),
                "for_in_statement" | "for_of_statement" | "for_await_statement" => {
                    if let Some(left) = node.child_by_field_name("left") {
                        p.collect_js_ts_binding_pattern_names(left, &mut out.names);
                    }
                }
                "import_alias" => {
                    if let Some(name) = node.named_child(0) {
                        out.names.insert(p.node_text(&name).to_owned());
                    }
                }
                _ => {}
            }
            // Explicit source names include named function expressions, classes,
            // enum/signature declarations and namespace identifiers. Extra names
            // only disable the negative filter; they cannot establish a binding.
            if let Some(name) = node.child_by_field_name("name") {
                out.names.insert(p.node_text(&name).to_owned());
            }
            let mut cursor = node.walk();
            stack.extend(node.children(&mut cursor).map(|child| (child, scope)));
        }
        out
    }
}
