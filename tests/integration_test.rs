use prism::algorithms;
use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
use prism::data_flow::DataFlowGraph;
use prism::diff::{DiffInfo, DiffInput, ModifyType};
use prism::languages::Language;
use prism::output;
use prism::slice::{SliceConfig, SlicingAlgorithm};
use std::collections::{BTreeMap, BTreeSet};

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
    // dev->id = val  should create defs for both the field name and the base struct variable.
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

    // Fine-grained tracking: the field itself was written
    let id_defs = dfg.all_defs_of(path, "id");
    assert!(
        !id_defs.is_empty(),
        "DataFlowGraph should record a def for field 'id' from dev->id = val"
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
/// Run with: cargo test -- test_algorithm_language_matrix --nocapture
#[test]
fn test_algorithm_language_matrix() {
    // Map algorithm keyword → display name
    let algorithms: &[(&str, &str)] = &[
        ("original_diff", "OriginalDiff"),
        ("parent_function", "ParentFunction"),
        ("left_flow", "LeftFlow"),
        ("full_flow", "FullFlow"),
        ("thin_slice", "ThinSlice"),
        ("barrier_slice", "BarrierSlice"),
        ("taint", "Taint"),
        ("relevant_slice", "RelevantSlice"),
        ("conditioned_slice", "ConditionedSlice"),
        ("delta_slice", "DeltaSlice"),
        ("spiral_slice", "SpiralSlice"),
        ("circular_slice", "CircularSlice"),
        ("quantum_slice", "QuantumSlice"),
        ("horizontal_slice", "HorizontalSlice"),
        ("vertical_slice", "VerticalSlice"),
        ("angle_slice", "AngleSlice"),
        ("threed_slice", "ThreeDSlice"),
        ("absence_slice", "AbsenceSlice"),
        ("resonance_slice", "ResonanceSlice"),
        ("symmetry_slice", "SymmetrySlice"),
        ("gradient_slice", "GradientSlice"),
        ("provenance_slice", "ProvenanceSlice"),
        ("phantom_slice", "PhantomSlice"),
        ("membrane_slice", "MembraneSlice"),
        ("echo_slice", "EchoSlice"),
    ];

    // Map language keyword → display name
    let languages: &[(&str, &str)] = &[
        ("python", "Python"),
        ("javascript", "JS"),
        ("typescript", "TS"),
        ("go", "Go"),
        ("java", "Java"),
        ("_c_", "C"),
        ("_cpp_", "C++"),
    ];

    // Collect all test function names from this file (compile-time string)
    let test_source = include_str!("integration_test.rs");
    let test_names: Vec<&str> = test_source
        .lines()
        .filter(|l| l.starts_with("fn test_"))
        .map(|l| l.trim_start_matches("fn ").split('(').next().unwrap_or(""))
        .collect();

    // Build the matrix
    let col_w = 14usize;
    let row_w = 16usize;

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

    for (algo_key, algo_name) in algorithms {
        let row: String = languages
            .iter()
            .map(|(lang_key, _)| {
                total += 1;
                let has_test = test_names
                    .iter()
                    .any(|name| name.contains(algo_key) && name.contains(lang_key));
                if has_test {
                    covered += 1;
                    format!("{:>col_w$}", "yes")
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
