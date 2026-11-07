//! Returns, trailing returns, gotos, labels, enclosing branches and condition variables.

use crate::ast::{ParsedFile, ReturnInfo, ReturnValueInfo};
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

impl ParsedFile {
    /// Find condition variables in control flow statements on the given lines.
    pub fn condition_variables_on_lines(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<(String, usize)> {
        // Condition vars are a composition: find control flow nodes on diff lines,
        // extract the condition sub-node, then collect identifiers within it.
        // The control flow node matching stays manual (too many node types per language),
        // but the identifier extraction within the condition uses the Identifiers query
        // via collect_all_identifiers (which is called on the condition sub-node).
        let mut vars = Vec::new();
        self.collect_condition_vars(*func_node, lines, &mut vars);
        vars
    }

    fn collect_condition_vars(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(String, usize)>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line) && self.language.is_control_flow_node(node.kind()) {
            if let Some(condition) = self.language.control_flow_condition(&node) {
                self.collect_all_identifiers(condition, out);
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_condition_vars(child, lines, out);
        }
    }

    /// Find all return statements within a function.
    pub fn return_statements(&self, func_node: &Node<'_>) -> Vec<usize> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(query) = get_query(self.language, QueryKind::Returns) {
            let ret_idx = query
                .capture_index_for_name("ret")
                .expect("Returns query must have @ret capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut lines = Vec::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == ret_idx {
                        lines.push(capture.node.start_position().row + 1);
                    }
                }
            }
            return lines;
        }

        let mut lines = Vec::new();
        self.collect_returns_manual(*func_node, &mut lines);
        lines
    }

    /// Manual recursive return collection (pre-query fallback).
    fn collect_returns_manual(&self, node: Node<'_>, out: &mut Vec<usize>) {
        if self.language.is_return_node(node.kind()) {
            out.push(node.start_position().row + 1);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_returns_manual(child, out);
        }
    }

    /// Collect return statements with their value expressions.
    ///
    /// For each return statement in the function, extracts the return value
    /// expression text and node kind. Also detects Rust trailing expressions
    /// (last expression in a block without semicolon).
    pub fn return_value_nodes(&self, func_node: &Node<'_>) -> Vec<ReturnInfo> {
        let func_node = self.unwrap_decorated(*func_node);
        let mut returns = Vec::new();
        self.collect_return_infos(func_node, &func_node, &mut returns);

        // For Rust: check for trailing expressions (last expr in block without `;`)
        if self.language == Language::Rust {
            self.collect_trailing_returns(&func_node, &mut returns);
        }

        returns
    }

    /// Recursively collect ReturnInfo from return statement nodes.
    fn collect_return_infos(
        &self,
        node: Node<'_>,
        func_node: &Node<'_>,
        out: &mut Vec<ReturnInfo>,
    ) {
        let kind = node.kind();
        if kind == "return_statement" || kind == "return_expression" {
            let line = node.start_position().row + 1;

            // Extract the return value expression (first named child that
            // isn't the `return` keyword itself).
            let mut values = Vec::new();
            let mut value_text = None;
            let mut value_kind = None;

            if kind == "return_statement" && self.language == Language::Go {
                // Go: return may have an expression_list child
                if let Some(child) = node.named_child(0) {
                    let ck = child.kind();
                    value_text = Some(self.node_text(&child).to_string());
                    value_kind = Some(ck.to_string());
                    if ck == "expression_list" {
                        let mut cursor = child.walk();
                        for (slot, value) in child.named_children(&mut cursor).enumerate() {
                            values.push(ReturnValueInfo {
                                slot,
                                line: value.start_position().row + 1,
                                text: self.node_text(&value).to_string(),
                                kind: value.kind().to_string(),
                                start_byte: value.start_byte(),
                                end_byte: value.end_byte(),
                            });
                        }
                    } else {
                        // Single expression (not expression_list)
                        values.push(ReturnValueInfo {
                            slot: 0,
                            line: child.start_position().row + 1,
                            text: self.node_text(&child).to_string(),
                            kind: ck.to_string(),
                            start_byte: child.start_byte(),
                            end_byte: child.end_byte(),
                        });
                    }
                }
            } else {
                // All other languages: first named child is the expression
                if let Some(child) = node.named_child(0) {
                    value_text = Some(self.node_text(&child).to_string());
                    value_kind = Some(child.kind().to_string());
                    values.push(ReturnValueInfo {
                        slot: 0,
                        line: child.start_position().row + 1,
                        text: self.node_text(&child).to_string(),
                        kind: child.kind().to_string(),
                        start_byte: child.start_byte(),
                        end_byte: child.end_byte(),
                    });
                }
            }

            let is_conditional = self.is_inside_conditional(&node, func_node);

            out.push(ReturnInfo {
                line,
                value_text,
                value_kind,
                is_conditional,
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
                values,
            });
            return; // Don't recurse into return children
        }

        // Don't recurse into nested callable scopes. Fence on node IDENTITY,
        // not kind: a nested definition may have the SAME kind as the enclosing
        // function, while anonymous closures may not be included in
        // `function_node_types()` at all.
        if self.language.callable_boundary_node_types().contains(&kind)
            && node.id() != func_node.id()
        {
            return;
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_return_infos(child, func_node, out);
        }
    }

    /// Check if a node is inside a conditional branch (if/else/match) within the function.
    fn is_inside_conditional(&self, node: &Node<'_>, func_node: &Node<'_>) -> bool {
        let mut current = node.parent();
        while let Some(parent) = current {
            if parent.id() == func_node.id() {
                return false;
            }
            let pk = parent.kind();
            if matches!(
                pk,
                "if_statement"
                    | "if_expression"
                    | "if_let_expression"
                    | "else_clause"
                    | "elif_clause"
                    | "match_expression"
                    | "switch_statement"
                    | "conditional_expression"
                    | "ternary_expression"
            ) {
                return true;
            }
            current = parent.parent();
        }
        false
    }

    /// Collect Rust trailing expressions that act as implicit returns.
    ///
    /// In Rust, the last expression in a block (without semicolon) is the
    /// return value. This detects such expressions in the function body
    /// and in if/else/match branches.
    fn collect_trailing_returns(&self, func_node: &Node<'_>, out: &mut Vec<ReturnInfo>) {
        let body = func_node.child_by_field_name("body");
        if let Some(body_node) = body {
            self.collect_block_trailing_returns(&body_node, func_node, out);
        }
    }

    fn collect_block_trailing_returns(
        &self,
        block: &Node<'_>,
        func_node: &Node<'_>,
        out: &mut Vec<ReturnInfo>,
    ) {
        let child_count = block.named_child_count();
        if child_count == 0 {
            return;
        }
        let last_child = block.named_child(child_count - 1).unwrap();
        let kind = last_child.kind();

        // If it's an expression_statement, check if it wraps an if/match expression
        // (Rust trailing if/match are often wrapped in expression_statement).
        if kind == "expression_statement" {
            // Check if it ends with a semicolon — if so, not a trailing return
            let text = self.node_text(&last_child);
            if text.ends_with(';') {
                return;
            }
            // Check children for if/match expressions
            if let Some(child) = last_child.named_child(0) {
                let ck = child.kind();
                if ck == "if_expression" || ck == "if_let_expression" {
                    self.collect_if_trailing_returns(&child, func_node, out);
                    return;
                }
                if ck == "match_expression" {
                    self.collect_match_trailing_returns(&child, func_node, out);
                    return;
                }
                // Other expression — treat as trailing return
                let line = child.start_position().row + 1;
                if !out.iter().any(|r| r.line == line) {
                    out.push(ReturnInfo {
                        line,
                        value_text: Some(self.node_text(&child).to_string()),
                        value_kind: Some(ck.to_string()),
                        is_conditional: self.is_inside_conditional(&child, func_node),
                        start_byte: child.start_byte(),
                        end_byte: child.end_byte(),
                        values: vec![ReturnValueInfo {
                            slot: 0,
                            line,
                            text: self.node_text(&child).to_string(),
                            kind: ck.to_string(),
                            start_byte: child.start_byte(),
                            end_byte: child.end_byte(),
                        }],
                    });
                }
                return;
            }
            return;
        }

        // If it's an if_expression or match_expression, recurse into branches
        if kind == "if_expression" || kind == "if_let_expression" {
            self.collect_if_trailing_returns(&last_child, func_node, out);
            return;
        }
        if kind == "match_expression" {
            self.collect_match_trailing_returns(&last_child, func_node, out);
            return;
        }

        // Skip return_expression — already handled by collect_return_infos
        if kind == "return_expression" {
            return;
        }

        // Skip statements that aren't expressions
        if kind.ends_with("_statement")
            || kind == "let_declaration"
            || kind == "macro_invocation"
            || kind == "empty_statement"
        {
            return;
        }

        // This is a trailing expression — implicit return
        let line = last_child.start_position().row + 1;
        // Check it's not already captured as a return
        if out.iter().any(|r| r.line == line) {
            return;
        }

        let text = self.node_text(&last_child).to_string();
        let is_conditional = self.is_inside_conditional(&last_child, func_node);
        out.push(ReturnInfo {
            line,
            value_text: Some(text.clone()),
            value_kind: Some(kind.to_string()),
            is_conditional,
            start_byte: last_child.start_byte(),
            end_byte: last_child.end_byte(),
            values: vec![ReturnValueInfo {
                slot: 0,
                line,
                text,
                kind: kind.to_string(),
                start_byte: last_child.start_byte(),
                end_byte: last_child.end_byte(),
            }],
        });
    }

    fn collect_if_trailing_returns(
        &self,
        if_node: &Node<'_>,
        func_node: &Node<'_>,
        out: &mut Vec<ReturnInfo>,
    ) {
        // Recurse into consequence (then block) and alternative (else block)
        if let Some(consequence) = if_node.child_by_field_name("consequence") {
            self.collect_block_trailing_returns(&consequence, func_node, out);
        }
        if let Some(alternative) = if_node.child_by_field_name("alternative") {
            let ak = alternative.kind();
            if ak == "else_clause" {
                // The else clause's child is either a block or another if_expression
                let mut cursor = alternative.walk();
                for child in alternative.named_children(&mut cursor) {
                    if child.kind() == "block" {
                        self.collect_block_trailing_returns(&child, func_node, out);
                    } else if child.kind() == "if_expression" || child.kind() == "if_let_expression"
                    {
                        self.collect_if_trailing_returns(&child, func_node, out);
                    }
                }
            } else if ak == "block" {
                self.collect_block_trailing_returns(&alternative, func_node, out);
            } else if ak == "if_expression" || ak == "if_let_expression" {
                self.collect_if_trailing_returns(&alternative, func_node, out);
            }
        }
    }

    fn collect_match_trailing_returns(
        &self,
        match_node: &Node<'_>,
        func_node: &Node<'_>,
        out: &mut Vec<ReturnInfo>,
    ) {
        let mut cursor = match_node.walk();
        for child in match_node.named_children(&mut cursor) {
            if child.kind() == "match_arm" {
                // The value of a match arm is its last named child (after the `=>`)
                let arm_count = child.named_child_count();
                if arm_count > 0 {
                    let arm_value = child.named_child(arm_count - 1).unwrap();
                    let vk = arm_value.kind();
                    if vk == "block" {
                        self.collect_block_trailing_returns(&arm_value, func_node, out);
                    } else if vk != "match_pattern" {
                        let line = arm_value.start_position().row + 1;
                        if !out.iter().any(|r| r.line == line) {
                            out.push(ReturnInfo {
                                line,
                                value_text: Some(self.node_text(&arm_value).to_string()),
                                value_kind: Some(vk.to_string()),
                                is_conditional: true,
                                start_byte: arm_value.start_byte(),
                                end_byte: arm_value.end_byte(),
                                values: vec![ReturnValueInfo {
                                    slot: 0,
                                    line,
                                    text: self.node_text(&arm_value).to_string(),
                                    kind: vk.to_string(),
                                    start_byte: arm_value.start_byte(),
                                    end_byte: arm_value.end_byte(),
                                }],
                            });
                        }
                    }
                }
            }
        }
    }

    /// Find all goto statements in a function node.
    /// Returns `(target_label, goto_line)` pairs.
    pub fn goto_statements(&self, func_node: &Node<'_>) -> Vec<(String, usize)> {
        let mut gotos = Vec::new();
        self.collect_gotos(*func_node, &mut gotos);
        gotos
    }

    fn collect_gotos(&self, node: Node<'_>, out: &mut Vec<(String, usize)>) {
        if node.kind() == "goto_statement" {
            // tree-sitter-c: goto_statement has a "label" field with the target name
            if let Some(label_node) = node.child_by_field_name("label") {
                let label = self.node_text(&label_node).to_string();
                let line = node.start_position().row + 1;
                out.push((label, line));
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_gotos(child, out);
        }
    }

    /// Find all label definitions in a function node.
    /// Returns `(label_name, label_line, section_end_line)` triples.
    /// `section_end_line` is the line of the next label or the function end,
    /// representing the code section "owned" by this label.
    pub fn label_sections(&self, func_node: &Node<'_>) -> Vec<(String, usize, usize)> {
        let mut labels: Vec<(String, usize)> = Vec::new();
        self.collect_labels(*func_node, &mut labels);

        let func_end = func_node.end_position().row + 1;

        // Sort labels by line number to determine sections
        labels.sort_by_key(|(_, line)| *line);

        let mut sections = Vec::new();
        for i in 0..labels.len() {
            let (ref name, start) = labels[i];
            let end = if i + 1 < labels.len() {
                labels[i + 1].1.saturating_sub(1)
            } else {
                func_end
            };
            sections.push((name.clone(), start, end));
        }
        sections
    }

    fn collect_labels(&self, node: Node<'_>, out: &mut Vec<(String, usize)>) {
        if node.kind() == "labeled_statement" {
            // tree-sitter-c: labeled_statement has a "label" field
            if let Some(label_node) = node.child_by_field_name("label") {
                let label = self.node_text(&label_node).to_string();
                let line = node.start_position().row + 1;
                out.push((label, line));
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_labels(child, out);
        }
    }

    /// Classify labels as cleanup-only (reachable only via goto) or
    /// flow-through (also reachable from sequential execution).
    ///
    /// A label is cleanup-only if the statement immediately preceding it
    /// is a flow-terminating statement (return, goto, break, continue).
    /// If the preceding statement is normal code, sequential execution
    /// falls through into the label — it's part of the normal path.
    ///
    /// Returns `(name, start, end, is_cleanup_only)` tuples.
    pub fn classify_labels(&self, func_node: &Node<'_>) -> Vec<(String, usize, usize, bool)> {
        let sections = self.label_sections(func_node);
        let (func_start, _func_end) = self.node_line_range(func_node);

        sections
            .into_iter()
            .map(|(name, start, end)| {
                let is_cleanup = if start <= func_start + 1 {
                    // Label at the very beginning of the function — flow-through
                    false
                } else {
                    // Check lines immediately before the label for a flow terminator.
                    // We scan backwards from label_start-1 to skip blank/brace lines.
                    let mut found_terminator = false;
                    for check_line in (func_start..start).rev() {
                        let row = check_line.saturating_sub(1);
                        if self.find_flow_terminator(self.tree.root_node(), row) {
                            found_terminator = true;
                            break;
                        }
                        // Check if this line has any real code (not just whitespace/braces)
                        if let Some(line_str) = self.source.lines().nth(row) {
                            let trimmed = line_str.trim();
                            if !trimmed.is_empty() && trimmed != "}" && trimmed != "{" {
                                // Found non-empty, non-brace code that isn't a terminator
                                break;
                            }
                        }
                    }
                    found_terminator
                };
                (name, start, end, is_cleanup)
            })
            .collect()
    }

    /// Check if a given row (0-indexed) contains a flow-terminating statement.
    fn find_flow_terminator(&self, node: Node<'_>, row: usize) -> bool {
        if node.start_position().row == row
            && matches!(
                node.kind(),
                "return_statement" | "goto_statement" | "break_statement" | "continue_statement"
            )
        {
            return true;
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.start_position().row <= row && child.end_position().row >= row {
                if self.find_flow_terminator(child, row) {
                    return true;
                }
            }
        }
        false
    }

    /// Partition a function's lines into normal-path lines and cleanup label sections.
    ///
    /// Returns `(normal_lines, label_lines)` where:
    /// - `normal_lines` are lines NOT in any cleanup-only label section
    /// - `label_lines` maps each cleanup-only label name to its line range
    ///
    /// Labels reachable via fall-through (no preceding flow terminator) are
    /// considered part of the normal path. Only cleanup-only labels (preceded
    /// by return/goto/break/continue) are separated out.
    pub fn partition_by_labels(
        &self,
        func_node: &Node<'_>,
    ) -> (Vec<usize>, BTreeMap<String, Vec<usize>>) {
        let (func_start, func_end) = self.node_line_range(func_node);
        let classified = self.classify_labels(func_node);
        let cleanup_labels: Vec<_> = classified
            .iter()
            .filter(|(_, _, _, is_cleanup)| *is_cleanup)
            .collect();

        if cleanup_labels.is_empty() {
            let normal: Vec<usize> = (func_start..=func_end).collect();
            return (normal, BTreeMap::new());
        }

        // Build a set of all lines in cleanup-only label sections
        let mut cleanup_line_set = std::collections::BTreeSet::new();
        let mut label_map: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (name, start, end, _) in &cleanup_labels {
            let lines: Vec<usize> = (*start..=*end).collect();
            cleanup_line_set.extend(lines.iter().copied());
            label_map.insert(name.clone(), lines);
        }

        let normal: Vec<usize> = (func_start..=func_end)
            .filter(|l| !cleanup_line_set.contains(l))
            .collect();

        (normal, label_map)
    }

    /// Get all lines reachable from a `goto label`, including fall-through
    /// to subsequent labels (unless a `return` breaks the fall-through).
    ///
    /// In the cascading kernel cleanup pattern:
    /// ```c
    ///   err_dev:
    ///       kfree(dev);          // reachable from goto err_dev
    ///   err_buf:                 // falls through from err_dev (no return above)
    ///       kfree(buf);          // also reachable from goto err_dev
    ///       return -1;
    /// ```
    ///
    /// `goto err_dev` reaches both sections. `goto err_buf` reaches only err_buf.
    pub fn lines_reachable_from_goto(
        &self,
        func_node: &Node<'_>,
        target_label: &str,
    ) -> Vec<usize> {
        let label_secs = self.label_sections(func_node);
        let returns = self.return_statements(func_node);
        let return_set: std::collections::BTreeSet<usize> = returns.into_iter().collect();

        let mut reachable = Vec::new();
        let mut found_target = false;

        for (name, start, end) in &label_secs {
            if name == target_label {
                found_target = true;
            }
            if !found_target {
                continue;
            }

            reachable.extend(*start..=*end);

            // Check if this section contains a return — if so, fall-through stops
            if (*start..=*end).any(|l| return_set.contains(&l)) {
                break;
            }
        }

        reachable
    }

    /// Find the enclosing control flow block (if/for/while) for a given line,
    /// and return its start and end lines.
    pub fn enclosing_branch(&self, line: usize) -> Option<(usize, usize)> {
        let row = line.saturating_sub(1);
        self.find_enclosing_branch(self.tree.root_node(), row)
    }

    fn find_enclosing_branch(&self, node: Node<'_>, row: usize) -> Option<(usize, usize)> {
        let start = node.start_position().row;
        let end = node.end_position().row;

        if row < start || row > end {
            return None;
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = self.find_enclosing_branch(child, row) {
                return Some(found);
            }
        }

        if self.language.is_control_flow_node(node.kind()) {
            Some((start + 1, end + 1))
        } else {
            None
        }
    }
}
