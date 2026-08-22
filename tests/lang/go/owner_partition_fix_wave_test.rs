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
