//! Per-occurrence proof for direct CommonJS callable values, not live bindings.
use super::ParsedFile;
use crate::ast::js_ts::exports::{CjsCallable, JsExportFacts, JsExportTarget};
use std::collections::BTreeSet;
use tree_sitter::Node;

impl ParsedFile {
    /// The raw export claim for an identifier value, plus (#598) the callable the terminal proof
    /// found when there is one. The claim is unchanged by the second element.
    pub(super) fn js_ts_cjs_local_target(
        &self,
        rhs: Node<'_>,
    ) -> (JsExportTarget, Option<CjsCallable>) {
        let name = self.node_text(&rhs);
        if let Some(callable) = self.js_ts_cjs_terminal_proven(rhs, name) {
            let target = self
                .js_ts_cjs_direct_target(callable, name)
                .unwrap_or_else(|| JsExportTarget::Local(name.to_owned()));
            (target, self.js_ts_cjs_callable_span(callable, name))
        } else {
            // Keep the claim: dropping it could erase duplicate or star conflicts.
            (JsExportTarget::UnprovenLocal(name.to_owned()), None)
        }
    }

    /// Records `exported` for an identifier value, and the proven callable under `exported`
    /// (`None`: the whole-module replacement `module.exports = f`).
    pub(super) fn insert_js_ts_cjs_local(
        &self,
        facts: &mut JsExportFacts,
        exported: Option<String>,
        rhs: Node<'_>,
    ) {
        let (target, callable) = self.js_ts_cjs_local_target(rhs);
        match (&exported, callable) {
            (Some(name), Some(c)) => {
                facts.cjs_callables.insert(name.clone(), c);
            }
            (None, Some(c)) => facts.cjs_module_callable = Some(c),
            _ => {}
        }
        facts.insert_named(exported.unwrap_or_else(|| "default".to_owned()), target);
    }

    /// The indexed callable the terminal proof already matched by bytes, as its line span.
    fn js_ts_cjs_callable_span(&self, callable: Node<'_>, name: &str) -> Option<CjsCallable> {
        let mut indexed = self
            .functions()
            .iter()
            .filter(|f| f.name.as_deref() == Some(name));
        let found = indexed.next()?;
        (indexed.next().is_none()
            && found.start_byte == callable.start_byte()
            && found.end_byte == callable.end_byte())
        .then(|| CjsCallable {
            local: name.to_owned(),
            start_line: found.start_line,
            end_line: found.end_line,
        })
    }

    /// Reuse the terminal's exact indexed callable, never a same-spelled node: the proven
    /// callable of an identifier export whose function expression carries that same name.
    /// (Direct member and module-value function expressions use `js_ts_cjs_named_expr_target`.)
    pub(super) fn js_ts_cjs_direct_target(
        &self,
        rhs: Node<'_>,
        exported: &str,
    ) -> Option<JsExportTarget> {
        if rhs.kind() != "function_expression"
            || !rhs
                .child_by_field_name("name")
                .is_some_and(|n| self.node_text(&n) == exported)
            || self.tree.root_node().has_error()
        {
            return None;
        }
        let mut indexed = self
            .functions()
            .iter()
            .filter(|f| f.name.as_deref() == Some(exported));
        let callable = indexed.next()?;
        if indexed.next().is_some()
            || callable.start_byte != rhs.start_byte()
            || callable.end_byte != rhs.end_byte()
        {
            return None;
        }
        Some(JsExportTarget::VerifiedLocal {
            local: exported.to_owned(),
            start_line: callable.start_line,
            end_line: callable.end_line,
        })
    }

    /// #627: a named function expression used directly as an export value
    /// (`module.exports = function g() {}`, `exports.f = function g() {}`). The target is the
    /// indexed callable with the expression's own bytes, so a same-spelled declaration elsewhere
    /// is never bound; the name is only the lookup key of the index entry. Anonymous
    /// expressions, arrows, classes and parenthesized values have no such identity and refuse.
    pub(super) fn js_ts_cjs_named_expr_target(&self, rhs: Node<'_>) -> Option<JsExportTarget> {
        if rhs.kind() != "function_expression" || self.tree.root_node().has_error() {
            return None;
        }
        let name = self.node_text(&rhs.child_by_field_name("name")?);
        let mut indexed = self.functions().iter().filter(|f| {
            f.name.as_deref() == Some(name)
                && f.start_byte == rhs.start_byte()
                && f.end_byte == rhs.end_byte()
        });
        let callable = indexed.next()?;
        indexed
            .next()
            .is_none()
            .then(|| JsExportTarget::VerifiedLocal {
                local: name.to_owned(),
                start_line: callable.start_line,
                end_line: callable.end_line,
            })
    }

    /// Records a `VerifiedLocal` target under `exported` as the proven callable (`None`: the
    /// whole-module replacement) and as the export claim.
    pub(super) fn insert_js_ts_cjs_verified(
        &self,
        facts: &mut JsExportFacts,
        exported: Option<String>,
        target: JsExportTarget,
    ) {
        if let JsExportTarget::VerifiedLocal {
            local,
            start_line,
            end_line,
        } = &target
        {
            let callable = CjsCallable {
                local: local.clone(),
                start_line: *start_line,
                end_line: *end_line,
            };
            match &exported {
                Some(name) => {
                    facts.cjs_callables.insert(name.clone(), callable);
                }
                None => facts.cjs_module_callable = Some(callable),
            }
        }
        facts.insert_named(exported.unwrap_or_else(|| "default".to_owned()), target);
    }

    pub(super) fn collect_js_ts_cjs_member_value(
        &self,
        name: String,
        rhs: Node<'_>,
        facts: &mut crate::ast::js_ts::exports::JsExportFacts,
    ) {
        if rhs.kind() == "identifier" {
            self.insert_js_ts_cjs_local(facts, Some(name), rhs);
        } else if let Some(target) = self.js_ts_cjs_named_expr_target(rhs) {
            self.insert_js_ts_cjs_verified(facts, Some(name), target);
        } else {
            facts.skipped_expr_count += 1;
        }
    }

    fn js_ts_cjs_terminal_proven<'a>(&'a self, rhs: Node<'a>, name: &str) -> Option<Node<'a>> {
        let root = self.tree.root_node();
        if root.has_error()
            || self
                .extract_import_bindings()
                .iter()
                .any(|b| b.local == name)
            || self.js_ts_type_only_imports().contains_key(name)
        {
            return None;
        }
        let mut capture = rhs;
        while let Some(parent) = capture.parent() {
            if parent.id() == root.id() {
                break;
            }
            capture = parent;
        }
        if capture.kind() != "expression_statement" {
            return None;
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
                    return None;
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
                    let value = d.child_by_field_name("value")?;
                    if !matches!(value.kind(), "arrow_function" | "function_expression")
                        || d.end_byte() > capture.start_byte()
                        || candidate.replace((d, value)).is_some()
                    {
                        return None;
                    }
                }
            }
        }
        let (declaration, callable) = candidate?;
        let mut indexed = self
            .functions()
            .iter()
            .filter(|f| f.name.as_deref() == Some(name));
        let indexed_callable = indexed.next()?;
        if indexed.next().is_some()
            || indexed_callable.start_byte != callable.start_byte()
            || indexed_callable.end_byte != callable.end_byte()
            || self.js_ts_forwarding_competitor_except(
                root,
                name,
                &[declaration.id(), callable.id()],
            )
        {
            return None;
        }
        (!self.js_ts_cjs_capture_written(root, name, capture.end_byte())).then_some(callable)
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
