//! Per-invocation zero-width use resolution. No AST work on individual lookups.
use super::Line;
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::data_flow::FlowEdge;
use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet};

type Key = (Line, Line, AccessPath); // legacy owner start line, use line, full path

#[derive(Default)]
pub(super) struct UseBytes {
    requests: BTreeSet<Key>,
    values: OnceCell<BTreeMap<Key, Option<usize>>>,
}

impl UseBytes {
    pub(super) fn new<'a>(edges: impl Iterator<Item = &'a FlowEdge>) -> Self {
        Self {
            requests: edges
                .filter(|edge| edge.to.start_byte >= edge.to.end_byte)
                .map(key)
                .collect(),
            values: OnceCell::new(),
        }
    }

    pub(super) fn get(&self, parsed: &ParsedFile, edge: &FlowEdge) -> Option<usize> {
        if edge.to.start_byte < edge.to.end_byte {
            return Some(edge.to.start_byte);
        }
        self.values.get_or_init(|| self.build(parsed))[&key(edge)]
    }

    fn build(&self, parsed: &ParsedFile) -> BTreeMap<Key, Option<usize>> {
        #[cfg(test)]
        super::scope::use_byte_tests::bump(|counts| counts.builds += 1);
        let mut owners = BTreeMap::new();
        // Preserve .find() on the original inventory, including same-line
        // nested/anonymous owners. The caller's exact node can differ from it.
        for owner in parsed.all_functions() {
            owners
                .entry(parsed.node_line_range(&owner).0)
                .or_insert(owner);
        }
        let mut lines_by_owner: BTreeMap<Line, BTreeSet<Line>> = BTreeMap::new();
        for (owner, line, _) in &self.requests {
            lines_by_owner.entry(*owner).or_default().insert(*line);
        }
        let mut spans = BTreeMap::new();
        for (start, lines) in lines_by_owner {
            let Some(owner) = owners.get(&start) else {
                continue; // Missing owner returns None before identifier fallback.
            };
            #[cfg(test)]
            super::scope::use_byte_tests::bump(|counts| counts.rvalue_batches += 1);
            for span in parsed.singleton_rvalue_spans_on_lines(owner, &lines) {
                // Do not deduplicate: even identical query emissions count as
                // multiple spans in the frozen lookup.
                add(&mut spans, (start, span.line, span.path), span.start_byte);
            }
        }
        let fallback_lines = self
            .requests
            .iter()
            .filter(|key| owners.contains_key(&key.0) && !spans.contains_key(*key))
            .map(|key| key.1)
            .collect::<BTreeSet<_>>();
        let identifiers = if fallback_lines.is_empty() {
            BTreeMap::new()
        } else {
            #[cfg(test)]
            super::scope::use_byte_tests::bump(|counts| counts.identifier_batches += 1);
            parsed.identifier_bytes_on_lines(&fallback_lines)
        };
        self.requests
            .iter()
            .map(|key| {
                let value = if !owners.contains_key(&key.0) {
                    None
                } else if let Some(value) = spans.get(key) {
                    *value
                } else {
                    identifiers
                        .get(&(key.1.saturating_sub(1), key.2.base.clone()))
                        .copied()
                        .flatten()
                };
                (key.clone(), value)
            })
            .collect()
    }
}

fn key(edge: &FlowEdge) -> Key {
    (
        edge.to.function_start_line,
        edge.to.line,
        edge.to.path.clone(),
    )
}

fn add<K: Ord>(map: &mut BTreeMap<K, Option<usize>>, key: K, byte: usize) {
    map.entry(key)
        .and_modify(|value| *value = None)
        .or_insert(Some(byte));
}
