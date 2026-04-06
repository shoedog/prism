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

/// Information about a single return statement within a function.
#[derive(Debug, Clone)]
pub struct ReturnInfo {
    /// Line number of the return statement (1-indexed).
    pub line: usize,
    /// The return value expression text, or None for bare `return`/`return;`.
    pub value_text: Option<String>,
    /// The tree-sitter node kind of the return value expression.
    pub value_kind: Option<String>,
    /// Whether this return is inside a conditional branch.
    pub is_conditional: bool,
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
    /// Byte offset of each line start (0-indexed by line number).
    /// `line_offsets[i]` is the byte offset where line `i+1` begins (1-indexed lines).
    line_offsets: Vec<usize>,
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
        // Precompute line→byte offset table for O(1) lookup in line_has_code_text.
        let mut line_offsets = vec![0usize]; // Line 1 starts at byte 0.
        for (i, &b) in source.as_bytes().iter().enumerate() {
            if b == b'\n' {
                line_offsets.push(i + 1);
            }
        }
        Ok(Self {
            path: path.to_string(),
            source: source.to_string(),
            tree,
            language,
            parse_error_count,
            parse_node_count,
            line_offsets,
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
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        // Use compiled tree-sitter query when available (faster: skips irrelevant subtrees).
        if let Some(query) = get_query(self.language, QueryKind::Functions) {
            let func_idx = query
                .capture_index_for_name("func")
                .expect("Functions query must have @func capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut functions = Vec::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == func_idx {
                        functions.push(capture.node);
                    }
                }
            }
            return functions;
        }

        // Fallback: manual recursive walk.
        let mut functions = Vec::new();
        self.collect_functions_manual(self.tree.root_node(), &mut functions);
        functions
    }

    /// Manual recursive function collection (pre-query fallback).
    /// `pub(crate)` for dual-path consistency testing in `queries::tests`.
    pub(crate) fn collect_functions_manual<'a>(&self, node: Node<'a>, out: &mut Vec<Node<'a>>) {
        let types = self.language.function_node_types();
        if types.contains(&node.kind()) {
            out.push(node);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_functions_manual(child, out);
        }
    }

    /// Find all identifiers (variable references) on a given line (1-indexed).
    ///
    /// Uses the manual recursive walk because it needs to match the broad set
    /// of node types in `Language::is_identifier_node()` (property_identifier,
    /// field_identifier, etc.), not just the core `identifier` type.
    pub fn identifiers_on_line(&self, line: usize) -> Vec<Node<'_>> {
        let row = line.saturating_sub(1);
        let mut result = Vec::new();
        self.collect_identifiers_at_row(self.tree.root_node(), row, &mut result);
        result
    }

    /// Manual recursive identifier-at-row collection (pre-query fallback).
    fn collect_identifiers_at_row<'a>(&self, node: Node<'a>, row: usize, out: &mut Vec<Node<'a>>) {
        if node.start_position().row == row && self.language.is_identifier_node(node.kind()) {
            out.push(node);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_identifiers_at_row(child, row, out);
        }
    }

    /// Check whether `text` appears on `line` (1-indexed) in actual code,
    /// i.e. NOT inside a comment or string literal AST node.
    ///
    /// For each occurrence of `text` on the line, we ask tree-sitter for the
    /// smallest node covering that byte range. If every occurrence lands in a
    /// comment or string, returns `false`.
    pub fn line_has_code_text(&self, line: usize, text: &str) -> bool {
        if text.is_empty() {
            return false;
        }
        let row = line.saturating_sub(1);
        let source = self.source.as_bytes();

        // O(1) line→byte offset lookup via precomputed table.
        let line_start = self.line_offsets.get(row).copied().unwrap_or(source.len());
        let line_end = self
            .line_offsets
            .get(row + 1)
            .map(|&off| off.saturating_sub(1)) // exclude the newline
            .unwrap_or(source.len());
        let line_bytes = &source[line_start..line_end];
        let line_str = std::str::from_utf8(line_bytes).unwrap_or("");

        // For each occurrence of `text` in this line, check the AST node.
        let text_bytes = text.as_bytes();
        let mut search_start = 0;
        while let Some(pos) = line_str[search_start..].find(text) {
            let abs_start = line_start + search_start + pos;
            let abs_end = abs_start + text_bytes.len();

            // Find the smallest AST node covering this byte range.
            let node = self
                .tree
                .root_node()
                .descendant_for_byte_range(abs_start, abs_end);
            if let Some(n) = node {
                // Walk up to check if any ancestor (up to the line boundary) is
                // a comment or string. The immediate node might be an identifier
                // inside a string interpolation, so we check ancestors too.
                if !self.is_inside_comment_or_string(n) {
                    return true; // At least one occurrence is in real code.
                }
            } else {
                // No AST node found — conservative: treat as code.
                return true;
            }
            search_start += pos + text_bytes.len();
        }
        false
    }

    /// Walk up from `node` to check if it (or any ancestor) is a comment or
    /// string literal node.
    fn is_inside_comment_or_string(&self, node: Node<'_>) -> bool {
        let mut current = Some(node);
        while let Some(n) = current {
            if self.language.is_comment_or_string_node(n.kind()) {
                return true;
            }
            current = n.parent();
        }
        false
    }

    /// Extract import bindings from the file.
    ///
    /// Returns a map of `alias → module_path` where:
    /// - Python: `import utils` → `("utils", "utils")`, `from utils import func` → `("func", "utils")`
    /// - JS/TS: `import x from './mod'` → `("x", "./mod")`, `const x = require('./mod')` → `("x", "./mod")`
    /// - Go: `import "pkg"` → `("pkg", "pkg")`, `import alias "pkg"` → `("alias", "pkg")`
    ///
    /// Module paths are returned as-is (not resolved to filesystem paths).
    pub fn extract_imports(&self) -> BTreeMap<String, String> {
        let mut imports = BTreeMap::new();
        self.collect_imports(self.tree.root_node(), &mut imports);
        imports
    }

    fn collect_imports(&self, node: Node<'_>, out: &mut BTreeMap<String, String>) {
        match self.language {
            Language::Python => self.collect_python_imports(node, out),
            Language::JavaScript | Language::TypeScript | Language::Tsx => {
                self.collect_js_imports(node, out)
            }
            Language::Go => self.collect_go_imports(node, out),
            _ => {} // C/C++/Rust/Java/Lua/Terraform/Bash: no module-qualified calls
        }
    }

    fn collect_python_imports(&self, node: Node<'_>, out: &mut BTreeMap<String, String>) {
        match node.kind() {
            // `import utils` or `import utils as u`
            "import_statement" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    match child.kind() {
                        "dotted_name" => {
                            let name = self.node_text(&child).to_string();
                            // Use last component as alias: `import os.path` → alias "path"
                            let alias = name.rsplit('.').next().unwrap_or(&name).to_string();
                            out.insert(alias, name);
                        }
                        "aliased_import" => {
                            let module = child
                                .child_by_field_name("name")
                                .map(|n| self.node_text(&n).to_string());
                            let alias = child
                                .child_by_field_name("alias")
                                .map(|n| self.node_text(&n).to_string());
                            if let (Some(module), Some(alias)) = (module, alias) {
                                out.insert(alias, module);
                            }
                        }
                        _ => {}
                    }
                }
            }
            // `from utils import func` or `from utils import func as f`
            "import_from_statement" => {
                let module = node
                    .child_by_field_name("module_name")
                    .map(|n| self.node_text(&n).to_string());
                let module = module.or_else(|| {
                    // tree-sitter-python uses different field names across versions
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if child.kind() == "dotted_name" || child.kind() == "relative_import" {
                            return Some(self.node_text(&child).to_string());
                        }
                    }
                    None
                });
                if let Some(module) = module {
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        match child.kind() {
                            "dotted_name" | "identifier" => {
                                let name = self.node_text(&child).to_string();
                                // Skip the module name itself (first dotted_name)
                                if name != module {
                                    out.insert(name, module.clone());
                                }
                            }
                            "aliased_import" => {
                                let alias = child
                                    .child_by_field_name("alias")
                                    .map(|n| self.node_text(&n).to_string());
                                if let Some(alias) = alias {
                                    out.insert(alias, module.clone());
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.collect_python_imports(child, out);
                }
            }
        }
    }

    fn collect_js_imports(&self, node: Node<'_>, out: &mut BTreeMap<String, String>) {
        match node.kind() {
            // ES6: `import x from './mod'` or `import { func } from './mod'`
            "import_statement" => {
                let source = node.child_by_field_name("source").map(|n| {
                    let text = self.node_text(&n);
                    text.trim_matches(|c| c == '\'' || c == '"').to_string()
                });
                if let Some(module_path) = source {
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        match child.kind() {
                            "import_clause" => {
                                self.collect_js_import_clause(&child, &module_path, out);
                            }
                            "identifier" => {
                                // `import x from './mod'`
                                let name = self.node_text(&child).to_string();
                                out.insert(name, module_path.clone());
                            }
                            _ => {}
                        }
                    }
                }
            }
            // CommonJS: `const x = require('./mod')`
            "lexical_declaration" | "variable_declaration" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "variable_declarator" {
                        self.collect_require_binding(&child, out);
                    }
                }
            }
            _ => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.collect_js_imports(child, out);
                }
            }
        }
    }

    fn collect_js_import_clause(
        &self,
        node: &Node<'_>,
        module_path: &str,
        out: &mut BTreeMap<String, String>,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    // Default import: `import utils from './mod'`
                    out.insert(self.node_text(&child).to_string(), module_path.to_string());
                }
                "named_imports" => {
                    // `import { func, other as alias } from './mod'`
                    let mut inner = child.walk();
                    for spec in child.children(&mut inner) {
                        if spec.kind() == "import_specifier" {
                            let name = spec
                                .child_by_field_name("name")
                                .map(|n| self.node_text(&n).to_string());
                            let alias = spec
                                .child_by_field_name("alias")
                                .map(|n| self.node_text(&n).to_string());
                            if let Some(local) = alias.or(name) {
                                out.insert(local, module_path.to_string());
                            }
                        }
                    }
                }
                "namespace_import" => {
                    // `import * as utils from './mod'`
                    if let Some(id) = child.child_by_field_name("name") {
                        out.insert(self.node_text(&id).to_string(), module_path.to_string());
                    }
                    // Fallback: find identifier child
                    let mut inner = child.walk();
                    for c in child.children(&mut inner) {
                        if c.kind() == "identifier" {
                            out.insert(self.node_text(&c).to_string(), module_path.to_string());
                            break;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn collect_require_binding(&self, node: &Node<'_>, out: &mut BTreeMap<String, String>) {
        // `const x = require('./mod')` or `const { a, b } = require('./mod')`
        let value = node.child_by_field_name("value");
        let name = node.child_by_field_name("name");

        if let Some(val) = value {
            // Check if value is a require() call
            if self.language.is_call_node(val.kind()) {
                if let Some(func_name) = self.language.call_function_name(&val) {
                    if self.node_text(&func_name) == "require" {
                        if let Some(args) = self.language.call_arguments(&val) {
                            // Extract the module path from first argument
                            let mut cursor = args.walk();
                            for child in args.children(&mut cursor) {
                                if child.is_named() {
                                    let text = self.node_text(&child);
                                    let path =
                                        text.trim_matches(|c| c == '\'' || c == '"').to_string();
                                    // Bind the name(s) to this module
                                    if let Some(n) = &name {
                                        if n.kind() == "object_pattern" {
                                            // Destructuring: `const { a, b } = require('./mod')`
                                            let mut inner = n.walk();
                                            for prop in n.children(&mut inner) {
                                                if prop.kind()
                                                    == "shorthand_property_identifier_pattern"
                                                    || prop.kind() == "identifier"
                                                {
                                                    out.insert(
                                                        self.node_text(&prop).to_string(),
                                                        path.clone(),
                                                    );
                                                }
                                            }
                                        } else {
                                            out.insert(self.node_text(n).to_string(), path.clone());
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn collect_go_imports(&self, node: Node<'_>, out: &mut BTreeMap<String, String>) {
        match node.kind() {
            "import_declaration" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "import_spec_list" {
                        let mut inner = child.walk();
                        for spec in child.children(&mut inner) {
                            if spec.kind() == "import_spec" {
                                self.extract_go_import_spec(&spec, out);
                            }
                        }
                    } else if child.kind() == "import_spec" {
                        self.extract_go_import_spec(&child, out);
                    } else if child.kind() == "interpreted_string_literal" {
                        // `import "pkg"` — single import without parens
                        let text = self.node_text(&child);
                        let path = text.trim_matches('"').to_string();
                        let alias = path.rsplit('/').next().unwrap_or(&path).to_string();
                        out.insert(alias, path);
                    }
                }
            }
            _ => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.collect_go_imports(child, out);
                }
            }
        }
    }

    fn extract_go_import_spec(&self, node: &Node<'_>, out: &mut BTreeMap<String, String>) {
        let mut path_str = None;
        let mut alias = None;
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "interpreted_string_literal" => {
                    let text = self.node_text(&child);
                    path_str = Some(text.trim_matches('"').to_string());
                }
                "package_identifier" | "blank_identifier" | "dot" => {
                    alias = Some(self.node_text(&child).to_string());
                }
                _ => {}
            }
        }
        if let Some(path) = path_str {
            let local =
                alias.unwrap_or_else(|| path.rsplit('/').next().unwrap_or(&path).to_string());
            if local != "_" && local != "." {
                out.insert(local, path);
            }
        }
    }

    /// Find all assignment targets (L-values) on diff lines within a function scope.
    pub fn assignment_lvalues_on_lines(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<(String, usize)> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(query) = get_query(self.language, QueryKind::Assignments) {
            let assign_idx = query
                .capture_index_for_name("assign")
                .expect("Assignments query must have @assign capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut lvalues = Vec::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == assign_idx {
                        let line = capture.node.start_position().row + 1;
                        if lines.contains(&line) {
                            self.extract_assignment_lvalues(&capture.node, line, &mut lvalues);
                        }
                    }
                }
            }
            return lvalues;
        }

        let mut lvalues = Vec::new();
        self.collect_assignments_manual(*func_node, lines, &mut lvalues);
        lvalues
    }

    /// Extract L-value names from a matched assignment/declaration node.
    fn extract_assignment_lvalues(
        &self,
        node: &Node<'_>,
        line: usize,
        out: &mut Vec<(String, usize)>,
    ) {
        if self.language.is_assignment_node(node.kind()) {
            if let Some(lhs) = self.language.assignment_target(node) {
                let lhs_text = self.node_text(&lhs).to_string();
                for name in extract_lvalue_names(&lhs_text) {
                    out.push((name, line));
                }
            }
        }
        if self.language.is_declaration_node(node.kind()) {
            if let Some(name_node) = self.language.declaration_name(node) {
                let name = self.node_text(&name_node).to_string();
                out.push((name, line));
            }
        }
    }

    /// Manual recursive assignment collection (pre-query fallback).
    pub(crate) fn collect_assignments_manual(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(String, usize)>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line) && self.language.is_assignment_node(node.kind()) {
            if let Some(lhs) = self.language.assignment_target(&node) {
                let lhs_text = self.node_text(&lhs).to_string();
                for name in extract_lvalue_names(&lhs_text) {
                    out.push((name, line));
                }
            }
        }

        if lines.contains(&line) && self.language.is_declaration_node(node.kind()) {
            if let Some(name_node) = self.language.declaration_name(&node) {
                let name = self.node_text(&name_node).to_string();
                out.push((name, line));
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_assignments_manual(child, lines, out);
        }
    }

    /// Like `assignment_lvalues_on_lines`, but returns structured `AccessPath`s
    /// instead of plain variable name strings. Used by the DFG for field-sensitive tracking.
    pub fn assignment_lvalue_paths_on_lines(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<(AccessPath, usize)> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(query) = get_query(self.language, QueryKind::Assignments) {
            let assign_idx = query
                .capture_index_for_name("assign")
                .expect("Assignments query must have @assign capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut paths = Vec::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == assign_idx {
                        let line = capture.node.start_position().row + 1;
                        if lines.contains(&line) {
                            self.extract_assignment_lvalue_paths(&capture.node, line, &mut paths);
                        }
                    }
                }
            }
            // The query may miss for_in_statement and as_pattern since they aren't
            // assignment/declaration nodes. Run the gap handlers on the full tree.
            self.collect_assignment_paths_gaps(*func_node, lines, &mut paths);
            return paths;
        }

        let mut paths = Vec::new();
        self.collect_assignment_paths_manual(*func_node, lines, &mut paths);
        paths
    }

    /// Extract L-value AccessPaths from a matched assignment/declaration node.
    fn extract_assignment_lvalue_paths(
        &self,
        node: &Node<'_>,
        line: usize,
        out: &mut Vec<(AccessPath, usize)>,
    ) {
        if self.language.is_assignment_node(node.kind()) {
            if let Some(lhs) = self.language.assignment_target(node) {
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
        if self.language.is_declaration_node(node.kind()) {
            if let Some(name_node) = self.language.declaration_name(node) {
                if name_node.kind() == "expression_list" {
                    self.extract_multi_target_lvalues(&name_node, line, out);
                } else {
                    let name = self.node_text(&name_node).to_string();
                    out.push((AccessPath::simple(name), line));
                }
            }
        }
    }

    /// Handle gap patterns not covered by the Assignments query (for_in, as_pattern).
    fn collect_assignment_paths_gaps(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(AccessPath, usize)>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line)
            && node.kind() == "for_in_statement"
            && matches!(
                self.language,
                Language::JavaScript | Language::TypeScript | Language::Tsx
            )
        {
            self.extract_for_in_lvalues(&node, line, out);
        }

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
            self.collect_assignment_paths_gaps(child, lines, out);
        }
    }

    /// Manual recursive assignment path collection (pre-query fallback).
    fn collect_assignment_paths_manual(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(AccessPath, usize)>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line) && self.language.is_assignment_node(node.kind()) {
            if let Some(lhs) = self.language.assignment_target(&node) {
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
                if name_node.kind() == "expression_list" {
                    self.extract_multi_target_lvalues(&name_node, line, out);
                } else {
                    let name = self.node_text(&name_node).to_string();
                    out.push((AccessPath::simple(name), line));
                }
            }
        }

        if lines.contains(&line)
            && node.kind() == "for_in_statement"
            && matches!(
                self.language,
                Language::JavaScript | Language::TypeScript | Language::Tsx
            )
        {
            self.extract_for_in_lvalues(&node, line, out);
        }

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
            self.collect_assignment_paths_manual(child, lines, out);
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
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        // Use the Assignments query to find assignment nodes, then extract RHS.
        // Also use the Calls query for call arguments on diff lines.
        if let Some(assign_query) = get_query(self.language, QueryKind::Assignments) {
            let assign_idx = assign_query
                .capture_index_for_name("assign")
                .expect("Assignments query must have @assign capture");
            let mut paths = Vec::new();

            // Collect R-values from assignments
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches =
                cursor.matches(assign_query, self.tree.root_node(), self.source.as_bytes());
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == assign_idx {
                        let line = capture.node.start_position().row + 1;
                        if lines.contains(&line)
                            && self.language.is_assignment_node(capture.node.kind())
                        {
                            if let Some(rhs) = self.language.assignment_value(&capture.node) {
                                self.collect_identifier_paths(rhs, &mut paths);
                            }
                        }
                    }
                }
            }

            // Collect R-values from call arguments using Calls query
            if let Some(call_query) = get_query(self.language, QueryKind::Calls) {
                let call_idx = call_query
                    .capture_index_for_name("call")
                    .expect("Calls query must have @call capture");
                let mut cursor2 = tree_sitter::QueryCursor::new();
                cursor2.set_byte_range(func_node.byte_range());
                let mut matches2 =
                    cursor2.matches(call_query, self.tree.root_node(), self.source.as_bytes());
                while let Some(m) = matches2.next() {
                    for capture in m.captures {
                        if capture.index == call_idx {
                            let line = capture.node.start_position().row + 1;
                            if lines.contains(&line) {
                                if let Some(args) = self.language.call_arguments(&capture.node) {
                                    self.collect_identifier_paths(args, &mut paths);
                                }
                                if let Some(func_name_node) =
                                    self.language.call_function_name(&capture.node)
                                {
                                    let name = self.node_text(&func_name_node).to_string();
                                    paths.push((AccessPath::simple(name), line));
                                }
                            }
                        }
                    }
                }
            }

            return paths;
        }

        let mut paths = Vec::new();
        self.collect_rvalue_paths_manual(*func_node, lines, &mut paths);
        paths
    }

    /// Manual recursive R-value path collection (pre-query fallback).
    fn collect_rvalue_paths_manual(
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
            self.collect_rvalue_paths_manual(child, lines, out);
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
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(assign_query) = get_query(self.language, QueryKind::Assignments) {
            let assign_idx = assign_query
                .capture_index_for_name("assign")
                .expect("Assignments query must have @assign capture");
            let mut rvalues = Vec::new();

            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches =
                cursor.matches(assign_query, self.tree.root_node(), self.source.as_bytes());
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == assign_idx {
                        let line = capture.node.start_position().row + 1;
                        if lines.contains(&line)
                            && self.language.is_assignment_node(capture.node.kind())
                        {
                            if let Some(rhs) = self.language.assignment_value(&capture.node) {
                                self.collect_all_identifiers(rhs, &mut rvalues);
                            }
                        }
                    }
                }
            }

            // Also collect from call arguments
            if let Some(call_query) = get_query(self.language, QueryKind::Calls) {
                let call_idx = call_query
                    .capture_index_for_name("call")
                    .expect("Calls query must have @call capture");
                let mut cursor2 = tree_sitter::QueryCursor::new();
                cursor2.set_byte_range(func_node.byte_range());
                let mut matches2 =
                    cursor2.matches(call_query, self.tree.root_node(), self.source.as_bytes());
                while let Some(m) = matches2.next() {
                    for capture in m.captures {
                        if capture.index == call_idx {
                            let line = capture.node.start_position().row + 1;
                            if lines.contains(&line) {
                                if let Some(args) = self.language.call_arguments(&capture.node) {
                                    self.collect_all_identifiers(args, &mut rvalues);
                                }
                                if let Some(func_name_node) =
                                    self.language.call_function_name(&capture.node)
                                {
                                    let name = self.node_text(&func_name_node).to_string();
                                    rvalues.push((name, line));
                                }
                            }
                        }
                    }
                }
            }

            return rvalues;
        }

        let mut rvalues = Vec::new();
        self.collect_rvalues_manual(*func_node, lines, &mut rvalues);
        rvalues
    }

    /// Manual recursive R-value collection (pre-query fallback).
    fn collect_rvalues_manual(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(String, usize)>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line) && self.language.is_assignment_node(node.kind()) {
            if let Some(rhs) = self.language.assignment_value(&node) {
                self.collect_all_identifiers(rhs, out);
            }
        }

        if lines.contains(&line) && self.language.is_call_node(node.kind()) {
            if let Some(args) = self.language.call_arguments(&node) {
                self.collect_all_identifiers(args, out);
            }
            if let Some(func_name_node) = self.language.call_function_name(&node) {
                let name = self.node_text(&func_name_node).to_string();
                out.push((name, line));
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_rvalues_manual(child, lines, out);
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
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(query) = get_query(self.language, QueryKind::Identifiers) {
            let ident_idx = query
                .capture_index_for_name("ident")
                .expect("Identifiers query must have @ident capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut lines = BTreeSet::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == ident_idx && self.node_text(&capture.node) == var_name {
                        lines.insert(capture.node.start_position().row + 1);
                    }
                }
            }
            return lines;
        }

        let mut lines = BTreeSet::new();
        self.collect_variable_refs_manual(*func_node, var_name, &mut lines);
        lines
    }

    /// Manual recursive variable ref collection (pre-query fallback).
    fn collect_variable_refs_manual(
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
            self.collect_variable_refs_manual(child, var_name, out);
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
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(query) = get_query(self.language, QueryKind::Identifiers) {
            let ident_idx = query
                .capture_index_for_name("ident")
                .expect("Identifiers query must have @ident capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut lines = BTreeSet::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == ident_idx && self.node_text(&capture.node) == var_name {
                        // Bottom-up scope check: walk from the capture node up to the
                        // function boundary. If any enclosing scope block re-declares
                        // var_name and doesn't contain def_line, skip this reference.
                        if !self.is_shadowed_at(&capture.node, func_node, var_name, def_line) {
                            lines.insert(capture.node.start_position().row + 1);
                        }
                    }
                }
            }
            return lines;
        }

        let mut lines = BTreeSet::new();
        self.collect_variable_refs_scoped_manual(*func_node, var_name, def_line, &mut lines);
        lines
    }

    /// Check whether a reference node is shadowed by a re-declaration in an inner scope.
    /// Walks bottom-up from the identifier node to the function boundary.
    fn is_shadowed_at(
        &self,
        node: &Node<'_>,
        func_node: &Node<'_>,
        var_name: &str,
        def_line: usize,
    ) -> bool {
        let func_id = func_node.id();
        let mut current = node.parent();
        while let Some(parent) = current {
            if parent.id() == func_id {
                break;
            }
            if self.language.is_scope_block(parent.kind()) {
                let scope_start = parent.start_position().row + 1;
                let scope_end = parent.end_position().row + 1;
                let def_in_scope = def_line >= scope_start && def_line <= scope_end;
                if !def_in_scope && self.scope_has_declaration(parent, var_name) {
                    return true;
                }
            }
            current = parent.parent();
        }
        false
    }

    /// Manual recursive scoped variable ref collection (pre-query fallback).
    fn collect_variable_refs_scoped_manual(
        &self,
        node: Node<'_>,
        var_name: &str,
        def_line: usize,
        out: &mut BTreeSet<usize>,
    ) {
        let node_start = node.start_position().row + 1;
        let node_end = node.end_position().row + 1;

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
            self.collect_variable_refs_scoped_manual(child, var_name, def_line, out);
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

    /// Check if a variable has any bare (non-field-access) references in a function
    /// body (excluding the parameter list itself).
    ///
    /// Returns true if the variable is used as a standalone identifier, not just as
    /// the base of a field/member access (e.g., `data` in `use(data)` counts, but
    /// `dev` in `dev.name` does not). Used to decide whether to register a parameter
    /// Def for interprocedural data flow.
    pub fn has_bare_references(&self, func_node: &Node<'_>, var_name: &str) -> bool {
        let func_start_line = func_node.start_position().row + 1;
        self.find_bare_ref(*func_node, var_name, func_start_line)
    }

    fn find_bare_ref(&self, node: Node<'_>, var_name: &str, skip_line: usize) -> bool {
        // If this is a field access (dev.name, dev->name), skip it — the `dev`
        // identifier inside is not a bare reference.
        if Self::is_field_access_node(node.kind()) {
            return false;
        }

        if self.language.is_identifier_node(node.kind()) && self.node_text(&node) == var_name {
            let line = node.start_position().row + 1;
            // Skip identifiers on the function definition line (parameter declarations).
            if line == skip_line {
                return false;
            }
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
            if self.find_bare_ref(child, var_name, skip_line) {
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
        // Condition vars are a composition: find control flow nodes on diff lines,
        // extract the condition sub-node, then collect identifiers within it.
        // The control flow node matching stays manual (too many node types per language),
        // but the identifier extraction within the condition uses the Identifiers query
        // via collect_all_identifiers (which is called on the condition sub-node).
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
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(query) = get_query(self.language, QueryKind::Returns) {
            let ret_idx = query
                .capture_index_for_name("ret")
                .expect("Returns query must have @ret capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut lines = Vec::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == ret_idx {
                        lines.push(capture.node.start_position().row + 1);
                    }
                }
            }
            return lines;
        }

        let mut lines = Vec::new();
        self.collect_returns_manual(*func_node, &mut lines);
        lines
    }

    /// Manual recursive return collection (pre-query fallback).
    fn collect_returns_manual(&self, node: Node<'_>, out: &mut Vec<usize>) {
        if self.language.is_return_node(node.kind()) {
            out.push(node.start_position().row + 1);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_returns_manual(child, out);
        }
    }

    /// Collect return statements with their value expressions.
    ///
    /// For each return statement in the function, extracts the return value
    /// expression text and node kind. Also detects Rust trailing expressions
    /// (last expression in a block without semicolon).
    pub fn return_value_nodes(&self, func_node: &Node<'_>) -> Vec<ReturnInfo> {
        let mut returns = Vec::new();
        self.collect_return_infos(*func_node, func_node, &mut returns);

        // For Rust: check for trailing expressions (last expr in block without `;`)
        if self.language == Language::Rust {
            self.collect_trailing_returns(func_node, &mut returns);
        }

        returns
    }

    /// Recursively collect ReturnInfo from return statement nodes.
    fn collect_return_infos(
        &self,
        node: Node<'_>,
        func_node: &Node<'_>,
        out: &mut Vec<ReturnInfo>,
    ) {
        let kind = node.kind();
        if kind == "return_statement" || kind == "return_expression" {
            let line = node.start_position().row + 1;

            // Extract the return value expression (first named child that
            // isn't the `return` keyword itself).
            let mut value_text = None;
            let mut value_kind = None;

            if kind == "return_statement" && self.language == Language::Go {
                // Go: return may have an expression_list child
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    let ck = child.kind();
                    if ck == "expression_list" {
                        value_text = Some(self.node_text(&child).to_string());
                        value_kind = Some(ck.to_string());
                        break;
                    }
                    // Single expression (not expression_list)
                    value_text = Some(self.node_text(&child).to_string());
                    value_kind = Some(ck.to_string());
                    break;
                }
            } else {
                // All other languages: first named child is the expression
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    value_text = Some(self.node_text(&child).to_string());
                    value_kind = Some(child.kind().to_string());
                    break;
                }
            }

            let is_conditional = self.is_inside_conditional(&node, func_node);

            out.push(ReturnInfo {
                line,
                value_text,
                value_kind,
                is_conditional,
            });
            return; // Don't recurse into return children
        }

        // Don't recurse into nested function definitions
        if self.language.function_node_types().contains(&kind) && kind != func_node.kind() {
            // Check if this is actually a different function (not the func_node itself)
            if node.start_position() != func_node.start_position() {
                return;
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_return_infos(child, func_node, out);
        }
    }

    /// Check if a node is inside a conditional branch (if/else/match) within the function.
    fn is_inside_conditional(&self, node: &Node<'_>, func_node: &Node<'_>) -> bool {
        let mut current = node.parent();
        while let Some(parent) = current {
            if parent.id() == func_node.id() {
                return false;
            }
            let pk = parent.kind();
            if matches!(
                pk,
                "if_statement"
                    | "if_expression"
                    | "if_let_expression"
                    | "else_clause"
                    | "elif_clause"
                    | "match_expression"
                    | "switch_statement"
                    | "conditional_expression"
                    | "ternary_expression"
            ) {
                return true;
            }
            current = parent.parent();
        }
        false
    }

    /// Collect Rust trailing expressions that act as implicit returns.
    ///
    /// In Rust, the last expression in a block (without semicolon) is the
    /// return value. This detects such expressions in the function body
    /// and in if/else/match branches.
    fn collect_trailing_returns(&self, func_node: &Node<'_>, out: &mut Vec<ReturnInfo>) {
        let body = func_node.child_by_field_name("body");
        if let Some(body_node) = body {
            self.collect_block_trailing_returns(&body_node, func_node, out);
        }
    }

    fn collect_block_trailing_returns(
        &self,
        block: &Node<'_>,
        func_node: &Node<'_>,
        out: &mut Vec<ReturnInfo>,
    ) {
        let child_count = block.named_child_count();
        if child_count == 0 {
            return;
        }
        let last_child = block.named_child(child_count - 1).unwrap();
        let kind = last_child.kind();

        // If it's an expression_statement, check if it wraps an if/match expression
        // (Rust trailing if/match are often wrapped in expression_statement).
        if kind == "expression_statement" {
            // Check if it ends with a semicolon — if so, not a trailing return
            let text = self.node_text(&last_child);
            if text.ends_with(';') {
                return;
            }
            // Check children for if/match expressions
            let mut cursor = last_child.walk();
            for child in last_child.named_children(&mut cursor) {
                let ck = child.kind();
                if ck == "if_expression" || ck == "if_let_expression" {
                    self.collect_if_trailing_returns(&child, func_node, out);
                    return;
                }
                if ck == "match_expression" {
                    self.collect_match_trailing_returns(&child, func_node, out);
                    return;
                }
                // Other expression — treat as trailing return
                let line = child.start_position().row + 1;
                if !out.iter().any(|r| r.line == line) {
                    out.push(ReturnInfo {
                        line,
                        value_text: Some(self.node_text(&child).to_string()),
                        value_kind: Some(ck.to_string()),
                        is_conditional: self.is_inside_conditional(&child, func_node),
                    });
                }
                return;
            }
            return;
        }

        // If it's an if_expression or match_expression, recurse into branches
        if kind == "if_expression" || kind == "if_let_expression" {
            self.collect_if_trailing_returns(&last_child, func_node, out);
            return;
        }
        if kind == "match_expression" {
            self.collect_match_trailing_returns(&last_child, func_node, out);
            return;
        }

        // Skip return_expression — already handled by collect_return_infos
        if kind == "return_expression" {
            return;
        }

        // Skip statements that aren't expressions
        if kind.ends_with("_statement")
            || kind == "let_declaration"
            || kind == "macro_invocation"
            || kind == "empty_statement"
        {
            return;
        }

        // This is a trailing expression — implicit return
        let line = last_child.start_position().row + 1;
        // Check it's not already captured as a return
        if out.iter().any(|r| r.line == line) {
            return;
        }

        let text = self.node_text(&last_child).to_string();
        let is_conditional = self.is_inside_conditional(&last_child, func_node);
        out.push(ReturnInfo {
            line,
            value_text: Some(text),
            value_kind: Some(kind.to_string()),
            is_conditional,
        });
    }

    fn collect_if_trailing_returns(
        &self,
        if_node: &Node<'_>,
        func_node: &Node<'_>,
        out: &mut Vec<ReturnInfo>,
    ) {
        // Recurse into consequence (then block) and alternative (else block)
        if let Some(consequence) = if_node.child_by_field_name("consequence") {
            self.collect_block_trailing_returns(&consequence, func_node, out);
        }
        if let Some(alternative) = if_node.child_by_field_name("alternative") {
            let ak = alternative.kind();
            if ak == "else_clause" {
                // The else clause's child is either a block or another if_expression
                let mut cursor = alternative.walk();
                for child in alternative.named_children(&mut cursor) {
                    if child.kind() == "block" {
                        self.collect_block_trailing_returns(&child, func_node, out);
                    } else if child.kind() == "if_expression" || child.kind() == "if_let_expression"
                    {
                        self.collect_if_trailing_returns(&child, func_node, out);
                    }
                }
            } else if ak == "block" {
                self.collect_block_trailing_returns(&alternative, func_node, out);
            } else if ak == "if_expression" || ak == "if_let_expression" {
                self.collect_if_trailing_returns(&alternative, func_node, out);
            }
        }
    }

    fn collect_match_trailing_returns(
        &self,
        match_node: &Node<'_>,
        func_node: &Node<'_>,
        out: &mut Vec<ReturnInfo>,
    ) {
        let mut cursor = match_node.walk();
        for child in match_node.named_children(&mut cursor) {
            if child.kind() == "match_arm" {
                // The value of a match arm is its last named child (after the `=>`)
                let arm_count = child.named_child_count();
                if arm_count > 0 {
                    let arm_value = child.named_child(arm_count - 1).unwrap();
                    let vk = arm_value.kind();
                    if vk == "block" {
                        self.collect_block_trailing_returns(&arm_value, func_node, out);
                    } else if vk != "match_pattern" {
                        let line = arm_value.start_position().row + 1;
                        if !out.iter().any(|r| r.line == line) {
                            out.push(ReturnInfo {
                                line,
                                value_text: Some(self.node_text(&arm_value).to_string()),
                                value_kind: Some(vk.to_string()),
                                is_conditional: true,
                            });
                        }
                    }
                }
            }
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

    /// Classify labels as cleanup-only (reachable only via goto) or
    /// flow-through (also reachable from sequential execution).
    ///
    /// A label is cleanup-only if the statement immediately preceding it
    /// is a flow-terminating statement (return, goto, break, continue).
    /// If the preceding statement is normal code, sequential execution
    /// falls through into the label — it's part of the normal path.
    ///
    /// Returns `(name, start, end, is_cleanup_only)` tuples.
    pub fn classify_labels(&self, func_node: &Node<'_>) -> Vec<(String, usize, usize, bool)> {
        let sections = self.label_sections(func_node);
        let (func_start, _func_end) = self.node_line_range(func_node);

        sections
            .into_iter()
            .map(|(name, start, end)| {
                let is_cleanup = if start <= func_start + 1 {
                    // Label at the very beginning of the function — flow-through
                    false
                } else {
                    // Check lines immediately before the label for a flow terminator.
                    // We scan backwards from label_start-1 to skip blank/brace lines.
                    let mut found_terminator = false;
                    for check_line in (func_start..start).rev() {
                        let row = check_line.saturating_sub(1);
                        if self.find_flow_terminator(self.tree.root_node(), row) {
                            found_terminator = true;
                            break;
                        }
                        // Check if this line has any real code (not just whitespace/braces)
                        if let Some(line_str) = self.source.lines().nth(row) {
                            let trimmed = line_str.trim();
                            if !trimmed.is_empty() && trimmed != "}" && trimmed != "{" {
                                // Found non-empty, non-brace code that isn't a terminator
                                break;
                            }
                        }
                    }
                    found_terminator
                };
                (name, start, end, is_cleanup)
            })
            .collect()
    }

    /// Check if a given row (0-indexed) contains a flow-terminating statement.
    fn find_flow_terminator(&self, node: Node<'_>, row: usize) -> bool {
        if node.start_position().row == row
            && matches!(
                node.kind(),
                "return_statement" | "goto_statement" | "break_statement" | "continue_statement"
            )
        {
            return true;
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.start_position().row <= row && child.end_position().row >= row {
                if self.find_flow_terminator(child, row) {
                    return true;
                }
            }
        }
        false
    }

    /// Partition a function's lines into normal-path lines and cleanup label sections.
    ///
    /// Returns `(normal_lines, label_lines)` where:
    /// - `normal_lines` are lines NOT in any cleanup-only label section
    /// - `label_lines` maps each cleanup-only label name to its line range
    ///
    /// Labels reachable via fall-through (no preceding flow terminator) are
    /// considered part of the normal path. Only cleanup-only labels (preceded
    /// by return/goto/break/continue) are separated out.
    pub fn partition_by_labels(
        &self,
        func_node: &Node<'_>,
    ) -> (Vec<usize>, BTreeMap<String, Vec<usize>>) {
        let (func_start, func_end) = self.node_line_range(func_node);
        let classified = self.classify_labels(func_node);
        let cleanup_labels: Vec<_> = classified
            .iter()
            .filter(|(_, _, _, is_cleanup)| *is_cleanup)
            .collect();

        if cleanup_labels.is_empty() {
            let normal: Vec<usize> = (func_start..=func_end).collect();
            return (normal, BTreeMap::new());
        }

        // Build a set of all lines in cleanup-only label sections
        let mut cleanup_line_set = std::collections::BTreeSet::new();
        let mut label_map: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (name, start, end, _) in &cleanup_labels {
            let lines: Vec<usize> = (*start..=*end).collect();
            cleanup_line_set.extend(lines.iter().copied());
            label_map.insert(name.clone(), lines);
        }

        let normal: Vec<usize> = (func_start..=func_end)
            .filter(|l| !cleanup_line_set.contains(l))
            .collect();

        (normal, label_map)
    }

    /// Get all lines reachable from a `goto label`, including fall-through
    /// to subsequent labels (unless a `return` breaks the fall-through).
    ///
    /// In the cascading kernel cleanup pattern:
    /// ```c
    ///   err_dev:
    ///       kfree(dev);          // reachable from goto err_dev
    ///   err_buf:                 // falls through from err_dev (no return above)
    ///       kfree(buf);          // also reachable from goto err_dev
    ///       return -1;
    /// ```
    ///
    /// `goto err_dev` reaches both sections. `goto err_buf` reaches only err_buf.
    pub fn lines_reachable_from_goto(
        &self,
        func_node: &Node<'_>,
        target_label: &str,
    ) -> Vec<usize> {
        let label_secs = self.label_sections(func_node);
        let returns = self.return_statements(func_node);
        let return_set: std::collections::BTreeSet<usize> = returns.into_iter().collect();

        let mut reachable = Vec::new();
        let mut found_target = false;

        for (name, start, end) in &label_secs {
            if name == target_label {
                found_target = true;
            }
            if !found_target {
                continue;
            }

            reachable.extend(*start..=*end);

            // Check if this section contains a return — if so, fall-through stops
            if (*start..=*end).any(|l| return_set.contains(&l)) {
                break;
            }
        }

        reachable
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
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(query) = get_query(self.language, QueryKind::Calls) {
            let call_idx = query
                .capture_index_for_name("call")
                .expect("Calls query must have @call capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut calls = Vec::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == call_idx {
                        let line = capture.node.start_position().row + 1;
                        if lines.contains(&line) {
                            if let Some(name_node) = self.language.call_function_name(&capture.node)
                            {
                                let name = self.node_text(&name_node).to_string();
                                calls.push((name, line));
                            }
                        }
                    }
                }
            }
            return calls;
        }

        let mut calls = Vec::new();
        self.collect_calls_manual(*func_node, lines, &mut calls);
        calls
    }

    /// Like `function_calls_on_lines`, but also extracts the module/object qualifier.
    /// Returns `(callee_name, line, qualifier)` tuples.
    pub fn function_calls_on_lines_with_qualifier(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<(String, usize, Option<String>)> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(query) = get_query(self.language, QueryKind::Calls) {
            let call_idx = query
                .capture_index_for_name("call")
                .expect("Calls query must have @call capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut calls = Vec::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == call_idx {
                        let line = capture.node.start_position().row + 1;
                        if lines.contains(&line) {
                            if let Some(name_node) = self.language.call_function_name(&capture.node)
                            {
                                let name = self.node_text(&name_node).to_string();
                                let qualifier = self
                                    .language
                                    .call_function_qualifier(&capture.node)
                                    .map(|q| self.node_text(&q).to_string());
                                calls.push((name, line, qualifier));
                            }
                        }
                    }
                }
            }
            return calls;
        }

        let mut calls = Vec::new();
        self.collect_calls_manual_with_qualifier(*func_node, lines, &mut calls);
        calls
    }

    fn collect_calls_manual_with_qualifier(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(String, usize, Option<String>)>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line) && self.language.is_call_node(node.kind()) {
            if let Some(name_node) = self.language.call_function_name(&node) {
                let name = self.node_text(&name_node).to_string();
                let qualifier = self
                    .language
                    .call_function_qualifier(&node)
                    .map(|q| self.node_text(&q).to_string());
                out.push((name, line, qualifier));
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_calls_manual_with_qualifier(child, lines, out);
        }
    }

    /// Manual recursive call collection (pre-query fallback).
    /// `pub(crate)` for dual-path consistency testing in `queries::tests`.
    pub(crate) fn collect_calls_manual(
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
            self.collect_calls_manual(child, lines, out);
        }
    }

    /// Collect all function call names on specific lines (1-indexed).
    /// Returns a map from line number to list of called function names found on that line.
    /// Only matches actual AST call nodes — ignores calls inside comments or string literals.
    pub fn call_names_on_lines(&self, lines: &[usize]) -> BTreeMap<usize, Vec<String>> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        let line_set: BTreeSet<usize> = lines.iter().copied().collect();

        if let Some(query) = get_query(self.language, QueryKind::Calls) {
            let call_idx = query
                .capture_index_for_name("call")
                .expect("Calls query must have @call capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut result: BTreeMap<usize, Vec<String>> = BTreeMap::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == call_idx {
                        let line = capture.node.start_position().row + 1;
                        if line_set.contains(&line) {
                            if let Some(name_node) = self.language.call_function_name(&capture.node)
                            {
                                let name = self.node_text(&name_node).to_string();
                                result.entry(line).or_default().push(name);
                            }
                        }
                    }
                }
            }
            return result;
        }

        let mut result: BTreeMap<usize, Vec<String>> = BTreeMap::new();
        self.collect_call_names_at_lines_manual(self.tree.root_node(), &line_set, &mut result);
        result
    }

    fn collect_call_names_at_lines_manual(
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
            self.collect_call_names_at_lines_manual(child, lines, out);
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

    /// Extract all argument texts from a call to `callee_name` on the given line.
    ///
    /// Returns a list of argument expressions as strings, preserving positional order.
    /// Used by interprocedural data flow to map arguments to parameters.
    pub fn call_argument_texts(&self, line: usize, callee_name: &str) -> Vec<String> {
        let mut args = Vec::new();
        self.collect_call_args(self.tree.root_node(), line, callee_name, &mut args);
        args
    }

    fn collect_call_args(
        &self,
        node: Node<'_>,
        line: usize,
        callee_name: &str,
        out: &mut Vec<String>,
    ) {
        let node_line = node.start_position().row + 1;

        if node_line == line && self.language.is_call_node(node.kind()) {
            if let Some(name_node) = self.language.call_function_name(&node) {
                let name = self.node_text(&name_node);
                if name == callee_name {
                    if let Some(args_node) = self.language.call_arguments(&node) {
                        let mut cursor = args_node.walk();
                        for child in args_node.children(&mut cursor) {
                            if child.is_named() {
                                let text = self.node_text(&child).trim().to_string();
                                let text = text.trim_start_matches('&').to_string();
                                out.push(text);
                            }
                        }
                    }
                    return; // Found the call, stop.
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if !out.is_empty() {
                return;
            }
            self.collect_call_args(child, line, callee_name, out);
        }
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
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(query) = get_query(self.language, QueryKind::Calls) {
            let call_idx = query
                .capture_index_for_name("call")
                .expect("Calls query must have @call capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut names = BTreeSet::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == call_idx {
                        if let Some(name_node) = self.language.call_function_name(&capture.node) {
                            names.insert(self.node_text(&name_node).to_string());
                        }
                    }
                }
            }
            return names.into_iter().collect();
        }

        let mut names = BTreeSet::new();
        self.collect_all_callees_manual(*func_node, &mut names);
        names.into_iter().collect()
    }

    fn collect_all_callees_manual(&self, node: Node<'_>, out: &mut BTreeSet<String>) {
        if self.language.is_call_node(node.kind()) {
            if let Some(name_node) = self.language.call_function_name(&node) {
                out.insert(self.node_text(&name_node).to_string());
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_all_callees_manual(child, out);
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
        if let Some(eq_pos) = find_assignment_eq(trimmed) {
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

/// Find the position of an assignment `=` in a trimmed line, skipping `==`, `!=`, `<=`, `>=`.
/// Returns `None` if no plain assignment is found.
pub fn find_assignment_eq(trimmed: &str) -> Option<usize> {
    if let Some(eq_pos) = trimmed.find('=') {
        // Skip ==
        if eq_pos + 1 < trimmed.len() {
            if trimmed.as_bytes().get(eq_pos + 1) == Some(&b'=') {
                return None;
            }
        }
        // Skip !=, <=, >=
        if eq_pos > 0 {
            let before = trimmed.as_bytes().get(eq_pos - 1);
            if before == Some(&b'!') || before == Some(&b'<') || before == Some(&b'>') {
                return None;
            }
        }
        Some(eq_pos)
    } else {
        None
    }
}

/// Scan source text for struct field assignments that assign a known function
/// to a field with the given name.
///
/// Matches patterns:
///   `anything->field_name = known_func;`
///   `anything.field_name = known_func;`
///   `.field_name = known_func` (designated initializer)
///
/// Returns all unique known function names found assigned to this field.
pub fn resolve_struct_field_assignment(
    source: &str,
    field_name: &str,
    known_fns: &BTreeSet<String>,
) -> Vec<String> {
    let mut targets = BTreeSet::new();
    let arrow_pattern = format!("->{}", field_name);
    let dot_pattern = format!(".{}", field_name);

    for line in source.lines() {
        let trimmed = line.trim();

        // Must contain the field name with a struct accessor
        let has_field = trimmed.contains(&arrow_pattern) || trimmed.contains(&dot_pattern);
        if !has_field {
            continue;
        }

        // A line may contain multiple `field = value` fragments (e.g. designated
        // initializers inside braces: `{ .callback = handler, .data = NULL }`).
        // Scan for every occurrence of the field pattern and check the assignment
        // that follows it.
        let mut search_from = 0usize;
        while search_from < trimmed.len() {
            // Find next occurrence of ->field or .field
            let field_pos = trimmed[search_from..]
                .find(&arrow_pattern)
                .map(|p| (p + search_from, arrow_pattern.len()))
                .or_else(|| {
                    trimmed[search_from..]
                        .find(&dot_pattern)
                        .map(|p| (p + search_from, dot_pattern.len()))
                });
            let (pos, pat_len) = match field_pos {
                Some(v) => v,
                None => break,
            };
            let after_field = pos + pat_len;
            search_from = after_field;

            // After the field name, skip whitespace and look for '='
            let rest = trimmed[after_field..].trim_start();
            if !rest.starts_with('=') {
                continue;
            }
            // Make sure it's not '=='
            if rest.starts_with("==") {
                continue;
            }
            let rhs = rest[1..].trim();
            // Extract the identifier (stop at ';', ',', '}', ')', whitespace)
            let rhs_end = rhs
                .find(|c: char| c == ';' || c == ',' || c == '}' || c == ')' || c.is_whitespace())
                .unwrap_or(rhs.len());
            let rhs_token = rhs[..rhs_end].trim().trim_start_matches('&');
            if !rhs_token.is_empty()
                && rhs_token.chars().all(|c| c.is_alphanumeric() || c == '_')
                && known_fns.contains(rhs_token)
            {
                targets.insert(rhs_token.to_string());
            }
        }
    }

    targets.into_iter().collect()
}

/// Collect line numbers containing ERROR or MISSING nodes (up to `max` lines).
pub fn collect_error_lines(tree: &Tree, max: usize) -> Vec<usize> {
    let mut lines = BTreeSet::new();
    collect_error_lines_recursive(tree.root_node(), &mut lines, max);
    lines.into_iter().collect()
}

fn collect_error_lines_recursive(node: Node<'_>, lines: &mut BTreeSet<usize>, max: usize) {
    if lines.len() >= max {
        return;
    }
    if node.is_error() || node.is_missing() {
        lines.insert(node.start_position().row + 1);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_error_lines_recursive(child, lines, max);
    }
}
