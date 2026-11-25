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
            state.join(&exit.state);
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
        if !self.charge(node, state.weight()) {
            self.inventory(node, context, &mut State::default());
            return;
        }
        let mut local = state.clone();
        self.enter(node, &mut local);
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
                self.write(parameter, origin, context, &mut local);
            }
        }
        if let Some(body) = field(node, "body") {
            self.visit(body, context, &mut local);
        }
        self.finish_owner(context, &mut local);
        if let Some(thrown) = self.jumps.remove(&(node.id(), "throw_statement")) {
            if iteration.is_some() {
                self.save_jump(
                    (outer.owner.id(), "throw_statement"),
                    &thrown.state,
                    thrown.returns,
                );
            }
        }
        if iteration.is_some() {
            local.values.retain(|binding, _| {
                self.lexical.binding_owners.get(binding) != Some(&AstSpan::from(node))
            });
            state.join(&local);
        }
    }

    pub(super) fn loop_body(&mut self, node: Node<'a>, context: Context<'a>, state: &mut State) {
        if let Some(init) = field(node, "initializer").or_else(|| field(node, "right")) {
            self.visit(init, context, state);
        }
        let entry = state.clone();
        let mut body = entry.clone();
        let condition = field(node, "condition");
        let statement = field(node, "body");
        // Initializers and the iterable run once. The condition, target, body,
        // and increment can see writes from any earlier iteration.
        if !statement.is_some_and(definitely_exits_iteration) {
            for region in [
                condition,
                statement,
                field(node, "left"),
                field(node, "increment"),
            ]
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
                exit.join(&entry);
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
            body.join(&next.state);
        }
        if let Some(increment) = field(node, "increment") {
            self.visit(increment, context, &mut body);
        }
        if !pretest {
            if let Some(test) = condition {
                self.visit(test, context, &mut body);
            }
            exit.join(&body);
        } else if can_skip {
            // Use the completed iteration state, so a dominating restoration
            // (including finally on continue) kills a selected cursor.
            exit.join(&body);
        }
        if let Some(breaks) = self.jumps.remove(&(node.id(), "break_statement")) {
            exit.join(&breaks.state);
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
        self.save_jump((target.id(), kind), state, returned);
        state.dead = true;
    }

    fn save_jump(
        &mut self,
        jump: (usize, &'static str),
        state: &State,
        returned: impl IntoIterator<Item = usize>,
    ) {
        if !state.dead {
            let pending = self.jumps.entry(jump).or_insert_with(|| PendingJump {
                state: State::unreachable(),
                returns: BTreeSet::new(),
            });
            pending.state.join(state);
            pending.returns.extend(returned);
        }
    }

    pub(super) fn visit_try(&mut self, node: Node<'a>, context: Context<'a>, state: &mut State) {
        let outside = std::mem::take(&mut self.jumps);
        let mut caught = state.clone();
        if let Some(body) = field(node, "body") {
            if field(node, "handler").is_some() {
                self.widen(body, context, &mut caught);
            }
            self.visit(body, context, state);
        }
        if let Some(handler) = field(node, "handler") {
            caught.join(state);
            if let Some(thrown) = self.jumps.remove(&(context.owner.id(), "throw_statement")) {
                caught.join(&thrown.state);
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
                self.visit(finalizer, context, &mut pending.state);
                self.save_jump(jump, &pending.state, pending.returns);
            }
        }
        for (jump, pending) in outside {
            self.save_jump(jump, &pending.state, pending.returns);
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
