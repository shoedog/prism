use super::*;
use crate::call_graph::{CallKind, CallSiteOrigin};

pub(super) fn execution_owner<'tree>(
    parsed: &ParsedFile,
    mut node: Node<'tree>,
) -> Option<Node<'tree>> {
    while let Some(parent) = node.parent() {
        if parsed
            .language
            .callable_boundary_node_types()
            .contains(&parent.kind())
        {
            // A computed method name executes outside the method.
            if !parent
                .child_by_field_name("name")
                .is_some_and(|name| name == node)
            {
                return Some(parent);
            }
        }
        node = parent;
    }
    (node.kind() == "program").then_some(node)
}

pub(super) fn call<'a>(
    site: &CallSite,
    caller: &FunctionId,
    parsed: &'a ParsedFile,
) -> Result<(Node<'a>, Node<'a>), &'static str> {
    if !parsed.language.is_js_family() {
        return Err("other_language");
    }
    if site.kind != CallKind::Call || site.origin != CallSiteOrigin::Source || site.jsx_element {
        return Err("not_source_call");
    }
    if &site.caller != caller || parsed.path != caller.file {
        return Err("caller_identity");
    }
    let call = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(site.start_byte, site.end_byte)
        .filter(|node| node.byte_range() == (site.start_byte..site.end_byte))
        .ok_or("call_span")?;
    if call.kind() != "call_expression" || call.has_error() {
        return Err("not_call_expression");
    }
    let owner = execution_owner(parsed, call)
        .and_then(|node| crate::anon_owner::identity(parsed, &node, &parsed.path));
    if owner.as_ref() != Some(caller) || site.line != call.start_position().row + 1 {
        return Err("caller_identity");
    }
    if !parsed
        .call_function_name(&call)
        .is_some_and(|name| parsed.node_text(&name) == site.callee_name)
    {
        return Err("callee_identity");
    }
    let arguments = call
        .child_by_field_name("arguments")
        .filter(|node| node.kind() == "arguments")
        .ok_or("not_argument_list")?;
    Ok((call, arguments))
}

pub(super) fn unwrap<'a>(parsed: &ParsedFile, mut value: Node<'a>) -> Option<Node<'a>> {
    loop {
        value = parsed.js_ts_unwrap_value(value)?;
        if value.kind() != "instantiation_expression" {
            return Some(value);
        }
        value = children(value).next()?;
    }
}

fn children(node: Node<'_>) -> impl Iterator<Item = Node<'_>> {
    let mut child = node.named_child(0);
    std::iter::from_fn(move || {
        while let Some(current) = child {
            child = current.next_named_sibling();
            if !current.is_extra() {
                return Some(current);
            }
        }
        None
    })
}

pub(super) fn member<'a>(
    parsed: &ParsedFile,
    node: Node<'a>,
) -> Result<(Node<'a>, String), &'static str> {
    let (field, kind) = match node.kind() {
        "member_expression" => ("property", "property_identifier"),
        "subscript_expression" => ("index", "string"),
        _ => return Err("detached_callee"),
    };
    let key = node
        .child_by_field_name(field)
        .filter(|node| node.kind() == kind)
        .ok_or("non_public_static_key")?;
    let name = if kind == "string" {
        parsed.js_ts_module_export_name(key)
    } else {
        ParsedFile::js_ts_decoded_identifier(parsed.node_text(&key))
    }
    .ok_or("non_public_static_key")?;
    let receiver = node
        .child_by_field_name("object")
        .ok_or("receiver_syntax")?;
    Ok((receiver, name))
}

pub(super) fn receiver_path(parsed: &ParsedFile, mut node: Node<'_>) -> Option<AccessPath> {
    let mut fields = Vec::new();
    loop {
        node = unwrap(parsed, node)?;
        match node.kind() {
            "identifier" | "this" => {
                fields.reverse();
                // The graph's path key uses the base token's source spelling.
                return Some(AccessPath::with_fields(parsed.node_text(&node), fields));
            }
            _ => {
                let (object, key) = member(parsed, node).ok()?;
                fields.push(key);
                node = object;
            }
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum Seed {
    Omitted,
    Supplied,
    Unknown,
}

pub(super) fn runtime_args<'a>(
    parsed: &'a ParsedFile,
    arguments: Node<'a>,
) -> (Result<Node<'a>, &'static str>, Seed) {
    let mut first = Err("missing_slot_zero");
    let mut least = 0;
    let mut unknown = false;
    for argument in children(arguments) {
        if argument.kind() == "spread_element" {
            let operand = parsed.spread_argument_operand(&argument.byte_range());
            if let Some((array, length)) = operand
                .and_then(|node| spread_args::dense_array_length(node).map(|length| (node, length)))
            {
                if !unknown && least == 0 && length != 0 {
                    first = children(array).next().ok_or("missing_slot_zero");
                }
                least += length;
            } else {
                if least == 0 {
                    first = Err("unknown_slot_zero");
                }
                unknown = true;
            }
        } else {
            if !unknown && least == 0 {
                first = Ok(argument);
            }
            least += 1;
        }
        if least >= 2 {
            return (first, Seed::Supplied);
        }
    }
    (
        first,
        if unknown {
            Seed::Unknown
        } else {
            Seed::Omitted
        },
    )
}
