//! Function parameter names, occurrences, slots and parameter-name extraction.

use crate::ast::{ParameterOccurrence, ParsedFile};
use crate::languages::Language;
use tree_sitter::Node;

impl ParsedFile {
    /// Extract binding names in declaration order for non-positional consumers.
    ///
    /// Go grouped declarations intentionally expand (`a, b string` yields both
    /// bindings), so DFG and reasoning receive one definition seed per name.
    pub fn function_parameter_names(&self, func_node: &Node<'_>) -> Vec<String> {
        if matches!(
            self.language,
            Language::Go | Language::TypeScript | Language::Tsx
        ) {
            return self
                .function_parameter_occurrences(func_node)
                .into_iter()
                .map(|(name, _, _)| name)
                .collect();
        }

        let mut names = Vec::new();
        if let Some(params) = self.find_parameters_node(func_node) {
            let mut cursor = params.walk();
            for child in params.children(&mut cursor) {
                if let Some(name) = self.extract_param_name(&child) {
                    names.push(name);
                }
            }
        }
        names
    }

    /// Byte-bearing sibling of `function_parameter_names`.
    ///
    /// The tuple is `(name, start_byte, end_byte)` for the actual parameter
    /// binding token. The DFG still anchors parameter defs to the function start
    /// line for call-boundary compatibility.
    pub fn function_parameter_occurrences(&self, func_node: &Node<'_>) -> Vec<ParameterOccurrence> {
        if matches!(self.language, Language::TypeScript | Language::Tsx) {
            return crate::parameter_slots::typescript_parameter_bindings(self, func_node);
        }
        if let Some(param) = self.js_ts_bare_arrow_parameter(func_node) {
            // js-param-defs PR-A: `x => …` binds exactly one formal; the
            // arrow has no `parameters` list for the loop below to walk.
            let name = self.node_text(&param);
            if self.js_ts_nested_callable_binds_formal(func_node, name) {
                return Vec::new();
            }
            return vec![(name.to_string(), param.start_byte(), param.end_byte())];
        }
        let mut params_out = Vec::new();
        if let Some(params) = self.find_parameters_node(func_node) {
            // js-param-defs PR-A: a JS rest formal is admitted only from a
            // recovery-free list without duplicate bindings (a rest element
            // makes the list non-simple, so duplicates are early errors).
            let admit_js_rest = self.language == Language::JavaScript
                && crate::parameter_slots::js_ts_parameter_list_is_clean(self, params);
            let mut cursor = params.walk();
            for child in params.children(&mut cursor) {
                if child.kind() == "rest_pattern" && self.language == Language::JavaScript {
                    if let Some(name_node) = (admit_js_rest
                        && crate::parameter_slots::js_ts_is_last_parameter(params, child))
                    .then(|| self.js_ts_rest_identifier(child))
                    .flatten()
                    .filter(|name| {
                        !self.js_ts_nested_callable_binds_formal(func_node, self.node_text(name))
                    }) {
                        params_out.push((
                            self.node_text(&name_node).to_string(),
                            name_node.start_byte(),
                            name_node.end_byte(),
                        ));
                    }
                    continue;
                }
                for name_node in self.parameter_binding_name_nodes(child) {
                    params_out.push((
                        self.node_text(&name_node).to_string(),
                        name_node.start_byte(),
                        name_node.end_byte(),
                    ));
                }
            }
            if self.language == Language::JavaScript {
                params_out.extend(
                    crate::parameter_slots::javascript_inert_default_occurrences(self, params),
                );
                params_out.sort_by_key(|occurrence| occurrence.1);
            }
        }
        params_out
    }

    /// Exact runtime parameter slots in declaration order. A deterministic
    /// prefix is returned through the first variadic, keyword-only, or
    /// unrepresentable parameter; duplicate bindings and parse recovery return
    /// `None` because they cannot support an exact positional mapping.
    pub fn function_parameter_slots(&self, func_node: &Node<'_>) -> Option<Vec<String>> {
        crate::parameter_slots::slots(self, func_node)
            .map(|occurrences| occurrences.into_iter().map(|(name, _, _)| name).collect())
    }

    /// Byte-bearing twin of [`Self::function_parameter_slots`].
    pub fn function_parameter_slot_occurrences(
        &self,
        func_node: &Node<'_>,
    ) -> Option<Vec<ParameterOccurrence>> {
        crate::parameter_slots::slots(self, func_node)
    }

    fn parameter_binding_name_nodes<'a>(&self, node: Node<'a>) -> Vec<Node<'a>> {
        if self.language == Language::Go
            && matches!(
                node.kind(),
                "parameter_declaration" | "variadic_parameter_declaration"
            )
        {
            let mut cursor = node.walk();
            return node
                .children(&mut cursor)
                .filter(|child| child.kind() == "identifier" && self.node_text(child) != "_")
                .collect();
        }
        self.extract_param_name_node(&node).into_iter().collect()
    }

    /// JS/TS/TSX byte region holding the parameter bindings: the `parameters`
    /// list, or the bare arrow formal. Other languages: `find_parameters_node`.
    pub(crate) fn parameter_binding_region<'a>(&self, func_node: &Node<'a>) -> Option<Node<'a>> {
        self.find_parameters_node(func_node)
            .or_else(|| self.js_ts_bare_arrow_parameter(func_node))
    }

    /// Find the parameters node within a function definition.
    /// Handles the C/C++ declarator chain (function_definition → declarator → function_declarator → parameters).
    pub(crate) fn find_parameters_node<'a>(&self, node: &Node<'a>) -> Option<Node<'a>> {
        let node = self.unwrap_decorated(*node);
        // Direct "parameters" field (Go, Rust, Python, JS/TS, Java, Lua)
        if let Some(params) = node.child_by_field_name("parameters") {
            return Some(params);
        }
        // C/C++: navigate declarator chain to find function_declarator with parameters
        if let Some(declarator) = node.child_by_field_name("declarator") {
            return self.find_params_in_declarator(&declarator);
        }
        None
    }

    /// Recursively search a C/C++ declarator chain for a parameters node.
    fn find_params_in_declarator<'a>(&self, node: &Node<'a>) -> Option<Node<'a>> {
        if let Some(params) = node.child_by_field_name("parameters") {
            return Some(params);
        }
        // Navigate: pointer_declarator → function_declarator, etc.
        if let Some(decl) = node.child_by_field_name("declarator") {
            return self.find_params_in_declarator(&decl);
        }
        // Walk children for other declarator wrappers
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind().contains("declarator") {
                if let Some(params) = self.find_params_in_declarator(&child) {
                    return Some(params);
                }
            }
        }
        None
    }

    /// Extract the parameter name from a parameter declaration node.
    pub(super) fn extract_param_name(&self, node: &Node<'_>) -> Option<String> {
        match node.kind() {
            "parameter_declaration" | "optional_parameter_declaration" => {
                // C/C++/Java: has a declarator field containing the identifier
                if let Some(decl) = node.child_by_field_name("declarator") {
                    return Some(self.innermost_identifier(&decl));
                }
                // Fallback: find any identifier child
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "identifier" {
                        return Some(self.node_text(&child).to_string());
                    }
                }
                None
            }
            "parameter" => {
                // Rust/Go: name field or pattern field
                if let Some(name) = node.child_by_field_name("name") {
                    return Some(self.node_text(&name).to_string());
                }
                if let Some(pattern) = node.child_by_field_name("pattern") {
                    return Some(self.node_text(&pattern).to_string());
                }
                // Go: last identifier before the type
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "identifier" {
                        return Some(self.node_text(&child).to_string());
                    }
                }
                None
            }
            "identifier" => {
                // Python/Lua: parameters may be direct identifiers
                Some(self.node_text(node).to_string())
            }
            "typed_parameter"
            | "typed_default_parameter"
            | "default_parameter"
            | "list_splat_pattern"
            | "dictionary_splat_pattern" => {
                // Python: typed / default / splat parameter forms.
                if let Some(name) = node.child_by_field_name("name") {
                    return Some(self.node_text(&name).to_string());
                }
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "identifier" {
                        return Some(self.node_text(&child).to_string());
                    }
                }
                None
            }
            _ => None,
        }
    }

    pub(crate) fn extract_param_name_node<'a>(&self, node: &Node<'a>) -> Option<Node<'a>> {
        match node.kind() {
            "parameter_declaration" | "optional_parameter_declaration" => {
                if let Some(decl) = node.child_by_field_name("declarator") {
                    return self.innermost_identifier_node(&decl);
                }
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "identifier" {
                        return Some(child);
                    }
                }
                None
            }
            "parameter" => {
                if let Some(name) = node.child_by_field_name("name") {
                    return Some(name);
                }
                if let Some(pattern) = node.child_by_field_name("pattern") {
                    return self.first_binding_identifier(pattern);
                }
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "identifier" {
                        return Some(child);
                    }
                }
                None
            }
            "identifier" => Some(*node),
            "typed_parameter"
            | "typed_default_parameter"
            | "default_parameter"
            | "list_splat_pattern"
            | "dictionary_splat_pattern" => {
                if let Some(name) = node.child_by_field_name("name") {
                    return Some(name);
                }
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "identifier" {
                        return Some(child);
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn first_binding_identifier<'a>(&self, node: Node<'a>) -> Option<Node<'a>> {
        if self.is_binding_identifier_node(node.kind()) {
            return Some(node);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = self.first_binding_identifier(child) {
                return Some(found);
            }
        }
        None
    }

    /// Find the innermost identifier in a C/C++ declarator chain.
    fn innermost_identifier(&self, node: &Node<'_>) -> String {
        if node.kind() == "identifier" || node.kind() == "field_identifier" {
            return self.node_text(node).to_string();
        }
        // Check declarator field first
        if let Some(decl) = node.child_by_field_name("declarator") {
            return self.innermost_identifier(&decl);
        }
        // Walk children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" || child.kind() == "field_identifier" {
                return self.node_text(&child).to_string();
            }
            if child.kind().contains("declarator") {
                return self.innermost_identifier(&child);
            }
        }
        self.node_text(node).to_string()
    }

    fn innermost_identifier_node<'a>(&self, node: &Node<'a>) -> Option<Node<'a>> {
        if node.kind() == "identifier" || node.kind() == "field_identifier" {
            return Some(*node);
        }
        if let Some(decl) = node.child_by_field_name("declarator") {
            if let Some(found) = self.innermost_identifier_node(&decl) {
                return Some(found);
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" || child.kind() == "field_identifier" {
                return Some(child);
            }
            if child.kind().contains("declarator") {
                if let Some(found) = self.innermost_identifier_node(&child) {
                    return Some(found);
                }
            }
        }
        None
    }
}
