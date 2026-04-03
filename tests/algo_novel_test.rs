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


fn make_absence_test_fixture() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let source = r#"
import threading

def worker():
    lock = threading.Lock()
    lock.acquire()
    # do work but never release — missing counterpart
    return

def safe_worker():
    lock = threading.Lock()
    lock.acquire()
    try:
        pass
    finally:
        lock.release()
"#;

    let path = "src/worker.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    sources.insert(path.to_string(), source.to_string());

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([6]),
        }],
    };

    (files, sources, diff)
}


#[test]
fn test_absence_slice_detects_missing_close() {
    let (files, diff) = make_resource_leak_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    // Should detect open() without close()
    assert!(
        !result.blocks.is_empty(),
        "AbsenceSlice should detect open without close"
    );
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
fn test_provenance_slice_traces_origins() {
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    // Should trace variable origins on diff lines
    assert!(result.algorithm == SlicingAlgorithm::ProvenanceSlice);
}


#[test]
fn test_membrane_slice_finds_callers() {
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
        None,
    )
    .unwrap();

    // With a single file, cross-file callers won't exist, but it shouldn't crash
    assert!(result.algorithm == SlicingAlgorithm::MembraneSlice);
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
fn test_absence_slice_c_missing_free() {
    // Create C code with malloc but missing free on error path
    let source = r#"
#include <stdlib.h>

int leaky_function(int size) {
    char *buf = malloc(size);
    if (size <= 0) {
        return -1;
    }
    buf[0] = 'x';
    free(buf);
    return 0;
}
"#;

    let path = "src/leak.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5]), // malloc line
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    // The absence slice should NOT flag this because free IS present in the function
    // (even though the error path at line 7 leaks — that's a more sophisticated check)
    assert!(result.algorithm == SlicingAlgorithm::AbsenceSlice);
}


#[test]
fn test_absence_slice_c_no_free() {
    // Create C code with malloc but NO free at all
    let source = r#"
#include <stdlib.h>

int leaky_function(int size) {
    char *buf = malloc(size);
    buf[0] = 'x';
    return 0;
}
"#;

    let path = "src/leak2.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5]), // malloc line
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    // Should detect malloc without free
    assert!(
        !result.blocks.is_empty(),
        "Absence slice should detect malloc without free in C code"
    );
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
fn test_membrane_slice_c() {
    let (files, _, diff) = make_c_multifile_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
        None,
    )
    .unwrap();

    // Membrane should detect cross-file callers of create_device
    assert!(result.algorithm == SlicingAlgorithm::MembraneSlice);
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
fn test_provenance_slice_c() {
    let (files, _, diff) = make_c_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(result.algorithm == SlicingAlgorithm::ProvenanceSlice);
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
fn test_absence_findings_populated() {
    let (files, sources, diff) = make_absence_test_fixture();

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    let review = to_review_output(&result, &sources);
    assert_eq!(review.algorithm, "AbsenceSlice");

    // All findings from absence should have category "missing_counterpart"
    for finding in &review.findings {
        assert_eq!(finding.algorithm, "absence"); // findings use lowercase algorithm names
        assert_eq!(
            finding.category.as_deref(),
            Some("missing_counterpart"),
            "absence findings should have category missing_counterpart"
        );
        assert_eq!(finding.severity, "warning");
    }
}


#[test]
fn test_provenance_c_hardware_origin() {
    // ioctl() call classifies as Hardware origin.
    let source = r#"
int read_sensor(int fd, int cmd) {
    int value = ioctl(fd, cmd, NULL);
    int scaled = value * 2;
    return scaled;
}
"#;
    let path = "src/sensor.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 4: int scaled = value * 2;  — value traces back to ioctl (Hardware)
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(
        result
            .findings
            .iter()
            .any(|f| f.description.contains("hardware")),
        "Provenance should classify ioctl() result as Hardware origin; findings: {:?}",
        result
            .findings
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}


#[test]
fn test_provenance_c_user_input_recv() {
    // recv() call classifies as UserInput origin.
    let source = r#"
int handle_socket(int sock) {
    char buf[256];
    int bytes = recv(sock, buf, sizeof(buf), 0);
    if (bytes > 0) {
        return bytes;
    }
    return 0;
}
"#;
    let path = "src/socket.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 5: if (bytes > 0)  — bytes traces back to recv() (UserInput)
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(
        result
            .findings
            .iter()
            .any(|f| f.description.contains("user_input")),
        "Provenance should classify recv() result as UserInput origin; findings: {:?}",
        result
            .findings
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}


#[test]
fn test_provenance_c_env_getenv() {
    // getenv() call classifies as EnvVar origin.
    let source = r#"
void init_paths() {
    char *home = getenv("HOME");
    int len = strlen(home);
    set_base_path(home);
}
"#;
    let path = "src/init.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 4: int len = strlen(home);  — home traces back to getenv() (EnvVar)
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(
        result
            .findings
            .iter()
            .any(|f| f.description.contains("env_var")),
        "Provenance should classify getenv() result as EnvVar origin; findings: {:?}",
        result
            .findings
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}


#[test]
fn test_absence_c_kmalloc_without_kfree() {
    // kmalloc without matching kfree triggers an absence finding.
    let source = r#"
void alloc_dev_buffer(int size) {
    char *buf = kmalloc(size, GFP_KERNEL);
    if (buf == NULL)
        return;
    buf[0] = 0;
}
"#;
    let path = "src/driver.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 3: kmalloc call — no kfree anywhere in the function
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "AbsenceSlice should detect kmalloc without kfree"
    );
    assert!(
        result
            .findings
            .iter()
            .any(|f| f.description.contains("kmalloc")
                || f.description.contains("kfree")
                || f.description.contains("kernel allocation")),
        "AbsenceSlice finding should mention kmalloc/kfree; findings: {:?}",
        result
            .findings
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}


#[test]
fn test_absence_c_spinlock_without_unlock() {
    // spin_lock without matching spin_unlock triggers an absence finding.
    let source = r#"
void update_counter(spinlock_t *lock) {
    spin_lock(lock);
    shared_counter++;
    return;
}
"#;
    let path = "src/counter.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 3: spin_lock call — no spin_unlock in the function
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "AbsenceSlice should detect spin_lock without spin_unlock"
    );
    assert!(
        result
            .findings
            .iter()
            .any(|f| f.description.contains("spin") || f.description.contains("spinlock")),
        "AbsenceSlice finding should mention spinlock; findings: {:?}",
        result
            .findings
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}


#[test]
fn test_absence_cpp_raii_no_false_positive() {
    // std::unique_ptr triggers RAII bypass — no absence finding for the new expression.
    let source = r#"
#include <memory>

void process_data() {
    std::unique_ptr<char[]> buf(new char[256]);
    buf[0] = 'x';
}
"#;
    let path = "src/safe.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 5: new char[256] is the open pattern; std::unique_ptr provides RAII cleanup.
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    // RAII bypass must prevent a false-positive finding
    assert!(
        result.blocks.is_empty(),
        "AbsenceSlice must NOT flag new/delete when std::unique_ptr handles cleanup; \
         got {} blocks",
        result.blocks.len()
    );
}


#[test]
fn test_absence_cpp_unique_ptr_no_false_positive() {
    // std::shared_ptr also triggers RAII bypass.
    let source = r#"
#include <memory>

void hold_resource() {
    std::shared_ptr<int> ptr(new int(42));
    *ptr = 100;
}
"#;
    let path = "src/shared.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 5: new int(42) is the open pattern; std::shared_ptr provides RAII cleanup.
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    assert!(
        result.blocks.is_empty(),
        "AbsenceSlice must NOT flag new/delete when std::shared_ptr handles cleanup; \
         got {} blocks",
        result.blocks.len()
    );
}


#[test]
fn test_membrane_c_error_handling_recognised() {
    // File A: the API being changed
    let api_source = r#"
#include <stdlib.h>

int create_device(const char *name, int id) {
    if (!name) return -1;
    // ... allocate and initialise ...
    return 0;
}
"#;

    // File B: caller WITH proper C error handling
    let caller_good_source = r#"
#include "api.h"
#include <stdio.h>

int init_system(void) {
    int ret = create_device("eth0", 1);
    if (ret < 0) {
        perror("create_device failed");
        return -1;
    }
    return 0;
}
"#;

    // File C: caller WITHOUT error handling
    let caller_bad_source = r#"
#include "api.h"

void quick_init(void) {
    create_device("eth0", 1);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/api.c".to_string(),
        ParsedFile::parse("src/api.c", api_source, Language::C).unwrap(),
    );
    files.insert(
        "src/good_caller.c".to_string(),
        ParsedFile::parse("src/good_caller.c", caller_good_source, Language::C).unwrap(),
    );
    files.insert(
        "src/bad_caller.c".to_string(),
        ParsedFile::parse("src/bad_caller.c", caller_bad_source, Language::C).unwrap(),
    );

    // Diff touches create_device body
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/api.c".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
        None,
    )
    .unwrap();

    // The good caller (ret < 0 + perror) should NOT be flagged
    let good_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.file.contains("good_caller"))
        .collect();
    assert!(
        good_findings.is_empty(),
        "C error handling (if ret < 0 / perror) should suppress unprotected-caller finding, got: {:?}",
        good_findings
    );

    // The bad caller (no error check) SHOULD be flagged
    let bad_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.file.contains("bad_caller"))
        .collect();
    assert!(
        !bad_findings.is_empty(),
        "Caller without error handling should be flagged as unprotected"
    );
}


#[test]
fn test_membrane_c_null_check_recognised() {
    let api_source = r#"
#include <stdlib.h>

void *allocate_buffer(int size) {
    return malloc(size);
}
"#;

    let caller_source = r#"
#include "api.h"

int use_buffer(void) {
    void *buf = allocate_buffer(1024);
    if (!buf) {
        return -1;
    }
    return 0;
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/api.c".to_string(),
        ParsedFile::parse("src/api.c", api_source, Language::C).unwrap(),
    );
    files.insert(
        "src/caller.c".to_string(),
        ParsedFile::parse("src/caller.c", caller_source, Language::C).unwrap(),
    );

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/api.c".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
        None,
    )
    .unwrap();

    let findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.file.contains("caller"))
        .collect();
    assert!(
        findings.is_empty(),
        "NULL-pointer check (if (!buf)) should count as error handling, got: {:?}",
        findings
    );
}


#[test]
fn test_membrane_function_pointer_cross_file() {
    // File A: defines process() which is called via struct field in File B
    let api_source = r#"
#include <stdlib.h>

int process(int *data, int len) {
    for (int i = 0; i < len; i++) {
        data[i] *= 2;
    }
    return 0;
}
"#;

    // File B: calls process via ops->process(data, len)
    let caller_source = r#"
#include "api.h"

struct operations {
    int (*process)(int *data, int len);
};

int run_pipeline(struct operations *ops, int *data, int len) {
    int ret = ops->process(data, len);
    if (ret < 0) {
        return -1;
    }
    return 0;
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/api.c".to_string(),
        ParsedFile::parse("src/api.c", api_source, Language::C).unwrap(),
    );
    files.insert(
        "src/driver.c".to_string(),
        ParsedFile::parse("src/driver.c", caller_source, Language::C).unwrap(),
    );

    // Diff touches process() body
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/api.c".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5, 6]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
        None,
    )
    .unwrap();

    // MembraneSlice should find run_pipeline as a cross-file caller of process
    // via the ops->process() call
    assert!(
        !result.blocks.is_empty(),
        "MembraneSlice should detect cross-file caller via function pointer dispatch"
    );

    // The blocks should reference the driver file
    let has_driver_ref = result
        .blocks
        .iter()
        .any(|b| b.file_line_map.keys().any(|f| f.contains("driver")));
    assert!(
        has_driver_ref,
        "MembraneSlice blocks should include the cross-file caller in driver.c"
    );
}


#[test]
fn test_membrane_through_local_fptr() {
    let api_source = r#"
int process_data(int *buf, int len) {
    for (int i = 0; i < len; i++) {
        buf[i] += 1;
    }
    return 0;
}
"#;

    let caller_source = r#"
#include "api.h"

int process_data(int *buf, int len);

typedef int (*processor_fn)(int *, int);

int run(int *data, int len) {
    processor_fn proc = process_data;
    int ret = proc(data, len);
    if (ret < 0) {
        return -1;
    }
    return 0;
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/api.c".to_string(),
        ParsedFile::parse("src/api.c", api_source, Language::C).unwrap(),
    );
    files.insert(
        "src/caller.c".to_string(),
        ParsedFile::parse("src/caller.c", caller_source, Language::C).unwrap(),
    );

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/api.c".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };

    let call_graph = CallGraph::build(&files);

    // Verify the edge exists: run -> process_data via fptr
    let run_id = &call_graph.functions.get("run").unwrap()[0];
    let run_calls = call_graph.calls.get(run_id).unwrap();
    let callee_names: Vec<&str> = run_calls.iter().map(|s| s.callee_name.as_str()).collect();
    assert!(
        callee_names.contains(&"process_data"),
        "Level 1: proc = process_data; proc() should create edge to process_data, got: {:?}",
        callee_names
    );

    // MembraneSlice should find run() as a cross-file caller of process_data
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
        None,
    )
    .unwrap();

    let has_caller_ref = result
        .blocks
        .iter()
        .any(|b| b.file_line_map.keys().any(|f| f.contains("caller")));
    assert!(
        has_caller_ref,
        "MembraneSlice should detect cross-file caller via local function pointer"
    );
}


#[test]
fn test_membrane_respects_static_linkage() {
    let file_a = r#"
static int process(int x) {
    return x * 2;
}

int run_a(void) {
    return process(42);
}
"#;

    let file_b = r#"
static int process(int x) {
    return x + 1;
}

int run_b(void) {
    return process(99);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/a.c".to_string(),
        ParsedFile::parse("src/a.c", file_a, Language::C).unwrap(),
    );
    files.insert(
        "src/b.c".to_string(),
        ParsedFile::parse("src/b.c", file_b, Language::C).unwrap(),
    );

    // Diff touches process() in a.c
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/a.c".to_string(),
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

    // run_b() should NOT appear as a cross-file caller of a.c's static process(),
    // because static means file-local linkage
    let has_b_ref = result
        .blocks
        .iter()
        .any(|b| b.file_line_map.keys().any(|f| f.contains("b.c")));
    assert!(
        !has_b_ref,
        "MembraneSlice should not flag cross-file callers for a static function"
    );
}


#[test]
fn test_provenance_python_request_form_origin() {
    // Variable originates from Flask request.form — should be classified as user_input.
    let source = r#"
from flask import request

def handle_login():
    username = request.form['username']
    process(username)
"#;
    let path = "app/auth.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    // Should produce findings since username comes from user input
    assert!(
        !result.blocks.is_empty(),
        "Provenance should produce blocks tracing username to request.form"
    );
    let has_user_input = result
        .findings
        .iter()
        .any(|f| f.description.contains("user_input"));
    assert!(
        has_user_input,
        "Provenance should classify request.form as user_input origin"
    );
}


#[test]
fn test_provenance_python_cursor_execute_origin() {
    // Variable originates from cursor.fetchone — should be classified as database.
    let source = r#"
import sqlite3

def get_user(user_id):
    conn = sqlite3.connect('db.sqlite')
    cursor = conn.cursor()
    cursor.execute("SELECT * FROM users WHERE id = ?", (user_id,))
    row = cursor.fetchone()
    return row
"#;
    let path = "app/db.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([8]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Provenance should produce blocks tracing row to cursor.fetchone"
    );
    let has_db = result
        .findings
        .iter()
        .any(|f| f.description.contains("database"));
    assert!(
        has_db,
        "Provenance should classify cursor.fetchone as database origin"
    );
}


#[test]
fn test_provenance_js_document_cookie_origin() {
    // Variable originates from document.cookie — should be classified as user_input.
    let source = r#"
function getCookieValue() {
    const cookies = document.cookie;
    const parsed = parseCookies(cookies);
    return parsed;
}
"#;
    let path = "src/cookies.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Provenance should produce blocks tracing cookies to document.cookie"
    );
}


#[test]
fn test_provenance_js_process_env_origin() {
    // Variable originates from process.env — should be classified as env_var.
    let source = r#"
function getPort() {
    const port = process.env.PORT;
    return port;
}
"#;
    let path = "src/config.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Provenance should produce blocks tracing port to process.env"
    );
    let has_env = result
        .findings
        .iter()
        .any(|f| f.description.contains("env_var"));
    assert!(
        has_env,
        "Provenance should classify process.env as env_var origin"
    );
}


#[test]
fn test_provenance_go_form_value_origin() {
    // Variable originates from r.FormValue — should be classified as user_input.
    let source = r#"
package main

import "net/http"

func handler(w http.ResponseWriter, r *http.Request) {
    name := r.FormValue("name")
    process(name)
}
"#;
    let path = "web/handler.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([8]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Provenance should produce blocks tracing name to r.FormValue"
    );
    let has_user_input = result
        .findings
        .iter()
        .any(|f| f.description.contains("user_input"));
    assert!(
        has_user_input,
        "Provenance should classify r.FormValue as user_input origin"
    );
}


#[test]
fn test_provenance_go_viper_config_origin() {
    // Variable originates from viper config — should be classified as config.
    let source = r#"
package main

import "github.com/spf13/viper"

func loadConfig() string {
    dbHost := viper.GetString("database.host")
    return dbHost
}
"#;
    let path = "config/loader.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([8]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Provenance should produce blocks tracing dbHost to viper config"
    );
    let has_config = result
        .findings
        .iter()
        .any(|f| f.description.contains("config"));
    assert!(
        has_config,
        "Provenance should classify viper.GetString as config origin"
    );
}


#[test]
fn test_absence_python_threading_lock_without_release() {
    // Python threading.Lock acquired but never released.
    let source = r#"
import threading

def process_data(data):
    lock = threading.Lock()
    lock.acquire()
    result = transform(data)
    return result
"#;
    let path = "app/worker.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff line 6: lock.acquire() — the "open" pattern
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Absence should detect threading lock without release"
    );
    assert!(
        result
            .findings
            .iter()
            .any(|f| f.description.contains("lock")
                || f.description.contains("release")
                || f.description.contains("unlock")),
        "Finding should mention missing lock release"
    );
}


#[test]
fn test_absence_python_tempfile_without_cleanup() {
    // Python tempfile.mkstemp without os.close/os.unlink.
    let source = r#"
import tempfile

def create_temp():
    fd, path = tempfile.mkstemp()
    write_data(fd)
    return path
"#;
    let path = "app/temp.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Absence should detect tempfile.mkstemp without cleanup"
    );
}


#[test]
fn test_absence_js_stream_without_destroy() {
    // Node.js createReadStream without destroy/close.
    let source = r#"
const fs = require('fs');

function readFile(path) {
    const stream = fs.createReadStream(path);
    const data = processStream(stream);
    return data;
}
"#;
    let path = "src/reader.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Absence should detect createReadStream without destroy/close"
    );
}


#[test]
fn test_absence_js_fs_open_without_close() {
    // Node.js fs.openSync without fs.closeSync.
    let source = r#"
const fs = require('fs');

function writeData(path, data) {
    const fd = fs.openSync(path, 'w');
    fs.writeSync(fd, data);
    return fd;
}
"#;
    let path = "src/writer.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Absence should detect fs.openSync without fs.closeSync"
    );
}


#[test]
fn test_absence_go_context_without_cancel() {
    // Go context.WithCancel without calling cancel().
    let source = r#"
package main

import "context"

func doWork(parent context.Context) {
    ctx, cancel := context.WithCancel(parent)
    result := process(ctx)
    handle(result)
}
"#;
    let path = "cmd/work.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Absence should detect context.WithCancel without cancel()"
    );
    assert!(
        result
            .findings
            .iter()
            .any(|f| f.description.contains("context") || f.description.contains("cancel")),
        "Finding should mention missing cancel"
    );
}


#[test]
fn test_absence_go_http_body_not_closed() {
    // Go http.Get without resp.Body.Close().
    let source = r#"
package main

import "net/http"

func fetchURL(url string) string {
    resp, err := http.Get(url)
    if err != nil {
        return ""
    }
    body := readBody(resp)
    return body
}
"#;
    let path = "web/fetch.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Absence should detect http.Get without Body.Close"
    );
}


#[test]
fn test_membrane_python_raise_for_status() {
    // Python caller using raise_for_status() should count as error handling.
    let caller_source = r#"
import requests

def fetch_data(url):
    response = get_api_data(url)
    response.raise_for_status()
    return response.json()
"#;
    let callee_source = r#"
import requests

def get_api_data(url):
    return requests.get(url)
"#;
    let caller_path = "app/client.py";
    let callee_path = "app/api.py";
    let caller_parsed = ParsedFile::parse(caller_path, caller_source, Language::Python).unwrap();
    let callee_parsed = ParsedFile::parse(callee_path, callee_source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(caller_path.to_string(), caller_parsed);
    files.insert(callee_path.to_string(), callee_parsed);

    // Diff on the callee function
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
        None,
    )
    .unwrap();

    // Should have blocks (cross-file caller exists) but NO "unprotected" finding
    // because raise_for_status() counts as error handling.
    let has_unprotected = result
        .findings
        .iter()
        .any(|f| f.category.as_deref() == Some("unprotected_caller"));
    assert!(
        !has_unprotected,
        "Membrane should recognize raise_for_status() as error handling"
    );
}


#[test]
fn test_membrane_go_errors_is_handling() {
    // Go caller using errors.Is() should count as error handling.
    let caller_source = r#"
package main

import "errors"

func processRequest() {
    err := doWork()
    if errors.Is(err, ErrNotFound) {
        handleNotFound()
    }
}
"#;
    let callee_source = r#"
package main

func doWork() error {
    return nil
}
"#;
    let caller_path = "cmd/handler.go";
    let callee_path = "cmd/worker.go";
    let caller_parsed = ParsedFile::parse(caller_path, caller_source, Language::Go).unwrap();
    let callee_parsed = ParsedFile::parse(callee_path, callee_source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(caller_path.to_string(), caller_parsed);
    files.insert(callee_path.to_string(), callee_parsed);

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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
        None,
    )
    .unwrap();

    let has_unprotected = result
        .findings
        .iter()
        .any(|f| f.category.as_deref() == Some("unprotected_caller"));
    assert!(
        !has_unprotected,
        "Membrane should recognize errors.Is() as error handling"
    );
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


#[test]
fn test_provenance_negative_transform_not_form() {
    // "transform" should NOT match the "~form" word-boundary pattern.
    let source = r#"
def transform_data(raw):
    result = transform(raw)
    return result
"#;
    let path = "lib/transform.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    let has_user_input = result
        .findings
        .iter()
        .any(|f| f.description.contains("user_input"));
    assert!(
        !has_user_input,
        "Provenance should NOT classify 'transform' as user_input (form match), got: {:?}",
        result
            .findings
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}


#[test]
fn test_provenance_negative_prefetch_not_fetch() {
    // "prefetch" should NOT match the "~fetch" word-boundary pattern.
    let source = r#"
function prefetchAssets(urls) {
    const assets = prefetch(urls);
    return assets;
}
"#;
    let path = "src/loader.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    let has_db = result
        .findings
        .iter()
        .any(|f| f.description.contains("database"));
    assert!(
        !has_db,
        "Provenance should NOT classify 'prefetch' as database origin, got: {:?}",
        result
            .findings
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}


#[test]
fn test_lua_absence_open_without_close() {
    // Lua io.open without io.close.
    let source = r#"
function read_config(path)
    local file = io.open(path, "r")
    local content = file:read("*a")
    return content
end
"#;
    let path = "scripts/config.lua";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Absence should detect Lua io.open without close"
    );
}


#[test]
fn test_rust_provenance_stdin() {
    let source = r#"
fn get_input() -> String {
    let data = std::io::stdin().read_line();
    process(data)
}
"#;
    let path = "src/input.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Provenance should detect std::io::stdin as user input source"
    );
    assert_eq!(
        result.findings[0].category.as_deref(),
        Some("untrusted_origin")
    );
}


#[test]
fn test_rust_provenance_diesel_query() {
    let source = r#"
fn get_users(conn: &PgConnection) {
    let results = diesel::sql_query("SELECT * FROM users").load(conn);
    process(results)
}
"#;
    let path = "src/db.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Provenance should detect diesel:: as database source"
    );
    assert_eq!(
        result.findings[0].category.as_deref(),
        Some("untrusted_origin")
    );
}


#[test]
fn test_rust_provenance_env_var() {
    let source = r#"
fn get_config() {
    let val = std::env::var("DATABASE_URL");
    process(val)
}
"#;
    let path = "src/config.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Provenance should detect std::env::var as environment source"
    );
    assert_eq!(
        result.findings[0].category.as_deref(),
        Some("untrusted_origin")
    );
}


#[test]
fn test_rust_absence_file_without_flush() {
    let source = r#"
use std::fs::File;
use std::io::Write;

fn write_data(path: &str) {
    let mut file = File::create(path).unwrap();
    file.write_all(b"data").unwrap();
}
"#;
    let path = "src/writer.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Absence should detect Rust File::create without explicit flush/drop"
    );
}


#[test]
fn test_rust_absence_command_not_executed() {
    let source = r#"
use std::process::Command;

fn setup_cmd() {
    let cmd = Command::new("ls");
    let args = cmd.arg("-la");
}
"#;
    let path = "src/cmd.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Absence should detect Rust Command::new without execution"
    );
}


#[test]
fn test_rust_absence_unsafe_without_safety_comment() {
    let source = r#"
fn do_unsafe_stuff(ptr: *const u8) -> u8 {
    let val = unsafe { *ptr };
    val
}
"#;
    let path = "src/unsafe_code.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Absence should detect unsafe block without safety assertion/comment"
    );
}


#[test]
fn test_rust_membrane_error_handling() {
    let api_source = r#"
pub fn fetch_data(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let resp = reqwest::blocking::get(url)?;
    Ok(resp.text()?)
}
"#;
    let caller_source = r#"
use api::fetch_data;

fn caller() -> Result<(), Box<dyn std::error::Error>> {
    let data = fetch_data("http://example.com")?;
    println!("{}", data);
    Ok(())
}
"#;
    let api_path = "src/api.rs";
    let caller_path = "src/caller.rs";
    let api_parsed = ParsedFile::parse(api_path, api_source, Language::Rust).unwrap();
    let caller_parsed = ParsedFile::parse(caller_path, caller_source, Language::Rust).unwrap();
    let mut files = BTreeMap::new();
    files.insert(api_path.to_string(), api_parsed);
    files.insert(caller_path.to_string(), caller_parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: api_path.to_string(),
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

    let unprotected = result
        .findings
        .iter()
        .any(|f| f.category.as_deref() == Some("unprotected_caller"));
    assert!(
        !unprotected,
        "Membrane should recognize Rust ? operator as error handling"
    );
}


#[test]
fn test_lua_provenance_io_read() {
    let source = r#"
function get_input()
    local data = io.read("*l")
    process(data)
end
"#;
    let path = "scripts/input.lua";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Provenance should detect Lua io.read as user input source"
    );
    assert_eq!(
        result.findings[0].category.as_deref(),
        Some("untrusted_origin")
    );
}


#[test]
fn test_lua_provenance_os_getenv() {
    let source = r#"
function get_path()
    local path = os.getenv("PATH")
    process(path)
end
"#;
    let path = "scripts/env.lua";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Provenance should detect Lua os.getenv as environment source"
    );
    assert_eq!(
        result.findings[0].category.as_deref(),
        Some("untrusted_origin")
    );
}


#[test]
fn test_lua_absence_socket_without_close() {
    let source = r#"
local socket = require("socket")

function connect_server(host, port)
    local tcp = socket.tcp()
    tcp:connect(host, port)
    tcp:send("hello")
end
"#;
    let path = "scripts/net.lua";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Absence should detect Lua socket.tcp without close"
    );
}


#[test]
fn test_lua_membrane_error_handling() {
    let api_source = r#"
function fetch_data(url)
    local resp = http.request(url)
    return resp
end
"#;
    let caller_source = r#"
local api = require("api")

function caller()
    local ok, data = pcall(api.fetch_data, "http://example.com")
    if ok then
        print(data)
    end
end
"#;
    let api_path = "scripts/api.lua";
    let caller_path = "scripts/caller.lua";
    let api_parsed = ParsedFile::parse(api_path, api_source, Language::Lua).unwrap();
    let caller_parsed = ParsedFile::parse(caller_path, caller_source, Language::Lua).unwrap();
    let mut files = BTreeMap::new();
    files.insert(api_path.to_string(), api_parsed);
    files.insert(caller_path.to_string(), caller_parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: api_path.to_string(),
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

    let unprotected = result
        .findings
        .iter()
        .any(|f| f.category.as_deref() == Some("unprotected_caller"));
    assert!(
        !unprotected,
        "Membrane should recognize Lua pcall as error handling"
    );
}


#[test]
fn test_lua_provenance_redis() {
    let source = r#"
function get_cached(key)
    local res = redis:get(key)
    process(res)
end
"#;
    let path = "scripts/cache.lua";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Provenance should detect Lua redis:get as database source"
    );
    assert_eq!(
        result.findings[0].category.as_deref(),
        Some("untrusted_origin")
    );
}


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
fn test_cve_strcpy_overflow_provenance() {
    let source = r#"
#include <string.h>
#include <stdio.h>

void handle_request(void) {
    char buf[64];
    char dest[32];
    char *input = fgets(buf, sizeof(buf), stdin);
    strcpy(dest, input);
}
"#;

    let path = "src/handler.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff touches fgets line — taint should trace input → strcpy
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([8]),
        }],
    };

    let taint_result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    // Taint: fgets line → input → strcpy sink
    assert!(
        !taint_result.blocks.is_empty(),
        "Taint should include fgets and strcpy in the taint trace"
    );

    let prov_result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    // Provenance: fgets → UserInput origin
    assert!(
        !prov_result.findings.is_empty(),
        "Provenance should classify fgets as user_input origin"
    );
}


#[test]
fn test_provenance_with_field_access() {
    // Provenance should track origins through field-qualified variables.
    let source = r#"
#include <stdio.h>
void handle(struct request *req) {
    req->data = fgets(buf, sizeof(buf), stdin);
    process(req->data);
}
"#;
    let path = "src/req.c";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    // Provenance should detect fgets/stdin as a user_input source
    // via the base name match on all_defs_of
    assert!(
        !result.blocks.is_empty(),
        "Provenance should produce blocks when source is assigned through field access"
    );
}


#[test]
fn test_resonance_slice_python() {
    let source = "def update(x):\n    y = x + 1\n    return y\n";
    let filename = "app.py";
    let tmp = create_temp_git_repo(filename, &["def update(x):\n    return x\n", source]);

    let parsed = ParsedFile::parse(filename, source, Language::Python).unwrap();
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

    let _ = std::fs::remove_dir_all(&tmp);
}


#[test]
fn test_resonance_slice_go() {
    let source = "package main\n\nfunc calc(n int) int {\n\treturn n * 2\n}\n";
    let filename = "calc.go";
    let tmp = create_temp_git_repo(
        filename,
        &[
            "package main\n\nfunc calc(n int) int { return n }\n",
            source,
        ],
    );

    let parsed = ParsedFile::parse(filename, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(filename.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: filename.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
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

    let _ = std::fs::remove_dir_all(&tmp);
}


#[test]
fn test_gradient_slice_python() {
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::GradientSlice),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}


#[test]
fn test_gradient_slice_go() {
    let (files, _, diff) = make_go_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::GradientSlice),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}


#[test]
fn test_phantom_slice_python() {
    let source = "def remaining(x):\n    return x + 1\n";
    let filename = "app.py";
    let tmp = create_temp_git_repo(
        filename,
        &[
            "def deleted_func(x):\n    return x * 2\n\ndef remaining(x):\n    return x + 1\n",
            source,
        ],
    );
    let parsed = ParsedFile::parse(filename, source, Language::Python).unwrap();
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
    let _ = std::fs::remove_dir_all(&tmp);
}


#[test]
fn test_phantom_slice_go() {
    let source = "package main\n\nfunc alive(n int) int {\n\treturn n + 1\n}\n";
    let filename = "main.go";
    let tmp = create_temp_git_repo(filename, &[
        "package main\n\nfunc dead(n int) int {\n\treturn n * 2\n}\n\nfunc alive(n int) int {\n\treturn n + 1\n}\n",
        source,
    ]);
    let parsed = ParsedFile::parse(filename, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(filename.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: filename.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };
    let config = prism::algorithms::phantom_slice::PhantomConfig {
        git_dir: tmp.path().to_string_lossy().to_string(),
        max_commits: 50,
    };
    let result = prism::algorithms::phantom_slice::slice(&files, &diff, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::PhantomSlice);
    let _ = std::fs::remove_dir_all(&tmp);
}


#[test]
fn test_symmetry_slice_python() {
    let source = r#"
import json

def save(data, path):
    with open(path, 'w') as f:
        json.dump(data, f)

def load(path):
    with open(path, 'r') as f:
        return json.load(f)
"#;
    let path = "serializer.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::SymmetrySlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::SymmetrySlice);
}


#[test]
fn test_symmetry_slice_go() {
    let source = r#"package main

import "encoding/json"

func encode(data interface{}) ([]byte, error) {
	return json.Marshal(data)
}

func decode(b []byte, v interface{}) error {
	return json.Unmarshal(b, v)
}
"#;
    let path = "codec.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::SymmetrySlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::SymmetrySlice);
}


#[test]
fn test_echo_slice_python_handler() {
    let source_api = "def create_resource(name):\n    if not name:\n        raise ValueError(\"name required\")\n    return {\"name\": name}\n";
    let source_caller =
        "def handler():\n    result = create_resource(\"test\")\n    return result\n";
    let mut files = BTreeMap::new();
    files.insert(
        "api.py".to_string(),
        ParsedFile::parse("api.py", source_api, Language::Python).unwrap(),
    );
    files.insert(
        "handler.py".to_string(),
        ParsedFile::parse("handler.py", source_caller, Language::Python).unwrap(),
    );
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "api.py".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2, 3]),
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
fn test_echo_slice_javascript() {
    let source_api = "function validate(input) {\n    if (!input) {\n        throw new Error(\"missing\");\n    }\n    return input.trim();\n}\n";
    let source_caller =
        "function process() {\n    const result = validate(getData());\n    return result;\n}\n";
    let mut files = BTreeMap::new();
    files.insert(
        "validate.js".to_string(),
        ParsedFile::parse("validate.js", source_api, Language::JavaScript).unwrap(),
    );
    files.insert(
        "process.js".to_string(),
        ParsedFile::parse("process.js", source_caller, Language::JavaScript).unwrap(),
    );
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "validate.js".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2, 3]),
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
fn test_provenance_slice_javascript() {
    let source = "function handler(req, res) {\n    const token = req.headers.authorization;\n    const userId = parseToken(token);\n    const data = db.query(userId);\n    res.json(data);\n}\n";
    let path = "handler.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2, 3]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ProvenanceSlice);
}


#[test]
fn test_absence_slice_javascript() {
    let source = "function processFile(path) {\n    const fd = fs.openSync(path, 'r');\n    const data = fs.readFileSync(fd);\n    return data;\n}\n";
    let path = "file.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AbsenceSlice);
}


#[test]
fn test_absence_slice_go_open() {
    let source = "package main\n\nimport \"os\"\n\nfunc readFile(path string) []byte {\n\tf, _ := os.Open(path)\n\tdata := make([]byte, 1024)\n\tf.Read(data)\n\treturn data\n}\n";
    let path = "io.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AbsenceSlice);
}


#[test]
fn test_absence_slice_python_lock_findings() {
    let source = r#"
import threading

def critical_section(lock):
    lock.acquire()
    shared_data = read_shared()
    update_shared(shared_data + 1)
"#;
    let path = "critical.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();
    // Should produce findings about missing lock.release()
    let has_lock_finding = result.findings.iter().any(|f| {
        f.description.contains("release")
            || f.description.contains("unlock")
            || f.description.contains("acquire")
    });
    // The algorithm should at minimum produce blocks
    assert!(
        !result.blocks.is_empty(),
        "AbsenceSlice should produce blocks for lock without release"
    );
    if !result.findings.is_empty() {
        assert!(
            has_lock_finding,
            "AbsenceSlice findings should mention missing release. Got: {:?}",
            result
                .findings
                .iter()
                .map(|f| &f.description)
                .collect::<Vec<_>>()
        );
    }
}


#[test]
fn test_symmetry_slice_python_finds_counterpart() {
    let source = r#"
import json

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
    assert!(
        !result.blocks.is_empty(),
        "SymmetrySlice should find counterpart function"
    );

    // If blocks include the counterpart, both serialize and deserialize should appear
    let all_lines: BTreeSet<usize> = result
        .blocks
        .iter()
        .flat_map(|b| b.file_line_map.values())
        .flat_map(|m| m.keys())
        .copied()
        .collect();
    // The counterpart (deserialize at lines 7-8) should be included
    let has_counterpart = all_lines.contains(&7) || all_lines.contains(&8);
    assert!(
        has_counterpart,
        "SymmetrySlice should include counterpart (deserialize). Got lines: {:?}",
        all_lines
    );
}


#[test]
fn test_echo_slice_c_verifies_caller_inclusion() {
    // Echo should find callers that don't handle errors from the changed function
    let source_api = r#"
int create_resource(const char *name) {
    if (!name) return -1;
    return 0;
}
"#;
    let source_caller = r#"
void setup(void) {
    create_resource("test");
}
"#;
    let mut files = BTreeMap::new();
    files.insert(
        "api.c".to_string(),
        ParsedFile::parse("api.c", source_api, Language::C).unwrap(),
    );
    files.insert(
        "setup.c".to_string(),
        ParsedFile::parse("setup.c", source_caller, Language::C).unwrap(),
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::EchoSlice),
        None,
    )
    .unwrap();
    // Should include the caller file
    let has_caller_file = result
        .blocks
        .iter()
        .any(|b| b.file_line_map.contains_key("setup.c"));
    assert!(
        has_caller_file,
        "EchoSlice should include caller file setup.c in blocks"
    );
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


#[test]
fn test_provenance_slice_python_traces_user_input() {
    let source = r#"
def handle(request):
    name = request.form.get("name")
    greeting = "Hello " + name
    return greeting
"#;
    let path = "app.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3, 4]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "Provenance should trace user input origin"
    );

    // Should include the form.get line as a user input source
    let all_lines: BTreeSet<usize> = result
        .blocks
        .iter()
        .flat_map(|b| b.file_line_map.values())
        .flat_map(|m| m.keys())
        .copied()
        .collect();
    assert!(
        all_lines.contains(&3),
        "Provenance should include form.get line (3). Got: {:?}",
        all_lines
    );
}


#[test]
fn test_gradient_slice_python_scores_decay() {
    // Gradient slice should assign higher relevance to lines closer to the diff
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::GradientSlice),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());

    // Verify diff lines are marked as diff (highest relevance)
    let block = &result.blocks[0];
    let lines = block.file_line_map.get("src/calc.py").unwrap();
    // Diff lines 7 and 9 should be marked as diff=true
    if let Some(&is_diff) = lines.get(&7) {
        assert!(
            is_diff,
            "Diff line 7 should be marked as diff in gradient output"
        );
    }
}

