use super::operation::Form;
use super::*;

pub(super) fn check_call(op: Node<'_>, form: Form) -> Result<(), Reason> {
    if named(op).iter().any(|child| child.kind() == "super") {
        return Err(Reason::OpSpecialForm);
    }
    if form == Form::Constructor {
        let ty = op.child_by_field_name("type");
        let args = op.child_by_field_name("arguments");
        let children = tokens(op);
        // A qualified creation has a primary expression and dot before `new`.
        let after_new = match children.as_slice() {
            [qualifier, dot, new, rest @ ..]
                if qualifier.is_named() && dot.kind() == "." && new.kind() == "new" =>
            {
                rest
            }
            children => children,
        };
        if after_new
            .iter()
            .any(|&child| child.is_named() && Some(child) != ty && Some(child) != args)
        {
            return Err(Reason::OpSpecialForm);
        }
    }
    Ok(())
}

pub(super) fn method<'tree>(op: Node<'tree>, head: Node<'tree>) -> Result<Path<'tree>, Reason> {
    if head.kind() != "identifier" {
        return Err(Reason::CalleeKindUnlisted);
    }
    Ok(Path::member(head, head, op.child_by_field_name("object")))
}

pub(super) fn constructor(head: Node<'_>) -> Result<Path<'_>, Reason> {
    if head.kind() == "type_identifier" {
        return Ok(Path::plain(head));
    }
    let mut scope = head;
    let mut terminal = None;
    loop {
        if scope.kind() != "scoped_type_identifier" {
            return Err(Reason::CalleeTypeUnlisted);
        }
        match tokens(scope).as_slice() {
            [prefix, dot, name] if dot.kind() == "." && name.kind() == "type_identifier" => {
                terminal.get_or_insert(*name);
                scope = *prefix;
            }
            _ => return Err(Reason::CalleeTypeUnlisted),
        }
        if scope.kind() == "type_identifier" {
            return Ok(Path::member(head, terminal.unwrap(), None));
        }
    }
}

pub(super) fn member(node: Node<'_>) -> bool {
    rooted_member(node, "field_access", |access| {
        field(access, "field", "identifier")?;
        access.child_by_field_name("object")
    })
}

pub(super) fn value(node: Node<'_>) -> bool {
    match node.kind() {
        "decimal_integer_literal"
        | "binary_integer_literal"
        | "octal_integer_literal"
        | "hex_integer_literal"
        | "decimal_floating_point_literal"
        | "hex_floating_point_literal"
        | "character_literal"
        | "string_literal"
        | "true"
        | "false"
        | "null_literal" => true,
        _ => member(node),
    }
}
