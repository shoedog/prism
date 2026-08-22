use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
use prism::go_build_profile::unconstrained_profile;
use prism::go_owner_partition::select_embedded_interface_route;
use prism::languages::Language;
use prism::resolution::{resolve_go_owner_identity, GoOwnerIdentity, ResolutionConfidence};
use prism::type_providers::go::GoTypeProvider;
use std::collections::{BTreeMap, BTreeSet};

fn profile(package_clause: &str, is_test_file: bool) -> prism::go_build_profile::GoBuildProfile {
    let mut profile = unconstrained_profile();
    profile.package_clause = package_clause.to_string();
    profile.is_test_file = is_test_file;
    profile
}

fn go_provider(files: &[(&str, &str)]) -> GoTypeProvider {
    let parsed = go_files(files);
    GoTypeProvider::from_parsed_files(&parsed)
}

fn go_files(files: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
    files
        .iter()
        .map(|(path, source)| {
            (
                (*path).to_string(),
                ParsedFile::parse(path, source, Language::Go).expect("parse Go fixture"),
            )
        })
        .collect()
}

#[test]
fn go_owner_identity_records_bare_callers_package_clause() {
    let imports = BTreeMap::new();
    let package_basenames: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut profiles = BTreeMap::new();
    profiles.insert(
        "pkg/blackbox_test.go".to_string(),
        profile("foo_test", true),
    );

    let owner = resolve_go_owner_identity(
        "T",
        "pkg/blackbox_test.go",
        &imports,
        &package_basenames,
        &profiles,
    )
    .expect("bare owner");
    let wire = serde_json::to_value(owner).expect("serialize owner");

    assert_eq!(wire["package_clause"], "foo_test");
}

#[test]
fn qualified_owner_chooses_the_single_ordinary_clause() {
    let imports = BTreeMap::from([(
        "pkg/blackbox_test.go".to_string(),
        BTreeMap::from([("foo".to_string(), "example/foo".to_string())]),
    )]);
    let package_basenames =
        BTreeMap::from([("foo".to_string(), BTreeSet::from(["pkg".to_string()]))]);
    let profiles = BTreeMap::from([
        (
            "pkg/blackbox_test.go".to_string(),
            profile("foo_test", true),
        ),
        ("pkg/prod.go".to_string(), profile("foo", false)),
        ("pkg/internal_test.go".to_string(), profile("foo", true)),
        (
            "pkg/external_test.go".to_string(),
            profile("foo_test", true),
        ),
    ]);

    let owner = resolve_go_owner_identity(
        "foo.T",
        "pkg/blackbox_test.go",
        &imports,
        &package_basenames,
        &profiles,
    )
    .expect("qualified ordinary owner");

    assert_eq!(owner.package_dir, "pkg");
    assert_eq!(owner.package_clause, "foo");
    assert_eq!(owner.name, "T");
}

#[test]
fn owner_identity_fails_closed_without_one_proven_clause() {
    let imports = BTreeMap::from([(
        "caller/main.go".to_string(),
        BTreeMap::from([("foo".to_string(), "example/foo".to_string())]),
    )]);
    let package_basenames =
        BTreeMap::from([("foo".to_string(), BTreeSet::from(["pkg".to_string()]))]);
    let profiles = BTreeMap::from([
        ("caller/main.go".to_string(), profile("", false)),
        ("pkg/a.go".to_string(), profile("foo", false)),
        ("pkg/b.go".to_string(), profile("bar", false)),
    ]);

    assert!(resolve_go_owner_identity(
        "T",
        "caller/main.go",
        &imports,
        &package_basenames,
        &profiles,
    )
    .is_none());
    assert!(resolve_go_owner_identity(
        "foo.T",
        "caller/main.go",
        &imports,
        &package_basenames,
        &profiles,
    )
    .is_none());
}

#[test]
fn provider_preserves_both_build_partition_field_declarations() {
    let provider = go_provider(&[
        (
            "pkg/a_linux.go",
            "package foo\ntype T struct { f LinuxConn }\n",
        ),
        (
            "pkg/z_windows.go",
            "package foo\ntype T struct { f WindowsConn }\n",
        ),
    ]);
    let owner = GoOwnerIdentity {
        package_dir: "pkg".to_string(),
        package_clause: "foo".to_string(),
        name: "T".to_string(),
    };
    let field_types: BTreeSet<String> = provider
        .go_struct_declarations()
        .into_iter()
        .filter(|(candidate, _)| candidate == &owner)
        .flat_map(|(_, declarations)| declarations)
        .filter_map(|declaration| declaration.fields.get("f").cloned())
        .collect();

    assert_eq!(
        field_types,
        BTreeSet::from(["LinuxConn".to_string(), "WindowsConn".to_string()])
    );
}

#[test]
fn provider_s4_route_does_not_cross_external_test_clause() {
    let provider = go_provider(&[
        (
            "pkg/a_prod.go",
            "package foo\ntype Doer interface { Prod() }\ntype Holder struct { Doer }\n",
        ),
        (
            "pkg/z_external_test.go",
            "package foo_test\ntype Doer interface { Test() }\n",
        ),
    ]);
    let owner = GoOwnerIdentity {
        package_dir: "pkg".to_string(),
        package_clause: "foo".to_string(),
        name: "Holder".to_string(),
    };
    let routes = provider.embedded_interface_method_routes();
    let methods = routes.get(&owner).expect("ordinary Holder routes");

    assert_eq!(methods.get("Prod"), Some(&"Doer".to_string()));
    assert!(!methods.contains_key("Test"));
}

#[test]
fn s4_linux_partition_selects_linux_interface_declaration() {
    let files = go_files(&[
        (
            "pkg/a_linux.go",
            "//go:build linux\n\npackage foo\ntype Doer interface { Prod() }\n",
        ),
        (
            "pkg/holder.go",
            "package foo\ntype Holder struct { Doer }\n",
        ),
        (
            "pkg/z_windows.go",
            "//go:build windows\n\npackage foo\ntype Doer interface { Test() }\n",
        ),
        (
            "pkg/use_linux.go",
            "//go:build linux\n\npackage foo\nfunc use(h Holder) { h.Prod() }\n",
        ),
    ]);
    let cg = CallGraph::build(&files);
    let owner = GoOwnerIdentity {
        package_dir: "pkg".to_string(),
        package_clause: "foo".to_string(),
        name: "Holder".to_string(),
    };
    let route = select_embedded_interface_route(
        &owner,
        "pkg/use_linux.go",
        "Holder",
        "Prod",
        &cg.go_field_types,
        &cg.go_interface_declarations,
        &cg.go_method_declarations,
        &cg.go_file_profiles,
    );

    assert_eq!(route.value, Some("Doer".to_string()));
}

fn build_partition_field_fixture(caller_path: &str, caller_header: &str) -> CallGraph {
    let caller = format!("{caller_header}package foo\nfunc use(t T) {{ t.f.Dial() }}\n");
    let files = go_files(&[
        (
            "pkg/a_linux.go",
            "//go:build linux\n\npackage foo\ntype T struct { f LinuxConn }\ntype LinuxConn struct{}\nfunc (LinuxConn) Dial() {}\n",
        ),
        (
            "pkg/z_windows.go",
            "//go:build windows\n\npackage foo\ntype T struct { f WindowsConn }\ntype WindowsConn struct{}\nfunc (WindowsConn) Dial() {}\n",
        ),
        (caller_path, &caller),
    ]);
    CallGraph::build(&files)
}

fn dial_outcome(cg: &CallGraph) -> prism::resolution::ResolutionOutcome<'_> {
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|site| site.callee_name == "Dial")
        .expect("Dial call site");
    cg.resolve_call_site_full(site)
}

#[test]
fn s2_linux_caller_selects_linux_field_declaration() {
    let cg = build_partition_field_fixture("pkg/use_linux.go", "//go:build linux\n\n");
    let outcome = dial_outcome(&cg);

    assert_eq!(outcome.resolved.len(), 1);
    assert_eq!(outcome.resolved[0].target.file, "pkg/a_linux.go");
}

#[test]
fn s2_unconstrained_caller_drops_conflicting_visible_field_declarations() {
    let cg = build_partition_field_fixture("pkg/use.go", "");
    let outcome = dial_outcome(&cg);

    assert!(outcome.resolved.is_empty());
}

#[test]
fn s2_qualified_owner_from_external_test_uses_ordinary_clause_in_both_orders() {
    for (prod_path, test_path) in [
        ("foo/a_prod.go", "foo/z_external_test.go"),
        ("foo/z_prod.go", "foo/a_external_test.go"),
    ] {
        let files = go_files(&[
            (
                prod_path,
                "package foo\ntype T struct { f Conn }\ntype Conn struct{}\nfunc (Conn) Dial() {}\n",
            ),
            (
                test_path,
                "package foo_test\ntype T struct { f Mock }\ntype Mock struct{}\nfunc (Mock) Dial() {}\n",
            ),
            (
                "foo/blackbox_test.go",
                "package foo_test\nimport foo \"example/foo\"\nfunc use(t foo.T) { t.f.Dial() }\n",
            ),
        ]);
        let cg = CallGraph::build(&files);
        let outcome = dial_outcome(&cg);

        assert_eq!(outcome.resolved.len(), 1, "order case {prod_path}");
        assert_eq!(outcome.resolved[0].target.file, prod_path);
    }
}

#[test]
fn s2_same_value_duplicate_visible_declarations_remain_exact() {
    let files = go_files(&[
        (
            "pkg/a_linux.go",
            "//go:build linux\n\npackage foo\ntype T struct { f Conn }\n",
        ),
        (
            "pkg/conn.go",
            "package foo\ntype Conn struct{}\nfunc (Conn) Dial() {}\n",
        ),
        (
            "pkg/z_windows.go",
            "//go:build windows\n\npackage foo\ntype T struct { f Conn }\n",
        ),
        ("pkg/use.go", "package foo\nfunc use(t T) { t.f.Dial() }\n"),
    ]);
    let cg = CallGraph::build(&files);
    let outcome = dial_outcome(&cg);

    assert_eq!(outcome.resolved.len(), 1);
    assert_eq!(outcome.resolved[0].confidence, ResolutionConfidence::Exact);
}

#[test]
fn s2_unparsed_build_expression_never_mints_exact() {
    let files = go_files(&[
        (
            "pkg/a_bad.go",
            "//go:build (\n\npackage foo\ntype T struct { f Conn }\n",
        ),
        (
            "pkg/conn.go",
            "package foo\ntype Conn struct{}\nfunc (Conn) Dial() {}\n",
        ),
        ("pkg/use.go", "package foo\nfunc use(t T) { t.f.Dial() }\n"),
    ]);
    let cg = CallGraph::build(&files);
    let outcome = dial_outcome(&cg);

    assert!(outcome
        .resolved
        .iter()
        .all(|callee| callee.confidence != ResolutionConfidence::Exact));
}

fn build_s4_struct_partition_fixture(caller_path: &str, caller_header: &str) -> CallGraph {
    let caller = format!("{caller_header}package foo\nfunc use(h Holder) {{ h.Act() }}\n");
    let files = go_files(&[
        (
            "pkg/holder_linux.go",
            "//go:build linux\n\npackage foo\ntype Holder struct { Doer }\n",
        ),
        (
            "pkg/holder_windows.go",
            "//go:build windows\n\npackage foo\ntype Holder struct{}\n",
        ),
        (
            "pkg/interface.go",
            "package foo\ntype Doer interface { Act() }\ntype Impl struct{}\nfunc (Impl) Act() {}\nfunc live() { _ = Impl{} }\n",
        ),
        (caller_path, &caller),
    ]);
    CallGraph::build(&files)
}

fn act_outcome(cg: &CallGraph) -> prism::resolution::ResolutionOutcome<'_> {
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|site| site.callee_name == "Act")
        .expect("Act call site");
    cg.resolve_call_site_full(site)
}

fn manifest_act_implementers(cg: &CallGraph) -> BTreeSet<String> {
    prism::navigation::queries::interface_dispatch_manifest(cg)["sites"]
        .as_array()
        .expect("manifest sites")
        .iter()
        .find(|site| site["method"] == "Act")
        .expect("Act manifest site")["implementers"]
        .as_array()
        .expect("implementers")
        .iter()
        .map(|value| value.as_str().expect("implementer string").to_string())
        .collect()
}

#[test]
fn s4_resolver_and_manifest_share_recovered_and_blocked_partition_decisions() {
    let linux = build_s4_struct_partition_fixture("pkg/use_linux.go", "//go:build linux\n\n");
    let linux_outcome = act_outcome(&linux);
    let linux_resolved: BTreeSet<String> = linux_outcome
        .resolved
        .iter()
        .filter_map(|callee| linux.method_owners.get(callee.target).cloned())
        .collect();
    assert_eq!(linux_resolved, BTreeSet::from(["Impl".to_string()]));
    assert_eq!(manifest_act_implementers(&linux), linux_resolved);

    let unconstrained = build_s4_struct_partition_fixture("pkg/use.go", "");
    let unconstrained_outcome = act_outcome(&unconstrained);
    assert!(unconstrained_outcome.resolved.is_empty());
    assert_eq!(manifest_act_implementers(&unconstrained), BTreeSet::new());
}

fn build_p5_partition_fixture(caller_path: &str, caller_header: &str) -> CallGraph {
    let caller = format!("{caller_header}package foo\nfunc invoke(c Command) {{ c.Run() }}\n");
    let files = go_files(&[
        (
            "pkg/a_linux.go",
            "//go:build linux\n\npackage foo\ntype Command struct { Run func() }\nfunc linuxHandler() {}\nfunc setupLinux() { _ = Command{Run: linuxHandler} }\n",
        ),
        (
            "pkg/z_windows.go",
            "//go:build windows\n\npackage foo\ntype Command struct { Run func() }\nfunc windowsHandler() {}\nfunc setupWindows() { _ = Command{Run: windowsHandler} }\n",
        ),
        (caller_path, &caller),
    ]);
    CallGraph::build(&files)
}

fn run_targets(cg: &CallGraph) -> BTreeSet<String> {
    run_targets_in(cg, "invoke")
}

fn run_targets_in(cg: &CallGraph, caller_name: &str) -> BTreeSet<String> {
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|site| site.callee_name == "Run" && site.caller.name == caller_name)
        .expect("Run call site");
    cg.resolve_call_site_full(site)
        .resolved
        .iter()
        .map(|callee| callee.target.name.clone())
        .collect()
}

#[test]
fn p5_registration_targets_follow_the_invocation_partition() {
    let linux = build_p5_partition_fixture("pkg/use_linux.go", "//go:build linux\n\n");
    assert_eq!(
        run_targets(&linux),
        BTreeSet::from(["linuxHandler".to_string()])
    );

    let windows = build_p5_partition_fixture("pkg/use_windows.go", "//go:build windows\n\n");
    assert_eq!(
        run_targets(&windows),
        BTreeSet::from(["windowsHandler".to_string()])
    );

    let unconstrained = build_p5_partition_fixture("pkg/use.go", "");
    assert!(run_targets(&unconstrained).is_empty());
}

#[test]
fn p5_compatible_common_and_linux_registrations_are_additive() {
    let files = go_files(&[
        (
            "pkg/type.go",
            "package foo\ntype Command struct { Run func() }\nfunc commonHandler() {}\nfunc setupCommon() { _ = Command{Run: commonHandler} }\n",
        ),
        (
            "pkg/setup_linux.go",
            "//go:build linux\n\npackage foo\nfunc linuxHandler() {}\nfunc setupLinux() { _ = Command{Run: linuxHandler} }\n",
        ),
        (
            "pkg/use_linux.go",
            "//go:build linux\n\npackage foo\nfunc invoke(c Command) { c.Run() }\n",
        ),
    ]);
    let cg = CallGraph::build(&files);
    assert_eq!(
        cg.go_registrations
            .iter()
            .map(|record| record.target.name.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["commonHandler", "linuxHandler"])
    );

    assert_eq!(
        run_targets(&cg),
        BTreeSet::from(["commonHandler".to_string(), "linuxHandler".to_string()])
    );
}

#[test]
fn p5_func_vs_nonfunc_declarations_conflict_and_nonfunc_registration_is_skipped() {
    let files = go_files(&[
        (
            "pkg/a_linux.go",
            "//go:build linux\n\npackage foo\ntype Command struct { Run func() }\nfunc linuxHandler() {}\nfunc setupLinux() { _ = Command{Run: linuxHandler} }\n",
        ),
        (
            "pkg/z_windows.go",
            "//go:build windows\n\npackage foo\ntype Command struct { Run int }\nfunc windowsHandler() {}\nfunc setupWindows() { _ = Command{Run: windowsHandler} }\n",
        ),
        (
            "pkg/use.go",
            "package foo\nfunc invoke(c Command) { c.Run() }\n",
        ),
    ]);
    let cg = CallGraph::build(&files);

    assert!(run_targets(&cg).is_empty());
    assert!(cg
        .go_registrations
        .iter()
        .all(|record| record.target.name != "windowsHandler"));
}

#[test]
fn p5_clause_partition_is_order_independent_and_qualified_access_targets_ordinary() {
    for (prod_path, test_path) in [
        ("foo/a_prod.go", "foo/z_external_test.go"),
        ("foo/z_prod.go", "foo/a_external_test.go"),
    ] {
        let files = go_files(&[
            (
                prod_path,
                "package foo\ntype Command struct { Run func() }\nfunc prodHandler() {}\nfunc setupProd() { _ = Command{Run: prodHandler} }\nfunc invokeProd(c Command) { c.Run() }\n",
            ),
            (
                test_path,
                "package foo_test\nimport foo \"example/foo\"\ntype Command struct { Run int }\nfunc testHandler() {}\nfunc setupTest() { _ = Command{Run: testHandler} }\nfunc invokeBare(c Command) { c.Run() }\nfunc invokeQualified(c foo.Command) { c.Run() }\n",
            ),
        ]);
        let cg = CallGraph::build(&files);

        assert_eq!(
            run_targets_in(&cg, "invokeProd"),
            BTreeSet::from(["prodHandler".to_string()])
        );
        assert!(run_targets_in(&cg, "invokeBare").is_empty());
        assert_eq!(
            run_targets_in(&cg, "invokeQualified"),
            BTreeSet::from(["prodHandler".to_string()])
        );
        assert!(cg
            .go_registrations
            .iter()
            .all(|record| record.target.name != "testHandler"));
    }
}

#[test]
fn direct_method_on_named_non_struct_type_survives_partition_gate() {
    let cg = CallGraph::build(&go_files(&[(
        "pkg/main.go",
        "package foo\ntype Status int\nfunc (Status) Act() {}\nfunc invoke(s Status) { s.Act() }\n",
    )]));
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|site| site.callee_name == "Act")
        .expect("Act call site");

    assert_eq!(site.receiver_type.as_deref(), Some("Status"));
    let outcome = cg.resolve_call_site_full(site);
    assert_eq!(outcome.resolved.len(), 1);
    assert_eq!(outcome.resolved[0].target.name, "Act");
}
