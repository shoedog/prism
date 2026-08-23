use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
use prism::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

fn build_workspace(
    sources: &[(&str, &str)],
    manifests: &[(&str, &str)],
    go_work: &str,
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
    std::fs::write(repo.path().join("go.work"), go_work).expect("write go.work fixture");
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
        "go 1.22\nuse (\n.\n./b\n)\n",
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
