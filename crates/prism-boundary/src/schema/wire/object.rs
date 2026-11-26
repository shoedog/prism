use super::composition::{Effect, Stop};
use super::{constraints, Dialect, Edge, Node, Reader, Tree};
use crate::element::PointerToken;
use crate::inventory::Property;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

impl<'r, 'a> Reader<'r, 'a> {
    pub(super) fn object(&self, nodes: &[Node<'r, 'a>], edge: Edge, tree: &mut Tree) -> bool {
        let (minimum, maximum) = constraints::limits(nodes, "minProperties", "maxProperties");
        let mut required: BTreeSet<_> = nodes.iter().flat_map(constraints::required).collect();
        let mut names = required.clone();
        for node in nodes {
            if let Some(properties) = node.get("properties").and_then(Value::as_object) {
                names.extend(properties.keys().cloned());
            }
        }
        let mut possible = BTreeMap::new();
        let mut impossible_required = false;
        for name in names {
            let mut declarations = Vec::new();
            let mut values = Vec::new();
            for node in nodes {
                if let Some(value) = node.get("properties").and_then(|props| props.get(&name)) {
                    declarations.push(node);
                    values.push(node.child(value, &["properties", &name]));
                } else if let Some(remainder) = node.get("additionalProperties") {
                    let mut node = node.child(remainder, &["additionalProperties"]);
                    node.policy = true;
                    values.push(node);
                }
            }
            if self.dialect == Dialect::OpenApi30 && required.contains(&name) {
                let opposite = if self.request {
                    "readOnly"
                } else {
                    "writeOnly"
                };
                if self
                    .expand(values.clone())
                    .branches
                    .iter()
                    .flatten()
                    .any(|node| {
                        node.get(opposite) == Some(&Value::Bool(true))
                            && node.get(if self.request {
                                "writeOnly"
                            } else {
                                "readOnly"
                            }) != Some(&Value::Bool(true))
                    })
                {
                    required.remove(&name);
                }
            }
            let is_required = required.contains(&name);
            let mut origins = BTreeMap::<_, BTreeSet<_>>::new();
            for owner in declarations {
                for (group, member) in &owner.origins {
                    origins.entry(group).or_default().insert(member);
                }
            }
            let overlap = origins.values().any(|members| members.len() > 1);
            let child = self.visit(values, edge.child(true, is_required, overlap));
            if is_required && !child.feasible && !child.has_limits() {
                impossible_required = true;
            }
            if child.feasible || child.shape.is_some() || child.has_limits() {
                possible.insert(name, child);
            }
        }
        let required_count = required.len() as u64;
        if impossible_required || minimum > maximum || required_count > maximum {
            return false;
        }
        let mut map_nodes = Vec::new();
        let mut closed = false;
        for node in nodes {
            match node.get("additionalProperties") {
                Some(Value::Bool(false)) => closed = true,
                Some(value) if value.is_object() => {
                    map_nodes.push(node.child(value, &["additionalProperties"]))
                }
                _ => {}
            }
        }
        let mut map = None;
        if !closed && !map_nodes.is_empty() {
            let child = self.visit(map_nodes, edge.child(false, false, false));
            closed = !child.feasible && !child.has_limits();
            if !closed {
                map = Some(child);
            }
        }
        let established = possible.values().filter(|child| child.feasible).count() as u64;
        let could_exist = possible
            .values()
            .filter(|child| child.feasible || child.has_limits())
            .count() as u64;
        if closed && could_exist < minimum {
            return false;
        }
        let uncertain_required = required
            .iter()
            .any(|name| possible.get(name).is_some_and(|child| !child.feasible));
        let uncertain_minimum =
            established < minimum && (closed || map.as_ref().is_some_and(|child| !child.feasible));
        tree.uncertain_presence |= uncertain_required || uncertain_minimum;
        let spare = required_count < maximum;
        for (name, child) in possible {
            if spare || required.contains(&name) {
                tree.child(PointerToken::Property(name), child, edge.depth);
            }
        }
        if spare {
            if let Some(child) = map {
                tree.child(PointerToken::MapValue, child, edge.depth);
            }
        }
        !(uncertain_required || uncertain_minimum)
    }

    pub(super) fn array(
        &self,
        nodes: &[Node<'r, 'a>],
        edge: Edge,
        tree: &mut Tree,
    ) -> (bool, Property<String>) {
        let (minimum, maximum) = constraints::limits(nodes, "minItems", "maxItems");
        if minimum > maximum {
            return (false, Property::Missing);
        }
        if maximum == 0 {
            return (true, Property::Missing);
        }
        let items: Vec<_> = nodes
            .iter()
            .filter_map(|node| node.get("items").map(|value| node.child(value, &["items"])))
            .collect();
        if items.is_empty() {
            return (true, Property::Missing);
        }
        let child = self.visit(items, edge.child(false, false, false));
        if !child.feasible && !child.has_limits() {
            return (minimum == 0, Property::Missing);
        }
        let item_type = child
            .shape
            .as_ref()
            .map(|shape| shape.field_type.clone())
            .unwrap_or(Property::Unknown);
        let feasible = minimum == 0 || child.feasible;
        tree.child(PointerToken::ListItem, child, edge.depth);
        tree.uncertain_presence |= !feasible;
        (feasible, item_type)
    }
}
