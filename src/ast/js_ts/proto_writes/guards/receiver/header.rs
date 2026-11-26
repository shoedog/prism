//! Single header descent with a checked scratch redirect.
use super::*;

impl<'a, F: FnMut(usize) -> bool> Proof<'a, '_, F> {
    pub(super) fn header_redirect(&mut self, input: &GuardInput<'a>) -> bool {
        let [step] = input.steps.as_slice() else {
            return false;
        };
        if step.shared_link
            || self.owner(step.effect) != Some(input.owner)
            || self.frame(step.effect) != self.frame(input.store)
        {
            return false;
        }
        let Some((binding, _)) = self.declaration(input.receiver) else {
            return false;
        };
        let routes = self.route(step.effect);
        let Some(header) = self.header_capture(step.key, &routes) else {
            return false;
        };
        let Some(field_check) = routes.iter().find(|route| {
            if route.accepts || route.guard.kind() != "if_statement" {
                return false;
            }
            let condition = peel(route.condition);
            let Some([left, right]) = self.binary(condition) else {
                return false;
            };
            operator(condition, self.file) == "==="
                && [(left, right), (right, left)]
                    .into_iter()
                    .any(|(value, name)| {
                        self.string(name) == Some("__proto__") && self.same_key(value, step.key)
                    })
        }) else {
            return false;
        };
        if !inside(header, field_check.guard) {
            return false;
        }
        let check = field_check.guard;
        let Some(redirect) = field(check, "consequence").and_then(assignment) else {
            return false;
        };
        let Some(advance) = field(check, "alternative").and_then(assignment) else {
            return false;
        };
        let Some(scratch) = field(redirect, "right").map(peel) else {
            return false;
        };
        if !self
            .initializer(scratch)
            .is_some_and(|value| self.fresh_empty(value))
            || !self.only_references(scratch, |_, read| read == scratch)
        {
            self.refuse(check, "unproved_child");
            return false;
        }
        let Some(install) = field(advance, "right")
            .map(peel)
            .filter(|node| node.kind() == "assignment_expression")
        else {
            return false;
        };
        let Some(slot) = field(install, "left").map(peel) else {
            return false;
        };
        let (Some(object), Some(key)) = (field(slot, "object"), field(slot, "index")) else {
            return false;
        };
        let Some(retrieved) = field(install, "right").map(peel) else {
            return false;
        };
        let Some([lookup, fallback]) = self.binary(retrieved) else {
            return false;
        };
        if operator(retrieved, self.file) != "||"
            || lookup != step.effect
            || !self.fresh_empty(fallback)
            || !self.same_key(key, step.key)
            || !self.same_key(object, step.object)
        {
            return false;
        }
        let Some(skip) = check.next_named_sibling() else {
            return false;
        };
        if skip.kind() != "continue_statement"
            || skip.next_named_sibling().is_some()
            || field(skip, "label").is_some()
            || self.frame(skip) != self.frame(step.effect)
            || self
                .frame(skip)
                .is_none_or(|frame| frame.kind() != "for_in_statement")
        {
            self.refuse(check, "wrong_exit");
            return false;
        }
        let Some(redirected) = field(redirect, "left").filter(|node| identifier(*node)) else {
            return false;
        };
        let Some(advanced) = field(advance, "left").filter(|node| identifier(*node)) else {
            return false;
        };
        let admitted = BTreeSet::from([
            binding.clone(),
            AstSpan::from(redirected),
            AstSpan::from(advanced),
        ]);
        if self.analysis.proof_writes(&binding) != Some(&admitted) {
            self.refuse(check, "wrong_version");
            return false;
        }
        true
    }

    fn header_capture(
        &mut self,
        key: Node<'a>,
        routes: &[super::super::flow::Edge<'a>],
    ) -> Option<Node<'a>> {
        let capture = self.initializer(key)?;
        if capture.kind() != "subscript_expression"
            || !field(capture, "index").is_some_and(|index| self.text(index) == "1")
        {
            return None;
        }
        let header = peel(field(capture, "object")?);
        let call = self.initializer(header)?;
        let (regex, "exec") = self.method(call)? else {
            return None;
        };
        let [_line] = self.args(call)?;
        if regex.kind() != "regex" || self.text(field(regex, "pattern")?) != r"^\[([^\]]+)\]$" {
            return None;
        }
        if field(regex, "flags").is_some()
            || !self.unwritten(call, &["RegExp", "prototype", "exec"])
        {
            self.refuse(call, "unproved_native_identity");
            return None;
        }
        routes
            .iter()
            .find(|route| {
                route.accepts
                    && route.guard.kind() == "if_statement"
                    && self.same_key(peel(route.condition), header)
            })
            .and_then(|route| field(route.guard, "consequence"))
    }
}

fn assignment(mut statement: Node<'_>) -> Option<Node<'_>> {
    if matches!(statement.kind(), "else_clause" | "statement_block") {
        let mut body = children(statement);
        statement = body.next()?;
        if body.next().is_some() {
            return None;
        }
    }
    if statement.kind() == "statement_block" {
        let mut body = children(statement);
        statement = body.next()?;
        if body.next().is_some() {
            return None;
        }
    }
    let expression = peel(children(statement).next()?);
    (statement.kind() == "expression_statement" && expression.kind() == "assignment_expression")
        .then_some(expression)
}
