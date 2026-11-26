use super::super::references::Located;
use super::constraints::{self, ALL};
use super::{definition, resource, Dialect, Node, Reader};
use crate::inventory::DefinitionId;
use serde_json::Value;
use std::collections::BTreeSet;

const ALTERNATIVES: usize = 128;
const EXPANSION_STEPS: usize = 128;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Effect {
    Notice,
    Children,
    Field,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Stop(pub &'static str, pub Effect);

type Branch<'r, 'a> = Vec<Node<'r, 'a>>;

pub(super) struct Expansion<'r, 'a> {
    pub branches: Vec<Branch<'r, 'a>>,
    pub stops: BTreeSet<Stop>,
    pub warnings: BTreeSet<&'static str>,
    pub definitions: BTreeSet<DefinitionId>,
}

impl<'r, 'a> Expansion<'r, 'a> {
    fn single(nodes: Branch<'r, 'a>) -> Self {
        Self {
            branches: vec![nodes],
            stops: BTreeSet::new(),
            warnings: BTreeSet::new(),
            definitions: BTreeSet::new(),
        }
    }

    fn stopped(reason: &'static str, effect: Effect) -> Self {
        let mut result = Self::single(Vec::new());
        result.stops.insert(Stop(reason, effect));
        result
    }

    fn absorb(&mut self, other: &mut Self) {
        self.stops.append(&mut other.stops);
        self.warnings.append(&mut other.warnings);
        self.definitions.append(&mut other.definitions);
    }

    fn conjoin(&mut self, mut other: Self) {
        self.absorb(&mut other);
        if self.branches.len().saturating_mul(other.branches.len()) > ALTERNATIVES {
            self.stops.insert(Stop("composition_budget", Effect::Field));
            self.branches.clear();
            return;
        }
        self.branches = self
            .branches
            .iter()
            .flat_map(|left| {
                other
                    .branches
                    .iter()
                    .map(move |right| left.iter().chain(right).cloned().collect())
            })
            .collect();
    }
}

impl<'r, 'a> Reader<'r, 'a> {
    pub(super) fn expand(&self, nodes: Vec<Node<'r, 'a>>) -> Expansion<'r, 'a> {
        let mut output = Expansion::single(Vec::new());
        for node in nodes {
            output.conjoin(self.expand_node(node, 0));
        }
        output
    }

    fn expand_node(&self, mut node: Node<'r, 'a>, steps: usize) -> Expansion<'r, 'a> {
        if steps == EXPANSION_STEPS {
            return Expansion::stopped("composition_budget", Effect::Field);
        }
        if super::keywords::dialect_unchecked(&node) {
            return Expansion::single(vec![node]);
        }
        if let Some(reference) = node.get("$ref") {
            let mut result = self.expand_reference(&node, reference, steps);
            if self.dialect != Dialect::OpenApi30
                && node
                    .value
                    .as_object()
                    .is_some_and(|object| object.len() > 1)
            {
                node.ref_ignored = true;
                let siblings = self.expand_node(node, steps + 1);
                result.conjoin(siblings);
                if self.dialect == Dialect::Swagger2 {
                    result
                        .stops
                        .insert(Stop("ref_siblings_unchecked", Effect::Field));
                }
            }
            return result;
        }
        let mut result = Expansion::single(vec![node.clone()]);
        for keyword in ["allOf", "anyOf", "oneOf"] {
            let Some(value) = node.get(keyword) else {
                continue;
            };
            if self.dialect == Dialect::Swagger2 && keyword != "allOf" {
                continue;
            }
            let Some(items) = value.as_array().filter(|items| !items.is_empty()) else {
                result
                    .stops
                    .insert(Stop("composition_unreadable", Effect::Field));
                continue;
            };
            let mut alternatives = Expansion::single(Vec::new());
            alternatives.branches.clear();
            let mut arms = Vec::new();
            for (i, value) in items.iter().enumerate() {
                let mut member = node.child(value, &[keyword, &i.to_string()]);
                if keyword == "allOf" {
                    member
                        .origins
                        .push((node.value as *const Value as usize, i));
                }
                if !(value.is_object() || self.dialect == Dialect::OpenApi31 && value.is_boolean())
                {
                    result
                        .stops
                        .insert(Stop("composition_unreadable", Effect::Field));
                    continue;
                }
                let mut expansion = self.expand_node(member, steps + 1);
                if keyword == "allOf" {
                    result.conjoin(expansion);
                } else {
                    arms.push(expansion.branches.clone());
                    alternatives.branches.append(&mut expansion.branches);
                    alternatives.absorb(&mut expansion);
                    if alternatives.branches.len() > ALTERNATIVES {
                        alternatives.branches.clear();
                        alternatives
                            .stops
                            .insert(Stop("composition_budget", Effect::Field));
                        break;
                    }
                }
            }
            if keyword == "allOf" {
                continue;
            }
            if alternatives.stops.is_empty() {
                result.warnings.insert(if keyword == "oneOf" {
                    "oneOf_union"
                } else {
                    "anyOf_union"
                });
                if keyword == "oneOf" && !self.disjoint(&arms, &result.branches) {
                    let nullable = arms
                        .iter()
                        .flatten()
                        .flatten()
                        .any(|node| node.get("nullable") == Some(&Value::Bool(true)));
                    alternatives.stops.insert(Stop(
                        if nullable {
                            "nullable_composition_unchecked"
                        } else {
                            "unsupported_schema_keyword"
                        },
                        Effect::Field,
                    ));
                }
            } else {
                alternatives.stops = alternatives
                    .stops
                    .into_iter()
                    .map(|Stop(reason, _)| Stop(reason, Effect::Field))
                    .collect();
            }
            if !alternatives.branches.is_empty() {
                result.conjoin(alternatives);
            } else {
                result.absorb(&mut alternatives);
            }
        }
        result
    }

    fn expand_reference(
        &self,
        node: &Node<'r, 'a>,
        reference: &Value,
        steps: usize,
    ) -> Expansion<'r, 'a> {
        if node.resource_scope_unchecked()
            || self.dialect != Dialect::OpenApi30 && super::keywords::reference_scope(node)
            || reference
                .as_str()
                .and_then(|s| s.split_once('#'))
                .is_some_and(|(_, fragment)| {
                    !fragment.is_empty()
                        && !fragment.starts_with('/')
                        && !fragment.to_ascii_lowercase().starts_with("%2f")
                })
        {
            return Expansion::stopped("reference_scope", Effect::Children);
        }
        let target = match self.documents.reference_target(node.document, reference) {
            Ok(target) if target.value.is_object() || target.value.is_boolean() => target,
            Ok(_) => return Expansion::stopped("reference_unresolved", Effect::Children),
            Err(reason) => {
                return Expansion::stopped(super::reference_reason(reason), Effect::Children)
            }
        };
        let key = (resource(target.document), target.site.pointer.clone());
        if node.active.contains(&key) {
            return Expansion::stopped("recursive", Effect::Children);
        }
        let reached = definition(target.document, &target.site.pointer);
        let mut next = Node::new(target);
        next.active = node.active.clone();
        next.active.push(key);
        next.origins = node.origins.clone();
        next.definitions = node.definitions.clone();
        if let Some(definition) = &reached {
            next.definitions.push(definition.clone());
        }
        let mut result = self.expand_node(next, steps + 1);
        result.definitions.extend(reached);
        result
    }

    fn disjoint(&self, arms: &[Vec<Branch<'r, 'a>>], outer: &[Branch<'r, 'a>]) -> bool {
        arms.iter().enumerate().all(|(i, left)| {
            arms[i + 1..].iter().all(|right| {
                outer.iter().all(|shared| {
                    left.iter().all(|a| {
                        right.iter().all(|b| {
                            let left: Vec<_> = shared.iter().chain(a).cloned().collect();
                            let right: Vec<_> = shared.iter().chain(b).cloned().collect();
                            self.disjoint_pair(&left, &right)
                        })
                    })
                })
            })
        })
    }

    fn disjoint_pair(&self, left: &[Node<'r, 'a>], right: &[Node<'r, 'a>]) -> bool {
        // Only an explicit type can rule out a JSON kind. An inferred object
        // shape can supply fields without asserting that other kinds fail.
        let types = |nodes: &[Node<'r, 'a>]| {
            let explicit: Vec<_> = nodes
                .iter()
                .filter(|node| node.get("type").is_some() || node.value.is_boolean())
                .cloned()
                .collect();
            if explicit.is_empty() {
                ALL
            } else {
                constraints::types(&explicit, self.dialect).0
            }
        };
        let shared = types(left) & types(right);
        if shared == 0
            || self.required_false(left, types(left))
            || self.required_false(right, types(right))
        {
            return true;
        }
        if let (Some(a), Some(b)) = (constraints::domain(left), constraints::domain(right)) {
            if a.keys().all(|key| !b.contains_key(key)) {
                return true;
            }
        }
        if shared != constraints::OBJECT {
            return false;
        }
        let required = |nodes: &[Node<'r, 'a>]| {
            nodes
                .iter()
                .flat_map(constraints::required)
                .collect::<BTreeSet<_>>()
        };
        required(left).intersection(&required(right)).any(|name| {
            let values = |nodes: &[Node<'r, 'a>]| -> Vec<Node<'r, 'a>> {
                nodes
                    .iter()
                    .filter_map(|node| {
                        let property = node.get("properties")?.get(name)?;
                        Some(node.child(property, &["properties", name]))
                    })
                    .collect()
            };
            let a = values(left);
            let b = values(right);
            let opposite = if self.request {
                "readOnly"
            } else {
                "writeOnly"
            };
            if a.iter()
                .chain(&b)
                .any(|node| node.get(opposite) == Some(&Value::Bool(true)))
            {
                return false;
            }
            let (Some(a), Some(b)) = (constraints::domain(&a), constraints::domain(&b)) else {
                return false;
            };
            a.keys().all(|key| !b.contains_key(key))
        })
    }

    fn required_false(&self, nodes: &[Node<'r, 'a>], types: u8) -> bool {
        self.dialect == Dialect::OpenApi31
            && types == constraints::OBJECT
            && nodes.iter().flat_map(constraints::required).any(|name| {
                nodes.iter().any(|node| {
                    node.get("properties")
                        .and_then(|properties| properties.get(&name))
                        == Some(&Value::Bool(false))
                })
            })
    }
}

pub(super) fn target_node<'r, 'a>(location: Located<'r, 'a>) -> Node<'r, 'a> {
    let key = (resource(location.document), location.site.pointer.clone());
    let mut node = Node::new(location);
    node.active.push(key);
    node
}
