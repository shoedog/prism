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
fn test_full_flow_javascript() {
    let (files, _, diff) = make_javascript_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    assert!(!result.blocks.is_empty());
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
fn test_destructuring_object_basic_js() {
    // const { name, id } = device → name aliases device.name, id aliases device.id
    let source = r#"
function process(device) {
    const { name, id } = device;
    console.log(name);
    console.log(id);
}
"#;
    let path = "src/process.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();

    let func_node = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=6).collect();
    let aliases = parsed.collect_alias_assignments(&func_node, &lines);

    // Should have: name → device.name, id → device.id
    assert!(
        aliases
            .iter()
            .any(|(a, t, _)| a == "name" && t == "device.name"),
        "Expected alias name → device.name, got: {:?}",
        aliases
    );
    assert!(
        aliases
            .iter()
            .any(|(a, t, _)| a == "id" && t == "device.id"),
        "Expected alias id → device.id, got: {:?}",
        aliases
    );
}


#[test]
fn test_destructuring_renamed_js() {
    // const { name: userName } = device → userName aliases device.name
    let source = r#"
function process(device) {
    const { name: userName, id: deviceId } = device;
    console.log(userName);
}
"#;
    let path = "src/process.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();

    let func_node = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=5).collect();
    let aliases = parsed.collect_alias_assignments(&func_node, &lines);

    assert!(
        aliases
            .iter()
            .any(|(a, t, _)| a == "userName" && t == "device.name"),
        "Expected alias userName → device.name, got: {:?}",
        aliases
    );
    assert!(
        aliases
            .iter()
            .any(|(a, t, _)| a == "deviceId" && t == "device.id"),
        "Expected alias deviceId → device.id, got: {:?}",
        aliases
    );
}


#[test]
fn test_destructuring_nested_js() {
    // const { config: { host, port } } = device → host aliases device.config.host
    let source = r#"
function connect(device) {
    const { config: { host, port } } = device;
    open(host, port);
}
"#;
    let path = "src/connect.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();

    let func_node = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=5).collect();
    let aliases = parsed.collect_alias_assignments(&func_node, &lines);

    assert!(
        aliases
            .iter()
            .any(|(a, t, _)| a == "host" && t == "device.config.host"),
        "Expected alias host → device.config.host, got: {:?}",
        aliases
    );
    assert!(
        aliases
            .iter()
            .any(|(a, t, _)| a == "port" && t == "device.config.port"),
        "Expected alias port → device.config.port, got: {:?}",
        aliases
    );
}


#[test]
fn test_destructuring_array_js() {
    // const [first, second] = items → first aliases items, second aliases items
    let source = r#"
function process(items) {
    const [first, second] = items;
    console.log(first);
}
"#;
    let path = "src/process.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();

    let func_node = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=5).collect();
    let aliases = parsed.collect_alias_assignments(&func_node, &lines);

    // Array destructuring aliases each element to the base array
    assert!(
        aliases.iter().any(|(a, t, _)| a == "first" && t == "items"),
        "Expected alias first → items, got: {:?}",
        aliases
    );
    assert!(
        aliases
            .iter()
            .any(|(a, t, _)| a == "second" && t == "items"),
        "Expected alias second → items, got: {:?}",
        aliases
    );
}


#[test]
fn test_destructuring_typescript() {
    // TypeScript destructuring should work identically to JavaScript
    let source = r#"
function process(device: Device): void {
    const { name, id }: { name: string; id: number } = device;
    console.log(name);
}
"#;
    let path = "src/process.ts";
    let parsed = ParsedFile::parse(path, source, Language::TypeScript).unwrap();

    let func_node = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=5).collect();
    let aliases = parsed.collect_alias_assignments(&func_node, &lines);

    assert!(
        aliases
            .iter()
            .any(|(a, t, _)| a == "name" && t == "device.name"),
        "Expected TS alias name → device.name, got: {:?}",
        aliases
    );
}


#[test]
fn test_destructuring_no_false_positive_js() {
    // Two independent destructurings should not cross-contaminate
    let source = r#"
function process(a, b) {
    const { x } = a;
    const { x: y } = b;
    console.log(x, y);
}
"#;
    let path = "src/process.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();

    let func_node = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=6).collect();
    let aliases = parsed.collect_alias_assignments(&func_node, &lines);

    assert!(
        aliases.iter().any(|(a, t, _)| a == "x" && t == "a.x"),
        "Expected alias x → a.x, got: {:?}",
        aliases
    );
    assert!(
        aliases.iter().any(|(a, t, _)| a == "y" && t == "b.x"),
        "Expected alias y → b.x, got: {:?}",
        aliases
    );
    // Ensure no cross-contamination
    assert!(
        !aliases.iter().any(|(a, t, _)| a == "x" && t == "b.x"),
        "x should NOT alias b.x"
    );
}


#[test]
fn test_destructuring_through_alias_js() {
    // const ref = obj; const { name } = ref → name should resolve to obj.name
    let source = r#"
function process(obj) {
    const ref = obj;
    const { name, id } = ref;
    console.log(name);
}
"#;
    let path = "src/through.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let obj_defs = dfg.all_defs_of(path, "obj");

    // name should resolve through ref → obj, producing a def for obj.name
    let has_obj_name = obj_defs
        .iter()
        .any(|d| d.path.base == "obj" && d.path.fields == vec!["name"]);
    assert!(
        has_obj_name,
        "Destructuring through alias: name via ref should resolve to obj.name. Got defs: {:?}",
        obj_defs.iter().map(|d| &d.path).collect::<Vec<_>>()
    );
}


#[test]
fn test_destructuring_through_chain_js() {
    // const a = obj; const b = a; const { x } = b → x should resolve to obj.x
    let source = r#"
function process(obj) {
    const a = obj;
    const b = a;
    const { x } = b;
    console.log(x);
}
"#;
    let path = "src/chain.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let obj_defs = dfg.all_defs_of(path, "obj");

    let has_obj_x = obj_defs
        .iter()
        .any(|d| d.path.base == "obj" && d.path.fields == vec!["x"]);
    assert!(
        has_obj_x,
        "Destructuring through chain: x via b=a=obj should resolve to obj.x. Got defs: {:?}",
        obj_defs.iter().map(|d| &d.path).collect::<Vec<_>>()
    );
}


#[test]
fn test_destructuring_nested_through_alias_js() {
    // const ref = obj; const { config: { host } } = ref → host should resolve to obj.config.host
    let source = r#"
function connect(obj) {
    const ref = obj;
    const { config: { host } } = ref;
    open(host);
}
"#;
    let path = "src/nested_alias.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let obj_defs = dfg.all_defs_of(path, "obj");

    let has_obj_config_host = obj_defs
        .iter()
        .any(|d| d.path.base == "obj" && d.path.fields == vec!["config", "host"]);
    assert!(
        has_obj_config_host,
        "Nested destructuring through alias: host should resolve to obj.config.host. Got defs: {:?}",
        obj_defs.iter().map(|d| &d.path).collect::<Vec<_>>()
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


#[test]
fn test_js_for_of_destructuring_def() {
    // Gap 3: for-of destructuring should produce defs for destructured variables
    let source = r#"
function process(items) {
    for (const { name, id } of items) {
        use(name);
        use(id);
    }
}
"#;
    let path = "src/process.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let name_defs = dfg.all_defs_of(path, "name");
    let id_defs = dfg.all_defs_of(path, "id");
    assert!(
        !name_defs.is_empty(),
        "for-of destructuring: 'name' should have def"
    );
    assert!(
        !id_defs.is_empty(),
        "for-of destructuring: 'id' should have def"
    );
}


#[test]
fn test_js_for_of_array_destructuring_def() {
    let source = r#"
function process(pairs) {
    for (const [key, value] of pairs) {
        use(key);
        use(value);
    }
}
"#;
    let path = "src/process.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let key_defs = dfg.all_defs_of(path, "key");
    let value_defs = dfg.all_defs_of(path, "value");
    assert!(
        !key_defs.is_empty(),
        "for-of array destructuring: 'key' should have def"
    );
    assert!(
        !value_defs.is_empty(),
        "for-of array destructuring: 'value' should have def"
    );
}


#[test]
fn test_js_for_in_simple_def() {
    let source = r#"
function listKeys(obj) {
    for (const key in obj) {
        use(key);
    }
}
"#;
    let path = "src/keys.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let key_defs = dfg.all_defs_of(path, "key");
    assert!(!key_defs.is_empty(), "for-in: 'key' should have def");
}


#[test]
fn test_js_for_of_destructuring_aliases() {
    // Destructured variables in for-of should create aliases to iterable.property
    let source = r#"
function process(items) {
    for (const { name, id } of items) {
        use(name);
    }
}
"#;
    let path = "src/process.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=6).collect();

    let aliases = parsed.collect_alias_assignments(&func, &lines);
    let name_alias = aliases.iter().any(|(a, _t, _l)| a == "name");
    let id_alias = aliases.iter().any(|(a, _t, _l)| a == "id");
    assert!(
        name_alias,
        "for-of destructuring: 'name' should have alias, got {:?}",
        aliases
    );
    assert!(
        id_alias,
        "for-of destructuring: 'id' should have alias, got {:?}",
        aliases
    );
}


#[test]
fn test_js_array_destructuring_rest_alias() {
    // Array destructuring with rest: const [first, ...rest] = arr
    let source = r#"
function process(arr) {
    const [first, ...rest] = arr;
    use(first);
    use(rest);
}
"#;
    let path = "src/process.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=5).collect();

    let aliases = parsed.collect_alias_assignments(&func, &lines);
    let first_alias = aliases.iter().any(|(a, _t, _l)| a == "first");
    let rest_alias = aliases.iter().any(|(a, _t, _l)| a == "rest");
    assert!(
        first_alias,
        "Array rest: 'first' should have alias, got {:?}",
        aliases
    );
    assert!(
        rest_alias,
        "Array rest: 'rest' should have alias, got {:?}",
        aliases
    );
}


#[test]
fn test_js_for_of_array_destructuring_aliases() {
    // Array destructuring in for-of: const [a, b] of pairs
    let source = r#"
function process(pairs) {
    for (const [a, b] of pairs) {
        use(a);
        use(b);
    }
}
"#;
    let path = "src/process.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=6).collect();

    let aliases = parsed.collect_alias_assignments(&func, &lines);
    let a_alias = aliases.iter().any(|(a, _t, _l)| a == "a");
    let b_alias = aliases.iter().any(|(a, _t, _l)| a == "b");
    assert!(
        a_alias,
        "for-of array destructuring: 'a' should have alias, got {:?}",
        aliases
    );
    assert!(
        b_alias,
        "for-of array destructuring: 'b' should have alias, got {:?}",
        aliases
    );
}

// ====== JS Algorithm Coverage: 12 "none" algorithms ======

#[test]
fn test_parent_function_javascript() {
    let (files, _, diff) = make_javascript_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty(), "ParentFunction JS should produce blocks");
    // ParentFunction should include the enclosing function of the diff lines
    let block = &result.blocks[0];
    assert!(
        block.file_line_map.contains_key("src/api.js"),
        "Should reference the JS file"
    );
}

#[test]
fn test_thin_slice_javascript() {
    let (files, _, diff) = make_javascript_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty(), "ThinSlice JS should produce blocks");
    assert_eq!(result.algorithm, SlicingAlgorithm::ThinSlice);
}

#[test]
fn test_relevant_slice_javascript() {
    let (files, _, diff) = make_javascript_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::RelevantSlice),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty(), "RelevantSlice JS should produce blocks");
    assert_eq!(result.algorithm, SlicingAlgorithm::RelevantSlice);
}

#[test]
fn test_symmetry_slice_javascript() {
    let source = r#"
function serialize(obj) {
    return JSON.stringify(obj);
}

function deserialize(str) {
    return JSON.parse(str);
}
"#;
    let path = "serializer.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::SymmetrySlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::SymmetrySlice);
}

#[test]
fn test_gradient_slice_javascript() {
    let (files, _, diff) = make_javascript_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::GradientSlice),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty(), "GradientSlice JS should produce blocks");
    assert_eq!(result.algorithm, SlicingAlgorithm::GradientSlice);
}

#[test]
fn test_circular_slice_javascript() {
    let source = r#"
function ping(n) {
    return pong(n + 1);
}

function pong(n) {
    return ping(n - 1);
}
"#;
    let path = "cycle.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::CircularSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::CircularSlice);
}

#[test]
fn test_spiral_slice_javascript() {
    let source = r#"
function inner(x) {
    return x + 1;
}

function outer(y) {
    const z = inner(y);
    return z * 2;
}

function caller() {
    const r = outer(10);
    console.log(r);
}
"#;
    let path = "spiral.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([7]),
        }],
    };

    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::SpiralSlice);
    let spiral_config = prism::algorithms::spiral_slice::SpiralConfig {
        max_ring: 4,
        auto_stop_threshold: 0.0,
    };
    let ctx = CpgContext::build(&files, None);
    let result =
        prism::algorithms::spiral_slice::slice(&ctx, &diff, &config, &spiral_config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::SpiralSlice);
    assert!(!result.blocks.is_empty(), "SpiralSlice JS should produce blocks");
}

#[test]
fn test_vertical_slice_javascript() {
    let source_handler = r#"
function apiHandler(req) {
    const data = req.body;
    const result = serviceProcess(data);
    return result;
}
"#;
    let source_service = r#"
function serviceProcess(data) {
    const validated = validate(data);
    return repoSave(validated);
}
"#;
    let source_repo = r#"
function repoSave(data) {
    db.insert(data);
    return true;
}
"#;
    let handler_path = "handler/api.js";
    let service_path = "service/processor.js";
    let repo_path = "repository/store.js";

    let mut files = BTreeMap::new();
    files.insert(
        handler_path.to_string(),
        ParsedFile::parse(handler_path, source_handler, Language::JavaScript).unwrap(),
    );
    files.insert(
        service_path.to_string(),
        ParsedFile::parse(service_path, source_service, Language::JavaScript).unwrap(),
    );
    files.insert(
        repo_path.to_string(),
        ParsedFile::parse(repo_path, source_repo, Language::JavaScript).unwrap(),
    );

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: service_path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::vertical_slice::slice(
        &ctx,
        &diff,
        &prism::algorithms::vertical_slice::VerticalConfig::default(),
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::VerticalSlice);
}

#[test]
fn test_delta_slice_javascript() {
    let tmp = TempDir::new().unwrap();

    let old_source = "function add(a, b) {\n    return a + b;\n}\n";
    std::fs::write(tmp.path().join("calc.js"), old_source).unwrap();

    let new_source = "function add(a, b) {\n    const result = a + b;\n    return result;\n}\n";
    let path = "calc.js";
    let parsed = ParsedFile::parse(path, new_source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2, 3]),
        }],
    };

    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::delta_slice::slice(&ctx, &diff, tmp.path()).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::DeltaSlice);
}

#[test]
fn test_resonance_slice_javascript() {
    let source = "function update(x) {\n    const y = x + 1;\n    return y;\n}\n";
    let filename = "app.js";
    let tmp = create_temp_git_repo(filename, &["function update(x) {\n    return x;\n}\n", source]);

    let parsed = ParsedFile::parse(filename, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(filename.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: filename.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2]),
        }],
    };

    let config = prism::algorithms::resonance_slice::ResonanceConfig {
        git_dir: tmp.path().to_string_lossy().to_string(),
        days: 365,
        min_co_changes: 1,
        min_ratio: 0.0,
    };
    let result = prism::algorithms::resonance_slice::slice(&files, &diff, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ResonanceSlice);
}

#[test]
fn test_phantom_slice_javascript() {
    let source = "function remaining(x) {\n    return x + 1;\n}\n";
    let filename = "app.js";
    let tmp = create_temp_git_repo(
        filename,
        &[
            "function deleted(x) {\n    return x * 2;\n}\n\nfunction remaining(x) {\n    return x + 1;\n}\n",
            source,
        ],
    );
    let parsed = ParsedFile::parse(filename, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(filename.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: filename.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2]),
        }],
    };
    let config = prism::algorithms::phantom_slice::PhantomConfig {
        git_dir: tmp.path().to_string_lossy().to_string(),
        max_commits: 50,
    };
    let result = prism::algorithms::phantom_slice::slice(&files, &diff, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::PhantomSlice);
}

#[test]
fn test_threed_slice_javascript() {
    let source = "function foo(x) {\n    const y = x + 1;\n    return y;\n}\n\nfunction bar() {\n    const r = foo(10);\n    console.log(r);\n}\n";
    let filename = "app.js";
    let tmp = create_temp_git_repo(filename, &["function foo(x) {\n    return x;\n}\n", source]);

    let parsed = ParsedFile::parse(filename, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(filename.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: filename.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2, 3]),
        }],
    };

    let config = prism::algorithms::threed_slice::ThreeDConfig {
        temporal_days: 365,
        git_dir: tmp.path().to_string_lossy().to_string(),
    };
    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::threed_slice::slice(&ctx, &diff, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ThreeDSlice);
}

// ====== JS "basic" → "full" coverage upgrades (8 algorithms) ======

#[test]
fn test_absence_slice_javascript_arrow_fn() {
    // Test absence detection with arrow functions and callbacks
    let source = r#"
const processFile = (path) => {
    const fd = fs.openSync(path, 'r');
    const data = fs.readFileSync(fd);
    return data;
};
"#;
    let path = "file.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AbsenceSlice);
    // Absence should detect missing closeSync for openSync
    let block = &result.blocks[0];
    assert!(
        block.file_line_map.contains_key(path),
        "AbsenceSlice should reference the JS file"
    );
}

#[test]
fn test_absence_slice_javascript_lock_unlock() {
    let source = r#"
function critical(mutex) {
    mutex.lock();
    const result = compute();
    return result;
}
"#;
    let path = "sync.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AbsenceSlice);
}

#[test]
fn test_angle_slice_javascript_error_handling() {
    // Angle slice with cross-cutting error handling concern
    let source = r#"
function fetchUser(id) {
    try {
        const response = fetch("/user/" + id);
        return response.json();
    } catch (error) {
        console.error("Failed to fetch user:", error);
        throw error;
    }
}

function fetchOrder(id) {
    try {
        const response = fetch("/order/" + id);
        return response.json();
    } catch (error) {
        console.error("Failed to fetch order:", error);
        throw error;
    }
}
"#;
    let path = "api.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([7]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AngleSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AngleSlice);
}

#[test]
fn test_barrier_slice_javascript_call_depth() {
    // Test barrier slice with call chain in JS
    let source = r#"
function level0(x) {
    return level1(x + 1);
}

function level1(y) {
    return level2(y * 2);
}

function level2(z) {
    return z + 10;
}
"#;
    let path = "chain.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::BarrierSlice),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty(), "BarrierSlice JS should produce blocks for call chain");
    assert_eq!(result.algorithm, SlicingAlgorithm::BarrierSlice);
}

#[test]
fn test_chop_javascript_async_pipeline() {
    let source = r#"
async function pipeline(input) {
    const validated = validate(input);
    const transformed = await transform(validated);
    const result = await save(transformed);
    return result;
}
function validate(x) { return x; }
async function transform(x) { return x; }
async function save(x) { return x; }
"#;
    let path = "pipeline.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let config = prism::algorithms::chop::ChopConfig {
        source_file: path.to_string(),
        source_line: 3,
        sink_file: path.to_string(),
        sink_line: 5,
    };
    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::chop::slice(&ctx, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::Chop);
}

#[test]
fn test_conditioned_slice_javascript_ternary() {
    let source = r#"
function classify(score) {
    const grade = score >= 90 ? "A" : score >= 80 ? "B" : "C";
    const pass = score >= 60;
    return { grade, pass };
}
"#;
    let path = "grades.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ConditionedSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ConditionedSlice);
}

#[test]
fn test_echo_slice_javascript_multifile() {
    // Multi-file echo: API change should ripple to callers
    let source_lib = r#"
function validateInput(data) {
    if (!data || !data.name) {
        throw new Error("invalid data");
    }
    return data;
}

function formatOutput(result) {
    return JSON.stringify(result);
}
"#;
    let source_caller1 = r#"
function handler1() {
    const data = validateInput(getRequest());
    const formatted = formatOutput(data);
    return formatted;
}
"#;
    let source_caller2 = r#"
function handler2() {
    const data = validateInput(getInput());
    return data;
}
"#;
    let mut files = BTreeMap::new();
    files.insert(
        "lib.js".to_string(),
        ParsedFile::parse("lib.js", source_lib, Language::JavaScript).unwrap(),
    );
    files.insert(
        "handler1.js".to_string(),
        ParsedFile::parse("handler1.js", source_caller1, Language::JavaScript).unwrap(),
    );
    files.insert(
        "handler2.js".to_string(),
        ParsedFile::parse("handler2.js", source_caller2, Language::JavaScript).unwrap(),
    );
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "lib.js".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3, 4]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::EchoSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::EchoSlice);
}

#[test]
fn test_horizontal_slice_javascript_class_methods() {
    let source = r#"
class UserService {
    async getUser(id) {
        return this.db.find(id);
    }

    async createUser(data) {
        return this.db.insert(data);
    }

    async deleteUser(id) {
        return this.db.remove(id);
    }
}
"#;
    let path = "service.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::HorizontalSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::HorizontalSlice);
}

#[test]
fn test_membrane_slice_javascript_multifile_callers() {
    // Changed API function with multiple cross-file callers
    let source_api = r#"
function fetchUser(id) {
    const user = db.get(id);
    if (!user) throw new Error("not found");
    return user;
}

function deleteUser(id) {
    return db.delete(id);
}
"#;
    let source_caller1 = r#"
function showProfile(id) {
    const user = fetchUser(id);
    render(user);
}
"#;
    let source_caller2 = r#"
function adminView(id) {
    const user = fetchUser(id);
    const canDelete = user.role !== "admin";
    return { user, canDelete };
}
"#;
    let mut files = BTreeMap::new();
    files.insert(
        "api.js".to_string(),
        ParsedFile::parse("api.js", source_api, Language::JavaScript).unwrap(),
    );
    files.insert(
        "profile.js".to_string(),
        ParsedFile::parse("profile.js", source_caller1, Language::JavaScript).unwrap(),
    );
    files.insert(
        "admin.js".to_string(),
        ParsedFile::parse("admin.js", source_caller2, Language::JavaScript).unwrap(),
    );
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "api.js".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3, 4]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::MembraneSlice);
}

