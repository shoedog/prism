//! Projection-only effects of evaluating the owner's own parameter list.

use super::pattern_execution as execution;
use super::pattern_formals::decoded_name;
use super::pattern_projection::member_key;
use crate::access_path::AccessPath;
use crate::ast::js_ts::{formal_binding, props_keys::strip_transparent_wrappers};
use crate::ast::{JsBindingCache, ParsedFile};
use tree_sitter::Node;

type Span = [usize; 2];

#[derive(Default)]
pub(super) struct Effects {
    sites: Vec<Effect>,
    bindings: Vec<Span>,
}

struct Effect {
    span: Span,
    at: usize,
    direct_store: Option<AccessPath>,
}

impl Effects {
    pub(super) fn collect(parsed: &ParsedFile, owner: Node<'_>) -> Self {
        let mut result = Self::default();
        let Some(parameters) = owner.child_by_field_name("parameters") else {
            return result;
        };
        let mut scope_cache = JsBindingCache::for_sites();
        let mut pending: Vec<_> = parameters.named_children(&mut parameters.walk()).collect();
        while let Some(node) = pending.pop() {
            match node.kind() {
                "array_pattern" | "rest_pattern" => {
                    result.bindings.push(span(node));
                    pending.extend(node.named_children(&mut node.walk()));
                }
                "object_pattern" => pending.extend(node.named_children(&mut node.walk())),
                "pair_pattern" => {
                    if let Some(binding) = node.child_by_field_name("value") {
                        result.add(node, binding.start_byte(), None);
                        pending.push(binding);
                    }
                }
                "shorthand_property_identifier_pattern" => {
                    result.add(node, node.start_byte(), None);
                }
                "object_assignment_pattern"
                | "assignment_pattern"
                | "required_parameter"
                | "optional_parameter" => {
                    let (binding, value) = if node.kind() == "object_assignment_pattern" {
                        if let Some(binding) = node.child_by_field_name("left") {
                            result.add(node, binding.start_byte(), None);
                        }
                        (
                            node.child_by_field_name("left"),
                            node.child_by_field_name("right"),
                        )
                    } else {
                        let Some((binding, value)) = formal_binding(node) else {
                            continue;
                        };
                        (Some(binding), value)
                    };
                    let Some(binding) = binding else {
                        continue;
                    };
                    if let Some(value) = value {
                        if !inert(parsed, value, &mut scope_cache) {
                            let direct = direct_store(parsed, owner, value, &mut scope_cache);
                            result.add(value, binding.start_byte(), direct);
                        }
                    }
                    // A shorthand leaf default has already recorded its preceding Get.
                    if node.kind() != "object_assignment_pattern" {
                        pending.push(binding);
                    }
                }
                _ => {}
            }
        }
        result
    }

    fn add(&mut self, node: Node<'_>, at: usize, direct_store: Option<AccessPath>) {
        self.sites.push(Effect {
            span: span(node),
            at,
            direct_store,
        });
    }

    pub(super) fn withholds(&self, path: &AccessPath, initialized: usize) -> bool {
        self.sites.iter().any(|effect| {
            effect.at >= initialized
                && effect.direct_store.as_ref().is_none_or(|store| {
                    store.base != path.base
                        || !store
                            .fields
                            .iter()
                            .zip(&path.fields)
                            .all(|(left, right)| left == right)
                })
        })
    }

    pub(super) fn has_binding_effect(&self) -> bool {
        !self.bindings.is_empty()
    }

    pub(super) fn sites(&self) -> impl Iterator<Item = Span> + '_ {
        self.sites
            .iter()
            .filter(|site| site.direct_store.is_none())
            .map(|site| site.span)
            .chain(self.bindings.iter().copied())
    }
}

fn span(node: Node<'_>) -> Span {
    [node.start_byte(), node.end_byte()]
}

/// Inspect evaluated children only. All operators, computed keys and spreads
/// leave this allowlist, including spreading a syntactically inert literal.
fn inert<'a>(parsed: &'a ParsedFile, value: Node<'a>, cache: &mut JsBindingCache<'a>) -> bool {
    let mut pending = vec![value];
    while let Some(value) = pending.pop() {
        let Some(node) = strip_transparent_wrappers(value) else {
            return false;
        };
        match node.kind() {
            "number"
            | "string"
            | "true"
            | "false"
            | "null"
            | "regex"
            | "comment"
            | "arrow_function"
            | "function_expression"
            | "generator_function" => {}
            "identifier" | "undefined" | "shorthand_property_identifier" => {
                if !decoded_name(parsed.node_text(&node)).is_some_and(|name| {
                    parsed.js_ts_declarative_signature_lookup(node, &name, cache)
                }) {
                    return false;
                }
            }
            "array" | "object" => pending.extend(node.named_children(&mut node.walk())),
            "pair" | "method_definition" => {
                let key =
                    node.child_by_field_name(if node.kind() == "pair" { "key" } else { "name" });
                if !key.is_some_and(|key| {
                    matches!(key.kind(), "property_identifier" | "number" | "string")
                }) {
                    return false;
                }
                if node.kind() == "pair" {
                    let Some(value) = node.child_by_field_name("value") else {
                        return false;
                    };
                    pending.push(value);
                }
            }
            _ => return false,
        }
    }
    true
}

/// Retain p24's directly tracked own-parameter member store. Its reaching
/// edge uses the existing own-params doubt label.
fn direct_store<'a>(
    parsed: &'a ParsedFile,
    owner: Node<'a>,
    value: Node<'a>,
    cache: &mut JsBindingCache<'a>,
) -> Option<AccessPath> {
    let value = strip_transparent_wrappers(value)?;
    if value.kind() != "assignment_expression" {
        return None;
    }
    let target = strip_transparent_wrappers(value.child_by_field_name("left")?)?;
    let root = strip_transparent_wrappers(target.child_by_field_name("object")?)?;
    if root.kind() != "identifier" || !inert(parsed, value.child_by_field_name("right")?, cache) {
        return None;
    }
    let base = decoded_name(parsed.node_text(&root))?.into_owned();
    if execution::binding_scope(parsed, owner, root, &base) != Some(owner.id()) {
        return None;
    }
    let key = member_key(parsed, target)?;
    Some(AccessPath::with_fields(base, vec![key]))
}
