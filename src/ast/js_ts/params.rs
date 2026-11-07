//! JS/TS formal parameters: member-only use classification, bare references, rest and arrow formals.

use crate::ast::{ParameterUse, ParsedFile};
use crate::languages::Language;
use tree_sitter::Node;

impl ParsedFile {
    /// PR-C: classify how a formal is used. `MemberOnly`
    /// is JS/TS/TSX only: no bare read, at least one `name.<prop>` member
    /// read in the body, callable not parse-recovered, formal not a seam
    /// binding, callable containing no `with` statement and no escaped identifier.
    /// Shared by `data_flow.rs` and `reasoning/seeds.rs` (lockstep).
    pub fn classify_parameter_use(&self, func_node: &Node<'_>, var_name: &str) -> ParameterUse {
        if self.has_bare_references(func_node, var_name) {
            return ParameterUse::Bare;
        }
        if !matches!(
            self.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) {
            return ParameterUse::Unused;
        }
        fn recovered(node: Node<'_>) -> bool {
            if node.is_error() || node.is_missing() {
                return true;
            }
            let mut cursor = node.walk();
            let found = node.children(&mut cursor).any(recovered);
            found
        }
        if recovered(*func_node)
            || self.js_ts_seam_binding(func_node, var_name)
            || self.callable_has_escaped_identifier(*func_node)
            || self.callable_has_with_statement(*func_node)
        {
            return ParameterUse::Unused;
        }
        let Some(body) = self.function_body_node(func_node) else {
            return ParameterUse::Unused;
        };
        if self.has_member_read_of(body, var_name) {
            ParameterUse::MemberOnly
        } else {
            ParameterUse::Unused
        }
    }

    fn has_member_read_of(&self, node: Node<'_>, var_name: &str) -> bool {
        if node.kind() == "member_expression" {
            if self.language.is_in_erased_type_context(node) {
                // Erased TS positions (`typeof p.x`, interface members) are not reads.
                return false;
            }
            if let Some(object) = node.child_by_field_name("object") {
                if object.kind() == "identifier" && self.node_text(&object) == var_name {
                    return true;
                }
            }
        }
        let mut cursor = node.walk();
        let found = node
            .children(&mut cursor)
            .any(|child| self.has_member_read_of(child, var_name));
        found
    }

    /// Check if a variable has any bare (non-field-access) references in a function
    /// body (excluding the parameter list itself).
    ///
    /// Returns true if the variable is used as a standalone identifier, not just as
    /// the base of a field/member access (e.g., `data` in `use(data)` counts, but
    /// `dev` in `dev.name` does not). Used to decide whether to register a parameter
    /// Def for interprocedural data flow.
    pub fn has_bare_references(&self, func_node: &Node<'_>, var_name: &str) -> bool {
        if let Some(body) = self.function_body_node(func_node) {
            return self.find_bare_ref(body, var_name);
        }
        self.find_bare_ref_excluding_parameters(*func_node, func_node, var_name)
    }

    fn find_bare_ref_excluding_parameters(
        &self,
        node: Node<'_>,
        func_node: &Node<'_>,
        var_name: &str,
    ) -> bool {
        if let Some(params) = self.find_parameters_node(func_node) {
            if node.start_byte() >= params.start_byte() && node.end_byte() <= params.end_byte() {
                return false;
            }
        }
        if Self::is_field_access_node(node.kind()) {
            return false;
        }
        if self.language.is_identifier_node(node.kind()) && self.node_text(&node) == var_name {
            if let Some(parent) = node.parent() {
                if Self::is_field_access_node(parent.kind()) {
                    return false;
                }
            }
            return true;
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if self.find_bare_ref_excluding_parameters(child, func_node, var_name) {
                return true;
            }
        }
        false
    }

    fn find_bare_ref(&self, node: Node<'_>, var_name: &str) -> bool {
        // If this is a field access (dev.name, dev->name), skip it — the `dev`
        // identifier inside is not a bare reference. `member_expression` exists only in
        // JS/TS/TSX, where the object can itself hold a bare read (`wrap(p).x`, `[p][0].x`).
        if Self::is_field_access_node(node.kind()) {
            return node.kind() == "member_expression"
                && node
                    .child_by_field_name("object")
                    .is_some_and(|object| self.find_bare_ref(object, var_name));
        }

        if self.language.is_identifier_node(node.kind()) && self.node_text(&node) == var_name {
            // Check parent: if parent is a field access, this isn't a bare ref.
            if let Some(parent) = node.parent() {
                if Self::is_field_access_node(parent.kind()) {
                    return false;
                }
            }
            return true;
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if self.find_bare_ref(child, var_name) {
                return true;
            }
        }
        false
    }

    /// js-param-defs PR-A: the single unparenthesised formal of a JS/TS/TSX
    /// `arrow_function` (`x => …`). Tree-sitter stores it in the `parameter`
    /// field, not in a `parameters` list, so `find_parameters_node` is `None`
    /// for this shape. `find_parameters_node` deliberately stays unchanged:
    /// its callers iterate the list's children (an identifier has none) and
    /// several treat `None` as "no list" with their own `parameter` fallback.
    pub(crate) fn js_ts_bare_arrow_parameter<'a>(&self, func_node: &Node<'a>) -> Option<Node<'a>> {
        if !matches!(
            self.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) || func_node.kind() != "arrow_function"
        {
            return None;
        }
        func_node
            .child_by_field_name("parameter")
            .filter(|param| param.kind() == "identifier" && !param.is_missing())
    }

    /// js-param-defs PR-A: `...name` binds one identifier (the engine-created
    /// array). Destructured rest (`...[a, b]`, `...{a}`) stays refused.
    pub(crate) fn js_ts_rest_identifier<'a>(&self, rest: Node<'a>) -> Option<Node<'a>> {
        if rest.kind() != "rest_pattern" {
            return None;
        }
        let mut cursor = rest.walk();
        let named: Vec<_> = rest
            .named_children(&mut cursor)
            .filter(|child| child.kind() != "comment")
            .collect();
        match named.as_slice() {
            [only] if only.kind() == "identifier" => Some(*only),
            _ => None,
        }
    }

    /// PR-A D11: refuse new shapes if another binding anywhere inside the
    /// owner's body may bind this name. Escaped binding spellings are ambiguous
    /// and fail closed. Plain-formal admission and the legacy walk stay intact.
    pub(crate) fn js_ts_nested_callable_binds_formal(
        &self,
        func_node: &Node<'_>,
        name: &str,
    ) -> bool {
        let boundaries = self.language.callable_boundary_node_types();
        let mut stack: Vec<_> = func_node.child_by_field_name("body").into_iter().collect();
        while let Some(node) = stack.pop() {
            if boundaries.contains(&node.kind()) {
                if let Some(params) = self
                    .find_parameters_node(&node)
                    .or_else(|| node.child_by_field_name("parameter"))
                {
                    if self.js_ts_binding_pattern_may_bind(params, name) {
                        return true;
                    }
                }
            }
            let binding = match node.kind() {
                "variable_declarator"
                | "function_expression"
                | "function_declaration"
                | "generator_function"
                | "generator_function_declaration"
                | "class"
                | "class_declaration"
                | "abstract_class_declaration"
                | "enum_declaration"
                | "internal_module"
                | "module" => node.child_by_field_name("name"),
                "catch_clause" => node.child_by_field_name("parameter"),
                "for_in_statement" => node.child_by_field_name("left"),
                _ => None,
            };
            if binding.is_some_and(|binding| self.js_ts_binding_pattern_may_bind(binding, name)) {
                return true;
            }
            let mut cursor = node.walk();
            stack.extend(node.named_children(&mut cursor));
        }
        false
    }

    fn js_ts_binding_pattern_may_bind(&self, node: Node<'_>, name: &str) -> bool {
        match node.kind() {
            "identifier" | "type_identifier" | "shorthand_property_identifier_pattern" => {
                let text = self.node_text(&node);
                text == name || text.contains('\\')
            }
            "pair_pattern" => node
                .child_by_field_name("value")
                .is_some_and(|n| self.js_ts_binding_pattern_may_bind(n, name)),
            "assignment_pattern" | "object_assignment_pattern" => node
                .child_by_field_name("left")
                .is_some_and(|n| self.js_ts_binding_pattern_may_bind(n, name)),
            "required_parameter" | "optional_parameter" => node
                .child_by_field_name("pattern")
                .is_some_and(|n| self.js_ts_binding_pattern_may_bind(n, name)),
            "formal_parameters" | "object_pattern" | "array_pattern" | "rest_pattern" => {
                let mut cursor = node.walk();
                let found = node
                    .named_children(&mut cursor)
                    .any(|n| self.js_ts_binding_pattern_may_bind(n, name));
                found
            }
            _ => false,
        }
    }
}
