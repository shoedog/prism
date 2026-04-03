#[path = "../../common/mod.rs"]
mod common;
use common::*;

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
