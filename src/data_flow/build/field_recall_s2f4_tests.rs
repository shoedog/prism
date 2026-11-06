//! #677: a func field declared in a `_test.go` file refuses only receiver
//! edges whose call site is in a `_test.go` file of the same package.
//! Programs are parsed, never executed.
use super::field_recall_s2f1_tests::{dataflow_edges, package_matrix, parse_sources, prefix_edges};
use crate::cpg::CodePropertyGraph;
use std::collections::{BTreeMap, BTreeSet};

macro_rules! cases {
    ($($test:ident => $case:literal),* $(,)?) => { $(
        #[test] fn $test() { package_matrix($case); }
    )* };
}
cases! {
    s2f4_a_test_field_nontest_call_admitted => "104_testfield_nontest_call",
    s2f4_b_test_field_test_call_refused => "105_testfield_test_call",
    s2f4_c_nontest_field_nontest_call_refused => "106_nontest_field_nontest_call",
    s2f4_d_external_package_no_effect => "107_external_test_field",
    s2f4_test_type_resolves_for_test_call => "108_testtype_test_call",
    s2f4_test_type_hidden_from_nontest_call => "109_testtype_nontest_call",
}

const CALL: &str = "package pkg\n\nfunc use(n *N, v int) {\n\tn.run.f = v\n\tn.run.init()\n}\n";
const FUNC_FIELD: &str = "package pkg\n\ntype N struct{ init func() }\n";
const INT_FIELD: &str = "package pkg\n\ntype N struct{ init int }\n";

fn sources(decl: &str, call_file: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("pkg/x_test.go".to_string(), decl.to_string()),
        (format!("pkg/{call_file}"), CALL.to_string()),
    ])
}

/// Cold, full-hit and partial-hit builds agree when only the `_test.go`
/// declaring file changes, in both directions.
fn flip(call_file: &str, edge_when_func: bool) {
    use crate::cpg_cache::{compute_file_hashes, load_cache, save_cache, CacheResult};
    let observe = |g: &CodePropertyGraph| {
        (
            prefix_edges(g),
            g.dfg_label_stats.dfg_recall_receiver_field_refused,
            dataflow_edges(g),
            g.dfg.rd_function_stats().clone(),
        )
    };
    for (before_decl, after_decl) in [(FUNC_FIELD, INT_FIELD), (INT_FIELD, FUNC_FIELD)] {
        let (before, after) = (
            sources(before_decl, call_file),
            sources(after_decl, call_file),
        );
        let (bf, af) = (parse_sources(&before), parse_sources(&after));
        let (cold_b, cold_a) = (CodePropertyGraph::build(&bf), CodePropertyGraph::build(&af));
        let (b, a) = (observe(&cold_b), observe(&cold_a));
        let has = |o: &BTreeSet<_>| !o.is_empty();
        let func_is = |decl: &str| decl == FUNC_FIELD;
        assert_eq!(
            has(&b.0),
            !func_is(before_decl) || edge_when_func,
            "{call_file}"
        );
        assert_eq!(
            has(&a.0),
            !func_is(after_decl) || edge_when_func,
            "{call_file}"
        );
        let hashes = compute_file_hashes(&before);
        let dir = tempfile::tempdir().unwrap();
        save_cache(&cold_b, &hashes, false, dir.path()).unwrap();
        let load = || match load_cache(&hashes, false, dir.path()) {
            CacheResult::Hit(l) => l,
            _ => panic!("cache miss"),
        };
        let l = load();
        let full =
            CodePropertyGraph::build_incremental(l.call_graph, l.dfg, &BTreeSet::new(), &bf, None);
        assert_eq!(observe(&full), b, "full hit");
        let l = load();
        let partial = CodePropertyGraph::build_incremental(
            l.call_graph,
            l.dfg,
            &BTreeSet::from(["pkg/x_test.go".to_string()]),
            &af,
            None,
        );
        assert_eq!(observe(&partial), a, "partial hit, declaring _test.go only");
    }
}

/// (e) An edge in a non-test file does not flip when only the `_test.go`
/// declaring file changes.
#[test]
fn s2f4_e_nontest_edge_stays_when_only_test_decl_changes() {
    flip("x.go", true);
}

/// Control: a `_test.go` call site does flip with its package's test field.
#[test]
fn s2f4_test_call_flips_with_test_decl() {
    flip("y_test.go", false);
}
