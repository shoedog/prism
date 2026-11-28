use super::operation::Form;
use super::*;

fn query_head(parsed: &ParsedFile, node: Node<'_>) -> bool {
    callee::unwrap_head(parsed.language, node).is_some_and(|head| {
        head.kind() == "identifier"
            && match parsed.node_text(&head) {
                "typeof" | "__typeof" | "__typeof__" => true,
                "noexcept" | "typeid" => parsed.language == Language::Cpp,
                _ => false,
            }
    })
}

pub(super) fn check_call(
    parsed: &ParsedFile,
    op: Node<'_>,
    form: Form,
    head: Node<'_>,
) -> Result<(), Reason> {
    if (form == Form::Call && query_head(parsed, head))
        || (form == Form::Constructor
            && ["placement", "declarator"]
                .iter()
                .any(|name| op.child_by_field_name(name).is_some()))
    {
        return Err(Reason::OpSpecialForm);
    }
    Ok(())
}

pub(super) fn non_runtime_edge(parsed: &ParsedFile, parent: Node<'_>, child: Node<'_>) -> bool {
    match parent.kind() {
        "attribute_specifier"
        | "attribute_declaration"
        | "attribute"
        | "alignas_qualifier"
        | "sizeof_expression"
        | "alignof_expression"
        | "generic_expression"
        | "decltype"
        | "noexcept"
        | "requires_expression"
        | "requires_clause"
        | "macro_type_specifier"
        | "static_assert_declaration"
        | "template_argument_list"
        | "concept_definition"
        | "bitfield_clause" => true,
        "preproc_if" | "preproc_elif" => parent.child_by_field_name("condition") == Some(child),
        "preproc_def" | "preproc_function_def" | "enumerator" | "case_statement" => {
            parent.child_by_field_name("value") == Some(child)
        }
        "call_expression" => parent
            .child_by_field_name("function")
            .is_some_and(|head| query_head(parsed, head)),
        _ => false,
    }
}

pub(super) fn callee(language: Language, head: Node<'_>) -> Result<Path<'_>, Reason> {
    match head.kind() {
        "identifier" => Ok(Path::plain(head)),
        "primitive_type" => Err(Reason::CalleeTypeUnlisted),
        "subscript_expression" => Err(Reason::SubscriptCFamily),
        "field_expression" => {
            let operator = head.child_by_field_name("operator").map(|node| node.kind());
            match operator {
                Some(".*") => return Err(Reason::MemberPointerOperator),
                Some("." | "->") => {}
                _ => return Err(Reason::CalleeKindUnlisted),
            }
            let name = head
                .child_by_field_name("field")
                .ok_or(Reason::CalleeKindUnlisted)?;
            let terminal = match name.kind() {
                "field_identifier" => name,
                "qualified_identifier" if language == Language::Cpp => {
                    qualified(name, "field_identifier").ok_or(Reason::CalleeKindUnlisted)?
                }
                _ => return Err(Reason::CalleeKindUnlisted),
            };
            Ok(Path::member(
                head,
                terminal,
                head.child_by_field_name("argument"),
            ))
        }
        "qualified_identifier" if language == Language::Cpp => qualified(head, "identifier")
            .map(|name| Path::member(head, name, None))
            .ok_or(Reason::CalleeKindUnlisted),
        _ => Err(Reason::CalleeKindUnlisted),
    }
}

fn qualified<'tree>(mut node: Node<'tree>, terminal_kind: &str) -> Option<Node<'tree>> {
    while node.kind() == "qualified_identifier" {
        field(node, "scope", "namespace_identifier")?;
        match fields(node, "name").as_slice() {
            [name] => node = *name,
            _ => return None,
        }
    }
    (node.kind() == terminal_kind).then_some(node)
}

pub(super) fn constructor(head: Node<'_>) -> Result<Path<'_>, Reason> {
    match head.kind() {
        "type_identifier" => Ok(Path::plain(head)),
        "qualified_identifier" => qualified(head, "type_identifier")
            .map(|name| Path::member(head, name, None))
            .ok_or(Reason::CalleeTypeUnlisted),
        _ => Err(Reason::CalleeTypeUnlisted),
    }
}

pub(super) fn member(node: Node<'_>) -> bool {
    rooted_member(node, "field_expression", |access| {
        let operator = access.child_by_field_name("operator")?;
        if !matches!(operator.kind(), "." | "->") {
            return None;
        }
        field(access, "field", "field_identifier")?;
        access.child_by_field_name("argument")
    })
}

pub(super) fn value(language: Language, node: Node<'_>) -> bool {
    match node.kind() {
        "number_literal" | "char_literal" | "string_literal" => true,
        "raw_string_literal" | "true" | "false" => language == Language::Cpp,
        _ => member(node),
    }
}
