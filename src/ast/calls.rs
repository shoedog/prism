//! Function calls on lines (with qualifiers and spans), callees and function lookup by name.

use crate::ast::{call_membership, CallSiteMeta, ParsedFile};
use std::collections::BTreeSet;
use tree_sitter::Node;

impl ParsedFile {
    /// Find function calls on the given lines and return the called function names.
    pub fn function_calls_on_lines(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<(String, usize)> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(query) = get_query(self.language, QueryKind::Calls) {
            let membership = call_membership::CallMembership::new(self, *func_node);
            let call_idx = query
                .capture_index_for_name("call")
                .expect("Calls query must have @call capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut calls = Vec::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == call_idx && membership.is_owned(capture.node) {
                        let line = capture.node.start_position().row + 1;
                        if lines.contains(&line) {
                            if let Some(name_node) = self.call_function_name(&capture.node) {
                                let name = self.node_text(&name_node).to_string();
                                calls.push((name, line));
                            }
                        }
                    }
                }
            }
            return calls;
        }

        let mut calls = Vec::new();
        self.collect_calls_manual(*func_node, lines, &mut calls);
        calls
    }

    /// Find function calls on the given lines and return the called function
    /// names plus the call node's byte range.
    ///
    /// Rust also mints value calls found INSIDE a transparent macro's
    /// arguments (`assert!(check(x))`) — see `crate::rust_macro_args`. Those
    /// entries carry `kind_override`/`origin_override` (`Call`/`MacroArg`);
    /// every grammar-parsed entry leaves both `None`, unchanged from before
    /// this struct existed. `macro_shadow` is the repo-wide macro-name shadow
    /// set (`crate::rust_macro_args::collect_macro_shadow_set`) — pass
    /// `&BTreeSet::new()` for non-Rust callers or callers with no shadow facts.
    pub fn function_calls_with_spans_on_lines<'a>(
        &'a self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
        macro_shadow: &BTreeSet<String>,
    ) -> Vec<CallSiteMeta<'a>> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(query) = get_query(self.language, QueryKind::Calls) {
            let membership = call_membership::CallMembership::new(self, *func_node);
            let call_idx = query
                .capture_index_for_name("call")
                .expect("Calls query must have @call capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut calls = Vec::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index != call_idx {
                        continue;
                    }
                    if !membership.is_owned(capture.node) {
                        continue;
                    }
                    let line = capture.node.start_position().row + 1;
                    if !lines.contains(&line) {
                        continue;
                    }
                    if self.language == crate::languages::Language::Rust
                        && capture.node.kind() == "macro_invocation"
                    {
                        let (sites, _facts) =
                            crate::rust_macro_args::extract_calls(self, capture.node, macro_shadow);
                        calls.extend(sites);
                        continue;
                    }
                    if let Some(name_node) = self.call_function_name(&capture.node) {
                        let name = self.node_text(&name_node).to_string();
                        calls.push(CallSiteMeta {
                            callee_name: name,
                            line,
                            qualifier: None,
                            start_byte: capture.node.start_byte(),
                            end_byte: capture.node.end_byte(),
                            receiver_node: None,
                            arg_count: None,
                            arg_spread: false,
                            kind_override: None,
                            origin_override: None,
                        });
                    }
                }
            }
            return calls;
        }

        let mut calls = Vec::new();
        self.collect_calls_manual_with_spans(*func_node, lines, &mut calls);
        calls
    }

    /// Like `function_calls_on_lines`, but also extracts the module/object qualifier.
    /// Returns `(callee_name, line, qualifier)` tuples.
    pub fn function_calls_on_lines_with_qualifier(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<(String, usize, Option<String>)> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(query) = get_query(self.language, QueryKind::Calls) {
            let membership = call_membership::CallMembership::new(self, *func_node);
            let call_idx = query
                .capture_index_for_name("call")
                .expect("Calls query must have @call capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut calls = Vec::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == call_idx && membership.is_owned(capture.node) {
                        let line = capture.node.start_position().row + 1;
                        if lines.contains(&line) {
                            if let Some(name_node) = self.call_function_name(&capture.node) {
                                let name = self.node_text(&name_node).to_string();
                                let qualifier = self
                                    .call_function_qualifier(&capture.node)
                                    .map(|q| self.node_text(&q).to_string());
                                calls.push((name, line, qualifier));
                            }
                        }
                    }
                }
            }
            return calls;
        }

        let mut calls = Vec::new();
        self.collect_calls_manual_with_qualifier(*func_node, lines, &mut calls);
        calls
    }

    /// Like `function_calls_on_lines_with_qualifier`, but also returns the call
    /// node's byte range plus the qualifier/receiver node, wrapped in
    /// `CallSiteMeta`. `receiver_node` (the selector operand — e.g. the
    /// `type_assertion_expression` in `x.(Module).M()`) feeds the S3
    /// `ReceiverClassifier`. It is surfaced on the query path only; the
    /// manual fallback yields `None` (type-assertion recovery is Go-only, and
    /// Go uses the query path).
    ///
    /// Rust also mints value calls found INSIDE a transparent macro's
    /// arguments (`assert!(check(x))` / `assert!(v.contains(x))`) — see
    /// `crate::rust_macro_args`. The returned `MacroArgFacts` aggregates
    /// telemetry for every macro invocation encountered on `lines` in this
    /// one call (the caller sums these per file). `macro_shadow` is the
    /// repo-wide macro-name shadow set
    /// (`crate::rust_macro_args::collect_macro_shadow_set`) — pass
    /// `&BTreeSet::new()` for non-Rust callers or callers with no shadow facts.
    pub fn function_calls_with_qualifier_and_spans_on_lines<'a>(
        &'a self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
        macro_shadow: &BTreeSet<String>,
    ) -> (Vec<CallSiteMeta<'a>>, crate::rust_macro_args::MacroArgFacts) {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        let mut facts = crate::rust_macro_args::MacroArgFacts::default();

        if let Some(query) = get_query(self.language, QueryKind::Calls) {
            let membership = call_membership::CallMembership::new(self, *func_node);
            let call_idx = query
                .capture_index_for_name("call")
                .expect("Calls query must have @call capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut calls = Vec::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index != call_idx {
                        continue;
                    }
                    if !membership.is_owned(capture.node) {
                        continue;
                    }
                    let line = capture.node.start_position().row + 1;
                    if !lines.contains(&line) {
                        continue;
                    }
                    if self.language == crate::languages::Language::Rust
                        && capture.node.kind() == "macro_invocation"
                    {
                        let (sites, site_facts) =
                            crate::rust_macro_args::extract_calls(self, capture.node, macro_shadow);
                        calls.extend(sites);
                        facts.calls_recorded += site_facts.calls_recorded;
                        facts.skipped_macros += site_facts.skipped_macros;
                        facts.ctor_skips += site_facts.ctor_skips;
                        continue;
                    }
                    if let Some(name_node) = self.call_function_name(&capture.node) {
                        let name = self.node_text(&name_node).to_string();
                        let qualifier_node = self.call_function_qualifier(&capture.node);
                        let qualifier = qualifier_node.map(|q| self.node_text(&q).to_string());
                        let (arg_count, arg_spread) = self
                            .language
                            .call_arguments(&capture.node)
                            .map(|args| {
                                let mut count = 0usize;
                                let mut spread = false;
                                let mut cursor2 = args.walk();
                                for child in args.children(&mut cursor2) {
                                    if !self.is_positional_argument_node(child) {
                                        continue; // punctuation and JS/TS comment trivia
                                    }
                                    if child.kind() == "variadic_argument" {
                                        spread = true;
                                    }
                                    count += 1;
                                }
                                (Some(count), spread)
                            })
                            .unwrap_or((None, false));
                        calls.push(CallSiteMeta {
                            callee_name: name,
                            line,
                            qualifier,
                            start_byte: capture.node.start_byte(),
                            end_byte: capture.node.end_byte(),
                            receiver_node: qualifier_node,
                            arg_count,
                            arg_spread,
                            kind_override: None,
                            origin_override: None,
                        });
                    }
                }
            }
            return (calls, facts);
        }

        let mut calls = Vec::new();
        self.collect_calls_manual_with_qualifier_and_spans(*func_node, lines, &mut calls);
        (calls, facts)
    }

    pub(super) fn collect_calls_manual_with_qualifier_and_spans<'a>(
        &'a self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<CallSiteMeta<'a>>,
    ) {
        self.collect_calls_manual_with_qualifier_and_spans_for_owner(node, node, lines, out);
    }

    fn collect_calls_manual_with_qualifier_and_spans_for_owner<'a>(
        &'a self,
        owner: Node<'_>,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<CallSiteMeta<'a>>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line)
            && self.language.is_call_node(node.kind())
            && self.call_capture_is_owned(&owner, node)
        {
            if let Some(name_node) = self.call_function_name(&node) {
                let name = self.node_text(&name_node).to_string();
                let qualifier = self
                    .call_function_qualifier(&node)
                    .map(|q| self.node_text(&q).to_string());
                // Manual fallback surfaces no receiver node (None); type-assertion
                // recovery is Go-only and Go uses the query path above.
                // arg_count/arg_spread left as None/false — arity filter treats None as keep.
                out.push(CallSiteMeta {
                    callee_name: name,
                    line,
                    qualifier,
                    start_byte: node.start_byte(),
                    end_byte: node.end_byte(),
                    receiver_node: None,
                    arg_count: None,
                    arg_spread: false,
                    kind_override: None,
                    origin_override: None,
                });
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_calls_manual_with_qualifier_and_spans_for_owner(owner, child, lines, out);
        }
    }

    pub(super) fn collect_calls_manual_with_qualifier(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(String, usize, Option<String>)>,
    ) {
        self.collect_calls_manual_with_qualifier_for_owner(node, node, lines, out);
    }

    fn collect_calls_manual_with_qualifier_for_owner(
        &self,
        owner: Node<'_>,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(String, usize, Option<String>)>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line)
            && self.language.is_call_node(node.kind())
            && self.call_capture_is_owned(&owner, node)
        {
            if let Some(name_node) = self.call_function_name(&node) {
                let name = self.node_text(&name_node).to_string();
                let qualifier = self
                    .call_function_qualifier(&node)
                    .map(|q| self.node_text(&q).to_string());
                out.push((name, line, qualifier));
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_calls_manual_with_qualifier_for_owner(owner, child, lines, out);
        }
    }

    pub(super) fn collect_calls_manual_with_spans<'a>(
        &'a self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<CallSiteMeta<'a>>,
    ) {
        self.collect_calls_manual_with_spans_for_owner(node, node, lines, out);
    }

    fn collect_calls_manual_with_spans_for_owner<'a>(
        &'a self,
        owner: Node<'_>,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<CallSiteMeta<'a>>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line)
            && self.language.is_call_node(node.kind())
            && self.call_capture_is_owned(&owner, node)
        {
            if let Some(name_node) = self.call_function_name(&node) {
                let name = self.node_text(&name_node).to_string();
                out.push(CallSiteMeta {
                    callee_name: name,
                    line,
                    qualifier: None,
                    start_byte: node.start_byte(),
                    end_byte: node.end_byte(),
                    receiver_node: None,
                    arg_count: None,
                    arg_spread: false,
                    kind_override: None,
                    origin_override: None,
                });
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_calls_manual_with_spans_for_owner(owner, child, lines, out);
        }
    }

    /// Manual recursive call collection (pre-query fallback).
    /// `pub(crate)` for dual-path consistency testing in `queries::tests`.
    pub(crate) fn collect_calls_manual(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(String, usize)>,
    ) {
        self.collect_calls_manual_for_owner(node, node, lines, out);
    }

    fn collect_calls_manual_for_owner(
        &self,
        owner: Node<'_>,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(String, usize)>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line)
            && self.language.is_call_node(node.kind())
            && self.call_capture_is_owned(&owner, node)
        {
            if let Some(name_node) = self.call_function_name(&node) {
                let name = self.node_text(&name_node).to_string();
                out.push((name, line));
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_calls_manual_for_owner(owner, child, lines, out);
        }
    }

    /// Find a function definition by name.
    pub fn find_function_by_name(&self, name: &str) -> Option<Node<'_>> {
        self.find_function_by_name_inner(self.tree.root_node(), name)
    }

    fn find_function_by_name_inner<'a>(&self, node: Node<'a>, name: &str) -> Option<Node<'a>> {
        let types = self.language.function_node_types();
        if types.contains(&node.kind()) {
            if let Some(name_node) = self.language.function_name(&node) {
                if self.node_text(&name_node) == name {
                    return Some(node);
                }
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = self.find_function_by_name_inner(child, name) {
                return Some(found);
            }
        }
        None
    }

    /// Find all call expression names inside a function body.
    /// Returns deduplicated list of callee names.
    pub fn callees_in_function(&self, func_node: &Node<'_>) -> Vec<String> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(query) = get_query(self.language, QueryKind::Calls) {
            let call_idx = query
                .capture_index_for_name("call")
                .expect("Calls query must have @call capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut names = BTreeSet::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == call_idx {
                        if let Some(name_node) = self.call_function_name(&capture.node) {
                            names.insert(self.node_text(&name_node).to_string());
                        }
                    }
                }
            }
            return names.into_iter().collect();
        }

        let mut names = BTreeSet::new();
        self.collect_all_callees_manual(*func_node, &mut names);
        names.into_iter().collect()
    }

    fn collect_all_callees_manual(&self, node: Node<'_>, out: &mut BTreeSet<String>) {
        if self.language.is_call_node(node.kind()) {
            if let Some(name_node) = self.call_function_name(&node) {
                out.insert(self.node_text(&name_node).to_string());
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_all_callees_manual(child, out);
        }
    }
}
