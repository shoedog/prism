//! Syntactic write census, computed before any occurrence can claim a complete version.
use super::{
    children, eager_owner_inputs, field, has_token, identifier, inline_iteration,
    instance_initializer, is_owner, is_scope, parameter_decorator, parameters, peel, Context,
    Lexical,
};
use crate::ast::ParsedFile;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::Arc,
};
use tree_sitter::Node;

use super::AstSpan;

mod proof;
use proof::CensusProof;

pub(super) const MAX_SUBSCRIPT_WRITES: usize = 10_000;

#[derive(Default)]
pub(super) struct Effects {
    pub(super) skipped: BTreeSet<usize>,
    pub(super) known: BTreeMap<usize, BTreeSet<AstSpan>>,
    pub(super) captured: BTreeMap<usize, BTreeSet<AstSpan>>,
    pub(super) member_writes: BTreeMap<usize, Arc<BTreeSet<AstSpan>>>,
    pub(super) captured_members: BTreeMap<usize, Arc<BTreeSet<AstSpan>>>,
    pub(super) mapped: BTreeMap<usize, Arc<BTreeSet<AstSpan>>>,
    opaque_rebinds: BTreeSet<usize>,
    pub(super) direct_eval: bool,
    pub(super) direct_define_property_write: bool,
    pub(super) proof: proof::Proof,
    pub(super) outer_bindings: BTreeMap<usize, BTreeSet<usize>>,
    pub(super) refused_file: bool,
    pub(super) member_aliases: BTreeMap<usize, Arc<Vec<usize>>>,
    pub(super) inline: BTreeMap<AstSpan, AstSpan>,
    writes: BTreeMap<(usize, usize), Write>,
    inline_ids: BTreeMap<usize, usize>,
}

pub(super) struct Write {
    pub(super) binding: usize,
    pub(super) site: AstSpan,
    pub(super) member: bool,
    pub(super) owner: usize,
    deferred: bool,
}

#[derive(Clone, Copy)]
struct Position<'a> {
    context: Context<'a>,
    strict: bool,
    deferred: bool,
    arguments_owner: Option<Node<'a>>,
}

struct Census<'a, 'b> {
    file: &'a ParsedFile,
    lexical: &'b Lexical<'a>,
    effects: Effects,
    counts: BTreeMap<usize, (Node<'a>, usize)>,
    aliases: BTreeMap<usize, BTreeSet<usize>>,
    mapped_owners: BTreeMap<usize, Node<'a>>,
    argument_stores: BTreeMap<usize, BTreeSet<AstSpan>>,
    owners: Vec<(usize, usize)>,
    proof: CensusProof<'a>,
}

impl Effects {
    pub(super) fn build<'a>(file: &'a ParsedFile, lexical: &Lexical<'a>) -> Self {
        let root = file.tree.root_node();
        let mut census = Census {
            file,
            lexical,
            effects: Self::default(),
            counts: BTreeMap::new(),
            aliases: BTreeMap::new(),
            mapped_owners: BTreeMap::new(),
            argument_stores: BTreeMap::new(),
            owners: Vec::new(),
            proof: CensusProof::default(),
        };
        for scope in lexical.scopes.values() {
            for (binding, site) in scope
                .entry
                .iter()
                .map(|site| (site.start_byte, site))
                .chain(
                    scope
                        .parameter_copies
                        .iter()
                        .map(|(binding, site)| (*binding, site)),
                )
            {
                census
                    .effects
                    .known
                    .entry(binding)
                    .or_default()
                    .insert(site.clone());
            }
        }
        let strict = file.path.ends_with(".mjs") || lexical.module_syntax;
        census.walk(
            root,
            Position {
                context: Context {
                    owner: root,
                    scope: root,
                },
                strict,
                deferred: false,
                arguments_owner: None,
            },
        );
        census.finish()
    }

    pub(super) fn writes_in(&self, region: Node<'_>) -> impl Iterator<Item = &Write> {
        self.writes
            .range((region.start_byte(), 0)..(region.end_byte(), 0))
            .map(|(_, write)| write)
    }

    pub(super) fn rebound_between(
        &self,
        binding: usize,
        start: usize,
        end: usize,
        pending_stores: &[AstSpan],
    ) -> bool {
        self.opaque_rebinds.range(start..end).next().is_some()
            || self.known.get(&binding).is_some_and(|sites| {
                sites
                    .range(
                        AstSpan {
                            start_byte: start,
                            end_byte: start,
                        }..,
                    )
                    .take_while(|site| site.start_byte < end)
                    .any(|site| !pending_stores.contains(site))
            })
    }

    pub(super) fn runs_with(&self, mut owner: usize, binding_owner: usize) -> bool {
        while owner != binding_owner {
            let Some(parent) = self.inline_ids.get(&owner) else {
                return false;
            };
            owner = *parent;
        }
        true
    }
}

impl<'a> Census<'a, '_> {
    fn finish(mut self) -> Effects {
        self.finish_proof();
        // A nested callable may need an outer value through several owners.
        // Charge propagation per reference before materializing the next set.
        let mut reference_work = 0usize;
        for (owner, parent) in self.owners.into_iter().rev() {
            let Some(refs) = self.effects.outer_bindings.remove(&owner) else {
                continue;
            };
            reference_work += refs.len();
            if reference_work > 1_000_000 {
                self.effects.refused_file = true;
                self.effects.outer_bindings.clear();
                break;
            }
            let outer = self.effects.outer_bindings.entry(parent).or_default();
            outer.extend(
                refs.iter()
                    .copied()
                    .filter(|r| self.lexical.binding_owner_ids[r] != parent),
            );
            self.effects.outer_bindings.insert(owner, refs);
        }
        for (owner, count) in self.counts.into_values() {
            if count > MAX_SUBSCRIPT_WRITES {
                eprintln!(
                    "warning: {}:{}: skipped prototype-write analysis for owner with {count} subscript writes",
                    self.file.path, owner.start_position().row + 1
                );
                self.effects.skipped.insert(owner.id());
            }
        }
        for owner in self.mapped_owners.into_values() {
            let sites = Arc::new(self.argument_stores.remove(&owner.id()).unwrap_or_default());
            for parameter in parameters(owner) {
                let parameter =
                    simple_parameter(parameter).expect("mapped owners have simple parameters");
                if let Some(binding) = self.lexical.binding(owner, self.file.node_text(&parameter))
                {
                    self.effects
                        .mapped
                        .insert(binding.start_byte, sites.clone());
                }
            }
        }
        // A deferred store through a possible alias can change any of the related
        // objects, including when the writer declares its alias inside the closure.
        let mut stores = BTreeMap::<_, Vec<_>>::new();
        for write in self.effects.writes.values().filter(|write| write.member) {
            stores.entry(write.binding).or_default().push(write);
        }
        let mut visited = BTreeSet::new();
        for &binding in stores.keys() {
            if !visited.insert(binding) {
                continue;
            }
            let mut component = Vec::new();
            let mut writers = Vec::new();
            let mut owners = BTreeSet::new();
            let mut queue = VecDeque::from([binding]);
            while let Some(alias) = queue.pop_front() {
                component.push(alias);
                owners.insert(self.lexical.binding_owner_ids[&alias]);
                writers.extend(stores.get(&alias).into_iter().flatten().copied());
                if let Some(edges) = self.aliases.get(&alias) {
                    queue.extend(edges.iter().copied().filter(|edge| visited.insert(*edge)));
                }
            }
            let all_sites = Arc::new(writers.iter().map(|write| write.site.clone()).collect());
            let sites: BTreeSet<_> = writers
                .into_iter()
                .filter(|write| {
                    // An inline chain can join a single binding owner. A component
                    // spanning several owners conservatively shares member effects.
                    write.deferred
                        || owners.len() > 1
                        || owners
                            .first()
                            .is_some_and(|owner| !self.effects.runs_with(write.owner, *owner))
                })
                .map(|write| write.site.clone())
                .collect();
            let sites = Arc::new(sites);
            let component = Arc::new(component);
            for &alias in component.iter() {
                self.effects
                    .member_writes
                    .insert(alias, Arc::clone(&all_sites));
                if !sites.is_empty() {
                    self.effects.captured_members.insert(alias, sites.clone());
                }
                if component.len() > 1 {
                    self.effects.member_aliases.insert(alias, component.clone());
                }
            }
        }
        self.effects
    }

    fn walk(&mut self, root: Node<'a>, position: Position<'a>) {
        let mut pending = vec![(root, position)];
        while let Some((node, parent)) = pending.pop() {
            if !self.census_charge(1) {
                break;
            }
            self.getter(node);
            let mut pos = parent;
            if is_scope(node) {
                pos.context.scope = node;
            }
            if is_owner(node) {
                if node != parent.context.owner {
                    self.owners.push((node.id(), parent.context.owner.id()));
                }
                if inline_iteration(self.file, node).is_some() && !pos.deferred {
                    self.effects
                        .inline_ids
                        .insert(node.id(), parent.context.owner.id());
                    self.effects
                        .inline
                        .insert(node.into(), parent.context.owner.into());
                }
                pos.context.owner = node;
                pos.deferred = false;
                let body = field(node, "body").unwrap_or(node);
                pos.strict |= strict_directive(self.file, body);
                if !matches!(node.kind(), "arrow_function" | "program") {
                    pos.arguments_owner = (!pos.strict
                        && parameters(node).all(|param| {
                            simple_parameter(param).is_some_and(|name| {
                                !special_name(self.file.node_text(&name), "arguments")
                            })
                        }))
                    .then_some(node);
                }
                for parameter in parameters(node) {
                    self.external_pattern(parameter, pos);
                    self.record(parameter, None, pos);
                }
                pending.extend(eager_owner_inputs(node).map(|input| (input, parent)));
            }
            if matches!(
                node.kind(),
                "class" | "class_declaration" | "abstract_class_declaration"
            ) {
                pos.strict = true;
            }
            match node.kind() {
                "variable_declarator" if !super::bare_var(node) => {
                    if let Some(target) = field(node, "name") {
                        self.record(target, field(node, "value"), pos);
                    }
                }
                "class" | "class_declaration" | "abstract_class_declaration" => {
                    if let Some(name) = field(node, "name") {
                        self.effects
                            .known
                            .entry(name.start_byte())
                            .or_default()
                            .insert(name.into());
                    }
                }
                "assignment_expression" | "augmented_assignment_expression" => {
                    if let Some(target) = field(node, "left") {
                        self.record(target, field(node, "right"), pos);
                    }
                }
                "for_in_statement" => {
                    if let Some(target) = field(node, "left") {
                        self.record(target, None, pos);
                    }
                }
                "update_expression" => {
                    if let Some(target) = field(node, "argument") {
                        self.record(target, None, pos);
                    }
                }
                "unary_expression" if has_token(node, "delete") => {
                    if let Some(target) = field(node, "argument") {
                        self.record(target, None, pos);
                    }
                }
                "catch_clause" => {
                    if let Some(target) = field(node, "parameter") {
                        self.record(target, None, pos);
                    }
                }
                "call_expression" | "new_expression" => {
                    if field(node, "function").is_some_and(|callee| {
                        special_name(self.file.node_text(&peel(callee)), "eval")
                    }) && field(node, "optional_chain").is_none()
                        && !has_token(node, "?.")
                        && self.lexical.binding(pos.context.scope, "eval").is_none()
                    {
                        self.effects.direct_eval = true;
                        self.effects.opaque_rebinds.insert(node.start_byte());
                    }
                    if node.kind() == "call_expression"
                        && field(node, "function").is_some_and(|callee| {
                            let callee = peel(callee);
                            identifier(callee)
                                && self.lexical.canonical(self.file.node_text(&callee)) == "eval"
                        })
                        && field(node, "optional_chain").is_none()
                        && !has_token(node, "?.")
                    {
                        self.proof.evals.push((node, pos.context.scope));
                    }
                    self.opaque_arguments(node, pos);
                    for target in mutation_inputs(self.file, self.lexical, node, pos.context) {
                        self.record_member(target, node, pos);
                    }
                }
                _ if identifier(node) => {
                    let name = self.file.node_text(&node);
                    if special_name(name, "arguments") {
                        if let Some(owner) = self.implicit_arguments(pos) {
                            self.mapped_owners.insert(owner.id(), owner);
                        }
                    }
                    if let Some(binding) = flatten_lookup(self.lexical, pos.context.scope, name) {
                        if self.lexical.binding_owner_ids[&binding] != pos.context.owner.id() {
                            self.effects
                                .outer_bindings
                                .entry(pos.context.owner.id())
                                .or_default()
                                .insert(binding);
                        }
                    }
                }
                _ => {}
            }
            let deferred = instance_initializer(node);
            for child in children(node) {
                if parameter_decorator(child, node)
                    || (is_owner(node)
                        && matches!(child.kind(), "decorator" | "computed_property_name"))
                {
                    continue;
                }
                let outer = node.kind() == "with_statement" && field(node, "object") == Some(child);
                let mut child_pos = if outer { parent } else { pos };
                child_pos.deferred |= deferred == Some(child);
                pending.push((child, child_pos));
            }
        }
    }

    fn record(&mut self, target: Node<'a>, source: Option<Node<'a>>, pos: Position<'a>) {
        // A destructuring RHS is shared by every target. Build a single
        // conservative root list, or refuse before multiplying alias edges.
        let roots = source.map_or_else(Vec::new, |source| self.receiver_roots(source));
        let external = source.is_some_and(|source| {
            !matches!(
                peel(source).kind(),
                "object"
                    | "array"
                    | "number"
                    | "string"
                    | "template_string"
                    | "true"
                    | "false"
                    | "null"
                    | "undefined"
            ) && !identifier(peel(source))
        });
        let mut pending = vec![target];
        while let Some(target) = pending.pop().map(peel) {
            if !self.census_charge(1) {
                return;
            }
            if matches!(target.kind(), "member_expression" | "subscript_expression") {
                if target.kind() == "subscript_expression" {
                    self.counts
                        .entry(pos.context.owner.id())
                        .or_insert((pos.context.owner, 0))
                        .1 += 1;
                }
                self.record_member(target, target, pos);
            } else if identifier(target) {
                if !self.census_charge(roots.len()) {
                    return;
                }
                self.record_binding(target, &roots, external, pos);
            } else {
                pending.extend(super::pattern_parts(target));
            }
        }
    }

    fn record_binding(
        &mut self,
        target: Node<'a>,
        roots: &[Node<'a>],
        external: bool,
        pos: Position<'a>,
    ) {
        let name = self.file.node_text(&target);
        if self.lexical.implicit_arguments(pos.context.scope, name) {
            return;
        }
        let Some(binding) = flatten_lookup(self.lexical, pos.context.scope, name) else {
            return;
        };
        let site = AstSpan::from(target);
        let binding_owner = self.lexical.binding_owner_ids[&binding];
        self.effects
            .known
            .entry(binding)
            .or_default()
            .insert(site.clone());
        if external {
            self.note_external(binding, site.start_byte);
        }
        if pos.deferred
            || !self
                .effects
                .runs_with(pos.context.owner.id(), binding_owner)
            || self.lexical.binding(pos.context.scope, name).is_none()
        {
            self.effects
                .captured
                .entry(binding)
                .or_default()
                .insert(site.clone());
        }
        for base in roots {
            if let Some(alias) =
                flatten_lookup(self.lexical, pos.context.scope, self.file.node_text(base))
            {
                self.aliases.entry(alias).or_default().insert(binding);
                self.aliases.entry(binding).or_default().insert(alias);
            }
        }
        self.effects.writes.insert(
            (target.start_byte(), binding),
            Write {
                binding,
                site,
                member: false,
                owner: pos.context.owner.id(),
                deferred: false,
            },
        );
    }

    fn implicit_arguments(&self, pos: Position<'a>) -> Option<Node<'a>> {
        let owner = pos.arguments_owner?;
        let mut scope = pos.context.scope.id();
        while scope != owner.id() {
            let current = &self.lexical.scopes[&scope];
            if current.names.contains_key("arguments") {
                return None;
            }
            scope = current.parent?;
        }
        Some(owner)
    }

    fn record_member(&mut self, target: Node<'a>, site: Node<'a>, pos: Position<'a>) {
        if target == site
            && field(target, "object").map(peel).is_some_and(|base| {
                identifier(base)
                    && special_name(self.file.node_text(&base), "Object")
                    && self.lexical.lookup(pos.context.scope, "Object").is_none()
            })
            && (field(target, "property")
                .is_some_and(|key| special_name(self.file.node_text(&key), "defineProperty"))
                || field(target, "index").map(peel).is_some_and(|key| {
                    matches!(
                        self.file.node_text(&key),
                        "'defineProperty'" | "\"defineProperty\""
                    )
                }))
        {
            self.effects.direct_define_property_write = true;
        }
        if let Some(owner) = pos.arguments_owner.filter(|_| target == site) {
            self.argument_stores
                .entry(owner.id())
                .or_default()
                .insert(site.into());
        }
        let receiver = if target == site {
            field(target, "object").unwrap_or(target)
        } else {
            target
        };
        for base in self.receiver_roots(receiver) {
            let name = self.file.node_text(&base);
            let site = AstSpan::from(site);
            if special_name(name, "arguments") {
                if let Some(owner) = self.implicit_arguments(pos) {
                    self.argument_stores
                        .entry(owner.id())
                        .or_default()
                        .insert(site);
                    continue;
                }
            }
            let Some(binding) = flatten_lookup(self.lexical, pos.context.scope, name) else {
                continue;
            };
            let owner = pos.context.owner.id();
            let deferred = pos.deferred || self.lexical.binding(pos.context.scope, name).is_none();
            self.effects.writes.insert(
                (site.start_byte, binding),
                Write {
                    binding,
                    site,
                    member: true,
                    owner,
                    deferred,
                },
            );
        }
    }
}

fn flatten_lookup(lexical: &Lexical<'_>, scope: Node<'_>, name: &str) -> Option<usize> {
    lexical
        .lookup_inner(scope, name, true)?
        .as_ref()
        .map(|site| site.start_byte)
}

fn simple_parameter(parameter: Node<'_>) -> Option<Node<'_>> {
    let value = if matches!(
        parameter.kind(),
        "optional_parameter" | "required_parameter"
    ) {
        if field(parameter, "value").is_some() {
            return None;
        }
        field(parameter, "pattern").or_else(|| field(parameter, "name"))?
    } else {
        parameter
    };
    identifier(value).then_some(value)
}

fn strict_directive(file: &ParsedFile, body: Node<'_>) -> bool {
    children(body)
        .map_while(|node| {
            (node.kind() == "expression_statement")
                .then(|| children(node).next())
                .flatten()
                .filter(|value| value.kind() == "string")
        })
        .any(|literal| matches!(file.node_text(&literal), "\"use strict\"" | "'use strict'"))
}

fn special_name(text: &str, expected: &str) -> bool {
    text == expected
        || (text.contains('\\')
            && ParsedFile::js_ts_decoded_identifier(text).as_deref() == Some(expected))
}

pub(super) fn mutation_inputs<'a>(
    file: &ParsedFile,
    lexical: &Lexical<'a>,
    call: Node<'a>,
    context: Context<'a>,
) -> impl Iterator<Item = Node<'a>> {
    let CallEffect {
        receiver,
        unknown,
        explicit_mutator,
        method_mutator,
        ..
    } = call_effect(file, lexical, call, context);
    let args = field(call, "arguments").filter(|args| args.kind() == "arguments");
    let mut arguments = args.into_iter().flat_map(children);
    let first = explicit_mutator.then(|| arguments.next()).flatten();
    receiver
        .filter(|_| unknown || method_mutator)
        .into_iter()
        .chain(first)
        .chain(arguments.filter(move |arg| unknown && identifier(peel(*arg))))
}

struct CallEffect<'a> {
    receiver: Option<Node<'a>>,
    unknown: bool,
    explicit_mutator: bool,
    method_mutator: bool,
    pure: bool,
}

fn call_effect<'a>(
    file: &ParsedFile,
    lexical: &Lexical<'a>,
    call: Node<'a>,
    context: Context<'a>,
) -> CallEffect<'a> {
    let callee = field(call, "function")
        .or_else(|| field(call, "constructor"))
        .map(peel);
    let receiver = callee.and_then(|callee| field(callee, "object")).map(peel);
    let builtin = receiver.zip(callee).and_then(|(receiver, callee)| {
        let name = file.node_text(&receiver);
        let method = file.node_text(&field(callee, "property")?);
        lexical
            .lookup(context.scope, name)
            .is_none()
            .then_some((name, method))
    });
    let explicit_mutator = matches!(
        builtin,
        Some(
            (
                "Object",
                "assign" | "defineProperty" | "defineProperties" | "setPrototypeOf"
            ) | (
                "Reflect",
                "set" | "defineProperty" | "deleteProperty" | "setPrototypeOf"
            )
        )
    );
    let method_mutator = callee.is_some_and(|callee| super::member_mutator(file, callee));
    let known_read = matches!(
        builtin,
        Some(("Object", "keys" | "values" | "entries") | ("Reflect", "get") | ("Array", "isArray"))
    );
    let iteration = super::iteration(file, call).is_some();
    let unknown = !explicit_mutator && !method_mutator && !known_read && !iteration;
    CallEffect {
        receiver,
        unknown,
        explicit_mutator,
        method_mutator,
        pure: builtin == Some(("Array", "isArray")),
    }
}
