use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
use prism::cpg::CodePropertyGraph;
use prism::data_flow::DataFlowGraph;
use prism::languages::Language;
use prism::navigation::queries::call_stats;
use std::collections::{BTreeMap, BTreeSet};

fn parse_go(sources: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
    sources
        .iter()
        .map(|(path, source)| {
            (
                (*path).to_string(),
                ParsedFile::parse(path, source, Language::Go).expect("parse Go fixture"),
            )
        })
        .collect()
}

fn build_go(sources: &[(&str, &str)]) -> CallGraph {
    CallGraph::build(&parse_go(sources))
}

fn build_field_partition(caller_path: &str, caller_header: &str) -> CallGraph {
    let caller = format!("{caller_header}package foo\nfunc invoke(t T) {{ t.f.Dial() }}\n");
    let sources = [
        (
            "pkg/a_linux.go",
            "//go:build linux\n\npackage foo\ntype T struct { f Conn }\ntype Conn struct{}\nfunc (Conn) Dial() {}\n",
        ),
        (
            "pkg/z_windows.go",
            "//go:build windows\n\npackage foo\ntype T struct { f Mock }\ntype Mock struct{}\nfunc (Mock) Dial() {}\n",
        ),
        (caller_path, caller.as_str()),
    ];
    build_go(&sources)
}

#[test]
fn call_stats_reports_owner_partition_recovery_site_and_edge() {
    let cg = build_field_partition("pkg/use_linux.go", "//go:build linux\n\n");
    let stats = call_stats(&cg);

    assert_eq!(stats["go_owner_identity_partition_recovered"], 1);
    assert_eq!(stats["go_owner_identity_partition_drop"], 0);
    assert_eq!(stats["go_owner_identity_partition_affected_sites"], 1);
    assert_eq!(stats["go_owner_identity_partition_affected_edges"], 1);
}

#[test]
fn call_stats_reports_owner_partition_conflict_drop_site_and_edge() {
    let cg = build_field_partition("pkg/use.go", "");
    let stats = call_stats(&cg);

    assert_eq!(stats["go_owner_identity_partition_recovered"], 0);
    assert_eq!(stats["go_owner_identity_partition_drop"], 1);
    assert_eq!(stats["go_owner_identity_partition_affected_sites"], 1);
    assert_eq!(stats["go_owner_identity_partition_affected_edges"], 1);
}

#[test]
fn direct_method_cross_package_pruning_reports_recovery() {
    let cg = build_go(&[
        (
            "a/a.go",
            "package a\ntype Handler struct{}\nfunc (Handler) Act() {}\nfunc invoke(h Handler) { h.Act() }\n",
        ),
        (
            "b/b.go",
            "package b\ntype Handler struct{}\nfunc (Handler) Act() {}\n",
        ),
    ]);
    let stats = call_stats(&cg);

    assert_eq!(act_owners(&cg), BTreeSet::from(["Handler".to_string()]));
    assert_eq!(stats["go_owner_identity_partition_recovered"], 1);
    assert_eq!(stats["go_owner_identity_partition_drop"], 0);
    assert_eq!(stats["go_owner_identity_partition_affected_edges"], 2);
}

#[test]
fn direct_method_conflicting_visible_build_survivors_report_drop() {
    let cg = build_go(&[
        (
            "pkg/a.go",
            "//go:build alpha\n\npackage foo\ntype T struct{}\nfunc (T) Act() {}\n",
        ),
        (
            "pkg/b.go",
            "//go:build beta\n\npackage foo\ntype T struct{}\nfunc (T) Act() {}\n",
        ),
        (
            "pkg/use.go",
            "//go:build alpha\n\npackage foo\nfunc invoke(t T) { t.Act() }\n",
        ),
    ]);
    let stats = call_stats(&cg);

    assert!(act_owners(&cg).is_empty());
    assert_eq!(stats["go_owner_identity_partition_recovered"], 0);
    assert_eq!(stats["go_owner_identity_partition_drop"], 1);
    assert_eq!(stats["go_owner_identity_partition_affected_edges"], 2);
}

#[test]
fn call_stats_omits_owner_partition_extension_for_non_go_graphs() {
    let stats = call_stats(&CallGraph::empty());

    for key in [
        "go_owner_identity_partition_affected_owners",
        "go_owner_identity_partition_drop",
        "go_owner_identity_partition_recovered",
        "go_owner_identity_partition_affected_sites",
        "go_owner_identity_partition_affected_edges",
    ] {
        assert!(stats.get(key).is_none(), "unexpected non-Go key: {key}");
    }
}

fn build_callback_partition(caller_path: &str, caller_header: &str) -> CallGraph {
    let caller = format!("{caller_header}package foo\nfunc invoke(c Command) {{ c.Run() }}\n");
    build_go(&[
        (
            "pkg/a_linux.go",
            "//go:build linux\n\npackage foo\ntype Command struct { Run func() }\nfunc linuxHandler() {}\nfunc setupLinux() { _ = Command{Run: linuxHandler} }\n",
        ),
        (
            "pkg/z_windows.go",
            "//go:build windows\n\npackage foo\ntype Command struct { Run func() }\nfunc windowsHandler() {}\nfunc setupWindows() { _ = Command{Run: windowsHandler} }\n",
        ),
        (caller_path, &caller),
    ])
}

#[test]
fn p5_runtime_partition_telemetry_reports_both_poles() {
    let recovered = call_stats(&build_callback_partition(
        "pkg/use_linux.go",
        "//go:build linux\n\n",
    ));
    assert_eq!(recovered["go_owner_identity_partition_recovered"], 1);
    assert_eq!(recovered["go_owner_identity_partition_drop"], 0);

    let dropped = call_stats(&build_callback_partition("pkg/use.go", ""));
    assert_eq!(dropped["go_owner_identity_partition_recovered"], 0);
    assert_eq!(dropped["go_owner_identity_partition_drop"], 1);
}

fn build_s4_partition(caller_path: &str, caller_header: &str) -> CallGraph {
    let caller = format!("{caller_header}package foo\nfunc invoke(h Holder) {{ h.Act() }}\n");
    build_go(&[
        (
            "pkg/a_linux.go",
            "//go:build linux\n\npackage foo\ntype Doer interface { Act() }\ntype Holder struct { Doer }\n",
        ),
        (
            "pkg/z_windows.go",
            "//go:build windows\n\npackage foo\ntype Other interface { Act() }\ntype Holder struct { Other }\n",
        ),
        (
            "pkg/impl.go",
            "package foo\ntype Impl struct{}\nfunc (Impl) Act() {}\n",
        ),
        (caller_path, &caller),
    ])
}

#[test]
fn s4_runtime_partition_telemetry_reports_both_poles() {
    let recovered = call_stats(&build_s4_partition(
        "pkg/use_linux.go",
        "//go:build linux\n\n",
    ));
    assert_eq!(recovered["go_owner_identity_partition_recovered"], 1);
    assert_eq!(recovered["go_owner_identity_partition_drop"], 0);

    let dropped = call_stats(&build_s4_partition("pkg/use.go", ""));
    assert_eq!(dropped["go_owner_identity_partition_recovered"], 0);
    assert_eq!(dropped["go_owner_identity_partition_drop"], 1);
}

#[test]
fn s4_signature_partition_recovery_is_reported_without_last_writer_help() {
    let cg = build_go(&[
        (
            "pkg/a_windows.go",
            "//go:build windows\n\npackage foo\ntype Doer interface { Act(int) }\n",
        ),
        (
            "pkg/z_linux.go",
            "//go:build linux\n\npackage foo\ntype Doer interface { Act() }\n",
        ),
        (
            "pkg/holder.go",
            "package foo\ntype Holder struct { Doer }\ntype Impl struct{}\nfunc (Impl) Act() {}\n",
        ),
        (
            "pkg/use_linux.go",
            "//go:build linux\n\npackage foo\nfunc invoke(h Holder) { h.Act() }\n",
        ),
    ]);
    let stats = call_stats(&cg);

    assert_eq!(act_owners(&cg), BTreeSet::from(["Impl".to_string()]));
    assert_eq!(stats["go_owner_identity_partition_recovered"], 1);
    assert_eq!(stats["go_owner_identity_partition_drop"], 0);
}

fn round_trip(cg: &CallGraph) -> CallGraph {
    let bytes = bincode::serialize(cg).expect("serialize CallGraph");
    bincode::deserialize(&bytes).expect("deserialize CallGraph")
}

#[test]
fn round_trip_preserves_partition_snapshots_registration_provenance_and_telemetry() {
    let field = build_field_partition("pkg/use_linux.go", "//go:build linux\n\n");
    let field_round = round_trip(&field);
    assert_eq!(field_round.go_field_types, field.go_field_types);
    assert_eq!(
        field_round.go_known_struct_identities,
        field.go_known_struct_identities
    );
    assert_eq!(field_round.go_func_typed_fields, field.go_func_typed_fields);
    assert_eq!(field_round.go_file_profiles, field.go_file_profiles);
    assert_eq!(
        field_round.go_owner_identity_partition,
        field.go_owner_identity_partition
    );
    assert!(field_round
        .go_field_types
        .keys()
        .all(|owner| owner.package_clause == "foo"));

    let callbacks = build_callback_partition("pkg/use_linux.go", "//go:build linux\n\n");
    let callbacks_round = round_trip(&callbacks);
    assert_eq!(callbacks_round.go_registrations, callbacks.go_registrations);
    assert_eq!(callbacks_round.go_registrations.len(), 2);
    assert!(callbacks_round.go_registrations.iter().all(|record| record
        .site
        .file
        .ends_with("_linux.go")
        || record.site.file.ends_with("_windows.go")));

    let s4 = build_s4_partition("pkg/use_linux.go", "//go:build linux\n\n");
    let s4_round = round_trip(&s4);
    assert_eq!(
        s4_round.go_interface_declarations,
        s4.go_interface_declarations
    );
    assert_eq!(s4_round.go_method_declarations, s4.go_method_declarations);
    assert_eq!(s4_round.go_interface_live_types, s4.go_interface_live_types);
    assert_eq!(
        s4_round.go_embedded_interface_methods,
        s4.go_embedded_interface_methods
    );
}

fn dial_owners(cg: &CallGraph) -> BTreeSet<String> {
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|site| site.caller.name == "invoke" && site.callee_name == "Dial")
        .expect("invoke Dial site");
    cg.resolve_call_site_full(site)
        .resolved
        .iter()
        .filter_map(|resolved| cg.method_owners.get(resolved.target).cloned())
        .collect()
}

fn normalized_vec_map<K, V>(map: &BTreeMap<K, Vec<V>>) -> BTreeMap<K, BTreeSet<V>>
where
    K: Clone + Ord,
    V: Clone + Ord,
{
    map.iter()
        .map(|(key, values)| (key.clone(), values.iter().cloned().collect()))
        .collect()
}

fn assert_incremental_matches_full(
    initial: &BTreeMap<String, ParsedFile>,
    edited: &BTreeMap<String, ParsedFile>,
    changed_path: &str,
) -> CallGraph {
    let initial_graph = CallGraph::build(initial);
    let changed = BTreeSet::from([changed_path.to_string()]);
    let incremental = CodePropertyGraph::build_incremental(
        initial_graph,
        DataFlowGraph::build(&BTreeMap::new()),
        &changed,
        edited,
        None,
    )
    .call_graph;
    let full = CallGraph::build(edited);
    assert_eq!(
        normalized_vec_map(&incremental.functions),
        normalized_vec_map(&full.functions)
    );
    assert_eq!(incremental.calls, full.calls);
    assert_eq!(
        normalized_vec_map(&incremental.callers),
        normalized_vec_map(&full.callers)
    );
    assert_eq!(
        normalized_vec_map(&incremental.methods),
        normalized_vec_map(&full.methods)
    );
    assert_eq!(incremental.method_owners, full.method_owners);
    assert_eq!(
        normalized_vec_map(&incremental.interface_impls),
        normalized_vec_map(&full.interface_impls)
    );
    assert_eq!(incremental.method_arity, full.method_arity);
    assert_eq!(incremental.go_file_profiles, full.go_file_profiles);
    assert_eq!(incremental.go_field_types, full.go_field_types);
    assert_eq!(
        incremental.go_known_struct_identities,
        full.go_known_struct_identities
    );
    assert_eq!(incremental.go_func_typed_fields, full.go_func_typed_fields);
    assert_eq!(
        incremental.go_interface_declarations,
        full.go_interface_declarations
    );
    assert_eq!(
        incremental.go_method_declarations,
        full.go_method_declarations
    );
    assert_eq!(incremental.go_registrations, full.go_registrations);
    assert_eq!(
        incremental.go_embedded_interface_methods,
        full.go_embedded_interface_methods
    );
    assert_eq!(
        incremental.go_interface_live_types,
        full.go_interface_live_types
    );
    assert_eq!(
        incremental.go_owner_identity_profile_conflict,
        full.go_owner_identity_profile_conflict
    );
    assert_eq!(
        incremental.go_owner_identity_partition,
        full.go_owner_identity_partition
    );
    assert_eq!(dial_owners(&incremental), dial_owners(&full));
    incremental
}

#[test]
fn incremental_declaring_file_edit_matches_full_partition_rebuild() {
    let initial = parse_go(&[
        (
            "pkg/a_linux.go",
            "//go:build linux\n\npackage foo\ntype T struct { f Conn }\ntype Conn struct{}\nfunc (Conn) Dial() {}\n",
        ),
        (
            "pkg/z_windows.go",
            "//go:build windows\n\npackage foo\ntype T struct { f Mock }\ntype Mock struct{}\nfunc (Mock) Dial() {}\n",
        ),
        (
            "pkg/use.go",
            "//go:build linux\n\npackage foo\nfunc invoke(t T) { t.f.Dial() }\n",
        ),
    ]);
    let mut edited = initial.clone();
    edited.insert(
        "pkg/a_linux.go".to_string(),
        ParsedFile::parse(
            "pkg/a_linux.go",
            "//go:build linux\n\npackage foo\ntype T struct { f NewConn }\ntype NewConn struct{}\nfunc (NewConn) Dial() {}\n",
            Language::Go,
        )
        .expect("parse declaring edit"),
    );

    let incremental = assert_incremental_matches_full(&initial, &edited, "pkg/a_linux.go");
    assert_eq!(
        dial_owners(&incremental),
        BTreeSet::from(["NewConn".to_string()])
    );
}

#[test]
fn incremental_consumer_profile_edit_matches_full_partition_rebuild() {
    let initial = parse_go(&[
        (
            "pkg/a_linux.go",
            "//go:build linux\n\npackage foo\ntype T struct { f Conn }\ntype Conn struct{}\nfunc (Conn) Dial() {}\n",
        ),
        (
            "pkg/z_windows.go",
            "//go:build windows\n\npackage foo\ntype T struct { f Mock }\ntype Mock struct{}\nfunc (Mock) Dial() {}\n",
        ),
        (
            "pkg/use.go",
            "//go:build linux\n\npackage foo\nfunc invoke(t T) { t.f.Dial() }\n",
        ),
    ]);
    let mut edited = initial.clone();
    edited.insert(
        "pkg/use.go".to_string(),
        ParsedFile::parse(
            "pkg/use.go",
            "//go:build windows\n\npackage foo\nfunc invoke(t T) { t.f.Dial() }\n",
            Language::Go,
        )
        .expect("parse consumer edit"),
    );

    let incremental = assert_incremental_matches_full(&initial, &edited, "pkg/use.go");
    assert_eq!(
        dial_owners(&incremental),
        BTreeSet::from(["Mock".to_string()])
    );
}

fn build_s4_own_method_partition(caller_path: &str, caller_header: &str) -> CallGraph {
    let caller = format!("{caller_header}package foo\nfunc invoke(h Holder) {{ h.Act() }}\n");
    build_go(&[
        (
            "pkg/a_linux.go",
            "//go:build linux\n\npackage foo\ntype Holder struct { Doer }\n",
        ),
        (
            "pkg/z_windows.go",
            "//go:build windows\n\npackage foo\ntype Holder struct { Doer }\nfunc (Holder) Act() {}\n",
        ),
        (
            "pkg/interface.go",
            "package foo\ntype Doer interface { Act() }\ntype Impl struct{}\nfunc (Impl) Act() {}\n",
        ),
        (caller_path, &caller),
    ])
}

fn act_owners(cg: &CallGraph) -> BTreeSet<String> {
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|site| site.caller.name == "invoke" && site.callee_name == "Act")
        .expect("invoke Act site");
    cg.resolve_call_site_full(site)
        .resolved
        .iter()
        .filter_map(|resolved| cg.method_owners.get(resolved.target).cloned())
        .collect()
}

#[test]
fn s4_own_method_presence_is_partitioned_before_direct_owner_lookup() {
    let linux = build_s4_own_method_partition("pkg/use_linux.go", "//go:build linux\n\n");
    assert_eq!(act_owners(&linux), BTreeSet::from(["Impl".to_string()]));

    let windows = build_s4_own_method_partition("pkg/use_windows.go", "//go:build windows\n\n");
    assert_eq!(act_owners(&windows), BTreeSet::from(["Holder".to_string()]));

    let unconstrained = build_s4_own_method_partition("pkg/use.go", "");
    assert!(act_owners(&unconstrained).is_empty());
}

#[test]
fn qualified_owner_resolves_across_package_directories() {
    let cg = build_go(&[
        (
            "lib/defs.go",
            "package lib\ntype T struct { F Conn }\ntype Conn struct{}\nfunc (Conn) Dial() {}\n",
        ),
        (
            "app/use.go",
            "package app\nimport l \"example/lib\"\nfunc invoke(t l.T) { t.F.Dial() }\n",
        ),
    ]);

    assert_eq!(dial_owners(&cg), BTreeSet::from(["Conn".to_string()]));
}

fn manifest_owners(cg: &CallGraph, method: &str) -> BTreeSet<String> {
    prism::navigation::queries::interface_dispatch_manifest(cg)["sites"]
        .as_array()
        .expect("manifest sites")
        .iter()
        .find(|site| site["method"] == method)
        .expect("method manifest site")["implementers"]
        .as_array()
        .expect("manifest implementers")
        .iter()
        .map(|owner| owner.as_str().expect("owner string").to_string())
        .collect()
}

#[test]
fn s4_clause_partition_is_order_independent_and_manifest_matches_resolver() {
    for (prod_path, test_path) in [
        ("pkg/a_prod.go", "pkg/z_external_test.go"),
        ("pkg/z_prod.go", "pkg/a_external_test.go"),
    ] {
        let cg = build_go(&[
            (
                prod_path,
                "package foo\ntype Doer interface { Act() }\ntype Holder struct { Doer }\ntype Impl struct{}\nfunc (Impl) Act() {}\nfunc invoke(h Holder) { h.Act() }\n",
            ),
            (
                test_path,
                "package foo_test\ntype Doer interface { Test() }\n",
            ),
        ]);
        let resolved = act_owners(&cg);

        assert_eq!(resolved, BTreeSet::from(["Impl".to_string()]));
        assert_eq!(manifest_owners(&cg, "Act"), resolved);
    }
}

#[test]
fn s4_unknown_embedded_interface_never_mints_exact() {
    let cg = build_go(&[(
        "pkg/main.go",
        "package foo\ntype Unknown interface { Missing; Act() }\ntype Holder struct { Unknown }\ntype Impl struct{}\nfunc (Impl) Act() {}\nfunc invoke(h Holder) { h.Act() }\n",
    )]);

    assert!(act_owners(&cg).is_empty());
    assert!(manifest_owners(&cg, "Act").is_empty());
}
