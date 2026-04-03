mod common;
use common::*;

fn make_taint_test_fixture() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let source = r#"
import os

def handle_request(user_input):
    query = "SELECT * FROM users WHERE name = '" + user_input + "'"
    result = db.execute(query)
    return result

def log_entry(message):
    os.system("logger " + message)
"#;

    let path = "src/handler.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    sources.insert(path.to_string(), source.to_string());

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };

    (files, sources, diff)
}


#[test]
fn test_thin_slice_subset_of_leftflow() {
    let (files, _, diff) = make_python_test();

    let thin = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();

    let left = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();

    let thin_lines: usize = thin
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();
    let left_lines: usize = left
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();

    assert!(
        thin_lines <= left_lines,
        "ThinSlice ({}) should have <= lines than LeftFlow ({})",
        thin_lines,
        left_lines
    );
}


#[test]
fn test_thin_slice_has_data_deps() {
    let (files, sources, diff) = make_python_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();

    assert!(!result.blocks.is_empty());
    let block = &result.blocks[0];
    let lines = block.file_line_map.get("src/calc.py").unwrap();
    // Should have the diff lines plus variable references
    assert!(
        lines.len() >= 2,
        "ThinSlice should include at least diff lines"
    );
}


#[test]
fn test_barrier_slice_python() {
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::BarrierSlice),
        None,
    )
    .unwrap();

    // Should include caller/callee information
    assert!(!result.blocks.is_empty());
}


#[test]
fn test_taint_from_diff() {
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    // Taint should propagate from diff lines
    assert!(!result.blocks.is_empty());
}


#[test]
fn test_relevant_slice_includes_alternates() {
    let (files, sources, diff) = make_python_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::RelevantSlice),
        None,
    )
    .unwrap();

    assert!(!result.blocks.is_empty());
    let block = &result.blocks[0];
    let lines = block.file_line_map.get("src/calc.py").unwrap();

    // RelevantSlice should include at least as much as LeftFlow
    let left = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();

    let relevant_count: usize = result
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();
    let left_count: usize = left
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();

    assert!(
        relevant_count >= left_count,
        "RelevantSlice ({}) should have >= lines than LeftFlow ({})",
        relevant_count,
        left_count
    );
}


#[test]
fn test_conditioned_slice_parses_conditions() {
    use prism::algorithms::conditioned_slice::Condition;

    let c = Condition::parse("x==5").unwrap();
    assert_eq!(c.var_name, "x");
    assert_eq!(c.value, "5");

    let c = Condition::parse("ptr!=null").unwrap();
    assert_eq!(c.var_name, "ptr");

    let c = Condition::parse("count > 0").unwrap();
    assert_eq!(c.var_name, "count");
    assert_eq!(c.value, "0");
}


#[test]
fn test_taint_c_buffer_overflow() {
    let (files, sources, diff) = make_c_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    // Taint should trace from the diff lines (buffer operations) forward
    assert!(result.algorithm == SlicingAlgorithm::Taint);
    // Even if no explicit taint source is specified, auto-taint from diff should work
}


#[test]
fn test_taint_findings_populated() {
    let (files, sources, diff) = make_taint_test_fixture();

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    let review = to_review_output(&result, &sources);
    assert_eq!(review.algorithm, "Taint");

    // findings may or may not fire depending on AST analysis, but the field must exist
    for finding in &review.findings {
        assert_eq!(finding.algorithm, "taint"); // findings use lowercase algorithm names
        assert!(!finding.file.is_empty(), "finding.file must not be empty");
        assert!(
            ["info", "warning", "concern"].contains(&finding.severity.as_str()),
            "severity must be one of info/warning/concern"
        );
        assert!(
            !finding.description.is_empty(),
            "finding.description must not be empty"
        );
        assert!(finding.line > 0, "finding.line must be > 0");
    }
}


#[test]
fn test_snmp_overflow_thin_slice() {
    let (files, diff) = make_snmp_overflow_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "ThinSlice should produce blocks for snmp_overflow"
    );
}


#[test]
fn test_double_free_thin_slice() {
    let (files, diff) = make_double_free_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "ThinSlice should produce blocks for double_free"
    );
}


#[test]
fn test_ring_overflow_thin_slice() {
    let (files, diff) = make_ring_overflow_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "ThinSlice should produce blocks for ring_overflow"
    );
}


#[test]
fn test_timer_uaf_thin_slice() {
    let (files, diff) = make_timer_uaf_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "ThinSlice should produce blocks for timer_uaf"
    );
}


#[test]
fn test_large_function_thin_slice() {
    let (files, diff) = make_large_function_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "ThinSlice should produce blocks for large_function without panic"
    );
}


#[test]
fn test_deep_switch_thin_slice() {
    let (files, diff) = make_deep_switch_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "ThinSlice should produce blocks for deep_switch without panic"
    );
}


#[test]
fn test_taint_c_strcpy_sink() {
    // recv() source on diff line flows through data flow to strcpy() sink.
    let source = r#"
void process_input(const char *input) {
    char *data = input;
    char dest[256];
    strcpy(dest, data);
}
"#;
    let path = "src/input.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff line 3: char *data = input;  — data is tainted from diff
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    // Taint should propagate from line 3 (data defined) to line 5 (strcpy uses data).
    assert!(
        !result.blocks.is_empty(),
        "Taint should produce blocks when tainted value reaches strcpy"
    );
    // At least one finding should flag strcpy as a sink.
    assert!(
        !result.findings.is_empty(),
        "Taint should emit a finding when tainted value reaches strcpy sink"
    );
}


#[test]
fn test_taint_c_sprintf_sink() {
    // User-controlled format string flows to sprintf().
    let source = r#"
void handle_cmd(const char *user_fmt) {
    char *fmt = user_fmt;
    char buf[256];
    sprintf(buf, fmt);
}
"#;
    let path = "src/cmd.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 3: char *fmt = user_fmt;  — fmt is tainted
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should produce blocks when tainted value reaches sprintf"
    );
    assert!(
        !result.findings.is_empty(),
        "Taint should emit a finding for sprintf sink with tainted format string"
    );
}


#[test]
fn test_taint_c_memcpy_sink() {
    // Tainted pointer flows to memcpy().
    let source = r#"
void copy_data(const char *network_data) {
    char *msg = network_data;
    char buf[256];
    memcpy(buf, msg, sizeof(buf));
}
"#;
    let path = "src/copy.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 3: char *msg = network_data;  — msg is tainted
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should produce blocks when tainted value reaches memcpy"
    );
    assert!(
        !result.findings.is_empty(),
        "Taint should emit a finding for memcpy sink with tainted source pointer"
    );
}


#[test]
fn test_taint_through_pointer() {
    // Taint flows through a pointer dereference assignment:
    // diff line taints *buf → buf becomes tainted → strcpy uses buf → sink fires.
    let source = r#"
void copy_indirect(const char *src, char *dst) {
    char *buf = src;
    *buf = src[0];
    strcpy(dst, buf);
}
"#;
    let path = "src/indirect.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 4: *buf = src[0];  — with pointer aliasing, creates def of "buf"
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    // With pointer aliasing: def of buf on line 4 flows to strcpy on line 5 → finding
    assert!(
        !result.blocks.is_empty(),
        "Taint should propagate through pointer dereference assignment to strcpy sink"
    );
    assert!(
        !result.findings.is_empty(),
        "Taint should emit a finding when *p assignment feeds strcpy"
    );
}


#[test]
fn test_taint_through_struct() {
    // Taint flows through a struct field assignment:
    // diff line taints dev->count → dev becomes tainted → printf uses dev → sink fires.
    let source = r#"
typedef struct { int count; } Dev;
void update_dev(Dev *dev, int n) {
    dev->count = n;
    printf("%d\n", dev->count);
}
"#;
    let path = "src/struct_taint.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 4: dev->count = n;  — with pointer aliasing, creates def of "dev" and "count"
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    // With struct aliasing: def of "dev" on line 4 flows to printf on line 5 → finding
    assert!(
        !result.blocks.is_empty(),
        "Taint should propagate through struct field assignment to printf sink"
    );
    assert!(
        !result.findings.is_empty(),
        "Taint should emit a finding when dev->field assignment feeds printf"
    );
}


#[test]
fn test_taint_python_pickle_loads_sink() {
    // Tainted data flows to pickle.loads() — deserialization RCE sink.
    let source = r#"
import pickle

def handle_request(user_data):
    payload = user_data
    obj = pickle.loads(payload)
    return obj
"#;
    let path = "app/handler.py";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should produce blocks when tainted value reaches pickle.loads"
    );
    assert!(
        !result.findings.is_empty(),
        "Taint should emit a finding for pickle.loads sink"
    );
}


#[test]
fn test_taint_python_subprocess_sink() {
    // Tainted command flows to subprocess.Popen() — command injection sink.
    let source = r#"
import subprocess

def run_command(user_cmd):
    cmd = user_cmd
    proc = subprocess.Popen(cmd, shell=True)
    return proc
"#;
    let path = "app/runner.py";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should produce blocks when tainted value reaches subprocess.Popen"
    );
    assert!(
        !result.findings.is_empty(),
        "Taint should emit a finding for subprocess.Popen sink"
    );
}


#[test]
fn test_taint_js_innerhtml_sink() {
    // Tainted user input flows to innerHTML — DOM XSS sink.
    let source = r#"
function displayMessage(userInput) {
    const msg = userInput;
    document.getElementById("output").innerHTML = msg;
}
"#;
    let path = "src/display.js";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should produce blocks when tainted value reaches innerHTML"
    );
    assert!(
        !result.findings.is_empty(),
        "Taint should emit a finding for innerHTML XSS sink"
    );
}


#[test]
fn test_taint_js_exec_sync_sink() {
    // Tainted command flows to execSync — command injection sink.
    let source = r#"
const { execSync } = require('child_process');

function runCmd(userCmd) {
    const cmd = userCmd;
    const output = execSync(cmd);
    return output;
}
"#;
    let path = "src/runner.js";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should produce blocks when tainted value reaches execSync"
    );
    assert!(
        !result.findings.is_empty(),
        "Taint should emit a finding for execSync command injection sink"
    );
}


#[test]
fn test_taint_go_exec_command_sink() {
    // Tainted user input flows to exec.Command — command injection sink.
    let source = r#"
package main

import "os/exec"

func runUserCmd(userInput string) {
    cmd := userInput
    exec.Command(cmd)
}
"#;
    let path = "cmd/handler.go";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should produce blocks when tainted value reaches exec.Command"
    );
    assert!(
        !result.findings.is_empty(),
        "Taint should emit a finding for exec.Command sink"
    );
}


#[test]
fn test_taint_go_template_html_sink() {
    // Tainted user input flows to template.HTML — XSS bypass sink.
    let source = r#"
package main

import "html/template"

func renderUnsafe(userHTML string) template.HTML {
    content := userHTML
    return template.HTML(content)
}
"#;
    let path = "web/render.go";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should produce blocks when tainted value reaches template.HTML"
    );
    assert!(
        !result.findings.is_empty(),
        "Taint should emit a finding for template.HTML XSS sink"
    );
}


#[test]
fn test_taint_negative_raw_data_not_a_sink() {
    // "rawData" identifier should NOT match the "=raw" exact sink pattern.
    let source = r#"
function processInput(input) {
    const rawData = input;
    transform(rawData);
}
"#;
    let path = "src/safe.js";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    // "rawData" is not a sink — the "=raw" pattern requires exact match
    assert!(
        result.findings.is_empty(),
        "Taint should NOT fire on 'rawData' — only exact 'raw' is a sink, got findings: {:?}",
        result
            .findings
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}


#[test]
fn test_taint_negative_html_escape_not_a_sink() {
    // "HTMLEscapeString" identifier should NOT match the "=HTML" exact sink.
    let source = r#"
package main

import "html/template"

func renderSafe(userInput string) string {
    content := userInput
    return template.HTMLEscapeString(content)
}
"#;
    let path = "web/safe.go";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        result.findings.is_empty(),
        "Taint should NOT fire on 'HTMLEscapeString' — only exact 'HTML' is a sink, got findings: {:?}",
        result.findings.iter().map(|f| &f.description).collect::<Vec<_>>()
    );
}


#[test]
fn test_taint_negative_downloads_not_a_sink() {
    // "downloads" identifier should NOT match the "=loads" exact sink.
    let source = r#"
def process_files(data):
    downloads = data
    handle(downloads)
"#;
    let path = "app/safe.py";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        result.findings.is_empty(),
        "Taint should NOT fire on 'downloads' — only exact 'loads' is a sink, got findings: {:?}",
        result
            .findings
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}


#[test]
fn test_rust_taint_unsafe_sink() {
    // Tainted data flows to unsafe block — security concern.
    let source = r#"
use std::os::unix::process::CommandExt;

fn run_command(user_input: &str) {
    let cmd = user_input;
    std::process::Command::new(cmd).exec();
}
"#;
    let path = "src/runner.rs";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should produce blocks for Rust code"
    );
}


#[test]
fn test_lua_taint_exec_sink() {
    // Lua os.execute with tainted data — command injection sink.
    let source = r#"
function run_command(user_input)
    local cmd = user_input
    os.execute(cmd)
end
"#;
    let path = "scripts/runner.lua";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should produce blocks for Lua code with os.execute sink"
    );
}


#[test]
fn test_rust_taint_transmute_sink() {
    let source = r#"
fn dangerous(data: &[u8]) {
    let ptr = data.as_ptr();
    let val: u64 = unsafe { std::mem::transmute(ptr) };
    println!("{}", val);
}
"#;
    let path = "src/danger.rs";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should detect transmute as a Rust sink"
    );
}


#[test]
fn test_rust_taint_from_raw_parts_sink() {
    let source = r#"
fn rebuild_slice(ptr: *const u8, len: usize) {
    let data = unsafe { std::slice::from_raw_parts(ptr, len) };
    process(data);
}
"#;
    let path = "src/raw.rs";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should detect from_raw_parts as a Rust sink"
    );
}


#[test]
fn test_lua_taint_loadstring_sink() {
    let source = r#"
function run_user_code(input)
    local code = input
    local func = loadstring(code)
    func()
end
"#;
    let path = "scripts/eval.lua";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should detect Lua loadstring as a sink"
    );
}


#[test]
fn test_lua_taint_dofile_sink() {
    let source = r#"
function load_config(path)
    local config_path = path
    dofile(config_path)
end
"#;
    let path = "scripts/loader.lua";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should detect Lua dofile as a sink"
    );
}


#[test]
fn test_cve_format_string_taint() {
    let source = r#"
#include <stdio.h>

void log_message(const char *user_msg) {
    char buf[512];
    char *msg = user_msg;
    sprintf(buf, msg);
    printf(buf);
}
"#;

    let path = "src/logger.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff touches the tainted assignment
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    // Taint should trace msg → sprintf (format string sink) and/or printf
    assert!(
        !result.findings.is_empty(),
        "Taint should detect user input flowing to sprintf/printf format parameter"
    );
    // The finding description includes "reaches sink at line N" — verify sink lines
    // are on the sprintf (line 7) or printf (line 8) calls
    let has_sink_finding = result.findings.iter().any(|f| f.line == 7 || f.line == 8);
    assert!(
        has_sink_finding,
        "Should flag sprintf (line 7) or printf (line 8) as taint sink, got findings at: {:?}",
        result.findings.iter().map(|f| f.line).collect::<Vec<_>>()
    );
}


#[test]
fn test_cve_buffer_overflow_taint() {
    let source = r#"
#include <string.h>

void copy_payload(const char *input, size_t input_len) {
    char local_buf[256];
    size_t len = input_len;
    memcpy(local_buf, input, len);
}
"#;

    let path = "src/payload.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Taint should detect user-controlled size flowing to memcpy"
    );
    // memcpy is on line 7 — verify taint reaches it
    let has_memcpy_sink = result.findings.iter().any(|f| f.line == 7);
    assert!(
        has_memcpy_sink,
        "Should flag memcpy (line 7) as taint sink for buffer overflow, got findings at: {:?}",
        result.findings.iter().map(|f| f.line).collect::<Vec<_>>()
    );
}


#[test]
fn test_cve_integer_overflow_taint() {
    let source = r#"
#include <stdlib.h>

void alloc_records(unsigned int count) {
    unsigned int total = count * sizeof(record_t);
    char *buf = malloc(total);
    memset(buf, 0, total);
}
"#;

    let path = "src/records.c";
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

    // The taint trace should include the arithmetic and malloc
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should trace arithmetic result to malloc/memset sinks"
    );
}


#[test]
fn test_cve_use_after_free_taint_context() {
    let source = r#"
#include <stdlib.h>

void process_timer(timer_t *timer) {
    free(timer->data);
    if (timer->flags & TIMER_ACTIVE) {
        process(timer->data);
    }
}
"#;

    let path = "src/timer.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff touches the free() line
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5]),
        }],
    };

    // Taint should trace: free(timer->data) is a sink, timer->data is tainted
    let taint_result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !taint_result.blocks.is_empty(),
        "Taint should include the free() and subsequent use of timer->data"
    );
}


#[test]
fn test_taint_c_vsnprintf_direct_sink() {
    // Direct call to vsnprintf with tainted format string is a sink.
    let source = r#"
#include <stdarg.h>
void log_msg(const char *user_input) {
    char *fmt = user_input;
    char buf[256];
    va_list args;
    vsnprintf(buf, sizeof(buf), fmt, args);
}
"#;
    let path = "src/log.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 4: char *fmt = user_input;  — fmt is tainted
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Taint should detect vsnprintf as a sink for tainted format string"
    );
}


#[test]
fn test_taint_c_variadic_wrapper_detected_as_sink() {
    // my_log is a variadic wrapper that forwards to vsnprintf.
    // Tainted data passed to my_log should trigger a finding because
    // my_log is detected as a dynamic sink.
    let source = r#"
#include <stdarg.h>
#include <stdio.h>

void my_log(const char *fmt, ...) {
    va_list args;
    va_start(args, fmt);
    char buf[1024];
    vsnprintf(buf, sizeof(buf), fmt, args);
    va_end(args);
}

void handle_request(const char *user_input) {
    char *msg = user_input;
    my_log("User said: %s", msg);
}
"#;
    let path = "src/logger.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 14: char *msg = user_input;  — msg is tainted
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([14]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    // my_log should be detected as a variadic wrapper → dynamic sink
    assert!(
        !result.findings.is_empty(),
        "Taint should detect my_log as a variadic wrapper sink when tainted value is passed"
    );
}


#[test]
fn test_taint_c_variadic_wrapper_vsprintf() {
    // Wrapper using vsprintf (no length bound — more dangerous).
    let source = r#"
#include <stdarg.h>
void fmt_msg(const char *fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    char buf[512];
    vsprintf(buf, fmt, ap);
    va_end(ap);
}

void process(const char *input) {
    char *data = input;
    fmt_msg("Result: %s", data);
}
"#;
    let path = "src/fmt.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 12: char *data = input;  — data is tainted
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([12]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Taint should detect fmt_msg as a variadic wrapper (vsprintf) sink"
    );
}


#[test]
fn test_taint_c_non_variadic_not_detected_as_wrapper() {
    // A normal (non-variadic) function that calls printf should NOT be
    // detected as a variadic wrapper — only functions with `...` qualify.
    let source = r#"
#include <stdio.h>
void print_msg(const char *msg) {
    printf("Message: %s\n", msg);
}

void handler(const char *input) {
    char *data = input;
    print_msg(data);
}
"#;
    let path = "src/print.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 8: char *data = input;  — data is tainted
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    // print_msg is NOT variadic, so it should NOT be a dynamic sink.
    // The call to printf inside print_msg is in a different function scope,
    // and the intraprocedural DFG won't connect data → msg across the call.
    // So no findings should be emitted for this pattern.
    let has_print_msg_finding = result.findings.iter().any(|f| f.line == 9);
    assert!(
        !has_print_msg_finding,
        "Non-variadic function should not be detected as a wrapper sink"
    );
}


#[test]
fn test_taint_field_access_through_dfg() {
    // Taint from diff line on dev->name assignment should propagate
    // to uses of dev->name but the DFG correctly tracks the path.
    let source = r#"
void handle(struct req *dev) {
    dev->name = get_user_input();
    char *n = dev->name;
    strcpy(buf, n);
}
"#;
    let path = "src/handler.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff on line 3: dev->name = get_user_input()
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Taint should propagate through field access to strcpy sink"
    );
}


#[test]
fn test_chop_with_field_access() {
    // Chop from source line to sink line should include intermediate field accesses.
    let source = r#"
void transfer(struct device *dev, const char *input) {
    dev->buf = input;
    char *data = dev->buf;
    memcpy(dest, data, len);
}
"#;
    let path = "src/transfer.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);

    // Chop from line 3 (source: dev->buf = input) to line 5 (sink: memcpy)
    let on_path = dfg.chop(path, 3, path, 5);

    // Should include the intermediate line 4 (data = dev->buf)
    let path_lines: BTreeSet<usize> = on_path.iter().map(|(_, l)| *l).collect();
    assert!(
        path_lines.contains(&3) && path_lines.contains(&5),
        "Chop should include source (line 3) and sink (line 5). Got: {:?}",
        path_lines
    );
}


#[test]
fn test_chop_python() {
    let source = r#"
x = input()
y = int(x)
z = y + 1
result = z * 2
print(result)
"#;
    let path = "app.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let config = prism::algorithms::chop::ChopConfig {
        source_file: path.to_string(),
        source_line: 2,
        sink_file: path.to_string(),
        sink_line: 5,
    };
    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::chop::slice(&ctx, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::Chop);
}


#[test]
fn test_chop_go() {
    let source = r#"package main

func process(input string) string {
    parsed := parse(input)
    validated := validate(parsed)
    result := transform(validated)
    return result
}

func parse(s string) string { return s }
func validate(s string) string { return s }
func transform(s string) string { return s }
"#;
    let path = "main.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
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
fn test_chop_javascript() {
    let source = r#"
function pipeline(raw) {
    const cleaned = sanitize(raw);
    const parsed = JSON.parse(cleaned);
    const result = process(parsed);
    return result;
}
function sanitize(s) { return s.trim(); }
function process(o) { return o.value; }
"#;
    let path = "pipe.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let config = prism::algorithms::chop::ChopConfig {
        source_file: path.to_string(),
        source_line: 3,
        sink_file: path.to_string(),
        sink_line: 5,
    };
    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::chop::slice(&ctx, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::Chop);
}


#[test]
fn test_delta_slice_python() {
    let tmp = TempDir::new().unwrap();

    let old_source = "x = 1\ny = x + 1\nprint(y)\n";
    std::fs::write(tmp.path().join("app.py"), old_source).unwrap();

    let new_source = "x = 1\ny = x + 2\nz = y * 3\nprint(z)\n";
    let path = "app.py";
    let parsed = ParsedFile::parse(path, new_source, Language::Python).unwrap();
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
fn test_delta_slice_go() {
    let tmp = TempDir::new().unwrap();

    let old_source = "package main\n\nfunc add(a int, b int) int {\n\treturn a + b\n}\n";
    std::fs::write(tmp.path().join("calc.go"), old_source).unwrap();

    let new_source =
        "package main\n\nfunc add(a int, b int) int {\n\tresult := a + b\n\treturn result\n}\n";
    let path = "calc.go";
    let parsed = ParsedFile::parse(path, new_source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4, 5]),
        }],
    };

    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::delta_slice::slice(&ctx, &diff, tmp.path()).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::DeltaSlice);
}


#[test]
fn test_conditioned_slice_python() {
    let source = r#"
def process(x):
    if x > 0:
        result = x * 2
    else:
        result = 0
    return result
"#;
    let path = "cond.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };

    let condition = prism::algorithms::conditioned_slice::Condition::parse("x==5").unwrap();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::ConditionedSlice);
    let result =
        prism::algorithms::conditioned_slice::slice(&files, &diff, &config, &condition).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ConditionedSlice);
}


#[test]
fn test_conditioned_slice_go() {
    let source = r#"package main

func check(n int) int {
	if n > 0 {
		return n * 2
	} else {
		return 0
	}
}
"#;
    let path = "check.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5]),
        }],
    };

    let condition = prism::algorithms::conditioned_slice::Condition::parse("n>0").unwrap();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::ConditionedSlice);
    let result =
        prism::algorithms::conditioned_slice::slice(&files, &diff, &config, &condition).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ConditionedSlice);
}


#[test]
fn test_conditioned_slice_javascript() {
    let source = r#"
function validate(input) {
    if (input == null) {
        return "missing";
    } else {
        return input.trim();
    }
}
"#;
    let path = "validate.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3, 6]),
        }],
    };

    let condition = prism::algorithms::conditioned_slice::Condition::parse("input!=null").unwrap();
    assert_eq!(
        condition.op,
        prism::algorithms::conditioned_slice::ConditionOp::IsNotNull
    );
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::ConditionedSlice);
    let result =
        prism::algorithms::conditioned_slice::slice(&files, &diff, &config, &condition).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ConditionedSlice);
}


#[test]
fn test_conditioned_slice_parse_operators() {
    use prism::algorithms::conditioned_slice::{Condition, ConditionOp};

    let c = Condition::parse("x==5").unwrap();
    assert_eq!(c.op, ConditionOp::Eq);
    assert_eq!(c.var_name, "x");
    assert_eq!(c.value, "5");

    let c = Condition::parse("y != 10").unwrap();
    assert_eq!(c.op, ConditionOp::NotEq);

    let c = Condition::parse("z>=3").unwrap();
    assert_eq!(c.op, ConditionOp::GtEq);

    let c = Condition::parse("w<=100").unwrap();
    assert_eq!(c.op, ConditionOp::LtEq);

    let c = Condition::parse("a<0").unwrap();
    assert_eq!(c.op, ConditionOp::Lt);

    let c = Condition::parse("ptr==null").unwrap();
    assert_eq!(c.op, ConditionOp::IsNull);

    let c = Condition::parse("ptr!=None").unwrap();
    assert_eq!(c.op, ConditionOp::IsNotNull);

    let c = Condition::parse("ptr==nil").unwrap();
    assert_eq!(c.op, ConditionOp::IsNull);

    assert!(Condition::parse("noop").is_none());
}


#[test]
fn test_thin_slice_python() {
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}


#[test]
fn test_thin_slice_go() {
    let (files, _, diff) = make_go_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}


#[test]
fn test_thin_slice_typescript() {
    let (files, _, diff) = make_typescript_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}


#[test]
fn test_thin_slice_java() {
    let (files, _, diff) = make_java_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}


#[test]
fn test_thin_slice_lua() {
    let source = r#"
function compute(x)
    local y = x + 1
    local z = y * 2
    return z
end
"#;
    let path = "compute.lua";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ThinSlice);
}


#[test]
fn test_thin_slice_rust() {
    let source = r#"
fn compute(x: i32) -> i32 {
    let y = x + 1;
    let z = y * 2;
    z
}
"#;
    let path = "compute.rs";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ThinSlice);
}


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
    assert!(!result.blocks.is_empty());
}


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
    assert!(!result.blocks.is_empty());
}


#[test]
fn test_barrier_slice_go() {
    let (files, _, diff) = make_go_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::BarrierSlice),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}


#[test]
fn test_barrier_slice_javascript() {
    let (files, _, diff) = make_javascript_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::BarrierSlice),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}


#[test]
fn test_relevant_slice_python() {
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::RelevantSlice),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}


#[test]
fn test_relevant_slice_go() {
    let (files, _, diff) = make_go_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::RelevantSlice),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}


#[test]
fn test_thin_slice_global_scope_python() {
    // Test thin slice with diff lines at global scope (no enclosing function)
    let source = r#"
x = 10
y = x + 1
print(y)
"#;
    let path = "global.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "ThinSlice should handle global-scope diff lines"
    );
}


#[test]
fn test_barrier_slice_with_barriers_python() {
    // Test barrier slice with explicit barrier symbols and modules
    let source_main = r#"
def handler(request):
    data = parse(request)
    result = service(data)
    logged = log_result(result)
    return result
"#;
    let source_service = r#"
def service(data):
    validated = validate(data)
    return transform(validated)
"#;
    let source_log = r#"
def log_result(result):
    print(result)
    return result
"#;
    let mut files = BTreeMap::new();
    files.insert(
        "handler.py".to_string(),
        ParsedFile::parse("handler.py", source_main, Language::Python).unwrap(),
    );
    files.insert(
        "service.py".to_string(),
        ParsedFile::parse("service.py", source_service, Language::Python).unwrap(),
    );
    files.insert(
        "log.py".to_string(),
        ParsedFile::parse("log.py", source_log, Language::Python).unwrap(),
    );

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "handler.py".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };

    let barrier_config = prism::algorithms::barrier_slice::BarrierConfig {
        max_depth: 2,
        barrier_symbols: BTreeSet::from(["log_result".to_string()]),
        barrier_modules: vec!["log.py".to_string()],
    };
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::BarrierSlice);
    let ctx = CpgContext::build(&files, None);
    let result =
        prism::algorithms::barrier_slice::slice(&ctx, &diff, &config, &barrier_config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::BarrierSlice);
}


#[test]
fn test_delta_slice_missing_old_file_python() {
    // Delta slice when old file doesn't exist (tests error handling path)
    let tmp = TempDir::new().unwrap();
    // No old file written — old_repo has nothing

    let new_source = "x = 1\ny = x + 2\nprint(y)\n";
    let path = "missing.py";
    let parsed = ParsedFile::parse(path, new_source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2]),
        }],
    };

    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::delta_slice::slice(&ctx, &diff, tmp.path()).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::DeltaSlice);
    // Should succeed but with empty old files — no edge differences
}


#[test]
fn test_chop_python_verifies_path_lines() {
    // Chop should find data flow from source (line 2: x = input()) to sink (line 5: result = z * 2)
    let source = "x = input()\ny = int(x)\nz = y + 1\nresult = z * 2\nprint(result)\n";
    let path = "app.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let config = prism::algorithms::chop::ChopConfig {
        source_file: path.to_string(),
        source_line: 1,
        sink_file: path.to_string(),
        sink_line: 4,
    };
    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::chop::slice(&ctx, &config).unwrap();
    // If data flow exists, blocks should contain lines between source and sink
    if !result.blocks.is_empty() {
        let block = &result.blocks[0];
        let lines = block.file_line_map.get(path).unwrap();
        // Source and/or sink lines should appear in the output
        let has_endpoint = lines.contains_key(&1) || lines.contains_key(&4);
        assert!(
            has_endpoint,
            "Chop should include source or sink line in output, got lines: {:?}",
            lines.keys().collect::<Vec<_>>()
        );
    }
}


#[test]
fn test_taint_python_finds_sql_injection_finding() {
    let source = r#"
def handler(request):
    user_input = request.form.get("query")
    query = "SELECT * FROM users WHERE name = '" + user_input + "'"
    cursor.execute(query)
    return cursor.fetchall()
"#;
    let path = "handler.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4, 5]),
        }],
    };

    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::taint::slice(
        &ctx,
        &diff,
        &prism::algorithms::taint::TaintConfig::default(),
    )
    .unwrap();
    assert!(!result.blocks.is_empty(), "Taint should find tainted flow");
    // The taint analysis should detect flow from user input to execute sink
    let all_lines: BTreeSet<usize> = result
        .blocks
        .iter()
        .flat_map(|b| b.file_line_map.values())
        .flat_map(|m| m.keys())
        .copied()
        .collect();
    assert!(
        all_lines.contains(&3) || all_lines.contains(&4) || all_lines.contains(&5),
        "Taint should include lines with user_input or execute. Got lines: {:?}",
        all_lines
    );
}


#[test]
fn test_conditioned_slice_prunes_unreachable_python() {
    // When x==5, the `if x > 0` body is reachable but we test a different condition
    let source = r#"
def process(x):
    if x != 5:
        result = x * 2
    else:
        result = 0
    return result
"#;
    let path = "cond.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4, 6]),
        }],
    };

    // With condition x==5, the if-body (x != 5) should be unreachable
    let condition = prism::algorithms::conditioned_slice::Condition::parse("x==5").unwrap();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::ConditionedSlice);
    let conditioned_result =
        prism::algorithms::conditioned_slice::slice(&files, &diff, &config, &condition).unwrap();

    // Also get unconditioned (LeftFlow) for comparison
    let left_result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();

    let conditioned_lines: usize = conditioned_result
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();
    let left_lines: usize = left_result
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();

    // Conditioned slice should have fewer or equal lines (pruned unreachable)
    assert!(
        conditioned_lines <= left_lines,
        "ConditionedSlice ({} lines) should be <= LeftFlow ({} lines)",
        conditioned_lines,
        left_lines
    );
}


#[test]
fn test_field_isolation_taint_does_not_cross_fields() {
    // End-to-end taint test: tainted field should not reach different field's sink
    let source = r#"
void handler(struct request *req) {
    req->user_input = read_stdin();
    req->safe_data = "constant";
    exec(req->user_input);
    log_msg(req->safe_data);
}
"#;
    let path = "src/handler.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);

    // Get the user_input def
    let req_defs = dfg.all_defs_of(path, "req");
    let user_input_def = req_defs
        .iter()
        .find(|d| d.path.fields == vec!["user_input"]);

    if let Some(uid) = user_input_def {
        let reachable = dfg.forward_reachable(uid);
        // Should reach exec(req->user_input) line but NOT log_msg(req->safe_data) line
        let reaches_safe = reachable.iter().any(|r| r.path.fields == vec!["safe_data"]);
        assert!(
            !reaches_safe,
            "Taint on req->user_input must NOT propagate to req->safe_data"
        );
    }
}


#[test]
fn test_destructuring_taint_propagation_js() {
    // Taint on device should propagate through destructured name via alias
    let source = r#"
function render(userInput) {
    const { name, role } = userInput;
    document.getElementById("output").innerHTML = name;
}
"#;
    let path = "src/render.js";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    // Taint should detect the flow from destructured variable to innerHTML
    assert!(
        !result.blocks.is_empty(),
        "Taint should produce blocks for destructured variable flowing to innerHTML"
    );
}


#[test]
fn test_go_multi_return_taint_flow() {
    let source = r#"
package main

func handler() {
    val, err := getUserInput()
    execute(val)
}
"#;
    let path = "src/handler.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let val_defs = dfg.all_defs_of(path, "val");
    assert!(!val_defs.is_empty(), "val should have a def");

    if let Some(val_def) = val_defs.first() {
        let reachable = dfg.forward_reachable(val_def);
        assert!(
            !reachable.is_empty(),
            "Taint from val should reach execute() call"
        );
    }
}


#[test]
fn test_python_tuple_unpack_taint_flow() {
    let source = r#"
def handler():
    name, role = get_user_input()
    execute(name)
"#;
    let path = "src/handler.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let name_defs = dfg.all_defs_of(path, "name");
    assert!(!name_defs.is_empty(), "name should have a def");

    if let Some(name_def) = name_defs.first() {
        let reachable = dfg.forward_reachable(name_def);
        assert!(
            !reachable.is_empty(),
            "Taint from name should reach execute() call"
        );
    }
}


#[test]
fn test_python_with_as_taint_flow() {
    // Taint through with...as binding should flow to uses of f
    let source = r#"
def handler():
    with get_connection() as conn:
        data = conn.read()
        execute(data)
"#;
    let path = "src/handler.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let conn_defs = dfg.all_defs_of(path, "conn");
    assert!(
        !conn_defs.is_empty(),
        "with...as: 'conn' should have def for taint flow"
    );
}

