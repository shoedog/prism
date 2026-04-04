#[path = "../../common/mod.rs"]
mod common;
use common::*;

// ---------------------------------------------------------------------------
// LeftFlow (Algorithm 8) — Bash
// ---------------------------------------------------------------------------

#[test]
fn test_left_flow_bash() {
    let (files, _, diff) = make_bash_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce blocks for Bash code"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::LeftFlow);
}

// ---------------------------------------------------------------------------
// FullFlow (Algorithm 9) — Bash
// ---------------------------------------------------------------------------

#[test]
fn test_full_flow_bash() {
    let (files, _, diff) = make_bash_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "FullFlow should produce blocks for Bash code"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::FullFlow);
}

// ---------------------------------------------------------------------------
// ThinSlice — Bash
// ---------------------------------------------------------------------------

#[test]
fn test_thin_slice_bash() {
    let (files, _, diff) = make_bash_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "ThinSlice should produce blocks for Bash code"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::ThinSlice);
}

// ---------------------------------------------------------------------------
// RelevantSlice — Bash
// ---------------------------------------------------------------------------

#[test]
fn test_relevant_slice_bash() {
    let (files, _, diff) = make_bash_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::RelevantSlice),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "RelevantSlice should produce blocks for Bash code"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::RelevantSlice);
}

// ---------------------------------------------------------------------------
// AngleSlice — Bash
// ---------------------------------------------------------------------------

#[test]
fn test_angle_slice_bash() {
    let (files, _, diff) = make_bash_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AngleSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AngleSlice);
}

// ---------------------------------------------------------------------------
// EchoSlice — Bash: changed function output affecting callers
// ---------------------------------------------------------------------------

#[test]
fn test_echo_slice_bash() {
    let source_lib = r#"#!/bin/bash

get_status() {
    echo "OK"
}
"#;
    let source_caller = r#"#!/bin/bash

check_health() {
    get_status
    echo "checked"
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "scripts/lib.sh".to_string(),
        ParsedFile::parse("scripts/lib.sh", source_lib, Language::Bash).unwrap(),
    );
    files.insert(
        "scripts/caller.sh".to_string(),
        ParsedFile::parse("scripts/caller.sh", source_caller, Language::Bash).unwrap(),
    );

    // Diff on the echo "OK" line inside get_status (line 4)
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "scripts/lib.sh".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
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

// ---------------------------------------------------------------------------
// HorizontalSlice — Bash: peer handler functions with similar patterns
// ---------------------------------------------------------------------------

#[test]
fn test_horizontal_slice_bash() {
    let source = r#"#!/bin/bash

handle_start() {
    echo "starting service"
    systemctl start myapp
}

handle_stop() {
    echo "stopping service"
    systemctl stop myapp
}

handle_restart() {
    echo "restarting service"
    systemctl restart myapp
}
"#;

    let path = "service.sh";
    let parsed = ParsedFile::parse(path, source, Language::Bash).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff on handle_start body (lines 4-5)
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4, 5]),
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
    let has_peers = !result.blocks.is_empty() || !result.findings.is_empty();
    assert!(
        has_peers,
        "HorizontalSlice should find peer handler functions (handle_stop, handle_restart)"
    );
}

// ---------------------------------------------------------------------------
// BarrierSlice — Bash: function call chain with depth limit
// ---------------------------------------------------------------------------

#[test]
fn test_barrier_slice_bash() {
    let source = r#"#!/bin/bash

level0() {
    level1 "$1"
}

level1() {
    level2 "$1"
}

level2() {
    echo "$1"
}
"#;

    let path = "chain.sh";
    let parsed = ParsedFile::parse(path, source, Language::Bash).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff on level0 body (line 4)
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::BarrierSlice),
        None,
    )
    .unwrap();

    assert_eq!(result.algorithm, SlicingAlgorithm::BarrierSlice);
}
