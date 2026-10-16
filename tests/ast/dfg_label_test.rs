//! Task 3 primary DFG label-store contracts.

use crate::common::dfg_labels;
use crate::common::*;
use prism::data_flow::VarLocation;

fn label_keys(dfg: &DataFlowGraph) -> BTreeSet<(VarLocation, VarLocation)> {
    dfg_labels(&dfg).keys().cloned().collect()
}

fn unique_edge_keys(dfg: &DataFlowGraph) -> BTreeSet<(VarLocation, VarLocation)> {
    dfg.edges()
        .iter()
        .map(|edge| (edge.from.clone(), edge.to.clone()))
        .collect()
}

fn parsed_python_files(sources: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
    sources
        .iter()
        .map(|(path, source)| {
            (
                (*path).to_string(),
                ParsedFile::parse(path, source, Language::Python).unwrap(),
            )
        })
        .collect()
}

#[test]
fn labels_have_exactly_one_entry_per_unique_edge() {
    let files = parsed_python_files(&[(
        "a.py",
        "def f():\n    x = source()\n    x = clean()\n    sink(x)\n",
    )]);
    let dfg = DataFlowGraph::build(&files);

    assert!(!dfg.edges().is_empty(), "fixture must construct DFG edges");
    assert_eq!(label_keys(&dfg), unique_edge_keys(&dfg));
    assert_eq!(dfg_labels(&dfg).len(), unique_edge_keys(&dfg).len());
}

#[test]
fn labels_carry_reaching_definition_results_for_existing_edges() {
    use prism::cpg::{FlowConfidence, FlowDoubt};

    let files = parsed_python_files(&[(
        "a.py",
        "def f():\n    x = source()\n    x = clean()\n    sink(x)\n",
    )]);
    let dfg = DataFlowGraph::build(&files);
    let label = |from_line, to_line| {
        dfg_labels(&dfg)
            .iter()
            .find_map(|((from, to), label)| {
                (from.path.base == "x"
                    && to.path.base == "x"
                    && from.line == from_line
                    && to.line == to_line)
                    .then_some(*label)
            })
            .unwrap_or_else(|| panic!("missing x edge {from_line}->{to_line}"))
    };

    assert_eq!(
        label(2, 4),
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 })
    );
    assert_eq!(label(3, 4), FlowConfidence::Exact);
}

#[test]
fn labels_use_cfg_incomplete_when_rd_has_no_cfg_edges() {
    use prism::cpg::{FlowConfidence, FlowDoubt};

    let files = parsed_python_files(&[("a.py", "def f(x):\n    return x\n")]);
    let dfg = DataFlowGraph::build(&files);
    assert_eq!(
        dfg_labels(&dfg).len(),
        1,
        "fixture must construct one param edge"
    );
    assert_eq!(
        dfg_labels(&dfg).values().copied().next(),
        Some(FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete))
    );
    assert_eq!(dfg.rd_function_stats()["a.py"].functions_without_cfg, 1);
}

#[test]
fn labels_survive_remove_files_and_merge_with_the_same_membership() {
    let files = parsed_python_files(&[
        ("a.py", "def a():\n    x = source()\n    sink(x)\n"),
        ("b.py", "def b():\n    y = source()\n    sink(y)\n"),
    ]);
    let mut dfg = DataFlowGraph::build(&files);
    assert_eq!(label_keys(&dfg), unique_edge_keys(&dfg));

    let excluded = BTreeSet::from(["b.py".to_string()]);
    dfg.remove_files(&excluded);
    assert_eq!(label_keys(&dfg), unique_edge_keys(&dfg));
    assert!(dfg_labels(&dfg)
        .keys()
        .all(|(from, to)| { from.file.as_str() == "a.py" && to.file.as_str() == "a.py" }));

    let fresh = DataFlowGraph::build_subset(&files, &excluded);
    assert_eq!(label_keys(&fresh), unique_edge_keys(&fresh));
    assert!(dfg_labels(&fresh)
        .keys()
        .all(|(from, to)| { from.file.as_str() == "b.py" && to.file.as_str() == "b.py" }));
    dfg.merge(fresh);
    assert_eq!(label_keys(&dfg), unique_edge_keys(&dfg));
    assert!(dfg_labels(&dfg).keys().any(|(from, _)| from.file == "a.py"));
    assert!(dfg_labels(&dfg).keys().any(|(from, _)| from.file == "b.py"));
}

#[test]
fn a_synthetic_flow_edge_lookup_is_none_not_a_default() {
    use prism::access_path::AccessPath;
    use prism::data_flow::{FlowEdge, VarAccessKind};

    let files = parsed_python_files(&[("a.py", "def f():\n    x = source()\n    sink(x)\n")]);
    let dfg = DataFlowGraph::build(&files);
    let synthetic = FlowEdge {
        from: VarLocation {
            file: "synthetic.py".into(),
            function: "f".into(),
            function_start_line: 1,
            line: 2,
            path: AccessPath::simple("x"),
            start_byte: 0,
            end_byte: 0,
            kind: VarAccessKind::Def,
        },
        to: VarLocation {
            file: "synthetic.py".into(),
            function: "f".into(),
            function_start_line: 1,
            line: 3,
            path: AccessPath::simple("x"),
            start_byte: 0,
            end_byte: 0,
            kind: VarAccessKind::Use,
        },
    };

    assert_eq!(
        dfg_labels(&dfg).get(&(synthetic.from, synthetic.to)),
        None,
        "a synthetic edge must not acquire a fabricated default label"
    );
}

#[test]
fn rd_function_stats_are_file_partitioned() {
    let files = parsed_python_files(&[
        ("a.py", "def a():\n    x = 1\n"),
        ("b.py", "def b():\n    y = 1\n"),
    ]);
    let mut dfg = DataFlowGraph::build(&files);
    let retained = dfg.rd_function_stats()["a.py"].clone();
    assert!(retained.functions_without_cfg > 0);
    assert!(dfg.rd_function_stats()["b.py"].functions_without_cfg > 0);

    let excluded = BTreeSet::from(["b.py".to_string()]);
    dfg.remove_files(&excluded);
    assert_eq!(dfg.rd_function_stats().len(), 1);
    assert_eq!(dfg.rd_function_stats()["a.py"], retained);

    dfg.merge(DataFlowGraph::build_subset(&files, &excluded));
    assert_eq!(dfg.rd_function_stats().len(), 2);
    assert_eq!(dfg.rd_function_stats()["a.py"], retained);
    assert!(dfg.rd_function_stats()["b.py"].functions_without_cfg > 0);
}

#[test]
fn rd_function_stats_merge_unions_function_identities() {
    let first = parsed_python_files(&[("a.py", "def u(value):\n    return value\n")]);
    let second = parsed_python_files(&[("a.py", "def v(value):\n    return value\n")]);
    let mut retained = DataFlowGraph::build(&first);
    let fresh = DataFlowGraph::build(&second);
    assert_eq!(
        retained.rd_function_stats()["a.py"].functions_without_cfg,
        1
    );
    assert_eq!(fresh.rd_function_stats()["a.py"].functions_without_cfg, 1);

    retained.merge(fresh);
    assert_eq!(
        retained.rd_function_stats()["a.py"].functions_without_cfg,
        2,
        "same-file merge must union distinct unavailable-function identities"
    );
}
