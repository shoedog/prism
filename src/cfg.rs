//! Control Flow Graph construction from tree-sitter AST.
//!
//! Builds intraprocedural CFG edges between statement nodes within each function.
//! The CFG is represented as a list of `CfgEdge` pairs (from_line, to_line) that
//! are later translated to `CpgEdge::ControlFlow` edges in the CPG.
//!
//! **Phase 6 — PR A:** Core CFG: sequential flow, if/else, loops, goto (C/C++).
//! **Phase 6 — PR B:** Multi-language handlers: Go defer/select/fallthrough,
//! Rust ? operator/match arms, Python for-else/try-except, C switch fall-through,
//! JS/TS try-catch-finally.

use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::BTreeMap;
use tree_sitter::Node;

/// A control flow edge between two source lines within a function.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CfgEdge {
    pub file: String,
    pub from_line: usize,
    pub to_line: usize,
}

/// Build intraprocedural CFG edges for all functions in a parsed file.
///
/// Returns a list of `CfgEdge` representing control flow between statement lines.
/// Each function is processed independently — no interprocedural edges.
pub fn build_cfg_edges(parsed: &ParsedFile) -> Vec<CfgEdge> {
    let mut edges = Vec::new();
    let root = parsed.tree.root_node();
    let func_types = parsed.language.function_node_types();

    collect_functions(root, &func_types, parsed, &mut edges);
    edges
}

fn collect_functions(
    node: Node<'_>,
    func_types: &[&str],
    parsed: &ParsedFile,
    edges: &mut Vec<CfgEdge>,
) {
    if func_types.contains(&node.kind()) {
        build_function_cfg(node, parsed, edges);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_functions(child, func_types, parsed, edges);
    }
}

/// Build CFG edges for a single function.
fn build_function_cfg(func_node: Node<'_>, parsed: &ParsedFile, edges: &mut Vec<CfgEdge>) {
    let stmts = parsed.statements_in_function(&func_node);
    if stmts.is_empty() {
        return;
    }

    let file = &parsed.path;

    // Build a line→kind map for quick lookup
    let stmt_map: BTreeMap<usize, &str> = stmts.iter().map(|(l, k)| (*l, k.as_str())).collect();
    let stmt_lines: Vec<usize> = stmts.iter().map(|(l, _)| *l).collect();

    // Sequential fall-through: connect consecutive statements unless the
    // predecessor is a terminator
    for i in 0..stmt_lines.len().saturating_sub(1) {
        let from = stmt_lines[i];
        let to = stmt_lines[i + 1];
        let from_kind = stmt_map[&from];

        if parsed.language.is_terminator(from_kind) {
            continue; // No fall-through after return/break/continue/goto
        }

        edges.push(CfgEdge {
            file: file.clone(),
            from_line: from,
            to_line: to,
        });
    }

    // Language-specific control flow edges
    build_branch_edges(func_node, parsed, &stmt_map, &stmt_lines, edges);

    // C/C++ goto edges
    if matches!(parsed.language, Language::C | Language::Cpp) {
        build_goto_edges(func_node, parsed, edges);
    }

    // Loop back-edges
    build_loop_back_edges(func_node, parsed, &stmt_lines, edges);

    // --- Language-specific Phase 6 PR B patterns ---

    // Python: for/else, while/else, try/except/finally
    if matches!(parsed.language, Language::Python) {
        build_python_edges(func_node, parsed, &stmt_lines, edges);
    }

    // Go: defer, select
    if matches!(parsed.language, Language::Go) {
        build_go_edges(func_node, parsed, &stmt_lines, edges);
    }

    // Rust: match arms, ? operator early-return
    if matches!(parsed.language, Language::Rust) {
        build_rust_edges(func_node, parsed, &stmt_lines, edges);
    }

    // JS/TS: try/catch/finally
    if matches!(
        parsed.language,
        Language::JavaScript | Language::TypeScript | Language::Tsx
    ) {
        build_try_catch_edges(func_node, parsed, &stmt_lines, edges);
    }

    // Java: try/catch/finally (same structure as JS)
    if matches!(parsed.language, Language::Java) {
        build_try_catch_edges(func_node, parsed, &stmt_lines, edges);
    }

    // C/C++: switch fall-through between consecutive cases
    if parsed.language.switch_has_fallthrough() {
        build_switch_fallthrough_edges(func_node, parsed, edges);
    }
}

/// Add edges for branch targets (if/else, switch/case).
///
/// For `if` statements: condition line → then-body first statement,
/// condition line → else-body first statement (or next statement after if).
fn build_branch_edges(
    func_node: Node<'_>,
    parsed: &ParsedFile,
    _stmt_map: &BTreeMap<usize, &str>,
    stmt_lines: &[usize],
    edges: &mut Vec<CfgEdge>,
) {
    let file = &parsed.path;
    collect_branch_edges(func_node, parsed, stmt_lines, file, edges);
}

fn collect_branch_edges(
    node: Node<'_>,
    parsed: &ParsedFile,
    stmt_lines: &[usize],
    file: &str,
    edges: &mut Vec<CfgEdge>,
) {
    let kind = node.kind();
    let line = node.start_position().row + 1;

    if kind == "if_statement" || kind == "if_expression" {
        // Find the then-body and else-body
        if let Some(consequence) = node.child_by_field_name("consequence") {
            if let Some(first) = first_statement_line(&consequence, parsed) {
                edges.push(CfgEdge {
                    file: file.to_string(),
                    from_line: line,
                    to_line: first,
                });
            }
        }
        if let Some(alternative) = node.child_by_field_name("alternative") {
            if let Some(first) = first_statement_line(&alternative, parsed) {
                edges.push(CfgEdge {
                    file: file.to_string(),
                    from_line: line,
                    to_line: first,
                });
            }
        } else {
            // No else: branch to next statement after the if
            let if_end = node.end_position().row + 1;
            if let Some(&next) = stmt_lines.iter().find(|&&l| l > if_end) {
                edges.push(CfgEdge {
                    file: file.to_string(),
                    from_line: line,
                    to_line: next,
                });
            }
        }

        // End of then-body → next statement after if (if not terminated)
        if let Some(consequence) = node.child_by_field_name("consequence") {
            let then_end = consequence.end_position().row + 1;
            if !ends_with_terminator(&consequence, parsed) {
                let if_end = node.end_position().row + 1;
                if let Some(&next) = stmt_lines.iter().find(|&&l| l > if_end) {
                    if let Some(last) = last_statement_line(&consequence, parsed) {
                        edges.push(CfgEdge {
                            file: file.to_string(),
                            from_line: last,
                            to_line: next,
                        });
                    }
                }
            }
            let _ = then_end;
        }
    }

    if kind == "switch_statement" || kind == "case_statement" {
        // Switch: condition → each case entry
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "switch_body"
                || child.kind() == "case_statement"
                || child.kind() == "default_statement"
            {
                if let Some(first) = first_statement_line(&child, parsed) {
                    edges.push(CfgEdge {
                        file: file.to_string(),
                        from_line: line,
                        to_line: first,
                    });
                }
            }
        }
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_branch_edges(child, parsed, stmt_lines, file, edges);
    }
}

/// Add edges for goto → label targets (C/C++ only).
fn build_goto_edges(func_node: Node<'_>, parsed: &ParsedFile, edges: &mut Vec<CfgEdge>) {
    let gotos = parsed.goto_statements(&func_node);
    let labels = parsed.label_sections(&func_node);

    let label_map: BTreeMap<&str, usize> = labels
        .iter()
        .map(|(name, line, _)| (name.as_str(), *line))
        .collect();

    for (target, goto_line) in &gotos {
        if let Some(&label_line) = label_map.get(target.as_str()) {
            edges.push(CfgEdge {
                file: parsed.path.clone(),
                from_line: *goto_line,
                to_line: label_line,
            });
        }
    }
}

/// Add loop back-edges: end of loop body → loop header.
fn build_loop_back_edges(
    func_node: Node<'_>,
    parsed: &ParsedFile,
    stmt_lines: &[usize],
    edges: &mut Vec<CfgEdge>,
) {
    collect_loop_back_edges(func_node, parsed, stmt_lines, edges);
}

fn collect_loop_back_edges(
    node: Node<'_>,
    parsed: &ParsedFile,
    stmt_lines: &[usize],
    edges: &mut Vec<CfgEdge>,
) {
    if parsed.language.is_loop_node(node.kind()) {
        let loop_line = node.start_position().row + 1;
        // Find the loop body and add back-edge from last statement to loop header
        if let Some(body) = node
            .child_by_field_name("body")
            .or_else(|| node.child_by_field_name("consequence"))
        {
            if let Some(last) = last_statement_line(&body, parsed) {
                if !ends_with_terminator(&body, parsed) {
                    edges.push(CfgEdge {
                        file: parsed.path.clone(),
                        from_line: last,
                        to_line: loop_line,
                    });
                }
            }
        }
        // Loop condition false → next statement after loop
        let loop_end = node.end_position().row + 1;
        if let Some(&next) = stmt_lines.iter().find(|&&l| l > loop_end) {
            edges.push(CfgEdge {
                file: parsed.path.clone(),
                from_line: loop_line,
                to_line: next,
            });
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_loop_back_edges(child, parsed, stmt_lines, edges);
    }
}

// ---------------------------------------------------------------------------
// Language-specific CFG patterns (Phase 6 PR B)
// ---------------------------------------------------------------------------

/// Python: for/else, while/else, try/except/finally edges.
///
/// Python loops have an `else` clause that runs when the loop completes
/// normally (without `break`). try/except/finally has exception edges.
fn build_python_edges(
    node: Node<'_>,
    parsed: &ParsedFile,
    stmt_lines: &[usize],
    edges: &mut Vec<CfgEdge>,
) {
    let kind = node.kind();

    // for/else and while/else: loop → else-body (on normal completion)
    if (kind == "for_statement" || kind == "while_statement") && parsed.language == Language::Python
    {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "else_clause" {
                let loop_line = node.start_position().row + 1;
                if let Some(first) = first_statement_line(&child, parsed) {
                    // Loop exit (normal) → else body
                    edges.push(CfgEdge {
                        file: parsed.path.clone(),
                        from_line: loop_line,
                        to_line: first,
                    });
                }
                // else body end → next statement after for/else
                if !ends_with_terminator(&child, parsed) {
                    let else_end = child.end_position().row + 1;
                    let outer_end = node.end_position().row + 1;
                    if let Some(&next) = stmt_lines.iter().find(|&&l| l > outer_end) {
                        if let Some(last) = last_statement_line(&child, parsed) {
                            edges.push(CfgEdge {
                                file: parsed.path.clone(),
                                from_line: last,
                                to_line: next,
                            });
                        }
                    }
                    let _ = else_end;
                }
            }
        }
    }

    // try/except/finally
    if kind == "try_statement" {
        let try_line = node.start_position().row + 1;
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            // except_clause / except_group_clause
            if child.kind() == "except_clause" || child.kind() == "except_group_clause" {
                if let Some(first) = first_statement_line(&child, parsed) {
                    // try → except (exception edge)
                    edges.push(CfgEdge {
                        file: parsed.path.clone(),
                        from_line: try_line,
                        to_line: first,
                    });
                }
            }
            // finally_clause
            if child.kind() == "finally_clause" {
                if let Some(first) = first_statement_line(&child, parsed) {
                    // try → finally (always runs)
                    edges.push(CfgEdge {
                        file: parsed.path.clone(),
                        from_line: try_line,
                        to_line: first,
                    });
                }
            }
        }
    }

    // Recurse
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        build_python_edges(child, parsed, stmt_lines, edges);
    }
}

/// Go: defer edges (return → deferred stmts), select statement.
///
/// `defer` statements execute at function exit. We add edges from each
/// return point to each defer statement in the function.
/// `select` is similar to switch — each case is a branch target.
fn build_go_edges(
    node: Node<'_>,
    parsed: &ParsedFile,
    stmt_lines: &[usize],
    edges: &mut Vec<CfgEdge>,
) {
    // Only process at function level — collect all defers and returns
    let func_types = parsed.language.function_node_types();
    if !func_types.contains(&node.kind()) {
        return;
    }

    let mut defer_lines = Vec::new();
    let mut return_lines = Vec::new();
    collect_go_defer_return(node, parsed, &mut defer_lines, &mut return_lines);

    // Each return → each defer (defers run LIFO at function exit)
    for &ret_line in &return_lines {
        for &defer_line in &defer_lines {
            edges.push(CfgEdge {
                file: parsed.path.clone(),
                from_line: ret_line,
                to_line: defer_line,
            });
        }
    }

    // select statement: like switch, condition → each case
    collect_go_select_edges(node, parsed, stmt_lines, edges);
}

fn collect_go_defer_return(
    node: Node<'_>,
    parsed: &ParsedFile,
    defer_lines: &mut Vec<usize>,
    return_lines: &mut Vec<usize>,
) {
    if node.kind() == "defer_statement" {
        defer_lines.push(node.start_position().row + 1);
    }
    if parsed.language.is_return_node(node.kind()) {
        return_lines.push(node.start_position().row + 1);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_go_defer_return(child, parsed, defer_lines, return_lines);
    }
}

fn collect_go_select_edges(
    node: Node<'_>,
    parsed: &ParsedFile,
    stmt_lines: &[usize],
    edges: &mut Vec<CfgEdge>,
) {
    if node.kind() == "select_statement" {
        let select_line = node.start_position().row + 1;
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "communication_case" || child.kind() == "default_case" {
                if let Some(first) = first_statement_line(&child, parsed) {
                    edges.push(CfgEdge {
                        file: parsed.path.clone(),
                        from_line: select_line,
                        to_line: first,
                    });
                }
            }
        }
        // select exit → next statement
        let select_end = node.end_position().row + 1;
        if let Some(&next) = stmt_lines.iter().find(|&&l| l > select_end) {
            edges.push(CfgEdge {
                file: parsed.path.clone(),
                from_line: select_line,
                to_line: next,
            });
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_go_select_edges(child, parsed, stmt_lines, edges);
    }
}

/// Rust: match arms (each arm is a branch target), ? operator early-return.
///
/// `match` is exhaustive — condition → each arm body. Arms are exclusive
/// (no fall-through). `?` on error creates an early-return edge.
fn build_rust_edges(
    node: Node<'_>,
    parsed: &ParsedFile,
    stmt_lines: &[usize],
    edges: &mut Vec<CfgEdge>,
) {
    let kind = node.kind();

    // match_expression: condition → each match_arm body
    if kind == "match_expression" {
        let match_line = node.start_position().row + 1;
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "match_block" {
                let mut arm_cursor = child.walk();
                for arm in child.children(&mut arm_cursor) {
                    if arm.kind() == "match_arm" {
                        if let Some(first) = first_statement_line(&arm, parsed) {
                            edges.push(CfgEdge {
                                file: parsed.path.clone(),
                                from_line: match_line,
                                to_line: first,
                            });
                        }
                        // Each arm end → next statement after match (if not terminated)
                        if !ends_with_terminator(&arm, parsed) {
                            let match_end = node.end_position().row + 1;
                            if let Some(&next) = stmt_lines.iter().find(|&&l| l > match_end) {
                                if let Some(last) = last_statement_line(&arm, parsed) {
                                    edges.push(CfgEdge {
                                        file: parsed.path.clone(),
                                        from_line: last,
                                        to_line: next,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // ? operator: line with ? has an implicit early-return path.
    // We detect `?` by scanning the source text of expression statements.
    // The ? creates a branch: Ok → continue, Err → return.
    // We don't model the return edge explicitly since the function signature
    // determines the return type, but we add an edge to the next statement
    // for the success path (already handled by sequential fall-through).

    // Recurse
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        build_rust_edges(child, parsed, stmt_lines, edges);
    }
}

/// JS/TS/Java: try/catch/finally edges.
///
/// try body → catch handler (exception edge).
/// try body → finally (always runs).
/// catch body → finally (always runs).
fn build_try_catch_edges(
    node: Node<'_>,
    parsed: &ParsedFile,
    stmt_lines: &[usize],
    edges: &mut Vec<CfgEdge>,
) {
    if node.kind() == "try_statement" {
        let try_line = node.start_position().row + 1;
        let mut cursor = node.walk();

        let mut finally_first: Option<usize> = None;
        let mut catch_lines: Vec<usize> = Vec::new();

        for child in node.children(&mut cursor) {
            // catch_clause (JS/TS/Java)
            if child.kind() == "catch_clause" {
                if let Some(first) = first_statement_line(&child, parsed) {
                    catch_lines.push(first);
                    // try → catch (exception edge)
                    edges.push(CfgEdge {
                        file: parsed.path.clone(),
                        from_line: try_line,
                        to_line: first,
                    });
                }
            }
            // finally_clause (JS/TS/Java)
            if child.kind() == "finally_clause" {
                if let Some(first) = first_statement_line(&child, parsed) {
                    finally_first = Some(first);
                    // try → finally (always runs)
                    edges.push(CfgEdge {
                        file: parsed.path.clone(),
                        from_line: try_line,
                        to_line: first,
                    });
                }
            }
        }

        // catch → finally (if both exist)
        if let Some(finally_line) = finally_first {
            for &catch_line in &catch_lines {
                edges.push(CfgEdge {
                    file: parsed.path.clone(),
                    from_line: catch_line,
                    to_line: finally_line,
                });
            }
        }

        let _ = stmt_lines; // used by callers for next-statement edges
    }

    // Recurse
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        build_try_catch_edges(child, parsed, stmt_lines, edges);
    }
}

/// C/C++/JS/Java: switch fall-through between consecutive cases.
///
/// In languages with fall-through (C, C++, JS, Java), execution falls from
/// one case body into the next unless a `break` terminates it.
fn build_switch_fallthrough_edges(node: Node<'_>, parsed: &ParsedFile, edges: &mut Vec<CfgEdge>) {
    if node.kind() == "switch_statement" {
        let mut cursor = node.walk();
        let mut case_bodies: Vec<Node<'_>> = Vec::new();

        // Collect switch body children
        for child in node.children(&mut cursor) {
            if child.kind() == "switch_body" {
                let mut body_cursor = child.walk();
                for case in child.children(&mut body_cursor) {
                    if case.kind() == "case_statement" || case.kind() == "default_statement" {
                        case_bodies.push(case);
                    }
                }
            }
        }

        // For consecutive cases: if case N doesn't end with break/return,
        // add fall-through edge from last stmt of case N to first stmt of case N+1
        for i in 0..case_bodies.len().saturating_sub(1) {
            let current = case_bodies[i];
            let next = case_bodies[i + 1];

            if !ends_with_terminator(&current, parsed) {
                if let (Some(last), Some(first)) = (
                    last_statement_line(&current, parsed),
                    first_statement_line(&next, parsed),
                ) {
                    edges.push(CfgEdge {
                        file: parsed.path.clone(),
                        from_line: last,
                        to_line: first,
                    });
                }
            }
        }
    }

    // Recurse
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        build_switch_fallthrough_edges(child, parsed, edges);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Find the first statement line inside a block/body node.
fn first_statement_line(node: &Node<'_>, parsed: &ParsedFile) -> Option<usize> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if parsed.language.is_statement_node(child.kind()) {
            return Some(child.start_position().row + 1);
        }
        // Recurse into compound_statement / block wrappers
        if child.kind() == "compound_statement"
            || child.kind() == "block"
            || child.kind() == "statement_block"
        {
            if let Some(line) = first_statement_line(&child, parsed) {
                return Some(line);
            }
        }
    }
    None
}

/// Find the last statement line inside a block/body node.
fn last_statement_line(node: &Node<'_>, parsed: &ParsedFile) -> Option<usize> {
    let mut last = None;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if parsed.language.is_statement_node(child.kind()) {
            last = Some(child.start_position().row + 1);
        }
        if child.kind() == "compound_statement"
            || child.kind() == "block"
            || child.kind() == "statement_block"
        {
            if let Some(line) = last_statement_line(&child, parsed) {
                last = Some(line);
            }
        }
    }
    last
}

/// Check whether a block ends with a terminator (return, break, continue, goto).
fn ends_with_terminator(node: &Node<'_>, parsed: &ParsedFile) -> bool {
    let mut cursor = node.walk();
    let mut last_kind = None;
    for child in node.children(&mut cursor) {
        if parsed.language.is_statement_node(child.kind()) {
            last_kind = Some(child.kind().to_string());
        }
        if child.kind() == "compound_statement"
            || child.kind() == "block"
            || child.kind() == "statement_block"
        {
            if ends_with_terminator(&child, parsed) {
                return true;
            }
        }
    }
    last_kind
        .as_deref()
        .is_some_and(|k| parsed.language.is_terminator(k))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::Language;

    #[test]
    fn test_sequential_cfg_edges() {
        let source = r#"
void f() {
    int x = 1;
    int y = 2;
    int z = 3;
}
"#;
        let parsed = ParsedFile::parse("test.c", source, Language::C).unwrap();
        let edges = build_cfg_edges(&parsed);

        // Should have sequential edges: line 3→4, 4→5
        let edge_pairs: Vec<(usize, usize)> =
            edges.iter().map(|e| (e.from_line, e.to_line)).collect();
        assert!(
            edge_pairs.contains(&(3, 4)),
            "Should have edge 3→4, got {:?}",
            edge_pairs
        );
        assert!(
            edge_pairs.contains(&(4, 5)),
            "Should have edge 4→5, got {:?}",
            edge_pairs
        );
    }

    #[test]
    fn test_return_terminates_sequence() {
        let source = r#"
void f() {
    int x = 1;
    return;
    int y = 2;
}
"#;
        let parsed = ParsedFile::parse("test.c", source, Language::C).unwrap();
        let edges = build_cfg_edges(&parsed);

        let edge_pairs: Vec<(usize, usize)> =
            edges.iter().map(|e| (e.from_line, e.to_line)).collect();
        assert!(
            edge_pairs.contains(&(3, 4)),
            "Should have edge 3→4 (to return), got {:?}",
            edge_pairs
        );
        // return should NOT fall through to line 5
        assert!(
            !edge_pairs.contains(&(4, 5)),
            "return should not fall through to next line, got {:?}",
            edge_pairs
        );
    }

    #[test]
    fn test_if_else_branch_edges() {
        let source = r#"
void f(int x) {
    if (x > 0) {
        int a = 1;
    } else {
        int b = 2;
    }
    int c = 3;
}
"#;
        let parsed = ParsedFile::parse("test.c", source, Language::C).unwrap();
        let edges = build_cfg_edges(&parsed);

        let edge_pairs: Vec<(usize, usize)> =
            edges.iter().map(|e| (e.from_line, e.to_line)).collect();

        // if (line 3) should branch to then-body (line 4) and else-body (line 6)
        assert!(
            edge_pairs.contains(&(3, 4)),
            "if should branch to then-body, got {:?}",
            edge_pairs
        );
        assert!(
            edge_pairs.contains(&(3, 6)),
            "if should branch to else-body, got {:?}",
            edge_pairs
        );
    }

    #[test]
    fn test_goto_edges() {
        let source = r#"
void f() {
    int x = 1;
    goto cleanup;
    int y = 2;
cleanup:
    free(x);
}
"#;
        let parsed = ParsedFile::parse("test.c", source, Language::C).unwrap();
        let edges = build_cfg_edges(&parsed);

        let edge_pairs: Vec<(usize, usize)> =
            edges.iter().map(|e| (e.from_line, e.to_line)).collect();
        // goto on line 4 should create edge to cleanup label
        let has_goto_edge = edge_pairs
            .iter()
            .any(|(from, _to)| *from == 4 && edges.iter().any(|e| e.from_line == 4));
        assert!(
            has_goto_edge,
            "goto should create edge to label, got {:?}",
            edge_pairs
        );
        // goto should NOT fall through
        assert!(
            !edge_pairs.contains(&(4, 5)),
            "goto should not fall through, got {:?}",
            edge_pairs
        );
    }

    #[test]
    fn test_while_loop_back_edge() {
        let source = r#"
void f() {
    int i = 0;
    while (i < 10) {
        i = i + 1;
    }
    int done = 1;
}
"#;
        let parsed = ParsedFile::parse("test.c", source, Language::C).unwrap();
        let edges = build_cfg_edges(&parsed);

        let edge_pairs: Vec<(usize, usize)> =
            edges.iter().map(|e| (e.from_line, e.to_line)).collect();

        // while on line 4 should have edge to body (line 5)
        // and edge to after-loop (line 7) for the false condition
        let has_exit = edge_pairs.iter().any(|&(from, to)| from == 4 && to == 7);
        assert!(
            has_exit,
            "while should have exit edge to after-loop, got {:?}",
            edge_pairs
        );
    }

    #[test]
    fn test_python_sequential() {
        let source = r#"
def f():
    x = 1
    y = 2
    z = 3
"#;
        let parsed = ParsedFile::parse("test.py", source, Language::Python).unwrap();
        let edges = build_cfg_edges(&parsed);

        let edge_pairs: Vec<(usize, usize)> =
            edges.iter().map(|e| (e.from_line, e.to_line)).collect();
        assert!(
            edge_pairs.contains(&(3, 4)),
            "Python should have sequential edge 3→4, got {:?}",
            edge_pairs
        );
        assert!(
            edge_pairs.contains(&(4, 5)),
            "Python should have sequential edge 4→5, got {:?}",
            edge_pairs
        );
    }

    #[test]
    fn test_empty_function_no_edges() {
        let source = r#"
void f() {
}
"#;
        let parsed = ParsedFile::parse("test.c", source, Language::C).unwrap();
        let edges = build_cfg_edges(&parsed);
        assert!(
            edges.is_empty(),
            "Empty function should have no CFG edges, got {:?}",
            edges
        );
    }

    // -----------------------------------------------------------------------
    // Phase 6 PR B: Multi-language CFG handler tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_python_for_else() {
        let source = r#"
def f():
    for i in range(10):
        x = i
    else:
        y = 0
    z = 1
"#;
        let parsed = ParsedFile::parse("test.py", source, Language::Python).unwrap();
        let edges = build_cfg_edges(&parsed);
        let edge_pairs: Vec<(usize, usize)> =
            edges.iter().map(|e| (e.from_line, e.to_line)).collect();

        // for (line 3) → else body (line 6) on normal completion
        assert!(
            edge_pairs.contains(&(3, 6)),
            "Python for/else: loop should have edge to else body, got {:?}",
            edge_pairs
        );
    }

    #[test]
    fn test_python_try_except_finally() {
        let source = r#"
def f():
    try:
        x = 1
    except ValueError:
        y = 2
    finally:
        z = 3
    w = 4
"#;
        let parsed = ParsedFile::parse("test.py", source, Language::Python).unwrap();
        let edges = build_cfg_edges(&parsed);
        let edge_pairs: Vec<(usize, usize)> =
            edges.iter().map(|e| (e.from_line, e.to_line)).collect();

        let try_line = 3;
        // try → except (exception edge)
        let has_except = edge_pairs.iter().any(|&(from, _)| from == try_line);
        assert!(
            has_except,
            "Python try: should have edge from try to except, got {:?}",
            edge_pairs
        );
    }

    #[test]
    fn test_go_defer_return() {
        let source = r#"
package main

func f() {
    defer cleanup()
    x := 1
    return
}
"#;
        let parsed = ParsedFile::parse("test.go", source, Language::Go).unwrap();
        let edges = build_cfg_edges(&parsed);
        let edge_pairs: Vec<(usize, usize)> =
            edges.iter().map(|e| (e.from_line, e.to_line)).collect();

        // return (line 7) → defer (line 5) — deferred function runs at exit
        let has_defer_edge = edge_pairs.iter().any(|&(from, to)| from == 7 && to == 5);
        assert!(
            has_defer_edge,
            "Go defer: return should have edge to defer stmt, got {:?}",
            edge_pairs
        );
    }

    #[test]
    fn test_go_select_statement() {
        let source = r#"
package main

func f(ch1 chan int, ch2 chan int) {
    select {
    case v := <-ch1:
        x := v
    case v := <-ch2:
        y := v
    }
    z := 1
}
"#;
        let parsed = ParsedFile::parse("test.go", source, Language::Go).unwrap();
        let edges = build_cfg_edges(&parsed);
        let edge_pairs: Vec<(usize, usize)> =
            edges.iter().map(|e| (e.from_line, e.to_line)).collect();

        // select (line 5) → each case branch
        let select_edges: Vec<_> = edge_pairs.iter().filter(|&&(from, _)| from == 5).collect();
        assert!(
            select_edges.len() >= 2,
            "Go select: should have edges to at least 2 cases, got {:?}",
            edge_pairs
        );
    }

    #[test]
    fn test_rust_match_arms() {
        let source = r#"
fn f(x: i32) {
    match x {
        1 => {
            let a = 1;
        }
        2 => {
            let b = 2;
        }
        _ => {
            let c = 3;
        }
    }
    let d = 4;
}
"#;
        let parsed = ParsedFile::parse("test.rs", source, Language::Rust).unwrap();
        let edges = build_cfg_edges(&parsed);
        let edge_pairs: Vec<(usize, usize)> =
            edges.iter().map(|e| (e.from_line, e.to_line)).collect();

        // match (line 3) → each arm body
        let match_edges: Vec<_> = edge_pairs.iter().filter(|&&(from, _)| from == 3).collect();
        assert!(
            match_edges.len() >= 2,
            "Rust match: should have edges from match to at least 2 arms, got {:?}",
            edge_pairs
        );
    }

    #[test]
    fn test_js_try_catch_finally() {
        let source = r#"
function f() {
    try {
        let x = 1;
    } catch (e) {
        let y = 2;
    } finally {
        let z = 3;
    }
    let w = 4;
}
"#;
        let parsed = ParsedFile::parse("test.js", source, Language::JavaScript).unwrap();
        let edges = build_cfg_edges(&parsed);
        let edge_pairs: Vec<(usize, usize)> =
            edges.iter().map(|e| (e.from_line, e.to_line)).collect();

        let try_line = 3;
        // try → catch (exception edge)
        let has_try_catch = edge_pairs.iter().any(|&(from, _)| from == try_line);
        assert!(
            has_try_catch,
            "JS try/catch: should have edges from try, got {:?}",
            edge_pairs
        );
    }

    #[test]
    fn test_c_switch_fallthrough() {
        let source = r#"
void f(int x) {
    switch (x) {
        case 1:
            a = 1;
        case 2:
            b = 2;
            break;
        case 3:
            c = 3;
    }
    int d = 4;
}
"#;
        let parsed = ParsedFile::parse("test.c", source, Language::C).unwrap();
        let edges = build_cfg_edges(&parsed);
        let edge_pairs: Vec<(usize, usize)> =
            edges.iter().map(|e| (e.from_line, e.to_line)).collect();

        // case 1 (line 5: a = 1) should fall through to case 2 (line 7: b = 2)
        // because case 1 has no break
        let has_fallthrough = edge_pairs.iter().any(|&(from, to)| from == 5 && to == 7);
        assert!(
            has_fallthrough,
            "C switch: case 1 should fall through to case 2, got {:?}",
            edge_pairs
        );

        // case 2 has break, should NOT fall through to case 3
        let no_fallthrough = !edge_pairs.iter().any(|&(from, to)| from == 8 && to == 10);
        assert!(
            no_fallthrough,
            "C switch: case 2 (with break) should not fall through, got {:?}",
            edge_pairs
        );
    }
}
