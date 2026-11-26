use super::walk::{supports_anchor_tie, Call};
use std::{
    cmp::{Ordering, Reverse},
    collections::BTreeMap,
};

pub(super) fn winners<'a, 'tree>(calls: &'a [Call<'tree>]) -> BTreeMap<usize, &'a Call<'tree>> {
    let mut candidates: BTreeMap<usize, (&Call<'_>, Vec<&Call<'_>>)> = BTreeMap::new();
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
            let (first, tied) = candidates.entry(line).or_insert((call, Vec::new()));
            match key(call).cmp(&key(first)) {
                Ordering::Less => {
                    *first = call;
                    tied.clear();
                }
                Ordering::Equal if call.node != first.node => tied.push(call),
                _ => {}
            }
        }
    }
    candidates
        .into_iter()
        .filter_map(|(line, (first, tied))| {
            // An unlisted order among any of the minimum-key candidates
            // leaves the line without a usable anchor.
            let mut winner = first;
            for (index, call) in tied.iter().enumerate() {
                if !supports_anchor_tie(first.node, call.node)
                    || tied[..index]
                        .iter()
                        .any(|other| !supports_anchor_tie(other.node, call.node))
                {
                    return None;
                }
                if call.order < winner.order {
                    winner = call;
                }
            }
            Some((line + 1, winner))
        })
        .collect()
}
