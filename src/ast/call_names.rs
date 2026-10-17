//! Line-filtered syntax call names, preserving query and fallback traversal order.
use super::*;

impl ParsedFile {
    /// Collect all function call names on specific lines (1-indexed).
    /// Returns a map from line number to list of called function names found on that line.
    /// Only matches actual AST call nodes — ignores calls inside comments or string literals.
    pub fn call_names_on_lines(&self, lines: &[usize]) -> BTreeMap<usize, Vec<String>> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        let line_set: BTreeSet<usize> = lines.iter().copied().collect();

        if let Some(query) = get_query(self.language, QueryKind::Calls) {
            let call_idx = query
                .capture_index_for_name("call")
                .expect("Calls query must have @call capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            #[cfg(test)]
            record_query(&self.path);
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut result: BTreeMap<usize, Vec<String>> = BTreeMap::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == call_idx {
                        let line = capture.node.start_position().row + 1;
                        if line_set.contains(&line) {
                            if let Some(name_node) = self.language.call_function_name(&capture.node)
                            {
                                let name = self.node_text(&name_node).to_string();
                                result.entry(line).or_default().push(name);
                            }
                        }
                    }
                }
            }
            return result;
        }

        let mut result: BTreeMap<usize, Vec<String>> = BTreeMap::new();
        self.collect_call_names_at_lines_manual(self.tree.root_node(), &line_set, &mut result);
        result
    }

    fn collect_call_names_at_lines_manual(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut BTreeMap<usize, Vec<String>>,
    ) {
        let line = node.start_position().row + 1;
        if self.language.is_call_node(node.kind()) && lines.contains(&line) {
            if let Some(name_node) = self.language.call_function_name(&node) {
                let name = self.node_text(&name_node).to_string();
                out.entry(line).or_default().push(name);
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_call_names_at_lines_manual(child, lines, out);
        }
    }
}

#[cfg(test)]
static QUERY_COUNTS: std::sync::Mutex<BTreeMap<String, usize>> =
    std::sync::Mutex::new(BTreeMap::new());

#[cfg(test)]
fn record_query(path: &str) {
    *QUERY_COUNTS
        .lock()
        .unwrap()
        .entry(path.to_owned())
        .or_default() += 1;
}

#[cfg(test)]
pub(crate) fn take_query_count(path: &str) -> usize {
    QUERY_COUNTS.lock().unwrap().remove(path).unwrap_or(0)
}

#[cfg(test)]
#[test]
fn manual_fallback_batch_preserves_order_and_line_filter() {
    let parsed = ParsedFile::parse(
        "manual-batch.js",
        "function f() {\n outer(inner()); second(); (() => { nested(); })();\n other();\n}\n",
        Language::JavaScript,
    )
    .unwrap();
    let mut batch = BTreeMap::new();
    parsed.collect_call_names_at_lines_manual(
        parsed.tree.root_node(),
        &BTreeSet::from([2, 3]),
        &mut batch,
    );
    for line in [2, 3] {
        let mut singleton = BTreeMap::new();
        parsed.collect_call_names_at_lines_manual(
            parsed.tree.root_node(),
            &BTreeSet::from([line]),
            &mut singleton,
        );
        assert_eq!(batch.get(&line), singleton.get(&line));
    }
    assert_eq!(&batch[&2][..3], &["outer", "inner", "second"]);
    assert!(batch[&2].iter().any(|n| n == "nested"));
    assert_eq!(batch[&3], ["other"]);
    let mut missing = BTreeMap::new();
    parsed.collect_call_names_at_lines_manual(
        parsed.tree.root_node(),
        &BTreeSet::from([99]),
        &mut missing,
    );
    assert!(missing.is_empty());
}
