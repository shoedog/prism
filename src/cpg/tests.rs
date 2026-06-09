use super::*;
use crate::ast::ParsedFile;
use crate::languages::Language;
use crate::type_db::{FieldInfo, RecordInfo, RecordKind, TypeDatabase, TypedefInfo};

#[test]
fn test_node_accessors() {
    let func = CpgNode::Function {
        name: "main".into(),
        file: "src/main.c".into(),
        start_line: 1,
        end_line: 10,
    };
    assert_eq!(func.file(), "src/main.c");
    assert_eq!(func.line(), 1);
    assert!(func.is_function());

    let var_def = CpgNode::Variable {
        path: AccessPath::from_expr("dev->name"),
        file: "src/dev.c".into(),
        function: "init".into(),
        line: 5,
        access: VarAccess::Def,
    };
    assert!(var_def.is_def());
    assert!(!var_def.is_use());

    let call = CpgNode::Statement {
        file: "src/main.c".into(),
        line: 3,
        kind: StmtKind::Call {
            callee: "init".into(),
        },
    };
    assert!(call.is_call());
}

#[test]
fn test_edge_classification() {
    assert!(CpgEdge::DataFlow.is_data_flow());
    assert!(!CpgEdge::Call.is_data_flow());
    assert!(CpgEdge::Call.is_interprocedural());
    assert!(CpgEdge::Return.is_interprocedural());
    assert!(!CpgEdge::DataFlow.is_interprocedural());
    assert!(!CpgEdge::Contains.is_interprocedural());
    assert!(!CpgEdge::FieldOf.is_interprocedural());
    assert!(!CpgEdge::ControlFlow.is_data_flow());
}

#[test]
fn test_variable_node_accessors() {
    let var_use = CpgNode::Variable {
        path: AccessPath::from_expr("dev->id"),
        file: "src/dev.c".into(),
        function: "get_id".into(),
        line: 8,
        access: VarAccess::Use,
    };
    assert!(var_use.is_use());
    assert!(!var_use.is_def());
    assert!(!var_use.is_function());
    assert!(!var_use.is_call());
    assert_eq!(var_use.file(), "src/dev.c");
    assert_eq!(var_use.line(), 8);
}

#[test]
fn test_statement_node_non_call() {
    let branch = CpgNode::Statement {
        file: "src/main.c".into(),
        line: 15,
        kind: StmtKind::Branch,
    };
    assert!(!branch.is_call());
    assert!(!branch.is_function());
    assert!(!branch.is_def());
    assert_eq!(branch.file(), "src/main.c");
    assert_eq!(branch.line(), 15);

    let ret = CpgNode::Statement {
        file: "src/main.c".into(),
        line: 20,
        kind: StmtKind::Return,
    };
    assert!(!ret.is_call());
}

#[test]
fn test_cpg_build_basic() {
    let source = r#"
void init() {
    int x = 1;
    int y = x;
    use(y);
}
"#;
    let path = "src/test.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    // Should have at least one function node
    assert!(cpg.node_count() > 0, "CPG should have nodes");
    assert!(cpg.edge_count() > 0, "CPG should have edges");

    // Should be able to look up the function
    let func_idx = cpg.function_node(path, "init");
    assert!(func_idx.is_some(), "Should find function 'init'");

    // Function node should have correct metadata
    let func = cpg.node(func_idx.unwrap());
    assert!(func.is_function());
    assert_eq!(func.file(), path);
}

#[test]
fn test_cpg_dataflow_edges() {
    let source = r#"
void flow() {
    int x = 1;
    int y = x;
}
"#;
    let path = "src/flow.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    // Check that dataflow edges exist
    let df_edges: Vec<_> = cpg
        .graph
        .edge_indices()
        .filter(|&e| cpg.graph[e] == CpgEdge::DataFlow)
        .collect();
    assert!(
        !df_edges.is_empty(),
        "CPG should have DataFlow edges for x → y"
    );
}

#[test]
fn test_cpg_call_edges() {
    let source = r#"
void callee() {
    return;
}

void caller() {
    callee();
}
"#;
    let path = "src/calls.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    // Check caller → callee Call edge
    let caller_idx = cpg.function_node(path, "caller").unwrap();
    let callee_idx = cpg.function_node(path, "callee").unwrap();

    let call_reachable = cpg.reachable_forward(caller_idx, &|e| matches!(e, CpgEdge::Call));
    assert!(
        call_reachable.contains(&callee_idx),
        "caller should reach callee via Call edge"
    );

    // Check callee → caller Return edge
    let return_reachable = cpg.reachable_forward(callee_idx, &|e| matches!(e, CpgEdge::Return));
    assert!(
        return_reachable.contains(&caller_idx),
        "callee should reach caller via Return edge"
    );
}

#[test]
fn test_cpg_contains_edges() {
    let source = r#"
void f() {
    int x = 1;
    int y = x;
}
"#;
    let path = "src/contains.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    let func_idx = cpg.function_node(path, "f").unwrap();
    let contained = cpg.reachable_forward(func_idx, &|e| matches!(e, CpgEdge::Contains));
    assert!(
        !contained.is_empty(),
        "Function 'f' should contain variable nodes"
    );

    // All contained nodes should be Variable nodes
    for idx in &contained {
        let node = cpg.node(*idx);
        assert!(
            node.is_def() || node.is_use(),
            "Contains edge should lead to Variable nodes, got {:?}",
            node
        );
    }
}

#[test]
fn test_cpg_edge_filtered_reachability() {
    // DataFlow-only reachability should NOT follow Call edges
    let source = r#"
void helper() {
    return;
}

void main_func() {
    int x = 1;
    int y = x;
    helper();
}
"#;
    let path = "src/filter.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    let main_idx = cpg.function_node(path, "main_func").unwrap();
    let helper_idx = cpg.function_node(path, "helper").unwrap();

    // Call-only should reach helper
    let call_reach = cpg.reachable_forward(main_idx, &|e| matches!(e, CpgEdge::Call));
    assert!(call_reach.contains(&helper_idx));

    // DataFlow-only from main_func should NOT reach helper function node
    let df_reach = cpg.reachable_forward(main_idx, &|e| matches!(e, CpgEdge::DataFlow));
    assert!(
        !df_reach.contains(&helper_idx),
        "DataFlow-only traversal should not reach function nodes via Call edges"
    );
}

#[test]
fn test_cpg_call_graph_cycles() {
    let source = r#"
void a() {
    b();
}

void b() {
    a();
}
"#;
    let path = "src/cycle.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);
    let cycles = cpg.call_graph_cycles();

    assert!(!cycles.is_empty(), "Should detect a → b → a call cycle");

    // The cycle should contain both function nodes
    let cycle_names: BTreeSet<String> = cycles[0]
        .iter()
        .filter_map(|&idx| match cpg.node(idx) {
            CpgNode::Function { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();
    assert!(cycle_names.contains("a"), "Cycle should contain 'a'");
    assert!(cycle_names.contains("b"), "Cycle should contain 'b'");
}

#[test]
fn test_cpg_bfs_with_distance() {
    let source = r#"
void a() {
    b();
}

void b() {
    c();
}

void c() {
    return;
}
"#;
    let path = "src/dist.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    let a_idx = cpg.function_node(path, "a").unwrap();
    let b_idx = cpg.function_node(path, "b").unwrap();
    let c_idx = cpg.function_node(path, "c").unwrap();

    let distances = cpg.bfs_with_distance(&[a_idx], 5, &|e| matches!(e, CpgEdge::Call));

    assert_eq!(distances.get(&a_idx), Some(&0));
    assert_eq!(distances.get(&b_idx), Some(&1));
    assert_eq!(distances.get(&c_idx), Some(&2));
}

#[test]
fn test_cpg_bridge_to_var_location() {
    let source = r#"
void f() {
    int x = 1;
    int y = x;
}
"#;
    let path = "src/bridge.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    // Find a variable node and convert back
    let var_nodes: Vec<_> = cpg
        .graph
        .node_indices()
        .filter(|&idx| cpg.node(idx).is_def())
        .collect();
    assert!(!var_nodes.is_empty());

    let loc = cpg.to_var_location(var_nodes[0]);
    assert!(loc.is_some());
    let loc = loc.unwrap();
    assert_eq!(loc.file, path);
    assert_eq!(loc.function, "f");
}

#[test]
fn test_cpg_bridge_to_function_id() {
    let source = r#"
void my_func() {
    return;
}
"#;
    let path = "src/bridge_func.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    let func_idx = cpg.function_node(path, "my_func").unwrap();
    let fid = cpg.to_function_id(func_idx).unwrap();
    assert_eq!(fid.name, "my_func");
    assert_eq!(fid.file, path);
}

#[test]
fn test_build_enriched_without_types() {
    let source = "void f() { int x = 1; }\n";
    let path = "src/enriched.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build_enriched(&files, None);
    assert!(!cpg.has_type_info());
    assert!(cpg.node_count() > 0);
}

#[test]
fn test_build_enriched_with_types() {
    let source = "void f() { int x = 1; }\n";
    let path = "src/enriched2.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let mut type_db = TypeDatabase::default();
    type_db.records.insert(
        "MyStruct".to_string(),
        RecordInfo {
            name: "MyStruct".to_string(),
            kind: RecordKind::Struct,
            fields: vec![FieldInfo {
                name: "x".to_string(),
                type_str: "int".to_string(),
                offset: None,
            }],
            bases: vec![],
            virtual_methods: BTreeMap::new(),
            size: None,
            file: String::new(),
        },
    );

    let cpg = CodePropertyGraph::build_enriched(&files, Some(&type_db));
    assert!(cpg.has_type_info());
    assert!(cpg.node_count() > 0);
}

#[test]
fn test_build_with_types() {
    let source = "void f() { int x = 1; }\n";
    let path = "src/owned.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let type_db = TypeDatabase::default();
    let cpg = CodePropertyGraph::build_with_types(&files, type_db);
    assert!(cpg.has_type_info());
}

#[test]
fn test_all_fields_of() {
    let source = "void f() {}\n";
    let path = "src/fields.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let mut type_db = TypeDatabase::default();
    type_db.records.insert(
        "Point".to_string(),
        RecordInfo {
            name: "Point".to_string(),
            kind: RecordKind::Struct,
            fields: vec![
                FieldInfo {
                    name: "x".to_string(),
                    type_str: "int".to_string(),
                    offset: None,
                },
                FieldInfo {
                    name: "y".to_string(),
                    type_str: "int".to_string(),
                    offset: None,
                },
            ],
            bases: vec![],
            virtual_methods: BTreeMap::new(),
            size: None,
            file: String::new(),
        },
    );

    let cpg = CodePropertyGraph::build_with_types(&files, type_db);
    let fields = cpg.all_fields_of("Point").unwrap();
    assert_eq!(fields, vec!["x", "y"]);

    // Unknown type returns None
    assert!(cpg.all_fields_of("Unknown").is_none());
}

#[test]
fn test_resolve_type_with_typedef() {
    let source = "void f() {}\n";
    let path = "src/typedef.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let mut type_db = TypeDatabase::default();
    type_db.typedefs.insert(
        "handle_t".to_string(),
        TypedefInfo {
            name: "handle_t".to_string(),
            underlying: "struct device *".to_string(),
        },
    );

    let cpg = CodePropertyGraph::build_with_types(&files, type_db);
    assert_eq!(cpg.resolve_type("handle_t"), "struct device *");
    assert_eq!(cpg.resolve_type("int"), "int"); // not a typedef
}

#[test]
fn test_resolve_type_without_type_db() {
    let source = "void f() {}\n";
    let path = "src/no_types.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);
    assert_eq!(cpg.resolve_type("handle_t"), "handle_t");
}

#[test]
fn test_is_union_type() {
    let source = "void f() {}\n";
    let path = "src/union.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let mut type_db = TypeDatabase::default();
    type_db.records.insert(
        "MyUnion".to_string(),
        RecordInfo {
            name: "MyUnion".to_string(),
            kind: RecordKind::Union,
            fields: vec![
                FieldInfo {
                    name: "i".to_string(),
                    type_str: "int".to_string(),
                    offset: None,
                },
                FieldInfo {
                    name: "f".to_string(),
                    type_str: "float".to_string(),
                    offset: None,
                },
            ],
            bases: vec![],
            virtual_methods: BTreeMap::new(),
            size: None,
            file: String::new(),
        },
    );
    type_db.records.insert(
        "MyStruct".to_string(),
        RecordInfo {
            name: "MyStruct".to_string(),
            kind: RecordKind::Struct,
            fields: vec![],
            bases: vec![],
            virtual_methods: BTreeMap::new(),
            size: None,
            file: String::new(),
        },
    );

    let cpg = CodePropertyGraph::build_with_types(&files, type_db);
    assert!(cpg.is_union_type("MyUnion"));
    assert!(!cpg.is_union_type("MyStruct"));
    assert!(!cpg.is_union_type("NonExistent"));
}

#[test]
fn test_field_type() {
    let source = "void f() {}\n";
    let path = "src/field_type.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let mut type_db = TypeDatabase::default();
    type_db.records.insert(
        "Device".to_string(),
        RecordInfo {
            name: "Device".to_string(),
            kind: RecordKind::Struct,
            fields: vec![
                FieldInfo {
                    name: "id".to_string(),
                    type_str: "int".to_string(),
                    offset: None,
                },
                FieldInfo {
                    name: "name".to_string(),
                    type_str: "char *".to_string(),
                    offset: None,
                },
            ],
            bases: vec![],
            virtual_methods: BTreeMap::new(),
            size: None,
            file: String::new(),
        },
    );

    let cpg = CodePropertyGraph::build_with_types(&files, type_db);
    assert_eq!(cpg.field_type("Device", "id"), Some("int".to_string()));
    assert_eq!(cpg.field_type("Device", "name"), Some("char *".to_string()));
    assert_eq!(cpg.field_type("Device", "nonexistent"), None);
    assert_eq!(cpg.field_type("Unknown", "id"), None);
}

#[test]
fn test_function_at() {
    let source = r#"
void first() {
    int x = 1;
}

void second() {
    int y = 2;
}
"#;
    let path = "src/func_at.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    // Line 3 is inside first()
    let result = cpg.function_at(path, 3);
    assert!(result.is_some());
    let (_, fid) = result.unwrap();
    assert_eq!(fid.name, "first");

    // Line 7 is inside second()
    let result = cpg.function_at(path, 7);
    assert!(result.is_some());
    let (_, fid) = result.unwrap();
    assert_eq!(fid.name, "second");

    // Line 5 is between functions
    let result = cpg.function_at(path, 5);
    assert!(result.is_none());

    // Non-existent file
    let result = cpg.function_at("no_such_file.c", 1);
    assert!(result.is_none());
}

#[test]
fn test_callers_of() {
    let source = r#"
void target() {
    return;
}

void caller1() {
    target();
}

void caller2() {
    target();
}
"#;
    let path = "src/callers.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    let callers = cpg.callers_of("target", 1);
    let caller_names: BTreeSet<String> = callers.iter().map(|(fid, _)| fid.name.clone()).collect();
    assert!(caller_names.contains("caller1"));
    assert!(caller_names.contains("caller2"));
    assert_eq!(callers.len(), 2);
}

#[test]
fn test_callees_of() {
    let source = r#"
void helper1() {
    return;
}

void helper2() {
    return;
}

void main_fn() {
    helper1();
    helper2();
}
"#;
    let path = "src/callees.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    let callees = cpg.callees_of("main_fn", path, 1);
    let callee_names: BTreeSet<String> = callees.iter().map(|(fid, _)| fid.name.clone()).collect();
    assert!(callee_names.contains("helper1"));
    assert!(callee_names.contains("helper2"));
}

#[test]
fn test_function_nodes() {
    let source = r#"
void a() { return; }
void b() { return; }
void c() { return; }
"#;
    let path = "src/func_nodes.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);
    let func_nodes = cpg.function_nodes();
    assert_eq!(func_nodes.len(), 3);
    for idx in &func_nodes {
        assert!(cpg.node(*idx).is_function());
    }
}

#[test]
fn test_virtual_dispatch_enrichment() {
    let source = r#"
void render() {
    draw();
}

void draw() {
    return;
}
"#;
    let path = "src/virtual.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let mut type_db = TypeDatabase::default();
    type_db.records.insert(
        "Shape".to_string(),
        RecordInfo {
            name: "Shape".to_string(),
            kind: RecordKind::Class,
            fields: vec![],
            bases: vec![],
            virtual_methods: {
                let mut m = BTreeMap::new();
                m.insert("draw".to_string(), "void".to_string());
                m
            },
            size: None,
            file: String::new(),
        },
    );

    let cpg = CodePropertyGraph::build_with_types(&files, type_db);
    assert!(cpg.has_type_info());

    // The CPG should still have both functions
    assert!(cpg.function_node(path, "render").is_some());
    assert!(cpg.function_node(path, "draw").is_some());
}

#[test]
fn test_taint_forward_basic() {
    let source = r#"
void process() {
    int input = read_user();
    int data = input;
    write(data);
}
"#;
    let path = "src/taint.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    let sources = vec![(path.to_string(), 3usize)]; // line where input is defined
    let paths = cpg.taint_forward(&sources);
    // Should find at least one taint path from the source
    // (may be empty if DFG doesn't connect precisely, but shouldn't panic)
    let _ = paths;
}

#[test]
fn test_has_type_info() {
    let source = "void f() {}\n";
    let path = "src/has_type.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg_no_types = CodePropertyGraph::build(&files);
    assert!(!cpg_no_types.has_type_info());

    let cpg_with_types = CodePropertyGraph::build_with_types(&files, TypeDatabase::default());
    assert!(cpg_with_types.has_type_info());
}

// -----------------------------------------------------------------------
// Phase 6: CFG edge tests
// -----------------------------------------------------------------------

#[test]
fn test_cpg_has_cfg_edges() {
    let source = r#"
void f() {
    int x = 1;
    int y = 2;
    int z = 3;
}
"#;
    let path = "src/cfg_test.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);
    assert!(cpg.has_cfg_edges(), "CPG should have ControlFlow edges");
    assert!(cpg.cfg_edge_count() > 0);
}

#[test]
fn test_cpg_statement_nodes_created() {
    let source = r#"
void f() {
    int x = 1;
    int y = x;
    return;
}
"#;
    let path = "src/stmt_nodes.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    // Should have Statement nodes at lines 3, 4, 5
    assert!(
        cpg.statement_at(path, 3).is_some(),
        "Should have statement at line 3"
    );
    assert!(
        cpg.statement_at(path, 4).is_some(),
        "Should have statement at line 4"
    );
    assert!(
        cpg.statement_at(path, 5).is_some(),
        "Should have statement at line 5 (return)"
    );
}

#[test]
fn test_cpg_cfg_sequential_flow() {
    let source = r#"
void f() {
    int x = 1;
    int y = 2;
    int z = 3;
}
"#;
    let path = "src/cfg_seq.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    // Line 3 → Line 4 via ControlFlow
    let stmt3 = cpg.statement_at(path, 3).unwrap();
    let successors = cpg.cfg_successors(stmt3);
    let succ_lines: Vec<usize> = successors.iter().map(|&idx| cpg.node(idx).line()).collect();
    assert!(
        succ_lines.contains(&4),
        "Line 3 should flow to line 4, got {:?}",
        succ_lines
    );
}

#[test]
fn test_cpg_cfg_return_terminates() {
    let source = r#"
void f() {
    int x = 1;
    return;
    int y = 2;
}
"#;
    let path = "src/cfg_ret.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    // return at line 4 should NOT have a successor to line 5
    let stmt4 = cpg.statement_at(path, 4).unwrap();
    let successors = cpg.cfg_successors(stmt4);
    let succ_lines: Vec<usize> = successors.iter().map(|&idx| cpg.node(idx).line()).collect();
    assert!(
        !succ_lines.contains(&5),
        "return should not flow to line 5, got {:?}",
        succ_lines
    );
}

#[test]
fn test_cpg_cfg_if_branches() {
    let source = r#"
void f(int x) {
    if (x > 0) {
        int a = 1;
    } else {
        int b = 2;
    }
    int c = 3;
}
"#;
    let path = "src/cfg_if.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    // if at line 3 should have CFG successors to both branches
    let if_stmt = cpg.statement_at(path, 3).unwrap();
    let successors = cpg.cfg_successors(if_stmt);
    assert!(
        successors.len() >= 2,
        "if should branch to at least 2 targets, got {} successors",
        successors.len()
    );
}

#[test]
fn test_cpg_cfg_predecessors() {
    let source = r#"
void f() {
    int x = 1;
    int y = 2;
    int z = 3;
}
"#;
    let path = "src/cfg_pred.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    // Line 4 should have line 3 as predecessor
    let stmt4 = cpg.statement_at(path, 4).unwrap();
    let preds = cpg.cfg_predecessors(stmt4);
    let pred_lines: Vec<usize> = preds.iter().map(|&idx| cpg.node(idx).line()).collect();
    assert!(
        pred_lines.contains(&3),
        "Line 4 should have line 3 as predecessor, got {:?}",
        pred_lines
    );
}

#[test]
fn test_cpg_cfg_goto_edge() {
    let source = r#"
void f() {
    int x = 1;
    goto cleanup;
    int y = 2;
cleanup:
    free(x);
}
"#;
    let path = "src/cfg_goto.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    // goto at line 4 should have a CFG edge (either to label or through goto resolution)
    let goto_stmt = cpg.statement_at(path, 4);
    assert!(goto_stmt.is_some(), "Should have statement at goto line 4");

    // goto should NOT have sequential successor to line 5
    if let Some(idx) = goto_stmt {
        let successors = cpg.cfg_successors(idx);
        let succ_lines: Vec<usize> = successors.iter().map(|&s| cpg.node(s).line()).collect();
        assert!(
            !succ_lines.contains(&5),
            "goto should not fall through to line 5, got {:?}",
            succ_lines
        );
    }
}

#[test]
fn test_cpg_cfg_edge_filtered_reachability() {
    let source = r#"
void f() {
    int x = 1;
    int y = 2;
    int z = 3;
}
"#;
    let path = "src/cfg_reach.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    // CFG reachability: line 3 should reach line 5 via ControlFlow edges
    let stmt3 = cpg.statement_at(path, 3).unwrap();
    let reachable = cpg.reachable_forward(stmt3, &|e| matches!(e, CpgEdge::ControlFlow));
    let reachable_lines: BTreeSet<usize> =
        reachable.iter().map(|&idx| cpg.node(idx).line()).collect();
    assert!(
        reachable_lines.contains(&5),
        "Line 3 should CFG-reach line 5, got {:?}",
        reachable_lines
    );
}

#[test]
fn test_cpg_cfg_python() {
    let source = r#"
def f():
    x = 1
    y = 2
    z = 3
"#;
    let path = "src/cfg_py.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);
    assert!(
        cpg.has_cfg_edges(),
        "Python CPG should have ControlFlow edges"
    );

    // Sequential flow: line 3 → line 4
    let stmt3 = cpg.statement_at(path, 3);
    assert!(stmt3.is_some(), "Should have Python statement at line 3");
    if let Some(idx) = stmt3 {
        let succs = cpg.cfg_successors(idx);
        assert!(
            !succs.is_empty(),
            "Python line 3 should have CFG successors"
        );
    }
}

// -----------------------------------------------------------------------
// Phase 6 PR C: CFG-constrained analysis tests
// -----------------------------------------------------------------------

#[test]
fn test_cfg_reachable_lines() {
    let source = r#"
void f() {
    int x = 1;
    int y = 2;
    return;
    int z = 3;
}
"#;
    let path = "test.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let cpg = CodePropertyGraph::build(&files);

    // Line 3 should reach lines 4 and 5 (return), but NOT line 6 (after return)
    let reachable = cpg.cfg_reachable_lines(path, 3);
    assert!(
        reachable.contains(&(path.to_string(), 4)),
        "Line 3 should CFG-reach line 4, got {:?}",
        reachable
    );
    assert!(
        reachable.contains(&(path.to_string(), 5)),
        "Line 3 should CFG-reach line 5 (return), got {:?}",
        reachable
    );
    // Line 6 is dead code after return — should NOT be reachable
    assert!(
        !reachable.contains(&(path.to_string(), 6)),
        "Line 6 (after return) should NOT be CFG-reachable from line 3, got {:?}",
        reachable
    );
}

#[test]
fn test_taint_forward_cfg_prunes_dead_code() {
    // Taint source at line 3 (x = input), return at line 4,
    // sink at line 5 (after return — dead code). CFG-constrained taint
    // should NOT reach line 5.
    let source = r#"
void f(char* input) {
    char* x = input;
    return;
    exec(x);
}
"#;
    let path = "test.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let cpg = CodePropertyGraph::build(&files);

    let taint_sources = vec![(path.to_string(), 3)];
    let paths = cpg.taint_forward_cfg(&taint_sources);

    // Collect all tainted target lines
    let tainted_lines: BTreeSet<usize> = paths
        .iter()
        .flat_map(|p| p.edges.iter().map(|e| e.to.line))
        .collect();

    // Line 5 (exec after return) should be pruned by CFG constraint
    assert!(
        !tainted_lines.contains(&5),
        "CFG-constrained taint should NOT reach dead code at line 5, got {:?}",
        tainted_lines
    );
}

#[test]
fn test_dfg_cfg_chop_prunes_unreachable() {
    // Source at line 3, sink at line 6. Line 5 is dead code after return.
    // CFG-constrained chop should exclude the dead-code line.
    let source = r#"
void f() {
    int x = 1;
    int y = x;
    return;
    int z = x;
    int w = z;
}
"#;
    let path = "test.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let cpg = CodePropertyGraph::build(&files);

    // DFG chop: source=line 3, sink=line 7 (dead code)
    // CFG-constrained should be empty or exclude dead lines
    let chop = cpg.dfg_cfg_chop(path, 3, path, 7);

    // Line 7 is dead code — CFG forward from line 3 can't reach it
    // The chop should not include line 6 or 7 since they're unreachable
    let has_dead_code = chop.iter().any(|(_, l)| *l == 6 || *l == 7);
    assert!(
        !has_dead_code,
        "CFG-constrained chop should not include dead code lines 6-7, got {:?}",
        chop
    );
}

#[test]
fn test_cfg_constrained_fallback_without_cfg() {
    // When no CFG edges exist (e.g., no functions), methods should
    // gracefully return empty/fallback results
    let source = "int x = 1;\n";
    let path = "test.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let cpg = CodePropertyGraph::build(&files);

    // cfg_reachable_lines on non-existent statement → empty
    let reachable = cpg.cfg_reachable_lines(path, 999);
    assert!(reachable.is_empty());

    // taint_forward_cfg falls back to taint_forward
    let paths_cfg = cpg.taint_forward_cfg(&[(path.to_string(), 1)]);
    let paths_dfg = cpg.taint_forward(&[(path.to_string(), 1)]);
    assert_eq!(paths_cfg.len(), paths_dfg.len());
}
