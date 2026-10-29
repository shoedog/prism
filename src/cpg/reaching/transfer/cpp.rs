use crate::ast::ParsedFile;
use crate::languages::Language;
use tree_sitter::Node;

pub(super) fn collect_introduced_identifiers<'a>(
    parsed: &ParsedFile,
    node: Node<'a>,
    identifiers: &mut Vec<Node<'a>>,
) -> bool {
    let Some(found) = introduced_identifiers(parsed, node) else {
        return false;
    };
    identifiers.extend(found);
    true
}

pub(super) fn collect_row<'a>(
    parsed: &ParsedFile,
    node: Node<'a>,
    fields: &[&str],
    identifiers: &mut Vec<Node<'a>>,
) {
    if fields.is_empty()
        && matches!(
            node.kind(),
            "structured_binding_declarator" | "lambda_capture_specifier"
        )
    {
        collect_introduced_identifiers(parsed, node, identifiers);
    }
}

fn reference_capture_identifier(node: Node<'_>) -> Option<Node<'_>> {
    let identifier = if node.kind() == "lambda_capture_initializer" {
        node.child_by_field_name("left")
    } else if node.kind() == "identifier" {
        Some(node)
    } else {
        None
    };
    identifier.filter(|identifier| {
        identifier
            .prev_sibling()
            .is_some_and(|previous| previous.kind() == "&")
    })
}

fn capture_is_by_reference(node: Node<'_>) -> bool {
    reference_capture_identifier(node).is_some()
}

pub(in crate::cpg::reaching) fn reference_capture_only(node: Node<'_>) -> bool {
    if node.kind() != "lambda_capture_specifier" {
        return false;
    }
    let mut cursor = node.walk();
    let captures: Vec<_> = node.named_children(&mut cursor).collect();
    !captures.is_empty() && captures.into_iter().all(capture_is_by_reference)
}

pub(in crate::cpg::reaching) fn normalize_cpp_pseudo_defs(
    parsed: &ParsedFile,
    defs: &mut [super::super::DefSite],
) {
    fn mark_identifier(
        parsed: &ParsedFile,
        defs: &mut [super::super::DefSite],
        identifier: Node<'_>,
        start: usize,
    ) {
        let name = parsed.node_text(&identifier);
        for def in defs.iter_mut().filter(|def| {
            def.path.base == name
                && start <= def.start_byte
                && def.start_byte < identifier.end_byte()
        }) {
            def.alias_derived = true;
        }
    }

    fn is_mutating_target(identifier: Node<'_>, stop: Node<'_>) -> bool {
        let mut current = identifier;
        while current.id() != stop.id() {
            let Some(parent) = current.parent() else {
                return false;
            };
            if parent.kind() == "assignment_expression"
                && parent.child_by_field_name("left").is_some_and(|left| {
                    left.start_byte() <= identifier.start_byte()
                        && identifier.end_byte() <= left.end_byte()
                })
            {
                return true;
            }
            if parent.kind() == "update_expression" {
                return true;
            }
            current = parent;
        }
        false
    }

    fn mark_reads(
        parsed: &ParsedFile,
        defs: &mut [super::super::DefSite],
        root: Node<'_>,
        names: Option<&[String]>,
    ) {
        if parsed
            .language
            .callable_boundary_node_types()
            .contains(&root.kind())
        {
            return;
        }
        let mut pending = vec![root];
        while let Some(descendant) = pending.pop() {
            if descendant.id() != root.id()
                && parsed
                    .language
                    .callable_boundary_node_types()
                    .contains(&descendant.kind())
            {
                continue;
            }
            if parsed.language.is_identifier_node(descendant.kind()) {
                let name = parsed.node_text(&descendant);
                if names.is_none_or(|names| names.iter().any(|candidate| candidate == name))
                    && !is_mutating_target(descendant, root)
                    && !is_declaration_introduction(parsed, descendant)
                {
                    mark_identifier(parsed, defs, descendant, descendant.start_byte());
                }
                continue;
            }
            let mut children = descendant.walk();
            pending.extend(descendant.named_children(&mut children));
        }
    }

    fn is_declaration_introduction(parsed: &ParsedFile, identifier: Node<'_>) -> bool {
        let name = parsed.node_text(&identifier);
        std::iter::successors(Some(identifier), |node| node.parent()).any(|node| {
            super::super::binding_table::select_binding_row(parsed, node).is_some_and(|row| {
                row.declaration.is_some()
                    && row_owns_source(parsed, node, row.fields, name, identifier.start_byte())
            })
        })
    }

    if parsed.language != Language::Cpp {
        return;
    }
    let mut stack = vec![parsed.tree.root_node()];
    while let Some(node) = stack.pop() {
        if node.kind() == "lambda_capture_specifier" {
            let mut cursor = node.walk();
            for capture in node.named_children(&mut cursor) {
                let Some(identifier) = reference_capture_identifier(capture) else {
                    continue;
                };
                let start = identifier
                    .prev_sibling()
                    .filter(|previous| previous.kind() == "&")
                    .map_or(identifier.start_byte(), |previous| previous.start_byte());
                mark_identifier(parsed, defs, identifier, start);
            }
            let mut cursor = node.walk();
            for initializer in node
                .named_children(&mut cursor)
                .filter(|child| child.kind() == "lambda_capture_initializer")
            {
                let Some(right) = initializer.child_by_field_name("right") else {
                    continue;
                };
                mark_reads(parsed, defs, right, None);
            }
            continue;
        }
        if node.kind() == "lambda_expression" {
            if let (Some(captures), Some(body)) = (
                node.child_by_field_name("captures"),
                node.child_by_field_name("body"),
            ) {
                let mut cursor = captures.walk();
                let names: Vec<_> = captures
                    .named_children(&mut cursor)
                    .filter_map(reference_capture_identifier)
                    .map(|identifier| parsed.node_text(&identifier).to_string())
                    .collect();
                if !names.is_empty() {
                    mark_reads(parsed, defs, body, Some(&names));
                }
            }
        }
        if node.kind() == "init_declarator" && belongs_to_declarator_chain(node) {
            if let Some(value) = node.child_by_field_name("value") {
                mark_reads(parsed, defs, value, None);
            }
        }
        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
}

fn belongs_to_declarator_chain(node: Node<'_>) -> bool {
    node.parent().is_some_and(|parent| {
        if parent.kind() != "declaration" {
            return false;
        }
        let mut declarators = parent.walk();
        let is_chain = parent
            .children_by_field_name("declarator", &mut declarators)
            .nth(1)
            .is_some();
        is_chain
    })
}

pub(super) fn declarator_producer_owns_identifier(
    parsed: &ParsedFile,
    row_node: Node<'_>,
    identifier: Node<'_>,
    producer_byte: usize,
) -> bool {
    if parsed.language != Language::Cpp {
        return false;
    }
    let Some(end_byte) = producer_byte.checked_add(1) else {
        return false;
    };
    let Some(mut current) = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(producer_byte, end_byte)
    else {
        return false;
    };
    loop {
        if current.start_byte() == producer_byte
            && current.start_byte() <= identifier.start_byte()
            && identifier.end_byte() <= current.end_byte()
            && matches!(
                current.kind(),
                "init_declarator"
                    | "array_declarator"
                    | "attributed_declarator"
                    | "function_declarator"
                    | "parenthesized_declarator"
                    | "pointer_declarator"
                    | "reference_declarator"
            )
        {
            return true;
        }
        if current.id() == row_node.id() {
            return false;
        }
        let Some(parent) = current.parent() else {
            return false;
        };
        current = parent;
    }
}

pub(in crate::cpg::reaching) fn row_owns_source(
    parsed: &ParsedFile,
    node: Node<'_>,
    fields: &[&str],
    name: &str,
    byte: usize,
) -> bool {
    super::row_introduced_identifiers(parsed, node, fields)
        .into_iter()
        .filter(|identifier| parsed.node_text(identifier) == name)
        .any(|identifier| {
            identifier.start_byte() == byte
                || declarator_producer_owns_identifier(parsed, node, identifier, byte)
        })
}

pub(in crate::cpg::reaching) fn row_owns_def(
    parsed: &ParsedFile,
    node: Node<'_>,
    fields: &[&str],
    def: &super::super::DefSite,
) -> bool {
    super::row_introduced_identifiers(parsed, node, fields)
        .into_iter()
        .any(|identifier| identifier_owns_def(parsed, node, identifier, def))
}

pub(super) fn identifier_owns_def(
    parsed: &ParsedFile,
    node: Node<'_>,
    identifier: Node<'_>,
    def: &super::super::DefSite,
) -> bool {
    if parsed.language == Language::Cpp {
        parsed.node_text(&identifier) == def.path.base
            && (identifier.start_byte() == def.start_byte
                || declarator_producer_owns_identifier(parsed, node, identifier, def.start_byte))
    } else {
        identifier.start_byte() == def.start_byte
    }
}

pub(in crate::cpg::reaching) fn row_excludes_def(
    parsed: &ParsedFile,
    node: Node<'_>,
    fields: &[&str],
    def: &super::super::DefSite,
) -> bool {
    (parsed.language == Language::Cpp || !fields.is_empty())
        && !row_owns_def(parsed, node, fields, def)
}

fn introduced_identifiers<'a>(parsed: &ParsedFile, node: Node<'a>) -> Option<Vec<Node<'a>>> {
    if parsed.language != Language::Cpp {
        return None;
    }
    let mut identifiers = Vec::new();
    match node.kind() {
        "lambda_capture_specifier" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                if child.kind() == "identifier" && !capture_is_by_reference(child) {
                    identifiers.push(child);
                } else if child.kind() == "lambda_capture_initializer"
                    && !capture_is_by_reference(child)
                {
                    identifiers.extend(child.child_by_field_name("left"));
                }
            }
        }
        "structured_binding_declarator" => {
            let mut cursor = node.walk();
            identifiers.extend(
                node.named_children(&mut cursor)
                    .filter(|child| parsed.language.is_identifier_node(child.kind())),
            );
        }
        "parameter_list" => {
            let mut cursor = node.walk();
            for parameter in node
                .named_children(&mut cursor)
                .filter(|child| child.kind() == "parameter_declaration")
            {
                collect(parsed, parameter, &mut identifiers);
            }
        }
        "init_declarator"
        | "parameter_declaration"
        | "array_declarator"
        | "attributed_declarator"
        | "function_declarator"
        | "parenthesized_declarator"
        | "pointer_declarator"
        | "reference_declarator" => {
            if let Some(declarator) = node.child_by_field_name("declarator").or_else(|| {
                (node.kind() == "reference_declarator")
                    .then(|| node.named_child(0))
                    .flatten()
            }) {
                collect(parsed, declarator, &mut identifiers);
            }
        }
        _ => return None,
    }
    Some(identifiers)
}

fn collect<'a>(parsed: &ParsedFile, node: Node<'a>, identifiers: &mut Vec<Node<'a>>) {
    if parsed.language.is_identifier_node(node.kind()) {
        identifiers.push(node);
    } else if let Some(nested) = introduced_identifiers(parsed, node) {
        identifiers.extend(nested);
    }
}
