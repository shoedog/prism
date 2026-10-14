use crate::common::*;

#[test]
fn test_dfg_js_field_access_paths() {
    // JS member_expression: obj.field
    let source = r#"
function setup(config) {
    config.timeout = 30;
    config.host = "localhost";
    let t = config.timeout;
}
"#;
    let path = "src/config.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let config_defs = dfg.all_defs_of(path, "config");

    let has_timeout = config_defs
        .iter()
        .any(|d| d.path.has_fields() && d.path.fields.contains(&"timeout".to_string()));
    assert!(
        has_timeout,
        "JS DFG should have AccessPath config.timeout from member_expression. Got: {:?}",
        config_defs
            .iter()
            .map(|d| d.path.to_string())
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_parent_function_typescript() {
    let (files, _, diff) = make_typescript_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_field_isolation_javascript() {
    let source = r#"
function process(obj) {
    obj.secret = getUserInput();
    obj.display = "safe";
    sink(obj.secret);
    render(obj.display);
}
"#;
    let path = "src/handler.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let obj_defs = dfg.all_defs_of(path, "obj");

    // PR-C contract: the formal is member-only, so it has exactly one base-only Def (the
    // parameter at the signature line); the field assignments create none.
    let base_only: Vec<_> = obj_defs.iter().filter(|d| !d.path.has_fields()).collect();
    assert_eq!(
        base_only.iter().map(|d| d.line).collect::<Vec<_>>(),
        vec![2],
        "Phase 2 JS: only the parameter Def is base-only; field assignments create none"
    );
    // Field isolation: every edge into or out of a field path stays on that path or starts at
    // the base Def; `obj.secret` never reaches `obj.display`.
    for edge in dfg.edges.iter().filter(|e| e.to.path.base == "obj") {
        assert!(
            edge.from.path == edge.to.path || edge.from.line == 2,
            "Phase 2 JS: cross-field edge {edge:?}"
        );
    }
    assert!(
        !dfg.edges
            .iter()
            .any(|e| e.from.path.to_string() == "obj.secret"
                && e.to.path.to_string() == "obj.display"),
        "Phase 2 JS: obj.secret must not reach obj.display"
    );
}

#[test]
fn test_field_isolation_typescript() {
    let source = r#"
function process(obj: Config) {
    obj.secret = getUserInput();
    obj.label = "safe";
    sink(obj.secret);
    render(obj.label);
}
"#;
    let path = "src/handler.ts";
    let parsed = ParsedFile::parse(path, source, Language::TypeScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let obj_defs = dfg.all_defs_of(path, "obj");

    // PR-C contract: the formal is member-only, so it has exactly one base-only Def (the
    // parameter at the signature line); the field assignments create none.
    let base_only: Vec<_> = obj_defs.iter().filter(|d| !d.path.has_fields()).collect();
    assert_eq!(
        base_only.iter().map(|d| d.line).collect::<Vec<_>>(),
        vec![2],
        "Phase 2 TypeScript: only the parameter Def is base-only; field assignments create none"
    );
    // Field isolation: every edge into or out of a field path stays on that path or starts at
    // the base Def; `obj.secret` never reaches `obj.label`.
    for edge in dfg.edges.iter().filter(|e| e.to.path.base == "obj") {
        assert!(
            edge.from.path == edge.to.path || edge.from.line == 2,
            "Phase 2 TypeScript: cross-field edge {edge:?}"
        );
    }
    assert!(
        !dfg.edges.iter().any(
            |e| e.from.path.to_string() == "obj.secret" && e.to.path.to_string() == "obj.label"
        ),
        "Phase 2 TypeScript: obj.secret must not reach obj.label"
    );
}

#[test]
fn test_js_optional_chaining_access_path() {
    let normal = AccessPath::from_expr("obj.config.host");
    let optional = AccessPath::from_expr("obj?.config?.host");
    assert_eq!(
        normal, optional,
        "Optional chaining should normalize to same AccessPath as dot access"
    );
    assert_eq!(optional.base, "obj");
    assert_eq!(optional.fields, vec!["config", "host"]);

    let mixed = AccessPath::from_expr("obj?.config.host");
    assert_eq!(mixed.base, "obj");
    assert_eq!(mixed.fields, vec!["config", "host"]);
}
