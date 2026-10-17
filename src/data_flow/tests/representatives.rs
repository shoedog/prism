use super::super::compact::*;
use super::super::*;
use super::identity::{arena, loc};
use crate::cpg::FlowDoubt;
#[test]
fn map_representatives_are_independent() {
    let d0 = loc("x", VarAccessKind::Def, 10);
    let d1 = loc("x", VarAccessKind::Def, 20);
    let u = loc("x", VarAccessKind::Use, 30);
    let g = arena(vec![d0.clone(), d1.clone(), u.clone()]);
    let (d0, d1, u) = (
        g.exact(&d0).unwrap(),
        g.exact(&d1).unwrap(),
        g.exact(&u).unwrap(),
    );
    let mut adj = BTreeMap::new();
    let mut labels = BTreeMap::new();
    add_adj(&mut adj, &g, d1, u);
    add_label(&mut labels, &g, d0, u, FlowConfidence::Exact);
    assert_eq!(
        g.get(adj.values().next().unwrap().representative)
            .start_byte,
        20
    );
    assert_eq!(g.get(labels.values().next().unwrap().from).start_byte, 10);
}
#[test]
fn adjacency_retains_duplicates_and_input_order() {
    let d = loc("x", VarAccessKind::Def, 1);
    let u0 = loc("x", VarAccessKind::Use, 10);
    let u1 = loc("x", VarAccessKind::Use, 20);
    let g = arena(vec![d.clone(), u0.clone(), u1.clone()]);
    let (d, u0, u1) = (
        g.exact(&d).unwrap(),
        g.exact(&u0).unwrap(),
        g.exact(&u1).unwrap(),
    );
    let mut adj = BTreeMap::new();
    for u in [u1, u0, u1, u0] {
        add_adj(&mut adj, &g, d, u);
    }
    assert_eq!(adj.values().next().unwrap().neighbors, vec![u1, u0, u1, u0]);
}
#[test]
fn labels_keep_first_pair_and_merge_worst() {
    let d = loc("x", VarAccessKind::Def, 1);
    let u0 = loc("x", VarAccessKind::Use, 10);
    let u1 = loc("x", VarAccessKind::Use, 20);
    let g = arena(vec![d.clone(), u0.clone(), u1.clone()]);
    let (d, u0, u1) = (
        g.exact(&d).unwrap(),
        g.exact(&u0).unwrap(),
        g.exact(&u1).unwrap(),
    );
    let mut labels = BTreeMap::new();
    add_label(&mut labels, &g, d, u1, FlowConfidence::Exact);
    add_label(
        &mut labels,
        &g,
        d,
        u0,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 7 }),
    );
    add_label(
        &mut labels,
        &g,
        d,
        u0,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
    );
    let v = labels.values().next().unwrap();
    assert_eq!(v.to, u1);
    assert_eq!(
        v.confidence,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 })
    );
}
#[test]
fn equal_legacy_neighbors_stable_sort_keeps_first_span() {
    let mut vs = vec![
        loc("x", VarAccessKind::Use, 20),
        loc("x", VarAccessKind::Use, 10),
    ];
    vs.sort();
    vs.dedup();
    assert_eq!(vs.len(), 1);
    assert_eq!(vs[0].start_byte, 20);
    let d20 = loc("x", VarAccessKind::Def, 20);
    let d10 = loc("x", VarAccessKind::Def, 10);
    let sink = loc("y", VarAccessKind::Use, 30);
    let mut p = test_support::parts::empty();
    p.edges = [d20.clone(), d10]
        .into_iter()
        .map(|from| FlowEdge {
            from,
            to: sink.clone(),
        })
        .collect();
    p.labels.insert((d20, sink.clone()), FlowConfidence::Exact);
    p.adjacency = None;
    let g = CompactDfg::from_parts(p).unwrap();
    let (visited, hops) = g.backward_reachable_labeled(&sink);
    assert_eq!(visited.iter().next().unwrap().start_byte, 20);
    assert!(
        matches!(&hops[0], crate::finding_confidence::EvidenceHop::DataFlow { from, .. } if from.start_byte == 20)
    );
}

/// Explicit, independently checked primary input; no generated expectations.
pub(super) fn adversarial_parts() -> Parts {
    let at = |name, kind, start, file: &str| {
        let mut v = loc(name, kind, start);
        v.file = file.into();
        v
    };
    let d10 = at("x", VarAccessKind::Def, 10, "a.py");
    let d20 = at("x", VarAccessKind::Def, 20, "a.py");
    let u30 = at("x", VarAccessKind::Use, 30, "a.py");
    let u40 = at("x", VarAccessKind::Use, 40, "a.py");
    let bu30 = at("x", VarAccessKind::Use, 30, "b.py");
    let bu40 = at("x", VarAccessKind::Use, 40, "b.py");
    let bd = at("z", VarAccessKind::Def, 50, "b.py");
    let bu = at("z", VarAccessKind::Use, 60, "b.py");
    let key = |file: &str, name: &str| (file.into(), "f".into(), 1, AccessPath::simple(name));
    let mut p = test_support::parts::empty();
    p.edges = [
        (d20.clone(), u40.clone()),
        (d10.clone(), u30.clone()),
        (d20.clone(), u40.clone()),
        (d20.clone(), bu30),
    ]
    .into_iter()
    .map(|(from, to)| FlowEdge { from, to })
    .collect();
    p.defs = BTreeMap::from([
        (key("a.py", "x"), vec![d20.clone(), d10.clone()]),
        (key("b.py", "empty"), vec![]),
        (key("b.py", "foreign"), vec![d20.clone()]),
    ]);
    p.uses = BTreeMap::from([(key("a.py", "x"), vec![u40.clone(), u30.clone()])]);
    let killed = |kill_line| FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line });
    for ((f, t), c) in [
        ((d10.clone(), u30.clone()), FlowConfidence::Exact),
        ((d20.clone(), u40.clone()), killed(7)),
        ((d20.clone(), u40.clone()), killed(3)),
    ] {
        p.labels
            .entry((f, t))
            .and_modify(|v| *v = v.worst(c))
            .or_insert(c);
    }
    p.labels.insert((bd, bu), FlowConfidence::Exact);
    p.exact_labels
        .insert(ExactFlowEdge::from_refs(&d20, &bu40), killed(7));
    p.rd_function_stats
        .entry("a.py".into())
        .or_default()
        .record_without_cfg("f".into(), 1);
    let only = at("adj-only", VarAccessKind::Use, 90, "a.py");
    p.adjacency = Some((
        BTreeMap::from([
            (d10.clone(), vec![u40.clone(), u30.clone(), u40.clone()]),
            (only, vec![]),
        ]),
        BTreeMap::from([(u30.clone(), vec![d10.clone(), d20.clone()])]),
    ));
    // Hand-selected byte observations precede use of these inputs as an oracle.
    let (f, b) = p.adjacency.as_ref().unwrap();
    assert_eq!(
        f.iter()
            .find(|(v, _)| v.path.base == "x")
            .unwrap()
            .0
            .start_byte,
        10
    );
    assert_eq!(b.first_key_value().unwrap().0.start_byte, 30);
    assert_eq!(
        test_support::bytes(&p.defs[&key("a.py", "x")]),
        test_support::bytes(&vec![d20, d10])
    );
    let ((from, to), c) = p.labels.first_key_value().unwrap();
    assert_eq!((from.start_byte, to.start_byte, *c), (10, 30, killed(3)));
    p
}

#[test]
fn imported_rows_preserve_independent_keys_empty_rows_and_non_edge_labels() {
    let p = adversarial_parts();
    let graph = test_support::parts::graph(p.clone());
    test_support::parts::assert_inputs(&p, &graph);
    test_support::parts::roundtrip(&graph);
    let new = &graph.storage;
    let key = p
        .adjacency
        .as_ref()
        .unwrap()
        .0
        .keys()
        .find(|v| v.path.base == "x")
        .unwrap();
    let mut unknown = key.clone();
    unknown.start_byte = 999;
    unknown.end_byte = 1000;
    assert!(new.arena.exact(&unknown).is_none());
    assert_eq!(new.successors(&unknown).unwrap().to_vec().len(), 3);
    assert!(new
        .successors(&loc("absent", VarAccessKind::Def, 0))
        .is_none());
    assert_eq!(new.labels().count(), 2);
    assert_eq!(
        new.forward()
            .map(|(_, v)| v.iter().count())
            .filter(|n| *n == 0)
            .count(),
        1
    );
}

#[test]
fn edge_derived_rows_preserve_sequence_and_distinct_spans() {
    let mut p = adversarial_parts();
    p.adjacency = None;
    let new = CompactDfg::from_parts(p.clone()).unwrap();
    test_support::parts::assert_inputs(
        &p,
        &DataFlowGraph {
            storage: new.clone(),
        },
    );
    test_support::parts::compact_roundtrip(&new);
    assert_eq!(new.forward().next().unwrap().0.start_byte, 20);
    assert_eq!(
        new.successors(&p.edges[0].from)
            .unwrap()
            .iter()
            .map(|v| v.start_byte)
            .collect::<Vec<_>>(),
        vec![40, 30, 40, 30]
    );
}
