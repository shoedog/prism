//! Dependency-identity verification for `dependency_hint::resolve`.
//!
//! `dependency_hint.rs` decides WHICH call a target is about and what its
//! source-verbatim `callee` is; this module decides whether that callee's root
//! provably names a real, single-purpose external library — the only condition
//! under which `kind` may be emitted at all (see that module's header for why
//! a wrong `kind` is worse than none). Split out to keep both files under the
//! repository's 600-line convention.

use super::dependency_hint::SiteContext;
use crate::ast::ParsedFile;
use crate::languages::Language;
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
    if binds_name_locally(parsed, at, root) {
        return None;
    }
    let Some(module) = imported_module(parsed, root) else {
        // Not imported: only a language builtin can still be the real thing.
        return rest
            .is_empty()
            .then(|| builtin_kind(parsed.language, root))?;
    };
    if is_local_module_path(parsed.language, &module) {
        return None;
    }
    if parsed.language == Language::Python && repo_has_local_module(ctx, module_root(&module)) {
        return None;
    }
    library_kind(parsed.language, &module, rest)
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

fn module_root(module: &str) -> &str {
    module.split('.').next().unwrap_or(module)
}

/// The module path `name` is bound to by an import in this file, if any.
fn imported_module(parsed: &ParsedFile, name: &str) -> Option<String> {
    if parsed.language == Language::Python {
        return python_imported_module(parsed, name);
    }
    // Go (`alias` → import path) and JS/TS (binding → specifier, ES6 and
    // `require` alike) are already exact in `extract_imports`.
    parsed.extract_imports().get(name).cloned()
}

/// Python's own binding rule, which `ParsedFile::extract_imports` does not
/// model: `import a.b` binds `a` (not `b`), `import a.b as c` binds `c` to
/// `a.b`, and `from a import b` binds `b` to `a.b`.
fn python_imported_module(parsed: &ParsedFile, name: &str) -> Option<String> {
    fn walk(parsed: &ParsedFile, node: Node<'_>, name: &str) -> Option<String> {
        match node.kind() {
            "import_statement" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    match child.kind() {
                        "dotted_name" => {
                            let path = parsed.node_text(&child);
                            if path.split('.').next() == Some(name) {
                                return Some(path.to_string());
                            }
                        }
                        "aliased_import" => {
                            let module = child.child_by_field_name("name")?;
                            let alias = child.child_by_field_name("alias")?;
                            if parsed.node_text(&alias) == name {
                                return Some(parsed.node_text(&module).to_string());
                            }
                        }
                        _ => {}
                    }
                }
                None
            }
            "import_from_statement" => {
                let module_node = node.child_by_field_name("module_name")?;
                let module = parsed.node_text(&module_node).to_string();
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.start_byte() == module_node.start_byte() {
                        continue;
                    }
                    match child.kind() {
                        "dotted_name" | "identifier" if parsed.node_text(&child) == name => {
                            return Some(format!("{module}.{name}"));
                        }
                        "aliased_import" => {
                            let imported = child.child_by_field_name("name")?;
                            let alias = child.child_by_field_name("alias")?;
                            if parsed.node_text(&alias) == name {
                                return Some(format!("{module}.{}", parsed.node_text(&imported)));
                            }
                        }
                        _ => {}
                    }
                }
                None
            }
            _ => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if let Some(found) = walk(parsed, child, name) {
                        return Some(found);
                    }
                }
                None
            }
        }
    }
    walk(parsed, parsed.tree.root_node(), name)
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
        for suffix in [format!("{root}.py"), format!("{root}/__init__.py")] {
            let candidate = if directory.is_empty() {
                suffix
            } else {
                format!("{directory}/{suffix}")
            };
            if ctx.known_files.contains_key(&candidate) || ctx.repo_root.join(&candidate).is_file()
            {
                return true;
            }
        }
        match directory.rsplit_once('/') {
            Some((parent, _)) => directory = parent,
            None if directory.is_empty() => return false,
            None => directory = "",
        }
    }
}

/// A binding of `name` — parameter, assignment, `def`/`class`, loop or `as`
/// target — in the function enclosing `at`, or at file scope. Import
/// statements are deliberately NOT bindings here: they are what
/// `imported_module` just proved, and counting them would refuse every
/// correctly-imported library.
fn binds_name_locally(parsed: &ParsedFile, at: Node<'_>, name: &str) -> bool {
    if let Some(function) = enclosing_function_node(parsed, at) {
        if function_parameters_bind(parsed, &function, name)
            || scope_binds(parsed, function, function, name)
        {
            return true;
        }
    }
    let root = parsed.tree.root_node();
    scope_binds(parsed, root, root, name)
}

pub(super) fn enclosing_function_node<'a>(parsed: &ParsedFile, node: Node<'a>) -> Option<Node<'a>> {
    let kinds = parsed.language.function_node_types();
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
fn builtin_kind(language: Language, root: &str) -> Option<&'static str> {
    match (language, root) {
        (Language::Python, "open") => Some("filesystem"),
        (Language::JavaScript | Language::TypeScript | Language::Tsx, "fetch") => Some("http"),
        _ => None,
    }
}

/// Look the library up by the identity its import established, not by the
/// spelling of the local binding: `import requests as rq` makes `rq.post`
/// `requests.post`.
fn library_kind(language: Language, module: &str, rest: &[&str]) -> Option<&'static str> {
    match language {
        Language::Python => {
            let mut path = module.to_string();
            for segment in rest {
                path.push('.');
                path.push_str(segment);
            }
            longest_prefix_kind(PYTHON_LIBRARIES, &path, '.')
        }
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
