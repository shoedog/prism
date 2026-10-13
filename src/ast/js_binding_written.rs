//! Slice R (owner decision 2026-10-13): where the values written to a JS/TS binding are. A
//! binding whose value prism cannot prove gets no name search; the one same-file function that
//! may still carry the site's spelling is a function written into the binding, so the binder
//! hands the resolver the line spans of those value expressions (the declaration's initializer
//! and the right-hand sides of assignments to the binding in its scope). Function identities
//! carry line ranges, so line spans decide "inside".
use super::js_binding::JsBindingCache;
use super::js_binding_walk::Walk;
use super::js_binding_writes::canonical_write_scope;
use super::ParsedFile;
use tree_sitter::Node;

impl ParsedFile {
    /// The 1-indexed inclusive line spans of every value expression written to the binding that
    /// `name`, used at `ident`, denotes: the initializer of its declarator (a function
    /// declaration is its own value) and the right-hand side of each assignment whose target
    /// the scoped write resolver ties to the same scope. Empty when the walk does not end at a
    /// plain scope, or when no value is visible (a parameter, a bare `let h;`).
    pub(crate) fn js_ts_written_value_lines<'a>(
        &'a self,
        ident: Node<'a>,
        name: &str,
        cache: &mut JsBindingCache<'a>,
    ) -> Vec<(usize, usize)> {
        let Walk::Found(scope, None) = self.js_ts_binding_walk(ident, name, cache) else {
            return Vec::new();
        };
        let mut spans = Vec::new();
        let (decls, _) = self.js_ts_scope_lookup(scope, name, cache);
        for decl in decls.unwrap_or_default() {
            let value = match decl.kind() {
                "variable_declarator" => decl.child_by_field_name("value"),
                "assignment_expression" => decl.child_by_field_name("right"),
                "function_declaration" | "generator_function_declaration" => Some(decl),
                _ => None,
            };
            spans.extend(value.map(|v| self.node_line_range(&v)));
        }
        let canon = canonical_write_scope(scope).id();
        for target in self.js_ts_named_write_targets(name, cache) {
            let ties = matches!(
                self.js_ts_binding_walk(target, name, cache),
                Walk::Found(s, _) if canonical_write_scope(s).id() == canon
            );
            if ties {
                spans.extend(
                    self.js_ts_assigned_value(target)
                        .map(|v| self.node_line_range(&v)),
                );
            }
        }
        spans.sort_unstable();
        spans.dedup();
        spans
    }

    /// The right-hand side of the assignment that writes `target` (the write resolver's target
    /// is the assignment's whole left side, a destructuring pattern included); `None` for an
    /// update, a `for` head or anything that assigns no visible value.
    fn js_ts_assigned_value<'a>(&self, target: Node<'a>) -> Option<Node<'a>> {
        let assignment = target.parent().filter(|p| {
            matches!(
                p.kind(),
                "assignment_expression" | "augmented_assignment_expression"
            )
        })?;
        let left = assignment.child_by_field_name("left")?;
        (left.id() == target.id())
            .then(|| assignment.child_by_field_name("right"))
            .flatten()
    }

    /// The callee node of the call or JSX element that spans exactly these bytes: the call's
    /// `function` field or the element's `name`.
    pub(crate) fn js_ts_callee_node(&self, start_byte: usize, end_byte: usize) -> Option<Node<'_>> {
        let site = self
            .tree
            .root_node()
            .descendant_for_byte_range(start_byte, end_byte)
            .filter(|n| n.start_byte() == start_byte && n.end_byte() == end_byte)?;
        match site.kind() {
            "call_expression" => site.child_by_field_name("function"),
            "jsx_self_closing_element" | "jsx_opening_element" => site.child_by_field_name("name"),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "js_binding_written_tests.rs"]
mod tests;
