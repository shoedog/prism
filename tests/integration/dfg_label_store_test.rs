//! Task 3 DFG label-store, graph occurrence, and cache lifecycle contracts.

use super::dfg_label_parity_test::corpus;
use prism::cpg::{CodePropertyGraph, CpgEdge, FlowConfidence, FlowDoubt};
use prism::cpg_cache::{self, CacheResult};
use prism::data_flow::{VarAccessKind, VarLocation};
use prism::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

fn graph_dataflow_payloads(
    cpg: &CodePropertyGraph,
) -> BTreeMap<(VarLocation, VarLocation), FlowConfidence> {
    let mut payloads = BTreeMap::new();
    for edge_index in cpg.graph.edge_indices() {
        let CpgEdge::DataFlow(label) = cpg.graph[edge_index] else {
            continue;
        };
        let Some((from_index, to_index)) = cpg.graph.edge_endpoints(edge_index) else {
            continue;
        };
        let (Some(from), Some(to)) = (
            cpg.to_var_location(from_index),
            cpg.to_var_location(to_index),
        ) else {
            continue;
        };
        payloads
            .entry((from, to))
            .and_modify(|stored: &mut FlowConfidence| *stored = stored.worst(label))
            .or_insert(label);
    }
    payloads
}

fn graph_dataflow_occurrences(
    cpg: &CodePropertyGraph,
) -> Vec<(VarLocation, VarLocation, FlowConfidence)> {
    cpg.graph
        .edge_indices()
        .filter_map(|edge_index| {
            let CpgEdge::DataFlow(label) = cpg.graph[edge_index] else {
                return None;
            };
            let (from_index, to_index) = cpg.graph.edge_endpoints(edge_index)?;
            Some((
                cpg.to_var_location(from_index)?,
                cpg.to_var_location(to_index)?,
                label,
            ))
        })
        .collect()
}

fn expected_step5b_labels(
    cpg: &CodePropertyGraph,
    from: &VarLocation,
    to: &VarLocation,
) -> BTreeSet<FlowConfidence> {
    let mut labels = BTreeSet::new();
    for (caller, sites) in &cpg.call_graph.calls {
        if (
            caller.file.as_str(),
            caller.name.as_str(),
            caller.start_line,
        ) != (
            from.file.as_str(),
            from.function.as_str(),
            from.function_start_line,
        ) {
            continue;
        }
        for site in sites {
            for resolved in cpg.call_graph.resolve_call_site(site) {
                if (
                    resolved.target.file.as_str(),
                    resolved.target.name.as_str(),
                    resolved.target.start_line,
                ) == (
                    to.file.as_str(),
                    to.function.as_str(),
                    to.function_start_line,
                ) {
                    labels.insert(FlowConfidence::from(resolved.confidence));
                }
            }
        }
    }
    labels
}

fn assert_graph_dataflow_payloads_match_store(cpg: &CodePropertyGraph) {
    for (from, to, payload) in graph_dataflow_occurrences(cpg) {
        if let Some(label) = cpg.dfg.labels.get(&(from.clone(), to.clone())) {
            assert_eq!(
                payload, *label,
                "Step-4 payload diverged from dfg.labels for {from:?} -> {to:?}"
            );
            continue;
        }

        assert_eq!(from.kind, VarAccessKind::Use);
        assert_eq!(to.kind, VarAccessKind::Def);
        assert_ne!(
            (
                from.file.as_str(),
                from.function.as_str(),
                from.function_start_line
            ),
            (
                to.file.as_str(),
                to.function.as_str(),
                to.function_start_line
            ),
            "a same-function DataFlow payload is missing from dfg.labels"
        );
        let expected = expected_step5b_labels(cpg, &from, &to);
        assert!(
            expected.contains(&payload),
            "Step-5b payload {payload:?} has no matching resolved-callee floor for {from:?} -> {to:?}; candidates={expected:?}"
        );
    }
}

#[test]
fn every_graph_dataflow_payload_equals_its_dfg_label() {
    let repo = corpus();
    let cpg = CodePropertyGraph::build(&repo.files);
    let payloads = graph_dataflow_payloads(&cpg);
    assert!(
        payloads.len() > 50,
        "fixture must exercise real DataFlow edges"
    );
    assert_graph_dataflow_payloads_match_store(&cpg);
}

#[test]
fn collapsed_same_line_and_capture_groups_have_no_exact_occurrence() {
    let sources = BTreeMap::from([
        (
            "same.py".to_string(),
            "def same():\n    value = 1; value = 2\n    use(value)\n".to_string(),
        ),
        (
            "capture.py".to_string(),
            "def capture():\n    value = 1\n    delayed = lambda: use(value)\n    value = 2\n    return delayed\n".to_string(),
        ),
    ]);
    let files = parse_python_sources(&sources);
    let cpg = CodePropertyGraph::build(&files);
    for (file, expected) in [
        ("same.py", FlowConfidence::NameOnly(FlowDoubt::SameLine)),
        (
            "capture.py",
            FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
        ),
    ] {
        let keys: BTreeSet<_> = cpg
            .dfg
            .labels
            .iter()
            .filter_map(|(key, label)| {
                (key.0.file == file && *label == expected).then_some(key.clone())
            })
            .collect();
        assert!(
            !keys.is_empty(),
            "{file} fixture did not produce its conservative label"
        );
        let occurrences: Vec<_> = graph_dataflow_occurrences(&cpg)
            .into_iter()
            .filter(|(from, to, _)| keys.contains(&(from.clone(), to.clone())))
            .collect();
        assert!(
            !occurrences.is_empty(),
            "{file} conservative group had no assembled graph occurrence"
        );
        for (_, _, payload) in occurrences {
            assert_ne!(
                payload,
                FlowConfidence::Exact,
                "{file} conservative group contained an Exact occurrence"
            );
        }
    }
}

#[test]
fn a_mixed_graph_payload_cannot_hide_behind_the_worst_label() {
    let sources = BTreeMap::from([(
        "same.py".to_string(),
        "def f():\n    a = 1; a = 2\n    use(a)\n".to_string(),
    )]);
    let files = parse_python_sources(&sources);
    let mut cpg = CodePropertyGraph::build(&files);
    let key = cpg
        .dfg
        .labels
        .iter()
        .find_map(|(key, label)| {
            (*label == FlowConfidence::NameOnly(FlowDoubt::SameLine)).then_some(key.clone())
        })
        .expect("fixture must produce a collapsed same-line label");
    let (from_index, to_index) = cpg
        .graph
        .edge_indices()
        .find_map(|edge_index| {
            let (from_index, to_index) = cpg.graph.edge_endpoints(edge_index)?;
            let from = cpg.to_var_location(from_index)?;
            let to = cpg.to_var_location(to_index)?;
            ((from, to) == key).then_some((from_index, to_index))
        })
        .expect("fixture must assemble the labeled same-line edge");
    cpg.graph.add_edge(
        from_index,
        to_index,
        CpgEdge::DataFlow(FlowConfidence::Exact),
    );

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        assert_graph_dataflow_payloads_match_store(&cpg);
    }));
    assert!(
        result.is_err(),
        "an Exact occurrence beside the stored NameOnly label was hidden by worst()"
    );
}

fn parse_python_sources(
    sources: &BTreeMap<String, String>,
) -> BTreeMap<String, prism::ast::ParsedFile> {
    sources
        .iter()
        .map(|(path, source)| {
            (
                path.clone(),
                prism::ast::ParsedFile::parse(path, source, Language::Python).unwrap(),
            )
        })
        .collect()
}

fn assert_dfg_label_membership_complete(cpg: &CodePropertyGraph) {
    let edge_keys: BTreeSet<_> = cpg
        .dfg
        .edges
        .iter()
        .map(|edge| (edge.from.clone(), edge.to.clone()))
        .collect();
    assert_eq!(
        edge_keys,
        cpg.dfg.labels.keys().cloned().collect(),
        "an existing DFG edge is missing a primary label, or a label has no edge"
    );
}

fn local_labels_for_file(
    cpg: &CodePropertyGraph,
    file: &str,
) -> BTreeMap<(VarLocation, VarLocation), FlowConfidence> {
    cpg.dfg
        .labels
        .iter()
        .filter(|((from, to), _)| from.file == file && to.file == file)
        .map(|(key, label)| (key.clone(), *label))
        .collect()
}

fn cross_file_payloads(cpg: &CodePropertyGraph) -> Vec<(VarLocation, VarLocation, FlowConfidence)> {
    graph_dataflow_occurrences(cpg)
        .into_iter()
        .filter(|(from, to, _)| from.file != to.file)
        .collect()
}

#[test]
fn cache_cold_full_hit_and_partial_hit_agree_on_every_label() {
    let sources = BTreeMap::from([
        (
            "a.py".to_string(),
            "def target(value):\n    sink(value)\n".to_string(),
        ),
        (
            "b.py".to_string(),
            "from a import target\n\ndef b():\n    y = source()\n    target(y)\n".to_string(),
        ),
        ("c.py".to_string(), "def c(q):\n    return q\n".to_string()),
    ]);
    let files = parse_python_sources(&sources);
    let cold = CodePropertyGraph::build(&files);
    assert_dfg_label_membership_complete(&cold);
    let cold_labels = cold.dfg.labels.clone();
    let cold_rd_function_stats = cold.dfg.rd_function_stats.clone();
    let cold_payloads = graph_dataflow_payloads(&cold);
    assert!(cold_labels.values().any(|label| label.is_exact()));
    assert!(
        cold_labels.values().any(|label| !label.is_exact()),
        "the serialized cold fixture must include a NameOnly label"
    );
    let retained_a_labels = local_labels_for_file(&cold, "a.py");
    let retained_c_labels = local_labels_for_file(&cold, "c.py");
    assert!(!retained_a_labels.is_empty());
    assert!(!retained_c_labels.is_empty());
    let cold_step5b = cross_file_payloads(&cold);
    assert!(
        cold_step5b.iter().any(|(from, to, label)| {
            from.file == "b.py"
                && from.path.base == "y"
                && to.file == "a.py"
                && to.path.base == "value"
                && *label == FlowConfidence::Exact
        }),
        "cold fixture must contain the cross-file y argument-to-value parameter edge"
    );

    let cache_dir = tempfile::tempdir().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&cold, &hashes, false, cache_dir.path()).unwrap();
    let full_hit = match cpg_cache::load_cache(&hashes, false, cache_dir.path()) {
        CacheResult::Hit(cpg) => cpg,
        CacheResult::PartialHit { .. } => panic!("expected full cache hit, got partial hit"),
        CacheResult::Miss => panic!("expected full cache hit, got miss"),
    };
    assert_dfg_label_membership_complete(&full_hit);
    assert_eq!(full_hit.dfg.labels, cold_labels);
    assert_eq!(full_hit.dfg.rd_function_stats, cold_rd_function_stats);
    assert_eq!(graph_dataflow_payloads(&full_hit), cold_payloads);

    let mut edited_sources = sources.clone();
    edited_sources.insert(
        "b.py".to_string(),
        "from a import target\n\ndef b():\n    z = source()\n    z = clean()\n    target(z)\n"
            .to_string(),
    );
    let edited_files = parse_python_sources(&edited_sources);
    let edited_hashes = cpg_cache::compute_file_hashes(&edited_sources);
    let incremental = match cpg_cache::load_cache(&edited_hashes, false, cache_dir.path()) {
        CacheResult::PartialHit {
            cached_call_graph,
            cached_dfg,
            changed_files,
        } => {
            assert_eq!(changed_files, BTreeSet::from(["b.py".to_string()]));
            CodePropertyGraph::build_incremental(
                cached_call_graph,
                cached_dfg,
                &changed_files,
                &edited_files,
                None,
            )
        }
        CacheResult::Hit(_) => panic!("one-file edit unexpectedly produced a full hit"),
        CacheResult::Miss => panic!("one-file edit unexpectedly produced a miss"),
    };
    let edited_cold = CodePropertyGraph::build(&edited_files);
    assert_dfg_label_membership_complete(&incremental);
    assert_dfg_label_membership_complete(&edited_cold);
    assert_eq!(incremental.dfg.labels, edited_cold.dfg.labels);
    assert_eq!(
        incremental.dfg.rd_function_stats,
        edited_cold.dfg.rd_function_stats
    );
    assert_eq!(
        incremental.dfg.rd_function_stats["a.py"], cold_rd_function_stats["a.py"],
        "the retained file's RD counters changed during a one-file edit"
    );
    assert_eq!(
        local_labels_for_file(&incremental, "a.py"),
        retained_a_labels,
        "the retained a.py label partition changed during a one-file edit"
    );
    assert_eq!(
        local_labels_for_file(&incremental, "c.py"),
        retained_c_labels,
        "the retained c.py label partition changed during a one-file edit"
    );
    assert_eq!(
        graph_dataflow_payloads(&incremental),
        graph_dataflow_payloads(&edited_cold)
    );

    assert!(incremental
        .dfg
        .labels
        .keys()
        .any(|(from, _)| from.file == "a.py"));
    assert!(!incremental
        .dfg
        .labels
        .keys()
        .any(|(from, to)| from.file == "b.py" && (from.path.base == "y" || to.path.base == "y")));
    assert!(incremental
        .dfg
        .labels
        .keys()
        .any(|(from, to)| from.file == "b.py" && (from.path.base == "z" || to.path.base == "z")));
    let incremental_step5b = cross_file_payloads(&incremental);
    assert!(!incremental_step5b.iter().any(|(from, to, _)| {
        from.file == "b.py" && from.path.base == "y" && to.file == "a.py" && to.path.base == "value"
    }));
    assert!(
        incremental_step5b.iter().any(|(from, to, label)| {
            from.file == "b.py"
                && from.path.base == "z"
                && to.file == "a.py"
                && to.path.base == "value"
                && *label == FlowConfidence::Exact
        }),
        "PartialHit must re-derive the edited z argument-to-value parameter edge"
    );
}

#[test]
fn repeated_partial_hits_do_not_accumulate_retained_missing_label_stats() {
    let sources = BTreeMap::from([
        (
            "a.py".to_string(),
            "def retained(value):\n    return value\n".to_string(),
        ),
        (
            "b.py".to_string(),
            "def edited():\n    value = source()\n    sink(value)\n".to_string(),
        ),
    ]);
    let files = parse_python_sources(&sources);
    let mut cached = CodePropertyGraph::build(&files);
    let cold_stats = cached.dfg.rd_function_stats["a.py"];
    assert_eq!(
        cold_stats.functions_without_cfg, 1,
        "the retained fixture must begin with exactly one unavailable function"
    );
    let retained_key = cached
        .dfg
        .labels
        .keys()
        .find(|(from, to)| from.file == "a.py" && to.file == "a.py")
        .cloned()
        .expect("the retained function must have a DFG label to remove");
    assert!(cached.dfg.labels.remove(&retained_key).is_some());

    let cache_dir = tempfile::tempdir().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&cached, &hashes, false, cache_dir.path()).unwrap();

    let mut first_sources = sources.clone();
    first_sources.insert(
        "b.py".to_string(),
        "def edited():\n    first = source()\n    sink(first)\n".to_string(),
    );
    let first_hashes = cpg_cache::compute_file_hashes(&first_sources);
    let first_files = parse_python_sources(&first_sources);
    let first = match cpg_cache::load_cache(&first_hashes, false, cache_dir.path()) {
        CacheResult::PartialHit {
            cached_call_graph,
            cached_dfg,
            changed_files,
        } => {
            assert_eq!(changed_files, BTreeSet::from(["b.py".to_string()]));
            CodePropertyGraph::build_incremental(
                cached_call_graph,
                cached_dfg,
                &changed_files,
                &first_files,
                None,
            )
        }
        CacheResult::Hit(_) => panic!("first edit unexpectedly produced a full hit"),
        CacheResult::Miss => panic!("first edit unexpectedly produced a miss"),
    };
    let first_stats = first.dfg.rd_function_stats["a.py"];
    assert!(!first.dfg.labels.contains_key(&retained_key));
    cpg_cache::save_cache(&first, &first_hashes, false, cache_dir.path()).unwrap();

    let mut second_sources = first_sources.clone();
    second_sources.insert(
        "b.py".to_string(),
        "def edited():\n    second = source()\n    sink(second)\n".to_string(),
    );
    let second_hashes = cpg_cache::compute_file_hashes(&second_sources);
    let second_files = parse_python_sources(&second_sources);
    let second = match cpg_cache::load_cache(&second_hashes, false, cache_dir.path()) {
        CacheResult::PartialHit {
            cached_call_graph,
            cached_dfg,
            changed_files,
        } => {
            assert_eq!(changed_files, BTreeSet::from(["b.py".to_string()]));
            CodePropertyGraph::build_incremental(
                cached_call_graph,
                cached_dfg,
                &changed_files,
                &second_files,
                None,
            )
        }
        CacheResult::Hit(_) => panic!("second edit unexpectedly produced a full hit"),
        CacheResult::Miss => panic!("second edit unexpectedly produced a miss"),
    };
    let second_stats = second.dfg.rd_function_stats["a.py"];
    assert!(!second.dfg.labels.contains_key(&retained_key));

    assert_eq!(
        first_stats, cold_stats,
        "the first PartialHit accumulated the retained fallback function"
    );
    assert_eq!(
        second_stats, cold_stats,
        "the second PartialHit accumulated the retained fallback function again"
    );
}
