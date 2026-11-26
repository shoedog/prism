use super::*;

pub(super) fn non_runtime_edge(parent: Node<'_>, child: Node<'_>) -> bool {
    let field = match parent.kind() {
        "array_type" => "length",
        "const_spec" => "value",
        "type_arguments" => return true,
        _ => return false,
    };
    parent.child_by_field_name(field) == Some(child)
}

pub(super) fn check_call(parsed: &ParsedFile, op: Node<'_>, head: Node<'_>) -> Result<(), Reason> {
    if op.child_by_field_name("type_arguments").is_some() {
        return Err(Reason::OpTypeArguments);
    }
    if callee::unwrap_head(Language::Go, head).is_some_and(|head| {
        head.kind() == "identifier" && matches!(parsed.node_text(&head), "make" | "new")
    }) {
        return Err(Reason::OpSpecialForm);
    }
    Ok(())
}

pub(super) fn callee(head: Node<'_>) -> Result<Path<'_>, Reason> {
    match head.kind() {
        "identifier" => Ok(Path::plain(head)),
        "selector_expression" => field(head, "field", "field_identifier")
            .map(|name| Path::member(head, name, head.child_by_field_name("operand")))
            .ok_or(Reason::CalleeKindUnlisted),
        _ => Err(Reason::CalleeKindUnlisted),
    }
}

pub(super) fn member(node: Node<'_>) -> bool {
    rooted_member(node, "selector_expression", |selector| {
        field(selector, "field", "field_identifier")?;
        selector.child_by_field_name("operand")
    })
}

pub(super) fn value(node: Node<'_>) -> bool {
    match node.kind() {
        "int_literal"
        | "float_literal"
        | "imaginary_literal"
        | "rune_literal"
        | "true"
        | "false"
        | "nil"
        | "interpreted_string_literal"
        | "raw_string_literal" => true,
        _ => member(node),
    }
}
