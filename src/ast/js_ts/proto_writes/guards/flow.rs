//! Predicates on the live route to an evaluated selector.
use super::*;

#[derive(Clone, Copy)]
pub(super) struct Edge<'a> {
    pub(super) guard: Node<'a>,
    pub(super) condition: Node<'a>,
    pub(super) accepts: bool,
    pub(super) complete: bool,
}

impl<'a, F: FnMut(usize) -> bool> Proof<'a, '_, F> {
    pub(super) fn route(&mut self, effect: Node<'a>) -> Vec<Edge<'a>> {
        let mut live = effect;
        let mut conditions = Vec::new();
        while let Some(parent) = live.parent() {
            if !self.spend(1) {
                break;
            }
            if deferred_field(parent, effect) {
                self.refuse(parent, "wrong_owner");
                break;
            }
            match parent.kind() {
                "statement_block" | "program" => {
                    self.earlier_edges(live, effect, &mut conditions);
                }
                "if_statement" | "ternary_expression" => {
                    if let Some(condition) = field(parent, "condition") {
                        let accepts = field(parent, "consequence") == Some(live);
                        if accepts || field(parent, "alternative") == Some(live) {
                            conditions.push(Edge {
                                guard: parent,
                                condition,
                                accepts,
                                complete: true,
                            });
                        }
                    }
                }
                "binary_expression" if field(parent, "right") == Some(live) => {
                    let op = operator(parent, self.file);
                    if let Some(condition) =
                        field(parent, "left").filter(|_| matches!(op, "&&" | "||"))
                    {
                        conditions.push(Edge {
                            guard: parent,
                            condition,
                            accepts: op == "&&",
                            complete: true,
                        });
                    }
                }
                "try_statement" if field(parent, "finalizer") == Some(live) => {
                    if self.guarded_try(parent) {
                        self.refuse(parent, "wrong_exit");
                    }
                }
                "with_statement" | "ERROR" => {
                    self.refuse(parent, "unmodelled_flow");
                    conditions.clear();
                    break;
                }
                _ => {}
            }
            if is_owner(parent) {
                break;
            }
            live = parent;
        }
        conditions
    }

    fn earlier_edges(
        &mut self,
        anchor: Node<'a>,
        effect: Node<'a>,
        conditions: &mut Vec<Edge<'a>>,
    ) {
        let mut earlier = anchor.prev_named_sibling();
        while let Some(statement) = earlier {
            if !self.spend(1) {
                return;
            }
            earlier = statement.prev_named_sibling();
            match statement.kind() {
                "if_statement" => {
                    let Some(condition) = field(statement, "condition") else {
                        continue;
                    };
                    for (arm, accepts) in [("consequence", false), ("alternative", true)] {
                        let Some(arm) = field(statement, arm) else {
                            continue;
                        };
                        if self.rejects(arm, effect, 0) {
                            conditions.push(Edge {
                                guard: statement,
                                condition,
                                accepts,
                                complete: true,
                            });
                        }
                    }
                }
                "switch_statement" => {
                    if let Some(condition) = field(statement, "value") {
                        conditions.push(Edge {
                            guard: statement,
                            condition,
                            accepts: false,
                            complete: false,
                        });
                    }
                }
                "try_statement" => {
                    // Inner checks need their own accepting paths. They
                    // cannot be projected through a catch or finalizer.
                    if self.guarded_try(statement) {
                        self.refuse(statement, "wrong_exit");
                    }
                }
                _ => {}
            }
        }
    }

    fn guarded_try(&mut self, statement: Node<'a>) -> bool {
        let mut pending: Vec<_> = ["body", "handler"]
            .into_iter()
            .filter_map(|part| field(statement, part))
            .collect();
        while let Some(node) = pending.pop() {
            if !self.spend(node.named_child_count() + 1) {
                return false;
            }
            if is_owner(node) {
                continue;
            }
            if node.kind() == "if_statement"
                && ["consequence", "alternative"]
                    .into_iter()
                    .filter_map(|arm| field(node, arm))
                    .any(|arm| {
                        let first = if matches!(arm.kind(), "statement_block" | "else_clause") {
                            children(arm).next()
                        } else {
                            Some(arm)
                        };
                        first.is_some_and(|jump| {
                            matches!(
                                jump.kind(),
                                "return_statement"
                                    | "throw_statement"
                                    | "continue_statement"
                                    | "break_statement"
                            )
                        })
                    })
                && field(node, "condition")
                    .is_some_and(|condition| self.string_comparison(condition))
            {
                return true;
            }
            if node.kind() == "call_expression"
                && node
                    .parent()
                    .is_some_and(|parent| parent.kind() == "expression_statement")
                && self.args::<1>(node).is_some()
                && self.asserts_string(node)
            {
                return true;
            }
            pending.extend(children(node));
        }
        false
    }

    fn string_comparison(&mut self, condition: Node<'a>) -> bool {
        let mut pending = vec![condition];
        while let Some(node) = pending.pop().map(peel) {
            if !self.spend(1) {
                return false;
            }
            let Some(arms) = self.binary(node) else {
                continue;
            };
            match operator(node, self.file) {
                "&&" | "||" => pending.extend(arms),
                "==" | "!=" | "===" | "!=="
                    if arms.iter().any(|arm| self.string(*arm).is_some()) =>
                {
                    return true
                }
                _ => {}
            }
        }
        false
    }

    fn rejects(&mut self, statement: Node<'a>, effect: Node<'a>, depth: usize) -> bool {
        if depth == 32 || !self.spend(1) {
            return false;
        }
        match statement.kind() {
            "else_clause" | "statement_block" => {
                // Admit a jump as the first statement or a nested block
                // with this property. Later statements add no exit proof.
                children(statement)
                    .next()
                    .is_some_and(|child| self.rejects(child, effect, depth + 1))
            }
            "return_statement" => self.jump_applies(statement, effect),
            "throw_statement" => self.jump_applies(statement, effect),
            "continue_statement" => self.jump_applies(statement, effect),
            _ => false,
        }
    }

    fn jump_applies(&mut self, jump: Node<'a>, effect: Node<'a>) -> bool {
        let is_throw = jump.kind() == "throw_statement";
        let is_continue = jump.kind() == "continue_statement";
        let label = field(jump, "label");
        let mut child = jump;
        while let Some(ancestor) = child.parent() {
            if !self.spend(1) {
                return false;
            }
            if ancestor.kind() == "try_statement"
                && (field(ancestor, "finalizer").is_some()
                    || is_throw
                        && field(ancestor, "handler").is_some()
                        && field(ancestor, "body") == Some(child))
            {
                self.refuse(jump, "wrong_exit");
                return false;
            }
            if is_continue {
                let target = if let Some(label) = label {
                    (ancestor.kind() == "labeled_statement"
                        && field(ancestor, "label")
                            .is_some_and(|name| self.text(name) == self.text(label)))
                    .then(|| field(ancestor, "body"))
                    .flatten()
                } else {
                    is_loop(ancestor).then_some(ancestor)
                };
                if let Some(target) = target {
                    // The check excludes one evaluation of this step.
                    // Proofs over pending cursor/key state need additional
                    // evidence before crediting later effects.
                    return field(target, "body").is_some_and(|body| inside(body, effect));
                }
            }
            if is_owner(ancestor) {
                return !is_continue && inside(ancestor, effect);
            }
            child = ancestor;
        }
        false
    }

    pub(super) fn owner(&mut self, node: Node<'a>) -> Option<Node<'a>> {
        let mut at = Some(node);
        while let Some(current) = at {
            if !self.spend(1) {
                return None;
            }
            if is_owner(current) {
                return Some(current);
            }
            at = current.parent();
        }
        None
    }

    pub(super) fn frame(&mut self, node: Node<'a>) -> Option<Node<'a>> {
        for ancestor in std::iter::successors(node.parent(), Node::parent) {
            if !self.spend(1) {
                return None;
            }
            if is_loop(ancestor) || is_owner(ancestor) || deferred_field(ancestor, node) {
                return Some(ancestor);
            }
        }
        None
    }
}

pub(super) fn deferred_field(parent: Node<'_>, effect: Node<'_>) -> bool {
    super::super::instance_initializer(parent).is_some_and(|body| inside(body, effect))
}

pub(super) fn inside(outer: Node<'_>, inner: Node<'_>) -> bool {
    outer.start_byte() <= inner.start_byte() && outer.end_byte() >= inner.end_byte()
}
