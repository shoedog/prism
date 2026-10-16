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

// Compact operation sequences compare exact storage and every public query after
// each intermediate state, including empty/unknown exclusions and held clones.
#[test]
fn overlap_merge_and_remove_match_legacy_bytes() {
    use super::representatives::adversarial_parts;
    use test_support::compact::{assert_all, pair};
    let (mut old, mut new) = pair(adversarial_parts());
    assert_all(&old, &new, &BTreeMap::new());
    let (rhs_old, rhs_new) = pair(adversarial_parts());
    old.merge(rhs_old);
    new.merge(rhs_new);
    assert_all(&old, &new, &BTreeMap::new());
    assert_eq!(new.rd_function_stats()["a.py"].functions_without_cfg, 1);
    old.remove_files(&BTreeSet::new());
    new.remove_files(&BTreeSet::new());
    assert_all(&old, &new, &BTreeMap::new());
    for excluded in [
        BTreeSet::from(["a.py".into()]),
        BTreeSet::from(["absent.py".into()]),
        BTreeSet::from(["b.py".into()]),
    ] {
        old.remove_files(&excluded);
        new.remove_files(&excluded);
        assert_all(&old, &new, &BTreeMap::new());
    }
    assert!(new.arena.rows.is_empty());
}

#[test]
fn removal_uses_map_key_file_and_either_edge_endpoint() {
    use super::representatives::adversarial_parts;
    use test_support::compact::{assert_all, pair};
    let (mut old, mut new) = pair(adversarial_parts());
    old.remove_files(&BTreeSet::from(["b.py".into()]));
    new.remove_files(&BTreeSet::from(["b.py".into()]));
    assert_all(&old, &new, &BTreeMap::new());
    assert_eq!(new.edges().len(), 3);
    assert!(new.defs().all(|(k, _)| k.0 == "a.py"));
    assert_eq!(new.labels().count(), 1);
    assert_eq!(new.exact_labels().count(), 0);
    assert!(new.arena.rows.iter().all(|v| v.file == "a.py"));
    // The inverse orientation must also be removed.
    let mut p = adversarial_parts();
    let mut e = p.edges.last().unwrap().clone();
    std::mem::swap(&mut e.from, &mut e.to);
    p.edges.push(e);
    let (mut old, mut new) = pair(p);
    old.remove_files(&BTreeSet::from(["b.py".into()]));
    new.remove_files(&BTreeSet::from(["b.py".into()]));
    assert_all(&old, &new, &BTreeMap::new());
    assert_eq!(new.edges().len(), 3);
}

#[test]
fn twenty_refreshes_reclaim_dead_locations_and_leave_clone_unchanged() {
    use test_support::compact::{assert_all, build, storage};
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
    let files = parse("retained.py");
    let mut old = oracle::LegacyDataFlowGraph::build(&files);
    let mut new = build(&files);
    let held = new.clone();
    let held_bytes = storage(&held);
    let initial = new.arena.rows.len();
    for generation in 0..20 {
        let file = format!("changed_{generation}.py");
        let files = parse(&file);
        old.merge(oracle::LegacyDataFlowGraph::build(&files));
        new.merge(build(&files));
        assert_all(&old, &new, &files);
        let excluded = BTreeSet::from([file]);
        old.remove_files(&excluded);
        new.remove_files(&excluded);
        assert_all(&old, &new, &files);
        assert_eq!(new.arena.rows.len(), initial);
        assert_eq!(storage(&held), held_bytes);
        assert!(new
            .arena
            .rows
            .iter()
            .all(|v| !v.file.starts_with("changed_")));
    }
}

#[test]
fn combine_overlapping_parts_keeps_exact_worst_and_stats_identity() {
    use super::representatives::adversarial_parts;
    use test_support::compact::{assert_all, pair};
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
    let (mut old, a) = pair(first);
    let (rhs, b) = pair(second);
    old.merge(rhs);
    let new = super::super::compact::combine(vec![a, b]).unwrap();
    assert_all(&old, &new, &BTreeMap::new());
    assert_eq!(
        new.exact_labels().next().unwrap().1,
        FlowConfidence::NameOnly(crate::cpg::FlowDoubt::Killed { kill_line: 3 })
    );
    assert_eq!(new.rd_function_stats()["a.py"].functions_without_cfg, 1);
    let empty = super::super::compact::combine(vec![]).unwrap();
    assert_all(
        &oracle::LegacyDataFlowGraph::empty(),
        &empty,
        &BTreeMap::new(),
    );
}

#[test]
fn empty_and_unknown_removal_rebuild_independent_rows() {
    use super::representatives::adversarial_parts;
    use test_support::compact::{assert_all, pair};
    for excluded in [BTreeSet::new(), BTreeSet::from(["absent.py".into()])] {
        let (mut old, mut new) = pair(adversarial_parts());
        let initial = new.arena.rows.len();
        old.remove_files(&excluded);
        new.remove_files(&excluded);
        assert_all(&old, &new, &BTreeMap::new());
        assert_eq!(new.forward().next().unwrap().0.start_byte, 20);
        assert!(
            new.arena.rows.len() < initial,
            "adjacency-only rows must be reclaimed"
        );
    }
}
