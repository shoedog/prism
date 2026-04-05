//! Multi-language live type collection for Rapid Type Analysis (RTA).
//!
//! Scans tree-sitter ASTs for instantiation patterns across all supported
//! languages, producing a unified `BTreeSet<String>` of type names that are
//! actually constructed at runtime. This set is passed to `DispatchProvider`
//! implementations to prune virtual dispatch targets to only those types
//! that are live (instantiated).
//!
//! **Supported instantiation patterns:**
//! - C++: `new X()`, `make_unique<X>()`, `make_shared<X>()`, `X var;`
//! - Java: `new ClassName(...)`
//! - Go: `StructName{...}` (composite literals)
//! - TypeScript/TSX: `new ClassName(...)`
//! - Rust: `StructName { ... }` (struct expressions)
//! - Python: `ClassName(...)` (calls matching known class names)

use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

/// Collect all live (instantiated) types from parsed files across all languages.
///
/// This is the generalized version of `TypeDatabase::collect_live_classes()`,
/// extended to cover Go, Java, TypeScript, Rust, and Python in addition to C++.
///
/// The `known_classes` set (optional) is used for languages like Python where
/// function calls and class instantiations look the same syntactically — only
/// calls to names in `known_classes` are counted as instantiations.
pub fn collect_live_types(
    files: &BTreeMap<String, ParsedFile>,
    known_classes: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut live = BTreeSet::new();

    for parsed in files.values() {
        match parsed.language {
            Language::Java => scan_java(parsed, &mut live),
            Language::Go => scan_go(parsed, &mut live),
            Language::TypeScript | Language::Tsx => scan_typescript(parsed, &mut live),
            Language::Rust => scan_rust(parsed, &mut live),
            Language::Python => scan_python(parsed, &mut live, known_classes),
            Language::Cpp => scan_cpp(parsed, &mut live),
            Language::JavaScript => scan_javascript(parsed, &mut live),
            // Lua, Terraform, Bash: no type system, no instantiations to track.
            _ => {}
        }
    }

    live
}

// ---------------------------------------------------------------------------
// C++ instantiation scanning
// ---------------------------------------------------------------------------

/// Scan C++ files for instantiation patterns.
///
/// Patterns: `new X(...)`, `make_unique<X>(...)`, `make_shared<X>(...)`,
/// `ClassName var;` (stack allocation).
fn scan_cpp(parsed: &ParsedFile, live: &mut BTreeSet<String>) {
    scan_tree_recursive(parsed.tree.root_node(), parsed, live, &scan_cpp_node);
}

fn scan_cpp_node(node: &tree_sitter::Node, parsed: &ParsedFile, live: &mut BTreeSet<String>) {
    match node.kind() {
        "new_expression" => {
            if let Some(type_node) = node.child_by_field_name("type") {
                insert_trimmed(parsed, &type_node, live);
            }
        }
        "call_expression" => {
            // make_unique<X>(...), make_shared<X>(...)
            if let Some(func) = node.child_by_field_name("function") {
                // The function may be a template_function directly (unqualified)
                // or a qualified_identifier containing a template_function (std::make_unique<X>).
                let template_node = if func.kind() == "template_function" {
                    Some(func)
                } else if func.kind() == "qualified_identifier" {
                    find_child_by_kind(&func, "template_function")
                } else {
                    None
                };
                if let Some(tmpl) = template_node {
                    let func_text = parsed.node_text(&tmpl);
                    if func_text.starts_with("make_unique") || func_text.starts_with("make_shared")
                    {
                        extract_template_arg(&tmpl, parsed, live);
                    }
                }
            }
        }
        "declaration" => {
            // Stack allocation: ClassName varname;
            if let Some(type_node) = node.child_by_field_name("type") {
                if type_node.kind() == "type_identifier"
                    || type_node.kind() == "qualified_identifier"
                {
                    insert_trimmed(parsed, &type_node, live);
                }
            }
        }
        _ => {}
    }
}

/// Extract the first template argument from a template_function node.
fn extract_template_arg(
    func: &tree_sitter::Node,
    parsed: &ParsedFile,
    live: &mut BTreeSet<String>,
) {
    let mut cursor = func.walk();
    for child in func.children(&mut cursor) {
        if child.kind() == "template_argument_list" {
            let mut arg_cursor = child.walk();
            for arg in child.children(&mut arg_cursor) {
                if arg.kind() == "type_descriptor" || arg.kind() == "type_identifier" {
                    insert_trimmed(parsed, &arg, live);
                    return;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Java instantiation scanning
// ---------------------------------------------------------------------------

/// Scan Java files for `new ClassName(...)` expressions.
fn scan_java(parsed: &ParsedFile, live: &mut BTreeSet<String>) {
    scan_tree_recursive(parsed.tree.root_node(), parsed, live, &scan_java_node);
}

fn scan_java_node(node: &tree_sitter::Node, parsed: &ParsedFile, live: &mut BTreeSet<String>) {
    if node.kind() == "object_creation_expression" {
        // `new ClassName(...)` — the type is the first child after "new"
        if let Some(type_node) = node.child_by_field_name("type") {
            let type_name = parsed.node_text(&type_node).trim().to_string();
            // Strip generic params: ArrayList<String> → ArrayList
            let base = type_name.split('<').next().unwrap_or(&type_name).trim();
            if !base.is_empty() {
                live.insert(base.to_string());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Go instantiation scanning
// ---------------------------------------------------------------------------

/// Scan Go files for struct literal expressions: `StructName{...}`.
fn scan_go(parsed: &ParsedFile, live: &mut BTreeSet<String>) {
    scan_tree_recursive(parsed.tree.root_node(), parsed, live, &scan_go_node);
}

fn scan_go_node(node: &tree_sitter::Node, parsed: &ParsedFile, live: &mut BTreeSet<String>) {
    if node.kind() == "composite_literal" {
        // Go composite literal: `StructName{field: value}`
        // The type is the first child (before the literal_value `{...}`).
        if let Some(type_node) = node.child_by_field_name("type") {
            let type_name = parsed.node_text(&type_node).trim().to_string();
            // Handle pointer types: &StructName{} → StructName
            let name = type_name.trim_start_matches('&');
            // Handle qualified: pkg.StructName → StructName
            let base = name.split('.').last().unwrap_or(name);
            if !base.is_empty() && base.starts_with(|c: char| c.is_ascii_uppercase()) {
                live.insert(base.to_string());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// TypeScript / JavaScript instantiation scanning
// ---------------------------------------------------------------------------

/// Scan TypeScript files for `new ClassName(...)` expressions.
fn scan_typescript(parsed: &ParsedFile, live: &mut BTreeSet<String>) {
    scan_tree_recursive(parsed.tree.root_node(), parsed, live, &scan_new_expression);
}

/// Scan JavaScript files for `new ClassName(...)` expressions.
fn scan_javascript(parsed: &ParsedFile, live: &mut BTreeSet<String>) {
    scan_tree_recursive(parsed.tree.root_node(), parsed, live, &scan_new_expression);
}

/// Shared scanner for `new_expression` nodes (TypeScript, JavaScript).
fn scan_new_expression(node: &tree_sitter::Node, parsed: &ParsedFile, live: &mut BTreeSet<String>) {
    if node.kind() == "new_expression" {
        // `new ClassName(...)` — constructor is the first child after "new"
        if let Some(constructor) = node.child_by_field_name("constructor") {
            let type_name = parsed.node_text(&constructor).trim().to_string();
            // Strip generic params: new Map<string, number>() → Map
            let base = type_name.split('<').next().unwrap_or(&type_name).trim();
            if !base.is_empty() {
                live.insert(base.to_string());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rust instantiation scanning
// ---------------------------------------------------------------------------

/// Scan Rust files for struct literal expressions: `StructName { ... }`.
fn scan_rust(parsed: &ParsedFile, live: &mut BTreeSet<String>) {
    scan_tree_recursive(parsed.tree.root_node(), parsed, live, &scan_rust_node);
}

fn scan_rust_node(node: &tree_sitter::Node, parsed: &ParsedFile, live: &mut BTreeSet<String>) {
    if node.kind() == "struct_expression" {
        // `StructName { field: value }` — the name is the first child.
        if let Some(name_node) = node.child_by_field_name("name") {
            let type_name = parsed.node_text(&name_node).trim().to_string();
            // Strip path prefix: module::StructName → StructName
            let base = type_name.split("::").last().unwrap_or(&type_name).trim();
            if !base.is_empty() {
                live.insert(base.to_string());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Python instantiation scanning
// ---------------------------------------------------------------------------

/// Scan Python files for class instantiations: `ClassName(...)`.
///
/// Python class instantiations are syntactically identical to function calls.
/// We use `known_classes` to distinguish: only calls where the function name
/// matches a known class are counted as instantiations.
fn scan_python(parsed: &ParsedFile, live: &mut BTreeSet<String>, known_classes: &BTreeSet<String>) {
    scan_python_recursive(parsed.tree.root_node(), parsed, live, known_classes);
}

fn scan_python_recursive(
    node: tree_sitter::Node,
    parsed: &ParsedFile,
    live: &mut BTreeSet<String>,
    known_classes: &BTreeSet<String>,
) {
    if node.kind() == "call" {
        if let Some(func) = node.child_by_field_name("function") {
            let name = parsed.node_text(&func).trim().to_string();
            // Handle dotted calls: module.ClassName() → ClassName
            let base = name.split('.').last().unwrap_or(&name);
            if known_classes.contains(base) {
                live.insert(base.to_string());
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        scan_python_recursive(child, parsed, live, known_classes);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Generic recursive tree scanner. Calls `scanner_fn` on every node, then
/// recurses into children.
fn scan_tree_recursive(
    node: tree_sitter::Node,
    parsed: &ParsedFile,
    live: &mut BTreeSet<String>,
    scanner_fn: &dyn Fn(&tree_sitter::Node, &ParsedFile, &mut BTreeSet<String>),
) {
    scanner_fn(&node, parsed, live);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        scan_tree_recursive(child, parsed, live, scanner_fn);
    }
}

/// Find the first direct child of `node` with the given kind.
fn find_child_by_kind<'a>(
    node: &tree_sitter::Node<'a>,
    kind: &str,
) -> Option<tree_sitter::Node<'a>> {
    let count = node.child_count();
    for i in 0..count {
        if let Some(child) = node.child(i) {
            if child.kind() == kind {
                return Some(child);
            }
        }
    }
    None
}

/// Insert a trimmed, non-empty text from a node into the live set.
fn insert_trimmed(parsed: &ParsedFile, node: &tree_sitter::Node, live: &mut BTreeSet<String>) {
    let text = parsed.node_text(node).trim().to_string();
    if !text.is_empty() {
        live.insert(text);
    }
}
