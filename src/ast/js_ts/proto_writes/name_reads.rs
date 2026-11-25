//! Byte-ordered local reaching definitions, shared by the listing and its consumers.
use super::{
    children,
    effects::{mutation_receiver, Effects},
    field, identifier, is_loop, is_owner, is_scope, operator, pattern_parts, peel, AstSpan,
    Context, Lexical, NameRead, ProtoWrite,
};
use crate::ast::ParsedFile;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

mod flow;
mod syntax;
use flow::PendingJump;
pub(super) use syntax::value_children;

const OWNER_WORK_BUDGET: usize = 500_000;

#[derive(Clone, Copy)]
// Versions retain the store site, while origins describe the value at that site.
// Element selects an untracked position. Child carries an explicit computed key.
pub(super) enum Origin<'a> {
    Value(Node<'a>),
    Element(Node<'a>),
    Child(Node<'a>, Node<'a>),
    Unknown,
}

#[derive(Clone, Debug, Default)]
pub(in crate::ast) struct Index {
    lines: BTreeMap<usize, Vec<NameRead>>,
    pub(super) facts: Vec<ProtoWrite>,
}

impl Index {
    pub(super) fn line(&self, line: usize) -> Vec<NameRead> {
        self.lines.get(&line).cloned().unwrap_or_default()
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
struct Version {
    sites: BTreeSet<AstSpan>,
    // Abstract receiver identities are allocation/definition sites. Alias copies
    // preserve them. A member alias conservatively shares its base identity.
    objects: BTreeSet<usize>,
    // A path is incomplete if a transfer or join cannot prove its reaching set.
    // Object mutations separately affect member reads, leaving bare aliases live.
    incomplete: bool,
    mutated: bool,
}

impl Version {
    fn merge(&mut self, other: &Self) {
        self.sites.extend(other.sites.iter().cloned());
        self.objects.extend(other.objects.iter().copied());
        self.incomplete |= other.incomplete;
        self.mutated |= other.mutated;
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
struct State {
    values: BTreeMap<usize, Version>,
    mutated_objects: BTreeMap<usize, BTreeSet<AstSpan>>,
    dead: bool,
}

impl State {
    fn unreachable() -> Self {
        Self {
            dead: true,
            ..Self::default()
        }
    }

    fn invalidate(&mut self) {
        for value in self.values.values_mut() {
            value.incomplete = true;
        }
    }

    fn join(&mut self, other: &Self) {
        if other.dead {
            return;
        }
        if self.dead {
            *self = other.clone();
            return;
        }
        for (object, sites) in &other.mutated_objects {
            self.mutated_objects
                .entry(*object)
                .or_default()
                .extend(sites.iter().cloned());
        }
        for (binding, value) in &mut self.values {
            if let Some(other) = other.values.get(binding) {
                value.merge(other);
            } else {
                value.incomplete = true;
            }
        }
        for (binding, value) in &other.values {
            self.values.entry(*binding).or_insert_with(|| Version {
                incomplete: true,
                ..value.clone()
            });
        }
    }

    fn weight(&self) -> usize {
        // Charge the material copied or joined, including candidate histories.
        self.values
            .values()
            .map(|version| version.sites.len() + version.objects.len() + 1)
            .chain(self.mutated_objects.values().map(|sites| sites.len() + 1))
            .sum::<usize>()
            + 1
    }

    fn install(&mut self, binding: usize, site: AstSpan) {
        if !self.dead {
            self.values.insert(
                binding,
                Version {
                    objects: BTreeSet::from([site.start_byte]),
                    sites: BTreeSet::from([site]),
                    ..Version::default()
                },
            );
        }
    }
}

struct Site {
    line: usize,
    owner: usize,
    read: NameRead,
    visible: bool,
    has_binding: bool,
    objects: BTreeSet<usize>,
}

pub(super) struct Analysis<'a> {
    file: &'a ParsedFile,
    pub(super) lexical: Lexical<'a>,
    pub(super) origins: BTreeMap<usize, Origin<'a>>,
    pub(super) defaults: BTreeMap<usize, Node<'a>>,
    pub(super) inline_owners: BTreeMap<AstSpan, AstSpan>,
    effects: Effects,
    spending: BTreeMap<usize, usize>,
    reachable_returns: BTreeSet<usize>,
    deferred_initializer: Option<usize>,
    sites: BTreeMap<AstSpan, Site>,
    deferred: BTreeMap<usize, BTreeMap<usize, (Node<'a>, Context<'a>)>>,
    jumps: BTreeMap<(usize, &'static str), PendingJump>,
}

impl<'a> Analysis<'a> {
    pub(super) fn build(file: &'a ParsedFile) -> Self {
        let root = file.tree.root_node();
        let context = Context {
            owner: root,
            scope: root,
        };
        let lexical = Lexical::build(file);
        let mut effects = Effects::build(file, &lexical);
        let inline_owners = std::mem::take(&mut effects.inline);
        let mut analysis = Self {
            file,
            lexical,
            origins: BTreeMap::new(),
            defaults: BTreeMap::new(),
            inline_owners,
            effects,
            spending: BTreeMap::new(),
            reachable_returns: BTreeSet::new(),
            deferred_initializer: None,
            sites: BTreeMap::new(),
            deferred: BTreeMap::new(),
            jumps: BTreeMap::new(),
        };
        let mut state = State::default();
        analysis.visit(root, context, &mut state);
        analysis.finish_owner(context, &mut state);
        analysis
    }

    pub(super) fn index(&self, facts: Vec<ProtoWrite>) -> Index {
        let mut lines = BTreeMap::<usize, Vec<NameRead>>::new();
        let selectors: BTreeSet<_> = facts
            .iter()
            .flat_map(|fact| &fact.key_operands)
            .filter(|operand| operand.role == super::KeyRole::Selector)
            .map(|operand| (operand.start_byte, operand.end_byte))
            .collect();
        for site in self.sites.values().filter(|site| {
            site.visible
                || selectors.contains(&(site.read.span.start_byte, site.read.span.end_byte))
        }) {
            let mut read = site.read.clone();
            read.resolution_complete &= !self.effects.skipped.contains(&site.owner);
            lines.entry(site.line).or_default().push(read);
        }
        Index { lines, facts }
    }

    pub(super) fn read(&self, node: Node<'_>) -> Option<&NameRead> {
        self.read_at(&AstSpan::from(peel(node)))
    }

    pub(super) fn skipped(&self, node: Node<'_>) -> bool {
        self.effects.skipped.contains(&node.id())
    }

    pub(super) fn reachable_return(&self, node: Node<'_>) -> bool {
        self.reachable_returns.contains(&node.id())
    }

    fn charge(&mut self, owner: Node<'_>, work: usize) -> bool {
        if self.skipped(owner) {
            return false;
        }
        let spent = self.spending.entry(owner.id()).or_default();
        *spent = spent.saturating_add(work);
        if *spent <= OWNER_WORK_BUDGET {
            return true;
        }
        self.effects.skipped.insert(owner.id());
        eprintln!(
            "warning: {}:{}: prototype-write analysis exhausted the owner work budget",
            self.file.path,
            owner.start_position().row + 1
        );
        false
    }

    fn widen(&mut self, region: Node<'a>, context: Context<'a>, state: &mut State) {
        // Bound the state passed across a back edge before visiting the body.
        // A write can be a candidate even when the first iteration has no value.
        let remaining = OWNER_WORK_BUDGET.saturating_sub(
            self.spending
                .get(&context.owner.id())
                .copied()
                .unwrap_or_default(),
        );
        let writes: Vec<_> = self
            .effects
            .writes_in(region, context.owner)
            .take(remaining + 1)
            .map(|write| (write.binding, write.site.clone(), write.member))
            .collect();
        if !self.charge(context.owner, writes.len()) {
            return;
        }
        for (binding, site, member) in writes {
            if member {
                let aliases = self.effects.member_aliases.get(&binding).cloned();
                let bindings = aliases
                    .as_deref()
                    .map_or(std::slice::from_ref(&binding), Vec::as_slice);
                if !self.charge(context.owner, bindings.len()) {
                    return;
                }
                for alias in bindings {
                    if let Some(version) = state.values.get(alias) {
                        for object in &version.objects {
                            state
                                .mutated_objects
                                .entry(*object)
                                .or_default()
                                .insert(site.clone());
                        }
                    }
                }
            } else {
                let version = state.values.entry(binding).or_default();
                if version.sites.insert(site.clone()) {
                    version.objects.insert(site.start_byte);
                    version.incomplete = true;
                }
            }
        }
    }

    pub(super) fn read_at(&self, span: &AstSpan) -> Option<&NameRead> {
        self.sites
            .get(span)
            .filter(|site| !self.effects.skipped.contains(&site.owner))
            .map(|site| &site.read)
    }

    pub(super) fn deferred_definition(&self, read: &NameRead, site: &AstSpan) -> bool {
        read.binding.as_ref().is_some_and(|binding| {
            self.effects
                .captured
                .get(&binding.start_byte)
                .is_some_and(|writes| writes.contains(site))
                || self
                    .effects
                    .mapped
                    .get(&binding.start_byte)
                    .is_some_and(|writes| writes.contains(site))
        })
    }

    pub(super) fn correlated_identifiers(&self, a: &NameRead, b: &NameRead) -> bool {
        // A widened loop header can have several prior values. Two identifiers
        // inside one assignment still name the same current slot if there is
        // no intervening rebind and no deferred or mapped write.
        a.binding.as_ref().is_some_and(|binding| {
            a.owner == b.owner
                && a.span.end_byte <= b.span.start_byte
                && !self.effects.captured.contains_key(&binding.start_byte)
                && !self.effects.mapped.contains_key(&binding.start_byte)
                && !self.effects.rebound_between(
                    binding.start_byte,
                    a.span.end_byte,
                    b.span.start_byte,
                )
                && self
                    .sites
                    .get(&a.span)
                    .is_some_and(|site| !self.effects.skipped.contains(&site.owner))
        })
    }

    pub(super) fn may_repeat(&self, node: Node<'_>) -> bool {
        let mut parent = node.parent();
        while let Some(node) = parent {
            if is_loop(node) || self.inline_owners.contains_key(&AstSpan::from(node)) {
                return true;
            }
            if is_owner(node) {
                break;
            }
            parent = node.parent();
        }
        false
    }

    pub(super) fn has_binding(&self, node: Node<'_>) -> bool {
        self.sites
            .get(&AstSpan::from(node))
            .is_some_and(|site| site.has_binding)
    }

    fn read_path(
        &mut self,
        node: Node<'a>,
        path: String,
        base: String,
        context: Context<'a>,
        version: Version,
        visible: bool,
    ) {
        let span = AstSpan::from(node);
        let within_budget = self.charge(context.owner, version.sites.len() + version.objects.len());
        let lexical = self.lexical.lookup(context.scope, &base);
        let binding = lexical.cloned().flatten();
        let resolution_complete = within_budget
            && binding.is_some()
            && !version.incomplete
            && !version.mutated
            && !version.sites.is_empty();
        let site = self.sites.entry(span.clone()).or_insert_with(|| Site {
            line: node.start_position().row + 1,
            owner: context.owner.id(),
            visible,
            has_binding: lexical.is_some(),
            objects: BTreeSet::new(),
            read: NameRead {
                path,
                base,
                span,
                owner: AstSpan::from(context.owner),
                binding,
                reaching_versions: Vec::new(),
                resolution_complete,
            },
        });
        site.visible |= visible;
        site.objects.extend(version.objects);
        site.read.resolution_complete &= resolution_complete;
        for definition in version.sites {
            if let Err(index) = site.read.reaching_versions.binary_search(&definition) {
                site.read.reaching_versions.insert(index, definition);
            }
        }
    }

    fn version(&self, base: &str, context: Context<'a>, state: &State) -> Version {
        let Some(binding) = self.lexical.binding(context.scope, base) else {
            return Version::default();
        };
        let mut version = state
            .values
            .get(&binding.start_byte)
            .cloned()
            .unwrap_or_default();
        if let Some(owner) = self.lexical.binding_owners.get(&binding.start_byte) {
            if self.deferred_initializer == Some(context.owner.id())
                || !self.in_inline_scope(&AstSpan::from(context.owner), owner)
            {
                if let Some(sites) = self.effects.known.get(&binding.start_byte) {
                    version.incomplete |= sites != &version.sites;
                    version.sites.extend(sites.iter().cloned());
                }
            }
        }
        for sites in [
            self.effects.captured.get(&binding.start_byte),
            self.effects
                .mapped
                .get(&binding.start_byte)
                .map(AsRef::as_ref),
        ]
        .into_iter()
        .flatten()
        {
            version.incomplete = true;
            version.sites.extend(sites.iter().cloned());
        }
        version.incomplete |= state.dead;
        version
    }

    pub(super) fn in_inline_scope<'scope>(
        &'scope self,
        mut owner: &'scope AstSpan,
        declaration_owner: &AstSpan,
    ) -> bool {
        while owner != declaration_owner {
            let Some(parent) = self.inline_owners.get(owner) else {
                return false;
            };
            owner = parent;
        }
        true
    }

    fn enter(&self, node: Node<'a>, state: &mut State) {
        let scope = &self.lexical.scopes[&node.id()];
        for binding in scope.names.values().flatten() {
            state.values.remove(&binding.start_byte);
        }
        for site in &scope.entry {
            state.install(site.start_byte, site.clone());
        }
        for (binding, formal) in &scope.parameter_copies {
            if let Some(value) = state.values.get(&formal.start_byte).cloned() {
                state.values.insert(*binding, value);
            }
        }
    }

    fn write(
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
                let objects: BTreeSet<_> = super::alias_sources(self.file, source)
                    .into_iter()
                    .filter_map(|source| self.sites.get(&AstSpan::from(source)))
                    .flat_map(|read| read.objects.iter().copied())
                    .collect();
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
                    ("array_pattern", Origin::Value(value)) if peel(value).kind() == "array" => {
                        super::array_element(peel(value), index)
                            .map_or(Origin::Unknown, Origin::Value)
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

    fn mutate(&self, objects: BTreeSet<usize>, site: AstSpan, state: &mut State) {
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

    fn target_reads(
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
                    .member(node, context, state, load)
                    .map_or_else(BTreeSet::new, |version| version.objects);
            }
            "assignment_pattern"
            | "object_assignment_pattern"
            | "required_parameter"
            | "optional_parameter" => {
                if let Some(right) = field(node, "right").or_else(|| field(node, "value")) {
                    let mut default = state.clone();
                    self.visit(right, context, &mut default);
                    state.join(&default);
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

    fn member(
        &mut self,
        node: Node<'a>,
        context: Context<'a>,
        state: &mut State,
        visible: bool,
    ) -> Option<Version> {
        let Some(object) = field(node, "object") else {
            return None;
        };
        let canonical = super::path(self.file, node);
        let mut version = canonical
            .as_ref()
            .map(|(base, _)| self.version(base, context, state));
        let object = peel(object);
        match object.kind() {
            "member_expression" | "subscript_expression" => {
                version = self.member(object, context, state, canonical.is_none());
            }
            _ if identifier(object) && canonical.is_some() => {
                let base = self.file.node_text(&object).to_string();
                let mut v = version.clone().unwrap_or_default();
                v.mutated = false;
                self.read_path(object, base.clone(), base, context, v, false);
            }
            _ => self.visit(object, context, state),
        }
        if let Some(key) = field(node, "index") {
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
                } else if super::literal_index(self.file, key).is_none() {
                    version.incomplete = true;
                }
            }
        }
        if let (Some((base, path)), Some(version)) = (canonical, &mut version) {
            if visible {
                let capacity = OWNER_WORK_BUDGET.saturating_sub(
                    self.spending
                        .get(&context.owner.id())
                        .copied()
                        .unwrap_or_default(),
                );
                let mut writes = BTreeSet::new();
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
                    writes.extend(sites.iter().take(capacity.saturating_add(1)).cloned());
                    if writes.len() > capacity {
                        break;
                    }
                }
                if self.charge(context.owner, writes.len()) {
                    version.mutated |= !writes.is_empty();
                    version.sites.extend(writes);
                } else {
                    version.incomplete = true;
                }
            }
            self.read_path(node, path, base, context, version.clone(), visible);
        }
        version
    }

    fn visit(&mut self, node: Node<'a>, mut context: Context<'a>, state: &mut State) {
        let outside = context;
        if !self.charge(context.owner, 1) {
            self.inventory(node, context, state);
            return;
        }
        if costly_transfer(node, self.file) && !self.charge(context.owner, state.weight()) {
            self.inventory(node, context, state);
            return;
        }
        if is_owner(node) && node != context.owner {
            for eager in children(node)
                .filter(|child| matches!(child.kind(), "decorator" | "computed_property_name"))
            {
                self.visit(eager, context, state);
            }
            let closures = self.deferred.entry(context.owner.id()).or_default();
            closures.insert(node.start_byte(), (node, context));
            return;
        }
        if is_scope(node) {
            context.scope = node;
            self.enter(node, state);
        }
        match node.kind() {
            "identifier" | "shorthand_property_identifier" | "this" | "super" => {
                let base = self.file.node_text(&node).to_string();
                let mut version = self.version(&base, context, state);
                version.mutated = false;
                version.incomplete |= state.dead;
                self.read_path(node, base.clone(), base, context, version, true);
            }
            "member_expression" | "subscript_expression" => {
                self.member(node, context, state, true);
            }
            "assignment_expression"
            | "augmented_assignment_expression"
            | "variable_declarator"
            | "update_expression" => {
                let declaration = node.kind() == "variable_declarator";
                let target = field(node, "left")
                    .or_else(|| field(node, "argument"))
                    .or_else(|| field(node, "name"));
                let right = field(node, "right").or_else(|| field(node, "value"));
                let assignment = declaration || node.kind() == "assignment_expression";
                let logical = matches!(operator(node, self.file), "||=" | "&&=" | "??=");
                if let Some(target) = target {
                    let member = matches!(
                        peel(target).kind(),
                        "member_expression" | "subscript_expression"
                    );
                    let captured = if !declaration {
                        self.target_reads(target, context, state, !assignment)
                    } else {
                        BTreeSet::new()
                    };
                    let bypass = logical.then(|| state.clone());
                    if let Some(right) = right {
                        self.visit(right, context, state);
                    }
                    let origin = if assignment || logical {
                        right.map_or(Origin::Unknown, Origin::Value)
                    } else {
                        Origin::Unknown
                    };
                    if member {
                        self.mutate(captured, peel(target).into(), state);
                    } else if !super::bare_var(node) {
                        self.write(target, origin, context, state);
                    }
                    if let Some(bypass) = bypass {
                        state.join(&bypass);
                    }
                }
            }
            "if_statement" | "ternary_expression" | "binary_expression"
                if node.kind() != "binary_expression"
                    || matches!(operator(node, self.file), "&&" | "||" | "??") =>
            {
                if let Some(test) = field(node, "condition").or_else(|| field(node, "left")) {
                    self.visit(test, context, state);
                }
                let mut alternative = state.clone();
                if let Some(left) = field(node, "consequence").or_else(|| field(node, "right")) {
                    self.visit(left, context, state);
                }
                if let Some(right) = field(node, "alternative") {
                    self.visit(right, context, &mut alternative);
                }
                state.join(&alternative);
            }
            _ if is_loop(node) => self.loop_body(node, context, state),
            "with_statement" => {
                if let Some(object) = field(node, "object") {
                    self.visit(object, outside, state);
                }
                if let Some(body) = field(node, "body") {
                    self.visit(body, context, state);
                }
            }
            "unary_expression" if operator(node, self.file) == "delete" => {
                if let Some(target) = field(node, "argument") {
                    let receiver = self.target_reads(target, context, state, false);
                    self.mutate(receiver, peel(target).into(), state);
                }
            }
            "return_statement" | "throw_statement" | "break_statement" | "continue_statement" => {
                for child in children(node) {
                    self.visit(child, context, state);
                }
                if !state.dead {
                    self.jump(node, context, state);
                }
            }
            "call_expression" => self.visit_call(node, context, state),
            "catch_clause" => {
                if let Some(parameter) = field(node, "parameter") {
                    self.write(parameter, Origin::Unknown, context, state);
                }
                if let Some(body) = field(node, "body") {
                    self.visit(body, context, state);
                }
            }
            "try_statement" => self.visit_try(node, context, state),
            "switch_statement" => self.visit_switch(node, context, state),
            "class" | "class_declaration" | "abstract_class_declaration" => {
                let name = field(node, "name");
                for child in children(node).filter(|child| Some(*child) != name) {
                    if child.kind() == "class_body" {
                        if let Some(name) = name {
                            state.install(name.start_byte(), name.into());
                        }
                    }
                    self.visit(child, context, state);
                }
            }
            "field_definition" | "public_field_definition" => {
                let deferred = super::instance_initializer(node);
                for child in value_children(self.file, node) {
                    if deferred == Some(child) {
                        let mut instance = state.clone();
                        let outer = self.deferred_initializer.replace(context.owner.id());
                        self.visit(child, context, &mut instance);
                        self.deferred_initializer = outer;
                    } else {
                        self.visit(child, context, state);
                    }
                }
            }
            "import_statement"
            | "type_alias_declaration"
            | "interface_declaration"
            | "type_annotation"
            | "type_arguments"
            | "type_parameters"
            | "function_signature"
            | "method_signature"
            | "abstract_method_signature" => {}
            "parenthesized_expression"
            | "non_null_expression"
            | "as_expression"
            | "satisfies_expression"
            | "type_assertion" => {
                let value = peel(node);
                if value != node {
                    self.visit(value, context, state);
                }
            }
            _ => {
                for child in value_children(self.file, node) {
                    self.visit(child, context, state);
                }
            }
        }
        if matches!(node.kind(), "labeled_statement" | "switch_statement") {
            if let Some(exit) = self.jumps.remove(&(node.id(), "break_statement")) {
                state.join(&exit.state);
            }
        }
    }
}

impl<'a> Analysis<'a> {
    fn visit_call(&mut self, node: Node<'a>, context: Context<'a>, state: &mut State) {
        let mutation = mutation_receiver(self.file, &self.lexical, node, context);
        if let Some(callee) = field(node, "function") {
            self.visit(callee, context, state);
            if self.file.node_text(&callee) == "eval" && !self.has_binding(callee) {
                state.invalidate();
            }
        }
        let iteration = super::iteration(self.file, node);
        let mut callback = None;
        if let Some(arguments) = field(node, "arguments") {
            for (index, arg) in children(arguments).enumerate() {
                if index == 0 && iteration.is_some() && super::is_inline_callback(peel(arg)) {
                    callback = Some(peel(arg));
                } else {
                    self.visit(arg, context, state);
                }
            }
        }
        if let Some(callback) = callback {
            if self.charge(context.owner, state.weight()) {
                self.function(callback, context, state, iteration);
            } else {
                self.function(callback, context, &mut State::default(), None);
            }
        }
        if let Some(receiver) = mutation {
            if let Some(site) = self.sites.get(&AstSpan::from(peel(receiver))) {
                self.mutate(site.objects.clone(), node.into(), state);
            }
        }
    }
}

fn costly_transfer(node: Node<'_>, file: &ParsedFile) -> bool {
    is_loop(node)
        || matches!(
            node.kind(),
            "if_statement"
                | "ternary_expression"
                | "try_statement"
                | "switch_statement"
                | "return_statement"
                | "throw_statement"
                | "continue_statement"
                | "break_statement"
                | "field_definition"
                | "public_field_definition"
        )
        || matches!(
            operator(node, file),
            "&&" | "||" | "??" | "||=" | "&&=" | "??="
        )
}
