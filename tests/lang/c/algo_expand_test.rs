//! Expanded algorithm coverage tests for C language.
//!
//! Covers 11 algorithms not yet tested with C fixtures:
//! FullFlow, BarrierSlice, Chop, RelevantSlice, ConditionedSlice,
//! DeltaSlice, SpiralSlice, CircularSlice, HorizontalSlice, AngleSlice,
//! VerticalSlice.

#[path = "../../common/mod.rs"]
mod common;
use common::*;

// ---------------------------------------------------------------------------
// FullFlow (Algorithm 9)
// ---------------------------------------------------------------------------

#[test]
fn test_full_flow_c() {
    let (files, _, diff) = make_c_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "FullFlow should produce blocks for C code"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::FullFlow);
}

// ---------------------------------------------------------------------------
// BarrierSlice — inline fixture with a call chain
// ---------------------------------------------------------------------------

#[test]
fn test_barrier_slice_c_call_depth() {
    let source = r#"
int level2(int z) {
    return z + 10;
}

int level1(int y) {
    return level2(y * 2);
}

int level0(int x) {
    return level1(x + 1);
}
"#;
    let path = "src/levels.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff touches line 11 inside level0
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([11]),
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
    // BarrierSlice may or may not produce blocks depending on depth config;
    // the key assertion is that it runs without error.
}

// ---------------------------------------------------------------------------
// Chop — source-to-sink data flow via CpgContext
// ---------------------------------------------------------------------------

#[test]
fn test_chop_c_source_to_sink() {
    let source = r#"
#include <stdio.h>
#include <string.h>

void process(void) {
    char input[256];
    char dest[256];
    fgets(input, sizeof(input), stdin);
    int len = strlen(input);
    strcpy(dest, input);
    printf("%s (%d)\n", dest, len);
}
"#;
    let path = "src/chop_target.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let config = prism::algorithms::chop::ChopConfig {
        source_file: path.to_string(),
        source_line: 8, // fgets reads input
        sink_file: path.to_string(),
        sink_line: 10, // strcpy uses input
    };
    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::chop::slice(&ctx, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::Chop);
}

// ---------------------------------------------------------------------------
// RelevantSlice — uses the shared C fixture
// ---------------------------------------------------------------------------

#[test]
fn test_relevant_slice_c() {
    let (files, _, diff) = make_c_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::RelevantSlice),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "RelevantSlice should produce blocks for C code"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::RelevantSlice);
}

// ---------------------------------------------------------------------------
// ConditionedSlice — inline fixture with if/else branches
// ---------------------------------------------------------------------------

#[test]
fn test_conditioned_slice_c() {
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
    let path = "src/classify.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5, 7]),
        }],
    };

    let condition = prism::algorithms::conditioned_slice::Condition {
        var_name: "score".to_string(),
        op: prism::algorithms::conditioned_slice::ConditionOp::GtEq,
        value: "90".to_string(),
    };
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::ConditionedSlice);
    let ctx = CpgContext::build(&files, None);
    let result =
        prism::algorithms::conditioned_slice::slice(&ctx, &diff, &config, &condition).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ConditionedSlice);
}

// ---------------------------------------------------------------------------
// DeltaSlice — temp dir with old version
// ---------------------------------------------------------------------------

#[test]
fn test_delta_slice_c() {
    let tmp = TempDir::new().unwrap();

    let old_source = "int add(int a, int b) {\n    return a + b;\n}\n";
    std::fs::write(tmp.path().join("calc.c"), old_source).unwrap();

    let new_source = "int add(int a, int b) {\n    int result = a + b;\n    return result;\n}\n";
    let path = "calc.c";
    let parsed = ParsedFile::parse(path, new_source, Language::C).unwrap();
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
// SpiralSlice — uses the shared C fixture
// ---------------------------------------------------------------------------

#[test]
fn test_spiral_slice_c() {
    let (files, _, diff) = make_c_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::SpiralSlice);
    let spiral_config = prism::algorithms::spiral_slice::SpiralConfig {
        max_ring: 4,
        auto_stop_threshold: 0.0,
    };
    let ctx = CpgContext::build(&files, None);
    let result =
        prism::algorithms::spiral_slice::slice(&ctx, &diff, &config, &spiral_config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::SpiralSlice);
    assert!(
        !result.blocks.is_empty(),
        "SpiralSlice should produce blocks for C code"
    );
}

// ---------------------------------------------------------------------------
// CircularSlice — inline fixture with mutual recursion
// ---------------------------------------------------------------------------

#[test]
fn test_circular_slice_c() {
    let source = r#"
int is_even(int n);

int is_odd(int n) {
    return n == 0 ? 0 : is_even(n - 1);
}

int is_even(int n) {
    return n == 0 ? 1 : is_odd(n - 1);
}
"#;
    let path = "src/mutual.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5]),
        }],
    };

    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::circular_slice::slice(&ctx, &diff).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::CircularSlice);
}

// ---------------------------------------------------------------------------
// HorizontalSlice — inline fixture with handler pattern
// ---------------------------------------------------------------------------

#[test]
fn test_horizontal_slice_c() {
    let source = r#"
int handle_get(int req) {
    return req + 1;
}

int handle_post(int req) {
    return req + 2;
}

int handle_delete(int req) {
    return req + 3;
}
"#;
    let path = "src/handlers.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let peer_pattern =
        prism::algorithms::horizontal_slice::PeerPattern::NamePattern("handle_".to_string());
    let result = prism::algorithms::horizontal_slice::slice(&files, &diff, &peer_pattern).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::HorizontalSlice);
}

// ---------------------------------------------------------------------------
// AngleSlice — uses the shared C fixture (has error handling patterns)
// ---------------------------------------------------------------------------

#[test]
fn test_angle_slice_c() {
    let (files, _, diff) = make_c_test();
    let concern = prism::algorithms::angle_slice::Concern::ErrorHandling;
    let result = prism::algorithms::angle_slice::slice(&files, &diff, &concern).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AngleSlice);
}

// ---------------------------------------------------------------------------
// VerticalSlice — uses the shared C multifile fixture
// ---------------------------------------------------------------------------

#[test]
fn test_vertical_slice_c() {
    let (files, _, diff) = make_c_multifile_test();
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
// ThinSlice — data deps only, no control flow
// ---------------------------------------------------------------------------

#[test]
fn test_thin_slice_c() {
    let (files, _, diff) = make_c_test();
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
// QuantumSlice — async/concurrent state enumeration
// ---------------------------------------------------------------------------

#[test]
fn test_quantum_slice_c_pthread() {
    let source = r#"
#include <pthread.h>

static int shared_state = 0;
static pthread_mutex_t lock;

void *worker(void *arg) {
    pthread_mutex_lock(&lock);
    shared_state += 1;
    pthread_mutex_unlock(&lock);
    return NULL;
}

void start_workers(void) {
    pthread_t t1, t2;
    pthread_create(&t1, NULL, worker, NULL);
    pthread_create(&t2, NULL, worker, NULL);
}
"#;
    let path = "src/thread.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([9]),
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

// ---------------------------------------------------------------------------
// GradientSlice — continuous relevance scoring
// ---------------------------------------------------------------------------

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
    assert_eq!(result.algorithm, SlicingAlgorithm::GradientSlice);
}

// ---------------------------------------------------------------------------
// SymmetrySlice — broken symmetry detection
// ---------------------------------------------------------------------------

#[test]
fn test_symmetry_slice_c() {
    let source = r#"
int serialize(const char *data, char *buf) {
    buf[0] = data[0];
    return 1;
}

int deserialize(const char *buf, char *data) {
    data[0] = buf[0];
    return 1;
}
"#;
    let path = "src/codec.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
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

// ---------------------------------------------------------------------------
// MembraneSlice — module boundary impact (multifile)
// ---------------------------------------------------------------------------

#[test]
fn test_membrane_slice_c() {
    let (files, _, diff) = make_c_multifile_test();
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
// EchoSlice — ripple effect modeling
// ---------------------------------------------------------------------------

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
    assert_eq!(result.algorithm, SlicingAlgorithm::EchoSlice);
}

// ---------------------------------------------------------------------------
// ContractSlice — implicit behavioral contract extraction
// ---------------------------------------------------------------------------

#[test]
fn test_contract_slice_c() {
    let source = r#"
int process_buffer(const char *buf, int len) {
    if (buf == NULL) return -1;
    if (len <= 0) return -1;
    if (len > 4096) return -1;
    char local[4096];
    for (int i = 0; i < len; i++) {
        local[i] = buf[i];
    }
    return len;
}
"#;
    let path = "src/process.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
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
// ThreeDSlice — temporal-structural risk (requires git)
// ---------------------------------------------------------------------------

#[test]
fn test_threed_slice_c() {
    let source_v1 = "int calc(int x) {\n    return x;\n}\n";
    let source_v2 = "int calc(int x) {\n    return x + 1;\n}\n";
    let filename = "calc.c";
    let tmp = create_temp_git_repo(filename, &[source_v1, source_v2]);

    let parsed = ParsedFile::parse(filename, source_v2, Language::C).unwrap();
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
fn test_resonance_slice_c() {
    let source_v1 = "void init(void) { }\n";
    let source_v2 = "void init(void) {\n    setup();\n}\n";
    let filename = "init.c";
    let tmp = create_temp_git_repo(filename, &[source_v1, source_v2]);

    let parsed = ParsedFile::parse(filename, source_v2, Language::C).unwrap();
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
fn test_phantom_slice_c() {
    let source_v1 = "void cleanup(int *p) {\n    free(p);\n}\nvoid work(void) {\n    int *p = malloc(4);\n    cleanup(p);\n}\n";
    let source_v2 = "void work(void) {\n    int *p = malloc(4);\n}\n";
    let filename = "phantom.c";
    let tmp = create_temp_git_repo(filename, &[source_v1, source_v2]);

    let parsed = ParsedFile::parse(filename, source_v2, Language::C).unwrap();
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
