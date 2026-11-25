//! Structured control transfers. Loop and inline-callback bodies are visited once.
use super::*;

pub(super) struct PendingJump {
    pub(super) state: State,
    // A finally block may replace this completion. Only the return operand
    // carried to the owner exit can establish the reducer's result.
    returns: BTreeSet<usize>,
}

impl<'a> Analysis<'a> {
    pub(super) fn finish_owner(&mut self, context: Context<'a>, state: &mut State) {
        if let Some(exit) = self.jumps.remove(&(context.owner.id(), "return_statement")) {
            self.join(context, state, &exit.state, false);
            self.reachable_returns.extend(exit.returns);
        }
        if let Some(closures) = self.deferred.remove(&context.owner.id()) {
            for (closure, outer) in closures.into_values() {
                self.function(closure, outer, state, None);
            }
        }
    }

    pub(super) fn function(
        &mut self,
        node: Node<'a>,
        outer: Context<'a>,
        state: &mut State,
        iteration: Option<(Node<'a>, &str)>,
    ) {
        let context = Context {
            scope: node,
            owner: node,
        };
        let Some(mut local) = self.callable_input(context, state) else {
            self.inventory(node, context, &mut State::default());
            if iteration.is_some() {
                self.widen(node, outer, state);
            }
            return;
        };
        self.enter(node, context, &mut local);
        if iteration.is_some() {
            self.widen(node, context, &mut local);
        }
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
                if !self.charge(node, 1) {
                    self.inventory(node, context, &mut State::default());
                    if iteration.is_some() {
                        self.widen(node, outer, state);
                    }
                    return;
                }
                self.write(parameter, origin, context, &mut local);
            }
        }
        if let Some(body) = field(node, "body") {
            self.visit(body, context, &mut local);
        }
        self.finish_owner(context, &mut local);
        let thrown = self.jumps.remove(&(node.id(), "throw_statement"));
        if iteration.is_some() {
            if self.skipped(node) {
                self.widen(node, outer, state);
                return;
            }
            if let Some(thrown) = thrown {
                self.save_jump(
                    outer,
                    (outer.owner.id(), "throw_statement"),
                    &thrown.state,
                    thrown.returns,
                );
            }
            if local.dead || !self.charge(outer.owner, local.weight()) {
                return;
            }
            for (binding, value) in local.values {
                if self.lexical.binding_owner_ids.get(&binding) == Some(&node.id()) {
                    continue;
                }
                if let Some(previous) = state.values.get_mut(&binding) {
                    previous.merge(&value);
                } else {
                    state.values.insert(
                        binding,
                        Version {
                            incomplete: true,
                            ..value
                        },
                    );
                }
            }
            for (object, sites) in local.mutated_objects {
                state
                    .mutated_objects
                    .entry(object)
                    .or_default()
                    .extend(sites);
            }
        }
    }

    fn callable_input(&mut self, context: Context<'a>, state: &State) -> Option<State> {
        if !self.charge(context.owner, 1) {
            return None;
        }
        let mut input = State::default();
        let Some(bindings) = self.effects.outer_bindings.get(&context.owner.id()) else {
            return Some(input);
        };
        // Compute the projection cost before copying it. Mutation sets are
        // brought along only for receiver identities present in the projection.
        let mut objects = BTreeSet::new();
        let mut work = bindings.len();
        for value in bindings
            .iter()
            .filter_map(|binding| state.values.get(binding))
        {
            work += value.weight();
            if work > OWNER_WORK_BUDGET {
                self.charge(context.owner, work);
                return None;
            }
            objects.extend(value.objects.iter().copied());
        }
        work += objects
            .iter()
            .filter_map(|object| state.mutated_objects.get(object))
            .map(|sites| 1 + sites.len())
            .sum::<usize>();
        if !self.charge(context.owner, work) {
            return None;
        }
        for binding in &self.effects.outer_bindings[&context.owner.id()] {
            if let Some(value) = state.values.get(binding) {
                input.values.insert(*binding, value.clone());
            }
        }
        for object in objects {
            if let Some(sites) = state.mutated_objects.get(&object) {
                input.mutated_objects.insert(object, sites.clone());
            }
        }
        Some(input)
    }

    pub(super) fn loop_body(&mut self, node: Node<'a>, context: Context<'a>, state: &mut State) {
        if let Some(init) = field(node, "initializer").or_else(|| field(node, "right")) {
            self.visit(init, context, state);
        }
        let Some(mut body) = self.fork(context, state) else {
            self.inventory(node, context, state);
            return;
        };
        let condition = field(node, "condition");
        let statement = field(node, "body");
        // Initializers and the iterable run once. The condition, target, body,
        // and increment can see writes from any earlier iteration.
        if !statement.is_some_and(definitely_exits_iteration) {
            if let Some(statement) = statement {
                let (cutoffs, safe_cutoffs) = self.back_edge_cutoffs(statement, context);
                self.widen_except(
                    statement,
                    context,
                    &mut body,
                    &cutoffs,
                    safe_cutoffs.as_ref(),
                );
            }
            for region in [condition, field(node, "left"), field(node, "increment")]
                .into_iter()
                .flatten()
            {
                self.widen(region, context, &mut body);
            }
        }
        let mut exit = State::unreachable();
        let pretest = node.kind() != "do_statement";
        let can_skip = pretest
            && (condition.is_some_and(|test| test.kind() != "empty_statement")
                || node.kind() == "for_in_statement");
        if pretest {
            if let Some(test) = condition {
                self.visit(test, context, &mut body);
            }
            if can_skip {
                self.join(context, &mut exit, state, false);
                if let Some(test) = condition {
                    // The condition may assign before deciding whether to enter.
                    self.widen(test, context, &mut exit);
                }
            }
        }
        if let (Some(left), Some(right)) = (field(node, "left"), field(node, "right")) {
            self.write(left, Origin::Element(right), context, &mut body);
        }
        if let Some(statement) = statement {
            self.visit(statement, context, &mut body);
        }
        if let Some(next) = self.jumps.remove(&(node.id(), "continue_statement")) {
            self.join(context, &mut body, &next.state, false);
        }
        if let Some(increment) = field(node, "increment") {
            self.visit(increment, context, &mut body);
        }
        if !pretest {
            if let Some(test) = condition {
                self.visit(test, context, &mut body);
            }
            self.join(context, &mut exit, &body, false);
        } else if can_skip {
            // Use the completed iteration state, so a dominating restoration
            // (including finally on continue) kills a selected cursor.
            self.join(context, &mut exit, &body, false);
        }
        if let Some(breaks) = self.jumps.remove(&(node.id(), "break_statement")) {
            self.join(context, &mut exit, &breaks.state, false);
        }
        *state = exit;
    }

    pub(super) fn jump(&mut self, node: Node<'a>, context: Context<'a>, state: &mut State) {
        let kind = node.kind();
        let label = field(node, "label").map(|label| self.file.node_text(&label));
        let mut outer = node.parent();
        let mut target = context.owner;
        if matches!(kind, "break_statement" | "continue_statement") {
            while let Some(parent) = outer.filter(|n| *n != context.owner) {
                if !self.charge(context.owner, 1) {
                    return;
                }
                if let Some(label) = label {
                    if parent.kind() == "labeled_statement"
                        && field(parent, "label").is_some_and(|n| self.file.node_text(&n) == label)
                    {
                        target = if kind == "continue_statement" {
                            let mut body = field(parent, "body").unwrap_or(parent);
                            while body.kind() == "labeled_statement" {
                                let Some(inner) = field(body, "body") else {
                                    break;
                                };
                                body = inner;
                            }
                            body
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
        let returned = (kind == "return_statement").then_some(node.id());
        self.save_jump(context, (target.id(), kind), state, returned);
        state.dead = true;
    }

    fn save_jump(
        &mut self,
        context: Context<'a>,
        jump: (usize, &'static str),
        state: &State,
        returned: impl IntoIterator<Item = usize>,
    ) {
        let previous = self
            .jumps
            .get(&jump)
            .map_or(0, |pending| pending.state.weight() + pending.returns.len());
        if !state.dead && self.charge(context.owner, previous + state.weight()) {
            let pending = self.jumps.entry(jump).or_insert_with(|| PendingJump {
                state: State::unreachable(),
                returns: BTreeSet::new(),
            });
            pending.state.join(state);
            pending.returns.extend(returned);
        }
    }

    pub(super) fn visit_try(&mut self, node: Node<'a>, context: Context<'a>, state: &mut State) {
        let handler = field(node, "handler");
        let finalizer = field(node, "finalizer");
        let Some(mut exceptional) = self.fork(context, state) else {
            self.inventory(node, context, state);
            return;
        };
        let outside = std::mem::take(&mut self.jumps);
        let body = field(node, "body");
        if let Some(body) = body {
            if handler.is_some() || finalizer.is_some() {
                self.widen(body, context, &mut exceptional);
            }
            self.visit(body, context, state);
        }
        if let Some(handler) = handler {
            if let Some(mut caught) = self.fork(context, &exceptional) {
                self.join(context, &mut caught, state, false);
                if let Some(thrown) = self.jumps.remove(&(context.owner.id(), "throw_statement")) {
                    self.join(context, &mut caught, &thrown.state, false);
                }
                caught.invalidate();
                self.visit(handler, context, &mut caught);
                self.join(context, state, &caught, false);
            } else {
                self.inventory(handler, context, state);
            }
            if finalizer.is_some() {
                self.widen(handler, context, &mut exceptional);
            }
        }
        if let Some(finalizer) = finalizer {
            // An expression in either arm can throw before subsequent stores.
            // Visit the finalizer once, using all pending completions as inputs.
            // A normal finalizer preserves their completion kinds.
            let exits = std::mem::take(&mut self.jumps);
            let normal = !state.dead;
            self.join(context, &mut exceptional, state, false);
            for pending in exits.values() {
                self.join(context, &mut exceptional, &pending.state, false);
            }
            exceptional.invalidate();
            self.visit(finalizer, context, &mut exceptional);
            for (jump, pending) in exits {
                self.save_jump(context, jump, &exceptional, pending.returns);
            }
            if normal {
                *state = exceptional;
            }
        }
        for (jump, pending) in outside {
            self.save_jump(context, jump, &pending.state, pending.returns);
        }
    }

    pub(super) fn visit_switch(&mut self, node: Node<'a>, context: Context<'a>, state: &mut State) {
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
        self.enter(body, context, state);
        self.widen(body, context, state);
        let Some(bypass) = self.fork(context, state) else {
            self.inventory(body, context, state);
            return;
        };
        let mut has_default = false;
        let mut cases = Vec::new();
        for case in children(body) {
            let at = field(case, "value");
            let input = if let Some(test) = at {
                self.visit(test, context, state);
                let Some(input) = self.fork(context, state) else {
                    self.inventory(body, context, state);
                    return;
                };
                Some(input)
            } else {
                has_default = true;
                None
            };
            cases.push((case, at, input));
        }
        let mut fallthrough = State::unreachable();
        for (case, test, input) in cases {
            self.join(
                context,
                &mut fallthrough,
                input.as_ref().unwrap_or(state),
                true,
            );
            for statement in children(case).filter(|statement| Some(*statement) != test) {
                self.visit(statement, context, &mut fallthrough);
            }
        }
        if !has_default {
            self.join(context, &mut fallthrough, state, true);
        }
        if !has_default
            && (!fallthrough.dead || self.jumps.contains_key(&(node.id(), "break_statement")))
        {
            self.join(context, &mut fallthrough, &bypass, true);
        }
        *state = fallthrough;
    }
    fn back_edge_cutoffs(
        &mut self,
        body: Node<'a>,
        context: Context<'a>,
    ) -> (BTreeMap<usize, usize>, Option<BTreeMap<usize, usize>>) {
        let mut stores = BTreeMap::new();
        let mut safe_cutoffs = None;
        let mut pending = vec![(body, false)];
        while let Some((node, after_abrupt_finally)) = pending.pop() {
            if after_abrupt_finally && safe_cutoffs.is_none() {
                if !self.charge(context.owner, stores.len()) {
                    return (BTreeMap::new(), None);
                }
                safe_cutoffs = Some(stores.clone());
                continue;
            }
            if !self.charge(context.owner, 1) {
                return (BTreeMap::new(), None);
            }
            match node.kind() {
                "statement_block" | "finally_clause" => {
                    let mut statements: Vec<_> = children(node).collect();
                    pending.extend(
                        statements
                            .drain(..)
                            .rev()
                            .map(|statement| (statement, false)),
                    );
                }
                "try_statement" if field(node, "finalizer").is_some() => {
                    let finalizer = field(node, "finalizer").unwrap();
                    if safe_cutoffs.is_none() && self.abrupt_try(node, finalizer, context) {
                        // The finalizer still runs on the abrupt edge. A
                        // following statement cannot kill the edge's store.
                        pending.push((node, true));
                    }
                    pending.push((finalizer, false));
                }
                "expression_statement" => {
                    if let Some(target) = children(node)
                        .next()
                        .map(peel)
                        .filter(|expr| expr.kind() == "assignment_expression")
                        .and_then(|expr| field(expr, "left"))
                        .map(peel)
                        .filter(|target| identifier(*target))
                    {
                        if let Some(write) = self.effects.writes_in(target).next() {
                            stores.insert(write.binding, write.site.start_byte);
                        }
                    }
                }
                _ if self.may_continue(node, context) => break,
                _ => {}
            }
        }
        (stores, safe_cutoffs)
    }

    fn abrupt_try(&mut self, node: Node<'a>, finalizer: Node<'a>, context: Context<'a>) -> bool {
        let mut pending: Vec<_> = children(node)
            .filter(|child| *child != finalizer)
            .map(|child| (child, false, false))
            .collect();
        while let Some((node, in_loop, in_switch)) = pending.pop() {
            if !self.charge(context.owner, 1) {
                return true;
            }
            let labelled = field(node, "label").is_some();
            match node.kind() {
                "continue_statement" if labelled || !in_loop => return true,
                "break_statement" if labelled || (!in_loop && !in_switch) => return true,
                _ if is_owner(node) => continue,
                _ => {}
            }
            pending.extend(children(node).map(|child| {
                (
                    child,
                    in_loop || is_loop(node),
                    in_switch || node.kind() == "switch_statement",
                )
            }));
        }
        false
    }

    fn may_continue(&mut self, region: Node<'a>, context: Context<'a>) -> bool {
        let mut pending = vec![region];
        while let Some(node) = pending.pop() {
            if !self.charge(context.owner, 1)
                || matches!(node.kind(), "continue_statement" | "break_statement")
            {
                return true;
            }
            if !is_owner(node) {
                pending.extend(children(node));
            }
        }
        false
    }
}

// A simple abrupt body has no back edge. In particular an unconditional
// break bypasses the increment, including when it targets a label.
fn definitely_exits_iteration(node: Node<'_>) -> bool {
    match node.kind() {
        "return_statement" | "throw_statement" | "break_statement" => true,
        "statement_block" => {
            let mut statements = children(node);
            statements.next().is_some_and(definitely_exits_iteration) && statements.next().is_none()
        }
        "if_statement" => ["consequence", "alternative"]
            .into_iter()
            .all(|arm| field(node, arm).is_some_and(definitely_exits_iteration)),
        _ => false,
    }
}
