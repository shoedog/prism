use super::children;
use prism::ast::ParsedFile;
use std::collections::BTreeMap;
use tree_sitter::Node;

struct Edit {
    star: usize,
    end: usize,
    separate_error: bool,
}

/// Keep byte positions and the enclosing tree while reparsing the expression
/// of a PEP 646 vararg annotation. The star belongs to the annotation's
/// outer AST node, outside the expression that the inventory walks.
pub(in crate::targets::inventory::python) fn recover(parsed: &ParsedFile) -> Option<ParsedFile> {
    if !parsed.source.contains('*') {
        return None;
    }
    let original = parsed.tree.root_node();
    let mut edits = BTreeMap::new();
    collect(original, &mut edits);
    edits.retain(|_, edit| is_starred_expression(&parsed.source[edit.star + 1..edit.end], parsed));
    if edits.is_empty() {
        return None;
    }
    let mut bytes = parsed.source.as_bytes().to_vec();
    for edit in edits.values() {
        bytes[edit.star] = b' ';
    }
    let text = String::from_utf8(bytes).ok()?;
    let recovered = ParsedFile::parse(&parsed.path, &text, parsed.language).ok()?;
    same_outside_annotations(original, recovered.tree.root_node(), &edits).then_some(recovered)
}

fn is_starred_expression(text: &str, parsed: &ParsedFile) -> bool {
    let Ok(fragment) = ParsedFile::parse(&parsed.path, &format!("(\n{text}\n)"), parsed.language)
    else {
        return false;
    };
    let root = fragment.tree.root_node();
    if root.has_error() {
        return false;
    }
    let mut statements = children(root);
    let (Some(statement), None) = (statements.next(), statements.next()) else {
        return false;
    };
    if statement.kind() != "expression_statement" {
        return false;
    }
    let Some(group) = children(statement).next() else {
        return false;
    };
    // In 3.11 the bare expression following this star has bitwise-or
    // precedence. Lower-precedence expressions need their own grouping.
    group.kind() == "parenthesized_expression"
        && children(group).next().is_some_and(|expr| {
            matches!(
                expr.kind(),
                "attribute"
                    | "await"
                    | "binary_operator"
                    | "call"
                    | "concatenated_string"
                    | "dictionary"
                    | "dictionary_comprehension"
                    | "ellipsis"
                    | "false"
                    | "float"
                    | "generator_expression"
                    | "identifier"
                    | "integer"
                    | "list"
                    | "list_comprehension"
                    | "none"
                    | "parenthesized_expression"
                    | "set"
                    | "set_comprehension"
                    | "string"
                    | "subscript"
                    | "true"
                    | "tuple"
                    | "unary_operator"
            )
        })
}

fn collect(node: Node<'_>, edits: &mut BTreeMap<usize, Edit>) {
    if node.kind() == "parameters"
        && node
            .parent()
            .is_some_and(|def| def.kind() == "function_definition")
    {
        for parameter in children(node) {
            if let Some(edit) = annotation_edit(parameter) {
                edits.insert(parameter.start_byte(), edit);
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect(child, edits);
    }
}

fn annotation_edit(parameter: Node<'_>) -> Option<Edit> {
    match parameter.kind() {
        "typed_parameter" => Some(Edit {
            star: annotation_star(parameter)?.start_byte(),
            end: parameter.end_byte(),
            separate_error: false,
        }),
        "list_splat_pattern" if is_vararg(parameter) => {
            // A grouped expression or display can recover as a separate
            // ERROR immediately after the vararg rather than inside it.
            let error = parameter.next_sibling()?;
            if error.kind() != "ERROR" || error.child_count() != 3 {
                return None;
            }
            let (colon, star, expression) = (error.child(0)?, error.child(1)?, error.child(2)?);
            (colon.kind() == ":"
                && star.kind() == "*"
                && expression.is_named()
                && !expression.has_error())
            .then_some(Edit {
                star: star.start_byte(),
                end: error.end_byte(),
                separate_error: true,
            })
        }
        _ => None,
    }
}

fn is_vararg(node: Node<'_>) -> bool {
    node.kind() == "list_splat_pattern"
        && !node.has_error()
        && children(node)
            .next()
            .is_some_and(|name| name.kind() == "identifier")
}

fn annotation_star(parameter: Node<'_>) -> Option<Node<'_>> {
    let items: Vec<_> = children(parameter).collect();
    let [target, rest @ ..] = items.as_slice() else {
        return None;
    };
    if !is_vararg(*target) {
        return None;
    }
    let annotation = parameter.child_by_field_name("type")?;
    if annotation.has_error() {
        return None;
    }
    // Follow only type wrappers and the left edge of primary expressions.
    // An operand inside an argument list or subscription remains untouched.
    let splat = match rest {
        [error, tail] if error.kind() == "ERROR" && *tail == annotation => {
            let [head] = children(*error).collect::<Vec<_>>().try_into().ok()?;
            leading_splat(head)?
        }
        [only] if *only == annotation => leading_splat(*only)?,
        _ => return None,
    };
    let star = splat.child(0)?;
    let colon = target.next_sibling()?;
    (star.kind() == "*" && colon.kind() == ":" && star.start_byte() == rest.first()?.start_byte())
        .then_some(star)
}

fn leading_splat(mut node: Node<'_>) -> Option<Node<'_>> {
    loop {
        node = match node.kind() {
            "splat_type" | "list_splat" => return Some(node),
            "type" | "member_type" | "generic_type" => children(node).next()?,
            "call" => node.child_by_field_name("function")?,
            "subscript" => node.child_by_field_name("value")?,
            "attribute" => node.child_by_field_name("object")?,
            _ => return None,
        };
    }
}

fn repaired_parameter(before: Node<'_>, after: Node<'_>, edit: &Edit) -> bool {
    let target = if edit.separate_error {
        Some(before)
    } else {
        children(before).next()
    };
    after.kind() == "typed_parameter"
        && !after.has_error()
        && after.start_byte() == before.start_byte()
        && after.end_byte() == edit.end
        && target
            .zip(children(after).next())
            .is_some_and(|(old, new)| {
                old.kind() == new.kind() && old.byte_range() == new.byte_range()
            })
        && after
            .child_by_field_name("type")
            .is_some_and(|value| value.start_byte() > edit.star && value.end_byte() == edit.end)
}

fn same_outside_annotations(
    original: Node<'_>,
    recovered: Node<'_>,
    edits: &BTreeMap<usize, Edit>,
) -> bool {
    if original.kind() != recovered.kind()
        || original.byte_range() != recovered.byte_range()
        || original.is_missing() != recovered.is_missing()
    {
        return false;
    }
    let mut left = original.walk();
    let mut right = recovered.walk();
    let mut before = original.children(&mut left);
    for after in recovered.children(&mut right) {
        let Some(old) = before.next() else {
            return false;
        };
        if let Some(edit) = edits.get(&old.start_byte()).filter(|_| {
            original.kind() == "parameters"
                && matches!(old.kind(), "list_splat_pattern" | "typed_parameter")
        }) {
            if !repaired_parameter(old, after, edit) {
                return false;
            }
            if edit.separate_error {
                before.next();
            }
        } else if !same_outside_annotations(old, after, edits) {
            return false;
        }
    }
    before.next().is_none()
}
