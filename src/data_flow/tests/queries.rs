use super::super::test_support::{parts, *};
use super::super::*;
use super::identity::loc;

#[test]
fn views_include_spans_and_preserve_native_order() {
    let p = super::representatives::adversarial_parts();
    let g = parts::graph(p.clone());
    // Expected bytes come from the authored inputs, never from another engine.
    parts::assert_inputs(&p, &g);
    assert_eq!(g.edges().len(), 4);
    assert!(!g.edges().is_empty());
    parts::roundtrip(&g);
}

#[test]
fn views_preserve_independent_representatives_duplicates_empty_rows_and_absence() {
    let mut p = super::representatives::adversarial_parts();
    let e = p.edges[0].clone();
    let mut shifted = e.from.clone();
    shifted.start_byte += 100;
    shifted.end_byte += 100;
    p.adjacency = Some((
        BTreeMap::from([(shifted.clone(), vec![e.to.clone(), e.to.clone()])]),
        BTreeMap::from([(e.to.clone(), vec![shifted.clone(), e.from.clone()])]),
    ));
    let empty_key = ("empty.py".into(), "f".into(), 1, AccessPath::simple("z"));
    p.defs.insert(empty_key.clone(), vec![]);
    p.uses.insert(empty_key, vec![]);
    let g = parts::graph(p.clone());
    parts::assert_inputs(&p, &g);
    let (forward, backward) = p.adjacency.as_ref().unwrap();
    assert_bytes(
        "successors",
        &forward[&e.from],
        &g.successors(&e.from).unwrap().to_vec(),
    );
    assert_bytes(
        "predecessors",
        &backward[&e.to],
        &g.predecessors(&e.to).unwrap().to_vec(),
    );
    for ((f, t), c) in &p.labels {
        assert_eq!(g.label(f, t), Some(*c));
    }
    assert_eq!(
        g.defs()
            .find(|(k, _)| k.0 == "empty.py")
            .unwrap()
            .1
            .iter()
            .count(),
        0
    );
    assert_eq!(
        g.uses()
            .find(|(k, _)| k.0 == "empty.py")
            .unwrap()
            .1
            .iter()
            .count(),
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
    parts::roundtrip(&g);
    parts::roundtrip(&empty);
}

#[test]
fn queries_cover_cycles_spans_absence_and_same_line_propagation() {
    let mut x = loc("x", VarAccessKind::Def, 10);
    x.line = 1;
    let xu = loc("x", VarAccessKind::Use, 20);
    let y = loc("y", VarAccessKind::Def, 30);
    let mut yu = loc("y", VarAccessKind::Use, 40);
    yu.line = 3;
    let mut foreign = loc("z", VarAccessKind::Def, 50);
    foreign.function_start_line = 99;
    let key = |v: &VarLocation| {
        (
            v.file.clone(),
            v.function.clone(),
            v.function_start_line,
            v.path.clone(),
        )
    };
    let mut p = parts::empty();
    p.edges = vec![
        FlowEdge {
            from: x.clone(),
            to: xu.clone(),
        },
        FlowEdge {
            from: y.clone(),
            to: yu.clone(),
        },
    ];
    p.defs = BTreeMap::from([
        (key(&x), vec![x.clone()]),
        (key(&y), vec![y.clone()]),
        (key(&foreign), vec![foreign.clone()]),
    ]);
    p.uses = BTreeMap::from([(key(&xu), vec![xu.clone()]), (key(&yu), vec![yu.clone()])]);
    p.labels
        .insert((y.clone(), yu.clone()), FlowConfidence::Exact);
    let g = parts::graph(p.clone());
    assert_bytes(
        "assignment propagation",
        &BTreeSet::from([xu.clone(), y.clone(), yu.clone()]),
        &g.forward_reachable(&x),
    );
    assert_bytes(
        "backward excludes assignment propagation",
        &BTreeSet::from([y.clone()]),
        &g.backward_reachable(&yu),
    );
    assert_bytes(
        "definition lookup",
        &vec![x.clone()],
        &g.all_defs_of("u.py", "x"),
    );
    assert_bytes(
        "line order: definitions then uses",
        &vec![y.clone(), foreign, xu.clone()],
        &g.all_locations_at("u.py", 2),
    );
    assert_eq!(
        g.chop("u.py", 1, "u.py", 3),
        BTreeSet::from([("u.py".into(), 1), ("u.py".into(), 2), ("u.py".into(), 3)])
    );
    let expected = FlowPath {
        edges: vec![
            FlowEdge {
                from: x.clone(),
                to: xu.clone(),
            },
            FlowEdge {
                from: x.clone(),
                to: y.clone(),
            },
            FlowEdge {
                from: x.clone(),
                to: yu.clone(),
            },
        ],
        cleansed_for: BTreeSet::new(),
    };
    assert_eq!(
        path_bytes(&g.taint_forward(&[("u.py".into(), 1), ("u.py".into(), 1)])),
        path_bytes(&[expected.clone(), expected])
    );
    let absent_path = loc("absent", VarAccessKind::Use, 0);
    assert!(g.successors(&absent_path).is_none());
    assert_bytes(
        "unknown use still propagates to same-line assignments",
        &BTreeSet::from([y.clone(), yu.clone()]),
        &g.forward_reachable(&absent_path),
    );
    let absent_def = VarLocation {
        kind: VarAccessKind::Def,
        ..absent_path
    };
    assert!(g.forward_reachable(&absent_def).is_empty());
    for at in [
        VarLocation {
            file: "absent.py".into(),
            ..x.clone()
        },
        VarLocation {
            function: "absent".into(),
            ..x.clone()
        },
    ] {
        assert!(g.forward_reachable(&at).is_empty());
        assert!(g.backward_reachable(&at).is_empty());
    }
    assert!(g.all_locations_at("u.py", 0).is_empty());
    assert!(g.all_defs_of("absent.py", "x").is_empty());
    assert!(g.taint_forward(&[]).is_empty());
    assert!(g.taint_forward(&[("absent.py".into(), 1)]).is_empty());
    assert_eq!(
        g.chop("absent.py", 0, "u.py", 99),
        BTreeSet::from([("absent.py".into(), 0), ("u.py".into(), 99)])
    );
    parts::roundtrip(&g);
    // A duplicate edge and a cycle must neither re-add the root nor loop forever.
    p.edges.extend([
        p.edges[0].clone(),
        FlowEdge {
            from: xu.clone(),
            to: x.clone(),
        },
    ]);
    let cycle = parts::graph(p);
    assert_bytes(
        "cycle forward",
        &BTreeSet::from([xu.clone(), y, yu]),
        &cycle.forward_reachable(&x),
    );
    assert_bytes(
        "cycle backward",
        &BTreeSet::from([xu]),
        &cycle.backward_reachable(&x),
    );
    parts::roundtrip(&cycle);
}
