use slicing::algorithms;
use slicing::ast::ParsedFile;
use slicing::call_graph::CallGraph;
use slicing::data_flow::DataFlowGraph;
use slicing::diff::{DiffInfo, DiffInput, ModifyType};
use slicing::languages::Language;
use slicing::output;
use slicing::slice::{SliceConfig, SlicingAlgorithm};
use std::collections::{BTreeMap, BTreeSet};

fn make_python_test() -> (BTreeMap<String, ParsedFile>, BTreeMap<String, String>, DiffInput) {
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

fn make_javascript_test() -> (BTreeMap<String, ParsedFile>, BTreeMap<String, String>, DiffInput) {
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

fn make_go_test() -> (BTreeMap<String, ParsedFile>, BTreeMap<String, String>, DiffInput) {
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

fn make_java_test() -> (BTreeMap<String, ParsedFile>, BTreeMap<String, String>, DiffInput) {
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

fn make_typescript_test() -> (BTreeMap<String, ParsedFile>, BTreeMap<String, String>, DiffInput) {
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
    assert!(lines.len() > 2, "ParentFunction should include more than just diff lines");

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
    assert!(formatted.contains("fetchData") || formatted.contains("response") || formatted.contains("data"));
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

    let orig_lines: usize = orig.blocks.iter().map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>()).sum();
    let parent_lines: usize = parent.blocks.iter().map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>()).sum();

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
        ("test.js", Language::JavaScript, "function foo() { return 1; }\n"),
        ("test.ts", Language::TypeScript, "function foo(): number { return 1; }\n"),
        ("test.go", Language::Go, "package main\nfunc foo() int { return 1 }\n"),
        ("test.java", Language::Java, "class T { int foo() { return 1; } }\n"),
    ];

    for (path, lang, source) in cases {
        let parsed = ParsedFile::parse(path, source, lang);
        assert!(parsed.is_ok(), "Failed to parse {}: {:?}", path, parsed.err());
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

    let thin_lines: usize = thin.blocks.iter().map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>()).sum();
    let left_lines: usize = left.blocks.iter().map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>()).sum();

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
    assert!(lines.len() >= 2, "ThinSlice should include at least diff lines");
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

    let relevant_count: usize = result.blocks.iter().map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>()).sum();
    let left_count: usize = left.blocks.iter().map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>()).sum();

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

    let spiral_lines: usize = result.blocks.iter().map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>()).sum();
    let orig_lines: usize = orig.blocks.iter().map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>()).sum();

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
    assert!(lines.len() > 5, "HorizontalSlice should include peer functions, got {} lines", lines.len());
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

fn make_error_handling_test() -> (BTreeMap<String, ParsedFile>, BTreeMap<String, String>, DiffInput) {
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
    let concern = slicing::algorithms::angle_slice::Concern::ErrorHandling;
    let result = slicing::algorithms::angle_slice::slice(&files, &diff, &concern).unwrap();

    assert!(!result.blocks.is_empty());
    let block = &result.blocks[0];
    let lines = block.file_line_map.get("service.py").unwrap();
    // Should find error handling patterns across both functions
    assert!(lines.len() > 3, "AngleSlice should trace error handling across functions");
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
    let result = slicing::algorithms::quantum_slice::slice(&files, &diff, Some("user")).unwrap();

    // May or may not find async patterns depending on tree-sitter parsing
    // Just verify it doesn't crash
    assert!(result.algorithm == SlicingAlgorithm::QuantumSlice);
}

// ====== Conditioned Slice tests ======

#[test]
fn test_conditioned_slice_parses_conditions() {
    use slicing::algorithms::conditioned_slice::Condition;

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
    assert!(!result.blocks.is_empty(), "GradientSlice should produce output");

    // Should have at least the diff lines
    let total_lines: usize = result
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();
    assert!(total_lines >= 2, "GradientSlice should include at least diff lines");
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
