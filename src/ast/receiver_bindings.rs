//! The receiver binding walk and constructor-type recovery.

use crate::ast::ParsedFile;
use tree_sitter::Node;

impl ParsedFile {
    // Mirrored in go_receiver_walk; change both and extend its tests.
    pub(super) fn walk_receiver_bindings(
        &self,
        node: Node<'_>,
        is_root: bool,
        receiver: &str,
        call_line: usize,
        call_start_byte: usize,
        go_filter_declarations_to_call_scope: bool,
        enable_go_same_scope_reuse: bool,
        found: &mut Option<(String, crate::resolution::ReceiverRecovery)>,
        first_found: &mut Option<(String, crate::resolution::ReceiverRecovery)>,
        bindings: &mut usize,
        go_lexical_rebinding: &mut bool,
        recover_var: bool,
    ) {
        use crate::languages::Language;
        use crate::resolution::ReceiverRecovery;

        if node.start_position().row + 1 > call_line {
            return;
        }
        if node.start_byte() >= call_start_byte {
            return;
        }
        if !is_root && self.language.function_node_types().contains(&node.kind()) {
            // Python: nested function name IS a binding in the enclosing scope.
            if matches!(self.language, Language::Python) {
                if let Some(name_node) = self.language.function_name(&node) {
                    if self.simple_binding_text(&name_node).as_deref() == Some(receiver) {
                        *bindings += 1;
                        *found = None;
                    }
                }
            }
            return;
        }
        if !is_root
            && matches!(self.language, Language::Python)
            && matches!(
                node.kind(),
                "class_definition" | "class_declaration" | "class"
            )
        {
            // The class name IS a binding in the enclosing scope.
            if let Some(name_node) = node.child_by_field_name("name") {
                if self.simple_binding_text(&name_node).as_deref() == Some(receiver) {
                    *bindings += 1;
                    *found = None;
                }
            }
            return;
        }
        // Python 3 comprehensions have their own scope — bindings inside them
        // do NOT shadow names in the enclosing function scope.
        if !is_root
            && matches!(self.language, Language::Python)
            && matches!(
                node.kind(),
                "list_comprehension"
                    | "set_comprehension"
                    | "dictionary_comprehension"
                    | "generator_expression"
            )
        {
            return;
        }

        match (self.language, node.kind()) {
            (Language::Rust, "let_declaration") => {
                if let Some(pattern) = node.child_by_field_name("pattern") {
                    if self.simple_binding_text(&pattern).as_deref() == Some(receiver) {
                        *bindings += 1;
                        if let Some(ty) = node.child_by_field_name("type") {
                            *found = Some((
                                self.node_text(&ty).to_string(),
                                ReceiverRecovery::ConstructorLocal,
                            ));
                        } else if let Some(value) = node.child_by_field_name("value") {
                            *found = self
                                .constructor_type(&value)
                                .map(|ty| (ty, ReceiverRecovery::ConstructorLocal));
                        } else {
                            *found = None;
                        }
                    } else if self.node_binds_name(pattern, receiver) {
                        *bindings += 1;
                        *found = None;
                    }
                }
            }
            (Language::Go, "short_var_declaration") => {
                let left = node
                    .child_by_field_name("left")
                    .or_else(|| node.child_by_field_name("name"));
                let right = node
                    .child_by_field_name("right")
                    .or_else(|| node.child_by_field_name("value"));
                let visible_scope = !go_filter_declarations_to_call_scope
                    || self.go_enclosing_binding_scope(node).is_some_and(|scope| {
                        scope.start_byte() <= call_start_byte && call_start_byte < scope.end_byte()
                    });
                if let Some(left) = left.filter(|_| visible_scope) {
                    if enable_go_same_scope_reuse
                        && self.go_short_decl_reuses_in_scope(
                            node,
                            &node
                                .parent()
                                .and_then(|mut parent| {
                                    while !self
                                        .language
                                        .function_node_types()
                                        .contains(&parent.kind())
                                        && parent.kind() != "func_literal"
                                    {
                                        parent = parent.parent()?;
                                    }
                                    Some(parent)
                                })
                                .unwrap_or(node),
                            receiver,
                            call_start_byte,
                        )
                    {
                        // Go reuses an existing binding for the already-declared
                        // LHS name. The post-merge pass proves the call's first
                        // return type still matches before trusting this fact.
                    } else if self.simple_binding_text(&left).as_deref() == Some(receiver) {
                        *go_lexical_rebinding |= *bindings > 0;
                        *bindings += 1;
                        *found = right.and_then(|r| {
                            self.constructor_type(&r)
                                .or_else(|| self.first_constructor_type_child(&r))
                                .map(|ty| (ty, ReceiverRecovery::ConstructorLocal))
                        });
                    } else if self.node_binds_name(left, receiver) {
                        *go_lexical_rebinding |= *bindings > 0;
                        *bindings += 1;
                        *found = None;
                    }
                }
            }
            (Language::Go, "type_switch_statement") => {
                let mut cursor = node.walk();
                let call_in_case = node.named_children(&mut cursor).any(|case| {
                    case.kind() == "type_case"
                        && case.start_byte() <= call_start_byte
                        && call_start_byte < case.end_byte()
                });
                if call_in_case {
                    if let Some(alias) = node.child_by_field_name("alias") {
                        if self.simple_binding_text(&alias).as_deref() == Some(receiver)
                            || self.node_binds_name(alias, receiver)
                        {
                            *go_lexical_rebinding |= *bindings > 0;
                            *bindings += 1;
                            *found = None;
                        }
                    }
                }
            }
            (Language::Go, "range_clause") => {
                let call_in_body = node
                    .parent()
                    .filter(|parent| parent.kind() == "for_statement")
                    .and_then(|parent| parent.child_by_field_name("body"))
                    .is_some_and(|body| {
                        body.start_byte() <= call_start_byte && call_start_byte < body.end_byte()
                    });
                if call_in_body && self.node_text(&node).contains(":=") {
                    if let Some(left) = node.child_by_field_name("left") {
                        if self.simple_binding_text(&left).as_deref() == Some(receiver)
                            || self.node_binds_name(left, receiver)
                        {
                            *go_lexical_rebinding |= *bindings > 0;
                            *bindings += 1;
                            *found = None;
                        }
                    }
                }
            }
            (Language::Go, "var_spec") if recover_var => {
                // var_spec.name is multiple:true; match only the bound name(s), never names in
                // the declared type or initializer (that would be a false recovery).
                let visible_scope = !go_filter_declarations_to_call_scope
                    || self.go_enclosing_binding_scope(node).is_some_and(|scope| {
                        scope.start_byte() <= call_start_byte && call_start_byte < scope.end_byte()
                    });
                let mut cur = node.walk();
                let names: Vec<_> = node.children_by_field_name("name", &mut cur).collect();
                let matched = visible_scope
                    && names
                        .iter()
                        .any(|n| self.simple_binding_text(n).as_deref() == Some(receiver));
                if matched {
                    *go_lexical_rebinding |= *bindings > 0;
                    *bindings += 1;
                    if let Some(ty) = node.child_by_field_name("type") {
                        // `var r T` / `var a, b T` — the declared type applies to every name.
                        *found = Some((self.node_text(&ty).to_string(), ReceiverRecovery::VarDecl));
                    } else if names.len() == 1 {
                        // `var r = X{}` — single name ↔ single value; safe to recover the type.
                        *found = node
                            .child_by_field_name("value")
                            .and_then(|value| {
                                self.constructor_type(&value)
                                    .or_else(|| self.first_constructor_type_child(&value))
                            })
                            .map(|ty| (ty, ReceiverRecovery::VarDecl));
                    } else {
                        // multi-name untyped (`var a, b = X{}, Y{}`) — name↔value alignment is
                        // ambiguous; bail rather than emit a wrong edge (whole-branch review #2).
                        *found = None;
                    }
                }
            }
            (Language::Python, "assignment") => {
                let left = node.child_by_field_name("left");
                if let Some(left) = left {
                    if self.simple_binding_text(&left).as_deref() == Some(receiver) {
                        *bindings += 1;
                        if !self.receiver_declaration_reaches_call(node, call_start_byte) {
                            *found = None;
                        } else if let Some(ty) = node.child_by_field_name("type") {
                            *found = Some((
                                self.node_text(&ty).to_string(),
                                ReceiverRecovery::ConstructorLocal,
                            ));
                        } else if let Some(right) = node.child_by_field_name("right") {
                            *found = self
                                .constructor_type(&right)
                                .map(|ty| (ty, ReceiverRecovery::ConstructorLocal));
                        } else {
                            *found = None;
                        }
                    } else if self.node_binds_name(left, receiver) {
                        *bindings += 1;
                        *found = None;
                    }
                }
            }
            (Language::Python, "augmented_assignment") => {
                // `Foo += x` — augmented assignment rebinds the name; type unrecoverable.
                if let Some(left) = node.child_by_field_name("left") {
                    if self.simple_binding_text(&left).as_deref() == Some(receiver) {
                        *bindings += 1;
                        *found = None;
                    }
                }
            }
            (Language::Python, "for_statement") => {
                // `for x in items:` — iteration variable; type unrecoverable.
                if let Some(left) = node.child_by_field_name("left") {
                    if self.simple_binding_text(&left).as_deref() == Some(receiver) {
                        *bindings += 1;
                        *found = None;
                    } else if self.node_binds_name(left, receiver) {
                        *bindings += 1;
                        *found = None;
                    }
                }
            }
            // NOTE: comprehension `for_in_clause` targets are NOT counted here.
            // Python 3 comprehensions have their own scope; the early-return
            // above prevents recursion into them, so they cannot over-suppress
            // names in the enclosing function scope.
            (Language::Python, "named_expression") => {
                // Walrus: `(x := compute())` — type unrecoverable.
                if let Some(name_node) = node.child_by_field_name("name") {
                    if self.simple_binding_text(&name_node).as_deref() == Some(receiver) {
                        *bindings += 1;
                        *found = None;
                    }
                }
            }
            (Language::Python, "as_pattern") => {
                // `with ... as x:` / `except ... as x:` / `case ... as x:`
                // tree-sitter-python: as_pattern field `alias` = as_pattern_target
                // wrapping an identifier.  Also handle destructuring targets
                // like `with cm() as (Foo, other):` where simple_binding_text
                // returns None but node_binds_name finds the name inside a tuple.
                if let Some(alias) = node.child_by_field_name("alias") {
                    if self.simple_binding_text(&alias).as_deref() == Some(receiver) {
                        *bindings += 1;
                        *found = None;
                    } else if self.node_binds_name(alias, receiver) {
                        *bindings += 1;
                        *found = None;
                    }
                }
            }
            (Language::Python, "case_clause") => {
                // Match/case capture patterns bind names in the enclosing scope.
                // `case Foo:` captures and binds `Foo` (if not dotted).
                if let Some(pattern) = node.child_by_field_name("pattern") {
                    if self.simple_binding_text(&pattern).as_deref() == Some(receiver) {
                        *bindings += 1;
                        *found = None;
                    } else if self.node_binds_name(pattern, receiver) {
                        *bindings += 1;
                        *found = None;
                    }
                }
            }
            (Language::Go, "func_literal") if !is_root => {
                let call_inside =
                    call_start_byte >= node.start_byte() && call_start_byte < node.end_byte();
                if !call_inside {
                    return;
                }
                if let Some(params) = self.find_parameters_node(&node) {
                    let mut pcursor = params.walk();
                    for param in params.children(&mut pcursor) {
                        if param.kind() != "parameter_declaration" {
                            continue;
                        }
                        let Some(ty) = param.child_by_field_name("type") else {
                            continue;
                        };
                        if self.go_parameter_binds_name(param, ty, receiver) {
                            *go_lexical_rebinding |= *bindings > 0;
                            *bindings += 1;
                            *found = Some((
                                self.node_text(&ty).to_string(),
                                ReceiverRecovery::TypedParam,
                            ));
                        }
                    }
                }
            }
            (Language::Go, "assignment_statement") | (Language::Rust, "assignment_expression") => {
                let left = node
                    .child_by_field_name("left")
                    .or_else(|| node.child_by_field_name("left_operand"));
                if let Some(left) = left {
                    if self.simple_binding_text(&left).as_deref() == Some(receiver) {
                        *bindings += 1;
                        *found = None;
                    }
                }
            }
            _ => {}
        }
        if *bindings == 1 && first_found.is_none() {
            *first_found = found.clone();
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.walk_receiver_bindings(
                child,
                false,
                receiver,
                call_line,
                call_start_byte,
                go_filter_declarations_to_call_scope,
                enable_go_same_scope_reuse,
                found,
                first_found,
                bindings,
                go_lexical_rebinding,
                recover_var,
            );
        }
    }

    pub(super) fn constructor_type(&self, node: &Node<'_>) -> Option<String> {
        use crate::languages::Language;

        fn python_constructor_name(text: &str) -> bool {
            let owner = crate::call_graph::python_qualified_receiver_parts(text)
                .map(|(_, owner)| owner)
                .unwrap_or(text);
            owner.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                && (owner != text || text.chars().all(|c| c.is_alphanumeric() || c == '_'))
        }

        match node.kind() {
            "call_expression" => {
                let function = self
                    .call_function_node(node)
                    .or_else(|| node.child_by_field_name("name"))?;
                let text = self.node_text(&function).trim();
                if let (Language::Rust, Some((ty, constructor))) =
                    (self.language, text.rsplit_once("::"))
                {
                    if !matches!(constructor, "new" | "default") {
                        return None;
                    }
                    return Some(ty.rsplit("::").next().unwrap_or(ty).to_string());
                }
                if self.language == Language::Go {
                    return text
                        .strip_prefix("New")
                        .filter(|s| !s.is_empty())
                        .map(str::to_string);
                }
                if self.language == Language::Python && python_constructor_name(text) {
                    return Some(text.to_string());
                }
                None
            }
            "call" if self.language == Language::Python => {
                let function = node
                    .child_by_field_name("function")
                    .or_else(|| node.child_by_field_name("name"))?;
                let text = self.node_text(&function).trim();
                if python_constructor_name(text) {
                    return Some(text.to_string());
                }
                None
            }
            "struct_expression" | "composite_literal" => {
                let ty = node
                    .child_by_field_name("name")
                    .or_else(|| node.child_by_field_name("type"))?;
                Some(self.node_text(&ty).to_string())
            }
            _ => None,
        }
    }

    pub(super) fn first_constructor_type_child(&self, node: &Node<'_>) -> Option<String> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(ty) = self.constructor_type(&child) {
                return Some(ty);
            }
        }
        None
    }

    pub(super) fn node_binds_name(&self, node: Node<'_>, receiver: &str) -> bool {
        if self.language.is_identifier_node(node.kind()) && self.node_text(&node) == receiver {
            return true;
        }
        // Attribute/subscript accesses reference but don't bind the name.
        if matches!(
            node.kind(),
            "attribute" | "subscript" | "member_expression" | "subscript_expression"
        ) {
            return false;
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if self.node_binds_name(child, receiver) {
                return true;
            }
        }
        false
    }

    pub(super) fn simple_binding_text(&self, node: &Node<'_>) -> Option<String> {
        let text = self.node_text(node).trim();
        if text.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Some(text.to_string());
        }
        text.strip_prefix("mut ")
            .map(str::trim)
            .filter(|s| s.chars().all(|c| c.is_alphanumeric() || c == '_'))
            .map(str::to_string)
    }

    pub(super) fn parameter_binds_name_before_type(
        &self,
        param: Node<'_>,
        ty: Node<'_>,
        receiver: &str,
    ) -> bool {
        if let Some(pattern) = param
            .child_by_field_name("pattern")
            .or_else(|| param.child_by_field_name("name"))
        {
            return self.simple_binding_text(&pattern).as_deref() == Some(receiver);
        }
        let mut cursor = param.walk();
        for child in param.children(&mut cursor) {
            if child.start_byte() >= ty.start_byte() {
                continue;
            }
            if self.simple_binding_text(&child).as_deref() == Some(receiver) {
                return true;
            }
        }
        false
    }
}
