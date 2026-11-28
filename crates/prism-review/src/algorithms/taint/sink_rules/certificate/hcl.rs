use super::*;

pub(super) fn arguments(op: Node<'_>) -> Result<Node<'_>, Reason> {
    match tokens(op).as_slice() {
        [head, open, args, close]
            if head.kind() == "identifier"
                && open.kind() == "("
                && args.kind() == "function_arguments"
                && close.kind() == ")" =>
        {
            Ok(*args)
        }
        _ => Err(Reason::OpArgumentsUnlisted),
    }
}

pub(super) fn assignment(op: Node<'_>) -> Result<(Node<'_>, Node<'_>), Reason> {
    match tokens(op).as_slice() {
        [head, equals, value]
            if head.kind() == "identifier"
                && equals.kind() == "="
                && value.kind() == "expression" =>
        {
            Ok((*head, *value))
        }
        _ => Err(Reason::RoleNotBound),
    }
}

pub(super) fn value(node: Node<'_>) -> bool {
    if node.kind() != "expression" {
        return false;
    }
    match tokens(node).as_slice() {
        [literal] if literal.kind() == "literal_value" => matches!(
            named(*literal).as_slice(),
            [value] if matches!(value.kind(), "numeric_lit" | "bool_lit" | "null_lit" | "string_lit")
        ),
        [base, tail @ ..] if base.kind() == "variable_expr" => {
            matches!(named(*base).as_slice(), [name] if name.kind() == "identifier")
                && tail.iter().all(|part| {
                    part.kind() == "get_attr"
                        && matches!(named(*part).as_slice(), [name] if name.kind() == "identifier")
                })
        }
        _ => false,
    }
}

pub(super) fn splat(node: Node<'_>) -> bool {
    node.kind() == "expression" && named(node).iter().any(|node| node.kind() == "splat")
}
