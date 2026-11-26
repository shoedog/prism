use super::operation::Form;
use super::*;

pub(in super::super) fn certify_callee(
    parsed: &ParsedFile,
    operation: Node<'_>,
    shape: &OperationShape<'_>,
    target: Node<'_>,
) -> Result<(), Reason> {
    if !same_node(operation, shape.operation)
        || operation.into_raw().tree != parsed.tree.root_node().into_raw().tree
    {
        return Err(Reason::CalleeKindUnlisted);
    }
    if let Some(path) = call_path(parsed, shape)? {
        if !path.matches(target) {
            return Err(Reason::CalleeTargetUnlisted);
        }
    }
    Ok(())
}

#[cfg(test)]
pub(in super::super) fn target_for_hit<'tree>(
    parsed: &ParsedFile,
    operation: Node<'tree>,
    shape: &OperationShape<'tree>,
    id: Node<'tree>,
) -> Result<Node<'tree>, Reason> {
    match certify_callee(parsed, operation, shape, id) {
        Ok(()) => Ok(id),
        Err(reason) => {
            if let Some((path, parent)) = call_path(parsed, shape).ok().flatten().zip(id.parent()) {
                if same_node(id, path.terminal) && same_node(parent, path.whole) {
                    certify_callee(parsed, operation, shape, parent)?;
                    return Ok(parent);
                }
            }
            Err(reason)
        }
    }
}

pub(super) fn call_path<'tree>(
    parsed: &ParsedFile,
    shape: &OperationShape<'tree>,
) -> Result<Option<Path<'tree>>, Reason> {
    use Language::*;
    let head = match shape.form {
        Form::Assignment | Form::HclAttribute | Form::JsxAttribute => return Ok(None),
        Form::Constructor if parsed.language == Cpp => {
            return c_family::constructor(shape.head).map(Some);
        }
        Form::Constructor if parsed.language == Java => {
            return java::constructor(shape.head).map(Some);
        }
        _ => unwrap_head(parsed.language, shape.head).ok_or(Reason::CalleeKindUnlisted)?,
    };
    let path = match parsed.language {
        Python => python::callee(parsed, head),
        JavaScript | TypeScript | Tsx => js::callee(head, shape.form == Form::Call),
        Go => go::callee(head),
        Java => java::method(shape.operation, head),
        C | Cpp => c_family::callee(parsed.language, head),
        Rust => rust::callee(head),
        Lua => lua::callee(head),
        Bash => bash::callee(parsed, head),
        Terraform if head.kind() == "identifier" => Ok(Path::plain(head)),
        Terraform => Err(Reason::CalleeKindUnlisted),
    }?;
    Ok(Some(path))
}

pub(super) fn unwrap_head(language: Language, mut head: Node<'_>) -> Option<Node<'_>> {
    loop {
        head = match (language, head.kind()) {
            (
                Language::Python
                | Language::JavaScript
                | Language::TypeScript
                | Language::Tsx
                | Language::Go
                | Language::C
                | Language::Cpp
                | Language::Rust
                | Language::Lua,
                "parenthesized_expression",
            ) => unwrapped(head, "(", ")")?,
            (Language::TypeScript | Language::Tsx, "non_null_expression") => {
                js::non_null_operand(head)?
            }
            _ => return Some(head),
        };
    }
}
