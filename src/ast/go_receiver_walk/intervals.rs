//! Interval reporting in source/preorder order; prune whole inactive subtrees.
use super::SpanValue;

pub(super) struct Intervals<T> {
    rows: Vec<SpanValue<T>>,
    max_end: Vec<usize>,
    size: usize,
}
impl<T> Default for Intervals<T> {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
impl<T> Intervals<T> {
    pub(super) fn new(mut rows: Vec<SpanValue<T>>) -> Self {
        rows.sort_by_key(|row| row.lo);
        let size = rows.len().max(1).next_power_of_two();
        let mut max_end = vec![0; size * 2];
        for (i, row) in rows.iter().enumerate() {
            max_end[size + i] = row.hi;
        }
        for i in (1..size).rev() {
            max_end[i] = max_end[i * 2].max(max_end[i * 2 + 1]);
        }
        Self {
            rows,
            max_end,
            size,
        }
    }
    pub(super) fn visit(&self, byte: usize, mut consume: impl FnMut(&T) -> bool) {
        fn walk<T>(
            index: &Intervals<T>,
            byte: usize,
            slot: usize,
            lo: usize,
            hi: usize,
            consume: &mut impl FnMut(&T) -> bool,
        ) -> bool {
            if index.max_end[slot] <= byte || index.rows.get(lo).is_none_or(|row| row.lo > byte) {
                return true;
            }
            if hi - lo == 1 {
                return consume(&index.rows[lo].value);
            }
            let mid = (lo + hi) / 2;
            walk(index, byte, slot * 2, lo, mid, consume)
                && walk(index, byte, slot * 2 + 1, mid, hi, consume)
        }
        walk(self, byte, 1, 0, self.size, &mut consume);
    }
}
