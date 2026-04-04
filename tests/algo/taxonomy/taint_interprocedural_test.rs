#[path = "../../common/mod.rs"]
mod common;
use common::*;

/// Multi-file fixture: caller.py passes tainted data to callee.py which has a sink.
fn make_python_interprocedural() -> (BTreeMap<String, ParsedFile>, DiffInput) {
    let caller_source = r#"
def handler(request):
    user_data = request.input
    result = process_data(user_data)
    return result
"#;

    let callee_source = r#"
def process_data(data):
    query = "SELECT * FROM users WHERE name = " + data
    cursor.execute(query)
    return query
"#;

    let mut files = BTreeMap::new();
    let caller = ParsedFile::parse("src/caller.py", caller_source, Language::Python).unwrap();
    let callee = ParsedFile::parse("src/callee.py", callee_source, Language::Python).unwrap();
    files.insert("src/caller.py".to_string(), caller);
    files.insert("src/callee.py".to_string(), callee);

    // Diff touches caller line 3: user_data = request.input
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/caller.py".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    (files, diff)
}

#[test]
fn test_taint_interprocedural_python_sql_injection() {
    let (files, diff) = make_python_interprocedural();

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    // Taint should flow: request.input → user_data → process_data(user_data) →
    // data parameter → query → cursor.execute(query)
    // The execute() call is a known taint sink.
    assert!(
        !result.findings.is_empty(),
        "Interprocedural taint should detect SQL injection through function call"
    );
}

#[test]
fn test_taint_interprocedural_c_format_string() {
    // C: tainted input flows through a non-variadic function to sprintf.
    let source = r#"
#include <stdio.h>
void format_msg(const char *msg, char *out) {
    sprintf(out, "%s: %s", "prefix", msg);
}

void handler(const char *input) {
    char buf[256];
    char *data = input;
    format_msg(data, buf);
}
"#;
    let path = "src/format.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 9: data = input (taint source)
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    // Taint should flow: input → data → format_msg(data, buf) → msg param → sprintf(...)
    // sprintf is a known sink.
    assert!(
        !result.findings.is_empty(),
        "Interprocedural taint should detect format string sink through non-variadic call"
    );
}

#[test]
fn test_taint_interprocedural_js_xss() {
    let source = r#"
function sanitize(input) {
    return input;
}

function renderPage(req) {
    let userInput = req.query.name;
    let cleaned = sanitize(userInput);
    document.innerHTML = cleaned;
}
"#;
    let path = "src/app.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 7: userInput = req.query.name (taint source)
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

    // innerHTML is a known XSS sink. Taint should flow:
    // req.query.name → userInput → sanitize(userInput) → input param → return → cleaned → innerHTML
    assert!(
        !result.blocks.is_empty(),
        "Interprocedural taint should trace through sanitize() to innerHTML"
    );
}

#[test]
fn test_taint_interprocedural_go_exec() {
    let source = r#"
package main

func buildCommand(input string) string {
    cmd := "echo " + input
    return cmd
}

func handler(userInput string) {
    command := buildCommand(userInput)
    exec.Command(command)
}
"#;
    let path = "src/main.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 10: command = buildCommand(userInput) — taint via return value
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([10]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    // exec.Command is a known sink.
    assert!(
        !result.blocks.is_empty(),
        "Interprocedural taint should trace through buildCommand() to exec.Command"
    );
}

#[test]
fn test_taint_interprocedural_multifile_c() {
    // Multi-file: caller in one file passes tainted data to callee in another.
    let caller_source = r#"
void process_request(const char *input) {
    char *data = input;
    log_message(data);
}
"#;

    let callee_source = r#"
void log_message(const char *msg) {
    printf(msg);
}
"#;

    let mut files = BTreeMap::new();
    let caller = ParsedFile::parse("src/handler.c", caller_source, Language::C).unwrap();
    let callee = ParsedFile::parse("src/logger.c", callee_source, Language::C).unwrap();
    files.insert("src/handler.c".to_string(), caller);
    files.insert("src/logger.c".to_string(), callee);

    // Line 3: data = input (taint source)
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/handler.c".to_string(),
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

    // printf is a known sink. Taint should flow across files:
    // input → data → log_message(data) → msg → printf(msg)
    assert!(
        !result.findings.is_empty(),
        "Interprocedural taint should propagate across files to printf sink"
    );
}
