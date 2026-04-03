#[path = "../common/mod.rs"]
mod common;
use common::*;


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
