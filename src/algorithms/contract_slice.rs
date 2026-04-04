//! Contract Slice — implicit behavioral contract extraction and violation detection.
//!
//! Given a changed function, extracts implicit contracts (preconditions from guard
//! clauses, assertions, validation patterns) and flags changes that may weaken or
//! break them. Emits `SliceFinding`s with `category: "contract"` for summaries and
//! `category: "contract_violation"` for modified guard clauses.

use crate::ast::ParsedFile;
use crate::diff::{DiffBlock, DiffInput};
use crate::slice::{SliceFinding, SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

/// A detected precondition established by a guard clause.
struct Precondition {
    variable: String,
    constraint: ConstraintKind,
    guard_line: usize,
    guard_text: String,
}

/// Classification of the constraint a guard clause establishes.
#[allow(dead_code)]
enum ConstraintKind {
    /// x != null/None/nil/NULL
    NonNull,
    /// len(x) > 0 / !x.is_empty()
    NonEmpty,
    /// isinstance(x, T) / typeof x === 'T'
    TypeCheck(String),
    /// x > low && x < high / x >= 0
    RangeCheck,
    /// x > 0
    Positive,
    /// assert(...) / require(...)
    CustomAssertion,
}

impl ConstraintKind {
    fn description(&self) -> &str {
        match self {
            Self::NonNull => "non-null",
            Self::NonEmpty => "non-empty",
            Self::TypeCheck(_) => "type-check",
            Self::RangeCheck => "range-check",
            Self::Positive => "positive",
            Self::CustomAssertion => "assertion",
        }
    }
}

pub fn slice(files: &BTreeMap<String, ParsedFile>, diff: &DiffInput) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::ContractSlice);
    let mut block_id = 0;

    for diff_info in &diff.files {
        let parsed = match files.get(&diff_info.file_path) {
            Some(f) => f,
            None => continue,
        };

        let mut seen_functions: BTreeSet<(usize, usize)> = BTreeSet::new();
        for &line in &diff_info.diff_lines {
            let func_node = match parsed.enclosing_function(line) {
                Some(f) => f,
                None => continue,
            };
            let range = parsed.node_line_range(&func_node);
            if !seen_functions.insert(range) {
                continue;
            }

            let (func_start, func_end) = range;
            let func_name = parsed
                .language
                .function_name(&func_node)
                .map(|n| parsed.node_text(&n).to_string())
                .unwrap_or_else(|| "<anonymous>".to_string());

            let preconditions = extract_preconditions(parsed, &func_node, func_start, func_end);

            // Check if diff touches any guard clause
            for pre in &preconditions {
                if diff_info.diff_lines.contains(&pre.guard_line) {
                    result.findings.push(SliceFinding {
                        algorithm: "contract".to_string(),
                        file: diff_info.file_path.clone(),
                        line: pre.guard_line,
                        severity: "warning".to_string(),
                        description: format!(
                            "Guard clause modified in '{}': {} constraint on '{}'. \
                             Verify callers still receive valid values.",
                            func_name,
                            pre.constraint.description(),
                            pre.variable,
                        ),
                        function_name: Some(func_name.clone()),
                        related_lines: vec![pre.guard_line],
                        related_files: vec![],
                        category: Some("contract_violation".to_string()),
                    });
                }
            }

            // Emit contract summary as info-level finding
            if !preconditions.is_empty() {
                let summary = preconditions
                    .iter()
                    .map(|p| {
                        format!(
                            "{}: {} (line {})",
                            p.variable,
                            p.constraint.description(),
                            p.guard_line
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("; ");

                result.findings.push(SliceFinding {
                    algorithm: "contract".to_string(),
                    file: diff_info.file_path.clone(),
                    line: func_start,
                    severity: "info".to_string(),
                    description: format!(
                        "Contract for '{}': preconditions: {}",
                        func_name, summary,
                    ),
                    function_name: Some(func_name.clone()),
                    related_lines: preconditions.iter().map(|p| p.guard_line).collect(),
                    related_files: vec![],
                    category: Some("contract".to_string()),
                });

                // Include guard clause lines in the block
                let mut block = DiffBlock::new(
                    block_id,
                    diff_info.file_path.clone(),
                    diff_info.modify_type.clone(),
                );
                block.add_line(&diff_info.file_path, func_start, false);
                for pre in &preconditions {
                    block.add_line(&diff_info.file_path, pre.guard_line, false);
                }
                for &dl in &diff_info.diff_lines {
                    if dl >= func_start && dl <= func_end {
                        block.add_line(&diff_info.file_path, dl, true);
                    }
                }
                block.add_line(&diff_info.file_path, func_end, false);
                result.blocks.push(block);
                block_id += 1;
            }
        }
    }

    Ok(result)
}

/// Extract preconditions from guard clauses in the early part of a function.
///
/// A guard clause is an `if` statement in the first 30% of the function body
/// (by line count, minimum 5 lines) whose body contains an early-exit keyword
/// (return, raise, throw, panic, error). Also detects assert/require statements.
fn extract_preconditions(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
    func_start: usize,
    func_end: usize,
) -> Vec<Precondition> {
    let func_len = func_end.saturating_sub(func_start);
    let guard_zone_end = func_start + (func_len * 30 / 100).max(5);

    let mut preconditions = Vec::new();

    // Find the function body node
    let body = func_node
        .child_by_field_name("body")
        .or_else(|| func_node.child_by_field_name("consequence"));
    let body_node = match body {
        Some(b) => b,
        None => return preconditions,
    };

    // Walk top-level statements in the function body
    let mut cursor = body_node.walk();
    for child in body_node.children(&mut cursor) {
        let child_line = child.start_position().row + 1;

        // Only consider statements in the guard zone
        if child_line > guard_zone_end {
            break;
        }

        // Check for assert/require statements
        if is_assert_statement(parsed, &child) {
            let text = parsed.node_text(&child).to_string();
            let var = extract_first_identifier(parsed, &child);
            preconditions.push(Precondition {
                variable: var,
                constraint: ConstraintKind::CustomAssertion,
                guard_line: child_line,
                guard_text: text.lines().next().unwrap_or(&text).to_string(),
            });
            continue;
        }

        // Find the if-node, which may be the child directly or nested inside
        // an expression_statement (Rust: `if_expression` is wrapped).
        let if_node = if is_if_node(child.kind()) {
            Some(child)
        } else {
            find_if_child(&child)
        };
        let if_node = match if_node {
            Some(n) => n,
            None => continue,
        };

        if !body_has_early_exit(parsed, &if_node) {
            continue;
        }

        // Extract the condition: try language-specific field, then fallback
        // to extracting from the source text.
        let cond_text = if let Some(cond_node) = parsed.language.control_flow_condition(&if_node) {
            parsed.node_text(&cond_node).to_string()
        } else {
            // Fallback: extract condition from source text between "if" and "{"/":"
            extract_condition_text(parsed.node_text(&if_node))
        };

        let guard_text = parsed
            .node_text(&if_node)
            .lines()
            .next()
            .unwrap_or("")
            .to_string();

        if let Some((var, constraint)) = classify_condition(&cond_text) {
            preconditions.push(Precondition {
                variable: var,
                constraint,
                guard_line: child_line,
                guard_text,
            });
        }
    }

    preconditions
}

/// Check if a node is an if-statement across languages.
fn is_if_node(kind: &str) -> bool {
    matches!(kind, "if_statement" | "if_expression" | "if_let_expression")
}

/// Find an if-node among children (e.g., expression_statement wrapping if_expression).
fn find_if_child<'a>(node: &Node<'a>) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if is_if_node(child.kind()) {
            return Some(child);
        }
    }
    None
}

/// Extract condition text from the source text of an if-statement.
/// Fallback for when tree-sitter doesn't provide a "condition" field.
fn extract_condition_text(if_text: &str) -> String {
    // Skip "if " prefix, take until "{" or ":"
    let text = if_text.trim();
    let text = text.strip_prefix("if").unwrap_or(text).trim();
    // Take until first '{' or ':'
    let end = text
        .find('{')
        .or_else(|| text.find(':'))
        .unwrap_or(text.len());
    text[..end].trim().to_string()
}

/// Check if the body of an if-statement contains an early exit.
fn body_has_early_exit(parsed: &ParsedFile, if_node: &Node<'_>) -> bool {
    let text = parsed.node_text(if_node);
    // Check for exit keywords in the if body
    let exit_keywords = [
        "return", "raise", "throw", "panic", "error", "exit", "abort",
    ];
    for kw in &exit_keywords {
        if text.contains(kw) {
            return true;
        }
    }
    false
}

/// Check if a statement is an assert/require call.
fn is_assert_statement(parsed: &ParsedFile, node: &Node<'_>) -> bool {
    let text = parsed.node_text(node);
    let trimmed = text.trim();
    trimmed.starts_with("assert")
        || trimmed.starts_with("require(")
        || trimmed.starts_with("assert!(")
        || trimmed.starts_with("assert_eq!")
        || trimmed.starts_with("debug_assert")
        || node.kind() == "assert_statement"
}

/// Extract the first identifier from a node (for assert/require).
fn extract_first_identifier(parsed: &ParsedFile, node: &Node<'_>) -> String {
    fn find_ident<'a>(parsed: &ParsedFile, node: Node<'a>) -> Option<String> {
        if parsed.language.is_identifier_node(node.kind()) {
            let text = parsed.node_text(&node).to_string();
            // Skip keywords
            if !matches!(
                text.as_str(),
                "assert"
                    | "require"
                    | "isinstance"
                    | "typeof"
                    | "null"
                    | "None"
                    | "nil"
                    | "true"
                    | "false"
            ) {
                return Some(text);
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(name) = find_ident(parsed, child) {
                return Some(name);
            }
        }
        None
    }
    find_ident(parsed, *node).unwrap_or_else(|| "<expr>".to_string())
}

/// Classify a condition expression into a constraint kind.
/// Returns (variable_name, constraint) or None if unrecognized.
fn classify_condition(cond: &str) -> Option<(String, ConstraintKind)> {
    // Strip outer parentheses (C/C++/Go if-conditions are often parenthesized)
    let trimmed = cond.trim();
    let trimmed = if trimmed.starts_with('(') && trimmed.ends_with(')') {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };
    let trimmed = trimmed.trim();

    // Null/None/nil checks: `x is None`, `x == None`, `x == null`, `!x`, `x == nil`, `x == NULL`
    for null_lit in &["None", "null", "nil", "NULL", "nullptr"] {
        // `x is None` or `x == None`
        if let Some(var) = trimmed
            .strip_suffix(&format!(" is {}", null_lit))
            .or_else(|| trimmed.strip_suffix(&format!(" == {}", null_lit)))
        {
            let var = var.trim();
            if is_simple_var(var) {
                return Some((var.to_string(), ConstraintKind::NonNull));
            }
        }
        // `x is not None` → non-null is already established, not a guard
        // `x != None` → same
    }

    // `!ptr` or `!x` (C-style null check)
    if let Some(var) = trimmed.strip_prefix('!') {
        let var = var.trim().trim_start_matches('(').trim_end_matches(')');
        if is_simple_var(var) {
            return Some((var.to_string(), ConstraintKind::NonNull));
        }
    }

    // `ptr == 0` or `ptr == false` — C null pointer check
    if let Some(var) = trimmed
        .strip_suffix(" == 0")
        .or_else(|| trimmed.strip_suffix(" == false"))
    {
        let var = var.trim();
        if is_simple_var(var) {
            return Some((var.to_string(), ConstraintKind::NonNull));
        }
    }

    // Empty checks: `len(x) == 0`, `x.length === 0`, `x.is_empty()`, `len(x) < 1`
    if trimmed.contains("len(") && (trimmed.contains("== 0") || trimmed.contains("< 1")) {
        if let Some(var) = extract_len_arg(trimmed) {
            return Some((var, ConstraintKind::NonEmpty));
        }
    }
    if trimmed.contains(".length")
        && (trimmed.contains("=== 0") || trimmed.contains("== 0") || trimmed.contains("< 1"))
    {
        if let Some(var) = trimmed.split(".length").next() {
            let var = var.trim();
            if is_simple_var(var) {
                return Some((var.to_string(), ConstraintKind::NonEmpty));
            }
        }
    }
    if trimmed.contains(".is_empty()") {
        if let Some(var) = trimmed.strip_suffix(".is_empty()").or_else(|| {
            trimmed
                .strip_prefix("!")
                .and_then(|s| s.strip_suffix(".is_empty()"))
        }) {
            let var = var.trim();
            if is_simple_var(var) {
                return Some((var.to_string(), ConstraintKind::NonEmpty));
            }
        }
    }

    // Type checks: `isinstance(x, T)`, `typeof x !== 'T'`, `!(x instanceof T)`
    if trimmed.starts_with("isinstance(") {
        if let Some(inner) = trimmed
            .strip_prefix("isinstance(")
            .and_then(|s| s.strip_suffix(')'))
        {
            let parts: Vec<&str> = inner.splitn(2, ',').collect();
            if parts.len() == 2 {
                let var = parts[0].trim();
                let ty = parts[1].trim();
                if is_simple_var(var) {
                    return Some((var.to_string(), ConstraintKind::TypeCheck(ty.to_string())));
                }
            }
        }
    }
    if trimmed.contains("typeof ") {
        // `typeof x !== 'string'`
        if let Some(var) = extract_typeof_var(trimmed) {
            return Some((var, ConstraintKind::TypeCheck("type".to_string())));
        }
    }

    // Range checks: `x < 0`, `x > high`, `x < low`, `x >= limit`
    for op in &[" < ", " > ", " <= ", " >= "] {
        if trimmed.contains(op) {
            let parts: Vec<&str> = trimmed.splitn(2, op).collect();
            if parts.len() == 2 {
                let left = parts[0].trim();
                let right = parts[1].trim();
                if is_simple_var(left) && is_numeric_or_var(right) {
                    // `x < 0` → positive constraint; `x > max` → range
                    if right == "0" && (*op == " < " || *op == " <= ") {
                        return Some((left.to_string(), ConstraintKind::Positive));
                    }
                    return Some((left.to_string(), ConstraintKind::RangeCheck));
                }
            }
        }
    }

    // Empty string check: `x == ""`, `x == ''`
    if let Some(var) = trimmed
        .strip_suffix(" == \"\"")
        .or_else(|| trimmed.strip_suffix(" == ''"))
    {
        let var = var.trim();
        if is_simple_var(var) {
            return Some((var.to_string(), ConstraintKind::NonEmpty));
        }
    }

    // Go error check: `err != nil`
    if let Some(var) = trimmed
        .strip_suffix(" != nil")
        .or_else(|| trimmed.strip_suffix(" != null"))
        .or_else(|| trimmed.strip_suffix(" != None"))
    {
        let var = var.trim();
        if is_simple_var(var) {
            return Some((var.to_string(), ConstraintKind::NonNull));
        }
    }

    None
}

/// Check if a string looks like a simple variable name.
fn is_simple_var(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
        && s.chars()
            .next()
            .map_or(false, |c| c.is_alphabetic() || c == '_')
}

/// Check if a string looks like a number or simple variable.
fn is_numeric_or_var(s: &str) -> bool {
    is_simple_var(s)
        || s.chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == '-')
}

/// Extract variable name from `len(x)` pattern.
fn extract_len_arg(s: &str) -> Option<String> {
    let start = s.find("len(")? + 4;
    let end = s[start..].find(')')? + start;
    let var = s[start..end].trim();
    if is_simple_var(var) {
        Some(var.to_string())
    } else {
        None
    }
}

/// Extract variable from `typeof x !== 'T'` pattern.
fn extract_typeof_var(s: &str) -> Option<String> {
    let idx = s.find("typeof ")?;
    let rest = &s[idx + 7..];
    let var_end = rest.find(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')?;
    let var = &rest[..var_end];
    if is_simple_var(var) {
        Some(var.to_string())
    } else {
        None
    }
}
