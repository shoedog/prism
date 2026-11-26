//! Source-local shared-slot checks for otherwise bounded receiver keys.
use super::proofs::SHARED_LINKS;
use super::*;

impl<'a> Classifier<'a, '_> {
    pub(super) fn no_visible_shared_slot(&self, receiver: Node<'a>, target: Node<'a>) -> bool {
        let receiver = peel(receiver);
        if !self.no_prototype_value(receiver, 0) {
            return false;
        }
        if !identifier(receiver) {
            return true;
        }
        let Some(binding) = self
            .analysis
            .read(receiver)
            .and_then(|read| read.binding.as_ref())
        else {
            return false;
        };
        let Some(owner) = self
            .analysis
            .lexical
            .binding_owners
            .get(&binding.start_byte)
        else {
            return false;
        };
        self.binding_references(binding).is_some_and(|references| {
            references
                .into_iter()
                .all(|reference| self.closed_slot_use(reference, receiver, target, owner))
        })
    }

    fn closed_slot_use(
        &self,
        reference: Node<'a>,
        receiver: Node<'a>,
        target: Node<'a>,
        owner: &AstSpan,
    ) -> bool {
        let mut value = reference;
        for depth in 0..RECEIVER_DEPTH_LIMIT {
            let Some(parent) = self.expression_use(value) else {
                return false;
            };
            if value == receiver
                || parent == target && field(parent, "object").map(peel) == Some(value)
            {
                return true;
            }
            match parent.kind() {
                "member_expression" | "subscript_expression"
                    if field(parent, "object").map(peel) == Some(value)
                        && !self.shared_link(parent) =>
                {
                    value = parent;
                }
                "assignment_expression" if field(parent, "left").map(peel) == Some(value) => {
                    // A primitive write through a child cannot install a
                    // shared value, even if the child refers back to the root.
                    if value != reference {
                        return depth >= 2
                            && field(parent, "right").map(peel).is_some_and(|rhs| {
                                matches!(
                                    rhs.kind(),
                                    "number" | "string" | "true" | "false" | "null"
                                )
                            });
                    }
                    // A direct standalone rebinding has no aliasing result.
                    // All reaching versions are already checked at the lookup.
                    return self
                        .expression_use(parent)
                        .is_some_and(|use_node| use_node.kind() == "expression_statement");
                }
                "if_statement" if field(parent, "condition").map(peel) == Some(value) => {
                    return true;
                }
                "binary_expression"
                    if value != reference
                        && matches!(
                            operator(parent, self.file),
                            "==" | "!=" | "===" | "!==" | "<" | "<=" | ">" | ">=" | "+" | "-"
                        ) =>
                {
                    return true;
                }
                "unary_expression" => {
                    return matches!(operator(parent, self.file), "!" | "typeof");
                }
                "variable_declarator" => {
                    // Reading a child does not expose the table itself.
                    return value != reference && field(parent, "value").map(peel) == Some(value);
                }
                "return_statement" | "pair" | "object" | "array" => {
                    return self.slot_return(value, owner);
                }
                "arguments" if value == reference => return self.closed_slot_call(parent, value),
                _ => return false,
            }
        }
        false
    }

    fn slot_return(&self, mut value: Node<'a>, owner: &AstSpan) -> bool {
        for _ in 0..RECEIVER_DEPTH_LIMIT {
            let Some(parent) = self.expression_use(value) else {
                return false;
            };
            match parent.kind() {
                "pair" if field(parent, "value").map(peel) == Some(value) => value = parent,
                "object" | "array" => value = parent,
                // Returning from the allocating callable exits its activation.
                // A nested callback can instead make the value available in
                // another execution of the target within the same activation.
                "return_statement" => {
                    return self
                        .enclosing(parent, |kind| {
                            kind == "program" || crate::ast::is_js_ts_function_like(kind)
                        })
                        .is_some_and(|return_owner| {
                            AstSpan::from(return_owner) == *owner
                                && !self.analysis.inline_owners.contains_key(owner)
                        });
                }
                _ => return false,
            }
        }
        false
    }

    fn closed_slot_call(&self, args: Node<'a>, receiver: Node<'a>) -> bool {
        let Some(call) = args.parent() else {
            return false;
        };
        if !self.static_call(call, "Object", "setPrototypeOf")
            || !self
                .expression_use(call)
                .is_some_and(|use_node| use_node.kind() == "expression_statement")
        {
            return false;
        }
        let Some([base, prototype_ref]) = self.args(call) else {
            return false;
        };
        let prototype_ref = peel(prototype_ref);
        let Some(binding) = self
            .analysis
            .read(prototype_ref)
            .and_then(|read| read.binding.as_ref())
        else {
            return false;
        };
        let Some(prototype) = self.initializer(prototype_ref) else {
            return false;
        };
        // The modeled sparse-array fallback is a local data record with
        // numeric slots. An opaque prototype or writer supplies no proof.
        peel(base) == receiver
            && prototype.kind() == "object"
            && self
                .binding_references(binding)
                .is_some_and(|references| references == [prototype_ref])
            && children(prototype).all(|pair| {
                self.proof_charge(1)
                    && pair.kind() == "pair"
                    && self.pair_name(pair).is_some_and(|name| {
                        !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_digit())
                    })
                    && field(pair, "value").is_some_and(|value| self.no_prototype_value(value, 0))
            })
    }

    fn no_prototype_value(&self, node: Node<'a>, depth: usize) -> bool {
        if depth == 24 || !self.proof_charge(1) {
            return false;
        }
        let node = peel(node);
        match node.kind() {
            "identifier" | "shorthand_property_identifier" => {
                let Some(read) = self
                    .analysis
                    .read(node)
                    .filter(|read| read.resolution_complete)
                else {
                    return false;
                };
                !read.reaching_versions.is_empty()
                    && read.reaching_versions.iter().all(|site| {
                        self.proof_charge(1)
                            && !self.analysis.defaults.contains_key(&site.start_byte)
                            && match self.origin(site) {
                                Origin::Value(value) => self.no_prototype_value(value, depth + 1),
                                Origin::Unknown if read.binding.as_ref() == Some(site) => {
                                    self.external_slot_input(site)
                                }
                                _ => false,
                            }
                    })
            }
            "array" => children(node).all(|value| self.no_prototype_value(value, depth + 1)),
            "object" => children(node).all(|pair| {
                self.proof_charge(1)
                    && pair.kind() == "pair"
                    && (self
                        .pair_name(pair)
                        .is_some_and(|name| !SHARED_LINKS.contains(&name))
                        || field(pair, "key").is_some_and(|key| {
                            key.kind() == "computed_property_name"
                                && children(key).next().is_some_and(|key| self.benign_key(key))
                        }))
                    && field(pair, "value")
                        .is_some_and(|value| self.no_prototype_value(value, depth + 1))
            }),
            "ternary_expression" => ["consequence", "alternative"].into_iter().all(|arm| {
                field(node, arm).is_some_and(|value| self.no_prototype_value(value, depth + 1))
            }),
            "call_expression" => self.closed_slot_initializer(node, depth),
            "number" | "string" | "true" | "false" | "null" => true,
            _ => false,
        }
    }

    fn external_slot_input(&self, site: &AstSpan) -> bool {
        let Some(decl) = self.node_at(site) else {
            return false;
        };
        let Some(parent) = decl.parent() else {
            return false;
        };
        if matches!(
            parent.kind(),
            "import_clause" | "import_specifier" | "namespace_import"
        ) {
            return true;
        }
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

    fn closed_slot_initializer(&self, call: Node<'a>, depth: usize) -> bool {
        if self.static_call(call, "Object", "create") {
            return self
                .args::<1>(call)
                .is_some_and(|[prototype]| peel(prototype).kind() == "null");
        }
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
                    && field(mapper, "body")
                        .is_some_and(|body| self.private_literal(body, depth + 1))
            });
        }
        self.method(call, "fill").is_some_and(|array| {
            array.kind() == "call_expression"
                && field(array, "function").is_some_and(|callee| self.intrinsic(callee, "Array"))
                && self
                    .args::<1>(array)
                    .is_some_and(|[length]| peel(length).kind() == "number")
                && self.args::<1>(call).is_some_and(|[value]| {
                    let value = self.initializer(value).unwrap_or_else(|| peel(value));
                    self.private_literal(value, depth + 1)
                })
        })
    }
}
