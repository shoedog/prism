//! Conditioned Slice — backward slice under a specific value assumption.
//!
//! Given a predicate like `var == value` or `var != null`, performs a backward
//! slice but prunes branches that are unreachable under that condition. This
//! focuses the slice on a specific execution scenario.
//!
//! When CFG edges are available in the CPG, uses CFG reachability to refine
//! the unreachable set: lines only reachable through the pruned branch are
//! also removed. Falls back to AST-range-based pruning when no CFG is present.

use crate::ast::ParsedFile;
use crate::cpg::{CpgContext, CpgEdge};
use crate::diff::DiffInput;
use crate::slice::{SliceConfig, SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::BTreeSet;

/// A condition predicate for conditioned slicing.
#[derive(Debug, Clone)]
pub struct Condition {
    pub var_name: String,
    pub op: ConditionOp,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConditionOp {
    Eq,
    NotEq,
    Gt,
    Lt,
    GtEq,
    LtEq,
    IsNull,
    IsNotNull,
}

impl Condition {
    /// Parse a condition string like "x==5", "x!=null", "x>0".
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();

        // Try two-char operators first
        for (op_str, op) in &[
            ("!=", ConditionOp::NotEq),
            ("==", ConditionOp::Eq),
            (">=", ConditionOp::GtEq),
            ("<=", ConditionOp::LtEq),
            (">", ConditionOp::Gt),
            ("<", ConditionOp::Lt),
        ] {
            if let Some(idx) = s.find(op_str) {
                let var_name = s[..idx].trim().to_string();
                let value = s[idx + op_str.len()..].trim().to_string();

                let op = if (value == "null" || value == "None" || value == "nil")
                    && *op == ConditionOp::Eq
                {
                    ConditionOp::IsNull
                } else if (value == "null" || value == "None" || value == "nil")
                    && *op == ConditionOp::NotEq
                {
                    ConditionOp::IsNotNull
                } else {
                    op.clone()
                };

                return Some(Condition {
                    var_name,
                    op,
                    value,
                });
            }
        }

        None
    }

    /// Check if a condition text in source code is made unreachable by this condition.
    fn makes_unreachable(&self, condition_text: &str) -> Option<bool> {
        let ct = condition_text.trim();

        // Simple pattern matching for common cases
        match &self.op {
            ConditionOp::Eq | ConditionOp::IsNull => {
                // If we know var == value, then `if var != value` is unreachable (the if body)
                // and `if var == value` always taken (else is unreachable)
                if ct.contains(&format!("{} == {}", self.var_name, self.value))
                    || ct.contains(&format!("{}=={}", self.var_name, self.value))
                {
                    return Some(false); // Condition is always true → else is unreachable
                }
                if ct.contains(&format!("{} != {}", self.var_name, self.value))
                    || ct.contains(&format!("{}!={}", self.var_name, self.value))
                {
                    return Some(true); // Condition is always false → if body unreachable
                }
            }
            ConditionOp::NotEq | ConditionOp::IsNotNull => {
                if ct.contains(&format!("{} == {}", self.var_name, self.value))
                    || ct.contains(&format!("{}=={}", self.var_name, self.value))
                {
                    return Some(true); // if body unreachable
                }
                if ct.contains(&format!("{} != {}", self.var_name, self.value))
                    || ct.contains(&format!("{}!={}", self.var_name, self.value))
                {
                    return Some(false); // else unreachable
                }
            }
            ConditionOp::Gt => {
                if ct.contains(&format!("{} > 0", self.var_name))
                    || ct.contains(&format!("{}>0", self.var_name))
                {
                    if let Ok(v) = self.value.parse::<i64>() {
                        return Some(v <= 0); // if v > 0, condition is true
                    }
                }
            }
            _ => {}
        }

        None // Can't determine
    }
}

pub fn slice(
    ctx: &CpgContext,
    diff: &DiffInput,
    config: &SliceConfig,
    condition: &Condition,
) -> Result<SliceResult> {
    // Start with LeftFlow
    let mut base = crate::algorithms::left_flow::slice(ctx, diff, config)?;
    base.algorithm = SlicingAlgorithm::ConditionedSlice;

    let has_cfg = ctx.cpg.has_cfg_edges();

    // Prune branches that are unreachable under the condition
    for block in &mut base.blocks {
        let file_path = block.file.clone();
        let parsed = match ctx.files.get(&file_path) {
            Some(f) => f,
            None => continue,
        };

        // AST-based: find lines in branches made unreachable by the condition
        let ast_unreachable = find_ast_unreachable_lines(parsed, condition);

        // CFG-enhanced: refine the unreachable set using CFG reachability
        let lines_to_remove = if has_cfg {
            refine_unreachable_with_cfg(ctx, &file_path, parsed, condition, &ast_unreachable)
        } else {
            ast_unreachable
        };

        if let Some(line_map) = block.file_line_map.get_mut(&file_path) {
            for line in &lines_to_remove {
                line_map.remove(line);
            }
        }
        block.diff_lines.retain(|l| !lines_to_remove.contains(l));
    }

    Ok(base)
}

// ---------------------------------------------------------------------------
// CFG-enhanced unreachable detection
// ---------------------------------------------------------------------------

/// Refine AST-based unreachable lines using CFG reachability analysis.
///
/// The AST approach marks all lines within the AST range of an unreachable
/// branch. The CFG approach improves this in two ways:
///
/// 1. **Dominance expansion**: Lines outside the AST branch range that are only
///    CFG-reachable through the pruned branch are also unreachable.
///
/// 2. **Merge point preservation**: Lines in the AST range that are also
///    reachable from other paths (e.g., after a branch merge point) are NOT
///    unreachable — the AST approach over-prunes these.
fn refine_unreachable_with_cfg(
    ctx: &CpgContext,
    file_path: &str,
    parsed: &ParsedFile,
    condition: &Condition,
    ast_unreachable: &BTreeSet<usize>,
) -> BTreeSet<usize> {
    let mut result = BTreeSet::new();

    // Find branch statement nodes where the condition resolves the branch
    let branch_info = find_resolved_branches(parsed, condition);

    for (branch_line, unreachable_entry_line, unreachable_end_line) in &branch_info {
        // Find the Branch statement node in the CFG
        let branch_stmt = match ctx.cpg.statement_at(file_path, *branch_line) {
            Some(idx) => idx,
            None => {
                // No CFG node at branch — fall back to AST range
                for l in *unreachable_entry_line..=*unreachable_end_line {
                    result.insert(l);
                }
                continue;
            }
        };

        // Find the CFG successor leading into the unreachable branch
        let successors = ctx.cpg.cfg_successors(branch_stmt);
        let unreachable_entry = successors.iter().find(|&&succ| {
            let succ_line = ctx.cpg.node(succ).line();
            succ_line >= *unreachable_entry_line && succ_line <= *unreachable_end_line
        });

        let unreachable_entry = match unreachable_entry {
            Some(&idx) => idx,
            None => {
                // CFG successor doesn't match — fall back to AST range
                for l in *unreachable_entry_line..=*unreachable_end_line {
                    result.insert(l);
                }
                continue;
            }
        };

        // Compute all CFG-reachable lines from the unreachable branch entry
        let branch_reachable = ctx
            .cpg
            .reachable_forward(unreachable_entry, &|e| matches!(e, CpgEdge::ControlFlow));

        // Collect lines reachable from the unreachable branch entry
        let mut branch_reachable_lines: BTreeSet<usize> = BTreeSet::new();
        branch_reachable_lines.insert(ctx.cpg.node(unreachable_entry).line());
        for &idx in &branch_reachable {
            if ctx.cpg.node(idx).file() == file_path {
                branch_reachable_lines.insert(ctx.cpg.node(idx).line());
            }
        }

        // Find the "taken" successor(s) — those NOT in the unreachable range
        let taken_successors: Vec<_> = successors
            .iter()
            .filter(|&&succ| {
                let sl = ctx.cpg.node(succ).line();
                sl < *unreachable_entry_line || sl > *unreachable_end_line
            })
            .collect();

        // Compute lines reachable from the taken branch(es)
        let mut taken_reachable_lines: BTreeSet<usize> = BTreeSet::new();
        for &&taken_idx in &taken_successors {
            taken_reachable_lines.insert(ctx.cpg.node(taken_idx).line());
            let taken_reachable = ctx
                .cpg
                .reachable_forward(taken_idx, &|e| matches!(e, CpgEdge::ControlFlow));
            for &idx in &taken_reachable {
                if ctx.cpg.node(idx).file() == file_path {
                    taken_reachable_lines.insert(ctx.cpg.node(idx).line());
                }
            }
        }

        // Lines only reachable through the unreachable branch are truly
        // unreachable. Lines reachable from both paths (merge points) are kept.
        for &line in &branch_reachable_lines {
            if !taken_reachable_lines.contains(&line) {
                result.insert(line);
            }
        }
    }

    // If CFG analysis found no unreachable lines but AST did, fall back to
    // the AST result (e.g., no Statement nodes at those locations).
    if result.is_empty() && !ast_unreachable.is_empty() {
        return ast_unreachable.clone();
    }

    result
}

/// Identify branch statements where the condition resolves to a known branch.
///
/// Returns `(branch_line, unreachable_start, unreachable_end)` for each
/// resolved branch.
fn find_resolved_branches(
    parsed: &ParsedFile,
    condition: &Condition,
) -> Vec<(usize, usize, usize)> {
    let mut branches = Vec::new();
    find_resolved_branches_inner(parsed, parsed.tree.root_node(), condition, &mut branches);
    branches
}

fn find_resolved_branches_inner(
    parsed: &ParsedFile,
    node: tree_sitter::Node<'_>,
    condition: &Condition,
    branches: &mut Vec<(usize, usize, usize)>,
) {
    if parsed.language.is_control_flow_node(node.kind()) {
        if let Some(cond_node) = parsed.language.control_flow_condition(&node) {
            let cond_text = parsed.node_text(&cond_node);

            if let Some(if_body_unreachable) = condition.makes_unreachable(cond_text) {
                let branch_line = node.start_position().row + 1;
                let mut cursor = node.walk();
                let children: Vec<tree_sitter::Node<'_>> = node.children(&mut cursor).collect();

                if if_body_unreachable {
                    // The if body is unreachable
                    for child in &children {
                        if child.kind() == "block"
                            || child.kind() == "statement_block"
                            || child.kind() == "consequence"
                        {
                            let (start, end) = parsed.node_line_range(child);
                            branches.push((branch_line, start, end));
                        }
                    }
                } else {
                    // The else clause is unreachable
                    for child in &children {
                        if child.kind() == "else_clause" || child.kind() == "else" {
                            let (start, end) = parsed.node_line_range(child);
                            branches.push((branch_line, start, end));
                        }
                    }
                }
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        find_resolved_branches_inner(parsed, child, condition, branches);
    }
}

// ---------------------------------------------------------------------------
// AST-based unreachable detection (fallback)
// ---------------------------------------------------------------------------

fn find_ast_unreachable_lines(parsed: &ParsedFile, condition: &Condition) -> BTreeSet<usize> {
    let mut unreachable = BTreeSet::new();
    find_ast_unreachable_inner(parsed, parsed.tree.root_node(), condition, &mut unreachable);
    unreachable
}

fn find_ast_unreachable_inner(
    parsed: &ParsedFile,
    node: tree_sitter::Node<'_>,
    condition: &Condition,
    unreachable: &mut BTreeSet<usize>,
) {
    if parsed.language.is_control_flow_node(node.kind()) {
        if let Some(cond_node) = parsed.language.control_flow_condition(&node) {
            let cond_text = parsed.node_text(&cond_node);

            if let Some(if_body_unreachable) = condition.makes_unreachable(cond_text) {
                let mut cursor = node.walk();
                let children: Vec<tree_sitter::Node<'_>> = node.children(&mut cursor).collect();

                if if_body_unreachable {
                    // The if body is unreachable — find the body/consequence
                    for child in &children {
                        if child.kind() == "block"
                            || child.kind() == "statement_block"
                            || child.kind() == "consequence"
                        {
                            let (start, end) = parsed.node_line_range(child);
                            for l in start..=end {
                                unreachable.insert(l);
                            }
                        }
                    }
                } else {
                    // The else clause is unreachable
                    for child in &children {
                        if child.kind() == "else_clause" || child.kind() == "else" {
                            let (start, end) = parsed.node_line_range(child);
                            for l in start..=end {
                                unreachable.insert(l);
                            }
                        }
                    }
                }
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        find_ast_unreachable_inner(parsed, child, condition, unreachable);
    }
}
