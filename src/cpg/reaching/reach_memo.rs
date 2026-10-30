//! Lazy reachability rows, owned by one function's classification pass.
//!
//! Only unflagged queries occur in production. A row includes its root (the
//! zero-edge path), then the closure under unflagged successors. Reversing the
//! edges gives exactly the sources that can reach a target. No CFG is mutated.
use super::graph::BitSet;

type Graph = [Vec<(usize, bool)>];

pub(super) struct Reachability<'a> {
    pub(super) successors: &'a Graph,
    reverse: Vec<Vec<(usize, bool)>>,
    forward: Vec<Option<BitSet>>,
    backward: Vec<Option<BitSet>>,
}

impl<'a> Reachability<'a> {
    pub(super) fn new(successors: &'a Graph) -> Self {
        let mut reverse = vec![Vec::new(); successors.len()];
        for (from, edges) in successors.iter().enumerate() {
            for &(to, incomplete) in edges {
                reverse[to].push((from, incomplete));
            }
        }
        Self {
            successors,
            reverse,
            forward: vec![None; successors.len()],
            backward: vec![None; successors.len()],
        }
    }

    pub(super) fn path_exists(&mut self, from: usize, to: usize) -> bool {
        self.forward[from]
            .get_or_insert_with(|| reachable(self.successors, from))
            .contains(to)
    }

    pub(super) fn reaches_target(&mut self, from: usize, to: usize) -> bool {
        self.backward[to]
            .get_or_insert_with(|| reachable(&self.reverse, to))
            .contains(from)
    }
}

fn reachable(graph: &Graph, root: usize) -> BitSet {
    #[cfg(test)]
    super::reach_memo_tests::walk();
    let mut seen = BitSet::new(graph.len());
    seen.insert(root);
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        for &(next, incomplete) in &graph[node] {
            if !incomplete && !seen.contains(next) {
                seen.insert(next);
                stack.push(next);
            }
        }
    }
    seen
}

/// One row per definition, not per use. Binding-aware and flat kills have
/// separate instances; their immutable inputs cannot alias a cache key.
pub(super) struct DefinitionReachability<'a> {
    successors: &'a Graph,
    kill: &'a [BitSet],
    mapped_defs: &'a [Option<usize>],
    rows: Vec<Option<BitSet>>,
}

impl<'a> DefinitionReachability<'a> {
    pub(super) fn new(
        successors: &'a Graph,
        kill: &'a [BitSet],
        mapped_defs: &'a [Option<usize>],
    ) -> Self {
        Self {
            successors,
            kill,
            mapped_defs,
            rows: vec![None; mapped_defs.len()],
        }
    }

    pub(super) fn reaches(&mut self, def_index: usize, to: usize) -> bool {
        let Some(from) = self.mapped_defs[def_index] else {
            return false;
        };
        self.rows[def_index]
            .get_or_insert_with(|| {
                #[cfg(test)]
                super::reach_memo_tests::walk();
                let mut seen = BitSet::new(self.successors.len());
                seen.insert(from);
                let mut stack = vec![from];
                while let Some(node) = stack.pop() {
                    for &(next, incomplete) in &self.successors[node] {
                        if !incomplete && !seen.contains(next) {
                            // The old walk accepts a killed target before
                            // checking kills. Include it, but never traverse
                            // through it. The source always expands, even if
                            // killed; returning to it cannot add new routes.
                            seen.insert(next);
                            if !self.kill[next].contains(def_index) {
                                stack.push(next);
                            }
                        }
                    }
                }
                seen
            })
            .contains(to)
    }
}

#[cfg(test)]
impl Reachability<'_> {
    pub(super) fn row_counts(&self) -> (usize, usize) {
        (
            self.forward.iter().flatten().count(),
            self.backward.iter().flatten().count(),
        )
    }
}
#[cfg(test)]
impl DefinitionReachability<'_> {
    pub(super) fn row_count(&self) -> usize {
        self.rows.iter().flatten().count()
    }
}
