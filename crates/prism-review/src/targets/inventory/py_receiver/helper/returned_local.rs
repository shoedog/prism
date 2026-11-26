use super::{Constructed, Resolver};
use crate::targets::inventory::{
    py_receiver::{census::Kind, lookup::direct, slots::Instance},
    python::{
        source::{name, ungroup},
        walk::ScopeId,
    },
    UnprovenCause,
};
use tree_sitter::Node;

impl<'tree> Resolver<'_, 'tree> {
    pub(super) fn returned_local(
        &self,
        returned: Node<'tree>,
        scope: ScopeId,
        method: &str,
    ) -> Option<Result<Constructed<'tree>, UnprovenCause>> {
        if returned.kind() != "identifier" {
            return None;
        }
        let key = name(self.parsed.node_text(&returned), self.walk.class(scope));
        let bindings = self.census.spaces[scope].bindings.get(&key)?;
        let body = self.walk.scopes[scope].body;
        let (binding, target, call, class) = bindings.iter().find_map(|binding| {
            let Kind::Assigned(Some(value)) = binding.kind else {
                return None;
            };
            if binding.scope != scope
                || !direct(body, binding.node)
                || binding.position > returned.start_byte()
            {
                return None;
            }
            let target = binding.node.child_by_field_name("left")?;
            let call = ungroup(value);
            Some((
                binding,
                target,
                call,
                self.constructor(call, scope, None, None)?,
            ))
        })?;
        Some(class.and_then(|class| {
            if self.instance_rebound(
                &Instance::Constructed(call.id()),
                method,
                returned,
                scope,
                None,
            ) {
                return Err(UnprovenCause::MemberRebound);
            }
            if bindings.len() != 1 || self.name_rebound(scope, &key, returned, scope) {
                return Err(UnprovenCause::ReceiverValueUnproven);
            }
            for (node, context) in &self.walk.nodes {
                if *node == target
                    || *node == returned
                    || node.kind() != "identifier"
                    || !context.evaluated
                    || !identifier_use(*node)
                    || name(self.parsed.node_text(node), self.walk.class(context.scope)) != key
                    || !self.lookup(context.scope, &key, *node).iter().any(|alt| {
                        alt.bindings
                            .iter()
                            .any(|candidate| std::ptr::eq(*candidate, binding))
                    })
                {
                    continue;
                }
                let expression = grouped(*node);
                let member_base = expression.parent().is_some_and(|parent| {
                    parent.kind() == "attribute"
                        && parent.child_by_field_name("object") == Some(expression)
                });
                if context.scope != scope
                    || !(member_base || self.literal_receiver_write(expression, scope))
                {
                    return Err(UnprovenCause::ReceiverValueUnproven);
                }
            }
            Ok(Constructed { class, call })
        }))
    }
}

fn grouped(mut node: Node<'_>) -> Node<'_> {
    while let Some(parent) = node
        .parent()
        .filter(|parent| parent.kind() == "parenthesized_expression")
    {
        node = parent;
    }
    node
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
