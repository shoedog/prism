use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
use prism::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

fn build_workspace(
    sources: &[(&str, &str)],
    manifests: &[(&str, &str)],
    go_work: Option<&str>,
) -> CallGraph {
    let files: BTreeMap<String, ParsedFile> = sources
        .iter()
        .map(|(path, source)| {
            (
                (*path).to_string(),
                ParsedFile::parse(path, source, Language::Go).expect("parse Go fixture"),
            )
        })
        .collect();
    let repo = tempfile::tempdir().expect("temporary Go workspace root");
    for (path, contents) in manifests {
        let path = repo.path().join(path);
        std::fs::create_dir_all(path.parent().expect("manifest parent"))
            .expect("create manifest directory");
        std::fs::write(path, contents).expect("write manifest fixture");
    }
    if let Some(go_work) = go_work {
        std::fs::write(repo.path().join("go.work"), go_work).expect("write go.work fixture");
    }
    let inputs = prism::repo_loader::scope_graph_build_inputs(repo.path(), &files);
    CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs))
}

fn resolved_target_files(cg: &CallGraph, caller: &str, method: &str) -> BTreeSet<String> {
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|site| site.caller.name == caller && site.callee_name == method)
        .expect("interface dispatch site");
    cg.resolve_call_site_full(site)
        .resolved
        .iter()
        .map(|resolved| resolved.target.file.clone())
        .collect()
}

fn manifest_target_files(cg: &CallGraph, caller_file: &str, method: &str) -> BTreeSet<String> {
    prism::navigation::queries::interface_dispatch_manifest(cg)["sites"]
        .as_array()
        .expect("manifest sites")
        .iter()
        .find(|site| site["file"] == caller_file && site["method"] == method)
        .expect("manifest dispatch site")["implementer_identities"]
        .as_array()
        .expect("manifest implementer identities")
        .iter()
        .map(|identity| identity["file"].as_str().expect("target file").to_string())
        .collect()
}

#[test]
fn active_main_wins_before_cross_module_replace_conflict_with_target_file_parity() {
    let cg = build_workspace(
        &[
            (
                "api.go",
                "package a\nimport b \"example.com/b\"\ntype Doer interface { Act(b.T) }\ntype Holder struct { Doer }\nfunc invoke(h Holder, value b.T) { h.Act(value) }\n",
            ),
            (
                "b/impl.go",
                "package b\ntype T struct{}\ntype Impl struct{}\nfunc (Impl) Act(T) {}\n",
            ),
        ],
        &[
            (
                "go.mod",
                "module example.com/a\n\ngo 1.22\nreplace example.com/a => ./fork-a\n",
            ),
            (
                "b/go.mod",
                "module example.com/b\n\ngo 1.22\nreplace example.com/a => ../fork-b\n",
            ),
        ],
        Some("go 1.22\nuse (\n.\n./b\n)\n"),
    );
    let expected = BTreeSet::from(["b/impl.go".to_string()]);

    assert_eq!(resolved_target_files(&cg, "invoke", "Act"), expected);
    assert_eq!(manifest_target_files(&cg, "api.go", "Act"), expected);
    assert_eq!(
        prism::navigation::queries::call_stats(&cg)["go_module_graph"],
        serde_json::json!({
            "modules": 2,
            "active": 2,
            "replaces_parsed": 2,
            "replaces_applied": 0,
            "workspace_invalid": false,
        })
    );
}

#[test]
fn invalid_active_root_version_blocks_resolver_and_manifest_targets() {
    let cg = build_workspace(
        &[
            (
                "api.go",
                "package root\nimport p \"example.com/root/p\"\ntype Doer interface { Act(p.T) }\ntype Holder struct { Doer }\nfunc invoke(h Holder, value p.T) { h.Act(value) }\n",
            ),
            (
                "p/impl.go",
                "package p\ntype T struct{}\ntype Impl struct{}\nfunc (Impl) Act(T) {}\n",
            ),
        ],
        &[(
            "go.mod",
            "module example.com/root\n\ngo 1.22\nrequire example.com/a vbogus\n",
        )],
        None,
    );

    assert!(resolved_target_files(&cg, "invoke", "Act").is_empty());
    assert!(manifest_target_files(&cg, "api.go", "Act").is_empty());
    let stats = prism::navigation::queries::call_stats(&cg);
    assert_eq!(
        stats["go_module_graph"],
        serde_json::json!({
            "modules": 0,
            "active": 0,
            "replaces_parsed": 0,
            "replaces_applied": 0,
            "workspace_invalid": true,
        })
    );
    assert_eq!(stats["go_import_path_proven_files"], 0);
    assert_eq!(stats["go_import_path_unproven_files"], 2);
    assert_eq!(
        stats["go_import_path_unproven_reasons"],
        serde_json::json!({"workspace_invalid": 2})
    );
}

#[test]
fn retract_versions_gate_resolver_and_manifest_targets() {
    let sources = [
        (
            "api.go",
            "package root\nimport p \"example.com/root/p\"\ntype Doer interface { Act(p.T) }\ntype Holder struct { Doer }\nfunc invoke(h Holder, value p.T) { h.Act(value) }\n",
        ),
        (
            "p/impl.go",
            "package p\ntype T struct{}\ntype Impl struct{}\nfunc (Impl) Act(T) {}\n",
        ),
    ];
    let expected = BTreeSet::from(["p/impl.go".to_string()]);

    for directive in [
        "retract v1.0.0",
        "retract [v1.0.0, v1.2.0] // security rationale",
    ] {
        let go_mod = format!("module example.com/root\n\ngo 1.22\n{directive}\n");
        let cg = build_workspace(&sources, &[("go.mod", go_mod.as_str())], None);

        assert_eq!(
            resolved_target_files(&cg, "invoke", "Act"),
            expected,
            "directive: {directive}"
        );
        assert_eq!(
            manifest_target_files(&cg, "api.go", "Act"),
            expected,
            "directive: {directive}"
        );
        let stats = prism::navigation::queries::call_stats(&cg);
        assert_eq!(stats["go_module_graph"]["workspace_invalid"], false);
        assert_eq!(stats["go_import_path_proven_files"], 2);
        assert_eq!(stats["go_import_path_unproven_files"], 0);
    }

    for directive in ["retract vbogus", "retract [v1.0.0, vbogus]"] {
        let go_mod = format!("module example.com/root\n\ngo 1.22\n{directive}\n");
        let cg = build_workspace(&sources, &[("go.mod", go_mod.as_str())], None);

        assert!(
            resolved_target_files(&cg, "invoke", "Act").is_empty(),
            "directive: {directive}"
        );
        assert!(
            manifest_target_files(&cg, "api.go", "Act").is_empty(),
            "directive: {directive}"
        );
        let stats = prism::navigation::queries::call_stats(&cg);
        assert_eq!(stats["go_module_graph"]["workspace_invalid"], true);
        assert_eq!(stats["go_import_path_proven_files"], 0);
        assert_eq!(stats["go_import_path_unproven_files"], 2);
        assert_eq!(
            stats["go_import_path_unproven_reasons"],
            serde_json::json!({"workspace_invalid": 2})
        );
    }
}

#[test]
fn workspace_replace_path_override_selects_good_target_with_resolver_manifest_parity() {
    let cg = build_workspace(
        &[
            (
                "api.go",
                "package root\nimport a \"original.example/a\"\ntype Doer interface { Act(a.T) }\ntype Holder struct { Doer }\nfunc invoke(h Holder, value a.T) { h.Act(value) }\n",
            ),
            (
                "good/impl.go",
                "package a\ntype T struct{}\ntype Impl struct{}\nfunc (Impl) Act(T) {}\n",
            ),
        ],
        &[
            (
                "go.mod",
                "module example.com/root\nrequire original.example/a v1.0.0\nreplace original.example/a v1.0.0 => ./bad\n",
            ),
            ("good/go.mod", "module good.example/a\n"),
            ("bad/go.mod", "module bad.example/a\n"),
        ],
        Some("go 1.22\nuse .\nreplace original.example/a => ./good\n"),
    );
    let expected = BTreeSet::from(["good/impl.go".to_string()]);

    assert_eq!(resolved_target_files(&cg, "invoke", "Act"), expected);
    assert_eq!(manifest_target_files(&cg, "api.go", "Act"), expected);
    let stats = prism::navigation::queries::call_stats(&cg);
    assert_eq!(
        stats["go_module_graph"],
        serde_json::json!({
            "modules": 3,
            "active": 1,
            "replaces_parsed": 2,
            "replaces_applied": 1,
            "workspace_invalid": false,
        })
    );
    assert_eq!(stats["go_import_path_proven_files"], 2);
    assert_eq!(stats["go_import_path_unproven_files"], 0);
    assert_eq!(
        stats["go_import_path_unproven_reasons"],
        serde_json::json!({})
    );
}

#[test]
fn module_versioned_replace_without_workspace_override_has_no_exact_targets() {
    let cg = build_workspace(
        &[
            (
                "api.go",
                "package root\nimport a \"original.example/a\"\ntype Doer interface { Act(a.T) }\ntype Holder struct { Doer }\nfunc invoke(h Holder, value a.T) { h.Act(value) }\n",
            ),
            (
                "bad/impl.go",
                "package a\ntype T struct{}\ntype Impl struct{}\nfunc (Impl) Act(T) {}\n",
            ),
        ],
        &[
            (
                "go.mod",
                "module example.com/root\nrequire original.example/a v1.0.0\nreplace original.example/a v1.0.0 => ./bad\n",
            ),
            ("bad/go.mod", "module bad.example/a\n"),
        ],
        None,
    );

    assert!(resolved_target_files(&cg, "invoke", "Act").is_empty());
    assert!(manifest_target_files(&cg, "api.go", "Act").is_empty());
    let stats = prism::navigation::queries::call_stats(&cg);
    assert_eq!(stats["go_module_graph"]["replaces_parsed"], 1);
    assert_eq!(stats["go_module_graph"]["replaces_applied"], 0);
    assert_eq!(stats["go_module_graph"]["workspace_invalid"], false);
    assert_eq!(stats["go_import_path_proven_files"], 1);
    assert_eq!(stats["go_import_path_unproven_files"], 1);
    assert_eq!(
        stats["go_import_path_unproven_reasons"],
        serde_json::json!({"replace_unproven": 1})
    );
}
