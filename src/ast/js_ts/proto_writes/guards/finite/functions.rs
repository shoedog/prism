//! Bounded summaries for direct assertions and pure string normalization.
use super::*;

impl<'a, F: FnMut(usize) -> bool> Proof<'a, '_, F> {
    pub(super) fn assertion(&mut self, step: Selection<'a>) -> bool {
        let mut at = step.effect;
        while let Some(parent) = at.parent() {
            if !self.spend(1) || is_owner(parent) {
                break;
            }
            if parent.kind() == "statement_block" {
                let mut prev = at.prev_named_sibling();
                while let Some(statement) = prev {
                    if !self.spend(1) {
                        return false;
                    }
                    prev = statement.prev_named_sibling();
                    if statement.kind() != "expression_statement" {
                        continue;
                    }
                    let Some(call) = children(statement).next().map(peel) else {
                        continue;
                    };
                    let Some([arg]) = self.args(call) else {
                        continue;
                    };
                    if !self.same_key(arg, step.key) {
                        continue;
                    }
                    if self.asserts_string(call) {
                        return true;
                    }
                }
            }
            at = parent;
        }
        false
    }

    pub(in super::super) fn asserts_string(&mut self, call: Node<'a>) -> bool {
        let Some(helper) = self.local_helper(call) else {
            return false;
        };
        if helper.kind() != "function_declaration" {
            return false;
        }
        let Some(parameter) = self.single_parameter(helper) else {
            return false;
        };
        let Some(body) = field(helper, "body") else {
            return false;
        };
        let mut statements = children(body);
        let Some(check) = statements
            .next()
            .filter(|stmt| stmt.kind() == "if_statement")
        else {
            self.refuse(call, "unsupported_predicate");
            return false;
        };
        if statements.next().is_some() || field(check, "alternative").is_some() {
            return false;
        }
        let Some(condition) = field(check, "condition") else {
            return false;
        };
        let edge = Edge {
            condition,
            guard: check,
            accepts: false,
            complete: true,
        };
        if self
            .finite_strings(edge, |proof, value| proof.parameter_use(value, parameter))
            .is_none()
        {
            return false;
        }
        let Some(exit) = field(check, "consequence").and_then(single_block_statement) else {
            return false;
        };
        if exit.kind() != "throw_statement" {
            self.refuse(call, "wrong_exit");
            return false;
        }
        true
    }

    pub(super) fn normalised(&mut self, step: Selection<'a>, edges: &[Edge<'a>]) -> bool {
        let Some(helper) = self.local_helper(step.key) else {
            return false;
        };
        let Some([argument]) = self.args(step.key) else {
            return false;
        };
        if !self.normalises(helper) {
            return false;
        }
        for edge in edges {
            let mut source = None;
            let allowed = self.finite_strings(*edge, |proof, checked| {
                let Some(conversion) = proof.initializer(checked) else {
                    return false;
                };
                if proof.local_helper(conversion) != Some(helper) {
                    return false;
                }
                let Some([checked_arg]) = proof.args(conversion) else {
                    return false;
                };
                if proof.same_key(checked_arg, argument) {
                    source = Some(checked_arg);
                    return true;
                }
                let reason = if proof.declaration(checked_arg).map(|(binding, _)| binding)
                    == proof.declaration(argument).map(|(binding, _)| binding)
                {
                    "wrong_version"
                } else {
                    "wrong_binding"
                };
                proof.refuse(step.key, reason);
                false
            });
            if allowed.is_none() {
                continue;
            }
            if let Some(source) = source {
                let routes = self.route(source);
                if self.checked_string(source, &routes) {
                    return self.unwritten(step.key, &["String", "prototype", "trim"])
                        && self.unwritten(step.key, &["String", "prototype", "toLowerCase"]);
                }
            }
            self.refuse(step.key, "absent_domain");
        }
        false
    }

    fn normalises(&self, helper: Node<'a>) -> bool {
        let Some(parameter) = self.single_parameter(helper) else {
            return false;
        };
        let Some(body) = field(helper, "body").map(peel) else {
            return false;
        };
        let result = if body.kind() == "statement_block" {
            let Some(ret) =
                single_block_statement(body).filter(|ret| ret.kind() == "return_statement")
            else {
                return false;
            };
            let Some(result) = children(ret).next().map(peel) else {
                return false;
            };
            result
        } else {
            body
        };
        let Some((trim, "toLowerCase")) = self.method(result) else {
            return false;
        };
        let Some((value, "trim")) = self.method(trim) else {
            return false;
        };
        self.args::<0>(result).is_some()
            && self.args::<0>(trim).is_some()
            && self.parameter_use(value, parameter)
    }

    fn local_helper(&mut self, call: Node<'a>) -> Option<Node<'a>> {
        if call.kind() != "call_expression" || field(call, "optional_chain").is_some() {
            return None;
        }
        let callee = peel(field(call, "function")?);
        if !identifier(callee) {
            return None;
        }
        let (binding, decl) = self.declaration(callee)?;
        if self.analysis.proof_writes(&binding)?.len() != 1 {
            return None;
        }
        let function = match decl.parent()?.kind() {
            "function_declaration" => decl.parent()?,
            "variable_declarator" => self.initializer(callee)?,
            _ => return None,
        };
        if !matches!(
            function.kind(),
            "function_declaration" | "arrow_function" | "function_expression"
        ) || super::super::super::has_token(function, "async")
        {
            return None;
        }
        let name = self.analysis.lexical.canonical(self.text(callee));
        if self.natives.written(&[name]) {
            self.refuse(call, "wrong_version");
            return None;
        }
        Some(function)
    }

    pub(in super::super) fn always_throws(&mut self, call: Node<'a>) -> bool {
        self.args::<0>(call).is_some()
            && self.local_helper(call).is_some_and(|helper| {
                helper.kind() == "function_declaration"
                    && field(helper, "parameters")
                        .is_some_and(|params| children(params).next().is_none())
                    && field(helper, "body")
                        .and_then(single_block_statement)
                        .is_some_and(|stmt| stmt.kind() == "throw_statement")
            })
    }

    fn single_parameter(&self, helper: Node<'a>) -> Option<Node<'a>> {
        let params = field(helper, "parameters").or_else(|| field(helper, "parameter"))?;
        let mut parameters = children(params);
        let param = if params.kind() == "formal_parameters" {
            let param = parameters.next()?;
            if parameters.next().is_some() {
                return None;
            }
            param
        } else {
            params
        };
        let param = if param.kind() == "required_parameter" {
            field(param, "pattern")?
        } else {
            param
        };
        identifier(param).then_some(param)
    }

    fn parameter_use(&self, value: Node<'a>, param: Node<'a>) -> bool {
        let Some(read) = self.analysis.read(value) else {
            return false;
        };
        read.binding.as_ref() == Some(&AstSpan::from(param))
            && read.reaching_versions.as_slice() == [AstSpan::from(param)]
    }
}

fn single_block_statement(node: Node<'_>) -> Option<Node<'_>> {
    if node.kind() != "statement_block" {
        return Some(node);
    }
    let mut statements = children(node);
    let statement = statements.next()?;
    statements.next().is_none().then_some(statement)
}
