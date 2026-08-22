use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
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
