use prism::ast::ParsedFile;
use prism::call_graph::{CallGraph, CallSite};
use prism::languages::Language;
use prism::resolution::{ResolutionConfidence, ResolutionKind};
use std::collections::{BTreeMap, BTreeSet};

fn build_nested_module(sources: &[(&str, &str)]) -> CallGraph {
    let files: BTreeMap<String, ParsedFile> = sources
        .iter()
        .map(|(path, source)| {
            (
                (*path).to_string(),
                ParsedFile::parse(path, source, Language::Go).expect("parse Go fixture"),
            )
        })
        .collect();
    let repo = tempfile::tempdir().expect("temporary module root");
    std::fs::create_dir_all(repo.path().join("tests")).expect("nested module directory");
    std::fs::write(
        repo.path().join("go.mod"),
        "module example.com/root\n\ngo 1.22\n",
    )
    .expect("root go.mod");
    std::fs::write(
        repo.path().join("tests/go.mod"),
        "module example.com/root/tests\n\ngo 1.22\n",
    )
    .expect("nested go.mod");
    std::fs::write(
        repo.path().join("go.work"),
        "go 1.22\nuse (\n .\n ./tests\n)\n",
    )
    .expect("go.work");
    let inputs = prism::repo_loader::scope_graph_build_inputs(repo.path(), &files);
    CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs))
}

fn site<'a>(cg: &'a CallGraph, caller: &str, method: &str) -> &'a CallSite {
    cg.calls
        .values()
        .flatten()
        .find(|site| site.caller.name == caller && site.callee_name == method)
        .unwrap_or_else(|| panic!("missing {caller}->{method}"))
}

fn manifest_route(cg: &CallGraph, caller_file: &str, line: usize) -> String {
    prism::navigation::queries::interface_dispatch_manifest(cg)["sites"]
        .as_array()
        .expect("manifest sites")
        .iter()
        .find(|entry| entry["file"] == caller_file && entry["line"] == line)
        .unwrap_or_else(|| panic!("missing manifest site {caller_file}:{line}"))["dispatch_route"]
        .as_str()
        .expect("dispatch route")
        .to_string()
}

#[test]
fn qualified_receivers_use_exact_import_paths_across_nested_modules() {
    let cg = build_nested_module(&[
        (
            "tests/framework/integration/cluster.go",
            "package integration\n\
             type Cluster struct{}\n\
             func (*Cluster) M() {}\n\
             func NewCluster() *Cluster { return &Cluster{} }\n",
        ),
        (
            "tests/framework/interfaces/interface.go",
            "package interfaces\n\
             type Cluster interface{ M() }\n\
             type Wrong struct{}\n\
             func (*Wrong) M() {}\n\
             func retain() { _ = &Wrong{} }\n",
        ),
        (
            "tests/integration/revision_test.go",
            "package integration_test\n\
             import integration \"example.com/root/tests/framework/integration\"\n\
             func literalParam() {\n\
               run := func(clus *integration.Cluster) { clus.M() }\n\
               _ = run\n\
             }\n\
             func factoryLocal() {\n\
               clus := integration.NewCluster()\n\
               clus.M()\n\
             }\n",
        ),
    ]);

    for (caller, line) in [("literalParam", 4), ("factoryLocal", 9)] {
        let call = site(&cg, caller, "M");
        let outcome = cg.resolve_call_site_full(call);
        let files: BTreeSet<_> = outcome
            .resolved
            .iter()
            .map(|resolved| resolved.target.file.clone())
            .collect();
        assert_eq!(
            files,
            BTreeSet::from(["tests/framework/integration/cluster.go".into()]),
            "{caller}: {outcome:?}"
        );
        assert_eq!(outcome.drop, None);
        assert_eq!(outcome.telemetry.go_concrete_receiver_direct, 1);
        assert_eq!(outcome.resolved[0].confidence, ResolutionConfidence::Exact);
        assert_ne!(outcome.resolved[0].kind, ResolutionKind::InterfaceDispatch);
        assert_eq!(
            manifest_route(&cg, "tests/integration/revision_test.go", line),
            "concrete_direct"
        );
    }
}
