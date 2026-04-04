#[path = "../../common/mod.rs"]
mod common;
use common::*;

// ── Shared Lua fixture ──

fn lua_fixture() -> (BTreeMap<String, ParsedFile>, DiffInput) {
    let source = r#"
local function process(data)
    local val = data
    local result = val * 2
    return result
end

function handle_request(req)
    local response = process(req)
    return response
end
"#;
    let path = "scripts/handler.lua";
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
    (files, diff)
}

// ── Paper algorithms ──

#[test]
fn test_original_diff_lua() {
    let (files, diff) = lua_fixture();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "OriginalDiff should produce blocks for Lua code"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::OriginalDiff);
}

#[test]
fn test_full_flow_lua() {
    let (files, diff) = lua_fixture();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "FullFlow should produce blocks for Lua code"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::FullFlow);
}

// ── Taxonomy algorithms ──

#[test]
fn test_relevant_slice_lua() {
    let (files, diff) = lua_fixture();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::RelevantSlice),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "RelevantSlice should produce blocks for Lua code"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::RelevantSlice);
}

#[test]
fn test_barrier_slice_lua() {
    let source = r#"
function level0(x)
    return level1(x + 1)
end

function level1(y)
    return level2(y * 2)
end

function level2(z)
    return z + 10
end
"#;
    let path = "scripts/chain.lua";
    let parsed = ParsedFile::parse(path, source, Language::Lua).unwrap();
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
    assert!(
        !result.blocks.is_empty(),
        "BarrierSlice should produce blocks for Lua call chain"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::BarrierSlice);
}

#[test]
fn test_chop_lua() {
    let source = r#"
function pipeline(input)
    local validated = validate(input)
    local transformed = transform(validated)
    local result = save(transformed)
    return result
end

function validate(x) return x end
function transform(x) return x end
function save(x) return x end
"#;
    let path = "scripts/pipeline.lua";
    let parsed = ParsedFile::parse(path, source, Language::Lua).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let config = prism::algorithms::chop::ChopConfig {
        source_file: path.to_string(),
        source_line: 3,
        sink_file: path.to_string(),
        sink_line: 6,
    };
    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::chop::slice(&ctx, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::Chop);
}

#[test]
fn test_conditioned_slice_lua() {
    let source = r#"
function classify(score)
    local grade
    if score >= 90 then
        grade = "A"
    elseif score >= 80 then
        grade = "B"
    else
        grade = "C"
    end
    return grade
end
"#;
    let path = "scripts/grades.lua";
    let parsed = ParsedFile::parse(path, source, Language::Lua).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5]),
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

// ── Novel algorithms ──

#[test]
fn test_echo_slice_lua() {
    let source_lib = r#"
function compute(x)
    return x * 2
end
"#;
    let source_caller = r#"
function caller()
    local val = compute(42)
    print(val)
end
"#;
    let mut files = BTreeMap::new();
    files.insert(
        "scripts/lib.lua".to_string(),
        ParsedFile::parse("scripts/lib.lua", source_lib, Language::Lua).unwrap(),
    );
    files.insert(
        "scripts/caller.lua".to_string(),
        ParsedFile::parse("scripts/caller.lua", source_caller, Language::Lua).unwrap(),
    );
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "scripts/lib.lua".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
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
fn test_symmetry_slice_lua() {
    let source = r#"
function encode(data)
    return string.format("%q", data)
end

function decode(s)
    return loadstring("return " .. s)()
end
"#;
    let path = "scripts/codec.lua";
    let parsed = ParsedFile::parse(path, source, Language::Lua).unwrap();
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

// ── Theoretical algorithms ──

#[test]
fn test_horizontal_slice_lua() {
    let source = r#"
function handle_get(req)
    return {status = 200, body = req}
end

function handle_post(req)
    return {status = 201, body = req}
end

function handle_delete(req)
    return {status = 204, body = req}
end
"#;
    let path = "scripts/handlers.lua";
    let parsed = ParsedFile::parse(path, source, Language::Lua).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::HorizontalSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::HorizontalSlice);
}

#[test]
fn test_angle_slice_lua() {
    let source = r#"
function read_file(path)
    local ok, err = pcall(function()
        local f = io.open(path, "r")
        local content = f:read("*a")
        f:close()
        return content
    end)
    if not ok then
        print("Error reading file: " .. tostring(err))
        return nil
    end
    return err
end

function write_file(path, data)
    local ok, err = pcall(function()
        local f = io.open(path, "w")
        f:write(data)
        f:close()
    end)
    if not ok then
        print("Error writing file: " .. tostring(err))
    end
end
"#;
    let path = "scripts/fileio.lua";
    let parsed = ParsedFile::parse(path, source, Language::Lua).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([10]),
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
