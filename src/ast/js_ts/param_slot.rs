//! Declaration-specific parameter selection at a JS-family value occurrence.

use super::pattern_params::formal_binding;
use super::props_keys::{definite_key, strip_transparent_wrappers};
pub(crate) use crate::ast::js_binding_walk::Walk;
use crate::ast::{is_js_ts_function_like, JsBindingCache, ParsedFile};
use tree_sitter::Node;

#[derive(Clone, Debug)]
pub(crate) enum Selection<'a> {
    Object(String),
    Array(usize),
    Computed(Node<'a>),
    Default(Node<'a>),
    Rest,
}

#[derive(Clone, Debug)]
pub(crate) struct Parameter<'a> {
    pub(crate) owner: Node<'a>,
    pub(crate) scope: Node<'a>,
    pub(crate) position: usize,
    pub(crate) binding: Node<'a>,
    pub(crate) selected: Vec<Selection<'a>>,
    pub(crate) body_read: Option<Node<'a>>,
    pub(crate) replaced: bool,
}

pub(crate) fn children(node: Node<'_>) -> impl DoubleEndedIterator<Item = Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter(|child| !child.is_extra())
        .collect::<Vec<_>>()
        .into_iter()
}

pub(crate) fn peel(mut node: Node<'_>) -> Option<Node<'_>> {
    loop {
        node = strip_transparent_wrappers(node)?;
        if node.kind() != "instantiation_expression" {
            return Some(node);
        }
        node = children(node).next()?;
    }
}

pub(crate) fn property_key(parsed: &ParsedFile, node: Node<'_>) -> Option<String> {
    if node.kind() == "computed_property_name" {
        return children(node)
            .next()
            .and_then(peel)
            .filter(|key| matches!(key.kind(), "string" | "number"))
            .and_then(|key| definite_key(parsed, key));
    }
    definite_key(parsed, node).or_else(|| {
        matches!(
            node.kind(),
            "identifier"
                | "property_identifier"
                | "shorthand_property_identifier"
                | "shorthand_property_identifier_pattern"
        )
        .then(|| ParsedFile::js_ts_decoded_identifier(parsed.node_text(&node)))
        .flatten()
    })
}

pub(crate) fn name(parsed: &ParsedFile, node: Node<'_>) -> Option<String> {
    if !matches!(
        node.kind(),
        "identifier"
            | "undefined"
            | "shorthand_property_identifier"
            | "shorthand_property_identifier_pattern"
    ) {
        return None;
    }
    let text = parsed.node_text(&node);
    if text.contains('\\') {
        ParsedFile::js_ts_decoded_identifier(text)
    } else {
        Some(text.to_owned())
    }
}

/// The entries of an array expression or pattern, including holes. A trailing
/// separator does not introduce an additional position.
pub(crate) fn array_entries(node: Node<'_>) -> Vec<Option<Node<'_>>> {
    let mut entries = Vec::new();
    let mut cursor = node.walk();
    let mut pending = false;
    for child in node.children(&mut cursor).filter(|child| !child.is_extra()) {
        match child.kind() {
            "," if !pending => entries.push(None),
            "," => pending = false,
            _ if child.is_named() => {
                entries.push(Some(child));
                pending = true;
            }
            _ => {}
        }
    }
    entries
}

fn find_binding<'a>(
    parsed: &'a ParsedFile,
    pattern: Node<'a>,
    wanted: &str,
    steps: &mut Vec<Selection<'a>>,
) -> Option<Node<'a>> {
    if name(parsed, pattern).as_deref() == Some(wanted) {
        return Some(pattern);
    }
    let mark = steps.len();
    let found = match pattern.kind() {
        "assignment_pattern" | "object_assignment_pattern" => {
            steps.push(Selection::Default(pattern.child_by_field_name("right")?));
            find_binding(parsed, pattern.child_by_field_name("left")?, wanted, steps)
        }
        "array_pattern" => {
            array_entries(pattern)
                .into_iter()
                .enumerate()
                .find_map(|(index, child)| {
                    let child = child?;
                    steps.push(Selection::Array(index));
                    let found = find_binding(parsed, child, wanted, steps);
                    if found.is_none() {
                        steps.truncate(mark);
                    }
                    found
                })
        }
        "object_pattern" => children(pattern).find_map(|entry| {
            let (key, binding) = match entry.kind() {
                "pair_pattern" => (
                    entry.child_by_field_name("key")?,
                    entry.child_by_field_name("value")?,
                ),
                "object_assignment_pattern" => (entry.child_by_field_name("left")?, entry),
                "rest_pattern" => return find_binding(parsed, entry, wanted, steps),
                _ => (entry, entry),
            };
            steps.push(match property_key(parsed, key) {
                Some(key) => Selection::Object(key),
                None => Selection::Computed(key),
            });
            let found = find_binding(parsed, binding, wanted, steps);
            if found.is_none() {
                steps.truncate(mark);
            }
            found
        }),
        "rest_pattern" => {
            steps.push(Selection::Rest);
            find_binding(parsed, children(pattern).next()?, wanted, steps)
        }
        _ => None,
    };
    if found.is_none() {
        steps.truncate(mark);
    }
    found
}

pub(crate) fn formals(owner: Node<'_>) -> impl Iterator<Item = (Node<'_>, Option<Node<'_>>)> {
    owner
        .child_by_field_name("parameters")
        .map(|list| children(list).collect())
        .or_else(|| {
            owner
                .child_by_field_name("parameter")
                .map(|parameter| vec![parameter])
        })
        .unwrap_or_default()
        .into_iter()
        .filter_map(formal_binding)
        .filter(|(binding, _)| binding.kind() != "this")
}

pub(crate) fn parameter<'a>(
    parsed: &'a ParsedFile,
    occurrence: Node<'a>,
    cache: &mut JsBindingCache<'a>,
) -> Option<Parameter<'a>> {
    let spelling = name(parsed, occurrence)?;
    let Walk::Found(scope, None) = parsed.js_ts_binding_walk(occurrence, &spelling, cache) else {
        return None;
    };
    let (declarations, annex_b) = parsed.js_ts_scope_lookup(scope, &spelling, cache);
    let declarations = declarations?;
    let owner = if scope.kind() == "formal_parameters" {
        scope.parent()?
    } else {
        scope
    };
    let list = owner
        .child_by_field_name("parameters")
        .or_else(|| owner.child_by_field_name("parameter"));
    if let Some(list) = list.filter(|list| declarations.contains(list)) {
        let (position, binding, selected) = formals(owner)
            .enumerate()
            .filter_map(|(position, (pattern, default))| {
                let mut selected: Vec<_> = default.into_iter().map(Selection::Default).collect();
                let binding = find_binding(parsed, pattern, &spelling, &mut selected)?;
                Some((position, binding, selected))
            })
            .last()?;
        let replaced = annex_b
            || declarations.iter().any(|declaration| {
                *declaration != list
                    && (declaration.kind() != "variable_declarator"
                        || declaration.child_by_field_name("value").is_some()
                        || parsed.js_ts_seam_binding(&owner, &spelling))
            });
        return Some(Parameter {
            owner,
            scope,
            position,
            binding,
            selected,
            body_read: None,
            replaced,
        });
    }

    let [declaration] = declarations.as_slice() else {
        return None;
    };
    if annex_b || !is_const(*declaration) {
        return None;
    }
    let pattern = declaration.child_by_field_name("name")?;
    if !matches!(pattern.kind(), "object_pattern" | "array_pattern") {
        return None;
    }
    let read = peel(declaration.child_by_field_name("value")?)?;
    let mut parent = direct_parameter(parsed, read, cache)?;
    if !parent
        .selected
        .iter()
        .all(|step| matches!(step, Selection::Default(_)))
    {
        return None;
    }
    let binding = find_binding(parsed, pattern, &spelling, &mut parent.selected)?;
    parent.scope = scope;
    parent.binding = binding;
    parent.body_read = Some(read);
    parent.replaced |= declaration.end_byte() > occurrence.start_byte();
    Some(parent)
}

fn direct_parameter<'a>(
    parsed: &'a ParsedFile,
    occurrence: Node<'a>,
    cache: &mut JsBindingCache<'a>,
) -> Option<Parameter<'a>> {
    let spelling = name(parsed, occurrence)?;
    let Walk::Found(scope, None) = parsed.js_ts_binding_walk(occurrence, &spelling, cache) else {
        return None;
    };
    if !is_js_ts_function_like(scope.kind()) && scope.kind() != "formal_parameters" {
        return None;
    }
    let owner = if scope.kind() == "formal_parameters" {
        scope.parent()?
    } else {
        scope
    };
    let list = owner
        .child_by_field_name("parameters")
        .or_else(|| owner.child_by_field_name("parameter"))?;
    let (declarations, _) = parsed.js_ts_scope_lookup(scope, &spelling, cache);
    if !declarations?.contains(&list) {
        return None;
    }
    parameter(parsed, occurrence, cache)
}

pub(crate) fn is_const(declaration: Node<'_>) -> bool {
    declaration.kind() == "variable_declarator"
        && declaration.parent().is_some_and(|list| {
            list.kind() == "lexical_declaration"
                && list
                    .child_by_field_name("kind")
                    .is_some_and(|kind| kind.kind() == "const")
        })
}

pub(crate) fn write_target<'a>(parsed: &ParsedFile, operation: Node<'a>) -> Option<Node<'a>> {
    parsed.js_ts_write_target(operation)
}

pub(crate) fn arguments_binding<'a>(
    parsed: &'a ParsedFile,
    occurrence: Node<'a>,
    owner: Node<'a>,
    cache: &mut JsBindingCache<'a>,
) -> bool {
    let Walk::Found(scope, None) = parsed.js_ts_binding_walk(occurrence, "arguments", cache) else {
        return false;
    };
    let list = owner.child_by_field_name("parameters");
    if scope != owner && Some(scope) != list {
        return false;
    }
    !formals(owner).any(|(formal, _)| name(parsed, formal).as_deref() == Some("arguments"))
        && parsed
            .js_ts_scope_lookup(scope, "arguments", cache)
            .0
            .is_some_and(|declarations| {
                declarations.iter().all(|declaration| {
                    Some(*declaration) == list
                        || declaration.kind() == "variable_declarator"
                            && declaration.child_by_field_name("value").is_none()
                })
            })
}
