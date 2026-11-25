use super::walk::Call;
use std::cmp::Reverse;
use std::collections::BTreeMap;

pub(super) fn winners<'a, 'tree>(calls: &'a [Call<'tree>]) -> BTreeMap<usize, &'a Call<'tree>> {
    let mut winners: BTreeMap<usize, &Call<'_>> = BTreeMap::new();
    for call in calls {
        let start = call.callee.start_position().row;
        let end = call.callee.end_position().row;
        for line in start..=end {
            let key = |candidate: &Call<'_>| {
                (
                    candidate.local_annotation,
                    candidate.callee.end_position().row - line,
                    candidate.callee.start_position().column,
                    Reverse(candidate.node.end_position().column),
                )
            };
            let winner = winners.entry(line).or_insert(call);
            if key(call)
                .cmp(&key(winner))
                .then_with(|| call.order.cmp(&winner.order))
                .is_lt()
            {
                *winner = call;
            }
        }
    }
    winners
        .into_iter()
        .map(|(line, call)| (line + 1, call))
        .collect()
}
