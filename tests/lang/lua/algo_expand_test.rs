//! Expanded algorithm coverage tests for Lua language.
//!
//! Covers 9 algorithms not yet tested with Lua fixtures:
//! LeftFlow, DeltaSlice, SpiralSlice, CircularSlice, VerticalSlice,
//! ThreeDSlice, ResonanceSlice, GradientSlice, PhantomSlice.

#[path = "../../common/mod.rs"]
mod common;
use common::*;

// ---------------------------------------------------------------------------
// LeftFlow — variable tracing in Lua scripts
// ---------------------------------------------------------------------------

#[test]
fn test_left_flow_lua() {
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

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce blocks for Lua code"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::LeftFlow);
}

// ---------------------------------------------------------------------------
// DeltaSlice — changed LuCI handler between versions
// ---------------------------------------------------------------------------

#[test]
fn test_delta_slice_lua() {
    let tmp = TempDir::new().unwrap();

    let old_source = "function handle(req)\n    return req\nend\n";
    std::fs::write(tmp.path().join("handler.lua"), old_source).unwrap();

    let new_source =
        "function handle(req)\n    local validated = validate(req)\n    return validated\nend\n";
    let path = "handler.lua";
    let parsed = ParsedFile::parse(path, new_source, Language::Lua).unwrap();
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

// ---------------------------------------------------------------------------
// SpiralSlice — adaptive depth around changed function
// ---------------------------------------------------------------------------

#[test]
fn test_spiral_slice_lua() {
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

    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::SpiralSlice);
    let spiral_config = prism::algorithms::spiral_slice::SpiralConfig {
        max_ring: 4,
        auto_stop_threshold: 0.0,
    };
    let ctx = CpgContext::build(&files, None);
    let result =
        prism::algorithms::spiral_slice::slice(&ctx, &diff, &config, &spiral_config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::SpiralSlice);
}

// ---------------------------------------------------------------------------
// CircularSlice — circular module requires
// ---------------------------------------------------------------------------

#[test]
fn test_circular_slice_lua() {
    let source = r#"
function ping(n)
    if n <= 0 then return 0 end
    return pong(n - 1)
end

function pong(n)
    if n <= 0 then return 0 end
    return ping(n - 1)
end
"#;
    let path = "scripts/pingpong.lua";
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

    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::circular_slice::slice(&ctx, &diff).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::CircularSlice);
}

// ---------------------------------------------------------------------------
// VerticalSlice — request → controller → model → UCI (LuCI MVC)
// ---------------------------------------------------------------------------

#[test]
fn test_vertical_slice_lua() {
    let source_ctrl = r#"
function controller_dispatch(req)
    local data = model_fetch(req.id)
    return view_render(data)
end
"#;
    let source_model = r#"
function model_fetch(id)
    return {id = id, name = "item"}
end

function view_render(data)
    return tostring(data.name)
end
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "scripts/controller.lua".to_string(),
        ParsedFile::parse("scripts/controller.lua", source_ctrl, Language::Lua).unwrap(),
    );
    files.insert(
        "scripts/model.lua".to_string(),
        ParsedFile::parse("scripts/model.lua", source_model, Language::Lua).unwrap(),
    );

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "scripts/controller.lua".to_string(),
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

// ---------------------------------------------------------------------------
// ThreeDSlice — temporal-structural risk (requires git)
// ---------------------------------------------------------------------------

#[test]
fn test_threed_slice_lua() {
    let source_v1 = "function setup()\n    print(\"v1\")\nend\n";
    let source_v2 = "function setup()\n    print(\"v2\")\n    log(\"setup\")\nend\n";
    let filename = "setup.lua";
    let tmp = create_temp_git_repo(filename, &[source_v1, source_v2]);

    let parsed = ParsedFile::parse(filename, source_v2, Language::Lua).unwrap();
    let mut files = BTreeMap::new();
    files.insert(filename.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: filename.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2]),
        }],
    };
    let ctx = CpgContext::build(&files, None);
    let config = prism::algorithms::threed_slice::ThreeDConfig {
        git_dir: tmp.path().to_string_lossy().to_string(),
        temporal_days: 365,
    };
    let result = prism::algorithms::threed_slice::slice(&ctx, &diff, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ThreeDSlice);
}

// ---------------------------------------------------------------------------
// ResonanceSlice — git co-change coupling
// ---------------------------------------------------------------------------

#[test]
fn test_resonance_slice_lua() {
    let source_v1 = "function init()\n    print(\"v1\")\nend\n";
    let source_v2 = "function init()\n    print(\"v2\")\n    setup()\nend\n";
    let filename = "init.lua";
    let tmp = create_temp_git_repo(filename, &[source_v1, source_v2]);

    let parsed = ParsedFile::parse(filename, source_v2, Language::Lua).unwrap();
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

// ---------------------------------------------------------------------------
// GradientSlice — continuous relevance scoring
// ---------------------------------------------------------------------------

#[test]
fn test_gradient_slice_lua() {
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

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::GradientSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::GradientSlice);
}

// ---------------------------------------------------------------------------
// PhantomSlice — recently deleted code (requires git)
// ---------------------------------------------------------------------------

#[test]
fn test_phantom_slice_lua() {
    let source_v1 = "function cleanup()\n    os.remove(\"/tmp/work\")\nend\nfunction work()\n    cleanup()\nend\n";
    let source_v2 = "function work()\n    -- simplified\nend\n";
    let filename = "phantom.lua";
    let tmp = create_temp_git_repo(filename, &[source_v1, source_v2]);

    let parsed = ParsedFile::parse(filename, source_v2, Language::Lua).unwrap();
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
