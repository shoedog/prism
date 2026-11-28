use super::{
    certificate::admitted_kind,
    children, comprehension, direct,
    imports::{import_items, imported_as},
    normalized, python_scope, ParsedFile,
};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

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
    /// Binding occurrences in a nested scope that declares `nonlocal K`, keyed by the
    /// enclosing function scope they redirect to.
    nonlocal_writes: BTreeMap<(usize, Cow<'a, str>), Vec<Node<'a>>>,
}

#[derive(Default)]
struct ScopeWrites<'a> {
    names: BTreeMap<Cow<'a, str>, Vec<Node<'a>>>,
    wildcard: Vec<Node<'a>>,
    imports: Vec<Node<'a>>,
    named_imports: BTreeMap<Cow<'a, str>, Vec<Node<'a>>>,
    directives: BTreeMap<Cow<'a, str>, Directive>,
    parameters: BTreeSet<Cow<'a, str>>,
    /// Names a bare annotation (`x: T`) declares: lexical ownership of the name in a
    /// function, no value written, so never a reaching write and never a binding occurrence.
    declared: BTreeSet<Cow<'a, str>>,
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

/// The byte from which a binding occurrence is in force. A `for` target is bound on entry to
/// each iteration, so its body reads it; every other occurrence binds when its node ends (an
/// `except .. as` target is its own `as_pattern` node, which ends before the clause body).
fn active_end(node: Node<'_>) -> usize {
    if node.kind() == "for_statement" {
        if let Some(left) = node.child_by_field_name("left") {
            return left.end_byte();
        }
    }
    node.end_byte()
}

fn before<'a>(sites: &[Node<'a>], offset: usize) -> Option<Node<'a>> {
    let end = sites.partition_point(|node| active_end(*node) <= offset);
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
                    // `global K` at module level redirects nothing: the module still binds K.
                    if scope.kind() != "module" {
                        writes.directives.insert(name, directive);
                    }
                    continue;
                }
                if import {
                    writes
                        .named_imports
                        .entry(name.clone())
                        .or_default()
                        .push(node);
                }
                if parameter {
                    writes.parameters.insert(name);
                    continue;
                }
                if value_site {
                    // A comprehension target binds the comprehension's own scope, so only
                    // a module-level occurrence (a walrus is placed in the enclosing
                    // scope above) is a module binding.
                    if scope.kind() == "module" {
                        result
                            .module_sites
                            .entry(name.to_string())
                            .or_default()
                            .push(node);
                    } else if !comprehension(scope) {
                        nested_writes.push((scope, name.clone(), node));
                    }
                }
                if value_site {
                    writes.names.entry(name).or_default().push(node);
                } else if scope.kind() == "function_definition" {
                    writes.declared.insert(name);
                }
            }
            if wildcard {
                writes.wildcard.push(node);
            }
        }
        for (scope, name, node) in nested_writes {
            match result.directive(scope, &name) {
                Some(Directive::Global) => {
                    result
                        .module_sites
                        .entry(name.to_string())
                        .or_default()
                        .push(node);
                    result.global_writes.entry(name).or_default().push(node);
                }
                Some(Directive::Nonlocal) => {
                    if let Some(target) = result.outer_binding_scope(scope, &name) {
                        result
                            .nonlocal_writes
                            .entry((target.id(), name))
                            .or_default()
                            .push(node);
                    }
                }
                None => {}
            }
        }
        for scope in result.scopes.values_mut() {
            for writes in scope
                .names
                .values_mut()
                .chain(scope.named_imports.values_mut())
                .chain([&mut scope.wildcard])
            {
                writes.sort_by_key(|node| (active_end(*node), node.start_byte()));
            }
        }
        result
    }

    /// True when a nested scope declares `global K` and binds it.
    pub(super) fn global_write(&self, name: &str) -> bool {
        self.global_writes.contains_key(name)
    }

    /// True when a nested scope declares `nonlocal K` (resolving to `scope`) and binds it.
    pub(super) fn nonlocal_write(&self, scope: Node<'a>, name: &str) -> bool {
        self.nonlocal_writes
            .contains_key(&(scope.id(), Cow::Borrowed(name)))
    }

    /// True when an import occurrence of `name` in the function scope `scope` is followed by
    /// another binding occurrence (a second alias of one statement, a second import, a later
    /// def or assignment): the import is no longer the one binding the key resolves to.
    pub(super) fn import_displaced(&self, scope: Node<'a>, name: &str) -> bool {
        self.scopes
            .get(&scope.id())
            .and_then(|writes| writes.names.get(name))
            .and_then(|sites| sites.split_last())
            .is_some_and(|(_, earlier)| {
                earlier
                    .iter()
                    .any(|site| matches!(site.kind(), "import_statement" | "import_from_statement"))
            })
    }

    pub(super) fn module_sites(&self, name: &str) -> &[Node<'a>] {
        self.module_sites.get(name).map_or(&[], Vec::as_slice)
    }

    pub(super) fn directive(&self, scope: Node<'a>, name: &str) -> Option<Directive> {
        self.scopes.get(&scope.id())?.directives.get(name).copied()
    }

    /// The scope a binding occurrence binds in: the module for a `global K` redirect, the
    /// enclosing function for a `nonlocal K` redirect, else the scope that holds it.
    pub(super) fn binding_scope_of_write(&self, node: Node<'a>, name: &str) -> Option<Node<'a>> {
        let scope = match node.kind() {
            "named_expression" => enclosing_non_comprehension(python_scope(node)),
            _ => python_scope(node),
        };
        match self.directive(scope, name) {
            Some(Directive::Global) => {
                let mut module = scope;
                while let Some(parent) = module.parent() {
                    module = parent;
                }
                Some(module)
            }
            Some(Directive::Nonlocal) => self.outer_binding_scope(scope, name),
            None => Some(scope),
        }
    }

    /// Every binding occurrence of `name` in `scope`, wherever it sits and runs (no
    /// position filter), with `global`/`nonlocal` redirects from nested scopes included.
    pub(super) fn binding_sites(&self, scope: Node<'a>, name: &str) -> Vec<Node<'a>> {
        if scope.kind() == "module" {
            return self.module_sites(name).to_vec();
        }
        let mut sites = self
            .scopes
            .get(&scope.id())
            .and_then(|writes| writes.names.get(name))
            .cloned()
            .unwrap_or_default();
        if let Some(redirected) = self.nonlocal_writes.get(&(scope.id(), Cow::Borrowed(name))) {
            sites.extend(redirected.iter().copied());
        }
        sites
    }

    /// The binding occurrences of the scope a read of `name` at `scope` resolves to. For a
    /// read in the function that binds the name, a definite write (an import, def, class or
    /// `K = <expr>` that is a direct statement of the body) before the read supersedes every
    /// occurrence before it: that write runs on every path to the read, loop back edges
    /// included, because it sits outside any loop. Later occurrences and redirected writes
    /// from nested scopes stay (a back edge or a later call can run them first).
    pub(super) fn occurrences(&self, scope: Node<'a>, name: &str, before: usize) -> Vec<Node<'a>> {
        let Some(binding) = self.read_binding_scope(scope, name, before) else {
            return Vec::new();
        };
        let mut sites = self.binding_sites(binding, name);
        if binding == scope && binding.kind() == "function_definition" {
            let cut = self
                .scopes
                .get(&binding.id())
                .and_then(|writes| writes.names.get(name))
                .and_then(|names| {
                    names.iter().copied().rfind(|site| {
                        site.end_byte() <= before && admitted_kind(*site) && direct(*site, binding)
                    })
                });
            if let Some(cut) = cut {
                let own = self.scopes[&binding.id()].names[name].len();
                let mut index = 0;
                sites.retain(|site| {
                    index += 1;
                    index > own || site.end_byte() >= cut.end_byte()
                });
            }
        }
        sites
    }

    pub(super) fn read_binding_scope(
        &self,
        mut scope: Node<'a>,
        name: &str,
        before: usize,
    ) -> Option<Node<'a>> {
        loop {
            let owns_name = match scope.kind() {
                "module" => true,
                "class_definition" => self.named_before(scope, name, before).is_some(),
                _ => self.binds_local(scope, name),
            };
            if owns_name && self.directive(scope, name).is_none() {
                return Some(scope);
            }
            scope = self.outer_binding_scope(scope, name)?;
        }
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

    /// True when `name` (an effective, class-mangled key) is a formal of `scope`.
    pub(super) fn parameter(&self, scope: Node<'a>, name: &str) -> bool {
        self.scopes
            .get(&scope.id())
            .is_some_and(|writes| writes.parameters.contains(name))
    }

    pub(super) fn binds_local(&self, scope: Node<'a>, name: &str) -> bool {
        self.scopes.get(&scope.id()).is_some_and(|writes| {
            writes.names.contains_key(name)
                || writes.parameters.contains(name)
                || writes.declared.contains(name)
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
