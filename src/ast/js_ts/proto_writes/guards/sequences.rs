//! Partial checks on earlier values and subscript sequences.
use super::flow::{deferred_field, Edge};
use super::names::RESERVED;
use super::*;

impl<'a, F: FnMut(usize) -> bool> Proof<'a, '_, F> {
    pub(super) fn partial_precheck(
        &mut self,
        step: Selection<'a>,
        edges: &[Edge<'a>],
        input: &GuardInput<'a>,
    ) -> bool {
        for switch in self.earlier_switches(step.effect) {
            if self.switch_blocks_names(switch, step.key) {
                self.refuse(switch, "absent_domain");
                return true;
            }
        }
        for value in self.origin_values(step.key, false) {
            let source = if identifier(value) {
                Some(value)
            } else {
                self.raw_pointer_token(value, 0)
            };
            if let Some(source) = source {
                let earlier = self.route(source);
                if self.has_name_coverage(source, &earlier) {
                    self.refuse(input.target, "incomplete_coverage");
                    return true;
                }
            }
        }

        for source in self.sequence_sources(step.key) {
            if self.sequence_pipeline(source, 0) {
                self.refuse(source, "unproved_native_identity");
                return true;
            }
            let sources = self.sequence_chain(source, 0);
            for edge in edges {
                if self.sequence_predicate(edge, &sources) {
                    self.refuse(edge.guard, "incomplete_coverage");
                    return true;
                }
            }
        }
        false
    }

    fn switch_blocks_names(&mut self, switch: Node<'a>, key: Node<'a>) -> bool {
        let (Some(body), Some(value)) = (field(switch, "body"), field(switch, "value")) else {
            return false;
        };
        if !self.same_key(value, key) {
            return false;
        }
        let mut blocked = false;
        for case in children(body) {
            if !self.spend(1) {
                return false;
            }
            let Some(value) = field(case, "value") else {
                return false;
            };
            if !self
                .string(value)
                .is_some_and(|name| RESERVED.contains(&name))
            {
                if self.string(value).is_some()
                    && children(case).any(|jump| {
                        jump.kind() == "break_statement" && field(jump, "label").is_none()
                    })
                {
                    self.refuse(case, "wrong_exit");
                }
                return false;
            }
            let mut body = children(case).filter(|node| *node != value);
            if let Some(jump) = body.next() {
                if jump.kind() != "return_statement" || body.next().is_some() {
                    self.refuse(switch, "wrong_exit");
                    return false;
                }
                blocked = true;
            }
        }
        blocked
    }

    fn raw_pointer_token(&mut self, value: Node<'a>, depth: usize) -> Option<Node<'a>> {
        if depth == 4 || !self.spend(1) {
            return None;
        }
        let (receiver, "replace") = self.method(value)? else {
            return None;
        };
        let [pattern, replacement] = self.args(value)?;
        let regexp = self.text(pattern);
        if !matches!(
            (regexp, self.string(replacement)?),
            ("/~1/g", "/") | ("/~0/g", "~")
        ) {
            return None;
        }
        if identifier(receiver) {
            Some(receiver)
        } else {
            self.raw_pointer_token(receiver, depth + 1)
        }
    }

    pub(super) fn sequence_sources(&mut self, key: Node<'a>) -> Vec<Node<'a>> {
        let key = peel(key);
        if key.kind() == "subscript_expression" {
            return field(key, "object").into_iter().collect();
        }
        let Some(read) = self.bindings.bare_read(key) else {
            return Vec::new();
        };
        if !self.spend(read.reaching_versions.len()) {
            return Vec::new();
        }
        read.reaching_versions
            .iter()
            .filter_map(|site| self.bindings.element_origin(site))
            .collect()
    }

    fn sequence_chain(&mut self, node: Node<'a>, depth: usize) -> Vec<Node<'a>> {
        if depth == 16 || !self.spend(1) {
            return Vec::new();
        }
        let mut nodes = vec![node];
        if let Some((receiver, "slice" | "split")) = self.method(node) {
            nodes.extend(self.sequence_chain(receiver, depth + 1));
        } else if identifier(node) {
            for origin in self.origin_values(node, false) {
                nodes.extend(self.sequence_chain(origin, depth + 1));
            }
        }
        nodes
    }

    fn sequence_predicate(&mut self, edge: &Edge<'a>, sources: &[Node<'a>]) -> bool {
        let mut pending = vec![(edge.condition, edge.accepts)];
        while let Some((condition, accepts)) = pending.pop() {
            if !self.spend(1) {
                return false;
            }
            let condition = peel(condition);
            if condition.kind() == "unary_expression" && operator(condition, self.file) == "!" {
                pending.extend(field(condition, "argument").map(|value| (value, !accepts)));
                continue;
            }
            if let Some([left, right]) = self.binary(condition) {
                let op = operator(condition, self.file);
                if matches!((op, accepts), ("&&", true) | ("||", false)) {
                    pending.extend([(left, accepts), (right, accepts)]);
                    continue;
                }
                for (value, literal) in [(left, right), (right, left)] {
                    let Some(literal) = self.string(literal) else {
                        continue;
                    };
                    if matches!((op, accepts), ("===", false) | ("!==", true))
                        && RESERVED.contains(&literal)
                        && sources.iter().any(|source| self.same_key(value, *source))
                    {
                        return true;
                    }
                    if matches!((op, accepts), ("===", true) | ("!==", false))
                        && value.kind() == "subscript_expression"
                        && field(value, "index").is_some_and(|index| self.text(index) == "0")
                        && !RESERVED.contains(&literal)
                        && field(value, "object").is_some_and(|base| {
                            sources.iter().any(|source| self.same_key(base, *source))
                        })
                    {
                        return true;
                    }
                }
            }
            if let Some((receiver, method)) = self.method(condition) {
                if matches!(method, "test" | "includes") && !accepts {
                    let positive = if method == "includes" {
                        self.args::<1>(condition).is_some_and(|[name]| {
                            self.string(name)
                                .is_some_and(|name| RESERVED.contains(&name))
                                && sources
                                    .iter()
                                    .any(|source| self.same_key(receiver, *source))
                        })
                    } else {
                        self.args::<1>(condition).is_some_and(|[path]| {
                            self.regex(receiver, 0).is_some_and(|regex| {
                                RESERVED.iter().any(|name| self.text(regex).contains(name))
                            }) && sources.iter().any(|source| self.same_key(path, *source))
                        })
                    };
                    if positive {
                        return true;
                    }
                }
                if matches!((method, accepts), ("every", true) | ("some", false))
                    && sources
                        .iter()
                        .any(|source| self.same_key(receiver, *source))
                    && self
                        .args::<1>(condition)
                        .is_some_and(|[callback]| self.predicate_callback(callback, method))
                {
                    return true;
                }
            }
        }
        false
    }

    fn sequence_pipeline(&mut self, node: Node<'a>, depth: usize) -> bool {
        if depth == 16 || !self.spend(1) {
            return false;
        }
        match self.method(node) {
            Some((receiver, "slice")) => self.sequence_pipeline(receiver, depth + 1),
            Some((_, method @ ("filter" | "map"))) => self
                .args::<1>(node)
                .is_some_and(|[callback]| self.predicate_callback(callback, method)),
            _ if identifier(node) => self
                .origin_values(node, false)
                .into_iter()
                .any(|value| self.sequence_pipeline(value, depth + 1)),
            _ => false,
        }
    }

    fn predicate_callback(&mut self, callback: Node<'a>, method: &str) -> bool {
        let callback = if identifier(callback) {
            let Some(decl) = self
                .bare_declaration(callback)
                .and_then(|(_, name)| name.parent())
                .filter(|decl| decl.kind() == "function_declaration")
            else {
                return false;
            };
            decl
        } else {
            callback
        };
        let Some(params) = field(callback, "parameters").or_else(|| field(callback, "parameter"))
        else {
            return false;
        };
        let Some(param) = (if identifier(params) {
            Some(params)
        } else {
            children(params).next()
        }) else {
            return false;
        };
        let Some(body) = field(callback, "body") else {
            return false;
        };
        let body = if body.kind() == "statement_block" {
            let Some(value) = children(body)
                .next()
                .filter(|stmt| stmt.kind() == "return_statement")
                .and_then(|stmt| children(stmt).next())
            else {
                return false;
            };
            peel(value)
        } else {
            peel(body)
        };
        match method {
            "some" | "every" => self.contains_literal_comparison(body, param),
            "filter" if body.kind() == "unary_expression" && operator(body, self.file) == "!" => {
                field(body, "argument").is_some_and(|call| self.block_list_argument(call, param))
            }
            "map" if body.kind() == "ternary_expression" => {
                field(body, "condition").is_some_and(|call| self.block_list_argument(call, param))
            }
            _ => false,
        }
    }

    fn block_list_argument(&mut self, call: Node<'a>, param: Node<'a>) -> bool {
        self.method(call).is_some_and(|(receiver, method)| {
            method == "includes"
                && self.literal_block(receiver, 0)
                && self.args::<1>(call).is_some_and(|[value]| {
                    self.bindings
                        .bare_read(value)
                        .and_then(|read| read.binding.as_ref())
                        == Some(&AstSpan::from(param))
                })
        })
    }

    pub(super) fn report_misplaced_checks(&mut self, input: &GuardInput<'a>) {
        let store_route = self.route(input.store);
        if field(input.target, "index")
            .is_some_and(|property| self.has_name_coverage(property, &store_route))
        {
            self.refuse(input.target, "incomplete_coverage");
        }
        for step in &input.steps {
            for source in self.sequence_sources(step.key) {
                let chain = self.sequence_chain(source, 0);
                self.report_sequence_after_store(input.store, &chain);
                self.report_earlier_sequence_refusals(source, &chain);
            }
        }
    }

    fn report_sequence_after_store(&mut self, effect: Node<'a>, sources: &[Node<'a>]) {
        let mut child = effect;
        while let Some(parent) = child.parent() {
            if !self.spend(1) || is_owner(parent) || deferred_field(parent, effect) {
                return;
            }
            if parent.kind() == "statement_block" {
                let Some(next) = child
                    .next_named_sibling()
                    .filter(|next| next.kind() == "if_statement")
                else {
                    return;
                };
                let Some(condition) = field(next, "condition") else {
                    return;
                };
                let edge = Edge {
                    guard: next,
                    condition,
                    accepts: false,
                };
                if self.sequence_predicate(&edge, sources) {
                    self.refuse(next, "incomplete_coverage");
                }
                return;
            }
            child = parent;
        }
    }

    fn report_earlier_sequence_refusals(&mut self, source: Node<'a>, sources: &[Node<'a>]) {
        let mut child = source;
        let mut awaited = false;
        while let Some(parent) = child.parent() {
            if !self.spend(1) || is_owner(parent) || deferred_field(parent, source) {
                return;
            }
            if parent.kind() == "statement_block" {
                let mut earlier = child.prev_named_sibling();
                while let Some(statement) = earlier {
                    if !self.spend(1) {
                        return;
                    }
                    earlier = statement.prev_named_sibling();
                    if statement.kind() == "expression_statement" {
                        if let Some(expression) = children(statement).next().map(peel) {
                            awaited |= expression.kind() == "await_expression";
                            if self.for_each_return(expression, sources) {
                                self.refuse(expression, "wrong_owner");
                            }
                        }
                    } else if awaited && statement.kind() == "if_statement" {
                        if let Some(expression) = field(statement, "condition").map(peel) {
                            if self.checked_before_await(expression, sources) {
                                self.refuse(expression, "wrong_version");
                            }
                        }
                    }
                }
            }
            child = parent;
        }
    }

    fn for_each_return(&mut self, call: Node<'a>, sources: &[Node<'a>]) -> bool {
        let Some((receiver, "forEach")) = self.method(call) else {
            return false;
        };
        if !sources
            .iter()
            .any(|source| self.same_binding(receiver, *source))
        {
            return false;
        }
        let Some([callback]) = self.args(call) else {
            return false;
        };
        let Some((parameter, body)) = callback_parameter_and_body(callback) else {
            return false;
        };
        let Some(check) = children(body)
            .next()
            .filter(|check| check.kind() == "if_statement")
        else {
            return false;
        };
        field(check, "consequence").is_some_and(|branch| branch.kind() == "return_statement")
            && field(check, "condition")
                .is_some_and(|test| self.contains_literal_comparison(test, parameter))
    }

    fn checked_before_await(&mut self, test: Node<'a>, sources: &[Node<'a>]) -> bool {
        if test.kind() != "unary_expression" || operator(test, self.file) != "!" {
            return false;
        }
        let Some(call) = field(test, "argument").map(peel) else {
            return false;
        };
        let Some((receiver, "every")) = self.method(call) else {
            return false;
        };
        if !sources
            .iter()
            .any(|source| self.same_binding(receiver, *source))
        {
            return false;
        }
        let Some([callback]) = self.args(call) else {
            return false;
        };
        let Some((parameter, body)) = callback_parameter_and_body(callback) else {
            return false;
        };
        let Some((set, "has")) = self.method(body) else {
            return false;
        };
        let Some([argument]) = self.args(body) else {
            return false;
        };
        self.bindings
            .bare_read(argument)
            .and_then(|read| read.binding.as_ref())
            == Some(&AstSpan::from(parameter))
            && self.initializer(set).is_some_and(|value| {
                value.kind() == "new_expression"
                    && field(value, "constructor").is_some_and(|ctor| self.text(ctor) == "Set")
            })
    }
}

fn callback_parameter_and_body(node: Node<'_>) -> Option<(Node<'_>, Node<'_>)> {
    if !matches!(node.kind(), "arrow_function" | "function_expression") {
        return None;
    }
    let params = field(node, "parameters").or_else(|| field(node, "parameter"))?;
    let parameter = if identifier(params) {
        params
    } else {
        children(params).next()?
    };
    identifier(parameter).then_some((parameter, peel(field(node, "body")?)))
}
