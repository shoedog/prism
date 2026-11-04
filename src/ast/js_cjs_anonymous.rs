//! #597: the consumer half of a CommonJS module call made from an anonymous callable. The
//! barrier is narrower than the named-owner binder's (`js_cjs_module_calls.rs`): the name must
//! be declared by a direct top-level `const` or `var` statement of the program (owner 2026-11-04;
//! `let` stays out), never written, declared exactly once in the file, and the
//! binding walk from the call site must reach the program scope without meeting another
//! declaration of the name. `let` bindings, inline `require('./x').f(..)` and every
//! declaration the binder does not prove keep the earlier outcome.
//!
//! Shapes: `m(..)` and `m.f(..)` with `const m = require('./x')`, `f(..)` with
//! `const f = require('./x').f`, and `f(..)` with `const { f } = require('./x')` or
//! `const { k: f } = require('./x')` (a flat object pattern: no rest, default or nested pattern).
//! The module is proven by `js_ts_cjs_require_literal` (one plain relative string literal,
//! `require` unshadowed at its own occurrence and unwritten, no enclosing `with` body).
use super::js_binding::{JsBinding, JsBindingCache};
use super::js_binding_walk::Walk;
use super::ParsedFile;
use crate::call_graph::JsLocalBinding;
use tree_sitter::Node;

impl ParsedFile {
    /// For the `call_expression` `call`: the module and member its callee takes from a
    /// top-level `const` CommonJS `require`, or `None` wherever the barrier does not hold.
    /// (`member`: `None` is the module value itself.)
    pub(crate) fn js_ts_cjs_anonymous_require<'a>(
        &'a self,
        call: Node<'a>,
        cache: &mut JsBindingCache<'a>,
    ) -> Option<(String, Option<String>)> {
        let callee = call.child_by_field_name("function")?;
        match callee.kind() {
            "identifier" => {
                let name = self.node_text(&callee);
                match self.js_ts_cjs_require_binding(callee, cache) {
                    Some(found) => self
                        .js_ts_cjs_declared_const_or_var(callee, name, cache)
                        .then_some(found),
                    None => self.js_ts_cjs_destructured(callee, name, cache),
                }
            }
            "member_expression" => {
                let object = callee.child_by_field_name("object")?;
                let property = callee.child_by_field_name("property")?;
                if object.kind() != "identifier" || property.kind() != "property_identifier" {
                    return None;
                }
                let member = self.node_text(&property);
                match self.js_ts_cjs_member_binding(callee, member, cache)? {
                    JsLocalBinding::RequireModule {
                        module_path,
                        member: Some(member),
                    } if self.js_ts_cjs_declared_const_or_var(
                        object,
                        self.node_text(&object),
                        cache,
                    ) =>
                    {
                        Some((module_path, Some(member)))
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// The program-scope declaration `name` resolves to from `ident` is `const` or `var`, not
    /// `let` (owner 2026-11-04). One declaration of the name in the whole file and no write to it
    /// are the binder's `Alias` proof: a second declaration (a hoisted `var` in a nested block
    /// included) is `duplicate_declaration`, any write makes the binding `MayCall`.
    fn js_ts_cjs_declared_const_or_var<'a>(
        &'a self,
        ident: Node<'a>,
        name: &str,
        cache: &mut JsBindingCache<'a>,
    ) -> bool {
        self.js_ts_cjs_program_declarator(ident, name, cache)
            .and_then(|decl| decl.parent())
            .is_some_and(js_ts_cjs_const_or_var)
    }

    /// The one `variable_declarator` of a direct top-level statement of the program that the
    /// walk from `ident` reaches for `name`.
    fn js_ts_cjs_program_declarator<'a>(
        &'a self,
        ident: Node<'a>,
        name: &str,
        cache: &mut JsBindingCache<'a>,
    ) -> Option<Node<'a>> {
        let Walk::Found(scope, None) = self.js_ts_binding_walk(ident, name, cache) else {
            return None;
        };
        if scope.kind() != "program" {
            return None;
        }
        let (decls, annex_b) = self.js_ts_scope_lookup(scope, name, cache);
        let decls = decls.filter(|_| !annex_b)?;
        let [decl] = decls.as_slice() else {
            return None;
        };
        let list = decl.parent()?;
        (decl.kind() == "variable_declarator"
            && !decl.has_error()
            && matches!(list.kind(), "lexical_declaration" | "variable_declaration")
            && list.parent().is_some_and(|p| p.id() == scope.id()))
        .then_some(*decl)
    }

    /// `const { k: name } = require('./x')`: the module and the key `k` (`name` for a shorthand).
    fn js_ts_cjs_destructured<'a>(
        &'a self,
        ident: Node<'a>,
        name: &str,
        cache: &mut JsBindingCache<'a>,
    ) -> Option<(String, Option<String>)> {
        // `Alias` is: one declaration, unwritten, no default, in a scope that passed the
        // recovery and parse checks.
        if self.js_ts_site_binding(ident, name, cache) != JsBinding::Alias {
            return None;
        }
        let decl = self.js_ts_cjs_program_declarator(ident, name, cache)?;
        let list = decl.parent()?;
        if !js_ts_cjs_const_or_var(list) {
            return None;
        }
        let pattern = decl
            .child_by_field_name("name")
            .filter(|p| p.kind() == "object_pattern")?;
        let module_path =
            self.js_ts_cjs_require_literal(decl.child_by_field_name("value")?, cache)?;
        let mut key = None;
        let mut cursor = pattern.walk();
        for entry in pattern.named_children(&mut cursor) {
            let (local, entry_key) = match entry.kind() {
                "comment" => continue,
                "shorthand_property_identifier_pattern" => {
                    (self.node_text(&entry), self.node_text(&entry))
                }
                "pair_pattern" => {
                    let k = entry.child_by_field_name("key")?;
                    let v = entry.child_by_field_name("value")?;
                    if k.kind() != "property_identifier" || v.kind() != "identifier" {
                        return None;
                    }
                    (self.node_text(&v), self.node_text(&k))
                }
                // A rest element, a default or a nested pattern: not a flat member import.
                _ => return None,
            };
            if local == name {
                if key.replace(entry_key.to_owned()).is_some() {
                    return None;
                }
            }
        }
        key.map(|k| (module_path, Some(k)))
    }
}

/// A `var` statement, or a `const` one (never `let`).
fn js_ts_cjs_const_or_var(list: Node<'_>) -> bool {
    list.kind() == "variable_declaration"
        || (list.kind() == "lexical_declaration"
            && list.child(0).is_some_and(|k| k.kind() == "const"))
}
