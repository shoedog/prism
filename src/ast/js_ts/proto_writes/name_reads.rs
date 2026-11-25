//! Local reaching definitions shared by the listing and its consumers.
//!
//! Completeness requires a positive transfer: straight-line evaluation, if/else,
//! short-circuit and conditional joins, plain assignments, plain loop entry
//! with widening, simple try/catch, and direct inline iteration callbacks.
//! Syntax that only groups values has no transfer of its own. Unmodelled
//! evaluation can keep candidate stores, but cannot certify a reaching set.
use super::{
    children,
    effects::{mutation_inputs, Effects},
    field, identifier, is_loop, is_owner, is_scope, operator, pattern_parts, peel, AstSpan,
    Context, Lexical, NameRead, ProtoWrite,
};
use crate::ast::ParsedFile;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

mod evaluate;
mod flow;
mod syntax;
mod transfer;
use flow::PendingJump;
pub(super) use syntax::value_children;

const OWNER_WORK_BUDGET: usize = 500_000;
const FILE_WORK_BUDGET: usize = 4_000_000;

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
    refused_reads: BTreeSet<AstSpan>,
    refused_writes: BTreeMap<usize, BTreeSet<AstSpan>>,
    member_writes: BTreeMap<usize, std::sync::Arc<BTreeSet<AstSpan>>>,
    widened_members: BTreeSet<AstSpan>,
}

impl Index {
    pub(super) fn line(&self, line: usize) -> Vec<NameRead> {
        let mut reads = self.lines.get(&line).cloned().unwrap_or_default();
        for read in &mut reads {
            let Some(binding) = read.binding.as_ref().map(|b| b.start_byte) else {
                continue;
            };
            let writes = self
                .refused_reads
                .contains(&read.span)
                .then(|| self.refused_writes.get(&binding))
                .flatten();
            let members = (self.widened_members.contains(&read.span)
                || (self.refused_reads.contains(&read.span) && read.path != read.base))
                .then(|| self.member_writes.get(&binding))
                .flatten();
            if writes.is_some() || members.is_some() {
                read.reaching_versions = read
                    .reaching_versions
                    .iter()
                    .chain(writes.into_iter().flatten())
                    .chain(members.into_iter().flat_map(|sites| sites.iter()))
                    .cloned()
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect();
            }
        }
        reads
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
    fn weight(&self) -> usize {
        self.sites.len() + self.objects.len() + 1
    }

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
            .map(Version::weight)
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
    receiver_mutated: bool,
    widened_member: bool,
}

pub(super) struct Analysis<'a> {
    file: &'a ParsedFile,
    pub(super) lexical: Lexical<'a>,
    pub(super) origins: BTreeMap<usize, Origin<'a>>,
    pub(super) defaults: BTreeMap<usize, Node<'a>>,
    pub(super) inline_owners: BTreeMap<AstSpan, AstSpan>,
    effects: Effects,
    spending: BTreeMap<usize, usize>,
    file_work: usize,
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
            file_work: 0,
            reachable_returns: BTreeSet::new(),
            deferred_initializer: None,
            sites: BTreeMap::new(),
            deferred: BTreeMap::new(),
            jumps: BTreeMap::new(),
        };
        let mut state = State::default();
        if analysis.effects.refused_file {
            eprintln!(
                "warning: {}: prototype-write analysis exhausted the capture work budget",
                file.path
            );
        }
        analysis.visit(root, context, &mut state);
        analysis.finish_owner(context, &mut state);
        analysis
    }

    pub(super) fn index(&self, facts: Vec<ProtoWrite>) -> Index {
        let mut lines = BTreeMap::<usize, Vec<NameRead>>::new();
        let mut refused_reads = BTreeSet::new();
        let mut widened_members = BTreeSet::new();
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
            if self.effects.direct_eval || self.effects.skipped.contains(&site.owner) {
                read.resolution_complete = false;
                refused_reads.insert(read.span.clone());
            }
            if site.widened_member {
                widened_members.insert(read.span.clone());
            }
            lines.entry(site.line).or_default().push(read);
        }
        let refused_writes = if refused_reads.is_empty() {
            BTreeMap::new()
        } else {
            self.effects.known.clone()
        };
        Index {
            lines,
            facts,
            refused_reads,
            refused_writes,
            widened_members,
            member_writes: self.effects.member_writes.clone(),
        }
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
        if self.effects.refused_file {
            self.effects.skipped.insert(owner.id());
            return false;
        }
        let spent = self.spending.entry(owner.id()).or_default();
        *spent = spent.saturating_add(work);
        let prior_file_work = self.file_work;
        self.file_work = self.file_work.saturating_add(work);
        if *spent <= OWNER_WORK_BUDGET && self.file_work <= FILE_WORK_BUDGET {
            return true;
        }
        self.effects.skipped.insert(owner.id());
        if self.file_work <= FILE_WORK_BUDGET || prior_file_work <= FILE_WORK_BUDGET {
            let limit = if self.file_work > FILE_WORK_BUDGET {
                "file"
            } else {
                "owner"
            };
            eprintln!(
                "warning: {}:{}: prototype-write analysis exhausted the {limit} work budget",
                self.file.path,
                owner.start_position().row + 1
            );
        }
        false
    }

    fn fork(&mut self, context: Context<'a>, state: &State) -> Option<State> {
        (!self.skipped(context.owner) && self.charge(context.owner, state.weight()))
            .then(|| state.clone())
    }

    fn join(&mut self, context: Context<'a>, state: &mut State, other: &State, uncertain: bool) {
        if !self.skipped(context.owner)
            && self.charge(context.owner, state.weight() + other.weight())
        {
            if uncertain && !other.dead && !state.dead {
                for (binding, value) in &mut state.values {
                    value.incomplete |= other.values.get(binding) != Some(value);
                }
            }
            state.join(other);
        }
    }

    fn widen(&mut self, region: Node<'a>, context: Context<'a>, state: &mut State) {
        self.widen_except(region, context, state, &BTreeMap::new());
    }

    fn widen_except(
        &mut self,
        region: Node<'a>,
        context: Context<'a>,
        state: &mut State,
        cutoffs: &BTreeMap<usize, usize>,
    ) {
        let remaining = OWNER_WORK_BUDGET.saturating_sub(
            self.spending
                .get(&context.owner.id())
                .copied()
                .unwrap_or_default(),
        );
        let writes: Vec<_> = self
            .effects
            .writes_in(region)
            .take(remaining + 1)
            .map(|write| (write.owner, write.binding, write.site.clone(), write.member))
            .collect();
        if !self.charge(context.owner, writes.len()) {
            return;
        }
        for (owner, binding, site, member) in writes {
            if site.end_byte > region.end_byte()
                || !self.effects.runs_with(owner, context.owner.id())
                || (!member
                    && cutoffs
                        .get(&binding)
                        .is_some_and(|cutoff| site.start_byte < *cutoff))
            {
                continue;
            }
            if member {
                let aliases = self.effects.member_aliases.get(&binding).cloned();
                let bindings = aliases
                    .as_deref()
                    .map_or(std::slice::from_ref(&binding), Vec::as_slice);
                if !self.charge(context.owner, bindings.len()) {
                    return;
                }
                for alias in bindings {
                    let Some(version) = state.values.get(alias) else {
                        continue;
                    };
                    if !self.charge(context.owner, version.objects.len()) {
                        return;
                    }
                    for object in &version.objects {
                        state
                            .mutated_objects
                            .entry(*object)
                            .or_default()
                            .insert(site.clone());
                    }
                }
            } else {
                let version = state.values.entry(binding).or_default();
                version.sites.insert(site.clone());
                version.objects.insert(site.start_byte);
                version.incomplete = true;
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
        !self.effects.direct_eval
            && a.binding.as_ref().is_some_and(|binding| {
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
        for _ in 0..128 {
            let Some(node) = parent else { return false };
            if is_loop(node) || self.inline_owners.contains_key(&AstSpan::from(node)) {
                return true;
            }
            if is_owner(node) {
                return false;
            }
            parent = node.parent();
        }
        true
    }

    pub(super) fn has_binding(&self, node: Node<'_>) -> bool {
        self.sites
            .get(&AstSpan::from(node))
            .is_some_and(|site| site.has_binding)
    }

    pub(super) fn unmutated_receiver(&self, receiver: Node<'_>) -> bool {
        self.sites
            .get(&AstSpan::from(peel(receiver)))
            .is_some_and(|site| !site.receiver_mutated)
    }

    pub(super) fn namespace_receiver(&self, receiver: Node<'_>) -> bool {
        self.read(receiver)
            .and_then(|read| read.binding.as_ref())
            .is_some_and(|binding| self.lexical.namespaces.contains(&binding.start_byte))
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
        let prior = self
            .sites
            .get(&span)
            .map_or(0, |site| site.read.reaching_versions.len());
        let within_budget = self.charge(
            context.owner,
            prior + version.sites.len() + version.objects.len(),
        );
        let version = if within_budget {
            version
        } else {
            Version::default()
        };
        let lexical = self.lexical.lookup(context.scope, &base);
        let binding = lexical.cloned().flatten();
        let resolution_complete = within_budget
            && !self.effects.direct_eval
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
            receiver_mutated: false,
            widened_member: false,
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
        if !version.sites.is_empty() {
            site.read.reaching_versions = site
                .read
                .reaching_versions
                .iter()
                .chain(&version.sites)
                .cloned()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
        }
    }

    fn version(&mut self, base: &str, context: Context<'a>, state: &State) -> Version {
        let Some(binding) = self.lexical.binding(context.scope, base) else {
            return Version::default();
        };
        let id = binding.start_byte;
        let deferred = self.lexical.binding_owners.get(&id).is_some_and(|owner| {
            self.deferred_initializer == Some(context.owner.id())
                || !self.in_inline_scope(&AstSpan::from(context.owner), owner)
        });
        let known = (deferred || self.effects.direct_eval)
            .then(|| self.effects.known.get(&id))
            .flatten();
        let work = state.values.get(&id).map_or(0, Version::weight)
            + known.map_or(0, BTreeSet::len)
            + self.effects.captured.get(&id).map_or(0, BTreeSet::len)
            + self.effects.mapped.get(&id).map_or(0, |sites| sites.len());
        if !self.charge(context.owner, work) {
            return Version::default();
        }
        let mut version = state
            .values
            .get(&binding.start_byte)
            .cloned()
            .unwrap_or_default();
        if let Some(sites) = (deferred || self.effects.direct_eval)
            .then(|| self.effects.known.get(&id))
            .flatten()
        {
            version.incomplete |= sites != &version.sites;
            version.sites.extend(sites.iter().cloned());
        }
        version.incomplete |=
            self.effects.direct_eval || (deferred && self.lexical.hoisted_vars.contains(&id));
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

    fn enter(&mut self, node: Node<'a>, context: Context<'a>, state: &mut State) {
        let scope = &self.lexical.scopes[&node.id()];
        let work = scope.names.len()
            + scope.entry.len()
            + scope
                .parameter_copies
                .iter()
                .map(|(_, formal)| {
                    state
                        .values
                        .get(&formal.start_byte)
                        .map_or(1, Version::weight)
                })
                .sum::<usize>();
        if !self.charge(context.owner, work) {
            return;
        }
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
}
