//! A stable partition of the Variable Use nodes present before return assembly.
use super::*;
use std::sync::OnceLock;

type Owner = (String, String, usize);
type UseSpan = (usize, usize, NodeIndex, OnceLock<bool>);

#[derive(Default)]
struct OwnerUses {
    spans: Vec<UseSpan>,
    // Preserve the old containment predicate even for malformed input spans.
    reversed: Vec<UseSpan>,
}

pub(super) struct UseIndex(BTreeMap<Owner, OwnerUses>);

impl UseIndex {
    pub(super) fn new(graph: &DiGraph<CpgNode, CpgEdge>) -> Self {
        let mut owners = BTreeMap::<Owner, OwnerUses>::new();
        for idx in graph.node_indices() {
            #[cfg(test)]
            tests::record_work(1);
            if let CpgNode::Variable {
                file,
                function,
                function_start_line,
                access: VarAccess::Use,
                start_byte,
                end_byte,
                ..
            } = &graph[idx]
            {
                let owner = owners
                    .entry((file.clone(), function.clone(), *function_start_line))
                    .or_default();
                let spans = if start_byte <= end_byte {
                    &mut owner.spans
                } else {
                    &mut owner.reversed
                };
                spans.push((*start_byte, *end_byte, idx, OnceLock::new()));
            }
        }
        for owner in owners.values_mut() {
            owner.spans.sort_by_key(|span| span.0);
        }
        Self(owners)
    }

    pub(super) fn within(
        &self,
        owner: &FunctionId,
        start: usize,
        end: usize,
        parsed: &ParsedFile,
    ) -> Vec<NodeIndex> {
        let Some(owner) = self
            .0
            .get(&(owner.file.clone(), owner.name.clone(), owner.start_line))
        else {
            return Vec::new();
        };
        let lo = owner.spans.partition_point(|span| span.0 < start);
        let mut selected: Vec<_> = owner.spans[lo..]
            .iter()
            .take_while(|span| span.0 <= end)
            .chain(&owner.reversed)
            .filter_map(|(use_start, use_end, idx, semantic)| {
                #[cfg(test)]
                tests::record_work(1);
                // Keep the original comparisons, including zero-width spans.
                (start <= *use_start
                    && *use_end <= end
                    && *semantic.get_or_init(|| {
                        #[cfg(test)]
                        tests::record_semantic();
                        parsed.is_semantic_value_use(*use_start, *use_end)
                    }))
                .then_some(*idx)
            })
            .collect();
        // Selection used to scan nodes in graph insertion order, not byte order.
        selected.sort_unstable_by_key(|idx| idx.index());
        selected
    }
}
