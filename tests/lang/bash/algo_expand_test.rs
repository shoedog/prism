//! Expanded algorithm coverage tests for Bash language.
//!
//! Covers 13 algorithms not yet tested with Bash fixtures:
//! ConditionedSlice, DeltaSlice, SpiralSlice, CircularSlice, VerticalSlice,
//! ThreeDSlice, ResonanceSlice, SymmetrySlice, GradientSlice, PhantomSlice,
//! MembraneSlice, ContractSlice, Chop.

#[path = "../../common/mod.rs"]
mod common;
use common::*;

// ---------------------------------------------------------------------------
// ConditionedSlice — conditional execution paths in init scripts
// ---------------------------------------------------------------------------

#[test]
fn test_conditioned_slice_bash() {
    let source = r#"#!/bin/bash

configure_mode() {
    local mode="$1"
    if [ "$mode" = "bridge" ]; then
        brctl addbr br0
        echo "bridge mode"
    elif [ "$mode" = "router" ]; then
        iptables -t nat -A POSTROUTING -j MASQUERADE
        echo "router mode"
    else
        echo "unknown mode"
    fi
}
"#;
    let path = "config.sh";
    let parsed = ParsedFile::parse(path, source, Language::Bash).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([6, 9]),
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
// DeltaSlice — changed script behavior between versions
// ---------------------------------------------------------------------------

#[test]
fn test_delta_slice_bash() {
    let tmp = TempDir::new().unwrap();

    let old_source = "#!/bin/bash\nsetup() {\n    echo \"old\"\n}\n";
    std::fs::write(tmp.path().join("init.sh"), old_source).unwrap();

    let new_source = "#!/bin/bash\nsetup() {\n    echo \"new\"\n    logger \"setup complete\"\n}\n";
    let path = "init.sh";
    let parsed = ParsedFile::parse(path, new_source, Language::Bash).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3, 4]),
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
fn test_spiral_slice_bash() {
    let (files, _, diff) = make_bash_test();
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
// CircularSlice — recursive function calls
// ---------------------------------------------------------------------------

#[test]
fn test_circular_slice_bash() {
    let source = r#"#!/bin/bash

process_dir() {
    local dir="$1"
    for entry in "$dir"/*; do
        if [ -d "$entry" ]; then
            process_dir "$entry"
        else
            echo "$entry"
        fi
    done
}
"#;
    let path = "recurse.sh";
    let parsed = ParsedFile::parse(path, source, Language::Bash).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([7]),
        }],
    };

    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::circular_slice::slice(&ctx, &diff).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::CircularSlice);
}

// ---------------------------------------------------------------------------
// VerticalSlice — end-to-end feature path through script layers
// ---------------------------------------------------------------------------

#[test]
fn test_vertical_slice_bash() {
    let source_main = r#"#!/bin/bash

main() {
    validate_input "$1"
    process_data "$1"
}
"#;
    let source_lib = r#"#!/bin/bash

validate_input() {
    [ -n "$1" ] || exit 1
}

process_data() {
    echo "processing: $1"
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "scripts/main.sh".to_string(),
        ParsedFile::parse("scripts/main.sh", source_main, Language::Bash).unwrap(),
    );
    files.insert(
        "scripts/lib.sh".to_string(),
        ParsedFile::parse("scripts/lib.sh", source_lib, Language::Bash).unwrap(),
    );

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "scripts/main.sh".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4, 5]),
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
fn test_threed_slice_bash() {
    let source_v1 = "#!/bin/bash\nsetup() {\n    echo \"init\"\n}\n";
    let source_v2 = "#!/bin/bash\nsetup() {\n    echo \"init v2\"\n    logger \"setup\"\n}\n";
    let filename = "setup.sh";
    let tmp = create_temp_git_repo(filename, &[source_v1, source_v2]);

    let parsed = ParsedFile::parse(filename, source_v2, Language::Bash).unwrap();
    let mut files = BTreeMap::new();
    files.insert(filename.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: filename.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
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
fn test_resonance_slice_bash() {
    let source_v1 = "#!/bin/bash\ninit() { echo \"v1\"; }\n";
    let source_v2 = "#!/bin/bash\ninit() { echo \"v2\"; logger \"done\"; }\n";
    let filename = "init.sh";
    let tmp = create_temp_git_repo(filename, &[source_v1, source_v2]);

    let parsed = ParsedFile::parse(filename, source_v2, Language::Bash).unwrap();
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
// SymmetrySlice — broken symmetry detection (encode/decode, start/stop)
// ---------------------------------------------------------------------------

#[test]
fn test_symmetry_slice_bash() {
    let source = r#"#!/bin/bash

encode_data() {
    local input="$1"
    echo "$input" | base64
}

decode_data() {
    local input="$1"
    echo "$input" | base64 -d
}
"#;
    let path = "codec.sh";
    let parsed = ParsedFile::parse(path, source, Language::Bash).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::SymmetrySlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::SymmetrySlice);
}

// ---------------------------------------------------------------------------
// GradientSlice — continuous relevance scoring
// ---------------------------------------------------------------------------

#[test]
fn test_gradient_slice_bash() {
    let (files, _, diff) = make_bash_test();
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
fn test_phantom_slice_bash() {
    let source_v1 = "#!/bin/bash\ncleanup() {\n    rm -rf /tmp/work\n}\nwork() {\n    mkdir /tmp/work\n    cleanup\n}\n";
    let source_v2 = "#!/bin/bash\nwork() {\n    mkdir /tmp/work\n}\n";
    let filename = "phantom.sh";
    let tmp = create_temp_git_repo(filename, &[source_v1, source_v2]);

    let parsed = ParsedFile::parse(filename, source_v2, Language::Bash).unwrap();
    let mut files = BTreeMap::new();
    files.insert(filename.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: filename.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
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
// MembraneSlice — module boundary (script sourcing)
// ---------------------------------------------------------------------------

#[test]
fn test_membrane_slice_bash() {
    let source_lib = r#"#!/bin/bash

get_config() {
    echo "/etc/myapp.conf"
}

validate() {
    [ -f "$1" ] || return 1
}
"#;
    let source_caller = r#"#!/bin/bash

run() {
    local cfg
    cfg=$(get_config)
    validate "$cfg"
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "scripts/lib.sh".to_string(),
        ParsedFile::parse("scripts/lib.sh", source_lib, Language::Bash).unwrap(),
    );
    files.insert(
        "scripts/main.sh".to_string(),
        ParsedFile::parse("scripts/main.sh", source_caller, Language::Bash).unwrap(),
    );

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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::MembraneSlice);
}

// ---------------------------------------------------------------------------
// ContractSlice — guard validation in scripts
// ---------------------------------------------------------------------------

#[test]
fn test_contract_slice_bash() {
    let source = r#"#!/bin/bash

flash_firmware() {
    local image="$1"
    [ -z "$image" ] && echo "no image" && exit 1
    [ ! -f "$image" ] && echo "not found" && exit 1
    mtd write "$image" firmware
}
"#;
    let path = "flash.sh";
    let parsed = ParsedFile::parse(path, source, Language::Bash).unwrap();
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
fn test_chop_bash() {
    let source = r#"#!/bin/bash

process() {
    local input="$1"
    local validated
    validated=$(echo "$input" | tr -d '/')
    eval "$validated"
}
"#;
    let path = "chop.sh";
    let parsed = ParsedFile::parse(path, source, Language::Bash).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let config = prism::algorithms::chop::ChopConfig {
        source_file: path.to_string(),
        source_line: 4,
        sink_file: path.to_string(),
        sink_line: 7,
    };
    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::chop::slice(&ctx, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::Chop);
}
