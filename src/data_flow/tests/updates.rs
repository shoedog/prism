use super::super::test_support::*;
use super::super::*;
#[test]
fn remove_merge_and_subset_preserve_primary_state() {
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
    let mut new = DataFlowGraph::build(&files);
    let held_new = new.clone();
    for exclude in [
        BTreeSet::new(),
        BTreeSet::from(["absent.py".into()]),
        BTreeSet::from(["a.py".into()]),
        BTreeSet::from(["b.py".into()]),
    ] {
        new.remove_files(&exclude);
        parts::roundtrip(&new);
        assert!(new
            .edges()
            .iter()
            .all(|e| !exclude.contains(&e.from.file) && !exclude.contains(&e.to.file)));
        new.merge(DataFlowGraph::build_subset(&files, &exclude));
        parts::roundtrip(&new);
    }
    // Removing and re-merging b.py last restores the cold build's a.py, b.py order.
    assert_eq!(
        current_storage(&new),
        current_storage(&DataFlowGraph::build(&files))
    );
    let before = edge_snapshot(&new);
    new.merge(held_new.clone());
    assert_eq!(new.edges().len(), before.len() + held_new.edges().len());
    assert_eq!(
        current_storage(&held_new),
        current_storage(&DataFlowGraph::build(&files))
    );
    parts::roundtrip(&new);
    parts::roundtrip(&held_new);
}

fn location(file: &str, line: usize, start: usize, kind: VarAccessKind) -> VarLocation {
    VarLocation {
        file: file.into(),
        function: "f".into(),
        function_start_line: 1,
        line,
        path: AccessPath::simple("x"),
        start_byte: start,
        end_byte: start + 1,
        kind,
    }
}
fn edge(from: &str, to: &str) -> FlowEdge {
    FlowEdge {
        from: location(from, 2, 10, VarAccessKind::Def),
        to: location(to, 3, 20, VarAccessKind::Use),
    }
}
// Synthetic authored primary inputs exercise production updates directly.
#[test]
fn removal_filters_exact_labels_at_either_endpoint_and_keeps_unrelated() {
    let mut new = parts::empty();
    for e in [
        edge("a.py", "b.py"),
        edge("b.py", "a.py"),
        edge("a.py", "a.py"),
    ] {
        new.exact_labels
            .insert(ExactFlowEdge::from_legacy(&e), FlowConfidence::Exact);
    }
    let mut new = parts::graph(new);
    for excluded in ["absent.py", "b.py"] {
        let exclude = BTreeSet::from([excluded.into()]);
        new.remove_files(&exclude);
        parts::roundtrip(&new);
    }
    assert_eq!(new.exact_labels().count(), 1);
    assert_eq!(new.exact_labels().next().unwrap().0.to.file, "a.py");
}
#[test]
fn removal_filters_cross_file_edges_and_rebuilds_retained_adjacency() {
    let mut new = parts::empty();
    new.edges = vec![
        edge("a.py", "b.py"),
        edge("b.py", "a.py"),
        edge("a.py", "a.py"),
    ];
    let mut new = parts::graph(new);
    new.remove_files(&BTreeSet::new());
    let unrelated = BTreeSet::from(["c.py".into()]);
    new.remove_files(&unrelated);
    parts::roundtrip(&new);
    assert!(
        edge_snapshot(&new).contains(&edge("a.py", "b.py")),
        "unrelated removal must retain cross-file edges"
    );
    let exclude = BTreeSet::from(["b.py".into()]);
    new.remove_files(&exclude);
    parts::roundtrip(&new);
    assert_eq!(edge_snapshot(&new), vec![edge("a.py", "a.py")]);
    assert_eq!(new.forward().next().unwrap().1.iter().count(), 1);
    assert_eq!(new.backward().next().unwrap().1.iter().count(), 1);
}
#[test]
fn overlapping_merge_keeps_killed_three_and_first_spans() {
    use crate::cpg::FlowDoubt;
    for reverse in [false, true] {
        let mut new = parts::empty();
        let mut incoming = parts::empty();
        let first = edge("a.py", "a.py");
        let mut later = first.clone();
        later.from.start_byte += 100;
        later.from.end_byte += 100;
        later.to.start_byte += 100;
        later.to.end_byte += 100;
        let killed = |kill_line| FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line });
        new.labels.insert(
            (first.from.clone(), first.to.clone()),
            killed(if reverse { 7 } else { 3 }),
        );
        incoming
            .labels
            .insert((later.from, later.to), killed(if reverse { 3 } else { 7 }));
        let exact = ExactFlowEdge::from_legacy(&first);
        new.exact_labels
            .insert(exact.clone(), killed(if reverse { 7 } else { 3 }));
        incoming
            .exact_labels
            .insert(exact.clone(), killed(if reverse { 3 } else { 7 }));
        let mut new = parts::graph(new);
        let incoming = parts::graph(incoming);
        new.merge(incoming);
        parts::roundtrip(&new);
        let (row, label) = new.labels().next().unwrap();
        let (from, to) = (row.from, row.to);
        assert_eq!((label, from.start_byte, to.start_byte), (killed(3), 10, 20));
        assert_eq!(
            new.exact_labels().find(|(e, _)| e == &exact).unwrap().1,
            killed(3)
        );
    }
}
#[test]
fn javascript_exact_labels_survive_partial_edit_retain_delete() {
    crate::build_pool::install(|| {
        let mut files = BTreeMap::new();
        let source = "function outer(value){sink(value);return sink(value);}\n";
        for path in ["a.js", "b.js"] {
            files.insert(
                path.into(),
                ParsedFile::parse(path, source, crate::languages::Language::JavaScript).unwrap(),
            );
        }
        let mut new = DataFlowGraph::build(&files);
        assert!(
            new.exact_labels().next().is_some(),
            "fixture must exercise supplemental exact labels"
        );
        for (path, replacement) in [
            (
                "a.js",
                Some("// shifted spans\nfunction outer(value){sink(value);return sink(value + 1);}\n"),
            ),
            ("b.js", None),
        ] {
            match replacement {
                Some(src) => {
                    files.insert(
                        path.into(),
                        ParsedFile::parse(path, src, crate::languages::Language::JavaScript)
                            .unwrap(),
                    );
                }
                None => {
                    files.remove(path);
                }
            }
            let changed = BTreeSet::from([path.into()]);
            new.remove_files(&changed);
            new.merge(DataFlowGraph::build_subset(&files, &changed));
            parts::roundtrip(&new);
            let cold = DataFlowGraph::build(&files);
            assert_bytes("incremental exact label bytes", &exact_label_snapshot(&new), &exact_label_snapshot(&cold));
            assert!(new.exact_labels().all(|(e, _)| files.contains_key(&e.from.file) && files.contains_key(&e.to.file)));
        }
    });
}

#[test]
fn labels_merge_duplicate_keys_with_the_worst_confidence() {
    use crate::access_path::AccessPath;
    use crate::cpg::{FlowConfidence, FlowDoubt};
    use crate::data_flow::{FlowEdge, VarAccessKind};

    let from = VarLocation {
        file: "a.py".into(),
        function: "f".into(),
        function_start_line: 1,
        line: 2,
        path: AccessPath::simple("x"),
        start_byte: 10,
        end_byte: 11,
        kind: VarAccessKind::Def,
    };
    let to = VarLocation {
        line: 3,
        start_byte: 20,
        end_byte: 21,
        kind: VarAccessKind::Use,
        ..from.clone()
    };
    let key = (from.clone(), to.clone());
    let edge = FlowEdge { from, to };
    let mut retained = parts::empty();
    retained.edges.push(edge.clone());
    retained.labels.insert(key.clone(), FlowConfidence::Exact);
    let mut fresh = parts::empty();
    fresh.edges.push(edge);
    fresh.labels.insert(
        key.clone(),
        FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
    );

    let mut retained = parts::graph(retained);
    retained.merge(parts::graph(fresh));
    assert_eq!(
        retained.label(&key.0, &key.1).unwrap(),
        FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)
    );
}

// Explicit representatives and binding sequences, plus compact codec invariants.
#[test]
fn overlap_merge_and_remove_match_legacy_bytes() {
    use super::representatives::adversarial_parts;
    let mut new = parts::graph(adversarial_parts());
    new.merge(parts::graph(adversarial_parts()));
    let binding = ("a.py".into(), "f".into(), 1, AccessPath::simple("x"));
    let starts = |g: &DataFlowGraph| {
        g.defs()
            .find(|(k, _)| **k == binding)
            .unwrap()
            .1
            .iter()
            .map(|v| v.start_byte)
            .collect::<Vec<_>>()
    };
    assert_eq!(starts(&new), [20, 10, 20, 10]);
    assert_eq!(new.edges().len(), 8);
    assert_eq!(new.rd_function_stats()["a.py"].functions_without_cfg, 1);
    parts::roundtrip(&new);
    new.remove_files(&BTreeSet::new());
    assert_eq!(starts(&new), [20, 10, 20, 10]);
    parts::roundtrip(&new);
    for excluded in [
        BTreeSet::from(["a.py".into()]),
        BTreeSet::from(["absent.py".into()]),
        BTreeSet::from(["b.py".into()]),
    ] {
        new.remove_files(&excluded);
        parts::roundtrip(&new);
    }
    assert!(new.storage.arena.rows().is_empty());
}

#[test]
fn removal_uses_map_key_file_and_either_edge_endpoint() {
    use super::representatives::adversarial_parts;
    for reverse in [false, true] {
        let mut p = adversarial_parts();
        if reverse {
            let mut e = p.edges.last().unwrap().clone();
            std::mem::swap(&mut e.from, &mut e.to);
            p.edges.push(e);
        }
        let expected = p.edges[..3].to_vec();
        let mut new = parts::graph(p);
        new.remove_files(&BTreeSet::from(["b.py".into()]));
        assert_bytes("retained edges", &expected, &edge_snapshot(&new));
        assert!(new.defs().all(|(k, _)| k.0 == "a.py"));
        assert_eq!(new.labels().count(), 1);
        assert_eq!(new.exact_labels().count(), 0);
        assert!(new.storage.arena.rows().iter().all(|v| v.file == "a.py"));
        parts::roundtrip(&new);
    }
}

#[test]
fn twenty_refreshes_reclaim_dead_locations_and_leave_clone_unchanged() {
    let parse = |file: &str| {
        BTreeMap::from([(
            file.into(),
            ParsedFile::parse(
                file,
                "def f(x):\n y=x\n return y\n",
                crate::languages::Language::Python,
            )
            .unwrap(),
        )])
    };
    let mut new = DataFlowGraph::build(&parse("retained.py"));
    let held = new.clone();
    let held_bytes = current_storage(&held);
    let initial = new.storage.arena.rows().len();
    for generation in 0..20 {
        let file = format!("changed_{generation}.py");
        new.merge(DataFlowGraph::build(&parse(&file)));
        parts::roundtrip(&new);
        new.remove_files(&BTreeSet::from([file]));
        parts::roundtrip(&new);
        assert_eq!(new.storage.arena.rows().len(), initial);
        assert_eq!(current_storage(&held), held_bytes);
        assert_eq!(current_storage(&new), held_bytes);
        assert!(new
            .storage
            .arena
            .rows()
            .iter()
            .all(|v| !v.file.starts_with("changed_")));
    }
}

#[test]
fn combine_overlapping_parts_keeps_exact_worst_and_stats_identity() {
    use super::super::compact::{combine, CompactDfg};
    use super::representatives::adversarial_parts;
    let mut first = adversarial_parts();
    let mut second = adversarial_parts();
    let key = first.exact_labels.keys().next().unwrap().clone();
    first.exact_labels.insert(
        key.clone(),
        FlowConfidence::NameOnly(crate::cpg::FlowDoubt::Killed { kill_line: 7 }),
    );
    second.exact_labels.insert(
        key,
        FlowConfidence::NameOnly(crate::cpg::FlowDoubt::Killed { kill_line: 3 }),
    );
    let new = combine(vec![
        CompactDfg::from_parts(first).unwrap(),
        CompactDfg::from_parts(second).unwrap(),
    ])
    .unwrap();
    assert_eq!(
        new.exact_labels().next().unwrap().1,
        FlowConfidence::NameOnly(crate::cpg::FlowDoubt::Killed { kill_line: 3 })
    );
    assert_eq!(new.rd_function_stats()["a.py"].functions_without_cfg, 1);
    parts::compact_roundtrip(&new);
    let empty = combine(vec![]).unwrap();
    assert_eq!(
        test_support::compact::storage(&empty),
        current_storage(&DataFlowGraph::empty())
    );
    parts::compact_roundtrip(&empty);
}

#[test]
fn empty_and_unknown_removal_rebuild_independent_rows() {
    use super::representatives::adversarial_parts;
    for excluded in [BTreeSet::new(), BTreeSet::from(["absent.py".into()])] {
        let mut new = parts::graph(adversarial_parts());
        let initial = new.storage.arena.rows().len();
        new.remove_files(&excluded);
        parts::roundtrip(&new);
        assert_eq!(new.forward().next().unwrap().0.start_byte, 20);
        assert!(
            new.storage.arena.rows().len() < initial,
            "adjacency-only rows must be reclaimed"
        );
    }
}
