//! Occurrence-bound writes to the live arguments map and to selected paths.
use super::*;

fn descendants(root: Node<'_>) -> impl Iterator<Item = Node<'_>> {
    let mut pending = vec![root];
    std::iter::from_fn(move || {
        let node = pending.pop()?;
        pending.extend(param_slot::children(node).rev());
        Some(node)
    })
}

fn member_key(parsed: &ParsedFile, member: Node<'_>) -> Option<String> {
    if let Some(property) = member.child_by_field_name("property") {
        return param_slot::property_key(parsed, property);
    }
    let index = member
        .child_by_field_name("index")
        .and_then(param_slot::peel)?;
    matches!(index.kind(), "number" | "string")
        .then(|| param_slot::property_key(parsed, index))
        .flatten()
}

fn member(node: Node<'_>) -> bool {
    matches!(node.kind(), "member_expression" | "subscript_expression")
}

fn assignment_targets(target: Node<'_>) -> Vec<Node<'_>> {
    let mut pending = vec![target];
    let mut leaves = Vec::new();
    while let Some(target) = pending.pop().and_then(param_slot::peel) {
        match target.kind() {
            "array_pattern" | "object_pattern" | "rest_pattern" => {
                pending.extend(param_slot::children(target))
            }
            "pair_pattern" => pending.extend(target.child_by_field_name("value")),
            "assignment_pattern" | "object_assignment_pattern" => {
                pending.extend(target.child_by_field_name("left"))
            }
            _ => leaves.push(target),
        }
    }
    leaves
}

fn execution_owner(parsed: &ParsedFile, mut node: Node<'_>) -> Option<usize> {
    let boundaries = parsed.language.callable_boundary_node_types();
    while let Some(parent) = node.parent() {
        if boundaries.contains(&parent.kind())
            || matches!(parent.kind(), "class_static_block" | "program")
        {
            return Some(parent.id());
        }
        node = parent;
    }
    None
}

fn delete(node: Node<'_>) -> bool {
    node.kind() == "unary_expression"
        && node
            .child_by_field_name("operator")
            .is_some_and(|operator| operator.kind() == "delete")
}

fn direct_body_statement(node: Node<'_>, owner: Node<'_>) -> bool {
    node.parent()
        .filter(|statement| statement.kind() == "expression_statement")
        .and_then(|statement| statement.parent())
        == owner.child_by_field_name("body")
}

pub(super) fn mapped_write<'a>(
    parsed: &'a ParsedFile,
    owner: Node<'a>,
    position: usize,
    cache: &mut JsBindingCache<'a>,
) -> bool {
    if owner.kind() == "arrow_function"
        || parsed.js_ts_is_module()
        || parsed.js_ts_callable_is_strict(&owner)
        || !owner
            .child_by_field_name("parameters")
            .is_some_and(|list| parsed.js_ts_parameters_are_simple(list))
    {
        return false;
    }
    if !parsed.source[owner.byte_range()].contains("arguments")
        && !parsed.source[owner.byte_range()].contains('\\')
    {
        return false;
    }
    let key = position.to_string();
    let mut detached_at = None;
    let at_live_index =
        |member_node: Node<'a>| member_key(parsed, member_node).is_none_or(|member| member == key);
    for operation in descendants(owner) {
        let detached = detached_at.is_some_and(|at| at <= operation.start_byte())
            && execution_owner(parsed, operation) == Some(owner.id());
        if let Some(target) = param_slot::write_target(parsed, operation) {
            for target in assignment_targets(target)
                .into_iter()
                .filter(|target| member(*target))
            {
                let Some(object) = target
                    .child_by_field_name("object")
                    .and_then(param_slot::peel)
                else {
                    continue;
                };
                if param_slot::name(parsed, object).as_deref() != Some("arguments")
                    || !param_slot::arguments_binding(parsed, object, owner, cache)
                    || !at_live_index(target)
                {
                    continue;
                }
                if delete(operation) {
                    if direct_body_statement(operation, owner)
                        && member_key(parsed, target).as_deref() == Some(key.as_str())
                    {
                        detached_at = Some(operation.end_byte());
                    }
                } else if !detached {
                    return true;
                }
            }
        }
        if detached
            || param_slot::name(parsed, operation).as_deref() != Some("arguments")
            || parsed.js_ts_identifier_is_not_a_read(operation)
            || !param_slot::arguments_binding(parsed, operation, owner, cache)
        {
            continue;
        }
        if escaped_map(parsed, operation, &key, cache) {
            return true;
        }
    }
    false
}

fn escaped_map<'a>(
    parsed: &'a ParsedFile,
    mut node: Node<'a>,
    position: &str,
    cache: &mut JsBindingCache<'a>,
) -> bool {
    while let Some(parent) = node.parent() {
        if param_slot::peel(parent) == Some(node) {
            node = parent;
            continue;
        }
        if member(parent) && parent.child_by_field_name("object") == Some(node) {
            return false; // The value of an index is not the arguments object.
        }
        if parent.kind() == "spread_element"
            && parent.parent().is_some_and(|container| {
                matches!(container.kind(), "array" | "object" | "arguments")
            })
        {
            return false;
        }
        if parent.kind() == "arguments" {
            let Some(call) = parent.parent() else {
                return true;
            };
            let Some(callee) = call
                .child_by_field_name("function")
                .and_then(param_slot::peel)
            else {
                return true;
            };
            let arguments: Vec<_> = param_slot::children(parent).collect();
            if arguments.first() != Some(&node) || !intrinsic_reflective(parsed, callee, cache) {
                return true;
            }
            let matches_position = arguments
                .get(1)
                .and_then(|argument| param_slot::peel(*argument))
                .filter(|key| matches!(key.kind(), "string" | "number"))
                .and_then(|key| param_slot::property_key(parsed, key))
                .is_none_or(|key| key == position);
            if !matches_position {
                return false;
            }
            if parsed.node_text(&callee) == "Reflect.set" {
                return arguments
                    .get(3)
                    .is_none_or(|receiver| !distinct_receiver(parsed, *receiver, cache));
            }
            return true;
        }
        return !matches!(
            parent.kind(),
            "expression_statement" | "unary_expression" | "binary_expression"
        );
    }
    true
}

fn intrinsic_reflective<'a>(
    parsed: &'a ParsedFile,
    callee: Node<'a>,
    cache: &mut JsBindingCache<'a>,
) -> bool {
    if !matches!(
        parsed.node_text(&callee),
        "Reflect.set" | "Reflect.defineProperty" | "Object.defineProperty"
    ) {
        return false;
    }
    let Some(object) = callee.child_by_field_name("object") else {
        return false;
    };
    matches!(
        parsed.js_ts_binding_walk(object, parsed.node_text(&object), cache),
        param_slot::Walk::Unbound
    )
}

fn distinct_receiver<'a>(
    parsed: &'a ParsedFile,
    receiver: Node<'a>,
    cache: &mut JsBindingCache<'a>,
) -> bool {
    let Some(receiver) = param_slot::peel(receiver) else {
        return false;
    };
    if matches!(receiver.kind(), "object" | "array") {
        return true;
    }
    let Some(name) = param_slot::name(parsed, receiver) else {
        return false;
    };
    let Some((scope, declaration)) = parsed.js_ts_site_declaration(receiver, &name, cache) else {
        return false;
    };
    param_slot::is_const(declaration)
        && identity::initialized_at(parsed, declaration, receiver)
        && !parsed.js_ts_scoped_written(scope, &name, cache)
        && declaration
            .child_by_field_name("value")
            .and_then(param_slot::peel)
            .is_some_and(|value| matches!(value.kind(), "object" | "array"))
}

pub(super) fn selected_path_written<'a>(
    parsed: &'a ParsedFile,
    owner: Node<'a>,
    read: Node<'a>,
    selected: &[Selection<'a>],
    cache: &mut JsBindingCache<'a>,
) -> bool {
    let Some(name) = param_slot::name(parsed, read) else {
        return true;
    };
    let param_slot::Walk::Found(scope, None) = parsed.js_ts_binding_walk(read, &name, cache) else {
        return true;
    };
    let selected: Vec<_> = selected_keys(selected).collect();
    let owner_id = execution_owner(parsed, read);
    descendants(owner)
        .filter(|node| {
            node.start_byte() < read.start_byte() || execution_owner(parsed, *node) != owner_id
        })
        .any(|node| touches_path(parsed, node, &name, scope, &selected, cache))
}

/// Entry destructuring reads an aliased prototype only after argument evaluation.
pub(super) fn later_argument_writes<'a>(
    parsed: &'a ParsedFile,
    call: Node<'a>,
    argument_byte: usize,
    reference: Node<'a>,
    key: &str,
    remaining: &[Selection<'a>],
    cache: &mut JsBindingCache<'a>,
) -> bool {
    let Some(reference) = param_slot::peel(reference) else {
        return false;
    };
    let Some(name) = param_slot::name(parsed, reference) else {
        return false;
    };
    let Some((scope, _)) = parsed.js_ts_site_declaration(reference, &name, cache) else {
        return false;
    };
    let Some(list) = call.child_by_field_name("arguments") else {
        return false;
    };
    let selected: Vec<_> = std::iter::once(Some(key.to_owned()))
        .chain(selected_keys(remaining))
        .collect();
    let owner_id = execution_owner(parsed, call);
    let later = param_slot::children(list).flat_map(|argument| {
        if argument.kind() == "spread_element" && argument.byte_range().contains(&argument_byte) {
            return parsed
                .spread_argument_operand(&argument.byte_range())
                .into_iter()
                .flat_map(param_slot::children)
                .collect::<Vec<_>>();
        }
        vec![argument]
    });
    later
        .filter(|argument| argument.start_byte() > argument_byte)
        .flat_map(descendants)
        .filter(|node| execution_owner(parsed, *node) == owner_id)
        .any(|node| touches_path(parsed, node, &name, scope, &selected, cache))
}

fn selected_keys<'a>(selected: &'a [Selection<'_>]) -> impl Iterator<Item = Option<String>> + 'a {
    selected.iter().filter_map(|step| match step {
        Selection::Object(key) => Some(Some(key.clone())),
        Selection::Array(index) => Some(Some(index.to_string())),
        Selection::Computed(_) | Selection::Rest => Some(None),
        Selection::Default(_) => None,
    })
}

fn touches_path<'a>(
    parsed: &'a ParsedFile,
    node: Node<'a>,
    name: &str,
    scope: Node<'a>,
    selected: &[Option<String>],
    cache: &mut JsBindingCache<'a>,
) -> bool {
    let mut path_of =
        |occurrence| rooted_path(parsed, occurrence, name, scope, cache, &mut BTreeSet::new());
    if let Some(target) = param_slot::write_target(parsed, node) {
        if assignment_targets(target)
            .into_iter()
            .filter_map(&mut path_of)
            .any(|path| is_prefix(&path, selected))
        {
            return true;
        }
    }
    node.kind() == "call_expression"
        && node
            .child_by_field_name("arguments")
            .is_some_and(|arguments| {
                param_slot::children(arguments)
                    .filter_map(path_of)
                    .any(|path| path.len() < selected.len() && is_prefix(&path, selected))
            })
}

fn is_prefix(write: &[Option<String>], read: &[Option<String>]) -> bool {
    write.len() <= read.len()
        && write
            .iter()
            .zip(read)
            .all(|(write, read)| write.is_none() || read.is_none() || write == read)
}

fn rooted_path<'a>(
    parsed: &'a ParsedFile,
    occurrence: Node<'a>,
    root: &str,
    scope: Node<'a>,
    cache: &mut JsBindingCache<'a>,
    visited: &mut BTreeSet<usize>,
) -> Option<Vec<Option<String>>> {
    let occurrence = param_slot::peel(occurrence)?;
    if member(occurrence) {
        let mut path = rooted_path(
            parsed,
            occurrence.child_by_field_name("object")?,
            root,
            scope,
            cache,
            visited,
        )?;
        path.push(member_key(parsed, occurrence));
        return Some(path);
    }
    let name = param_slot::name(parsed, occurrence)?;
    if name == root
        && matches!(parsed.js_ts_binding_walk(occurrence, &name, cache), param_slot::Walk::Found(found, None) if found == scope)
    {
        return Some(Vec::new());
    }
    let (_, decl) = parsed.js_ts_site_declaration(occurrence, &name, cache)?;
    if !param_slot::is_const(decl)
        || !identity::initialized_at(parsed, decl, occurrence)
        || !visited.insert(decl.id())
    {
        return None;
    }
    rooted_path(
        parsed,
        decl.child_by_field_name("value")?,
        root,
        scope,
        cache,
        visited,
    )
}
