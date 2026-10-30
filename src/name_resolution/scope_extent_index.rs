//! Lazy per-file scope extents. No nesting assumption: crossing, duplicate and
//! disjoint extents all participate in the same minimum-width / largest-ID rule.

use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::sync::OnceLock;

use super::graph::ScopeGraph;
use super::types::{FileId, ScopeId};

/// Derived data, like GraphBindingIndex: not serialized or part of identity.
/// Cloning drops the cache so the clone can be mutated independently.
#[derive(Debug, Default)]
pub struct GraphScopeExtentIndex(OnceLock<ExtentIndex>);

impl Clone for GraphScopeExtentIndex {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl PartialEq for GraphScopeExtentIndex {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for GraphScopeExtentIndex {}

impl GraphScopeExtentIndex {
    pub(super) fn invalidate(&mut self) {
        self.0.take();
    }
}

#[derive(Debug)]
struct Extent {
    lo: usize,
    hi: usize,
    scope: ScopeId,
}

#[derive(Debug)]
struct ExtentIndex {
    scope_count: usize,
    by_file: BTreeMap<FileId, Vec<Extent>>,
}

impl ExtentIndex {
    fn build(graph: &ScopeGraph) -> Self {
        let mut by_file: BTreeMap<FileId, Vec<Extent>> = BTreeMap::new();
        for (&scope, record) in &graph.scopes {
            for ext in &record.extents {
                by_file.entry(ext.file).or_default().push(Extent {
                    lo: ext.range.lo.byte,
                    hi: ext.range.hi.byte,
                    // Use the map key, exactly as the original scan does.
                    scope,
                });
            }
        }
        for extents in by_file.values_mut() {
            extents.sort_unstable_by_key(|e| e.lo);
        }
        Self {
            scope_count: graph.scopes.len(),
            by_file,
        }
    }

    fn enclosing(&self, file: FileId, byte: usize) -> Option<ScopeId> {
        let extents = self.by_file.get(&file)?;
        let end = extents.partition_point(|e| e.lo <= byte);
        extents[..end]
            .iter()
            .filter(|e| byte < e.hi)
            .min_by_key(|e| (e.hi - e.lo, Reverse(e.scope)))
            .map(|e| e.scope)
    }
}

/// Safe path when a public-map insertion/removal changes the scope count.
/// In-place edits (including extent additions) require explicit invalidation,
/// just like public binding/edge key edits in GraphBindingIndex.
fn scan(graph: &ScopeGraph, file: FileId, byte: usize) -> Option<ScopeId> {
    graph
        .scopes
        .iter()
        .flat_map(|(&id, scope)| scope.extents.iter().map(move |ext| (id, ext)))
        .filter(|(_, ext)| {
            ext.file == file && byte >= ext.range.lo.byte && byte < ext.range.hi.byte
        })
        .min_by_key(|(id, ext)| (ext.range.hi.byte - ext.range.lo.byte, Reverse(*id)))
        .map(|(id, _)| id)
}

impl ScopeGraph {
    /// O(log(files) + log(file extents) + file prefix), independent of scopes in
    /// other files after the first lookup. OnceLock permits concurrent readers;
    /// invalidation requires exclusive &mut access and no hot-path mutex.
    pub(crate) fn indexed_enclosing_scope(&self, file: FileId, byte: usize) -> Option<ScopeId> {
        let index = self
            .scope_extent_index
            .0
            .get_or_init(|| ExtentIndex::build(self));
        if index.scope_count != self.scopes.len() {
            return scan(self, file, byte);
        }
        let found = index.enclosing(file, byte);
        // Same bounded debug cross-check as #546. Release never scans here.
        #[cfg(debug_assertions)]
        if self.scopes.len() <= 4096 {
            assert_eq!(
                found,
                scan(self, file, byte),
                "scope extent lookup equivalence: invalidate lookup indexes after editing extents"
            );
        }
        found
    }
}

#[cfg(test)]
#[path = "scope_extent_index_tests.rs"]
mod tests;
