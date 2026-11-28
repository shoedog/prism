use super::{annotations_postponed, children, identifier};
use prism::ast::ParsedFile;
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
};
use tree_sitter::Node;

struct Edit {
    star: usize,
    end: usize,
    trailing_error: bool,
}

#[derive(Default)]
pub(in crate::targets::inventory::python) struct StarredAnnotations {
    pub(in crate::targets::inventory::python) parsed: Option<ParsedFile>,
    pub(in crate::targets::inventory::python) refused: Vec<Range<usize>>,
}

/// Keep byte positions and the enclosing tree while reparsing the expression
/// of a PEP 646 vararg annotation. The star belongs to the annotation's
/// outer AST node, outside the expression that the inventory walks.
pub(in crate::targets::inventory::python) fn recover(parsed: &ParsedFile) -> StarredAnnotations {
    let mut result = StarredAnnotations::default();
    if !parsed.source.contains('*') {
        return result;
    }
    let original = parsed.tree.root_node();
    let mut edits = BTreeMap::new();
    collect(
        parsed,
        original,
        annotations_postponed(parsed),
        &mut edits,
        &mut result.refused,
    );
    if !edits.is_empty() {
        result.parsed = reparse(parsed, &edits);
        if result.parsed.is_none() {
            result
                .refused
                .extend(edits.values().map(|edit| edit.star..edit.end));
        }
    }
    result.refused.sort_unstable_by_key(|range| range.start);
    result
}

fn reparse(parsed: &ParsedFile, edits: &BTreeMap<usize, Edit>) -> Option<ParsedFile> {
    let mut bytes = parsed.source.as_bytes().to_vec();
    for edit in edits.values() {
        bytes[edit.star] = b' ';
    }
    let text = String::from_utf8(bytes).ok()?;
    let recovered = ParsedFile::parse(&parsed.path, &text, parsed.language).ok()?;
    same_outside_annotations(parsed.tree.root_node(), recovered.tree.root_node(), edits)
        .then_some(recovered)
}

fn is_starred_expression(text: &str, parsed: &ParsedFile, async_scope: bool) -> bool {
    let Ok(fragment) = ParsedFile::parse(&parsed.path, &format!("(\n{text}\n)"), parsed.language)
    else {
        return false;
    };
    let root = fragment.tree.root_node();
    if root.has_error() {
        return false;
    }
    let mut statements = children(root);
    let (Some(statement), None) = (statements.next(), statements.next()) else {
        return false;
    };
    if statement.kind() != "expression_statement" {
        return false;
    }
    let mut expressions = children(statement);
    let (Some(group), None) = (expressions.next(), expressions.next()) else {
        return false;
    };
    // CPython 3.11 star_annotation takes a bitwise_or operand. Admit only
    // names, unformatted literals, displays and comprehensions, explicit groups, primary
    // trailers (attribute, subscript, call), arithmetic/bitwise operators,
    // and await in an eager annotation in an enclosing async function.
    // Lambda, conditional, boolean and comparison operands need grouping.
    group.kind() == "parenthesized_expression"
        && children(group).next().is_some_and(|expr| {
            matches!(
                expr.kind(),
                "attribute"
                    | "await"
                    | "binary_operator"
                    | "call"
                    | "concatenated_string"
                    | "dictionary"
                    | "dictionary_comprehension"
                    | "ellipsis"
                    | "false"
                    | "float"
                    | "generator_expression"
                    | "identifier"
                    | "integer"
                    | "list"
                    | "list_comprehension"
                    | "none"
                    | "parenthesized_expression"
                    | "set"
                    | "set_comprehension"
                    | "string"
                    | "subscript"
                    | "true"
                    | "tuple"
                    | "unary_operator"
            ) && valid_annotation_context(expr, &fragment, async_scope)
        })
}

fn valid_annotation_context(node: Node<'_>, parsed: &ParsedFile, async_scope: bool) -> bool {
    match node.kind() {
        "await" if !async_scope => return false,
        "yield" | "named_expression" => return false,
        "for_in_clause"
            if !async_scope && node.child(0).is_some_and(|token| token.kind() == "async") =>
        {
            return false
        }
        "argument_list" if !valid_arguments(node, parsed) => return false,
        "string" => return string_is_bytes(node, parsed).is_some(),
        "concatenated_string" => {
            let mut parts = children(node);
            return parts
                .next()
                .and_then(|part| string_is_bytes(part, parsed))
                .is_some_and(|bytes| {
                    parts.all(|part| string_is_bytes(part, parsed) == Some(bytes))
                });
        }
        "lambda_parameters" => {
            // Admit simple, distinct positional names for grouped lambdas.
            let mut names = BTreeSet::new();
            return children(node).all(|arg| {
                arg.kind() == "identifier" && names.insert(identifier(parsed.node_text(&arg)))
            });
        }
        _ => {}
    }
    let lambda_body = (node.kind() == "lambda")
        .then(|| node.child_by_field_name("body"))
        .flatten();
    children(node).all(|child| {
        valid_annotation_context(child, parsed, async_scope && Some(child) != lambda_body)
    })
}

fn valid_arguments(node: Node<'_>, parsed: &ParsedFile) -> bool {
    let mut keywords = BTreeSet::new();
    let mut unpacked_keywords = false;
    children(node).all(|arg| match arg.kind() {
        "dictionary_splat" => {
            unpacked_keywords = true;
            true
        }
        "list_splat" => !unpacked_keywords,
        "keyword_argument" => arg
            .child_by_field_name("name")
            .is_some_and(|name| keywords.insert(identifier(parsed.node_text(&name)))),
        _ => !unpacked_keywords && keywords.is_empty(),
    })
}

fn string_is_bytes(node: Node<'_>, parsed: &ParsedFile) -> Option<bool> {
    let text = parsed.node_text(&node);
    let (prefix, _) = text.split_once(['\'', '"'])?;
    match prefix.to_ascii_lowercase().as_str() {
        "" | "r" | "u" => Some(false),
        "b" | "br" | "rb" if text.is_ascii() => Some(true),
        _ => None,
    }
}

fn collect(
    parsed: &ParsedFile,
    node: Node<'_>,
    postponed: bool,
    edits: &mut BTreeMap<usize, Edit>,
    refused: &mut Vec<Range<usize>>,
) {
    if let Some(def) = node
        .parent()
        .filter(|def| node.kind() == "parameters" && def.kind() == "function_definition")
    {
        let async_scope = !postponed
            && std::iter::successors(def.parent(), Node::parent)
                .find(|owner| matches!(owner.kind(), "function_definition" | "class_definition"))
                .is_some_and(|owner| owner.child(0).is_some_and(|token| token.kind() == "async"));
        for parameter in children(node) {
            if let Some((edit, vararg)) = starred_region(parameter, parsed) {
                if vararg
                    && is_starred_expression(
                        &parsed.source[edit.star + 1..edit.end],
                        parsed,
                        async_scope,
                    )
                {
                    edits.insert(parameter.start_byte(), edit);
                } else {
                    refused.push(edit.star..edit.end);
                }
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect(parsed, child, postponed, edits, refused);
    }
}

fn starred_region(parameter: Node<'_>, parsed: &ParsedFile) -> Option<(Edit, bool)> {
    match parameter.kind() {
        "typed_parameter" | "typed_default_parameter" => {
            let mut cursor = parameter.walk();
            let mut tokens = parameter.children(&mut cursor);
            tokens.find(|token| token.kind() == ":")?;
            let start = tokens
                .find(|token| !matches!(token.kind(), "comment" | "line_continuation"))?
                .start_byte();
            if parsed.source.as_bytes().get(start) != Some(&b'*') {
                return None;
            }
            let tail = parameter
                .next_sibling()
                .filter(|node| node.kind() == "ERROR");
            let end = tail.map_or_else(|| parameter.end_byte(), |node| node.end_byte());
            let vararg = parameter.kind() == "typed_parameter"
                && children(parameter).next().is_some_and(is_vararg)
                // A bare await may split its operand into a sibling ERROR.
                && (tail.is_none() || parsed.source[start + 1..parameter.end_byte()].trim() == "await");
            Some((
                Edit {
                    star: start,
                    end,
                    trailing_error: tail.is_some(),
                },
                vararg,
            ))
        }
        "identifier" | "list_splat_pattern" | "dictionary_splat_pattern" => {
            let error = parameter.next_sibling()?;
            if error.kind() != "ERROR" || error.child(0)?.kind() != ":" {
                return None;
            }
            let star = error.child(1)?.start_byte();
            (parsed.source.as_bytes().get(star) == Some(&b'*')).then_some((
                Edit {
                    star,
                    end: error.end_byte(),
                    trailing_error: true,
                },
                is_vararg(parameter),
            ))
        }
        _ => None,
    }
}

fn is_vararg(node: Node<'_>) -> bool {
    node.kind() == "list_splat_pattern"
        && !node.has_error()
        && children(node)
            .next()
            .is_some_and(|name| name.kind() == "identifier")
}

fn repaired_parameter(before: Node<'_>, after: Node<'_>, edit: &Edit) -> bool {
    let target = if before.kind() == "list_splat_pattern" {
        Some(before)
    } else {
        children(before).next()
    };
    after.kind() == "typed_parameter"
        && !after.has_error()
        && after.start_byte() == before.start_byte()
        && after.end_byte() == edit.end
        && target
            .zip(children(after).next())
            .is_some_and(|(old, new)| {
                old.kind() == new.kind() && old.byte_range() == new.byte_range()
            })
        && after
            .child_by_field_name("type")
            .is_some_and(|value| value.start_byte() > edit.star && value.end_byte() == edit.end)
}

fn same_outside_annotations(
    original: Node<'_>,
    recovered: Node<'_>,
    edits: &BTreeMap<usize, Edit>,
) -> bool {
    if original.kind() != recovered.kind()
        || original.byte_range() != recovered.byte_range()
        || original.is_missing() != recovered.is_missing()
    {
        return false;
    }
    let mut left = original.walk();
    let mut right = recovered.walk();
    let mut before = original.children(&mut left);
    for after in recovered.children(&mut right) {
        let Some(old) = before.next() else {
            return false;
        };
        if let Some(edit) = edits.get(&old.start_byte()).filter(|_| {
            original.kind() == "parameters"
                && matches!(old.kind(), "list_splat_pattern" | "typed_parameter")
        }) {
            if !repaired_parameter(old, after, edit) {
                return false;
            }
            if edit.trailing_error {
                before.next();
            }
        } else if !same_outside_annotations(old, after, edits) {
            return false;
        }
    }
    before.next().is_none()
}
