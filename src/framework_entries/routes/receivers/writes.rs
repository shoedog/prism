use super::{
    children, comprehension,
    imports::{import_items, imported_as},
    python_scope, ParsedFile,
};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;
use unicode_normalization::UnicodeNormalization;

pub(super) fn normalized(name: &str) -> Cow<'_, str> {
    if name.is_ascii() {
        Cow::Borrowed(name)
    } else {
        Cow::Owned(name.nfkc().collect())
    }
}

pub(super) fn private_name(name: &str) -> bool {
    name.starts_with("__") && !name.ends_with("__")
}

pub(super) fn effective_name<'a>(
    file: &ParsedFile,
    mut scope: Node<'_>,
    name: &'a str,
) -> Cow<'a, str> {
    let name = normalized(name);
    if private_name(&name) {
        loop {
            if scope.kind() == "class_definition" {
                if let Some(class) = scope.child_by_field_name("name") {
                    let class = normalized(file.node_text(&class));
                    let prefix = class.trim_start_matches('_');
                    if !prefix.is_empty() {
                        return Cow::Owned(format!("_{prefix}{name}"));
                    }
                }
                break;
            }
            let outer = python_scope(scope);
            if outer == scope {
                break;
            }
            scope = outer;
        }
    }
    name
}

#[derive(Default)]
pub(super) struct LexicalWrites<'a> {
    scopes: BTreeMap<usize, ScopeWrites<'a>>,
    module_sites: BTreeMap<String, Vec<Node<'a>>>,
    global_writes: BTreeMap<Cow<'a, str>, Vec<Node<'a>>>,
    capture_sites: BTreeMap<Cow<'a, str>, Vec<Node<'a>>>,
}

#[derive(Default)]
struct ScopeWrites<'a> {
    names: BTreeMap<Cow<'a, str>, Vec<Node<'a>>>,
    wildcard: Vec<Node<'a>>,
    imports: Vec<Node<'a>>,
    named_imports: BTreeMap<Cow<'a, str>, Vec<Node<'a>>>,
    directives: BTreeMap<Cow<'a, str>, Directive>,
    parameters: BTreeSet<Cow<'a, str>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Directive {
    Global,
    Nonlocal,
}

fn enclosing_non_comprehension(mut scope: Node<'_>) -> Node<'_> {
    while comprehension(scope) {
        scope = python_scope(scope);
    }
    scope
}

fn before<'a>(sites: &[Node<'a>], offset: usize) -> Option<Node<'a>> {
    let end = sites.partition_point(|node| node.end_byte() <= offset);
    end.checked_sub(1).map(|index| sites[index])
}

impl<'a> LexicalWrites<'a> {
    pub(super) fn index(file: &'a ParsedFile, nodes: &[Node<'a>]) -> Self {
        let mut result = Self::default();
        let mut nested_writes = Vec::new();
        for &node in nodes {
            let (names, wildcard) = written_names(file, node);
            if names.is_empty() && !wildcard {
                continue;
            }
            let parameter = matches!(node.kind(), "parameters" | "lambda_parameters");
            let scope = match node.kind() {
                "parameters" | "lambda_parameters" => node.parent().expect("parameter owner"),
                "named_expression" => enclosing_non_comprehension(python_scope(node)),
                _ => python_scope(node),
            };
            let writes = result.scopes.entry(scope.id()).or_default();
            let import = matches!(node.kind(), "import_statement" | "import_from_statement");
            if import {
                writes.imports.push(node);
            }
            let directive = match node.kind() {
                "global_statement" => Some(Directive::Global),
                "nonlocal_statement" => Some(Directive::Nonlocal),
                _ => None,
            };
            let value_site =
                node.kind() != "assignment" || node.child_by_field_name("right").is_some();
            for name in names {
                let name = effective_name(file, scope, name);
                if let Some(directive) = directive {
                    writes.directives.insert(name, directive);
                    continue;
                }
                if import {
                    writes
                        .named_imports
                        .entry(name.clone())
                        .or_default()
                        .push(node);
                }
                if value_site {
                    result
                        .capture_sites
                        .entry(name.clone())
                        .or_default()
                        .push(node);
                }
                if parameter {
                    writes.parameters.insert(name);
                    continue;
                }
                if value_site {
                    let owner = enclosing_non_comprehension(scope);
                    if owner.kind() == "module" {
                        result
                            .module_sites
                            .entry(name.to_string())
                            .or_default()
                            .push(node);
                    } else if owner == scope {
                        nested_writes.push((scope, name.clone(), node));
                    }
                }
                if scope.kind() != "module" || value_site {
                    writes.names.entry(name).or_default().push(node);
                }
            }
            if wildcard {
                writes.wildcard.push(node);
            }
        }
        for (scope, name, node) in nested_writes {
            if result.directive(scope, &name) == Some(Directive::Global) {
                result
                    .module_sites
                    .entry(name.to_string())
                    .or_default()
                    .push(node);
                result.global_writes.entry(name).or_default().push(node);
            }
        }
        for scope in result.scopes.values_mut() {
            for writes in scope
                .names
                .values_mut()
                .chain(scope.named_imports.values_mut())
                .chain([&mut scope.wildcard])
            {
                writes.sort_by_key(|node| (node.end_byte(), node.start_byte()));
            }
        }
        result
    }

    pub(super) fn global_write(&self, name: &str) -> bool {
        self.global_writes.contains_key(name)
    }

    pub(super) fn competing_global(&self, name: &str, candidate: Node<'a>) -> bool {
        self.global_writes
            .get(name)
            .is_some_and(|sites| sites.as_slice() != [candidate])
    }

    pub(super) fn capture_sites(&self, name: &str) -> &[Node<'a>] {
        self.capture_sites.get(name).map_or(&[], Vec::as_slice)
    }

    pub(super) fn module_sites(&self, name: &str) -> &[Node<'a>] {
        self.module_sites.get(name).map_or(&[], Vec::as_slice)
    }

    pub(super) fn directive(&self, scope: Node<'a>, name: &str) -> Option<Directive> {
        self.scopes.get(&scope.id())?.directives.get(name).copied()
    }

    pub(super) fn outer_binding_scope(&self, scope: Node<'a>, name: &str) -> Option<Node<'a>> {
        if scope.kind() == "module" {
            return None;
        }
        let directive = self.directive(scope, name);
        if directive == Some(Directive::Global) {
            let mut module = scope;
            while let Some(parent) = module.parent() {
                module = parent;
            }
            return Some(module);
        }
        let mut outer = python_scope(scope);
        while outer.kind() != "module" {
            if outer.kind() != "class_definition"
                && (directive != Some(Directive::Nonlocal)
                    || (self.directive(outer, name).is_none() && self.binds_local(outer, name)))
            {
                break;
            }
            outer = python_scope(outer);
        }
        (outer.kind() != "module" || directive != Some(Directive::Nonlocal)).then_some(outer)
    }

    pub(super) fn imports(&self, scope: Node<'a>) -> &[Node<'a>] {
        self.scopes
            .get(&scope.id())
            .map_or(&[], |scope| &scope.imports)
    }

    pub(super) fn wildcard(&self, scope: Node<'a>) -> &[Node<'a>] {
        self.scopes
            .get(&scope.id())
            .map_or(&[], |scope| &scope.wildcard)
    }

    pub(super) fn reaching(
        &self,
        scope: Node<'a>,
        name: &str,
        offset: usize,
        mut may_supply: impl FnMut(Node<'a>) -> bool,
    ) -> Option<Node<'a>> {
        let named = self.named_before(scope, name, offset);
        let stars = self.wildcard(scope);
        let end = stars.partition_point(|star| star.end_byte() <= offset);
        stars[..end]
            .iter()
            .rev()
            .copied()
            .take_while(|star| named.is_none_or(|write| star.start_byte() > write.start_byte()))
            .find(|&star| may_supply(star))
            .or(named)
    }

    pub(super) fn binds_local(&self, scope: Node<'a>, name: &str) -> bool {
        self.scopes.get(&scope.id()).is_some_and(|writes| {
            writes.names.contains_key(name) || writes.parameters.contains(name)
        })
    }

    pub(super) fn named_before(
        &self,
        scope: Node<'a>,
        name: &str,
        offset: usize,
    ) -> Option<Node<'a>> {
        before(self.scopes.get(&scope.id())?.names.get(name)?, offset)
    }

    pub(super) fn import_before(
        &self,
        scope: Node<'a>,
        name: &str,
        offset: usize,
    ) -> Option<Node<'a>> {
        before(
            self.scopes.get(&scope.id())?.named_imports.get(name)?,
            offset,
        )
    }

    pub(super) fn names(&self, scope: Node<'a>) -> impl Iterator<Item = &str> + '_ {
        self.scopes
            .get(&scope.id())
            .into_iter()
            .flat_map(|scope| scope.names.keys().map(Cow::as_ref))
    }
}

pub(super) fn targets(node: Node<'_>) -> Vec<Node<'_>> {
    match node.kind() {
        "identifier" | "attribute" | "subscript" => vec![node],
        "pattern_list"
        | "expression_list"
        | "tuple"
        | "list"
        | "tuple_pattern"
        | "list_pattern"
        | "list_splat_pattern"
        | "dictionary_splat_pattern"
        | "as_pattern_target"
        | "parenthesized_expression" => children(node).into_iter().flat_map(targets).collect(),
        "default_parameter" | "typed_default_parameter" | "typed_parameter" => {
            node.named_child(0).map_or_else(Vec::new, targets)
        }
        _ => vec![],
    }
}

pub(super) fn write_targets(node: Node<'_>) -> Vec<Node<'_>> {
    match node.kind() {
        "assignment" | "augmented_assignment" | "for_statement" | "for_in_clause" => node
            .child_by_field_name("left")
            .map_or_else(Vec::new, targets),
        "named_expression" => node
            .child_by_field_name("name")
            .map_or_else(Vec::new, targets),
        "as_pattern" | "except_clause" => node
            .child_by_field_name("alias")
            .map_or_else(Vec::new, targets),
        "except_group_clause" => node.named_child(1).map_or_else(Vec::new, targets),
        "delete_statement" | "global_statement" | "nonlocal_statement" => {
            children(node).into_iter().flat_map(targets).collect()
        }
        _ => vec![],
    }
}

fn written_names<'a>(file: &'a ParsedFile, node: Node<'a>) -> (Vec<&'a str>, bool) {
    let names = match node.kind() {
        "function_definition" | "class_definition" => {
            node.child_by_field_name("name").into_iter().collect()
        }
        "parameters" | "lambda_parameters" => {
            children(node).into_iter().flat_map(targets).collect()
        }
        "type_alias_statement" => {
            let name = node
                .child_by_field_name("left")
                .and_then(|left| left.named_child(0));
            name.and_then(|name| {
                if name.kind() == "generic_type" {
                    name.named_child(0)
                } else {
                    Some(name)
                }
            })
            .into_iter()
            .collect()
        }
        "future_import_statement" | "import_from_statement" | "import_statement" => {
            let names = import_items(node)
                .map(|(path, alias)| imported_as(file, path, alias))
                .collect();
            let wildcard = children(node)
                .iter()
                .any(|child| child.kind() == "wildcard_import");
            return (names, wildcard);
        }
        "case_pattern"
            if node
                .parent()
                .is_some_and(|parent| parent.kind() == "case_clause") =>
        {
            match_captures(node)
        }
        _ => write_targets(node),
    };
    (
        names
            .into_iter()
            .filter(|id| id.kind() == "identifier")
            .map(|id| file.node_text(&id))
            .collect(),
        false,
    )
}

fn match_captures(node: Node<'_>) -> Vec<Node<'_>> {
    match node.kind() {
        "dotted_name" if node.named_child_count() == 1 => children(node),
        "as_pattern" | "splat_pattern" => children(node)
            .into_iter()
            .flat_map(|child| {
                if child.kind() == "identifier" {
                    vec![child]
                } else {
                    match_captures(child)
                }
            })
            .collect(),
        "class_pattern" | "keyword_pattern" => children(node)
            .into_iter()
            .skip(1)
            .flat_map(match_captures)
            .collect(),
        "dict_pattern" => {
            let mut cursor = node.walk();
            node.children_by_field_name("value", &mut cursor)
                .flat_map(match_captures)
                .chain(
                    children(node)
                        .into_iter()
                        .filter(|child| child.kind() == "splat_pattern")
                        .flat_map(match_captures),
                )
                .collect()
        }
        "case_pattern" | "union_pattern" | "list_pattern" | "tuple_pattern" => children(node)
            .into_iter()
            .flat_map(match_captures)
            .collect(),
        _ => vec![],
    }
}
