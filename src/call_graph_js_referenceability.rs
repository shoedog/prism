//! Negative target syntax and caller binding evidence for E5. No module path resolution.
use crate::ast::ParsedFile;
use crate::call_graph::FunctionId;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

fn is_module(parsed: &ParsedFile) -> bool {
    let root = parsed.tree.root_node();
    let mut cursor = root.walk();
    let module = root
        .named_children(&mut cursor)
        .any(|n| matches!(n.kind(), "import_statement" | "export_statement"));
    module
}

// E5-specific evidence: these names exempt refusal, never supply a positive target.
fn additional_import_evidence(parsed: &ParsedFile, names: &mut BTreeSet<String>) {
    let mut stack = vec![parsed.tree.root_node()];
    while let Some(node) = stack.pop() {
        match node.kind() {
            "import_require_clause" => {
                let mut cursor = node.walk();
                if let Some(local) = node
                    .named_children(&mut cursor)
                    .find(|n| n.kind() == "identifier")
                {
                    names.insert(parsed.node_text(&local).to_string());
                };
            }
            "import_alias" => {
                if let (Some(local), Some(entity)) = (node.named_child(0), node.named_child(1)) {
                    let terminal = parsed
                        .node_text(&entity)
                        .rsplit('.')
                        .next()
                        .unwrap_or("")
                        .trim();
                    if parsed.node_text(&local) == terminal
                        || terminal == "require" && exported_require_alias(node, entity)
                    {
                        names.insert(parsed.node_text(&local).to_string());
                    }
                }
            }
            "variable_declarator" => {
                if let (Some(pattern), Some(value)) = (
                    node.child_by_field_name("name"),
                    node.child_by_field_name("value"),
                ) {
                    let call = value
                        .named_child(0)
                        .filter(|_| value.kind() == "await_expression");
                    if pattern.kind() == "object_pattern"
                        && call.is_some_and(|call| {
                            call.kind() == "call_expression"
                                && call
                                    .child_by_field_name("function")
                                    .is_some_and(|f| f.kind() == "import")
                        })
                    {
                        destructured_names(parsed, pattern, names);
                    }
                }
            }
            _ => {}
        }
        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
}

// The vendored TS grammar recovers `export import f = require('m')` as an
// import_alias ending at `require` with a missing semicolon, then a string in
// parentheses. Recognize only that external-import shape for E5's exemption.
fn exported_require_alias(node: tree_sitter::Node<'_>, entity: tree_sitter::Node<'_>) -> bool {
    let mut cursor = node.walk();
    let missing_semicolon = node
        .children(&mut cursor)
        .any(|n| n.is_missing() && n.kind() == ";");
    entity.kind() == "identifier"
        && missing_semicolon
        && node
            .parent()
            .filter(|p| p.kind() == "export_statement")
            .and_then(|p| p.next_named_sibling())
            .filter(|n| n.kind() == "expression_statement")
            .and_then(|n| n.named_child(0))
            .filter(|n| n.kind() == "parenthesized_expression")
            .is_some_and(|n| {
                n.named_child_count() == 1 && n.named_child(0).is_some_and(|c| c.kind() == "string")
            })
}

fn destructured_names(
    parsed: &ParsedFile,
    node: tree_sitter::Node<'_>,
    names: &mut BTreeSet<String>,
) {
    match node.kind() {
        "identifier" | "shorthand_property_identifier_pattern" => {
            names.insert(parsed.node_text(&node).to_string());
        }
        "pair_pattern" => {
            if let Some(value) = node.child_by_field_name("value") {
                destructured_names(parsed, value, names);
            }
        }
        "assignment_pattern" | "object_assignment_pattern" => {
            if let Some(left) = node.child_by_field_name("left") {
                destructured_names(parsed, left, names);
            }
        }
        "object_pattern" | "array_pattern" | "rest_pattern" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                destructured_names(parsed, child, names);
            }
        }
        _ => {}
    }
}

pub(super) fn module_binding_evidence<'a>(
    files: impl IntoIterator<Item = (&'a String, &'a ParsedFile)>,
) -> BTreeMap<String, BTreeSet<String>> {
    files
        .into_iter()
        .filter_map(|(file, parsed)| {
            if !matches!(
                parsed.language,
                Language::JavaScript | Language::TypeScript | Language::Tsx
            ) {
                return None;
            }
            // The existing syntactic extractor sees scalar, destructured and
            // nested requires. Even conservative evidence keeps the fallback;
            // this mask never grants a positive binding or traverses a target.
            is_module(parsed).then(|| {
                let mut names = parsed.extract_imports().into_keys().collect();
                additional_import_evidence(parsed, &mut names);
                (file.clone(), names)
            })
        })
        .collect()
}

pub(super) fn free_single_refusals<'a>(
    files: impl IntoIterator<Item = (&'a String, &'a ParsedFile)>,
) -> BTreeSet<FunctionId> {
    let mut refusals = BTreeSet::new();
    for (file, parsed) in files {
        if !matches!(
            parsed.language,
            crate::languages::Language::JavaScript
                | crate::languages::Language::TypeScript
                | crate::languages::Language::Tsx
        ) {
            continue;
        }
        let module = is_module(parsed);
        for function in parsed.all_functions() {
            let inferred = function.child_by_field_name("name").is_none();
            let parent = function.parent();
            let unreachable_name = function.kind() == "method_definition"
                || inferred
                    && parent.is_some_and(|p| match p.kind() {
                        "pair" | "public_field_definition" | "field_definition" => true,
                        "assignment_expression" => {
                            p.child_by_field_name("left").is_some_and(|lhs| {
                                matches!(lhs.kind(), "member_expression" | "subscript_expression")
                            })
                        }
                        "arguments" => p
                            .parent()
                            .filter(|p| p.kind() == "call_expression")
                            .and_then(|p| p.parent())
                            .is_some_and(|p| p.kind() == "variable_declarator"),
                        _ => false,
                    });
            if !module && !unreachable_name {
                continue;
            }
            let Some(name) = parsed.language.function_name(&function) else {
                continue;
            };
            let (start_line, end_line) = parsed.node_line_range(&function);
            refusals.insert(FunctionId {
                file: file.clone(),
                name: parsed.node_text(&name).to_string(),
                start_line,
                end_line,
            });
        }
    }
    refusals
}
