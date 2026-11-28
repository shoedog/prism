//! Closed receiver forms for bounded-key exclusions.
//!
//! Slot proofs model standard built-in identity for Array.from, Array(n).fill,
//! Object.create and Object.defineProperty. Runtime mutation is outside the
//! model. Installation also refuses direct source writes to Object.defineProperty.
use super::*;

impl<'a> Classifier<'a, '_> {
    pub(super) fn bounded_key_receiver(&self, receiver: Node<'a>) -> bool {
        let receiver = peel(receiver);
        if !identifier(receiver) {
            return self.direct_slot_source(receiver);
        }
        let Some(read) = self
            .analysis
            .read(receiver)
            .filter(|read| read.resolution_complete)
        else {
            return false;
        };
        let [site] = read.reaching_versions.as_slice() else {
            return false;
        };
        let Some(binding) = &read.binding else {
            return false;
        };
        if self.script_global_binding(binding)
            || self.analysis.defaults.contains_key(&site.start_byte)
        {
            return false;
        }
        match self.origin(site) {
            Origin::Value(value) => {
                self.enclosing(value, |kind| {
                    kind == "program" || crate::ast::is_js_ts_function_like(kind)
                })
                .is_some_and(|owner| AstSpan::from(owner) == read.owner)
                    && self.direct_slot_source(peel(value))
                    && (self.analysis.unmutated_receiver(receiver)
                        || self.direct_receiver_uses(binding, receiver, &read.owner))
            }
            Origin::Unknown if binding == site => {
                self.formal_slot_input(site)
                    && self.direct_receiver_uses(site, receiver, &read.owner)
            }
            _ => false,
        }
    }

    fn direct_slot_source(&self, value: Node<'a>) -> bool {
        self.private_literal(value, 0)
            || value.kind() == "call_expression" && self.closed_slot_initializer(value)
    }

    fn formal_slot_input(&self, site: &AstSpan) -> bool {
        let Some(decl) = self.node_at(site) else {
            return false;
        };
        let Some(parent) = decl.parent() else {
            return false;
        };
        if is_owner(parent) {
            return field(parent, "parameter") == Some(decl);
        }
        self.enclosing(decl, |kind| {
            matches!(
                kind,
                "formal_parameters" | "variable_declarator" | "statement_block"
            ) || crate::ast::is_js_ts_function_like(kind)
        })
        .is_some_and(|parent| parent.kind() == "formal_parameters")
    }

    fn direct_receiver_uses(&self, binding: &AstSpan, receiver: Node<'a>, owner: &AstSpan) -> bool {
        if self
            .analysis
            .lexical
            .binding_owners
            .get(&binding.start_byte)
            != Some(owner)
            || self.analysis.inline_owners.contains_key(owner)
        {
            return false;
        }
        self.binding_references(binding).is_some_and(|references| {
            references.into_iter().all(|reference| {
                if reference == receiver {
                    return true;
                }
                let Some(use_node) = self.expression_use(reference) else {
                    return false;
                };
                if !self
                    .enclosing(use_node, |kind| {
                        kind == "program" || crate::ast::is_js_ts_function_like(kind)
                    })
                    .is_some_and(|found| AstSpan::from(found) == *owner)
                {
                    return false;
                }
                if use_node.kind() == "return_statement" {
                    return true;
                }
                use_node.kind() == "member_expression"
                    && field(use_node, "object").map(peel) == Some(reference)
                    && field(use_node, "property").is_some_and(|name| self.text(name) == "length")
                    && self
                        .expression_use(use_node)
                        .is_some_and(|parent| match parent.kind() {
                            "binary_expression" => matches!(
                                operator(parent, self.file),
                                "==" | "!=" | "===" | "!==" | "<" | "<=" | ">" | ">=" | "+" | "-"
                            ),
                            "unary_expression" => {
                                matches!(operator(parent, self.file), "!" | "typeof")
                            }
                            _ => false,
                        })
            })
        })
    }

    fn closed_slot_initializer(&self, call: Node<'a>) -> bool {
        if self.static_call(call, "Array", "from") {
            return self.args::<2>(call).is_some_and(|[source, mapper]| {
                let source = peel(source);
                let mapper = peel(mapper);
                source.kind() == "object"
                    && children(source).next().is_some_and(|pair| {
                        pair.kind() == "pair" && self.pair_name(pair) == Some("length")
                    })
                    && children(source).nth(1).is_none()
                    && mapper.kind() == "arrow_function"
                    && is_inline_callback(mapper)
                    && field(mapper, "body").is_some_and(|body| self.private_literal(body, 0))
            });
        }
        self.method(call, "fill").is_some_and(|array| {
            array.kind() == "call_expression"
                && field(array, "function")
                    .is_some_and(|callee| self.unshadowed_name(callee, "Array"))
                && self
                    .args::<1>(array)
                    .is_some_and(|[length]| peel(length).kind() == "number")
                && self.args::<1>(call).is_some_and(|[value]| {
                    let value = peel(value);
                    if !identifier(value) {
                        return self.private_literal(value, 0);
                    }
                    self.initializer(value).is_some_and(|literal| {
                        self.private_literal(literal, 0)
                            && self
                                .analysis
                                .read(value)
                                .and_then(|read| read.binding.as_ref())
                                .and_then(|binding| self.binding_references(binding))
                                .is_some_and(|references| references == [value])
                    })
                })
        })
    }
}
