//! Assignments, member references and abstract receiver identities.
use super::evaluate::optional_chain;
use super::*;

#[derive(Clone, Copy)]
enum MemberUse {
    Value,
    Target { load: bool },
    TargetReceiver,
    Prefix,
}

impl MemberUse {
    fn receiver(self) -> Self {
        match self {
            Self::Value | Self::Prefix => Self::Prefix,
            Self::Target { .. } | Self::TargetReceiver => Self::TargetReceiver,
        }
    }
}

impl<'a> Analysis<'a> {
    pub(super) fn write(
        &mut self,
        target: Node<'a>,
        origin: Origin<'a>,
        context: Context<'a>,
        state: &mut State,
    ) {
        let target = peel(target);
        if identifier(target) {
            let Some(binding) = self
                .lexical
                .binding(context.scope, self.file.node_text(&target))
            else {
                return;
            };
            self.origins.insert(target.start_byte(), origin);
            let site = AstSpan::from(target);
            state.install(binding.start_byte, site);
            let source = match origin {
                Origin::Value(source) | Origin::Child(source, _) | Origin::Element(source) => {
                    Some(source)
                }
                Origin::Unknown => None,
            };
            if let Some(source) = source {
                let objects = self.receiver_objects(source, context);
                if !objects.is_empty() {
                    if let Some(value) = state.values.get_mut(&binding.start_byte) {
                        value.objects = objects;
                    }
                }
            }
        } else if matches!(target.kind(), "member_expression" | "subscript_expression") {
            let objects = self.target_reads(target, context, state, false);
            self.mutate(objects, target.into(), state);
        } else {
            self.target_reads(target, context, state, false);
            let parts = if target.kind() == "array_pattern" {
                let mut position = 0;
                let mut cursor = target.walk();
                target
                    .children(&mut cursor)
                    .filter_map(|child| {
                        if child.kind() == "," {
                            position += 1;
                        }
                        (child.is_named() && !child.is_extra()).then_some((position, child))
                    })
                    .collect::<Vec<_>>()
            } else {
                pattern_parts(target).into_iter().enumerate().collect()
            };
            if !self.charge(context.owner, parts.len()) {
                self.inventory(target, context, state);
                return;
            }
            let mut positions = match origin {
                Origin::Value(value)
                    if target.kind() == "array_pattern" && peel(value).kind() == "array" =>
                {
                    Some(super::super::array_positions(peel(value)).peekable())
                }
                _ => None,
            };
            for (index, part) in parts {
                if identifier(part)
                    && matches!(
                        target.kind(),
                        "assignment_pattern"
                            | "object_assignment_pattern"
                            | "required_parameter"
                            | "optional_parameter"
                    )
                {
                    if let Some(default) = field(target, "right").or_else(|| field(target, "value"))
                    {
                        self.defaults.insert(part.start_byte(), default);
                    }
                }
                let source = match (target.kind(), origin) {
                    ("array_pattern", _) if positions.is_some() => {
                        let elements = positions.as_mut().unwrap();
                        while elements
                            .peek()
                            .is_some_and(|(position, _)| *position < index)
                        {
                            elements.next();
                        }
                        elements
                            .next_if(|(position, _)| *position == index)
                            .map_or(Origin::Unknown, |(_, value)| Origin::Value(value))
                    }
                    ("pair_pattern", Origin::Value(value)) => field(target, "key")
                        .filter(|key| key.kind() == "computed_property_name")
                        .and_then(|key| children(key).next())
                        .map_or(Origin::Element(value), |key| Origin::Child(value, key)),
                    (
                        "assignment_pattern"
                        | "object_assignment_pattern"
                        | "required_parameter"
                        | "optional_parameter"
                        | "formal_parameters",
                        _,
                    ) => origin,
                    (_, Origin::Value(value)) => Origin::Element(value),
                    _ => origin,
                };
                let source = if target.kind() == "object_pattern" && part.kind() == "pair_pattern" {
                    origin
                } else {
                    source
                };
                self.write(part, source, context, state);
            }
        }
    }

    pub(super) fn mutate(&self, objects: BTreeSet<usize>, site: AstSpan, state: &mut State) {
        if !state.dead {
            for object in objects {
                state
                    .mutated_objects
                    .entry(object)
                    .or_default()
                    .insert(site.clone());
            }
        }
    }

    pub(super) fn target_reads(
        &mut self,
        node: Node<'a>,
        context: Context<'a>,
        state: &mut State,
        load: bool,
    ) -> BTreeSet<usize> {
        let node = peel(node);
        match node.kind() {
            "subscript_expression" | "member_expression" => {
                return self
                    .member_value(node, context, state, MemberUse::Target { load })
                    .map_or_else(BTreeSet::new, |version| version.objects);
            }
            "assignment_pattern"
            | "object_assignment_pattern"
            | "required_parameter"
            | "optional_parameter" => {
                if let Some(right) = field(node, "right").or_else(|| field(node, "value")) {
                    if let Some(mut default) = self.fork(context, state) {
                        self.visit(right, context, &mut default);
                        self.join(context, state, &default, true);
                    } else {
                        self.inventory(right, context, state);
                    }
                }
            }
            "pair_pattern" => {
                if let Some(key) =
                    field(node, "key").filter(|key| key.kind() == "computed_property_name")
                {
                    self.visit(key, context, state);
                }
            }
            _ if load && identifier(node) => self.visit(node, context, state),
            _ => {}
        }
        BTreeSet::new()
    }

    pub(super) fn member(
        &mut self,
        node: Node<'a>,
        context: Context<'a>,
        state: &mut State,
    ) -> Option<Version> {
        self.member_value(node, context, state, MemberUse::Value)
    }

    fn member_value(
        &mut self,
        node: Node<'a>,
        context: Context<'a>,
        state: &mut State,
        member_use: MemberUse,
    ) -> Option<Version> {
        let visible = !matches!(
            member_use,
            MemberUse::Target { load: false } | MemberUse::Prefix
        );
        let inventory_receiver = matches!(member_use, MemberUse::TargetReceiver);
        let Some(object) = field(node, "object") else {
            return None;
        };
        let optional = optional_chain(node);
        let canonical = super::super::path(self.file, node);
        let mut version = canonical
            .as_ref()
            .map(|(base, _)| self.version(base, context, state));
        let object = peel(object);
        match object.kind() {
            "member_expression" | "subscript_expression" => {
                let nested_use = if matches!(member_use, MemberUse::Value) && canonical.is_none() {
                    MemberUse::Value
                } else {
                    member_use.receiver()
                };
                version = self.member_value(object, context, state, nested_use);
            }
            _ if identifier(object) && canonical.is_some() => {
                let base = self.file.node_text(&object).to_string();
                let mut v = version.clone().unwrap_or_default();
                v.mutated = false;
                let receiver_mutated = self.has_member_effect(&base, &v, context, state);
                let receiver_visible = matches!(member_use, MemberUse::Target { .. });
                self.read_path(object, base.clone(), base, context, v, receiver_visible);
                if let Some(site) = self.sites.get_mut(&AstSpan::from(object)) {
                    site.receiver_mutated |= receiver_mutated;
                }
            }
            _ => {
                self.visit(object, context, state);
                if version.is_none() {
                    version = Some(Version {
                        objects: self.receiver_objects(object, context),
                        ..Version::default()
                    });
                }
            }
        }
        if let Some(key) = field(node, "index") {
            let bypass = if optional {
                let Some(bypass) = self.fork(context, state) else {
                    self.inventory(key, context, state);
                    return version;
                };
                Some(bypass)
            } else {
                None
            };
            self.visit(key, context, state);
            if let Some(version) = &mut version {
                // Element equality additionally depends on the evaluated subscript.
                let key = peel(key);
                if identifier(key) {
                    let mut key = self.version(self.file.node_text(&key), context, state);
                    version.incomplete |= key.sites.is_empty();
                    key.mutated = false;
                    key.objects.clear();
                    version.merge(&key);
                } else if super::super::literal_index(self.file, key).is_none() {
                    version.incomplete = true;
                }
            }
            if let Some(bypass) = bypass {
                self.join(context, state, &bypass, true);
            }
        }
        if let (Some((base, path)), Some(version)) = (canonical, &mut version) {
            version.incomplete |= optional;
            let widen_member =
                inventory_receiver && self.has_member_effect(&base, version, context, state);
            if widen_member {
                version.mutated = true;
            } else if visible {
                let capacity = OWNER_WORK_BUDGET.saturating_sub(
                    self.spending
                        .get(&context.owner.id())
                        .copied()
                        .unwrap_or_default(),
                );
                let mut writes = BTreeSet::new();
                let mut work = 0;
                for sites in version
                    .objects
                    .iter()
                    .filter_map(|object| state.mutated_objects.get(object))
                    .chain(
                        self.lexical
                            .binding(context.scope, &base)
                            .and_then(|binding| {
                                self.effects.captured_members.get(&binding.start_byte)
                            })
                            .map(AsRef::as_ref),
                    )
                {
                    let available = capacity.saturating_sub(work);
                    writes.extend(sites.iter().take(available.saturating_add(1)).cloned());
                    work += sites.len() + 1;
                    if work > capacity {
                        break;
                    }
                }
                if self.charge(context.owner, work) {
                    version.mutated |= !writes.is_empty();
                    version.sites.extend(writes);
                } else {
                    version.incomplete = true;
                }
            }
            self.read_path(node, path, base, context, version.clone(), visible);
            if widen_member {
                if let Some(site) = self.sites.get_mut(&AstSpan::from(node)) {
                    site.widened_member = true;
                }
            }
        }
        version
    }

    fn has_member_effect(
        &self,
        base: &str,
        version: &Version,
        context: Context<'a>,
        state: &State,
    ) -> bool {
        version
            .objects
            .iter()
            .any(|id| state.mutated_objects.contains_key(id))
            || self
                .lexical
                .binding(context.scope, base)
                .is_some_and(|binding| {
                    self.effects
                        .captured_members
                        .contains_key(&binding.start_byte)
                })
    }

    pub(super) fn receiver_objects(
        &mut self,
        expression: Node<'a>,
        context: Context<'a>,
    ) -> BTreeSet<usize> {
        let mut objects = BTreeSet::new();
        for source in super::super::alias_sources(self.file, expression) {
            let work = self
                .sites
                .get(&AstSpan::from(source))
                .map_or(1, |site| site.objects.len() + 1);
            if !self.charge(context.owner, work) {
                break;
            }
            if let Some(site) = self.sites.get(&AstSpan::from(source)) {
                objects.extend(site.objects.iter().copied());
            }
        }
        objects
    }
}
