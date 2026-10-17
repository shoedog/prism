use super::super::test_support::*;
use super::super::*;

#[test]
fn views_include_spans_and_preserve_native_order() {
    let files = BTreeMap::from([(
        "fixture.py".into(),
        ParsedFile::parse(
            "fixture.py",
            "def f(x):\n y = x + x\n return y\n",
            crate::languages::Language::Python,
        )
        .unwrap(),
    )]);
    let g = DataFlowGraph::build(&files);
    let old = oracle::LegacyDataFlowGraph::build(&files);
    let actual: Vec<_> = g.edges().iter().map(|e| e.to_owned()).collect();
    assert_eq!(bytes(&old.edges), bytes(&actual));
    let actual: Vec<_> = g.defs().map(|(k, v)| (k, v.to_vec())).collect();
    assert_eq!(bytes(&old.defs.iter().collect::<Vec<_>>()), bytes(&actual));
    let actual: Vec<_> = g.uses().map(|(k, v)| (k, v.to_vec())).collect();
    assert_eq!(bytes(&old.uses.iter().collect::<Vec<_>>()), bytes(&actual));
    assert_eq!(g.edges().len(), old.edges.len());
    assert_eq!(g.edges().is_empty(), old.edges.is_empty());
    assert_eq!(current_storage(&g), old_storage(&old));
}

#[test]
fn views_preserve_independent_representatives_duplicates_empty_rows_and_absence() {
    let files = BTreeMap::from([(
        "fixture.py".into(),
        ParsedFile::parse(
            "fixture.py",
            "def f(x):\n y = x + x\n return y\n",
            crate::languages::Language::Python,
        )
        .unwrap(),
    )]);
    let mut raw = oracle::LegacyDataFlowGraph::build(&files);
    let e = raw.edges[0].clone();
    let mut shifted = e.from.clone();
    shifted.start_byte += 100;
    shifted.end_byte += 100;
    raw.forward.clear();
    raw.forward
        .insert(shifted.clone(), vec![e.to.clone(), e.to.clone()]);
    raw.backward.clear();
    raw.backward
        .insert(e.to.clone(), vec![shifted.clone(), e.from.clone()]);
    let empty_key = ("empty.py".into(), "f".into(), 1, AccessPath::simple("z"));
    raw.defs.insert(empty_key.clone(), vec![]);
    raw.uses.insert(empty_key, vec![]);
    let g = from_legacy(raw.clone());
    assert_eq!(
        bytes(&raw.forward.iter().collect::<Vec<_>>()),
        bytes(
            &g.forward()
                .map(|(k, v)| (k, v.to_vec()))
                .collect::<Vec<_>>()
        )
    );
    assert_eq!(
        bytes(&raw.backward.iter().collect::<Vec<_>>()),
        bytes(
            &g.backward()
                .map(|(k, v)| (k, v.to_vec()))
                .collect::<Vec<_>>()
        )
    );
    assert_eq!(
        bytes(&raw.forward[&e.from]),
        bytes(&g.successors(&e.from).unwrap().to_vec())
    );
    assert_eq!(
        bytes(&raw.backward[&e.to]),
        bytes(&g.predecessors(&e.to).unwrap().to_vec())
    );
    let actual: Vec<_> = g.labels().map(|(e, c)| ((e.from, e.to), c)).collect();
    assert_eq!(
        bytes(&raw.labels.iter().collect::<Vec<_>>()),
        bytes(&actual)
    );
    for ((f, t), c) in &raw.labels {
        assert_eq!(g.label(f, t), Some(*c));
    }
    assert_eq!(
        bytes(&raw.exact_labels),
        bytes(&g.exact_labels().collect::<Vec<_>>())
    );
    assert_eq!(bytes(&raw.rd_function_stats), bytes(g.rd_function_stats()));
    assert_eq!(
        g.defs()
            .find(|(k, _)| k.0 == "empty.py")
            .unwrap()
            .1
            .to_vec()
            .len(),
        0
    );
    assert_eq!(
        g.uses()
            .find(|(k, _)| k.0 == "empty.py")
            .unwrap()
            .1
            .to_vec()
            .len(),
        0
    );
    let mut absent = shifted;
    absent.file = "absent.py".into();
    assert!(g.successors(&absent).is_none());
    assert!(g.predecessors(&absent).is_none());
    assert!(g.label(&absent, &e.to).is_none());
    let empty = DataFlowGraph::empty();
    assert!(empty.edges().is_empty());
    assert_eq!(empty.edges().len(), 0);
    assert_eq!(empty.edges().iter().count(), 0);
    assert_eq!(empty.defs().count(), 0);
    assert_eq!(empty.uses().count(), 0);
    assert_eq!(empty.forward().count(), 0);
    assert_eq!(empty.backward().count(), 0);
    assert_eq!(empty.labels().count(), 0);
    assert_eq!(empty.exact_labels().count(), 0);
}
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
    let mut raw = to_legacy(&new);
    let e = raw.edges[0].clone();
    raw.edges.push(e.clone());
    raw.forward
        .entry(e.to.clone())
        .or_default()
        .push(e.from.clone());
    new = from_legacy(raw);
    let mut old = old;
    old.edges.push(e.clone());
    old.forward.entry(e.to).or_default().push(e.from);
    assert_pair(&old, &new, &files);
}
