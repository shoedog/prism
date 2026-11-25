mod strings;

use super::{
    census::Kind,
    lookup::{args, Alternative},
    slots::{Read, Slot},
    Resolver,
};
use crate::targets::inventory::python::{
    source::{chain, children, name, ungroup},
    walk::ScopeId,
};
use tree_sitter::Node;

pub(super) struct Write {
    pub slot: Slot,
    read_point: Read,
}

impl Write {
    pub(super) fn reaches(&self, read: Read) -> bool {
        let write = self.read_point;
        // A direct module initialization read precedes a later direct module
        // write. A deferred activation may instead see the persistent slot
        // after a previous activation has written it.
        !(write.scope == 0
            && read.scope == 0
            && write.direct
            && read.direct
            && write.byte > read.byte)
    }
}

impl<'tree> Resolver<'_, 'tree> {
    pub(super) fn index_writes(&mut self) {
        // Reflective name writes decide whether other reflection helpers
        // still denote builtins. Populate name slots before member slots.
        for names_only in [true, false] {
            let mut writes = Vec::new();
            for &(node, context) in &self.walk.nodes {
                if !context.evaluated {
                    continue;
                }
                let scope = context.scope;
                let read_point = Read {
                    byte: node.end_byte(),
                    ..self.read(node, scope)
                };
                let slots: Vec<Slot> = match node.kind() {
                    "assignment"
                    | "augmented_assignment"
                    | "for_statement"
                    | "for_in_clause"
                    | "as_pattern" => {
                        let name = if node.kind() == "as_pattern" {
                            "alias"
                        } else {
                            "left"
                        };
                        if node.child_by_field_name("type").is_some()
                            && node.child_by_field_name("right").is_none()
                        {
                            continue;
                        }
                        node.child_by_field_name(name)
                            .into_iter()
                            .flat_map(|target| self.targets(target, scope))
                            .collect()
                    }
                    "delete_statement" => children(node)
                        .flat_map(|target| self.targets(target, scope))
                        .collect(),
                    "call" if !names_only => {
                        self.reflection_write(node, scope).into_iter().collect()
                    }
                    _ => continue,
                };
                writes.extend(
                    slots
                        .into_iter()
                        .filter(|slot| matches!(slot, Slot::Name { .. }) == names_only)
                        .map(|slot| Write { slot, read_point }),
                );
            }
            self.writes.extend(writes);
        }
    }

    fn targets(&self, target: Node<'tree>, scope: ScopeId) -> Vec<Slot> {
        let target = ungroup(target);
        match target.kind() {
            "attribute" => self.attribute_write(target, scope).into_iter().collect(),
            "subscript" => self.subscript_write(target, scope).into_iter().collect(),
            "expression_list" | "list" | "tuple" | "pattern_list" | "list_pattern"
            | "tuple_pattern" | "list_splat_pattern" | "list_splat" | "as_pattern_target" => {
                children(target)
                    .flat_map(|item| self.targets(item, scope))
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    fn attribute_write(&self, target: Node<'tree>, scope: ScopeId) -> Option<Slot> {
        let value = target.child_by_field_name("object")?;
        let attribute = target.child_by_field_name("attribute")?;
        let key = name(self.parsed.node_text(&attribute), self.walk.class(scope));
        Some(Slot::Member {
            object: self.object(value, scope, true)?,
            key,
        })
    }

    fn subscript_write(&self, target: Node<'tree>, scope: ScopeId) -> Option<Slot> {
        let value = ungroup(target.child_by_field_name("value")?);
        let index = target.child_by_field_name("subscript")?;
        let key = strings::literal(self.parsed, index)?;
        if value.kind() == "call" {
            if self.reflection_helper(value, scope, "globals") && args(value).next().is_none() {
                return Some(Slot::Name {
                    owner: 0,
                    name: key,
                });
            }
            if self.reflection_helper(value, scope, "vars") {
                let [object] = args(value).collect::<Vec<_>>().try_into().ok()?;
                return Some(Slot::Member {
                    object: self.object(object, scope, true)?,
                    key,
                });
            }
        }
        if value.kind() == "attribute" {
            let attribute = value.child_by_field_name("attribute")?;
            if name(self.parsed.node_text(&attribute), self.walk.class(scope)) == "__dict__" {
                let object = value.child_by_field_name("object")?;
                return Some(Slot::Member {
                    object: self.object(object, scope, true)?,
                    key,
                });
            }
        }
        self.implicit_builtins(value, scope).then(|| Slot::Member {
            object: "builtins".to_owned(),
            key,
        })
    }

    fn reflection_write(&self, call: Node<'tree>, scope: ScopeId) -> Option<Slot> {
        let callee = ungroup(self.parsed.call_function_node(&call)?);
        let effect = name(self.parsed.node_text(&callee), self.walk.class(scope));
        let effect = effect.as_str();
        let mut arguments = args(call);
        let object = arguments.next()?;
        let key = strings::literal(self.parsed, arguments.next()?)?;
        let count = arguments.count();
        if !matches!((effect, count), ("setattr", 1) | ("delattr", 0))
            || !self.reflection_helper(call, scope, effect)
        {
            return None;
        }
        Some(Slot::Member {
            object: self.object(object, scope, true)?,
            key,
        })
    }

    fn reflection_helper(&self, call: Node<'tree>, scope: ScopeId, helper: &str) -> bool {
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

    fn object(&self, node: Node<'tree>, scope: ScopeId, alias: bool) -> Option<String> {
        let node = ungroup(node);
        if self.implicit_builtins(node, scope) {
            return Some("builtins".to_owned());
        }
        match node.kind() {
            "attribute" => {
                let object = node.child_by_field_name("object")?;
                let member = node.child_by_field_name("attribute")?;
                let mut path = self.object(object, scope, alias)?;
                path.push('.');
                path.push_str(&name(
                    self.parsed.node_text(&member),
                    self.walk.class(scope),
                ));
                Some(path)
            }
            "identifier" => {
                let names = chain(self.parsed, node, self.walk.class(scope))?;
                self.lookup_object(self.lookup(scope, &names[0], node), node, scope, alias)
            }
            "subscript" => {
                let value = ungroup(node.child_by_field_name("value")?);
                let key = strings::literal(self.parsed, node.child_by_field_name("subscript")?)?;
                if value.kind() == "call"
                    && self.reflection_helper(value, scope, "globals")
                    && args(value).next().is_none()
                {
                    return self.lookup_object(
                        vec![self.module_slot(&key, node, scope)],
                        node,
                        scope,
                        alias,
                    );
                }
                self.implicit_builtins(value, scope)
                    .then(|| format!("builtins.{key}"))
            }
            _ => None,
        }
    }

    fn module_slot(&self, key: &str, at: Node<'tree>, scope: ScopeId) -> Alternative<'_, 'tree> {
        let mut alternative = self.lookup(0, key, at).pop().expect("module lookup");
        // A deferred scope looks into a persistent namespace.
        if scope == 0 {
            alternative
                .bindings
                .retain(|binding| binding.scope != 0 || binding.position <= at.start_byte());
        }
        alternative
    }

    fn lookup_object(
        &self,
        alternatives: Vec<Alternative<'_, 'tree>>,
        node: Node<'tree>,
        scope: ScopeId,
        alias: bool,
    ) -> Option<String> {
        let mut canonical = None;
        for alt in alternatives {
            if !self.coalesces(&alt) {
                return None;
            }
            let binding = alt
                .bindings
                .iter()
                .copied()
                .find(|binding| self.available(binding, node, scope))?;
            let path = match &binding.kind {
                Kind::Import(import)
                    if !import.relative && !self.repository_local(&import.path) =>
                {
                    import.path.clone()
                }
                Kind::Assigned(Some(value)) if alias && ungroup(*value).kind() == "identifier" => {
                    self.object(*value, binding.scope, false)?
                }
                _ => return None,
            };
            if canonical.as_ref().is_some_and(|existing| existing != &path) {
                return None;
            }
            canonical = Some(path);
        }
        canonical
    }
}
