//! Nonrecursive receiver selection and the operands contributing to it.
use super::name_reads::{Analysis, Origin};
use super::{
    children, field, identifier, is_inline_callback, is_owner, iteration, operator, pattern_parts,
    peel, AstSpan, GuardState, KeyOperand, KeyRole, ProtoWrite, ProtoWriteClass,
};
use crate::ast::ParsedFile;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

const MAX_SUBSCRIPT_WRITES: usize = 10_000;

type Selectors<'a> = BTreeMap<usize, Node<'a>>;

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
            let (_, members) = targets.entry(owner.id()).or_insert((owner, Vec::new()));
            collect_targets(target, node, members);
        }
        work.extend(children(node).map(|child| (child, owner)));
    }
    let mut classifier = Classifier { file, analysis };
    let mut result = Vec::new();
    for (owner, members) in targets.into_values() {
        let count = members
            .iter()
            .filter(|(target, _)| target.kind() == "subscript_expression")
            .count();
        if count > MAX_SUBSCRIPT_WRITES {
            eprintln!(
                "warning: {}:{}: skipped prototype-write classification for owner with {count} subscript writes",
                file.path, owner.start_position().row + 1
            );
            continue;
        }
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
}

impl<'a> Classifier<'a, '_> {
    fn classify(
        &mut self,
        owner: Node<'a>,
        target: Node<'a>,
        store: Node<'a>,
    ) -> Option<ProtoWrite> {
        let object = field(target, "object")?;
        let mut seen = BTreeSet::new();
        let mut relation = self.receiver(object, &mut seen, None);
        let key = field(target, "index");
        if let Some(key) = key.filter(|_| self.advances(store, target)) {
            self.add_key(key, &mut relation);
        }
        if relation.selectors.is_empty() {
            return None;
        }

        let mut operands = BTreeMap::new();
        for node in relation.selectors.into_values() {
            self.key_reads(node, KeyRole::Selector, &mut operands, &mut BTreeSet::new());
        }
        if let Some(key) = key {
            self.key_reads(key, KeyRole::Property, &mut operands, &mut BTreeSet::new());
        }
        if !operands
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
        seen: &mut BTreeSet<usize>,
        accumulator: Option<usize>,
    ) -> Relation<'a> {
        let value = peel(expression);
        let mut relation = Relation::default();
        match value.kind() {
            "identifier" => {
                if let Some(read) = self
                    .analysis
                    .read(value)
                    .filter(|read| self.local_receiver(read))
                {
                    for site in &read.reaching_versions {
                        if !seen.insert(site.start_byte) {
                            continue;
                        }
                        if let Some(default) = self.analysis.defaults.get(&site.start_byte) {
                            relation.extend(self.receiver(*default, seen, accumulator));
                        }
                        match self.origin(site) {
                            Origin::Value(source) => {
                                relation.extend(self.receiver(source, seen, accumulator))
                            }
                            Origin::Child(source, key) => {
                                if self.from_accumulator(source, accumulator, &mut BTreeSet::new())
                                {
                                    relation.extend(self.receiver(source, seen, accumulator));
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
                            relation = self.receiver(object, seen, accumulator);
                            relation.nested |= self.add_key(key, &mut relation);
                        } else if self.shared_link(value) {
                            relation = self.receiver(object, seen, accumulator);
                        }
                    }
                }
            }
            "assignment_expression" => {
                if let Some(right) = field(value, "right") {
                    relation = self.receiver(right, seen, accumulator);
                }
            }
            "augmented_assignment_expression" => {
                let op = operator(value, self.file);
                if matches!(op, "||=" | "??=" | "&&=") {
                    if op != "&&=" {
                        if let Some(left) = field(value, "left") {
                            relation.extend(self.receiver(left, seen, accumulator));
                        }
                    }
                    if let Some(right) = field(value, "right") {
                        relation.extend(self.receiver(right, seen, accumulator));
                    }
                }
            }
            "ternary_expression" => {
                for arm in ["consequence", "alternative"]
                    .into_iter()
                    .filter_map(|arm| field(value, arm))
                {
                    relation.extend(self.receiver(arm, seen, accumulator));
                }
            }
            "binary_expression" => {
                let op = operator(value, self.file);
                if matches!(op, "||" | "??" | "&&") {
                    if op != "&&" {
                        if let Some(left) = field(value, "left") {
                            relation.extend(self.receiver(left, seen, accumulator));
                        }
                    }
                    if let Some(right) = field(value, "right") {
                        relation.extend(self.receiver(right, seen, accumulator));
                    }
                }
            }
            "sequence_expression" => {
                if let Some(tail) = children(value).last() {
                    relation = self.receiver(tail, seen, accumulator);
                }
            }
            "call_expression" => {
                if let Some((object, key)) = self.reflect_get(value) {
                    if self.from_accumulator(object, accumulator, &mut BTreeSet::new()) {
                        relation = self.receiver(object, seen, accumulator);
                        self.add_key(key, &mut relation);
                    }
                } else if accumulator.is_none() {
                    relation = self.reduce(value, seen);
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
        let value = peel(value);
        if let Some(object) = field(value, "object") {
            return self.from_accumulator(object, Some(accumulator), seen);
        }
        self.analysis.read(value).is_some_and(|read| {
            read.reaching_versions.iter().any(|site| {
                if site.start_byte == accumulator {
                    return true;
                }
                if !seen.insert(site.start_byte) {
                    return false;
                }
                match self.origin(site) {
                    Origin::Value(value) | Origin::Child(value, _) => {
                        self.from_accumulator(value, Some(accumulator), seen)
                    }
                    _ => false,
                }
            })
        })
    }

    fn reduce(&self, call: Node<'a>, seen: &mut BTreeSet<usize>) -> Relation<'a> {
        let mut relation = Relation::default();
        let Some((_, "reduce")) = iteration(self.file, call) else {
            return relation;
        };
        let Some(args) = field(call, "arguments") else {
            return relation;
        };
        let mut args = children(args);
        let (Some(callback), Some(initial), None) =
            (args.next().map(peel), args.next(), args.next())
        else {
            return relation;
        };
        if !is_inline_callback(callback) {
            return relation;
        }
        let Some(param) = field(callback, "parameters")
            .and_then(|params| children(params).next())
            .or_else(|| field(callback, "parameter"))
            .map(|param| pattern_parts(param).into_iter().next().unwrap_or(param))
            .filter(|param| identifier(*param))
        else {
            return relation;
        };
        let Some(body) = field(callback, "body") else {
            return relation;
        };
        let mut returned = if body.kind() == "statement_block" {
            Vec::new()
        } else {
            vec![body]
        };
        if body.kind() == "statement_block" {
            let mut todo = vec![body];
            while let Some(node) = todo.pop() {
                if node.kind() == "return_statement" {
                    returned.extend(children(node).take(1));
                } else if !is_owner(node) {
                    todo.extend(children(node));
                }
            }
        }
        for value in returned {
            relation.extend(self.receiver(value, seen, Some(param.start_byte())));
        }
        if !relation.selectors.is_empty() {
            relation.extend(self.receiver(initial, seen, None));
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

    fn advances(&self, store: Node<'a>, target: Node<'a>) -> bool {
        if field(store, "left").map(peel) != Some(target) {
            return false;
        }
        let mut enclosing = store;
        while let Some(parent) = enclosing.parent() {
            match parent.kind() {
                "parenthesized_expression"
                | "non_null_expression"
                | "as_expression"
                | "satisfies_expression" => enclosing = parent,
                "assignment_expression" => {
                    if field(parent, "right") != Some(enclosing)
                        || !field(parent, "left").is_some_and(|left| identifier(peel(left)))
                    {
                        return false;
                    }
                    return self.retains_slot(store, target);
                }
                "variable_declarator" if field(parent, "value") == Some(enclosing) => {
                    return self.retains_slot(store, target);
                }
                "return_statement" | "arrow_function" => {
                    let mut current = Some(parent);
                    while let Some(node) = current {
                        if is_owner(node) {
                            let mut wrapper = node;
                            while wrapper
                                .parent()
                                .is_some_and(|n| n.kind() == "parenthesized_expression")
                            {
                                wrapper = wrapper.parent().unwrap();
                            }
                            return is_inline_callback(node)
                                && wrapper
                                    .parent()
                                    .filter(|args| children(*args).next() == Some(wrapper))
                                    .and_then(|args| args.parent())
                                    .and_then(|call| iteration(self.file, call))
                                    .is_some_and(|(_, method)| method == "reduce")
                                && self.retains_slot(store, target);
                        }
                        current = node.parent();
                    }
                    return false;
                }
                _ => return false,
            }
        }
        false
    }

    fn retains_slot(&self, store: Node<'a>, target: Node<'a>) -> bool {
        if matches!(operator(store, self.file), "||=" | "??=") {
            // The branch retaining the selected child does not perform the store.
            return false;
        }
        let mut values: Vec<_> = field(store, "right").into_iter().collect();
        while let Some(value) = values.pop() {
            let value = peel(value);
            match value.kind() {
                "binary_expression" if matches!(operator(value, self.file), "||" | "??" | "&&") => {
                    if operator(value, self.file) != "&&" {
                        values.extend(field(value, "left"));
                    }
                    values.extend(field(value, "right"));
                }
                "ternary_expression" => {
                    values.extend(field(value, "consequence"));
                    values.extend(field(value, "alternative"));
                }
                "subscript_expression" if self.same_slot(target, value) => return true,
                _ => {}
            }
        }
        false
    }

    fn same_slot(&self, write: Node<'a>, read: Node<'a>) -> bool {
        ["object", "index"].into_iter().all(|part| {
            let Some(left) = field(write, part).map(peel) else {
                return false;
            };
            let Some(right) = field(read, part).map(peel) else {
                return false;
            };
            if self.file.node_text(&left) != self.file.node_text(&right) {
                return false;
            }
            match (self.analysis.read(left), self.analysis.read(right)) {
                (Some(a), Some(b)) => {
                    a.binding.is_some()
                        && a.binding == b.binding
                        && a.reaching_versions == b.reaching_versions
                        && !a.reaching_versions.is_empty()
                        && a.resolution_complete
                        && b.resolution_complete
                }
                _ => self.fixed_key(left).is_some(),
            }
        })
    }

    fn key_reads(
        &self,
        node: Node<'a>,
        role: KeyRole,
        out: &mut BTreeMap<AstSpan, KeyOperand>,
        seen: &mut BTreeSet<usize>,
    ) {
        let node = peel(node);
        if self.fixed_key(node).is_some() {
            return;
        }
        if let Some(read) = self.analysis.read(node) {
            out.entry(read.span.clone()).or_insert_with(|| KeyOperand {
                name: read.path.clone(),
                base: read.base.clone(),
                role,
                owner: read.owner.clone(),
                binding: read.binding.clone(),
                reaching_versions: read.reaching_versions.clone(),
                line: node.start_position().row + 1,
                start_byte: read.span.start_byte,
                end_byte: read.span.end_byte,
            });
            if role == KeyRole::Selector
                && identifier(node)
                && read.resolution_complete
                && read.reaching_versions.len() == 1
            {
                for site in &read.reaching_versions {
                    if !seen.insert(site.start_byte) {
                        continue;
                    }
                    match self.origin(site) {
                        Origin::Value(source) | Origin::Element(source) => {
                            self.key_source(source, role, out, seen)
                        }
                        Origin::Child(source, _) => self.key_source(source, role, out, seen),
                        Origin::Unknown => {}
                    }
                }
            }
            return;
        }
        match node.kind() {
            "call_expression" | "new_expression" => {
                if let Some(args) = field(node, "arguments") {
                    for arg in children(args) {
                        self.key_reads(arg, role, out, seen);
                    }
                }
            }
            "ternary_expression" => {
                for arm in ["consequence", "alternative"]
                    .into_iter()
                    .filter_map(|arm| field(node, arm))
                {
                    self.key_reads(arm, role, out, seen);
                }
            }
            "sequence_expression" => {
                if let Some(tail) = children(node).last() {
                    self.key_reads(tail, role, out, seen);
                }
            }
            "assignment_expression" | "augmented_assignment_expression" => {
                if let Some(right) = field(node, "right") {
                    self.key_reads(right, role, out, seen);
                }
            }
            "binary_expression"
            | "template_string"
            | "template_substitution"
            | "unary_expression"
            | "spread_element"
            | "await_expression" => {
                for child in children(node) {
                    self.key_reads(child, role, out, seen);
                }
            }
            _ => {}
        }
    }

    fn key_source(
        &self,
        source: Node<'a>,
        role: KeyRole,
        out: &mut BTreeMap<AstSpan, KeyOperand>,
        seen: &mut BTreeSet<usize>,
    ) {
        let source = peel(source);
        match source.kind() {
            "identifier" | "member_expression" | "subscript_expression" | "ternary_expression" => {
                self.key_reads(source, role, out, seen);
            }
            "call_expression" => {
                if let Some(callee) = field(source, "function") {
                    if let Some(method) = field(callee, "property").filter(|method| {
                        matches!(
                            self.file.node_text(method),
                            "shift" | "pop" | "slice" | "split"
                        )
                    }) {
                        if let Some(object) = field(callee, "object") {
                            if self.analysis.read(callee).is_some()
                                && matches!(self.file.node_text(&method), "shift" | "pop")
                            {
                                self.key_reads(callee, role, out, seen);
                            } else {
                                self.key_source(object, role, out, seen);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
