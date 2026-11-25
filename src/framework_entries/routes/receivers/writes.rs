use super::{children, import_parts, python_scope, ParsedFile};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

#[derive(Default)]
pub(super) struct LexicalWrites<'a> {
    scopes: BTreeMap<usize, ScopeWrites<'a>>,
    uncertain_globals: BTreeSet<&'a str>,
}

#[derive(Default)]
struct ScopeWrites<'a> {
    names: BTreeMap<&'a str, Vec<Node<'a>>>,
    wildcard: Vec<Node<'a>>,
}

impl<'a> LexicalWrites<'a> {
    pub(super) fn index(file: &'a ParsedFile, nodes: &[Node<'a>]) -> Self {
        let mut result = Self::default();
        let mut globals = BTreeSet::new();
        let mut nested_writes = BTreeSet::new();
        for &node in nodes {
            let (names, wildcard) = written_names(file, node);
            if names.is_empty() && !wildcard {
                continue;
            }
            let scope = python_scope(node);
            let writes = result.scopes.entry(scope.id()).or_default();
            for name in names {
                if node.kind() == "global_statement" {
                    globals.insert((scope.id(), name));
                } else if matches!(scope.kind(), "function_definition" | "class_definition") {
                    nested_writes.insert((scope.id(), name));
                }
                writes.names.entry(name).or_default().push(node);
            }
            if wildcard {
                writes.wildcard.push(node);
            }
        }
        result.uncertain_globals = globals
            .intersection(&nested_writes)
            .map(|&(_, name)| name)
            .collect();
        for scope in result.scopes.values_mut() {
            for writes in scope.names.values_mut().chain([&mut scope.wildcard]) {
                writes.sort_by_key(|node| (node.end_byte(), node.start_byte()));
            }
        }
        result
    }

    pub(super) fn uncertain_global(&self, name: &str) -> bool {
        self.uncertain_globals.contains(name)
    }

    pub(super) fn reaching(&self, scope: Node<'a>, name: &str, before: usize) -> Option<Node<'a>> {
        let scope = self.scopes.get(&scope.id())?;
        let previous = |writes: &[Node<'a>]| {
            let end = writes.partition_point(|node| node.end_byte() <= before);
            end.checked_sub(1).map(|i| writes[i])
        };
        let named = scope.names.get(name).and_then(|writes| previous(writes));
        [named, previous(&scope.wildcard)]
            .into_iter()
            .flatten()
            .max_by_key(|node| node.start_byte())
    }

    pub(super) fn names(&self, scope: Node<'a>) -> impl Iterator<Item = &'a str> + '_ {
        self.scopes
            .get(&scope.id())
            .into_iter()
            .flat_map(|scope| scope.names.keys().copied())
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
        "as_pattern" => node
            .child_by_field_name("alias")
            .map_or_else(Vec::new, targets),
        "delete_statement" | "global_statement" | "nonlocal_statement" => {
            children(node).into_iter().flat_map(targets).collect()
        }
        _ => vec![],
    }
}

fn written_names<'a>(file: &'a ParsedFile, node: Node<'a>) -> (Vec<&'a str>, bool) {
    let mut wildcard = false;
    let names = match node.kind() {
        "function_definition" | "class_definition" => {
            node.child_by_field_name("name").into_iter().collect()
        }
        "import_from_statement" | "import_statement" => {
            let names = children(node)
                .into_iter()
                .filter(|child| Some(*child) != node.child_by_field_name("module_name"))
                .filter_map(|child| {
                    wildcard |= child.kind() == "wildcard_import";
                    let (path, alias) = import_parts(child)?;
                    Some(file.node_text(&alias.unwrap_or(path)).split('.').next()?)
                })
                .collect();
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
        wildcard,
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
