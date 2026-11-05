//! S2 fold 1 (#639): package-level func-field refusal for receiver edges and
//! lane-local receiver Uses. The programs are parsed, never executed.
use crate::ast::ParsedFile;
use crate::cpg::{CodePropertyGraph, CpgEdge, CpgNode, FlowConfidence, FlowDoubt};
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

/// Rows whose expectation needs the assembled repository (package facts).
pub(super) const PACKAGE_ROWS: &[&str] = &[
    "54_recv_field_call",
    "87_xfile_type_func",
    "88_xfile_func_lit",
    "89_xfile_qualified",
    "90_local_struct_admitted",
    "91_field_and_method_refused",
    "92_external_test_package",
    "93_internal_test_package",
    "94_noncallable_fields",
    "95_alias_func",
    "96_unclassified_type",
    "97_embedded_foreign",
];

pub(super) fn parse_dir(case: &str) -> BTreeMap<String, ParsedFile> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/go_field_write")
        .join(case);
    let mut files = BTreeMap::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "go") {
            let name = format!("pkg/{}", path.file_name().unwrap().to_str().unwrap());
            let source = std::fs::read_to_string(&path).unwrap();
            let parsed = ParsedFile::parse(&name, &source, Language::Go).unwrap();
            files.insert(name, parsed);
        }
    }
    files
}

pub(super) type EdgeRow = (String, usize, String, usize, String);

/// Strict-prefix edges of the assembled DFG, as the single-file harness reports.
pub(super) fn prefix_edges(graph: &CodePropertyGraph) -> BTreeSet<EdgeRow> {
    let mut out = BTreeSet::new();
    for e in graph.dfg.edges().iter() {
        let (d, u) = (&e.from.path, &e.to.path);
        if d.base == u.base && d.fields.len() > u.fields.len() && d.fields.starts_with(&u.fields) {
            let label = match graph.dfg.label(e.from, e.to) {
                Some(FlowConfidence::Exact) => "Exact",
                Some(FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)) => "CfgIncomplete",
                Some(FlowConfidence::NameOnly(FlowDoubt::SameLine)) => "SameLine",
                other => panic!("{}: {other:?}", e.from.path),
            };
            out.insert((
                d.to_string(),
                e.from.line,
                u.to_string(),
                e.to.line,
                label.to_string(),
            ));
        }
    }
    out
}

pub(super) fn package_matrix(case: &str) {
    let rows: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/go_field_write/expected.json"
    ))
    .unwrap();
    let row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["case"] == case)
        .unwrap();
    let expected: BTreeSet<EdgeRow> = row["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            (
                e[0].as_str().unwrap().to_string(),
                e[1].as_u64().unwrap() as usize,
                e[2].as_str().unwrap().to_string(),
                e[3].as_u64().unwrap() as usize,
                e[4].as_str().unwrap().to_string(),
            )
        })
        .collect();
    let graph = CodePropertyGraph::build(&parse_dir(case));
    assert_eq!(prefix_edges(&graph), expected, "{case}");
    // The counter agrees with the matrix: a refused row refuses its edge(s).
    let refused = graph.dfg_label_stats.dfg_recall_receiver_field_refused;
    assert_eq!(
        refused > 0,
        expected.is_empty(),
        "{case}: refused={refused}"
    );
}

macro_rules! cases {
    ($($test:ident => $case:literal),* $(,)?) => { $(
        #[test] fn $test() { package_matrix($case); }
    )* };
}
cases! {
    s2f1_54 => "54_recv_field_call", s2f1_87 => "87_xfile_type_func",
    s2f1_88 => "88_xfile_func_lit", s2f1_89 => "89_xfile_qualified",
    s2f1_90 => "90_local_struct_admitted", s2f1_91 => "91_field_and_method_refused",
    s2f1_92 => "92_external_test_package", s2f1_93 => "93_internal_test_package",
    s2f1_94 => "94_noncallable_fields", s2f1_95 => "95_alias_func",
    s2f1_96 => "96_unclassified_type", s2f1_97 => "97_embedded_foreign",
}

fn describe(graph: &CodePropertyGraph, i: petgraph::graph::NodeIndex) -> String {
    match &graph.graph[i] {
        CpgNode::Variable {
            path,
            line,
            access,
            start_byte,
            ..
        } => format!("{path}@{line}:{start_byte}:{access:?}"),
        other => format!("{other:?}"),
    }
}

pub(super) fn dataflow_edges(graph: &CodePropertyGraph) -> BTreeSet<String> {
    use petgraph::visit::EdgeRef;
    graph
        .graph
        .edge_references()
        .filter_map(|e| match e.weight() {
            CpgEdge::DataFlow(c) => Some(format!(
                "{} -> {} {c:?}",
                describe(graph, e.source()),
                describe(graph, e.target())
            )),
            _ => None,
        })
        .collect()
}

const LEAK: &str = r#"package pkg

type R struct{ f int }

func (r *R) clock() int { return 1 }

func sink(now int) { println(now) }

func tick(r *R, x int) {
	r.f = x
	sink(r.clock().Now())
}
"#;

fn leak_graph(recall: bool) -> CodePropertyGraph {
    graph_with_recall(LEAK, recall)
}

/// The assembled CPG of one source, with the recall lane on, or off (a zero
/// budget: no recall edge, no receiver Use), standing in for main.
pub(super) fn graph_with_recall(source: &str, recall: bool) -> CodePropertyGraph {
    let files = BTreeMap::from([(
        "pkg/tick.go".to_string(),
        ParsedFile::parse("pkg/tick.go", source, Language::Go).unwrap(),
    )]);
    if recall {
        CodePropertyGraph::build(&files)
    } else {
        // A zero budget caps the recall lane: no recall edge, no receiver Use.
        // The cap is thread-local, so the producer runs on this thread.
        let mut dfg = crate::data_flow::DataFlowGraph::empty();
        for (file, parsed) in &files {
            let parts = crate::cpg::with_recall_cap(0, || super::Producer::file_refs(file, parsed));
            dfg.merge(crate::data_flow::DataFlowGraph {
                storage: crate::data_flow::compact::CompactDfg::from_parts(parts.into_parts())
                    .unwrap(),
            });
        }
        let base = CodePropertyGraph::build(&files);
        CodePropertyGraph::build_incremental(base.call_graph, dfg, &BTreeSet::new(), &files, None)
    }
}

/// Defect 2: a receiver Use that exists only as a recall endpoint is never
/// bound by Step 5b to a callee formal (`sink(r.clock().Now())` binds the
/// argument whose base fallback `r` used to find the recall Use).
#[test]
fn recall_only_receiver_use_is_lane_local() {
    let (main, head) = (leak_graph(false), leak_graph(true));
    let (m, h) = (dataflow_edges(&main), dataflow_edges(&head));
    assert!(m.is_subset(&h), "head only adds: {:?}", m.difference(&h));
    let added: Vec<_> = h.difference(&m).collect();
    // The recall edge itself, and nothing else.
    assert_eq!(added.len(), 1, "{added:#?}");
    assert!(added[0].starts_with("r.f@10:") && added[0].contains("-> r@11:"));
    assert!(!h.iter().any(|e| e.starts_with("r@")), "{h:#?}");
    assert_eq!(main.return_flow_stats, head.return_flow_stats);
    assert_eq!(main.graph.node_count() + 1, head.graph.node_count());
}

pub(super) fn parse_sources(sources: &BTreeMap<String, String>) -> BTreeMap<String, ParsedFile> {
    sources
        .iter()
        .map(|(p, s)| (p.clone(), ParsedFile::parse(p, s, Language::Go).unwrap()))
        .collect()
}

pub(super) const MAIN: &str =
    "package pkg\n\nfunc use(p *B, x string) {\n\tp.f = x\n\tp.run()\n}\n";
pub(super) const FUNC_FIELD: &str = "package pkg\n\ntype A struct{ run func() }\n";
pub(super) const INT_FIELD: &str = "package pkg\n\ntype A struct{ run int }\n";

/// Cold, full-hit and partial-hit builds agree, including when only the
/// sibling file that declares the field changes (the receiver file's cached
/// DFG is reused and its edge flips), in both directions.
#[test]
fn receiver_refusal_is_identical_across_cold_full_hit_and_partial_hit() {
    flip_matrix(FUNC_FIELD, INT_FIELD);
}

/// `refusing` declares a callable-possible field named `run`; `plain` does not.
pub(super) fn flip_matrix(refusing: &str, plain: &str) {
    use crate::cpg_cache::{compute_file_hashes, load_cache, save_cache, CacheResult};
    let sources = |decl: &str, main: &str| {
        BTreeMap::from([
            ("pkg/decl.go".to_string(), decl.to_string()),
            ("pkg/main.go".to_string(), main.to_string()),
        ])
    };
    let observe = |g: &CodePropertyGraph| {
        (
            prefix_edges(g),
            g.dfg_label_stats.dfg_recall_receiver_field_refused,
            dataflow_edges(g),
            g.dfg.rd_function_stats().clone(),
        )
    };
    let main_edge: BTreeSet<EdgeRow> = BTreeSet::from([(
        "p.f".to_string(),
        4,
        "p".to_string(),
        5,
        "Exact".to_string(),
    )]);
    for (before_decl, after_decl, before_edge, after_edge) in [
        (refusing, plain, false, true),
        (plain, refusing, true, false),
    ] {
        let before = sources(before_decl, MAIN);
        let after = sources(after_decl, MAIN);
        let (before_files, after_files) = (parse_sources(&before), parse_sources(&after));
        let cold_before = CodePropertyGraph::build(&before_files);
        let cold_after = CodePropertyGraph::build(&after_files);
        let (b, a) = (observe(&cold_before), observe(&cold_after));
        assert_eq!(b.0 == main_edge, before_edge, "{:?}", b.0);
        assert_eq!(a.0 == main_edge, after_edge, "{:?}", a.0);
        assert_eq!(b.1, usize::from(!before_edge));
        assert_eq!(a.1, usize::from(!after_edge));
        let hashes = compute_file_hashes(&before);
        let dir = tempfile::tempdir().unwrap();
        save_cache(&cold_before, &hashes, false, dir.path()).unwrap();
        // Full hit: nothing changed.
        let CacheResult::Hit(loaded) = load_cache(&hashes, false, dir.path()) else {
            panic!("cache miss");
        };
        let full = CodePropertyGraph::build_incremental(
            loaded.call_graph,
            loaded.dfg,
            &BTreeSet::new(),
            &before_files,
            None,
        );
        assert_eq!(observe(&full), b, "full hit");
        // Partial hit: only the declaring file changed.
        let CacheResult::Hit(loaded) = load_cache(&hashes, false, dir.path()) else {
            panic!("cache miss");
        };
        let partial = CodePropertyGraph::build_incremental(
            loaded.call_graph,
            loaded.dfg,
            &BTreeSet::from(["pkg/decl.go".to_string()]),
            &after_files,
            None,
        );
        assert_eq!(observe(&partial), a, "partial hit, declaring file only");
        // Partial hit: only the receiver file changed (its edge is rebuilt).
        let CacheResult::Hit(loaded) = load_cache(&hashes, false, dir.path()) else {
            panic!("cache miss");
        };
        let edited = sources(before_decl, &format!("{MAIN}\n"));
        let edited_files = parse_sources(&edited);
        let receiver_only = CodePropertyGraph::build_incremental(
            loaded.call_graph,
            loaded.dfg,
            &BTreeSet::from(["pkg/main.go".to_string()]),
            &edited_files,
            None,
        );
        let cold_edited = CodePropertyGraph::build(&edited_files);
        assert_eq!(
            observe(&receiver_only),
            observe(&cold_edited),
            "receiver file only"
        );
        // The state survives a save/load of an already-flipped graph.
        let hashes_after = compute_file_hashes(&after);
        let dir2 = tempfile::tempdir().unwrap();
        save_cache(&partial, &hashes_after, false, dir2.path()).unwrap();
        let CacheResult::Hit(loaded) = load_cache(&hashes_after, false, dir2.path()) else {
            panic!("cache miss");
        };
        let again = CodePropertyGraph::build_incremental(
            loaded.call_graph,
            loaded.dfg,
            &BTreeSet::new(),
            &after_files,
            None,
        );
        assert_eq!(observe(&again), a, "reload of a flipped graph");
    }
}
