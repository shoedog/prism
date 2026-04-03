#[path = "../../common/mod.rs"]
mod common;
use common::*;

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

    let base_only: Vec<_> = obj_defs.iter().filter(|d| !d.path.has_fields()).collect();
    assert!(
        base_only.is_empty(),
        "Phase 2 JS: field assignments should NOT create base-only defs"
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

    let base_only: Vec<_> = obj_defs.iter().filter(|d| !d.path.has_fields()).collect();
    assert!(
        base_only.is_empty(),
        "Phase 2 TypeScript: field assignments should NOT create base-only defs"
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
