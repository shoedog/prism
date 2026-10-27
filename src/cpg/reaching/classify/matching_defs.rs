use super::*;

// Frozen main scanner, independent of bucket construction.
pub(in crate::cpg::reaching) fn matching_defs(edge: &FlowEdge, defs: &[DefSite]) -> Vec<usize> {
    super::super::work_tests::bump(|counts| {
        counts.matching_defs_calls += 1;
        counts.matching_defs_items += defs.len();
    });
    let mut matches: Vec<usize> = defs
        .iter()
        .enumerate()
        .filter_map(|(index, def)| {
            (!def.synthetic
                && def.path == edge.from.path
                && def.line == edge.from.line
                && def.start_byte == edge.from.start_byte)
                .then_some(index)
        })
        .collect();
    if matches.is_empty() {
        super::super::work_tests::bump(|counts| {
            counts.matching_defs_fallbacks += 1;
            counts.matching_defs_items += defs.len();
        });
        matches.extend(defs.iter().enumerate().filter_map(|(index, def)| {
            (!def.synthetic && def.path == edge.from.path && def.line == edge.from.line)
                .then_some(index)
        }));
    }
    super::super::work_tests::bump(|counts| counts.projected_matching_defs_items += matches.len());
    matches
}
