use super::*;

pub(super) fn declaration_seed(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
    def: &DefSite,
    function_scope: ScopeSpan,
) -> Option<(ScopeSpan, DeclarationKind, usize)> {
    // Twins name the alias source at another identifier's bytes. Real defs
    // are registered separately; a twin never introduces a Go binding.
    if parsed.language == Language::Go && def.alias_derived {
        return None;
    }
    // js-param-defs PR-A: the bare arrow formal (`x => …`) has no
    // `parameters` list; without its region the Def is seeded as a non-
    // parameter binding and its labels diverge from the `(x) => …` control.
    if parsed
        .parameter_binding_region(func_node)
        .is_some_and(|parameters| {
            parameters.start_byte() <= def.start_byte && def.start_byte < parameters.end_byte()
        })
    {
        return Some((
            function_scope,
            DeclarationKind::Parameter,
            function_scope.start_byte,
        ));
    }

    let end_byte = def.start_byte.checked_add(1)?;
    let mut node = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(def.start_byte, end_byte)?;
    loop {
        if matches!(
            parsed.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) && node.kind() == "for_in_statement"
        {
            let left = node.child_by_field_name("left");
            let lexical_kind = node
                .child_by_field_name("kind")
                .map(|kind| parsed.node_text(&kind));
            if left.is_some_and(|left| {
                left.start_byte() <= def.start_byte && def.start_byte < left.end_byte()
            }) && matches!(lexical_kind, Some("let" | "const"))
            {
                return Some((
                    scope_span(node),
                    DeclarationKind::Other,
                    left.expect("checked above").end_byte(),
                ));
            }
        }
        let rule = binding_scope_rule(parsed.language, node.kind());
        if let Some(kind) = rule.declaration {
            let value = if kind == DeclarationKind::PythonAssignment {
                parsed.language.assignment_value(&node)
            } else {
                parsed.language.declaration_value(&node)
            };
            if value.is_some_and(|value| {
                value.start_byte() <= def.start_byte && def.start_byte < value.end_byte()
            }) {
                return None;
            }
            let scope = declaration_scope(parsed, func_node, node, kind, function_scope);
            let visible_from = match kind {
                DeclarationKind::Parameter
                | DeclarationKind::JavaScriptVar
                | DeclarationKind::PythonAssignment => scope.start_byte,
                DeclarationKind::GoShort | DeclarationKind::Other => node.end_byte(),
            };
            return Some((scope, kind, visible_from));
        }
        if node.id() == func_node.id() {
            return None;
        }
        node = node.parent()?;
    }
}

fn declaration_scope(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
    node: Node<'_>,
    kind: DeclarationKind,
    function_scope: ScopeSpan,
) -> ScopeSpan {
    if kind == DeclarationKind::JavaScriptVar {
        return function_scope;
    }
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.id() == func_node.id() {
            break;
        }
        if binding_scope_rule(parsed.language, parent.kind()).creates_scope {
            return scope_span(parent);
        }
        current = parent.parent();
    }
    function_scope
}

pub(super) fn binding_scope_rule(language: Language, kind: &str) -> BindingScopeRule {
    let (creates_scope, declaration) = match language {
        Language::Python => (
            matches!(
                kind,
                "function_definition"
                    | "lambda"
                    | "class_definition"
                    | "list_comprehension"
                    | "set_comprehension"
                    | "dictionary_comprehension"
                    | "generator_expression"
            ),
            matches!(
                kind,
                "assignment" | "augmented_assignment" | "named_expression"
            )
            .then_some(DeclarationKind::PythonAssignment),
        ),
        Language::JavaScript | Language::TypeScript | Language::Tsx => (
            matches!(
                kind,
                "statement_block"
                    | "class_body"
                    | "function_declaration"
                    | "function_expression"
                    | "arrow_function"
                    | "for_statement"
                    | "for_in_statement"
            ),
            match kind {
                "variable_declaration" => Some(DeclarationKind::JavaScriptVar),
                "lexical_declaration" | "class_declaration" => Some(DeclarationKind::Other),
                _ => None,
            },
        ),
        Language::Go => (
            matches!(
                kind,
                "block"
                    | "if_statement"
                    | "for_statement"
                    | "expression_switch_statement"
                    | "type_switch_statement"
                    | "switch_statement"
                    | "select_statement"
                    | "expression_case"
                    | "type_case"
                    | "communication_case"
                    | "default_case"
            ),
            match kind {
                "short_var_declaration" => Some(DeclarationKind::GoShort),
                "var_declaration" | "const_declaration" => Some(DeclarationKind::Other),
                _ => None,
            },
        ),
        Language::Rust => (
            kind == "block",
            matches!(kind, "let_declaration" | "const_item" | "static_item")
                .then_some(DeclarationKind::Other),
        ),
        _ => (false, None),
    };
    BindingScopeRule {
        creates_scope,
        declaration,
    }
}

pub(super) fn collect_unclassified_binding_lines(
    parsed: &ParsedFile,
    node: Node<'_>,
    root: &RootFunction<'_>,
    callable_boundary_kinds: &[&str],
    introducing_fields: &IntroducingFields,
    lines: &mut BTreeMap<String, BTreeSet<Line>>,
) {
    #[cfg(test)]
    super::super::reaching::work_tests::bump(|counts| counts.binding_nodes += 1);
    let root_function_id = root.function.id();
    if node.id() != root_function_id && callable_boundary_kinds.contains(&node.kind()) {
        return;
    }

    if node.id() != root_function_id {
        if let Some(fields) = introducing_fields.get(node.kind()) {
            for field in fields {
                if let Some(target) = node.child_by_field_name(field) {
                    if !introduction_is_classified(parsed, node, target, root) {
                        let dominance_lines = dominance_route_lines(node, root_function_id);
                        collect_binding_identifiers(parsed, target, &dominance_lines, lines);
                    }
                }
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_unclassified_binding_lines(
            parsed,
            child,
            root,
            callable_boundary_kinds,
            introducing_fields,
            lines,
        );
    }
}

fn collect_binding_identifiers(
    parsed: &ParsedFile,
    node: Node<'_>,
    dominance_lines: &[Line],
    lines: &mut BTreeMap<String, BTreeSet<Line>>,
) {
    let mut cursor = node.walk();
    let named_children: Vec<_> = node.named_children(&mut cursor).collect();
    if named_children.is_empty() {
        let name = parsed.node_text(&node);
        if is_binding_name(name) {
            let entry = lines.entry(name.to_string()).or_default();
            entry.insert(node.start_position().row + 1);
            entry.extend(dominance_lines.iter().copied());
        }
        return;
    }
    if parsed.language.is_identifier_node(node.kind()) {
        lines
            .entry(parsed.node_text(&node).to_string())
            .or_default()
            .insert(node.start_position().row + 1);
        return;
    }
    for child in named_children {
        collect_binding_identifiers(parsed, child, dominance_lines, lines);
    }
}

fn dominance_route_lines(node: Node<'_>, root_function_id: usize) -> Vec<Line> {
    #[cfg(test)]
    super::super::reaching::work_tests::bump(|counts| counts.dominance_calls += 1);
    let introduction_line = node.start_position().row + 1;
    let mut current = Some(node);
    while let Some(candidate) = current {
        if candidate.id() == root_function_id {
            break;
        }
        if let Some(body) = candidate.child_by_field_name("body") {
            let mut lines = BTreeSet::new();
            collect_lines_after(body, introduction_line, &mut lines);
            if !lines.is_empty() {
                return lines.into_iter().collect();
            }
        }
        current = candidate.parent();
    }
    Vec::new()
}

fn collect_lines_after(node: Node<'_>, introduction_line: Line, lines: &mut BTreeSet<Line>) {
    #[cfg(test)]
    super::super::reaching::work_tests::bump(|counts| counts.dominance_nodes += 1);
    let line = node.start_position().row + 1;
    if line > introduction_line {
        lines.insert(line);
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_lines_after(child, introduction_line, lines);
    }
}

fn is_binding_name(text: &str) -> bool {
    text != "_"
        && !text.is_empty()
        && text.bytes().enumerate().all(|(index, byte)| {
            byte == b'_' || byte.is_ascii_alphanumeric() && (index > 0 || !byte.is_ascii_digit())
        })
}

pub(super) fn introduction_is_classified(
    parsed: &ParsedFile,
    node: Node<'_>,
    target: Node<'_>,
    root: &RootFunction<'_>,
) -> bool {
    #[cfg(test)]
    super::super::reaching::work_tests::bump(|counts| counts.introductions += 1);
    if parsed.language == Language::Go && go_bindings::classified(node.kind()) {
        return true;
    }
    let parameters = root.parameters(parsed);
    #[cfg(test)]
    let parameters =
        super::super::reaching::work_tests::parameters(parsed, root.function, parameters);
    if parameters.is_some_and(|parameters| {
        parameters.start_byte() <= target.start_byte() && target.end_byte() <= parameters.end_byte()
    }) {
        return true;
    }

    if parsed.language == Language::Python && is_python_comprehension_target(node) {
        return true;
    }

    if matches!(
        parsed.language,
        Language::JavaScript | Language::TypeScript | Language::Tsx
    ) && node.kind() == "for_in_statement"
        && target.kind() == "identifier"
        && node
            .child_by_field_name("kind")
            .is_some_and(|kind| matches!(parsed.node_text(&kind), "let" | "const"))
    {
        return true;
    }

    let composite_pattern = matches!(
        parsed.language,
        Language::JavaScript | Language::TypeScript | Language::Tsx | Language::Rust
    ) && target.kind() != "identifier";
    let mut current = Some(node);
    while let Some(candidate) = current {
        if binding_scope_rule(parsed.language, candidate.kind())
            .declaration
            .is_some()
        {
            return !composite_pattern;
        }
        if candidate.id() == root.function.id() {
            break;
        }
        current = candidate.parent();
    }
    false
}

fn is_python_comprehension_target(node: Node<'_>) -> bool {
    if node.kind() != "for_in_clause" {
        return false;
    }
    let mut current = node.parent();
    while let Some(candidate) = current {
        if matches!(
            candidate.kind(),
            "list_comprehension"
                | "set_comprehension"
                | "dictionary_comprehension"
                | "generator_expression"
        ) {
            return true;
        }
        current = candidate.parent();
    }
    false
}

pub(super) fn grammar_introducing_fields(language: Language) -> &'static IntroducingFields {
    static BY_LANGUAGE: OnceLock<BTreeMap<Language, IntroducingFields>> = OnceLock::new();
    BY_LANGUAGE
        .get_or_init(|| {
            Language::all()
                .into_iter()
                .map(|language| (language, derive_introducing_fields(language)))
                .collect()
        })
        .get(&language)
        .expect("every supported language has node-type metadata")
}

fn derive_introducing_fields(language: Language) -> IntroducingFields {
    let schemas: Vec<NodeTypeSchema> = serde_json::from_str(language.node_types_json())
        .expect("tree-sitter node-types.json must be valid");
    let members: BTreeMap<_, _> = schemas
        .iter()
        .map(|schema| {
            let mut kinds: Vec<_> = schema
                .subtypes
                .iter()
                .map(|member| member.kind.as_str())
                .collect();
            if let Some(children) = &schema.children {
                kinds.extend(children.types.iter().map(|member| member.kind.as_str()));
            }
            for field in schema.fields.values() {
                kinds.extend(field.types.iter().map(|member| member.kind.as_str()));
            }
            (schema.kind.as_str(), kinds)
        })
        .collect();
    schemas
        .iter()
        .filter_map(|schema| {
            let fields: BTreeSet<_> = schema
                .fields
                .iter()
                .filter(|(field, members_for_field)| {
                    is_name_introducing_field(field)
                        && (is_intrinsically_introducing_field(field)
                            || looks_name_introducing_kind(&schema.kind))
                        && members_for_field.types.iter().any(|member| {
                            type_can_contain_identifier(
                                &member.kind,
                                &members,
                                &mut BTreeSet::new(),
                            )
                        })
                })
                .map(|(field, _)| field.clone())
                .collect();
            (!fields.is_empty()).then_some((schema.kind.clone(), fields))
        })
        .collect()
}

fn looks_name_introducing_kind(kind: &str) -> bool {
    [
        "declaration",
        "declarator",
        "parameter",
        "pattern",
        "clause",
        "for_",
        "for_statement",
        "with_",
        "except_",
        "catch_",
        "binding",
        "variable",
    ]
    .iter()
    .any(|part| kind.contains(part))
        || kind.ends_with("_item")
        || kind.ends_with("_arm")
}

fn is_name_introducing_field(field: &str) -> bool {
    matches!(
        field,
        "name" | "pattern" | "left" | "target" | "alias" | "parameter" | "parameters"
    ) || field.ends_with("_name")
        || field.ends_with("_pattern")
        || field.ends_with("_target")
}

fn is_intrinsically_introducing_field(field: &str) -> bool {
    matches!(field, "pattern" | "target" | "parameter" | "parameters")
        || field.ends_with("_pattern")
        || field.ends_with("_target")
}

fn type_can_contain_identifier<'a>(
    kind: &'a str,
    members: &BTreeMap<&'a str, Vec<&'a str>>,
    visiting: &mut BTreeSet<&'a str>,
) -> bool {
    if kind.contains("identifier")
        || kind.ends_with("_target")
        || matches!(kind, "variable_name" | "name")
    {
        return true;
    }
    if !visiting.insert(kind) {
        return false;
    }
    let result = members.get(kind).is_some_and(|children| {
        children
            .iter()
            .any(|child| type_can_contain_identifier(child, members, visiting))
    });
    visiting.remove(kind);
    result
}

pub(super) fn collect_python_comprehension_declarations(
    parsed: &ParsedFile,
    node: Node<'_>,
    root_function_id: usize,
    declarations: &mut Vec<Declaration>,
) {
    if node.id() != root_function_id
        && matches!(
            node.kind(),
            "function_definition" | "lambda" | "class_definition"
        )
    {
        return;
    }

    if node.kind() == "for_in_clause" {
        let mut parent = node.parent();
        while let Some(candidate) = parent {
            if matches!(
                candidate.kind(),
                "list_comprehension"
                    | "set_comprehension"
                    | "dictionary_comprehension"
                    | "generator_expression"
            ) {
                if let Some(target) = node.child_by_field_name("left") {
                    let visible_from = node
                        .child_by_field_name("right")
                        .map(|iterable| iterable.end_byte())
                        .unwrap_or_else(|| node.end_byte());
                    collect_python_target_identifiers(
                        parsed,
                        target,
                        scope_span(candidate),
                        visible_from,
                        candidate.child_by_field_name("body").map(scope_span),
                        declarations,
                    );
                }
                break;
            }
            parent = candidate.parent();
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_python_comprehension_declarations(parsed, child, root_function_id, declarations);
    }
}

fn collect_python_target_identifiers(
    parsed: &ParsedFile,
    node: Node<'_>,
    scope: ScopeSpan,
    visible_from: usize,
    body: Option<ScopeSpan>,
    declarations: &mut Vec<Declaration>,
) {
    if node.kind() == "identifier" {
        let declaration_index = declarations.len();
        declarations.push(Declaration {
            binding: Binding {
                id: BindingId::Declaration(declaration_index),
                scope,
                declaration_line: Some(node.start_position().row + 1),
            },
            name: parsed.node_text(&node).to_string(),
            visible_from,
            additional_visibility: body.into_iter().chain([scope_span(node)]).collect(),
        });
        return;
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_python_target_identifiers(parsed, child, scope, visible_from, body, declarations);
    }
}

pub(super) fn function_scope(_parsed: &ParsedFile, func_node: &Node<'_>) -> ScopeSpan {
    func_node
        .child_by_field_name("body")
        .map(scope_span)
        .unwrap_or_else(|| scope_span(*func_node))
}

pub(super) fn scope_span(node: Node<'_>) -> ScopeSpan {
    ScopeSpan {
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
        end_line: node.end_position().row + 1,
    }
}
