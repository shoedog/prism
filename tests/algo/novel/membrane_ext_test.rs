use crate::common::*;

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

// F2 (P3 review-fix wave): an unknown-receiver, multi-owner candidate edge
// (`R6MultiOwnerCandidate`) is an unverified maybe-caller — nav-only. Membrane
// must not assert it as a cross-module "unprotected call" finding fact (it
// traverses `ConfidenceFilter::All`, so the candidate Call edge is otherwise
// visible to it).
#[test]
fn membrane_slice_skips_multi_owner_candidate_caller() {
    let api_source = "class A:\n    def handle(self):\n        return 1\n\n\nclass B:\n    def handle(self):\n        return 2\n";
    let caller_source = "def run(x):\n    x.handle()\n";

    let mut files = BTreeMap::new();
    files.insert(
        "api.py".to_string(),
        ParsedFile::parse("api.py", api_source, Language::Python).unwrap(),
    );
    files.insert(
        "caller.py".to_string(),
        ParsedFile::parse("caller.py", caller_source, Language::Python).unwrap(),
    );

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "api.py".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();

    assert!(
        !result
            .findings
            .iter()
            .any(|f| f.function_name.as_deref() == Some("run")),
        "MembraneSlice must not name the candidate-only caller 'run' as fact: {:?}",
        result
            .findings
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}

// Review-fix (post-F2): the F2 candidate-exclusion gate was derived ONLY from
// resolved_caller_edges (a scan of literal, text-based call sites) and used to
// gate cross_file_callers wholesale. That over-filters: a CPG-graph-only Call
// edge — e.g. the type-enriched CHA virtual-dispatch fan-out minted directly
// onto the CPG graph in cpg/build.rs (C/C++ with a TypeDatabase) — never
// appears in resolved_caller_edges at all (it bypasses CallGraph::calls
// entirely), so the old gate silently dropped a legitimate cross-file caller
// that has zero raw-edge evidence, not just candidate-only evidence.
//
// Fixture: `drive()` in caller.cpp makes one literal, qualified call —
// `D::Handle();` — which resolves (Exact) straight to `D::Handle`. CHA then
// fans out an additional CPG-graph-only Exact edge `drive -> E::Handle` (same
// virtual method name, sibling override), since caller.cpp and types.cpp are
// both type_db-owned files. `E::Handle` therefore has a caller (`drive`)
// visible only via ConfidenceFilter::All graph traversal, with NO entry at all
// in resolved_caller_edges(E::Handle) — the "no raw edges" case the fix must
// pass through unfiltered.
#[test]
fn membrane_includes_graph_only_cha_caller_with_no_raw_edge() {
    use prism::type_db::{RecordInfo, RecordKind, TypeDatabase};

    let types_source = r#"
struct Base { virtual void Handle(); };
struct D : Base { void Handle() override {} };
struct E : Base { void Handle() override {} };
"#;
    let caller_source = r#"
void drive() {
    D::Handle();
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/types.cpp".to_string(),
        ParsedFile::parse("src/types.cpp", types_source, Language::Cpp).unwrap(),
    );
    files.insert(
        "src/caller.cpp".to_string(),
        ParsedFile::parse("src/caller.cpp", caller_source, Language::Cpp).unwrap(),
    );

    let mut tdb = TypeDatabase::default();
    tdb.records.insert(
        "Base".to_string(),
        RecordInfo {
            name: "Base".to_string(),
            kind: RecordKind::Struct,
            fields: vec![],
            bases: vec![],
            virtual_methods: BTreeMap::from([("Handle".to_string(), "void()".to_string())]),
            size: None,
            file: "src/types.cpp".to_string(),
        },
    );
    // Marks caller.cpp as type_db-owned too (by file path only — no relation to
    // Base needed), so the CHA fan-out isn't suppressed at the caller's file
    // boundary (see `cha_does_not_seed_from_unowned_cpp_caller` in cpg_test.rs
    // for the inverse case).
    tdb.records.insert(
        "CallerMarker".to_string(),
        RecordInfo {
            name: "CallerMarker".to_string(),
            kind: RecordKind::Struct,
            fields: vec![],
            bases: vec![],
            virtual_methods: BTreeMap::new(),
            size: None,
            file: "src/caller.cpp".to_string(),
        },
    );

    // E::Handle is line 4 of types_source (leading blank line from the raw string).
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/types.cpp".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };

    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, Some(&tdb)).unwrap();

    let has_caller = result
        .blocks
        .iter()
        .any(|b| b.file_line_map.contains_key("src/caller.cpp"));
    assert!(
        has_caller,
        "MembraneSlice must include the graph-only CHA caller (drive) even though \
         it has no raw call-site edge to E::Handle, got blocks: {:?}",
        result.blocks
    );
}
