//! JS/TS receiver binding evidence at a call: lexical binding, candidates, method slots.

use crate::ast::{is_js_ts_function_like, is_plain_ident, JsTsReceiverBindingEvidence, ParsedFile};
use crate::languages::Language;
use std::collections::BTreeSet;
use tree_sitter::Node;

impl ParsedFile {
    /// Conservative JS/TS function-scope bindings used to avoid false exact
    /// import-member edges when an imported local is shadowed.
    pub fn js_ts_function_local_bindings(&self, func_node: &Node<'_>) -> BTreeSet<String> {
        if !matches!(
            self.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) {
            return BTreeSet::new();
        }

        let mut out: BTreeSet<String> = BTreeSet::new();
        self.collect_js_ts_parameter_bindings(*func_node, &mut out);
        let root_key = (
            func_node.start_byte(),
            func_node.end_byte(),
            func_node.kind_id(),
        );
        self.collect_js_ts_local_bindings(*func_node, root_key, &mut out);
        out
    }

    /// Whether a simple JS/TS receiver identifier is bound in the lexical scope
    /// that contains this call. This is deliberately a binding fact, not type
    /// recovery: later receiver classification may consume it, while import-
    /// qualified resolution uses it only as a fail-closed shadow guard.
    pub fn js_ts_receiver_lexically_bound_at_call(
        &self,
        func_node: &Node<'_>,
        receiver_node: Option<Node<'_>>,
    ) -> bool {
        if !matches!(
            self.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) {
            return false;
        }
        let Some(receiver) = receiver_node else {
            return false;
        };
        if receiver.kind() != "identifier" {
            return false;
        }
        let receiver_name = self.node_text(&receiver);
        if !is_plain_ident(receiver_name) {
            return false;
        }

        let mut seen_functions = BTreeSet::new();
        let mut current = Some(receiver);
        while let Some(node) = current {
            if is_js_ts_function_like(node.kind()) && seen_functions.insert(node.id()) {
                if self.js_ts_function_scope_binds_receiver(&node, &receiver, receiver_name) {
                    return true;
                }
            }
            current = node.parent();
        }

        if seen_functions.insert(func_node.id())
            && self.js_ts_function_scope_binds_receiver(func_node, &receiver, receiver_name)
        {
            return true;
        }

        let root = self.tree.root_node();
        self.js_ts_receiver_binding_reaches_call(root, root.id(), &receiver, receiver_name)
    }

    /// Call-position-aware JS/TS value-binding evidence for receiver recovery.
    /// The nearest reaching lexical scope wins. Constructor origins are limited
    /// to the call's innermost function; typed parameters may be captured by a
    /// nested callable because their static type remains declared by the outer
    /// signature.
    pub(crate) fn js_ts_receiver_binding_evidence_at_call(
        &self,
        func_node: &Node<'_>,
        receiver_node: Option<Node<'_>>,
    ) -> Option<JsTsReceiverBindingEvidence> {
        if !matches!(
            self.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) {
            return None;
        }
        let receiver = receiver_node?;
        if receiver.kind() == "member_expression"
            && receiver
                .child_by_field_name("object")
                .is_some_and(|n| n.kind() == "this")
        {
            return Some(self.js_ts_constructor_field_type(receiver).map_or(
                JsTsReceiverBindingEvidence::Materialized,
                |static_type| JsTsReceiverBindingEvidence::Recovered {
                    static_type,
                    recovery: crate::resolution::ReceiverRecovery::FieldTyped,
                    declaration_end_byte: None,
                },
            ));
        }
        if receiver.kind() != "identifier" {
            return None;
        }
        let receiver_name = self.node_text(&receiver);
        if !is_plain_ident(receiver_name) {
            return None;
        }

        let mut seen_functions = BTreeSet::new();
        let mut current = Some(receiver);
        let mut function_rank = 0usize;
        while let Some(node) = current {
            if is_js_ts_function_like(node.kind()) && seen_functions.insert(node.id()) {
                if let Some(evidence) = self.js_ts_scope_receiver_binding_evidence(
                    node,
                    &receiver,
                    receiver_name,
                    function_rank == 0,
                    true,
                ) {
                    return Some(evidence);
                }
                function_rank += 1;
            }
            current = node.parent();
        }

        if seen_functions.insert(func_node.id()) {
            if let Some(evidence) = self.js_ts_scope_receiver_binding_evidence(
                *func_node,
                &receiver,
                receiver_name,
                false,
                true,
            ) {
                return Some(evidence);
            }
        }

        let root = self.tree.root_node();
        self.js_ts_scope_receiver_binding_evidence(root, &receiver, receiver_name, false, false)
            .or_else(|| {
                self.js_ts_receiver_lexically_bound_at_call(func_node, Some(receiver))
                    .then_some(JsTsReceiverBindingEvidence::Materialized)
            })
    }

    pub(super) fn js_ts_scope_receiver_binding_evidence(
        &self,
        root_scope: Node<'_>,
        receiver: &Node<'_>,
        receiver_name: &str,
        allow_constructor: bool,
        include_parameters: bool,
    ) -> Option<JsTsReceiverBindingEvidence> {
        let mut candidates = Vec::new();
        let mut parse_recovery = false;
        self.collect_js_ts_receiver_binding_candidates(
            root_scope,
            true,
            root_scope,
            receiver,
            receiver_name,
            allow_constructor,
            &mut candidates,
            &mut parse_recovery,
        );

        if include_parameters {
            if matches!(
                root_scope.kind(),
                "function_declaration"
                    | "function_expression"
                    | "generator_function_declaration"
                    | "generator_function"
            ) && root_scope
                .child_by_field_name("name")
                .is_some_and(|name| self.node_text(&name) == receiver_name)
            {
                if let Some(distance) = self.js_ts_scope_distance(receiver, root_scope.id()) {
                    candidates.push((
                        distance,
                        root_scope.id(),
                        JsTsReceiverBindingEvidence::Materialized,
                    ));
                }
            }
            if let Some(evidence) = self.js_ts_parameter_receiver_binding(root_scope, receiver_name)
            {
                if let Some(distance) = self.js_ts_scope_distance(receiver, root_scope.id()) {
                    candidates.push((distance, root_scope.id(), evidence));
                }
            }
        }

        let nearest = candidates.iter().map(|(distance, _, _)| *distance).min()?;
        let nearest_candidates: Vec<_> = candidates
            .into_iter()
            .filter(|(distance, _, _)| *distance == nearest)
            .map(|(_, scope_id, evidence)| (scope_id, evidence))
            .collect();
        if parse_recovery {
            return Some(JsTsReceiverBindingEvidence::Materialized);
        }
        if nearest_candidates.len() != 1 {
            return Some(
                if nearest_candidates.iter().all(|(_, evidence)| {
                    matches!(evidence, JsTsReceiverBindingEvidence::ClassOwner)
                }) {
                    JsTsReceiverBindingEvidence::ClassOwner
                } else {
                    JsTsReceiverBindingEvidence::Materialized
                },
            );
        }
        let (binding_scope_id, evidence) = nearest_candidates.into_iter().next()?;
        if let JsTsReceiverBindingEvidence::Recovered {
            declaration_end_byte: Some(declaration_end_byte),
            ..
        }
        | JsTsReceiverBindingEvidence::ImportedProps {
            declaration_end_byte,
            ..
        } = &evidence
        {
            if self.js_ts_receiver_written_between(
                root_scope,
                binding_scope_id,
                receiver,
                receiver_name,
                *declaration_end_byte,
            ) {
                return Some(JsTsReceiverBindingEvidence::Materialized);
            }
        }
        Some(evidence)
    }

    #[allow(clippy::too_many_arguments)]
    fn collect_js_ts_receiver_binding_candidates(
        &self,
        node: Node<'_>,
        is_root: bool,
        root_scope: Node<'_>,
        receiver: &Node<'_>,
        receiver_name: &str,
        allow_constructor: bool,
        out: &mut Vec<(usize, usize, JsTsReceiverBindingEvidence)>,
        parse_recovery: &mut bool,
    ) {
        if node.is_error() || node.is_missing() {
            *parse_recovery = true;
            return;
        }

        let mut push = |scope_id: usize, evidence: JsTsReceiverBindingEvidence| {
            if let Some(distance) = self.js_ts_scope_distance(receiver, scope_id) {
                out.push((distance, scope_id, evidence));
            }
        };

        if !is_root && is_js_ts_function_like(node.kind()) {
            if matches!(
                node.kind(),
                "function_declaration" | "generator_function_declaration"
            ) && self
                .language
                .function_name(&node)
                .is_some_and(|name| self.node_text(&name) == receiver_name)
            {
                if let Some(scope_id) = self.js_ts_nearest_lexical_scope_id(&node, root_scope.id())
                {
                    push(scope_id, JsTsReceiverBindingEvidence::Materialized);
                }
            }
            return;
        }

        if !is_root
            && matches!(
                node.kind(),
                "interface_declaration" | "type_alias_declaration"
            )
        {
            return;
        }

        if !is_root
            && matches!(
                node.kind(),
                "class_declaration" | "abstract_class_declaration"
            )
        {
            if node
                .child_by_field_name("name")
                .is_some_and(|name| self.node_text(&name) == receiver_name)
            {
                if let Some(scope_id) = self.js_ts_nearest_lexical_scope_id(&node, root_scope.id())
                {
                    push(scope_id, JsTsReceiverBindingEvidence::ClassOwner);
                }
            }
            return;
        }

        if !is_root
            && matches!(
                node.kind(),
                "enum_declaration" | "function_signature" | "import_alias"
            )
        {
            let name = node.child_by_field_name("name").or_else(|| {
                (node.kind() == "import_alias")
                    .then(|| node.named_child(0))
                    .flatten()
            });
            if name.is_some_and(|name| self.node_text(&name) == receiver_name) {
                if let Some(scope_id) = self.js_ts_nearest_lexical_scope_id(&node, root_scope.id())
                {
                    push(scope_id, JsTsReceiverBindingEvidence::Materialized);
                }
            }
            return;
        }

        if !is_root && matches!(node.kind(), "internal_module" | "module") {
            if node.child_by_field_name("name").is_some_and(|name| {
                name.kind() == "identifier" && self.node_text(&name) == receiver_name
            }) {
                if let Some(scope_id) = self.js_ts_nearest_lexical_scope_id(&node, root_scope.id())
                {
                    push(scope_id, JsTsReceiverBindingEvidence::Materialized);
                }
            }
            return;
        }

        match node.kind() {
            "variable_declarator" => {
                let mut names = BTreeSet::new();
                if let Some(name) = node.child_by_field_name("name") {
                    self.collect_js_ts_binding_pattern_names(name, &mut names);
                }
                if names.contains(receiver_name) {
                    let parent_is_var = node
                        .parent()
                        .is_some_and(|parent| parent.kind() == "variable_declaration");
                    let scope_id = if parent_is_var {
                        Some(root_scope.id())
                    } else {
                        self.js_ts_nearest_lexical_scope_id(&node, root_scope.id())
                    };
                    if let Some(scope_id) = scope_id {
                        let simple_name = node.child_by_field_name("name").is_some_and(|name| {
                            name.kind() == "identifier" && self.node_text(&name) == receiver_name
                        });
                        let evidence = if allow_constructor
                            && simple_name
                            && self.receiver_declaration_reaches_call(node, receiver.start_byte())
                        {
                            node.child_by_field_name("value")
                                .and_then(|value| {
                                    self.js_ts_direct_new_constructor(value, root_scope)
                                })
                                .map_or(JsTsReceiverBindingEvidence::Materialized, |static_type| {
                                    JsTsReceiverBindingEvidence::Recovered {
                                        static_type,
                                        recovery:
                                            crate::resolution::ReceiverRecovery::ConstructorLocal,
                                        declaration_end_byte: Some(node.end_byte()),
                                    }
                                })
                        } else {
                            JsTsReceiverBindingEvidence::Materialized
                        };
                        push(scope_id, evidence);
                    }
                }
            }
            "for_in_statement" | "for_of_statement" | "for_await_statement" => {
                let mut declared = false;
                let mut is_var = false;
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    match child.kind() {
                        "var" => {
                            declared = true;
                            is_var = true;
                        }
                        "let" | "const" => declared = true,
                        _ => {}
                    }
                }
                if declared {
                    let mut names = BTreeSet::new();
                    if let Some(left) = node.child_by_field_name("left") {
                        self.collect_js_ts_binding_pattern_names(left, &mut names);
                    }
                    if names.contains(receiver_name) {
                        let scope_id = if is_var {
                            Some(root_scope.id())
                        } else {
                            self.js_ts_nearest_lexical_scope_id(&node, root_scope.id())
                        };
                        if let Some(scope_id) = scope_id {
                            push(scope_id, JsTsReceiverBindingEvidence::Materialized);
                        }
                    }
                }
            }
            "catch_clause" => {
                let mut names = BTreeSet::new();
                self.collect_js_ts_catch_binding_names(node, &mut names);
                if names.contains(receiver_name) {
                    if let Some(scope_id) =
                        self.js_ts_nearest_lexical_scope_id(&node, root_scope.id())
                    {
                        push(scope_id, JsTsReceiverBindingEvidence::Materialized);
                    }
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.collect_js_ts_receiver_binding_candidates(
                child,
                false,
                root_scope,
                receiver,
                receiver_name,
                allow_constructor,
                out,
                parse_recovery,
            );
        }
    }

    pub(super) fn js_ts_scope_distance(&self, receiver: &Node<'_>, scope_id: usize) -> Option<usize> {
        let mut current = Some(*receiver);
        let mut distance = 0usize;
        while let Some(node) = current {
            if node.id() == scope_id {
                return Some(distance);
            }
            current = node.parent();
            distance += 1;
        }
        None
    }

    /// Normalize a field callable to the member carrying its name and modifiers.
    fn js_ts_callable_member<'a>(&self, method: &Node<'a>) -> Node<'a> {
        method
            .parent()
            .filter(|n| matches!(n.kind(), "field_definition" | "public_field_definition"))
            .unwrap_or(*method)
    }

    /// Direct methods and bounded arrow fields share instance-slot authority.
    pub(crate) fn js_ts_method_slot_unproven(&self, method: &Node<'_>) -> bool {
        if !matches!(
            self.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) {
            return false;
        }
        let slot = self.js_ts_callable_member(method);
        let Some(body) = slot.parent().filter(|n| n.kind() == "class_body") else {
            return false;
        };
        let Some(name) = slot
            .child_by_field_name("name")
            .or_else(|| slot.child_by_field_name("property"))
        else {
            return true;
        };
        let field = slot.id() != method.id();
        if field {
            let mut cursor = slot.walk();
            let modifiers = slot.children(&mut cursor).any(|n| {
                n.kind() == "decorator"
                    || matches!(
                        self.node_text(&n),
                        "private" | "protected" | "declare" | "abstract"
                    )
            });
            let class = body.parent().unwrap();
            let mut cursor = class.walk();
            let decorated_class = class.children(&mut cursor).any(|n| n.kind() == "decorator");
            if method.kind() != "arrow_function"
                || name.kind() != "property_identifier"
                || slot.child_by_field_name("value") != Some(*method)
                || body.has_error()
                || modifiers
                || decorated_class
            {
                return true;
            }
        }
        let name = self.node_text(&name);
        let mut matches = 0;
        let mut cursor = body.walk();
        for member in body.named_children(&mut cursor) {
            if self.js_ts_method_is_static(&member) {
                continue;
            }
            let key = member
                .child_by_field_name("name")
                .or_else(|| member.child_by_field_name("property"));
            if let Some(key) = key {
                if key.kind() == "computed_property_name" {
                    return true;
                }
                if self.node_text(&key).trim_matches(['\'', '"']) == name {
                    matches += 1;
                    let mut cursor = member.walk();
                    let accessor = member
                        .children(&mut cursor)
                        .any(|n| matches!(n.kind(), "get" | "set"));
                    if accessor
                        || (member.kind() != "method_definition"
                            && !(field && member.id() == slot.id()))
                    {
                        return true;
                    }
                }
            }
        }
        if matches != 1 {
            return true;
        }
        fn writes_this(parsed: &ParsedFile, node: Node<'_>, name: &str) -> bool {
            if parsed
                .js_ts_write_target(node)
                .is_some_and(|target| parsed.js_ts_target_writes_member(target, "this", Some(name)))
            {
                return true;
            }
            let mut cursor = node.walk();
            let written = node
                .named_children(&mut cursor)
                .any(|n| writes_this(parsed, n, name));
            written
        }
        writes_this(self, body, name)
    }

    pub(crate) fn js_ts_method_is_static(&self, method: &Node<'_>) -> bool {
        if !matches!(
            self.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) {
            return false;
        }
        let slot = self.js_ts_callable_member(method);
        let mut cursor = slot.walk();
        let found = slot
            .children(&mut cursor)
            .any(|child| child.kind() == "static");
        found
    }
}
