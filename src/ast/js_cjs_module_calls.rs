//! #598: the consumer half of a CommonJS module call. A top-level declaration whose value is
//! `require('<relative literal>')`, or a member of one, names that module's export; the call
//! graph's CJS export facts (`JsExportFacts::cjs_callables`) decide whether a call through it
//! reaches one declared callable.
//!
//! The binder answers only what the name denotes at the site:
//! - `m(...)`, `m.f(...)` with `m` bound by `const|let|var m = require('./x')`;
//! - `require('./x').f(...)`;
//! - `f(...)` with `f` bound by `const|let|var f = require('./x').f`.
//!
//! Everything else keeps the base outcome (the caller returns `None`): a declaration that is
//! not a direct top-level statement (a block, a function, an `export`), a destructuring pattern
//! (the member-import route already owns it), a binding the base classifies as anything but an
//! unwritten value-flow alias (a write, a duplicate declaration, a shadowing parameter or inner
//! declaration), an ambient `require` that is shadowed, written or inside a `with` body,
//! a member the file writes through the module object (`m.f = ..`), and any specifier that is not
//! one plain relative string literal.
use super::js_binding::{JsBinding, JsBindingCache};
use super::js_binding_walk::Walk;
use super::ParsedFile;
use crate::call_graph::JsLocalBinding;
use std::collections::BTreeSet;
use tree_sitter::Node;

impl ParsedFile {
    /// The binding of a member-call callee (`q.f(...)`, `require('./x').f(...)`) as a CommonJS
    /// module member, or `None` when the callee is neither shape.
    pub(crate) fn js_ts_cjs_member_binding<'a>(
        &'a self,
        callee: Node<'a>,
        member: &str,
        cache: &mut JsBindingCache<'a>,
    ) -> Option<JsLocalBinding> {
        let object = callee.child_by_field_name("object")?;
        let property = callee.child_by_field_name("property")?;
        if property.kind() != "property_identifier"
            || self.node_text(&property) != member
            || self.js_ts_has_optional_chain(callee)
        {
            return None;
        }
        let module_path = match object.kind() {
            "call_expression" => self.js_ts_cjs_require_literal(object, cache)?,
            "identifier" => match self.js_ts_cjs_require_binding(object, cache)? {
                (module_path, None)
                    if !self.js_ts_cjs_member_written(self.node_text(&object), member, cache) =>
                {
                    module_path
                }
                // `q` is itself a member of a module (`q.f` is not a module export), or the
                // file writes `q.f` (a consumer-side stub).
                _ => return None,
            },
            _ => return None,
        };
        Some(JsLocalBinding::RequireModule {
            module_path,
            member: Some(member.to_owned()),
        })
    }

    /// For an identifier whose base binding is an unwritten value-flow alias: the module and
    /// member its single top-level declarator takes from `require('<relative>')` (no member:
    /// `x = require(..)`; a member: `x = require(..).m`).
    pub(crate) fn js_ts_cjs_require_binding<'a>(
        &'a self,
        ident: Node<'a>,
        cache: &mut JsBindingCache<'a>,
    ) -> Option<(String, Option<String>)> {
        let name = self.node_text(&ident);
        if !self.js_ts_is_require_local(name, cache) {
            return None;
        }
        // `Alias` already means: one declarator, unwritten (any write is `MayCall`), in a scope
        // that passed the recovery and parse checks.
        if self.js_ts_site_binding(ident, name, cache) != JsBinding::Alias {
            return None;
        }
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
        let declarator = self.js_ts_top_level_declarator(*decl, scope, name)?;
        let value = declarator.child_by_field_name("value")?;
        self.js_ts_require_value(value, cache)
    }

    /// `decl` is a `variable_declarator` of identifier `name` whose declaration is a statement of
    /// `program` itself.
    fn js_ts_top_level_declarator<'a>(
        &self,
        decl: Node<'a>,
        program: Node<'a>,
        name: &str,
    ) -> Option<Node<'a>> {
        let list = decl.parent()?;
        let plain = decl.kind() == "variable_declarator"
            && decl
                .child_by_field_name("name")
                .is_some_and(|n| n.kind() == "identifier" && self.node_text(&n) == name)
            && matches!(list.kind(), "lexical_declaration" | "variable_declaration")
            && list.parent().is_some_and(|p| p.id() == program.id())
            && list
                .child(0)
                .is_some_and(|k| matches!(k.kind(), "const" | "let" | "var"));
        plain.then_some(decl)
    }

    /// `require('<relative>')` or `require('<relative>').m`, nothing wrapped around it.
    fn js_ts_require_value<'a>(
        &'a self,
        value: Node<'a>,
        cache: &mut JsBindingCache<'a>,
    ) -> Option<(String, Option<String>)> {
        match value.kind() {
            "call_expression" => Some((self.js_ts_cjs_require_literal(value, cache)?, None)),
            "member_expression" => {
                let object = value.child_by_field_name("object")?;
                let property = value.child_by_field_name("property")?;
                if property.kind() != "property_identifier" || self.js_ts_has_optional_chain(value)
                {
                    return None;
                }
                Some((
                    self.js_ts_cjs_require_literal(object, cache)?,
                    Some(self.node_text(&property).to_owned()),
                ))
            }
            _ => None,
        }
    }

    /// The specifier of an unshadowed `require("<relative>")` with exactly one plain string
    /// argument: no template, no escape, no spread, no second argument, no optional call.
    /// Unshadowed is proved at this occurrence: no declaration of `require` binds it, no `with`
    /// body encloses it (an object environment may supply the name), the position is one the
    /// binding walk proves, and the file never writes `require`.
    pub(super) fn js_ts_cjs_require_literal<'a>(
        &'a self,
        call: Node<'a>,
        cache: &mut JsBindingCache<'a>,
    ) -> Option<String> {
        if call.kind() != "call_expression" || self.js_ts_has_optional_chain(call) {
            return None;
        }
        let function = call.child_by_field_name("function")?;
        if function.kind() != "identifier"
            || self.node_text(&function) != "require"
            || !self.js_ts_unshadowed_ambient(function)
            || !matches!(
                self.js_ts_binding_walk(function, "require", cache),
                Walk::Unbound
            )
        {
            return None;
        }
        let arguments = call.child_by_field_name("arguments")?;
        if arguments.kind() != "arguments" {
            return None;
        }
        let mut cursor = arguments.walk();
        let mut named = arguments
            .named_children(&mut cursor)
            .filter(|n| n.kind() != "comment");
        let literal = named.next().filter(|n| n.kind() == "string")?;
        if named.next().is_some() {
            return None;
        }
        let text = self.node_text(&literal);
        let spec = text.get(1..text.len().checked_sub(1)?)?;
        let plain = !spec.contains(['\\', '\'', '"', '`']) && !literal.has_error();
        (plain && (spec.starts_with("./") || spec.starts_with("../"))).then(|| spec.to_owned())
    }

    /// `?.` is an `optional_chain` child in JavaScript and a `?.` token in TypeScript.
    fn js_ts_has_optional_chain(&self, node: Node<'_>) -> bool {
        let mut cursor = node.walk();
        let found = node
            .children(&mut cursor)
            .any(|c| matches!(c.kind(), "optional_chain" | "?."));
        found
    }

    /// Names declared by a top-level declarator whose value is syntactically a `require(..)` call
    /// or a member of one, once per file. A cheap gate in front of the binding proof; it grants
    /// nothing.
    fn js_ts_is_require_local<'a>(&'a self, name: &str, cache: &mut JsBindingCache<'a>) -> bool {
        let locals = cache.require_locals.get_or_insert_with(|| {
            let mut names = BTreeSet::new();
            let root = self.tree.root_node();
            let mut cursor = root.walk();
            for statement in root.named_children(&mut cursor) {
                if !matches!(
                    statement.kind(),
                    "lexical_declaration" | "variable_declaration"
                ) {
                    continue;
                }
                let mut inner = statement.walk();
                for declarator in statement.named_children(&mut inner) {
                    let (Some(name), Some(value)) = (
                        declarator.child_by_field_name("name"),
                        declarator.child_by_field_name("value"),
                    ) else {
                        continue;
                    };
                    let call = if value.kind() == "member_expression" {
                        value.child_by_field_name("object")
                    } else {
                        Some(value)
                    };
                    if name.kind() == "identifier"
                        && call.is_some_and(|c| {
                            c.kind() == "call_expression"
                                && c.child_by_field_name("function").is_some_and(|f| {
                                    f.kind() == "identifier" && self.node_text(&f) == "require"
                                })
                        })
                    {
                        names.insert(self.node_text(&name).to_owned());
                    }
                }
            }
            names
        });
        locals.contains(name)
    }
}
