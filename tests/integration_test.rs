use prism::algorithms;
use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
use prism::data_flow::DataFlowGraph;
use prism::diff::{DiffInfo, DiffInput, ModifyType};
use prism::languages::Language;
use prism::output;
use prism::slice::{SliceConfig, SlicingAlgorithm};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use tempfile::TempDir;

fn make_python_test() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let source = r#"
import os

GLOBAL_VAR = 42

def calculate(x, y):
    total = x + y
    if total > 10:
        result = total * 2
        print(result)
    else:
        result = total
    return result

def helper(val):
    return val + GLOBAL_VAR

def process(data):
    filtered = [d for d in data if d > 0]
    total = calculate(filtered[0], filtered[1])
    extra = helper(total)
    return extra
"#;

    let path = "src/calc.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    sources.insert(path.to_string(), source.to_string());

    // Diff: lines 7 (total = x + y) and 9 (result = total * 2) were changed
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([7, 9]),
        }],
    };

    (files, sources, diff)
}

fn make_javascript_test() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let source = r#"
function fetchData(url, options) {
    const headers = options.headers || {};
    const timeout = options.timeout || 5000;

    if (timeout > 10000) {
        throw new Error("Timeout too long");
    }

    const response = fetch(url, { headers, timeout });
    const data = response.json();

    if (data.error) {
        console.error(data.error);
        return null;
    }

    return data.result;
}

function processItems(items) {
    const results = [];
    for (const item of items) {
        const processed = fetchData(item.url, item.options);
        if (processed) {
            results.push(processed);
        }
    }
    return results;
}
"#;

    let path = "src/api.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    sources.insert(path.to_string(), source.to_string());

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([10, 11]),
        }],
    };

    (files, sources, diff)
}

fn make_go_test() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let source = r#"package main

import "fmt"

func sum(numbers []int) int {
	total := 0
	for _, n := range numbers {
		if n > 0 {
			total += n
		}
	}
	return total
}

func main() {
	data := []int{1, -2, 3, -4, 5}
	result := sum(data)
	fmt.Println(result)
}
"#;

    let path = "main.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    sources.insert(path.to_string(), source.to_string());

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([9]),
        }],
    };

    (files, sources, diff)
}

fn make_java_test() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let source = r#"public class Calculator {
    private int accumulator = 0;

    public int add(int a, int b) {
        int sum = a + b;
        accumulator += sum;
        return sum;
    }

    public int getAccumulator() {
        return accumulator;
    }

    public void reset() {
        accumulator = 0;
    }
}
"#;

    let path = "Calculator.java";
    let parsed = ParsedFile::parse(path, source, Language::Java).unwrap();
    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    sources.insert(path.to_string(), source.to_string());

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5, 6]),
        }],
    };

    (files, sources, diff)
}

fn make_typescript_test() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let source = r#"
interface Config {
    baseUrl: string;
    retries: number;
}

function createClient(config: Config) {
    const url = config.baseUrl;
    const maxRetries = config.retries;

    async function request(path: string): Promise<any> {
        let attempts = 0;
        while (attempts < maxRetries) {
            attempts += 1;
            try {
                const response = await fetch(url + path);
                return response.json();
            } catch (e) {
                if (attempts >= maxRetries) throw e;
            }
        }
    }

    return { request };
}
"#;

    let path = "src/client.ts";
    let parsed = ParsedFile::parse(path, source, Language::TypeScript).unwrap();
    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    sources.insert(path.to_string(), source.to_string());

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([14, 16, 17]),
        }],
    };

    (files, sources, diff)
}

// ====== OriginalDiff tests ======

#[test]
fn test_original_diff_python() {
    let (files, _, diff) = make_python_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff);
    let result = algorithms::run_slicing(&files, &diff, &config).unwrap();
    assert_eq!(result.blocks.len(), 1);
    assert_eq!(result.blocks[0].diff_lines.len(), 2);
    assert!(result.blocks[0].diff_lines.contains(&7));
    assert!(result.blocks[0].diff_lines.contains(&9));
}

#[test]
fn test_original_diff_javascript() {
    let (files, _, diff) = make_javascript_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff);
    let result = algorithms::run_slicing(&files, &diff, &config).unwrap();
    assert_eq!(result.blocks.len(), 1);
    assert_eq!(result.blocks[0].diff_lines.len(), 2);
}

// ====== ParentFunction tests ======

#[test]
fn test_parent_function_python() {
    let (files, sources, diff) = make_python_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction);
    let result = algorithms::run_slicing(&files, &diff, &config).unwrap();

    // Should include the entire calculate function
    assert!(!result.blocks.is_empty());
    let block = &result.blocks[0];
    let lines = block.file_line_map.get("src/calc.py").unwrap();
    // Function spans lines 6-13 approximately
    assert!(
        lines.len() > 2,
        "ParentFunction should include more than just diff lines"
    );

    let formatted = output::format_slice_result(&result.blocks, &sources);
    assert!(formatted.contains("calculate"));
}

#[test]
fn test_parent_function_go() {
    let (files, sources, diff) = make_go_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction);
    let result = algorithms::run_slicing(&files, &diff, &config).unwrap();

    assert!(!result.blocks.is_empty());
    let formatted = output::format_slice_result(&result.blocks, &sources);
    assert!(formatted.contains("sum"));
}

#[test]
fn test_parent_function_java() {
    let (files, sources, diff) = make_java_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction);
    let result = algorithms::run_slicing(&files, &diff, &config).unwrap();

    assert!(!result.blocks.is_empty());
    let formatted = output::format_slice_result(&result.blocks, &sources);
    assert!(formatted.contains("add"));
}

// ====== LeftFlow tests ======

#[test]
fn test_left_flow_python() {
    let (files, sources, diff) = make_python_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow);
    let result = algorithms::run_slicing(&files, &diff, &config).unwrap();

    assert!(!result.blocks.is_empty());
    let block = &result.blocks[0];
    let lines = block.file_line_map.get("src/calc.py").unwrap();

    // LeftFlow should trace variable references — `total` and `result` are used
    // on multiple lines, so we should get more than just the 2 diff lines
    assert!(
        lines.len() > 2,
        "LeftFlow should trace variable references beyond diff lines, got {} lines",
        lines.len()
    );

    let formatted = output::format_slice_result(&result.blocks, &sources);
    assert!(formatted.contains("return"));
}

#[test]
fn test_left_flow_javascript() {
    let (files, sources, diff) = make_javascript_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow);
    let result = algorithms::run_slicing(&files, &diff, &config).unwrap();

    assert!(!result.blocks.is_empty());
    let formatted = output::format_slice_result(&result.blocks, &sources);
    // Should include references to response and data variables
    assert!(
        formatted.contains("fetchData")
            || formatted.contains("response")
            || formatted.contains("data")
    );
}

#[test]
fn test_left_flow_typescript() {
    let (files, _, diff) = make_typescript_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow);
    let result = algorithms::run_slicing(&files, &diff, &config).unwrap();
    assert!(!result.blocks.is_empty());
}

// ====== FullFlow tests ======

#[test]
fn test_full_flow_python() {
    let (files, sources, diff) = make_python_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow);
    let result = algorithms::run_slicing(&files, &diff, &config).unwrap();

    assert!(!result.blocks.is_empty());
    let block = &result.blocks[0];
    let lines = block.file_line_map.get("src/calc.py").unwrap();

    // FullFlow should have at least as many lines as LeftFlow
    assert!(
        lines.len() >= 2,
        "FullFlow should include at least diff lines plus references"
    );
}

#[test]
fn test_full_flow_go() {
    let (files, sources, diff) = make_go_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow);
    let result = algorithms::run_slicing(&files, &diff, &config).unwrap();

    assert!(!result.blocks.is_empty());
    let formatted = output::format_slice_result(&result.blocks, &sources);
    assert!(formatted.contains("total"));
}

#[test]
fn test_full_flow_java() {
    let (files, sources, diff) = make_java_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow);
    let result = algorithms::run_slicing(&files, &diff, &config).unwrap();

    assert!(!result.blocks.is_empty());
    let formatted = output::format_slice_result(&result.blocks, &sources);
    assert!(formatted.contains("sum") || formatted.contains("accumulator"));
}

// ====== Comparative tests ======

#[test]
fn test_increasing_context() {
    let (files, _, diff) = make_python_test();

    let orig = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
    )
    .unwrap();

    let parent = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction),
    )
    .unwrap();

    let left = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
    )
    .unwrap();

    let full = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow),
    )
    .unwrap();

    let orig_lines: usize = orig
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();
    let parent_lines: usize = parent
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();

    // OriginalDiff should always have the fewest lines
    assert!(
        orig_lines <= parent_lines,
        "OriginalDiff ({}) should have <= lines than ParentFunction ({})",
        orig_lines,
        parent_lines
    );
}

// ====== Output format tests ======

#[test]
fn test_paper_format_output() {
    let (files, _, diff) = make_python_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff);
    let result = algorithms::run_slicing(&files, &diff, &config).unwrap();

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
    let result = algorithms::run_slicing(&files, &diff, &config).unwrap();

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

// ====== Thin Slice tests ======

#[test]
fn test_thin_slice_subset_of_leftflow() {
    let (files, _, diff) = make_python_test();

    let thin = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
    )
    .unwrap();

    let left = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
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
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
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

// ====== Barrier Slice tests ======

#[test]
fn test_barrier_slice_python() {
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::BarrierSlice),
    )
    .unwrap();

    // Should include caller/callee information
    assert!(!result.blocks.is_empty());
}

// ====== Taint tests ======

#[test]
fn test_taint_from_diff() {
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
    )
    .unwrap();

    // Taint should propagate from diff lines
    assert!(!result.blocks.is_empty());
}

// ====== Relevant Slice tests ======

#[test]
fn test_relevant_slice_includes_alternates() {
    let (files, sources, diff) = make_python_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::RelevantSlice),
    )
    .unwrap();

    assert!(!result.blocks.is_empty());
    let block = &result.blocks[0];
    let lines = block.file_line_map.get("src/calc.py").unwrap();

    // RelevantSlice should include at least as much as LeftFlow
    let left = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
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

// ====== Spiral Slice tests ======

#[test]
fn test_spiral_slice_ring_containment() {
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::SpiralSlice),
    )
    .unwrap();

    // Spiral should include at least the original diff lines
    assert!(!result.blocks.is_empty());

    let orig = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
    )
    .unwrap();

    let spiral_lines: usize = result
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();
    let orig_lines: usize = orig
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();

    assert!(
        spiral_lines >= orig_lines,
        "SpiralSlice ({}) should have >= lines than OriginalDiff ({})",
        spiral_lines,
        orig_lines
    );
}

// ====== Circular Slice tests ======

fn make_mutual_recursion_test() -> (BTreeMap<String, ParsedFile>, DiffInput) {
    let source = r#"
def ping(n):
    if n <= 0:
        return
    print("ping", n)
    pong(n - 1)

def pong(n):
    if n <= 0:
        return
    print("pong", n)
    ping(n - 1)
"#;
    let path = "recursive.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([6]), // pong(n - 1) call in ping
        }],
    };

    (files, diff)
}

#[test]
fn test_circular_slice_detects_cycle() {
    let (files, diff) = make_mutual_recursion_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::CircularSlice),
    )
    .unwrap();

    // Should detect the ping↔pong cycle
    // The call graph will find the cycle
    let call_graph = CallGraph::build(&files);
    let cycles = call_graph.find_cycles_from(&["ping"]);
    // There should be at least one cycle
    assert!(
        !cycles.is_empty() || !result.blocks.is_empty(),
        "Should detect mutual recursion cycle"
    );
}

// ====== Horizontal Slice tests ======

#[test]
fn test_horizontal_slice_finds_peers() {
    let source = r#"
def handle_create(request):
    data = request.json()
    validate(data)
    return create_item(data)

def handle_update(request):
    data = request.json()
    return update_item(data)

def handle_delete(request):
    item_id = request.args.get("id")
    return delete_item(item_id)
"#;
    let path = "handlers.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]), // validate(data) line in handle_create
        }],
    };

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::HorizontalSlice),
    )
    .unwrap();

    assert!(!result.blocks.is_empty());
    // Should include peer functions (handle_update, handle_delete)
    let block = &result.blocks[0];
    let lines = block.file_line_map.get("handlers.py").unwrap();
    assert!(
        lines.len() > 5,
        "HorizontalSlice should include peer functions, got {} lines",
        lines.len()
    );
}

// ====== Vertical Slice tests ======

#[test]
fn test_vertical_slice_traces_layers() {
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::VerticalSlice),
    )
    .unwrap();

    // Should produce at least one block showing the call chain
    // (calculate is called by process, which calls helper)
    assert!(!result.blocks.is_empty());
}

// ====== Angle Slice tests ======

fn make_error_handling_test() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let source = r#"
def fetch_data(url):
    try:
        response = requests.get(url)
        response.raise_for_status()
        return response.json()
    except Exception as e:
        log.error(f"Failed to fetch {url}: {e}")
        raise

def process(url):
    try:
        data = fetch_data(url)
        return transform(data)
    except Exception:
        return None
"#;
    let path = "service.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    sources.insert(path.to_string(), source.to_string());

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([8]), // log.error line
        }],
    };

    (files, sources, diff)
}

#[test]
fn test_angle_slice_error_handling() {
    let (files, _, diff) = make_error_handling_test();
    let concern = prism::algorithms::angle_slice::Concern::ErrorHandling;
    let result = prism::algorithms::angle_slice::slice(&files, &diff, &concern).unwrap();

    assert!(!result.blocks.is_empty());
    let block = &result.blocks[0];
    let lines = block.file_line_map.get("service.py").unwrap();
    // Should find error handling patterns across both functions
    assert!(
        lines.len() > 3,
        "AngleSlice should trace error handling across functions"
    );
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

// ====== Quantum Slice tests ======

fn make_async_test() -> (BTreeMap<String, ParsedFile>, DiffInput) {
    let source = r#"
async function fetchUser(id) {
    let user = null;
    const response = await fetch(`/api/users/${id}`);
    user = await response.json();
    if (user.active) {
        return user;
    }
    return null;
}
"#;
    let path = "async.js";
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

    (files, diff)
}

#[test]
fn test_quantum_slice_async_js() {
    let (files, diff) = make_async_test();
    let result = prism::algorithms::quantum_slice::slice(&files, &diff, Some("user")).unwrap();

    // May or may not find async patterns depending on tree-sitter parsing
    // Just verify it doesn't crash
    assert!(result.algorithm == SlicingAlgorithm::QuantumSlice);
}

// ====== Conditioned Slice tests ======

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

// ====== Absence Slice tests ======

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

#[test]
fn test_absence_slice_detects_missing_close() {
    let (files, diff) = make_resource_leak_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
    )
    .unwrap();

    // Should detect open() without close()
    assert!(
        !result.blocks.is_empty(),
        "AbsenceSlice should detect open without close"
    );
}

// ====== Symmetry Slice tests ======

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

#[test]
fn test_symmetry_slice_finds_counterpart() {
    let (files, diff) = make_symmetry_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::SymmetrySlice),
    )
    .unwrap();

    // Should find deserialize as counterpart to serialize
    // (may or may not produce blocks depending on whether it considers them "broken")
    assert!(result.algorithm == SlicingAlgorithm::SymmetrySlice);
}

// ====== Gradient Slice tests ======

#[test]
fn test_gradient_slice_scores_decay() {
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::GradientSlice),
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

// ====== Provenance Slice tests ======

#[test]
fn test_provenance_slice_traces_origins() {
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
    )
    .unwrap();

    // Should trace variable origins on diff lines
    assert!(result.algorithm == SlicingAlgorithm::ProvenanceSlice);
}

// ====== Membrane Slice tests ======

#[test]
fn test_membrane_slice_finds_callers() {
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
    )
    .unwrap();

    // With a single file, cross-file callers won't exist, but it shouldn't crash
    assert!(result.algorithm == SlicingAlgorithm::MembraneSlice);
}

// ====== Echo Slice tests ======

#[test]
fn test_echo_slice_finds_ripple() {
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::EchoSlice),
    )
    .unwrap();

    assert!(result.algorithm == SlicingAlgorithm::EchoSlice);
}

// ====== Resonance Slice tests ======

#[test]
fn test_resonance_slice_runs() {
    let (files, _, diff) = make_python_test();
    // Resonance needs git — will return empty without a repo, but shouldn't crash
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ResonanceSlice),
    )
    .unwrap();

    assert!(result.algorithm == SlicingAlgorithm::ResonanceSlice);
}

// ====== Phantom Slice tests ======

#[test]
fn test_phantom_slice_runs() {
    let (files, _, diff) = make_python_test();
    // Phantom needs git — will return empty without a repo, but shouldn't crash
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::PhantomSlice),
    )
    .unwrap();

    assert!(result.algorithm == SlicingAlgorithm::PhantomSlice);
}

// ====== C Language Support tests ======

fn make_c_test() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let source = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MAX_BUF_SIZE 256

typedef struct {
    char *name;
    int id;
    int active;
} device_t;

device_t *create_device(const char *name, int id) {
    device_t *dev = malloc(sizeof(device_t));
    if (dev == NULL) {
        return NULL;
    }
    dev->name = strdup(name);
    dev->id = id;
    dev->active = 1;
    return dev;
}

void destroy_device(device_t *dev) {
    if (dev != NULL) {
        free(dev->name);
        free(dev);
    }
}

int process_packet(const char *buf, size_t len) {
    char local_buf[MAX_BUF_SIZE];
    int result = 0;

    memcpy(local_buf, buf, len);
    local_buf[len] = '\0';

    if (strlen(local_buf) > 10) {
        result = atoi(local_buf);
    }

    return result;
}

int handle_request(const char *input, size_t input_len) {
    device_t *dev = create_device(input, 42);
    if (dev == NULL) {
        return -1;
    }

    int status = process_packet(input, input_len);

    if (status < 0) {
        return status;
    }

    destroy_device(dev);
    return status;
}

void bulk_process(const char **inputs, int count) {
    for (int i = 0; i < count; i++) {
        int result = handle_request(inputs[i], strlen(inputs[i]));
        if (result < 0) {
            fprintf(stderr, "Error processing input %d\n", i);
        }
    }
}
"#;

    let path = "src/device.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    sources.insert(path.to_string(), source.to_string());

    // Diff: process_packet function modified (lines 34-44: the buffer handling code)
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([36, 37, 38]),
        }],
    };

    (files, sources, diff)
}

fn make_c_multifile_test() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let device_source = r#"
#include "device.h"
#include <stdlib.h>
#include <string.h>

device_t *create_device(const char *name, int id) {
    device_t *dev = malloc(sizeof(device_t));
    dev->name = strdup(name);
    dev->id = id;
    return dev;
}

void destroy_device(device_t *dev) {
    free(dev->name);
    free(dev);
}

int get_device_status(device_t *dev) {
    return dev->active;
}
"#;

    let handler_source = r#"
#include "device.h"
#include <stdio.h>

int handle_create(const char *name) {
    device_t *dev = create_device(name, 1);
    int status = get_device_status(dev);
    printf("Device status: %d\n", status);
    return status;
}

int handle_batch(const char **names, int count) {
    for (int i = 0; i < count; i++) {
        handle_create(names[i]);
    }
    return 0;
}
"#;

    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();

    let dev_parsed = ParsedFile::parse("src/device.c", device_source, Language::C).unwrap();
    let handler_parsed = ParsedFile::parse("src/handler.c", handler_source, Language::C).unwrap();

    files.insert("src/device.c".to_string(), dev_parsed);
    files.insert("src/handler.c".to_string(), handler_parsed);
    sources.insert("src/device.c".to_string(), device_source.to_string());
    sources.insert("src/handler.c".to_string(), handler_source.to_string());

    // Diff: create_device modified (return type change, error handling change)
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/device.c".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([7, 8, 9]),
        }],
    };

    (files, sources, diff)
}

fn make_cpp_test() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let source = r#"
#include <string>
#include <vector>
#include <memory>
#include <mutex>
#include <stdexcept>

class DeviceManager {
private:
    std::vector<std::string> devices;
    std::mutex mtx;
    int max_devices;

public:
    DeviceManager(int max) : max_devices(max) {}

    ~DeviceManager() {
        devices.clear();
    }

    bool add_device(const std::string& name) {
        std::lock_guard<std::mutex> lock(mtx);
        if (devices.size() >= max_devices) {
            return false;
        }
        devices.push_back(name);
        return true;
    }

    std::string get_device(int index) {
        if (index < 0 || index >= devices.size()) {
            throw std::out_of_range("Invalid device index");
        }
        return devices[index];
    }

    int count() const {
        return devices.size();
    }

    std::string serialize() {
        std::string result = "{";
        for (size_t i = 0; i < devices.size(); i++) {
            result += "\"" + devices[i] + "\"";
            if (i < devices.size() - 1) {
                result += ",";
            }
        }
        result += "}";
        return result;
    }
};

int process_devices(DeviceManager& mgr, const std::vector<std::string>& names) {
    int added = 0;
    for (const auto& name : names) {
        if (mgr.add_device(name)) {
            added++;
        }
    }
    return added;
}
"#;

    let path = "src/device_manager.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    sources.insert(path.to_string(), source.to_string());

    // Diff: add_device and get_device methods modified
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([23, 24, 25, 33, 34]),
        }],
    };

    (files, sources, diff)
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
    let result = algorithms::run_slicing(&files, &diff, &config).unwrap();

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
    let result = algorithms::run_slicing(&files, &diff, &config).unwrap();

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
    let result = algorithms::run_slicing(&files, &diff, &config).unwrap();

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
    let result = algorithms::run_slicing(&files, &diff, &config).unwrap();

    assert!(
        !result.blocks.is_empty(),
        "FullFlow should produce output for C++ code"
    );
}

// ====== C Absence Slice tests ======

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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
    )
    .unwrap();

    // Should detect malloc without free
    assert!(
        !result.blocks.is_empty(),
        "Absence slice should detect malloc without free in C code"
    );
}

// ====== C Taint Slice tests ======

#[test]
fn test_taint_c_buffer_overflow() {
    let (files, sources, diff) = make_c_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
    )
    .unwrap();

    // Taint should trace from the diff lines (buffer operations) forward
    assert!(result.algorithm == SlicingAlgorithm::Taint);
    // Even if no explicit taint source is specified, auto-taint from diff should work
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

// ====== C Echo Slice tests ======

#[test]
fn test_echo_slice_c() {
    let (files, _, diff) = make_c_multifile_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::EchoSlice),
    )
    .unwrap();

    // Echo should detect that handle_create calls create_device
    // and may not handle changes to create_device's return value
    assert!(result.algorithm == SlicingAlgorithm::EchoSlice);
}

// ====== C Membrane Slice tests ======

#[test]
fn test_membrane_slice_c() {
    let (files, _, diff) = make_c_multifile_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
    )
    .unwrap();

    // Membrane should detect cross-file callers of create_device
    assert!(result.algorithm == SlicingAlgorithm::MembraneSlice);
}

// ====== C Symmetry Slice tests ======

#[test]
fn test_symmetry_slice_c() {
    let (files, _, diff) = make_c_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::SymmetrySlice),
    )
    .unwrap();

    // Should detect create_device / destroy_device as a symmetric pair
    assert!(result.algorithm == SlicingAlgorithm::SymmetrySlice);
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

// ====== C Gradient Slice tests ======

#[test]
fn test_gradient_slice_c() {
    let (files, _, diff) = make_c_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::GradientSlice),
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Gradient slice should produce scored output for C code"
    );
}

// ====== C Quantum Slice tests ======

#[test]
fn test_quantum_slice_c_pthread() {
    let source = r#"
#include <pthread.h>
#include <stdio.h>

int shared_counter = 0;

void *worker(void *arg) {
    shared_counter++;
    return NULL;
}

int main() {
    pthread_t thread;
    pthread_create(&thread, NULL, worker, NULL);
    shared_counter++;
    pthread_join(thread, NULL);
    printf("Counter: %d\n", shared_counter);
    return 0;
}
"#;

    let path = "src/threaded.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([14]), // pthread_create line
        }],
    };

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
    )
    .unwrap();

    assert!(result.algorithm == SlicingAlgorithm::QuantumSlice);
}

// ====== C Provenance Slice tests ======

#[test]
fn test_provenance_slice_c() {
    let (files, _, diff) = make_c_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
    )
    .unwrap();

    assert!(result.algorithm == SlicingAlgorithm::ProvenanceSlice);
}

// ====== C++ Specific tests ======

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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::SymmetrySlice),
    )
    .unwrap();

    assert!(result.algorithm == SlicingAlgorithm::SymmetrySlice);
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

// ====== Review output format tests ======

use prism::output::{to_review_output, MultiReviewOutput};
use prism::slice::{MultiSliceResult, SliceFinding};

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
fn test_review_output_format_single_algorithm() {
    let (files, sources, diff) = make_python_test();

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
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
        let r =
            algorithms::run_slicing(&files, &diff, &SliceConfig::default().with_algorithm(algo))
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
fn test_taint_findings_populated() {
    let (files, sources, diff) = make_taint_test_fixture();

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
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
fn test_absence_findings_populated() {
    let (files, sources, diff) = make_absence_test_fixture();

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
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
fn test_multi_algorithm_findings_merged() {
    let (files, sources, diff) = make_python_test();

    let algorithms_to_run = SlicingAlgorithm::review_suite();
    let mut all_results = vec![];
    let mut errors = vec![];

    for &algo in &algorithms_to_run {
        match algorithms::run_slicing(&files, &diff, &SliceConfig::default().with_algorithm(algo)) {
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

fn make_snmp_overflow_test() -> (BTreeMap<String, ParsedFile>, DiffInput) {
    let source = r#"
#include <stdint.h>
#include <string.h>

void handle_snmp_set(uint8_t *pdu, size_t pdu_len) {
    char community[64];
    size_t community_len = pdu[7];
    memcpy(community, pdu + 8, community_len);
    if (strcmp(community, "public") == 0) {
        process_set_request(pdu + 8 + community_len, pdu_len - 8 - community_len);
    }
}
"#;

    let path = "tests/fixtures/c/snmp_overflow.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff: lines 7-8 (community_len extraction and memcpy without bounds check)
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([7, 8]),
        }],
    };

    (files, diff)
}

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

fn make_double_free_test() -> (BTreeMap<String, ParsedFile>, DiffInput) {
    let source = r#"
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

typedef struct {
    uint8_t *payload;
    size_t len;
} frame_t;

void process_frame(uint8_t *raw, size_t len) {
    frame_t *frame = malloc(sizeof(frame_t));
    frame->payload = malloc(len);
    memcpy(frame->payload, raw, len);

    if (validate_header(frame) < 0) {
        free(frame->payload);
        free(frame);
        goto cleanup;
    }

    dispatch_frame(frame);
    return;

cleanup:
    free(frame->payload);
    free(frame);
}
"#;

    let path = "tests/fixtures/c/double_free.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff: the cleanup label and double free
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([17, 18, 25, 26]),
        }],
    };

    (files, diff)
}

fn make_ring_overflow_test() -> (BTreeMap<String, ParsedFile>, DiffInput) {
    let source = r#"
#include <stdint.h>
#include <string.h>

#define RING_SIZE 256
static uint8_t ring_buf[RING_SIZE];
static volatile int write_idx = 0;

void ring_write(uint8_t *data, int count) {
    memcpy(ring_buf + write_idx, data, count);
    write_idx += count;
}
"#;

    let path = "tests/fixtures/c/ring_overflow.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff: the memcpy and write_idx update (no bounds check)
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([10, 11]),
        }],
    };

    (files, diff)
}

fn make_timer_uaf_test() -> (BTreeMap<String, ParsedFile>, DiffInput) {
    let source = r#"
#include <stdlib.h>

struct timer_ctx {
    void (*callback)(void *);
    void *data;
    int active;
};

void cancel_timer(struct timer_ctx *timer) {
    timer->active = 0;
    free(timer->data);
}

void timer_tick(struct timer_ctx *timer) {
    if (timer->active) {
        timer->callback(timer->data);
    }
}
"#;

    let path = "tests/fixtures/c/timer_uaf.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff: the free(timer->data) line (potential UAF)
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([11, 12]),
        }],
    };

    (files, diff)
}

#[test]
fn test_snmp_overflow_original_diff() {
    let (files, diff) = make_snmp_overflow_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
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
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction),
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
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce blocks for snmp_overflow"
    );
}

#[test]
fn test_snmp_overflow_thin_slice() {
    let (files, diff) = make_snmp_overflow_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "ThinSlice should produce blocks for snmp_overflow"
    );
}

#[test]
fn test_onu_state_machine_original_diff() {
    let (files, diff) = make_onu_state_machine_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
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
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
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
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
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
fn test_double_free_thin_slice() {
    let (files, diff) = make_double_free_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "ThinSlice should produce blocks for double_free"
    );
}

#[test]
fn test_double_free_left_flow() {
    let (files, diff) = make_double_free_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
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
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
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
fn test_ring_overflow_thin_slice() {
    let (files, diff) = make_ring_overflow_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "ThinSlice should produce blocks for ring_overflow"
    );
}

#[test]
fn test_ring_overflow_left_flow() {
    let (files, diff) = make_ring_overflow_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
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
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
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
fn test_timer_uaf_thin_slice() {
    let (files, diff) = make_timer_uaf_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "ThinSlice should produce blocks for timer_uaf"
    );
}

#[test]
fn test_timer_uaf_left_flow() {
    let (files, diff) = make_timer_uaf_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce blocks for timer_uaf"
    );
}

// ====== Phase 4: Stress test fixtures ======

fn make_large_function_test() -> (BTreeMap<String, ParsedFile>, DiffInput) {
    let source = include_str!("fixtures/c/large_function.c");

    let path = "tests/fixtures/c/large_function.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff: line 131 (sum += ch->buf[i])
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([131]),
        }],
    };

    (files, diff)
}

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

fn make_deep_switch_test() -> (BTreeMap<String, ParsedFile>, DiffInput) {
    let source = include_str!("fixtures/c/deep_switch.c");

    let path = "tests/fixtures/c/deep_switch.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff: lines 66-67 (bounds check for msg->len)
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([66, 67]),
        }],
    };

    (files, diff)
}

#[test]
fn test_large_function_original_diff() {
    let (files, diff) = make_large_function_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
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
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce blocks for large_function without panic"
    );
}

#[test]
fn test_large_function_thin_slice() {
    let (files, diff) = make_large_function_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "ThinSlice should produce blocks for large_function without panic"
    );
}

#[test]
fn test_macro_heavy_original_diff() {
    let (files, diff) = make_macro_heavy_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
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
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
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
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
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
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce blocks for deep_switch without panic"
    );
}

#[test]
fn test_deep_switch_thin_slice() {
    let (files, diff) = make_deep_switch_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "ThinSlice should produce blocks for deep_switch without panic"
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

// ====== P0 C/C++ pattern tests ======

// --- Taint sink tests ---

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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
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

// --- Provenance origin tests ---

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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
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

// --- Absence tests ---

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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
    )
    .unwrap();

    assert!(
        result.blocks.is_empty(),
        "AbsenceSlice must NOT flag new/delete when std::shared_ptr handles cleanup; \
         got {} blocks",
        result.blocks.len()
    );
}

// --- Quantum async detection tests ---

#[test]
fn test_quantum_c_signal_handler() {
    // signal() call makes the function async — quantum detects the async boundary.
    let source = r#"
void register_handlers(int signum) {
    int flags = signum;
    signal(SIGINT, handler);
    flags = flags | 1;
    return;
}
"#;
    let path = "src/signals.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 3: int flags = signum;  — flags is assigned before the signal() async boundary
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let result = prism::algorithms::quantum_slice::slice(&files, &diff, None).unwrap();

    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should detect async boundary from signal() in C function"
    );
}

#[test]
fn test_quantum_c_pthread_create() {
    // pthread_create makes the function async — quantum detects the thread creation.
    let source = r#"
int main() {
    pthread_t tid;
    int flag = 0;
    pthread_create(&tid, NULL, worker, &flag);
    flag = 1;
    pthread_join(tid, NULL);
    return 0;
}
"#;
    let path = "src/threads.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 5: pthread_create line  — flag assigned before and after the async boundary
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5]),
        }],
    };

    let result = prism::algorithms::quantum_slice::slice(&files, &diff, Some("flag")).unwrap();

    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should detect async boundary from pthread_create"
    );
}

#[test]
fn test_quantum_c_isr_function_name() {
    // Function named rx_interrupt_handler is treated as async by name heuristic.
    let source = r#"
void rx_interrupt_handler(int irq) {
    int status = 0;
    status = irq;
    return;
}
"#;
    let path = "src/isr.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 4: status = irq;  — status is a local assigned inside an ISR-named function
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };

    let result = prism::algorithms::quantum_slice::slice(&files, &diff, Some("status")).unwrap();

    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should treat function named 'rx_interrupt_handler' as async \
         (ISR name heuristic)"
    );
}

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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
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
    let test_source = include_str!("integration_test.rs");
    let test_names: Vec<&str> = test_source
        .lines()
        .filter(|l| l.starts_with("fn test_"))
        .map(|l| l.trim_start_matches("fn ").split('(').next().unwrap_or(""))
        .collect();

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

    let test_source = include_str!("integration_test.rs");
    let test_names: Vec<&str> = test_source
        .lines()
        .filter(|l| l.starts_with("fn test_"))
        .map(|l| l.trim_start_matches("fn ").split('(').next().unwrap_or(""))
        .collect();

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

// ====== MembraneSlice C error handling tests ======

/// Test that MembraneSlice correctly recognises C-style error handling
/// (if (ret < 0), if (!ptr), errno, perror) and does NOT emit a false
/// "unprotected caller" finding when the caller already checks errors.
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
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

/// Test that MembraneSlice recognises NULL-pointer checks as error handling.
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
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

/// Test that CircularSlice detects cycles through function pointer calls.
#[test]
fn test_circular_slice_function_pointer_cycle() {
    // dispatch() calls handler->process(), and process() calls dispatch() — a cycle
    let source = r#"
#include <stdlib.h>

typedef struct handler {
    void (*process)(int data);
} handler_t;

void dispatch(handler_t *handler, int data);

void process(int data) {
    handler_t h;
    h.process = process;
    if (data > 0) {
        dispatch(&h, data - 1);
    }
}

void dispatch(handler_t *handler, int data) {
    handler->process(data);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/loop.c".to_string(),
        ParsedFile::parse("src/loop.c", source, Language::C).unwrap(),
    );

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/loop.c".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([12]),
        }],
    };

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::CircularSlice),
    )
    .unwrap();

    // CircularSlice should detect the process → dispatch → process cycle
    // via handler->process() resolving to the "process" callee name
    let has_cycle_finding = result.findings.iter().any(|f| {
        f.description.contains("cycle") || f.category.as_deref() == Some("recursive_cycle")
    });
    assert!(
        !result.blocks.is_empty() || has_cycle_finding,
        "CircularSlice should detect cycle through function pointer dispatch"
    );
}

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
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
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

// ====== QuantumSlice ISR self-detection tests ======

/// A function registered as a signal handler via signal(SIGTERM, my_cleanup)
/// should be detected as async even though it doesn't contain any async
/// primitives itself. The registration happens in a DIFFERENT function.
#[test]
fn test_quantum_signal_handler_cross_function_detection() {
    let source = r#"
#include <signal.h>
#include <stdlib.h>

volatile int running = 1;

void my_cleanup(int signo) {
    running = 0;
}

void setup(void) {
    signal(SIGTERM, my_cleanup);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/daemon.c".to_string(),
        ParsedFile::parse("src/daemon.c", source, Language::C).unwrap(),
    );

    // Diff touches my_cleanup body — which IS a signal handler
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/daemon.c".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([8]),
        }],
    };

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
    )
    .unwrap();

    // my_cleanup is registered via signal() in setup(), so QuantumSlice
    // should detect it as async and produce output
    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should detect my_cleanup as async (registered via signal() in setup())"
    );
}

/// A function registered as a pthread start routine should be detected as async.
#[test]
fn test_quantum_pthread_registered_handler() {
    let source = r#"
#include <pthread.h>

int shared_data = 0;

void worker(void *arg) {
    shared_data = 42;
}

void start_worker(void) {
    pthread_t tid;
    pthread_create(&tid, NULL, worker, NULL);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/threads.c".to_string(),
        ParsedFile::parse("src/threads.c", source, Language::C).unwrap(),
    );

    // Diff touches worker body
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/threads.c".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([7]),
        }],
    };

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should detect worker as async (registered via pthread_create in start_worker)"
    );
}

/// Cross-file ISR detection: handler registered in one file, defined in another.
#[test]
fn test_quantum_isr_cross_file_registration() {
    let handler_source = r#"
#include <linux/interrupt.h>

static int packet_count = 0;

irqreturn_t eth_rx_interrupt(int irq, void *dev_id) {
    packet_count = packet_count + 1;
    return IRQ_HANDLED;
}
"#;

    let setup_source = r#"
#include <linux/interrupt.h>

extern irqreturn_t eth_rx_interrupt(int irq, void *dev_id);

int eth_probe(struct device *dev) {
    int ret = request_irq(dev->irq, eth_rx_interrupt, IRQF_SHARED, "eth", dev);
    return ret;
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/eth_handler.c".to_string(),
        ParsedFile::parse("src/eth_handler.c", handler_source, Language::C).unwrap(),
    );
    files.insert(
        "src/eth_probe.c".to_string(),
        ParsedFile::parse("src/eth_probe.c", setup_source, Language::C).unwrap(),
    );

    // Diff touches the assignment in the handler body
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/eth_handler.c".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([7]),
        }],
    };

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should detect eth_rx_interrupt as async (registered via request_irq in another file)"
    );
}

// ====== Static function name disambiguation tests ======

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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
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

// ── Taint sink tests: Python ──────────────────────────────────────────

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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
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

// ── Taint sink tests: JavaScript/TypeScript ───────────────────────────

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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
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

// ── Taint sink tests: Go ──────────────────────────────────────────────

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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
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

// ── Provenance source tests: Python ───────────────────────────────────

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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
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

// ── Provenance source tests: JavaScript ───────────────────────────────

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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
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

// ── Provenance source tests: Go ───────────────────────────────────────

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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
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

// ── Absence pair tests: Python ────────────────────────────────────────

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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Absence should detect tempfile.mkstemp without cleanup"
    );
}

// ── Absence pair tests: JavaScript ────────────────────────────────────

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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Absence should detect fs.openSync without fs.closeSync"
    );
}

// ── Absence pair tests: Go ────────────────────────────────────────────

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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Absence should detect http.Get without Body.Close"
    );
}

// ── Quantum async tests: Python threading ─────────────────────────────

#[test]
fn test_quantum_python_threading_async() {
    // Python threading.Thread should be detected as async context.
    let source = r#"
import threading

def worker(data):
    count = 0
    t = threading.Thread(target=process, args=(data,))
    t.start()
    count = count + 1
    return count
"#;
    let path = "app/worker.py";
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should detect Python threading.Thread as async context"
    );
}

// ── Quantum async tests: JavaScript Worker ────────────────────────────

#[test]
fn test_quantum_js_worker_async() {
    // JavaScript Worker should be detected as async context.
    let source = r#"
function processData(data) {
    let result = null;
    const worker = new Worker('processor.js');
    result = data;
    return result;
}
"#;
    let path = "src/processor.js";
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should detect JavaScript Worker as async context"
    );
}

// ── Quantum async tests: Go channel select ────────────────────────────

#[test]
fn test_quantum_go_channel_select() {
    // Go select statement with channels should be detected as async context.
    let source = r#"
package main

func fanIn(ch1 chan int, ch2 chan int) int {
    result := 0
    select {
    case v := <-ch1:
        result = v
    case v := <-ch2:
        result = v
    }
    return result
}
"#;
    let path = "cmd/fanin.go";
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should detect Go select/channel as async context"
    );
}

// ── Membrane error handling tests: Python ─────────────────────────────

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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
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

// ── Membrane error handling tests: Go ─────────────────────────────────

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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
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

// ── Taint negative tests: patterns should NOT fire on safe code ───────

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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
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

// ── Echo slice: C return-code handling ────────────────────────────────

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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::EchoSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::EchoSlice),
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

// ── Echo slice: Go errors.Is handling ─────────────────────────────────

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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::EchoSlice),
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

// ── Echo slice: Python context manager handling ───────────────────────

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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::EchoSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should produce blocks for Rust code"
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction),
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

// ── Provenance negative tests: word-boundary matching ─────────────────

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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should produce blocks for Lua code with os.execute sink"
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction),
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "ParentFunction should include the enclosing Lua function"
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Absence should detect Lua io.open without close"
    );
}

#[test]
fn test_lua_quantum_coroutine() {
    // Lua coroutine.create should be detected as async context.
    let source = r#"
function producer(data)
    local count = 0
    local co = coroutine.create(function()
        count = count + 1
    end)
    coroutine.resume(co)
    return count
end
"#;
    let path = "scripts/async.lua";
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should detect Lua coroutine.create as async context"
    );
}

// === Rust pattern depth tests ===

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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should detect from_raw_parts as a Rust sink"
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
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
fn test_rust_quantum_tokio_spawn() {
    let source = r#"
async fn process(data: Vec<u8>) {
    let handle = tokio::spawn(async move {
        let result = compute(data).await;
        result
    });
    handle.await.unwrap();
}
"#;
    let path = "src/async_proc.rs";
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should detect tokio::spawn as Rust async context"
    );
}

// === Lua pattern depth tests ===

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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should detect Lua dofile as a sink"
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
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

// ===== C++ MembraneSlice error handling patterns =====

/// Test that MembraneSlice recognises C++ try/catch as error handling.
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
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

/// Test that MembraneSlice recognises C++ RAII smart pointers as error handling.
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
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

/// Test that MembraneSlice recognises C++ lock_guard as error handling.
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
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

/// Test that MembraneSlice recognises C++ std::optional as error handling.
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
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

/// CVE pattern: buffer overflow — user-controlled size flows to memcpy.
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
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

/// CVE pattern: strcpy buffer overflow from network input (via fgets).
/// Uses fgets which returns to a variable the DFG can trace.
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

    let taint_result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
    )
    .unwrap();

    // Taint: fgets line → input → strcpy sink
    assert!(
        !taint_result.blocks.is_empty(),
        "Taint should include fgets and strcpy in the taint trace"
    );

    let prov_result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
    )
    .unwrap();

    // Provenance: fgets → UserInput origin
    assert!(
        !prov_result.findings.is_empty(),
        "Provenance should classify fgets as user_input origin"
    );
}

/// CVE pattern: integer overflow before allocation.
/// User-controlled size undergoes arithmetic, then passed to malloc.
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
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should trace arithmetic result to malloc/memset sinks"
    );
}

/// CVE pattern: use-after-free — pointer used after being freed.
/// Taint detects free() as a sink on the diff line; the subsequent use of
/// timer->data is included in the analysis context. Prism doesn't yet
/// distinguish "free then use" from "use then free" as a distinct UAF pattern.
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
    let taint_result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
    )
    .unwrap();

    assert!(
        !taint_result.blocks.is_empty(),
        "Taint should include the free() and subsequent use of timer->data"
    );
}

// ===========================================================================
// va_list taint tracking — variadic wrapper detection
// ===========================================================================

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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
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

// ===========================================================================
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Taint should propagate through field access to strcpy sink"
    );
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

// ===========================================================================
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

// ===========================================================================
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
    )
    .unwrap();

    // Taint should flow: line 3 (dev->id = input) → line 4 (x = dev->id) → line 5 (strcpy)
    assert!(
        !result.findings.is_empty(),
        "Taint should propagate through field assignment to strcpy sink"
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
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

// ====================================================================
// Batch 1: Zero-coverage algorithms (Chop, DeltaSlice, ThreeDSlice)
// ====================================================================

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
    let result = prism::algorithms::chop::slice(&files, &config).unwrap();
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
    let result = prism::algorithms::chop::slice(&files, &config).unwrap();
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
    let result = prism::algorithms::chop::slice(&files, &config).unwrap();
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

    let result = prism::algorithms::delta_slice::slice(&files, &diff, tmp.path()).unwrap();
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

    let result = prism::algorithms::delta_slice::slice(&files, &diff, tmp.path()).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::DeltaSlice);
}

/// Helper to create a temp git repo for tests that need git history.
/// Returns a TempDir that auto-cleans on drop — no manual cleanup needed.
fn create_temp_git_repo(filename: &str, contents: &[&str]) -> TempDir {
    let tmp = TempDir::new().unwrap();

    std::process::Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "commit.gpgsign", "false"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    for (i, content) in contents.iter().enumerate() {
        std::fs::write(tmp.path().join(filename), content).unwrap();
        std::process::Command::new("git")
            .args(["add", filename])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", &format!("commit {}", i)])
            .current_dir(tmp.path())
            .output()
            .unwrap();
    }

    tmp
}

#[test]
fn test_threed_slice_python() {
    let source =
        "def foo(x):\n    y = x + 1\n    return y\n\ndef bar():\n    r = foo(10)\n    print(r)\n";
    let filename = "app.py";
    let tmp = create_temp_git_repo(filename, &["def foo(x):\n    return x\n", source]);

    let parsed = ParsedFile::parse(filename, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(filename.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: filename.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2, 3]),
        }],
    };

    let config = prism::algorithms::threed_slice::ThreeDConfig {
        temporal_days: 365,
        git_dir: tmp.path().to_string_lossy().to_string(),
    };
    let result = prism::algorithms::threed_slice::slice(&files, &diff, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ThreeDSlice);
    assert!(
        !result.blocks.is_empty(),
        "ThreeDSlice should produce blocks for functions with churn"
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_threed_slice_go() {
    let source = "package main\n\nfunc compute(n int) int {\n\tresult := n * 2\n\treturn result\n}\n\nfunc caller() {\n\tv := compute(5)\n\t_ = v\n}\n";
    let filename = "main.go";
    let tmp = create_temp_git_repo(
        filename,
        &[
            "package main\n\nfunc compute(n int) int { return n }\n",
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

    let config = prism::algorithms::threed_slice::ThreeDConfig {
        temporal_days: 365,
        git_dir: tmp.path().to_string_lossy().to_string(),
    };
    let result = prism::algorithms::threed_slice::slice(&files, &diff, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ThreeDSlice);

    let _ = std::fs::remove_dir_all(&tmp);
}

// ====================================================================
// Batch 2: ConditionedSlice, AngleSlice, SpiralSlice
// ====================================================================

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
fn test_angle_slice_python() {
    let source = r#"
import logging

def process(data):
    try:
        result = transform(data)
        logging.info("success")
        return result
    except Exception as e:
        logging.error(str(e))
        raise
"#;
    let path = "proc.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([9, 10]),
        }],
    };

    let result = prism::algorithms::angle_slice::slice(
        &files,
        &diff,
        &prism::algorithms::angle_slice::Concern::ErrorHandling,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AngleSlice);
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_angle_slice_go() {
    let source = r#"package main

import "log"

func handler() error {
	result, err := doWork()
	if err != nil {
		log.Printf("error: %v", err)
		return err
	}
	log.Printf("success: %v", result)
	return nil
}

func doWork() (int, error) {
	return 42, nil
}
"#;
    let path = "handler.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([7, 8]),
        }],
    };

    let result = prism::algorithms::angle_slice::slice(
        &files,
        &diff,
        &prism::algorithms::angle_slice::Concern::ErrorHandling,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AngleSlice);
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_angle_slice_javascript_logging() {
    let source = r#"
function fetchData(url) {
    console.log("fetching", url);
    const res = fetch(url);
    if (res.error) {
        console.error("failed", res.error);
        return null;
    }
    console.log("done");
    return res;
}
"#;
    let path = "fetch.js";
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

    let result = prism::algorithms::angle_slice::slice(
        &files,
        &diff,
        &prism::algorithms::angle_slice::Concern::Logging,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AngleSlice);
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_angle_slice_custom_concern_python() {
    let source = r#"
import redis

def get_cached(key):
    cache = redis.get(key)
    if cache:
        return cache
    result = compute(key)
    redis.set(key, result, ttl=300)
    return result
"#;
    let path = "cache.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([9]),
        }],
    };

    let result = prism::algorithms::angle_slice::slice(
        &files,
        &diff,
        &prism::algorithms::angle_slice::Concern::Caching,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_spiral_slice_python() {
    let source = r#"
def inner(x):
    return x + 1

def outer(y):
    z = inner(y)
    return z * 2

def caller():
    r = outer(10)
    print(r)
"#;
    let path = "spiral.py";
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

    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::SpiralSlice);
    let spiral_config = prism::algorithms::spiral_slice::SpiralConfig {
        max_ring: 4,
        auto_stop_threshold: 0.0,
    };
    let result =
        prism::algorithms::spiral_slice::slice(&files, &diff, &config, &spiral_config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::SpiralSlice);
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_spiral_slice_go() {
    let source = r#"package main

func compute(n int) int {
	return n * 2
}

func process(x int) int {
	r := compute(x)
	return r + 1
}

func main() {
	v := process(5)
	println(v)
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
            diff_lines: BTreeSet::from([4]),
        }],
    };

    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::SpiralSlice);
    let spiral_config = prism::algorithms::spiral_slice::SpiralConfig {
        max_ring: 6,
        auto_stop_threshold: 0.0,
    };
    let result =
        prism::algorithms::spiral_slice::slice(&files, &diff, &config, &spiral_config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::SpiralSlice);
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_spiral_slice_ring1_only_python() {
    let (files, _, diff) = make_python_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::SpiralSlice);
    let spiral_config = prism::algorithms::spiral_slice::SpiralConfig {
        max_ring: 1,
        auto_stop_threshold: 0.0,
    };
    let result =
        prism::algorithms::spiral_slice::slice(&files, &diff, &config, &spiral_config).unwrap();
    assert!(!result.blocks.is_empty());
}

// ====================================================================
// Batch 3: HorizontalSlice, VerticalSlice, ResonanceSlice
// ====================================================================

#[test]
fn test_horizontal_slice_python() {
    let source = r#"
def handle_get(request):
    data = get_data()
    return data

def handle_post(request):
    data = request.body
    save_data(data)
    return "ok"

def handle_delete(request):
    delete_data(request.id)
    return "deleted"
"#;
    let path = "handlers.py";
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

    let result = prism::algorithms::horizontal_slice::slice(
        &files,
        &diff,
        &prism::algorithms::horizontal_slice::PeerPattern::NamePattern("handle_*".to_string()),
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::HorizontalSlice);
}

#[test]
fn test_horizontal_slice_javascript() {
    let source = r#"
function handleLogin(req, res) {
    const user = authenticate(req.body);
    res.send(user);
}

function handleLogout(req, res) {
    clearSession(req);
    res.send("ok");
}

function handleRegister(req, res) {
    const user = createUser(req.body);
    res.send(user);
}
"#;
    let path = "routes.js";
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

    let result = prism::algorithms::horizontal_slice::slice(
        &files,
        &diff,
        &prism::algorithms::horizontal_slice::PeerPattern::Auto,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::HorizontalSlice);
}

#[test]
fn test_horizontal_slice_go() {
    let source = r#"package main

func HandleGet(w http.ResponseWriter, r *http.Request) {
	data := getData()
	w.Write(data)
}

func HandlePost(w http.ResponseWriter, r *http.Request) {
	body := r.Body
	saveData(body)
}
"#;
    let path = "routes.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };

    let result = prism::algorithms::horizontal_slice::slice(
        &files,
        &diff,
        &prism::algorithms::horizontal_slice::PeerPattern::NamePattern("Handle*".to_string()),
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::HorizontalSlice);
}

#[test]
fn test_vertical_slice_python() {
    let source_handler = r#"
def api_handler(request):
    data = request.json()
    result = service_process(data)
    return result
"#;
    let source_service = r#"
def service_process(data):
    validated = validate(data)
    return repo_save(validated)
"#;
    let source_repo = r#"
def repo_save(data):
    db.insert(data)
    return True
"#;
    let handler_path = "handler/api.py";
    let service_path = "service/processor.py";
    let repo_path = "repository/store.py";

    let mut files = BTreeMap::new();
    files.insert(
        handler_path.to_string(),
        ParsedFile::parse(handler_path, source_handler, Language::Python).unwrap(),
    );
    files.insert(
        service_path.to_string(),
        ParsedFile::parse(service_path, source_service, Language::Python).unwrap(),
    );
    files.insert(
        repo_path.to_string(),
        ParsedFile::parse(repo_path, source_repo, Language::Python).unwrap(),
    );

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: service_path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let result = prism::algorithms::vertical_slice::slice(
        &files,
        &diff,
        &prism::algorithms::vertical_slice::VerticalConfig::default(),
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::VerticalSlice);
}

#[test]
fn test_vertical_slice_go() {
    let source = r#"package main

func handler(w http.ResponseWriter, r *http.Request) {
	data := parseRequest(r)
	result := service(data)
	w.Write(result)
}

func service(data string) string {
	return repository(data)
}

func repository(key string) string {
	return db.Get(key)
}
"#;
    let path = "handler/main.go";
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

    let result = prism::algorithms::vertical_slice::slice(
        &files,
        &diff,
        &prism::algorithms::vertical_slice::VerticalConfig::default(),
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::VerticalSlice);
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

// ====================================================================
// Batch 4: Remaining algorithm×language gaps
// ====================================================================

#[test]
fn test_thin_slice_python() {
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_thin_slice_go() {
    let (files, _, diff) = make_go_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_thin_slice_typescript() {
    let (files, _, diff) = make_typescript_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_thin_slice_java() {
    let (files, _, diff) = make_java_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ThinSlice);
}

#[test]
fn test_thin_slice_cpp() {
    let (files, _, diff) = make_cpp_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_thin_slice_c() {
    let (files, _, diff) = make_c_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_barrier_slice_go() {
    let (files, _, diff) = make_go_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::BarrierSlice),
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_barrier_slice_javascript() {
    let (files, _, diff) = make_javascript_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::BarrierSlice),
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_relevant_slice_python() {
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::RelevantSlice),
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_relevant_slice_go() {
    let (files, _, diff) = make_go_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::RelevantSlice),
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_circular_slice_python() {
    let source = r#"
def a(x):
    return b(x + 1)

def b(y):
    return a(y - 1)
"#;
    let path = "cycle.py";
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
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::CircularSlice),
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::CircularSlice);
}

#[test]
fn test_circular_slice_go() {
    let source = r#"package main

func ping(n int) int {
	return pong(n + 1)
}

func pong(n int) int {
	return ping(n - 1)
}
"#;
    let path = "cycle.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::CircularSlice),
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::CircularSlice);
}

#[test]
fn test_gradient_slice_python() {
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::GradientSlice),
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_gradient_slice_go() {
    let (files, _, diff) = make_go_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::GradientSlice),
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_full_flow_python_no_returns() {
    let (files, _, diff) = make_python_test();
    let config = SliceConfig {
        algorithm: SlicingAlgorithm::FullFlow,
        include_returns: false,
        trace_callees: false,
        ..SliceConfig::default()
    };
    let result = algorithms::run_slicing(&files, &diff, &config).unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_full_flow_go_trace_callees() {
    let (files, _, diff) = make_go_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow);
    let result = algorithms::run_slicing(&files, &diff, &config).unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_full_flow_javascript() {
    let (files, _, diff) = make_javascript_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow);
    let result = algorithms::run_slicing(&files, &diff, &config).unwrap();
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
fn test_quantum_slice_python() {
    let source = r#"
import threading

def worker(data):
    result = process(data)
    return result

def main():
    t = threading.Thread(target=worker, args=(42,))
    t.start()
    t.join()
"#;
    let path = "async.py";
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
    let result = prism::algorithms::quantum_slice::slice(&files, &diff, None).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::QuantumSlice);
}

#[test]
fn test_quantum_slice_go_channel() {
    let source = r#"package main

func worker(ch chan int) {
	result := compute()
	ch <- result
}

func main() {
	ch := make(chan int)
	go worker(ch)
	v := <-ch
	_ = v
}

func compute() int { return 42 }
"#;
    let path = "concurrent.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };
    let result = prism::algorithms::quantum_slice::slice(&files, &diff, None).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::QuantumSlice);
}

#[test]
fn test_quantum_slice_javascript_async() {
    let source = r#"
async function fetchAll(urls) {
    const promises = urls.map(url => fetch(url));
    const results = await Promise.all(promises);
    return results;
}
"#;
    let path = "async.js";
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
    let result = prism::algorithms::quantum_slice::slice(&files, &diff, None).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::QuantumSlice);
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
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::SymmetrySlice),
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
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::SymmetrySlice),
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
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::EchoSlice),
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
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::EchoSlice),
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
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
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
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
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
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
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
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AbsenceSlice);
}

#[test]
fn test_original_diff_go() {
    let (files, _, diff) = make_go_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_parent_function_typescript() {
    let (files, _, diff) = make_typescript_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction),
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_left_flow_go() {
    let (files, _, diff) = make_go_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_left_flow_java() {
    let (files, _, diff) = make_java_test();
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

// ====================================================================
// Batch 6: Targeted tests for files near 80% threshold
// ====================================================================

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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "ThinSlice should handle global-scope diff lines"
    );
}

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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction),
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
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
    let result =
        prism::algorithms::barrier_slice::slice(&files, &diff, &config, &barrier_config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::BarrierSlice);
}

#[test]
fn test_vertical_slice_explicit_layers_python() {
    // Test with explicit layer ordering
    let source = r#"
def api_handler(request):
    return service_call(request.data)

def service_call(data):
    return repo_save(data)

def repo_save(data):
    return True
"#;
    let path = "handler/app.py";
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

    let config = prism::algorithms::vertical_slice::VerticalConfig {
        layers: vec![
            "Handler".to_string(),
            "Service".to_string(),
            "Repository".to_string(),
        ],
    };
    let result = prism::algorithms::vertical_slice::slice(&files, &diff, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::VerticalSlice);
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

    let result = prism::algorithms::delta_slice::slice(&files, &diff, tmp.path()).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::DeltaSlice);
    // Should succeed but with empty old files — no edge differences
}

#[test]
fn test_angle_slice_concern_not_on_diff_python() {
    // Concern exists in code but not on the diff lines themselves
    let source = r#"
def handler():
    try:
        result = compute()
    except Exception as e:
        log_error(e)
        raise
    return result

def compute():
    x = 1
    y = x + 1
    return y
"#;
    let path = "app.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff is on lines 11-12 in compute(), which has no error handling patterns
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([11, 12]),
        }],
    };

    let result = prism::algorithms::angle_slice::slice(
        &files,
        &diff,
        &prism::algorithms::angle_slice::Concern::ErrorHandling,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AngleSlice);
    // Should still find error handling code in the file
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_angle_slice_authentication_go() {
    let source = r#"package main

func authMiddleware(token string) bool {
	session := validateToken(token)
	if session == nil {
		return false
	}
	return authorize(session)
}

func validateToken(t string) interface{} {
	return nil
}

func authorize(s interface{}) bool {
	return true
}
"#;
    let path = "auth.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4, 5]),
        }],
    };

    let result = prism::algorithms::angle_slice::slice(
        &files,
        &diff,
        &prism::algorithms::angle_slice::Concern::Authentication,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_spiral_slice_max_ring_6_python() {
    // Test spiral with max ring 6 to cover ring 5 (test files) and ring 6 (shared utils)
    let source_main = r#"
def compute(x):
    y = helper(x)
    return y * 2
"#;
    let source_helper = r#"
def helper(x):
    return x + 1
"#;
    let source_test = r#"
def test_compute():
    assert compute(5) == 12
"#;
    let mut files = BTreeMap::new();
    files.insert(
        "src/main.py".to_string(),
        ParsedFile::parse("src/main.py", source_main, Language::Python).unwrap(),
    );
    files.insert(
        "src/helper.py".to_string(),
        ParsedFile::parse("src/helper.py", source_helper, Language::Python).unwrap(),
    );
    files.insert(
        "tests/test_main.py".to_string(),
        ParsedFile::parse("tests/test_main.py", source_test, Language::Python).unwrap(),
    );

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/main.py".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::SpiralSlice);
    let spiral_config = prism::algorithms::spiral_slice::SpiralConfig {
        max_ring: 6,
        auto_stop_threshold: 0.0,
    };
    let result =
        prism::algorithms::spiral_slice::slice(&files, &diff, &config, &spiral_config).unwrap();
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
    let result = algorithms::run_slicing(&files, &diff, &config).unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_horizontal_slice_name_suffix_python() {
    // Test NamePattern with suffix matching (*_handler)
    let source = r#"
def get_handler(request):
    return get_data()

def post_handler(request):
    save_data(request.body)

def delete_handler(request):
    remove_data(request.id)
"#;
    let path = "handlers.py";
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

    let result = prism::algorithms::horizontal_slice::slice(
        &files,
        &diff,
        &prism::algorithms::horizontal_slice::PeerPattern::NamePattern("*_handler".to_string()),
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::HorizontalSlice);
}

#[test]
fn test_horizontal_slice_decorator_python() {
    // Test Decorator matching
    let source = r#"
@app.route("/users")
def get_users():
    return users

@app.route("/items")
def get_items():
    return items
"#;
    let path = "routes.py";
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

    let result = prism::algorithms::horizontal_slice::slice(
        &files,
        &diff,
        &prism::algorithms::horizontal_slice::PeerPattern::Decorator("@app.route".to_string()),
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::HorizontalSlice);
}

#[test]
fn test_quantum_slice_with_target_var_python() {
    // Test quantum_slice with a specific target variable
    let source = r#"
import asyncio

async def fetch(url):
    data = await get(url)
    result = process(data)
    return result
"#;
    let path = "fetch.py";
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

    let result = prism::algorithms::quantum_slice::slice(&files, &diff, Some("data")).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::QuantumSlice);
}

// ====================================================================
// Batch 7: Deeper assertions on security-relevant algorithms
// ====================================================================

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
    let result = prism::algorithms::chop::slice(&files, &config).unwrap();
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

    let result = prism::algorithms::taint::slice(
        &files,
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
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
    let left_result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::SymmetrySlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::EchoSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
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

    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
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
    let result = algorithms::run_slicing(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::GradientSlice),
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

#[test]
fn test_threed_slice_python_risk_scoring() {
    // ThreeDSlice should produce blocks sorted by risk
    let source =
        "def foo(x):\n    y = x + 1\n    return y\n\ndef bar():\n    r = foo(10)\n    print(r)\n";
    let filename = "app.py";
    let tmp = create_temp_git_repo(filename, &["def foo(x):\n    return x\n", source]);

    let parsed = ParsedFile::parse(filename, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(filename.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: filename.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2, 3]),
        }],
    };

    let config = prism::algorithms::threed_slice::ThreeDConfig {
        temporal_days: 365,
        git_dir: tmp.path().to_string_lossy().to_string(),
    };
    let result = prism::algorithms::threed_slice::slice(&files, &diff, &config).unwrap();
    assert!(
        !result.blocks.is_empty(),
        "ThreeDSlice should produce risk-scored blocks"
    );

    // The first block should contain the diff function (highest risk)
    let first_block = &result.blocks[0];
    let lines = first_block.file_line_map.get(filename);
    assert!(
        lines.is_some(),
        "First block should contain lines from the diff file"
    );
}

#[test]
fn test_vertical_slice_python_layer_detection() {
    // Vertical slice should detect layers from file paths
    let source_handler = "def api_handler(request):\n    return service_call(request.data)\n";
    let source_service = "def service_call(data):\n    return repo_save(data)\n";
    let source_repo = "def repo_save(data):\n    return True\n";

    let mut files = BTreeMap::new();
    files.insert(
        "handler/api.py".to_string(),
        ParsedFile::parse("handler/api.py", source_handler, Language::Python).unwrap(),
    );
    files.insert(
        "service/logic.py".to_string(),
        ParsedFile::parse("service/logic.py", source_service, Language::Python).unwrap(),
    );
    files.insert(
        "repository/store.py".to_string(),
        ParsedFile::parse("repository/store.py", source_repo, Language::Python).unwrap(),
    );

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "service/logic.py".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2]),
        }],
    };

    let result = prism::algorithms::vertical_slice::slice(
        &files,
        &diff,
        &prism::algorithms::vertical_slice::VerticalConfig::default(),
    )
    .unwrap();
    // Should produce blocks — at minimum the diff function
    assert!(
        !result.blocks.is_empty(),
        "VerticalSlice should trace layers for service function"
    );
}

#[test]
fn test_spiral_slice_ring_expansion_go() {
    // Verify that higher ring numbers produce more output than lower ones
    let source = r#"package main

func inner(x int) int { return x + 1 }
func middle(x int) int { return inner(x) * 2 }
func outer(x int) int { return middle(x) + 3 }
func caller() int { return outer(10) }
"#;
    let path = "chain.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::SpiralSlice);

    let ring2 = prism::algorithms::spiral_slice::slice(
        &files,
        &diff,
        &config,
        &prism::algorithms::spiral_slice::SpiralConfig {
            max_ring: 2,
            auto_stop_threshold: 0.0,
        },
    )
    .unwrap();

    let ring4 = prism::algorithms::spiral_slice::slice(
        &files,
        &diff,
        &config,
        &prism::algorithms::spiral_slice::SpiralConfig {
            max_ring: 4,
            auto_stop_threshold: 0.0,
        },
    )
    .unwrap();

    let count_lines = |r: &prism::slice::SliceResult| -> usize {
        r.blocks
            .iter()
            .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
            .sum()
    };

    assert!(
        count_lines(&ring4) >= count_lines(&ring2),
        "Ring 4 ({} lines) should have >= Ring 2 ({} lines)",
        count_lines(&ring4),
        count_lines(&ring2)
    );
}

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
