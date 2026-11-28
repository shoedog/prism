mod objects;
mod strings;
pub(super) use strings::literal as string_literal;

use super::{
    lookup::args,
    slots::{Read, Slot},
    Resolver,
};
use crate::targets::inventory::python::{
    source::{children, has_comma, name, outer_group, scalar_subscript, ungroup, ungroup_target},
    walk::{Context, ScopeId},
};
use objects::{Objects, WriteAccess};
use tree_sitter::Node;

#[derive(PartialEq, Eq)]
pub(super) struct Write {
    pub slot: Slot,
    effect: Objects,
    read_point: Read,
    suite: Option<usize>,
}

impl Write {
    pub(super) fn reaches(&self, read: Read) -> bool {
        let write = self.read_point;
        // Module initialization is ordered. A deferred activation can see
        // effects of previous activations of its own body or another body.
        !(write.scope == 0 && read.scope == 0 && write.direct && write.byte > read.byte)
    }

    fn replaces(&self, read: Read) -> bool {
        self.read_point.scope == 0
            && self.read_point.direct
            && read.scope == 0
            && self.read_point.byte <= read.byte
    }

    pub(super) fn follows_in_suite(&self, read: Read, suite: Node<'_>) -> bool {
        self.read_point.scope == read.scope
            && self.read_point.byte > read.byte
            && self.suite == Some(suite.id())
    }
}

impl<'tree> Resolver<'_, 'tree> {
    pub(super) fn index_writes(&mut self) {
        let mut nodes: Vec<_> = self
            .walk
            .nodes
            .iter()
            .copied()
            .filter(|(node, context)| {
                context.evaluated
                    && matches!(
                        node.kind(),
                        "assignment"
                            | "augmented_assignment"
                            | "for_statement"
                            | "for_in_clause"
                            | "as_pattern"
                            | "delete_statement"
                            | "call"
                    )
            })
            .collect();
        nodes.sort_by_key(|(node, _)| node.end_byte());
        self.index_write_pass(&nodes, true);

        // Resolve live member hops against effects at their point of use.
        // A bounded repeat can see effects from deferred or repeated
        // execution whose syntax comes later in the file. If identities keep
        // changing, retain a key-specific veto instead of trusting the last pass.
        let names = self.writes.len();
        self.index_write_pass(&nodes, false);
        for _ in 0..3 {
            let previous = self.writes.len();
            self.index_write_pass(&nodes, false);
            let settled = self.writes[names..previous] == self.writes[previous..];
            if settled {
                self.writes.truncate(previous);
                return;
            }
            self.writes.drain(names..previous);
        }
        for write in &mut self.writes[names..] {
            if let Slot::Member {
                object, definite, ..
            } = &mut write.slot
            {
                *object = None;
                *definite = false;
            }
        }
    }

    fn index_write_pass(&mut self, nodes: &[(Node<'tree>, Context)], names_only: bool) {
        for &(node, context) in nodes {
            let scope = context.scope;
            let read_point = Read {
                byte: node.end_byte(),
                ..self.read(node, scope)
            };
            let (slots, value) = if node.kind() == "call" {
                if names_only {
                    continue;
                }
                let Some((object, key, value)) = self.reflection_write(node, scope) else {
                    continue;
                };
                let access = if value.is_some() {
                    WriteAccess::Attribute
                } else {
                    WriteAccess::DeleteAttribute
                };
                (self.objects(object, scope, true).slots(key, access), value)
            } else {
                let roots = write_targets(node);
                let value = match roots.as_slice() {
                    [target]
                        if node.kind() == "assignment"
                            && matches!(
                                ungroup_target(*target).kind(),
                                "attribute" | "subscript"
                            ) =>
                    {
                        node.child_by_field_name("right")
                    }
                    _ => None,
                };
                let access = if node.kind() == "delete_statement" {
                    WriteAccess::DeleteAttribute
                } else {
                    WriteAccess::Attribute
                };
                let slots = roots
                    .into_iter()
                    .flat_map(|target| self.targets(target, scope, names_only, access))
                    .collect();
                (slots, value)
            };
            if slots.is_empty() {
                continue;
            }
            let effect = if node.kind() == "delete_statement" {
                Objects::other()
            } else {
                value.map_or_else(Objects::unknown, |value| self.objects(value, scope, true))
            };
            let suite = std::iter::successors(node.parent(), Node::parent)
                .find(|parent| matches!(parent.kind(), "block" | "module"))
                .filter(|suite| super::lookup::direct(*suite, node))
                .map(|suite| suite.id());
            self.writes.extend(slots.into_iter().map(|slot| Write {
                slot,
                effect: effect.clone(),
                read_point,
                suite,
            }));
        }
    }

    fn targets(
        &self,
        target: Node<'tree>,
        scope: ScopeId,
        names_only: bool,
        access: WriteAccess,
    ) -> Vec<Slot> {
        match target.kind() {
            "attribute" if !names_only => {
                let Some(object) = target.child_by_field_name("object") else {
                    return Vec::new();
                };
                let key = target
                    .child_by_field_name("attribute")
                    .map(|attr| name(self.parsed.node_text(&attr), self.walk.class(scope)));
                self.objects(object, scope, true).slots(key, access)
            }
            "subscript" => self
                .subscript_write(target, scope, names_only)
                .unwrap_or_default(),
            _ => target_parts(target)
                .into_iter()
                .flat_map(|item| self.targets(item, scope, names_only, access))
                .collect(),
        }
    }

    fn subscript_write(
        &self,
        target: Node<'tree>,
        scope: ScopeId,
        names_only: bool,
    ) -> Option<Vec<Slot>> {
        let value = ungroup(target.child_by_field_name("value")?);
        let index = scalar_subscript(target)?;
        let key = strings::write_key(self.parsed, index)?;
        if self.reflection_helper(value, scope, "globals") && args(value).next().is_none() {
            return names_only.then(|| {
                vec![Slot::Name {
                    owner: 0,
                    name: key,
                }]
            });
        }
        if names_only {
            return None;
        }
        if let Some(object) = self.vars_target(value, target, scope) {
            return Some(
                self.objects(object, scope, true)
                    .slots(key, WriteAccess::Namespace),
            );
        }
        if value.kind() == "attribute" {
            let attribute = value.child_by_field_name("attribute")?;
            if name(self.parsed.node_text(&attribute), self.walk.class(scope)) == "__dict__" {
                let object = value.child_by_field_name("object")?;
                return Some(
                    self.objects(object, scope, true)
                        .slots(key, WriteAccess::Namespace),
                );
            }
        }
        self.implicit_builtins(value, scope).then(|| {
            vec![Slot::Member {
                object: Some("builtins".to_owned()),
                definite: key.is_some(),
                key,
            }]
        })
    }

    pub(super) fn literal_receiver_write(&self, receiver: Node<'tree>, scope: ScopeId) -> bool {
        let Some(call) = receiver.parent().and_then(|args| args.parent()) else {
            return false;
        };
        if call.kind() != "call" || args(call).next() != Some(receiver) {
            return false;
        }
        if self
            .reflection_write(call, scope)
            .is_some_and(|(object, _, _)| object == receiver)
        {
            return true;
        }
        let base = outer_group(call);
        let Some(target) = base.parent().filter(|parent| {
            parent.kind() == "subscript" && parent.child_by_field_name("value") == Some(base)
        }) else {
            return false;
        };
        is_write_target(target)
            && self.vars_target(call, target, scope) == Some(receiver)
            && (has_comma(target)
                || scalar_subscript(target).is_some_and(|index| {
                    ungroup(index).kind() == "tuple"
                        || strings::write_key(self.parsed, index).is_some()
                }))
    }

    fn vars_target(
        &self,
        call: Node<'tree>,
        target: Node<'tree>,
        scope: ScopeId,
    ) -> Option<Node<'tree>> {
        if call.kind() != "call" {
            return None;
        }
        let mut callee = ungroup(self.parsed.call_function_node(&call)?);
        // A starred unpacking target can put the star on the call's
        // identifier instead of wrapping the full subscript in the tree.
        if callee.kind() == "list_splat"
            && target.parent().is_some_and(|pattern| {
                matches!(pattern.kind(), "list_pattern" | "pattern_list")
                    || (pattern.kind() == "tuple_pattern" && has_comma(pattern))
            })
        {
            let [identifier] = children(callee).collect::<Vec<_>>().try_into().ok()?;
            callee = identifier;
        }
        let [object] = args(call).collect::<Vec<_>>().try_into().ok()?;
        self.reflection_name(callee, scope, "vars")
            .then_some(object)
    }

    fn reflection_write(
        &self,
        call: Node<'tree>,
        scope: ScopeId,
    ) -> Option<(Node<'tree>, Option<String>, Option<Node<'tree>>)> {
        let callee = ungroup(self.parsed.call_function_node(&call)?);
        let effect = name(self.parsed.node_text(&callee), self.walk.class(scope));
        if !matches!(effect.as_str(), "setattr" | "delattr")
            || !self.reflection_helper(call, scope, &effect)
        {
            return None;
        }
        let mut arguments = args(call);
        let object = arguments.next()?;
        let key = strings::write_key(self.parsed, arguments.next()?)?;
        let value = arguments.next();
        if (effect == "setattr") != value.is_some() || arguments.next().is_some() {
            return None;
        }
        Some((object, key, value))
    }

    fn reflection_helper(&self, call: Node<'tree>, scope: ScopeId, helper: &str) -> bool {
        if call.kind() != "call" {
            return false;
        }
        let Some(callee) = self.parsed.call_function_node(&call).map(ungroup) else {
            return false;
        };
        self.reflection_name(callee, scope, helper)
    }

    fn reflection_name(&self, callee: Node<'tree>, scope: ScopeId, helper: &str) -> bool {
        callee.kind() == "identifier"
            && name(self.parsed.node_text(&callee), self.walk.class(scope)) == helper
            && self.builtin_unshadowed(scope, helper, callee)
    }

    fn implicit_builtins(&self, value: Node<'tree>, scope: ScopeId) -> bool {
        value.kind() == "identifier"
            && name(self.parsed.node_text(&value), self.walk.class(scope)) == "__builtins__"
            && self
                .lookup(scope, "__builtins__", value)
                .iter()
                .all(|alt| alt.owner == 0)
    }
}

fn write_targets(node: Node<'_>) -> Vec<Node<'_>> {
    match node.kind() {
        "assignment"
            if node.child_by_field_name("type").is_some()
                && node.child_by_field_name("right").is_none() =>
        {
            Vec::new()
        }
        "assignment" | "augmented_assignment" | "for_statement" | "for_in_clause" => {
            node.child_by_field_name("left").into_iter().collect()
        }
        "as_pattern" => node.child_by_field_name("alias").into_iter().collect(),
        "delete_statement" => children(node).collect(),
        _ => Vec::new(),
    }
}

fn target_parts(node: Node<'_>) -> Vec<Node<'_>> {
    match node.kind() {
        "parenthesized_expression"
        | "expression_list"
        | "list"
        | "tuple"
        | "pattern_list"
        | "list_pattern"
        | "tuple_pattern"
        | "list_splat_pattern"
        | "list_splat"
        | "as_pattern_target" => children(node).collect(),
        _ => Vec::new(),
    }
}

fn is_write_target(mut node: Node<'_>) -> bool {
    while let Some(parent) = node.parent() {
        if !target_parts(parent).contains(&node) {
            return write_targets(parent).contains(&node);
        }
        node = parent;
    }
    false
}
