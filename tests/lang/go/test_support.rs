use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
use prism::repo_loader::load_repo;
use std::collections::BTreeMap;

pub(crate) fn build_go(sources: &[(&str, &str)]) -> CallGraph {
    build_go_with_module(sources, "example")
}

pub(crate) fn build_go_with_module(sources: &[(&str, &str)], module: &str) -> CallGraph {
    let root = tempfile::tempdir().expect("temporary Go module");
    std::fs::write(
        root.path().join("go.mod"),
        format!("module {module}\n\ngo 1.22\n"),
    )
    .expect("write go.mod fixture");
    for (path, source) in sources {
        let path = root.path().join(path);
        std::fs::create_dir_all(path.parent().expect("fixture parent"))
            .expect("create fixture directory");
        std::fs::write(path, source).expect("write Go fixture");
    }
    let repo = load_repo(root.path()).expect("load Go fixture through repo_loader");
    CallGraph::build_with_scope_graph_inputs(&repo.files, repo.scope_graph_inputs.as_ref())
}

pub(crate) fn build_parsed_go_with_module(
    files: &BTreeMap<String, ParsedFile>,
    module: &str,
) -> CallGraph {
    let root = tempfile::tempdir().expect("temporary Go module");
    std::fs::write(
        root.path().join("go.mod"),
        format!("module {module}\n\ngo 1.22\n"),
    )
    .expect("write go.mod fixture");
    let inputs = prism::repo_loader::scope_graph_build_inputs(root.path(), files);
    CallGraph::build_with_scope_graph_inputs(files, Some(&inputs))
}
