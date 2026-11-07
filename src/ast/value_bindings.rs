//! Function-local value bindings (Python, Go and the shared dispatch).

use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::BTreeSet;
use tree_sitter::Node;

impl ParsedFile {
    /// Conservative function-local value bindings for callback value-reference
    /// resolution. `None` means this language or syntax is not modeled strongly
    /// enough to prove that a plain identifier is free.
    pub fn function_local_value_bindings(&self, func_node: &Node<'_>) -> Option<BTreeSet<String>> {
        self.function_local_value_bindings_at(func_node, None)
    }

    /// As `function_local_value_bindings`, but with `site` (a byte range) a
    /// Python comprehension target counts only when its comprehension contains
    /// the site: the target is local to the comprehension, not to the function.
    /// `None` keeps the function-wide over-collection.
    pub(crate) fn function_local_value_bindings_at(
        &self,
        func_node: &Node<'_>,
        site: Option<(usize, usize)>,
    ) -> Option<BTreeSet<String>> {
        fn contains_recovery(node: Node<'_>) -> bool {
            if node.is_error() || node.is_missing() {
                return true;
            }
            let mut cursor = node.walk();
            let found = node.children(&mut cursor).any(contains_recovery);
            found
        }

        if contains_recovery(*func_node) {
            return None;
        }
        if matches!(
            self.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) {
            return Some(self.js_ts_function_local_bindings(func_node));
        }
        if !matches!(self.language, Language::Python | Language::Go) {
            return None;
        }

        let mut out: BTreeSet<String> = self
            .function_parameter_occurrences(func_node)
            .into_iter()
            .map(|(name, _, _)| name)
            .collect();
        let root_key = (
            func_node.start_byte(),
            func_node.end_byte(),
            func_node.kind_id(),
        );
        self.collect_python_go_local_value_bindings(*func_node, root_key, site, &mut out);
        Some(out)
    }

    fn collect_python_go_local_value_bindings(
        &self,
        node: Node<'_>,
        root_key: (usize, usize, u16),
        site: Option<(usize, usize)>,
        out: &mut BTreeSet<String>,
    ) {
        let is_root = (node.start_byte(), node.end_byte(), node.kind_id()) == root_key;
        if !is_root
            && matches!(
                node.kind(),
                "function_definition"
                    | "lambda"
                    | "function_declaration"
                    | "method_declaration"
                    | "func_literal"
            )
        {
            if node.kind() == "function_definition" {
                if let Some(name) = node.child_by_field_name("name") {
                    out.insert(self.node_text(&name).to_string());
                }
            }
            return;
        }

        match (self.language, node.kind()) {
            (Language::Python, "assignment" | "augmented_assignment") => {
                if let Some(left) = node.child_by_field_name("left") {
                    self.collect_local_binding_pattern(left, out);
                }
            }
            (Language::Python, "named_expression") => {
                if let Some(name) = node.child_by_field_name("name") {
                    self.collect_local_binding_pattern(name, out);
                }
            }
            (Language::Python, "for_statement" | "for_in_clause") => {
                // A comprehension target is local to its comprehension.
                let visible = node.kind() != "for_in_clause"
                    || site.is_none_or(|(start, end)| {
                        node.parent()
                            .is_none_or(|c| c.start_byte() <= start && end <= c.end_byte())
                    });
                if let Some(left) = node.child_by_field_name("left").filter(|_| visible) {
                    self.collect_local_binding_pattern(left, out);
                }
            }
            (Language::Python, "as_pattern") => {
                if let Some(alias) = node.child_by_field_name("alias") {
                    self.collect_local_binding_pattern(alias, out);
                }
            }
            (Language::Python, "except_clause" | "except_group_clause") => {
                if let Some(alias) = node.child_by_field_name("alias") {
                    self.collect_local_binding_pattern(alias, out);
                } else {
                    // tree-sitter-python exposes the regular `except` alias as
                    // a field, but the `except*` shape has no named fields.
                    let mut cursor = node.walk();
                    for child in node.named_children(&mut cursor) {
                        if child.prev_sibling().is_some_and(|prev| prev.kind() == "as") {
                            self.collect_local_binding_pattern(child, out);
                        }
                    }
                }
            }
            (Language::Python, "delete_statement") => {
                let mut cursor = node.walk();
                for target in node.named_children(&mut cursor) {
                    self.collect_local_binding_pattern(target, out);
                }
            }
            (Language::Python, "type_alias_statement") => {
                if let Some(left) = node.child_by_field_name("left") {
                    // A generic alias may wrap the bound name with its type
                    // parameters. Over-collecting those identifiers is the
                    // fail-closed choice for imported-root authority.
                    self.collect_identifier_names(left, out);
                }
            }
            (Language::Python, "import_statement" | "import_from_statement") => {
                // Python imports inside a function bind names in that function's
                // local namespace. Collecting every identifier is deliberately
                // conservative: an extra local is safer than an omitted binding.
                self.collect_identifier_names(node, out);
            }
            (Language::Python, "case_pattern") => {
                // Capture patterns bind locals, while value/class patterns can
                // also contain identifiers. Over-collecting the latter is the
                // fail-closed choice for callback identity.
                self.collect_identifier_names(node, out);
            }
            (Language::Python, "class_definition") if !is_root => {
                if let Some(name) = node.child_by_field_name("name") {
                    out.insert(self.node_text(&name).to_string());
                }
                return;
            }
            (Language::Go, "short_var_declaration") => {
                if let Some(left) = node.child_by_field_name("left") {
                    self.collect_local_binding_pattern(left, out);
                }
            }
            (Language::Go, "parameter_declaration" | "variadic_parameter_declaration") => {
                let type_start = node
                    .child_by_field_name("type")
                    .map_or(node.end_byte(), |ty| ty.start_byte());
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    if child.start_byte() >= type_start {
                        break;
                    }
                    if child.kind() == "identifier" {
                        self.collect_local_binding_pattern(child, out);
                    }
                }
            }
            (Language::Go, "var_spec") => {
                let value_start = node
                    .child_by_field_name("value")
                    .map_or(node.end_byte(), |value| value.start_byte());
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    if child.start_byte() >= value_start {
                        break;
                    }
                    if child.kind() == "identifier" {
                        self.collect_local_binding_pattern(child, out);
                    }
                }
            }
            (Language::Go, "range_clause") => {
                let text = self.node_text(&node);
                if text.contains(":=") {
                    if let Some(left) = node.child_by_field_name("left") {
                        self.collect_local_binding_pattern(left, out);
                    }
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_python_go_local_value_bindings(child, root_key, site, out);
        }
    }

    fn collect_local_binding_pattern(&self, node: Node<'_>, out: &mut BTreeSet<String>) {
        match node.kind() {
            "identifier" => {
                let name = self.node_text(&node);
                if name != "_" {
                    out.insert(name.to_string());
                }
            }
            "pattern_list"
            | "tuple_pattern"
            | "list_pattern"
            | "list_splat_pattern"
            | "as_pattern_target"
            | "expression_list"
            | "parenthesized_expression" => {
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    self.collect_local_binding_pattern(child, out);
                }
            }
            _ => {}
        }
    }

    pub(super) fn collect_identifier_names(&self, node: Node<'_>, out: &mut BTreeSet<String>) {
        if node.kind() == "identifier" {
            out.insert(self.node_text(&node).to_string());
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_identifier_names(child, out);
        }
    }
}
