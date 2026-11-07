//! JS/TS local and parameter binding names and the scopes that reach a receiver.

use crate::ast::{is_js_ts_function_like, ParsedFile};
use std::collections::BTreeSet;
use tree_sitter::Node;

/// S1b-1b: whether a JS/TS declaration sits in `collect_js_ts_local_bindings`' root scope: at
/// module scope (directly or through `export`) or in a function's body block. A bare block,
/// even at module scope, is nested. That collector never enters a non-root function, so a
/// function-like parent here is the root.
fn js_ts_declaration_at_top(decl: Node<'_>) -> bool {
    decl.parent()
        .is_some_and(|h| match (h.kind(), h.parent().map_or("", |p| p.kind())) {
            ("program", _) | ("export_statement", "program") => true,
            ("statement_block", parent) => is_js_ts_function_like(parent),
            _ => false,
        })
}

impl ParsedFile {
    pub(in crate::ast) fn collect_js_ts_parameter_bindings(&self, func_node: Node<'_>, out: &mut BTreeSet<String>) {
        if let Some(params) = self.find_parameters_node(&func_node) {
            let mut cursor = params.walk();
            for child in params.children(&mut cursor) {
                self.collect_js_ts_parameter_binding_names(child, out);
            }
        } else if let Some(param) = func_node.child_by_field_name("parameter") {
            // S1b F3: a single unparenthesized arrow parameter (`x => x()`).
            self.collect_js_ts_binding_pattern_names(param, out);
        }
    }

    pub(super) fn collect_js_ts_parameter_binding_names(&self, node: Node<'_>, out: &mut BTreeSet<String>) {
        if let Some(pattern) = node.child_by_field_name("pattern") {
            self.collect_js_ts_binding_pattern_names(pattern, out);
            return;
        }
        if let Some(name) = node.child_by_field_name("name") {
            self.collect_js_ts_binding_pattern_names(name, out);
            return;
        }
        if let Some(left) = node.child_by_field_name("left") {
            self.collect_js_ts_binding_pattern_names(left, out);
            return;
        }

        match node.kind() {
            "object_pattern" | "array_pattern" | "identifier" | "rest_pattern" => {
                self.collect_js_ts_binding_pattern_names(node, out);
            }
            kind if kind.contains("parameter") => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if matches!(child.kind(), "type_annotation" | "return_type") {
                        continue;
                    }
                    self.collect_js_ts_binding_pattern_names(child, out);
                }
            }
            _ => {}
        }
    }

    pub(in crate::ast) fn collect_js_ts_binding_pattern_names(&self, node: Node<'_>, out: &mut BTreeSet<String>) {
        match node.kind() {
            "identifier" | "shorthand_property_identifier_pattern" => {
                let name = self.node_text(&node);
                // S1b-3 (spec r1 sol W1): the parser confirmed an ECMAScript IdentifierName, so
                // any spelling without a `\` escape is a BoundName (`é` with U+0301, ZWNJ/ZWJ,
                // `$`, S1b-1b); an escaped spelling stays out and B0 refuses its scope.
                if !name.contains('\\') {
                    out.insert(name.to_string());
                }
            }
            "pair_pattern" => {
                if let Some(value) = node.child_by_field_name("value") {
                    self.collect_js_ts_binding_pattern_names(value, out);
                }
            }
            "rest_pattern" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.collect_js_ts_binding_pattern_names(child, out);
                }
            }
            // S1b F2: a `{ f = d }` destructuring default binds `f`, like `f = d`.
            "assignment_pattern" | "object_assignment_pattern" => {
                if let Some(left) = node.child_by_field_name("left") {
                    self.collect_js_ts_binding_pattern_names(left, out);
                }
            }
            // S1b F1: a TS assertion around a write target still writes its binding.
            "as_expression" | "satisfies_expression" | "non_null_expression" => {
                if let Some(inner) = node.named_child(0) {
                    self.collect_js_ts_binding_pattern_names(inner, out);
                }
            }
            // `<T>x`: the asserted expression is the last named child.
            "type_assertion" => {
                if let Some(inner) = node.named_child(node.named_child_count().saturating_sub(1)) {
                    self.collect_js_ts_binding_pattern_names(inner, out);
                }
            }
            "object_pattern"
            | "array_pattern"
            | "parenthesized_expression"
            | "parenthesized_pattern" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.collect_js_ts_binding_pattern_names(child, out);
                }
            }
            _ => {}
        }
    }

    pub(super) fn collect_js_ts_local_bindings(
        &self,
        node: Node<'_>,
        root_key: (usize, usize, u16),
        out: &mut BTreeSet<String>,
    ) {
        let is_root = (node.start_byte(), node.end_byte(), node.kind_id()) == root_key;

        if !is_root && is_js_ts_function_like(node.kind()) {
            if node.kind() == "function_declaration" {
                if let Some(name) = self.language.function_name(&node) {
                    out.insert(self.node_text(&name).to_string());
                }
            }
            return;
        }

        match node.kind() {
            // S1b-1b (F4): a top-scope declarator binds the pattern's BoundNames (shorthand
            // properties included, destructuring-default expressions excluded). A nested one
            // keeps base (Option K): this set is block-blind; S1b-3's site walk owns blocks.
            "variable_declarator" if node.parent().is_some_and(js_ts_declaration_at_top) => {
                if let Some(name) = node.child_by_field_name("name") {
                    self.collect_js_ts_binding_pattern_names(name, out);
                }
            }
            "variable_declarator" => {
                if let Some(name) = node.child_by_field_name("name") {
                    self.collect_identifier_names(name, out);
                }
            }
            "catch_clause" => {
                self.collect_js_ts_catch_binding_names(node, out);
            }
            "class_declaration" if !is_root => {
                if let Some(name) = node.child_by_field_name("name") {
                    out.insert(self.node_text(&name).to_string());
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_js_ts_local_bindings(child, root_key, out);
        }
    }

    pub(super) fn js_ts_receiver_binding_reaches_call(
        &self,
        node: Node<'_>,
        root_scope_id: usize,
        receiver: &Node<'_>,
        receiver_name: &str,
    ) -> bool {
        let is_root = node.id() == root_scope_id;

        if !is_root && is_js_ts_function_like(node.kind()) {
            return matches!(
                node.kind(),
                "function_declaration" | "generator_function_declaration"
            ) && self
                .language
                .function_name(&node)
                .is_some_and(|name| self.node_text(&name) == receiver_name)
                && self.js_ts_lexical_scope_reaches_receiver(&node, receiver, root_scope_id);
        }

        if !is_root && node.kind() == "class_declaration" {
            return node
                .child_by_field_name("name")
                .is_some_and(|name| self.node_text(&name) == receiver_name)
                && self.js_ts_lexical_scope_reaches_receiver(&node, receiver, root_scope_id);
        }

        if !is_root && node.kind() == "class" {
            let self_name_matches = node
                .child_by_field_name("name")
                .is_some_and(|name| self.node_text(&name) == receiver_name);
            let mut current = Some(*receiver);
            let mut receiver_is_inside = false;
            while let Some(ancestor) = current {
                if ancestor.id() == node.id() {
                    receiver_is_inside = true;
                    break;
                }
                current = ancestor.parent();
            }
            return self_name_matches && receiver_is_inside;
        }

        if !is_root
            && matches!(
                node.kind(),
                "interface_declaration" | "type_alias_declaration"
            )
        {
            return false;
        }

        if !is_root
            && matches!(
                node.kind(),
                "abstract_class_declaration" | "enum_declaration" | "function_signature"
            )
        {
            return node
                .child_by_field_name("name")
                .is_some_and(|name| self.node_text(&name) == receiver_name)
                && self.js_ts_lexical_scope_reaches_receiver(&node, receiver, root_scope_id);
        }

        if !is_root && node.kind() == "import_alias" {
            return node
                .named_child(0)
                .is_some_and(|name| self.node_text(&name) == receiver_name)
                && self.js_ts_lexical_scope_reaches_receiver(&node, receiver, root_scope_id);
        }

        if !is_root && matches!(node.kind(), "internal_module" | "module") {
            let name_binds =
                node.child_by_field_name("name").is_some_and(|name| {
                    name.kind() == "identifier" && self.node_text(&name) == receiver_name
                }) && self.js_ts_lexical_scope_reaches_receiver(&node, receiver, root_scope_id);
            if name_binds {
                return true;
            }

            let mut current = Some(*receiver);
            let mut receiver_is_inside = false;
            while let Some(ancestor) = current {
                if ancestor.id() == node.id() {
                    receiver_is_inside = true;
                    break;
                }
                current = ancestor.parent();
            }
            if !receiver_is_inside {
                return false;
            }
        }

        if node.is_error() || node.is_missing() {
            return true;
        }

        match node.kind() {
            "for_in_statement" | "for_of_statement" | "for_await_statement" => {
                let mut is_var = false;
                let mut is_lexical = false;
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    match child.kind() {
                        "var" => is_var = true,
                        "let" | "const" => is_lexical = true,
                        _ => {}
                    }
                }
                if is_var || is_lexical {
                    let mut names = BTreeSet::new();
                    if let Some(left) = node.child_by_field_name("left") {
                        self.collect_js_ts_binding_pattern_names(left, &mut names);
                    }
                    if names.contains(receiver_name)
                        && (is_var
                            || self.js_ts_lexical_scope_reaches_receiver(
                                &node,
                                receiver,
                                root_scope_id,
                            ))
                    {
                        return true;
                    }
                }
            }
            "variable_declarator" => {
                let mut names = BTreeSet::new();
                if let Some(name) = node.child_by_field_name("name") {
                    self.collect_js_ts_binding_pattern_names(name, &mut names);
                }
                if names.contains(receiver_name) {
                    let is_function_scoped_var = node
                        .parent()
                        .is_some_and(|parent| parent.kind() == "variable_declaration");
                    if is_function_scoped_var
                        || self.js_ts_lexical_scope_reaches_receiver(&node, receiver, root_scope_id)
                    {
                        return true;
                    }
                }
            }
            "catch_clause" => {
                let mut names = BTreeSet::new();
                self.collect_js_ts_catch_binding_names(node, &mut names);
                if names.contains(receiver_name)
                    && self.js_ts_lexical_scope_reaches_receiver(&node, receiver, root_scope_id)
                {
                    return true;
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        let found = node.named_children(&mut cursor).any(|child| {
            self.js_ts_receiver_binding_reaches_call(child, root_scope_id, receiver, receiver_name)
        });
        found
    }

    pub(super) fn js_ts_function_scope_binds_receiver(
        &self,
        func_node: &Node<'_>,
        receiver: &Node<'_>,
        receiver_name: &str,
    ) -> bool {
        let mut parameters = BTreeSet::new();
        self.collect_js_ts_parameter_bindings(*func_node, &mut parameters);
        if parameters.contains(receiver_name) {
            return true;
        }

        // Callable display names may come from assignment properties or object
        // keys. Only an explicit source name establishes this binding.
        if matches!(
            func_node.kind(),
            "function_declaration"
                | "function_expression"
                | "generator_function_declaration"
                | "generator_function"
        ) && func_node
            .child_by_field_name("name")
            .is_some_and(|name| self.node_text(&name) == receiver_name)
        {
            return true;
        }

        self.js_ts_receiver_binding_reaches_call(
            *func_node,
            func_node.id(),
            receiver,
            receiver_name,
        )
    }

    fn js_ts_lexical_scope_reaches_receiver(
        &self,
        binding: &Node<'_>,
        receiver: &Node<'_>,
        root_scope_id: usize,
    ) -> bool {
        let Some(scope_id) = self.js_ts_nearest_lexical_scope_id(binding, root_scope_id) else {
            return false;
        };
        let mut current = Some(*receiver);
        while let Some(node) = current {
            if node.id() == scope_id {
                return true;
            }
            if node.id() == root_scope_id {
                return false;
            }
            current = node.parent();
        }
        false
    }

    pub(super) fn js_ts_nearest_lexical_scope_id(
        &self,
        binding: &Node<'_>,
        root_scope_id: usize,
    ) -> Option<usize> {
        let mut current = Some(*binding);
        while let Some(node) = current {
            if node.id() == root_scope_id {
                return Some(node.id());
            }
            if self.language.is_scope_block(node.kind())
                || matches!(
                    node.kind(),
                    "for_statement"
                        | "for_in_statement"
                        | "for_of_statement"
                        | "for_await_statement"
                        | "switch_statement"
                        | "switch_body"
                        | "catch_clause"
                )
            {
                return Some(node.id());
            }
            current = node.parent();
        }
        None
    }

    pub(in crate::ast) fn collect_js_ts_catch_binding_names(&self, node: Node<'_>, out: &mut BTreeSet<String>) {
        if let Some(param) = node.child_by_field_name("parameter") {
            self.collect_js_ts_binding_pattern_names(param, out);
            return;
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "statement_block" {
                continue;
            }
            self.collect_js_ts_binding_pattern_names(child, out);
        }
    }
}
