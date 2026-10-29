//! Derived, non-serialized lookup accelerators. Keys borrow the graph's strings;
//! each binding costs one u32 per applicable key, with no per-query allocation.

use std::sync::OnceLock;

use super::graph::ScopeGraph;
use super::types::{BindTarget, Binding, FileId, NamespaceId, ScopeId, Target};

/// Like glob telemetry, this cache is not part of graph identity. Cloning drops
/// it so a clone can be extended independently without copying derived arrays.
#[derive(Debug, Default)]
pub struct GraphBindingIndex(OnceLock<BindingIndex>);

impl Clone for GraphBindingIndex {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl PartialEq for GraphBindingIndex {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for GraphBindingIndex {}

impl GraphBindingIndex {
    pub(super) fn invalidate(&mut self) {
        self.0.take();
    }
}

#[derive(Debug)]
struct BindingIndex {
    binding_count: usize,
    by_name: Vec<u32>,
    by_target: Vec<u32>,
    file_count: usize,
    // Only one path copy per file, rather than an owned name per binding.
    paths: Vec<(FileId, String)>,
}

fn name_key(binding: &Binding) -> (ScopeId, NamespaceId, &str) {
    (binding.scope, binding.ns, &binding.name)
}

fn resolved_target(binding: &Binding) -> &Target {
    match &binding.target {
        BindTarget::Resolved(target) => target,
        BindTarget::Pending(..) => unreachable!("only resolved bindings enter by_target"),
    }
}

impl BindingIndex {
    fn build(graph: &ScopeGraph) -> Self {
        let mut by_name: Vec<u32> = (0..graph.bindings.len() as u32).collect();
        by_name.sort_unstable_by(|&a, &b| {
            name_key(&graph.bindings[a as usize])
                .cmp(&name_key(&graph.bindings[b as usize]))
                .then(a.cmp(&b))
        });
        let mut by_target: Vec<u32> = graph
            .bindings
            .iter()
            .enumerate()
            .filter(|(_, b)| matches!(b.target, BindTarget::Resolved(_)))
            .map(|(i, _)| i as u32)
            .collect();
        by_target.sort_unstable_by(|&a, &b| {
            resolved_target(&graph.bindings[a as usize])
                .cmp(resolved_target(&graph.bindings[b as usize]))
                .then(a.cmp(&b))
        });
        by_target.shrink_to_fit();
        let mut paths: Vec<_> = graph
            .file_paths
            .iter()
            .map(|(path, &file)| (file, path.clone()))
            .collect();
        paths.sort_unstable(); // duplicate IDs retain the lexically first path
        Self {
            binding_count: graph.bindings.len(),
            by_name,
            by_target,
            file_count: graph.file_paths.len(),
            paths,
        }
    }
}

pub(crate) enum BindingIndices<'a> {
    Indexed(std::slice::Iter<'a, u32>),
    Scan {
        bindings: std::iter::Enumerate<std::slice::Iter<'a, Binding>>,
        key: LookupKey<'a>,
    },
}

pub(crate) enum LookupKey<'a> {
    Name(ScopeId, NamespaceId, &'a str),
    Target(&'a Target),
}

impl Iterator for BindingIndices<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Indexed(indices) => indices.next().map(|&i| i as usize),
            Self::Scan { bindings, key } => bindings
                .find(|(_, b)| match key {
                    LookupKey::Name(scope, ns, name) => name_key(b) == (*scope, *ns, *name),
                    LookupKey::Target(target) => {
                        matches!(&b.target, BindTarget::Resolved(t) if t == *target)
                    }
                })
                .map(|(i, _)| i),
        }
    }
}

impl ScopeGraph {
    fn current_binding_index(&self) -> Option<&BindingIndex> {
        // Extremely large graphs cannot be represented by u32 indices.
        if self.bindings.len() > u32::MAX as usize {
            return None;
        }
        let index = self
            .binding_index
            .0
            .get_or_init(|| BindingIndex::build(self));
        // Direct appends to the public Vec bypass add_binding's invalidation.
        (index.binding_count == self.bindings.len()).then_some(index)
    }

    /// Matching indices in insertion order. Remaining policy filters belong to
    /// the caller, exactly as with a linear scan. No query string is copied.
    pub(crate) fn binding_indices<'a>(
        &'a self,
        scope: ScopeId,
        name: &'a str,
        ns: NamespaceId,
    ) -> BindingIndices<'a> {
        if let Some(index) = self.current_binding_index() {
            let key = (scope, ns, name);
            let lo = index
                .by_name
                .partition_point(|&i| name_key(&self.bindings[i as usize]) < key);
            let hi = index
                .by_name
                .partition_point(|&i| name_key(&self.bindings[i as usize]) <= key);
            BindingIndices::Indexed(index.by_name[lo..hi].iter())
        } else {
            BindingIndices::Scan {
                bindings: self.bindings.iter().enumerate(),
                key: LookupKey::Name(scope, ns, name),
            }
        }
    }

    pub(crate) fn resolved_binding_indices<'a>(&'a self, target: &'a Target) -> BindingIndices<'a> {
        if let Some(index) = self.current_binding_index() {
            let lo = index
                .by_target
                .partition_point(|&i| resolved_target(&self.bindings[i as usize]) < target);
            let hi = index
                .by_target
                .partition_point(|&i| resolved_target(&self.bindings[i as usize]) <= target);
            BindingIndices::Indexed(index.by_target[lo..hi].iter())
        } else {
            BindingIndices::Scan {
                bindings: self.bindings.iter().enumerate(),
                key: LookupKey::Target(target),
            }
        }
    }

    pub(crate) fn path_for_file(&self, file: FileId) -> Option<&str> {
        if let Some(index) = self.current_binding_index() {
            if index.file_count == self.file_paths.len() {
                let i = index.paths.partition_point(|(id, _)| *id < file);
                return index
                    .paths
                    .get(i)
                    .filter(|(id, _)| *id == file)
                    .map(|(_, path)| path.as_str());
            }
        }
        self.file_paths
            .iter()
            .find_map(|(path, &id)| (id == file).then_some(path.as_str()))
    }
}

#[cfg(test)]
#[path = "binding_index_tests.rs"]
mod tests;
