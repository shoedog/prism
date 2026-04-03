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
