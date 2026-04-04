//! Conditioned Slice — backward slice under a specific value assumption.
//!
//! Given a predicate like `var == value` or `var != null`, performs a backward
//! slice but prunes branches that are unreachable under that condition. This
//! focuses the slice on a specific execution scenario.

use crate::ast::ParsedFile;
use crate::cpg::CpgContext;
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

    // Prune branches that are unreachable under the condition
    for block in &mut base.blocks {
        let file_path = block.file.clone();
        let parsed = match ctx.files.get(&file_path) {
            Some(f) => f,
            None => continue,
        };

        let lines_to_remove = find_unreachable_lines(parsed, condition);

        if let Some(line_map) = block.file_line_map.get_mut(&file_path) {
            for line in &lines_to_remove {
                line_map.remove(line);
            }
        }
        block.diff_lines.retain(|l| !lines_to_remove.contains(l));
    }

    Ok(base)
}

fn find_unreachable_lines(parsed: &ParsedFile, condition: &Condition) -> BTreeSet<usize> {
    let mut unreachable = BTreeSet::new();
    find_unreachable_inner(parsed, parsed.tree.root_node(), condition, &mut unreachable);
    unreachable
}

fn find_unreachable_inner(
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
        find_unreachable_inner(parsed, child, condition, unreachable);
    }
}
