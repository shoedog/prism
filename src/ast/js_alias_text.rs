//! Slice R, rule 3 (owner decision 2026-10-13): the textual function-pointer path respects
//! bindings in JS/TS. `resolve_fptr_assignment` reads `h = f` from source text and ignores
//! scope; for a JS/TS site whose callee is a bare identifier it may emit its record only when
//! the binder agrees on both names: (i) the alias name at the site is bound, by the binder's
//! walk from that site, to the declaration or assignment the scan matched (the scope that
//! binds the site's `h` is the scope that binds the matched `h`), and (ii) the right-hand
//! identifier, looked up by the binder at its own position, is bound to a function or an
//! import. Anything else emits nothing. Other callee shapes and other languages are untouched.
use super::js_binding::{JsBinding, JsBindingCache};
use super::js_binding_walk::Walk;
use super::js_binding_writes::canonical_write_scope;
use super::ParsedFile;
use tree_sitter::Node;

/// The languages whose textual alias records the binder gates (rule 3).
pub(crate) fn is_js_ts_language(language: crate::languages::Language) -> bool {
    use crate::languages::Language;
    matches!(
        language,
        Language::JavaScript | Language::TypeScript | Language::Tsx
    )
}

/// The scan's match in the AST: the alias-name identifier and the right-hand identifier.
struct Matched<'a> {
    alias: Node<'a>,
    target: Node<'a>,
}

impl ParsedFile {
    /// Whether the textual path may emit `alias = target` for the call or JSX site spanning
    /// `site_start..site_end`, whose caller starts at `function_start`. `true` without a check
    /// when the site's callee is not a bare identifier (the rule does not apply to it).
    pub(crate) fn js_ts_text_alias_agrees<'a>(
        &'a self,
        function_start: usize,
        site_start: usize,
        site_end: usize,
        alias: &str,
        target: &str,
        cache: &mut JsBindingCache<'a>,
    ) -> bool {
        let Some(callee) = self.js_ts_callee_node(site_start, site_end) else {
            return true;
        };
        if self.node_text(&callee) != alias {
            return true;
        }
        let Some(m) = self.js_ts_text_alias_match(function_start, site_start, alias, target) else {
            return false;
        };
        let (Walk::Found(at_site, None), Walk::Found(at_decl, None)) = (
            self.js_ts_binding_walk(callee, alias, cache),
            self.js_ts_binding_walk(m.alias, alias, cache),
        ) else {
            return false;
        };
        canonical_write_scope(at_site).id() == canonical_write_scope(at_decl).id()
            && matches!(
                self.js_ts_site_binding(m.target, target, cache),
                JsBinding::Callable(_) | JsBinding::Import
            )
    }

    /// The one `alias = target` declarator or assignment ending before `site_start` inside the
    /// caller, with both sides plain identifiers (what the scan can match); `None` for none or
    /// several.
    fn js_ts_text_alias_match<'a>(
        &'a self,
        function_start: usize,
        site_start: usize,
        alias: &str,
        target: &str,
    ) -> Option<Matched<'a>> {
        let mut found: Option<Matched<'a>> = None;
        let mut stack = vec![self.tree.root_node()];
        while let Some(node) = stack.pop() {
            if node.end_byte() <= function_start || node.start_byte() >= site_start {
                continue;
            }
            let mut cursor = node.walk();
            stack.extend(node.named_children(&mut cursor));
            let (left, right) = match node.kind() {
                "variable_declarator" => (
                    node.child_by_field_name("name"),
                    node.child_by_field_name("value"),
                ),
                "assignment_expression" => (
                    node.child_by_field_name("left"),
                    node.child_by_field_name("right"),
                ),
                _ => continue,
            };
            let plain = |n: Option<Node<'a>>, text: &str| {
                n.filter(|n| n.kind() == "identifier" && self.node_text(n) == text)
            };
            if let (Some(alias), Some(target)) = (plain(left, alias), plain(right, target)) {
                if node.end_byte() > site_start || found.is_some() {
                    return None;
                }
                found = Some(Matched { alias, target });
            }
        }
        found
    }
}
