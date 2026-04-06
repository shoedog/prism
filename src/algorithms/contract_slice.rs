//! Contract Slice — implicit behavioral contract extraction and violation detection.
//!
//! Given a changed function, extracts implicit contracts (preconditions from guard
//! clauses, assertions, validation patterns) and flags changes that may weaken or
//! break them. Emits `SliceFinding`s with `category: "contract"` for summaries and
//! `category: "contract_violation"` for modified guard clauses.

use crate::ast::ParsedFile;
use crate::diff::{DiffBlock, DiffInput};
use crate::languages::Language;
use crate::slice::{SliceFinding, SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use tree_sitter::Node;

/// A detected precondition established by a guard clause.
struct Precondition {
    variable: String,
    constraint: ConstraintKind,
    guard_line: usize,
    /// First line of the guard clause source text (for informative findings).
    guard_text: String,
}

/// Classification of the constraint a guard clause establishes.
enum ConstraintKind {
    /// x != null/None/nil/NULL — execution continues only when x is non-null.
    NonNull,
    /// err != nil early return — execution continues only when err is nil.
    /// Semantically the guard checks "if error exists, bail out", so the
    /// postcondition is that the error variable is nil/null.
    NilCheck,
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
    fn description(&self) -> String {
        match self {
            Self::NonNull => "non-null".to_string(),
            Self::NilCheck => "nil-check (error handled)".to_string(),
            Self::NonEmpty => "non-empty".to_string(),
            Self::TypeCheck(ty) => format!("type-check (expected: {})", ty),
            Self::RangeCheck => "range-check".to_string(),
            Self::Positive => "positive".to_string(),
            Self::CustomAssertion => "assertion".to_string(),
        }
    }

    /// Returns a short tag for comparison (ignoring TypeCheck's inner data).
    fn tag(&self) -> &str {
        match self {
            Self::NonNull => "non-null",
            Self::NilCheck => "nil-check",
            Self::NonEmpty => "non-empty",
            Self::TypeCheck(_) => "type-check",
            Self::RangeCheck => "range-check",
            Self::Positive => "positive",
            Self::CustomAssertion => "assertion",
        }
    }
}

impl PostconditionKind {
    fn kind_name(&self) -> &str {
        match self {
            Self::AlwaysNonNull => "always-non-null",
            Self::ConsistentType(_) => "consistent-type",
            Self::Nullable { .. } => "nullable",
            Self::NonNullOrThrows => "non-null-or-throws",
            Self::GoResultPair => "go-result-pair",
            Self::Void => "void",
            Self::AlwaysBool => "always-bool",
            Self::Mixed => "mixed",
        }
    }
}

/// A detected postcondition about a function's return behavior.
struct Postcondition {
    /// Classification of the return pattern.
    kind: PostconditionKind,
    /// Return statement lines that establish this postcondition.
    return_lines: Vec<usize>,
    /// Human-readable summary.
    description: String,
}

/// Classification of a function's return pattern.
enum PostconditionKind {
    /// All return paths return a non-null/non-None value.
    AlwaysNonNull,
    /// All return paths return the same type (by text classification).
    ConsistentType(String),
    /// Some paths return null/None (nullable return).
    Nullable {
        /// Lines that return null/None.
        null_lines: Vec<usize>,
        /// Lines that return non-null values (used in Phase 3c echo integration).
        #[allow(dead_code)]
        value_lines: Vec<usize>,
    },
    /// Function raises/throws on error, never returns null.
    NonNullOrThrows,
    /// Go-style (value, error) return pattern.
    GoResultPair,
    /// All paths return void (no return expression, or `return;`).
    Void,
    /// Returns a boolean (true/false) on all paths.
    AlwaysBool,
    /// Mixed or unclassifiable return patterns.
    Mixed,
}

/// Classification of a single return value expression.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ReturnValueClass {
    Null,
    Bool,
    Numeric,
    StringLit,
    EmptyCollection,
    GoError,
    GoSuccess,
    ConstructedValue,
    Expression,
    Void,
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
                            "Guard clause modified in '{}': {} constraint on '{}' (`{}`). \
                             Verify callers still receive valid values.",
                            func_name,
                            pre.constraint.description(),
                            pre.variable,
                            pre.guard_text,
                        ),
                        function_name: Some(func_name.clone()),
                        related_lines: vec![pre.guard_line],
                        related_files: vec![],
                        category: Some("contract_violation".to_string()),
                        parse_quality: None,
                    });
                }
            }

            // Phase 2: Extract postconditions
            let postconditions = extract_postconditions(parsed, &func_node);

            // Check if diff touches any return statement
            for post in &postconditions {
                for &ret_line in &post.return_lines {
                    if diff_info.diff_lines.contains(&ret_line) {
                        result.findings.push(SliceFinding {
                            algorithm: "contract".to_string(),
                            file: diff_info.file_path.clone(),
                            line: ret_line,
                            severity: "warning".to_string(),
                            description: format!(
                                "Return behavior modified in '{}': {} postcondition. \
                                 Verify callers handle the new return pattern.",
                                func_name, post.description,
                            ),
                            function_name: Some(func_name.clone()),
                            related_lines: post.return_lines.clone(),
                            related_files: vec![],
                            category: Some("contract_violation".to_string()),
                            parse_quality: None,
                        });
                        break; // One violation per postcondition
                    }
                }
            }

            // Detect "new null return path" — diff adds return None/null to
            // a function whose other returns are all non-null.
            // Even if the overall classification isn't AlwaysNonNull (because
            // the new null made it Nullable), we detect added null lines.
            for post in &postconditions {
                if let PostconditionKind::Nullable {
                    null_lines,
                    value_lines: _,
                } = &post.kind
                {
                    for &nl in null_lines {
                        if diff_info.diff_lines.contains(&nl) {
                            result.findings.push(SliceFinding {
                                algorithm: "contract".to_string(),
                                file: diff_info.file_path.clone(),
                                line: nl,
                                severity: "warning".to_string(),
                                description: format!(
                                    "New `return None/null` path added in '{}'. \
                                     Function has non-null returns on other paths. \
                                     Callers may not handle null.",
                                    func_name,
                                ),
                                function_name: Some(func_name.clone()),
                                related_lines: post.return_lines.clone(),
                                related_files: vec![],
                                category: Some("contract_postcondition_new_null".to_string()),
                                parse_quality: None,
                            });
                        }
                    }
                }
            }
            // Emit postcondition summary
            if !postconditions.is_empty() {
                let post_summary = postconditions
                    .iter()
                    .map(|p| p.description.clone())
                    .collect::<Vec<_>>()
                    .join("; ");

                result.findings.push(SliceFinding {
                    algorithm: "contract".to_string(),
                    file: diff_info.file_path.clone(),
                    line: func_start,
                    severity: "info".to_string(),
                    description: format!("Postconditions for '{}': {}", func_name, post_summary,),
                    function_name: Some(func_name.clone()),
                    related_lines: postconditions
                        .iter()
                        .flat_map(|p| p.return_lines.iter().copied())
                        .collect(),
                    related_files: vec![],
                    category: Some("contract_postcondition".to_string()),
                    parse_quality: None,
                });
            }

            // Emit contract summary as info-level finding (preconditions)
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

                let pre_desc = if postconditions.is_empty() {
                    format!("Contract for '{}': preconditions: {}", func_name, summary)
                } else {
                    let post_summary = postconditions
                        .iter()
                        .map(|p| p.description.clone())
                        .collect::<Vec<_>>()
                        .join("; ");
                    format!(
                        "Contract for '{}': preconditions: {}; postconditions: {}",
                        func_name, summary, post_summary,
                    )
                };

                result.findings.push(SliceFinding {
                    algorithm: "contract".to_string(),
                    file: diff_info.file_path.clone(),
                    line: func_start,
                    severity: "info".to_string(),
                    description: pre_desc,
                    function_name: Some(func_name.clone()),
                    related_lines: preconditions.iter().map(|p| p.guard_line).collect(),
                    related_files: vec![],
                    category: Some("contract".to_string()),
                    parse_quality: None,
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
///
/// Uses tree-sitter node types to avoid false positives from comments or
/// string literals that happen to contain exit keywords.
fn body_has_early_exit(parsed: &ParsedFile, if_node: &Node<'_>) -> bool {
    has_exit_node(parsed, if_node)
}

/// Recursively check if any descendant is an early-exit node.
fn has_exit_node(parsed: &ParsedFile, node: &Node<'_>) -> bool {
    let kind = node.kind();
    // Direct early-exit node types across all supported languages
    if matches!(
        kind,
        "return_statement"
            | "return_expression" // Rust
            | "raise_statement"   // Python
            | "throw_statement" // JS/TS, Java, C++
    ) {
        return true;
    }

    // Rust panic!(), unreachable!() macros
    if kind == "macro_invocation" {
        let text = parsed.node_text(node);
        if text.starts_with("panic!") || text.starts_with("unreachable!") {
            return true;
        }
    }

    // Call expressions: match known exit functions (panic, exit, abort)
    if kind == "call_expression" || kind == "call" || kind == "function_call" {
        if let Some(func_name) = parsed.language.call_function_name(node) {
            let name = parsed.node_text(&func_name);
            if matches!(name, "panic" | "exit" | "abort" | "unreachable" | "error") {
                return true;
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if has_exit_node(parsed, &child) {
            return true;
        }
    }
    false
}

/// Check if a statement is an assert/require call.
///
/// Prefers tree-sitter node kind (Python's `assert_statement`) over text
/// matching. For languages without a dedicated assert node, falls back to
/// checking call function names to avoid matching `assert_valid_input()` etc.
fn is_assert_statement(parsed: &ParsedFile, node: &Node<'_>) -> bool {
    // Python: tree-sitter has a dedicated assert_statement node
    if node.kind() == "assert_statement" {
        return true;
    }

    // Rust: macro_invocation with assert!/debug_assert! prefix
    if node.kind() == "expression_statement" || node.kind() == "macro_invocation" {
        let text = parsed.node_text(node).trim().to_string();
        if text.starts_with("assert!(")
            || text.starts_with("assert_eq!(")
            || text.starts_with("assert_ne!(")
            || text.starts_with("debug_assert!(")
        {
            return true;
        }
    }

    // Other languages: check if it's a call to assert/require.
    // The call may be the node itself, or wrapped in expression_statement.
    if check_assert_call(parsed, node) {
        return true;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if parsed.language.is_call_node(child.kind()) && check_assert_call(parsed, &child) {
            return true;
        }
    }

    false
}

/// Check if a call node calls assert/require.
fn check_assert_call(parsed: &ParsedFile, node: &Node<'_>) -> bool {
    if !parsed.language.is_call_node(node.kind()) {
        return false;
    }
    if let Some(name_node) = parsed.language.call_function_name(node) {
        let name = parsed.node_text(&name_node);
        return matches!(name, "assert" | "require");
    }
    false
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
    // Also Yoda conditions: `NULL == ptr`, `None == x`, etc.
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
        // Yoda: `NULL == ptr`, `None == x`
        if let Some(var) = trimmed
            .strip_prefix(&format!("{} == ", null_lit))
            .or_else(|| trimmed.strip_prefix(&format!("{} is ", null_lit)))
        {
            let var = var.trim();
            if is_simple_var(var) {
                return Some((var.to_string(), ConstraintKind::NonNull));
            }
        }
    }

    // `!ptr` or `!x` (C-style null check)
    if let Some(var) = trimmed.strip_prefix('!') {
        let var = var.trim().trim_start_matches('(').trim_end_matches(')');
        if is_simple_var(var) {
            return Some((var.to_string(), ConstraintKind::NonNull));
        }
    }

    // `ptr == 0` or `ptr == false` — C null pointer check
    // Also Yoda: `0 == ptr`, `false == ptr`
    if let Some(var) = trimmed
        .strip_suffix(" == 0")
        .or_else(|| trimmed.strip_suffix(" == false"))
    {
        let var = var.trim();
        if is_simple_var(var) {
            return Some((var.to_string(), ConstraintKind::NonNull));
        }
    }
    if let Some(var) = trimmed
        .strip_prefix("0 == ")
        .or_else(|| trimmed.strip_prefix("false == "))
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

    // Error/nil check: `err != nil`, `x != null`, `x != None`
    // Also Yoda: `nil != err`, `null != x`, `None != x`
    // These guards exit early when the variable IS non-nil, so the postcondition
    // is that the variable is nil/null after the guard.
    for nil_lit in &["nil", "null", "None"] {
        if let Some(var) = trimmed.strip_suffix(&format!(" != {}", nil_lit)) {
            let var = var.trim();
            if is_simple_var(var) {
                return Some((var.to_string(), ConstraintKind::NilCheck));
            }
        }
        // Yoda: `nil != err`
        if let Some(var) = trimmed.strip_prefix(&format!("{} != ", nil_lit)) {
            let var = var.trim();
            if is_simple_var(var) {
                return Some((var.to_string(), ConstraintKind::NilCheck));
            }
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

// ---------------------------------------------------------------------------
// Phase 2: Postcondition extraction
// ---------------------------------------------------------------------------

/// Classify a return value expression into a ReturnValueClass.
fn classify_return_value(value_text: Option<&str>, language: Language) -> ReturnValueClass {
    let text = match value_text {
        Some(t) => t.trim(),
        None => return ReturnValueClass::Void,
    };

    if text.is_empty() {
        return ReturnValueClass::Void;
    }

    // Null/None/nil returns
    if matches!(text, "None" | "null" | "nil" | "NULL" | "nullptr") {
        return ReturnValueClass::Null;
    }

    // Boolean returns
    if matches!(text, "true" | "false" | "True" | "False") {
        return ReturnValueClass::Bool;
    }

    // Go multi-return patterns: `val, nil` or `nil, err`
    if language == Language::Go {
        return classify_go_multi_return(text);
    }

    // Numeric literals
    if text.parse::<f64>().is_ok()
        || text.starts_with("0x")
        || text.starts_with("0b")
        || text.starts_with("-") && text[1..].parse::<f64>().is_ok()
    {
        return ReturnValueClass::Numeric;
    }

    // String literals
    if (text.starts_with('"') && text.ends_with('"'))
        || (text.starts_with('\'') && text.ends_with('\''))
        || (text.starts_with("f\"") || text.starts_with("f'"))
    {
        return ReturnValueClass::StringLit;
    }

    // Empty collection literals
    if matches!(text, "[]" | "{}" | "()" | "new Array()" | "new Map()") {
        return ReturnValueClass::EmptyCollection;
    }

    // Constructor/factory calls
    if text.starts_with("new ")
        || text.contains("::new(")
        || (text.ends_with("()") && text.chars().next().map_or(false, |c| c.is_uppercase()))
    {
        return ReturnValueClass::ConstructedValue;
    }

    // Variable or expression — could be anything
    ReturnValueClass::Expression
}

/// Classify Go multi-return expressions like `result, nil` or `nil, err`.
fn classify_go_multi_return(text: &str) -> ReturnValueClass {
    let parts: Vec<&str> = text.splitn(2, ',').map(|s| s.trim()).collect();
    if parts.len() == 2 {
        let (val, err) = (parts[0], parts[1]);
        if err == "nil" {
            return ReturnValueClass::GoSuccess;
        }
        if val == "nil" || val == "\"\"" || val == "0" {
            return ReturnValueClass::GoError;
        }
    }
    ReturnValueClass::Expression
}

/// Extract postconditions from return statements in a function.
fn extract_postconditions(parsed: &ParsedFile, func_node: &Node<'_>) -> Vec<Postcondition> {
    let returns = parsed.return_value_nodes(func_node);

    if returns.is_empty() {
        // No explicit returns — check if language requires them
        return match parsed.language {
            Language::Go | Language::Rust | Language::C | Language::Cpp | Language::Java => {
                vec![Postcondition {
                    kind: PostconditionKind::Void,
                    return_lines: vec![],
                    description: "returns: void".to_string(),
                }]
            }
            _ => vec![],
        };
    }

    // Classify each return
    let classified: Vec<(usize, ReturnValueClass)> = returns
        .iter()
        .map(|r| {
            let cls = classify_return_value(r.value_text.as_deref(), parsed.language);
            (r.line, cls)
        })
        .collect();

    // Separate into categories
    let mut null_lines = Vec::new();
    let mut value_lines = Vec::new();
    let mut void_lines = Vec::new();
    let mut bool_lines = Vec::new();
    let mut go_error_lines = Vec::new();
    let mut go_success_lines = Vec::new();
    let mut type_names: Vec<(usize, &str)> = Vec::new();

    for (line, cls) in &classified {
        match cls {
            ReturnValueClass::Null => null_lines.push(*line),
            ReturnValueClass::Void => void_lines.push(*line),
            ReturnValueClass::Bool => {
                bool_lines.push(*line);
                type_names.push((*line, "bool"));
            }
            ReturnValueClass::GoError => go_error_lines.push(*line),
            ReturnValueClass::GoSuccess => go_success_lines.push(*line),
            ReturnValueClass::Numeric => {
                type_names.push((*line, "numeric"));
                value_lines.push(*line);
            }
            ReturnValueClass::StringLit => {
                type_names.push((*line, "string"));
                value_lines.push(*line);
            }
            ReturnValueClass::EmptyCollection => {
                type_names.push((*line, "collection"));
                value_lines.push(*line);
            }
            ReturnValueClass::ConstructedValue => {
                type_names.push((*line, "object"));
                value_lines.push(*line);
            }
            ReturnValueClass::Expression => {
                type_names.push((*line, "expression"));
                value_lines.push(*line);
            }
        }
    }

    let all_lines: Vec<usize> = classified.iter().map(|(l, _)| *l).collect();

    // Determine overall postcondition
    let has_throws = body_has_throw_or_raise(parsed, func_node);

    // All void
    if void_lines.len() == classified.len() {
        return vec![Postcondition {
            kind: PostconditionKind::Void,
            return_lines: all_lines,
            description: "returns: void".to_string(),
        }];
    }

    // All bool
    if bool_lines.len() == classified.len() {
        return vec![Postcondition {
            kind: PostconditionKind::AlwaysBool,
            return_lines: all_lines,
            description: "returns: bool".to_string(),
        }];
    }

    // Go result pair
    if !go_error_lines.is_empty() && !go_success_lines.is_empty() {
        return vec![Postcondition {
            kind: PostconditionKind::GoResultPair,
            return_lines: all_lines,
            description: format!(
                "returns: (T, error) — success on lines {:?}, error on lines {:?}",
                go_success_lines, go_error_lines,
            ),
        }];
    }

    // Check nullable vs always-non-null
    let non_null_lines: Vec<usize> = value_lines
        .iter()
        .chain(bool_lines.iter())
        .chain(go_success_lines.iter())
        .copied()
        .collect();

    if !null_lines.is_empty() && !non_null_lines.is_empty() {
        return vec![Postcondition {
            kind: PostconditionKind::Nullable {
                null_lines: null_lines.clone(),
                value_lines: non_null_lines.clone(),
            },
            return_lines: all_lines,
            description: format!(
                "returns: nullable (None on lines {:?}; value on lines {:?})",
                null_lines, non_null_lines,
            ),
        }];
    }

    if null_lines.is_empty()
        && !non_null_lines.is_empty()
        && void_lines.is_empty()
        && go_error_lines.is_empty()
    {
        // All returns are non-null
        if has_throws {
            return vec![Postcondition {
                kind: PostconditionKind::NonNullOrThrows,
                return_lines: all_lines,
                description: "returns: non-null-or-throws".to_string(),
            }];
        }

        // Check consistent type
        let type_set: BTreeSet<&str> = type_names.iter().map(|(_, t)| *t).collect();
        if type_set.len() == 1 {
            let ty = type_set.into_iter().next().unwrap().to_string();
            if ty != "expression" {
                return vec![Postcondition {
                    kind: PostconditionKind::ConsistentType(ty.clone()),
                    return_lines: all_lines,
                    description: format!("returns: consistent-type ({})", ty),
                }];
            }
        }

        return vec![Postcondition {
            kind: PostconditionKind::AlwaysNonNull,
            return_lines: all_lines,
            description: "returns: non-null".to_string(),
        }];
    }

    // Fallback: mixed
    vec![Postcondition {
        kind: PostconditionKind::Mixed,
        return_lines: all_lines,
        description: "returns: mixed".to_string(),
    }]
}

/// Check if a function body contains raise/throw statements (not inside nested functions).
fn body_has_throw_or_raise(parsed: &ParsedFile, func_node: &Node<'_>) -> bool {
    fn walk_for_throws(parsed: &ParsedFile, node: Node<'_>, func_node: &Node<'_>) -> bool {
        let kind = node.kind();
        if matches!(kind, "raise_statement" | "throw_statement") {
            return true;
        }
        // Don't recurse into nested function definitions
        if parsed.language.function_node_types().contains(&kind)
            && node.start_position() != func_node.start_position()
        {
            return false;
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if walk_for_throws(parsed, child, func_node) {
                return true;
            }
        }
        false
    }
    walk_for_throws(parsed, *func_node, func_node)
}

// ---------------------------------------------------------------------------
// Phase 3: Delta contract comparison
// ---------------------------------------------------------------------------

/// Describes a precondition change between old and new versions of a function.
enum PreconditionChange {
    /// Guard clause removed (weakened).
    Removed {
        variable: String,
        constraint: String,
        old_line: usize,
    },
    /// Guard clause added (strengthened).
    Added {
        variable: String,
        constraint: String,
        new_line: usize,
    },
    /// Guard clause modified (constraint changed).
    Modified {
        variable: String,
        old_constraint: String,
        new_constraint: String,
        new_line: usize,
    },
}

/// Describes a postcondition change between old and new versions.
enum PostconditionChange {
    /// New null return path added (weakened).
    NullPathAdded { new_line: usize },
    /// Null return path removed (strengthened).
    NullPathRemoved { old_line: usize },
    /// Return type changed.
    TypeChanged { old_type: String, new_type: String },
    /// Overall postcondition kind changed.
    KindChanged { old_kind: String, new_kind: String },
}

/// Compare old and new preconditions for a single function.
fn compare_preconditions(
    old_preconds: &[Precondition],
    new_preconds: &[Precondition],
) -> Vec<PreconditionChange> {
    let mut changes = Vec::new();

    // Build variable → (constraint_tag, constraint_desc, line) maps
    let old_map: BTreeMap<&str, (&str, String, usize)> = old_preconds
        .iter()
        .map(|p| {
            (
                p.variable.as_str(),
                (p.constraint.tag(), p.constraint.description(), p.guard_line),
            )
        })
        .collect();

    let new_map: BTreeMap<&str, (&str, String, usize)> = new_preconds
        .iter()
        .map(|p| {
            (
                p.variable.as_str(),
                (p.constraint.tag(), p.constraint.description(), p.guard_line),
            )
        })
        .collect();

    // Removed: in old but not in new
    for (var, (_, desc, old_line)) in &old_map {
        if !new_map.contains_key(var) {
            changes.push(PreconditionChange::Removed {
                variable: var.to_string(),
                constraint: desc.clone(),
                old_line: *old_line,
            });
        }
    }

    // Added: in new but not in old
    for (var, (_, desc, new_line)) in &new_map {
        if !old_map.contains_key(var) {
            changes.push(PreconditionChange::Added {
                variable: var.to_string(),
                constraint: desc.clone(),
                new_line: *new_line,
            });
        }
    }

    // Modified: in both but constraint tag changed
    for (var, (new_tag, new_desc, new_line)) in &new_map {
        if let Some((old_tag, old_desc, _)) = old_map.get(var) {
            if old_tag != new_tag {
                changes.push(PreconditionChange::Modified {
                    variable: var.to_string(),
                    old_constraint: old_desc.clone(),
                    new_constraint: new_desc.clone(),
                    new_line: *new_line,
                });
            }
        }
    }

    changes
}

/// Compare old and new postconditions for a single function.
fn compare_postconditions(
    old_posts: &[Postcondition],
    new_posts: &[Postcondition],
) -> Vec<PostconditionChange> {
    let mut changes = Vec::new();

    let old_kind = old_posts
        .first()
        .map(|p| &p.kind)
        .unwrap_or(&PostconditionKind::Mixed);
    let new_kind = new_posts
        .first()
        .map(|p| &p.kind)
        .unwrap_or(&PostconditionKind::Mixed);

    let old_name = old_kind.kind_name();
    let new_name = new_kind.kind_name();

    if old_name == new_name {
        // Same kind — check for type changes within ConsistentType
        if let (
            PostconditionKind::ConsistentType(old_ty),
            PostconditionKind::ConsistentType(new_ty),
        ) = (old_kind, new_kind)
        {
            if old_ty != new_ty {
                changes.push(PostconditionChange::TypeChanged {
                    old_type: old_ty.clone(),
                    new_type: new_ty.clone(),
                });
            }
        }
        return changes;
    }

    // AlwaysNonNull → Nullable: new null path added (weakened)
    if matches!(
        old_kind,
        PostconditionKind::AlwaysNonNull | PostconditionKind::NonNullOrThrows
    ) && matches!(new_kind, PostconditionKind::Nullable { .. })
    {
        if let PostconditionKind::Nullable { null_lines, .. } = new_kind {
            for &nl in null_lines {
                changes.push(PostconditionChange::NullPathAdded { new_line: nl });
            }
        }
    }

    // Nullable → AlwaysNonNull: null path removed (strengthened)
    if matches!(old_kind, PostconditionKind::Nullable { .. })
        && matches!(
            new_kind,
            PostconditionKind::AlwaysNonNull | PostconditionKind::NonNullOrThrows
        )
    {
        if let PostconditionKind::Nullable { null_lines, .. } = old_kind {
            for &ol in null_lines {
                changes.push(PostconditionChange::NullPathRemoved { old_line: ol });
            }
        }
    }

    // Any other kind change
    changes.push(PostconditionChange::KindChanged {
        old_kind: old_name.to_string(),
        new_kind: new_name.to_string(),
    });

    changes
}

/// Phase 3 entry point: delta contract comparison with `--old-repo`.
///
/// Parses old versions of changed files, matches functions by name,
/// extracts contracts from both versions, and emits findings for changes.
pub fn slice_delta(
    files: &BTreeMap<String, ParsedFile>,
    diff: &DiffInput,
    old_repo: &Path,
) -> Result<SliceResult> {
    // First run Phase 1+2 for the current version
    let mut result = slice(files, diff)?;

    // Parse old versions of changed files
    let mut old_files: BTreeMap<String, ParsedFile> = BTreeMap::new();
    for diff_info in &diff.files {
        let old_path = old_repo.join(&diff_info.file_path);
        if let Ok(source) = fs::read_to_string(&old_path) {
            if let Some(lang) = Language::from_path(&diff_info.file_path) {
                if let Ok(parsed) = ParsedFile::parse(&diff_info.file_path, &source, lang) {
                    old_files.insert(diff_info.file_path.clone(), parsed);
                }
            }
        }
    }

    // For each changed file, compare function contracts
    for diff_info in &diff.files {
        let new_parsed = match files.get(&diff_info.file_path) {
            Some(f) => f,
            None => continue,
        };
        let old_parsed = match old_files.get(&diff_info.file_path) {
            Some(f) => f,
            None => continue,
        };

        let matched = match_functions(old_parsed, new_parsed);

        for (func_name, old_line, new_line) in &matched {
            if func_name == "<anonymous>" {
                continue;
            }

            match (old_line, new_line) {
                (Some(ol), Some(nl)) => {
                    let old_fn = match old_parsed.enclosing_function(*ol) {
                        Some(f) => f,
                        None => continue,
                    };
                    let new_fn = match new_parsed.enclosing_function(*nl) {
                        Some(f) => f,
                        None => continue,
                    };

                    // Function exists in both — compare contracts
                    let (old_start, old_end) = old_parsed.node_line_range(&old_fn);
                    let old_preconds =
                        extract_preconditions(old_parsed, &old_fn, old_start, old_end);
                    let old_posts = extract_postconditions(old_parsed, &old_fn);

                    let (new_start, new_end) = new_parsed.node_line_range(&new_fn);
                    let new_preconds =
                        extract_preconditions(new_parsed, &new_fn, new_start, new_end);
                    let new_posts = extract_postconditions(new_parsed, &new_fn);

                    // Compare preconditions
                    let pre_changes = compare_preconditions(&old_preconds, &new_preconds);
                    for change in &pre_changes {
                        match change {
                            PreconditionChange::Removed {
                                variable,
                                constraint,
                                old_line,
                            } => {
                                result.findings.push(SliceFinding {
                                    algorithm: "contract".to_string(),
                                    file: diff_info.file_path.clone(),
                                    line: new_start,
                                    severity: "warning".to_string(),
                                    description: format!(
                                        "Guard clause removed in '{}': {} constraint on '{}' \
                                         was at old line {}. Callers that relied on '{}' \
                                         rejecting invalid values will now receive \
                                         unvalidated results.",
                                        func_name, constraint, variable, old_line, func_name,
                                    ),
                                    function_name: Some(func_name.clone()),
                                    related_lines: vec![],
                                    related_files: vec![],
                                    category: Some("contract_precondition_weakened".to_string()),
                                    parse_quality: None,
                                });
                            }
                            PreconditionChange::Added {
                                variable,
                                constraint,
                                new_line,
                            } => {
                                result.findings.push(SliceFinding {
                                    algorithm: "contract".to_string(),
                                    file: diff_info.file_path.clone(),
                                    line: *new_line,
                                    severity: "info".to_string(),
                                    description: format!(
                                        "New guard clause in '{}': {} constraint on '{}' \
                                         at line {}. Callers get tighter guarantees.",
                                        func_name, constraint, variable, new_line,
                                    ),
                                    function_name: Some(func_name.clone()),
                                    related_lines: vec![*new_line],
                                    related_files: vec![],
                                    category: Some(
                                        "contract_precondition_strengthened".to_string(),
                                    ),
                                    parse_quality: None,
                                });
                            }
                            PreconditionChange::Modified {
                                variable,
                                old_constraint,
                                new_constraint,
                                new_line,
                            } => {
                                result.findings.push(SliceFinding {
                                    algorithm: "contract".to_string(),
                                    file: diff_info.file_path.clone(),
                                    line: *new_line,
                                    severity: "warning".to_string(),
                                    description: format!(
                                        "Guard constraint changed in '{}': '{}' was {} \
                                         now {} at line {}.",
                                        func_name,
                                        variable,
                                        old_constraint,
                                        new_constraint,
                                        new_line,
                                    ),
                                    function_name: Some(func_name.clone()),
                                    related_lines: vec![*new_line],
                                    related_files: vec![],
                                    category: Some("contract_precondition_weakened".to_string()),
                                    parse_quality: None,
                                });
                            }
                        }
                    }

                    // Compare postconditions
                    let post_changes = compare_postconditions(&old_posts, &new_posts);
                    for change in &post_changes {
                        match change {
                            PostconditionChange::NullPathAdded { new_line } => {
                                result.findings.push(SliceFinding {
                                    algorithm: "contract".to_string(),
                                    file: diff_info.file_path.clone(),
                                    line: *new_line,
                                    severity: "warning".to_string(),
                                    description: format!(
                                        "New null return path in '{}' at line {}. \
                                         Function previously always returned a non-null \
                                         value. Callers may not handle null.",
                                        func_name, new_line,
                                    ),
                                    function_name: Some(func_name.clone()),
                                    related_lines: vec![*new_line],
                                    related_files: vec![],
                                    category: Some("contract_postcondition_weakened".to_string()),
                                    parse_quality: None,
                                });
                            }
                            PostconditionChange::NullPathRemoved { old_line } => {
                                result.findings.push(SliceFinding {
                                    algorithm: "contract".to_string(),
                                    file: diff_info.file_path.clone(),
                                    line: new_start,
                                    severity: "info".to_string(),
                                    description: format!(
                                        "Null return path removed in '{}' (was at old \
                                         line {}). Function now always returns a value. \
                                         Callers that checked for null can simplify.",
                                        func_name, old_line,
                                    ),
                                    function_name: Some(func_name.clone()),
                                    related_lines: vec![],
                                    related_files: vec![],
                                    category: Some(
                                        "contract_postcondition_strengthened".to_string(),
                                    ),
                                    parse_quality: None,
                                });
                            }
                            PostconditionChange::TypeChanged { old_type, new_type } => {
                                result.findings.push(SliceFinding {
                                    algorithm: "contract".to_string(),
                                    file: diff_info.file_path.clone(),
                                    line: new_start,
                                    severity: "warning".to_string(),
                                    description: format!(
                                        "Return type changed in '{}': was {}, now {}.",
                                        func_name, old_type, new_type,
                                    ),
                                    function_name: Some(func_name.clone()),
                                    related_lines: vec![],
                                    related_files: vec![],
                                    category: Some("contract_postcondition_weakened".to_string()),
                                    parse_quality: None,
                                });
                            }
                            PostconditionChange::KindChanged { old_kind, new_kind } => {
                                // Determine if weakened or strengthened
                                let is_weakened = is_postcondition_weakened(old_kind, new_kind);
                                let (severity, category) = if is_weakened {
                                    ("warning", "contract_postcondition_weakened")
                                } else {
                                    ("info", "contract_postcondition_strengthened")
                                };
                                result.findings.push(SliceFinding {
                                    algorithm: "contract".to_string(),
                                    file: diff_info.file_path.clone(),
                                    line: new_start,
                                    severity: severity.to_string(),
                                    description: format!(
                                        "Postcondition changed in '{}': was {}, now {}.",
                                        func_name, old_kind, new_kind,
                                    ),
                                    function_name: Some(func_name.clone()),
                                    related_lines: vec![],
                                    related_files: vec![],
                                    category: Some(category.to_string()),
                                    parse_quality: None,
                                });
                            }
                        }
                    }
                }
                (None, Some(_)) => {
                    // New function — info only (not a contract change)
                }
                (Some(_), None) => {
                    // Function removed — info only
                }
                (None, None) => unreachable!(),
            }
        }
    }

    Ok(result)
}

/// Match functions between old and new versions of a file by name.
///
/// Returns `(func_name, old_line, new_line)` triples, where the line is
/// a line inside the function (used to look up the node via enclosing_function).
/// Both Some → function exists in both versions.
/// Only old Some → function removed. Only new Some → function added.
fn match_functions(
    old_parsed: &ParsedFile,
    new_parsed: &ParsedFile,
) -> Vec<(String, Option<usize>, Option<usize>)> {
    let old_funcs: BTreeMap<String, usize> = old_parsed
        .all_functions()
        .into_iter()
        .filter_map(|f| {
            old_parsed.language.function_name(&f).map(|n| {
                (
                    old_parsed.node_text(&n).to_string(),
                    f.start_position().row + 1,
                )
            })
        })
        .collect();

    let new_funcs: BTreeMap<String, usize> = new_parsed
        .all_functions()
        .into_iter()
        .filter_map(|f| {
            new_parsed.language.function_name(&f).map(|n| {
                (
                    new_parsed.node_text(&n).to_string(),
                    f.start_position().row + 1,
                )
            })
        })
        .collect();

    let all_names: BTreeSet<&str> = old_funcs
        .keys()
        .chain(new_funcs.keys())
        .map(|s| s.as_str())
        .collect();

    all_names
        .into_iter()
        .map(|name| {
            (
                name.to_string(),
                old_funcs.get(name).copied(),
                new_funcs.get(name).copied(),
            )
        })
        .collect()
}

/// Determine if a postcondition kind change represents weakening.
///
/// Weakening means the function's guarantees got looser (callers need to
/// handle more cases). Strengthening means guarantees got tighter (safe).
fn is_postcondition_weakened(old_kind: &str, new_kind: &str) -> bool {
    // Transitions that weaken the contract:
    // always-non-null → nullable, mixed, void
    // non-null-or-throws → nullable, mixed
    // always-bool → mixed
    // consistent-type → mixed
    // void → mixed (adding return values to void function)
    matches!(
        (old_kind, new_kind),
        ("always-non-null", "nullable")
            | ("always-non-null", "mixed")
            | ("always-non-null", "void")
            | ("non-null-or-throws", "nullable")
            | ("non-null-or-throws", "mixed")
            | ("always-bool", "mixed")
            | ("always-bool", "always-non-null")
            | ("consistent-type", "mixed")
            | ("consistent-type", "always-non-null")
            | ("go-result-pair", "mixed")
    )
}
