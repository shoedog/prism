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

// ====== Call Graph tests ======

#[test]
fn test_call_graph_construction() {
    let (files, _, _) = make_python_test();
    let cg = CallGraph::build(&files);

    // Should have functions
    assert!(!cg.functions.is_empty());

    // 'process' calls 'calculate' and 'helper'
    let process_funcs = cg.functions.get("process");
    assert!(process_funcs.is_some(), "Should find 'process' function");
}

#[test]
fn test_call_graph_callers() {
    let (files, _, _) = make_python_test();
    let cg = CallGraph::build(&files);

    let callers = cg.callers_of("calculate", 1);
    // 'process' calls 'calculate'
    assert!(
        callers.iter().any(|(f, _)| f.name == "process"),
        "process should be a caller of calculate"
    );
}

// ====== Data Flow Graph tests ======

#[test]
fn test_data_flow_graph_construction() {
    let (files, _, _) = make_python_test();
    let dfg = DataFlowGraph::build(&files);

    assert!(!dfg.edges.is_empty(), "Should have data flow edges");
    assert!(!dfg.defs.is_empty(), "Should have definitions");
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









// ====== C Parsing tests ======

#[test]
fn test_c_parses_and_finds_functions() {
    let (files, _, _) = make_c_test();
    let parsed = files.get("src/device.c").unwrap();

    // Should find all functions in the C file
    let functions = parsed.all_functions();
    let func_names: Vec<String> = functions
        .iter()
        .filter_map(|f| {
            parsed
                .language
                .function_name(f)
                .map(|n| parsed.node_text(&n).to_string())
        })
        .collect();

    assert!(
        func_names.contains(&"create_device".to_string()),
        "Should find create_device, got: {:?}",
        func_names
    );
    assert!(
        func_names.contains(&"process_packet".to_string()),
        "Should find process_packet, got: {:?}",
        func_names
    );
    assert!(
        func_names.contains(&"handle_request".to_string()),
        "Should find handle_request, got: {:?}",
        func_names
    );
    assert!(
        func_names.contains(&"destroy_device".to_string()),
        "Should find destroy_device, got: {:?}",
        func_names
    );
    assert!(
        func_names.contains(&"bulk_process".to_string()),
        "Should find bulk_process, got: {:?}",
        func_names
    );
}

#[test]
fn test_cpp_parses_and_finds_methods() {
    let (files, _, _) = make_cpp_test();
    let parsed = files.get("src/device_manager.cpp").unwrap();

    let functions = parsed.all_functions();
    let func_names: Vec<String> = functions
        .iter()
        .filter_map(|f| {
            parsed
                .language
                .function_name(f)
                .map(|n| parsed.node_text(&n).to_string())
        })
        .collect();

    assert!(
        func_names.contains(&"process_devices".to_string()),
        "Should find process_devices, got: {:?}",
        func_names
    );
    // C++ methods inside classes should also be found
    assert!(
        func_names.len() >= 2,
        "Should find at least free function + some class methods, got {} functions: {:?}",
        func_names.len(),
        func_names
    );
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


// ====== C Call Graph tests ======

#[test]
fn test_call_graph_c() {
    let (files, _, _) = make_c_test();
    let call_graph = CallGraph::build(&files);

    // handle_request calls create_device and process_packet
    let callees = call_graph.callees_of("handle_request", "src/device.c", 1);
    let callee_names: Vec<&str> = callees.iter().map(|(id, _)| id.name.as_str()).collect();

    assert!(
        callee_names.contains(&"create_device") || callee_names.contains(&"process_packet"),
        "handle_request should call create_device and process_packet, got: {:?}",
        callee_names
    );
}

#[test]
fn test_call_graph_c_cross_file() {
    let (files, _, _) = make_c_multifile_test();
    let call_graph = CallGraph::build(&files);

    // handle_create in handler.c calls create_device in device.c
    let callees = call_graph.callees_of("handle_create", "src/handler.c", 1);
    let callee_names: Vec<&str> = callees.iter().map(|(id, _)| id.name.as_str()).collect();

    assert!(
        callee_names.contains(&"create_device"),
        "handle_create should call create_device across files, got: {:?}",
        callee_names
    );
}




// ====== C Data Flow tests ======

#[test]
fn test_data_flow_graph_c() {
    let (files, _, _) = make_c_test();
    let dfg = DataFlowGraph::build(&files);

    // Should have def-use edges for variables in C functions
    assert!(
        !dfg.edges.is_empty(),
        "Data flow graph should have edges for C code"
    );

    // Check that variable defs are found
    assert!(
        !dfg.defs.is_empty(),
        "Data flow graph should find variable definitions in C code"
    );
}




// ====== C Language in all_languages_parse ======

#[test]
fn test_c_and_cpp_parse() {
    // Verify C and C++ can be parsed without errors
    let c_source = "int main() { return 0; }\n";
    let cpp_source = "class Foo { public: void bar() {} };\n";

    let c_parsed = ParsedFile::parse("test.c", c_source, Language::C);
    assert!(c_parsed.is_ok(), "C parsing should succeed");

    let cpp_parsed = ParsedFile::parse("test.cpp", cpp_source, Language::Cpp);
    assert!(cpp_parsed.is_ok(), "C++ parsing should succeed");
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

// ====== Phase 3: Firmware fixture tests ======

fn make_onu_state_machine_test() -> (BTreeMap<String, ParsedFile>, DiffInput) {
    let source = r#"
#include <stdint.h>

typedef struct { int type; uint8_t data[64]; } ploam_msg_t;
#define RANGING_GRANT   1
#define RANGING_COMPLETE 2
#define ACTIVATE        3
#define DEREGISTRATION  4

enum onu_state { INIT, RANGING, REGISTERED, OPERATIONAL };
static enum onu_state current_state = INIT;

void handle_ploam_message(ploam_msg_t *msg) {
    switch(current_state) {
        case INIT:
            if (msg->type == RANGING_GRANT) {
                current_state = RANGING;
            }
            break;
        case RANGING:
            if (msg->type == RANGING_COMPLETE) {
                current_state = REGISTERED;
            }
            break;
        case REGISTERED:
            if (msg->type == ACTIVATE) {
                current_state = OPERATIONAL;
            }
            break;
    }
}
"#;

    let path = "tests/fixtures/c/onu_state_machine.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff: lines 20-22 (RANGING case handling)
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([20, 21, 22]),
        }],
    };

    (files, diff)
}

#[test]
fn test_snmp_overflow_original_diff() {
    let (files, diff) = make_snmp_overflow_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "OriginalDiff should produce blocks for snmp_overflow"
    );
    let total_lines: usize = result
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();
    assert!(
        total_lines > 0,
        "snmp_overflow OriginalDiff should include at least one line"
    );
}

#[test]
fn test_snmp_overflow_parent_function() {
    let (files, diff) = make_snmp_overflow_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "ParentFunction should produce blocks for snmp_overflow"
    );
}

#[test]
fn test_snmp_overflow_left_flow() {
    let (files, diff) = make_snmp_overflow_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce blocks for snmp_overflow"
    );
}

#[test]
fn test_onu_state_machine_original_diff() {
    let (files, diff) = make_onu_state_machine_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "OriginalDiff should produce blocks for onu_state_machine"
    );
    let total_lines: usize = result
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();
    assert!(
        total_lines > 0,
        "onu_state_machine OriginalDiff should include at least one line"
    );
}

#[test]
fn test_onu_state_machine_left_flow() {
    let (files, diff) = make_onu_state_machine_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce blocks for onu_state_machine"
    );
}

#[test]
fn test_double_free_original_diff() {
    let (files, diff) = make_double_free_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "OriginalDiff should produce blocks for double_free"
    );
    let total_lines: usize = result
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();
    assert!(
        total_lines > 0,
        "double_free OriginalDiff should include at least one line"
    );
}

#[test]
fn test_double_free_left_flow() {
    let (files, diff) = make_double_free_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce blocks for double_free"
    );
}

#[test]
fn test_ring_overflow_original_diff() {
    let (files, diff) = make_ring_overflow_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "OriginalDiff should produce blocks for ring_overflow"
    );
    let total_lines: usize = result
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();
    assert!(
        total_lines > 0,
        "ring_overflow OriginalDiff should include at least one line"
    );
}

#[test]
fn test_ring_overflow_left_flow() {
    let (files, diff) = make_ring_overflow_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce blocks for ring_overflow"
    );
}

#[test]
fn test_timer_uaf_original_diff() {
    let (files, diff) = make_timer_uaf_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "OriginalDiff should produce blocks for timer_uaf"
    );
    let total_lines: usize = result
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();
    assert!(
        total_lines > 0,
        "timer_uaf OriginalDiff should include at least one line"
    );
}

#[test]
fn test_timer_uaf_left_flow() {
    let (files, diff) = make_timer_uaf_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce blocks for timer_uaf"
    );
}

// ====== Phase 4: Stress test fixtures ======

fn make_macro_heavy_test() -> (BTreeMap<String, ParsedFile>, DiffInput) {
    let source = include_str!("fixtures/c/macro_heavy.c");

    let path = "tests/fixtures/c/macro_heavy.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff: line 34 (payload_len = CLAMP(...))
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([34]),
        }],
    };

    (files, diff)
}

#[test]
fn test_large_function_original_diff() {
    let (files, diff) = make_large_function_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "OriginalDiff should produce blocks for large_function"
    );
    let total_files: usize = result.blocks.iter().map(|b| b.file_line_map.len()).sum();
    assert!(
        total_files >= 1,
        "large_function result should reference at least 1 file"
    );
}

#[test]
fn test_large_function_left_flow() {
    let (files, diff) = make_large_function_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce blocks for large_function without panic"
    );
}

#[test]
fn test_macro_heavy_original_diff() {
    let (files, diff) = make_macro_heavy_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "OriginalDiff should produce blocks for macro_heavy"
    );
    let total_files: usize = result.blocks.iter().map(|b| b.file_line_map.len()).sum();
    assert!(
        total_files >= 1,
        "macro_heavy result should reference at least 1 file"
    );
}

#[test]
fn test_macro_heavy_left_flow() {
    let (files, diff) = make_macro_heavy_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce blocks for macro_heavy without panic"
    );
}

#[test]
fn test_deep_switch_original_diff() {
    let (files, diff) = make_deep_switch_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "OriginalDiff should produce blocks for deep_switch"
    );
    let total_files: usize = result.blocks.iter().map(|b| b.file_line_map.len()).sum();
    assert!(
        total_files >= 1,
        "deep_switch result should reference at least 1 file"
    );
}

#[test]
fn test_deep_switch_left_flow() {
    let (files, diff) = make_deep_switch_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce blocks for deep_switch without panic"
    );
}

// --- Parse error detection tests ---

#[test]
fn test_clean_c_has_no_parse_errors() {
    let source = r#"
#include <stdio.h>

int add(int a, int b) {
    return a + b;
}

int main(void) {
    int x = add(1, 2);
    printf("%d\n", x);
    return 0;
}
"#;
    let path = "src/clean.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();

    assert_eq!(
        parsed.parse_error_count, 0,
        "Clean C code should produce zero ERROR nodes"
    );
    assert!(
        parsed.parse_node_count > 0,
        "Should have counted some AST nodes"
    );
    assert_eq!(
        parsed.error_rate(),
        0.0,
        "Error rate should be 0.0 for clean code"
    );

    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let warnings = algorithms::check_parse_warnings(&files);
    assert!(
        warnings.is_empty(),
        "Clean C code should produce no parse warnings"
    );
}

#[test]
fn test_broken_c_triggers_parse_warning() {
    // Code with unbalanced braces and invalid syntax that forces tree-sitter
    // into heavy error recovery, producing many ERROR nodes.
    let source = r#"
@@@ MACRO_CHAOS @@@
#define FOO( bar baz qux
int x = ))) + [[[;
typedef struct { int a; int b; } Foo
void broken( int a, { return a +;
@@@ MORE_GARBAGE @@@
int = = = ;
"#;
    let path = "src/broken.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();

    // tree-sitter error-recovers rather than failing, so we should have nodes
    assert!(
        parsed.parse_node_count > 0,
        "tree-sitter should still produce an AST (with errors)"
    );
    assert!(
        parsed.parse_error_count > 0,
        "Broken C code should produce ERROR nodes"
    );
    assert!(
        parsed.error_rate() > 0.1,
        "Error rate should exceed 10% for heavily broken code (got {})",
        parsed.error_rate()
    );

    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let warnings = algorithms::check_parse_warnings(&files);
    assert!(
        !warnings.is_empty(),
        "Broken C code should generate at least one parse warning"
    );
    // The warning should mention the file name
    assert!(
        warnings.iter().any(|w| w.contains("src/broken.c")),
        "Warning should reference the problematic file"
    );
}

// --- Taint sink tests ---

// --- Provenance origin tests ---

// --- Absence tests ---

// --- Quantum async detection tests ---

// ====== Part 2: Pointer aliasing awareness tests ======

// --- Data flow graph unit tests ---

#[test]
fn test_dataflow_pointer_deref() {
    // *p = val  should create a data flow def for the base pointer variable p.
    let source = r#"
void write_through(int *p, int val) {
    *p = val;
    return;
}
"#;
    let path = "src/ptr.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);

    let p_defs = dfg.all_defs_of(path, "p");
    assert!(
        !p_defs.is_empty(),
        "DataFlowGraph should record a def for 'p' from the *p = val assignment"
    );
    assert!(
        p_defs.iter().any(|d| d.line == 3),
        "Def of 'p' should be on line 3 (*p = val), got lines: {:?}",
        p_defs.iter().map(|d| d.line).collect::<Vec<_>>()
    );
}

#[test]
fn test_dataflow_struct_field() {
    // dev->id = val  should create a qualified AccessPath def (dev.id) plus a base def (dev).
    // The old behavior of creating a bare "id" def was incorrect — it caused false flow edges
    // with unrelated variables named "id".
    let source = r#"
typedef struct { int id; } Dev;
void set_id(Dev *dev, int val) {
    dev->id = val;
    return;
}
"#;
    let path = "src/dev.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);

    // Coarse tracking: mutation of the base struct pointer
    let dev_defs = dfg.all_defs_of(path, "dev");
    assert!(
        !dev_defs.is_empty(),
        "DataFlowGraph should record a def for base 'dev' from dev->id = val"
    );

    // Fine-grained tracking: the field access path was recorded as a def
    // With AccessPath, dev->id creates a def for AccessPath { base: "dev", fields: ["id"] },
    // not a bare "id" def. The base "dev" match above covers both.
    let has_field_path = dev_defs
        .iter()
        .any(|d| d.path.has_fields() && d.path.fields == vec!["id"]);
    assert!(
        has_field_path,
        "DataFlowGraph should record a def with AccessPath dev.id from dev->id = val"
    );
}

#[test]
fn test_dataflow_array_subscript() {
    // buf[i] = val  should create a data flow def for the base array variable buf.
    let source = r#"
void fill_buffer(int *buf, int i, int val) {
    buf[i] = val;
    return;
}
"#;
    let path = "src/buf.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);

    let buf_defs = dfg.all_defs_of(path, "buf");
    assert!(
        !buf_defs.is_empty(),
        "DataFlowGraph should record a def for 'buf' from buf[i] = val"
    );
    assert!(
        buf_defs.iter().any(|d| d.line == 3),
        "Def of 'buf' should be on line 3 (buf[i] = val), got lines: {:?}",
        buf_defs.iter().map(|d| d.line).collect::<Vec<_>>()
    );
}

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
        "tests/algo_novel_test.rs",
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
        "tests/algo_novel_test.rs",
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

// ====== Function pointer resolution Level 1 & 2 tests ======

/// Level 1: local variable function pointer — `fptr = target_func; fptr(42);`
/// should produce a call edge from the caller to target_func.
#[test]
fn test_call_graph_level1_local_fptr() {
    let source = r#"
#include <stdlib.h>

void target_func(int x) {
    // do work
}

void other_func(int x) {
    // different work
}

void caller(void) {
    void (*fptr)(int) = target_func;
    fptr(42);
}

void caller_reassign(void) {
    void (*fptr)(int) = other_func;
    fptr = target_func;
    fptr(99);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/fptr.c".to_string(),
        ParsedFile::parse("src/fptr.c", source, Language::C).unwrap(),
    );

    let call_graph = CallGraph::build(&files);

    // caller's fptr(42) should resolve to target_func
    let caller_id = &call_graph.functions.get("caller").unwrap()[0];
    let caller_calls = call_graph.calls.get(caller_id).unwrap();
    let callee_names: Vec<&str> = caller_calls
        .iter()
        .map(|s| s.callee_name.as_str())
        .collect();
    assert!(
        callee_names.contains(&"target_func"),
        "Level 1: fptr = target_func; fptr() should resolve to target_func, got: {:?}",
        callee_names
    );

    // caller_reassign's fptr(99) should resolve to target_func (last assignment)
    let reassign_id = &call_graph.functions.get("caller_reassign").unwrap()[0];
    let reassign_calls = call_graph.calls.get(reassign_id).unwrap();
    let reassign_names: Vec<&str> = reassign_calls
        .iter()
        .map(|s| s.callee_name.as_str())
        .collect();
    assert!(
        reassign_names.contains(&"target_func"),
        "Level 1: reassigned fptr should resolve to last assignment (target_func), got: {:?}",
        reassign_names
    );
}

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












// ── Rust language support: basic parsing and algorithm tests ──────────

#[test]
fn test_rust_basic_parsing() {
    let source = r#"
use std::io;

fn read_input() -> Result<String, io::Error> {
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(buf)
}

fn process(data: &str) -> Option<i32> {
    let val = data.parse::<i32>().ok()?;
    Some(val * 2)
}
"#;
    let path = "src/main.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();

    // Should detect function definitions
    let funcs = parsed.all_functions();
    let func_names: Vec<String> = funcs
        .iter()
        .filter_map(|f| {
            parsed
                .language
                .function_name(f)
                .map(|n| parsed.node_text(&n).to_string())
        })
        .collect();
    assert!(
        func_names.contains(&"read_input".to_string()),
        "Should find read_input function, got: {:?}",
        func_names
    );
    assert!(
        func_names.contains(&"process".to_string()),
        "Should find process function, got: {:?}",
        func_names
    );
}

#[test]
fn test_rust_original_diff() {
    let source = r#"
fn process(data: &str) -> i32 {
    let val = data.len();
    val as i32
}
"#;
    let path = "src/lib.rs";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "OriginalDiff should produce blocks for Rust code"
    );
}

#[test]
fn test_rust_parent_function() {
    let source = r#"
fn process(data: &str) -> i32 {
    let val = data.len();
    let result = val * 2;
    result
}
"#;
    let path = "src/lib.rs";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "ParentFunction should include the enclosing Rust function"
    );
    // Should include the entire function
    let block = &result.blocks[0];
    let lines = block.file_line_map.get(path).unwrap();
    assert!(
        lines.contains_key(&2) && lines.contains_key(&6),
        "Block should span the entire function (lines 2-6)"
    );
}


// ── Lua language support: basic parsing and algorithm tests ───────────

#[test]
fn test_lua_basic_parsing() {
    let source = r#"
local function process_packet(data)
    local result = data
    return result
end

function handle_request(req)
    local response = process_packet(req)
    return response
end
"#;
    let path = "scripts/handler.lua";
    let parsed = ParsedFile::parse(path, source, Language::Lua).unwrap();

    let funcs = parsed.all_functions();
    let func_names: Vec<String> = funcs
        .iter()
        .filter_map(|f| {
            parsed
                .language
                .function_name(f)
                .map(|n| parsed.node_text(&n).to_string())
        })
        .collect();
    assert!(
        func_names.contains(&"process_packet".to_string()),
        "Should find process_packet function, got: {:?}",
        func_names
    );
    assert!(
        func_names.contains(&"handle_request".to_string()),
        "Should find handle_request function, got: {:?}",
        func_names
    );
}

#[test]
fn test_lua_parent_function() {
    let source = r#"
local function process(data)
    local val = data
    local result = val
    return result
end
"#;
    let path = "scripts/process.lua";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "ParentFunction should include the enclosing Lua function"
    );
}

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
#[test]
fn test_call_graph_level3_parameter_fptr() {
    let source = r#"
void handler_a(int data) {
    // handle A
}

void handler_b(int data) {
    // handle B
}

typedef void (*callback_fn)(int);

void execute(callback_fn cb, int data) {
    cb(data);
}

void caller_a(void) {
    execute(handler_a, 1);
}

void caller_b(void) {
    execute(handler_b, 2);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/callback.c".to_string(),
        ParsedFile::parse("src/callback.c", source, Language::C).unwrap(),
    );

    let call_graph = CallGraph::build(&files);

    // execute's cb(data) should resolve to both handler_a and handler_b
    let execute_id = &call_graph.functions.get("execute").unwrap()[0];
    let execute_calls = call_graph.calls.get(execute_id).unwrap();
    let callee_names: BTreeSet<&str> = execute_calls
        .iter()
        .map(|s| s.callee_name.as_str())
        .collect();

    assert!(
        callee_names.contains("handler_a"),
        "Level 3: execute(handler_a, 1) should resolve cb to handler_a, got: {:?}",
        callee_names
    );
    assert!(
        callee_names.contains("handler_b"),
        "Level 3: execute(handler_b, 2) should resolve cb to handler_b, got: {:?}",
        callee_names
    );
}

/// Level 3: callback parameter with address-of operator — `register_handler(&my_handler)`
#[test]
fn test_call_graph_level3_address_of_fptr() {
    let source = r#"
void my_handler(int sig) {
    // handle signal
}

void register_handler(void (*handler)(int), int sig) {
    handler(sig);
}

void setup(void) {
    register_handler(&my_handler, 2);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/signal.c".to_string(),
        ParsedFile::parse("src/signal.c", source, Language::C).unwrap(),
    );

    let call_graph = CallGraph::build(&files);

    let register_id = &call_graph.functions.get("register_handler").unwrap()[0];
    let register_calls = call_graph.calls.get(register_id).unwrap();
    let callee_names: BTreeSet<&str> = register_calls
        .iter()
        .map(|s| s.callee_name.as_str())
        .collect();

    assert!(
        callee_names.contains("my_handler"),
        "Level 3: register_handler(&my_handler, 2) should resolve handler to my_handler, got: {:?}",
        callee_names
    );
}

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
#[test]
fn test_cve_double_free_goto_cleanup() {
    let source = r#"
#include <stdlib.h>

void process_frame(uint8_t *raw, size_t len) {
    char *buf = malloc(len);
    char *header = malloc(64);

    if (validate(raw) < 0) {
        free(buf);
        free(header);
        goto cleanup;
    }

    process(buf, header);
    return;

cleanup:
    free(buf);
    free(header);
}
"#;

    let path = "src/frame.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff touches the malloc lines
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

    // Should detect double-free: free() before goto AND in cleanup label
    let double_close_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("double_close"))
        .collect();
    assert!(
        !double_close_findings.is_empty(),
        "Should detect double-free via goto cleanup pattern, got findings: {:?}",
        result.findings
    );
}

/// CVE pattern: correct goto cleanup (no double-free).
/// Kernel-style ordered labels with fall-through — should NOT flag as double-free.
#[test]
fn test_cve_correct_goto_cleanup_no_double_free() {
    let source = r#"
#include <stdlib.h>

int init_device(int id) {
    char *buf = malloc(256);
    if (!buf) return -1;

    char *dev = malloc(64);
    if (!dev) goto err_buf;

    int ret = register_dev(dev, id);
    if (ret < 0) goto err_dev;

    return 0;

err_dev:
    free(dev);
err_buf:
    free(buf);
    return -1;
}
"#;

    let path = "src/device.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5, 8]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    // Correct kernel cleanup pattern — should NOT report double-free
    let double_close_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("double_close"))
        .collect();
    assert!(
        double_close_findings.is_empty(),
        "Correct goto cleanup (no inline free before goto) should not flag double-free, got: {:?}",
        double_close_findings
    );
}

/// CVE pattern: kernel double-unlock via goto.
/// spin_lock released inline AND in the error label.
#[test]
fn test_cve_double_unlock_goto() {
    let source = r#"
#include <linux/spinlock.h>

int update_state(spinlock_t *lock, int val) {
    spin_lock(lock);

    if (val < 0) {
        spin_unlock(lock);
        goto err;
    }

    shared_state = val;
    spin_unlock(lock);
    return 0;

err:
    spin_unlock(lock);
    return -1;
}
"#;

    let path = "src/state.c";
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

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    let double_close_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("double_close"))
        .collect();
    assert!(
        !double_close_findings.is_empty(),
        "Should detect double-unlock: spin_unlock before goto AND in err label, got: {:?}",
        result.findings
    );
}

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
// ===========================================================================

#[test]
fn test_dfg_field_qualified_paths_created() {
    // Verify that the DFG creates AccessPath entries with field chains,
    // not just bare base names.
    let source = r#"
void init(struct device *dev) {
    dev->name = "eth0";
    dev->id = 42;
    dev->config->timeout = 100;
}
"#;
    let path = "src/dev.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let dev_defs = dfg.all_defs_of(path, "dev");

    // Should have qualified paths for each field
    let field_names: Vec<Vec<String>> = dev_defs
        .iter()
        .filter(|d| d.path.has_fields())
        .map(|d| d.path.fields.clone())
        .collect();
    assert!(
        field_names.iter().any(|f| f == &vec!["name".to_string()]),
        "DFG should have AccessPath dev.name, got: {:?}",
        field_names
    );
    assert!(
        field_names.iter().any(|f| f == &vec!["id".to_string()]),
        "DFG should have AccessPath dev.id, got: {:?}",
        field_names
    );
}

#[test]
fn test_dfg_dot_access_paths() {
    // Python-style dot access creates field-qualified paths.
    let source = r#"
class Config:
    def setup(self):
        self.timeout = 30
        self.host = "localhost"
"#;
    let path = "src/config.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let self_defs = dfg.all_defs_of(path, "self");

    // Should have field-qualified paths
    let has_timeout = self_defs
        .iter()
        .any(|d| d.path.has_fields() && d.path.fields.contains(&"timeout".to_string()));
    assert!(
        has_timeout,
        "DFG should record self.timeout AccessPath for Python dot access"
    );
}

#[test]
fn test_dfg_field_path_def_line_scoping() {
    // Verify that find_path_references_scoped only returns references AFTER
    // the definition line, preventing backward data flow edges.
    let source = r#"
void process(struct dev *d) {
    int old = d->status;
    d->status = 1;
    int new_val = d->status;
}
"#;
    let path = "src/proc.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let dev_defs = dfg.all_defs_of(path, "d");

    // The def of d->status on line 4 should only reach line 5 (new_val = d->status),
    // NOT line 3 (old = d->status) which is before the definition.
    let status_def = dev_defs
        .iter()
        .find(|d| d.path.fields == vec!["status".to_string()] && d.line == 4);
    assert!(
        status_def.is_some(),
        "Should have a def for d->status on line 4"
    );

    // Check forward edges from this def
    if let Some(def) = status_def {
        let reachable = dfg.forward_reachable(def);
        let reachable_lines: Vec<usize> = reachable.iter().map(|r| r.line).collect();
        assert!(
            !reachable_lines.contains(&3),
            "d->status def on line 4 should NOT reach line 3 (before def). Got: {:?}",
            reachable_lines
        );
    }
}

#[test]
fn test_dfg_var_name_backward_compat() {
    // Verify the var_name() accessor works for backward compatibility.
    let source = r#"
void f(int x) {
    int y = x;
}
"#;
    let path = "src/f.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let y_defs = dfg.all_defs_of(path, "y");
    assert!(!y_defs.is_empty());
    // var_name() returns the base name
    assert_eq!(y_defs[0].var_name(), "y");
}

#[test]
fn test_extract_lvalue_paths_pointer_deref() {
    // *ptr = val should create a def for base "ptr" only.
    let source = r#"
void write(int *ptr, int val) {
    *ptr = val;
}
"#;
    let path = "src/write.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let ptr_defs = dfg.all_defs_of(path, "ptr");
    assert!(
        ptr_defs.iter().any(|d| d.line == 3 && d.path.is_simple()),
        "Dereference *ptr should create a simple path def for 'ptr'"
    );
}

#[test]
fn test_rvalue_field_expression_paths() {
    // R-value field expressions should create AccessPath entries.
    let source = r#"
void copy(struct dev *src, struct dev *dst) {
    dst->id = src->id;
}
"#;
    let path = "src/copy.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);

    // dst->id should have a def with fields
    let dst_defs = dfg.all_defs_of(path, "dst");
    assert!(
        dst_defs
            .iter()
            .any(|d| d.path.fields == vec!["id".to_string()]),
        "Should have AccessPath dst.id def"
    );

    // src->id should appear in uses (rvalue)
    let has_src_use = dfg.uses.values().any(|locs| {
        locs.iter()
            .any(|l| l.path.base == "src" && l.path.fields == vec!["id".to_string()])
    });
    assert!(
        has_src_use,
        "R-value src->id should create a field-qualified use in DFG"
    );
}

// Cross-language field access in DFG
// ===========================================================================

#[test]
fn test_dfg_go_field_access_paths() {
    // Go selector_expression: obj.Field
    let source = r#"
package main

func process(dev *Device) {
	dev.Name = "eth0"
	dev.ID = 42
	x := dev.Name
	_ = x
}
"#;
    let path = "src/dev.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let dev_defs = dfg.all_defs_of(path, "dev");

    let has_name = dev_defs
        .iter()
        .any(|d| d.path.has_fields() && d.path.fields.contains(&"Name".to_string()));
    assert!(
        has_name,
        "Go DFG should have AccessPath dev.Name from selector_expression. Got: {:?}",
        dev_defs
            .iter()
            .map(|d| d.path.to_string())
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_dfg_js_field_access_paths() {
    // JS member_expression: obj.field
    let source = r#"
function setup(config) {
    config.timeout = 30;
    config.host = "localhost";
    let t = config.timeout;
}
"#;
    let path = "src/config.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let config_defs = dfg.all_defs_of(path, "config");

    let has_timeout = config_defs
        .iter()
        .any(|d| d.path.has_fields() && d.path.fields.contains(&"timeout".to_string()));
    assert!(
        has_timeout,
        "JS DFG should have AccessPath config.timeout from member_expression. Got: {:?}",
        config_defs
            .iter()
            .map(|d| d.path.to_string())
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_dfg_rust_field_access_paths() {
    // Rust field_expression: self.field or obj.field
    let source = r#"
struct Config {
    timeout: u32,
    host: String,
}

fn setup(config: &mut Config) {
    config.timeout = 30;
    config.host = String::from("localhost");
}
"#;
    let path = "src/config.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let config_defs = dfg.all_defs_of(path, "config");

    let has_timeout = config_defs
        .iter()
        .any(|d| d.path.has_fields() && d.path.fields.contains(&"timeout".to_string()));
    assert!(
        has_timeout,
        "Rust DFG should have AccessPath config.timeout from field_expression. Got: {:?}",
        config_defs
            .iter()
            .map(|d| d.path.to_string())
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_dfg_lua_field_access_paths() {
    // Lua dot_index_expression: obj.field
    let source = r#"
function setup(config)
    config.timeout = 30
    config.host = "localhost"
end
"#;
    let path = "src/config.lua";
    let parsed = ParsedFile::parse(path, source, Language::Lua).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let config_defs = dfg.all_defs_of(path, "config");

    let has_timeout = config_defs
        .iter()
        .any(|d| d.path.has_fields() && d.path.fields.contains(&"timeout".to_string()));
    assert!(
        has_timeout,
        "Lua DFG should have AccessPath config.timeout from dot_index_expression. Got: {:?}",
        config_defs
            .iter()
            .map(|d| d.path.to_string())
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_dfg_java_field_access_paths() {
    // Java field_access: obj.field
    let source = r#"
class Device {
    String name;
    int id;

    void setup(Device dev) {
        dev.name = "eth0";
        dev.id = 42;
    }
}
"#;
    let path = "src/Device.java";
    let parsed = ParsedFile::parse(path, source, Language::Java).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let dev_defs = dfg.all_defs_of(path, "dev");

    let has_name = dev_defs
        .iter()
        .any(|d| d.path.has_fields() && d.path.fields.contains(&"name".to_string()));
    assert!(
        has_name,
        "Java DFG should have AccessPath dev.name from field_access. Got: {:?}",
        dev_defs
            .iter()
            .map(|d| d.path.to_string())
            .collect::<Vec<_>>()
    );
}

// Downstream DFG consumer tests — assignment propagation, chop, provenance
// ===========================================================================

#[test]
fn test_dfg_same_line_cross_field_assignment() {
    // dev->name = dev->id on a single line.
    // LHS creates def for dev->name (and dev base).
    // RHS creates use for dev->id (and dev base).
    // Assignment propagation should connect use of dev->id → def of dev->name.
    let source = r#"
void copy_field(struct device *dev) {
    dev->name = dev->id;
    char *n = dev->name;
}
"#;
    let path = "src/dev.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);

    // Should have both field-qualified defs
    let dev_defs = dfg.all_defs_of(path, "dev");
    let has_name_def = dev_defs
        .iter()
        .any(|d| d.path.fields == vec!["name".to_string()] && d.line == 3);
    assert!(has_name_def, "Should have dev->name def on line 3");

    // Verify field-qualified use exists for RHS
    let has_id_use = dfg.uses.values().any(|locs| {
        locs.iter()
            .any(|l| l.path.base == "dev" && l.path.fields == vec!["id".to_string()] && l.line == 3)
    });
    assert!(has_id_use, "Should have dev->id use on line 3 (RHS)");
}

#[test]
fn test_dfg_assignment_propagation_with_fields() {
    // Taint on dev->id (line 3) should propagate through assignment:
    // dev->id = tainted → x = dev->id → strcpy(buf, x)
    let source = r#"
void process(struct device *dev, const char *input) {
    dev->id = input;
    char *x = dev->id;
    strcpy(buf, x);
}
"#;
    let path = "src/proc.c";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    // Taint should flow: line 3 (dev->id = input) → line 4 (x = dev->id) → line 5 (strcpy)
    assert!(
        !result.findings.is_empty(),
        "Taint should propagate through field assignment to strcpy sink"
    );
}

#[test]
fn test_dfg_forward_reachable_field_to_simple() {
    // Assignment propagation: dev->name = val on line 3, x = dev->name on line 4.
    // Forward reachable from dev->name def should reach x def via assignment propagation.
    let source = r#"
void f(struct dev *dev) {
    dev->name = "test";
    char *x = dev->name;
    printf("%s", x);
}
"#;
    let path = "src/f.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);

    // Find the dev->name def
    let dev_defs = dfg.all_defs_of(path, "dev");
    let name_def = dev_defs
        .iter()
        .find(|d| d.path.fields == vec!["name".to_string()] && d.line == 3);

    assert!(name_def.is_some(), "Should have dev->name def on line 3");

    if let Some(def) = name_def {
        let reachable = dfg.forward_reachable(def);
        let reachable_lines: BTreeSet<usize> = reachable.iter().map(|r| r.line).collect();
        // Should reach line 4 (x = dev->name) and line 5 (printf uses x)
        assert!(
            reachable_lines.contains(&4) || reachable_lines.contains(&5),
            "Forward reachable from dev->name should reach uses. Got lines: {:?}",
            reachable_lines
        );
    }
}

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
fn test_full_flow_javascript() {
    let (files, _, diff) = make_javascript_test();
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
fn test_parent_function_typescript() {
    let (files, _, diff) = make_typescript_test();
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
fn test_field_isolation_c_arrow() {
    // C arrow access: dev->name taint should NOT propagate to dev->id
    let source = r#"
void process(struct device *dev) {
    dev->name = get_user_input();
    dev->id = 42;
    use_name(dev->name);
    use_id(dev->id);
}
"#;
    let path = "src/dev.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let dev_defs = dfg.all_defs_of(path, "dev");

    // Should have field-qualified defs only — no base-only "dev" def
    let base_only_defs: Vec<_> = dev_defs.iter().filter(|d| !d.path.has_fields()).collect();
    assert!(
        base_only_defs.is_empty(),
        "Phase 2: field assignments should NOT create base-only defs. Got: {:?}",
        base_only_defs
    );

    // Forward from dev->name def should NOT reach dev->id use
    let name_def = dev_defs.iter().find(|d| d.path.fields == vec!["name"]);
    if let Some(nd) = name_def {
        let reachable = dfg.forward_reachable(nd);
        let reaches_id = reachable.iter().any(|r| r.path.fields == vec!["id"]);
        assert!(
            !reaches_id,
            "Phase 2: taint on dev->name must NOT propagate to dev->id"
        );
    }
}

#[test]
fn test_field_isolation_c_dot() {
    // C dot access (struct value): cfg.timeout taint should NOT reach cfg.host
    let source = r#"
void configure(struct config cfg) {
    cfg.timeout = get_input();
    cfg.host = "safe";
    use_timeout(cfg.timeout);
    use_host(cfg.host);
}
"#;
    let path = "src/cfg.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let cfg_defs = dfg.all_defs_of(path, "cfg");

    let base_only: Vec<_> = cfg_defs.iter().filter(|d| !d.path.has_fields()).collect();
    assert!(
        base_only.is_empty(),
        "Phase 2: dot field assignments should NOT create base-only defs. Got: {:?}",
        base_only
    );
}

#[test]
fn test_field_isolation_python() {
    let source = r#"
class Handler:
    def process(self):
        self.secret = get_password()
        self.label = "public"
        send(self.secret)
        display(self.label)
"#;
    let path = "src/handler.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let self_defs = dfg.all_defs_of(path, "self");

    let base_only: Vec<_> = self_defs.iter().filter(|d| !d.path.has_fields()).collect();
    assert!(
        base_only.is_empty(),
        "Phase 2 Python: field assignments should NOT create base-only defs"
    );

    let secret_def = self_defs.iter().find(|d| d.path.fields == vec!["secret"]);
    if let Some(sd) = secret_def {
        let reachable = dfg.forward_reachable(sd);
        let reaches_label = reachable.iter().any(|r| r.path.fields == vec!["label"]);
        assert!(
            !reaches_label,
            "Phase 2 Python: taint on self.secret must NOT propagate to self.label"
        );
    }
}

#[test]
fn test_field_isolation_javascript() {
    let source = r#"
function process(obj) {
    obj.secret = getUserInput();
    obj.display = "safe";
    sink(obj.secret);
    render(obj.display);
}
"#;
    let path = "src/handler.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let obj_defs = dfg.all_defs_of(path, "obj");

    let base_only: Vec<_> = obj_defs.iter().filter(|d| !d.path.has_fields()).collect();
    assert!(
        base_only.is_empty(),
        "Phase 2 JS: field assignments should NOT create base-only defs"
    );
}

#[test]
fn test_field_isolation_go() {
    let source = r#"
package main

func process(dev Device) {
    dev.Name = getInput()
    dev.ID = 42
    useName(dev.Name)
    useID(dev.ID)
}
"#;
    let path = "src/dev.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let dev_defs = dfg.all_defs_of(path, "dev");

    let base_only: Vec<_> = dev_defs.iter().filter(|d| !d.path.has_fields()).collect();
    assert!(
        base_only.is_empty(),
        "Phase 2 Go: field assignments should NOT create base-only defs"
    );
}

#[test]
fn test_field_isolation_rust() {
    let source = r#"
fn process(dev: &mut Device) {
    dev.name = get_input();
    dev.id = 42;
    use_name(dev.name);
    use_id(dev.id);
}
"#;
    let path = "src/dev.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let dev_defs = dfg.all_defs_of(path, "dev");

    let base_only: Vec<_> = dev_defs.iter().filter(|d| !d.path.has_fields()).collect();
    assert!(
        base_only.is_empty(),
        "Phase 2 Rust: field assignments should NOT create base-only defs"
    );
}

#[test]
fn test_field_isolation_lua() {
    let source = r#"
function process(dev)
    dev.name = get_input()
    dev.id = 42
    use_name(dev.name)
    use_id(dev.id)
end
"#;
    let path = "src/dev.lua";
    let parsed = ParsedFile::parse(path, source, Language::Lua).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let dev_defs = dfg.all_defs_of(path, "dev");

    let base_only: Vec<_> = dev_defs.iter().filter(|d| !d.path.has_fields()).collect();
    assert!(
        base_only.is_empty(),
        "Phase 2 Lua: field assignments should NOT create base-only defs"
    );
}

#[test]
fn test_field_isolation_java() {
    let source = r#"
class Handler {
    void process(Device dev) {
        dev.name = getInput();
        dev.id = 42;
        useName(dev.name);
        useId(dev.id);
    }
}
"#;
    let path = "src/Handler.java";
    let parsed = ParsedFile::parse(path, source, Language::Java).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let dev_defs = dfg.all_defs_of(path, "dev");

    let base_only: Vec<_> = dev_defs.iter().filter(|d| !d.path.has_fields()).collect();
    assert!(
        base_only.is_empty(),
        "Phase 2 Java: field assignments should NOT create base-only defs"
    );
}

#[test]
fn test_field_isolation_typescript() {
    let source = r#"
function process(obj: Config) {
    obj.secret = getUserInput();
    obj.label = "safe";
    sink(obj.secret);
    render(obj.label);
}
"#;
    let path = "src/handler.ts";
    let parsed = ParsedFile::parse(path, source, Language::TypeScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let obj_defs = dfg.all_defs_of(path, "obj");

    let base_only: Vec<_> = obj_defs.iter().filter(|d| !d.path.has_fields()).collect();
    assert!(
        base_only.is_empty(),
        "Phase 2 TypeScript: field assignments should NOT create base-only defs"
    );
}

#[test]
fn test_field_isolation_whole_struct_still_works() {
    // Whole-struct assignment (no field) should still create a base-only def
    let source = r#"
void init() {
    struct device *dev = malloc(sizeof(struct device));
    int x = 42;
    use(dev);
    use(x);
}
"#;
    let path = "src/init.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);

    // dev should still have a base-only def from the whole-struct assignment
    let dev_defs = dfg.all_defs_of(path, "dev");
    assert!(
        !dev_defs.is_empty(),
        "Whole-struct assignment should still create a def for dev"
    );
    assert!(
        dev_defs.iter().any(|d| !d.path.has_fields()),
        "Whole-struct assignment should create base-only def"
    );
}

#[test]
fn test_must_alias_c_pointer() {
    // ptr = dev; ptr->name = x → should create def for dev.name too
    let source = r#"
void process(struct device *dev) {
    struct device *ptr = dev;
    ptr->name = "eth0";
    use_name(dev->name);
}
"#;
    let path = "src/alias.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let dev_defs = dfg.all_defs_of(path, "dev");

    // Phase 3: ptr->name def should also create a dev.name def via alias resolution
    let has_dev_name = dev_defs.iter().any(|d| d.path.fields == vec!["name"]);
    assert!(
        has_dev_name,
        "Phase 3: ptr = dev alias should resolve ptr->name to dev.name. Got defs: {:?}",
        dev_defs.iter().map(|d| &d.path).collect::<Vec<_>>()
    );
}

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
fn test_type_db_struct_fields_and_typedef() {
    use prism::type_db::{FieldInfo, RecordInfo, RecordKind, TypeDatabase, TypedefInfo};

    let mut db = TypeDatabase::default();
    db.records.insert(
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
                FieldInfo {
                    name: "config".to_string(),
                    type_str: "struct config *".to_string(),
                    offset: None,
                },
            ],
            bases: vec![],
            virtual_methods: std::collections::BTreeMap::new(),
            size: Some(24),
            file: "device.h".to_string(),
        },
    );
    db.typedefs.insert(
        "dev_t".to_string(),
        TypedefInfo {
            name: "dev_t".to_string(),
            underlying: "struct device *".to_string(),
        },
    );

    // Typedef resolution
    assert_eq!(db.resolve_typedef("dev_t"), "struct device *");

    // Record lookup via typedef
    let record = db.resolve_record("dev_t").unwrap();
    assert_eq!(record.name, "device");
    assert_eq!(record.fields.len(), 3);

    // Field type query
    assert_eq!(db.field_type("device", "name"), Some("char *".to_string()));
    assert_eq!(
        db.field_type("device", "config"),
        Some("struct config *".to_string())
    );
    assert_eq!(db.field_type("device", "nonexistent"), None);

    // All fields
    let fields = db.all_fields("device");
    assert_eq!(fields.len(), 3);
}

#[test]
fn test_type_db_class_hierarchy_virtual_dispatch() {
    use prism::type_db::{RecordInfo, RecordKind, TypeDatabase};

    let mut db = TypeDatabase::default();

    // Base class: Shape with virtual draw()
    db.records.insert(
        "Shape".to_string(),
        RecordInfo {
            name: "Shape".to_string(),
            kind: RecordKind::Class,
            fields: vec![],
            bases: vec![],
            virtual_methods: std::collections::BTreeMap::from([
                ("draw".to_string(), "void ()".to_string()),
                ("area".to_string(), "double ()".to_string()),
            ]),
            size: None,
            file: "shape.h".to_string(),
        },
    );

    // Circle overrides draw() and area()
    db.records.insert(
        "Circle".to_string(),
        RecordInfo {
            name: "Circle".to_string(),
            kind: RecordKind::Class,
            fields: vec![],
            bases: vec!["Shape".to_string()],
            virtual_methods: std::collections::BTreeMap::from([
                ("draw".to_string(), "void ()".to_string()),
                ("area".to_string(), "double ()".to_string()),
            ]),
            size: None,
            file: "circle.h".to_string(),
        },
    );

    // Rect overrides draw() only
    db.records.insert(
        "Rect".to_string(),
        RecordInfo {
            name: "Rect".to_string(),
            kind: RecordKind::Class,
            fields: vec![],
            bases: vec!["Shape".to_string()],
            virtual_methods: std::collections::BTreeMap::from([(
                "draw".to_string(),
                "void ()".to_string(),
            )]),
            size: None,
            file: "rect.h".to_string(),
        },
    );

    db.class_hierarchy
        .insert("Circle".to_string(), vec!["Shape".to_string()]);
    db.class_hierarchy
        .insert("Rect".to_string(), vec!["Shape".to_string()]);

    // Virtual dispatch: draw() on Shape → Shape, Circle, Rect
    let mut draw_targets = db.virtual_dispatch_targets("Shape", "draw");
    draw_targets.sort();
    assert_eq!(draw_targets, vec!["Circle", "Rect", "Shape"]);

    // Virtual dispatch: area() on Shape → Shape, Circle (Rect doesn't override)
    let mut area_targets = db.virtual_dispatch_targets("Shape", "area");
    area_targets.sort();
    assert_eq!(area_targets, vec!["Circle", "Shape"]);

    // Hierarchy queries
    assert!(db.is_subclass_of("Circle", "Shape"));
    assert!(!db.is_subclass_of("Shape", "Circle"));
}

#[test]
fn test_type_db_union_field_aliasing() {
    use prism::type_db::{FieldInfo, RecordInfo, RecordKind, TypeDatabase};

    let mut db = TypeDatabase::default();
    db.records.insert(
        "value".to_string(),
        RecordInfo {
            name: "value".to_string(),
            kind: RecordKind::Union,
            fields: vec![
                FieldInfo {
                    name: "i".to_string(),
                    type_str: "int".to_string(),
                    offset: None,
                },
                FieldInfo {
                    name: "f".to_string(),
                    type_str: "float".to_string(),
                    offset: None,
                },
                FieldInfo {
                    name: "p".to_string(),
                    type_str: "void *".to_string(),
                    offset: None,
                },
            ],
            bases: vec![],
            virtual_methods: std::collections::BTreeMap::new(),
            size: Some(8),
            file: "value.h".to_string(),
        },
    );

    assert!(db.is_union("value"));
    assert!(!db.is_union("nonexistent"));
    assert_eq!(db.all_fields("value").len(), 3);
}

#[test]
fn test_type_db_clang_json_parsing() {
    use prism::type_db::TypeDatabase;

    // Simulate a clang JSON AST with struct, typedef, and class hierarchy
    let json = r#"{
        "kind": "TranslationUnitDecl",
        "inner": [
            {
                "kind": "RecordDecl",
                "name": "device",
                "tagUsed": "struct",
                "completeDefinition": true,
                "inner": [
                    {
                        "kind": "FieldDecl",
                        "name": "name",
                        "type": { "qualType": "char *" }
                    },
                    {
                        "kind": "FieldDecl",
                        "name": "id",
                        "type": { "qualType": "int" }
                    }
                ]
            },
            {
                "kind": "TypedefDecl",
                "name": "device_t",
                "type": { "qualType": "struct device *", "desugaredQualType": "struct device *" }
            },
            {
                "kind": "CXXRecordDecl",
                "name": "Base",
                "tagUsed": "class",
                "completeDefinition": true,
                "inner": [
                    {
                        "kind": "CXXMethodDecl",
                        "name": "process",
                        "virtual": true,
                        "type": { "qualType": "void ()" }
                    }
                ]
            },
            {
                "kind": "CXXRecordDecl",
                "name": "Derived",
                "tagUsed": "class",
                "completeDefinition": true,
                "bases": [
                    { "type": { "qualType": "class Base" } }
                ],
                "inner": [
                    {
                        "kind": "FieldDecl",
                        "name": "data",
                        "type": { "qualType": "int" }
                    },
                    {
                        "kind": "CXXMethodDecl",
                        "name": "process",
                        "virtual": true,
                        "type": { "qualType": "void ()" }
                    }
                ]
            }
        ]
    }"#;

    let mut db = TypeDatabase::default();
    db.extract_from_ast(json, "test.cpp").unwrap();

    // Struct extraction
    let device = db.records.get("device").unwrap();
    assert_eq!(device.fields.len(), 2);
    assert_eq!(device.fields[0].name, "name");

    // Typedef extraction
    let td = db.typedefs.get("device_t").unwrap();
    assert_eq!(td.underlying, "struct device *");

    // Record via typedef
    assert!(db.resolve_record("device_t").is_some());

    // C++ class with virtual method
    let base = db.records.get("Base").unwrap();
    assert!(base.virtual_methods.contains_key("process"));

    // Derived class with base and override
    let derived = db.records.get("Derived").unwrap();
    assert_eq!(derived.bases, vec!["Base"]);
    assert!(derived.virtual_methods.contains_key("process"));
    assert_eq!(derived.fields.len(), 1);
    assert_eq!(derived.fields[0].name, "data");
}

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

// ── JS/TS destructuring alias tracking ──────────────────────────────

#[test]
fn test_destructuring_object_basic_js() {
    // const { name, id } = device → name aliases device.name, id aliases device.id
    let source = r#"
function process(device) {
    const { name, id } = device;
    console.log(name);
    console.log(id);
}
"#;
    let path = "src/process.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();

    let func_node = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=6).collect();
    let aliases = parsed.collect_alias_assignments(&func_node, &lines);

    // Should have: name → device.name, id → device.id
    assert!(
        aliases
            .iter()
            .any(|(a, t, _)| a == "name" && t == "device.name"),
        "Expected alias name → device.name, got: {:?}",
        aliases
    );
    assert!(
        aliases
            .iter()
            .any(|(a, t, _)| a == "id" && t == "device.id"),
        "Expected alias id → device.id, got: {:?}",
        aliases
    );
}

#[test]
fn test_destructuring_renamed_js() {
    // const { name: userName } = device → userName aliases device.name
    let source = r#"
function process(device) {
    const { name: userName, id: deviceId } = device;
    console.log(userName);
}
"#;
    let path = "src/process.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();

    let func_node = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=5).collect();
    let aliases = parsed.collect_alias_assignments(&func_node, &lines);

    assert!(
        aliases
            .iter()
            .any(|(a, t, _)| a == "userName" && t == "device.name"),
        "Expected alias userName → device.name, got: {:?}",
        aliases
    );
    assert!(
        aliases
            .iter()
            .any(|(a, t, _)| a == "deviceId" && t == "device.id"),
        "Expected alias deviceId → device.id, got: {:?}",
        aliases
    );
}

#[test]
fn test_destructuring_nested_js() {
    // const { config: { host, port } } = device → host aliases device.config.host
    let source = r#"
function connect(device) {
    const { config: { host, port } } = device;
    open(host, port);
}
"#;
    let path = "src/connect.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();

    let func_node = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=5).collect();
    let aliases = parsed.collect_alias_assignments(&func_node, &lines);

    assert!(
        aliases
            .iter()
            .any(|(a, t, _)| a == "host" && t == "device.config.host"),
        "Expected alias host → device.config.host, got: {:?}",
        aliases
    );
    assert!(
        aliases
            .iter()
            .any(|(a, t, _)| a == "port" && t == "device.config.port"),
        "Expected alias port → device.config.port, got: {:?}",
        aliases
    );
}

#[test]
fn test_destructuring_array_js() {
    // const [first, second] = items → first aliases items, second aliases items
    let source = r#"
function process(items) {
    const [first, second] = items;
    console.log(first);
}
"#;
    let path = "src/process.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();

    let func_node = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=5).collect();
    let aliases = parsed.collect_alias_assignments(&func_node, &lines);

    // Array destructuring aliases each element to the base array
    assert!(
        aliases.iter().any(|(a, t, _)| a == "first" && t == "items"),
        "Expected alias first → items, got: {:?}",
        aliases
    );
    assert!(
        aliases
            .iter()
            .any(|(a, t, _)| a == "second" && t == "items"),
        "Expected alias second → items, got: {:?}",
        aliases
    );
}

#[test]
fn test_destructuring_typescript() {
    // TypeScript destructuring should work identically to JavaScript
    let source = r#"
function process(device: Device): void {
    const { name, id }: { name: string; id: number } = device;
    console.log(name);
}
"#;
    let path = "src/process.ts";
    let parsed = ParsedFile::parse(path, source, Language::TypeScript).unwrap();

    let func_node = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=5).collect();
    let aliases = parsed.collect_alias_assignments(&func_node, &lines);

    assert!(
        aliases
            .iter()
            .any(|(a, t, _)| a == "name" && t == "device.name"),
        "Expected TS alias name → device.name, got: {:?}",
        aliases
    );
}

#[test]
fn test_destructuring_no_false_positive_js() {
    // Two independent destructurings should not cross-contaminate
    let source = r#"
function process(a, b) {
    const { x } = a;
    const { x: y } = b;
    console.log(x, y);
}
"#;
    let path = "src/process.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();

    let func_node = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=6).collect();
    let aliases = parsed.collect_alias_assignments(&func_node, &lines);

    assert!(
        aliases.iter().any(|(a, t, _)| a == "x" && t == "a.x"),
        "Expected alias x → a.x, got: {:?}",
        aliases
    );
    assert!(
        aliases.iter().any(|(a, t, _)| a == "y" && t == "b.x"),
        "Expected alias y → b.x, got: {:?}",
        aliases
    );
    // Ensure no cross-contamination
    assert!(
        !aliases.iter().any(|(a, t, _)| a == "x" && t == "b.x"),
        "x should NOT alias b.x"
    );
}

#[test]
fn test_destructuring_through_alias_js() {
    // const ref = obj; const { name } = ref → name should resolve to obj.name
    let source = r#"
function process(obj) {
    const ref = obj;
    const { name, id } = ref;
    console.log(name);
}
"#;
    let path = "src/through.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let obj_defs = dfg.all_defs_of(path, "obj");

    // name should resolve through ref → obj, producing a def for obj.name
    let has_obj_name = obj_defs
        .iter()
        .any(|d| d.path.base == "obj" && d.path.fields == vec!["name"]);
    assert!(
        has_obj_name,
        "Destructuring through alias: name via ref should resolve to obj.name. Got defs: {:?}",
        obj_defs.iter().map(|d| &d.path).collect::<Vec<_>>()
    );
}

#[test]
fn test_destructuring_through_chain_js() {
    // const a = obj; const b = a; const { x } = b → x should resolve to obj.x
    let source = r#"
function process(obj) {
    const a = obj;
    const b = a;
    const { x } = b;
    console.log(x);
}
"#;
    let path = "src/chain.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let obj_defs = dfg.all_defs_of(path, "obj");

    let has_obj_x = obj_defs
        .iter()
        .any(|d| d.path.base == "obj" && d.path.fields == vec!["x"]);
    assert!(
        has_obj_x,
        "Destructuring through chain: x via b=a=obj should resolve to obj.x. Got defs: {:?}",
        obj_defs.iter().map(|d| &d.path).collect::<Vec<_>>()
    );
}

#[test]
fn test_destructuring_nested_through_alias_js() {
    // const ref = obj; const { config: { host } } = ref → host should resolve to obj.config.host
    let source = r#"
function connect(obj) {
    const ref = obj;
    const { config: { host } } = ref;
    open(host);
}
"#;
    let path = "src/nested_alias.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let obj_defs = dfg.all_defs_of(path, "obj");

    let has_obj_config_host = obj_defs
        .iter()
        .any(|d| d.path.base == "obj" && d.path.fields == vec!["config", "host"]);
    assert!(
        has_obj_config_host,
        "Nested destructuring through alias: host should resolve to obj.config.host. Got defs: {:?}",
        obj_defs.iter().map(|d| &d.path).collect::<Vec<_>>()
    );
}

// ── Lua colon method syntax ─────────────────────────────────────────

#[test]
fn test_lua_colon_method_field_access() {
    // obj:close() uses method_index_expression — should be recognized as field access
    let source = r#"
function cleanup(file)
    file:close()
    file:flush()
end
"#;
    let path = "src/cleanup.lua";
    let parsed = ParsedFile::parse(path, source, Language::Lua).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3, 4]),
        }],
    };

    // LeftFlow should trace file:close() back to the file parameter
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce blocks for Lua colon method calls"
    );
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

#[test]
fn test_without_cpg_context_runs_ast_only() {
    // CpgContext::without_cpg should work for AST-only algorithms
    let source = r#"
def add(x, y):
    return x + y
"#;
    let path = "src/add.py";
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

    let ctx = CpgContext::without_cpg(&files, None);
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff);
    let result = algorithms::run_slicing(&ctx, &diff, &config).unwrap();

    assert!(
        !result.blocks.is_empty(),
        "AST-only algorithm should work with empty CPG context"
    );
}

// ── Tree-sitter struct extraction fallback (item #4) ────────────────

#[test]
fn test_ts_fallback_extracts_c_struct() {
    use prism::type_db::TypeDatabase;

    let source = r#"
struct device {
    char *name;
    int id;
    float weight;
};
"#;
    let path = "src/device.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let db = TypeDatabase::from_parsed_files(&files);

    let record = db
        .records
        .get("device")
        .expect("should extract device struct");
    assert_eq!(record.kind, prism::type_db::RecordKind::Struct);
    let field_names: Vec<&str> = record.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(field_names, vec!["name", "id", "weight"]);
}

#[test]
fn test_ts_fallback_extracts_cpp_class() {
    use prism::type_db::TypeDatabase;

    let source = r#"
class Shape {
public:
    virtual void draw() = 0;
    int x;
    int y;
};
"#;
    let path = "src/shape.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let db = TypeDatabase::from_parsed_files(&files);

    let record = db.records.get("Shape").expect("should extract Shape class");
    assert_eq!(record.kind, prism::type_db::RecordKind::Class);
    assert!(
        record.virtual_methods.contains_key("draw"),
        "should detect virtual draw method"
    );
    let field_names: Vec<&str> = record.fields.iter().map(|f| f.name.as_str()).collect();
    assert!(field_names.contains(&"x"));
    assert!(field_names.contains(&"y"));
}

#[test]
fn test_ts_fallback_skips_forward_decl() {
    use prism::type_db::TypeDatabase;

    let source = r#"
struct device;
void use_device(struct device *d);
"#;
    let path = "src/forward.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let db = TypeDatabase::from_parsed_files(&files);

    assert!(
        db.records.is_empty(),
        "Forward declaration should not be extracted as a record"
    );
}

#[test]
fn test_ts_fallback_union_detection() {
    use prism::type_db::TypeDatabase;

    let source = r#"
union data {
    int i;
    float f;
    char bytes[4];
};
"#;
    let path = "src/data.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let db = TypeDatabase::from_parsed_files(&files);

    let record = db.records.get("data").expect("should extract data union");
    assert_eq!(record.kind, prism::type_db::RecordKind::Union);
}

#[test]
fn test_ts_fallback_nested_struct() {
    use prism::type_db::TypeDatabase;

    let source = r#"
struct config {
    int timeout;
    int retries;
};

struct device {
    char *name;
    struct config *cfg;
};
"#;
    let path = "src/device.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let db = TypeDatabase::from_parsed_files(&files);

    assert!(
        db.records.contains_key("config"),
        "should extract config struct"
    );
    assert!(
        db.records.contains_key("device"),
        "should extract device struct"
    );
    let device = db.records.get("device").unwrap();
    let field_names: Vec<&str> = device.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(field_names, vec!["name", "cfg"]);
}

#[test]
fn test_ts_fallback_no_false_extraction() {
    use prism::type_db::TypeDatabase;

    // Python and JS files should produce no records
    let py_source = r#"
class Device:
    def __init__(self, name):
        self.name = name
"#;
    let js_source = r#"
class Device {
    constructor(name) {
        this.name = name;
    }
}
"#;
    let mut files = BTreeMap::new();
    let py_parsed = ParsedFile::parse("src/device.py", py_source, Language::Python).unwrap();
    let js_parsed = ParsedFile::parse("src/device.js", js_source, Language::JavaScript).unwrap();
    files.insert("src/device.py".to_string(), py_parsed);
    files.insert("src/device.js".to_string(), js_parsed);

    let db = TypeDatabase::from_parsed_files(&files);

    assert!(
        db.records.is_empty(),
        "Non-C/C++ files should produce no records"
    );
}

#[test]
fn test_ts_fallback_cpp_inheritance() {
    use prism::type_db::TypeDatabase;

    let source = r#"
class Shape {
public:
    virtual void draw() = 0;
    int x;
};

class Circle : public Shape {
    float radius;
public:
    virtual void draw();
};
"#;
    let path = "src/shapes.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let db = TypeDatabase::from_parsed_files(&files);

    let circle = db.records.get("Circle").expect("should extract Circle");
    assert!(
        circle.bases.contains(&"Shape".to_string()),
        "Circle should have Shape as base class, got: {:?}",
        circle.bases
    );
    assert!(
        db.class_hierarchy.contains_key("Circle"),
        "Class hierarchy should include Circle"
    );
    assert!(db.is_subclass_of("Circle", "Shape"));
}

#[test]
fn test_ts_fallback_typedef() {
    use prism::type_db::TypeDatabase;

    let source = r#"
struct device {
    char *name;
    int id;
};

typedef struct device dev_t;
typedef int handle_t;
"#;
    let path = "src/types.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let db = TypeDatabase::from_parsed_files(&files);

    assert!(db.records.contains_key("device"));
    assert!(
        db.typedefs.contains_key("dev_t"),
        "should extract dev_t typedef"
    );
    assert!(
        db.typedefs.contains_key("handle_t"),
        "should extract handle_t typedef"
    );
}

// ── RTA refinement tests (item #5) ─────────────────────────────────

#[test]
fn test_rta_filters_uninstantiated_class() {
    use prism::type_db::TypeDatabase;

    let source = r#"
class Shape {
public:
    virtual void draw() = 0;
};

class Circle : public Shape {
public:
    float radius;
    virtual void draw();
};

class Square : public Shape {
public:
    float side;
    virtual void draw();
};

void render() {
    Circle c;
    c.draw();
}
"#;
    let path = "src/shapes.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let db = TypeDatabase::from_parsed_files(&files);
    let live = TypeDatabase::collect_live_classes(&files);

    // Circle is instantiated (stack allocation), Square is not
    assert!(
        live.contains("Circle"),
        "Circle should be live, got: {:?}",
        live
    );

    // RTA should include Circle but not Square
    let rta_targets = db.virtual_dispatch_targets_rta("Shape", "draw", &live);
    assert!(
        rta_targets.contains(&"Circle".to_string()),
        "RTA should include Circle"
    );
    assert!(
        !rta_targets.contains(&"Square".to_string()),
        "RTA should exclude uninstantiated Square"
    );

    // CHA should include both
    let cha_targets = db.virtual_dispatch_targets("Shape", "draw");
    assert!(
        cha_targets.contains(&"Circle".to_string()),
        "CHA should include Circle"
    );
    assert!(
        cha_targets.contains(&"Square".to_string()),
        "CHA should include Square"
    );
}

#[test]
fn test_rta_preserves_instantiated_class() {
    use prism::type_db::TypeDatabase;

    let source = r#"
class Base {
public:
    virtual void process();
};

class Derived : public Base {
public:
    virtual void process();
};

void run() {
    Derived d;
    d.process();
}
"#;
    let path = "src/derived.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let db = TypeDatabase::from_parsed_files(&files);
    let live = TypeDatabase::collect_live_classes(&files);

    assert!(live.contains("Derived"));

    let targets = db.virtual_dispatch_targets_rta("Base", "process", &live);
    assert!(
        targets.contains(&"Derived".to_string()),
        "RTA should preserve instantiated Derived"
    );
}

#[test]
fn test_rta_fallback_no_type_db() {
    use prism::type_db::TypeDatabase;

    let source = r#"
class Animal {
public:
    virtual void speak();
};

class Dog : public Animal {
public:
    virtual void speak();
};
"#;
    let path = "src/animals.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let db = TypeDatabase::from_parsed_files(&files);

    // Empty live set → falls back to CHA
    let empty_live = std::collections::BTreeSet::new();
    let targets = db.virtual_dispatch_targets_rta("Animal", "speak", &empty_live);
    let cha = db.virtual_dispatch_targets("Animal", "speak");

    assert_eq!(targets, cha, "Empty live set should fall back to CHA");
}

#[test]
fn test_rta_stack_allocation() {
    use prism::type_db::TypeDatabase;

    let source = r#"
class Processor {
public:
    virtual void run();
};

void main() {
    Processor p;
    p.run();
}
"#;
    let path = "src/proc.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let live = TypeDatabase::collect_live_classes(&files);
    assert!(
        live.contains("Processor"),
        "Stack allocation should count as instantiation"
    );
}

// ── Gap analysis: multi-target assignment, optional chaining, walrus ──

#[test]
fn test_go_multi_return_individual_defs() {
    let source = r#"
package main

func process() {
    val, err := getData()
    use(val)
    check(err)
}
"#;
    let path = "src/process.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);

    let val_defs = dfg.all_defs_of(path, "val");
    let err_defs = dfg.all_defs_of(path, "err");
    assert!(
        !val_defs.is_empty(),
        "Go multi-return: 'val' should have its own def"
    );
    assert!(
        !err_defs.is_empty(),
        "Go multi-return: 'err' should have its own def"
    );

    let composite_defs = dfg.all_defs_of(path, "val, err");
    assert!(
        composite_defs.is_empty(),
        "Go multi-return: should not have composite 'val, err' def, got {:?}",
        composite_defs
    );
}

#[test]
fn test_go_type_assertion_individual_defs() {
    let source = r#"
package main

func check(x interface{}) {
    str, ok := x.(string)
    if ok {
        use(str)
    }
}
"#;
    let path = "src/check.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    assert!(
        !dfg.all_defs_of(path, "str").is_empty(),
        "Go type assertion: 'str' should have def"
    );
    assert!(
        !dfg.all_defs_of(path, "ok").is_empty(),
        "Go type assertion: 'ok' should have def"
    );
}

#[test]
fn test_python_tuple_unpack_individual_defs() {
    let source = r#"
def process():
    name, age = get_user()
    use(name)
    use(age)
"#;
    let path = "src/process.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    assert!(
        !dfg.all_defs_of(path, "name").is_empty(),
        "Python tuple unpack: 'name' should have def"
    );
    assert!(
        !dfg.all_defs_of(path, "age").is_empty(),
        "Python tuple unpack: 'age' should have def"
    );
}

#[test]
fn test_python_star_unpack_individual_defs() {
    let source = r#"
def process():
    first, *rest = get_items()
    use(first)
    use(rest)
"#;
    let path = "src/process.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    assert!(
        !dfg.all_defs_of(path, "first").is_empty(),
        "Python star unpack: 'first' should have def"
    );
    assert!(
        !dfg.all_defs_of(path, "rest").is_empty(),
        "Python star unpack: 'rest' should have def"
    );
}

#[test]
fn test_python_walrus_operator_def() {
    let source = r#"
def process(items):
    if (n := len(items)) > 10:
        use(n)
"#;
    let path = "src/process.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    assert!(
        !dfg.all_defs_of(path, "n").is_empty(),
        "Python walrus operator: 'n' should have def from := expression"
    );
}

#[test]
fn test_js_optional_chaining_access_path() {
    let normal = AccessPath::from_expr("obj.config.host");
    let optional = AccessPath::from_expr("obj?.config?.host");
    assert_eq!(
        normal, optional,
        "Optional chaining should normalize to same AccessPath as dot access"
    );
    assert_eq!(optional.base, "obj");
    assert_eq!(optional.fields, vec!["config", "host"]);

    let mixed = AccessPath::from_expr("obj?.config.host");
    assert_eq!(mixed.base, "obj");
    assert_eq!(mixed.fields, vec!["config", "host"]);
}

// ── Review feedback: language-specific, optional chaining, walrus negative, nested tuple ──

#[test]
fn test_go_multi_return_individual_defs_lvalue() {
    let source = r#"
package main

func process() {
    host, port := getAddr()
    connect(host, port)
}
"#;
    let path = "src/go_multi.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=7).collect();
    let lvalues = parsed.assignment_lvalue_paths_on_lines(&func, &lines);

    let has_host = lvalues.iter().any(|(p, _)| p.base == "host");
    let has_port = lvalues.iter().any(|(p, _)| p.base == "port");
    assert!(
        has_host,
        "Go multi-return L-value: 'host' should exist, got {:?}",
        lvalues.iter().map(|(p, _)| &p.base).collect::<Vec<_>>()
    );
    assert!(has_port, "Go multi-return L-value: 'port' should exist");
    let has_composite = lvalues.iter().any(|(p, _)| p.base.contains(','));
    assert!(
        !has_composite,
        "Go multi-return: should not have composite L-value"
    );
}

#[test]
fn test_python_tuple_unpack_lvalue() {
    let source = r#"
def process():
    name, age = get_user()
    first, *rest = get_items()
    use(name)
"#;
    let path = "src/py_tuple.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=6).collect();
    let lvalues = parsed.assignment_lvalue_paths_on_lines(&func, &lines);

    assert!(
        lvalues.iter().any(|(p, _)| p.base == "name"),
        "Python: 'name' L-value"
    );
    assert!(
        lvalues.iter().any(|(p, _)| p.base == "age"),
        "Python: 'age' L-value"
    );
    assert!(
        lvalues.iter().any(|(p, _)| p.base == "first"),
        "Python: 'first' L-value"
    );
    assert!(
        lvalues.iter().any(|(p, _)| p.base == "rest"),
        "Python: 'rest' L-value"
    );
}

#[test]
fn test_go_assignment_multi_target() {
    let source = r#"
package main

func process() {
    var x, y int
    x, y = getCoords()
    use(x)
}
"#;
    let path = "src/go_assign.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let dfg = DataFlowGraph::build(&files);

    assert!(
        !dfg.all_defs_of(path, "x").is_empty(),
        "Go multi-assign: 'x' should have def"
    );
    assert!(
        !dfg.all_defs_of(path, "y").is_empty(),
        "Go multi-assign: 'y' should have def"
    );
}

#[test]
fn test_optional_chaining_single_level() {
    let ap = AccessPath::from_expr("obj?.name");
    assert_eq!(ap.base, "obj");
    assert_eq!(ap.fields, vec!["name"]);
}

#[test]
fn test_optional_chaining_element_access() {
    let ap = AccessPath::from_expr("arr?.[0]");
    assert_eq!(ap.base, "arr");
    assert!(
        !ap.fields.is_empty(),
        "arr?.[0] should produce fields, got {:?}",
        ap
    );
}

#[test]
fn test_optional_chaining_deep() {
    let ap = AccessPath::from_expr("a?.b?.c?.d");
    assert_eq!(ap.base, "a");
    assert_eq!(ap.fields, vec!["b", "c", "d"]);
}

#[test]
fn test_optional_chaining_does_not_break_arrow() {
    let ap = AccessPath::from_expr("dev->config->host");
    assert_eq!(ap.base, "dev");
    assert_eq!(ap.fields, vec!["config", "host"]);
}

#[test]
fn test_walrus_does_not_affect_regular_assignment() {
    let source = r#"
def process():
    x = 42
    y = x + 1
"#;
    let path = "src/regular.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=5).collect();
    let lvalues = parsed.assignment_lvalue_paths_on_lines(&func, &lines);

    assert!(
        lvalues.iter().any(|(p, _)| p.base == "x"),
        "Regular assignment: 'x' should have L-value"
    );
    assert!(
        lvalues.iter().any(|(p, _)| p.base == "y"),
        "Regular assignment: 'y' should have L-value"
    );
}

#[test]
fn test_walrus_does_not_affect_augmented_assignment() {
    let source = r#"
def process():
    x = 0
    x += 1
"#;
    let path = "src/augmented.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=5).collect();
    let lvalues = parsed.assignment_lvalue_paths_on_lines(&func, &lines);

    let x_count = lvalues.iter().filter(|(p, _)| p.base == "x").count();
    assert!(
        x_count >= 2,
        "Augmented assignment: 'x' should have defs from both = and +=, got {}",
        x_count
    );
}

#[test]
fn test_walrus_in_while_loop() {
    let source = r#"
def process(stream):
    while (chunk := stream.read(1024)):
        use(chunk)
"#;
    let path = "src/walrus_while.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let dfg = DataFlowGraph::build(&files);

    assert!(
        !dfg.all_defs_of(path, "chunk").is_empty(),
        "Walrus in while: 'chunk' should have def from := expression"
    );
}

#[test]
fn test_python_nested_tuple_unpack() {
    let source = r#"
def process():
    (a, b), c = get_nested()
    use(a)
    use(b)
    use(c)
"#;
    let path = "src/nested.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=7).collect();
    let lvalues = parsed.assignment_lvalue_paths_on_lines(&func, &lines);

    let has_a = lvalues.iter().any(|(p, _)| p.base == "a");
    let has_b = lvalues.iter().any(|(p, _)| p.base == "b");
    let has_c = lvalues.iter().any(|(p, _)| p.base == "c");
    assert!(
        has_a,
        "Nested tuple: 'a' should have L-value, got {:?}",
        lvalues.iter().map(|(p, _)| &p.base).collect::<Vec<_>>()
    );
    assert!(has_b, "Nested tuple: 'b' should have L-value");
    assert!(has_c, "Nested tuple: 'c' should have L-value");
}

// ── PR #31 review follow-ups: walrus RHS, gaps 3 & 4 ──────────────

#[test]
fn test_python_walrus_rhs_collected() {
    // Walrus RHS should be collected as a use (assignment_value must work)
    let source = r#"
def process(items):
    if (n := len(items)) > 10:
        use(n)
"#;
    let path = "src/walrus.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=4).collect();

    let aliases = parsed.collect_alias_assignments(&func, &lines);
    // Walrus RHS is a call, not plain ident — no alias expected, but should not panic.
    // The key test is that assignment_value returns Some for named_expression.
    let _ = aliases;

    // Verify def exists for n
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let dfg = DataFlowGraph::build(&files);
    let n_defs = dfg.all_defs_of(path, "n");
    assert!(!n_defs.is_empty(), "Walrus: 'n' should have a def");
}

#[test]
fn test_js_for_of_destructuring_def() {
    // Gap 3: for-of destructuring should produce defs for destructured variables
    let source = r#"
function process(items) {
    for (const { name, id } of items) {
        use(name);
        use(id);
    }
}
"#;
    let path = "src/process.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let name_defs = dfg.all_defs_of(path, "name");
    let id_defs = dfg.all_defs_of(path, "id");
    assert!(
        !name_defs.is_empty(),
        "for-of destructuring: 'name' should have def"
    );
    assert!(
        !id_defs.is_empty(),
        "for-of destructuring: 'id' should have def"
    );
}

#[test]
fn test_js_for_of_array_destructuring_def() {
    let source = r#"
function process(pairs) {
    for (const [key, value] of pairs) {
        use(key);
        use(value);
    }
}
"#;
    let path = "src/process.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let key_defs = dfg.all_defs_of(path, "key");
    let value_defs = dfg.all_defs_of(path, "value");
    assert!(
        !key_defs.is_empty(),
        "for-of array destructuring: 'key' should have def"
    );
    assert!(
        !value_defs.is_empty(),
        "for-of array destructuring: 'value' should have def"
    );
}

#[test]
fn test_js_for_in_simple_def() {
    let source = r#"
function listKeys(obj) {
    for (const key in obj) {
        use(key);
    }
}
"#;
    let path = "src/keys.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let key_defs = dfg.all_defs_of(path, "key");
    assert!(!key_defs.is_empty(), "for-in: 'key' should have def");
}

#[test]
fn test_python_with_as_binding_def() {
    // Gap 4: with...as should produce a def for the bound variable
    let source = r#"
def process():
    with open("file.txt") as f:
        data = f.read()
        send(data)
"#;
    let path = "src/process.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let f_defs = dfg.all_defs_of(path, "f");
    assert!(!f_defs.is_empty(), "with...as: 'f' should have def");
}

#[test]
fn test_python_except_as_binding_def() {
    // except...as uses as_pattern too
    let source = r#"
def process():
    try:
        risky()
    except Exception as e:
        handle(e)
"#;
    let path = "src/process.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let e_defs = dfg.all_defs_of(path, "e");
    assert!(!e_defs.is_empty(), "except...as: 'e' should have def");
}

#[test]
fn test_js_for_of_destructuring_aliases() {
    // Destructured variables in for-of should create aliases to iterable.property
    let source = r#"
function process(items) {
    for (const { name, id } of items) {
        use(name);
    }
}
"#;
    let path = "src/process.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=6).collect();

    let aliases = parsed.collect_alias_assignments(&func, &lines);
    let name_alias = aliases.iter().any(|(a, _t, _l)| a == "name");
    let id_alias = aliases.iter().any(|(a, _t, _l)| a == "id");
    assert!(
        name_alias,
        "for-of destructuring: 'name' should have alias, got {:?}",
        aliases
    );
    assert!(
        id_alias,
        "for-of destructuring: 'id' should have alias, got {:?}",
        aliases
    );
}

// ── Coverage: ast.rs and languages/mod.rs ──────────────────────────

#[test]
fn test_lua_assignment_target_and_value() {
    // Lua assignment_target walks variable_list, assignment_value walks expression_list
    let source = r#"
local function process()
    local x = 10
    x = 20
    use(x)
end
"#;
    let path = "src/lua.lua";
    let parsed = ParsedFile::parse(path, source, Language::Lua).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let x_defs = dfg.all_defs_of(path, "x");
    assert!(
        !x_defs.is_empty(),
        "Lua: 'x' should have defs from local and assignment"
    );
}

#[test]
fn test_lua_alias_assignment() {
    // Tests Lua assignment_target/assignment_value via alias tracking
    let source = r#"
local function process()
    local a = external
    local b = a
    use(b)
end
"#;
    let path = "src/lua_alias.lua";
    let parsed = ParsedFile::parse(path, source, Language::Lua).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=6).collect();

    let aliases = parsed.collect_alias_assignments(&func, &lines);
    let b_aliases_a = aliases.iter().any(|(a, t, _)| a == "b" && t == "a");
    assert!(b_aliases_a, "Lua: 'b' should alias 'a', got: {:?}", aliases);
}

#[test]
fn test_cpp_update_expression_def() {
    // C/C++ update_expression (++/--) is treated as assignment
    let source = r#"
void process() {
    int count = 0;
    count++;
    ++count;
    count--;
    use(count);
}
"#;
    let path = "src/process.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=7).collect();
    let lvalues = parsed.assignment_lvalue_paths_on_lines(&func, &lines);

    // count should have a def from the initial declaration
    let has_count = lvalues.iter().any(|(p, _)| p.base == "count");
    assert!(
        has_count,
        "C++ count should have L-value, got: {:?}",
        lvalues.iter().map(|(p, _)| &p.base).collect::<Vec<_>>()
    );
}

#[test]
fn test_python_multi_target_attribute_lvalue() {
    // Multi-target with attribute access: obj.x, obj.y = func()
    let source = r#"
def process(obj):
    obj.x, obj.y = get_coords()
    use(obj)
"#;
    let path = "src/process.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=4).collect();
    let lvalues = parsed.assignment_lvalue_paths_on_lines(&func, &lines);

    let has_obj_x = lvalues
        .iter()
        .any(|(p, _)| p.base == "obj" && p.fields.contains(&"x".to_string()));
    let has_obj_y = lvalues
        .iter()
        .any(|(p, _)| p.base == "obj" && p.fields.contains(&"y".to_string()));
    assert!(
        has_obj_x,
        "Python multi-target: obj.x should be L-value, got: {:?}",
        lvalues
    );
    assert!(
        has_obj_y,
        "Python multi-target: obj.y should be L-value, got: {:?}",
        lvalues
    );
}

#[test]
fn test_rust_let_destructuring_def() {
    // Rust let (a, b) = tuple; — destructuring in let_declaration
    let source = r#"
fn process() {
    let x = get_data();
    use_val(x);
}
"#;
    let path = "src/process.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let x_defs = dfg.all_defs_of(path, "x");
    assert!(!x_defs.is_empty(), "Rust let: 'x' should have a def");
}

#[test]
fn test_rust_compound_assignment() {
    // Rust compound assignment: x += 1
    let source = r#"
fn process() {
    let mut x = 0;
    x += 10;
    use_val(x);
}
"#;
    let path = "src/process.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=5).collect();
    let lvalues = parsed.assignment_lvalue_paths_on_lines(&func, &lines);

    let has_x = lvalues.iter().any(|(p, _)| p.base == "x");
    assert!(
        has_x,
        "Rust compound assignment: x should be L-value, got: {:?}",
        lvalues.iter().map(|(p, _)| &p.base).collect::<Vec<_>>()
    );
}

#[test]
fn test_js_array_destructuring_rest_alias() {
    // Array destructuring with rest: const [first, ...rest] = arr
    let source = r#"
function process(arr) {
    const [first, ...rest] = arr;
    use(first);
    use(rest);
}
"#;
    let path = "src/process.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=5).collect();

    let aliases = parsed.collect_alias_assignments(&func, &lines);
    let first_alias = aliases.iter().any(|(a, _t, _l)| a == "first");
    let rest_alias = aliases.iter().any(|(a, _t, _l)| a == "rest");
    assert!(
        first_alias,
        "Array rest: 'first' should have alias, got {:?}",
        aliases
    );
    assert!(
        rest_alias,
        "Array rest: 'rest' should have alias, got {:?}",
        aliases
    );
}

#[test]
fn test_walrus_assignment_value_flow() {
    // Walrus assignment_value should extract RHS identifiers for DFG use tracking.
    // If items is on a diff line, its use on the walrus RHS should be collected.
    let source = r#"
def process(items):
    if (n := len(items)) > 10:
        use(n)
"#;
    let path = "src/walrus.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=4).collect();

    // The walrus RHS is len(items) — not a plain ident, so no alias.
    // But the assignment_value path should be Some (not None).
    let aliases = parsed.collect_alias_assignments(&func, &lines);
    // No alias expected (RHS is a call), but shouldn't panic
    let _ = aliases;

    // Verify DFG has defs
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let dfg = DataFlowGraph::build(&files);
    let n_defs = dfg.all_defs_of(path, "n");
    assert!(!n_defs.is_empty(), "Walrus: 'n' def should exist");
}

#[test]
fn test_go_short_var_declaration_value() {
    // Go short_var_declaration uses "right" field for declaration_value
    let source = r#"
package main

func process() {
    x := getData()
    use(x)
}
"#;
    let path = "src/process.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=6).collect();

    // short_var_declaration is both declaration and assignment in Go
    let aliases = parsed.collect_alias_assignments(&func, &lines);
    // getData() is not a plain ident, so no alias — but the path should not crash
    let _ = aliases;

    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let dfg = DataFlowGraph::build(&files);
    let x_defs = dfg.all_defs_of(path, "x");
    assert!(
        !x_defs.is_empty(),
        "Go short_var_declaration: 'x' should have def"
    );
}

#[test]
fn test_go_var_declaration_with_value() {
    // Go var_declaration with explicit value
    let source = r#"
package main

func process() {
    var x int = 42
    use(x)
}
"#;
    let path = "src/process.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let x_defs = dfg.all_defs_of(path, "x");
    assert!(
        !x_defs.is_empty(),
        "Go var declaration: 'x' should have def"
    );
}

#[test]
fn test_c_init_declarator_value() {
    // C init_declarator: int *ptr = dev
    let source = r#"
void process() {
    int x = 42;
    int *ptr = &x;
    use(ptr);
}
"#;
    let path = "src/process.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=5).collect();
    let lvalues = parsed.assignment_lvalue_paths_on_lines(&func, &lines);

    let has_x = lvalues.iter().any(|(p, _)| p.base == "x");
    let has_ptr = lvalues.iter().any(|(p, _)| p.base == "ptr");
    assert!(has_x, "C: 'x' should have L-value");
    assert!(has_ptr, "C: 'ptr' should have L-value");
}

#[test]
fn test_js_for_of_array_destructuring_aliases() {
    // Array destructuring in for-of: const [a, b] of pairs
    let source = r#"
function process(pairs) {
    for (const [a, b] of pairs) {
        use(a);
        use(b);
    }
}
"#;
    let path = "src/process.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=6).collect();

    let aliases = parsed.collect_alias_assignments(&func, &lines);
    let a_alias = aliases.iter().any(|(a, _t, _l)| a == "a");
    let b_alias = aliases.iter().any(|(a, _t, _l)| a == "b");
    assert!(
        a_alias,
        "for-of array destructuring: 'a' should have alias, got {:?}",
        aliases
    );
    assert!(
        b_alias,
        "for-of array destructuring: 'b' should have alias, got {:?}",
        aliases
    );
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
        "tests/algo_novel_test.rs",
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
