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
// Synthetic state has no producer: explicitly feed the same primary inputs to
// both engines. Query/update behavior, not construction, is under test here.
fn frozen(g: &DataFlowGraph) -> oracle::LegacyDataFlowGraph {
    oracle::LegacyDataFlowGraph {
        edges: g.edges.clone(),
        defs: g.defs.clone(),
        uses: g.uses.clone(),
        forward: g.forward.clone(),
        backward: g.backward.clone(),
        labels: g.labels.clone(),
        exact_labels: g.exact_labels.clone(),
        rd_function_stats: g.rd_function_stats.clone(),
    }
}
#[test]
fn removal_filters_exact_labels_at_either_endpoint_and_keeps_unrelated() {
    let mut new = DataFlowGraph::empty();
    for e in [
        edge("a.py", "b.py"),
        edge("b.py", "a.py"),
        edge("a.py", "a.py"),
    ] {
        new.exact_labels
            .insert(ExactFlowEdge::from_legacy(&e), FlowConfidence::Exact);
    }
    let mut old = frozen(&new);
    for excluded in ["absent.py", "b.py"] {
        let exclude = BTreeSet::from([excluded.into()]);
        old.remove_files(&exclude);
        new.remove_files(&exclude);
        assert_pair(&old, &new, &BTreeMap::new());
    }
    assert_eq!(new.exact_labels.len(), 1);
    assert_eq!(new.exact_labels.keys().next().unwrap().to.file, "a.py");
}
#[test]
fn removal_filters_cross_file_edges_and_rebuilds_retained_adjacency() {
    let mut new = DataFlowGraph::empty();
    new.edges = vec![
        edge("a.py", "b.py"),
        edge("b.py", "a.py"),
        edge("a.py", "a.py"),
    ];
    new.rebuild_adjacency();
    let mut old = frozen(&new);
    let unrelated = BTreeSet::from(["c.py".into()]);
    old.remove_files(&unrelated);
    new.remove_files(&unrelated);
    assert_pair(&old, &new, &BTreeMap::new());
    assert!(
        new.edges.contains(&edge("a.py", "b.py")),
        "unrelated removal must retain cross-file edges"
    );
    let exclude = BTreeSet::from(["b.py".into()]);
    old.remove_files(&exclude);
    new.remove_files(&exclude);
    assert_pair(&old, &new, &BTreeMap::new());
    assert_eq!(new.edges, vec![edge("a.py", "a.py")]);
    assert_eq!(new.forward.values().next().unwrap().len(), 1);
    assert_eq!(new.backward.values().next().unwrap().len(), 1);
}
#[test]
fn overlapping_merge_keeps_killed_three_and_first_spans() {
    use crate::cpg::FlowDoubt;
    for reverse in [false, true] {
        let mut new = DataFlowGraph::empty();
        let mut incoming = DataFlowGraph::empty();
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
        let mut old = frozen(&new);
        old.merge(frozen(&incoming));
        new.merge(incoming);
        assert_pair(&old, &new, &BTreeMap::new());
        let ((from, to), label) = new.labels.first_key_value().unwrap();
        assert_eq!(
            (*label, from.start_byte, to.start_byte),
            (killed(3), 10, 20)
        );
        assert_eq!(new.exact_labels[&exact], killed(3));
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
        let mut old = oracle::LegacyDataFlowGraph::build(&files);
        let mut new = DataFlowGraph::build(&files);
        assert!(
            !new.exact_labels.is_empty(),
            "fixture must exercise supplemental exact labels"
        );
        for (path, replacement) in [
            (
                "a.js",
                Some("// shifted spans\nfunction outer(value){sink(value);return sink(value + 1);}\n"),
            ),
            ("b.js", None),
        ] {
            eprintln!("DFG_UNIT_REFRESH {path} edit={}", replacement.is_some());
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
            old.remove_files(&changed);
            new.remove_files(&changed);
            old.merge(oracle::LegacyDataFlowGraph::build_subset(&files, &changed));
            new.merge(DataFlowGraph::build_subset(&files, &changed));
            assert_pair(&old, &new, &files);
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
    let mut retained = DataFlowGraph::empty();
    retained.edges.push(edge.clone());
    retained.labels.insert(key.clone(), FlowConfidence::Exact);
    let mut fresh = DataFlowGraph::empty();
    fresh.edges.push(edge);
    fresh.labels.insert(
        key.clone(),
        FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
    );

    retained.merge(fresh);
    assert_eq!(
        retained.labels[&key],
        FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)
    );
}
