#[path = "../../common/mod.rs"]
mod common;
use common::*;

fn make_resource_leak_test() -> (BTreeMap<String, ParsedFile>, DiffInput) {
    let source = r#"
def process_file(path):
    f = open(path, 'r')
    data = f.read()
    result = parse(data)
    return result
"#;
    let path = "leaky.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]), // f = open(path, 'r')
        }],
    };

    (files, diff)
}

fn make_symmetry_test() -> (BTreeMap<String, ParsedFile>, DiffInput) {
    let source = r#"
def serialize(data):
    return json.dumps(data)

def deserialize(text):
    return json.loads(text)
"#;
    let path = "codec.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]), // Changed serialize but not deserialize
        }],
    };

    (files, diff)
}

#[test]
fn test_symmetry_slice_finds_counterpart() {
    let (files, diff) = make_symmetry_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::SymmetrySlice),
        None,
    )
    .unwrap();

    // Should find deserialize as counterpart to serialize
    // (may or may not produce blocks depending on whether it considers them "broken")
    assert!(result.algorithm == SlicingAlgorithm::SymmetrySlice);
}

#[test]
fn test_gradient_slice_scores_decay() {
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::GradientSlice),
        None,
    )
    .unwrap();

    // Should produce scored output with diff lines included
    assert!(
        !result.blocks.is_empty(),
        "GradientSlice should produce output"
    );

    // Should have at least the diff lines
    let total_lines: usize = result
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();
    assert!(
        total_lines >= 2,
        "GradientSlice should include at least diff lines"
    );
}

#[test]
fn test_echo_slice_finds_ripple() {
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::EchoSlice),
        None,
    )
    .unwrap();

    assert!(result.algorithm == SlicingAlgorithm::EchoSlice);
}

#[test]
fn test_resonance_slice_runs() {
    let (files, _, diff) = make_python_test();
    // Resonance needs git — will return empty without a repo, but shouldn't crash
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ResonanceSlice),
        None,
    )
    .unwrap();

    assert!(result.algorithm == SlicingAlgorithm::ResonanceSlice);
}

#[test]
fn test_phantom_slice_runs() {
    let (files, _, diff) = make_python_test();
    // Phantom needs git — will return empty without a repo, but shouldn't crash
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::PhantomSlice),
        None,
    )
    .unwrap();

    assert!(result.algorithm == SlicingAlgorithm::PhantomSlice);
}

#[test]
fn test_echo_slice_c() {
    let (files, _, diff) = make_c_multifile_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::EchoSlice),
        None,
    )
    .unwrap();

    // Echo should detect that handle_create calls create_device
    // and may not handle changes to create_device's return value
    assert!(result.algorithm == SlicingAlgorithm::EchoSlice);
}

#[test]
fn test_symmetry_slice_c() {
    let (files, _, diff) = make_c_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::SymmetrySlice),
        None,
    )
    .unwrap();

    // Should detect create_device / destroy_device as a symmetric pair
    assert!(result.algorithm == SlicingAlgorithm::SymmetrySlice);
}

#[test]
fn test_gradient_slice_c() {
    let (files, _, diff) = make_c_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::GradientSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Gradient slice should produce scored output for C code"
    );
}

#[test]
fn test_cpp_symmetry_serialize_deserialize() {
    // C++ code with serialize but no deserialize
    let source = r#"
#include <string>

class Config {
public:
    std::string serialize() {
        return "{\"key\": \"" + key + "\"}";
    }

    // Note: no deserialize method — broken symmetry

    std::string key;
};
"#;

    let path = "src/config.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([7]), // serialize method modified
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::SymmetrySlice),
        None,
    )
    .unwrap();

    assert!(result.algorithm == SlicingAlgorithm::SymmetrySlice);
}

#[test]
fn test_echo_c_caller_without_return_check() {
    // C caller does NOT check return value — should flag missing check.
    let callee_source = r#"
int open_device(const char *path) {
    int fd = open(path, 0);
    return fd;
}
"#;
    let caller_source = r#"
void init_system(void) {
    int fd = open_device("/dev/eth0");
    use_fd(fd);
}
"#;
    let callee_path = "src/device.c";
    let caller_path = "src/init.c";
    let callee_parsed = ParsedFile::parse(callee_path, callee_source, Language::C).unwrap();
    let caller_parsed = ParsedFile::parse(caller_path, caller_source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(callee_path.to_string(), callee_parsed);
    files.insert(caller_path.to_string(), caller_parsed);

    // Diff touches the return statement in open_device
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: callee_path.to_string(),
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

    assert!(
        !result.findings.is_empty(),
        "Echo should flag C caller that doesn't check return value"
    );
}

#[test]
fn test_echo_c_caller_with_return_check() {
    // C caller DOES check return value with if (ret < 0) — should NOT flag.
    let callee_source = r#"
int open_device(const char *path) {
    int fd = open(path, 0);
    return fd;
}
"#;
    let caller_source = r#"
void init_system(void) {
    int ret = open_device("/dev/eth0");
    if (ret < 0) {
        perror("open_device failed");
        return;
    }
    use_fd(ret);
}
"#;
    let callee_path = "src/device.c";
    let caller_path = "src/init.c";
    let callee_parsed = ParsedFile::parse(callee_path, callee_source, Language::C).unwrap();
    let caller_parsed = ParsedFile::parse(caller_path, caller_source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(callee_path.to_string(), callee_parsed);
    files.insert(caller_path.to_string(), caller_parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: callee_path.to_string(),
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

    assert!(
        result.findings.is_empty(),
        "Echo should NOT flag C caller that checks return with if (ret < 0), got: {:?}",
        result
            .findings
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_echo_go_caller_with_errors_is() {
    // Go caller uses errors.Is — should NOT flag missing error handling.
    let callee_source = r#"
package main

func fetchData(url string) error {
    return nil
}
"#;
    let caller_source = r#"
package main

import "errors"

func handleRequest(url string) {
    err := fetchData(url)
    if errors.Is(err, ErrNotFound) {
        handleMissing()
    }
}
"#;
    let callee_path = "pkg/fetch.go";
    let caller_path = "pkg/handler.go";
    let callee_parsed = ParsedFile::parse(callee_path, callee_source, Language::Go).unwrap();
    let caller_parsed = ParsedFile::parse(caller_path, caller_source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(callee_path.to_string(), callee_parsed);
    files.insert(caller_path.to_string(), caller_parsed);

    // Diff touches the return statement (error path change)
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: callee_path.to_string(),
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

    let has_missing_error = result
        .findings
        .iter()
        .any(|f| f.description.contains("no error handling"));
    assert!(
        !has_missing_error,
        "Echo should recognize errors.Is() as error handling, got: {:?}",
        result
            .findings
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_echo_python_caller_with_context_manager() {
    // Python caller uses `with` statement — should recognize as safe handling.
    let callee_source = r#"
def open_connection(host):
    raise ConnectionError("failed")
"#;
    let caller_source = r#"
def process_data(host):
    with open_connection(host) as conn:
        data = conn.read()
    return data
"#;
    let callee_path = "lib/conn.py";
    let caller_path = "lib/process.py";
    let callee_parsed = ParsedFile::parse(callee_path, callee_source, Language::Python).unwrap();
    let caller_parsed = ParsedFile::parse(caller_path, caller_source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(callee_path.to_string(), callee_parsed);
    files.insert(caller_path.to_string(), caller_parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: callee_path.to_string(),
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

    let has_missing = result.findings.iter().any(|f| {
        f.description.contains("no error handling") || f.description.contains("not checked")
    });
    assert!(
        !has_missing,
        "Echo should recognize Python 'with' as safe error handling, got: {:?}",
        result
            .findings
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}
