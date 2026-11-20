//! B01-B05: CONTROLLED block and coordinate custody, including arithmetic boundaries.
use super::*;

#[test]
fn b01_sparse_baseline_ids_are_respected() {
    let mut f = Fixture::new();
    f.baseline.blocks = vec![block(0, "original.py", 1), block(7, "other.py", 2)];
    f.expanded.blocks = vec![block(0, "ctx.py", 900), block(1, "zctx.py", 901)];
    f.sink("zctx.py", 5, None, vec![]);
    f.sink("ctx.py", 6, None, vec![]);
    let out = f.run().unwrap();
    assert_eq!(bytes(&out.result.blocks[..2]), bytes(&f.baseline.blocks));
    assert_eq!(out.added_block_ids, vec![8, 9]);
    assert_eq!(
        out.result
            .blocks
            .iter()
            .map(|b| b.block_id)
            .collect::<BTreeSet<_>>()
            .len(),
        4
    );
    assert_eq!(out.result.blocks[2].file, "ctx.py");
    assert_eq!(out.result.blocks[3].file, "zctx.py");
    assert_eq!(out.result.blocks[2].diff_lines, BTreeSet::from([6]));
    assert_eq!(out.result.blocks[3].diff_lines, BTreeSet::from([5]));
}

#[test]
fn b02_id_overflow_is_a_typed_refusal_only_when_needed() {
    let mut f = Fixture::new();
    f.baseline.blocks.push(block(usize::MAX, "original.py", 1));
    let no_add = f.run().unwrap();
    assert_eq!(bytes(&no_add.result), bytes(&f.baseline));
    assert!(no_add.added_block_ids.is_empty());
    f.sink("ctx.py", 5, None, vec![]);
    let baseline = bytes(&f.baseline);
    let expanded = bytes(&f.expanded);
    assert!(matches!(f.run(), Err(UnionError::BlockIdOverflow)));
    assert_eq!(bytes(&f.baseline), baseline);
    assert_eq!(bytes(&f.expanded), expanded);
    f.baseline.blocks[0].block_id = usize::MAX - 1;
    let one = f.run().unwrap();
    assert_eq!(one.added_block_ids, vec![usize::MAX]);
    f.sink("zctx.py", 6, None, vec![]);
    assert!(matches!(f.run(), Err(UnionError::BlockIdOverflow)));
}

#[test]
fn b03_empty_baseline_starts_at_zero() {
    let mut f = Fixture::new();
    assert!(f.run().unwrap().result.blocks.is_empty());
    f.sink("ctx.py", 5, None, vec![]);
    let out = f.run().unwrap();
    assert_eq!(out.added_block_ids, vec![0]);
    assert_eq!(out.result.blocks.len(), 1);
    assert_eq!(out.result.blocks[0].block_id, 0);
}

#[test]
fn b04_dropped_perturbations_leave_no_block_or_line() {
    let mut f = Fixture::new();
    f.expanded.blocks = vec![block(0, "foreign.py", 100), block(1, "original.py", 500)];
    let excluded = path(("original.py", 2), ("original.py", 9), EXACT);
    f.sink(
        "original.py",
        9,
        Some(excluded.clone()),
        vec![license("original.py", 2, excluded)],
    );
    f.source("original.py", 2, Some(call("original.py", 200)));
    let dropped = f.run().unwrap();
    assert!(dropped.result.blocks.is_empty());
    assert_eq!(dropped.stats.dropped_sinks, 1);
    assert_eq!(dropped.stats.dropped_sources, 1);
    f.sink(
        "ctx.py",
        5,
        Some(path(("foreign.py", 1), ("ctx.py", 6), WEAK)),
        vec![],
    );
    let out = f.run().unwrap();
    assert_eq!(out.result.blocks.len(), 1);
    assert_eq!(out.result.blocks[0].file, "ctx.py");
    assert_eq!(
        out.result.blocks[0].file_line_map["ctx.py"],
        std::collections::BTreeMap::from([(5, true), (6, false)])
    );
}

#[test]
fn b05_additions_never_enter_baseline_and_true_wins() {
    for reverse in [false, true] {
        let mut f = Fixture::new();
        f.baseline.blocks = vec![block(7, "original.py", 1)];
        let evidence = path(("original.py", 2), ("ctx.py", 5), EXACT);
        f.sink(
            "ctx.py",
            5,
            Some(evidence.clone()),
            vec![license("original.py", 2, evidence.clone())],
        );
        f.source("original.py", 2, Some(evidence));
        f.sink(
            "original.py",
            2,
            Some(path(("ctx.py", 8), ("original.py", 9), EXACT)),
            vec![],
        );
        if reverse {
            f.expanded.findings.swap(0, 2);
            f.expanded.evidence.swap(0, 2);
            f.attribution.sinks[0].finding_index = 2;
            f.attribution.sinks[1].finding_index = 0;
        }
        let out = f.run().unwrap();
        assert_eq!(bytes(&out.result.blocks[0]), bytes(&f.baseline.blocks[0]));
        assert_eq!(out.result.blocks.len(), 3);
        assert_eq!(out.result.blocks[1].file, "ctx.py");
        assert_eq!(out.result.blocks[2].file, "original.py");
        let original = &out.result.blocks[2];
        assert_eq!(
            original.file_line_map["original.py"],
            std::collections::BTreeMap::from([(2, true), (9, false)])
        );
        assert_eq!(original.diff_lines, BTreeSet::from([2]));
        assert_eq!(
            out.result.blocks[1].file_line_map["ctx.py"],
            std::collections::BTreeMap::from([(5, true), (8, false)])
        );
    }
}
