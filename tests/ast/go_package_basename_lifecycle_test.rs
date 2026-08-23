use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
use prism::languages::Language;
use std::collections::BTreeMap;

fn parsed_files(sources: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
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

#[test]
fn func_value_reapply_after_edit_discards_stale_import_path_keys() {
    let repo = tempfile::tempdir().expect("temporary module root");
    std::fs::write(
        repo.path().join("go.mod"),
        "module example.com/root\n\ngo 1.22\n",
    )
    .expect("go.mod");

    let before = parsed_files(&[
        (
            "oldpkg/types.go",
            "package oldpkg\ntype Cmd struct{ Run func() }\n",
        ),
        (
            "app/use.go",
            "package app\nimport \"example.com/root/oldpkg\"\nfunc use(c oldpkg.Cmd) { c.Run() }\n",
        ),
    ]);
    let inputs = prism::repo_loader::scope_graph_build_inputs(repo.path(), &before);
    let mut graph = CallGraph::build_with_scope_graph_inputs(&before, Some(&inputs));
    let old_key = "@go-import:example.com/root/oldpkg";
    assert_eq!(
        graph.go_package_basenames[old_key],
        std::collections::BTreeSet::from(["oldpkg".to_string()]),
        "fixture must contain the scoped exact import-path key"
    );

    let after = parsed_files(&[
        (
            "newpkg/types.go",
            "package newpkg\ntype Cmd struct{ Run func() }\n",
        ),
        (
            "app/use.go",
            "package app\nimport \"example.com/root/newpkg\"\nfunc use(c newpkg.Cmd) { c.Run() }\n",
        ),
    ]);
    graph.apply_go_func_value_fields(&after);

    assert!(!graph.go_package_basenames.contains_key(old_key));
    assert!(graph
        .go_package_basenames
        .values()
        .all(|dirs| !dirs.contains("oldpkg")));
    assert_eq!(
        graph.go_package_basenames["newpkg"],
        std::collections::BTreeSet::from(["newpkg".to_string()])
    );
}
