mod objects;
mod strings;

use super::{
    lookup::args,
    slots::{Read, Slot},
    Resolver,
};
use crate::targets::inventory::python::{
    source::{children, name, ungroup},
    walk::{Context, ScopeId},
};
use objects::Objects;
use tree_sitter::Node;

#[derive(PartialEq, Eq)]
pub(super) struct Write {
    pub slot: Slot,
    effect: Objects,
    read_point: Read,
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
            let mut value = None;
            let slots = match node.kind() {
                "assignment"
                | "augmented_assignment"
                | "for_statement"
                | "for_in_clause"
                | "as_pattern" => {
                    let field = if node.kind() == "as_pattern" {
                        "alias"
                    } else {
                        "left"
                    };
                    let Some(target) = node.child_by_field_name(field) else {
                        continue;
                    };
                    if node.kind() == "assignment" {
                        let right = node.child_by_field_name("right");
                        if node.child_by_field_name("type").is_some() && right.is_none() {
                            continue;
                        }
                        if matches!(ungroup(target).kind(), "attribute" | "subscript") {
                            value = right;
                        }
                    }
                    self.targets(target, scope, names_only)
                }
                "delete_statement" => children(node)
                    .flat_map(|target| self.targets(target, scope, names_only))
                    .collect(),
                "call" if !names_only => {
                    let Some((slots, replacement)) = self.reflection_write(node, scope) else {
                        continue;
                    };
                    value = replacement;
                    slots
                }
                _ => continue,
            };
            if slots.is_empty() {
                continue;
            }
            let effect = if node.kind() == "delete_statement" {
                Objects::other()
            } else {
                value.map_or_else(Objects::unknown, |value| self.objects(value, scope, true))
            };
            self.writes.extend(slots.into_iter().map(|slot| Write {
                slot,
                effect: effect.clone(),
                read_point,
            }));
        }
    }

    fn targets(&self, target: Node<'tree>, scope: ScopeId, names_only: bool) -> Vec<Slot> {
        let target = ungroup(target);
        match target.kind() {
            "attribute" if !names_only => {
                let Some(object) = target.child_by_field_name("object") else {
                    return Vec::new();
                };
                let key = target
                    .child_by_field_name("attribute")
                    .map(|attr| name(self.parsed.node_text(&attr), self.walk.class(scope)));
                self.objects(object, scope, true).slots(key)
            }
            "subscript" => self
                .subscript_write(target, scope, names_only)
                .unwrap_or_default(),
            "expression_list" | "list" | "tuple" | "pattern_list" | "list_pattern"
            | "tuple_pattern" | "list_splat_pattern" | "list_splat" | "as_pattern_target" => {
                children(target)
                    .flat_map(|item| self.targets(item, scope, names_only))
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    fn subscript_write(
        &self,
        target: Node<'tree>,
        scope: ScopeId,
        names_only: bool,
    ) -> Option<Vec<Slot>> {
        let value = ungroup(target.child_by_field_name("value")?);
        let index = target.child_by_field_name("subscript")?;
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
        if self.reflection_helper(value, scope, "vars") {
            let [object] = args(value).collect::<Vec<_>>().try_into().ok()?;
            return Some(self.objects(object, scope, true).slots(key));
        }
        if value.kind() == "attribute" {
            let attribute = value.child_by_field_name("attribute")?;
            if name(self.parsed.node_text(&attribute), self.walk.class(scope)) == "__dict__" {
                let object = value.child_by_field_name("object")?;
                return Some(self.objects(object, scope, true).slots(key));
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

    fn reflection_write(
        &self,
        call: Node<'tree>,
        scope: ScopeId,
    ) -> Option<(Vec<Slot>, Option<Node<'tree>>)> {
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
        Some((self.objects(object, scope, true).slots(key), value))
    }

    fn reflection_helper(&self, call: Node<'tree>, scope: ScopeId, helper: &str) -> bool {
        if call.kind() != "call" {
            return false;
        }
        let Some(callee) = self.parsed.call_function_node(&call).map(ungroup) else {
            return false;
        };
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
