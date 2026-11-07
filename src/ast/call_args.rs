//! The call-argument index and call-argument text queries.

use crate::ast::{CallArg, CallArgsIndex, ParsedFile};
use crate::languages::Language;
use std::collections::BTreeMap;
use tree_sitter::Node;

impl ParsedFile {
    pub(crate) fn call_args_index(&self) -> &CallArgsIndex {
        self.call_args.get_or_init(|| self.build_call_args_index())
    }

    fn build_call_args_index(&self) -> CallArgsIndex {
        let mut by_call = BTreeMap::new();
        self.index_call_args(self.tree.root_node(), &mut by_call);
        CallArgsIndex { by_call }
    }

    fn index_call_args(&self, node: Node<'_>, out: &mut BTreeMap<(usize, String), Vec<CallArg>>) {
        if self.language.is_call_node(node.kind()) {
            if let Some(name_node) = self.call_function_name(&node) {
                let key = (node.start_byte(), self.node_text(&name_node).to_string());
                // First pre-order occurrence wins — mirrors the legacy first-match.
                out.entry(key)
                    .or_insert_with(|| self.named_arg_spans(&node));
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.index_call_args(child, out);
        }
    }

    fn named_arg_spans(&self, call_node: &Node<'_>) -> Vec<CallArg> {
        let mut args = Vec::new();
        if let Some(args_node) = self.language.call_arguments(call_node) {
            let mut cursor = args_node.walk();
            for child in args_node.children(&mut cursor) {
                if self.is_positional_argument_node(child) {
                    args.push(CallArg {
                        start_byte: child.start_byte(),
                        end_byte: child.end_byte(),
                    });
                }
            }
        }
        args
    }

    /// JS/TS grammars expose comment trivia as named argument-list children.
    /// Preserve expression slots (including spread), but never give trivia an
    /// argument index. Other languages retain their existing extraction contract.
    pub(super) fn is_positional_argument_node(&self, node: Node<'_>) -> bool {
        node.is_named()
            && !(matches!(
                self.language,
                Language::JavaScript | Language::TypeScript | Language::Tsx
            ) && node.kind() == "comment")
    }

    /// Derive an argument's text from its span, byte-identically to the legacy
    /// `node_text(child).trim().trim_start_matches('&')`. Mirrors `node_text`'s
    /// `utf8_text(...).unwrap_or("")` (panic-safe on a malformed span — yields "").
    pub(super) fn arg_text(&self, a: &CallArg) -> String {
        // `.get(..)` (not direct `[..]`) so an out-of-range span yields "" instead of
        // panicking — matches the stated panic-safe guarantee and `node_text`'s
        // `utf8_text(...).unwrap_or("")`.
        self.source
            .as_bytes()
            .get(a.start_byte..a.end_byte)
            .and_then(|b| std::str::from_utf8(b).ok())
            .unwrap_or("")
            .trim()
            .trim_start_matches('&')
            .to_string()
    }

    /// Extract the Nth argument expression text from a call expression on a given line.
    ///
    /// Searches for call expressions on the specified line, then returns the text
    /// of the argument at `arg_index` (0-based).
    pub fn call_argument_text_at(
        &self,
        line: usize,
        callee_name: &str,
        arg_index: usize,
    ) -> Option<String> {
        self.find_call_arg_at(self.tree.root_node(), line, callee_name, arg_index)
    }

    fn find_call_arg_at(
        &self,
        node: Node<'_>,
        line: usize,
        callee_name: &str,
        arg_index: usize,
    ) -> Option<String> {
        let node_line = node.start_position().row + 1;

        if node_line == line && self.language.is_call_node(node.kind()) {
            if let Some(name_node) = self.call_function_name(&node) {
                let name = self.node_text(&name_node);
                if name == callee_name {
                    if let Some(args_node) = self.language.call_arguments(&node) {
                        // Count expression children, excluding JS/TS comment trivia.
                        let mut arg_idx = 0;
                        let mut cursor = args_node.walk();
                        for child in args_node.children(&mut cursor) {
                            if self.is_positional_argument_node(child) {
                                if arg_idx == arg_index {
                                    let text = self.node_text(&child).trim().to_string();
                                    // Strip address-of operator
                                    let text = text.trim_start_matches('&').to_string();
                                    return Some(text);
                                }
                                arg_idx += 1;
                            }
                        }
                    }
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(result) = self.find_call_arg_at(child, line, callee_name, arg_index) {
                return Some(result);
            }
        }
        None
    }

    /// Extract all argument texts from a call to `callee_name` on the given line.
    ///
    /// Returns a list of argument expressions as strings, preserving positional order.
    /// Used by interprocedural data flow to map arguments to parameters.
    pub fn call_argument_texts(&self, line: usize, callee_name: &str) -> Vec<String> {
        let mut args = Vec::new();
        self.collect_call_args(self.tree.root_node(), line, callee_name, &mut args);
        args
    }

    /// Like `call_argument_texts`, but selects the call expression whose start
    /// byte == `start_byte` (disambiguates multiple calls on one line).
    pub fn call_argument_texts_at(&self, start_byte: usize, callee_name: &str) -> Vec<String> {
        self.call_argument_texts_and_spans_at(start_byte, callee_name)
            .into_iter()
            .map(|(text, _)| text)
            .collect()
    }

    /// Like `call_argument_texts_at`, while retaining each argument node's
    /// half-open source-byte span. The text-only API remains a wrapper because
    /// its consumers intentionally do not need occurrence identity.
    pub(crate) fn call_argument_texts_and_spans_at(
        &self,
        start_byte: usize,
        callee_name: &str,
    ) -> Vec<(String, std::ops::Range<usize>)> {
        self.call_args_index()
            .by_call
            .get(&(start_byte, callee_name.to_string()))
            .map(|spans| {
                spans
                    .iter()
                    .map(|a| (self.arg_text(a), a.start_byte..a.end_byte))
                    .collect()
            })
            .unwrap_or_default()
    }

    #[cfg(test)]
    pub(super) fn call_argument_texts_at_reference(
        &self,
        start_byte: usize,
        callee_name: &str,
    ) -> Vec<String> {
        let mut args = Vec::new();
        self.collect_call_args_at_reference(
            self.tree.root_node(),
            start_byte,
            callee_name,
            &mut args,
        );
        args
    }

    fn collect_call_args(
        &self,
        node: Node<'_>,
        line: usize,
        callee_name: &str,
        out: &mut Vec<String>,
    ) {
        let node_line = node.start_position().row + 1;

        if node_line == line && self.language.is_call_node(node.kind()) {
            if let Some(name_node) = self.call_function_name(&node) {
                let name = self.node_text(&name_node);
                if name == callee_name {
                    if let Some(args_node) = self.language.call_arguments(&node) {
                        let mut cursor = args_node.walk();
                        for child in args_node.children(&mut cursor) {
                            if self.is_positional_argument_node(child) {
                                let text = self.node_text(&child).trim().to_string();
                                let text = text.trim_start_matches('&').to_string();
                                out.push(text);
                            }
                        }
                    }
                    return; // Found the call, stop.
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if !out.is_empty() {
                return;
            }
            self.collect_call_args(child, line, callee_name, out);
        }
    }

    #[cfg(test)]
    fn collect_call_args_at_reference(
        &self,
        node: Node<'_>,
        start_byte: usize,
        callee_name: &str,
        out: &mut Vec<String>,
    ) {
        if node.start_byte() == start_byte && self.language.is_call_node(node.kind()) {
            if let Some(name_node) = self.call_function_name(&node) {
                let name = self.node_text(&name_node);
                if name == callee_name {
                    if let Some(args_node) = self.language.call_arguments(&node) {
                        let mut cursor = args_node.walk();
                        for child in args_node.children(&mut cursor) {
                            if self.is_positional_argument_node(child) {
                                let text = self.node_text(&child).trim().to_string();
                                let text = text.trim_start_matches('&').to_string();
                                out.push(text);
                            }
                        }
                    }
                    return; // Found the call, stop.
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if !out.is_empty() {
                return;
            }
            self.collect_call_args_at_reference(child, start_byte, callee_name, out);
        }
    }

    /// Check if a function node has a variadic parameter (`...`).
    ///
    /// In C/C++, tree-sitter represents `...` as a `variadic_parameter` node
    /// inside the parameter list.
    pub fn is_variadic_function(&self, func_node: &Node<'_>) -> bool {
        if let Some(params) = self.find_parameters_node(func_node) {
            let mut cursor = params.walk();
            for child in params.children(&mut cursor) {
                if child.kind() == "variadic_parameter" {
                    return true;
                }
            }
        }
        false
    }
}
