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

// ---------------------------------------------------------------------------
// LeftFlow + FullFlow — use shared C++ fixture
// ---------------------------------------------------------------------------

#[test]
fn test_left_flow_cpp() {
    let (files, _, diff) = make_cpp_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce blocks for C++ code"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::LeftFlow);
}

#[test]
fn test_full_flow_cpp() {
    let (files, _, diff) = make_cpp_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "FullFlow should produce blocks for C++ code"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::FullFlow);
}

// ---------------------------------------------------------------------------
// ThinSlice — data deps only
// ---------------------------------------------------------------------------

#[test]
fn test_thin_slice_cpp() {
    let (files, _, diff) = make_cpp_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ThinSlice);
}

// ---------------------------------------------------------------------------
// VerticalSlice — end-to-end feature path
// ---------------------------------------------------------------------------

#[test]
fn test_vertical_slice_cpp() {
    let (files, _, diff) = make_cpp_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::VerticalSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::VerticalSlice);
}

// ---------------------------------------------------------------------------
// GradientSlice — continuous relevance scoring
// ---------------------------------------------------------------------------

#[test]
fn test_gradient_slice_cpp() {
    let (files, _, diff) = make_cpp_test();
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
// SymmetrySlice — broken symmetry detection
// ---------------------------------------------------------------------------

#[test]
fn test_symmetry_slice_cpp() {
    let source = r#"
class Codec {
public:
    void encode(const char *data, char *buf) {
        buf[0] = data[0];
    }
    void decode(const char *buf, char *data) {
        data[0] = buf[0];
    }
};
"#;
    let path = "src/codec.cpp";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::SymmetrySlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::SymmetrySlice);
}

// ---------------------------------------------------------------------------
// ProvenanceSlice — data origin tracing
// ---------------------------------------------------------------------------

#[test]
fn test_provenance_slice_cpp() {
    let (files, _, diff) = make_cpp_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ProvenanceSlice);
}

// ---------------------------------------------------------------------------
// MembraneSlice — module boundary impact
// ---------------------------------------------------------------------------

#[test]
fn test_membrane_slice_cpp() {
    let (files, _, diff) = make_cpp_test();
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
// ContractSlice — implicit behavioral contract extraction
// ---------------------------------------------------------------------------

#[test]
fn test_contract_slice_cpp() {
    let source = r#"
class Buffer {
public:
    int process(const char *data, int len) {
        if (data == nullptr) return -1;
        if (len <= 0) return -1;
        for (int i = 0; i < len; i++) {
            buf_[i] = data[i];
        }
        return len;
    }
private:
    char buf_[4096];
};
"#;
    let path = "src/buffer.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([6]),
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
// ThreeDSlice — temporal-structural risk (requires git)
// ---------------------------------------------------------------------------

#[test]
fn test_threed_slice_cpp() {
    let source_v1 = "class Foo {\npublic:\n    int calc(int x) { return x; }\n};\n";
    let source_v2 = "class Foo {\npublic:\n    int calc(int x) { return x + 1; }\n};\n";
    let filename = "foo.cpp";
    let tmp = create_temp_git_repo(filename, &[source_v1, source_v2]);

    let parsed = ParsedFile::parse(filename, source_v2, Language::Cpp).unwrap();
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
fn test_resonance_slice_cpp() {
    let source_v1 = "class Svc {\npublic:\n    void init() { }\n};\n";
    let source_v2 = "class Svc {\npublic:\n    void init() { setup(); }\n};\n";
    let filename = "svc.cpp";
    let tmp = create_temp_git_repo(filename, &[source_v1, source_v2]);

    let parsed = ParsedFile::parse(filename, source_v2, Language::Cpp).unwrap();
    let mut files = BTreeMap::new();
    files.insert(filename.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: filename.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
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
fn test_phantom_slice_cpp() {
    let source_v1 = "class Helper {\npublic:\n    void cleanup() { }\n};\nvoid work() {\n    Helper h;\n    h.cleanup();\n}\n";
    let source_v2 = "void work() {\n    // simplified\n}\n";
    let filename = "phantom.cpp";
    let tmp = create_temp_git_repo(filename, &[source_v1, source_v2]);

    let parsed = ParsedFile::parse(filename, source_v2, Language::Cpp).unwrap();
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
