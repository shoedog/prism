//! Relevant Slice — backward slice plus alternate branch paths.
//!
//! Extends LeftFlow by also including the first N statements of alternate
//! branches for each control flow node in the slice. This catches the
//! "one branch flip away from a bug" scenario: missing else clauses,
//! unhandled switch cases, unchecked error returns.
//!
//! When CFG edges are available in the CPG, uses CFG successor traversal to
//! find alternate paths more precisely than AST pattern matching. Falls back
//! to AST-based detection when no CFG edges exist.

use crate::ast::ParsedFile;
use crate::cpg::{CpgContext, CpgNode, StmtKind};
use crate::diff::DiffInput;
use crate::slice::{SliceConfig, SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use tree_sitter::Node;

/// How many lines of alternate-path code to include.
const DEFAULT_FLIP_DEPTH: usize = 3;

pub fn slice(ctx: &CpgContext, diff: &DiffInput, config: &SliceConfig) -> Result<SliceResult> {
    // Start with LeftFlow as the base
    let mut base = crate::algorithms::left_flow::slice(ctx, diff, config)?;
    base.algorithm = SlicingAlgorithm::RelevantSlice;

    let flip_depth = config.max_branch_lines.max(DEFAULT_FLIP_DEPTH);
    let has_cfg = ctx.cpg.has_cfg_edges();

    // For each block, find control flow nodes and add alternate paths
    for block in &mut base.blocks {
        let file_path = block.file.clone();
        let parsed = match ctx.files.get(&file_path) {
            Some(f) => f,
            None => continue,
        };

        // Collect existing lines in the slice
        let existing_lines: BTreeSet<usize> = block
            .file_line_map
            .get(&file_path)
            .map(|m| m.keys().copied().collect())
            .unwrap_or_default();

        let mut alternate_lines: BTreeMap<usize, bool> = BTreeMap::new();

        // CFG-based alternate path detection: for each Branch/Loop statement in
        // the slice, find CFG successors not already included. Follow each
        // alternate successor forward through the CFG for up to flip_depth
        // statements.
        if has_cfg {
            collect_cfg_alternate_paths(
                ctx,
                &file_path,
                &existing_lines,
                flip_depth,
                &mut alternate_lines,
            );
        }

        // AST-based alternate path detection (original approach). When CFG is
        // available this acts as a complement — it catches structural patterns
        // (missing else, elif) that CFG successor traversal may not surface.
        for &line in &existing_lines {
            if let Some(func_node) = parsed.enclosing_function(line) {
                collect_ast_alternate_branches(
                    parsed,
                    func_node,
                    &existing_lines,
                    flip_depth,
                    &mut alternate_lines,
                );
                break; // Only need to process the function once
            }
        }

        // Add alternate lines to the block
        for (line, is_diff) in &alternate_lines {
            if !existing_lines.contains(line) {
                block.add_line(&file_path, *line, *is_diff);
            }
        }
    }

    Ok(base)
}

// ---------------------------------------------------------------------------
// CFG-based alternate path detection
// ---------------------------------------------------------------------------

/// Find alternate branch paths using CFG successor edges.
///
/// For each Branch or Loop statement in the slice, examines its CFG successors.
/// Successors whose lines are not in the slice represent alternate paths. We
/// follow each alternate forward through the CFG for up to `flip_depth`
/// statements to collect context lines.
fn collect_cfg_alternate_paths(
    ctx: &CpgContext,
    file_path: &str,
    slice_lines: &BTreeSet<usize>,
    flip_depth: usize,
    out: &mut BTreeMap<usize, bool>,
) {
    for &line in slice_lines {
        let stmt_idx = match ctx.cpg.statement_at(file_path, line) {
            Some(idx) => idx,
            None => continue,
        };

        // Only look for alternates on Branch and Loop statements
        match ctx.cpg.node(stmt_idx) {
            CpgNode::Statement {
                kind: StmtKind::Branch,
                ..
            }
            | CpgNode::Statement {
                kind: StmtKind::Loop,
                ..
            } => {}
            _ => continue,
        }

        let successors = ctx.cpg.cfg_successors(stmt_idx);

        // A branch/loop with only one successor has no alternate path
        if successors.len() <= 1 {
            continue;
        }

        for succ_idx in &successors {
            let succ_line = ctx.cpg.node(*succ_idx).line();
            let succ_file = ctx.cpg.node(*succ_idx).file();

            // Only consider same-file successors not already in the slice
            if succ_file != file_path || slice_lines.contains(&succ_line) {
                continue;
            }

            // Follow CFG forward from this alternate entry for up to
            // flip_depth statements
            collect_cfg_path_lines(ctx, *succ_idx, file_path, flip_depth, out);
        }
    }
}

/// BFS forward through CFG from `start`, collecting up to `max_stmts` statement
/// lines into `out`.
fn collect_cfg_path_lines(
    ctx: &CpgContext,
    start: petgraph::graph::NodeIndex,
    file_path: &str,
    max_stmts: usize,
    out: &mut BTreeMap<usize, bool>,
) {
    let mut visited = BTreeSet::new();
    let mut queue = VecDeque::new();
    let mut count = 0;
    queue.push_back(start);

    while let Some(node) = queue.pop_front() {
        if !visited.insert(node) {
            continue;
        }

        let cpg_node = ctx.cpg.node(node);
        if cpg_node.file() == file_path {
            out.entry(cpg_node.line()).or_insert(false);
            count += 1;
            if count >= max_stmts {
                break;
            }
        }

        for succ in ctx.cpg.cfg_successors(node) {
            if !visited.contains(&succ) {
                queue.push_back(succ);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// AST-based alternate path detection (fallback / complement)
// ---------------------------------------------------------------------------

fn collect_ast_alternate_branches(
    parsed: &ParsedFile,
    func_node: Node<'_>,
    slice_lines: &BTreeSet<usize>,
    flip_depth: usize,
    out: &mut BTreeMap<usize, bool>,
) {
    collect_ast_alternates_inner(parsed, func_node, slice_lines, flip_depth, out);
}

fn collect_ast_alternates_inner(
    parsed: &ParsedFile,
    node: Node<'_>,
    slice_lines: &BTreeSet<usize>,
    flip_depth: usize,
    out: &mut BTreeMap<usize, bool>,
) {
    let line = node.start_position().row + 1;

    // If this is a control flow node with a line in the slice
    if parsed.language.is_control_flow_node(node.kind()) && slice_lines.contains(&line) {
        // Find alternative/else branches
        let mut cursor = node.walk();
        let children: Vec<Node<'_>> = node.children(&mut cursor).collect();

        for child in &children {
            let child_kind = child.kind();
            // Look for else clauses, elif, default cases
            if child_kind == "else_clause"
                || child_kind == "elif_clause"
                || child_kind == "else"
                || child_kind == "default_case"
                || child_kind == "switch_default"
            {
                let (alt_start, alt_end) = parsed.node_line_range(child);
                // Include up to flip_depth lines of the alternate path
                let end = alt_end.min(alt_start + flip_depth - 1);
                for l in alt_start..=end {
                    out.entry(l).or_insert(false);
                }
                // Always include the closing line
                out.entry(alt_end).or_insert(false);
            }
        }

        // For if statements without else, note the absence (include the closing brace line)
        if node.kind() == "if_statement" || node.kind() == "if_expression" {
            let has_else = children.iter().any(|c| {
                c.kind() == "else_clause" || c.kind() == "elif_clause" || c.kind() == "else"
            });
            if !has_else {
                // Include the line after the if block ends — the "missing else" zone
                let (_, if_end) = parsed.node_line_range(&node);
                out.entry(if_end).or_insert(false);
            }
        }
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_ast_alternates_inner(parsed, child, slice_lines, flip_depth, out);
    }
}
