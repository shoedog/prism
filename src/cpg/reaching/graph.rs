//! Small graph primitives used by the reaching-definitions fixed point.

use std::collections::VecDeque;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BitSet(Vec<u64>);

impl BitSet {
    pub(super) fn new(bits: usize) -> Self {
        Self(vec![0; bits.div_ceil(64)])
    }

    pub(super) fn insert(&mut self, bit: usize) {
        self.0[bit / 64] |= 1 << (bit % 64);
    }

    pub(super) fn contains(&self, bit: usize) -> bool {
        self.0
            .get(bit / 64)
            .is_some_and(|word| word & (1 << (bit % 64)) != 0)
    }

    pub(super) fn members(&self) -> impl Iterator<Item = usize> + '_ {
        self.0.iter().enumerate().flat_map(|(word_index, word)| {
            (0..64).filter_map(move |bit| (word & (1 << bit) != 0).then_some(word_index * 64 + bit))
        })
    }

    pub(super) fn union_with(&mut self, other: &Self) {
        for (left, right) in self.0.iter_mut().zip(&other.0) {
            *left |= right;
        }
    }

    pub(super) fn subtract(&mut self, other: &Self) {
        for (left, right) in self.0.iter_mut().zip(&other.0) {
            *left &= !right;
        }
    }
}

pub(super) fn reverse_postorder(successors: &[Vec<(usize, bool)>], entry: usize) -> Vec<usize> {
    let mut seen = vec![false; successors.len()];
    let mut stack = vec![(entry, false)];
    let mut postorder = Vec::new();
    while let Some((node, expanded)) = stack.pop() {
        if expanded {
            postorder.push(node);
            continue;
        }
        if seen[node] {
            continue;
        }
        seen[node] = true;
        stack.push((node, true));
        for (successor, _) in successors[node].iter().rev() {
            if !seen[*successor] {
                stack.push((*successor, false));
            }
        }
    }
    postorder.reverse();
    postorder
}

pub(super) fn path_exists(
    successors: &[Vec<(usize, bool)>],
    from: usize,
    to: usize,
    allow_incomplete: bool,
) -> bool {
    let mut seen = vec![false; successors.len()];
    let mut queue = VecDeque::from([from]);
    while let Some(node) = queue.pop_front() {
        if node == to {
            return true;
        }
        if seen[node] {
            continue;
        }
        seen[node] = true;
        for (successor, incomplete) in &successors[node] {
            if (allow_incomplete || !incomplete) && !seen[*successor] {
                queue.push_back(*successor);
            }
        }
    }
    false
}

pub(super) fn definition_reaches_unflagged(
    def_index: usize,
    from: usize,
    to: usize,
    kill: &[BitSet],
    successors: &[Vec<(usize, bool)>],
) -> bool {
    let mut seen = vec![false; successors.len()];
    let mut queue = VecDeque::from([from]);
    while let Some(node) = queue.pop_front() {
        if node == to {
            return true;
        }
        if seen[node] {
            continue;
        }
        seen[node] = true;
        for (successor, incomplete) in &successors[node] {
            if !incomplete && !seen[*successor] {
                if *successor == to {
                    return true;
                }
                if !kill[*successor].contains(def_index) {
                    queue.push_back(*successor);
                }
            }
        }
    }
    false
}
