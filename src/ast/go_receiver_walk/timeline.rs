//! Ordered binding summaries. Sweep byte intervals once; queries binary-search
//! checkpoints instead of replaying declarations before each call.
use super::{Evidence, Recovery};
use std::collections::BTreeMap;

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub(super) struct Summary {
    count: usize,
    first: Option<Recovery>,
    last: Option<Recovery>,
    lexical: bool,
    later_lexical: bool,
}

impl Summary {
    pub(super) fn binding(value: Option<Recovery>, lexical: bool) -> Self {
        Self {
            count: 1,
            first: value.clone(),
            last: value,
            lexical,
            later_lexical: false,
        }
    }
    pub(super) fn join(&self, rhs: &Self) -> Self {
        if self.count == 0 {
            return rhs.clone();
        }
        if rhs.count == 0 {
            return self.clone();
        }
        Self {
            count: self.count + rhs.count,
            first: self.first.clone(),
            last: rhs.last.clone(),
            lexical: self.lexical || rhs.lexical,
            later_lexical: self.later_lexical || rhs.later_lexical || rhs.lexical,
        }
    }
    pub(super) fn parameters(mut self) -> Self {
        // The reference sets first_found only AFTER the entire parameter list.
        if self.count > 1 {
            self.first = None;
        }
        self
    }
    pub(super) fn evidence(&self) -> Evidence {
        if self.count > 1 {
            (
                None,
                self.count,
                if self.later_lexical {
                    self.first.clone()
                } else {
                    None
                },
            )
        } else {
            (self.last.clone(), self.count, self.first.clone())
        }
    }
}

pub(super) struct Event {
    pub lo: usize,
    pub hi: usize,
    pub value: Summary,
    pub var: bool,
    pub reuse: bool,
}

pub(super) struct Timeline(Vec<(usize, Summary)>);
impl Timeline {
    pub(super) fn new(seed: &Summary, events: &[Event], var: bool, reuse: bool) -> Self {
        let size = events.len().max(1).next_power_of_two();
        let mut tree = vec![Summary::default(); size * 2];
        let mut changes: BTreeMap<usize, Vec<(usize, bool)>> = BTreeMap::new();
        for (i, event) in events.iter().enumerate() {
            if (event.var && !var) || (event.reuse && reuse) || event.lo >= event.hi {
                continue;
            }
            changes.entry(event.lo).or_default().push((i, true));
            changes.entry(event.hi).or_default().push((i, false));
        }
        let mut points = vec![(0, seed.clone())];
        for (byte, edits) in changes {
            for (i, active) in edits {
                let mut slot = size + i;
                tree[slot] = if active {
                    events[i].value.clone()
                } else {
                    Summary::default()
                };
                while slot > 1 {
                    slot /= 2;
                    tree[slot] = tree[slot * 2].join(&tree[slot * 2 + 1]);
                }
            }
            let state = seed.join(&tree[1]);
            if points.last().unwrap().1 != state {
                points.push((byte, state));
            }
        }
        Self(points)
    }
    pub(super) fn at(&self, byte: usize) -> Evidence {
        self.0[self.0.partition_point(|(start, _)| *start <= byte) - 1]
            .1
            .evidence()
    }
}
