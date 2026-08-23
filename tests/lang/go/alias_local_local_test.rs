use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
use prism::languages::Language;
use prism::resolution::{ResolutionConfidence, ResolutionKind};
use std::collections::{BTreeMap, BTreeSet};

fn build_go(sources: &[(&str, &str)], module: Option<&str>) -> CallGraph {
    let files: BTreeMap<String, ParsedFile> = sources
        .iter()
        .map(|(path, source)| {
            (
                (*path).to_string(),
                ParsedFile::parse(path, source, Language::Go).expect("parse Go fixture"),
            )
        })
        .collect();
    let Some(module) = module else {
        return CallGraph::build(&files);
    };
    let repo = tempfile::tempdir().expect("temporary Go module root");
    std::fs::write(
        repo.path().join("go.mod"),
        format!("module {module}\n\ngo 1.24\n"),
    )
    .expect("write go.mod fixture");
    let inputs = prism::repo_loader::scope_graph_build_inputs(repo.path(), &files);
    CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs))
}

fn resolver_target_files(cg: &CallGraph, caller: &str, method: &str) -> BTreeSet<String> {
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|site| site.caller.name == caller && site.callee_name == method)
        .expect("interface dispatch site");
    cg.resolve_call_site_full(site)
        .resolved
        .iter()
        .map(|resolved| {
            assert_eq!(resolved.confidence, ResolutionConfidence::Exact);
            assert_eq!(resolved.kind, ResolutionKind::InterfaceDispatch);
            resolved.target.file.clone()
        })
        .collect()
}

fn manifest_target_files(cg: &CallGraph, caller_file: &str, method: &str) -> BTreeSet<String> {
    prism::navigation::queries::interface_dispatch_manifest(cg)["sites"]
        .as_array()
        .expect("manifest sites")
        .iter()
        .find(|site| site["file"] == caller_file && site["method"] == method)
        .expect("manifest interface dispatch site")["implementer_identities"]
        .as_array()
        .expect("manifest implementer identities")
        .iter()
        .map(|identity| identity["file"].as_str().expect("target file").to_string())
        .collect()
}

fn assert_target_files(
    cg: &CallGraph,
    caller_file: &str,
    caller: &str,
    method: &str,
    expected: &[&str],
) {
    let expected: BTreeSet<String> = expected.iter().map(|path| (*path).to_string()).collect();
    assert_eq!(resolver_target_files(cg, caller, method), expected);
    assert_eq!(manifest_target_files(cg, caller_file, method), expected);
}

#[test]
fn alias_to_local_substitutes_the_complete_rhs() {
    let cg = build_go(
        &[
            (
                "api/api.go",
                "package api\ntype ID struct{}\ntype Alias = ID\ntype Doer interface{ Use(Alias) }\ntype Holder struct{ Doer }\nfunc invoke(h Holder, id ID){ h.Use(id) }\n",
            ),
            (
                "api/impl.go",
                "package api\ntype Impl struct{}\nfunc (Impl) Use(ID){}\n",
            ),
        ],
        Some("example.com/root"),
    );
    assert_target_files(&cg, "api/api.go", "invoke", "Use", &["api/impl.go"]);
}

#[test]
fn alias_to_qualified_substitutes_the_import_path_identity() {
    let cg = build_go(
        &[
            ("base/base.go", "package base\ntype ID struct{}\n"),
            (
                "api/api.go",
                "package api\nimport base \"example.com/root/base\"\ntype Alias = base.ID\ntype Doer interface{ Use(Alias) }\ntype Holder struct{ Doer }\nfunc invoke(h Holder, id Alias){ h.Use(id) }\n",
            ),
            (
                "worker/impl.go",
                "package worker\nimport base \"example.com/root/base\"\ntype Impl struct{}\nfunc (Impl) Use(base.ID){}\n",
            ),
        ],
        Some("example.com/root"),
    );
    assert_target_files(&cg, "api/api.go", "invoke", "Use", &["worker/impl.go"]);
}

#[test]
fn alias_to_instantiated_generic_preserves_arguments() {
    let cg = build_go(
        &[
            (
                "base/base.go",
                "package base\ntype List[T any] struct{ Value T }\n",
            ),
            (
                "api/api.go",
                "package api\nimport base \"example.com/root/base\"\ntype L = base.List[int]\ntype Doer interface{ Use(L) }\ntype Holder struct{ Doer }\nfunc invoke(h Holder, v L){ h.Use(v) }\n",
            ),
            (
                "worker/impl.go",
                "package worker\nimport base \"example.com/root/base\"\ntype Impl struct{}\nfunc (Impl) Use(base.List[int]){}\n",
            ),
        ],
        Some("example.com/root"),
    );
    assert_target_files(&cg, "api/api.go", "invoke", "Use", &["worker/impl.go"]);
}

#[test]
fn predeclared_aliases_normalize_byte_and_rune() {
    let cg = build_go(
        &[
            (
                "api/api.go",
                "package api\ntype B = byte\ntype R = rune\ntype Doer interface{ Bytes(B); Runes(R) }\ntype Holder struct{ Doer }\nfunc invokeBytes(h Holder, v B){ h.Bytes(v) }\nfunc invokeRunes(h Holder, v R){ h.Runes(v) }\n",
            ),
            (
                "worker/impl.go",
                "package worker\ntype Impl struct{}\nfunc (Impl) Bytes(uint8){}\nfunc (Impl) Runes(int32){}\n",
            ),
        ],
        Some("example.com/root"),
    );
    assert_target_files(
        &cg,
        "api/api.go",
        "invokeBytes",
        "Bytes",
        &["worker/impl.go"],
    );
    assert_target_files(
        &cg,
        "api/api.go",
        "invokeRunes",
        "Runes",
        &["worker/impl.go"],
    );
}

#[test]
fn alias_to_composite_substitutes_nested_pointer_slice_map_and_func() {
    let cg = build_go(
        &[
            ("base/base.go", "package base\ntype ID struct{}\n"),
            (
                "api/api.go",
                "package api\nimport base \"example.com/root/base\"\ntype Payload = map[string][]*base.ID\ntype Callback = func(Payload) error\ntype Doer interface{ Use(Callback) }\ntype Holder struct{ Doer }\nfunc invoke(h Holder, v Callback){ h.Use(v) }\n",
            ),
            (
                "worker/impl.go",
                "package worker\nimport base \"example.com/root/base\"\ntype Impl struct{}\nfunc (Impl) Use(func(map[string][]*base.ID) error){}\n",
            ),
        ],
        Some("example.com/root"),
    );
    assert_target_files(&cg, "api/api.go", "invoke", "Use", &["worker/impl.go"]);
}

#[test]
fn aliases_in_two_packages_expand_to_one_base_type() {
    let cg = build_go(
        &[
            ("base/base.go", "package base\ntype ID struct{}\n"),
            (
                "api/api.go",
                "package api\nimport base \"example.com/root/base\"\ntype A = base.ID\ntype Doer interface{ Use(A) }\ntype Holder struct{ Doer }\nfunc invoke(h Holder, v A){ h.Use(v) }\n",
            ),
            (
                "worker/impl.go",
                "package worker\nimport base \"example.com/root/base\"\ntype B = base.ID\ntype Impl struct{}\nfunc (Impl) Use(B){}\n",
            ),
        ],
        Some("example.com/root"),
    );
    assert_target_files(&cg, "api/api.go", "invoke", "Use", &["worker/impl.go"]);
}

#[test]
fn bare_to_bare_without_module_keeps_the_name_rule() {
    let cg = build_go(
        &[
            (
                "api/api.go",
                "package api\ntype ID struct{}\ntype Doer interface{ Use(ID) }\ntype Holder struct{ Doer }\nfunc invoke(h Holder, id ID){ h.Use(id) }\n",
            ),
            (
                "worker/impl.go",
                "package worker\ntype ID struct{}\ntype Impl struct{}\nfunc (Impl) Use(ID){}\n",
            ),
        ],
        None,
    );
    assert_target_files(&cg, "api/api.go", "invoke", "Use", &["worker/impl.go"]);
}
