//! Receiver type evidence inside a function (the shared entry points).

use crate::ast::{go_receiver_walk, ParsedFile};
use crate::languages::Language;
use tree_sitter::Node;

impl ParsedFile {
    /// S3 P6-lite: syntactically-provable receiver type for `receiver` at a call
    /// on `call_line`. Typed params + constructor locals; when `recover_var` is true
    /// also recovers `var r T` declarations. Only bindings at or before `call_line`
    /// count; >1 binding before the call means shadow bail. Rust + Go +
    /// guarded Python.
    /// Returns `(type_found, binding_count)`: the raw, unpeeled type text +
    /// which fact recovered it, plus how many local bindings of `receiver`
    /// were seen (so the caller can distinguish "no bindings" from "bindings
    /// present but type unrecoverable").
    pub fn receiver_type_in_fn(
        &self,
        func_node: &Node<'_>,
        receiver: &str,
        call_line: usize,
        call_start_byte: usize,
        recover_var: bool,
    ) -> (Option<(String, crate::resolution::ReceiverRecovery)>, usize) {
        let (found, bindings, _) = self.receiver_type_evidence_in_fn(
            func_node,
            receiver,
            call_line,
            call_start_byte,
            recover_var,
        );
        (found, bindings)
    }

    /// Ordinary evidence plus the first recoverable binding for Go's R3 ladder.
    pub(crate) fn receiver_type_evidence_in_fn(
        &self,
        func_node: &Node<'_>,
        receiver: &str,
        call_line: usize,
        call_start_byte: usize,
        recover_var: bool,
    ) -> (
        Option<(String, crate::resolution::ReceiverRecovery)>,
        usize,
        Option<(String, crate::resolution::ReceiverRecovery)>,
    ) {
        self.receiver_type_evidence_in_fn_mode(
            func_node,
            receiver,
            call_line,
            call_start_byte,
            recover_var,
            false,
        )
    }

    // Mirrored in go_receiver_walk; change both and extend its tests.
    pub(super) fn receiver_type_evidence_in_fn_mode(
        &self,
        func_node: &Node<'_>,
        receiver: &str,
        call_line: usize,
        call_start_byte: usize,
        recover_var: bool,
        enable_go_same_scope_reuse: bool,
    ) -> (
        Option<(String, crate::resolution::ReceiverRecovery)>,
        usize,
        Option<(String, crate::resolution::ReceiverRecovery)>,
    ) {
        use crate::languages::Language;
        use crate::resolution::ReceiverRecovery;

        if let Some(answer) = go_receiver_walk::evidence(
            self,
            *func_node,
            receiver,
            call_line,
            call_start_byte,
            recover_var,
            enable_go_same_scope_reuse,
        ) {
            return answer;
        }

        if !matches!(
            self.language,
            Language::Rust | Language::Go | Language::Python
        ) {
            return (None, 0, None);
        }

        let mut found: Option<(String, ReceiverRecovery)> = None;
        let mut first_found: Option<(String, ReceiverRecovery)> = None;
        let mut bindings = 0usize;
        let mut go_lexical_rebinding = false;

        if let Some(params) = self.find_parameters_node(func_node) {
            let mut cursor = params.walk();
            for param in params.children(&mut cursor) {
                match self.language {
                    Language::Rust if param.kind() == "parameter" => {
                        let (Some(pattern), Some(ty)) = (
                            param.child_by_field_name("pattern"),
                            param.child_by_field_name("type"),
                        ) else {
                            continue;
                        };
                        if self.simple_binding_text(&pattern).as_deref() == Some(receiver) {
                            found = Some((
                                self.node_text(&ty).to_string(),
                                ReceiverRecovery::TypedParam,
                            ));
                            bindings += 1;
                        } else if self.node_binds_name(pattern, receiver) {
                            bindings += 1;
                            found = None;
                        }
                    }
                    Language::Go if param.kind() == "parameter_declaration" => {
                        let Some(ty) = param.child_by_field_name("type") else {
                            continue;
                        };
                        if self.go_parameter_binds_name(param, ty, receiver) {
                            found = Some((
                                self.node_text(&ty).to_string(),
                                ReceiverRecovery::TypedParam,
                            ));
                            bindings += 1;
                        }
                    }
                    Language::Python
                        if matches!(
                            param.kind(),
                            "typed_parameter" | "typed_default_parameter"
                        ) =>
                    {
                        let Some(ty) = param.child_by_field_name("type") else {
                            continue;
                        };
                        if self.parameter_binds_name_before_type(param, ty, receiver) {
                            found = Some((
                                self.node_text(&ty).to_string(),
                                ReceiverRecovery::TypedParam,
                            ));
                            bindings += 1;
                        }
                    }
                    _ => {}
                }
            }
        }
        if bindings == 1 {
            first_found = found.clone();
        }

        // Python: count bare (untyped) parameters as bindings — `def run(Foo):`
        // shadows an import/class of the same name. Typed params were already
        // counted above, so only increment when `bindings` is still 0 for this
        // receiver to avoid double-counting.
        if matches!(self.language, Language::Python) && bindings == 0 {
            if let Some(params) = self.find_parameters_node(func_node) {
                let mut pcursor = params.walk();
                for param in params.children(&mut pcursor) {
                    match param.kind() {
                        "identifier"
                        | "default_parameter"
                        | "list_splat_pattern"
                        | "dictionary_splat_pattern" => {
                            let name_node = if param.kind() == "identifier" {
                                Some(param)
                            } else if matches!(
                                param.kind(),
                                "list_splat_pattern" | "dictionary_splat_pattern"
                            ) {
                                // Splat patterns have no `name` field; find the identifier child.
                                let mut sc = param.walk();
                                let found =
                                    param.children(&mut sc).find(|c| c.kind() == "identifier");
                                found
                            } else {
                                param.child_by_field_name("name")
                            };
                            if let Some(n) = name_node {
                                if self.simple_binding_text(&n).as_deref() == Some(receiver) {
                                    bindings += 1;
                                    // No type recovery possible from a bare param.
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Go declaration visibility is independent of same-scope reuse recovery.
        let go_filter_declarations_to_call_scope = matches!(self.language, Language::Go);
        self.walk_receiver_bindings(
            *func_node,
            true,
            receiver,
            call_line,
            call_start_byte,
            go_filter_declarations_to_call_scope,
            enable_go_same_scope_reuse,
            &mut found,
            &mut first_found,
            &mut bindings,
            &mut go_lexical_rebinding,
            recover_var,
        );
        if self.language == Language::Python && found.is_some() && bindings == 1 {
            let mut ancestor =
                func_node.descendant_for_byte_range(call_start_byte, call_start_byte);
            while let Some(scope) = ancestor {
                if scope.id() == func_node.id() {
                    break;
                }
                if matches!(scope.kind(), "for_statement" | "while_statement") {
                    let mut prefix_bindings = 0;
                    self.walk_receiver_bindings(
                        scope,
                        true,
                        receiver,
                        call_line,
                        call_start_byte,
                        false,
                        false,
                        &mut None,
                        &mut None,
                        &mut prefix_bindings,
                        &mut false,
                        recover_var,
                    );
                    // With one proven binding overall, a binding in this loop's
                    // prefix is the origin and resets it before each call. An
                    // origin outside the loop must also survive its back edge.
                    if prefix_bindings == 0
                        && self
                            .function_local_value_bindings(&scope)
                            .is_none_or(|names| names.contains(receiver))
                    {
                        found = None;
                        bindings += 1;
                        break;
                    }
                }
                ancestor = scope.parent();
            }
        }
        if bindings > 1 {
            return (
                None,
                bindings,
                go_lexical_rebinding.then_some(first_found).flatten(),
            );
        }
        (found, bindings, first_found)
    }

    /// A declaration must finish in a statement list that reaches the call.
    /// Crossing a conditional, loop, exception handler, or callable is not proof
    /// of execution. Ordinary nested blocks are transparent.
    pub(super) fn receiver_declaration_reaches_call(&self, declaration: Node<'_>, call_byte: usize) -> bool {
        if declaration.end_byte() > call_byte {
            return false;
        }
        let mut current = declaration.parent();
        while let Some(node) = current {
            let statement_list = matches!(
                node.kind(),
                "block" | "statement_block" | "module" | "program"
            );
            if node.start_byte() <= call_byte && call_byte < node.end_byte() {
                return statement_list;
            }
            if !statement_list
                && !matches!(
                    node.kind(),
                    "expression_statement" | "lexical_declaration" | "variable_declaration"
                )
            {
                return false;
            }
            current = node.parent();
        }
        false
    }
}
