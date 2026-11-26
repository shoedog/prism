use super::{Constructed, Resolver};
use crate::targets::inventory::{
    py_receiver::{census::Kind, lookup::direct, slots::Instance},
    python::{
        source::{children, name, outer_group, ungroup, ungroup_target},
        walk::ScopeId,
    },
    Family, UnprovenCause,
};
use tree_sitter::Node;

impl<'tree> Resolver<'_, 'tree> {
    pub(super) fn returned_local(
        &self,
        returned: Node<'tree>,
        scope: ScopeId,
        method: Option<&str>,
    ) -> Option<(Option<Family>, Result<Constructed<'tree>, UnprovenCause>)> {
        if returned.kind() != "identifier" {
            return None;
        }
        let key = name(self.parsed.node_text(&returned), self.walk.class(scope));
        let bindings = self.census.spaces[scope].bindings.get(&key)?;
        let body = self.walk.scopes[scope].body;
        let constructions: Vec<_> = bindings
            .iter()
            .filter_map(|binding| {
                let Kind::Assigned(Some(value)) = binding.kind else {
                    return None;
                };
                if binding.scope != scope
                    || !direct(body, binding.node)
                    || binding.position > returned.start_byte()
                {
                    return None;
                }
                let target = ungroup_target(binding.node.child_by_field_name("left")?);
                let call = ungroup(value);
                let (family, class) = if let Some(selected) =
                    method.and_then(|method| self.method_constructor(call, scope, method, None))
                {
                    (Some(selected.0), selected.1)
                } else {
                    (None, self.constructor(call, scope, None)?)
                };
                Some((binding, target, call, family, class))
            })
            .collect();
        if constructions.is_empty() {
            return None;
        }
        let family = constructions
            .iter()
            .filter_map(|(_, _, _, family, _)| *family)
            .min();
        let refuse = |cause| Some((family, Err(cause)));
        let mut causes = Vec::new();
        for (_, _, call, _, class) in &constructions {
            if let Err(cause) = class {
                causes.push(*cause);
                continue;
            }
            if self.instance_rebound(
                &Instance::Constructed(call.id()),
                method,
                returned,
                scope,
                None,
            ) {
                causes.push(UnprovenCause::MemberRebound);
            }
        }
        if bindings.len() != 1 || self.name_rebound(scope, &key, returned, scope) {
            causes.push(UnprovenCause::ReceiverValueUnproven);
        }
        if let Some(cause) = causes.into_iter().min() {
            return refuse(cause);
        }

        let (binding, target, call, _, class) = constructions.into_iter().next()?;
        let uses_binding = |node: Node<'tree>, scope| {
            name(self.parsed.node_text(&node), self.walk.class(scope)) == key
                && self.lookup(scope, &key, node).iter().any(|alt| {
                    alt.bindings
                        .iter()
                        .any(|candidate| std::ptr::eq(*candidate, binding))
                })
        };
        for &(node, context) in &self.walk.nodes {
            if !context.evaluated || node == target || node == returned {
                continue;
            }
            if node.kind() == "case_pattern"
                && bare_class_heads(node, &|head| uses_binding(head, context.scope))
            {
                return refuse(UnprovenCause::ReceiverValueUnproven);
            }
            if node.kind() != "identifier"
                || !identifier_use(node)
                || !uses_binding(node, context.scope)
            {
                continue;
            }
            let expression = outer_group(node);
            let member_base = expression.parent().is_some_and(|parent| {
                parent.kind() == "attribute"
                    && parent.child_by_field_name("object") == Some(expression)
            });
            if context.scope != scope
                || !(member_base || self.literal_receiver_write(expression, scope))
            {
                return refuse(UnprovenCause::ReceiverValueUnproven);
            }
        }
        Some((family, class.map(|class| Constructed { class, call })))
    }
}

// The call walk stops at case patterns. Only a bare class head consumes the
// local itself. A dotted head reads a member, and a keyword label binds nothing.
fn bare_class_heads<'tree>(node: Node<'tree>, matches: &impl Fn(Node<'tree>) -> bool) -> bool {
    if node.kind() == "class_pattern" {
        let mut path = children(node).next().into_iter().flat_map(children);
        if let (Some(head), None) = (path.next(), path.next()) {
            if matches(head) {
                return true;
            }
        }
    }
    children(node).any(|child| bare_class_heads(child, matches))
}

fn identifier_use(node: Node<'_>) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    match parent.kind() {
        "attribute" => parent.child_by_field_name("object") == Some(node),
        "parameters" | "lambda_parameters" | "list_splat_pattern" | "dictionary_splat_pattern" => {
            false
        }
        "typed_parameter" => parent.child_by_field_name("type") == Some(node),
        "typed_default_parameter"
        | "default_parameter"
        | "keyword_argument"
        | "function_definition"
        | "class_definition" => parent.child_by_field_name("name") != Some(node),
        _ => true,
    }
}
