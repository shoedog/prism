//! Byte-ordered local reaching definitions, shared by the listing and its consumers.
use super::{
    children, field, identifier, is_inline_callback, is_owner, is_scope, operator, pattern_parts,
    peel, AstSpan, Context, Lexical, NameRead, ProtoWrite,
};
use crate::ast::ParsedFile;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

#[derive(Clone, Copy)]
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
    objects: BTreeSet<usize>,
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
    mutated_objects: BTreeSet<usize>,
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
        self.mutated_objects
            .extend(other.mutated_objects.iter().copied());
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
    known_writes: BTreeMap<usize, BTreeSet<AstSpan>>,
    sites: BTreeMap<AstSpan, Site>,
    deferred: BTreeMap<usize, BTreeMap<usize, (Node<'a>, Context<'a>)>>,
    jumps: BTreeMap<(usize, &'static str), State>,
}

impl<'a> Analysis<'a> {
    pub(super) fn build(file: &'a ParsedFile) -> Self {
        let root = file.tree.root_node();
        let context = Context {
            owner: root,
            scope: root,
            dynamic: false,
        };
        let mut analysis = Self {
            file,
            lexical: Lexical::build(file),
            origins: BTreeMap::new(),
            defaults: BTreeMap::new(),
            inline_owners: BTreeMap::new(),
            known_writes: BTreeMap::new(),
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
        for site in self.sites.values().filter(|site| site.visible) {
            lines.entry(site.line).or_default().push(site.read.clone());
        }
        Index { lines, facts }
    }

    pub(super) fn read(&self, node: Node<'_>) -> Option<&NameRead> {
        self.sites
            .get(&AstSpan::from(peel(node)))
            .map(|site| &site.read)
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
        let lexical = self.lexical.lookup(context.scope, &base);
        let binding = lexical.cloned().flatten().filter(|_| !context.dynamic);
        let resolution_complete = binding.is_some()
            && !version.incomplete
            && !version.mutated
            && !version.sites.is_empty();
        let site = self.sites.entry(span.clone()).or_insert_with(|| Site {
            line: node.start_position().row + 1,
            visible,
            has_binding: lexical.is_some() || context.dynamic,
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
        let Some(binding) = self
            .lexical
            .binding(context.scope, base)
            .filter(|_| !context.dynamic)
        else {
            return Version::default();
        };
        let mut version = state
            .values
            .get(&binding.start_byte)
            .cloned()
            .unwrap_or_default();
        if let Some(owner) = self.lexical.binding_owners.get(&binding.start_byte) {
            if !self.in_inline_scope(&AstSpan::from(context.owner), owner) {
                if let Some(sites) = self.known_writes.get(&binding.start_byte) {
                    version.incomplete |= sites != &version.sites;
                    version.sites.extend(sites.iter().cloned());
                }
            }
        }
        version.incomplete |= state.dead;
        version.mutated |= !version.objects.is_disjoint(&state.mutated_objects);
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
            if context.dynamic {
                state.invalidate();
                return;
            }
            let Some(binding) = self
                .lexical
                .binding(context.scope, self.file.node_text(&target))
            else {
                return;
            };
            self.origins.insert(target.start_byte(), origin);
            let site = AstSpan::from(target);
            self.known_writes
                .entry(binding.start_byte)
                .or_default()
                .insert(site.clone());
            state.install(binding.start_byte, site);
            if let Origin::Value(source) = origin {
                let source = peel(source);
                if let Some(read) = self.sites.get(&AstSpan::from(source)).filter(|read| {
                    identifier(source) && read.read.binding.is_some() && !read.objects.is_empty()
                }) {
                    if let Some(value) = state.values.get_mut(&binding.start_byte) {
                        value.objects.clone_from(&read.objects);
                    }
                }
            }
        } else if let Some((base, _)) = super::path(self.file, target) {
            if let Some(binding) = self.lexical.binding(context.scope, &base) {
                if let Some(version) = state.values.get(&binding.start_byte) {
                    state
                        .mutated_objects
                        .extend(version.objects.iter().copied());
                }
            }
        } else {
            let mut array_index = 0;
            let mut at = target.start_byte();
            self.target_reads(target, context, state, false);
            for part in pattern_parts(target) {
                if matches!(
                    peel(part).kind(),
                    "member_expression" | "subscript_expression"
                ) {
                    self.target_reads(part, context, state, false);
                }
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
                        array_index += self.file.source[at..part.start_byte()]
                            .bytes()
                            .filter(|&b| b == b',')
                            .count();
                        at = part.end_byte();
                        super::array_element(peel(value), array_index)
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

    fn target_reads(
        &mut self,
        node: Node<'a>,
        context: Context<'a>,
        state: &mut State,
        load: bool,
    ) {
        let node = peel(node);
        match node.kind() {
            "subscript_expression" | "member_expression" if load => {
                self.member(node, context, state, true);
            }
            "subscript_expression" | "member_expression" => {
                if let Some(object) = field(node, "object") {
                    self.visit(object, context, state);
                }
                if let Some(key) = field(node, "index") {
                    self.visit(key, context, state);
                }
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
        if let Some(version) = &mut version {
            version.mutated |= !version.objects.is_disjoint(&state.mutated_objects);
        }
        if let (Some((base, path)), Some(version)) = (canonical, &version) {
            self.read_path(node, path, base, context, version.clone(), visible);
        }
        version
    }

    fn finish_owner(&mut self, context: Context<'a>, state: &mut State) {
        if let Some(exit) = self.jumps.remove(&(context.owner.id(), "return_statement")) {
            state.join(&exit);
        }
        if let Some(closures) = self.deferred.remove(&context.owner.id()) {
            for (closure, outer) in closures.into_values() {
                self.function(closure, outer, state, None);
            }
        }
    }

    fn function(
        &mut self,
        node: Node<'a>,
        outer: Context<'a>,
        state: &mut State,
        iteration: Option<(Node<'a>, &str)>,
    ) {
        let mut local = state.clone();
        let context = Context {
            scope: node,
            owner: node,
            dynamic: outer.dynamic,
        };
        if iteration.is_some() {
            self.inline_owners
                .insert(AstSpan::from(node), AstSpan::from(outer.owner));
        }
        self.enter(node, &mut local);
        if let Some(params) = field(node, "parameters").or_else(|| field(node, "parameter")) {
            let parameters = if params.kind() == "formal_parameters" {
                children(params).collect()
            } else {
                vec![params]
            };
            for (index, parameter) in parameters.into_iter().enumerate() {
                let origin = iteration
                    .filter(|(_, method)| index == usize::from(*method == "reduce"))
                    .map_or(Origin::Unknown, |(receiver, _)| Origin::Element(receiver));
                self.write(parameter, origin, context, &mut local);
            }
        }
        if let Some(body) = field(node, "body") {
            self.visit(body, context, &mut local);
        }
        self.finish_owner(context, &mut local);
        if let Some(thrown) = self.jumps.remove(&(node.id(), "throw_statement")) {
            if iteration.is_some() {
                self.save_jump((outer.owner.id(), "throw_statement"), &thrown);
            }
        }
        if iteration.is_some() {
            local.values.retain(|binding, _| {
                self.lexical.binding_owners.get(binding) != Some(&AstSpan::from(node))
            });
            state.join(&local);
        }
    }

    fn loop_body(&mut self, node: Node<'a>, context: Context<'a>, state: &mut State) {
        if let Some(init) = field(node, "initializer").or_else(|| field(node, "right")) {
            self.visit(init, context, state);
        }
        let entry = state.clone();
        let mut header = entry.clone();
        let mut exit = State::unreachable();
        loop {
            let mut body = header.clone();
            let condition = field(node, "condition");
            if node.kind() != "do_statement" {
                if let Some(test) = condition {
                    self.visit(test, context, &mut body);
                }
                if condition.is_some_and(|test| test.kind() != "empty_statement")
                    || node.kind() == "for_in_statement"
                {
                    exit.join(&body);
                }
            }
            if let (Some(left), Some(right)) = (field(node, "left"), field(node, "right")) {
                self.target_reads(left, context, &mut body, false);
                self.write(left, Origin::Element(right), context, &mut body);
            }
            if let Some(statement) = field(node, "body") {
                self.visit(statement, context, &mut body);
            }
            if let Some(next) = self.jumps.remove(&(node.id(), "continue_statement")) {
                body.join(&next);
            }
            if let Some(increment) = field(node, "increment") {
                self.visit(increment, context, &mut body);
            }
            if node.kind() == "do_statement" {
                if let Some(test) = condition {
                    self.visit(test, context, &mut body);
                }
                exit.join(&body);
            }
            if let Some(breaks) = self.jumps.remove(&(node.id(), "break_statement")) {
                exit.join(&breaks);
            }
            let mut next = entry.clone();
            next.join(&body);
            if next == header {
                break;
            }
            header = next;
        }
        *state = exit;
    }

    fn jump(&mut self, node: Node<'a>, context: Context<'a>, state: &mut State) {
        let kind = node.kind();
        let label = field(node, "label").map(|label| self.file.node_text(&label));
        let mut outer = node.parent();
        let mut target = context.owner;
        if matches!(kind, "break_statement" | "continue_statement") {
            while let Some(parent) = outer.filter(|n| *n != context.owner) {
                if let Some(label) = label {
                    if parent.kind() == "labeled_statement"
                        && field(parent, "label").is_some_and(|n| self.file.node_text(&n) == label)
                    {
                        target = if kind == "continue_statement" {
                            field(parent, "body").unwrap_or(parent)
                        } else {
                            parent
                        };
                        break;
                    }
                } else if is_loop(parent)
                    || (kind == "break_statement" && parent.kind() == "switch_statement")
                {
                    target = parent;
                    break;
                }
                outer = parent.parent();
            }
        }
        self.save_jump((target.id(), kind), state);
        state.dead = true;
    }

    fn save_jump(&mut self, jump: (usize, &'static str), state: &State) {
        if !state.dead {
            self.jumps
                .entry(jump)
                .and_modify(|saved| saved.join(state))
                .or_insert_with(|| state.clone());
        }
    }

    fn visit_try(&mut self, node: Node<'a>, context: Context<'a>, state: &mut State) {
        let outside = std::mem::take(&mut self.jumps);
        let mut caught = state.clone();
        if let Some(body) = field(node, "body") {
            self.visit(body, context, state);
        }
        if let Some(handler) = field(node, "handler") {
            caught.join(state);
            if let Some(thrown) = self.jumps.remove(&(context.owner.id(), "throw_statement")) {
                caught.join(&thrown);
            }
            // An implicit exception can leave from inside an expression.
            caught.invalidate();
            self.visit(handler, context, &mut caught);
            state.join(&caught);
        }
        if let Some(finalizer) = field(node, "finalizer") {
            let exits = std::mem::take(&mut self.jumps);
            if !state.dead || exits.is_empty() {
                self.visit(finalizer, context, state);
            }
            for (jump, mut pending) in exits {
                self.visit(finalizer, context, &mut pending);
                self.save_jump(jump, &pending);
            }
        }
        for (jump, state) in outside {
            self.save_jump(jump, &state);
        }
    }

    fn visit_switch(&mut self, node: Node<'a>, context: Context<'a>, state: &mut State) {
        if let Some(value) = field(node, "value") {
            self.visit(value, context, state);
        }
        let Some(body) = field(node, "body") else {
            return;
        };
        let context = Context {
            scope: body,
            ..context
        };
        self.enter(body, state);
        let mut has_default = false;
        let mut cases = Vec::new();
        for case in children(body) {
            let at = field(case, "value");
            let input = if let Some(test) = at {
                self.visit(test, context, state);
                Some(state.clone())
            } else {
                has_default = true;
                None
            };
            cases.push((case, at, input));
        }
        let mut fallthrough = State::unreachable();
        for (case, test, input) in cases {
            fallthrough.join(input.as_ref().unwrap_or(state));
            for statement in children(case).filter(|statement| Some(*statement) != test) {
                self.visit(statement, context, &mut fallthrough);
            }
        }
        if !has_default {
            fallthrough.join(state);
        }
        *state = fallthrough;
    }

    fn visit(&mut self, node: Node<'a>, mut context: Context<'a>, state: &mut State) {
        if is_owner(node) && node != context.owner {
            if let Some(name) = field(node, "name").filter(|n| n.kind() == "computed_property_name")
            {
                self.visit(name, context, state);
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
                    if !declaration
                        && !matches!(peel(target).kind(), "array_pattern" | "object_pattern")
                    {
                        self.target_reads(target, context, state, !assignment);
                    }
                    let bypass = logical.then(|| state.clone());
                    if let Some(right) = right {
                        self.visit(right, context, state);
                    }
                    let origin = if assignment || logical {
                        right.map_or(Origin::Unknown, Origin::Value)
                    } else {
                        Origin::Unknown
                    };
                    self.write(target, origin, context, state);
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
                    self.visit(object, context, state);
                }
                if let Some(body) = field(node, "body") {
                    self.visit(
                        body,
                        Context {
                            dynamic: true,
                            ..context
                        },
                        state,
                    );
                }
            }
            "unary_expression" if operator(node, self.file) == "delete" => {
                if let Some(target) = field(node, "argument") {
                    self.target_reads(target, context, state, false);
                    self.write(target, Origin::Unknown, context, state);
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
            "call_expression" => {
                if let Some(callee) = field(node, "function") {
                    self.visit(callee, context, state);
                    if self.file.node_text(&callee) == "eval" && !self.has_binding(callee) {
                        state.invalidate();
                    }
                }
                let receiver = super::iteration(self.file, node);
                if let Some(arguments) = field(node, "arguments") {
                    let mut callback = None;
                    for (index, arg) in children(arguments).enumerate() {
                        if index == 0 && receiver.is_some() && is_inline_callback(peel(arg)) {
                            callback = Some(peel(arg));
                        } else {
                            self.visit(arg, context, state);
                        }
                    }
                    if let Some(callback) = callback {
                        let entry = state.clone();
                        loop {
                            let before = state.clone();
                            self.function(callback, context, state, receiver);
                            state.join(&entry);
                            if *state == before {
                                break;
                            }
                        }
                    }
                }
                if let Some(callee) = field(node, "function")
                    .filter(|callee| super::member_mutator(self.file, *callee))
                {
                    self.write(callee, Origin::Unknown, context, state);
                }
            }
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
            "class_declaration" | "abstract_class_declaration" => {
                for child in children(node).filter(|child| Some(*child) != field(node, "name")) {
                    self.visit(child, context, state);
                }
                if let (Some(name), Some(parent)) = (
                    field(node, "name"),
                    self.lexical.scopes[&context.scope.id()].parent,
                ) {
                    if let Some(Some(binding)) = self.lexical.scopes[&parent]
                        .names
                        .get(self.file.node_text(&name))
                    {
                        state.install(binding.start_byte, AstSpan::from(name));
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
                for child in children(node) {
                    self.visit(child, context, state);
                }
            }
        }
        if matches!(node.kind(), "labeled_statement" | "switch_statement") {
            if let Some(exit) = self.jumps.remove(&(node.id(), "break_statement")) {
                state.join(&exit);
            }
        }
    }
}

fn is_loop(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "for_statement" | "for_in_statement" | "while_statement" | "do_statement"
    )
}
