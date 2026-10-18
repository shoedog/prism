use super::owner::OwnerRecord;
use serde_json::{json, Value};

pub(super) fn summarize(owners: &[OwnerRecord], top_n: usize) -> Value {
    let mut ranked: Vec<_> = owners.iter().collect();
    ranked.sort_by(|a, b| {
        b.total_ns
            .cmp(&a.total_ns)
            .then_with(|| (&a.file, a.start_byte, &a.owner).cmp(&(&b.file, b.start_byte, &b.owner)))
    });
    let split = top_n.min(ranked.len());
    let top = &ranked[..split];
    let mut other: Vec<_> = ranked[split..].iter().map(|o| o.total_ns).collect();
    other.sort_unstable();
    let total: u64 = ranked.iter().map(|o| o.total_ns).sum();
    let top_time: u64 = top.iter().map(|o| o.total_ns).sum();
    json!({"record":"summary", "time_kind":"summed_owner_work", "owner_count":owners.len(), "total_owner_ns":total,
        "top_dfg_share": if total == 0 {0.0} else {top_time as f64 / total as f64},
        "top_owners":top.iter().map(|o| json!({"file":o.file,"owner":o.owner,"start_line":o.start_line,"start_byte":o.start_byte,"total_ns":o.total_ns})).collect::<Vec<_>>(),
        "other_owners": {"count":other.len(), "total_ns":other.iter().sum::<u64>(), "p50_ns":percentile(&other,50), "p90_ns":percentile(&other,90), "p99_ns":percentile(&other,99), "max_ns":other.last().copied().unwrap_or(0)}})
}

fn percentile(sorted: &[u64], percent: usize) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    sorted[(sorted.len() * percent).div_ceil(100) - 1]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nearest_rank_handles_empty_and_tail() {
        assert_eq!(percentile(&[], 99), 0);
        assert_eq!(percentile(&[7], 50), 7);
        assert_eq!(percentile(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10], 50), 5);
        assert_eq!(percentile(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10], 99), 10);
    }
}
