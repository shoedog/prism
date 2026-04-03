mod common;
use common::*;

// ====== Output format tests ======

#[test]
fn test_paper_format_output() {
    let (files, _, diff) = make_python_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();

    let paper = output::to_paper_format(&result.blocks);
    assert!(paper.is_array());
    let arr = paper.as_array().unwrap();
    assert!(!arr.is_empty());
    assert!(arr[0].get("block_id").is_some());
    assert!(arr[0].get("diff_lines").is_some());
    assert!(arr[0].get("diff_list").is_some());
}

#[test]
fn test_text_format_output() {
    let (files, sources, diff) = make_python_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();

    let text = output::format_slice_result(&result.blocks, &sources);
    // Should have diff markers
    assert!(text.contains('+'), "Should have + markers for diff lines");
    // Should have block header
    assert!(text.contains("Block"), "Should have block header");
}

// ====== Diff parsing tests ======

#[test]
fn test_json_diff_input() {
    let json = r#"{
        "files": [
            {
                "file_path": "test.py",
                "modify_type": "Modified",
                "diff_lines": [1, 5, 10]
            }
        ]
    }"#;

    let input = DiffInput::from_json(json).unwrap();
    assert_eq!(input.files.len(), 1);
    assert_eq!(input.files[0].diff_lines.len(), 3);
}

// ====== Multi-language parsing tests ======

#[test]
fn test_all_languages_parse() {
    let cases = vec![
        ("test.py", Language::Python, "def foo():\n    return 1\n"),
        (
            "test.js",
            Language::JavaScript,
            "function foo() { return 1; }\n",
        ),
        (
            "test.ts",
            Language::TypeScript,
            "function foo(): number { return 1; }\n",
        ),
        (
            "test.go",
            Language::Go,
            "package main\nfunc foo() int { return 1 }\n",
        ),
        (
            "test.java",
            Language::Java,
            "class T { int foo() { return 1; } }\n",
        ),
    ];

    for (path, lang, source) in cases {
        let parsed = ParsedFile::parse(path, source, lang);
        assert!(
            parsed.is_ok(),
            "Failed to parse {}: {:?}",
            path,
            parsed.err()
        );
    }
}

// ====== Algorithm listing test ======

#[test]
fn test_all_algorithms_listed() {
    let all = SlicingAlgorithm::all();
    assert_eq!(all.len(), 26, "Should have 26 algorithms total");

    // Verify each can be round-tripped through from_str
    for algo in &all {
        let name = algo.name();
        let parsed = SlicingAlgorithm::from_str(name);
        assert!(parsed.is_some(), "Should parse algorithm name: {}", name);
    }
}

// ====== C LeftFlow tests ======

#[test]
fn test_left_flow_c() {
    let (files, sources, diff) = make_c_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();

    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce output for C code"
    );
    let formatted = output::format_slice_result(&result.blocks, &sources);
    // Should include the process_packet function context
    assert!(
        formatted.contains("local_buf")
            || formatted.contains("memcpy")
            || formatted.contains("buf"),
        "LeftFlow should trace buffer-related variables in C code"
    );
}

#[test]
fn test_left_flow_cpp() {
    let (files, sources, diff) = make_cpp_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();

    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce output for C++ code"
    );
    let formatted = output::format_slice_result(&result.blocks, &sources);
    assert!(
        formatted.contains("device")
            || formatted.contains("lock")
            || formatted.contains("add_device"),
        "LeftFlow should include C++ method context"
    );
}

// ====== C FullFlow tests ======

#[test]
fn test_full_flow_c() {
    let (files, sources, diff) = make_c_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();

    assert!(
        !result.blocks.is_empty(),
        "FullFlow should produce output for C code"
    );
    let formatted = output::format_slice_result(&result.blocks, &sources);
    assert!(
        formatted.contains("result") || formatted.contains("local_buf"),
        "FullFlow should trace forward from the buffer operations"
    );
}

#[test]
fn test_full_flow_cpp() {
    let (files, sources, diff) = make_cpp_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();

    assert!(
        !result.blocks.is_empty(),
        "FullFlow should produce output for C++ code"
    );
}

use prism::output::{to_review_output, MultiReviewOutput};
use prism::slice::{MultiSliceResult, SliceFinding};

#[test]
fn test_review_output_format_single_algorithm() {
    let (files, sources, diff) = make_python_test();

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();

    let review = to_review_output(&result, &sources);

    // Verify schema fields
    assert_eq!(review.algorithm, "LeftFlow");
    assert!(
        review.slices.iter().all(|s| !s.file.is_empty()),
        "Each slice block should have a file"
    );
    assert!(
        review.slices.iter().all(|s| !s.modify_type.is_empty()),
        "Each slice block should have a modify_type"
    );

    // Verify serialization to JSON succeeds and is valid
    let json = serde_json::to_string_pretty(&review).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["algorithm"], "LeftFlow");
    assert!(parsed["slices"].is_array());
    assert!(parsed["findings"].is_array());
}

#[test]
fn test_review_output_json_schema_multi() {
    let (files, sources, diff) = make_python_test();

    let algorithms_to_run = vec![SlicingAlgorithm::LeftFlow, SlicingAlgorithm::ThinSlice];
    let mut results = vec![];
    for &algo in &algorithms_to_run {
        let r = algorithms::run_slicing_compat(
            &files,
            &diff,
            &SliceConfig::default().with_algorithm(algo),
            None,
        )
        .unwrap();
        results.push(r);
    }

    let algorithms_run: Vec<String> = algorithms_to_run
        .iter()
        .map(|a| a.name().to_string())
        .collect();
    let all_findings: Vec<SliceFinding> = results.iter().flat_map(|r| r.findings.clone()).collect();
    let review_results: Vec<_> = results
        .iter()
        .map(|r| to_review_output(r, &sources))
        .collect();

    let multi = MultiReviewOutput {
        version: "1.0".to_string(),
        algorithms_run: algorithms_run.clone(),
        results: review_results,
        all_findings,
        errors: vec![],
        warnings: vec![],
    };

    // Verify schema
    assert_eq!(multi.version, "1.0");
    assert_eq!(multi.algorithms_run.len(), 2);
    assert!(multi.algorithms_run.contains(&"LeftFlow".to_string()));
    assert!(multi.algorithms_run.contains(&"ThinSlice".to_string()));
    assert_eq!(multi.results.len(), 2);

    // Verify valid JSON
    let json = serde_json::to_string_pretty(&multi).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["version"], "1.0");
    assert_eq!(parsed["algorithms_run"].as_array().unwrap().len(), 2);
    assert!(parsed["results"].is_array());
    assert!(parsed["all_findings"].is_array());
}

#[test]
fn test_review_suite_list() {
    let suite = SlicingAlgorithm::review_suite();
    // Review suite should be non-empty and contain core algorithms
    assert!(!suite.is_empty());
    assert!(suite.contains(&SlicingAlgorithm::LeftFlow));
    assert!(suite.contains(&SlicingAlgorithm::FullFlow));
    assert!(suite.contains(&SlicingAlgorithm::Taint));
    assert!(suite.contains(&SlicingAlgorithm::AbsenceSlice));
    assert!(suite.contains(&SlicingAlgorithm::EchoSlice));
    // Git-history-only algorithms should NOT be in the review suite
    assert!(!suite.contains(&SlicingAlgorithm::ResonanceSlice));
    assert!(!suite.contains(&SlicingAlgorithm::PhantomSlice));
}

#[test]
fn test_multi_algorithm_findings_merged() {
    let (files, sources, diff) = make_python_test();

    let algorithms_to_run = SlicingAlgorithm::review_suite();
    let mut all_results = vec![];
    let mut errors = vec![];

    for &algo in &algorithms_to_run {
        match algorithms::run_slicing_compat(
            &files,
            &diff,
            &SliceConfig::default().with_algorithm(algo),
            None,
        ) {
            Ok(r) => all_results.push(r),
            Err(e) => errors.push(e.to_string()),
        }
    }

    // Collect all findings across all algorithms
    let merged_findings: Vec<SliceFinding> = all_results
        .iter()
        .flat_map(|r| r.findings.clone())
        .collect();

    // All findings should have non-empty required fields
    for finding in &merged_findings {
        assert!(!finding.algorithm.is_empty());
        assert!(!finding.file.is_empty());
        assert!(!finding.description.is_empty());
        assert!(["info", "warning", "concern"].contains(&finding.severity.as_str()));
    }

    // Results count should match algorithms that succeeded (no panics)
    let review_results: Vec<_> = all_results
        .iter()
        .map(|r| to_review_output(r, &sources))
        .collect();
    let multi = MultiSliceResult {
        version: "1.0".to_string(),
        algorithms_run: algorithms_to_run
            .iter()
            .map(|a| a.name().to_string())
            .collect(),
        results: all_results,
        findings: merged_findings,
        errors: vec![],
        warnings: vec![],
    };

    assert_eq!(multi.version, "1.0");
    assert_eq!(multi.results.len(), review_results.len());
    assert!(multi.algorithms_run.contains(&"LeftFlow".to_string()));

    // JSON round-trip
    let json = serde_json::to_string_pretty(&multi).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["results"].is_array());
    assert!(parsed["findings"].is_array());
}

// (C-specific tests moved to c_test.rs)

// --- Quantum async detection tests ---

// --- Data flow graph unit tests ---

// --- End-to-end taint tests through pointer/struct ---

// ---------------------------------------------------------------------------
// Algorithm × Language coverage matrix
// ---------------------------------------------------------------------------

/// Prints a matrix of which algorithm × language combinations have integration
/// tests. This test always passes — it is a documentation/reporting tool that
/// makes coverage gaps visible at a glance.
///
/// Check whether test name `name` matches language `lang_key`.
///
/// Uses word-boundary-aware matching so that e.g. "java" does not
/// false-positive on "javascript", and "_c_" does not match "circular".
fn lang_matches(name: &str, lang_key: &str) -> bool {
    // For multi-char language keys that are full words (python, javascript,
    // typescript, rust, lua) a simple contains is safe because no other
    // language name is a prefix/suffix of these.
    match lang_key {
        "python" | "javascript" | "typescript" | "rust" | "lua" => name.contains(lang_key),
        // "go" is short — require _go_ or _go at end to avoid matching "algorithm"
        "go" => name.contains("_go_") || name.ends_with("_go"),
        // "java" must not match "javascript"
        "java" => {
            !name.contains("javascript") && (name.contains("_java_") || name.ends_with("_java"))
        }
        // "c" must not match cpp, circular, conditioned, chop, etc.
        "c" => !name.contains("_cpp") && (name.contains("_c_") || name.ends_with("_c")),
        // "cpp" — require _cpp boundary
        "cpp" => name.contains("_cpp_") || name.ends_with("_cpp"),
        _ => name.contains(lang_key),
    }
}

/// Run with: cargo test -- test_algorithm_language_matrix --nocapture
#[test]
fn test_algorithm_language_matrix() {
    // Map algorithm keywords → display name.
    // Each entry is (&[keywords], display_name). A test matches if it
    // contains ANY of the keywords. This accommodates tests that use
    // either the short form ("membrane") or the full form ("membrane_slice").
    let algorithms: &[(&[&str], &str)] = &[
        (&["original_diff"], "OriginalDiff"),
        (&["parent_function"], "ParentFunction"),
        (&["left_flow"], "LeftFlow"),
        (&["full_flow"], "FullFlow"),
        (&["thin_slice"], "ThinSlice"),
        (&["barrier_slice"], "BarrierSlice"),
        (&["taint"], "Taint"),
        (&["relevant_slice"], "RelevantSlice"),
        (&["conditioned_slice", "conditioned"], "ConditionedSlice"),
        (&["delta_slice"], "DeltaSlice"),
        (&["spiral_slice", "spiral"], "SpiralSlice"),
        (&["circular_slice", "circular"], "CircularSlice"),
        (&["quantum_slice", "quantum"], "QuantumSlice"),
        (&["horizontal_slice", "horizontal"], "HorizontalSlice"),
        (&["vertical_slice", "vertical"], "VerticalSlice"),
        (&["angle_slice", "angle"], "AngleSlice"),
        (&["threed_slice", "threed"], "ThreeDSlice"),
        (&["absence_slice", "absence"], "AbsenceSlice"),
        (&["resonance_slice", "resonance"], "ResonanceSlice"),
        (&["symmetry_slice", "symmetry"], "SymmetrySlice"),
        (&["gradient_slice", "gradient"], "GradientSlice"),
        (&["provenance_slice", "provenance"], "ProvenanceSlice"),
        (&["phantom_slice", "phantom"], "PhantomSlice"),
        (&["membrane_slice", "membrane"], "MembraneSlice"),
        (&["echo_slice", "echo"], "EchoSlice"),
        (&["chop"], "Chop"),
    ];

    // All 9 supported languages
    let languages: &[(&str, &str)] = &[
        ("python", "Python"),
        ("javascript", "JS"),
        ("typescript", "TS"),
        ("go", "Go"),
        ("java", "Java"),
        ("c", "C"),
        ("cpp", "C++"),
        ("rust", "Rust"),
        ("lua", "Lua"),
    ];

    // Collect all test function names from this file (compile-time string)
    let all_test_files = &[
        "tests/integration_test.rs",
        "tests/algo_paper_test.rs",
        "tests/algo_taxonomy_test.rs",
        "tests/algo_theoretical_test.rs",
        "tests/algo_novel_absence_test.rs",
        "tests/algo_novel_provenance_test.rs",
        "tests/algo_novel_membrane_test.rs",
        "tests/algo_novel_echo_misc_test.rs",
        "tests/c_test.rs",
        "tests/cpp_test.rs",
        "tests/rust_lang_test.rs",
        "tests/lua_test.rs",
        "tests/javascript_test.rs",
        "tests/typescript_test.rs",
        "tests/ast_test.rs",
    ];
    let mut test_names_buf: Vec<String> = Vec::new();
    for tf in all_test_files {
        if let Ok(src) = std::fs::read_to_string(tf) {
            for line in src.lines() {
                let t = line.trim();
                if t.starts_with("fn test_") {
                    if let Some(n) = t.trim_start_matches("fn ").split('(').next() {
                        test_names_buf.push(n.to_string());
                    }
                }
            }
        }
    }
    let test_names: Vec<&str> = test_names_buf.iter().map(|s| s.as_str()).collect();

    // Build the matrix
    let col_w = 10usize;
    let row_w = 18usize;

    // Header
    let header: String = languages
        .iter()
        .map(|(_, name)| format!("{:>col_w$}", name))
        .collect::<Vec<_>>()
        .join("");
    println!("\nAlgorithm × Language Test Coverage Matrix");
    println!("{}", "=".repeat(row_w + col_w * languages.len()));
    println!("{:<row_w$}{}", "", header);
    println!("{}", "-".repeat(row_w + col_w * languages.len()));

    let mut covered = 0usize;
    let mut total = 0usize;

    for (algo_keys, algo_name) in algorithms {
        let row: String = languages
            .iter()
            .map(|(lang_key, _)| {
                total += 1;
                let has_test = test_names.iter().any(|name| {
                    algo_keys.iter().any(|k| name.contains(k)) && lang_matches(name, lang_key)
                });
                if has_test {
                    covered += 1;
                    format!("{:>col_w$}", "✓")
                } else {
                    format!("{:>col_w$}", "-")
                }
            })
            .collect::<Vec<_>>()
            .join("");
        println!("{:<row_w$}{}", algo_name, row);
    }

    println!("{}", "=".repeat(row_w + col_w * languages.len()));
    println!(
        "Coverage: {}/{} algorithm×language combinations ({:.0}%)",
        covered,
        total,
        covered as f64 / total as f64 * 100.0
    );
    println!();

    // Always passes — this is a reporting tool, not an enforcement test
}

/// Enforces that every algorithm has tests for at least MIN_LANGS languages.
/// Fails CI if a new algorithm is added without cross-language tests.
///
/// Run with: cargo test -- test_language_coverage_minimum
#[test]
fn test_language_coverage_minimum() {
    const MIN_LANGS: usize = 2;

    let algorithms: &[(&[&str], &str)] = &[
        (&["original_diff"], "OriginalDiff"),
        (&["parent_function"], "ParentFunction"),
        (&["left_flow"], "LeftFlow"),
        (&["full_flow"], "FullFlow"),
        (&["thin_slice"], "ThinSlice"),
        (&["barrier_slice"], "BarrierSlice"),
        (&["taint"], "Taint"),
        (&["relevant_slice"], "RelevantSlice"),
        (&["conditioned_slice", "conditioned"], "ConditionedSlice"),
        (&["delta_slice"], "DeltaSlice"),
        (&["spiral_slice", "spiral"], "SpiralSlice"),
        (&["circular_slice", "circular"], "CircularSlice"),
        (&["quantum_slice", "quantum"], "QuantumSlice"),
        (&["horizontal_slice", "horizontal"], "HorizontalSlice"),
        (&["vertical_slice", "vertical"], "VerticalSlice"),
        (&["angle_slice", "angle"], "AngleSlice"),
        (&["threed_slice", "threed"], "ThreeDSlice"),
        (&["absence_slice", "absence"], "AbsenceSlice"),
        (&["resonance_slice", "resonance"], "ResonanceSlice"),
        (&["symmetry_slice", "symmetry"], "SymmetrySlice"),
        (&["gradient_slice", "gradient"], "GradientSlice"),
        (&["provenance_slice", "provenance"], "ProvenanceSlice"),
        (&["phantom_slice", "phantom"], "PhantomSlice"),
        (&["membrane_slice", "membrane"], "MembraneSlice"),
        (&["echo_slice", "echo"], "EchoSlice"),
        (&["chop"], "Chop"),
    ];

    let lang_keys: &[&str] = &[
        "python",
        "javascript",
        "typescript",
        "go",
        "java",
        "c",
        "cpp",
        "rust",
        "lua",
    ];

    let all_test_files = &[
        "tests/integration_test.rs",
        "tests/algo_paper_test.rs",
        "tests/algo_taxonomy_test.rs",
        "tests/algo_theoretical_test.rs",
        "tests/algo_novel_absence_test.rs",
        "tests/algo_novel_provenance_test.rs",
        "tests/algo_novel_membrane_test.rs",
        "tests/algo_novel_echo_misc_test.rs",
        "tests/c_test.rs",
        "tests/cpp_test.rs",
        "tests/rust_lang_test.rs",
        "tests/lua_test.rs",
        "tests/javascript_test.rs",
        "tests/typescript_test.rs",
        "tests/ast_test.rs",
    ];
    let mut test_names_buf: Vec<String> = Vec::new();
    for tf in all_test_files {
        if let Ok(src) = std::fs::read_to_string(tf) {
            for line in src.lines() {
                let t = line.trim();
                if t.starts_with("fn test_") {
                    if let Some(n) = t.trim_start_matches("fn ").split('(').next() {
                        test_names_buf.push(n.to_string());
                    }
                }
            }
        }
    }
    let test_names: Vec<&str> = test_names_buf.iter().map(|s| s.as_str()).collect();

    let mut failures = Vec::new();
    for (algo_keys, algo_name) in algorithms {
        let lang_count = lang_keys
            .iter()
            .filter(|lang| {
                test_names.iter().any(|name| {
                    algo_keys.iter().any(|k| name.contains(k)) && lang_matches(name, lang)
                })
            })
            .count();
        if lang_count < MIN_LANGS {
            failures.push(format!(
                "  {} — tested in {} language(s), need ≥ {}",
                algo_name, lang_count, MIN_LANGS
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "Algorithms below minimum language coverage ({} languages):\n{}",
        MIN_LANGS,
        failures.join("\n")
    );
}

/// Test that MembraneSlice correctly recognises C-style error handling
/// (if (ret < 0), if (!ptr), errno, perror) and does NOT emit a false
/// "unprotected caller" finding when the caller already checks errors.

/// Test that MembraneSlice recognises NULL-pointer checks as error handling.

// ====== Function pointer call edge resolution tests ======

/// Test that the call graph resolves function pointer calls through struct fields.
/// `timer->callback(data)` should produce an edge to `callback` in the call graph.
#[test]
fn test_call_graph_field_expression_call() {
    let source = r#"
#include <stdlib.h>

typedef struct {
    void (*callback)(void *data);
    void *data;
} timer_t;

void timeout_handler(void *data) {
    // handle timeout
}

void fire_timer(timer_t *timer) {
    timer->callback(timer->data);
}

void setup_timer(timer_t *timer) {
    timer->callback = timeout_handler;
    timer->data = NULL;
    fire_timer(timer);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/timer.c".to_string(),
        ParsedFile::parse("src/timer.c", source, Language::C).unwrap(),
    );

    let call_graph = CallGraph::build(&files);

    // fire_timer calls timer->callback — should resolve to callee_name "callback"
    let fire_timer_id = call_graph
        .functions
        .get("fire_timer")
        .expect("fire_timer should be in call graph");

    let fire_timer_calls = call_graph
        .calls
        .get(&fire_timer_id[0])
        .expect("fire_timer should have call sites");

    let callee_names: Vec<&str> = fire_timer_calls
        .iter()
        .map(|s| s.callee_name.as_str())
        .collect();
    assert!(
        callee_names.contains(&"callback"),
        "timer->callback(...) should resolve callee_name to 'callback', got: {:?}",
        callee_names
    );
}

/// Test that MembraneSlice detects cross-file callers through function pointer dispatch.
/// When ops->process() is called in another file and process() is modified,
/// MembraneSlice should find the cross-file caller.

/// Test that CircularSlice detects cycles through function pointer calls.

/// Level 1: local variable function pointer — `fptr = target_func; fptr(42);`
/// should produce a call edge from the caller to target_func.

/// Level 2: array dispatch table — `handlers[idx](data)` should produce
/// edges to all functions in the initializer list.
#[test]
fn test_call_graph_level2_dispatch_table() {
    let source = r#"
#include <stdlib.h>

void handle_get(int req) { }
void handle_set(int req) { }
void handle_delete(int req) { }

typedef void (*handler_fn)(int);

void dispatch(int cmd, int req) {
    handler_fn handlers[] = {handle_get, handle_set, handle_delete};
    handlers[cmd](req);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/dispatch.c".to_string(),
        ParsedFile::parse("src/dispatch.c", source, Language::C).unwrap(),
    );

    let call_graph = CallGraph::build(&files);

    let dispatch_id = &call_graph.functions.get("dispatch").unwrap()[0];
    let dispatch_calls = call_graph.calls.get(dispatch_id).unwrap();
    let callee_names: BTreeSet<&str> = dispatch_calls
        .iter()
        .map(|s| s.callee_name.as_str())
        .collect();

    assert!(
        callee_names.contains("handle_get"),
        "Level 2: dispatch table should resolve to handle_get, got: {:?}",
        callee_names
    );
    assert!(
        callee_names.contains("handle_set"),
        "Level 2: dispatch table should resolve to handle_set, got: {:?}",
        callee_names
    );
    assert!(
        callee_names.contains("handle_delete"),
        "Level 2: dispatch table should resolve to handle_delete, got: {:?}",
        callee_names
    );
}

/// Level 2: global/file-scope dispatch table with designated initializers.
#[test]
fn test_call_graph_level2_global_dispatch_table() {
    let source = r#"
#include <stdlib.h>

int my_open(int fd) { return 0; }
int my_read(int fd) { return 0; }
int my_write(int fd) { return 0; }

typedef int (*file_op)(int);

file_op file_ops[] = {my_open, my_read, my_write};

void do_operation(int op, int fd) {
    file_ops[op](fd);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/fileops.c".to_string(),
        ParsedFile::parse("src/fileops.c", source, Language::C).unwrap(),
    );

    let call_graph = CallGraph::build(&files);

    let do_op_id = &call_graph.functions.get("do_operation").unwrap()[0];
    let do_op_calls = call_graph.calls.get(do_op_id).unwrap();
    let callee_names: BTreeSet<&str> = do_op_calls.iter().map(|s| s.callee_name.as_str()).collect();

    assert!(
        callee_names.contains("my_open"),
        "Level 2: global dispatch table should resolve to my_open, got: {:?}",
        callee_names
    );
    assert!(
        callee_names.contains("my_read"),
        "Level 2: global dispatch table should resolve to my_read, got: {:?}",
        callee_names
    );
    assert!(
        callee_names.contains("my_write"),
        "Level 2: global dispatch table should resolve to my_write, got: {:?}",
        callee_names
    );
}

/// Level 1 end-to-end: MembraneSlice should detect a cross-file caller through
/// a local function pointer variable.

/// A function registered as a signal handler via signal(SIGTERM, my_cleanup)
/// should be detected as async even though it doesn't contain any async
/// primitives itself. The registration happens in a DIFFERENT function.

/// A function registered as a pthread start routine should be detected as async.

/// Cross-file ISR detection: handler registered in one file, defined in another.

/// Two files with same-named `static init()` functions should NOT be conflated
/// in the call graph. A call to `init()` in file A should resolve to file A's
/// static init, not file B's.
#[test]
fn test_call_graph_static_disambiguation() {
    let file_a = r#"
static int init(void) {
    return 0;
}

int setup_a(void) {
    return init();
}
"#;

    let file_b = r#"
static int init(void) {
    return 1;
}

int setup_b(void) {
    return init();
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

    let call_graph = CallGraph::build(&files);

    // Both init functions should be registered as static
    assert!(
        call_graph
            .static_functions
            .contains(&("src/a.c".to_string(), "init".to_string())),
        "init in a.c should be detected as static"
    );
    assert!(
        call_graph
            .static_functions
            .contains(&("src/b.c".to_string(), "init".to_string())),
        "init in b.c should be detected as static"
    );

    // setup_a's call to init() should resolve to a.c's init, not b.c's
    let callees_a = call_graph.resolve_callees("init", "src/a.c");
    assert_eq!(
        callees_a.len(),
        1,
        "init() called from a.c should resolve to exactly 1 function, got: {:?}",
        callees_a
    );
    assert_eq!(
        callees_a[0].file, "src/a.c",
        "init() called from a.c should resolve to a.c's init"
    );

    // setup_b's call to init() should resolve to b.c's init
    let callees_b = call_graph.resolve_callees("init", "src/b.c");
    assert_eq!(
        callees_b.len(),
        1,
        "init() called from b.c should resolve to exactly 1 function, got: {:?}",
        callees_b
    );
    assert_eq!(
        callees_b[0].file, "src/b.c",
        "init() called from b.c should resolve to b.c's init"
    );
}

/// A non-static function should be visible cross-file, while a static one
/// in the caller's file takes priority when both exist.
#[test]
fn test_call_graph_static_vs_non_static() {
    let file_a = r#"
static int helper(int x) {
    return x * 2;
}

int process_a(int x) {
    return helper(x);
}
"#;

    let file_b = r#"
int helper(int x) {
    return x + 1;
}
"#;

    let file_c = r#"
int process_c(int x) {
    return helper(x);
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
    files.insert(
        "src/c.c".to_string(),
        ParsedFile::parse("src/c.c", file_c, Language::C).unwrap(),
    );

    let call_graph = CallGraph::build(&files);

    // a.c has static helper — process_a's call should resolve to a.c's helper only
    let callees_a = call_graph.resolve_callees("helper", "src/a.c");
    assert_eq!(
        callees_a.len(),
        1,
        "helper() from a.c should resolve to a.c's static helper only"
    );
    assert_eq!(callees_a[0].file, "src/a.c");

    // c.c has no local helper — should resolve to b.c's non-static helper
    // (but NOT a.c's static helper)
    let callees_c = call_graph.resolve_callees("helper", "src/c.c");
    assert_eq!(
        callees_c.len(),
        1,
        "helper() from c.c should resolve to b.c's non-static helper only, got: {:?}",
        callees_c
    );
    assert_eq!(
        callees_c[0].file, "src/b.c",
        "helper() from c.c should resolve to b.c (not a.c's static)"
    );
}

/// MembraneSlice should NOT flag cross-file callers for a static function,
/// since static functions are file-local and can't actually be called cross-file.

// === Rust pattern depth tests ===

// === Lua pattern depth tests ===

// ===== C++ MembraneSlice error handling patterns =====

/// Test that MembraneSlice recognises C++ try/catch as error handling.

/// Test that MembraneSlice recognises C++ RAII smart pointers as error handling.

/// Test that MembraneSlice recognises C++ lock_guard as error handling.

/// Test that MembraneSlice recognises C++ std::optional as error handling.

// ===== Function Pointer Level 3: parameter-passed fptrs =====

/// Level 3: basic callback parameter — `execute(cb, data)` where `cb` is a parameter
/// should resolve to the functions passed as arguments by callers.

/// Level 3: callback parameter with address-of operator — `register_handler(&my_handler)`

/// Level 3: cross-file callback — callback function defined in one file, executor in another.
#[test]
fn test_call_graph_level3_cross_file_callback() {
    let executor_source = r#"
typedef void (*event_cb)(int);

void on_event(event_cb callback, int event_id) {
    callback(event_id);
}
"#;

    let caller_source = r#"
void handle_connect(int id) {
    // process connect event
}

void init_events(void) {
    on_event(handle_connect, 1);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/executor.c".to_string(),
        ParsedFile::parse("src/executor.c", executor_source, Language::C).unwrap(),
    );
    files.insert(
        "src/caller.c".to_string(),
        ParsedFile::parse("src/caller.c", caller_source, Language::C).unwrap(),
    );

    let call_graph = CallGraph::build(&files);

    let on_event_id = &call_graph.functions.get("on_event").unwrap()[0];
    let on_event_calls = call_graph.calls.get(on_event_id).unwrap();
    let callee_names: BTreeSet<&str> = on_event_calls
        .iter()
        .map(|s| s.callee_name.as_str())
        .collect();

    assert!(
        callee_names.contains("handle_connect"),
        "Level 3: cross-file on_event(handle_connect, 1) should resolve callback, got: {:?}",
        callee_names
    );
}

/// Level 3: membrane slice should detect unprotected callers through parameter-passed fptrs.

/// Level 3: argument passed as local variable (Level 1 + Level 3 composition).
#[test]
fn test_call_graph_level3_with_local_variable() {
    let source = r#"
void real_handler(int x) {
    // actual implementation
}

typedef void (*handler_fn)(int);

void invoke(handler_fn cb, int val) {
    cb(val);
}

void setup(void) {
    handler_fn h = real_handler;
    invoke(h, 10);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/composed.c".to_string(),
        ParsedFile::parse("src/composed.c", source, Language::C).unwrap(),
    );

    let call_graph = CallGraph::build(&files);

    let invoke_id = &call_graph.functions.get("invoke").unwrap()[0];
    let invoke_calls = call_graph.calls.get(invoke_id).unwrap();
    let callee_names: BTreeSet<&str> = invoke_calls
        .iter()
        .map(|s| s.callee_name.as_str())
        .collect();

    assert!(
        callee_names.contains("real_handler"),
        "Level 3+1: invoke(h, 10) where h = real_handler should resolve cb to real_handler, got: {:?}",
        callee_names
    );
}

// ===== CVE-Pattern Test Fixtures =====

/// CVE pattern: double-free via goto error path.
/// Classic kernel bug where a resource is freed inline before a goto,
/// and the goto target label also frees it.

/// CVE pattern: correct goto cleanup (no double-free).
/// Kernel-style ordered labels with fall-through — should NOT flag as double-free.

/// CVE pattern: kernel double-unlock via goto.
/// spin_lock released inline AND in the error label.

/// CVE pattern: format string injection — user input flows to printf format parameter.

/// CVE pattern: buffer overflow — user-controlled size flows to memcpy.

/// CVE pattern: strcpy buffer overflow from network input (via fgets).
/// Uses fgets which returns to a variable the DFG can trace.

/// CVE pattern: integer overflow before allocation.
/// User-controlled size undergoes arithmetic, then passed to malloc.

/// CVE pattern: use-after-free — pointer used after being freed.
/// Taint detects free() as a sink on the diff line; the subsequent use of
/// timer->data is included in the analysis context. Prism doesn't yet
/// distinguish "free then use" from "use then free" as a distinct UAF pattern.

// va_list taint tracking — variadic wrapper detection

// AccessPath / field-qualified DFG tracking

// Cross-language field access in DFG

// Downstream DFG consumer tests — assignment propagation, chop, provenance

// Batch 1: Zero-coverage algorithms (Chop, DeltaSlice, ThreeDSlice)

/// Helper to create a temp git repo for tests that need git history.
/// Returns a TempDir that auto-cleans on drop — no manual cleanup needed.

// Batch 2: ConditionedSlice, AngleSlice, SpiralSlice

// Batch 3: HorizontalSlice, VerticalSlice, ResonanceSlice

// Batch 4: Remaining algorithm×language gaps
// ====================================================================

#[test]
fn test_full_flow_python_no_returns() {
    let (files, _, diff) = make_python_test();
    let config = SliceConfig {
        algorithm: SlicingAlgorithm::FullFlow,
        include_returns: false,
        trace_callees: false,
        ..SliceConfig::default()
    };
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_full_flow_go_trace_callees() {
    let (files, _, diff) = make_go_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_original_diff_go() {
    let (files, _, diff) = make_go_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_left_flow_go() {
    let (files, _, diff) = make_go_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_left_flow_java() {
    let (files, _, diff) = make_java_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

// Batch 6: Targeted tests for files near 80% threshold
// ====================================================================

#[test]
fn test_parent_function_global_scope_python() {
    // Test parent function with diff lines outside any function
    let source = r#"
import os

CONFIG = os.getenv("CONFIG", "default")
DATA_DIR = "/tmp/data"

def process():
    return CONFIG
"#;
    let path = "config.py";
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

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "ParentFunction should include global diff lines"
    );
}

#[test]
fn test_parent_function_global_scope_go() {
    let source = r#"package main

var Config = "default"
var Port = 8080

func main() {
	println(Config)
}
"#;
    let path = "main.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_full_flow_trace_callees_python() {
    // Test with trace_callees enabled to cover cross-file R-value resolution
    let source_main = r#"
def process(data):
    result = transform(data)
    return result
"#;
    let source_transform = r#"
def transform(x):
    return x * 2
"#;
    let mut files = BTreeMap::new();
    files.insert(
        "main.py".to_string(),
        ParsedFile::parse("main.py", source_main, Language::Python).unwrap(),
    );
    files.insert(
        "transform.py".to_string(),
        ParsedFile::parse("transform.py", source_transform, Language::Python).unwrap(),
    );

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "main.py".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let config = SliceConfig {
        algorithm: SlicingAlgorithm::FullFlow,
        include_returns: true,
        trace_callees: true,
        ..SliceConfig::default()
    };
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    assert!(!result.blocks.is_empty());
}

// Batch 7: Deeper assertions on security-relevant algorithms

// ---------------------------------------------------------------------------
// Phase 2: Field isolation tests — taint on obj.fieldA must NOT reach obj.fieldB
// ---------------------------------------------------------------------------

#[test]
fn test_must_alias_python() {
    // ref = self; ref.secret = x → should create def for self.secret too
    let source = r#"
class Handler:
    def process(self):
        ref = self
        ref.secret = get_password()
        send(self.secret)
"#;
    let path = "src/alias.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let self_defs = dfg.all_defs_of(path, "self");

    let has_self_secret = self_defs.iter().any(|d| d.path.fields == vec!["secret"]);
    assert!(
        has_self_secret,
        "Phase 3 Python: ref = self alias should resolve ref.secret to self.secret"
    );
}

#[test]
fn test_must_alias_javascript() {
    let source = r#"
function process(config) {
    const ref = config;
    ref.timeout = getUserInput();
    use(config.timeout);
}
"#;
    let path = "src/alias.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let config_defs = dfg.all_defs_of(path, "config");

    let has_config_timeout = config_defs.iter().any(|d| d.path.fields == vec!["timeout"]);
    assert!(
        has_config_timeout,
        "Phase 3 JS: ref = config alias should resolve ref.timeout to config.timeout"
    );
}

#[test]
fn test_must_alias_go() {
    let source = r#"
package main

func process(dev Device) {
    ref := dev
    ref.Name = getInput()
    useName(dev.Name)
}
"#;
    let path = "src/alias.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let dev_defs = dfg.all_defs_of(path, "dev");

    let has_dev_name = dev_defs.iter().any(|d| d.path.fields == vec!["Name"]);
    assert!(
        has_dev_name,
        "Phase 3 Go: ref := dev alias should resolve ref.Name to dev.Name"
    );
}

#[test]
fn test_must_alias_rust() {
    let source = r#"
fn process(dev: &mut Device) {
    let ptr = dev;
    ptr.name = get_input();
    use_name(dev.name);
}
"#;
    let path = "src/alias.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let dev_defs = dfg.all_defs_of(path, "dev");

    let has_dev_name = dev_defs.iter().any(|d| d.path.fields == vec!["name"]);
    assert!(
        has_dev_name,
        "Phase 3 Rust: ptr = dev alias should resolve ptr.name to dev.name"
    );
}

#[test]
fn test_must_alias_chain() {
    // Chain: a = dev; b = a; b->field → should resolve to dev.field
    let source = r#"
void chain(struct device *dev) {
    struct device *a = dev;
    struct device *b = a;
    b->name = "test";
    use_name(dev->name);
}
"#;
    let path = "src/chain.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let dev_defs = dfg.all_defs_of(path, "dev");

    let has_dev_name = dev_defs.iter().any(|d| d.path.fields == vec!["name"]);
    assert!(
        has_dev_name,
        "Phase 3: chained aliases (b = a = dev) should resolve b->name to dev.name"
    );
}

#[test]
fn test_must_alias_no_false_positive() {
    // x = unrelated_var should NOT alias to dev
    let source = r#"
void no_alias(struct device *dev, struct device *other) {
    struct device *ptr = other;
    ptr->name = "test";
    use_name(dev->name);
}
"#;
    let path = "src/no_alias.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let dev_defs = dfg.all_defs_of(path, "dev");

    // dev should NOT have a name def from ptr->name (ptr aliases other, not dev)
    let has_dev_name = dev_defs.iter().any(|d| d.path.fields == vec!["name"]);
    assert!(
        !has_dev_name,
        "Phase 3: ptr = other should NOT create alias to dev. Got defs: {:?}",
        dev_defs.iter().map(|d| &d.path).collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Type Database tests
// ---------------------------------------------------------------------------

#[test]
fn test_cpg_with_type_enrichment() {
    use prism::cpg::CodePropertyGraph;
    use prism::type_db::{FieldInfo, RecordInfo, RecordKind, TypeDatabase, TypedefInfo};

    // Build a TypeDatabase manually
    let mut type_db = TypeDatabase::default();
    type_db.records.insert(
        "device".to_string(),
        RecordInfo {
            name: "device".to_string(),
            kind: RecordKind::Struct,
            fields: vec![
                FieldInfo {
                    name: "name".to_string(),
                    type_str: "char *".to_string(),
                    offset: None,
                },
                FieldInfo {
                    name: "id".to_string(),
                    type_str: "int".to_string(),
                    offset: None,
                },
            ],
            bases: vec![],
            virtual_methods: std::collections::BTreeMap::new(),
            size: None,
            file: "device.h".to_string(),
        },
    );
    type_db.typedefs.insert(
        "dev_t".to_string(),
        TypedefInfo {
            name: "dev_t".to_string(),
            underlying: "struct device *".to_string(),
        },
    );

    // Build a CPG with type enrichment
    let source = r#"
struct device {
    char *name;
    int id;
};

void init(struct device *dev) {
    dev->name = "test";
    dev->id = 42;
}
"#;
    let path = "test.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build_with_types(&files, type_db);

    // CPG should have type info
    assert!(cpg.has_type_info());
    assert_eq!(
        cpg.all_fields_of("device"),
        Some(vec!["name".to_string(), "id".to_string()])
    );
    assert_eq!(cpg.resolve_type("dev_t"), "struct device *");
    assert_eq!(cpg.field_type("device", "name"), Some("char *".to_string()));
}

// ── CpgContext::without_cpg / needs_cpg tests ───────────────────────

#[test]
fn test_needs_cpg_classification() {
    // Verify that AST-only algorithms don't need CPG
    let ast_only = vec![
        SlicingAlgorithm::OriginalDiff,
        SlicingAlgorithm::ParentFunction,
        SlicingAlgorithm::LeftFlow,
        SlicingAlgorithm::FullFlow,
        SlicingAlgorithm::ThinSlice,
        SlicingAlgorithm::RelevantSlice,
        SlicingAlgorithm::ConditionedSlice,
        SlicingAlgorithm::QuantumSlice,
        SlicingAlgorithm::HorizontalSlice,
        SlicingAlgorithm::AngleSlice,
        SlicingAlgorithm::AbsenceSlice,
        SlicingAlgorithm::ResonanceSlice,
        SlicingAlgorithm::SymmetrySlice,
        SlicingAlgorithm::PhantomSlice,
    ];
    for algo in &ast_only {
        assert!(!algo.needs_cpg(), "{:?} should not need CPG", algo);
    }

    // Verify that CPG-consuming algorithms do need CPG
    let cpg_algos = vec![
        SlicingAlgorithm::BarrierSlice,
        SlicingAlgorithm::Chop,
        SlicingAlgorithm::Taint,
        SlicingAlgorithm::DeltaSlice,
        SlicingAlgorithm::SpiralSlice,
        SlicingAlgorithm::CircularSlice,
        SlicingAlgorithm::VerticalSlice,
        SlicingAlgorithm::ThreeDSlice,
        SlicingAlgorithm::GradientSlice,
        SlicingAlgorithm::ProvenanceSlice,
        SlicingAlgorithm::MembraneSlice,
        SlicingAlgorithm::EchoSlice,
    ];
    for algo in &cpg_algos {
        assert!(algo.needs_cpg(), "{:?} should need CPG", algo);
    }
}






// ── Cross-language coverage matrix validation ──────────────────────────

/// Simple glob matching: splits pattern on `*` and checks all fragments
/// appear in order in the text. e.g. "test_*fptr*" matches "test_c_fptr_level1".
fn glob_match(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    let mut pos = 0;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        match text[pos..].find(part) {
            Some(idx) => {
                if i == 0 && !pattern.starts_with('*') && idx != 0 {
                    return false;
                }
                pos += idx + part.len();
            }
            None => return false,
        }
    }
    if !pattern.ends_with('*') {
        pos == text.len()
    } else {
        true
    }
}

#[test]
fn test_coverage_matrix_validation() {
    use std::fs;

    let matrix_str =
        fs::read_to_string("coverage/matrix.json").expect("coverage/matrix.json should exist");
    let matrix: serde_json::Value =
        serde_json::from_str(&matrix_str).expect("matrix.json should be valid JSON");

    // Read all test function names from test files AND unit test modules
    let test_files = &[
        "tests/integration_test.rs",
        "tests/algo_paper_test.rs",
        "tests/algo_taxonomy_test.rs",
        "tests/algo_theoretical_test.rs",
        "tests/algo_novel_absence_test.rs",
        "tests/algo_novel_provenance_test.rs",
        "tests/algo_novel_membrane_test.rs",
        "tests/algo_novel_echo_misc_test.rs",
        "tests/c_test.rs",
        "tests/cpp_test.rs",
        "tests/rust_lang_test.rs",
        "tests/lua_test.rs",
        "tests/javascript_test.rs",
        "tests/ast_test.rs",
        "tests/cli_test.rs",
        "src/cfg.rs",
        "src/cpg.rs",
        "src/type_db.rs",
        "src/data_flow.rs",
        "src/ast.rs",
        "src/call_graph.rs",
        "src/access_path.rs",
    ];
    let mut test_names: Vec<String> = Vec::new();
    for path in test_files {
        if let Ok(source) = fs::read_to_string(path) {
            for line in source.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("fn test_") {
                    if let Some(name) = trimmed.trim_start_matches("fn ").split('(').next() {
                        if !name.is_empty() {
                            test_names.push(name.to_string());
                        }
                    }
                }
            }
        }
    }

    assert!(
        test_names.len() > 300,
        "Should find >300 test names, found {}",
        test_names.len()
    );

    let mut handled_count = 0;
    let mut verified_count = 0;
    let mut warnings: Vec<String> = Vec::new();

    if let Some(features) = matrix["language_features"].as_object() {
        for (category, cat_features) in features {
            if let Some(cat_obj) = cat_features.as_object() {
                for (feature_name, spec) in cat_obj {
                    let status = spec["status"].as_str().unwrap_or("unknown");
                    if status != "handled" {
                        continue;
                    }
                    handled_count += 1;

                    if let Some(test_patterns) = spec["tests"].as_array() {
                        let has_match = test_patterns.iter().any(|pattern| {
                            let pat = pattern.as_str().unwrap_or("");
                            test_names.iter().any(|t| glob_match(pat, t))
                        });

                        if has_match {
                            verified_count += 1;
                        } else {
                            warnings.push(format!(
                                "{}/{}: claims handled with tests {:?} but no matching test found",
                                category,
                                feature_name,
                                test_patterns
                                    .iter()
                                    .map(|p| p.as_str().unwrap_or(""))
                                    .collect::<Vec<_>>()
                            ));
                        }
                    } else {
                        // No test patterns specified — covered by general algorithm tests
                        verified_count += 1;
                    }
                }
            }
        }
    }

    if !warnings.is_empty() {
        eprintln!("\nCoverage matrix warnings:");
        for w in &warnings {
            eprintln!("  WARN: {}", w);
        }
    }

    let mut gap_count = 0;
    if let Some(features) = matrix["language_features"].as_object() {
        for (_category, cat_features) in features {
            if let Some(cat_obj) = cat_features.as_object() {
                for (_feature_name, spec) in cat_obj {
                    if spec["status"].as_str() == Some("gap") {
                        gap_count += 1;
                    }
                }
            }
        }
    }

    eprintln!(
        "\nCoverage matrix: {}/{} handled features verified, {} gaps remaining",
        verified_count, handled_count, gap_count
    );

    // Coverage threshold: per-feature (not per-language-feature)
    let total = handled_count + gap_count;
    let coverage_pct = if total > 0 {
        100 * handled_count / total
    } else {
        0
    };
    assert!(
        coverage_pct >= 80,
        "Language feature coverage dropped below 80%: {}% ({}/{})",
        coverage_pct,
        handled_count,
        total
    );

    assert!(
        warnings.is_empty(),
        "Coverage matrix has {} unverified claims:\n  {}",
        warnings.len(),
        warnings.join("\n  ")
    );
}

#[test]
fn test_coverage_matrix_algorithm_completeness() {
    use std::fs;

    let matrix_str =
        fs::read_to_string("coverage/matrix.json").expect("coverage/matrix.json should exist");
    let matrix: serde_json::Value =
        serde_json::from_str(&matrix_str).expect("matrix.json should be valid JSON");

    let algo_cov = matrix["algorithm_coverage"]
        .as_object()
        .expect("algorithm_coverage should be an object");

    let expected_algos = vec![
        "original_diff",
        "parent_function",
        "left_flow",
        "full_flow",
        "thin_slice",
        "barrier_slice",
        "taint",
        "chop",
        "relevant_slice",
        "conditioned_slice",
        "delta_slice",
        "spiral_slice",
        "circular_slice",
        "quantum_slice",
        "horizontal_slice",
        "vertical_slice",
        "angle_slice",
        "threed_slice",
        "absence_slice",
        "resonance_slice",
        "symmetry_slice",
        "gradient_slice",
        "provenance_slice",
        "phantom_slice",
        "membrane_slice",
        "echo_slice",
    ];

    let mut missing = Vec::new();
    for algo in &expected_algos {
        if !algo_cov.contains_key(*algo) {
            missing.push(*algo);
        }
    }
    assert!(
        missing.is_empty(),
        "Algorithms missing from coverage matrix: {:?}",
        missing
    );

    let languages = [
        "python",
        "javascript",
        "typescript",
        "go",
        "java",
        "c",
        "cpp",
        "rust",
        "lua",
    ];
    for (algo, langs) in algo_cov {
        let covered = languages
            .iter()
            .filter(|l| langs[**l].as_str().unwrap_or("none") != "none")
            .count();
        assert!(
            covered >= 2,
            "Algorithm '{}' has coverage in only {} languages (need ≥2)",
            algo,
            covered
        );
    }
}
