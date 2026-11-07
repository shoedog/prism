//! Go import extraction.

use crate::ast::ParsedFile;
use std::collections::BTreeMap;
use tree_sitter::Node;

impl ParsedFile {
    pub(in crate::ast) fn collect_go_imports(
        &self,
        node: Node<'_>,
        out: &mut BTreeMap<String, String>,
    ) {
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
}
