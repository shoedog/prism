use super::super::test_support::*;
use super::super::*;
#[test]
fn frozen_queries_cover_cycles_spans_absence_and_same_line_propagation() {
    let files = BTreeMap::from([(
        "a.py".into(),
        ParsedFile::parse(
            "a.py",
            "def f(x):\n    y = x\n    x = y\n    return x\n",
            crate::languages::Language::Python,
        )
        .unwrap(),
    )]);
    let old = oracle::LegacyDataFlowGraph::build(&files);
    let mut new = DataFlowGraph::build(&files);
    assert_pair(&old, &new, &files);
    let e = new.edges[0].clone();
    new.edges.push(e.clone());
    new.forward
        .entry(e.to.clone())
        .or_default()
        .push(e.from.clone());
    let mut old = old;
    old.edges.push(e.clone());
    old.forward.entry(e.to).or_default().push(e.from);
    assert_pair(&old, &new, &files);
}
