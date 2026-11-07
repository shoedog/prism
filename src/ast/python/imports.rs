//! Python import extraction and inert package initializers.

use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::BTreeMap;
use tree_sitter::Node;

impl ParsedFile {
    pub(in crate::ast) fn collect_python_imports(
        &self,
        node: Node<'_>,
        out: &mut BTreeMap<String, String>,
    ) {
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
                            "wildcard_import" => {
                                out.insert("*".to_string(), "*".to_string());
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

    /// Collect imports from a single module-scope Python node.
    ///
    /// Called for each direct child of the root. If the node is a compound
    /// statement (`if`/`try`/`for`/`while`/`with`), we walk one level into
    /// their block/clause children for imports — those ARE module-scope in
    /// Python. We do NOT recurse into `function_definition` or
    /// `class_definition` bodies (their imports are function-/class-local).
    pub(in crate::ast) fn collect_python_module_scope_imports(
        &self,
        node: Node<'_>,
        out: &mut Vec<crate::call_graph::ImportBinding>,
    ) {
        match node.kind() {
            "import_statement" | "import_from_statement" => {
                self.collect_python_import_node(node, out);
            }
            // Module-scope compound statements can contain imports that are
            // still module-scope (e.g. `if TYPE_CHECKING: from x import y`).
            "if_statement" | "try_statement" | "for_statement" | "while_statement"
            | "with_statement" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    match child.kind() {
                        "block" | "except_clause" | "finally_clause" | "else_clause" => {
                            let mut bc = child.walk();
                            for stmt in child.children(&mut bc) {
                                if stmt.kind() == "import_statement"
                                    || stmt.kind() == "import_from_statement"
                                {
                                    self.collect_python_import_node(stmt, out);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            // Do NOT walk into function_definition, class_definition,
            // decorated_definition — their imports are not module-scope.
            _ => {}
        }
    }

    /// Extract binding data from a single Python import/import_from node.
    fn collect_python_import_node(
        &self,
        node: Node<'_>,
        out: &mut Vec<crate::call_graph::ImportBinding>,
    ) {
        use crate::call_graph::{ImportBinding, ImportBindingKind};
        match node.kind() {
            "import_statement" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    match child.kind() {
                        "dotted_name" => {
                            let name = self.node_text(&child).to_string();
                            // Python binds the root for an unaliased dotted import:
                            // `import pkg.models` creates local `pkg`, not `models`.
                            let alias = name.split('.').next().unwrap_or(&name).to_string();
                            out.push(ImportBinding {
                                local: alias,
                                module_path: name,
                                member: None,
                                kind: ImportBindingKind::ModuleImport,
                                eligible: false, // module imports don't resolve unqualified calls
                            });
                        }
                        "aliased_import" => {
                            let module = child
                                .child_by_field_name("name")
                                .map(|n| self.node_text(&n).to_string());
                            let alias = child
                                .child_by_field_name("alias")
                                .map(|n| self.node_text(&n).to_string());
                            if let (Some(module), Some(alias)) = (module, alias) {
                                out.push(ImportBinding {
                                    local: alias,
                                    module_path: module,
                                    member: None,
                                    kind: ImportBindingKind::AliasedModuleImport,
                                    eligible: false,
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }
            "import_from_statement" => {
                let module = node
                    .child_by_field_name("module_name")
                    .map(|n| self.node_text(&n).to_string())
                    .or_else(|| {
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
                                if name != module {
                                    out.push(ImportBinding {
                                        local: name.clone(),
                                        module_path: module.clone(),
                                        member: Some(name),
                                        kind: ImportBindingKind::MemberImport,
                                        eligible: true, // eligibility set later
                                    });
                                }
                            }
                            "aliased_import" => {
                                let original = child
                                    .child_by_field_name("name")
                                    .map(|n| self.node_text(&n).to_string());
                                let alias = child
                                    .child_by_field_name("alias")
                                    .map(|n| self.node_text(&n).to_string());
                                if let Some(alias) = alias {
                                    out.push(ImportBinding {
                                        local: alias,
                                        module_path: module.clone(),
                                        member: original,
                                        kind: ImportBindingKind::MemberImport,
                                        eligible: true,
                                    });
                                }
                            }
                            "wildcard_import" => {
                                out.push(ImportBinding {
                                    local: "*".to_string(),
                                    module_path: module.clone(),
                                    member: None,
                                    kind: ImportBindingKind::WildcardImport,
                                    eligible: false,
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// An inert initializer cannot install a competing package attribute or
    /// execute an import hook. Deliberately narrower than a binding-name scan.
    pub(crate) fn python_inert_initializer(&self) -> bool {
        if self.language != Language::Python {
            return false;
        }
        let root = self.tree.root_node();
        if root.has_error() {
            return false;
        }
        let mut cursor = root.walk();
        let inert = root.named_children(&mut cursor).all(|node| {
            if matches!(node.kind(), "comment" | "pass_statement") {
                return true;
            }
            if node.kind() != "expression_statement" || node.named_child_count() != 1 {
                return false;
            }
            let Some(value) = node.named_child(0) else {
                return false;
            };
            if value.kind() != "string" {
                return false;
            }
            let text = self.node_text(&value);
            let prefix = text
                .split(['\'', '"'])
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            if !matches!(prefix.as_str(), "" | "r" | "u") {
                return false;
            }
            let mut cursor = value.walk();
            let plain = value
                .named_children(&mut cursor)
                .all(|n| n.kind() != "interpolation");
            plain
        });
        inert
    }
}
