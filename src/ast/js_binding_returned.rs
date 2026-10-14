//! Slice V: a `const` binding that takes its value from a call to a bare identifier `H` is a
//! returned-value binding when `H` is proven to be one in-repo function. This is the binder
//! half: it classifies the declarator's form (whole value, object key, array slot) and `H`'s
//! terminal; what `H` returns is the call graph's returned-value fact, read by the resolver.
//!
//! The query is separate from `js_ts_classify`: every declarator it follows is classified
//! `Alias` or `MayCall` there (an admitted React wrapper is `Callable` and never reaches this
//! query), so the outcome of every site it does not follow stays what the base gives.
use super::js_binding::{JsBinding, JsBindingCache};
use super::ParsedFile;
use crate::ast::JsFollow;
use crate::js_returns::JsReturnPath;
use tree_sitter::Node;

impl ParsedFile {
    /// For the identifier `ident` (spelled `name`) whose binding the base classifies `Alias` or
    /// `MayCall`: the terminal of the called `H` and the part of its result `name` takes, when
    /// the declarator is a unique, unwritten `const` of form (a) `x = H(...)`, (b) a flat object
    /// pattern or (c) a flat array pattern over `H(...)`, `H` a bare identifier and the call
    /// neither `new`, optional-chained nor awaited.
    pub(crate) fn js_ts_follow_returned<'a>(
        &'a self,
        ident: Node<'a>,
        name: &str,
        cache: &mut JsBindingCache<'a>,
    ) -> Option<(JsFollow, JsReturnPath)> {
        if !matches!(
            self.js_ts_site_binding(ident, name, cache),
            JsBinding::Alias | JsBinding::MayCall
        ) {
            return None;
        }
        let (scope, decl) = self.js_ts_alias_declarator(ident, name, cache)?;
        if self.js_ts_scoped_written(scope, name, cache) {
            return None;
        }
        let pattern = decl.child_by_field_name("name")?;
        let call = self.js_ts_unwrap_value(decl.child_by_field_name("value")?)?;
        let hook = self.js_ts_hook_identifier(call)?;
        let path = match pattern.kind() {
            "identifier" => JsReturnPath::Whole,
            "object_pattern" => JsReturnPath::Key(self.js_ts_object_pattern_key(pattern, name)?),
            "array_pattern" => JsReturnPath::Slot(self.js_ts_array_pattern_slot(pattern, name)?),
            _ => return None,
        };
        let hook_name = self.node_text(&hook).to_string();
        let terminal = self.js_ts_terminal_of(hook, &hook_name, cache)?;
        Some((terminal, path))
    }

    /// The bare identifier `H` of a plain call `H(...)`: not `new`, not a tagged template, not
    /// optional-chained (`H?.()` is an `optional_chain` child in JavaScript and a `?.` token in
    /// TypeScript).
    fn js_ts_hook_identifier<'a>(&self, call: Node<'a>) -> Option<Node<'a>> {
        if call.kind() != "call_expression"
            || call
                .child_by_field_name("arguments")
                .is_none_or(|a| a.kind() != "arguments")
        {
            return None;
        }
        let mut cursor = call.walk();
        let optional = call
            .children(&mut cursor)
            .any(|c| matches!(c.kind(), "optional_chain" | "?."));
        call.child_by_field_name("function")
            .filter(|f| f.kind() == "identifier" && !optional)
    }

    /// The slot `name` takes in a flat array pattern: every element anywhere in the pattern is a
    /// plain identifier or a hole (no rest, default or nested pattern), and exactly one is
    /// `name`.
    pub(super) fn js_ts_array_pattern_slot(&self, pattern: Node<'_>, name: &str) -> Option<usize> {
        let mut slots = Vec::new();
        let mut index = 0;
        let mut cursor = pattern.walk();
        for child in pattern.children(&mut cursor) {
            match child.kind() {
                "[" | "]" | "comment" => {}
                "," => index += 1,
                "identifier" => {
                    if self.node_text(&child) == name {
                        slots.push(index);
                    }
                }
                _ => return None,
            }
        }
        match slots.as_slice() {
            [slot] => Some(*slot),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "js_binding_returned_tests.rs"]
mod tests;
