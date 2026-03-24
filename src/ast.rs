use crate::languages::Language;
use anyhow::{Context, Result};
use std::collections::BTreeSet;
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
        self.find_enclosing_node(self.tree.root_node(), row, &self.language.function_node_types())
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

    fn collect_identifiers_at_row<'a>(
        &self,
        node: Node<'a>,
        row: usize,
        out: &mut Vec<Node<'a>>,
    ) {
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
            // Get the left side of the assignment
            if let Some(lhs) = self.language.assignment_target(&node) {
                let name = self.node_text(&lhs).to_string();
                out.push((name, line));
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

    fn collect_all_identifiers<'a>(
        &self,
        node: Node<'a>,
        out: &mut Vec<(String, usize)>,
    ) {
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

    fn collect_variable_refs(
        &self,
        node: Node<'_>,
        var_name: &str,
        out: &mut BTreeSet<usize>,
    ) {
        if self.language.is_identifier_node(node.kind()) && self.node_text(&node) == var_name {
            out.insert(node.start_position().row + 1);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_variable_refs(child, var_name, out);
        }
    }

    /// Get the line range (1-indexed, inclusive) of a node.
    pub fn node_line_range(&self, node: &Node) -> (usize, usize) {
        (
            node.start_position().row + 1,
            node.end_position().row + 1,
        )
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

    /// Find a function definition by name.
    pub fn find_function_by_name(&self, name: &str) -> Option<Node<'_>> {
        self.find_function_by_name_inner(self.tree.root_node(), name)
    }

    fn find_function_by_name_inner<'a>(
        &self,
        node: Node<'a>,
        name: &str,
    ) -> Option<Node<'a>> {
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
