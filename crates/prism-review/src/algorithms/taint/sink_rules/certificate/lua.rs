use super::*;

pub(super) fn callee(head: Node<'_>) -> Result<Path<'_>, Reason> {
    let receiver = head.child_by_field_name("table");
    let name = match head.kind() {
        "identifier" => return Ok(Path::plain(head)),
        "dot_index_expression" => field(head, "field", "identifier"),
        "method_index_expression" => field(head, "method", "identifier"),
        _ => None,
    };
    name.map(|name| Path::member(head, name, receiver))
        .ok_or(Reason::CalleeKindUnlisted)
}

pub(super) fn member(node: Node<'_>) -> bool {
    rooted_member(node, "dot_index_expression", |access| {
        field(access, "field", "identifier")?;
        access.child_by_field_name("table")
    })
}

pub(super) fn value(node: Node<'_>) -> bool {
    match node.kind() {
        "number" | "string" | "true" | "false" | "nil" => true,
        _ => member(node),
    }
}

pub(super) fn assignment(op: Node<'_>) -> Result<(Node<'_>, Node<'_>), Reason> {
    let children = named(op);
    let [left, right] = children.as_slice() else {
        return Err(Reason::RoleNotBound);
    };
    if left.kind() != "variable_list" || right.kind() != "expression_list" {
        return Err(Reason::RoleNotBound);
    }
    match (
        fields(*left, "name").as_slice(),
        fields(*right, "value").as_slice(),
    ) {
        ([target], [value]) => Ok((*target, *value)),
        _ => Err(Reason::ParallelAssignment),
    }
}
