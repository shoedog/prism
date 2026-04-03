use crate::access_path::AccessPath;
use crate::languages::Language;
use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::{Node, Parser, Tree};

/// Count ERROR and MISSING nodes in a parse tree.
///
/// Returns `(error_count, total_nodes)` so callers can compute an error rate.
/// A high error rate indicates tree-sitter could not parse the source cleanly —
/// common with macro-heavy C/C++ code.
pub fn count_error_nodes(tree: &Tree) -> (usize, usize) {
    let mut error_count = 0usize;
    let mut total_count = 0usize;
    count_nodes_recursive(tree.root_node(), &mut error_count, &mut total_count);
    (error_count, total_count)
}

fn count_nodes_recursive(node: Node<'_>, errors: &mut usize, total: &mut usize) {
    *total += 1;
    if node.is_error() || node.is_missing() {
        *errors += 1;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        count_nodes_recursive(child, errors, total);
    }
}

/// Wraps a tree-sitter parse tree with helpers for slicing analysis.
#[derive(Clone)]
pub struct ParsedFile {
    pub path: String,
    pub source: String,
    pub tree: Tree,
    pub language: Language,
    /// Number of ERROR or MISSING nodes in the parse tree.
    pub parse_error_count: usize,
    /// Total number of nodes in the parse tree.
    pub parse_node_count: usize,
}

impl ParsedFile {
    /// Parse source code in the given language.
    pub fn parse(path: &str, source: &str, language: Language) -> Result<Self> {
        let mut parser = Parser::new();
        let ts_language = language.tree_sitter_language();
        parser
            .set_language(&ts_language)
            .context("Failed to set language")?;
        let tree = parser
            .parse(source, None)
            .context("Failed to parse source")?;
        let (parse_error_count, parse_node_count) = count_error_nodes(&tree);
        Ok(Self {
            path: path.to_string(),
            source: source.to_string(),
            tree,
            language,
            parse_error_count,
            parse_node_count,
        })
    }

    /// Fraction of AST nodes that are ERROR or MISSING (0.0–1.0).
    pub fn error_rate(&self) -> f64 {
        if self.parse_node_count == 0 {
            return 0.0;
        }
        self.parse_error_count as f64 / self.parse_node_count as f64
    }

    /// Get text for a node.
    pub fn node_text(&self, node: &Node) -> &str {
        node.utf8_text(self.source.as_bytes()).unwrap_or("")
    }

    /// Find the smallest function/method node containing the given line (1-indexed).
    pub fn enclosing_function(&self, line: usize) -> Option<Node<'_>> {
        let row = line.saturating_sub(1); // tree-sitter uses 0-indexed rows
        self.find_enclosing_node(
            self.tree.root_node(),
            row,
            &self.language.function_node_types(),
        )
    }

    fn find_enclosing_node<'a>(
        &self,
        node: Node<'a>,
        row: usize,
        types: &[&str],
    ) -> Option<Node<'a>> {
        let start = node.start_position().row;
        let end = node.end_position().row;

        if row < start || row > end {
            return None;
        }

        // Check children first (prefer smallest/deepest match)
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = self.find_enclosing_node(child, row, types) {
                return Some(found);
            }
        }

        if types.contains(&node.kind()) {
            Some(node)
        } else {
            None
        }
    }

    /// Find all function/method definitions in the file.
    pub fn all_functions(&self) -> Vec<Node<'_>> {
        let mut functions = Vec::new();
        self.collect_functions(self.tree.root_node(), &mut functions);
        functions
    }

    fn collect_functions<'a>(&self, node: Node<'a>, out: &mut Vec<Node<'a>>) {
        let types = self.language.function_node_types();
        if types.contains(&node.kind()) {
            out.push(node);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_functions(child, out);
        }
    }

    /// Find all identifiers (variable references) on a given line (1-indexed).
    pub fn identifiers_on_line(&self, line: usize) -> Vec<Node<'_>> {
        let row = line.saturating_sub(1);
        let mut result = Vec::new();
        self.collect_identifiers_at_row(self.tree.root_node(), row, &mut result);
        result
    }

    fn collect_identifiers_at_row<'a>(&self, node: Node<'a>, row: usize, out: &mut Vec<Node<'a>>) {
        if node.start_position().row == row && self.language.is_identifier_node(node.kind()) {
            out.push(node);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_identifiers_at_row(child, row, out);
        }
    }

    /// Find all assignment targets (L-values) on diff lines within a function scope.
    pub fn assignment_lvalues_on_lines(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<(String, usize)> {
        let mut lvalues = Vec::new();
        self.collect_assignments(*func_node, lines, &mut lvalues);
        lvalues
    }

    fn collect_assignments(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(String, usize)>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line) && self.language.is_assignment_node(node.kind()) {
            // Get the left side of the assignment, extracting alias names so that
            // pointer derefs (*p), field accesses (dev->field), and array subscripts
            // (buf[i]) create flow edges for their base variables.
            if let Some(lhs) = self.language.assignment_target(&node) {
                let lhs_text = self.node_text(&lhs).to_string();
                for name in extract_lvalue_names(&lhs_text) {
                    out.push((name, line));
                }
            }
        }

        // Also check variable declarations with initializers on diff lines
        if lines.contains(&line) && self.language.is_declaration_node(node.kind()) {
            if let Some(name_node) = self.language.declaration_name(&node) {
                let name = self.node_text(&name_node).to_string();
                out.push((name, line));
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_assignments(child, lines, out);
        }
    }

    /// Like `assignment_lvalues_on_lines`, but returns structured `AccessPath`s
    /// instead of plain variable name strings. Used by the DFG for field-sensitive tracking.
    pub fn assignment_lvalue_paths_on_lines(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<(AccessPath, usize)> {
        let mut paths = Vec::new();
        self.collect_assignment_paths(*func_node, lines, &mut paths);
        paths
    }

    fn collect_assignment_paths(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(AccessPath, usize)>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line) && self.language.is_assignment_node(node.kind()) {
            if let Some(lhs) = self.language.assignment_target(&node) {
                // Multi-target: pattern_list (Python: name, age = func())
                // or expression_list (Go: val, err := func())
                // Split into individual identifiers instead of treating as one string.
                if lhs.kind() == "pattern_list" || lhs.kind() == "expression_list" {
                    self.extract_multi_target_lvalues(&lhs, line, out);
                } else {
                    let lhs_text = self.node_text(&lhs).to_string();
                    for path in extract_lvalue_paths(&lhs_text) {
                        out.push((path, line));
                    }
                }
            }
        }

        if lines.contains(&line) && self.language.is_declaration_node(node.kind()) {
            if let Some(name_node) = self.language.declaration_name(&node) {
                // Go declarations can also have expression_list as the name
                if name_node.kind() == "expression_list" {
                    self.extract_multi_target_lvalues(&name_node, line, out);
                } else {
                    let name = self.node_text(&name_node).to_string();
                    out.push((AccessPath::simple(name), line));
                }
            }
        }

        // Gap 3: JS/TS for-of/for-in loop variable bindings
        // `for (const { name, id } of items)` or `for (const key in obj)`
        // The loop variable isn't inside a standard declaration node.
        if lines.contains(&line)
            && node.kind() == "for_in_statement"
            && matches!(
                self.language,
                Language::JavaScript | Language::TypeScript | Language::Tsx
            )
        {
            self.extract_for_in_lvalues(&node, line, out);
        }

        // Gap 4: Python with...as and except...as bindings
        // `with open("f") as f:` or `except Exception as e:`
        // The as_pattern_target is the bound variable.
        if lines.contains(&line)
            && node.kind() == "as_pattern"
            && matches!(self.language, Language::Python)
        {
            let mut c = node.walk();
            for child in node.children(&mut c) {
                if child.kind() == "as_pattern_target" {
                    let name = self.node_text(&child).to_string();
                    if is_plain_ident(&name) {
                        out.push((AccessPath::simple(name), line));
                    }
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_assignment_paths(child, lines, out);
        }
    }

    /// Extract individual L-value paths from a multi-target node (pattern_list, expression_list).
    ///
    /// Handles:
    /// - Go: `val, err := getData()` — expression_list with identifier children
    /// - Python: `name, age = get_user()` — pattern_list with identifier children
    /// - Python: `first, *rest = items` — pattern_list with identifier + list_splat_pattern
    /// - Go: `for key, value := range m` — expression_list in range clause
    fn extract_multi_target_lvalues(
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

    /// Extract L-value defs from a JS/TS `for_in_statement` loop header.
    ///
    /// Handles:
    /// - `for (const key in obj)` → def for "key"
    /// - `for (const { name, id } of items)` → defs for "name", "id"
    /// - `for (const [a, b] of pairs)` → defs for "a", "b"
    fn extract_for_in_lvalues(
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
                    if is_plain_ident(&name) && name != "const" && name != "let" && name != "var" {
                        out.push((AccessPath::simple(name), line));
                    }
                }
                // Destructuring: { name, id } or [a, b]
                "object_pattern" | "array_pattern" => {
                    self.extract_destructuring_defs(&child, line, out);
                }
                _ => {}
            }
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

    /// Like `rvalue_identifiers_on_lines`, but returns structured `AccessPath`s.
    /// Used by the DFG for field-sensitive tracking.
    pub fn rvalue_identifier_paths_on_lines(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<(AccessPath, usize)> {
        let mut paths = Vec::new();
        self.collect_rvalue_paths(*func_node, lines, &mut paths);
        paths
    }

    fn collect_rvalue_paths(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(AccessPath, usize)>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line) && self.language.is_assignment_node(node.kind()) {
            if let Some(rhs) = self.language.assignment_value(&node) {
                self.collect_identifier_paths(rhs, out);
            }
        }

        if lines.contains(&line) && self.language.is_call_node(node.kind()) {
            if let Some(args) = self.language.call_arguments(&node) {
                self.collect_identifier_paths(args, out);
            }
            if let Some(func_name_node) = self.language.call_function_name(&node) {
                let name = self.node_text(&func_name_node).to_string();
                out.push((AccessPath::simple(name), line));
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_rvalue_paths(child, lines, out);
        }
    }

    /// Check if a node kind represents a field/member access expression.
    /// Each language has a different tree-sitter node kind for this:
    /// - C/C++, Rust: field_expression
    /// - JS/TS: member_expression
    /// - Go: selector_expression
    /// - Python: attribute
    /// - Java: field_access
    /// - Lua: dot_index_expression, method_index_expression (colon syntax)
    fn is_field_access_node(kind: &str) -> bool {
        matches!(
            kind,
            "field_expression"
                | "member_expression"
                | "selector_expression"
                | "attribute"
                | "field_access"
                | "dot_index_expression"
                | "method_index_expression"
        )
    }

    fn collect_identifier_paths<'a>(&self, node: Node<'a>, out: &mut Vec<(AccessPath, usize)>) {
        // Check for field/member access expressions — emit the full qualified
        // path instead of individual identifiers.
        if Self::is_field_access_node(node.kind()) {
            let text = self.node_text(&node).to_string();
            let line = node.start_position().row + 1;
            out.push((AccessPath::from_expr(&text), line));
            // Also emit the base identifier for field-insensitive fallback.
            // Different tree-sitter grammars use different field names:
            //   C/C++/Rust: "argument", Go: "operand", JS/TS/Python/Java: "object"
            //   Lua: first named child (no standard field name)
            let base_node = node
                .child_by_field_name("argument")
                .or_else(|| node.child_by_field_name("object"))
                .or_else(|| node.child_by_field_name("operand"))
                .or_else(|| node.named_child(0));
            if let Some(base) = base_node {
                if self.language.is_identifier_node(base.kind()) {
                    let base_name = self.node_text(&base).to_string();
                    out.push((AccessPath::simple(base_name), line));
                }
            }
            return; // Don't recurse into children — we've handled them
        }

        if self.language.is_identifier_node(node.kind()) {
            let name = self.node_text(&node).to_string();
            let line = node.start_position().row + 1;
            out.push((AccessPath::simple(name), line));
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_identifier_paths(child, out);
        }
    }

    /// Find all R-value identifiers on diff lines within a function (excluding L-values).
    pub fn rvalue_identifiers_on_lines(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<(String, usize)> {
        let mut rvalues = Vec::new();
        self.collect_rvalues(*func_node, lines, &mut rvalues);
        rvalues
    }

    fn collect_rvalues(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(String, usize)>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line) && self.language.is_assignment_node(node.kind()) {
            // Get the right side of the assignment
            if let Some(rhs) = self.language.assignment_value(&node) {
                self.collect_all_identifiers(rhs, out);
            }
        }

        // Function call arguments on diff lines
        if lines.contains(&line) && self.language.is_call_node(node.kind()) {
            if let Some(args) = self.language.call_arguments(&node) {
                self.collect_all_identifiers(args, out);
            }
            // Also capture the function name being called
            if let Some(func_name_node) = self.language.call_function_name(&node) {
                let name = self.node_text(&func_name_node).to_string();
                out.push((name, line));
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_rvalues(child, lines, out);
        }
    }

    fn collect_all_identifiers<'a>(&self, node: Node<'a>, out: &mut Vec<(String, usize)>) {
        if self.language.is_identifier_node(node.kind()) {
            let name = self.node_text(&node).to_string();
            let line = node.start_position().row + 1;
            out.push((name, line));
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_all_identifiers(child, out);
        }
    }

    /// Find all lines in a function scope where a variable name is referenced.
    pub fn find_variable_references(
        &self,
        func_node: &Node<'_>,
        var_name: &str,
    ) -> BTreeSet<usize> {
        let mut lines = BTreeSet::new();
        self.collect_variable_refs(*func_node, var_name, &mut lines);
        lines
    }

    fn collect_variable_refs(&self, node: Node<'_>, var_name: &str, out: &mut BTreeSet<usize>) {
        if self.language.is_identifier_node(node.kind()) && self.node_text(&node) == var_name {
            out.insert(node.start_position().row + 1);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_variable_refs(child, var_name, out);
        }
    }

    /// Find variable references with basic scope awareness.
    ///
    /// Like `find_variable_references`, but filters out references that lie inside
    /// an inner scope block which re-declares the same variable name — i.e., the
    /// reference would be bound to the inner declaration, not the one at `def_line`.
    pub fn find_variable_references_scoped(
        &self,
        func_node: &Node<'_>,
        var_name: &str,
        def_line: usize,
    ) -> BTreeSet<usize> {
        let mut lines = BTreeSet::new();
        self.collect_variable_refs_scoped(*func_node, var_name, def_line, &mut lines);
        lines
    }

    fn collect_variable_refs_scoped(
        &self,
        node: Node<'_>,
        var_name: &str,
        def_line: usize,
        out: &mut BTreeSet<usize>,
    ) {
        let node_start = node.start_position().row + 1;
        let node_end = node.end_position().row + 1;

        // If this is a scope block that does NOT contain the definition,
        // and it re-declares the variable, skip it entirely — references inside
        // bind to the inner declaration, not the one we're tracking.
        if self.language.is_scope_block(node.kind()) {
            let def_in_scope = def_line >= node_start && def_line <= node_end;
            if !def_in_scope && self.scope_has_declaration(node, var_name) {
                return;
            }
        }

        if self.language.is_identifier_node(node.kind()) && self.node_text(&node) == var_name {
            out.insert(node.start_position().row + 1);
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_variable_refs_scoped(child, var_name, def_line, out);
        }
    }

    /// Check whether a scope block directly declares a variable with the given name.
    /// Does not recurse into nested scope blocks (those have their own scope).
    fn scope_has_declaration(&self, node: Node<'_>, var_name: &str) -> bool {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if self.language.is_declaration_node(child.kind()) {
                if let Some(name_node) = self.language.declaration_name(&child) {
                    if self.node_text(&name_node) == var_name {
                        return true;
                    }
                }
            }
            // Recurse into non-scope children (e.g., for-loop init, expression statements)
            // but stop at nested scope blocks to avoid false positives.
            if !self.language.is_scope_block(child.kind())
                && self.scope_has_declaration(child, var_name)
            {
                return true;
            }
        }
        false
    }

    /// Find all references to an AccessPath within a function scope.
    ///
    /// For simple paths (no fields), delegates to `find_variable_references_scoped`.
    /// For field-qualified paths (`dev->name`), searches for matching field_expression
    /// nodes as well as bare identifier references to the base.
    pub fn find_path_references_scoped(
        &self,
        func_node: &Node<'_>,
        path: &AccessPath,
        def_line: usize,
    ) -> BTreeSet<usize> {
        if path.is_simple() {
            return self.find_variable_references_scoped(func_node, &path.base, def_line);
        }
        // For field-qualified paths, find matching field expressions.
        // Filter to references after def_line, matching the scoping behavior
        // of find_variable_references_scoped for simple paths.
        let mut lines = BTreeSet::new();
        self.collect_path_refs(*func_node, path, def_line, &mut lines);
        lines
    }

    fn collect_path_refs(
        &self,
        node: Node<'_>,
        path: &AccessPath,
        def_line: usize,
        out: &mut BTreeSet<usize>,
    ) {
        let line = node.start_position().row + 1;

        // Check field/member access expressions (all languages)
        if Self::is_field_access_node(node.kind()) {
            let text = self.node_text(&node).to_string();
            let node_path = AccessPath::from_expr(&text);
            if node_path == *path && line > def_line {
                out.insert(line);
                return; // Don't recurse into matched field expression
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_path_refs(child, path, def_line, out);
        }
    }

    /// Get the line range (1-indexed, inclusive) of a node.
    pub fn node_line_range(&self, node: &Node) -> (usize, usize) {
        (node.start_position().row + 1, node.end_position().row + 1)
    }

    /// Find condition variables in control flow statements on the given lines.
    pub fn condition_variables_on_lines(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<(String, usize)> {
        let mut vars = Vec::new();
        self.collect_condition_vars(*func_node, lines, &mut vars);
        vars
    }

    fn collect_condition_vars(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(String, usize)>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line) && self.language.is_control_flow_node(node.kind()) {
            if let Some(condition) = self.language.control_flow_condition(&node) {
                self.collect_all_identifiers(condition, out);
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_condition_vars(child, lines, out);
        }
    }

    /// Find all return statements within a function.
    pub fn return_statements(&self, func_node: &Node<'_>) -> Vec<usize> {
        let mut lines = Vec::new();
        self.collect_returns(*func_node, &mut lines);
        lines
    }

    fn collect_returns(&self, node: Node<'_>, out: &mut Vec<usize>) {
        if self.language.is_return_node(node.kind()) {
            out.push(node.start_position().row + 1);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_returns(child, out);
        }
    }

    /// Collect all statement-level nodes within a function for CFG construction.
    ///
    /// Returns `(line, node_kind)` pairs in source order. Only direct children of
    /// the function body and top-level children of compound statements are included
    /// — nested expressions within a statement are not separate CFG nodes.
    pub fn statements_in_function(&self, func_node: &Node<'_>) -> Vec<(usize, String)> {
        let mut stmts = Vec::new();
        // Find the function body (compound_statement, block, etc.)
        let body = func_node
            .child_by_field_name("body")
            .or_else(|| func_node.child_by_field_name("consequence"));
        if let Some(body_node) = body {
            self.collect_statements(body_node, &mut stmts);
        }
        stmts.sort_by_key(|(line, _)| *line);
        stmts.dedup_by_key(|(line, _)| *line);
        stmts
    }

    fn collect_statements(&self, node: Node<'_>, out: &mut Vec<(usize, String)>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind();
            let line = child.start_position().row + 1;

            if self.language.is_statement_node(kind) {
                out.push((line, kind.to_string()));

                // For control flow nodes, also recurse into their bodies
                // to find nested statements (then-branch, else-branch, loop body)
                if self.language.is_control_flow_node(kind) {
                    self.collect_nested_statements(child, out);
                }
            } else if kind == "compound_statement" || kind == "block" || kind == "statement_block" {
                // Recurse into blocks
                self.collect_statements(child, out);
            }
        }
    }

    fn collect_nested_statements(&self, node: Node<'_>, out: &mut Vec<(usize, String)>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind();
            if kind == "compound_statement"
                || kind == "block"
                || kind == "statement_block"
                || kind == "else_clause"
                || kind == "elif_clause"
                || kind == "else_if_clause"
                || kind == "switch_body"
                || kind == "case_statement"
                || kind == "default_statement"
                || kind == "match_block"
                || kind == "match_arm"
            {
                self.collect_statements(child, out);
                self.collect_nested_statements(child, out);
            } else if self.language.is_control_flow_node(kind) {
                // Nested control flow (if inside if, etc.)
                let line = child.start_position().row + 1;
                out.push((line, kind.to_string()));
                self.collect_nested_statements(child, out);
            }
        }
    }

    /// Find all goto statements in a function node.
    /// Returns `(target_label, goto_line)` pairs.
    pub fn goto_statements(&self, func_node: &Node<'_>) -> Vec<(String, usize)> {
        let mut gotos = Vec::new();
        self.collect_gotos(*func_node, &mut gotos);
        gotos
    }

    fn collect_gotos(&self, node: Node<'_>, out: &mut Vec<(String, usize)>) {
        if node.kind() == "goto_statement" {
            // tree-sitter-c: goto_statement has a "label" field with the target name
            if let Some(label_node) = node.child_by_field_name("label") {
                let label = self.node_text(&label_node).to_string();
                let line = node.start_position().row + 1;
                out.push((label, line));
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_gotos(child, out);
        }
    }

    /// Find all label definitions in a function node.
    /// Returns `(label_name, label_line, section_end_line)` triples.
    /// `section_end_line` is the line of the next label or the function end,
    /// representing the code section "owned" by this label.
    pub fn label_sections(&self, func_node: &Node<'_>) -> Vec<(String, usize, usize)> {
        let mut labels: Vec<(String, usize)> = Vec::new();
        self.collect_labels(*func_node, &mut labels);

        let func_end = func_node.end_position().row + 1;

        // Sort labels by line number to determine sections
        labels.sort_by_key(|(_, line)| *line);

        let mut sections = Vec::new();
        for i in 0..labels.len() {
            let (ref name, start) = labels[i];
            let end = if i + 1 < labels.len() {
                labels[i + 1].1.saturating_sub(1)
            } else {
                func_end
            };
            sections.push((name.clone(), start, end));
        }
        sections
    }

    fn collect_labels(&self, node: Node<'_>, out: &mut Vec<(String, usize)>) {
        if node.kind() == "labeled_statement" {
            // tree-sitter-c: labeled_statement has a "label" field
            if let Some(label_node) = node.child_by_field_name("label") {
                let label = self.node_text(&label_node).to_string();
                let line = node.start_position().row + 1;
                out.push((label, line));
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_labels(child, out);
        }
    }

    /// Find the enclosing control flow block (if/for/while) for a given line,
    /// and return its start and end lines.
    pub fn enclosing_branch(&self, line: usize) -> Option<(usize, usize)> {
        let row = line.saturating_sub(1);
        self.find_enclosing_branch(self.tree.root_node(), row)
    }

    fn find_enclosing_branch(&self, node: Node<'_>, row: usize) -> Option<(usize, usize)> {
        let start = node.start_position().row;
        let end = node.end_position().row;

        if row < start || row > end {
            return None;
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = self.find_enclosing_branch(child, row) {
                return Some(found);
            }
        }

        if self.language.is_control_flow_node(node.kind()) {
            Some((start + 1, end + 1))
        } else {
            None
        }
    }

    /// Find function calls on the given lines and return the called function names.
    pub fn function_calls_on_lines(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<(String, usize)> {
        let mut calls = Vec::new();
        self.collect_calls(*func_node, lines, &mut calls);
        calls
    }

    fn collect_calls(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(String, usize)>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line) && self.language.is_call_node(node.kind()) {
            if let Some(name_node) = self.language.call_function_name(&node) {
                let name = self.node_text(&name_node).to_string();
                out.push((name, line));
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_calls(child, lines, out);
        }
    }

    /// Collect all function call names on specific lines (1-indexed).
    /// Returns a map from line number to list of called function names found on that line.
    /// Only matches actual AST call nodes — ignores calls inside comments or string literals.
    pub fn call_names_on_lines(&self, lines: &[usize]) -> BTreeMap<usize, Vec<String>> {
        let mut result: BTreeMap<usize, Vec<String>> = BTreeMap::new();
        let line_set: BTreeSet<usize> = lines.iter().copied().collect();
        self.collect_call_names_at_lines(self.tree.root_node(), &line_set, &mut result);
        result
    }

    fn collect_call_names_at_lines(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut BTreeMap<usize, Vec<String>>,
    ) {
        let line = node.start_position().row + 1;
        if self.language.is_call_node(node.kind()) && lines.contains(&line) {
            if let Some(name_node) = self.language.call_function_name(&node) {
                let name = self.node_text(&name_node).to_string();
                out.entry(line).or_default().push(name);
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_call_names_at_lines(child, lines, out);
        }
    }

    /// Find a function definition by name.
    pub fn find_function_by_name(&self, name: &str) -> Option<Node<'_>> {
        self.find_function_by_name_inner(self.tree.root_node(), name)
    }

    fn find_function_by_name_inner<'a>(&self, node: Node<'a>, name: &str) -> Option<Node<'a>> {
        let types = self.language.function_node_types();
        if types.contains(&node.kind()) {
            if let Some(name_node) = self.language.function_name(&node) {
                if self.node_text(&name_node) == name {
                    return Some(node);
                }
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = self.find_function_by_name_inner(child, name) {
                return Some(found);
            }
        }
        None
    }

    /// Extract the ordered parameter names from a function definition node.
    ///
    /// Walks the tree-sitter AST to find `parameter_declaration` (C/C++),
    /// `parameter` (Go/Rust), or `identifier` children of the parameters node.
    /// Returns the names in declaration order.
    pub fn function_parameter_names(&self, func_node: &Node<'_>) -> Vec<String> {
        let mut names = Vec::new();
        // Find the parameters node — could be in a declarator chain (C/C++) or direct
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

    /// Find the parameters node within a function definition.
    /// Handles the C/C++ declarator chain (function_definition → declarator → function_declarator → parameters).
    fn find_parameters_node<'a>(&self, node: &Node<'a>) -> Option<Node<'a>> {
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
    fn extract_param_name(&self, node: &Node<'_>) -> Option<String> {
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
            _ => None,
        }
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

    /// Extract the Nth argument expression text from a call expression on a given line.
    ///
    /// Searches for call expressions on the specified line, then returns the text
    /// of the argument at `arg_index` (0-based).
    pub fn call_argument_text_at(
        &self,
        line: usize,
        callee_name: &str,
        arg_index: usize,
    ) -> Option<String> {
        self.find_call_arg_at(self.tree.root_node(), line, callee_name, arg_index)
    }

    fn find_call_arg_at(
        &self,
        node: Node<'_>,
        line: usize,
        callee_name: &str,
        arg_index: usize,
    ) -> Option<String> {
        let node_line = node.start_position().row + 1;

        if node_line == line && self.language.is_call_node(node.kind()) {
            if let Some(name_node) = self.language.call_function_name(&node) {
                let name = self.node_text(&name_node);
                if name == callee_name {
                    if let Some(args_node) = self.language.call_arguments(&node) {
                        // Count non-punctuation children to find the Nth argument
                        let mut arg_idx = 0;
                        let mut cursor = args_node.walk();
                        for child in args_node.children(&mut cursor) {
                            // Skip punctuation: ( ) , and whitespace
                            if child.is_named() {
                                if arg_idx == arg_index {
                                    let text = self.node_text(&child).trim().to_string();
                                    // Strip address-of operator
                                    let text = text.trim_start_matches('&').to_string();
                                    return Some(text);
                                }
                                arg_idx += 1;
                            }
                        }
                    }
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(result) = self.find_call_arg_at(child, line, callee_name, arg_index) {
                return Some(result);
            }
        }
        None
    }

    /// Check if a function node has a variadic parameter (`...`).
    ///
    /// In C/C++, tree-sitter represents `...` as a `variadic_parameter` node
    /// inside the parameter list.
    pub fn is_variadic_function(&self, func_node: &Node<'_>) -> bool {
        if let Some(params) = self.find_parameters_node(func_node) {
            let mut cursor = params.walk();
            for child in params.children(&mut cursor) {
                if child.kind() == "variadic_parameter" {
                    return true;
                }
            }
        }
        false
    }

    /// Find all call expression names inside a function body.
    /// Returns deduplicated list of callee names.
    pub fn callees_in_function(&self, func_node: &Node<'_>) -> Vec<String> {
        let mut names = BTreeSet::new();
        self.collect_all_callees(*func_node, &mut names);
        names.into_iter().collect()
    }

    fn collect_all_callees(&self, node: Node<'_>, out: &mut BTreeSet<String>) {
        if self.language.is_call_node(node.kind()) {
            if let Some(name_node) = self.language.call_function_name(&node) {
                out.insert(self.node_text(&name_node).to_string());
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_all_callees(child, out);
        }
    }
}

/// Scan a function's source text for assignments of the form `var_name = func_name`
/// where `func_name` is a known function. Returns the last such assignment's RHS
/// (simple must-alias within a single function).
///
/// Used by call graph Phase 3 to resolve local function pointer variables.
pub fn resolve_fptr_assignment(
    func_source: &str,
    var_name: &str,
    known_fns: &BTreeSet<String>,
) -> Option<String> {
    let mut resolved = None;
    for line in func_source.lines() {
        let trimmed = line.trim();
        // Match: var_name = identifier  (with optional trailing semicolon/comma)
        // Also match initialization: type (*var_name)(...) = identifier;
        //   and typedef:  callback_fn var_name = identifier;

        // Strategy: find `var_name` followed by `=` (but not `==`), then extract RHS identifier
        if let Some(eq_pos) = trimmed.find('=') {
            // Skip ==, !=, <=, >=
            if eq_pos + 1 < trimmed.len() {
                let after_eq = trimmed.as_bytes().get(eq_pos + 1);
                if after_eq == Some(&b'=') {
                    continue;
                }
            }
            if eq_pos > 0 {
                let before_eq = trimmed.as_bytes().get(eq_pos - 1);
                if before_eq == Some(&b'!') || before_eq == Some(&b'<') || before_eq == Some(&b'>')
                {
                    continue;
                }
            }

            let lhs = trimmed[..eq_pos].trim();
            let rhs = trimmed[eq_pos + 1..].trim().trim_end_matches(';').trim();

            // Check if LHS contains var_name as the assigned variable
            // Handles: `var_name =`, `type var_name =`, `type (*var_name)(args) =`
            let lhs_has_var = lhs == var_name
                || lhs.ends_with(&format!(" {}", var_name))
                || lhs.ends_with(&format!("*{}", var_name))
                || lhs.contains(&format!("(*{})", var_name))
                || lhs.contains(&format!(" {} ", var_name));

            if !lhs_has_var {
                continue;
            }

            // RHS should be a plain identifier (possibly with & prefix for address-of)
            let rhs_name = rhs.trim_start_matches('&');
            if !rhs_name.is_empty()
                && rhs_name.chars().all(|c| c.is_alphanumeric() || c == '_')
                && known_fns.contains(rhs_name)
            {
                resolved = Some(rhs_name.to_string());
            }
        }
    }
    resolved
}

/// Scan a function (or file-scope) source for an array initializer of the form:
///   `type array_name[] = { func_a, func_b, func_c };`
/// Returns all known function names that appear in the initializer list.
///
/// Used by call graph Phase 3 to resolve dispatch table calls.
pub fn resolve_array_dispatch(
    source: &str,
    array_name: &str,
    known_fns: &BTreeSet<String>,
) -> Vec<String> {
    let mut targets = Vec::new();

    // Find the array initializer: look for `array_name` followed by `[` ... `]` ... `=` ... `{`
    // This is a text heuristic — we look for lines containing the array name and an initializer
    let lines: Vec<&str> = source.lines().collect();
    let mut in_initializer = false;
    let mut brace_depth = 0;

    for line in &lines {
        let trimmed = line.trim();

        if !in_initializer {
            // Look for: array_name ... [] = { or array_name ... [N] = {
            if trimmed.contains(array_name) && trimmed.contains('[') && trimmed.contains('=') {
                in_initializer = true;
                // Count braces on this line
                for ch in trimmed.chars() {
                    match ch {
                        '{' => brace_depth += 1,
                        '}' => {
                            brace_depth -= 1;
                            if brace_depth <= 0 {
                                in_initializer = false;
                            }
                        }
                        _ => {}
                    }
                }
                // Extract identifiers from this line's initializer portion
                if let Some(brace_start) = trimmed.find('{') {
                    extract_fn_names_from_init(&trimmed[brace_start..], known_fns, &mut targets);
                }
                continue;
            }
        }

        if in_initializer {
            for ch in trimmed.chars() {
                match ch {
                    '{' => brace_depth += 1,
                    '}' => {
                        brace_depth -= 1;
                        if brace_depth <= 0 {
                            in_initializer = false;
                        }
                    }
                    _ => {}
                }
            }
            extract_fn_names_from_init(trimmed, known_fns, &mut targets);
        }
    }

    targets
}

/// Extract known function names from an initializer fragment like `{func_a, func_b, .field = func_c}`.
fn extract_fn_names_from_init(text: &str, known_fns: &BTreeSet<String>, out: &mut Vec<String>) {
    // Split on commas and braces, then check each token
    for token in text.split(|c: char| c == ',' || c == '{' || c == '}' || c == '(' || c == ')') {
        let token = token.trim();
        // Handle designated initializers: `.field = func_name`
        let ident = if let Some(eq_pos) = token.find('=') {
            let rhs = token[eq_pos + 1..].trim();
            // Skip ==
            if token.as_bytes().get(eq_pos + 1) == Some(&b'=') {
                continue;
            }
            rhs
        } else {
            token
        };
        let ident = ident.trim_start_matches('&');
        if !ident.is_empty()
            && ident.chars().all(|c| c.is_alphanumeric() || c == '_')
            && known_fns.contains(ident)
            && !out.contains(&ident.to_string())
        {
            out.push(ident.to_string());
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
fn extract_lvalue_paths(lhs_text: &str) -> Vec<AccessPath> {
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

fn extract_lvalue_names(lhs_text: &str) -> Vec<String> {
    let lhs = lhs_text.trim();

    // Pointer dereference: *p, **p
    if lhs.starts_with('*') {
        let inner = lhs.trim_start_matches('*').trim();
        // Strip surrounding parens: (*p)
        let inner = inner.trim_start_matches('(').trim_end_matches(')').trim();
        if !inner.is_empty() && is_plain_ident(inner) {
            return vec![inner.to_string()];
        }
        return vec![];
    }

    // Field via arrow: dev->field
    if let Some(arrow) = lhs.find("->") {
        let base = lhs[..arrow].trim();
        let field = lhs[arrow + 2..].trim();
        let mut names = Vec::new();
        if !field.is_empty() && is_plain_ident(field) {
            names.push(field.to_string());
        }
        if !base.is_empty() && is_plain_ident(base) {
            names.push(base.to_string());
        }
        return names;
    }

    // Array subscript: buf[i]  — only track the base name
    if let Some(bracket) = lhs.find('[') {
        let base = lhs[..bracket].trim();
        if !base.is_empty() && is_plain_ident(base) {
            return vec![base.to_string()];
        }
        return vec![];
    }

    // Simple identifier (also covers `obj.field` by treating `obj` as the def).
    // We intentionally ignore dot access for non-pointer structs here; the base
    // identifier appears as a separate rvalue and will be tracked via rvalue edges.
    if !lhs.is_empty() && is_plain_ident(lhs) {
        return vec![lhs.to_string()];
    }

    vec![]
}

fn is_plain_ident(s: &str) -> bool {
    !s.is_empty()
        && s.chars().all(|c| c.is_alphanumeric() || c == '_')
        && s.chars()
            .next()
            .map_or(false, |c| c.is_alphabetic() || c == '_')
}
