//! Dependency-identity verification for `dependency_hint::resolve`.
//!
//! `dependency_hint.rs` decides WHICH call a target is about and what its
//! source-verbatim `callee` is. This module verifies import paths and looks up
//! the legacy `kind` for proven bindings.

use super::dependency_hint::SiteContext;
use prism::ast::ParsedFile;
use prism::languages::Language;
use std::borrow::Cow;
use tree_sitter::Node;

/// The kind of the library `root` names at `at`, if `root` provably names an
/// imported, unshadowed, single-purpose library.
pub(super) fn kind_for_binding(
    parsed: &ParsedFile,
    ctx: &SiteContext<'_>,
    at: Node<'_>,
    root: &str,
    rest: &[&str],
) -> Option<&'static str> {
    if parsed.language == Language::Python {
        return match python_root_binding(parsed, ctx, at, root, rest).0 {
            PythonBinding::External(path) => python_kind_for_path(&path),
            PythonBinding::NoDecidingImport if rest.is_empty() => {
                builtin_kind(parsed.language, root)
            }
            _ => None,
        };
    }
    if binds_name_locally(parsed, at, root) {
        return None;
    }
    let Some(module) = imported_module(parsed, at, root) else {
        // Not imported: only a language builtin can still be the real thing.
        return rest
            .is_empty()
            .then(|| builtin_kind(parsed.language, root))?;
    };
    if is_local_module_path(parsed.language, &module) {
        return None;
    }
    library_kind(parsed.language, &module)
}

/// A module path that names something inside this repository rather than an
/// external library. Python relative imports (`from . import x`) and JS
/// relative/absolute specifiers (`./client`, `../client`, `/client`) can never
/// name a catalogued package.
fn is_local_module_path(language: Language, module: &str) -> bool {
    match language {
        Language::Python => module.starts_with('.'),
        Language::JavaScript | Language::TypeScript | Language::Tsx => {
            module.starts_with('.') || module.starts_with('/')
        }
        // Go import paths are absolute; a repo-local package is excluded by the
        // table being keyed on the library's own import path (`net/http`),
        // which a module-qualified local package can never equal.
        _ => false,
    }
}

pub(super) fn module_root(module: &str) -> &str {
    module.split('.').next().unwrap_or(module)
}

/// The non-Python module path `name` is bound to by an import visible at `at`.
fn imported_module(parsed: &ParsedFile, at: Node<'_>, name: &str) -> Option<String> {
    if matches!(
        parsed.language,
        Language::JavaScript | Language::TypeScript | Language::Tsx
    ) {
        return js_imported_module(parsed, at, name);
    }
    // Go imports are necessarily file-scoped and exact in `extract_imports`.
    parsed.extract_imports().get(name).cloned()
}

pub(super) enum PythonBinding {
    External(String),
    NoDecidingImport,
    Refused,
}

pub(super) fn python_root_binding(
    parsed: &ParsedFile,
    ctx: &SiteContext<'_>,
    at: Node<'_>,
    root: &str,
    rest: &[&str],
) -> (PythonBinding, bool) {
    let catalog_spelling = PYTHON_LIBRARIES
        .iter()
        .any(|(key, _)| module_root(key) == root);
    let shadowed = binds_name_locally(parsed, at, root);
    if shadowed && catalog_spelling {
        return (PythonBinding::Refused, true);
    }
    let (import, import_spelling) = python_imported_module(parsed, at, root, rest, shadowed);
    let binding = if shadowed {
        PythonBinding::Refused
    } else if let Some((module, consumed_rest)) = import {
        python_external_path(ctx, &module, &rest[consumed_rest..])
            .map_or(PythonBinding::Refused, PythonBinding::External)
    } else {
        PythonBinding::NoDecidingImport
    };
    (binding, catalog_spelling || import_spelling)
}

/// Construct the path for an already-decided import and the remaining callee
/// chain. Relative and repository-local modules cannot prove external identity.
pub(super) fn python_external_path(
    ctx: &SiteContext<'_>,
    module: &str,
    rest: &[&str],
) -> Option<String> {
    if is_local_module_path(Language::Python, module)
        || repo_has_local_module(ctx, module_root(module))
    {
        return None;
    }
    Some(python_dotted_path(module, rest.iter().copied()))
}

pub(super) fn python_dotted_path<'a>(
    module: &str,
    rest: impl IntoIterator<Item = &'a str>,
) -> String {
    let mut path = module.to_string();
    for segment in rest {
        path.push('.');
        path.push_str(segment);
    }
    path
}

fn binding_scope<'a>(parsed: &'a ParsedFile, binding: Node<'a>, at: Node<'_>) -> Option<Node<'a>> {
    let scope = enclosing_function_node(parsed, binding).unwrap_or_else(|| parsed.tree.root_node());
    (scope.start_byte() <= at.start_byte() && at.end_byte() <= scope.end_byte()).then_some(scope)
}

fn binding_reaches_site(
    parsed: &ParsedFile,
    binding: Node<'_>,
    at: Node<'_>,
    name: &str,
) -> Option<usize> {
    let scope = binding_scope(parsed, binding, at)?;
    parsed
        .find_variable_references_scoped(&scope, name, binding.start_position().row + 1)
        .contains(&(at.start_position().row + 1))
        .then(|| scope_depth(scope))
}

fn scope_depth(mut node: Node<'_>) -> usize {
    let mut depth = 0;
    while let Some(parent) = node.parent() {
        depth += 1;
        node = parent;
    }
    depth
}

/// Source decomposition for one binding in a Python import statement.
pub(super) struct PythonImport<'a, 'tree> {
    pub(super) item: Node<'tree>,
    pub(super) from: Option<Node<'tree>>,
    pub(super) name: &'a str,
    pub(super) module: Cow<'a, str>,
    pub(super) required_rest: Option<&'a str>,
}

impl PythonImport<'_, '_> {
    fn consumed_rest(&self, rest: &[&str]) -> Option<usize> {
        let mut consumed = 0;
        if let Some(required) = self.required_rest {
            for segment in required.split('.') {
                if rest.get(consumed) != Some(&segment) {
                    return None;
                }
                consumed += 1;
            }
        }
        Some(consumed)
    }
}

/// Decompose a statement so callers can apply their own binding admission.
pub(super) fn python_imports<'a, 'tree>(
    parsed: &'a ParsedFile,
    node: Node<'tree>,
) -> Vec<PythonImport<'a, 'tree>> {
    let from = match node.kind() {
        "import_statement" => None,
        "import_from_statement" => match node.child_by_field_name("module_name") {
            Some(module) => Some(module),
            None => return Vec::new(),
        },
        _ => return Vec::new(),
    };
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .filter(|child| from.is_none_or(|module| child.start_byte() != module.start_byte()))
        .filter_map(|child| {
            let (name, imported, required_rest) = match child.kind() {
                "aliased_import" => (
                    parsed.node_text(&child.child_by_field_name("alias")?),
                    parsed.node_text(&child.child_by_field_name("name")?),
                    None,
                ),
                "dotted_name" if from.is_none() => {
                    let imported = parsed.node_text(&child);
                    match imported.split_once('.') {
                        Some((name, tail)) => (name, imported, Some(tail)),
                        None => (imported, imported, None),
                    }
                }
                "dotted_name" | "identifier" if from.is_some() => {
                    let imported = parsed.node_text(&child);
                    (imported, imported, None)
                }
                _ => return None,
            };
            let module = match from {
                Some(module) => Cow::Owned(format!("{}.{imported}", parsed.node_text(&module))),
                None => Cow::Borrowed(imported),
            };
            Some(PythonImport {
                item: child,
                from,
                name,
                module,
                required_rest,
            })
        })
        .collect()
}

/// The legacy binder requires an unaliased dotted import's tail to match the
/// call chain, then accepts a unique binding in the nearest visible scope.
fn python_imported_module<'a>(
    parsed: &'a ParsedFile,
    at: Node<'_>,
    name: &str,
    rest: &[&str],
    shadowed: bool,
) -> (Option<(Cow<'a, str>, usize)>, bool) {
    fn visit_imports<'tree>(node: Node<'tree>, visit: &mut impl FnMut(Node<'tree>)) {
        if matches!(node.kind(), "import_statement" | "import_from_statement") {
            visit(node);
            return;
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            visit_imports(child, visit);
        }
    }

    let mut spelling = false;
    let mut bindings = Vec::new();
    visit_imports(parsed.tree.root_node(), &mut |node| {
        for import in python_imports(parsed, node) {
            // Spelling evidence only requires a lexically visible module name.
            if !spelling && module_root(&import.module) == name {
                spelling = binding_scope(parsed, node, at).is_some();
            }
            if shadowed || import.name != name {
                continue;
            }
            if let Some(consumed) = import.consumed_rest(rest) {
                if let Some(depth) = binding_reaches_site(parsed, node, at, name) {
                    bindings.push((import.module, consumed, depth));
                }
            }
        }
    });
    let max_depth = bindings.iter().map(|(_, _, depth)| *depth).max();
    let mut nearest = bindings
        .into_iter()
        .filter(|(_, _, depth)| Some(*depth) == max_depth);
    let selected = nearest
        .next()
        .filter(|_| nearest.next().is_none())
        .map(|(module, consumed, _)| (module, consumed));
    (selected, spelling)
}

/// JS/TS imports and CommonJS requires are parsed by Prism's existing import
/// extractor one declaration at a time, then filtered by the scoped binder.
fn js_imported_module(parsed: &ParsedFile, at: Node<'_>, name: &str) -> Option<String> {
    fn walk(
        parsed: &ParsedFile,
        node: Node<'_>,
        at: Node<'_>,
        name: &str,
        out: &mut Vec<(String, usize)>,
    ) {
        if matches!(
            node.kind(),
            "import_statement" | "lexical_declaration" | "variable_declaration"
        ) {
            if let Ok(fragment) =
                ParsedFile::parse("binding.js", parsed.node_text(&node), parsed.language)
            {
                if let Some(module) = fragment.extract_imports().get(name) {
                    if let Some(depth) = binding_reaches_site(parsed, node, at, name) {
                        out.push((module.clone(), depth));
                    }
                }
            }
            return;
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            walk(parsed, child, at, name, out);
        }
    }
    let mut bindings = Vec::new();
    walk(parsed, parsed.tree.root_node(), at, name, &mut bindings);
    let max_depth = bindings.iter().map(|(_, depth)| *depth).max()?;
    let mut nearest = bindings
        .into_iter()
        .filter(|(_, depth)| *depth == max_depth);
    let binding = nearest.next()?;
    nearest.next().is_none().then_some(binding.0)
}

/// Does the repository ship its own module named `root`? Python resolves
/// `import requests` against the source tree before site-packages, so a
/// sibling `requests.py` or `requests/__init__.py` under any ancestor
/// directory of the site's file makes the import name a repo-local module.
fn repo_has_local_module(ctx: &SiteContext<'_>, root: &str) -> bool {
    if root.is_empty() {
        return false;
    }
    let mut directory = ctx.file.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");
    loop {
        if python_module_paths(directory, root)
            .iter()
            .any(|candidate| {
                ctx.known_files.contains_key(candidate) || ctx.repo_root.join(candidate).is_file()
            })
        {
            return true;
        }
        match directory.rsplit_once('/') {
            Some((parent, _)) => directory = parent,
            None if directory.is_empty() => return false,
            None => directory = "",
        }
    }
}

pub(super) fn python_module_paths(directory: &str, root: &str) -> [String; 2] {
    let paths = [format!("{root}.py"), format!("{root}/__init__.py")];
    if directory.is_empty() {
        paths
    } else {
        paths.map(|suffix| format!("{directory}/{suffix}"))
    }
}

/// A binding of `name` — parameter, assignment, `def`/`class`, loop or `as`
/// target — in the function enclosing `at`, or at file scope. Import
/// statements are deliberately NOT bindings here: they are what
/// `imported_module` just proved, and counting them would refuse every
/// correctly-imported library.
fn binds_name_locally(parsed: &ParsedFile, at: Node<'_>, name: &str) -> bool {
    let mut current = Some(at);
    while let Some(function) = current.and_then(|node| enclosing_function_node(parsed, node)) {
        if function_parameters_bind(parsed, &function, name)
            || scope_binds(parsed, function, function, name)
        {
            return true;
        }
        current = function.parent();
    }
    let root = parsed.tree.root_node();
    scope_binds(parsed, root, root, name)
}

pub(super) fn enclosing_function_node<'a>(parsed: &ParsedFile, node: Node<'a>) -> Option<Node<'a>> {
    let kinds = parsed.language.callable_boundary_node_types();
    let mut current = Some(node);
    while let Some(candidate) = current {
        if kinds.contains(&candidate.kind()) {
            return Some(candidate);
        }
        current = candidate.parent();
    }
    None
}

/// Every identifier in the parameter list binds. Over-collecting (a type
/// annotation's identifiers, say) only ever drops a `kind`, never invents one.
fn function_parameters_bind(parsed: &ParsedFile, function: &Node<'_>, name: &str) -> bool {
    let Some(parameters) = function
        .child_by_field_name("parameters")
        .or_else(|| function.child_by_field_name("parameter"))
    else {
        return false;
    };
    fn contains(parsed: &ParsedFile, node: Node<'_>, name: &str) -> bool {
        if node.kind() == "identifier" && parsed.node_text(&node) == name {
            return true;
        }
        let mut cursor = node.walk();
        let children: Vec<Node<'_>> = node.children(&mut cursor).collect();
        children
            .into_iter()
            .any(|child| contains(parsed, child, name))
    }
    contains(parsed, parameters, name)
}

/// Walk `node` for a binding of `name`, without descending into nested
/// function or class bodies — those are their own scopes, and only the name
/// they declare binds here.
fn scope_binds(parsed: &ParsedFile, node: Node<'_>, root: Node<'_>, name: &str) -> bool {
    let is_root = node.start_byte() == root.start_byte() && node.kind() == root.kind();
    if !is_root && is_scope_definition(node.kind()) {
        return declared_name(parsed, &node).as_deref() == Some(name);
    }
    if !is_root && declares_binding(parsed, node, name) {
        return true;
    }
    let mut cursor = node.walk();
    let children: Vec<Node<'_>> = node.children(&mut cursor).collect();
    children
        .into_iter()
        .any(|child| scope_binds(parsed, child, root, name))
}

fn is_scope_definition(kind: &str) -> bool {
    matches!(
        kind,
        "function_definition"
            | "class_definition"
            | "function_declaration"
            | "method_declaration"
            | "class_declaration"
            | "method_definition"
            | "lambda"
            | "func_literal"
            | "arrow_function"
            | "function_expression"
    )
}

fn declared_name(parsed: &ParsedFile, node: &Node<'_>) -> Option<String> {
    node.child_by_field_name("name")
        .map(|name| parsed.node_text(&name).to_string())
}

/// One statement's own binding effect on `name`.
fn declares_binding(parsed: &ParsedFile, node: Node<'_>, name: &str) -> bool {
    let language = parsed.language;
    let bound = |target: Option<Node<'_>>| -> bool {
        target.is_some_and(|target| pattern_binds(parsed, target, name))
    };
    match (language, node.kind()) {
        (Language::Python, "assignment" | "augmented_assignment") => {
            bound(node.child_by_field_name("left"))
        }
        (Language::Python, "for_statement" | "for_in_clause") => {
            bound(node.child_by_field_name("left"))
        }
        (Language::Python, "named_expression") => bound(node.child_by_field_name("name")),
        (Language::Python, "as_pattern") => bound(node.child_by_field_name("alias")),
        (Language::Go, "short_var_declaration" | "range_clause" | "assignment_statement") => {
            bound(node.child_by_field_name("left"))
        }
        (Language::Go, "var_spec" | "const_spec") => bound(node.child_by_field_name("name")),
        (Language::JavaScript | Language::TypeScript | Language::Tsx, "variable_declarator") => {
            // `const axios = require('axios')` is an import, not a shadow.
            let is_require = node
                .child_by_field_name("value")
                .and_then(|value| value.child_by_field_name("function"))
                .is_some_and(|function| parsed.node_text(&function) == "require");
            !is_require && bound(node.child_by_field_name("name"))
        }
        _ => false,
    }
}

fn pattern_binds(parsed: &ParsedFile, node: Node<'_>, name: &str) -> bool {
    match node.kind() {
        "identifier" => parsed.node_text(&node) == name,
        "pattern_list"
        | "tuple_pattern"
        | "list_pattern"
        | "expression_list"
        | "parenthesized_expression"
        | "array_pattern"
        | "object_pattern" => {
            let mut cursor = node.walk();
            let children: Vec<Node<'_>> = node.named_children(&mut cursor).collect();
            children
                .into_iter()
                .any(|child| pattern_binds(parsed, child, name))
        }
        _ => false,
    }
}

/// Language builtins that are genuinely single-purpose external-call targets
/// and have no import to verify. Still subject to the shadow check.
pub(super) fn builtin_kind(language: Language, root: &str) -> Option<&'static str> {
    match (language, root) {
        (Language::Python, "open") => Some("filesystem"),
        (Language::JavaScript | Language::TypeScript | Language::Tsx, "fetch") => Some("http"),
        _ => None,
    }
}

pub(super) fn python_kind_for_path(path: &str) -> Option<&'static str> {
    longest_prefix_kind(PYTHON_LIBRARIES, path, '.')
}

fn library_kind(language: Language, module: &str) -> Option<&'static str> {
    match language {
        Language::Go => longest_prefix_kind(GO_LIBRARIES, module, '/'),
        Language::JavaScript | Language::TypeScript | Language::Tsx => longest_prefix_kind(
            JS_LIBRARIES,
            module.strip_prefix("node:").unwrap_or(module),
            '/',
        ),
        _ => None,
    }
}

/// Longest table key that `path` equals or descends from, on `separator`
/// boundaries — so `urllib.request` beats nothing while `urllib.parse` matches
/// nothing, and `fs/promises` still reaches `fs`.
fn longest_prefix_kind(
    table: &[(&'static str, &'static str)],
    path: &str,
    separator: char,
) -> Option<&'static str> {
    table
        .iter()
        .filter(|(key, _)| {
            path == *key
                || (path.len() > key.len()
                    && path.starts_with(*key)
                    && path[key.len()..].starts_with(separator))
        })
        .max_by_key(|(key, _)| key.len())
        .map(|(_, kind)| *kind)
}

/// Python module paths → harness `kind`. Every entry is single-purpose: a
/// library whose operations span kinds (`redis` is cache AND queue; bare `os`
/// is filesystem, process AND environment; `urllib` is http AND pure parsing)
/// is either absent or keyed at the submodule that is not.
const PYTHON_LIBRARIES: &[(&str, &str)] = &[
    ("requests", "http"),
    ("httpx", "http"),
    ("urllib3", "http"),
    ("aiohttp", "http"),
    ("urllib.request", "http"),
    ("psycopg2", "db"),
    ("sqlalchemy", "db"),
    ("pymysql", "db"),
    ("sqlite3", "db"),
    ("asyncpg", "db"),
    ("pymongo", "db"),
    ("kombu", "queue"),
    ("pika", "queue"),
    ("celery", "queue"),
    ("confluent_kafka", "queue"),
    ("aiokafka", "queue"),
    ("pathlib", "filesystem"),
    ("shutil", "filesystem"),
    ("subprocess", "process"),
    ("os.system", "process"),
    ("time.sleep", "clock"),
];

/// Go import paths → harness `kind`. Keyed on the import path (what the task's
/// own table named) rather than the package identifier, so a repo-local
/// package that happens to be called `http` cannot claim `net/http`'s kind.
const GO_LIBRARIES: &[(&str, &str)] = &[
    ("net/http", "http"),
    ("database/sql", "db"),
    ("github.com/jackc/pgx", "db"),
    ("gorm.io/gorm", "db"),
    ("github.com/Shopify/sarama", "queue"),
    ("github.com/IBM/sarama", "queue"),
    ("github.com/segmentio/kafka-go", "queue"),
    ("github.com/streadway/amqp", "queue"),
    ("github.com/rabbitmq/amqp091-go", "queue"),
    ("os/exec", "process"),
];

/// JS/TS module specifiers → harness `kind` (`node:` prefix already stripped).
const JS_LIBRARIES: &[(&str, &str)] = &[
    ("axios", "http"),
    ("got", "http"),
    ("node-fetch", "http"),
    ("http", "http"),
    ("https", "http"),
    ("pg", "db"),
    ("mysql2", "db"),
    ("knex", "db"),
    ("@prisma/client", "db"),
    ("mongoose", "db"),
    ("amqplib", "queue"),
    ("kafkajs", "queue"),
    ("bullmq", "queue"),
    ("fs", "filesystem"),
    ("child_process", "process"),
];
