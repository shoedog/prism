use crate::common::*;
use prism::cpg::{CodePropertyGraph, CpgEdge, CpgNode};

fn build_cpg(files: &[(&str, &str, Language)]) -> CodePropertyGraph {
    let mut parsed_files = BTreeMap::new();
    for (path, source, language) in files {
        parsed_files.insert(
            (*path).to_string(),
            ParsedFile::parse(path, source, *language).unwrap(),
        );
    }
    CodePropertyGraph::build(&parsed_files)
}

fn cpg_var_node_matches(
    node: &CpgNode,
    file: &str,
    function: &str,
    line: usize,
    base: &str,
) -> bool {
    matches!(
        node,
        CpgNode::Variable {
            path,
            file: node_file,
            function: node_function,
            line: node_line,
            ..
        } if node_file == file
            && node_function == function
            && *node_line == line
            && path.base == base
    )
}

fn has_dataflow_edge(
    cpg: &CodePropertyGraph,
    from: (&str, &str, usize, &str),
    to: (&str, &str, usize, &str),
) -> bool {
    cpg.graph.edge_indices().any(|edge| {
        cpg.graph[edge] == CpgEdge::DataFlow
            && cpg
                .graph
                .edge_endpoints(edge)
                .map(|(source, target)| {
                    cpg_var_node_matches(cpg.node(source), from.0, from.1, from.2, from.3)
                        && cpg_var_node_matches(cpg.node(target), to.0, to.1, to.2, to.3)
                })
                .unwrap_or(false)
    })
}

#[test]
fn test_without_cpg_context_runs_ast_only() {
    // CpgContext::without_cpg should work for AST-only algorithms
    let source = r#"
def add(x, y):
    return x + y
"#;
    let path = "src/add.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let ctx = CpgContext::without_cpg(&files, None);
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff);
    let result = algorithms::run_slicing(&ctx, &diff, &config).unwrap();

    assert!(
        !result.blocks.is_empty(),
        "AST-only algorithm should work with empty CPG context"
    );
}

#[test]
fn cpg_call_edges_exclude_multi_owner_drops_include_demoted() {
    use prism::languages::Language::Rust;

    let cpg = build_cpg(&[
        (
            "a.rs",
            "impl A {\n    fn poll(&self) {}\n}\nimpl OnlyOwner {\n    fn frob(&self) {}\n}\n",
            Rust,
        ),
        ("b.rs", "impl B {\n    fn poll(&self) {}\n}\n", Rust),
        (
            "m.rs",
            "fn drive() {\n    let x = mystery();\n    let y = mystery();\n    x.poll();\n    y.frob();\n}\n",
            Rust,
        ),
    ]);

    let drive_callees = cpg.callees_of("drive", "m.rs", 1);
    let names: Vec<&str> = drive_callees
        .iter()
        .map(|(function, _)| function.name.as_str())
        .collect();
    assert!(!names.contains(&"poll"), "multi-owner dropped: {names:?}");
    assert!(
        names.contains(&"frob"),
        "single-owner NameOnly included: {names:?}"
    );
}

#[test]
fn cpg_python_method_arg_binds_to_explicit_param_not_receiver() {
    let cpg = build_cpg(&[(
        "m.py",
        "class C:\n    def method(self, a):\n        return a\n\ndef call(obj, x):\n    obj.method(x)\n",
        Language::Python,
    )]);

    assert!(has_dataflow_edge(
        &cpg,
        ("m.py", "call", 6, "x"),
        ("m.py", "method", 2, "a")
    ));
    assert!(!has_dataflow_edge(
        &cpg,
        ("m.py", "call", 6, "x"),
        ("m.py", "method", 2, "self")
    ));
}
