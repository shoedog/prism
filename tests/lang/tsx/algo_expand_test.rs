//! Expanded algorithm coverage tests for TSX language.
//!
//! Covers 11 algorithms not yet tested with TSX fixtures:
//! Taint, ConditionedSlice, DeltaSlice, SpiralSlice, CircularSlice,
//! VerticalSlice, ThreeDSlice, ResonanceSlice, PhantomSlice, ContractSlice, Chop.

#[path = "../../common/mod.rs"]
mod common;
use common::*;

// ---------------------------------------------------------------------------
// Taint — user input in JSX expressions
// ---------------------------------------------------------------------------

#[test]
fn test_taint_tsx() {
    let source = r#"
import React from 'react';

function SearchPage({ query }: { query: string }) {
    const result = eval(query);
    return <div>{result}</div>;
}
"#;
    let path = "src/Search.tsx";
    let parsed = ParsedFile::parse(path, source, Language::Tsx).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::Taint);
}

// ---------------------------------------------------------------------------
// ConditionedSlice — conditional rendering
// ---------------------------------------------------------------------------

#[test]
fn test_conditioned_slice_tsx() {
    let source = r#"
import React from 'react';

function Status({ code }: { code: number }) {
    let message: string;
    if (code >= 200 && code < 300) {
        message = "Success";
    } else if (code >= 400) {
        message = "Error";
    } else {
        message = "Unknown";
    }
    return <span>{message}</span>;
}
"#;
    let path = "src/Status.tsx";
    let parsed = ParsedFile::parse(path, source, Language::Tsx).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ConditionedSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ConditionedSlice);
}

// ---------------------------------------------------------------------------
// DeltaSlice — changed component between versions
// ---------------------------------------------------------------------------

#[test]
fn test_delta_slice_tsx() {
    let tmp = TempDir::new().unwrap();

    let old_source = "function App() {\n    return <div>Hello</div>;\n}\n";
    std::fs::write(tmp.path().join("App.tsx"), old_source).unwrap();

    let new_source =
        "function App() {\n    const msg = \"Hello World\";\n    return <div>{msg}</div>;\n}\n";
    let path = "App.tsx";
    let parsed = ParsedFile::parse(path, new_source, Language::Tsx).unwrap();
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
// SpiralSlice — adaptive depth
// ---------------------------------------------------------------------------

#[test]
fn test_spiral_slice_tsx() {
    let (files, _, diff) = make_tsx_test();
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
// CircularSlice — mutual function calls
// ---------------------------------------------------------------------------

#[test]
fn test_circular_slice_tsx() {
    let source = r#"
function ping(n: number): number {
    if (n <= 0) return 0;
    return pong(n - 1);
}

function pong(n: number): number {
    if (n <= 0) return 0;
    return ping(n - 1);
}

function App() {
    return <div>{ping(5)}</div>;
}
"#;
    let path = "src/App.tsx";
    let parsed = ParsedFile::parse(path, source, Language::Tsx).unwrap();
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
// VerticalSlice — component hierarchy
// ---------------------------------------------------------------------------

#[test]
fn test_vertical_slice_tsx() {
    let (files, _, diff) = make_tsx_test();
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
fn test_threed_slice_tsx() {
    let source_v1 = "function App() {\n    return <div>v1</div>;\n}\n";
    let source_v2 = "function App() {\n    return <div>v2</div>;\n}\n";
    let filename = "App.tsx";
    let tmp = create_temp_git_repo(filename, &[source_v1, source_v2]);

    let parsed = ParsedFile::parse(filename, source_v2, Language::Tsx).unwrap();
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
fn test_resonance_slice_tsx() {
    let source_v1 = "function App() {\n    return <div>v1</div>;\n}\n";
    let source_v2 = "function App() {\n    return <div>v2</div>;\n}\n";
    let filename = "App.tsx";
    let tmp = create_temp_git_repo(filename, &[source_v1, source_v2]);

    let parsed = ParsedFile::parse(filename, source_v2, Language::Tsx).unwrap();
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
// PhantomSlice — recently deleted code (requires git)
// ---------------------------------------------------------------------------

#[test]
fn test_phantom_slice_tsx() {
    let source_v1 = "function Helper() {\n    return <span>help</span>;\n}\nfunction App() {\n    return <Helper />;\n}\n";
    let source_v2 = "function App() {\n    return <div>simplified</div>;\n}\n";
    let filename = "App.tsx";
    let tmp = create_temp_git_repo(filename, &[source_v1, source_v2]);

    let parsed = ParsedFile::parse(filename, source_v2, Language::Tsx).unwrap();
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

// ---------------------------------------------------------------------------
// ContractSlice — guard validation in components
// ---------------------------------------------------------------------------

#[test]
fn test_contract_slice_tsx() {
    let source = r#"
function UserCard({ user }: { user: any }) {
    if (!user) return <div>No user</div>;
    if (!user.name) return <div>Invalid</div>;
    return <div>{user.name}</div>;
}
"#;
    let path = "src/UserCard.tsx";
    let parsed = ParsedFile::parse(path, source, Language::Tsx).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ContractSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ContractSlice);
}

// ---------------------------------------------------------------------------
// Chop — data flow between source and sink
// ---------------------------------------------------------------------------

#[test]
fn test_chop_tsx() {
    let source = r#"
function transform(input: string): string {
    const trimmed = input.trim();
    const upper = trimmed.toUpperCase();
    return upper;
}

function App() {
    return <div>{transform("hello")}</div>;
}
"#;
    let path = "src/App.tsx";
    let parsed = ParsedFile::parse(path, source, Language::Tsx).unwrap();
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
