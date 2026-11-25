use crate::targets::dependency_identity::python_imports;
use crate::targets::inventory::python::{
    source::{children, name, ungroup},
    walk::{ScopeId, ScopeKind, Walk},
};
use prism::ast::ParsedFile;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

#[derive(Debug)]
pub(super) struct Import {
    /// Dotted identity of the binding, with relative level kept separately.
    pub path: String,
    pub relative: bool,
    pub from: bool,
    pub alias: bool,
}

pub(super) enum Kind<'tree> {
    Import(Import),
    Assigned(Option<Node<'tree>>),
    Delete,
    Annotation,
    Other,
}

pub(super) struct Binding<'tree> {
    pub scope: ScopeId,
    pub node: Node<'tree>,
    pub position: usize,
    pub kind: Kind<'tree>,
}

#[derive(Default)]
pub(super) struct Namespace<'tree> {
    pub bindings: BTreeMap<String, Vec<Binding<'tree>>>,
    pub globals: BTreeSet<String>,
    pub nonlocals: BTreeSet<String>,
}

pub(super) struct Census<'tree> {
    pub spaces: Vec<Namespace<'tree>>,
    pub wildcard: bool,
}

impl<'tree> Census<'tree> {
    pub(super) fn new(parsed: &ParsedFile, walk: &Walk<'tree>) -> Self {
        let mut this = Self {
            spaces: walk.scopes.iter().map(|_| Namespace::default()).collect(),
            wildcard: false,
        };
        for (scope, block) in walk.scopes.iter().enumerate() {
            if block.kind.is_function() {
                if let Some(params) = block.node.child_by_field_name("parameters") {
                    for parameter in children(params) {
                        if let Some(target) = parameter_name(parameter) {
                            this.add(parsed, walk, scope, target, parameter, Kind::Other);
                        }
                    }
                }
            }
        }
        for &(node, context) in &walk.nodes {
            let scope = context.scope;
            match node.kind() {
                "function_definition" | "class_definition" => {
                    if let Some(target) = node.child_by_field_name("name") {
                        this.add(parsed, walk, scope, target, node, Kind::Other);
                    }
                }
                "import_statement" | "import_from_statement" => {
                    this.wildcard |= children(node).any(|child| child.kind() == "wildcard_import");
                    for item in python_imports(parsed, node) {
                        let from = item.from.is_some();
                        let alias = item.item.kind() == "aliased_import";
                        let imported = item.item.child_by_field_name("name").unwrap_or(item.item);
                        let mut path = dotted(parsed, imported);
                        if !from && !alias {
                            path.truncate(path.find('.').unwrap_or(path.len()));
                        }
                        let relative = item
                            .from
                            .is_some_and(|source| source.kind() == "relative_import");
                        if let Some(source) = item.from {
                            let module = dotted(parsed, source);
                            if !module.is_empty() {
                                path = format!("{module}.{path}");
                            }
                        }
                        let binding = Binding {
                            scope,
                            node,
                            position: item.item.end_byte(),
                            kind: Kind::Import(Import {
                                path,
                                relative,
                                from,
                                alias,
                            }),
                        };
                        this.insert(scope, name(item.name, walk.class(scope)), binding);
                    }
                }
                "global_statement" | "nonlocal_statement" => {
                    let space = &mut this.spaces[scope];
                    let names = if node.kind() == "global_statement" {
                        &mut space.globals
                    } else {
                        &mut space.nonlocals
                    };
                    names.extend(
                        children(node).map(|id| name(parsed.node_text(&id), walk.class(scope))),
                    );
                }
                "assignment" | "augmented_assignment" => {
                    let annotation = node.child_by_field_name("type").is_some();
                    let right = node.child_by_field_name("right");
                    if let Some(left) = node.child_by_field_name("left") {
                        if annotation && walk.scopes[scope].kind == ScopeKind::Class {
                            this.insert(
                                scope,
                                "__annotations__".to_owned(),
                                Binding {
                                    scope,
                                    node: walk.scopes[scope].body,
                                    position: walk.scopes[scope].body.start_byte(),
                                    kind: Kind::Other,
                                },
                            );
                        }
                        if !annotation || right.is_some() || left.kind() == "identifier" {
                            let kind = if annotation && right.is_none() {
                                Kind::Annotation
                            } else if node.kind() == "assignment" && left.kind() == "identifier" {
                                Kind::Assigned(right)
                            } else {
                                Kind::Other
                            };
                            this.targets(parsed, walk, scope, left, node, kind);
                        }
                    }
                }
                "for_statement" | "for_in_clause" => {
                    if let Some(left) = node.child_by_field_name("left") {
                        this.targets(parsed, walk, scope, left, node, Kind::Other);
                    }
                }
                "named_expression" => {
                    let mut owner = scope;
                    while walk.scopes[owner].kind == ScopeKind::Comprehension {
                        owner = walk.scopes[owner].parent;
                    }
                    if let Some(target) = node.child_by_field_name("name") {
                        this.insert(
                            owner,
                            name(parsed.node_text(&target), walk.class(scope)),
                            Binding {
                                scope,
                                node,
                                position: node.end_byte(),
                                kind: Kind::Other,
                            },
                        );
                    }
                }
                "delete_statement" => {
                    for target in children(node) {
                        this.targets(parsed, walk, scope, target, node, Kind::Delete);
                    }
                }
                "as_pattern" => {
                    if let Some(alias) = node.child_by_field_name("alias") {
                        this.targets(parsed, walk, scope, alias, node, Kind::Other);
                    }
                }
                "case_pattern" => {
                    for target in pattern_names(node) {
                        if parsed.node_text(&target) != "_" {
                            this.add(parsed, walk, scope, target, node, Kind::Other);
                        }
                    }
                }
                "type_alias_statement" => {
                    if let Some(target) = children(node).next().and_then(first_identifier) {
                        this.add(parsed, walk, scope, target, node, Kind::Other);
                    }
                }
                _ => {}
            }
        }
        this.redirect(walk);
        this
    }

    fn targets(
        &mut self,
        parsed: &ParsedFile,
        walk: &Walk<'tree>,
        scope: ScopeId,
        target: Node<'tree>,
        node: Node<'tree>,
        kind: Kind<'tree>,
    ) {
        let target = ungroup(target);
        if target.kind() == "identifier" {
            self.add(parsed, walk, scope, target, node, kind);
        } else if matches!(
            target.kind(),
            "expression_list"
                | "list"
                | "tuple"
                | "pattern_list"
                | "list_pattern"
                | "tuple_pattern"
                | "list_splat_pattern"
                | "list_splat"
                | "as_pattern_target"
        ) {
            for child in children(target) {
                let kind = if matches!(kind, Kind::Delete) {
                    Kind::Delete
                } else {
                    Kind::Other
                };
                self.targets(parsed, walk, scope, child, node, kind);
            }
        }
    }

    fn add(
        &mut self,
        parsed: &ParsedFile,
        walk: &Walk<'tree>,
        scope: ScopeId,
        target: Node<'tree>,
        node: Node<'tree>,
        kind: Kind<'tree>,
    ) {
        self.insert(
            scope,
            name(parsed.node_text(&target), walk.class(scope)),
            Binding {
                scope,
                node,
                kind,
                position: node.end_byte(),
            },
        );
    }

    fn insert(&mut self, scope: ScopeId, name: String, binding: Binding<'tree>) {
        self.spaces[scope]
            .bindings
            .entry(name)
            .or_default()
            .push(binding);
    }

    fn redirect(&mut self, walk: &Walk<'tree>) {
        let mut redirected = Vec::new();
        for scope in 1..self.spaces.len() {
            let names: Vec<_> = self.spaces[scope].bindings.keys().cloned().collect();
            for name in names {
                let owner = self.declared_owner(walk, scope, &name);
                if owner != scope {
                    let bindings = self.spaces[scope].bindings.remove(&name).unwrap();
                    redirected.push((owner, name, bindings));
                }
            }
        }
        for (owner, name, bindings) in redirected {
            self.spaces[owner]
                .bindings
                .entry(name)
                .or_default()
                .extend(bindings);
        }
        for space in &mut self.spaces {
            for bindings in space.bindings.values_mut() {
                bindings.sort_by_key(|binding| binding.position);
            }
        }
    }

    pub(super) fn declared_owner(&self, walk: &Walk<'tree>, scope: ScopeId, name: &str) -> ScopeId {
        let space = &self.spaces[scope];
        if space.globals.contains(name) {
            return 0;
        }
        if space.nonlocals.contains(name) {
            let mut parent = walk.scopes[scope].parent;
            while parent != 0 {
                let ns = &self.spaces[parent];
                if walk.scopes[parent].kind.is_function()
                    && ns.bindings.contains_key(name)
                    && !ns.nonlocals.contains(name)
                    && !ns.globals.contains(name)
                {
                    return parent;
                }
                parent = walk.scopes[parent].parent;
            }
        }
        scope
    }
}

fn parameter_name(node: Node<'_>) -> Option<Node<'_>> {
    match node.kind() {
        "identifier" => Some(node),
        "default_parameter" | "typed_default_parameter" => node.child_by_field_name("name"),
        "typed_parameter" | "list_splat_pattern" | "dictionary_splat_pattern" => {
            children(node).next().and_then(parameter_name)
        }
        _ => None,
    }
}

fn first_identifier(node: Node<'_>) -> Option<Node<'_>> {
    if node.kind() == "identifier" {
        Some(node)
    } else {
        children(node).find_map(first_identifier)
    }
}

fn pattern_names(node: Node<'_>) -> Vec<Node<'_>> {
    match node.kind() {
        "identifier" => vec![node],
        "dotted_name" if node.named_child_count() == 1 => children(node).collect(),
        "dotted_name" => Vec::new(),
        "class_pattern" | "keyword_pattern" => {
            children(node).skip(1).flat_map(pattern_names).collect()
        }
        "dict_pattern" => children(node)
            .filter(|child| child.kind() == "splat_pattern" || child.kind() == "case_pattern")
            .flat_map(pattern_names)
            .collect(),
        "case_pattern" | "list_pattern" | "tuple_pattern" | "union_pattern" | "splat_pattern"
        | "as_pattern" => children(node).flat_map(pattern_names).collect(),
        _ => Vec::new(),
    }
}

fn dotted(parsed: &ParsedFile, node: Node<'_>) -> String {
    match node.kind() {
        "identifier" => name(parsed.node_text(&node), None),
        "dotted_name" | "relative_import" => children(node)
            .filter(|part| part.kind() != "import_prefix")
            .map(|part| dotted(parsed, part))
            .collect::<Vec<_>>()
            .join("."),
        _ => String::new(),
    }
}
