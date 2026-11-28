use super::*;

fn intrinsic_reflect_set<'a>(
    parsed: &'a ParsedFile,
    refs: &References<'a>,
    callee: Node<'a>,
    cache: &mut JsBindingCache<'a>,
) -> bool {
    if parsed.node_text(&callee) != "Reflect.set" {
        return false;
    }
    let Some(object) = callee.child_by_field_name("object") else {
        return false;
    };
    matches!(
        parsed.js_ts_binding_walk(object, "Reflect", cache),
        param_slot::Walk::Unbound
    ) && refs
        .in_scope("Reflect", parsed.tree.root_node())
        .iter()
        .copied()
        .all(|use_site| {
            if use_site == object
                || parsed.language.is_in_erased_type_context(use_site)
                || matches!(parsed.js_ts_binding_walk(use_site, "Reflect", cache),
                param_slot::Walk::Found(scope, _) if scope.kind() != "with_statement")
            {
                return true;
            }
            let member = use_site
                .parent()
                .filter(|member| parsed.node_text(member) == "Reflect.set");
            member.is_some_and(|member| {
                use_site == member.child_by_field_name("object").unwrap_or(member)
                    && member.parent().is_some_and(|call| {
                        call.kind() == "call_expression"
                            && call.child_by_field_name("function") == Some(member)
                    })
            })
        })
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

/// An entry value may be observed directly, or through a body destructure.
///
/// A proved straight-line deletion severs the map entry. Other uses must
/// preserve the live slot and, for a destructure, the object behind it.
pub(in super::super) fn arguments_instability<'a>(
    parsed: &'a ParsedFile,
    bindings: &FileBindings<'a>,
    owner: Node<'a>,
    position: usize,
    body_path: Option<(Node<'a>, &[Selection<'a>])>,
    plain_access: &DisjointProof<'a, '_>,
    cache: &mut JsBindingCache<'a>,
) -> Option<&'static str> {
    let source = &parsed.source[owner.byte_range()];
    if owner.kind() == "arrow_function" || (!source.contains("arguments") && !source.contains('\\'))
    {
        return None;
    }
    let mapped = !parsed.js_ts_is_module()
        && !parsed.js_ts_callable_is_strict(&owner)
        && owner
            .child_by_field_name("parameters")
            .is_some_and(|list| parsed.js_ts_parameters_are_simple(list));
    let reason = if mapped {
        "mapped_arguments"
    } else {
        body_path?;
        "stale_path"
    };
    let refs = bindings.references(parsed);
    let key = position.to_string();
    let mut selected = vec![Some(key.clone())];
    let before = body_path.map(|(read, path)| {
        selected.extend(selected_keys(path));
        Before::new(parsed, read)
    });
    let disjoint = |usage: &PathUse<'a>, _| usage.disjoint == Some(1) || plain_access(usage, 1);
    let mut detached_at = None;
    for &occurrence in refs.in_scope("arguments", owner) {
        if before
            .as_ref()
            .is_some_and(|before| !before.may_include(parsed, occurrence))
            || detached_at.is_some_and(|at| {
                at <= occurrence.start_byte()
                    && execution_owner(parsed, occurrence) == Some(owner.id())
            })
            || parsed.language.is_in_erased_type_context(occurrence)
        {
            continue;
        }
        if !param_slot::arguments_binding(parsed, occurrence, owner, cache) {
            match parsed.js_ts_binding_walk(occurrence, "arguments", cache) {
                param_slot::Walk::Unchecked(_) => return Some(reason),
                param_slot::Walk::Found(scope, _) if scope.kind() == "with_statement" => {
                    return Some(reason);
                }
                _ => {}
            }
            continue;
        }
        let usage = PathUse::new(parsed, occurrence, &selected);
        if usage.depth == 1 && usage.disjoint.is_none() && !usage.prototype_access {
            if let Some(operation) =
                write_operation(parsed, usage.node).filter(|node| node.kind() == "unary_expression")
            {
                if member_key(parsed, param_slot::peel(usage.node).unwrap()).as_deref()
                    == Some(&key)
                    && operation
                        .parent()
                        .filter(|statement| statement.kind() == "expression_statement")
                        .and_then(|statement| statement.parent())
                        == owner.child_by_field_name("body")
                {
                    detached_at = Some(operation.end_byte());
                }
                // Deleting the map entry alone does not replace the formal.
                continue;
            }
        }
        if usage.depth == 0
            && (matches!(
                usage.node.parent(),
                Some(parent) if harmless_reflection(parsed, refs, parent, usage.node, &key, cache)
            ) || copied_arguments_preserve(
                parsed,
                refs,
                &usage,
                &selected,
                (before.as_ref(), &disjoint),
                cache,
            ))
        {
            continue;
        }
        // Unmapped arguments still hold a reference to the supplied object,
        // just like a shallow copy of the live map.
        if !usage.preserves(parsed, usize::from(!mapped), &disjoint) {
            return Some(reason);
        }
    }
    None
}

fn harmless_reflection<'a>(
    parsed: &'a ParsedFile,
    refs: &References<'a>,
    arguments: Node<'a>,
    node: Node<'a>,
    position: &str,
    cache: &mut JsBindingCache<'a>,
) -> bool {
    if arguments.kind() != "arguments" {
        return false;
    }
    if !arguments
        .parent()
        .and_then(|call| call.child_by_field_name("function"))
        .and_then(param_slot::peel)
        .is_some_and(|callee| intrinsic_reflect_set(parsed, refs, callee, cache))
    {
        return false;
    };
    let mut arguments = param_slot::children(arguments);
    if arguments.next() != Some(node) {
        return false;
    }
    let selected_key = arguments
        .next()
        .and_then(param_slot::peel)
        .filter(|key| matches!(key.kind(), "string" | "number"))
        .and_then(|key| param_slot::property_key(parsed, key))
        .is_some_and(|key| key == position);
    // A reflective update to another key can install an accessor that later
    // receives the actual map as its receiver. Only preserve a proved other
    // receiver for a set against the live slot.
    selected_key
        && arguments
            .nth(1)
            .is_some_and(|receiver| distinct_receiver(parsed, receiver, cache))
}

fn copied_arguments_preserve<'a>(
    parsed: &'a ParsedFile,
    refs: &References<'a>,
    usage: &PathUse<'a>,
    selected: &[Option<String>],
    (before, plain_access): (Option<&Before>, &DisjointProof<'a, '_>),
    cache: &mut JsBindingCache<'a>,
) -> bool {
    let Some(spread) = usage
        .node
        .parent()
        .filter(|parent| parent.kind() == "spread_element")
    else {
        return false;
    };
    let Some(array) = spread.parent().filter(|parent| parent.kind() == "array") else {
        return !(selected.len() > 1)
            && spread
                .parent()
                .is_some_and(|parent| matches!(parent.kind(), "object" | "arguments"));
    };
    let Some(before) = before else {
        return true; // Shallow copies cannot rebind a scalar formal.
    };
    let array = unwrap_use(array);
    // A dense one-spread array preserves the slot numbers and copies only the
    // array itself. Prove that its uses cannot mutate a shared element.
    if array.kind() != "array"
        || !param_slot::array_entries(array)
            .into_iter()
            .eq([Some(spread)])
    {
        return false;
    }
    let Some(declaration) = array.parent().filter(|parent| {
        param_slot::is_const(*parent) && parent.child_by_field_name("value") == Some(array)
    }) else {
        return false;
    };
    let Some(binding) = declaration.child_by_field_name("name") else {
        return false;
    };
    let Some(name) = param_slot::name(parsed, binding) else {
        return false;
    };
    let Some(scope) = declaration.parent().and_then(|list| list.parent()) else {
        return false;
    };
    refs.in_scope(&name, scope).iter().copied().all(|occurrence| {
        if occurrence == binding || !before.may_include(parsed, occurrence)
            || parsed.language.is_in_erased_type_context(occurrence) {
            return true;
        }
        match parsed.js_ts_site_declaration(occurrence, &name, cache) {
            Some((_, found)) if found == declaration => {
                PathUse::new(parsed, occurrence, selected).preserves(parsed, 1, plain_access)
            }
            _ => matches!(parsed.js_ts_binding_walk(occurrence, &name, cache),
                param_slot::Walk::Found(found, _) if found != scope && found.kind() != "with_statement"),
        }
    })
}
