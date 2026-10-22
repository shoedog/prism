//! A stable partition of the Variable Use nodes present before return assembly.
use super::*;

type Owner = (String, String, usize);
type UseSpan = (usize, usize, NodeIndex);

pub(super) struct UseIndex(BTreeMap<Owner, Vec<UseSpan>>);

impl UseIndex {
    pub(super) fn new(graph: &DiGraph<CpgNode, CpgEdge>) -> Self {
        let mut owners = BTreeMap::<Owner, Vec<UseSpan>>::new();
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
                owners
                    .entry((file.clone(), function.clone(), *function_start_line))
                    .or_default()
                    .push((*start_byte, *end_byte, idx));
            }
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
        self.0
            .get(&(owner.file.clone(), owner.name.clone(), owner.start_line))
            .into_iter()
            .flatten()
            .filter_map(|&(use_start, use_end, idx)| {
                #[cfg(test)]
                tests::record_work(1);
                // Keep the original comparisons, including zero-width spans.
                (start <= use_start
                    && use_end <= end
                    && parsed.is_semantic_value_use(use_start, use_end))
                .then_some(idx)
            })
            .collect()
    }
}
