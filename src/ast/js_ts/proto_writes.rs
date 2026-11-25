use crate::ast::{
    is_js_ts_function_like, member_projection::strip_transparent_wrappers, ParsedFile,
};
use std::collections::BTreeMap;
use tree_sitter::Node;

mod classes;
mod name_reads;

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
    loop {
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

fn array_element(array: Node<'_>, index: usize) -> Option<Node<'_>> {
    let mut position = 0;
    let mut cursor = array.walk();
    for child in array.children(&mut cursor) {
        if child.kind() == "," {
            position += 1;
        } else if child.kind() == "spread_element" {
            return None;
        } else if position == index && child.is_named() && !child.is_extra() {
            return Some(child);
        }
    }
    None
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
                | "class_static_block"
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
    dynamic: bool,
}

#[derive(Default)]
struct Scope<'a> {
    parent: Option<usize>,
    names: BTreeMap<&'a str, Option<AstSpan>>,
    entry: Vec<AstSpan>,
}

struct Lexical<'a> {
    file: &'a ParsedFile,
    scopes: BTreeMap<usize, Scope<'a>>,
    binding_owners: BTreeMap<usize, AstSpan>,
}

impl<'a> Lexical<'a> {
    fn build(file: &'a ParsedFile) -> Self {
        let mut lexical = Self {
            file,
            scopes: BTreeMap::new(),
            binding_owners: BTreeMap::new(),
        };
        let root = file.tree.root_node();
        lexical.collect(
            root,
            Context {
                scope: root,
                owner: root,
                dynamic: false,
            },
        );
        lexical
    }
    fn declare(&mut self, pattern: Node<'a>, scope: Node<'a>, owner: Node<'a>, entry: bool) {
        if identifier(pattern) {
            let span = AstSpan::from(pattern);
            self.binding_owners
                .insert(span.start_byte, AstSpan::from(owner));
            let scope = self.scopes.entry(scope.id()).or_default();
            scope
                .names
                .entry(self.file.node_text(&pattern))
                .and_modify(|name| *name = None)
                .or_insert(Some(span.clone()));
            if entry {
                scope.entry.push(span);
            }
        } else {
            for part in pattern_parts(pattern) {
                self.declare(part, scope, owner, entry);
            }
        }
    }

    fn collect(&mut self, node: Node<'a>, parent: Context<'a>) {
        let mut context = parent;
        if is_scope(node) {
            self.scopes.insert(
                node.id(),
                Scope {
                    parent: (node != parent.scope).then_some(parent.scope.id()),
                    ..Scope::default()
                },
            );
            context.scope = node;
        }
        if is_owner(node) {
            context.owner = node;
        }
        match node.kind() {
            "variable_declarator" => {
                if let Some(name) = field(node, "name") {
                    let hoisted = node
                        .parent()
                        .is_some_and(|n| n.kind() == "variable_declaration");
                    self.declare(
                        name,
                        if hoisted {
                            context.owner
                        } else {
                            context.scope
                        },
                        context.owner,
                        hoisted,
                    );
                }
            }
            "function_declaration"
            | "generator_function_declaration"
            | "class_declaration"
            | "abstract_class_declaration" => {
                if let Some(name) = field(node, "name") {
                    self.declare(name, parent.scope, parent.owner, is_owner(node));
                }
            }
            "function_expression" | "generator_function" | "class" => {
                if let Some(name) = field(node, "name") {
                    self.declare(name, context.scope, context.owner, true);
                }
            }
            "for_in_statement" if field(node, "kind").is_some() => {
                if let Some(name) = field(node, "left") {
                    let hoisted = field(node, "kind").is_some_and(|n| n.kind() == "var");
                    self.declare(
                        name,
                        if hoisted {
                            context.owner
                        } else {
                            context.scope
                        },
                        context.owner,
                        hoisted,
                    );
                }
            }
            "catch_clause" => {
                if let Some(name) = field(node, "parameter") {
                    self.declare(name, context.scope, context.owner, false);
                }
            }
            "import_clause" | "namespace_import" | "import_specifier" => {
                let name = if node.kind() == "import_specifier" {
                    field(node, "alias").or_else(|| field(node, "name"))
                } else {
                    children(node).find(|&n| identifier(n))
                };
                if let Some(name) = name {
                    self.declare(name, context.scope, context.owner, true);
                }
            }
            _ => {}
        }
        if is_owner(node) {
            if let Some(params) = field(node, "parameters").or_else(|| field(node, "parameter")) {
                self.declare(params, node, node, false);
            }
        }
        for child in children(node) {
            self.collect(child, context);
        }
    }

    fn lookup(&self, node: Node<'a>, name: &str) -> Option<&Option<AstSpan>> {
        let mut next = Some(node.id());
        while let Some(id) = next {
            let scope = &self.scopes[&id];
            if let Some(binding) = scope.names.get(name) {
                return Some(binding);
            }
            next = scope.parent;
        }
        None
    }

    fn binding(&self, node: Node<'a>, name: &str) -> Option<AstSpan> {
        self.lookup(node, name).cloned().flatten()
    }
}
