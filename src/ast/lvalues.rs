//! Lvalue path and span extraction, destructuring and alias assignments.

use crate::access_path::AccessPath;
use crate::ast::{is_plain_ident, ParsedFile, PathSpan};
use crate::languages::Language;
use std::collections::BTreeSet;
use tree_sitter::Node;

impl ParsedFile {
    /// Extract individual L-value paths from a multi-target node (pattern_list, expression_list).
    ///
    /// Handles:
    /// - Go: `val, err := getData()` — expression_list with identifier children
    /// - Python: `name, age = get_user()` — pattern_list with identifier children
    /// - Python: `first, *rest = items` — pattern_list with identifier + list_splat_pattern
    /// - Go: `for key, value := range m` — expression_list in range clause
    pub(super) fn extract_multi_target_lvalues(
        &self,
        node: &Node<'_>,
        line: usize,
        out: &mut Vec<(AccessPath, usize)>,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    let name = self.node_text(&child).to_string();
                    out.push((AccessPath::simple(name), line));
                }
                // Python star unpack: *rest
                "list_splat_pattern" => {
                    let mut inner_cursor = child.walk();
                    for inner in child.children(&mut inner_cursor) {
                        if inner.kind() == "identifier" {
                            let name = self.node_text(&inner).to_string();
                            out.push((AccessPath::simple(name), line));
                        }
                    }
                }
                // Field access as L-value: obj.field, dev->field in multi-target
                "field_expression"
                | "member_expression"
                | "selector_expression"
                | "attribute"
                | "field_access"
                | "dot_index_expression"
                | "method_index_expression" => {
                    let text = self.node_text(&child).to_string();
                    for path in extract_lvalue_paths(&text) {
                        out.push((path, line));
                    }
                }
                // Nested tuple: (a, b), c = func()
                "pattern_list" | "tuple_pattern" | "parenthesized_expression" => {
                    self.extract_multi_target_lvalues(&child, line, out);
                }
                // Skip punctuation (commas, etc.)
                _ => {}
            }
        }
    }

    pub(super) fn extract_lvalue_spans_from_node(&self, node: Node<'_>, out: &mut Vec<PathSpan>) {
        let kind = node.kind();

        if let Some(member) = self.bounded_member_path(node) {
            out.push(PathSpan {
                path: member.path,
                line: node.start_position().row + 1,
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
            });
            return;
        }

        if Self::is_field_access_node(kind) || Self::is_index_access_node(kind) {
            let text = self.node_text(&node).to_string();
            for path in extract_lvalue_paths(&text) {
                out.push(PathSpan {
                    path,
                    line: node.start_position().row + 1,
                    start_byte: node.start_byte(),
                    end_byte: node.end_byte(),
                });
            }
            return;
        }

        if self.is_binding_identifier_node(kind) {
            self.push_simple_lvalue_span(node, out);
            return;
        }

        match kind {
            "object_pattern" | "array_pattern" => {
                self.extract_destructuring_def_spans(&node, out);
                return;
            }
            "pair_pattern" => {
                if let Some(value) = node.child_by_field_name("value") {
                    self.extract_lvalue_spans_from_node(value, out);
                    return;
                }
            }
            "rest_pattern"
            | "list_splat_pattern"
            | "dictionary_splat_pattern"
            | "spread_element"
            | "mutable_pattern"
            | "mut_pattern"
            | "reference_pattern"
            | "ref_pattern"
            | "tuple_pattern"
            | "pattern_list"
            | "expression_list"
            | "parenthesized_expression"
            | "parenthesized_pattern" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.extract_lvalue_spans_from_node(child, out);
                }
                return;
            }
            _ => {}
        }

        let text = self.node_text(&node).to_string();
        let paths = extract_lvalue_paths(&text);
        if !paths.is_empty() {
            for path in paths {
                out.push(PathSpan {
                    path,
                    line: node.start_position().row + 1,
                    start_byte: node.start_byte(),
                    end_byte: node.end_byte(),
                });
            }
            return;
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.extract_lvalue_spans_from_node(child, out);
        }
    }

    pub(super) fn is_binding_identifier_node(&self, kind: &str) -> bool {
        matches!(
            kind,
            "identifier"
                | "shorthand_property_identifier_pattern"
                | "property_identifier"
                | "field_identifier"
        )
    }

    fn is_index_access_node(kind: &str) -> bool {
        matches!(
            kind,
            "subscript_expression" | "index_expression" | "slice_expression"
        )
    }

    pub(super) fn push_simple_lvalue_span(&self, node: Node<'_>, out: &mut Vec<PathSpan>) {
        let name = self.node_text(&node).to_string();
        if is_plain_ident(&name) {
            out.push(PathSpan {
                path: AccessPath::simple(name),
                line: node.start_position().row + 1,
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
            });
        }
    }

    /// Extract L-value defs from a JS/TS `for_in_statement` loop header.
    ///
    /// Handles:
    /// - `for (const key in obj)` → def for "key"
    /// - `for (const { name, id } of items)` → defs for "name", "id"
    /// - `for (const [a, b] of pairs)` → defs for "a", "b"
    pub(super) fn extract_for_in_lvalues(
        &self,
        node: &Node<'_>,
        line: usize,
        out: &mut Vec<(AccessPath, usize)>,
    ) {
        // The iterable may itself be an identifier. Only the grammar's left
        // field is a binding; scanning siblings invents a Def at a read token.
        let Some(left) = node.child_by_field_name("left") else {
            return;
        };
        match left.kind() {
            "identifier" => {
                let name = self.node_text(&left).to_string();
                if is_plain_ident(&name) && name != "const" && name != "let" && name != "var" {
                    out.push((AccessPath::simple(name), line));
                }
            }
            "object_pattern" | "array_pattern" => {
                self.extract_destructuring_defs(&left, line, out);
            }
            _ => {}
        }
    }

    pub(super) fn extract_for_in_lvalue_spans(&self, node: &Node<'_>, out: &mut Vec<PathSpan>) {
        // Keep byte-bearing extraction under the same left-only contract.
        let Some(left) = node.child_by_field_name("left") else {
            return;
        };
        match left.kind() {
            "identifier" => {
                let name = self.node_text(&left).to_string();
                if is_plain_ident(&name) && name != "const" && name != "let" && name != "var" {
                    self.push_simple_lvalue_span(left, out);
                }
            }
            "object_pattern" | "array_pattern" => {
                self.extract_destructuring_def_spans(&left, out);
            }
            _ => {}
        }
    }

    /// Extract defs from a destructuring pattern (object_pattern or array_pattern).
    fn extract_destructuring_defs(
        &self,
        pattern: &Node<'_>,
        line: usize,
        out: &mut Vec<(AccessPath, usize)>,
    ) {
        let mut cursor = pattern.walk();
        for child in pattern.children(&mut cursor) {
            match child.kind() {
                "shorthand_property_identifier_pattern" | "identifier" => {
                    let name = self.node_text(&child).to_string();
                    if is_plain_ident(&name) {
                        out.push((AccessPath::simple(name), line));
                    }
                }
                "pair_pattern" => {
                    // { name: alias } — the value side is the bound variable
                    if let Some(val) = child.child_by_field_name("value") {
                        if val.kind() == "identifier" {
                            let name = self.node_text(&val).to_string();
                            if is_plain_ident(&name) {
                                out.push((AccessPath::simple(name), line));
                            }
                        } else if val.kind() == "object_pattern" || val.kind() == "array_pattern" {
                            self.extract_destructuring_defs(&val, line, out);
                        }
                    }
                }
                "rest_pattern" => {
                    let mut inner_cursor = child.walk();
                    for inner in child.children(&mut inner_cursor) {
                        if inner.kind() == "identifier" {
                            let name = self.node_text(&inner).to_string();
                            if is_plain_ident(&name) {
                                out.push((AccessPath::simple(name), line));
                            }
                        }
                    }
                }
                "object_pattern" | "array_pattern" => {
                    self.extract_destructuring_defs(&child, line, out);
                }
                _ => {}
            }
        }
    }

    fn extract_destructuring_def_spans(&self, pattern: &Node<'_>, out: &mut Vec<PathSpan>) {
        let mut cursor = pattern.walk();
        for child in pattern.children(&mut cursor) {
            match child.kind() {
                "shorthand_property_identifier_pattern" | "identifier" => {
                    self.push_simple_lvalue_span(child, out);
                }
                "pair_pattern" => {
                    if let Some(val) = child.child_by_field_name("value") {
                        self.extract_lvalue_spans_from_node(val, out);
                    }
                }
                "rest_pattern" => {
                    let mut inner_cursor = child.walk();
                    for inner in child.children(&mut inner_cursor) {
                        if inner.kind() == "identifier" {
                            self.push_simple_lvalue_span(inner, out);
                        }
                    }
                }
                "object_pattern" | "array_pattern" => {
                    self.extract_destructuring_def_spans(&child, out);
                }
                _ => {}
            }
        }
    }

    /// Collect simple alias assignments within a function: `ptr = dev` where both
    /// sides are plain identifiers. Returns (alias, target, line) triples sorted by line.
    ///
    /// Used by Phase 3 must-alias tracking to resolve `ptr->field` to `dev->field`.
    pub fn collect_alias_assignments(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<(String, String, usize)> {
        let mut aliases = Vec::new();
        self.collect_aliases_inner(*func_node, lines, &mut aliases);
        aliases.sort_by_key(|(_a, _t, line)| *line);
        aliases
    }

    fn collect_aliases_inner(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(String, String, usize)>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line) {
            // Check assignments: ptr = dev
            if self.language.is_assignment_node(node.kind()) {
                if let Some(lhs) = self.language.assignment_target(&node) {
                    if let Some(rhs) = self.language.assignment_value(&node) {
                        let lhs_text = self.node_text(&lhs).to_string();
                        let rhs_text = self.node_text(&rhs).to_string().trim().to_string();
                        if is_plain_ident(&lhs_text) && is_plain_ident(&rhs_text) {
                            out.push((lhs_text, rhs_text, line));
                        }
                    }
                }
            }

            // Check declarations with initializers: type *ptr = dev, let ptr = dev
            if self.language.is_declaration_node(node.kind()) {
                // JS/TS destructuring: const { name, id } = obj → name aliases obj.name
                if self.extract_destructuring_aliases(&node, line, out) {
                    // Destructuring was handled, skip normal declaration path
                } else if let Some(name_node) = self.language.declaration_name(&node) {
                    if let Some(val_node) = self.language.declaration_value(&node) {
                        let name = self.node_text(&name_node).to_string();
                        let val = self.node_text(&val_node).to_string().trim().to_string();
                        if is_plain_ident(&name) && is_plain_ident(&val) {
                            out.push((name, val, line));
                        }
                    }
                }
            }

            // Gap 3: JS/TS for-of/for-in with destructuring patterns
            // `for (const { name, id } of items)` → name aliases items.name
            if node.kind() == "for_in_statement"
                && matches!(
                    self.language,
                    Language::JavaScript | Language::TypeScript | Language::Tsx
                )
            {
                self.extract_for_in_aliases(&node, line, out);
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_aliases_inner(child, lines, out);
        }
    }

    /// Extract aliases from JS/TS destructuring declarations.
    ///
    /// Handles:
    /// - `const { name, id } = obj` → name aliases obj.name, id aliases obj.id
    /// - `const { name: userName } = obj` → userName aliases obj.name
    /// - `const [first, second] = arr` → first aliases arr, second aliases arr
    /// - Nested: `const { config: { host } } = obj` → host aliases obj.config.host
    ///
    /// Returns true if a destructuring pattern was found (even if no aliases emitted).
    fn extract_destructuring_aliases(
        &self,
        node: &Node<'_>,
        line: usize,
        out: &mut Vec<(String, String, usize)>,
    ) -> bool {
        // Only JS/TS have destructuring patterns
        if !matches!(
            self.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) {
            return false;
        }

        // Find variable_declarator children with object_pattern or array_pattern
        let declarator = if node.kind() == "variable_declarator" {
            *node
        } else {
            // lexical_declaration/variable_declaration → variable_declarator
            let mut found = None;
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "variable_declarator" {
                    found = Some(child);
                    break;
                }
            }
            match found {
                Some(d) => d,
                None => return false,
            }
        };

        let name_node = match declarator.child_by_field_name("name") {
            Some(n) => n,
            None => return false,
        };

        if name_node.kind() != "object_pattern" && name_node.kind() != "array_pattern" {
            return false;
        }

        let val_node = match declarator.child_by_field_name("value") {
            Some(v) => v,
            None => return false,
        };

        let rhs = self.node_text(&val_node).to_string().trim().to_string();
        if !is_plain_ident(&rhs) {
            return true; // It's destructuring, but RHS is complex — skip
        }

        self.extract_pattern_aliases(&name_node, &rhs, line, out);
        true
    }

    /// Extract aliases from a JS/TS for-in/for-of statement with destructuring.
    ///
    /// `for (const { name, id } of items)` → name aliases items.name, id aliases items.id
    /// `for (const key in obj)` → key aliases obj (no destructuring, but simple binding)
    fn extract_for_in_aliases(
        &self,
        node: &Node<'_>,
        line: usize,
        out: &mut Vec<(String, String, usize)>,
    ) {
        // Find the iterable (right side): the "right" field of for_in_statement
        let rhs = match node.child_by_field_name("right") {
            Some(r) => r,
            None => return,
        };
        let rhs_text = self.node_text(&rhs).to_string().trim().to_string();
        if !is_plain_ident(&rhs_text) {
            return;
        }

        // Find the pattern or identifier (left side)
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "object_pattern" => {
                    self.extract_pattern_aliases(&child, &rhs_text, line, out);
                }
                "array_pattern" => {
                    self.extract_pattern_aliases(&child, &rhs_text, line, out);
                }
                _ => {}
            }
        }
    }

    /// Recursively extract aliases from a destructuring pattern node.
    fn extract_pattern_aliases(
        &self,
        pattern: &Node<'_>,
        rhs_base: &str,
        line: usize,
        out: &mut Vec<(String, String, usize)>,
    ) {
        if pattern.kind() == "object_pattern" {
            let mut cursor = pattern.walk();
            for child in pattern.children(&mut cursor) {
                match child.kind() {
                    // { name } — shorthand, variable name matches property name
                    "shorthand_property_identifier_pattern" => {
                        let field = self.node_text(&child).to_string();
                        if is_plain_ident(&field) {
                            out.push((field.clone(), format!("{}.{}", rhs_base, field), line));
                        }
                    }
                    // { name: userName } — renamed, or { config: { host } } — nested
                    "pair_pattern" => {
                        if let Some(key_node) = child.child_by_field_name("key") {
                            let key = self.node_text(&key_node).to_string();
                            if let Some(val_node) = child.child_by_field_name("value") {
                                if val_node.kind() == "object_pattern"
                                    || val_node.kind() == "array_pattern"
                                {
                                    // Nested destructuring: { config: { host } }
                                    let nested_base = format!("{}.{}", rhs_base, key);
                                    self.extract_pattern_aliases(
                                        &val_node,
                                        &nested_base,
                                        line,
                                        out,
                                    );
                                } else {
                                    // Renamed: { name: userName }
                                    let alias = self.node_text(&val_node).to_string();
                                    if is_plain_ident(&alias) && is_plain_ident(&key) {
                                        out.push((alias, format!("{}.{}", rhs_base, key), line));
                                    }
                                }
                            }
                        }
                    }
                    // { ...rest } — rest element, aliases the whole object
                    "rest_pattern" => {
                        let mut inner_cursor = child.walk();
                        for inner in child.children(&mut inner_cursor) {
                            if inner.kind() == "identifier" {
                                let name = self.node_text(&inner).to_string();
                                if is_plain_ident(&name) {
                                    out.push((name, rhs_base.to_string(), line));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        } else if pattern.kind() == "array_pattern" {
            // Array destructuring: [first, second] = arr
            // We can't track indices, so alias each to the base array
            let mut cursor = pattern.walk();
            for child in pattern.children(&mut cursor) {
                match child.kind() {
                    "identifier" => {
                        let name = self.node_text(&child).to_string();
                        if is_plain_ident(&name) {
                            out.push((name, rhs_base.to_string(), line));
                        }
                    }
                    "object_pattern" | "array_pattern" => {
                        // Nested pattern inside array — alias to base
                        self.extract_pattern_aliases(&child, rhs_base, line, out);
                    }
                    // Rest element: [...rest] = arr → rest aliases arr
                    "rest_pattern" => {
                        let mut inner_cursor = child.walk();
                        for inner in child.children(&mut inner_cursor) {
                            if inner.kind() == "identifier" {
                                let name = self.node_text(&inner).to_string();
                                if is_plain_ident(&name) {
                                    out.push((name, rhs_base.to_string(), line));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Extract the variable names that are logically written by an lvalue expression.
///
/// Handles three C/C++ indirection patterns:
///
/// - `*p`       → `["p"]`          pointer dereference — the pointer itself is mutated through
/// - `dev->f`   → `["f"]`          field via arrow — track only the qualified field path
/// - `buf[i]`   → `["buf"]`        array subscript — track the base array
/// - `x`        → `["x"]`          simple identifier — unchanged behaviour
///
/// For anything that doesn't match a known pattern (e.g. complex nested expressions)
/// the function returns an empty vec so the caller silently skips it rather than
/// storing an unusable composite name like `"*p"` or `"buf[0]"`.
/// Extract structured AccessPaths from an L-value expression.
///
/// For field access expressions (dev->field, obj.field), returns only the
/// fully qualified path: AccessPath { base: "dev", fields: ["field"] }.
///
/// Phase 2 field-sensitive matching: field assignments no longer emit a
/// base-only def, so `dev->name = x` creates a def for `dev.name` only —
/// taint on `dev.name` does NOT leak to `dev.id` through the base.
pub(super) fn extract_lvalue_paths(lhs_text: &str) -> Vec<AccessPath> {
    let lhs = lhs_text.trim();

    // Pointer dereference: *p, **p
    if lhs.starts_with('*') {
        let inner = lhs.trim_start_matches('*').trim();
        let inner = inner.trim_start_matches('(').trim_end_matches(')').trim();
        if !inner.is_empty() && is_plain_ident(inner) {
            return vec![AccessPath::simple(inner)];
        }
        return vec![];
    }

    // Field via arrow: dev->field or dev->config->timeout
    if lhs.contains("->") {
        let full = AccessPath::from_expr(lhs);
        if full.has_fields() {
            return vec![full];
        }
        let base = AccessPath::simple(full.base.clone());
        return vec![base];
    }

    // Dot access: obj.field
    if lhs.contains('.') {
        let full = AccessPath::from_expr(lhs);
        if full.has_fields() {
            return vec![full];
        }
        let base = AccessPath::simple(full.base.clone());
        return vec![base];
    }

    // Array subscript: buf[i]
    if let Some(bracket) = lhs.find('[') {
        let base_str = lhs[..bracket].trim();
        if !base_str.is_empty() && is_plain_ident(base_str) {
            return vec![AccessPath::from_expr(lhs)];
        }
        return vec![];
    }

    // Simple identifier
    if !lhs.is_empty() && is_plain_ident(lhs) {
        return vec![AccessPath::simple(lhs)];
    }

    vec![]
}
