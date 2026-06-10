use prism::cpg::CodePropertyGraph;
use prism::navigation::types::Reachability;
use prism::reasoning::shape::reachability_at;

fn build_py(src: &str) -> CodePropertyGraph {
    let parsed =
        prism::ast::ParsedFile::parse("test.py", src, prism::languages::Language::Python).unwrap();
    let mut files = std::collections::BTreeMap::new();
    files.insert("test.py".to_string(), parsed);
    CodePropertyGraph::build(&files)
}

#[test]
fn taint_trace_reachability_end_to_end() {
    let cpg =
        build_py("def g(p):\n    sink(p)\n\ndef f():\n    u = input()\n    v = u\n    g(u)\n");
    let trace = cpg.taint_trace(&[("test.py".to_string(), 5usize)]);
    assert_eq!(
        reachability_at(&cpg, &trace, "test.py", 6),
        Reachability::Reached
    );
    assert_eq!(
        reachability_at(&cpg, &trace, "test.py", 2),
        Reachability::BoundaryExited
    );
}
