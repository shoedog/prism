use super::*;

pub(super) fn non_runtime_edge(parent: Node<'_>, child: Node<'_>) -> bool {
    let field = match parent.kind() {
        "attribute_item" | "inner_attribute_item" | "const_block" | "type_arguments" => {
            return true
        }
        "array_type" => "length",
        "const_item" | "static_item" | "const_parameter" | "enum_variant" => "value",
        _ => return false,
    };
    parent.child_by_field_name(field) == Some(child)
}

pub(super) fn callee(head: Node<'_>) -> Result<Path<'_>, Reason> {
    match head.kind() {
        "identifier" => Ok(Path::plain(head)),
        "field_expression" => field(head, "field", "field_identifier")
            .map(|name| Path::member(head, name, head.child_by_field_name("value")))
            .ok_or(Reason::CalleeKindUnlisted),
        "scoped_identifier" => {
            let name = field(head, "name", "identifier").ok_or(Reason::CalleeKindUnlisted)?;
            let mut path = head.child_by_field_name("path");
            while let Some(segment) = path {
                match segment.kind() {
                    "identifier" | "crate" | "self" | "super" => break,
                    "scoped_identifier" => {
                        if !segment
                            .child_by_field_name("name")
                            .is_some_and(|node| matches!(node.kind(), "identifier" | "super"))
                        {
                            return Err(Reason::CalleeKindUnlisted);
                        }
                        path = segment.child_by_field_name("path");
                    }
                    _ => return Err(Reason::CalleeKindUnlisted),
                }
            }
            Ok(Path::member(head, name, None))
        }
        _ => Err(Reason::CalleeKindUnlisted),
    }
}

pub(super) fn member(node: Node<'_>) -> bool {
    rooted_member(node, "field_expression", |access| {
        field(access, "field", "field_identifier")?;
        access.child_by_field_name("value")
    })
}

pub(super) fn value(node: Node<'_>) -> bool {
    match node.kind() {
        "integer_literal" | "float_literal" | "char_literal" | "string_literal"
        | "raw_string_literal" | "boolean_literal" => true,
        _ => member(node),
    }
}
