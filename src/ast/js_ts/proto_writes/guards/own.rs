//! Property-key identity and closed own-slot selection.
use super::flow::{deferred_field, inside};
use super::*;

mod graph;

#[derive(Clone, Copy)]
struct Check<'a> {
    call: Node<'a>,
    namespace: Node<'a>,
    object: Node<'a>,
    key: Node<'a>,
    fallback: Option<Node<'a>>,
    legacy: bool,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Slot<'a> {
    String(&'a str),
    Symbol(&'a str),
}

impl<'a, F: FnMut(usize) -> bool> Proof<'a, '_, F> {
    pub(super) fn own_guard(&mut self, input: &GuardInput<'a>) -> (GuardState, Vec<GuardForm>) {
        let Some(first) = input.steps.first().copied() else {
            return (GuardState::None, Vec::new());
        };
        let edges = self.route(first.effect);
        let mut atoms = Vec::new();
        for edge in &edges {
            self.true_atoms(*edge, &mut atoms);
        }
        let mut checks: Vec<_> = atoms
            .into_iter()
            .filter_map(|atom| self.own_call(atom))
            .collect();
        checks.extend(self.adjacent_own_checks(first.effect));
        let mut partial = false;
        for check in checks {
            if !self.same_key(check.key, first.key) || !self.same_key(check.object, first.object) {
                self.refuse(check.call, "wrong_binding");
                continue;
            }
            partial = true;
            let path: &[&str] = if check.legacy {
                &["Object", "prototype", "hasOwnProperty"]
            } else {
                &["Object", "hasOwn"]
            };
            if !self.native(check.call, check.namespace, path)
                || check.legacy && !self.unwritten(check.call, &["Function", "prototype", "call"])
            {
                continue;
            }
            let is_string = self.checked_string(first.key, &edges);
            if !is_string && !self.reflect_key(first.key) {
                self.refuse(check.call, "unstable_key");
                continue;
            }
            if input.steps.iter().any(|step| step.shared_link) {
                self.refuse(check.call, "unproved_child");
                continue;
            }
            if let Some(form) = self.own_slots(input, first, check, is_string) {
                return (GuardState::Sufficient, vec![form]);
            }
            self.refuse(check.call, "unproved_child");
        }
        let state = if partial {
            GuardState::Partial
        } else {
            GuardState::None
        };
        (state, Vec::new())
    }

    fn own_call(&self, call: Node<'a>) -> Option<Check<'a>> {
        let (receiver, method) = self.method(call)?;
        let (namespace, legacy) = match method {
            "hasOwn" if identifier(receiver) && self.text(receiver) == "Object" => {
                (receiver, false)
            }
            "call"
                if field(receiver, "property")
                    .is_some_and(|part| self.text(part) == "hasOwnProperty") =>
            {
                let prototype = peel(field(receiver, "object")?);
                let namespace = peel(field(prototype, "object")?);
                if self.text(field(prototype, "property")?) != "prototype"
                    || self.text(namespace) != "Object"
                {
                    return None;
                }
                (namespace, true)
            }
            _ => return None,
        };
        let [object, key] = self.args(call)?;
        Some(Check {
            call,
            namespace,
            object,
            key,
            legacy,
            fallback: None,
        })
    }

    fn adjacent_own_checks(&mut self, effect: Node<'a>) -> Vec<Check<'a>> {
        let mut checks = Vec::new();
        let mut child = effect;
        while let Some(parent) = child.parent() {
            if !self.spend(1) || is_owner(parent) || deferred_field(parent, effect) {
                break;
            }
            if parent.kind() == "statement_block" {
                let mut earlier = child.prev_named_sibling();
                while let Some(statement) = earlier {
                    if !self.spend(1) {
                        return checks;
                    }
                    earlier = statement.prev_named_sibling();
                    if let Some(check) = self.own_failure(statement, effect) {
                        checks.push(check);
                    }
                }
            }
            child = parent;
        }
        checks
    }

    fn own_failure(&mut self, stmt: Node<'a>, effect: Node<'a>) -> Option<Check<'a>> {
        if stmt.kind() != "if_statement" || field(stmt, "alternative").is_some() {
            return None;
        }
        let condition = peel(field(stmt, "condition")?);
        if condition.kind() != "unary_expression" || operator(condition, self.file) != "!" {
            return None;
        }
        let mut check = self.own_call(peel(field(condition, "argument")?))?;
        let mut branch = field(stmt, "consequence")?;
        if branch.kind() == "statement_block" {
            let mut nodes = children(branch);
            branch = nodes.next()?;
            if nodes.next().is_some() {
                return None;
            }
        }
        if branch.kind() == "break_statement" && self.break_excludes(branch, effect) {
            return Some(check);
        }
        if branch.kind() == "expression_statement" {
            let value = peel(children(branch).next()?);
            if value.kind() == "assignment_expression" {
                check.fallback = Some(value);
                return Some(check);
            }
            if self.always_throws(value) {
                return Some(check);
            }
        }
        None
    }

    fn break_excludes(&mut self, jump: Node<'a>, effect: Node<'a>) -> bool {
        let Some(label) = field(jump, "label") else {
            return false;
        };
        for ancestor in std::iter::successors(jump.parent(), Node::parent) {
            if !self.spend(1) || is_owner(ancestor) || ancestor.kind() == "try_statement" {
                return false;
            }
            if ancestor.kind() == "labeled_statement"
                && field(ancestor, "label").is_some_and(|name| self.text(name) == self.text(label))
            {
                return inside(ancestor, effect);
            }
        }
        false
    }

    fn reflect_key(&mut self, key: Node<'a>) -> bool {
        let Some((_, declaration)) = self.declaration(key) else {
            return false;
        };
        let Some(pattern) = declaration
            .parent()
            .filter(|pattern| pattern.kind() == "array_pattern" && children(*pattern).count() == 1)
        else {
            return false;
        };
        let Some(declarator) = pattern
            .parent()
            .filter(|decl| decl.kind() == "variable_declarator")
        else {
            return false;
        };
        let Some(call) = field(declarator, "value").map(peel) else {
            return false;
        };
        let Some((namespace, "ownKeys")) = self.method(call) else {
            return false;
        };
        let Some([object]) = self.args(call) else {
            return false;
        };
        let elements: Vec<_> = children(object).take(2).collect();
        object.kind() == "object"
            && matches!(elements.as_slice(), [pair] if pair.kind() == "pair"
                && field(*pair, "key").is_some_and(|key| key.kind() == "computed_property_name"))
            && self.only_references(key, |_, _| true)
            && self.native(call, namespace, &["Reflect", "ownKeys"])
            && self.unwritten(call, &["Array", "prototype", "@@iterator"])
    }

    fn own_slots(
        &mut self,
        input: &GuardInput<'a>,
        first: Selection<'a>,
        check: Check<'a>,
        is_string: bool,
    ) -> Option<GuardForm> {
        if self.owner(first.effect) != Some(input.owner) {
            return None;
        }
        let table = self.initializer(first.object)?;
        let [one] = input.steps.as_slice() else {
            if let [outer, inner] = input.steps.as_slice() {
                return self.diagonal(input, *outer, *inner, check, table, is_string);
            }
            return None;
        };
        if !self.table_references(input, *one, check) {
            return None;
        }
        if table.kind() == "call_expression" {
            let [graph] = self.args(table)?;
            let callee = field(table, "function").map(peel)?;
            if check.fallback.is_none()
                && self.native(table, callee, &["structuredClone"])
                && self.closed_clone(graph, table)
            {
                return Some(GuardForm::OwnClosedClone);
            }
            return None;
        }
        let slots = self.closed_slots(table)?;
        if check.fallback.is_some() {
            self.refuse(check.call, "incomplete_coverage");
            return None;
        }
        if !slots.is_empty()
            && slots.iter().all(|(slot, child)| {
                is_string && matches!(slot, Slot::Symbol(_))
                    || self.private_child(*child, input, table)
            })
            && self.fallback_is_private(table, check, *one)
        {
            return Some(GuardForm::OwnClosedSlots);
        }
        None
    }

    fn diagonal(
        &mut self,
        input: &GuardInput<'a>,
        outer: Selection<'a>,
        inner: Selection<'a>,
        check: Check<'a>,
        table: Node<'a>,
        is_string: bool,
    ) -> Option<GuardForm> {
        if !is_string
            || check.fallback.is_some()
            || peel(inner.object) != outer.effect
            || !self.same_key(outer.key, inner.key)
            || !self.table_references(input, outer, check)
        {
            self.refuse(check.call, "incomplete_coverage");
            return None;
        }
        let slots = self.closed_slots(table)?;
        let mut selected = 0;
        for (key, row) in slots {
            if !self.spend(1) {
                return None;
            }
            if matches!(key, Slot::Symbol(_)) {
                continue;
            }
            let child = self.closed_slots(row)?.remove(&key)?;
            if !self.private_child(child, input, table) {
                return None;
            }
            selected += 1;
        }
        (selected > 0).then_some(GuardForm::OwnStableDiagonal)
    }

    fn table_references(
        &mut self,
        input: &GuardInput<'a>,
        step: Selection<'a>,
        check: Check<'a>,
    ) -> bool {
        self.only_references(step.object, |proof, read| {
            inside(check.call, read)
                || inside(step.effect, read)
                || check.fallback.is_some_and(|store| inside(store, read))
                || proof.terminal_value(read, input.owner)
        })
    }

    fn fallback_is_private(
        &mut self,
        table: Node<'a>,
        check: Check<'a>,
        step: Selection<'a>,
    ) -> bool {
        let Some(store) = check.fallback else {
            return true;
        };
        let Some(target) = field(store, "left").map(peel) else {
            return false;
        };
        let (Some(object), Some(key)) = (field(target, "object"), field(target, "index")) else {
            return false;
        };
        self.same_key(object, step.object)
            && self.same_key(key, step.key)
            && children(table).any(|pair| {
                self.pair_name(pair) == Some("__proto__")
                    && field(pair, "value").is_some_and(|value| value.kind() == "null")
            })
            && field(store, "right").is_some_and(|value| self.fresh_empty(value))
    }
}
