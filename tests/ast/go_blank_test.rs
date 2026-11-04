//! Go's blank name has no value; all remaining occurrences retain exact spans and labels.
use crate::common::*;
use prism::data_flow::VarLocation;
use serde_json::{json, Value};

fn cases() -> Vec<Value> {
    serde_json::from_str(include_str!("../fixtures/go_blank/cases.json")).unwrap()
}

fn graph(case: &Value) -> DataFlowGraph {
    let language = match case["language"].as_str().unwrap() {
        "Go" => Language::Go,
        "Python" => Language::Python,
        "JavaScript" => Language::JavaScript,
        _ => unreachable!(),
    };
    let file = case["file"].as_str().unwrap();
    let source = case["source"].as_str().unwrap();
    let parsed = ParsedFile::parse(file, source, language).unwrap();
    DataFlowGraph::build(&BTreeMap::from([(file.to_owned(), parsed)]))
}

fn snapshot(dfg: &DataFlowGraph) -> Value {
    let locations = |defs: bool| -> Vec<VarLocation> {
        if defs {
            dfg.defs().flat_map(|(_, v)| v.to_vec()).collect()
        } else {
            dfg.uses().flat_map(|(_, v)| v.to_vec()).collect()
        }
    };
    json!({
        "defs": locations(true),
        "uses": locations(false),
        "edges": dfg.edges().iter().map(|e| e.to_owned()).collect::<Vec<_>>(),
        "labels": dfg.labels().map(|(e, label)| (e.to_owned(), label)).collect::<Vec<_>>(),
        "stats": dfg.rd_function_stats(),
    })
}

fn nonblank(snapshot: &Value) -> Value {
    let mut out = snapshot.clone();
    for field in ["defs", "uses", "edges", "labels"] {
        out[field].as_array_mut().unwrap().retain(|row| {
            let blank = |loc: &Value| loc["path"]["base"] == "_";
            match field {
                "defs" | "uses" => !blank(row),
                "edges" => !blank(&row["from"]) && !blank(&row["to"]),
                "labels" => !blank(&row[0]["from"]) && !blank(&row[0]["to"]),
                _ => unreachable!(),
            }
        });
    }
    // Prune telemetry counts producer candidates; removing blank candidates may lower it.
    for stats in out["stats"].as_object_mut().unwrap().values_mut() {
        for field in [
            "dfg_pruned_write_only",
            "dfg_pruned_scope",
            "dfg_pruned_dominated",
            "dfg_pruned_order",
        ] {
            stats.as_object_mut().unwrap().remove(field);
        }
    }
    out
}

fn check(name: &str) {
    let all = cases();
    let case = all.iter().find(|c| c["name"] == name).unwrap();
    let dfg = graph(case);
    let actual = snapshot(&dfg);
    let baseline: Value =
        serde_json::from_str(include_str!("../fixtures/go_blank/base.json")).unwrap();
    if case["language"] == "Go" {
        // Check every collection, including labels and both endpoints; compare bytes because
        // VarLocation Eq deliberately ignores start/end byte positions.
        let filtered = nonblank(&actual);
        let mut failures = Vec::new();
        for field in ["defs", "uses", "edges", "labels"] {
            if actual[field] != filtered[field] {
                failures.push(format!("blank {field}"));
            }
        }
        if filtered != nonblank(&baseline[name]) {
            failures.push("nonblank population changed".to_owned());
        }
        assert!(failures.is_empty(), "{name}: {}", failures.join(", "));
    } else {
        assert_eq!(actual, baseline[name], "{name}: language control changed");
        let encoded: Vec<u8> =
            serde_json::from_value(baseline[format!("{name}_bytes")].clone()).unwrap();
        assert_eq!(
            bincode::serialize(&dfg).unwrap(),
            encoded,
            "{name}: serialized graph changed"
        );
        assert!(!dfg
            .all_defs_of(case["file"].as_str().unwrap(), "_")
            .is_empty());
        assert!(dfg.edges().iter().any(|e| e.from.path.base == "_"));
    }
}

macro_rules! fixtures {
    ($($name:ident),* $(,)?) => {$(#[test] fn $name() { check(stringify!($name)); })*};
}
fixtures!(
    discard_call,
    short_declaration,
    multi_assignment,
    range_value,
    range_assignment,
    range_map,
    range_both_assignment,
    select_assignment,
    select_declaration,
    var_inferred,
    var_typed,
    var_grouped,
    const_iota,
    package_assertion,
    parameter,
    grouped_parameter,
    variadic_parameter,
    receiver,
    pointer_receiver,
    named_results,
    multi_swap,
    literal_parameter,
    literal_body,
    field_rhs,
    plain_rhs,
    if_initializer,
    switch_initializer,
    loop_init_post,
    comma_ok,
    type_assertion,
    named_controls,
    blank_field_control,
    blank_import_control,
    invalid_blank_read,
    invalid_blank_field,
    invalid_blank_update,
    python_underscore,
    javascript_underscore,
);

#[test]
fn cpg_dataflow_endpoints() {
    use petgraph::visit::EdgeRef;
    use prism::cpg::{CpgEdge, CpgNode};
    let mut failures = Vec::new();
    for case in cases().iter().filter(|c| c["language"] == "Go") {
        let cpg = build_cpg(
            case["file"].as_str().unwrap(),
            case["source"].as_str().unwrap(),
            Language::Go,
        );
        for edge in cpg
            .graph
            .edge_references()
            .filter(|e| matches!(e.weight(), CpgEdge::DataFlow(_)))
        {
            for node in [edge.source(), edge.target()] {
                if matches!(cpg.node(node), CpgNode::Variable { path, .. } if path.base == "_") {
                    failures.push(case["name"].as_str().unwrap().to_owned());
                }
            }
        }
    }
    failures.sort();
    failures.dedup();
    assert!(
        failures.is_empty(),
        "blank CPG DataFlow endpoints: {failures:?}"
    );
}
