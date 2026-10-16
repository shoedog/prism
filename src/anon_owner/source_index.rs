//! Step-1 source lookup bound to one immutable parse; no tree nodes are retained.
use super::{
    candidate_nodes, identity, is_anonymous_owner_name, source_bytes, FunctionId, ParsedFile,
};
use std::cell::OnceCell;
use std::collections::BTreeMap;

type SourceSpans = BTreeMap<(String, usize, usize), (usize, usize)>;

/// Owned only by CPG Step 1, then dropped before variable construction.
pub(crate) struct Step1Sources<'a> {
    files: &'a BTreeMap<String, ParsedFile>,
    indexes: BTreeMap<&'a str, SourceIndex<'a>>,
}

impl<'a> Step1Sources<'a> {
    pub(crate) fn new(files: &'a BTreeMap<String, ParsedFile>) -> Self {
        Self {
            files,
            indexes: BTreeMap::new(),
        }
    }

    pub(crate) fn source_bytes(&mut self, id: &FunctionId) -> Option<(usize, usize)> {
        let (file, parsed) = self.files.get_key_value(&id.file)?;
        if !is_anonymous_owner_name(&id.name) {
            return source_bytes(parsed, id);
        }
        self.indexes
            .entry(file.as_str())
            .or_insert_with(|| SourceIndex::new(parsed))
            .source_bytes(id)
    }
}

pub(crate) struct SourceIndex<'a> {
    parsed: &'a ParsedFile,
    anonymous: OnceCell<SourceSpans>,
}

impl<'a> SourceIndex<'a> {
    pub(crate) fn new(parsed: &'a ParsedFile) -> Self {
        Self {
            parsed,
            anonymous: OnceCell::new(),
        }
    }

    pub(crate) fn source_bytes(&self, id: &FunctionId) -> Option<(usize, usize)> {
        if !is_anonymous_owner_name(&id.name) {
            return source_bytes(self.parsed, id);
        }
        self.anonymous_spans()
            .get(&(id.name.clone(), id.start_line, id.end_line))
            .copied()
    }

    fn anonymous_spans(&self) -> &SourceSpans {
        self.anonymous.get_or_init(|| {
            let mut spans = SourceSpans::new();
            for node in candidate_nodes(self.parsed, true) {
                // The old lookup injects the requested file into every identity,
                // so that always-equal component is omitted to preserve aliases.
                if let Some(id) = identity(self.parsed, &node, "") {
                    spans
                        .entry((id.name, id.start_line, id.end_line))
                        // Match the original ordered `.find()`, including collisions.
                        .or_insert((node.start_byte(), node.end_byte()));
                }
            }
            spans
        })
    }
}

#[cfg(test)]
#[path = "source_index_tests.rs"]
mod tests;
