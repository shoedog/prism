#[path = "../../common/mod.rs"]
mod common;
use common::*;

#[test]
fn test_lua_basic_parsing() {
    let source = r#"
local function process_packet(data)
    local result = data
    return result
end

function handle_request(req)
    local response = process_packet(req)
    return response
end
"#;
    let path = "scripts/handler.lua";
    let parsed = ParsedFile::parse(path, source, Language::Lua).unwrap();

    let funcs = parsed.all_functions();
    let func_names: Vec<String> = funcs
        .iter()
        .filter_map(|f| {
            parsed
                .language
                .function_name(f)
                .map(|n| parsed.node_text(&n).to_string())
        })
        .collect();
    assert!(
        func_names.contains(&"process_packet".to_string()),
        "Should find process_packet function, got: {:?}",
        func_names
    );
    assert!(
        func_names.contains(&"handle_request".to_string()),
        "Should find handle_request function, got: {:?}",
        func_names
    );
}


#[test]
fn test_lua_parent_function() {
    let source = r#"
local function process(data)
    local val = data
    local result = val
    return result
end
"#;
    let path = "scripts/process.lua";
    let parsed = ParsedFile::parse(path, source, Language::Lua).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "ParentFunction should include the enclosing Lua function"
    );
}


#[test]
fn test_dfg_lua_field_access_paths() {
    // Lua dot_index_expression: obj.field
    let source = r#"
function setup(config)
    config.timeout = 30
    config.host = "localhost"
end
"#;
    let path = "src/config.lua";
    let parsed = ParsedFile::parse(path, source, Language::Lua).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let config_defs = dfg.all_defs_of(path, "config");

    let has_timeout = config_defs
        .iter()
        .any(|d| d.path.has_fields() && d.path.fields.contains(&"timeout".to_string()));
    assert!(
        has_timeout,
        "Lua DFG should have AccessPath config.timeout from dot_index_expression. Got: {:?}",
        config_defs
            .iter()
            .map(|d| d.path.to_string())
            .collect::<Vec<_>>()
    );
}


#[test]
fn test_lua_colon_method_field_access() {
    // obj:close() uses method_index_expression — should be recognized as field access
    let source = r#"
function cleanup(file)
    file:close()
    file:flush()
end
"#;
    let path = "src/cleanup.lua";
    let parsed = ParsedFile::parse(path, source, Language::Lua).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3, 4]),
        }],
    };

    // LeftFlow should trace file:close() back to the file parameter
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce blocks for Lua colon method calls"
    );
}


#[test]
fn test_lua_assignment_target_and_value() {
    // Lua assignment_target walks variable_list, assignment_value walks expression_list
    let source = r#"
local function process()
    local x = 10
    x = 20
    use(x)
end
"#;
    let path = "src/lua.lua";
    let parsed = ParsedFile::parse(path, source, Language::Lua).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let x_defs = dfg.all_defs_of(path, "x");
    assert!(
        !x_defs.is_empty(),
        "Lua: 'x' should have defs from local and assignment"
    );
}


#[test]
fn test_lua_alias_assignment() {
    // Tests Lua assignment_target/assignment_value via alias tracking
    let source = r#"
local function process()
    local a = external
    local b = a
    use(b)
end
"#;
    let path = "src/lua_alias.lua";
    let parsed = ParsedFile::parse(path, source, Language::Lua).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=6).collect();

    let aliases = parsed.collect_alias_assignments(&func, &lines);
    let b_aliases_a = aliases.iter().any(|(a, t, _)| a == "b" && t == "a");
    assert!(b_aliases_a, "Lua: 'b' should alias 'a', got: {:?}", aliases);
}

