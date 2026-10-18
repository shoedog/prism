//! Visit each (syntax node, collector mode) once, retaining first-visit order.
//!
//! A repeated statement/span state emits the same sequence. Stable line sorting
//! and first-per-line deduplication therefore make its later emissions redundant.
//! Ordinary and nested modes differ and MUST have separate visit sets.
//!
//! Arm context can differ on a repeat (e.g. a bare block reached through both
//! modes). But a completed arm state has already inserted every line it emits
//! and opened every arm node it reaches. Replaying it in any parent arm only
//! hits occupied `of_line` entries and reused `seen` IDs; no `newly_opened`
//! subtree bound is written. It is a no-op even with a different parent arm.
//! Descent is strictly to children, so a repeat cannot precede completion of
//! its first visit. Removing these no-ops preserves the allocation preorder,
//! first line winner and every subtree bound, including empty arms.
//!
//! Each node has at most two admitted states, each scanning its own children:
//! at most 2N state visits and 2(N-1) child edges, plus final output ordering.

use super::{LexicalArms, ParsedFile, StatementSpan};
use std::collections::{BTreeMap, HashSet};
use tree_sitter::Node;

#[derive(Default)]
struct Visits {
    ordinary: HashSet<usize>,
    nested: HashSet<usize>,
}

/// Preorder allocator, retaining the historical node-to-arm identity. The old
/// walker revisited blocks exponentially: without arm reuse a 154-line C
/// fixture allocated 55,356 arms for the 70 holding statement lines. Traversal
/// visitation and arm allocation remain distinct: the two modes share arm IDs.
struct ArmAllocator {
    next: u32,
    seen: BTreeMap<usize, u32>,
}

impl ParsedFile {
    /// Collect all statement-level nodes within a function for CFG construction.
    ///
    /// Returns `(line, node_kind)` pairs in source order. Only direct children of
    /// the function body and top-level children of compound statements are included
    /// — nested expressions within a statement are not separate CFG nodes.
    pub fn statements_in_function(&self, func_node: &Node<'_>) -> Vec<(usize, String)> {
        let func_node = self.unwrap_decorated(*func_node);
        let mut stmts = Vec::new();
        // Find the function body (compound_statement, block, etc.)
        let body = func_node
            .child_by_field_name("body")
            .or_else(|| func_node.child_by_field_name("consequence"));
        if let Some(body_node) = body {
            self.collect_statements(body_node, &mut Visits::default(), &mut stmts);
        }
        stmts.sort_by_key(|(line, _)| *line);
        stmts.dedup_by_key(|(line, _)| *line);
        stmts
    }

    /// Build the lexical branch-arm tree for `func_node`.
    ///
    /// **Why this exists.** `statements_in_function` sorts and
    /// line-deduplicates its result, so `cfg::build_function_cfg`'s sequential
    /// fall-through loop walks one globally ordered line list and can join two
    /// statements that live in *different, mutually exclusive* lexical arms —
    /// e.g. the last statement of a `then` block to the first statement of the
    /// `else` block. Such an edge is not a control-flow fact. This tree is the
    /// evidence `cfg::build_cfg_edges_with_arms` pairs with each edge so the
    /// reaching-definitions pass (item 2 Task 3, `src/cpg/reaching.rs`) can
    /// refuse to call such an edge a proof.
    ///
    /// Entering or leaving a *nested* arm is genuine control flow, so the
    /// consumer asks whether the two arms are **incomparable** (neither contains
    /// the other), not merely whether they differ — hence the preorder interval.
    ///
    /// **This is metadata only.** It does not change which lines
    /// `statements_in_function` returns; it walks the same tree with the same
    /// recursion and the same node-kind predicates, and records the first
    /// traversal occurrence of each line — matching the stable
    /// `sort_by_key` + `dedup_by_key` pair that decides which duplicate line
    /// `statements_in_function` keeps.
    pub(crate) fn statement_arms_in_function(&self, func_node: &Node<'_>) -> LexicalArms {
        let func_node = self.unwrap_decorated(*func_node);
        let mut arms = LexicalArms {
            of_line: BTreeMap::new(),
            // Arm 0 (the body) is allocated up front; its subtree bound is
            // patched below once every nested arm has been allocated.
            subtree_last: vec![0],
        };
        let mut alloc = ArmAllocator {
            next: 1,
            seen: BTreeMap::new(),
        };
        let body = func_node
            .child_by_field_name("body")
            .or_else(|| func_node.child_by_field_name("consequence"));
        if let Some(body_node) = body {
            self.collect_statement_arms(
                body_node,
                &mut Visits::default(),
                0,
                &mut alloc,
                &mut arms,
            );
        }
        arms.subtree_last[0] = alloc.next - 1;
        arms
    }

    /// The arm for `node`, allocating one in preorder on first sight. Returns
    /// `(arm, newly_opened)`; only a newly opened arm may have its
    /// `subtree_last` written, because on a repeat visit the ids allocated since
    /// belong to siblings, not to this arm's subtree.
    fn open_arm(node: &Node<'_>, alloc: &mut ArmAllocator, arms: &mut LexicalArms) -> (u32, bool) {
        if let Some(&existing) = alloc.seen.get(&node.id()) {
            return (existing, false);
        }
        let id = alloc.next;
        alloc.next += 1;
        alloc.seen.insert(node.id(), id);
        arms.subtree_last.push(id);
        (id, true)
    }

    /// Arm-tagging twin of `collect_statements`. Same children, same
    /// predicates, same recursion — only the payload differs.
    fn collect_statement_arms(
        &self,
        node: Node<'_>,
        visits: &mut Visits,
        arm: u32,
        alloc: &mut ArmAllocator,
        arms: &mut LexicalArms,
    ) {
        if !visits.ordinary.insert(node.id()) {
            return;
        }
        #[cfg(test)]
        tests::record(4, node.id(), node.child_count());
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind();
            let line = child.start_position().row + 1;

            if self.language.is_statement_node(kind) {
                arms.of_line.entry(line).or_insert(arm);

                if self.language.is_control_flow_node(kind)
                    || self.language.statement_wrapper_kinds().contains(&kind)
                {
                    self.collect_nested_statement_arms(child, visits, arm, alloc, arms);
                }
            } else if self.language.statement_wrapper_kinds().contains(&kind) {
                self.collect_nested_statement_arms(child, visits, arm, alloc, arms);
            } else if kind == "compound_statement" || kind == "block" || kind == "statement_block" {
                // A bare nested block is not a branch arm — it inherits the
                // current arm. Arms are allocated in
                // `collect_nested_statement_arms`, which is the only place a
                // block is reached *through* a control-flow node.
                self.collect_statement_arms(child, visits, arm, alloc, arms);
            }
        }
    }

    /// Arm-tagging twin of `collect_nested_statements`. A block-ish
    /// child of a control-flow node is a new arm only when it is a *mutually
    /// exclusive* alternative:
    ///
    /// - `switch_body` / `match_block` are containers of arms, not arms.
    /// - `case_statement` / `default_statement` in a language whose `switch`
    ///   falls through (`Language::switch_has_fallthrough`: C, C++, JS, TS, TSX,
    ///   Java) are **not** mutually exclusive — control runs from one case into
    ///   the next, which `cfg::tests::test_c_switch_fallthrough` pins as genuine
    ///   — so they share the enclosing arm. In a language without fall-through
    ///   each case is a real alternative and gets its own arm.
    fn collect_nested_statement_arms(
        &self,
        node: Node<'_>,
        visits: &mut Visits,
        arm: u32,
        alloc: &mut ArmAllocator,
        arms: &mut LexicalArms,
    ) {
        if !visits.nested.insert(node.id()) {
            return;
        }
        #[cfg(test)]
        tests::record(5, node.id(), node.child_count());
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind();
            if kind == "compound_statement"
                || kind == "block"
                || kind == "statement_block"
                || kind == "else_clause"
                || kind == "elif_clause"
                || kind == "else_if_clause"
                || kind == "switch_body"
                || kind == "case_statement"
                || kind == "default_statement"
                || kind == "match_block"
                || kind == "match_arm"
                || self.language.statement_wrapper_kinds().contains(&kind)
            {
                let container = matches!(
                    kind,
                    "switch_body"
                        | "match_block"
                        | "expression_statement"
                        | "labeled_statement"
                        | "attributed_statement"
                        | "with_statement"
                        | "finally_clause"
                        | "select_statement"
                        | "expression_switch_statement"
                        | "type_switch_statement"
                        | "switch_expression"
                        | "switch_block"
                        | "synchronized_statement"
                        | "try_with_resources_statement"
                        | "async_block"
                        | "const_block"
                        | "gen_block"
                        | "try_block"
                        | "unsafe_block"
                        | "subshell"
                        | "redirected_statement"
                );
                let fallthrough_case = matches!(
                    kind,
                    "case_statement"
                        | "default_statement"
                        | "switch_case"
                        | "switch_default"
                        | "switch_block_statement_group"
                ) && self.language.switch_has_fallthrough();
                let (child_arm, newly_opened) = if container || fallthrough_case {
                    (arm, false)
                } else {
                    Self::open_arm(&child, alloc, arms)
                };
                self.collect_statement_arms(child, visits, child_arm, alloc, arms);
                self.collect_nested_statement_arms(child, visits, child_arm, alloc, arms);
                if newly_opened {
                    // Preorder: everything allocated since `child_arm` opened is
                    // lexically inside `child`, so the ids are contiguous.
                    arms.subtree_last[child_arm as usize] = alloc.next - 1;
                }
            } else if self.language.is_control_flow_node(kind) {
                // Nested control flow (if inside if, etc.) — the header itself
                // belongs to the enclosing arm.
                let line = child.start_position().row + 1;
                arms.of_line.entry(line).or_insert(arm);
                self.collect_nested_statement_arms(child, visits, arm, alloc, arms);
            } else if self.language.is_statement_node(kind) {
                // Some wrappers (notably Bash subshells) contain statements
                // directly rather than through a block node.
                let line = child.start_position().row + 1;
                arms.of_line.entry(line).or_insert(arm);
            }
        }
    }

    /// Byte-bearing sibling of `statements_in_function`.
    pub fn statement_spans_in_function(&self, func_node: &Node<'_>) -> Vec<StatementSpan> {
        let func_node = self.unwrap_decorated(*func_node);
        let mut stmts = Vec::new();
        if let Some(body_node) = self.function_body_node(&func_node) {
            self.collect_statement_spans(body_node, &mut Visits::default(), &mut stmts);
        }
        stmts.sort_by_key(|stmt| stmt.line);
        stmts.dedup_by_key(|stmt| stmt.line);
        stmts
    }

    fn collect_statements(
        &self,
        node: Node<'_>,
        visits: &mut Visits,
        out: &mut Vec<(usize, String)>,
    ) {
        if !visits.ordinary.insert(node.id()) {
            return;
        }
        #[cfg(test)]
        tests::record(0, node.id(), node.child_count());
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind();
            let line = child.start_position().row + 1;

            if self.language.is_statement_node(kind) {
                out.push((line, kind.to_string()));

                // For control flow nodes, also recurse into their bodies
                // to find nested statements (then-branch, else-branch, loop body)
                if self.language.is_control_flow_node(kind)
                    || self.language.statement_wrapper_kinds().contains(&kind)
                {
                    self.collect_nested_statements(child, visits, out);
                }
            } else if self.language.statement_wrapper_kinds().contains(&kind) {
                self.collect_nested_statements(child, visits, out);
            } else if kind == "compound_statement" || kind == "block" || kind == "statement_block" {
                // Recurse into blocks
                self.collect_statements(child, visits, out);
            }
        }
    }

    fn collect_statement_spans(
        &self,
        node: Node<'_>,
        visits: &mut Visits,
        out: &mut Vec<StatementSpan>,
    ) {
        if !visits.ordinary.insert(node.id()) {
            return;
        }
        #[cfg(test)]
        tests::record(2, node.id(), node.child_count());
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind();
            let line = child.start_position().row + 1;

            if self.language.is_statement_node(kind) {
                out.push(StatementSpan {
                    line,
                    kind: kind.to_string(),
                    start_byte: child.start_byte(),
                    end_byte: child.end_byte(),
                });

                if self.language.is_control_flow_node(kind)
                    || self.language.statement_wrapper_kinds().contains(&kind)
                {
                    self.collect_nested_statement_spans(child, visits, out);
                }
            } else if self.language.statement_wrapper_kinds().contains(&kind) {
                self.collect_nested_statement_spans(child, visits, out);
            } else if kind == "compound_statement" || kind == "block" || kind == "statement_block" {
                self.collect_statement_spans(child, visits, out);
            }
        }
    }

    fn collect_nested_statements(
        &self,
        node: Node<'_>,
        visits: &mut Visits,
        out: &mut Vec<(usize, String)>,
    ) {
        if !visits.nested.insert(node.id()) {
            return;
        }
        #[cfg(test)]
        tests::record(1, node.id(), node.child_count());
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind();
            if kind == "compound_statement"
                || kind == "block"
                || kind == "statement_block"
                || kind == "else_clause"
                || kind == "elif_clause"
                || kind == "else_if_clause"
                || kind == "switch_body"
                || kind == "case_statement"
                || kind == "default_statement"
                || kind == "match_block"
                || kind == "match_arm"
                || self.language.statement_wrapper_kinds().contains(&kind)
            {
                self.collect_statements(child, visits, out);
                self.collect_nested_statements(child, visits, out);
            } else if self.language.is_control_flow_node(kind) {
                // Nested control flow (if inside if, etc.)
                let line = child.start_position().row + 1;
                out.push((line, kind.to_string()));
                self.collect_nested_statements(child, visits, out);
            } else if self.language.is_statement_node(kind) {
                // Some wrappers (notably Bash subshells) contain statements
                // directly rather than through a block node.
                let line = child.start_position().row + 1;
                out.push((line, kind.to_string()));
            }
        }
    }

    fn collect_nested_statement_spans(
        &self,
        node: Node<'_>,
        visits: &mut Visits,
        out: &mut Vec<StatementSpan>,
    ) {
        if !visits.nested.insert(node.id()) {
            return;
        }
        #[cfg(test)]
        tests::record(3, node.id(), node.child_count());
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind();
            if kind == "compound_statement"
                || kind == "block"
                || kind == "statement_block"
                || kind == "else_clause"
                || kind == "elif_clause"
                || kind == "else_if_clause"
                || kind == "switch_body"
                || kind == "case_statement"
                || kind == "default_statement"
                || kind == "match_block"
                || kind == "match_arm"
                || self.language.statement_wrapper_kinds().contains(&kind)
            {
                self.collect_statement_spans(child, visits, out);
                self.collect_nested_statement_spans(child, visits, out);
            } else if self.language.is_control_flow_node(kind) {
                out.push(StatementSpan {
                    line: child.start_position().row + 1,
                    kind: kind.to_string(),
                    start_byte: child.start_byte(),
                    end_byte: child.end_byte(),
                });
                self.collect_nested_statement_spans(child, visits, out);
            } else if self.language.is_statement_node(kind) {
                out.push(StatementSpan {
                    line: child.start_position().row + 1,
                    kind: kind.to_string(),
                    start_byte: child.start_byte(),
                    end_byte: child.end_byte(),
                });
            }
        }
    }
}

#[cfg(test)]
mod oracle;
#[cfg(test)]
mod tests;
