//! S2 fold 2 (#639, PR #665 review round 1) at the assembled-CPG level:
//! public lookup parity, Step 5c isolation, every struct form for func fields,
//! and scoped builds. The programs are parsed, never executed.
use super::field_recall_s2f1_tests::{
    dataflow_edges, flip_matrix, graph_with_recall, parse_sources, prefix_edges,
};
use crate::ast::ParsedFile;
use crate::cpg::{CodePropertyGraph, CpgContext, CpgEdge};
use crate::data_flow::VarLocation;
use crate::diff::{DiffInfo, DiffInput, ModifyType};
use crate::languages::Language;
use petgraph::visit::EdgeRef;
use std::collections::{BTreeMap, BTreeSet};

// ---- 1. lookup index parity --------------------------------------------------

fn use_of(graph: &CodePropertyGraph, path: &str, line: usize) -> VarLocation {
    graph
        .dfg
        .edges()
        .iter()
        .map(|e| e.to.clone())
        .find(|u| u.path.to_string() == path && u.line == line)
        .unwrap_or_else(|| panic!("no Use {path}@{line}"))
}

fn lookups(graph: &CodePropertyGraph, probe: &VarLocation) -> String {
    format!(
        "{:#?}\nnode={:?}\nbackward={:#?}",
        graph.var_index,
        graph.var_node_for_location(probe),
        graph.dfg_backward_reachable(probe),
    )
}

/// The stored/public `var_index` keeps recall-only receiver Uses (a cache load
/// rebuilds it from every node), so a cold build, a direct full hit and a
/// partial hit answer `var_node_for_location` and `dfg_backward_reachable`
/// alike for the `cmd` receiver of `cmd.Run()`.
#[test]
fn lookup_indexes_are_identical_on_cold_full_hit_and_partial_hit() {
    use crate::cpg_cache::{compute_file_hashes, load_cache, save_cache, CacheResult};
    let source = include_str!("../../../tests/fixtures/go_field_write/60_exec_cmd/main.go");
    let sources = BTreeMap::from([("cmd.go".to_string(), source.to_string())]);
    let files = parse_sources(&sources);
    let cold = CodePropertyGraph::build(&files);
    let probe = use_of(&cold, "cmd", 10);
    assert!(cold.var_node_for_location(&probe).is_some(), "cold lookup");
    assert!(
        !cold.dfg_backward_reachable(&probe).is_empty(),
        "cold backward reachability from the receiver"
    );
    let hashes = compute_file_hashes(&sources);
    let dir = tempfile::tempdir().unwrap();
    save_cache(&cold, &hashes, false, dir.path()).unwrap();
    let CacheResult::Hit(full) = load_cache(&hashes, false, dir.path()) else {
        panic!("cache miss");
    };
    let CacheResult::Hit(loaded) = load_cache(&hashes, false, dir.path()) else {
        panic!("cache miss");
    };
    let partial = CodePropertyGraph::build_incremental(
        loaded.call_graph,
        loaded.dfg,
        &BTreeSet::from(["cmd.go".to_string()]),
        &files,
        None,
    );
    let expected = lookups(&cold, &probe);
    assert_eq!(lookups(&full, &probe), expected, "direct full hit");
    assert_eq!(lookups(&partial, &probe), expected, "partial hit");
}

// ---- 2. Step 5c never consumes a recall-only receiver Use ---------------------

const RETURN_RECEIVER: &str = r#"package pkg

type R struct{ f int }

func (r *R) Value() int { return r.f }

func f(r *R, x int) int {
	r.f = x
	return r.Value()
}

func caller(r *R, x int) int {
	y := f(r, x)
	return y
}
"#;

const CALLER_RHS_RECEIVER: &str = r#"package pkg

type R struct{ f int }

func (r *R) Value() int { return r.f }

func g(v int) int { return v }

func caller(r *R, x int) {
	r.f = x
	y := g(r.Value())
	println(y)
}
"#;

fn non_dataflow_edges(graph: &CodePropertyGraph) -> usize {
    graph
        .graph
        .edge_references()
        .filter(|e| !matches!(e.weight(), CpgEdge::DataFlow(_)))
        .count()
}

/// Every counter outside `dfg_labels` (the return-flow counters and the graph's
/// non-data-flow edges, `ReturnInput` among them) equals the recall-disabled
/// build; the only addition is the recall edge itself.
fn assert_lane_local(source: &str) {
    let (main, head) = (
        graph_with_recall(source, false),
        graph_with_recall(source, true),
    );
    assert_eq!(main.return_flow_stats, head.return_flow_stats);
    assert_eq!(non_dataflow_edges(&main), non_dataflow_edges(&head));
    let (m, h) = (dataflow_edges(&main), dataflow_edges(&head));
    assert!(m.is_subset(&h), "head only adds: {:?}", m.difference(&h));
    let added: Vec<_> = h.difference(&m).collect();
    assert_eq!(added.len(), 1, "{added:#?}");
    assert!(added[0].starts_with("r.f@"), "{added:#?}");
    // The call-stats labels outside the recall class are untouched too.
    let (a, b) = (&main.dfg_label_stats, &head.dfg_label_stats);
    assert_eq!(a.dfg_label_exact, b.dfg_label_exact - 1);
}

#[test]
fn step5c_return_expression_receiver_is_lane_local() {
    assert_lane_local(RETURN_RECEIVER);
}

#[test]
fn step5c_caller_rhs_receiver_is_lane_local() {
    assert_lane_local(CALLER_RHS_RECEIVER);
}

// ---- 8. callable fields in every struct form ------------------------------------

const MAIN: &str = super::field_recall_s2f1_tests::MAIN;

fn assert_refused(files: &[(&str, &str)], refused: bool) {
    let sources: BTreeMap<String, String> = files
        .iter()
        .map(|(n, s)| (format!("pkg/{n}"), s.to_string()))
        .collect();
    let graph = CodePropertyGraph::build(&parse_sources(&sources));
    let edges = prefix_edges(&graph);
    assert_eq!(edges.is_empty(), refused, "{edges:?}");
    assert_eq!(
        graph.dfg_label_stats.dfg_recall_receiver_field_refused,
        usize::from(refused)
    );
}

#[test]
fn anonymous_struct_func_field_is_refused() {
    assert_refused(
        &[(
            "main.go",
            "package pkg\n\nfunc f(x string) {\n\tp := struct {\n\t\tf   string\n\t\trun func()\n\t}{run: func() {}}\n\tp.f = x\n\tp.run()\n}\n",
        )],
        true,
    );
}

#[test]
fn function_local_named_struct_func_field_is_refused() {
    assert_refused(
        &[(
            "main.go",
            "package pkg\n\nfunc f(x string) {\n\ttype local struct {\n\t\tf   string\n\t\trun func()\n\t}\n\tvar p local\n\tp.f = x\n\tp.run()\n}\n",
        )],
        true,
    );
}

#[test]
fn local_struct_without_a_callable_field_is_admitted() {
    assert_refused(
        &[(
            "main.go",
            "package pkg\n\nfunc f(x string) {\n\ttype local struct {\n\t\tf   string\n\t\trun int\n\t}\n\tvar p local\n\tp.f = x\n\tp.run()\n}\n",
        )],
        false,
    );
}

#[test]
fn sibling_file_anonymous_and_local_struct_fields_flip_with_the_sibling() {
    let anon = |ty: &str| format!("package pkg\n\nvar a = struct{{ run {ty} }}{{}}\n");
    flip_matrix(&anon("func()"), &anon("int"));
    let local = |ty: &str| {
        format!("package pkg\n\nfunc other() {{\n\ttype t struct{{ run {ty} }}\n\t_ = t{{}}\n}}\n")
    };
    flip_matrix(&local("func()"), &local("int"));
}

// ---- 4. scoped builds -----------------------------------------------------------

fn diff(path: &str) -> DiffInput {
    DiffInput {
        files: vec![DiffInfo {
            file_path: path.into(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    }
}

/// A scoped review build assembles only its in-scope files, but the field is
/// declared in an out-of-scope, types-only sibling: the refusal must match a
/// full build's.
#[test]
fn scoped_build_refuses_with_an_out_of_scope_declaring_sibling() {
    let mut sources = BTreeMap::from([
        ("pkg/decl.go".to_string(), super::field_recall_s2f1_tests::FUNC_FIELD.to_string()),
        ("pkg/main.go".to_string(), MAIN.to_string()),
    ]);
    for i in 0..4 {
        sources.insert(
            format!("other{i}/f.go"),
            format!("package other{i}\n\nfunc g{i}() int {{\n\treturn {i}\n}}\n"),
        );
    }
    let files: BTreeMap<String, ParsedFile> = sources
        .iter()
        .map(|(p, s)| (p.clone(), ParsedFile::parse(p, s, Language::Go).unwrap()))
        .collect();
    let full = CodePropertyGraph::build(&files);
    assert!(prefix_edges(&full).is_empty(), "a full build refuses");
    let ctx = CpgContext::build_scoped(&files, &diff("pkg/main.go"), None);
    let scope = ctx.scope.as_ref().expect("a scoped build");
    assert!(scope.scoped_files.contains("pkg/main.go"));
    assert!(!scope.scoped_files.contains("pkg/decl.go"));
    assert!(prefix_edges(&ctx.cpg).is_empty(), "{:?}", prefix_edges(&ctx.cpg));
    assert_eq!(ctx.cpg.dfg_label_stats.dfg_recall_receiver_field_refused, 1);
}
