//! Go receiver scope rules: short variable declarations, type parameters, local type shadowing.

use crate::ast::{go_calls, go_receiver_walk, ParsedFile};
use crate::languages::Language;
use anyhow::Result;
use tree_sitter::Node;

impl ParsedFile {
    /// Same-scope reuse exception for the post-merge direct-route proof.
    pub(crate) fn go_same_scope_reuse_receiver_type_evidence_in_fn(
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
            true,
        )
    }

    // Mirrored in go_receiver_walk; change both and extend its tests.
    pub(crate) fn go_func_literal_parameter_binds_receiver(
        &self,
        func_node: &Node<'_>,
        receiver: &str,
        call_start_byte: usize,
    ) -> bool {
        if self.language != Language::Go {
            return false;
        }
        if let Some(answer) =
            go_receiver_walk::auxiliary(self, *func_node, receiver, call_start_byte, true)
        {
            return answer;
        }

        fn walk(
            parsed: &ParsedFile,
            node: Node<'_>,
            receiver: &str,
            call_start_byte: usize,
        ) -> bool {
            if call_start_byte < node.start_byte() || call_start_byte >= node.end_byte() {
                return false;
            }
            if node.kind() == "func_literal"
                && parsed.find_parameters_node(&node).is_some_and(|params| {
                    let mut cursor = params.walk();
                    let found = params.children(&mut cursor).any(|param| {
                        param
                            .child_by_field_name("type")
                            .is_some_and(|ty| parsed.go_parameter_binds_name(param, ty, receiver))
                    });
                    found
                })
            {
                return true;
            }
            let mut cursor = node.walk();
            let found = node
                .children(&mut cursor)
                .any(|child| walk(parsed, child, receiver, call_start_byte));
            found
        }

        walk(self, *func_node, receiver, call_start_byte)
    }

    /// Whether this Go file contains a dot import. Dot imports stay separate
    /// from the named-import map because `.` is not a resolvable qualifier.
    pub(crate) fn go_has_dot_import(&self) -> bool {
        if self.language != Language::Go {
            return false;
        }

        fn walk(node: Node<'_>) -> bool {
            if node.kind() == "import_spec" {
                let mut cursor = node.walk();
                return node
                    .children(&mut cursor)
                    .any(|child| child.kind() == "dot");
            }
            let mut cursor = node.walk();
            let found = node.children(&mut cursor).any(walk);
            found
        }

        walk(self.tree.root_node())
    }

    /// True when the recovered bare receiver type is bound by the enclosing
    /// Go declaration's own generic parameters. Free functions declare them in
    /// `type_parameters`; methods declare receiver-specific names in the
    /// receiver type's `type_arguments` (`Store[T]`).
    pub(crate) fn go_type_parameter_binds_receiver(
        &self,
        func_node: &Node<'_>,
        receiver_type: &str,
    ) -> bool {
        if self.language != Language::Go {
            return false;
        }
        let receiver_type = receiver_type
            .trim()
            .trim_start_matches('&')
            .trim_start_matches('*')
            .trim();
        if receiver_type.is_empty()
            || receiver_type.contains('.')
            || receiver_type.contains('[')
            || !receiver_type
                .chars()
                .all(|ch| ch.is_alphanumeric() || ch == '_')
        {
            return false;
        }

        if let Some(params) = func_node.child_by_field_name("type_parameters") {
            let mut cursor = params.walk();
            if params.children(&mut cursor).any(|declaration| {
                if declaration.kind() != "type_parameter_declaration" {
                    return false;
                }
                let mut names = declaration.walk();
                let found = declaration.children(&mut names).any(|name| {
                    name.kind() == "identifier" && self.node_text(&name) == receiver_type
                });
                found
            }) {
                return true;
            }
        }

        go_calls::receiver_type_parameters(*func_node)
            .into_iter()
            .any(|name| self.node_text(&name).trim() == receiver_type)
    }

    // Mirrored in go_receiver_walk; change both and extend its tests.
    pub(crate) fn go_local_type_shadows(
        &self,
        func_node: &Node<'_>,
        receiver_type: &str,
        call_start_byte: usize,
    ) -> bool {
        if self.language != Language::Go {
            return false;
        }
        let type_name = receiver_type
            .trim()
            .trim_start_matches('&')
            .trim_start_matches('*')
            .trim();
        if type_name.is_empty() || type_name.contains('.') || type_name.contains('[') {
            return false;
        }
        if let Some(answer) =
            go_receiver_walk::auxiliary(self, *func_node, type_name, call_start_byte, false)
        {
            return answer;
        }

        fn scope_contains_call(node: Node<'_>, call_start_byte: usize) -> bool {
            let mut parent = node.parent();
            while let Some(scope) = parent {
                if matches!(
                    scope.kind(),
                    "block" | "expression_case" | "type_case" | "communication_case"
                ) {
                    return scope.start_byte() <= call_start_byte
                        && call_start_byte < scope.end_byte();
                }
                parent = scope.parent();
            }
            false
        }

        fn walk(
            parsed: &ParsedFile,
            node: Node<'_>,
            type_name: &str,
            call_start_byte: usize,
        ) -> bool {
            if node.start_byte() >= call_start_byte {
                return false;
            }
            if matches!(node.kind(), "type_spec" | "type_alias")
                && node
                    .child_by_field_name("name")
                    .is_some_and(|name| parsed.node_text(&name).trim() == type_name)
                && scope_contains_call(node, call_start_byte)
            {
                return true;
            }
            let mut cursor = node.walk();
            let found = node
                .children(&mut cursor)
                .any(|child| walk(parsed, child, type_name, call_start_byte));
            found
        }

        walk(self, *func_node, type_name, call_start_byte)
    }

    // Mirrored in go_receiver_walk; change both and extend its tests.
    pub(crate) fn go_short_var_call_rhs(
        &self,
        func_node: &Node<'_>,
        receiver: &str,
        call_line: usize,
        call_start_byte: usize,
    ) -> Option<String> {
        if let Some(answer) =
            go_receiver_walk::rhs(self, *func_node, receiver, call_line, call_start_byte)
        {
            return answer;
        }
        fn walk(
            this: &ParsedFile,
            node: Node<'_>,
            is_root: bool,
            receiver: &str,
            call_line: usize,
            call_start_byte: usize,
        ) -> Option<String> {
            if node.start_position().row + 1 > call_line {
                return None;
            }
            if node.start_byte() >= call_start_byte {
                return None;
            }
            if !is_root && this.language.function_node_types().contains(&node.kind()) {
                return None; // same convention as walk_receiver_bindings.
            }
            // Skip a literal's entire subtree unless it contains the call.
            if !is_root && node.kind() == "func_literal" {
                let call_inside =
                    call_start_byte >= node.start_byte() && call_start_byte < node.end_byte();
                if !call_inside {
                    return None;
                }
            }
            if node.kind() == "short_var_declaration" {
                if let (Some(left), Some(right)) = (
                    node.child_by_field_name("left"),
                    node.child_by_field_name("right"),
                ) {
                    let mut lcur = left.walk();
                    let names: Vec<Node> = left.named_children(&mut lcur).collect();
                    if names
                        .first()
                        .map(|n| this.node_text(n).trim() == receiver)
                        .unwrap_or(false)
                    {
                        let mut rcur = right.walk();
                        let rhs: Vec<Node> = right.named_children(&mut rcur).collect();
                        if rhs.len() == 1 && rhs[0].kind() == "call_expression" {
                            if let Some(func) = this.call_function_node(&rhs[0]) {
                                if matches!(func.kind(), "identifier" | "selector_expression") {
                                    return Some(this.node_text(&func).trim().to_string());
                                }
                            }
                        }
                        return None; // receiver IS first, but RHS doesn't qualify.
                    }
                }
            }
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if let Some(r) = walk(this, child, false, receiver, call_line, call_start_byte) {
                    return Some(r);
                }
            }
            None
        }
        walk(self, *func_node, true, receiver, call_line, call_start_byte)
    }

    // Mirrored in go_receiver_walk; change both and extend its tests.
    pub(crate) fn go_same_scope_short_var_reuse_calls(
        &self,
        func_node: &Node<'_>,
        receiver: &str,
        call_start_byte: usize,
    ) -> Result<Vec<String>, ()> {
        if let Some(answer) = go_receiver_walk::reuses(self, *func_node, receiver, call_start_byte)
        {
            return answer;
        }
        fn walk(
            this: &ParsedFile,
            node: Node<'_>,
            is_root: bool,
            func_node: &Node<'_>,
            receiver: &str,
            call_start_byte: usize,
            out: &mut Vec<String>,
        ) -> Result<(), ()> {
            if node.start_byte() >= call_start_byte {
                return Ok(());
            }
            if !is_root && this.language.function_node_types().contains(&node.kind()) {
                return Ok(());
            }
            if !is_root && node.kind() == "func_literal" {
                let call_inside =
                    node.start_byte() <= call_start_byte && call_start_byte < node.end_byte();
                if !call_inside {
                    return Ok(());
                }
            }
            if node.kind() == "short_var_declaration"
                && this.go_short_decl_reuses_in_scope(node, func_node, receiver, call_start_byte)
            {
                let left = node.child_by_field_name("left").ok_or(())?;
                let names = this.go_short_decl_names(left);
                if names.first().map(String::as_str) != Some(receiver) {
                    return Err(());
                }
                let right = node.child_by_field_name("right").ok_or(())?;
                let mut rcur = right.walk();
                let rhs: Vec<Node> = right.named_children(&mut rcur).collect();
                if rhs.len() != 1 || rhs[0].kind() != "call_expression" {
                    return Err(());
                }
                let func = this.call_function_node(&rhs[0]).ok_or(())?;
                if !matches!(func.kind(), "identifier" | "selector_expression") {
                    return Err(());
                }
                out.push(this.node_text(&func).trim().to_string());
            }
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                walk(
                    this,
                    child,
                    false,
                    func_node,
                    receiver,
                    call_start_byte,
                    out,
                )?;
            }
            Ok(())
        }

        let mut calls = Vec::new();
        walk(
            self,
            *func_node,
            true,
            func_node,
            receiver,
            call_start_byte,
            &mut calls,
        )?;
        Ok(calls)
    }

    pub(in crate::ast) fn go_short_decl_names(&self, left: Node<'_>) -> Vec<String> {
        if let Some(name) = self.simple_binding_text(&left) {
            return vec![name];
        }
        let mut cursor = left.walk();
        left.named_children(&mut cursor)
            .filter_map(|child| self.simple_binding_text(&child))
            .collect()
    }

    pub(in crate::ast) fn go_enclosing_binding_scope<'a>(&self, node: Node<'a>) -> Option<Node<'a>> {
        let mut parent = node.parent();
        while let Some(scope) = parent {
            if matches!(
                scope.kind(),
                "block"
                    | "expression_case"
                    | "type_case"
                    | "communication_case"
                    | "if_statement"
                    | "for_statement"
                    | "expression_switch_statement"
                    | "type_switch_statement"
            ) {
                return Some(scope);
            }
            parent = scope.parent();
        }
        None
    }

    // Mirrored in go_receiver_walk; change both and extend its tests.
    fn go_scope_declares_name_before(
        &self,
        func_node: &Node<'_>,
        scope: Node<'_>,
        name: &str,
        before_byte: usize,
    ) -> bool {
        fn walk(
            this: &ParsedFile,
            node: Node<'_>,
            is_root: bool,
            name: &str,
            before_byte: usize,
        ) -> bool {
            if node.start_byte() >= before_byte {
                return false;
            }
            if !is_root
                && (this.language.function_node_types().contains(&node.kind())
                    || matches!(
                        node.kind(),
                        "block"
                            | "expression_case"
                            | "type_case"
                            | "communication_case"
                            | "func_literal"
                            | "if_statement"
                            | "for_statement"
                            | "expression_switch_statement"
                            | "type_switch_statement"
                            | "select_statement"
                    ))
            {
                return false;
            }
            if node.kind() == "short_var_declaration" {
                if let Some(left) = node.child_by_field_name("left") {
                    return this.go_short_decl_names(left).iter().any(|n| n == name);
                }
            }
            if node.kind() == "var_spec" {
                let mut cursor = node.walk();
                if node
                    .children_by_field_name("name", &mut cursor)
                    .any(|child| this.node_text(&child).trim() == name)
                {
                    return true;
                }
            }
            let mut cursor = node.walk();
            let found = node
                .children(&mut cursor)
                .any(|child| walk(this, child, false, name, before_byte));
            found
        }

        let parameter_owner = scope.parent().filter(|parent| {
            matches!(
                parent.kind(),
                "function_declaration" | "method_declaration" | "func_literal"
            )
        });
        let parameter_owner = parameter_owner.or_else(|| {
            func_node
                .child_by_field_name("body")
                .filter(|body| body.start_byte() == scope.start_byte())
                .map(|_| *func_node)
        });
        if parameter_owner.is_some_and(|owner| {
            self.find_parameters_node(&owner).is_some_and(|params| {
                let mut cursor = params.walk();
                let found = params.children(&mut cursor).any(|param| {
                    param
                        .child_by_field_name("type")
                        .is_some_and(|ty| self.go_parameter_binds_name(param, ty, name))
                });
                found
            })
        }) {
            return true;
        }

        walk(self, scope, true, name, before_byte)
    }

    // Mirrored in go_receiver_walk; change both and extend its tests.
    pub(in crate::ast) fn go_short_decl_reuses_in_scope(
        &self,
        node: Node<'_>,
        func_node: &Node<'_>,
        receiver: &str,
        call_start_byte: usize,
    ) -> bool {
        let Some(scope) = self.go_enclosing_binding_scope(node) else {
            return false;
        };
        if !(scope.start_byte() <= call_start_byte && call_start_byte < scope.end_byte()) {
            return false;
        }
        let Some(left) = node.child_by_field_name("left") else {
            return false;
        };
        let names = self.go_short_decl_names(left);
        if names.len() < 2
            || !names.iter().any(|name| name == receiver)
            || !self.go_scope_declares_name_before(func_node, scope, receiver, node.start_byte())
        {
            return false;
        }
        names.iter().any(|name| {
            name != receiver
                && name != "_"
                && !self.go_scope_declares_name_before(func_node, scope, name, node.start_byte())
        })
    }

    pub(in crate::ast) fn go_parameter_binds_name(&self, param: Node<'_>, ty: Node<'_>, receiver: &str) -> bool {
        let mut cursor = param.walk();
        for child in param.children(&mut cursor) {
            if child.start_byte() >= ty.start_byte() {
                continue;
            }
            if child.kind() == "identifier" && self.node_text(&child) == receiver {
                return true;
            }
        }
        false
    }
}
