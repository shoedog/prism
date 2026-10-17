use super::*;
use crate::{anon_owner, ast::ParsedFile, languages::Language};
use std::collections::BTreeMap;
use tree_sitter::Node;

/// Reuses the existing inventory and identities, including early-error owners.
/// No graph or resolution facts are constructed by this syntax-only pass.
pub(crate) fn extract(parsed: &ParsedFile, file: &str) -> BTreeMap<FunctionId, AnonMeta> {
    let mut out = BTreeMap::new();
    if !matches!(
        parsed.language,
        Language::JavaScript | Language::TypeScript | Language::Tsx
    ) {
        return out;
    }
    let mut nodes = anon_owner::owner_nodes(parsed);
    nodes.sort_by_key(|n| (n.start_byte(), std::cmp::Reverse(n.end_byte())));
    let ids: BTreeMap<_, _> = nodes
        .iter()
        .filter_map(|n| anon_owner::identity(parsed, n, file).map(|id| (n.id(), id)))
        .collect();
    let mut contexts: BTreeMap<usize, super::label::Context> = BTreeMap::new();
    let mut siblings: BTreeMap<usize, usize> = BTreeMap::new();
    for node in nodes {
        let Some(id) = ids.get(&node.id()) else {
            continue;
        };
        if !anon_owner::is_anonymous_owner_name(&id.name) {
            contexts.insert(node.id(), super::label::Context::named(parsed, node, id));
            continue;
        }
        let mut child = node;
        let mut ancestor = node.parent();
        let mut enclosing = None;
        let mut field = None;
        let mut parent_key = parsed.tree.root_node().id();
        while let Some(n) = ancestor {
            // A computed key executes in the surrounding scope. Only the
            // initializer value introduces a field scope barrier.
            if is_field(n) && same_field(n, "value", child) {
                field = Some(n);
                parent_key = n.id();
                break;
            }
            if let Some(parent_id) = ids.get(&n.id()) {
                enclosing = Some(parent_id.clone());
                parent_key = n.id();
                break;
            }
            child = n;
            ancestor = n.parent();
        }
        // Known gap (Opus S4): top-level class static blocks use module
        // context; dedicated static-block labels are deferred.
        let (scope, display) = field
            .map(|f| {
                let (class, name) = field_names(parsed, f);
                (
                    format!("class {class} field {name}"),
                    format!("{class}.{name}"),
                )
            })
            .unwrap_or_else(|| ("module".into(), "module".into()));
        let mut context = contexts
            .get(&parent_key)
            .cloned()
            .unwrap_or_else(|| super::label::Context::scope(file, &scope, display));
        let ordinal = siblings.entry(parent_key).or_default();
        context.ordinals.push(*ordinal);
        *ordinal += 1;
        let value = peel_out(node);
        let handoff = passed_to(parsed, value);
        let passed_to = handoff.as_ref().map(|(p, _)| p.clone());
        let bound_as = role(parsed, node, value, enclosing.as_ref());
        let mut meta = AnonMeta {
            anonymous: true,
            col: node.start_position().column + 1,
            kind: match node.kind() {
                "arrow_function" => AnonKind::Arrow,
                "generator_function" => AnonKind::Generator,
                _ => AnonKind::Function,
            },
            r#async: {
                let mut c = node.walk();
                let found = node.children(&mut c).any(|n| n.kind() == "async");
                found
            },
            enclosing_scope: enclosing.is_none().then_some(scope),
            enclosing,
            anchor: AnonAnchor {
                version: 1,
                root: context.root.clone(),
                ordinals: context.ordinals.clone(),
                callee_text: passed_to.as_ref().map(|p| p.callee_text.clone()),
            },
            passed_to,
            bound_as,
            label: String::new(),
        };
        context.segments.push(super::label::segment(
            &meta,
            handoff.as_ref().map(|(_, text)| text.as_str()),
        ));
        meta.label = context.render(id.start_line, meta.col);
        contexts.insert(node.id(), context);
        out.insert(id.clone(), meta);
    }
    out
}

fn same_field(parent: Node<'_>, field: &str, child: Node<'_>) -> bool {
    parent.child_by_field_name(field) == Some(child)
}

/// A wrapper is transparent only through its value child, never a type child.
fn wrapper_value(node: Node<'_>) -> Option<Node<'_>> {
    let mut c = node.walk();
    let mut children = node
        .named_children(&mut c)
        .filter(|n| n.kind() != "comment");
    match node.kind() {
        "parenthesized_expression"
        | "as_expression"
        | "satisfies_expression"
        | "non_null_expression"
        | "jsx_expression" => children.next(),
        "type_assertion" => children.find(|n| n.kind() != "type_arguments"),
        _ => None,
    }
}

fn peel_out(mut node: Node<'_>) -> Node<'_> {
    while let Some(parent) = node.parent() {
        if wrapper_value(parent) != Some(node) {
            break;
        }
        node = parent;
    }
    node
}

fn passed_to(parsed: &ParsedFile, value: Node<'_>) -> Option<(PassedTo, String)> {
    let args = value.parent().filter(|p| p.kind() == "arguments")?;
    let call = args
        .parent()
        .filter(|p| matches!(p.kind(), "call_expression" | "new_expression"))?;
    let callee = call
        .child_by_field_name("function")
        .or_else(|| call.child_by_field_name("constructor"))?;
    let mut c = args.walk();
    let arg_index = args
        .named_children(&mut c)
        .filter(|n| n.kind() != "comment")
        .position(|n| n == value)?;
    let display = if callee.kind() == "member_expression" {
        format!(
            ".{}",
            parsed.node_text(&callee.child_by_field_name("property")?)
        )
    } else {
        parsed.node_text(&callee).to_string()
    };
    Some((
        PassedTo {
            callee_text: parsed.node_text(&callee).into(),
            arg_index,
            call_line: call.start_position().row + 1,
            call_span: (call.start_byte(), call.end_byte()),
            callee: None,
        },
        display,
    ))
}

fn is_field(node: Node<'_>) -> bool {
    matches!(node.kind(), "public_field_definition" | "field_definition")
}

fn field_names(parsed: &ParsedFile, field: Node<'_>) -> (String, String) {
    let mut n = field.parent();
    let mut class = "<anonymous>".to_string();
    while let Some(parent) = n {
        if matches!(
            parent.kind(),
            "class" | "class_declaration" | "abstract_class_declaration"
        ) {
            if let Some(name) = parent.child_by_field_name("name") {
                class = parsed.node_text(&name).into();
            }
            break;
        }
        n = parent.parent();
    }
    let name = field
        .child_by_field_name("property")
        .or_else(|| field.child_by_field_name("name"))
        .map(|n| parsed.node_text(&n).to_string())
        .unwrap_or_else(|| "<unknown>".into());
    (class, name)
}

fn role(
    parsed: &ParsedFile,
    node: Node<'_>,
    value: Node<'_>,
    enclosing: Option<&FunctionId>,
) -> Option<BoundAs> {
    let parent = value.parent()?;
    if parent.kind() == "jsx_attribute" {
        let prop = parent
            .child_by_field_name("name")
            .or_else(|| parent.named_child(0))?;
        let element = parent.parent()?.child_by_field_name("name")?;
        return Some(BoundAs::JsxProp {
            element: parsed.node_text(&element).into(),
            prop: parsed.node_text(&prop).into(),
        });
    }
    if parent.kind() == "call_expression" && same_field(parent, "function", value) {
        return Some(BoundAs::Iife);
    }
    if matches!(
        parent.kind(),
        "assignment_pattern"
            | "object_assignment_pattern"
            | "required_parameter"
            | "optional_parameter"
    ) && (same_field(parent, "right", value) || same_field(parent, "value", value))
        && in_parameters(parsed, parent)
    {
        let param = parent
            .child_by_field_name("left")
            .or_else(|| parent.child_by_field_name("pattern"))
            .or_else(|| parent.child_by_field_name("name"))?;
        return Some(BoundAs::DefaultParam {
            function: enclosing?.clone(),
            param: parsed.node_text(&param).into(),
        });
    }
    if parent.kind() == "return_statement"
        || (parent.kind() == "arrow_function" && same_field(parent, "body", value))
    {
        return Some(BoundAs::Returned);
    }
    if parent.kind() == "export_statement" {
        let mut c = parent.walk();
        if parent.children(&mut c).any(|n| n.kind() == "default") {
            return Some(BoundAs::DefaultExport);
        }
    }
    if parent.kind() == "variable_declarator" && value != node && same_field(parent, "value", value)
    {
        return Some(BoundAs::VariableParenthesized {
            name: parsed
                .node_text(&parent.child_by_field_name("name")?)
                .into(),
        });
    }
    // Holders may wrap a direct handoff, but never cross another callable or
    // a value-changing container (array, conditional, binary expression, etc.).
    let mut value = value;
    while let Some(parent) = value.parent() {
        if parent.kind() == "pair" && same_field(parent, "value", value) {
            return Some(BoundAs::ObjectKey {
                key: parsed.node_text(&parent.child_by_field_name("key")?).into(),
            });
        }
        if is_field(parent) && same_field(parent, "value", value) {
            let (class, field) = field_names(parsed, parent);
            return Some(BoundAs::ClassField { class, field });
        }
        if parent.kind() != "arguments" {
            break;
        }
        let Some(call) = parent
            .parent()
            .filter(|n| matches!(n.kind(), "call_expression" | "new_expression"))
        else {
            break;
        };
        value = peel_out(call);
    }
    None
}

fn in_parameters(parsed: &ParsedFile, mut node: Node<'_>) -> bool {
    let barriers = parsed.language.callable_boundary_node_types();
    while let Some(parent) = node.parent() {
        if parent.kind() == "formal_parameters" {
            return true;
        }
        if barriers.contains(&parent.kind()) {
            break;
        }
        node = parent;
    }
    false
}
