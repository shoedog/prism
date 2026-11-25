//! Nonrecursive receiver selection and the operands contributing to it.
use super::name_reads::{Analysis, Origin};
use super::{
    children, field, identifier, inline_iteration, is_inline_callback, is_owner, iteration,
    operator, pattern_parts, peel, AstSpan, GuardState, KeyRole, ProtoWrite, ProtoWriteClass,
};
use crate::ast::ParsedFile;
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};
use tree_sitter::Node;

mod operands;
mod store;
const TARGET_WORK_LIMIT: usize = 16_384;
const FILE_WORK_LIMIT: usize = 2_000_000;
const RECEIVER_DEPTH_LIMIT: usize = 128;

type Selectors<'a> = BTreeMap<usize, Node<'a>>;
type ReceiverKey = (usize, Option<usize>);

#[derive(Clone, Default)]
struct Relation<'a> {
    nested: bool,
    selectors: Selectors<'a>,
}

impl<'a> Relation<'a> {
    fn extend(&mut self, other: Self) {
        self.nested |= other.nested;
        self.selectors.extend(other.selectors);
    }
}

pub(super) fn facts<'a>(file: &'a ParsedFile, analysis: &Analysis<'a>) -> Vec<ProtoWrite> {
    let root = file.tree.root_node();
    let mut work = vec![(root, root)];
    let mut targets = BTreeMap::new();
    while let Some((node, previous_owner)) = work.pop() {
        let owner = if is_owner(node) { node } else { previous_owner };
        let target = match node.kind() {
            "assignment_expression" | "augmented_assignment_expression" => field(node, "left"),
            "update_expression" => field(node, "argument"),
            "for_in_statement" if field(node, "kind").is_none() => field(node, "left"),
            _ => None,
        };
        if let Some(target) = target {
            let (_, members) = targets
                .entry((owner.start_byte(), owner.end_byte()))
                .or_insert((owner, Vec::new()));
            collect_targets(target, node, members);
        }
        if is_owner(node) {
            work.extend(super::eager_owner_inputs(node).map(|child| (child, previous_owner)));
        }
        work.extend(
            children(node)
                .filter(|child| {
                    !super::parameter_decorator(*child, node)
                        && !(is_owner(node)
                            && matches!(child.kind(), "decorator" | "computed_property_name"))
                        && child.kind() != "implements_clause"
                })
                .map(|child| (child, owner)),
        );
    }
    let mut classifier = Classifier {
        file,
        analysis,
        receivers: RefCell::default(),
        file_work: Cell::new(0),
        target_work: Cell::new(0),
    };
    let mut result = Vec::new();
    for (owner, mut members) in targets.into_values() {
        if analysis.skipped(owner) {
            continue;
        }
        members.sort_by_key(|(target, _)| target.start_byte());
        result.extend(
            members
                .into_iter()
                .filter_map(|(target, store)| classifier.classify(owner, target, store)),
        );
    }
    result.sort_by_key(|write| (write.line, write.start_byte));
    result
}

fn collect_targets<'a>(target: Node<'a>, store: Node<'a>, out: &mut Vec<(Node<'a>, Node<'a>)>) {
    let target = peel(target);
    if matches!(target.kind(), "member_expression" | "subscript_expression") {
        out.push((target, store));
    } else {
        for part in pattern_parts(target) {
            collect_targets(part, store, out);
        }
    }
}

struct Classifier<'a, 'b> {
    file: &'a ParsedFile,
    analysis: &'b Analysis<'a>,
    receivers: RefCell<BTreeMap<ReceiverKey, Rc<Relation<'a>>>>,
    file_work: Cell<usize>,
    target_work: Cell<usize>,
}

impl<'a> Classifier<'a, '_> {
    fn charge(&self, work: usize) -> bool {
        let target = self.target_work.get().saturating_add(work);
        self.target_work.set(target);
        if target > TARGET_WORK_LIMIT {
            return false;
        }
        let file = self.file_work.get().saturating_add(work);
        self.file_work.set(file);
        file <= FILE_WORK_LIMIT
    }

    fn classify(
        &mut self,
        owner: Node<'a>,
        target: Node<'a>,
        store: Node<'a>,
    ) -> Option<ProtoWrite> {
        self.target_work.set(0);
        if !self.charge(1) {
            return None;
        }
        let object = field(target, "object")?;
        let mut walk = ReceiverWalk::default();
        let mut relation = self.receiver(object, &mut walk, None);
        let key = field(target, "index");
        if let Some(key) = key.filter(|_| self.advances(store, target)) {
            self.add_key(key, &mut relation);
        }
        if relation.selectors.is_empty() {
            return None;
        }

        let mut operands = BTreeMap::new();
        for &node in relation.selectors.values() {
            self.key_reads(node, KeyRole::Selector, &mut operands, &mut BTreeSet::new());
        }
        if let Some(key) = key {
            self.key_reads(key, KeyRole::Property, &mut operands, &mut BTreeSet::new());
        }
        self.promote_reused_primitives(&relation.selectors, &mut operands);
        if !self.charge(0)
            || !operands
                .values()
                .any(|operand| operand.role == KeyRole::Selector)
        {
            return None;
        }
        let function = field(owner, "name")
            .or_else(|| {
                owner
                    .parent()
                    .filter(|node| node.kind() == "variable_declarator")
                    .and_then(|decl| field(decl, "name"))
            })
            .map(|node| self.file.node_text(&node))
            .unwrap_or(if owner.kind() == "program" {
                "<module>"
            } else {
                "<anonymous>"
            });
        Some(ProtoWrite {
            line: target.start_position().row + 1,
            end_line: target.end_position().row + 1,
            start_byte: target.start_byte(),
            end_byte: target.end_byte(),
            function: function.to_string(),
            owner: AstSpan::from(owner),
            function_start_line: owner.start_position().row + 1,
            function_end_line: owner.end_position().row + 1,
            class: if relation.nested {
                ProtoWriteClass::NestedKeys
            } else {
                ProtoWriteClass::PathWalk
            },
            key_operands: operands.into_values().collect(),
            guard: GuardState::None,
        })
    }

    fn origin(&self, site: &AstSpan) -> Origin<'a> {
        self.analysis
            .origins
            .get(&site.start_byte)
            .copied()
            .unwrap_or(Origin::Unknown)
    }

    fn local_receiver(&self, read: &super::NameRead) -> bool {
        let Some(binding) = &read.binding else {
            return false;
        };
        let Some(binding_owner) = self
            .analysis
            .lexical
            .binding_owners
            .get(&binding.start_byte)
        else {
            return false;
        };
        self.analysis.in_inline_scope(&read.owner, binding_owner)
    }

    fn receiver(
        &self,
        expression: Node<'a>,
        walk: &mut ReceiverWalk,
        accumulator: Option<usize>,
    ) -> Relation<'a> {
        let mut value = peel(expression);
        let mut aliases = Vec::new();
        let cuts = walk.cuts;
        let relation = loop {
            if !self.charge(1) {
                walk.cuts += 1;
                break Rc::new(Relation::default());
            }
            let key = (value.id(), accumulator);
            if let Some(relation) = self.receivers.borrow().get(&key) {
                break Rc::clone(relation);
            }
            if walk.depth == RECEIVER_DEPTH_LIMIT || !walk.active.insert(key) {
                walk.cuts += 1;
                break Rc::new(Relation::default());
            }
            // Long chains of single-definition aliases need no recursive stack.
            if let Some(source) = self.single_receiver_origin(value) {
                aliases.push(key);
                value = peel(source);
                continue;
            }
            walk.depth += 1;
            let relation = Rc::new(self.receiver_inner(value, walk, accumulator));
            walk.depth -= 1;
            walk.active.remove(&key);
            if walk.cuts == cuts {
                self.receivers
                    .borrow_mut()
                    .insert(key, Rc::clone(&relation));
            }
            break relation;
        };
        if !self.charge(relation.selectors.len()) {
            for key in aliases {
                walk.active.remove(&key);
            }
            walk.cuts += 1;
            return Relation::default();
        }
        if aliases.is_empty() {
            return (*relation).clone();
        }
        let relation = if relation.nested {
            Rc::new(Relation {
                nested: false,
                selectors: relation.selectors.clone(),
            })
        } else {
            relation
        };
        // Do not memoize a partial answer that depends on the active cycle.
        for key in aliases {
            walk.active.remove(&key);
            if walk.cuts == cuts {
                self.receivers
                    .borrow_mut()
                    .insert(key, Rc::clone(&relation));
            }
        }
        (*relation).clone()
    }

    fn single_receiver_origin(&self, value: Node<'a>) -> Option<Node<'a>> {
        if !identifier(value) {
            return None;
        }
        let read = self
            .analysis
            .read(value)
            .filter(|read| self.local_receiver(read))?;
        let [site] = read.reaching_versions.as_slice() else {
            return None;
        };
        if self.analysis.defaults.contains_key(&site.start_byte)
            || self.analysis.deferred_definition(read, site)
        {
            return None;
        }
        match self.origin(site) {
            Origin::Value(source) => Some(source),
            _ => None,
        }
    }

    fn receiver_inner(
        &self,
        value: Node<'a>,
        walk: &mut ReceiverWalk,
        accumulator: Option<usize>,
    ) -> Relation<'a> {
        let mut relation = Relation::default();
        match value.kind() {
            "identifier" => {
                if let Some(read) = self
                    .analysis
                    .read(value)
                    .filter(|read| self.local_receiver(read))
                {
                    for site in &read.reaching_versions {
                        if !self.charge(1) {
                            break;
                        }
                        if self.analysis.deferred_definition(read, site) {
                            continue;
                        }
                        if let Some(default) = self.analysis.defaults.get(&site.start_byte) {
                            relation.extend(self.receiver(*default, walk, accumulator));
                        }
                        match self.origin(site) {
                            Origin::Value(source) => {
                                relation.extend(self.receiver(source, walk, accumulator))
                            }
                            Origin::Child(source, key) => {
                                if self.from_accumulator(source, accumulator, &mut BTreeSet::new())
                                {
                                    relation.extend(self.receiver(source, walk, accumulator));
                                    self.add_key(key, &mut relation);
                                }
                            }
                            _ => {}
                        }
                    }
                    relation.nested = false;
                }
            }
            "subscript_expression" | "member_expression" => {
                if let Some(object) = field(value, "object") {
                    if self.from_accumulator(object, accumulator, &mut BTreeSet::new()) {
                        if let Some(key) =
                            field(value, "index").filter(|key| self.fixed_key(*key).is_none())
                        {
                            relation = self.receiver(object, walk, accumulator);
                            relation.nested |= self.add_key(key, &mut relation);
                        } else if self.shared_link(value) {
                            relation = self.receiver(object, walk, accumulator);
                        }
                    }
                }
            }
            "assignment_expression" => {
                if let Some(right) = field(value, "right") {
                    relation = self.receiver(right, walk, accumulator);
                }
            }
            "augmented_assignment_expression" => {
                let op = operator(value, self.file);
                if matches!(op, "||=" | "??=" | "&&=") {
                    if op != "&&=" {
                        if let Some(left) = field(value, "left") {
                            relation.extend(self.receiver(left, walk, accumulator));
                        }
                    }
                    if let Some(right) = field(value, "right") {
                        relation.extend(self.receiver(right, walk, accumulator));
                    }
                }
            }
            "ternary_expression" => {
                for arm in ["consequence", "alternative"]
                    .into_iter()
                    .filter_map(|arm| field(value, arm))
                {
                    relation.extend(self.receiver(arm, walk, accumulator));
                }
            }
            "binary_expression" => {
                let op = operator(value, self.file);
                if matches!(op, "||" | "??" | "&&") {
                    if op != "&&" {
                        if let Some(left) = field(value, "left") {
                            relation.extend(self.receiver(left, walk, accumulator));
                        }
                    }
                    if let Some(right) = field(value, "right") {
                        relation.extend(self.receiver(right, walk, accumulator));
                    }
                }
            }
            "sequence_expression" => {
                if let Some(tail) = children(value).last() {
                    relation = self.receiver(tail, walk, accumulator);
                }
            }
            "call_expression" => {
                if let Some((object, key)) = self.reflect_get(value) {
                    if self.from_accumulator(object, accumulator, &mut BTreeSet::new()) {
                        relation = self.receiver(object, walk, accumulator);
                        self.add_key(key, &mut relation);
                    }
                } else if accumulator.is_none() {
                    relation = self.reduce(value, walk);
                }
                relation.nested = false;
            }
            _ => {}
        }
        relation
    }

    fn shared_link(&self, member: Node<'a>) -> bool {
        let fixed = field(member, "index")
            .and_then(|key| self.fixed_key(key))
            .or_else(|| field(member, "property").map(|property| self.file.node_text(&property)));
        fixed.is_some_and(|key| {
            matches!(
                key.trim_matches(['\'', '"', '`']),
                "__proto__" | "constructor" | "prototype"
            )
        })
    }

    fn fixed_key(&self, key: Node<'a>) -> Option<&'a str> {
        let key = peel(key);
        let literal = |value| match peel(value).kind() {
            "string" | "number" | "true" | "false" | "null" => {
                Some(self.file.node_text(&peel(value)))
            }
            "template_string"
                if !children(peel(value)).any(|child| child.kind() == "template_substitution") =>
            {
                Some(self.file.node_text(&peel(value)))
            }
            _ => None,
        };
        literal(key).or_else(|| {
            let read = self
                .analysis
                .read(key)
                .filter(|read| read.resolution_complete)?;
            if !identifier(key) {
                return None;
            }
            let [site] = read.reaching_versions.as_slice() else {
                return None;
            };
            match self.origin(site) {
                Origin::Value(value) => literal(value),
                _ => None,
            }
        })
    }

    fn add_key(&self, key: Node<'a>, relation: &mut Relation<'a>) -> bool {
        if self.fixed_key(key).is_some() {
            return false;
        }
        let key = peel(key);
        relation.selectors.insert(key.start_byte(), key);
        true
    }

    fn from_accumulator(
        &self,
        value: Node<'a>,
        accumulator: Option<usize>,
        seen: &mut BTreeSet<usize>,
    ) -> bool {
        let Some(accumulator) = accumulator else {
            return true;
        };
        if seen.len() >= RECEIVER_DEPTH_LIMIT || !self.charge(1) || !seen.insert(value.id()) {
            return false;
        }
        let value = peel(value);
        if let Some(object) = field(value, "object") {
            return self.from_accumulator(object, Some(accumulator), seen);
        }
        if seen.len() >= RECEIVER_DEPTH_LIMIT {
            return false;
        }
        match value.kind() {
            "ternary_expression" => ["consequence", "alternative"]
                .into_iter()
                .filter_map(|arm| field(value, arm))
                .any(|arm| self.from_accumulator(arm, Some(accumulator), seen)),
            "assignment_expression" => field(value, "right")
                .is_some_and(|right| self.from_accumulator(right, Some(accumulator), seen)),
            "binary_expression" if matches!(operator(value, self.file), "||" | "??" | "&&") => {
                ["left", "right"]
                    .into_iter()
                    .filter(|part| *part != "left" || operator(value, self.file) != "&&")
                    .filter_map(|part| field(value, part))
                    .any(|part| self.from_accumulator(part, Some(accumulator), seen))
            }
            "sequence_expression" => children(value)
                .last()
                .is_some_and(|tail| self.from_accumulator(tail, Some(accumulator), seen)),
            _ => self.analysis.read(value).is_some_and(|read| {
                read.reaching_versions.iter().any(|site| {
                    if site.start_byte == accumulator {
                        return true;
                    }
                    if !self.charge(1) || self.analysis.deferred_definition(read, site) {
                        return false;
                    }
                    match self.origin(site) {
                        Origin::Value(value) | Origin::Child(value, _) => {
                            self.from_accumulator(value, Some(accumulator), seen)
                        }
                        _ => false,
                    }
                })
            }),
        }
    }

    fn reducer(&self, call: Node<'a>) -> Option<(Node<'a>, Node<'a>, usize)> {
        let (_, "reduce") = iteration(self.file, call)? else {
            return None;
        };
        let mut args = children(field(call, "arguments")?);
        let callback = peel(args.next()?);
        let initial = args.next()?;
        if args.next().is_some()
            || !is_inline_callback(callback)
            || initial.kind() == "spread_element"
        {
            return None;
        }
        let param = super::parameters(callback).next()?;
        let param = if matches!(param.kind(), "required_parameter" | "optional_parameter") {
            pattern_parts(param).into_iter().next()?
        } else {
            param
        };
        identifier(param).then_some((callback, initial, param.start_byte()))
    }

    fn reduce(&self, call: Node<'a>, walk: &mut ReceiverWalk) -> Relation<'a> {
        let mut relation = Relation::default();
        let Some((callback, initial, accumulator)) = self.reducer(call) else {
            return relation;
        };
        if self.analysis.skipped(callback) {
            return relation;
        }
        let Some(body) = field(callback, "body") else {
            return relation;
        };
        if body.kind() != "statement_block" {
            relation.extend(self.receiver(body, walk, Some(accumulator)));
        } else {
            let mut todo = vec![body];
            while let Some(node) = todo.pop() {
                if !self.charge(1) {
                    return Relation::default();
                }
                if node.kind() == "return_statement" && self.analysis.reachable_return(node) {
                    if let Some(value) = children(node).next() {
                        relation.extend(self.receiver(value, walk, Some(accumulator)));
                    }
                } else if !is_owner(node) {
                    todo.extend(children(node));
                }
            }
        }
        if !relation.selectors.is_empty() {
            relation.extend(self.receiver(initial, walk, None));
        }
        relation
    }

    fn reflect_get(&self, call: Node<'a>) -> Option<(Node<'a>, Node<'a>)> {
        let callee = field(call, "function")?;
        let receiver = field(callee, "object").map(peel)?;
        if self.file.node_text(&receiver) != "Reflect"
            || self.file.node_text(&field(callee, "property")?) != "get"
            || self.analysis.has_binding(receiver)
        {
            return None;
        }
        let mut args = children(field(call, "arguments")?);
        match (args.next(), args.next(), args.next()) {
            (Some(object), Some(key), None)
                if object.kind() != "spread_element" && key.kind() != "spread_element" =>
            {
                Some((object, key))
            }
            _ => None,
        }
    }
}

#[derive(Default)]
struct ReceiverWalk {
    active: BTreeSet<ReceiverKey>,
    cuts: usize,
    depth: usize,
}
