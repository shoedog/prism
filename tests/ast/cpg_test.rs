use crate::common::*;
use prism::cpg::{CodePropertyGraph, CpgEdge, CpgNode};

fn build_cpg_files(files: &[(&str, &str, Language)]) -> CodePropertyGraph {
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
fn function_node_carries_real_byte_span() {
    let src = "fn alpha() {}\nfn beta() {}\n";
    let cpg = build_rust_cpg(src);
    let (sb, eb) = cpg
        .function_nodes()
        .into_iter()
        .find_map(|n| match cpg.node(n) {
            prism::cpg::CpgNode::Function {
                name,
                start_byte,
                end_byte,
                ..
            } if name == "alpha" => Some((*start_byte, *end_byte)),
            _ => None,
        })
        .expect("alpha");
    assert_eq!(&src[sb..eb], "fn alpha() {}");
}

#[test]
fn variable_occurrence_carries_real_member_span() {
    let src = "def f(o):\n    o.config.timeout = 5\n    return o.config.timeout\n";
    let cpg = build_python_cpg(src);
    let (sb, eb) = cpg
        .node_indices()
        .find_map(|n| match cpg.node(n) {
            CpgNode::Variable {
                path,
                line,
                access,
                start_byte,
                end_byte,
                ..
            } if *line == 2 && *access == prism::cpg::VarAccess::Def && path.has_fields() => {
                Some((*start_byte, *end_byte))
            }
            _ => None,
        })
        .expect("o.config.timeout def");
    assert_eq!(&src[sb..eb], "o.config.timeout");
}

#[test]
fn scoped_return_use_keeps_real_member_span() {
    let src = "def f(o, x):\n    o.config.timeout = x\n    return o.config.timeout\n";
    let cpg = build_python_cpg(src);
    let (sb, eb) = cpg
        .nodes_at("test.py", 3)
        .into_iter()
        .find_map(|n| match cpg.node(n) {
            CpgNode::Variable {
                path,
                access,
                start_byte,
                end_byte,
                ..
            } if *access == prism::cpg::VarAccess::Use && path.has_fields() => {
                Some((*start_byte, *end_byte))
            }
            _ => None,
        })
        .expect("return o.config.timeout use");
    assert!(eb > sb, "return use should not be a line anchor");
    assert_eq!(&src[sb..eb], "o.config.timeout");
}

#[test]
fn same_line_assignment_orders_by_source_position() {
    let src = "def f(p):\n    q = p\n    return q\n";
    let cpg = build_python_cpg(src);

    // RAW nodes_at order (production order) must be byte-sorted, leftmost first.
    let order = same_line_var_byte_order(&cpg, "test.py", 2);
    let q = order.iter().position(|s| s == "def:q").unwrap();
    let p = order.iter().position(|s| s == "use:p").unwrap();
    assert!(q < p, "lhs `q` precedes rhs `p` by source byte: {order:?}");

    // And the production location_index bucket is already in this order (not just the test's sort).
    let raw: Vec<_> = cpg
        .nodes_at("test.py", 2)
        .into_iter()
        .filter_map(|n| match cpg.node(n) {
            prism::cpg::CpgNode::Variable {
                path, start_byte, ..
            } => Some((*start_byte, path.base.clone())),
            _ => None,
        })
        .collect();
    let mut sorted = raw.clone();
    sorted.sort();
    assert_eq!(
        raw, sorted,
        "location_index bucket is byte-ordered in production"
    );
}

#[test]
fn statement_node_carries_real_span() {
    let src = "def f():\n    return 1\n";
    let cpg = build_python_cpg(src);
    let (sb, eb) = cpg
        .nodes_at("test.py", 2)
        .into_iter()
        .find_map(|n| match cpg.node(n) {
            CpgNode::Statement {
                start_byte,
                end_byte,
                ..
            } => Some((*start_byte, *end_byte)),
            _ => None,
        })
        .expect("stmt");
    assert!(eb > sb && src[sb..eb].contains("return"));
}

#[test]
fn augmented_assignment_emits_def_and_use() {
    let src = "def f(x):\n    x += 1\n    return x\n";
    let cpg = build_python_cpg(src);
    let on2: Vec<_> = cpg
        .nodes_at("test.py", 2)
        .into_iter()
        .filter_map(|n| match cpg.node(n) {
            CpgNode::Variable { path, access, .. } if path.base == "x" => Some(*access),
            _ => None,
        })
        .collect();
    assert!(
        on2.contains(&prism::cpg::VarAccess::Def) && on2.contains(&prism::cpg::VarAccess::Use),
        "x += 1 reads and writes x"
    );
}

#[test]
fn augmented_assignment_emits_def_and_use_across_languages() {
    let cases = [
        (Language::Python, "test.py", "def f(x):\n    x += 1\n", 2),
        (
            Language::Go,
            "test.go",
            "package main\nfunc f() {\n    x := 0\n    x += 1\n}\n",
            4,
        ),
        (
            Language::Java,
            "Test.java",
            "class Test {\n    void f() {\n        int x = 0;\n        x += 1;\n    }\n}\n",
            4,
        ),
        (
            Language::Rust,
            "test.rs",
            "fn f() {\n    let mut x = 0;\n    x += 1;\n}\n",
            3,
        ),
        (
            Language::JavaScript,
            "test.js",
            "function f() {\n    let x = 0;\n    x += 1;\n}\n",
            3,
        ),
    ];

    for (lang, file, src, line) in cases {
        let cpg = build_cpg(file, src, lang);
        let accesses: Vec<_> = cpg
            .nodes_at(file, line)
            .into_iter()
            .filter_map(|n| match cpg.node(n) {
                CpgNode::Variable { path, access, .. } if path.base == "x" => Some(*access),
                _ => None,
            })
            .collect();
        assert!(
            accesses.contains(&prism::cpg::VarAccess::Def)
                && accesses.contains(&prism::cpg::VarAccess::Use),
            "{lang:?} augmented assignment should read and write x, got {accesses:?}"
        );
    }
}

#[test]
fn augmented_member_assignment_emits_member_and_base_use() {
    let src = "def f(o):\n    o.field += 1\n";
    let cpg = build_python_cpg(src);
    let mut has_member_def = false;
    let mut has_member_use = false;
    let mut has_base_use = false;
    for node in cpg.nodes_at("test.py", 2) {
        if let CpgNode::Variable { path, access, .. } = cpg.node(node) {
            has_member_def |= path.has_fields() && *access == prism::cpg::VarAccess::Def;
            has_member_use |= path.has_fields() && *access == prism::cpg::VarAccess::Use;
            has_base_use |=
                path.is_simple() && path.base == "o" && *access == prism::cpg::VarAccess::Use;
        }
    }
    assert!(has_member_def, "member target should be a def");
    assert!(has_member_use, "augmented member target should be a use");
    assert!(
        has_base_use,
        "augmented member target should emit base fallback use"
    );
}

#[test]
fn rust_mut_pattern_def_uses_identifier_span() {
    let src = "fn f() {\n    let mut x = 0;\n    x\n}\n";
    let cpg = build_rust_cpg(src);
    let (sb, eb) = cpg
        .nodes_at("test.rs", 2)
        .into_iter()
        .find_map(|n| match cpg.node(n) {
            CpgNode::Variable {
                path,
                access,
                start_byte,
                end_byte,
                ..
            } if path.base == "x" && *access == prism::cpg::VarAccess::Def => {
                Some((*start_byte, *end_byte))
            }
            _ => None,
        })
        .expect("x def");
    assert_eq!(&src[sb..eb], "x");
}

#[test]
fn alias_resolved_def_keeps_raw_occurrence_span() {
    let src = "function f(device) {\n    const { name } = device;\n    return name;\n}\n";
    let cpg = build_cpg("test.js", src, Language::JavaScript);
    let (sb, eb) = cpg
        .nodes_at("test.js", 2)
        .into_iter()
        .find_map(|n| match cpg.node(n) {
            CpgNode::Variable {
                path,
                access,
                start_byte,
                end_byte,
                ..
            } if path.base == "device"
                && path.fields.len() == 1
                && path.fields[0].as_str() == "name"
                && *access == prism::cpg::VarAccess::Def =>
            {
                Some((*start_byte, *end_byte))
            }
            _ => None,
        })
        .expect("device.name alias-resolved def");
    assert_eq!(&src[sb..eb], "name");
}

#[test]
fn multiline_field_only_parameter_does_not_create_base_def() {
    let src = "def f(\n    dev,\n):\n    return dev.name\n";
    let cpg = build_python_cpg(src);
    let has_param_def = cpg.nodes_at("test.py", 1).into_iter().any(|n| {
        matches!(
            cpg.node(n),
            CpgNode::Variable {
                path,
                access: prism::cpg::VarAccess::Def,
                ..
            } if path.is_simple() && path.base == "dev"
        )
    });
    assert!(
        !has_param_def,
        "field-only multiline parameter should keep field isolation"
    );
}

#[test]
fn multiline_parameter_same_line_body_use_keeps_dataflow_edge() {
    let src = "def f(\n    x): return x\n";
    let cpg = build_python_cpg(src);

    assert!(has_dataflow_edge(
        &cpg,
        ("test.py", "f", 1, "x"),
        ("test.py", "f", 2, "x")
    ));
}

#[test]
fn cpg_rust_mut_parameter_call_binding_uses_normalized_name() {
    let src = "fn callee(mut x: i32) -> i32 {\n    x += 1;\n    x\n}\nfn caller(y: i32) {\n    callee(y);\n}\n";
    let cpg = build_rust_cpg(src);

    assert!(has_dataflow_edge(
        &cpg,
        ("test.rs", "caller", 6, "y"),
        ("test.rs", "callee", 1, "x")
    ));
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

    let cpg = build_cpg_files(&[
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
    let cpg = build_cpg_files(&[(
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

#[test]
fn cpg_python_free_function_with_self_param_keeps_all_args() {
    // Review fix (MAJOR 3): the self/cls receiver skip must be gated on actual
    // method ownership. A FREE function whose first param happens to be named
    // `self` keeps all its params — args must not shift by one.
    let cpg = build_cpg_files(&[(
        // both params used so each gets a var node (an unused param has none).
        "m.py",
        "def helper(self, x):\n    return self + x\n\ndef call(a, b):\n    helper(a, b)\n",
        Language::Python,
    )]);

    // helper is module-level (no owner) -> no skip: a -> self, b -> x.
    assert!(has_dataflow_edge(
        &cpg,
        ("m.py", "call", 5, "a"),
        ("m.py", "helper", 1, "self")
    ));
    assert!(has_dataflow_edge(
        &cpg,
        ("m.py", "call", 5, "b"),
        ("m.py", "helper", 1, "x")
    ));
}

// S2 Task 4 (review-requested negative regression): a Rust `let mut x` wrapper pattern
// must yield a Def for `x` and must NOT leak a bogus `mut` variable path.
#[test]
fn rust_let_mut_does_not_emit_mut_path() {
    let cpg = build_rust_cpg("fn f() {\n    let mut x = 0;\n    let _ = x;\n}\n");
    let has_mut = cpg.node_indices().any(
        |n| matches!(cpg.node(n), prism::cpg::CpgNode::Variable { path, .. } if path.base == "mut"),
    );
    assert!(
        !has_mut,
        "`let mut x` must not produce a `mut` variable path"
    );
    let has_x_def = cpg.node_indices().any(|n| {
        matches!(cpg.node(n), prism::cpg::CpgNode::Variable { path, access, .. }
            if path.base == "x" && *access == prism::cpg::VarAccess::Def)
    });
    assert!(has_x_def, "`let mut x` must produce a Def for x");
}

// S2 Task 10 (acceptance): byte fields + byte-ordered buckets are deterministic across
// builds (BTreeMap/BTreeSet throughout + total byte tie-breaks).
#[test]
fn cpg_build_is_deterministic() {
    let src = "def f(p):\n    q = p\n    r = q\n    return r\n";
    assert_eq!(
        node_byte_dump(&build_python_cpg(src)),
        node_byte_dump(&build_python_cpg(src))
    );
}

#[test]
fn call_edge_carries_resolution_confidence() {
    // Two same-named methods; a typed receiver call should materialize as Exact.
    use prism::cpg::CpgEdge;
    use prism::resolution::ResolutionConfidence;

    let cpg = build_cpg_files(&[(
        "src/lib.rs",
        r#"
struct A; struct B;
impl A { fn run(&self) {} }
impl B { fn run(&self) {} }
fn exact_caller(a: A) { a.run(); }      // typed receiver -> Exact
"#,
        Language::Rust,
    )]);
    let confs: Vec<ResolutionConfidence> = cpg
        .graph
        .edge_weights()
        .filter_map(|w| match w {
            CpgEdge::Call(c) => Some(*c),
            _ => None,
        })
        .collect();
    assert!(
        confs.contains(&ResolutionConfidence::Exact),
        "expected at least one Call(Exact) edge, got {confs:?}"
    );
}

fn build_rust_cpg_with_virtual_methods(files: &[(&str, &str)]) -> CodePropertyGraph {
    use prism::type_db::{RecordInfo, RecordKind, TypeDatabase};

    let mut parsed_files = BTreeMap::new();
    for (path, source) in files {
        parsed_files.insert(
            (*path).to_string(),
            ParsedFile::parse(path, source, Language::Rust).unwrap(),
        );
    }

    let mut type_db = TypeDatabase::default();
    for class_name in ["A", "B"] {
        type_db.records.insert(
            class_name.to_string(),
            RecordInfo {
                name: class_name.to_string(),
                kind: RecordKind::Class,
                fields: vec![],
                bases: vec![],
                virtual_methods: BTreeMap::from([("draw".to_string(), "void".to_string())]),
                size: None,
                file: String::new(),
            },
        );
    }

    CodePropertyGraph::build_with_types(&parsed_files, type_db)
}

fn count_call_edges(
    cpg: &CodePropertyGraph,
    caller_name: &str,
    callee_name: &str,
    confidence: prism::resolution::ResolutionConfidence,
) -> usize {
    cpg.graph
        .edge_indices()
        .filter(|&edge| cpg.graph[edge] == CpgEdge::Call(confidence))
        .filter(|&edge| {
            cpg.graph
                .edge_endpoints(edge)
                .map(|(source, target)| {
                    matches!(
                        (cpg.node(source), cpg.node(target)),
                        (
                            CpgNode::Function { name: caller, .. },
                            CpgNode::Function { name: callee, .. },
                        ) if caller == caller_name && callee == callee_name
                    )
                })
                .unwrap_or(false)
        })
        .count()
}

#[test]
fn cha_does_not_launder_nameonly_into_exact() {
    // A NameOnly call edge alone must not seed a CHA Exact edge.
    use prism::resolution::ResolutionConfidence;

    let cpg = build_rust_cpg_with_virtual_methods(&[
        (
            "src/a.rs",
            r#"
impl A { fn draw(&self) {} }
fn caller() {
    let x = mystery();
    x.draw();
}
"#,
        ),
        ("src/b.rs", "impl B { fn draw(&self) {} }\n"),
    ]);

    assert_eq!(
        count_call_edges(&cpg, "caller", "draw", ResolutionConfidence::NameOnly),
        1,
        "expected the local single-owner receiver dispatch to produce one NameOnly edge",
    );
    assert_eq!(
        count_call_edges(&cpg, "caller", "draw", ResolutionConfidence::Exact),
        0,
        "a NameOnly seed must not be laundered into Exact CHA edges",
    );
}

#[test]
fn cha_upgrades_nameonly_pair_to_exact() {
    // A CHA-confirmed override dispatch should add Call(Exact) even when the
    // same caller/callee pair already has a demoted TraitCha edge.
    use prism::resolution::ResolutionConfidence;

    let cpg = build_rust_cpg_with_virtual_methods(&[(
        "src/lib.rs",
        r#"
impl Render for A { fn draw(&self) {} }
impl Render for B { fn draw(&self) {} }
fn caller(x: &dyn Render) {
    Render::draw(x);
    A::draw();
}
"#,
    )]);

    assert!(
        count_call_edges(&cpg, "caller", "draw", ResolutionConfidence::NameOnly) >= 1,
        "expected a pre-existing NameOnly trait dispatch edge",
    );
    assert!(
        count_call_edges(&cpg, "caller", "draw", ResolutionConfidence::Exact) >= 2,
        "expected the Exact owner call plus a CHA-upgraded override edge",
    );
}

#[test]
fn callers_of_node_filters_by_confidence_and_excludes_seed() {
    use prism::cpg::query::ConfidenceFilter;

    let cpg = build_cpg_files(&[(
        "src/lib.rs",
        r#"
struct A; struct B;
impl A { fn run(&self) {} }
impl B { fn run(&self) {} }
fn exact_caller(a: A) { a.run(); }       // Exact caller of A::run
fn nameonly_caller(x: &dyn Run) { x.run(); } // name-only caller (not A::run specifically)
trait Run { fn run(&self); }
"#,
        Language::Rust,
    )]);

    // Seed = A::run by exact identity (file, name, start_line).
    let a_run = cpg
        .function_candidates("src/lib.rs", "run")
        .into_iter()
        .find(|&n| cpg.to_function_id(n).is_some())
        .expect("A::run node");
    let exact = cpg.callers_of_node(a_run, 2, ConfidenceFilter::ExactOnly);
    let all = cpg.callers_of_node(a_run, 2, ConfidenceFilter::All);
    assert!(all.len() >= exact.len(), "All is a superset of ExactOnly");
    assert!(
        !exact.iter().any(|(n, _)| *n == a_run),
        "seed excluded at depth 0"
    );
    assert!(
        exact.iter().all(|(_, d)| *d >= 1),
        "depths are 1-based for callers"
    );
    let all_nodes: std::collections::BTreeSet<_> = all.iter().map(|(n, _)| *n).collect();
    assert!(
        exact.iter().all(|(n, _)| all_nodes.contains(n)),
        "ExactOnly is a subset of All"
    );
    assert!(!all.is_empty(), "the typed Exact caller is found under All");
}

#[test]
fn function_node_for_id_is_start_line_keyed() {
    let cpg = build_cpg_files(&[(
        "src/lib.rs",
        "fn dup() {}\nmod a { pub fn dup() {} }\n",
        Language::Rust,
    )]);
    let ids: Vec<_> = cpg
        .call_graph
        .functions
        .get("dup")
        .cloned()
        .unwrap_or_default();
    assert!(ids.len() >= 2, "two `dup` defs");
    let n0 = cpg.function_node_for_id(&ids[0]).unwrap();
    let n1 = cpg.function_node_for_id(&ids[1]).unwrap();
    assert_ne!(n0, n1, "distinct nodes by start_line, not first-candidate");
}
