#[path = "../../common/mod.rs"]
mod common;
use common::*;

#[test]
fn test_membrane_cpp_try_catch_recognised() {
    let api_source = r#"
int init_device(int port) {
    if (port < 0) return -1;
    return 0;
}
"#;

    let caller_good_source = r#"
#include "api.h"
#include <stdexcept>

void setup() {
    try {
        int ret = init_device(8080);
        if (ret < 0) throw std::runtime_error("init failed");
    } catch (std::exception& e) {
        log_error(e.what());
    }
}
"#;

    let caller_bad_source = r#"
#include "api.h"

void quick_setup() {
    init_device(8080);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/api.cpp".to_string(),
        ParsedFile::parse("src/api.cpp", api_source, Language::Cpp).unwrap(),
    );
    files.insert(
        "src/good.cpp".to_string(),
        ParsedFile::parse("src/good.cpp", caller_good_source, Language::Cpp).unwrap(),
    );
    files.insert(
        "src/bad.cpp".to_string(),
        ParsedFile::parse("src/bad.cpp", caller_bad_source, Language::Cpp).unwrap(),
    );

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/api.cpp".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
        None,
    )
    .unwrap();

    let good_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.file.contains("good"))
        .collect();
    assert!(
        good_findings.is_empty(),
        "C++ try/catch should suppress unprotected-caller finding, got: {:?}",
        good_findings
    );

    let bad_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.file.contains("bad"))
        .collect();
    assert!(
        !bad_findings.is_empty(),
        "Caller without error handling should be flagged as unprotected"
    );
}

#[test]
fn test_membrane_cpp_smart_ptr_recognised() {
    let api_source = r#"
struct Device {
    int id;
};

Device* create_device(int id) {
    return new Device{id};
}
"#;

    let caller_raii_source = r#"
#include "api.h"
#include <memory>

void safe_init() {
    std::unique_ptr<Device> dev(create_device(42));
    dev->id = 100;
}
"#;

    let caller_raw_source = r#"
#include "api.h"

void unsafe_init() {
    Device* dev = create_device(42);
    dev->id = 100;
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/api.cpp".to_string(),
        ParsedFile::parse("src/api.cpp", api_source, Language::Cpp).unwrap(),
    );
    files.insert(
        "src/raii.cpp".to_string(),
        ParsedFile::parse("src/raii.cpp", caller_raii_source, Language::Cpp).unwrap(),
    );
    files.insert(
        "src/raw.cpp".to_string(),
        ParsedFile::parse("src/raw.cpp", caller_raw_source, Language::Cpp).unwrap(),
    );

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/api.cpp".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([6]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
        None,
    )
    .unwrap();

    let raii_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.file.contains("raii"))
        .collect();
    assert!(
        raii_findings.is_empty(),
        "C++ unique_ptr RAII should suppress unprotected-caller finding, got: {:?}",
        raii_findings
    );

    let raw_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.file.contains("raw"))
        .collect();
    assert!(
        !raw_findings.is_empty(),
        "Caller with raw pointer (no RAII) should be flagged as unprotected"
    );
}

#[test]
fn test_membrane_cpp_lock_guard_recognised() {
    let api_source = r#"
void update_shared_state(int val) {
    global_state = val;
}
"#;

    let caller_guarded_source = r#"
#include "api.h"
#include <mutex>

std::mutex mtx;

void safe_update(int val) {
    std::lock_guard<std::mutex> lock(mtx);
    update_shared_state(val);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/api.cpp".to_string(),
        ParsedFile::parse("src/api.cpp", api_source, Language::Cpp).unwrap(),
    );
    files.insert(
        "src/guarded.cpp".to_string(),
        ParsedFile::parse("src/guarded.cpp", caller_guarded_source, Language::Cpp).unwrap(),
    );

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/api.cpp".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
        None,
    )
    .unwrap();

    let guarded_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.file.contains("guarded"))
        .collect();
    assert!(
        guarded_findings.is_empty(),
        "C++ lock_guard RAII should suppress unprotected-caller finding, got: {:?}",
        guarded_findings
    );
}

#[test]
fn test_membrane_cpp_optional_recognised() {
    let api_source = r#"
#include <optional>

std::optional<int> find_port(const char* name) {
    if (!name) return std::nullopt;
    return 8080;
}
"#;

    let caller_checked_source = r#"
#include "api.h"

void connect() {
    auto port = find_port("eth0");
    if (port.has_value()) {
        use_port(port.value());
    }
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/api.cpp".to_string(),
        ParsedFile::parse("src/api.cpp", api_source, Language::Cpp).unwrap(),
    );
    files.insert(
        "src/checked.cpp".to_string(),
        ParsedFile::parse("src/checked.cpp", caller_checked_source, Language::Cpp).unwrap(),
    );

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/api.cpp".to_string(),
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

    let checked_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.file.contains("checked"))
        .collect();
    assert!(
        checked_findings.is_empty(),
        "C++ .has_value() check should suppress unprotected-caller finding, got: {:?}",
        checked_findings
    );
}

#[test]
fn test_membrane_through_parameter_fptr() {
    // File A: the API being changed
    let api_source = r#"
int process_data(int val) {
    if (val < 0) return -1;
    return val * 2;
}
"#;

    // File B: executor that calls through a callback parameter
    let executor_source = r#"
typedef int (*transform_fn)(int);

int apply_transform(transform_fn fn, int data) {
    return fn(data);
}
"#;

    // File C: caller that passes process_data as callback, no error handling
    let caller_source = r#"
void run(void) {
    apply_transform(process_data, 42);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/api.c".to_string(),
        ParsedFile::parse("src/api.c", api_source, Language::C).unwrap(),
    );
    files.insert(
        "src/executor.c".to_string(),
        ParsedFile::parse("src/executor.c", executor_source, Language::C).unwrap(),
    );
    files.insert(
        "src/caller.c".to_string(),
        ParsedFile::parse("src/caller.c", caller_source, Language::C).unwrap(),
    );

    // Diff touches process_data body
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/api.c".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
        None,
    )
    .unwrap();

    // The call graph should have resolved apply_transform → process_data via Level 3.
    // The executor calls process_data through the `fn` parameter, and the caller
    // passes process_data as the argument. Membrane should detect the cross-file call.
    // (Either the executor or the direct caller without error handling may be flagged.)
    let has_blocks = !result.blocks.is_empty();
    assert!(
        has_blocks,
        "Membrane should detect cross-file dependency through parameter-passed fptr"
    );
}

#[test]
fn test_membrane_slice_javascript() {
    let source_api = "function fetchUser(id) {\n    const user = db.get(id);\n    if (!user) throw new Error(\"not found\");\n    return user;\n}\n";
    let source_caller =
        "function showProfile(id) {\n    const user = fetchUser(id);\n    render(user);\n}\n";
    let mut files = BTreeMap::new();
    files.insert(
        "api.js".to_string(),
        ParsedFile::parse("api.js", source_api, Language::JavaScript).unwrap(),
    );
    files.insert(
        "profile.js".to_string(),
        ParsedFile::parse("profile.js", source_caller, Language::JavaScript).unwrap(),
    );
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "api.js".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2, 3]),
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

#[test]
fn test_membrane_slice_c_verifies_unprotected_caller() {
    let source_api = r#"
int allocate(int size) {
    if (size <= 0) return -1;
    return 0;
}
"#;
    let source_good = r#"
void safe_caller(void) {
    int ret = allocate(10);
    if (ret < 0) return;
}
"#;
    let source_bad = r#"
void unsafe_caller(void) {
    allocate(10);
}
"#;
    let mut files = BTreeMap::new();
    files.insert(
        "api.c".to_string(),
        ParsedFile::parse("api.c", source_api, Language::C).unwrap(),
    );
    files.insert(
        "safe.c".to_string(),
        ParsedFile::parse("safe.c", source_good, Language::C).unwrap(),
    );
    files.insert(
        "unsafe.c".to_string(),
        ParsedFile::parse("unsafe.c", source_bad, Language::C).unwrap(),
    );

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "api.c".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
        None,
    )
    .unwrap();

    // Membrane should include the unsafe caller
    let has_unsafe = result
        .blocks
        .iter()
        .any(|b| b.file_line_map.contains_key("unsafe.c"));
    assert!(
        has_unsafe,
        "MembraneSlice should include unprotected caller in unsafe.c"
    );

    // If findings are produced, at least one should mention unprotected/missing error handling
    if !result.findings.is_empty() {
        let has_warning = result.findings.iter().any(|f| {
            f.description.contains("error")
                || f.description.contains("unprotected")
                || f.description.contains("check")
        });
        assert!(
            has_warning,
            "MembraneSlice findings should warn about missing error handling. Got: {:?}",
            result
                .findings
                .iter()
                .map(|f| &f.description)
                .collect::<Vec<_>>()
        );
    }
}
