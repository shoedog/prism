//! Unchanged raw walks, evaluated once per session key.
#[cfg(test)]
use super::super::js_module_forwarding;
use super::super::{js_binding_decls, ParsedFile};
use crate::languages::Language;
use std::collections::BTreeSet;
use tree_sitter::Node;

impl ParsedFile {
    /// Replacing module.exports may detach the original exports object. Refuse
    /// exports.* throughout the file, including possible reattachment; no heap
    /// alias or execution-order proof is available in this export-fact model.
    pub(super) fn js_ts_module_exports_replaced_uncached(&self, node: Node<'_>) -> bool {
        if let Some(left) = self.js_ts_write_target(node) {
            if left.kind() == "member_expression"
                && left
                    .child_by_field_name("object")
                    .is_some_and(|n| self.node_text(&n) == "module")
                && left
                    .child_by_field_name("property")
                    .is_some_and(|n| self.node_text(&n) == "exports")
            {
                return true;
            }
        }
        let mut cursor = node.walk();
        let found = node
            .named_children(&mut cursor)
            .any(|n| self.js_ts_module_exports_replaced_uncached(n));
        found
    }

    /// The module write scan; with `skip_using`, a TS `using` declaration declares and is
    /// never a W1 write (S1b SPEC §3.1 D4, W1).
    pub(super) fn js_ts_module_written_uncached(&self, name: &str, skip_using: bool) -> bool {
        if !matches!(
            self.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) {
            return false;
        }
        fn walk(parsed: &ParsedFile, node: Node<'_>, root_id: usize, name: &str, u: bool) -> bool {
            #[cfg(test)]
            js_module_forwarding::tests::visit();
            if matches!(
                node.kind(),
                "assignment_expression"
                    | "augmented_assignment_expression"
                    | "update_expression"
                    | "for_in_statement"
                    | "for_of_statement"
                    | "for_await_statement"
            ) && !(u && js_binding_decls::js_ts_is_using(node))
            {
                if let Some(target) = node
                    .child_by_field_name("left")
                    .or_else(|| node.child_by_field_name("argument"))
                    .or_else(|| node.named_child(0))
                {
                    let mut names = BTreeSet::new();
                    parsed.collect_js_ts_binding_pattern_names(target, &mut names);
                    if names.contains(name)
                        && !parsed.js_ts_has_closer_binding(&target, name, root_id, true)
                    {
                        return true;
                    }
                }
            }
            let mut cursor = node.walk();
            let mut children = node.named_children(&mut cursor);
            children.any(|child| walk(parsed, child, root_id, name, u))
        }
        let root = self.tree.root_node();
        walk(self, root, root.id(), name, skip_using)
    }
}
