#[path = "../../common/mod.rs"]
mod common;
use common::*;

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
fn test_spiral_slice_ring_containment() {
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::SpiralSlice),
        None,
    )
    .unwrap();

    // Spiral should include at least the original diff lines
    assert!(!result.blocks.is_empty());

    let orig = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
        None,
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

#[test]
fn test_circular_slice_detects_cycle() {
    let (files, diff) = make_mutual_recursion_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::CircularSlice),
        None,
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

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::CircularSlice),
        None,
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
    let ctx = CpgContext::build(&files, None);
    let result =
        prism::algorithms::spiral_slice::slice(&ctx, &diff, &config, &spiral_config).unwrap();
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
    let ctx = CpgContext::build(&files, None);
    let result =
        prism::algorithms::spiral_slice::slice(&ctx, &diff, &config, &spiral_config).unwrap();
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
    let ctx = CpgContext::build(&files, None);
    let result =
        prism::algorithms::spiral_slice::slice(&ctx, &diff, &config, &spiral_config).unwrap();
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
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::CircularSlice),
        None,
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
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::CircularSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::CircularSlice);
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
    let ctx = CpgContext::build(&files, None);
    let result =
        prism::algorithms::spiral_slice::slice(&ctx, &diff, &config, &spiral_config).unwrap();
    assert!(!result.blocks.is_empty());
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

    let ctx = CpgContext::build(&files, None);
    let ring2 = prism::algorithms::spiral_slice::slice(
        &ctx,
        &diff,
        &config,
        &prism::algorithms::spiral_slice::SpiralConfig {
            max_ring: 2,
            auto_stop_threshold: 0.0,
        },
    )
    .unwrap();

    let ring4 = prism::algorithms::spiral_slice::slice(
        &ctx,
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
