//! File-local Step-7 statement classification. Syntax names only: never targets.
use super::*;
use std::cell::OnceCell;

#[derive(Default)]
struct StatementCalls(OnceCell<BTreeMap<usize, Vec<String>>>);

impl StatementCalls {
    fn first(&self, parsed: &ParsedFile, line: usize) -> Option<&String> {
        // Preserve the existing query, capture order and manual fallback by
        // requesting every possible start line in one invocation. This value
        // belongs only to collect_file, not ParsedFile or a persistent cache.
        self.0
            .get_or_init(|| {
                let lines: Vec<_> = (1..=parsed.tree.root_node().end_position().row + 1).collect();
                parsed.call_names_on_lines(&lines)
            })
            .get(&line)
            .and_then(|names| names.first())
    }
}

pub(super) fn collect_file(parsed: &ParsedFile) -> Vec<PendingStatement> {
    let calls = StatementCalls::default();
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    CodePropertyGraph::collect_pending(
        parsed.tree.root_node(),
        &parsed.language.function_node_types(),
        parsed,
        &calls,
        &mut seen,
        &mut out,
    );
    out
}

impl CodePropertyGraph {
    /// Read-only per-file collect: the `collect_function_statements` recursion with
    /// node creation removed. Whole-file `seen` (by line), checked BEFORE classify -
    /// exactly the current `if stmt_index.contains_key(&(file,line)) { continue }`.
    fn collect_pending(
        node: tree_sitter::Node<'_>,
        func_types: &[&str],
        parsed: &ParsedFile,
        calls: &StatementCalls,
        seen: &mut std::collections::BTreeSet<usize>,
        out: &mut Vec<PendingStatement>,
    ) {
        if func_types.contains(&node.kind()) {
            for span in parsed.statement_spans_in_function(&node) {
                if !seen.insert(span.line) {
                    continue; // duplicate (file,line) - skip BEFORE classify, as today
                }
                let kind = classify_stmt_kind(&span.kind, parsed, span.line, calls);
                out.push(PendingStatement {
                    line: span.line,
                    kind,
                    start_byte: span.start_byte,
                    end_byte: span.end_byte,
                });
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            Self::collect_pending(child, func_types, parsed, calls, seen, out);
        }
    }
}

/// Classify a tree-sitter node kind string into a `StmtKind`.
fn classify_stmt_kind(
    kind_str: &str,
    parsed: &ParsedFile,
    line: usize,
    calls: &StatementCalls,
) -> StmtKind {
    if parsed.language.is_return_node(kind_str) {
        return StmtKind::Return;
    }
    if kind_str == "goto_statement" {
        // Extract target label
        let target = CodePropertyGraph::extract_goto_target(parsed, line);
        return StmtKind::Goto {
            target: target.unwrap_or_default(),
        };
    }
    if kind_str == "labeled_statement" {
        let name = CodePropertyGraph::extract_label_name(parsed, line);
        return StmtKind::Label {
            name: name.unwrap_or_default(),
        };
    }
    if parsed.language.is_loop_node(kind_str) {
        return StmtKind::Loop;
    }
    if parsed.language.is_control_flow_node(kind_str) {
        return StmtKind::Branch;
    }
    if parsed.language.is_assignment_node(kind_str) {
        return StmtKind::Assignment;
    }
    if parsed.language.is_declaration_node(kind_str) {
        return StmtKind::Declaration;
    }
    if parsed.language.is_call_node(kind_str) || kind_str == "expression_statement" {
        // expression_statement often wraps a call
        if let Some(callee) = calls.first(parsed, line) {
            return StmtKind::Call {
                callee: callee.clone(),
            };
        }
    }
    StmtKind::Other
}
