use super::super::test_support::*;
use super::super::*;
#[test]
fn frozen_remove_merge_and_subset_preserve_primary_state() {
    let mut files = BTreeMap::new();
    for path in ["a.py", "b.py"] {
        files.insert(
            path.into(),
            ParsedFile::parse(
                path,
                "def f(x):\n    y = x\n    return y\n",
                crate::languages::Language::Python,
            )
            .unwrap(),
        );
    }
    let mut old = oracle::LegacyDataFlowGraph::build(&files);
    let mut new = DataFlowGraph::build(&files);
    let held = old.clone();
    let held_new = new.clone();
    for exclude in [
        BTreeSet::new(),
        BTreeSet::from(["absent.py".into()]),
        BTreeSet::from(["a.py".into()]),
        BTreeSet::from(["b.py".into()]),
    ] {
        old.remove_files(&exclude);
        new.remove_files(&exclude);
        assert_pair(&old, &new, &files);
        old.merge(oracle::LegacyDataFlowGraph::build_subset(&files, &exclude));
        new.merge(DataFlowGraph::build_subset(&files, &exclude));
        assert_pair(&old, &new, &files);
    }
    old.merge(held.clone());
    new.merge(held_new.clone());
    assert_pair(&old, &new, &files);
    assert_pair(&held, &held_new, &files);
}
