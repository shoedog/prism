#[path = "../../common/mod.rs"]
mod common;
use common::*;

// ── Paper algorithms ──

#[test]
fn test_original_diff_cpp() {
    let (files, _, diff) = make_cpp_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "OriginalDiff should produce blocks for C++ code"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::OriginalDiff);
}

#[test]
fn test_parent_function_cpp() {
    let (files, _, diff) = make_cpp_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "ParentFunction should produce blocks for C++ code"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::ParentFunction);
}

// ── Taxonomy algorithms ──

#[test]
fn test_barrier_slice_cpp_method_depth() {
    let source = r#"
class Logger {
public:
    void log(const char* msg) {
        write(msg);
    }
    void write(const char* msg) {
        flush();
    }
    void flush() {
    }
};
"#;
    let path = "src/logger.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::BarrierSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::BarrierSlice);
}

#[test]
fn test_chop_cpp_data_flow() {
    let source = r#"
#include <string>
std::string transform(const std::string& input) {
    std::string validated = input;
    std::string processed = validated + "_done";
    return processed;
}
"#;
    let path = "src/transform.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let config = prism::algorithms::chop::ChopConfig {
        source_file: path.to_string(),
        source_line: 4,
        sink_file: path.to_string(),
        sink_line: 6,
    };
    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::chop::slice(&ctx, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::Chop);
}

#[test]
fn test_relevant_slice_cpp() {
    let (files, _, diff) = make_cpp_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::RelevantSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::RelevantSlice);
}

#[test]
fn test_conditioned_slice_cpp() {
    let source = r#"
int classify(int score) {
    int grade;
    if (score >= 90) {
        grade = 1;
    } else if (score >= 80) {
        grade = 2;
    } else {
        grade = 3;
    }
    return grade;
}
"#;
    let path = "src/grades.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
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

#[test]
fn test_delta_slice_cpp() {
    let tmp = TempDir::new().unwrap();
    let old_source = "int add(int a, int b) {\n    return a + b;\n}\n";
    std::fs::write(tmp.path().join("calc.cpp"), old_source).unwrap();

    let new_source = "int add(int a, int b) {\n    int result = a + b;\n    return result;\n}\n";
    let path = "calc.cpp";
    let parsed = ParsedFile::parse(path, new_source, Language::Cpp).unwrap();
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
fn test_taint_cpp_system_call() {
    let source = r#"
#include <string>
#include <cstdlib>
void execute(const std::string& input) {
    std::string cmd = "ls " + input;
    system(cmd.c_str());
}
"#;
    let path = "src/exec.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5, 6]),
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

// ── Theoretical algorithms ──

#[test]
fn test_spiral_slice_cpp() {
    let (files, _, diff) = make_cpp_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::SpiralSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::SpiralSlice);
}

#[test]
fn test_circular_slice_cpp() {
    let source = r#"
int is_even(int n);
int is_odd(int n) {
    if (n == 0) return 0;
    return is_even(n - 1);
}
int is_even(int n) {
    if (n == 0) return 1;
    return is_odd(n - 1);
}
"#;
    let path = "src/mutual.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::CircularSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::CircularSlice);
}

#[test]
fn test_quantum_slice_cpp_thread() {
    let source = r#"
#include <thread>
void worker(int id) {
    process(id);
}
void launch() {
    std::thread t(worker, 42);
    t.join();
}
"#;
    let path = "src/async.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::QuantumSlice);
}

#[test]
fn test_horizontal_slice_cpp_methods() {
    let source = r#"
class Handler {
public:
    int handle_get(int req) { return req + 1; }
    int handle_post(int req) { return req + 2; }
    int handle_delete(int req) { return req + 3; }
};
"#;
    let path = "src/handler.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
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
fn test_angle_slice_cpp() {
    let (files, _, diff) = make_cpp_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AngleSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AngleSlice);
}

// ── Novel algorithms ──

#[test]
fn test_echo_slice_cpp_virtual_method() {
    let source = r#"
class Base {
public:
    virtual int compute(int x) {
        return x * 2;
    }
};

void caller(Base& b) {
    int val = b.compute(42);
}
"#;
    let path = "src/virtual.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::EchoSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::EchoSlice);
}
