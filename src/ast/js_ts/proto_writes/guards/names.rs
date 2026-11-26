//! Deny-list comparisons and the closed set of weaker name checks.
use super::flow::Edge;
use super::*;

pub(super) const RESERVED: [&str; 3] = ["__proto__", "constructor", "prototype"];

#[derive(Default)]
struct Coverage {
    names: [bool; 3],
    string: bool,
    candidates: BTreeSet<usize>,
}

impl<'a, F: FnMut(usize) -> bool> Proof<'a, '_, F> {
    pub(super) fn name_guard(&mut self, input: &GuardInput<'a>) -> (GuardState, Vec<bool>) {
        let mut proved = Vec::with_capacity(input.steps.len());
        let mut any = false;
        for step in &input.steps {
            let edges = self.route(step.effect);
            let coverage = self.coverage(step.key, &edges);
            let relevant = !coverage.candidates.is_empty();
            let names = coverage.names[0] && coverage.names[1];
            let safe = if relevant && names {
                coverage.string || self.stored_string(step.key)
            } else {
                false
            };
            let shared = relevant && (step.shared_link || self.shared_receiver(step.object));
            let sufficient = safe && !shared && self.owner(step.effect) == Some(input.owner);
            if relevant && !sufficient {
                let reason = if shared {
                    "unproved_child"
                } else if !names {
                    "incomplete_coverage"
                } else {
                    "absent_domain"
                };
                for candidate in coverage.candidates {
                    self.refusals.insert((candidate, reason));
                }
            }
            proved.push(sufficient);
            any |= relevant || self.partial_precheck(*step, &edges, input);
        }
        if proved.iter().any(|&step| step) && self.shared_receiver(input.receiver) {
            proved.fill(false);
            self.refuse(input.target, "unproved_child");
        }
        if !proved.is_empty() && proved.iter().all(|&step| step) {
            (GuardState::Sufficient, proved)
        } else if any {
            if input.steps.len() > 1 {
                self.refuse(input.target, "incomplete_coverage");
            }
            (GuardState::Partial, proved)
        } else {
            (GuardState::None, proved)
        }
    }

    pub(super) fn has_name_coverage(&mut self, key: Node<'a>, edges: &[Edge<'a>]) -> bool {
        !self.coverage(key, edges).candidates.is_empty()
    }

    pub(super) fn checked_string(&mut self, key: Node<'a>, edges: &[Edge<'a>]) -> bool {
        self.coverage(key, edges).string || self.stored_string(key)
    }

    fn coverage(&mut self, key: Node<'a>, edges: &[Edge<'a>]) -> Coverage {
        let mut coverage = Coverage::default();
        for edge in edges {
            let mut pending = vec![(edge.condition, edge.accepts)];
            while let Some((condition, accepts)) = pending.pop() {
                if !self.spend(1) {
                    return coverage;
                }
                let condition = peel(condition);
                let op = operator(condition, self.file);
                if condition.kind() == "unary_expression" && op == "!" {
                    pending
                        .extend(field(condition, "argument").map(|argument| (argument, !accepts)));
                    continue;
                }
                if condition.kind() == "binary_expression"
                    && matches!((op, accepts), ("&&", true) | ("||", false))
                {
                    pending.extend(["left", "right"].into_iter().filter_map(|part| {
                        field(condition, part).map(|operand| (operand, accepts))
                    }));
                    continue;
                }
                if self.name_atom(condition, accepts, key, &mut coverage) {
                    coverage.candidates.insert(edge.guard.id());
                }
            }
        }
        coverage
    }

    fn name_atom(
        &mut self,
        atom: Node<'a>,
        accepts: bool,
        key: Node<'a>,
        coverage: &mut Coverage,
    ) -> bool {
        if atom.kind() != "binary_expression" {
            return self.name_call(atom, key, accepts);
        }
        let Some([left, right]) = self.binary(atom) else {
            return false;
        };
        let op = operator(atom, self.file);
        if op == "in" && !accepts && self.is_object_prototype(right) {
            return self.matched_key(left, key, atom);
        }
        if self.index_of_rejects(left, right, op, accepts, key)
            || self.index_of_rejects(right, left, op, accepts, key)
        {
            return true;
        }
        let tests_equality = matches!((op, accepts), ("===", true) | ("!==", false));
        let tests_inequality = matches!(
            (op, accepts),
            ("===", false) | ("!==", true) | ("==", false) | ("!=", true)
        );
        for (value, literal) in [(left, right), (right, left)] {
            let Some(literal) = self.string(literal) else {
                continue;
            };
            if tests_equality
                && literal == "string"
                && value.kind() == "unary_expression"
                && operator(value, self.file) == "typeof"
            {
                if field(value, "argument").is_some_and(|value| self.same_key(value, key)) {
                    coverage.string = true;
                }
            }
            if tests_inequality {
                if let Some(index) = RESERVED.iter().position(|name| *name == literal) {
                    if !self.matched_key(value, key, atom) {
                        return false;
                    }
                    if matches!(op, "===" | "!==") {
                        coverage.names[index] = true;
                    } else {
                        self.refuse(atom, "absent_domain");
                    }
                    return true;
                }
            }
        }
        false
    }

    pub(super) fn binary(&self, node: Node<'a>) -> Option<[Node<'a>; 2]> {
        Some([peel(field(node, "left")?), peel(field(node, "right")?)])
    }

    pub(super) fn same_key(&mut self, first: Node<'a>, second: Node<'a>) -> bool {
        let (first, second) = (peel(first), peel(second));
        if first == second {
            return identifier(first);
        }
        if !identifier(first) || !identifier(second) || !self.spend(1) {
            return false;
        }
        let (a, b) = if first.start_byte() < second.start_byte() {
            (first, second)
        } else {
            (second, first)
        };
        let (Some(a_read), Some(b_read)) = (self.bindings.bare_read(a), self.bindings.bare_read(b))
        else {
            return false;
        };
        let Some(binding) = a_read.binding.as_ref() else {
            return false;
        };
        if b_read.binding.as_ref() != Some(binding)
            || !self.spend(a_read.reaching_versions.len() + b_read.reaching_versions.len())
        {
            return false;
        }
        let mut pending_stores = Vec::new();
        let mut child = b;
        while let Some(parent) = child.parent() {
            if parent.start_byte() < a.end_byte() || is_owner(parent) {
                break;
            }
            if !self.spend(1) {
                return false;
            }
            if parent.kind() == "assignment_expression" && field(parent, "right") == Some(child) {
                pending_stores.extend(
                    field(parent, "left")
                        .map(peel)
                        .filter(|left| identifier(*left))
                        .map(AstSpan::from),
                );
            }
            child = parent;
        }
        if !self
            .bindings
            .correlated_before_stores(a_read, b_read, &pending_stores)
        {
            return false;
        }
        if self.frame(a) == self.frame(b) {
            return true;
        }
        // A predicate outside a repeated step can prove a stable outer
        // binding only if the census sees a single assignment in the file.
        self.bindings
            .proof_writes(binding)
            .is_some_and(|sites| sites.len() == 1)
    }

    fn matched_key(&mut self, checked: Node<'a>, key: Node<'a>, candidate: Node<'a>) -> bool {
        if self.same_key(checked, key) {
            return true;
        }
        if identifier(checked) && identifier(key) {
            if self.same_binding(checked, key) {
                self.refuse(candidate, "wrong_version");
            } else if self.rereads_member(checked, key) {
                self.refuse(candidate, "unstable_key");
            } else {
                self.refuse(candidate, "wrong_binding");
            }
        } else {
            self.refuse(candidate, "unstable_key");
        }
        false
    }

    pub(super) fn same_binding(&self, first: Node<'a>, second: Node<'a>) -> bool {
        self.bindings
            .bare_read(first)
            .zip(self.bindings.bare_read(second))
            .is_some_and(|(a, b)| a.binding.is_some() && a.binding == b.binding)
    }

    fn rereads_member(&mut self, checked: Node<'a>, key: Node<'a>) -> bool {
        let left = self.origin_values(checked, false);
        let right = self.origin_values(key, false);
        let ([a], [b]) = (left.as_slice(), right.as_slice()) else {
            return false;
        };
        a != b
            && a.kind() == "member_expression"
            && b.kind() == "member_expression"
            && field(*a, "property")
                .zip(field(*b, "property"))
                .is_some_and(|(a, b)| self.text(a) == self.text(b))
            && field(*a, "object")
                .zip(field(*b, "object"))
                .is_some_and(|(a, b)| self.same_binding(a, b))
    }

    pub(super) fn origin_values(&mut self, node: Node<'a>, complete: bool) -> Vec<Node<'a>> {
        let Some(read) = self.bindings.bare_read(node) else {
            return Vec::new();
        };
        if !self.spend(read.reaching_versions.len() + 1) || complete && !read.resolution_complete {
            return Vec::new();
        }
        read.reaching_versions
            .iter()
            .filter_map(|site| self.bindings.value_origin(read, site))
            .collect()
    }

    pub(super) fn initializer(&mut self, node: Node<'a>) -> Option<Node<'a>> {
        if self.bindings.bare_read(node)?.reaching_versions.len() != 1 {
            return None;
        }
        let [value] = self.origin_values(node, true).try_into().ok()?;
        Some(value)
    }

    fn stored_string(&mut self, key: Node<'a>) -> bool {
        let Some(value) = self.initializer(key) else {
            return false;
        };
        if self.string(value).is_some() {
            return true;
        }
        let Some(callee) = field(value, "function").map(peel) else {
            return false;
        };
        if value.kind() != "call_expression"
            || !identifier(callee)
            || self.bindings.canonical(self.text(callee)) != "String"
            || field(value, "optional_chain").is_some()
            || self.args::<1>(value).is_none()
            || self.frame(value) != self.frame(key)
        {
            return false;
        }
        self.native(value, callee, &["String"])
    }

    pub(super) fn args<const N: usize>(&self, call: Node<'a>) -> Option<[Node<'a>; N]> {
        let args: Vec<_> = children(field(call, "arguments")?)
            .take(N + 1)
            .map(peel)
            .collect();
        if args.iter().any(|arg| arg.kind() == "spread_element") {
            return None;
        }
        args.try_into().ok()
    }

    pub(super) fn method(&self, call: Node<'a>) -> Option<(Node<'a>, &'a str)> {
        let callee = peel(field(call, "function")?);
        if call.kind() != "call_expression"
            || field(call, "optional_chain").is_some()
            || field(callee, "optional_chain").is_some()
        {
            return None;
        }
        Some((
            peel(field(callee, "object")?),
            self.text(field(callee, "property")?),
        ))
    }

    pub(super) fn is_object_prototype(&self, node: Node<'a>) -> bool {
        let property = |member| {
            field(member, "property")
                .map(|property| self.text(property))
                .or_else(|| field(member, "index").and_then(|index| self.string(index)))
        };
        if property(node) != Some("prototype") {
            return false;
        }
        let Some(base) = field(node, "object").map(peel) else {
            return false;
        };
        if identifier(base) {
            return matches!(self.text(base), "Object" | "Function" | "Array")
                && !self.bindings.is_shadowed(base);
        }
        matches!(property(base), Some("Object" | "Function" | "Array"))
            && field(base, "object").map(peel).is_some_and(|global| {
                identifier(global)
                    && matches!(
                        self.text(global),
                        "globalThis" | "global" | "window" | "self"
                    )
                    && !self.bindings.is_shadowed(global)
            })
    }

    pub(super) fn shared_receiver(&mut self, receiver: Node<'a>) -> bool {
        let mut pending = vec![receiver];
        let mut seen = BTreeSet::new();
        while let Some(node) = pending.pop().map(peel) {
            if !self.spend(1) {
                return false;
            }
            if !seen.insert(node.id()) {
                continue;
            }
            if self.is_object_prototype(node) {
                return true;
            }
            if identifier(node) {
                pending.extend(self.origin_values(node, false));
                continue;
            }
            match node.kind() {
                "object" | "array" => {
                    if !self.spend(node.named_child_count()) {
                        return false;
                    }
                    pending.extend(children(node));
                }
                "pair" | "assignment_expression" => {
                    pending.extend(field(node, "value").or_else(|| field(node, "right")))
                }
                "member_expression" | "subscript_expression" => {
                    pending.extend(field(node, "object"))
                }
                "ternary_expression" => pending.extend(
                    ["consequence", "alternative"]
                        .into_iter()
                        .filter_map(|arm| field(node, arm)),
                ),
                "binary_expression" if matches!(operator(node, self.file), "||" | "??" | "&&") => {
                    pending.extend(self.binary(node).into_iter().flatten());
                }
                _ => {}
            }
        }
        false
    }

    pub(super) fn literal_block(&mut self, node: Node<'a>, depth: usize) -> bool {
        if depth == 8 || !self.spend(1) {
            return false;
        }
        let node = peel(node);
        if node.kind() == "array" {
            let mut fields = children(node).peekable();
            return fields.peek().is_some()
                && fields.all(|item| {
                    self.spend(1) && self.string(item).is_some_and(|s| RESERVED.contains(&s))
                });
        }
        if node.kind() == "new_expression"
            && field(node, "constructor").is_some_and(|callee| self.text(callee) == "Set")
        {
            return self
                .args::<1>(node)
                .is_some_and(|[parts]| self.literal_block(parts, depth + 1));
        }
        identifier(node)
            && self
                .initializer(node)
                .is_some_and(|value| self.literal_block(value, depth + 1))
    }

    pub(super) fn regex(&mut self, node: Node<'a>, depth: usize) -> Option<Node<'a>> {
        if depth == 8 || !self.spend(1) {
            return None;
        }
        if node.kind() == "regex" {
            return Some(node);
        }
        let value = self.initializer(node)?;
        self.regex(value, depth + 1)
    }

    fn index_of_rejects(
        &mut self,
        call: Node<'a>,
        offset: Node<'a>,
        op: &str,
        accepts: bool,
        key: Node<'a>,
    ) -> bool {
        matches!((op, accepts), ("!==", false) | ("===", true))
            && self.text(offset) == "-1"
            && self.method(call).is_some_and(|(receiver, method)| {
                method == "indexOf" && self.literal_block(receiver, 0)
            })
            && self
                .args::<1>(call)
                .is_some_and(|[value]| self.matched_key(value, key, call))
    }

    pub(super) fn name_call(&mut self, call: Node<'a>, key: Node<'a>, accepts: bool) -> bool {
        if call.kind() != "call_expression" {
            return false;
        }
        if let Some((receiver, method)) = self.method(call) {
            if accepts {
                return false;
            }
            if matches!(method, "includes" | "has") && self.literal_block(receiver, 0) {
                return self
                    .args::<1>(call)
                    .is_some_and(|[value]| self.matched_key(value, key, call));
            }
            if method == "test" {
                let Some(regex) = self.regex(receiver, 0) else {
                    return false;
                };
                let Some([value]) = self.args(call) else {
                    return false;
                };
                if !self.matched_key(value, key, call) {
                    return false;
                }
                if field(regex, "flags").is_some_and(|flags| self.text(flags).contains(['g', 'y']))
                {
                    self.refuse(call, "unproved_native_identity");
                }
                return field(regex, "pattern").is_some_and(|pattern| {
                    let text = self.text(pattern);
                    RESERVED.iter().any(|s| text.contains(s))
                        || text
                            .strip_prefix('^')
                            .and_then(|s| s.strip_suffix('$'))
                            .is_some_and(|word| {
                                !word.is_empty() && word.bytes().all(|b| b.is_ascii_alphanumeric())
                            })
                });
            }
            if method == "call"
                && field(receiver, "property").is_some_and(|p| self.text(p) == "hasOwnProperty")
                && field(receiver, "object").is_some_and(|o| self.is_object_prototype(o))
            {
                return self.args::<2>(call).is_some_and(|[object, value]| {
                    self.is_object_prototype(object) && self.matched_key(value, key, call)
                });
            }
        }
        self.denial_helper(call, key, accepts)
    }

    fn denial_helper(&mut self, call: Node<'a>, key: Node<'a>, accepts: bool) -> bool {
        let Some(callee) = field(call, "function").filter(|callee| identifier(*callee)) else {
            return false;
        };
        let Some(decl) = self
            .bare_declaration(callee)
            .and_then(|(_, name)| name.parent())
        else {
            return false;
        };
        if decl.kind() != "function_declaration" {
            return false;
        }
        let Some(param) = field(decl, "parameters").and_then(|list| children(list).next()) else {
            return false;
        };
        let Some(ret) = field(decl, "body").and_then(|body| children(body).next()) else {
            return false;
        };
        if !identifier(param) || ret.kind() != "return_statement" || !accepts {
            return false;
        }
        let Some(condition) = children(ret).next() else {
            return false;
        };
        let Some([arg]) = self.args(call) else {
            return false;
        };
        self.contains_literal_comparison(condition, param) && self.matched_key(arg, key, call)
    }

    pub(super) fn contains_literal_comparison(
        &mut self,
        expression: Node<'a>,
        parameter: Node<'a>,
    ) -> bool {
        let mut nodes = vec![expression];
        while let Some(node) = nodes.pop().map(peel) {
            if !self.spend(1) {
                return false;
            }
            if node.kind() == "binary_expression" {
                let Some([left, right]) = self.binary(node) else {
                    continue;
                };
                let op = operator(node, self.file);
                if op == "&&" || op == "||" {
                    nodes.extend([left, right]);
                } else if matches!(op, "===" | "!==")
                    && identifier(left)
                    && self.text(left) == self.text(parameter)
                    && self.string(right).is_some()
                {
                    return true;
                }
            }
        }
        false
    }
}
