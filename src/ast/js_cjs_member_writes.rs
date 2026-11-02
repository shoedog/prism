//! #598: consumer-side monkey-patching of a CommonJS module object. `const m = require('./x');
//! m.f = stub;` replaces what `m.f()` calls, but the binding `m` itself is never written, so the
//! base write resolver calls the declarator unwritten. A member call `m.f()` is therefore refused
//! when the file statically writes `m.f`, or any member of `m` it cannot name.
//!
//! The index is by spelling and file-wide, not by scope: a write to `m.f` in an unrelated
//! callable that shadows `m` refuses too. That is the conservative direction (a refusal keeps
//! the earlier outcome). A write is the shape the parameter-Def lane already classifies
//! (`member_is_write_target`: assignment of every kind, `++`/`--`, `delete`, array/object
//! pattern targets, `for (m.f of ..)`); a computed or string-keyed member write, and a first
//! argument `m` of an `Object.*`/`Reflect.*` mutator, may replace any key. Passing `m` to an
//! arbitrary function stays out of model, as for every other module object.
//!
//! This is a partial, spelling-based guard, not a mutation model: an alias (`n = m; n.f = …`),
//! `with (m)`, re-acquisition through `require('./x').f = …`, `(0, m).f = …`, and reflective or
//! computed mutator calls (`Object['assign'](m, …)`, `Object.assign.call(null, m, …)`) are not
//! seen and stay under CLAUDE.md's runtime-mutation rule.
use super::js_binding::JsBindingCache;
use super::member_projection::strip_transparent_wrappers;
use super::ParsedFile;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

/// What a file writes through one identifier: named members, and whether any write may hit an
/// unnamed key.
#[derive(Default)]
pub(crate) struct MemberWrites {
    named: BTreeSet<String>,
    any: bool,
}

/// `Object.<name>(target, ..)` and `Reflect.<name>(target, ..)` that change `target`'s keys.
const OBJECT_MUTATORS: &[&str] = &[
    "assign",
    "defineProperty",
    "defineProperties",
    "setPrototypeOf",
];
const REFLECT_MUTATORS: &[&str] = &["set", "defineProperty", "deleteProperty", "setPrototypeOf"];

impl ParsedFile {
    /// Whether the file writes `name.member` (or may write any member of `name`).
    pub(super) fn js_ts_cjs_member_written<'a>(
        &'a self,
        name: &str,
        member: &str,
        cache: &mut JsBindingCache<'a>,
    ) -> bool {
        let writes = cache
            .member_writes
            .get_or_insert_with(|| self.js_ts_collect_member_writes());
        writes
            .get(name)
            .is_some_and(|w| w.any || w.named.contains(member))
    }

    fn js_ts_collect_member_writes(&self) -> BTreeMap<String, MemberWrites> {
        let mut out: BTreeMap<String, MemberWrites> = BTreeMap::new();
        let mut stack = vec![self.tree.root_node()];
        while let Some(node) = stack.pop() {
            let mut cursor = node.walk();
            stack.extend(node.children(&mut cursor));
            match node.kind() {
                "member_expression" | "subscript_expression" => {
                    let Some(base) = node
                        .child_by_field_name("object")
                        .and_then(strip_transparent_wrappers)
                        .filter(|b| b.kind() == "identifier")
                    else {
                        continue;
                    };
                    if !self.member_is_write_target(node) {
                        continue;
                    }
                    let entry = out.entry(self.node_text(&base).to_owned()).or_default();
                    let property = node.child_by_field_name("property");
                    match property.filter(|p| {
                        node.kind() == "member_expression" && p.kind() == "property_identifier"
                    }) {
                        Some(p) => {
                            entry.named.insert(self.node_text(&p).to_owned());
                        }
                        None => entry.any = true,
                    }
                }
                "call_expression" => {
                    if let Some(target) = self.js_ts_mutator_target(node) {
                        out.entry(self.node_text(&target).to_owned())
                            .or_default()
                            .any = true;
                    }
                }
                _ => {}
            }
        }
        out
    }

    /// The identifier first argument of `Object.assign(m, ..)` and its siblings.
    fn js_ts_mutator_target<'a>(&self, call: Node<'a>) -> Option<Node<'a>> {
        let callee = call.child_by_field_name("function")?;
        if callee.kind() != "member_expression" {
            return None;
        }
        let object = callee.child_by_field_name("object")?;
        let property = callee.child_by_field_name("property")?;
        let mutators = match (object.kind() == "identifier").then(|| self.node_text(&object))? {
            "Object" => OBJECT_MUTATORS,
            "Reflect" => REFLECT_MUTATORS,
            _ => return None,
        };
        if !mutators.contains(&self.node_text(&property)) {
            return None;
        }
        let arguments = call.child_by_field_name("arguments")?;
        let mut cursor = arguments.walk();
        let first = arguments
            .named_children(&mut cursor)
            .find(|n| n.kind() != "comment")?;
        let first = strip_transparent_wrappers(first)?;
        (first.kind() == "identifier").then_some(first)
    }
}
