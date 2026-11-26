//! Frozen expectations for the typed reader. Records are written literally here, not derived
//! from the reader's own types.
use super::*;
use petgraph::graph::DiGraph;
use prism::access_path::AccessPath;
use prism::cpg::{CpgEdge, CpgNode, FlowConfidence, FlowDoubt, StmtKind, VarAccess};
use prism::resolution::ResolutionConfidence;

fn function(name: &str, start: usize) -> CpgNode {
    CpgNode::Function {
        name: name.into(),
        file: "a.py".into(),
        start_line: start,
        end_line: start + 3,
        start_byte: start * 10,
        end_byte: start * 10 + 30,
    }
}

fn var(owner: &str, owner_line: usize, path: &str, access: VarAccess) -> CpgNode {
    let path = match path.split_once('.') {
        Some((base, field)) => AccessPath::with_fields(base, vec![field.into()]),
        None => AccessPath::simple(path),
    };
    CpgNode::variable_occurrence(
        path,
        "a.py".into(),
        owner.into(),
        owner_line,
        5,
        access,
        40,
        44,
    )
}

fn lines(graph: &DiGraph<CpgNode, CpgEdge>) -> Vec<String> {
    let mut out = Vec::new();
    write_graph(graph, &mut out).expect("graph records");
    String::from_utf8(out)
        .unwrap()
        .lines()
        .map(String::from)
        .collect()
}

fn kind<'a>(all: &'a [String], kind: &str) -> Vec<&'a str> {
    let tag = format!("{{\"kind\":\"{kind}\"");
    all.iter()
        .filter(|l| l.starts_with(&tag))
        .map(String::as_str)
        .collect()
}

#[test]
fn typed_reader_includes_isolated_nodes_and_all_edge_payloads() {
    let mut g = DiGraph::new();
    let f = g.add_node(function("f", 1));
    let def = g.add_node(var("f", 1, "d.k", VarAccess::Def));
    let usage = g.add_node(var("f", 1, "d", VarAccess::Use));
    let stmt = g.add_node(CpgNode::Statement {
        file: "a.py".into(),
        line: 5,
        kind: StmtKind::Call {
            callee: "sink".into(),
        },
        start_byte: 40,
        end_byte: 50,
    });
    let ret = g.add_node(CpgNode::ReturnValue {
        file: "a.py".into(),
        function: "f".into(),
        function_start_line: 1,
        line: 6,
        return_start_byte: 60,
        return_end_byte: 70,
        child_slot: 2,
        start_byte: 62,
        end_byte: 68,
    });
    let isolated = g.add_node(function("lonely", 20));
    for (a, b, e) in [
        (def, usage, CpgEdge::DataFlow(FlowConfidence::Exact)),
        (
            def,
            usage,
            CpgEdge::DataFlow(FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 7 })),
        ),
        (
            def,
            usage,
            CpgEdge::DataFlow(FlowConfidence::NameOnly(FlowDoubt::SameLine)),
        ),
        (f, stmt, CpgEdge::Call(ResolutionConfidence::Exact)),
        (stmt, f, CpgEdge::Return(ResolutionConfidence::NameOnly)),
        (
            def,
            ret,
            CpgEdge::ReturnFlow {
                suppress_shortcut: true,
            },
        ),
        (usage, ret, CpgEdge::ReturnInput),
        (stmt, usage, CpgEdge::ControlFlow),
        (f, def, CpgEdge::Contains),
        (def, usage, CpgEdge::FieldOf),
        (def, usage, CpgEdge::DataFlow(FlowConfidence::Exact)),
    ] {
        g.add_edge(a, b, e);
    }
    let all = lines(&g);
    assert_eq!(kind(&all, "node").len(), 6, "every node, isolated included");
    let lonely = r#"{"kind":"node","type":"function","name":"lonely","file":"a.py","start_line":20,"end_line":23,"start_byte":200,"end_byte":230,"isolated":true}"#;
    assert!(all.iter().any(|l| l == lonely), "{all:#?}");
    let exact = r#""from":{"type":"variable","file":"a.py","owner":"f","owner_start_line":1,"line":5,"access":"def","path":{"base":"d","fields":["k"]},"start_byte":40,"end_byte":44},"to":{"type":"variable","file":"a.py","owner":"f","owner_start_line":1,"line":5,"access":"use","path":{"base":"d","fields":[]},"start_byte":40,"end_byte":44}"#;
    let edges = kind(&all, "edge");
    let flow = |payload: &str, n: usize| {
        let want = format!("{{\"kind\":\"edge\",{exact},\"edge\":{payload}}}");
        assert_eq!(
            edges.iter().filter(|l| **l == want).count(),
            n,
            "{payload}\n{edges:#?}"
        );
    };
    flow(
        r#"{"variant":"data_flow","confidence":"exact","doubt":null,"kill_line":null}"#,
        2,
    );
    flow(
        r#"{"variant":"data_flow","confidence":"nameonly","doubt":"killed","kill_line":7}"#,
        1,
    );
    flow(
        r#"{"variant":"data_flow","confidence":"nameonly","doubt":"sameline","kill_line":null}"#,
        1,
    );
    flow(r#"{"variant":"field_of"}"#, 1);
    assert_eq!(edges.len(), 11);
    for (needle, n) in [
        (r#""edge":{"variant":"call","confidence":"exact"}"#, 1),
        (r#""edge":{"variant":"return","confidence":"nameonly"}"#, 1),
        (
            r#""edge":{"variant":"return_flow","suppress_shortcut":true}"#,
            1,
        ),
        (r#""edge":{"variant":"return_input"}"#, 1),
        (r#""edge":{"variant":"control_flow"}"#, 1),
        (r#""edge":{"variant":"contains"}"#, 1),
        (
            r#""to":{"type":"return_value","file":"a.py","owner":"f","owner_start_line":1,"line":6,"return_start_byte":60,"return_end_byte":70,"child_slot":2,"start_byte":62,"end_byte":68}"#,
            2,
        ),
    ] {
        assert_eq!(
            edges.iter().filter(|l| l.contains(needle)).count(),
            n,
            "{needle}"
        );
    }
    let summary = kind(&all, "summary");
    assert_eq!(summary.len(), 1);
    assert!(
        summary[0].contains(r#""nodes":6,"isolated_nodes":1,"edges":11"#),
        "{}",
        summary[0]
    );

    // Controls: each payload bit moves the output with node and edge counts unchanged.
    let mut flipped = g.clone();
    for e in flipped.edge_weights_mut() {
        if let CpgEdge::ReturnFlow { suppress_shortcut } = e {
            *suppress_shortcut = false;
        }
    }
    assert_ne!(lines(&flipped), all);
    let mut conf = g.clone();
    for e in conf.edge_weights_mut() {
        if let CpgEdge::Call(c) = e {
            *c = ResolutionConfidence::NameOnly;
        }
    }
    assert_ne!(lines(&conf), all);
    let mut moved = g.clone();
    if let CpgNode::Function { end_byte, .. } = &mut moved[isolated] {
        *end_byte += 1;
    }
    assert_ne!(lines(&moved), all);

    // Insertion order does not matter.
    let mut rev = DiGraph::new();
    let order: Vec<_> = g.node_indices().rev().collect();
    let map: std::collections::BTreeMap<_, _> = order
        .iter()
        .map(|old| (*old, rev.add_node(g[*old].clone())))
        .collect();
    let mut es: Vec<_> = g.edge_indices().collect();
    es.reverse();
    for e in es {
        let (a, b) = g.edge_endpoints(e).unwrap();
        rev.add_edge(map[&a], map[&b], g[e].clone());
    }
    assert_eq!(lines(&rev), all);
}

#[test]
fn typed_reader_preserves_owner_on_equal_byte_endpoints() {
    let build = |target_owner: &str| {
        let mut g = DiGraph::new();
        let def = g.add_node(var("owner_a", 1, "x", VarAccess::Def));
        let use_a = g.add_node(var("owner_a", 1, "x", VarAccess::Use));
        let use_b = g.add_node(var("owner_b", 9, "x", VarAccess::Use));
        let to = if target_owner == "owner_a" {
            use_a
        } else {
            use_b
        };
        g.add_edge(def, to, CpgEdge::DataFlow(FlowConfidence::Exact));
        g
    };
    let to_a = lines(&build("owner_a"));
    let to_b = lines(&build("owner_b"));
    // Same nodes, same counts, byte-identical endpoint bytes; only the edge's owner differs.
    let identities = |all: &[String]| -> Vec<String> {
        kind(all, "node")
            .iter()
            .map(|l| l.rsplit_once(",\"isolated\"").unwrap().0.to_string())
            .collect()
    };
    assert_eq!(identities(&to_a), identities(&to_b));
    assert_eq!(kind(&to_a, "edge").len(), kind(&to_b, "edge").len());
    assert_ne!(kind(&to_a, "edge"), kind(&to_b, "edge"));
    let edge_b = kind(&to_b, "edge")[0];
    assert!(
        edge_b.contains(
            r#""to":{"type":"variable","file":"a.py","owner":"owner_b","owner_start_line":9"#
        ),
        "{edge_b}"
    );
    let edge_a = kind(&to_a, "edge")[0];
    assert!(
        edge_a.contains(
            r#""to":{"type":"variable","file":"a.py","owner":"owner_a","owner_start_line":1"#
        ),
        "{edge_a}"
    );
}

#[test]
fn typed_reader_rejects_ambiguous_node_identities() {
    let mut g = DiGraph::new();
    g.add_node(var("f", 1, "x", VarAccess::Use));
    g.add_node(var("f", 1, "x", VarAccess::Use));
    let mut out = Vec::new();
    let err = write_graph(&g, &mut out).expect_err("duplicate identity must be refused");
    assert!(err.to_string().contains("ambiguous node identity"), "{err}");
}

#[test]
fn typed_reader_reads_a_tiny_repo_twice_identically() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("m.py"),
        "def f(t):\n    d = {}\n    d[\"k\"] = t\n    return d\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("note.txt"), "not code\n").unwrap();
    let run = || {
        let mut out = Vec::new();
        read_repo(dir.path(), &mut out).expect("read");
        String::from_utf8(out).unwrap()
    };
    let first = run();
    assert_eq!(first, run());
    let all: Vec<String> = first.lines().map(String::from).collect();
    assert!(
        all[0].starts_with(r#"{"kind":"meta","schema":"element_recall_graph/1""#),
        "{}",
        all[0]
    );
    let identity = prism::api::build_info().git_sha;
    assert!(!identity.is_empty());
    assert_eq!(
        all[0],
        format!(
            r#"{{"kind":"meta","schema":"element_recall_graph/1","prism_version":"{}","build_identity":"{identity}"}}"#,
            env!("CARGO_PKG_VERSION")
        )
    );
    assert_eq!(kind(&all, "file").len(), 1);
    assert!(kind(&all, "file")[0].starts_with(r#"{"kind":"file","key":"m.py","sha256":""#));
    assert!(
        kind(&all, "skip")
            .iter()
            .any(|l| l.contains(r#""path":"note.txt""#) && l.contains("Unsupported")),
        "{all:#?}"
    );
    assert!(!kind(&all, "node").is_empty());
    assert!(all.last().unwrap().starts_with(r#"{"kind":"summary""#));
}
