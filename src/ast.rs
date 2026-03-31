use crate::languages::Language;
use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::{Node, Parser, Tree};

/// Wraps a tree-sitter parse tree with helpers for slicing analysis.
pub struct ParsedFile {
    pub path: String,
    pub source: String,
    pub tree: Tree,
    pub language: Language,
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
        Ok(Self {
            path: path.to_string(),
            source: source.to_string(),
            tree,
            language,
        })
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
}

/// Extract the variable names that are logically written by an lvalue expression.
///
/// Handles three C/C++ indirection patterns:
///
/// - `*p`       → `["p"]`          pointer dereference — the pointer itself is mutated through
/// - `dev->f`   → `["f", "dev"]`   field via arrow — track both the field and the base struct
/// - `buf[i]`   → `["buf"]`        array subscript — track the base array
/// - `x`        → `["x"]`          simple identifier — unchanged behaviour
///
/// For anything that doesn't match a known pattern (e.g. complex nested expressions)
/// the function returns an empty vec so the caller silently skips it rather than
/// storing an unusable composite name like `"*p"` or `"buf[0]"`.
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

/// Return true if `s` looks like a plain C/Rust identifier (alphanumeric + underscore).
#[inline]
fn is_plain_ident(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}
