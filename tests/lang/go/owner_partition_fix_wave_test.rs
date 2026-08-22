use prism::ast::ParsedFile;
use prism::call_graph::{CallGraph, ScopeGraphBuildInputs};
use prism::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

fn build_go(sources: &[(&str, &str)]) -> CallGraph {
    let files: BTreeMap<String, ParsedFile> = sources
        .iter()
        .map(|(path, source)| {
            (
                (*path).to_string(),
                ParsedFile::parse(path, source, Language::Go).expect("parse Go fixture"),
            )
        })
        .collect();
    CallGraph::build(&files)
}

fn build_go_with_module(sources: &[(&str, &str)], module_path: &str) -> CallGraph {
    build_go_with_modules(sources, &[("", module_path)])
}

fn build_go_with_modules(sources: &[(&str, &str)], modules: &[(&str, &str)]) -> CallGraph {
    let files: BTreeMap<String, ParsedFile> = sources
        .iter()
        .map(|(path, source)| {
            (
                (*path).to_string(),
                ParsedFile::parse(path, source, Language::Go).expect("parse Go fixture"),
            )
        })
        .collect();
    let repo = tempfile::tempdir().expect("temporary Go module root");
    for (directory, module_path) in modules {
        let module_root = repo.path().join(directory);
        std::fs::create_dir_all(&module_root).expect("create Go module fixture directory");
        std::fs::write(
            module_root.join("go.mod"),
            format!("module {module_path}\n\ngo 1.22\n"),
        )
        .expect("write go.mod fixture");
    }
    let mut inputs = ScopeGraphBuildInputs::from_files_convention(&files);
    inputs.repo_root = repo.path().to_path_buf();
    CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs))
}

fn resolved_method_owners(cg: &CallGraph, caller: &str, method: &str) -> BTreeSet<String> {
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|site| site.caller.name == caller && site.callee_name == method)
        .expect("method call site");
    cg.resolve_call_site_full(site)
        .resolved
        .iter()
        .filter_map(|resolved| cg.method_owners.get(resolved.target).cloned())
        .collect()
}

fn resolved_target_names(cg: &CallGraph, caller: &str, callee: &str) -> BTreeSet<String> {
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|site| site.caller.name == caller && site.callee_name == callee)
        .expect("call site");
    cg.resolve_call_site_full(site)
        .resolved
        .iter()
        .map(|resolved| resolved.target.name.clone())
        .collect()
}

fn manifest_owners(cg: &CallGraph, caller: &str, method: &str) -> BTreeSet<String> {
    prism::navigation::queries::interface_dispatch_manifest(cg)["sites"]
        .as_array()
        .expect("manifest sites")
        .iter()
        .find(|site| site["file"] == caller && site["method"] == method)
        .expect("manifest method site")["implementers"]
        .as_array()
        .expect("manifest implementers")
        .iter()
        .map(|owner| owner.as_str().expect("owner string").to_string())
        .collect()
}

#[test]
fn imported_s4_target_excludes_the_imported_packages_test_only_implementers() {
    let cg = build_go(&[
        (
            "lib/defs.go",
            "package lib\ntype Doer interface { Act() }\ntype Holder struct { Doer }\n",
        ),
        (
            "lib/internal_test.go",
            "package lib\ntype Impl struct{}\nfunc (Impl) Act() {}\n",
        ),
        (
            "app/app_test.go",
            "package app\nimport l \"example/lib\"\nfunc invoke(h l.Holder) { h.Act() }\n",
        ),
    ]);

    assert!(resolved_method_owners(&cg, "invoke", "Act").is_empty());
    assert!(manifest_owners(&cg, "app/app_test.go", "Act").is_empty());
}

#[test]
fn same_package_test_s4_target_keeps_its_own_test_only_implementers() {
    let cg = build_go(&[
        (
            "lib/defs.go",
            "package lib\ntype Doer interface { Act() }\ntype Holder struct { Doer }\n",
        ),
        (
            "lib/internal_test.go",
            "package lib\ntype Impl struct{}\nfunc (Impl) Act() {}\nfunc invoke(h Holder) { h.Act() }\n",
        ),
    ]);
    let expected = BTreeSet::from(["Impl".to_string()]);

    assert_eq!(resolved_method_owners(&cg, "invoke", "Act"), expected);
    assert_eq!(
        manifest_owners(&cg, "lib/internal_test.go", "Act"),
        expected
    );
}

#[test]
fn qualified_return_type_uses_the_imported_packages_ordinary_clause() {
    let cg = build_go(&[
        (
            "foo/prod.go",
            "package foo\ntype Prod struct{}\nfunc (Prod) Dial() {}\nfunc New() Prod { return Prod{} }\n",
        ),
        (
            "foo/external_test.go",
            "package foo_test\ntype Mock struct{}\nfunc (Mock) Dial() {}\nfunc New() Mock { return Mock{} }\n",
        ),
        (
            "foo/use_test.go",
            "package foo_test\nimport foo \"example/foo\"\nfunc invokeQualified() { x := foo.New(); x.Dial() }\nfunc invokeBare() { x := New(); x.Dial() }\n",
        ),
    ]);

    assert_eq!(
        resolved_method_owners(&cg, "invokeQualified", "Dial"),
        BTreeSet::from(["Prod".to_string()])
    );
    assert_eq!(
        resolved_method_owners(&cg, "invokeBare", "Dial"),
        BTreeSet::from(["Mock".to_string()])
    );
}

#[test]
fn return_type_partition_decisions_reach_call_stats() {
    let cg = build_go(&[
        (
            "pkg/new_linux.go",
            "//go:build linux\npackage pkg\ntype Linux struct{}\nfunc (Linux) Dial() {}\nfunc New() Linux { return Linux{} }\n",
        ),
        (
            "pkg/new_windows.go",
            "//go:build windows\npackage pkg\ntype Windows struct{}\nfunc (Windows) Dial() {}\nfunc New() Windows { return Windows{} }\n",
        ),
        (
            "pkg/use_linux.go",
            "//go:build linux\npackage pkg\nfunc invokeLinux() { x := New(); x.Dial() }\n",
        ),
        (
            "pkg/use.go",
            "package pkg\nfunc invokeUnconstrained() { x := New(); x.Dial() }\n",
        ),
    ]);
    let stats = prism::navigation::queries::call_stats(&cg);

    assert_eq!(
        resolved_method_owners(&cg, "invokeLinux", "Dial"),
        BTreeSet::from(["Linux".to_string()])
    );
    assert!(resolved_method_owners(&cg, "invokeUnconstrained", "Dial").is_empty());
    assert_eq!(stats["go_owner_identity_partition_recovered"], 1);
    assert_eq!(stats["go_owner_identity_partition_drop"], 1);
    assert_eq!(stats["go_owner_identity_partition_affected_sites"], 2);
}

#[test]
fn cross_package_p5_registrations_follow_the_invocation_partition() {
    let cg = build_go(&[
        (
            "lib/command_linux.go",
            "//go:build linux\npackage lib\ntype Command struct { Run func() }\n",
        ),
        (
            "lib/command_windows.go",
            "//go:build windows\npackage lib\ntype Command struct { Run func() }\n",
        ),
        (
            "app/register_linux.go",
            "//go:build linux\npackage app\nimport l \"example/lib\"\nfunc linuxHandler() {}\nfunc setupLinux() { _ = l.Command{Run: linuxHandler} }\n",
        ),
        (
            "app/register_windows.go",
            "//go:build windows\npackage app\nimport l \"example/lib\"\nfunc windowsHandler() {}\nfunc setupWindows() { _ = l.Command{Run: windowsHandler} }\n",
        ),
        (
            "app/use_linux.go",
            "//go:build linux\npackage app\nimport l \"example/lib\"\nfunc invokeLinux(c l.Command) { c.Run() }\n",
        ),
        (
            "app/use_windows.go",
            "//go:build windows\npackage app\nimport l \"example/lib\"\nfunc invokeWindows(c l.Command) { c.Run() }\n",
        ),
        (
            "app/use.go",
            "package app\nimport l \"example/lib\"\nfunc invokeUnconstrained(c l.Command) { c.Run() }\n",
        ),
    ]);

    assert_eq!(
        cg.go_registrations
            .iter()
            .map(|registration| registration.target.name.clone())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["linuxHandler".to_string(), "windowsHandler".to_string()])
    );

    assert_eq!(
        resolved_target_names(&cg, "invokeLinux", "Run"),
        BTreeSet::from(["linuxHandler".to_string()])
    );
    assert_eq!(
        resolved_target_names(&cg, "invokeWindows", "Run"),
        BTreeSet::from(["windowsHandler".to_string()])
    );
    assert!(resolved_target_names(&cg, "invokeUnconstrained", "Run").is_empty());
}

#[test]
fn cross_package_p5_compatible_registrations_are_additive() {
    let cg = build_go(&[
        (
            "lib/command.go",
            "package lib\ntype Command struct { Run func() }\n",
        ),
        (
            "app/register.go",
            "package app\nimport l \"example/lib\"\nfunc appHandler() {}\nfunc setupApp() { _ = l.Command{Run: appHandler} }\n",
        ),
        (
            "plugin/register.go",
            "package plugin\nimport l \"example/lib\"\nfunc pluginHandler() {}\nfunc setupPlugin() { _ = l.Command{Run: pluginHandler} }\n",
        ),
        (
            "client/use.go",
            "package client\nimport l \"example/lib\"\nfunc invoke(c l.Command) { c.Run() }\n",
        ),
    ]);

    assert_eq!(
        resolved_target_names(&cg, "invoke", "Run"),
        BTreeSet::from(["appHandler".to_string(), "pluginHandler".to_string()])
    );
}

#[test]
fn cross_package_p5_excludes_foreign_test_registrations() {
    let cg = build_go(&[
        (
            "lib/command.go",
            "package lib\ntype Command struct { Run func() }\n",
        ),
        (
            "app/register.go",
            "package app\nimport l \"example/lib\"\nfunc appHandler() {}\nfunc setupApp() { _ = l.Command{Run: appHandler} }\n",
        ),
        (
            "plugin/register_test.go",
            "package plugin\nimport l \"example/lib\"\nfunc testHandler() {}\nfunc setupTest() { _ = l.Command{Run: testHandler} }\n",
        ),
        (
            "client/use_test.go",
            "package client\nimport l \"example/lib\"\nfunc invoke(c l.Command) { c.Run() }\n",
        ),
    ]);

    assert_eq!(
        resolved_target_names(&cg, "invoke", "Run"),
        BTreeSet::from(["appHandler".to_string()])
    );
}

fn resolved_target_files(cg: &CallGraph, caller: &str, method: &str) -> BTreeSet<String> {
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|site| site.caller.name == caller && site.callee_name == method)
        .expect("method call site");
    cg.resolve_call_site_full(site)
        .resolved
        .iter()
        .map(|resolved| resolved.target.file.clone())
        .collect()
}

#[test]
fn qualified_return_recovery_preserves_the_return_types_package_owner() {
    let cg = build_go(&[
        (
            "factory/factory.go",
            "package factory\ntype Widget struct{}\nfunc (Widget) Use() {}\nfunc New() Widget { return Widget{} }\n",
        ),
        (
            "app/local.go",
            "package app\ntype Widget struct{}\nfunc (Widget) Use() {}\nfunc Make() Widget { return Widget{} }\n",
        ),
        (
            "app/use.go",
            "package app\nimport factory \"example/factory\"\nfunc invokeFactory() { w := factory.New(); w.Use() }\nfunc invokeLocal() { w := Make(); w.Use() }\n",
        ),
    ]);

    let factory_site = cg
        .calls
        .values()
        .flatten()
        .find(|site| site.caller.name == "invokeFactory" && site.callee_name == "Use")
        .expect("qualified-return method site");
    let factory_owner = factory_site
        .receiver_owner_identity
        .as_ref()
        .expect("qualified-return owner identity");
    assert_eq!(factory_owner.package_dir, "factory");
    assert_eq!(factory_owner.package_clause, "factory");
    assert_eq!(factory_owner.name, "Widget");
    let local_site = cg
        .calls
        .values()
        .flatten()
        .find(|site| site.caller.name == "invokeLocal" && site.callee_name == "Use")
        .expect("bare-return method site");
    let local_owner = local_site
        .receiver_owner_identity
        .as_ref()
        .expect("bare-return owner identity");
    assert_eq!(local_owner.package_dir, "app");
    assert_eq!(local_owner.package_clause, "app");
    assert_eq!(local_owner.name, "Widget");

    assert_eq!(
        resolved_target_files(&cg, "invokeFactory", "Use"),
        BTreeSet::from(["factory/factory.go".to_string()])
    );
    let mut stale_factory_site = factory_site.clone();
    stale_factory_site.receiver_owner_identity = None;
    assert!(cg
        .resolve_call_site_full(&stale_factory_site)
        .resolved
        .is_empty());
    let local_outcome = cg.resolve_call_site_full(local_site);
    let local_files: BTreeSet<_> = local_outcome
        .resolved
        .iter()
        .map(|resolved| resolved.target.file.clone())
        .collect();
    assert_eq!(
        local_files,
        BTreeSet::from(["app/local.go".to_string()]),
        "drop={:?}; telemetry={:?}",
        local_outcome.drop,
        local_outcome.telemetry
    );
}

#[test]
fn qualified_returned_interface_uses_its_proven_package_identity() {
    let cg = build_go(&[
        (
            "factory/factory.go",
            "package factory\ntype sealed interface { seal(); Act() }\ntype Local struct{}\nfunc (Local) seal() {}\nfunc (Local) Act() {}\nfunc New() sealed { return Local{} }\n",
        ),
        (
            "other/other.go",
            "package other\ntype sealed interface { seal(); Act() }\ntype Impl struct{}\nfunc (Impl) seal() {}\nfunc (Impl) Act() {}\n",
        ),
        (
            "app/use.go",
            "package app\nimport factory \"example/factory\"\nfunc invoke() { value := factory.New(); value.Act() }\n",
        ),
    ]);
    let expected = BTreeSet::from(["Local".to_string()]);
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|site| site.caller.name == "invoke" && site.callee_name == "Act")
        .expect("qualified-return interface site");
    let outcome = cg.resolve_call_site_full(site);
    let owners: BTreeSet<_> = outcome
        .resolved
        .iter()
        .filter_map(|resolved| cg.method_owners.get(resolved.target).cloned())
        .collect();

    assert_eq!(
        owners, expected,
        "site={site:?}; outcome={outcome:?}; interface_impls={:?}",
        cg.interface_impls
    );
    assert_eq!(manifest_owners(&cg, "app/use.go", "Act"), expected);
}

#[test]
fn qualified_returned_interface_filters_same_named_signature_decoys() {
    let cg = build_go(&[
        (
            "factory/factory.go",
            "package factory\ntype Doer interface { Act(string) }\ntype Local struct{}\nfunc (Local) Act(string) {}\nfunc New() Doer { return Local{} }\n",
        ),
        (
            "other/other.go",
            "package other\ntype Doer interface { Act(int) }\ntype Impl struct{}\nfunc (Impl) Act(int) {}\n",
        ),
        (
            "app/use.go",
            "package app\nimport factory \"example/factory\"\nfunc invoke() { value := factory.New(); value.Act(\"ok\") }\n",
        ),
    ]);
    let expected = BTreeSet::from(["Local".to_string()]);

    assert_eq!(resolved_method_owners(&cg, "invoke", "Act"), expected);
    assert_eq!(manifest_owners(&cg, "app/use.go", "Act"), expected);
}

#[test]
fn qualified_return_with_unbound_declaring_package_fails_closed() {
    let cg = build_go(&[
        (
            "factory/factory.go",
            "package factory\nimport missing \"example/missing\"\nfunc New() missing.Widget { panic(\"unreachable\") }\n",
        ),
        (
            "app/local.go",
            "package app\ntype Widget struct{}\nfunc (Widget) Use() {}\n",
        ),
        (
            "app/use.go",
            "package app\nimport factory \"example/factory\"\nfunc invoke() { w := factory.New(); w.Use() }\n",
        ),
    ]);
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|site| site.caller.name == "invoke" && site.callee_name == "Use")
        .expect("qualified-return method site");

    assert!(site.receiver_owner_identity.is_none());
    assert!(site.receiver_materialized);
    assert!(cg.resolve_call_site_full(site).resolved.is_empty());
}

#[test]
fn s4_unexported_methods_require_the_interface_package_owner() {
    let cg = build_go(&[
        (
            "lib/defs.go",
            "package lib\ntype sealed interface { seal() }\ntype Holder struct { sealed }\ntype Local struct{}\nfunc (Local) seal() {}\nfunc invoke(h Holder) { h.seal() }\n",
        ),
        (
            "other/impl.go",
            "package other\ntype Impl struct{}\nfunc (Impl) seal() {}\n",
        ),
    ]);
    let expected = BTreeSet::from(["Local".to_string()]);

    assert_eq!(resolved_method_owners(&cg, "invoke", "seal"), expected);
    assert_eq!(manifest_owners(&cg, "lib/defs.go", "seal"), expected);
}

#[test]
fn s4_qualified_parameter_types_never_match_by_bare_name() {
    let cg = build_go(&[
        ("left/id.go", "package left\ntype ID struct{}\n"),
        ("right/id.go", "package right\ntype ID struct{}\n"),
        (
            "lib/defs.go",
            "package lib\nimport left \"example/left\"\ntype Doer interface { Act(left.ID) }\ntype Holder struct { Doer }\nfunc invoke(h Holder, id left.ID) { h.Act(id) }\n",
        ),
        (
            "other/impl.go",
            "package other\nimport right \"example/right\"\ntype Impl struct{}\nfunc (Impl) Act(right.ID) {}\n",
        ),
    ]);

    assert!(resolved_method_owners(&cg, "invoke", "Act").is_empty());
    assert!(manifest_owners(&cg, "lib/defs.go", "Act").is_empty());
}

#[test]
fn s4_qualified_context_signature_matches_same_import_path_across_aliases() {
    let cg = build_go(&[
        (
            "lib/defs.go",
            "package lib\nimport ifacectx \"context\"\ntype Doer interface { Act(ifacectx.Context) }\ntype Holder struct { Doer }\nfunc invoke(h Holder, ctx ifacectx.Context) { h.Act(ctx) }\n",
        ),
        (
            "other/impl.go",
            "package other\nimport implctx \"context\"\ntype Impl struct{}\nfunc (Impl) Act(implctx.Context) {}\n",
        ),
    ]);
    let expected = BTreeSet::from(["Impl".to_string()]);

    assert_eq!(resolved_method_owners(&cg, "invoke", "Act"), expected);
    assert_eq!(manifest_owners(&cg, "lib/defs.go", "Act"), expected);
}

#[test]
fn s4_module_major_version_default_name_matches_explicit_alias() {
    let cg = build_go(&[
        (
            "lib/defs.go",
            "package lib\nimport \"example/widget/v2\"\ntype Doer interface { Act(widget.Context) }\ntype Holder struct { Doer }\nfunc invoke(h Holder, ctx widget.Context) { h.Act(ctx) }\n",
        ),
        (
            "other/impl.go",
            "package other\nimport implwidget \"example/widget/v2\"\ntype Impl struct{}\nfunc (Impl) Act(implwidget.Context) {}\n",
        ),
    ]);
    let expected = BTreeSet::from(["Impl".to_string()]);

    assert_eq!(resolved_method_owners(&cg, "invoke", "Act"), expected);
    assert_eq!(manifest_owners(&cg, "lib/defs.go", "Act"), expected);
}

#[test]
fn s4_local_module_type_matches_its_imported_identity_without_name_only_fallback() {
    let cg = build_go_with_module(
        &[
            (
                "context.go",
                "package caddy\ntype Context struct{}\ntype Provisioner interface { Provision(Context) }\ntype Holder struct { Provisioner }\nfunc invoke(h Holder, ctx Context) { h.Provision(ctx) }\n",
            ),
            (
                "direct.go",
                "package caddy\nfunc invokeDirect(p Provisioner, ctx Context) { p.Provision(ctx) }\n",
            ),
            (
                "good/impl.go",
                "package good\nimport caddy \"example.com/caddy/v2\"\ntype Impl struct{}\nfunc (Impl) Provision(caddy.Context) {}\n",
            ),
            (
                "other/context.go",
                "package other\ntype Context struct{}\n",
            ),
            (
                "bad/impl.go",
                "package bad\nimport other \"example.com/caddy/v2/other\"\ntype Decoy struct{}\nfunc (Decoy) Provision(other.Context) {}\n",
            ),
        ],
        "example.com/caddy/v2",
    );
    let expected = BTreeSet::from(["Impl".to_string()]);

    assert_eq!(resolved_method_owners(&cg, "invoke", "Provision"), expected);
    assert_eq!(manifest_owners(&cg, "context.go", "Provision"), expected);
    assert_eq!(
        resolved_method_owners(&cg, "invokeDirect", "Provision"),
        expected
    );
    assert_eq!(manifest_owners(&cg, "direct.go", "Provision"), expected);
}

#[test]
fn s4_nested_module_mixed_bare_and_qualified_types_fail_closed() {
    let cg = build_go_with_modules(
        &[
            (
                "nested/context.go",
                "package nested\ntype Context struct{}\ntype Doer interface { Act(Context) }\ntype Holder struct { Doer }\nfunc invoke(h Holder, ctx Context) { h.Act(ctx) }\n",
            ),
            (
                "good/impl.go",
                "package good\nimport nested \"example.com/nested\"\ntype Impl struct{}\nfunc (Impl) Act(nested.Context) {}\n",
            ),
            (
                "bad/impl.go",
                "package bad\nimport wrong \"example.com/root/nested\"\ntype Decoy struct{}\nfunc (Decoy) Act(wrong.Context) {}\n",
            ),
        ],
        &[("", "example.com/root"), ("nested", "example.com/nested")],
    );
    assert!(resolved_method_owners(&cg, "invoke", "Act").is_empty());
    assert!(manifest_owners(&cg, "nested/context.go", "Act").is_empty());
}

#[test]
fn s4_nested_module_bare_interface_rejects_root_local_same_name() {
    let cg = build_go_with_modules(
        &[
            (
                "nested/context.go",
                "package nested\ntype Context struct{}\ntype Doer interface { Act(Context) }\ntype Holder struct { Doer }\nfunc invoke(h Holder, ctx Context) { h.Act(ctx) }\n",
            ),
            (
                "good/impl.go",
                "package good\nimport nested \"example.com/nested\"\ntype Good struct{}\nfunc (Good) Act(nested.Context) {}\n",
            ),
            (
                "bad/impl.go",
                "package bad\ntype Context struct{}\ntype Bad struct{}\nfunc (Bad) Act(Context) {}\n",
            ),
        ],
        &[("", "example.com/root"), ("nested", "example.com/nested")],
    );

    assert!(resolved_method_owners(&cg, "invoke", "Act").is_empty());
    assert!(manifest_owners(&cg, "nested/context.go", "Act").is_empty());
}

#[test]
fn s4_root_local_interface_rejects_nested_bare_and_keeps_qualified_same_path() {
    let root_interface = (
        "context.go",
        "package root\ntype Context struct{}\ntype Doer interface { Act(Context) }\ntype Holder struct { Doer }\nfunc invoke(h Holder, ctx Context) { h.Act(ctx) }\n",
    );
    let nested_implementer = (
        "nested/impl.go",
        "package nested\ntype Context struct{}\ntype Impl struct{}\nfunc (Impl) Act(Context) {}\n",
    );
    let modules = &[("", "example.com/root"), ("nested", "example.com/nested")];
    let nested_only = build_go_with_modules(&[root_interface, nested_implementer], modules);

    assert!(resolved_method_owners(&nested_only, "invoke", "Act").is_empty());
    assert!(manifest_owners(&nested_only, "context.go", "Act").is_empty());

    let with_qualified_root_implementer = build_go_with_modules(
        &[
            root_interface,
            nested_implementer,
            (
                "good/impl.go",
                "package good\nimport root \"example.com/root\"\ntype Impl2 struct{}\nfunc (Impl2) Act(root.Context) {}\n",
            ),
        ],
        modules,
    );
    let expected = BTreeSet::from(["Impl2".to_string()]);

    assert_eq!(
        resolved_method_owners(&with_qualified_root_implementer, "invoke", "Act"),
        expected
    );
    assert_eq!(
        manifest_owners(&with_qualified_root_implementer, "context.go", "Act"),
        expected
    );
}

#[test]
fn s4_unqualified_named_types_keep_the_existing_bare_name_rule() {
    let cg = build_go_with_module(
        &[
            (
                "lib/defs.go",
                "package lib\ntype ID struct{}\ntype Doer interface { Act(ID) }\ntype Holder struct { Doer }\nfunc invoke(h Holder, id ID) { h.Act(id) }\n",
            ),
            (
                "other/impl.go",
                "package other\ntype ID struct{}\ntype Impl struct{}\nfunc (Impl) Act(ID) {}\n",
            ),
        ],
        "example.com/root",
    );
    let expected = BTreeSet::from(["Impl".to_string()]);

    assert_eq!(resolved_method_owners(&cg, "invoke", "Act"), expected);
    assert_eq!(manifest_owners(&cg, "lib/defs.go", "Act"), expected);
}

#[test]
fn s4_exported_primitive_signature_still_matches_across_packages() {
    let cg = build_go(&[
        (
            "lib/defs.go",
            "package lib\ntype Doer interface { Act(string) }\ntype Holder struct { Doer }\nfunc invoke(h Holder) { h.Act(\"ok\") }\n",
        ),
        (
            "other/impl.go",
            "package other\ntype Impl struct{}\nfunc (Impl) Act(string) {}\n",
        ),
    ]);
    let expected = BTreeSet::from(["Impl".to_string()]);

    assert_eq!(resolved_method_owners(&cg, "invoke", "Act"), expected);
    assert_eq!(manifest_owners(&cg, "lib/defs.go", "Act"), expected);
}

#[test]
fn qualified_return_nested_field_preserves_each_package_owner() {
    let cg = build_go(&[
        (
            "factory/factory.go",
            "package factory\ntype Gadget struct{}\nfunc (Gadget) Use() {}\ntype Widget struct { Part Gadget }\nfunc New() Widget { return Widget{} }\n",
        ),
        (
            "app/local.go",
            "package app\ntype Gadget struct{}\nfunc (Gadget) Use() {}\ntype Widget struct { Part Gadget }\n",
        ),
        (
            "app/use.go",
            "package app\nimport factory \"example/factory\"\nfunc invoke() { w := factory.New(); w.Part.Use() }\n",
        ),
    ]);
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|site| site.caller.name == "invoke" && site.callee_name == "Use")
        .expect("nested qualified-return method site");
    let owner = site
        .receiver_owner_identity
        .as_ref()
        .expect("nested field owner identity");

    assert_eq!(owner.package_dir, "factory");
    assert_eq!(owner.package_clause, "factory");
    assert_eq!(owner.name, "Gadget");
    assert_eq!(
        resolved_target_files(&cg, "invoke", "Use"),
        BTreeSet::from(["factory/factory.go".to_string()])
    );
}

#[test]
fn direct_interface_identity_never_reintroduces_a_bare_signature_decoy() {
    let cg = build_go(&[
        (
            "factory/factory.go",
            "package factory\ntype Doer interface { Act(string) }\ntype Local struct{}\nfunc (Local) Act(string) {}\nfunc New() Doer { panic(\"unreachable\") }\n",
        ),
        (
            "other/other.go",
            "package other\ntype Doer interface { Act(int) }\ntype Impl struct{}\nfunc (Impl) Act(int) {}\nfunc keepLive() { _ = Impl{} }\n",
        ),
        (
            "app/use.go",
            "package app\nimport factory \"example/factory\"\nfunc invoke() { value := factory.New(); value.Act(\"ok\") }\n",
        ),
    ]);
    let expected = BTreeSet::from(["Local".to_string()]);

    assert_eq!(resolved_method_owners(&cg, "invoke", "Act"), expected);
    assert_eq!(manifest_owners(&cg, "app/use.go", "Act"), expected);
}

#[test]
fn s4_identity_never_reintroduces_a_bare_signature_decoy() {
    let cg = build_go(&[
        (
            "lib/defs.go",
            "package lib\ntype Doer interface { Act(string) }\ntype Holder struct { Doer }\ntype Local struct{}\nfunc (Local) Act(string) {}\nfunc invoke(h Holder) { h.Act(\"ok\") }\n",
        ),
        (
            "other/other.go",
            "package other\ntype Doer interface { Act(int) }\ntype Impl struct{}\nfunc (Impl) Act(int) {}\nfunc keepLive() { _ = Impl{} }\n",
        ),
    ]);
    let expected = BTreeSet::from(["Local".to_string()]);

    assert_eq!(resolved_method_owners(&cg, "invoke", "Act"), expected);
    assert_eq!(manifest_owners(&cg, "lib/defs.go", "Act"), expected);
}

#[test]
fn qualified_return_uses_the_caller_visible_concrete_declaration_kind() {
    let cg = build_go(&[
        (
            "factory/widget_linux.go",
            "package factory\ntype Widget struct{}\nfunc (Widget) Use() {}\nfunc New() Widget { return Widget{} }\n",
        ),
        (
            "factory/widget_windows.go",
            "package factory\ntype Widget interface { Use() }\n",
        ),
        (
            "app/use_linux.go",
            "package app\nimport factory \"example/factory\"\nfunc invoke() { value := factory.New(); value.Use() }\n",
        ),
    ]);

    assert_eq!(
        resolved_target_files(&cg, "invoke", "Use"),
        BTreeSet::from(["factory/widget_linux.go".to_string()])
    );
}

#[test]
fn nested_selector_preserves_a_materialized_unresolved_return_base() {
    let cg = build_go(&[
        (
            "factory/factory.go",
            "package factory\nimport missing \"example/missing\"\nfunc New() missing.Widget { panic(\"unreachable\") }\n",
        ),
        (
            "other/decoy.go",
            "package other\ntype Decoy struct{}\nfunc (Decoy) Use() {}\n",
        ),
        (
            "app/use.go",
            "package app\nimport factory \"example/factory\"\nfunc invoke() { w := factory.New(); w.Part.Use() }\n",
        ),
    ]);
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|site| site.caller.name == "invoke" && site.callee_name == "Use")
        .expect("nested unresolved-return method site");

    assert!(site.receiver_materialized);
    assert!(site.receiver_owner_identity.is_none());
    assert!(cg.resolve_call_site_full(site).resolved.is_empty());
}

#[test]
fn nested_selector_preserves_return_partition_recovery_telemetry() {
    let cg = build_go(&[
        (
            "factory/box_linux.go",
            "//go:build linux\n\npackage factory\ntype LinuxGadget struct{}\nfunc (LinuxGadget) Use() {}\ntype LinuxBox struct { Part LinuxGadget }\nfunc New() LinuxBox { return LinuxBox{} }\n",
        ),
        (
            "factory/box_windows.go",
            "//go:build windows\n\npackage factory\ntype WindowsGadget struct{}\nfunc (WindowsGadget) Use() {}\ntype WindowsBox struct { Part WindowsGadget }\nfunc New() WindowsBox { return WindowsBox{} }\n",
        ),
        (
            "app/use_linux.go",
            "//go:build linux\n\npackage app\nimport factory \"example/factory\"\nfunc invoke() { w := factory.New(); w.Part.Use() }\n",
        ),
    ]);
    let stats = prism::navigation::queries::call_stats(&cg);

    assert_eq!(
        resolved_method_owners(&cg, "invoke", "Use"),
        BTreeSet::from(["LinuxGadget".to_string()])
    );
    assert_eq!(stats["go_owner_identity_partition_recovered"], 1);
    assert_eq!(stats["go_owner_identity_partition_drop"], 0);
}

#[test]
fn qualified_return_func_value_field_uses_the_proven_package_owner() {
    let cg = build_go(&[
        (
            "factory/command.go",
            "package factory\ntype Command struct { Run func() }\nfunc factoryHandler() {}\nfunc New() Command { return Command{Run: factoryHandler} }\n",
        ),
        (
            "app/local.go",
            "package app\ntype Command struct { Run func() }\nfunc appHandler() {}\nfunc setup() { _ = Command{Run: appHandler} }\n",
        ),
        (
            "app/use.go",
            "package app\nimport factory \"example/factory\"\nfunc invoke() { command := factory.New(); command.Run() }\n",
        ),
    ]);
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|site| site.caller.name == "invoke" && site.callee_name == "Run")
        .expect("qualified-return func-value-field site");
    let owner = site
        .receiver_owner_identity
        .as_ref()
        .expect("qualified-return Command owner");

    assert_eq!(owner.package_dir, "factory");
    assert_eq!(owner.package_clause, "factory");
    assert_eq!(owner.name, "Command");
    assert_eq!(
        resolved_target_names(&cg, "invoke", "Run"),
        BTreeSet::from(["factoryHandler".to_string()])
    );
}
