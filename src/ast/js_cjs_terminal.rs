//! Per-occurrence proof for direct CommonJS callable values, not live bindings.
use super::ParsedFile;
use crate::js_exports::JsExportTarget;
use std::collections::BTreeSet;
use tree_sitter::Node;

impl ParsedFile {
    pub(super) fn js_ts_cjs_local_target(&self, rhs: Node<'_>) -> JsExportTarget {
        let name = self.node_text(&rhs);
        if self.js_ts_cjs_terminal_proven(rhs, name) {
            JsExportTarget::Local(name.to_owned())
        } else {
            // Keep the claim: dropping it could erase duplicate or star conflicts.
            JsExportTarget::UnprovenLocal(name.to_owned())
        }
    }

    fn js_ts_cjs_terminal_proven(&self, rhs: Node<'_>, name: &str) -> bool {
        let root = self.tree.root_node();
        if root.has_error()
            || self
                .extract_import_bindings()
                .iter()
                .any(|b| b.local == name)
            || self.js_ts_type_only_imports().contains_key(name)
        {
            return false;
        }
        let mut capture = rhs;
        while let Some(parent) = capture.parent() {
            if parent.id() == root.id() {
                break;
            }
            capture = parent;
        }
        if capture.kind() != "expression_statement" {
            return false;
        }
        let mut candidate = None;
        let mut cursor = root.walk();
        for wrapper in root.named_children(&mut cursor) {
            let declaration = if wrapper.kind() == "export_statement" {
                let Some(d) = wrapper.child_by_field_name("declaration") else {
                    continue;
                };
                d
            } else {
                wrapper
            };
            if matches!(
                declaration.kind(),
                "function_declaration" | "generator_function_declaration"
            ) && declaration
                .child_by_field_name("name")
                .is_some_and(|n| self.node_text(&n) == name)
                && declaration.child_by_field_name("body").is_some()
            {
                if candidate.replace((declaration, declaration)).is_some() {
                    return false;
                }
            } else if matches!(
                declaration.kind(),
                "lexical_declaration" | "variable_declaration"
            ) {
                let mut dc = declaration.walk();
                for d in declaration.named_children(&mut dc) {
                    if d.kind() != "variable_declarator"
                        || !d
                            .child_by_field_name("name")
                            .is_some_and(|n| n.kind() == "identifier" && self.node_text(&n) == name)
                    {
                        continue;
                    }
                    let Some(value) = d.child_by_field_name("value") else {
                        return false;
                    };
                    if !matches!(value.kind(), "arrow_function" | "function_expression")
                        || d.end_byte() > capture.start_byte()
                        || candidate.replace((d, value)).is_some()
                    {
                        return false;
                    }
                }
            }
        }
        let Some((declaration, callable)) = candidate else {
            return false;
        };
        let mut indexed = self
            .functions()
            .iter()
            .filter(|f| f.name.as_deref() == Some(name));
        let Some(indexed_callable) = indexed.next() else {
            return false;
        };
        if indexed.next().is_some()
            || indexed_callable.start_byte != callable.start_byte()
            || indexed_callable.end_byte != callable.end_byte()
            || self.js_ts_forwarding_competitor_except(
                root,
                name,
                &[declaration.id(), callable.id()],
            )
        {
            return false;
        }
        !self.js_ts_cjs_capture_written(root, name, capture.end_byte())
    }

    fn js_ts_cjs_capture_written(&self, node: Node<'_>, name: &str, capture_end: usize) -> bool {
        let root = self.tree.root_node();
        if let Some(target) = self.js_ts_write_target(node) {
            let mut names = BTreeSet::new();
            self.collect_js_ts_binding_pattern_names(target, &mut names);
            if names.contains(name)
                && !self.js_ts_has_closer_binding(&target, name, root.id(), true)
            {
                // Only a plain, later root assignment has an unambiguous capture
                // ordering. Nested writes can execute earlier despite their span.
                let later_root_assignment = node.kind() == "assignment_expression"
                    && target.kind() == "identifier"
                    && node.start_byte() >= capture_end
                    && node.parent().is_some_and(|p| {
                        p.kind() == "expression_statement"
                            && p.parent().is_some_and(|g| g.id() == root.id())
                    });
                if !later_root_assignment {
                    return true;
                }
            }
        }
        let mut cursor = node.walk();
        let written = node
            .named_children(&mut cursor)
            .any(|child| self.js_ts_cjs_capture_written(child, name, capture_end));
        written
    }
}
