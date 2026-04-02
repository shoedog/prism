//! Control Flow Graph construction from tree-sitter AST.
//!
//! Builds intraprocedural CFG edges between statement nodes within each function.
//! The CFG is represented as a list of `CfgEdge` pairs (from_line, to_line) that
//! are later translated to `CpgEdge::ControlFlow` edges in the CPG.
//!
//! **Phase 6 — PR A:** C/C++ focus. Other languages get sequential fall-through
//! only; language-specific patterns (Python for/else, Go defer, Rust `?`) are
//! deferred to PR B.

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
}
