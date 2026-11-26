use super::{
    children,
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

fn effective_name(file: &ParsedFile, mut scope: Node<'_>, name: &str) -> String {
    if private_name(name) {
        loop {
            if scope.kind() == "class_definition" {
                if let Some(class) = scope.child_by_field_name("name") {
                    let class = normalized(file.node_text(&class));
                    let prefix = class.trim_start_matches('_');
                    if !prefix.is_empty() {
                        return format!("_{prefix}{name}");
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
    name.to_owned()
}

#[derive(Default)]
pub(super) struct LexicalWrites<'a> {
    scopes: BTreeMap<usize, ScopeWrites<'a>>,
    module_sites: BTreeMap<String, Vec<Node<'a>>>,
    global_writes: BTreeSet<String>,
}

#[derive(Default)]
struct ScopeWrites<'a> {
    names: BTreeMap<Cow<'a, str>, Vec<Node<'a>>>,
    wildcard: Vec<Node<'a>>,
    imports: Vec<Node<'a>>,
}

impl<'a> LexicalWrites<'a> {
    pub(super) fn index(file: &'a ParsedFile, nodes: &[Node<'a>]) -> Self {
        let mut result = Self::default();
        let mut globals = BTreeSet::new();
        let mut nested_writes = Vec::new();
        for &node in nodes {
            let (names, wildcard) = written_names(file, node);
            if names.is_empty() && !wildcard {
                continue;
            }
            let scope = python_scope(node);
            let writes = result.scopes.entry(scope.id()).or_default();
            if matches!(node.kind(), "import_statement" | "import_from_statement") {
                writes.imports.push(node);
            }
            let value_site = !matches!(node.kind(), "global_statement" | "nonlocal_statement")
                && (node.kind() != "assignment" || node.child_by_field_name("right").is_some());
            for name in names.into_iter().map(normalized) {
                if node.kind() == "global_statement" {
                    if scope.kind() != "module" {
                        globals.insert((scope.id(), effective_name(file, scope, &name)));
                    }
                } else if value_site {
                    if scope.kind() == "module" {
                        result
                            .module_sites
                            .entry(name.to_string())
                            .or_default()
                            .push(node);
                    } else if node.kind() != "for_in_clause"
                        && matches!(scope.kind(), "function_definition" | "class_definition")
                    {
                        nested_writes.push((scope.id(), effective_name(file, scope, &name), node));
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
            if globals.contains(&(scope, name.clone())) {
                result.global_writes.insert(name.clone());
                result.module_sites.entry(name).or_default().push(node);
            }
        }
        for scope in result.scopes.values_mut() {
            for writes in scope.names.values_mut().chain([&mut scope.wildcard]) {
                writes.sort_by_key(|node| (node.end_byte(), node.start_byte()));
            }
        }
        result
    }

    pub(super) fn global_write(&self, name: &str) -> bool {
        self.global_writes.contains(name)
    }

    pub(super) fn module_sites(&self, name: &str) -> &[Node<'a>] {
        self.module_sites.get(name).map_or(&[], Vec::as_slice)
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

    pub(super) fn reaching(&self, scope: Node<'a>, name: &str, before: usize) -> Option<Node<'a>> {
        let scope = self.scopes.get(&scope.id())?;
        let previous = |writes: &[Node<'a>]| {
            let end = writes.partition_point(|node| node.end_byte() <= before);
            end.checked_sub(1).map(|i| writes[i])
        };
        let named = scope
            .names
            .get(normalized(name).as_ref())
            .and_then(|writes| previous(writes));
        [named, previous(&scope.wildcard)]
            .into_iter()
            .flatten()
            .max_by_key(|node| node.start_byte())
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
        "import_from_statement" | "import_statement" => {
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
