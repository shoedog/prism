//! Batched equivalent of singleton-line rvalue queries followed by span.line
//! filtering. The general rvalue API cannot be used with the union of lines:
//! multiline captures emit spans on lines other than their triggering line.
use super::{ParsedFile, PathSpan, RvalueOwnerDecision, RvalueQueryScope};
use crate::queries::{get_query, QueryKind};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::{Node, QueryCursor, StreamingIterator};

impl ParsedFile {
    pub(crate) fn singleton_rvalue_spans_on_lines(
        &self,
        owner: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<PathSpan> {
        let scope = self.rvalue_query_scope(owner);
        let mut spans = Vec::new();
        if get_query(self.language, QueryKind::Assignments).is_none() {
            self.singleton_spans_manual(*owner, lines, &scope, true, &mut spans);
        } else {
            for (kind, capture_name) in [
                (QueryKind::Assignments, "assign"),
                (QueryKind::Calls, "call"),
                (QueryKind::Returns, "ret"),
            ] {
                let Some(query) = get_query(self.language, kind) else {
                    continue;
                };
                let index = query.capture_index_for_name(capture_name).unwrap();
                let mut cursor = QueryCursor::new();
                // Exactly the old byte range, never a union point range.
                cursor.set_byte_range(owner.byte_range());
                let mut matches =
                    cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
                while let Some(m) = matches.next() {
                    for capture in m.captures {
                        let node = capture.node;
                        let line = node.start_position().row + 1;
                        if capture.index != index
                            || !lines.contains(&line)
                            || !matches!(
                                self.rvalue_capture_owner(owner, &scope, node),
                                RvalueOwnerDecision::Owned
                            )
                        {
                            continue;
                        }
                        let begin = spans.len();
                        match capture_name {
                            "assign" if self.language.is_assignment_node(node.kind()) => {
                                self.singleton_assignment_spans(node, &scope, &mut spans);
                            }
                            "call" => self.singleton_call_spans(node, &scope, &mut spans),
                            "ret" => {
                                self.collect_return_node_value_spans_scoped(
                                    node, &scope, &mut spans,
                                );
                            }
                            _ => {}
                        }
                        retain_origin(&mut spans, begin, line);
                    }
                }
            }
        }
        spans.sort_by_key(|span| (span.line, span.start_byte, span.end_byte));
        spans
    }

    fn singleton_assignment_spans(
        &self,
        node: Node<'_>,
        scope: &RvalueQueryScope,
        out: &mut Vec<PathSpan>,
    ) {
        if self.is_augmented_assignment(&node) {
            if let Some(lhs) = self.language.assignment_target(&node) {
                self.collect_identifier_path_spans_scoped(lhs, scope, out);
            }
        }
        if let Some(rhs) = self.language.assignment_value(&node) {
            self.collect_identifier_path_spans_scoped(rhs, scope, out);
        }
    }

    fn singleton_call_spans(
        &self,
        node: Node<'_>,
        scope: &RvalueQueryScope,
        out: &mut Vec<PathSpan>,
    ) {
        if let Some(args) = self.language.call_arguments(&node) {
            self.collect_identifier_path_spans_scoped(args, scope, out);
        }
        if let Some(name) = self.language.call_function_name(&node) {
            self.push_identifier_path_span(name, out);
        }
    }

    fn singleton_spans_manual(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        scope: &RvalueQueryScope,
        root: bool,
        out: &mut Vec<PathSpan>,
    ) {
        if !self.rvalue_scope_allows_node(scope, node, root) {
            return;
        }
        if !root && matches!(scope, RvalueQueryScope::CallableBody { .. }) {
            if Self::is_rvalue_class_node(node.kind()) {
                self.singleton_spans_manual(
                    node,
                    lines,
                    &RvalueQueryScope::LegacyInventory,
                    false,
                    out,
                );
                return;
            }
            if self.rvalue_is_nested_callable(scope, node) {
                if node.kind() == "method_definition" {
                    if let Some(name) = node.child_by_field_name("name") {
                        let line = name.start_position().row + 1;
                        if name.kind() == "computed_property_name" && lines.contains(&line) {
                            let begin = out.len();
                            self.collect_identifier_path_spans_scoped(name, scope, out);
                            retain_origin(out, begin, line);
                        }
                    }
                }
                return;
            }
        }
        let line = node.start_position().row + 1;
        if lines.contains(&line) {
            let begin = out.len();
            if self.language.is_assignment_node(node.kind()) {
                self.singleton_assignment_spans(node, scope, out);
            }
            if self.language.is_call_node(node.kind()) {
                self.singleton_call_spans(node, scope, out);
            }
            if self.language.is_return_node(node.kind()) {
                self.collect_return_node_value_spans_scoped(node, scope, out);
                retain_origin(out, begin, line);
                return;
            }
            retain_origin(out, begin, line);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.singleton_spans_manual(child, lines, scope, false, out);
        }
    }

    /// Same broad identifier kinds and start-row test as identifiers_on_line,
    /// with one traversal for the complete set of fallback rows. Keep duplicate
    /// nodes (including missing/zero-width nodes) ambiguous.
    pub(crate) fn identifier_bytes_on_lines(
        &self,
        lines: &BTreeSet<usize>,
    ) -> BTreeMap<(usize, String), Option<usize>> {
        let rows = lines.iter().map(|line| line.saturating_sub(1)).collect();
        let mut out = BTreeMap::new();
        self.identifier_bytes_walk(self.tree.root_node(), &rows, &mut out);
        out
    }

    fn identifier_bytes_walk(
        &self,
        node: Node<'_>,
        rows: &BTreeSet<usize>,
        out: &mut BTreeMap<(usize, String), Option<usize>>,
    ) {
        let start = node.start_position().row;
        if rows.contains(&start) && self.language.is_identifier_node(node.kind()) {
            out.entry((start, self.node_text(&node).to_owned()))
                .and_modify(|value| *value = None)
                .or_insert(Some(node.start_byte()));
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if rows
                .range(child.start_position().row..=child.end_position().row)
                .next()
                .is_some()
            {
                self.identifier_bytes_walk(child, rows, out);
            }
        }
    }
}

fn retain_origin(spans: &mut Vec<PathSpan>, begin: usize, line: usize) {
    let mut write = begin;
    for read in begin..spans.len() {
        if spans[read].line == line {
            spans.swap(write, read);
            write += 1;
        }
    }
    spans.truncate(write);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::Language;

    #[test]
    fn manual_singletons_match_frozen_manual_walk() {
        for (language, source) in [
            (Language::TypeScript, "function f(x,obj) {\nobj.a += x;\nconsume(\nconsume(x));\nfunction nested() { consume(x); }\nclass C { [obj.a]() { return x; } }\nreturn x;\n}\n"),
            (Language::Python, "def f(x):\n    x += x\n    consume(\n        consume(x))\n    def nested():\n        return x\n    return x\n"),
            (Language::Bash, "f() {\nx=$y\necho $x $x\n}\n"),
        ] {
            let parsed = ParsedFile::parse("manual", source, language).unwrap();
            let lines: BTreeSet<_> = (0..=source.lines().count() + 1).collect();
            for owner in parsed.all_functions().into_iter().chain([parsed.tree.root_node()]) {
                let scope = parsed.rvalue_query_scope(&owner);
                let mut actual = Vec::new();
                parsed.singleton_spans_manual(owner, &lines, &scope, true, &mut actual);
                actual.sort_by_key(|span| (span.line, span.start_byte, span.end_byte));
                let mut expected: Vec<_> = lines.iter().flat_map(|line| {
                    let mut spans = Vec::new();
                    parsed.collect_rvalue_spans_manual(owner, &BTreeSet::from([*line]), &mut spans);
                    spans.into_iter().filter(move |span| span.line == *line)
                }).collect();
                expected.sort_by_key(|span| (span.line, span.start_byte, span.end_byte));
                assert_eq!(actual, expected, "manual singleton: {language:?}, {owner:?}");
            }
        }
    }
}
