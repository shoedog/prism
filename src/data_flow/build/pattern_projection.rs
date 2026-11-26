//! Owner-wide projection vetoes, independent of read admission.

use super::pattern_execution as execution;
use super::pattern_formals::decoded_name;
use super::pattern_reads::reads_value;
use crate::access_path::AccessPath;
use crate::ast::js_ts::props_keys::{definite_key, strip_transparent_wrappers};
use crate::ast::ParsedFile;
use crate::data_flow::s1_evidence::ProjectionCause;
use std::collections::BTreeSet;
use tree_sitter::Node;

pub(super) fn read_operand<'a>(
    parsed: &ParsedFile,
    root: Node<'a>,
    base: &AccessPath,
) -> Option<(Node<'a>, AccessPath)> {
    if parsed.js_ts_identifier_is_not_a_read(root) {
        return None;
    }
    let mut receiver = root;
    let mut operand = root;
    let mut path = base.clone();
    let mut dynamic = false;
    while let Some(parent) = receiver.parent() {
        match parent.kind() {
            "jsx_closing_element" | "meta_property" => return None,
            "unary_expression"
                if root == operand
                    && parent
                        .child_by_field_name("argument")
                        .is_some_and(|argument| {
                            strip_transparent_wrappers(argument) == Some(root)
                        })
                    && parent
                        .child_by_field_name("operator")
                        .is_some_and(|operator| operator.kind() == "delete") =>
            {
                return None;
            }
            "member_expression" | "subscript_expression"
                if parent.child_by_field_name("object") == Some(receiver) =>
            {
                operand = parent;
                if let Some(key) = member_key(parsed, parent) {
                    path.fields.push(key);
                } else {
                    dynamic = true;
                }
            }
            _ if strip_transparent_wrappers(parent) == Some(operand) => {}
            _ => break,
        }
        receiver = parent;
    }
    if operand != root && parsed.member_is_write_target(operand) && !reads_value(operand) {
        return None;
    }
    if dynamic {
        Some((root, base.clone()))
    } else {
        Some((operand, path))
    }
}

pub(super) fn member_key(parsed: &ParsedFile, member: Node<'_>) -> Option<String> {
    match member.kind() {
        "member_expression" => {
            let property = member.child_by_field_name("property")?;
            decoded_name(parsed.node_text(&property)).map(|key| key.into_owned())
        }
        "subscript_expression" => {
            let key = strip_transparent_wrappers(member.child_by_field_name("index")?)?;
            match key.kind() {
                // The path representation uses a leading '#' for private names.
                "string" => parsed
                    .js_ts_module_export_name(key)
                    .filter(|key| !key.starts_with('#')),
                "number" => definite_key(parsed, key),
                _ => None,
            }
        }
        _ => None,
    }
}

type Store = (Option<String>, Option<usize>, Vec<Option<String>>);

pub(super) struct Stores {
    targets: BTreeSet<Store>,
    pub(super) has_with: bool,
}

impl Stores {
    pub(super) fn collect(parsed: &ParsedFile, owner: Node<'_>) -> Self {
        let mut result = Self {
            targets: BTreeSet::new(),
            has_with: false,
        };
        let mut stack = vec![owner];
        while let Some(node) = stack.pop() {
            if !execution::runtime_node(parsed, node) {
                continue;
            }
            result.has_with |= node.kind() == "with_statement";
            if is_member(node) && parsed.member_is_write_target(node) {
                let (root, fields) = store_path(parsed, node);
                let name = root
                    .filter(|node| matches!(node.kind(), "identifier" | "undefined"))
                    .and_then(|node| decoded_name(parsed.node_text(&node)));
                let scope = root.zip(name.as_deref()).and_then(|(root, name)| {
                    if super::pattern_formals::in_body_seam(parsed, owner, name, root.start_byte())
                    {
                        return None;
                    }
                    execution::binding_scope(parsed, owner, root, name)
                });
                result
                    .targets
                    .insert((name.map(|name| name.into_owned()), scope, fields));
            }
            stack.extend(node.named_children(&mut node.walk()));
        }
        result
    }

    pub(super) fn causes(
        &self,
        owner: Node<'_>,
        path: &AccessPath,
    ) -> impl Iterator<Item = ProjectionCause> {
        let mut other_root = false;
        let mut non_comparable = false;
        for (name, scope, fields) in &self.targets {
            if name.as_deref() != Some(path.base.as_str()) || *scope != Some(owner.id()) {
                other_root = true;
            } else if !fields
                .iter()
                .zip(&path.fields)
                .all(|(key, part)| key.as_ref() == Some(part))
            {
                non_comparable = true;
            }
        }
        [
            (self.has_with, ProjectionCause::With),
            (other_root, ProjectionCause::OtherRootStore),
            (non_comparable, ProjectionCause::SameRootNonComparableStore),
        ]
        .into_iter()
        .filter_map(|(present, cause)| present.then_some(cause))
    }
}

fn is_member(node: Node<'_>) -> bool {
    matches!(node.kind(), "member_expression" | "subscript_expression")
}

fn store_path<'a>(
    parsed: &ParsedFile,
    target: Node<'a>,
) -> (Option<Node<'a>>, Vec<Option<String>>) {
    let mut fields = Vec::new();
    let mut cursor = Some(target);
    while let Some(node) = cursor.filter(|&node| is_member(node)) {
        fields.push(member_key(parsed, node));
        cursor = node
            .child_by_field_name("object")
            .and_then(strip_transparent_wrappers);
    }
    fields.reverse();
    (cursor, fields)
}

/// An ordinary function's own arguments object remains visible through arrows.
pub(super) fn reads_owner_arguments(
    parsed: &ParsedFile,
    owner: Node<'_>,
    operand: Node<'_>,
) -> bool {
    if owner.kind() == "arrow_function"
        || parsed.js_ts_identifier_is_not_a_read(operand)
        || parsed.js_ts_span_in_with_body(operand.start_byte(), operand.end_byte())
    {
        return false;
    }
    let binding = parsed.js_ts_def_scope_id(operand.start_byte(), "arguments");
    let boundaries = parsed.language.callable_boundary_node_types();
    let mut child = operand;
    let mut decorated = None;
    while let Some(parent) = child.parent() {
        if binding == Some(parent.id()) {
            return false;
        }
        if let Some(callable) = execution::decorated_callable(parent) {
            decorated = Some(callable);
        }
        if boundaries.contains(&parent.kind())
            && parent.kind() != "arrow_function"
            && decorated != Some(parent)
            && ["parameters", "body"]
                .iter()
                .any(|field| parent.child_by_field_name(field) == Some(child))
        {
            return parent == owner;
        }
        if parent == owner {
            return false;
        }
        child = parent;
    }
    false
}
