//! Execution regions for runtime operands inside object-pattern formal owners.

use super::Region;
use crate::ast::js_ts::props_keys::{definite_key, strip_transparent_wrappers};
use crate::ast::ParsedFile;
use crate::js_write_census::tables::Tables;
use tree_sitter::Node;

pub(super) fn runtime_node(parsed: &ParsedFile, node: Node<'_>) -> bool {
    !node.is_error()
        && !node.is_missing()
        && !parsed.language.is_erased_type_boundary(node)
        && Tables::shared().order_class(node.kind()) != Some("none")
}

pub(super) fn decorated_callable(decorator: Node<'_>) -> Option<Node<'_>> {
    if decorator.kind() != "decorator" {
        return None;
    }
    let parameter = decorator.parent()?;
    let list = parameter.parent()?;
    (matches!(
        parameter.kind(),
        "required_parameter" | "optional_parameter"
    ) && list.kind() == "formal_parameters")
        .then(|| list.parent())
        .flatten()
}

pub(super) fn binding_scope(
    parsed: &ParsedFile,
    owner: Node<'_>,
    operand: Node<'_>,
    name: &str,
) -> Option<usize> {
    let mut ancestor = operand.parent();
    let mut ignored = None;
    while let Some(node) = ancestor.filter(|node| *node != owner) {
        if let Some(callable) = decorated_callable(node) {
            if callable == owner {
                return None;
            }
            ignored = Some(callable);
        }
        if name == "arguments"
            && Some(node) != ignored
            && node.kind() != "arrow_function"
            && parsed
                .language
                .callable_boundary_node_types()
                .contains(&node.kind())
            && ["body", "parameters"].iter().any(|field| {
                node.child_by_field_name(field)
                    .is_some_and(|part| part.byte_range().contains(&operand.start_byte()))
            })
        {
            // Ordinary functions install their own arguments binding. A computed
            // method name and a parameter decorator execute outside that binding.
            return None;
        }
        ancestor = node.parent();
    }
    if let Some(callable) = ignored {
        parsed.js_ts_pattern_decorator_scope(operand, name, callable)
    } else {
        parsed.js_ts_def_scope_id(operand.start_byte(), name)
    }
}

pub(super) fn child_region(
    parsed: &ParsedFile,
    parent: Node<'_>,
    child: Node<'_>,
    region: Region,
) -> Region {
    let callable = parsed
        .language
        .callable_boundary_node_types()
        .contains(&parent.kind());
    let body = in_field(parent, "body", child);
    let callable_region = callable
        && (body || in_field(parent, "parameters", child) || in_field(parent, "parameter", child));
    let instance_field = matches!(
        parent.kind(),
        "field_definition" | "public_field_definition"
    ) && in_field(parent, "value", child)
        && !has_token(parent, "static");
    let deferred = if instance_field {
        !parent
            .parent()
            .and_then(|body| body.parent())
            .is_some_and(directly_constructed)
    } else if callable_region {
        region != Region::Parameter || !invocation_runs_now(parsed, parent, body)
    } else {
        false
    };
    if deferred {
        return match region {
            Region::Parameter | Region::NestedParameter(_) => Region::NestedParameter(parent.id()),
            _ => Region::NestedBody(parent.id()),
        };
    }
    // Only consider suspension on paths actually evaluated before this child. A
    // preceding `if (flag) await ready` does not prove that execution suspended.
    if region == Region::Parameter && follows_suspension(parent, child) {
        return Region::NestedParameter(parent.id());
    }
    region
}

fn invocation_runs_now(parsed: &ParsedFile, callable: Node<'_>, body: bool) -> bool {
    if callable.kind() == "method_definition" {
        return method_runs_now(parsed, callable);
    }
    let expression = wrapped_expression(callable);
    let invocation = called(expression).or_else(|| {
        let member = expression.parent()?;
        let key = member_key(parsed, member, expression)?;
        matches!(key.as_str(), "call" | "apply")
            .then(|| called(wrapped_expression(member)))
            .flatten()
    });
    let Some(invocation) = invocation else {
        return false;
    };
    if body
        && matches!(
            callable.kind(),
            "generator_function" | "generator_function_declaration"
        )
    {
        // Calling a generator initializes its formals. Its body starts only when the
        // iterator is resumed, including a direct `generator().next()` in this default.
        let iterator = wrapped_expression(invocation);
        return iterator.parent().is_some_and(|member| {
            member_key(parsed, member, iterator).as_deref() == Some("next")
                && called(wrapped_expression(member)).is_some()
        });
    }
    true
}

fn method_runs_now(parsed: &ParsedFile, method: Node<'_>) -> bool {
    let Some(container) = method.parent() else {
        return false;
    };
    let name = method
        .child_by_field_name("name")
        .and_then(|key| definite_key(parsed, key));
    let is_getter = has_token(method, "get");
    let object = match container.kind() {
        "class_body" => {
            let Some(class) = container.parent() else {
                return false;
            };
            let constructor = class_construction(class);
            let static_method = has_token(method, "static");
            if !static_method && !is_getter && name.as_deref() == Some("constructor") {
                return constructor.is_some();
            }
            if !is_getter {
                return false;
            }
            if static_method {
                wrapped_expression(class)
            } else {
                let Some(construction) = constructor else {
                    return false;
                };
                wrapped_expression(construction)
            }
        }
        "object" if is_getter => wrapped_expression(container),
        _ => return false,
    };
    object.parent().is_some_and(|access| match access.kind() {
        "member_expression" => member_key(parsed, access, object)
            .zip(name)
            .is_none_or(|(key, name)| key == name),
        "subscript_expression" if in_field(access, "object", object) => access
            .child_by_field_name("index")
            .and_then(|key| definite_key(parsed, key))
            .zip(name)
            .is_none_or(|(key, name)| key == name),
        _ => false,
    })
}

fn directly_constructed(class: Node<'_>) -> bool {
    class_construction(class).is_some()
}

fn class_construction(class: Node<'_>) -> Option<Node<'_>> {
    let expression = wrapped_expression(class);
    expression.parent().filter(|parent| {
        parent.kind() == "new_expression" && in_field(*parent, "constructor", expression)
    })
}

fn called(expression: Node<'_>) -> Option<Node<'_>> {
    let parent = expression.parent()?;
    (matches!(parent.kind(), "call_expression" | "new_expression")
        && (in_field(parent, "function", expression)
            || in_field(parent, "constructor", expression)))
    .then_some(parent)
}

fn wrapped_expression(node: Node<'_>) -> Node<'_> {
    let mut expression = node;
    while let Some(parent) = expression.parent() {
        if strip_transparent_wrappers(parent) != Some(node) {
            break;
        }
        expression = parent;
    }
    expression
}

fn member_key(parsed: &ParsedFile, member: Node<'_>, object: Node<'_>) -> Option<String> {
    if !in_field(member, "object", object) {
        return None;
    }
    let key = match member.kind() {
        "member_expression" => member.child_by_field_name("property")?,
        "subscript_expression" => {
            let key = member.child_by_field_name("index")?;
            return (key.kind() == "string")
                .then(|| definite_key(parsed, key))
                .flatten();
        }
        _ => return None,
    };
    ParsedFile::js_ts_decode_identifier_text(parsed.node_text(&key))
}

fn in_field(parent: Node<'_>, field: &str, child: Node<'_>) -> bool {
    parent.child_by_field_name(field) == Some(child)
}

fn has_token(node: Node<'_>, kind: &str) -> bool {
    let mut cursor = node.walk();
    let found = node.children(&mut cursor).any(|child| child.kind() == kind);
    found
}

fn follows_suspension(parent: Node<'_>, child: Node<'_>) -> bool {
    match parent.kind() {
        "binary_expression" => {
            in_field(parent, "right", child)
                && parent
                    .child_by_field_name("left")
                    .is_some_and(necessarily_suspends)
        }
        "if_statement" | "ternary_expression" => {
            !in_field(parent, "condition", child)
                && parent
                    .child_by_field_name("condition")
                    .is_some_and(necessarily_suspends)
        }
        _ => false,
    }
}

pub(super) fn suspends_following_siblings(parent: Node<'_>, child: Node<'_>) -> bool {
    matches!(
        parent.kind(),
        "statement_block"
            | "sequence_expression"
            | "arguments"
            | "array"
            | "lexical_declaration"
            | "variable_declaration"
    ) && necessarily_suspends(child)
}

fn necessarily_suspends(node: Node<'_>) -> bool {
    let Some(node) = strip_transparent_wrappers(node) else {
        return false;
    };
    match node.kind() {
        "await_expression" | "yield_expression" => true,
        "variable_declarator" => node
            .child_by_field_name("value")
            .is_some_and(necessarily_suspends),
        "binary_expression" => node
            .child_by_field_name("left")
            .is_some_and(necessarily_suspends),
        "if_statement" | "ternary_expression" => node
            .child_by_field_name("condition")
            .is_some_and(necessarily_suspends),
        "expression_statement"
        | "sequence_expression"
        | "lexical_declaration"
        | "variable_declaration" => {
            let mut cursor = node.walk();
            let found = node.named_children(&mut cursor).any(necessarily_suspends);
            found
        }
        _ => false,
    }
}
