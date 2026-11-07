//! Language dispatch for import extraction, import bindings and module-scope bindings.

use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::BTreeMap;
use tree_sitter::Node;

impl ParsedFile {
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

    // -------------------------------------------------------------------
    // R4c: structured import-binding extraction (parallel to extract_imports)
    // -------------------------------------------------------------------

    /// Extract structured import bindings from Python/JS/TS files.
    /// Returns one `ImportBinding` per import clause.
    ///
    /// Only collects module-scope imports: direct children of the module root,
    /// plus imports inside module-scope compound statements (if/try/for/while/with)
    /// which ARE module-scope in Python. Function-local and class-nested imports
    /// are excluded — they don't create file-wide bindings.
    pub fn extract_import_bindings(&self) -> Vec<crate::call_graph::ImportBinding> {
        let mut out = Vec::new();
        match self.language {
            Language::Python => {
                let root = self.tree.root_node();
                let mut cursor = root.walk();
                for child in root.children(&mut cursor) {
                    self.collect_python_module_scope_imports(child, &mut out);
                }
            }
            Language::JavaScript | Language::TypeScript | Language::Tsx => {
                // ES module imports are syntactically top-level only; walk
                // direct children of the root `program` node.
                let root = self.tree.root_node();
                let mut cursor = root.walk();
                for child in root.children(&mut cursor) {
                    self.collect_js_import_bindings_node(child, &mut out);
                }
            }
            _ => {}
        }
        out
    }

    // -------------------------------------------------------------------
    // R4c: module-scope binding extraction (occurrence-clean eligibility)
    // -------------------------------------------------------------------

    /// Extract module-scope binding kinds from Python/JS/TS files.
    /// Walks direct children of the module root AND descends into Python
    /// compound statements (`if`/`try`/`for`/`while`/`with`) whose bodies
    /// are still module-scope.
    pub fn extract_module_bindings(
        &self,
    ) -> BTreeMap<String, crate::call_graph::ModuleBindingKind> {
        let mut out = BTreeMap::new();
        let root = self.tree.root_node();
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            Self::extract_module_bindings_from_stmt(child, &self.language, &self.source, &mut out);
        }
        // Walk compound statement bodies (still module-scope in Python).
        let mut cursor2 = root.walk();
        for child in root.children(&mut cursor2) {
            Self::descend_compound_for_bindings(child, &self.language, &self.source, &mut out);
        }
        out
    }

    /// Process a single statement node for module-scope bindings.
    fn extract_module_bindings_from_stmt(
        node: tree_sitter::Node<'_>,
        language: &crate::languages::Language,
        source: &str,
        out: &mut BTreeMap<String, crate::call_graph::ModuleBindingKind>,
    ) {
        use crate::call_graph::ModuleBindingKind;
        let text = |n: &tree_sitter::Node<'_>| -> String {
            n.utf8_text(source.as_bytes()).unwrap_or("").to_string()
        };
        match node.kind() {
            // Python
            "import_statement" | "import_from_statement" => {
                let mut ic = node.walk();
                for c in node.children(&mut ic) {
                    match c.kind() {
                        "dotted_name" => {
                            let name = text(&c);
                            // Keep occurrence-clean eligibility keyed by the name
                            // Python actually binds for `import pkg.models`.
                            let alias = name.split('.').next().unwrap_or(&name).to_string();
                            out.entry(alias).or_insert(ModuleBindingKind::Import);
                        }
                        "aliased_import" => {
                            if let Some(a) = c.child_by_field_name("alias") {
                                out.entry(text(&a)).or_insert(ModuleBindingKind::Import);
                            }
                        }
                        "identifier" => {
                            let name = text(&c);
                            if node.kind() == "import_from_statement" {
                                if let Some(mod_node) = node.child_by_field_name("module_name") {
                                    if name == text(&mod_node) {
                                        return;
                                    }
                                }
                            }
                            out.entry(name).or_insert(ModuleBindingKind::Import);
                        }
                        _ => {}
                    }
                }
            }
            "class_definition" | "class_declaration" => {
                if let Some(name) = node.child_by_field_name("name") {
                    out.insert(text(&name), ModuleBindingKind::ClassDef);
                }
            }
            "function_definition" | "function_declaration" => {
                if let Some(name) = language.function_name(&node) {
                    out.insert(text(&name), ModuleBindingKind::FunctionDef);
                }
            }
            "decorated_definition" => {
                let mut dc = node.walk();
                for inner in node.children(&mut dc) {
                    match inner.kind() {
                        "class_definition" | "class_declaration" => {
                            if let Some(name) = inner.child_by_field_name("name") {
                                out.insert(text(&name), ModuleBindingKind::ClassDef);
                            }
                        }
                        "function_definition" | "function_declaration" => {
                            if let Some(name) = language.function_name(&inner) {
                                out.insert(text(&name), ModuleBindingKind::FunctionDef);
                            }
                        }
                        _ => {}
                    }
                }
            }
            "expression_statement" => {
                let mut ec = node.walk();
                for inner in node.children(&mut ec) {
                    if inner.kind() == "assignment" {
                        if let Some(left) = inner.child_by_field_name("left") {
                            if left.kind() == "identifier" {
                                out.insert(text(&left), ModuleBindingKind::Assignment);
                            }
                        }
                    }
                }
            }
            // JS/TS: variable declarations, exported functions/classes
            "lexical_declaration" | "variable_declaration" => {
                let mut vc = node.walk();
                for decl in node.children(&mut vc) {
                    if decl.kind() == "variable_declarator" {
                        if let Some(name) = decl.child_by_field_name("name") {
                            if name.kind() == "identifier" {
                                out.insert(text(&name), ModuleBindingKind::Assignment);
                            }
                        }
                    }
                }
            }
            "export_statement" => {
                let mut ec = node.walk();
                for inner in node.children(&mut ec) {
                    match inner.kind() {
                        "class_declaration" => {
                            if let Some(name) = inner.child_by_field_name("name") {
                                out.insert(text(&name), ModuleBindingKind::ClassDef);
                            }
                        }
                        "function_declaration" => {
                            if let Some(name) = language.function_name(&inner) {
                                out.insert(text(&name), ModuleBindingKind::FunctionDef);
                            }
                        }
                        "lexical_declaration" | "variable_declaration" => {
                            let mut vc = inner.walk();
                            for decl in inner.children(&mut vc) {
                                if decl.kind() == "variable_declarator" {
                                    if let Some(name) = decl.child_by_field_name("name") {
                                        if name.kind() == "identifier" {
                                            out.insert(text(&name), ModuleBindingKind::Assignment);
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    /// Descend into Python compound statement bodies (`if`/`try`/`for`/`while`/`with`)
    /// at module scope to find bindings that are still effectively module-scope.
    fn descend_compound_for_bindings(
        node: tree_sitter::Node<'_>,
        language: &crate::languages::Language,
        source: &str,
        out: &mut BTreeMap<String, crate::call_graph::ModuleBindingKind>,
    ) {
        match node.kind() {
            "if_statement" | "try_statement" | "for_statement" | "while_statement"
            | "with_statement" => {
                let mut bcursor = node.walk();
                for block_child in node.children(&mut bcursor) {
                    match block_child.kind() {
                        "block" => {
                            let mut ic = block_child.walk();
                            for stmt in block_child.children(&mut ic) {
                                Self::extract_module_bindings_from_stmt(
                                    stmt, language, source, out,
                                );
                                // Recurse into nested compound statements.
                                Self::descend_compound_for_bindings(stmt, language, source, out);
                            }
                        }
                        "else_clause" | "elif_clause" | "except_clause" | "finally_clause" => {
                            let mut cc = block_child.walk();
                            for clause_child in block_child.children(&mut cc) {
                                if clause_child.kind() == "block" {
                                    let mut ic = clause_child.walk();
                                    for stmt in clause_child.children(&mut ic) {
                                        Self::extract_module_bindings_from_stmt(
                                            stmt, language, source, out,
                                        );
                                        Self::descend_compound_for_bindings(
                                            stmt, language, source, out,
                                        );
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}
