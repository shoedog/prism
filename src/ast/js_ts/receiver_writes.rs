//! JS/TS receiver writes: constructor field types, member writes, closer bindings.

use crate::ast::{is_js_ts_function_like, is_plain_ident, JsTsReceiverBindingEvidence, ParsedFile};
use std::collections::BTreeSet;
use tree_sitter::Node;

impl ParsedFile {
    /// Own constructor assignment establishes a bounded instance-field invariant.
    /// Dynamic-this callables and field initializer evaluation are not instances.
    pub(super) fn js_ts_constructor_field_type(&self, receiver: Node<'_>) -> Option<String> {
        let field = receiver.child_by_field_name("property")?;
        if field.kind() != "property_identifier" {
            return None;
        }
        let field = self.node_text(&field);
        let mut current = receiver.parent();
        let member = loop {
            let node = current?;
            if node.kind() == "method_definition" {
                if node.parent()?.kind() != "class_body" {
                    return None;
                }
                break node;
            }
            if node.kind() == "arrow_function" {
                let parent = node.parent()?;
                if matches!(
                    parent.kind(),
                    "field_definition" | "public_field_definition"
                ) && parent.parent()?.kind() == "class_body"
                    && parent.child_by_field_name("value") == Some(node)
                {
                    break parent;
                }
            } else if is_js_ts_function_like(node.kind()) || node.kind() == "class_body" {
                return None;
            }
            current = node.parent();
        };
        if self.js_ts_method_is_static(&member) {
            return None;
        }
        if let Some(params) = member.child_by_field_name("parameters") {
            let mut cursor = params.walk();
            if params.named_children(&mut cursor).any(|p| {
                p.child_by_field_name("pattern")
                    .or_else(|| p.child_by_field_name("name"))
                    .is_some_and(|n| self.node_text(&n) == "this")
            }) {
                return None;
            }
        }
        let body = member.parent()?;
        let class = body.parent()?;
        let mut cursor = class.walk();
        if class.kind() != "class_declaration"
            || body.has_error()
            || class.children(&mut cursor).any(|n| n.kind() == "decorator")
        {
            return None;
        }
        let mut constructor = None;
        let mut slots = 0;
        let mut cursor = body.walk();
        for slot in body.named_children(&mut cursor) {
            if self.js_ts_method_is_static(&slot) {
                continue;
            }
            let key = slot
                .child_by_field_name("name")
                .or_else(|| slot.child_by_field_name("property"));
            if slot.kind() == "index_signature" {
                return None;
            }
            let Some(key) = key else {
                continue;
            };
            if key.kind() == "computed_property_name" {
                return None;
            }
            if self.node_text(&key) == "constructor" {
                if slot.kind() != "method_definition" || constructor.replace(slot).is_some() {
                    return None;
                }
            }
            if self.node_text(&key).trim_matches(['\'', '"']) == field {
                slots += 1;
                let mut cursor = slot.walk();
                if slots > 1
                    || key.kind() != "property_identifier"
                    || !matches!(slot.kind(), "field_definition" | "public_field_definition")
                    || slot.child_by_field_name("value").is_some()
                    || slot.children(&mut cursor).any(|n| {
                        n.kind() == "decorator"
                            || n.kind() == "?"
                            || matches!(
                                self.node_text(&n),
                                "private" | "protected" | "declare" | "abstract"
                            )
                    })
                {
                    return None;
                }
            }
        }
        // Without an own field, a base-class accessor can intercept assignment.
        let mut cursor = class.walk();
        if slots == 0
            && class
                .named_children(&mut cursor)
                .any(|n| n.kind() == "class_heritage")
        {
            return None;
        }
        let constructor = constructor?;
        let ctor_body = constructor.child_by_field_name("body")?;
        fn returns(node: Node<'_>) -> bool {
            if is_js_ts_function_like(node.kind()) {
                return false;
            }
            if node.kind() == "return_statement" {
                return true;
            }
            let mut cursor = node.walk();
            let found = node.named_children(&mut cursor).any(returns);
            found
        }
        if returns(ctor_body) {
            return None;
        }
        let mut initialization = None;
        let mut cursor = ctor_body.walk();
        for statement in ctor_body.named_children(&mut cursor) {
            if statement.kind() != "expression_statement" {
                continue;
            }
            let Some(assignment) = statement
                .named_child(0)
                .filter(|n| n.kind() == "assignment_expression")
            else {
                continue;
            };
            let left = assignment.child_by_field_name("left")?;
            if left.kind() == "member_expression"
                && left
                    .child_by_field_name("object")
                    .is_some_and(|n| n.kind() == "this")
                && left.child_by_field_name("property").is_some_and(|n| {
                    n.kind() == "property_identifier" && self.node_text(&n) == field
                })
                && initialization.replace(assignment).is_some()
            {
                return None;
            }
        }
        let initialization = initialization?;
        if member == constructor && receiver.start_byte() < initialization.end_byte() {
            return None;
        }
        if self.js_ts_constructor_field_written(body, field, initialization.id()) {
            return None;
        }
        self.js_ts_direct_new_constructor(initialization.child_by_field_name("right")?, constructor)
    }

    /// Include nested member writes and destructuring without text-based object
    /// comparison, so whitespace/parentheses cannot hide a field mutation.
    fn js_ts_target_writes_this_field(&self, target: Node<'_>, field: &str) -> bool {
        if self.js_ts_target_writes_member(target, "this", Some(field)) {
            return true;
        }
        if matches!(target.kind(), "member_expression" | "subscript_expression") {
            return target
                .child_by_field_name("object")
                .is_some_and(|n| self.js_ts_target_writes_this_field(n, field));
        }
        if matches!(
            target.kind(),
            "pair_pattern" | "assignment_pattern" | "object_assignment_pattern"
        ) {
            return target
                .child_by_field_name(if target.kind() == "pair_pattern" {
                    "value"
                } else {
                    "left"
                })
                .is_some_and(|n| self.js_ts_target_writes_this_field(n, field));
        }
        if matches!(
            target.kind(),
            "object_pattern" | "array_pattern" | "rest_pattern" | "parenthesized_expression"
        ) {
            let mut cursor = target.walk();
            return target
                .named_children(&mut cursor)
                .any(|n| self.js_ts_target_writes_this_field(n, field));
        }
        false
    }

    fn js_ts_constructor_field_written(
        &self,
        node: Node<'_>,
        field: &str,
        initialization: usize,
    ) -> bool {
        if node.id() != initialization
            && self
                .js_ts_write_target(node)
                .is_some_and(|n| self.js_ts_target_writes_this_field(n, field))
        {
            return true;
        }
        // Explicit reflective mutators are barriers; this is not alias analysis.
        if node.kind() == "call_expression" {
            if let Some(callee) = node
                .child_by_field_name("function")
                .filter(|n| matches!(n.kind(), "member_expression" | "subscript_expression"))
            {
                let object = callee
                    .child_by_field_name("object")
                    .map(|n| self.node_text(&n));
                let method = callee
                    .child_by_field_name("property")
                    .or_else(|| callee.child_by_field_name("index"))
                    .map(|n| self.node_text(&n).trim_matches(['\'', '"']));
                if matches!(object, Some("Object" | "Reflect"))
                    && matches!(
                        method,
                        Some(
                            "assign"
                                | "defineProperty"
                                | "defineProperties"
                                | "set"
                                | "deleteProperty"
                                | "setPrototypeOf"
                        )
                    )
                    && node
                        .child_by_field_name("arguments")
                        .and_then(|n| {
                            let mut cursor = n.walk();
                            let first = n
                                .named_children(&mut cursor)
                                .find(|child| self.is_positional_argument_node(*child));
                            first
                        })
                        .is_some_and(|mut n| {
                            while n.kind() == "parenthesized_expression" {
                                let mut cursor = n.walk();
                                let Some(inner) = n
                                    .named_children(&mut cursor)
                                    .find(|child| self.is_positional_argument_node(*child))
                                else {
                                    return true;
                                };
                                n = inner;
                            }
                            n.kind() == "this" || self.js_ts_target_writes_this_field(n, field)
                        })
                {
                    return true;
                }
            }
        }
        let mut cursor = node.walk();
        let written = node
            .named_children(&mut cursor)
            .any(|n| self.js_ts_constructor_field_written(n, field, initialization));
        written
    }

    pub(in crate::ast) fn js_ts_write_target<'a>(&self, node: Node<'a>) -> Option<Node<'a>> {
        let writes = matches!(
            node.kind(),
            "assignment_expression"
                | "augmented_assignment_expression"
                | "update_expression"
                | "for_in_statement"
                | "for_of_statement"
                | "for_await_statement"
        ) || (node.kind() == "unary_expression"
            && node
                .child_by_field_name("operator")
                .is_some_and(|n| self.node_text(&n) == "delete"));
        writes
            .then(|| {
                node.child_by_field_name("left")
                    .or_else(|| node.child_by_field_name("argument"))
                    .or_else(|| node.named_child(0))
            })
            .flatten()
    }

    /// Inspect assignment targets, not RHS reads/default initializers.
    pub(super) fn js_ts_target_writes_member(
        &self,
        target: Node<'_>,
        receiver: &str,
        member: Option<&str>,
    ) -> bool {
        if matches!(target.kind(), "member_expression" | "subscript_expression") {
            return target.child_by_field_name("object").is_some_and(|mut n| {
                while n.kind() == "parenthesized_expression" && n.named_child_count() == 1 {
                    n = n.named_child(0).unwrap();
                }
                self.node_text(&n) == receiver
            }) && (target.kind() == "subscript_expression"
                || member.is_none()
                || target
                    .child_by_field_name("property")
                    .is_some_and(|n| Some(self.node_text(&n)) == member));
        }
        if matches!(
            target.kind(),
            "pair_pattern" | "assignment_pattern" | "object_assignment_pattern"
        ) {
            return target
                .child_by_field_name(if target.kind() == "pair_pattern" {
                    "value"
                } else {
                    "left"
                })
                .is_some_and(|n| self.js_ts_target_writes_member(n, receiver, member));
        }
        if matches!(
            target.kind(),
            "object_pattern" | "array_pattern" | "rest_pattern" | "parenthesized_expression"
        ) {
            let mut cursor = target.walk();
            return target
                .named_children(&mut cursor)
                .any(|n| self.js_ts_target_writes_member(n, receiver, member));
        }
        false
    }

    pub(super) fn js_ts_direct_new_constructor(
        &self,
        value: Node<'_>,
        function_scope: Node<'_>,
    ) -> Option<String> {
        if value.kind() != "new_expression" {
            return None;
        }
        let constructor = value.child_by_field_name("constructor")?;
        if constructor.kind() != "identifier" {
            return None;
        }
        let text = self.node_text(&constructor);
        if !is_plain_ident(text) {
            return None;
        }
        let mut scope = Some(function_scope);
        while let Some(node) = scope {
            if node.kind() == "class"
                && node
                    .child_by_field_name("name")
                    .is_some_and(|name| self.node_text(&name) == text)
            {
                return None;
            }
            if is_js_ts_function_like(node.kind())
                && self
                    .js_ts_scope_receiver_binding_evidence(node, &constructor, text, false, true)
                    .is_some()
            {
                return None;
            }
            scope = node.parent();
        }
        let module_evidence = self.js_ts_scope_receiver_binding_evidence(
            self.tree.root_node(),
            &constructor,
            text,
            false,
            false,
        );
        (matches!(
            module_evidence,
            Some(JsTsReceiverBindingEvidence::ClassOwner)
        ) || (module_evidence.is_none()
            && self.js_ts_esm_named_imports(Some(text)).contains(text)))
        .then(|| text.to_string())
    }

    pub(super) fn js_ts_receiver_written_between(
        &self,
        node: Node<'_>,
        binding_scope_id: usize,
        receiver: &Node<'_>,
        receiver_name: &str,
        after_byte: usize,
    ) -> bool {
        let mut before_byte = receiver.start_byte();
        let mut ancestor = receiver.parent();
        while let Some(scope) = ancestor {
            if is_js_ts_function_like(scope.kind()) {
                break;
            }
            if matches!(
                scope.kind(),
                "for_statement"
                    | "for_in_statement"
                    | "for_of_statement"
                    | "for_await_statement"
                    | "while_statement"
                    | "do_statement"
            ) && after_byte <= scope.start_byte()
            {
                before_byte = before_byte.max(scope.end_byte());
            }
            ancestor = scope.parent();
        }
        if node.start_byte() >= before_byte || node.end_byte() <= after_byte {
            return false;
        }
        if let Some(target) = self.js_ts_write_target(node) {
            let mut written_names = BTreeSet::new();
            self.collect_js_ts_binding_pattern_names(target, &mut written_names);
            if (written_names.contains(receiver_name)
                || self.js_ts_target_writes_member(target, receiver_name, None))
                && !self.js_ts_receiver_has_closer_binding(&target, receiver_name, binding_scope_id)
            {
                return true;
            }
        }
        let mut cursor = node.walk();
        let found = node.named_children(&mut cursor).any(|child| {
            self.js_ts_receiver_written_between(
                child,
                binding_scope_id,
                receiver,
                receiver_name,
                after_byte,
            )
        });
        found
    }

    pub(in crate::ast) fn js_ts_receiver_has_closer_binding(
        &self,
        target: &Node<'_>,
        receiver_name: &str,
        binding_scope_id: usize,
    ) -> bool {
        self.js_ts_has_closer_binding(target, receiver_name, binding_scope_id, false)
    }

    /// Write proofs distinguish expression self bindings from declarations.
    /// Receiver lookup also uses source names, but retains declaration-name
    /// shadow refusal; do not conflate that policy with declaration writes.
    pub(in crate::ast) fn js_ts_has_closer_binding(
        &self,
        target: &Node<'_>,
        receiver_name: &str,
        binding_scope_id: usize,
        source_self_bindings: bool,
    ) -> bool {
        let mut current = target.parent();
        while let Some(scope) = current {
            if scope.id() == binding_scope_id {
                return false;
            }
            if is_js_ts_function_like(scope.kind()) {
                let binds = if source_self_bindings {
                    let mut parameters = BTreeSet::new();
                    self.collect_js_ts_parameter_bindings(scope, &mut parameters);
                    parameters.contains(receiver_name)
                        || (matches!(scope.kind(), "function_expression" | "generator_function")
                            && scope
                                .child_by_field_name("name")
                                .is_some_and(|n| self.node_text(&n) == receiver_name))
                        || self.js_ts_receiver_binding_reaches_call(
                            scope,
                            scope.id(),
                            target,
                            receiver_name,
                        )
                } else {
                    self.js_ts_function_scope_binds_receiver(&scope, target, receiver_name)
                };
                if binds {
                    return true;
                }
            } else if self.language.is_scope_block(scope.kind())
                || matches!(
                    scope.kind(),
                    "for_statement"
                        | "for_in_statement"
                        | "for_of_statement"
                        | "for_await_statement"
                        | "switch_statement"
                        | "switch_body"
                        | "catch_clause"
                )
            {
                if self.js_ts_receiver_binding_reaches_call(
                    scope,
                    scope.id(),
                    target,
                    receiver_name,
                ) {
                    return true;
                }
            }
            current = scope.parent();
        }
        true
    }
}
