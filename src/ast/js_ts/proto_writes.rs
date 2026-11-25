use crate::ast::{
    is_js_ts_function_like, member_projection::strip_transparent_wrappers, ParsedFile,
};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

mod classes;
mod effects;
mod name_reads;
mod scope;
use scope::Lexical;

pub(in crate::ast) use name_reads::Index as NameReadIndex;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtoWriteClass {
    NestedKeys,
    PathWalk,
    RecursiveMerge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardState {
    None,
    Partial,
    Sufficient,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyRole {
    Selector,
    Property,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub struct AstSpan {
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct KeyOperand {
    pub name: String,
    pub base: String,
    pub role: KeyRole,
    pub owner: AstSpan,
    pub binding: Option<AstSpan>,
    pub reaching_versions: Vec<AstSpan>,
    pub line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct NameRead {
    pub path: String,
    pub base: String,
    pub span: AstSpan,
    pub owner: AstSpan,
    pub binding: Option<AstSpan>,
    pub reaching_versions: Vec<AstSpan>,
    pub resolution_complete: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ProtoWrite {
    pub line: usize,
    pub end_line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
    pub function: String,
    pub owner: AstSpan,
    pub function_start_line: usize,
    pub function_end_line: usize,
    pub class: ProtoWriteClass,
    pub key_operands: Vec<KeyOperand>,
    pub guard: GuardState,
}

impl ParsedFile {
    /// Pollution-capable dynamic-key writes, sorted by (line, start_byte).
    pub fn js_ts_proto_writes(&self) -> Vec<ProtoWrite> {
        self.js_proto_write_index()
            .map(|index| index.facts.clone())
            .unwrap_or_default()
    }

    /// JS-family value reads on a one-based line, including unresolved reads.
    pub fn js_ts_name_reads(&self, line: usize) -> Vec<NameRead> {
        self.js_proto_write_index()
            .map(|index| index.line(line))
            .unwrap_or_default()
    }

    fn js_proto_write_index(&self) -> Option<&NameReadIndex> {
        self.language.is_js_family().then(|| {
            self.js_name_reads.get_or_init(|| {
                let analysis = name_reads::Analysis::build(self);
                analysis.index(classes::facts(self, &analysis))
            })
        })
    }
}

impl From<Node<'_>> for AstSpan {
    fn from(node: Node<'_>) -> Self {
        Self {
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
        }
    }
}

fn field<'a>(node: Node<'a>, name: &str) -> Option<Node<'a>> {
    node.child_by_field_name(name)
}

fn children(node: Node<'_>) -> impl Iterator<Item = Node<'_>> {
    let mut cursor = node.walk();
    let mut next = cursor.goto_first_child();
    std::iter::from_fn(move || {
        while next {
            let child = cursor.node();
            next = cursor.goto_next_sibling();
            if child.is_named() && !child.is_extra() {
                return Some(child);
            }
        }
        None
    })
}

fn peel(node: Node<'_>) -> Node<'_> {
    strip_transparent_wrappers(node).unwrap_or(node)
}

fn identifier(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "identifier" | "shorthand_property_identifier" | "shorthand_property_identifier_pattern"
    )
}

fn operator<'a>(node: Node<'_>, file: &'a ParsedFile) -> &'a str {
    field(node, "operator").map_or("", |op| file.node_text(&op))
}

fn path(file: &ParsedFile, mut node: Node<'_>) -> Option<(String, String)> {
    let mut segments = Vec::new();
    for _ in 0..128 {
        node = peel(node);
        match node.kind() {
            "member_expression" => segments.push(file.node_text(&field(node, "property")?)),
            "subscript_expression" => {
                let index = peel(field(node, "index")?);
                segments.push(literal_index(file, index).unwrap_or("[]"));
            }
            _ if identifier(node) || matches!(node.kind(), "this" | "super") => {
                let base = file.node_text(&node);
                segments.push(base);
                segments.reverse();
                return Some((base.to_string(), segments.join(".")));
            }
            _ => return None,
        }
        node = field(node, "object")?;
    }
    None
}

fn literal_index<'a>(file: &'a ParsedFile, index: Node<'_>) -> Option<&'a str> {
    let text = file.node_text(&index);
    match index.kind() {
        "number" if text.bytes().all(|b| b.is_ascii_digit()) => Some(text),
        "string" => text.get(1..text.len().saturating_sub(1)).filter(|value| {
            !value.is_empty()
                && value
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'$')
        }),
        _ => None,
    }
}

// One pass for all positions. A spread makes the following positions unknown.
fn array_positions(array: Node<'_>) -> impl Iterator<Item = (usize, Node<'_>)> {
    let mut position = 0;
    let mut cursor = array.walk();
    let mut next = cursor.goto_first_child();
    std::iter::from_fn(move || {
        while next {
            let child = cursor.node();
            next = cursor.goto_next_sibling();
            match child.kind() {
                "," => position += 1,
                "spread_element" => {
                    next = false;
                    break;
                }
                _ if child.is_named() && !child.is_extra() => return Some((position, child)),
                _ => {}
            }
        }
        None
    })
}

fn iteration<'a>(file: &ParsedFile, call: Node<'a>) -> Option<(Node<'a>, &'static str)> {
    let callee = field(call, "function")?;
    let receiver = field(callee, "object")?;
    let method = match file.node_text(&field(callee, "property")?) {
        "map" | "forEach" | "every" | "some" | "filter" | "find" | "findIndex" => "iterate",
        "reduce" | "reduceRight" => "reduce",
        _ => return None,
    };
    Some((receiver, method))
}

fn member_mutator(file: &ParsedFile, callee: Node<'_>) -> bool {
    field(callee, "property").is_some_and(|property| {
        matches!(
            file.node_text(&property),
            "push"
                | "pop"
                | "shift"
                | "unshift"
                | "splice"
                | "reverse"
                | "sort"
                | "fill"
                | "copyWithin"
        )
    })
}

fn is_owner(node: Node<'_>) -> bool {
    node.kind() == "program" || is_js_ts_function_like(node.kind())
}

fn is_inline_callback(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    let is_async = node
        .children(&mut cursor)
        .any(|child| child.kind() == "async");
    matches!(node.kind(), "arrow_function" | "function_expression") && !is_async
}

fn is_scope(node: Node<'_>) -> bool {
    is_owner(node)
        || matches!(
            node.kind(),
            "statement_block"
                | "for_statement"
                | "for_in_statement"
                | "switch_body"
                | "catch_clause"
                | "class"
                | "class_declaration"
                | "abstract_class_declaration"
                | "class_static_block"
                | "with_statement"
        )
}

fn pattern_parts(node: Node<'_>) -> Vec<Node<'_>> {
    match node.kind() {
        "pair_pattern" => field(node, "value").into_iter().collect(),
        "assignment_pattern" | "object_assignment_pattern" => {
            field(node, "left").into_iter().collect()
        }
        "required_parameter" | "optional_parameter" => field(node, "pattern")
            .or_else(|| field(node, "name"))
            .into_iter()
            .collect(),
        "object_pattern" | "array_pattern" | "rest_pattern" | "formal_parameters" => {
            children(node).collect()
        }
        _ => Vec::new(),
    }
}

#[derive(Clone, Copy)]
struct Context<'a> {
    scope: Node<'a>,
    owner: Node<'a>,
}

fn parameters(owner: Node<'_>) -> impl Iterator<Item = Node<'_>> {
    field(owner, "parameters")
        .into_iter()
        .flat_map(children)
        .chain(field(owner, "parameter"))
}

fn eager_owner_inputs(owner: Node<'_>) -> impl Iterator<Item = Node<'_>> {
    children(owner)
        .filter(|child| matches!(child.kind(), "decorator" | "computed_property_name"))
        .chain(
            parameters(owner).flat_map(|parameter| {
                children(parameter).filter(|child| child.kind() == "decorator")
            }),
        )
}

fn parameter_decorator(node: Node<'_>, parent: Node<'_>) -> bool {
    node.kind() == "decorator"
        && matches!(parent.kind(), "required_parameter" | "optional_parameter")
}

fn parameter_expressions(owner: Node<'_>) -> bool {
    parameters(owner).any(|parameter| {
        let mut pending = vec![parameter];
        while let Some(part) = pending.pop() {
            if matches!(
                part.kind(),
                "assignment_pattern" | "object_assignment_pattern" | "computed_property_name"
            ) || field(part, "value")
                .is_some_and(|_| matches!(part.kind(), "required_parameter" | "optional_parameter"))
            {
                return true;
            }
            pending.extend(children(part).filter(|child| child.kind() != "type_annotation"));
        }
        false
    })
}

fn has_token(node: Node<'_>, token: &str) -> bool {
    let mut cursor = node.walk();
    let found = node
        .children(&mut cursor)
        .any(|child| child.kind() == token);
    found
}

fn instance_initializer(node: Node<'_>) -> Option<Node<'_>> {
    (matches!(node.kind(), "field_definition" | "public_field_definition")
        && !has_token(node, "static"))
    .then(|| field(node, "value"))
    .flatten()
}

fn inline_iteration<'a>(file: &ParsedFile, callback: Node<'a>) -> Option<Node<'a>> {
    if !is_inline_callback(callback) {
        return None;
    }
    let mut wrapper = callback;
    while wrapper
        .parent()
        .is_some_and(|parent| peel(parent) == callback && parent != wrapper)
    {
        wrapper = wrapper.parent()?;
    }
    let args = wrapper.parent().filter(|node| node.kind() == "arguments")?;
    if children(args).next() != Some(wrapper) {
        return None;
    }
    let call = args.parent()?;
    iteration(file, call)?;
    Some(call)
}

fn is_loop(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "for_statement" | "for_in_statement" | "while_statement" | "do_statement"
    )
}

fn alias_sources<'a>(file: &ParsedFile, expression: Node<'a>) -> Vec<Node<'a>> {
    let mut pending = vec![expression];
    let mut sources = Vec::new();
    while let Some(value) = pending.pop().map(peel) {
        match value.kind() {
            "ternary_expression" => pending.extend(
                ["consequence", "alternative"]
                    .into_iter()
                    .filter_map(|arm| field(value, arm)),
            ),
            "sequence_expression" => pending.extend(children(value).last()),
            "assignment_expression" => pending.extend(field(value, "right")),
            "binary_expression" if matches!(operator(value, file), "||" | "??" | "&&") => {
                if operator(value, file) != "&&" {
                    pending.extend(field(value, "left"));
                }
                pending.extend(field(value, "right"));
            }
            "augmented_assignment_expression"
                if matches!(operator(value, file), "||=" | "??=" | "&&=") =>
            {
                pending.extend(field(value, "left"));
                pending.extend(field(value, "right"));
            }
            _ => sources.push(value),
        }
    }
    sources
}

fn bare_var(declaration: Node<'_>) -> bool {
    declaration.kind() == "variable_declarator"
        && field(declaration, "value").is_none()
        && declaration
            .parent()
            .is_some_and(|parent| parent.kind() == "variable_declaration")
}
