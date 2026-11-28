use super::*;
use crate::framework_entries::routes::{
    python::{first_iterable, walk},
    Invocation,
};
use std::collections::BTreeMap;

/// These wrappers evaluate their children in the surrounding iterable expression.
pub(super) fn first_iterable_call(mut node: Node<'_>) -> bool {
    while let Some(parent) = node.parent() {
        match parent.kind() {
            "parenthesized_expression"
            | "tuple"
            | "list"
            | "set"
            | "dictionary"
            | "pair"
            | "list_splat"
            | "dictionary_splat" => node = parent,
            "for_in_clause" => {
                return first_iterable(parent)
                    && parent
                        .children_by_field_name("right", &mut parent.walk())
                        .any(|iterable| iterable == node);
            }
            _ => return false,
        }
    }
    false
}

/// An unambiguous direct setup call supplies the enclosing module's lookup cutoff.
pub(super) fn setup_invocations<'a>(
    bindings: &PythonBindings<'a>,
    file: &'a ParsedFile,
) -> BTreeMap<usize, SetupContext<'a>> {
    let module = file.tree.root_node();
    let mut calls = BTreeMap::new();
    for &call in bindings.nodes(&file.path) {
        if call.kind() != "call"
            || !call
                .parent()
                .is_some_and(|p| p.kind() == "expression_statement" && p.parent() == Some(module))
            || !arguments(call).nodes().is_empty()
        {
            continue;
        }
        if let Some(node) = setup_function(bindings, file, call, module) {
            let frame = SetupContext {
                factory: Some(PythonMember { file, node }),
                frames: vec![Invocation::Call {
                    file: file.path.clone(),
                    start_byte: call.start_byte(),
                    end_byte: call.end_byte(),
                }],
            };
            calls
                .entry(node.id())
                .and_modify(|entry| *entry = None)
                .or_insert(Some(frame));
        }
    }
    calls
        .into_iter()
        .filter_map(|(id, context)| Some((id, context?)))
        .collect()
}

fn setup_function<'a>(
    bindings: &PythonBindings<'a>,
    file: &ParsedFile,
    call: Node<'a>,
    module: Node<'a>,
) -> Option<Node<'a>> {
    let callee = ungroup(call.child_by_field_name("function")?);
    let (scope, name, method) = if let Some((object, method)) = attribute(callee) {
        let object = ungroup(object);
        if object.kind() != "call" || !arguments(object).nodes().is_empty() {
            return None;
        }
        let class_name = ungroup(object.child_by_field_name("function")?);
        if class_name.kind() != "identifier" {
            return None;
        }
        let class =
            bindings.reaching(file, module, file.node_text(&class_name), call.start_byte())?;
        if class.kind() != "class_definition"
            || class.parent() != Some(module)
            || class.child_by_field_name("superclasses").is_some()
            || !children(class.child_by_field_name("body")?)
                .into_iter()
                .all(|member| {
                    matches!(
                        member.kind(),
                        "comment" | "pass_statement" | "function_definition"
                    )
                })
            || ["__init__", "__new__", "__getattribute__", "__getattr__"]
                .iter()
                .any(|name| {
                    bindings
                        .reaching(file, class, name, class.end_byte())
                        .is_some()
                })
        {
            return None;
        }
        (class, method, true)
    } else {
        (module, callee, false)
    };
    if name.kind() != "identifier" {
        return None;
    }
    let function = bindings.reaching(
        file,
        scope,
        file.node_text(&name),
        if method {
            scope.end_byte()
        } else {
            call.start_byte()
        },
    )?;
    if function.kind() != "function_definition"
        || function.parent() != scope.child_by_field_name("body").or(Some(scope))
        || function.child(0)?.kind() == "async"
        || walk(function.child_by_field_name("body")?)
            .into_iter()
            .any(|node| node.kind() == "yield" && python_scope(node) == function)
    {
        return None;
    }
    let parameters = children(function.child_by_field_name("parameters")?);
    match (method, parameters.as_slice()) {
        (false, []) => Some(function),
        (true, [receiver]) if receiver.kind() == "identifier" => Some(function),
        _ => None,
    }
}
