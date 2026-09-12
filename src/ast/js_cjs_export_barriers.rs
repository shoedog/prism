//! Refusal-only custody of the producer's syntactic export-object uses.
//! This does not prove callable-local identity or require-time snapshots.
use super::ParsedFile;
use std::collections::BTreeSet;
use tree_sitter::Node;

impl ParsedFile {
    pub(super) fn js_ts_cjs_export_object_safe(&self) -> bool {
        let root = self.tree.root_node();
        if root.has_error() {
            return false;
        }
        let mut allowed = BTreeSet::new();
        let mut replacements = 0;
        let mut members = 0;
        let mut cursor = root.walk();
        for statement in root.named_children(&mut cursor) {
            if statement.kind() != "expression_statement" {
                continue;
            }
            let mut children = statement.walk();
            let Some(assign) = statement
                .named_children(&mut children)
                .find(|n| n.kind() != "comment")
            else {
                continue;
            };
            if assign.kind() != "assignment_expression" {
                continue;
            }
            let (Some(left), Some(right)) = (
                assign.child_by_field_name("left"),
                assign.child_by_field_name("right"),
            ) else {
                continue;
            };
            let Some((ambient, whole)) = self.js_ts_cjs_export_lhs(left) else {
                continue;
            };
            if !self.js_ts_unshadowed_ambient(ambient) {
                return false;
            }
            allowed.insert(ambient.id());
            if whole {
                replacements += 1;
                if !self.js_ts_cjs_replacement_safe(right) {
                    return false;
                }
            } else {
                members += 1;
                // Unsupported expression writes must not silently disappear
                // while a previous identifier assignment remains authoritative.
                if right.kind() != "identifier" {
                    return false;
                }
            }
        }
        replacements <= 1
            && (replacements == 0 || members == 0)
            && !self.js_ts_cjs_unaccounted_use(root, &allowed)
    }

    /// The exact ambient occurrence is admitted, never its spelling elsewhere.
    fn js_ts_cjs_export_lhs<'a>(&self, node: Node<'a>) -> Option<(Node<'a>, bool)> {
        if node.kind() != "member_expression" {
            return None;
        }
        let object = node.child_by_field_name("object")?;
        let property = node.child_by_field_name("property")?;
        if property.kind() != "property_identifier" {
            return None;
        }
        if object.kind() == "identifier" {
            return match (self.node_text(&object), self.node_text(&property)) {
                ("module", "exports") => Some((object, true)),
                ("exports", _) => Some((object, false)),
                _ => None,
            };
        }
        let (ambient, whole) = self.js_ts_cjs_export_lhs(object)?;
        whole.then_some((ambient, false))
    }

    fn js_ts_cjs_replacement_safe(&self, rhs: Node<'_>) -> bool {
        if rhs.kind() == "identifier" {
            return true;
        }
        if rhs.kind() != "object" {
            return false;
        }
        let mut keys = BTreeSet::new();
        let mut cursor = rhs.walk();
        for property in rhs.named_children(&mut cursor) {
            let key = match property.kind() {
                "comment" => continue,
                "shorthand_property_identifier" => self.node_text(&property),
                "pair" => {
                    let (Some(key), Some(value)) = (
                        property.child_by_field_name("key"),
                        property.child_by_field_name("value"),
                    ) else {
                        return false;
                    };
                    if !matches!(key.kind(), "property_identifier" | "string")
                        || value.kind() != "identifier"
                    {
                        return false;
                    }
                    self.node_text(&key).trim_matches(['\'', '"'])
                }
                _ => return false, // computed keys, methods, accessors, spreads
            };
            // No string-escape decoding or special prototype-setter semantics
            // in this bounded proof. Both can hide ordinary-key collisions.
            if key.contains('\\') || key == "__proto__" || !keys.insert(key) {
                return false;
            }
        }
        true
    }

    fn js_ts_cjs_unaccounted_use(&self, node: Node<'_>, allowed: &BTreeSet<usize>) -> bool {
        let spelling = self.node_text(&node);
        // Do not infer identity from an undecoded escaped identifier/property.
        // `arguments` can expose exports/module through the CommonJS wrapper.
        if matches!(
            node.kind(),
            "identifier"
                | "property_identifier"
                | "shorthand_property_identifier"
                | "shorthand_property_identifier_pattern"
        ) && spelling.contains('\\')
        {
            return true;
        }
        if matches!(node.kind(), "this" | "with_statement")
            || (matches!(
                node.kind(),
                "identifier"
                    | "shorthand_property_identifier"
                    | "shorthand_property_identifier_pattern"
            ) && (matches!(spelling, "eval" | "arguments")
                || (matches!(spelling, "module" | "exports")
                    && !allowed.contains(&node.id())
                    && !self.js_ts_receiver_has_closer_binding(
                        &node,
                        spelling,
                        self.tree.root_node().id(),
                    ))))
        {
            return true;
        }
        let mut cursor = node.walk();
        let found = node
            .named_children(&mut cursor)
            .any(|child| self.js_ts_cjs_unaccounted_use(child, allowed));
        found
    }
}
