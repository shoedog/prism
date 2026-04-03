#[path = "../../common/mod.rs"]
mod common;
use common::*;

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
