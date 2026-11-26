//! Closed local data graphs, tracked by allocation and binding.
use super::*;
use std::collections::BTreeMap;

#[derive(Default)]
struct Graph<'a> {
    references: BTreeSet<usize>,
    bindings: BTreeMap<usize, Node<'a>>,
    prototypes: BTreeSet<usize>,
}

impl<'a, F: FnMut(usize) -> bool> Proof<'a, '_, F> {
    pub(super) fn closed_slots(
        &mut self,
        object: Node<'a>,
    ) -> Option<BTreeMap<Slot<'a>, Node<'a>>> {
        if object.kind() != "object" {
            return None;
        }
        let mut slots = BTreeMap::new();
        for pair in children(object) {
            if !self.spend(1) {
                return None;
            }
            if pair.kind() == "shorthand_property_identifier" {
                if slots.insert(Slot::String(self.text(pair)), pair).is_some() {
                    return None;
                }
                continue;
            }
            if pair.kind() != "pair" {
                return None;
            }
            let key = field(pair, "key")?;
            let value = peel(field(pair, "value")?);
            let name = if key.kind() == "computed_property_name" {
                let reference = children(key).next()?;
                let symbol = self.initializer(reference)?;
                let (callee, "for") = self.method(symbol)? else {
                    return None;
                };
                let [name] = self.args(symbol)?;
                if !self.native(symbol, callee, &["Symbol", "for"]) {
                    return None;
                }
                Slot::Symbol(self.string(name)?)
            } else {
                let name = self.pair_name(pair)?;
                if name == "__proto__" {
                    if value.kind() != "null" {
                        return None;
                    }
                    continue;
                }
                Slot::String(name)
            };
            if slots.insert(name, value).is_some() {
                return None;
            }
        }
        Some(slots)
    }

    pub(in super::super) fn fresh_empty(&self, value: Node<'a>) -> bool {
        let value = peel(value);
        value.kind() == "object" && children(value).next().is_none()
    }

    pub(in super::super) fn terminal_value(&mut self, read: Node<'a>, owner: Node<'a>) -> bool {
        let mut value = read;
        while let Some(parent) = value.parent() {
            if !self.spend(1) {
                return false;
            }
            match parent.kind() {
                "return_statement" => return self.owner(parent) == Some(owner),
                "pair" if field(parent, "value") == Some(value) => {}
                "object" | "array" | "parenthesized_expression" => {}
                _ => return false,
            }
            value = parent;
        }
        false
    }

    pub(super) fn private_child(
        &mut self,
        value: Node<'a>,
        input: &GuardInput<'a>,
        table: Node<'a>,
    ) -> bool {
        let Some(child) = self.allocation(value, 0) else {
            return false;
        };
        let mut graph = Graph::default();
        if !self.walk_data(table, &mut graph, 0) || graph.prototypes.contains(&child.id()) {
            return false;
        }
        self.closed_bindings(&graph, input.owner)
    }

    pub(super) fn closed_clone(&mut self, input: Node<'a>, call: Node<'a>) -> bool {
        let Some(owner) = self.owner(call) else {
            return false;
        };
        let Some(_allocation) = self.allocation(input, 0) else {
            return false;
        };
        let mut graph = Graph::default();
        self.walk_data(input, &mut graph, 0) && self.closed_bindings(&graph, owner)
    }

    fn closed_bindings(&mut self, graph: &Graph<'a>, owner: Node<'a>) -> bool {
        graph.bindings.values().all(|reference| {
            self.only_references(*reference, |proof, read| {
                graph.references.contains(&read.id()) || proof.terminal_value(read, owner)
            })
        })
    }

    fn allocation(&mut self, value: Node<'a>, depth: usize) -> Option<Node<'a>> {
        if depth == 32 || !self.spend(1) {
            return None;
        }
        let value = peel(value);
        if identifier(value) {
            let origin = self.initializer(value)?;
            return self.allocation(origin, depth + 1);
        }
        if matches!(value.kind(), "object" | "array") || self.object_create(value).is_some() {
            return Some(value);
        }
        None
    }

    fn object_create(&mut self, call: Node<'a>) -> Option<Node<'a>> {
        let (namespace, "create") = self.method(call)? else {
            return None;
        };
        let [proto] = self.args(call)?;
        self.native(call, namespace, &["Object", "create"])
            .then_some(proto)
    }

    fn walk_data(&mut self, value: Node<'a>, graph: &mut Graph<'a>, depth: usize) -> bool {
        if depth == 32 || !self.spend(1) {
            return false;
        }
        let value = peel(value);
        if identifier(value) {
            let Some((binding, _)) = self.declaration(value) else {
                return false;
            };
            let Some(origin) = self.initializer(value) else {
                return false;
            };
            graph.references.insert(value.id());
            graph.bindings.insert(binding.start_byte, value);
            return self.walk_data(origin, graph, depth + 1);
        }
        match value.kind() {
            "null" | "string" | "number" | "true" | "false" => true,
            "object" => self.closed_slots(value).is_some_and(|slots| {
                slots
                    .into_values()
                    .all(|child| self.walk_data(child, graph, depth + 1))
            }),
            "array" => children(value).all(|item| self.walk_data(item, graph, depth + 1)),
            "call_expression" => {
                let Some(proto) = self.object_create(value) else {
                    return false;
                };
                if proto.kind() == "null" {
                    return true;
                }
                let Some(proto_alloc) = self.allocation(proto, 0) else {
                    return false;
                };
                graph.prototypes.insert(proto_alloc.id());
                self.walk_data(proto, graph, depth + 1)
            }
            _ => false,
        }
    }
}
