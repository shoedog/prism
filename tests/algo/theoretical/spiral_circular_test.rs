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

#[test]
fn circular_emits_cycle_diagram_with_bold_back_edge() {
    // Two mutually-recursive Python functions guarantee a 2-cycle in the call graph.
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
            diff_lines: BTreeSet::from([3]), // line inside `a` — the `return b(x + 1)` call
        }],
    };
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::CircularSlice);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();

    // The mutual-recursion fixture (a → b → a) guarantees CPG cycle detection.
    // If blocks are empty the fixture or algorithm has regressed.
    assert!(
        !result.blocks.is_empty(),
        "CircularSlice must detect the a→b→a mutual-recursion cycle and produce blocks"
    );

    assert!(
        !result.diagrams.is_empty(),
        "expected at least one Cycle diagram"
    );
    let g = &result.diagrams[0];
    assert!(
        matches!(g.shape, prism::slice::GraphShape::Cycle),
        "diagram shape should be Cycle, got {:?}",
        g.shape
    );
    assert!(
        g.edges
            .iter()
            .any(|e| matches!(e.style, prism::slice::EdgeStyle::Bold)),
        "expected a Bold back-edge in the cycle diagram"
    );
    assert!(
        g.mermaid.starts_with("flowchart LR"),
        "mermaid should start with 'flowchart LR', got: {:?}",
        &g.mermaid[..g.mermaid.len().min(40)]
    );
    assert!(
        g.mermaid.contains("==>|\"cycle\"|"),
        "mermaid should contain the bold back-edge '==>|\"cycle\"|'"
    );
}

#[test]
fn circular_diagram_uses_actual_graph_edges_not_invented_pairs() {
    // Reproducer from PR review:
    // a -> b, b -> a, b -> c, c -> b. SCC = {a, b, c}.
    // The actual call edges form two 2-cycles sharing b.
    // The diagram MUST NOT contain a -> c or c -> a (which don't exist in the graph).
    let source = "\
def a():
    b()

def b():
    a()
    c()

def c():
    b()
";
    let path = "rec.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2]),
        }],
    };

    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::CircularSlice);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    if result.diagrams.is_empty() {
        panic!(
            "expected at least one cycle diagram for mutual recursion fixture; \
             got blocks={} diagrams=0",
            result.blocks.len()
        );
    }

    // Aggregate all (from_fn, to_fn) pairs across all cycle diagrams.
    let mut emitted_edges: BTreeSet<(String, String)> = BTreeSet::new();
    for g in &result.diagrams {
        for e in &g.edges {
            let from_label = g
                .nodes
                .iter()
                .find(|n| n.id == e.from)
                .map(|n| n.label.clone())
                .unwrap_or_default();
            let to_label = g
                .nodes
                .iter()
                .find(|n| n.id == e.to)
                .map(|n| n.label.clone())
                .unwrap_or_default();
            // Labels look like "rec.py:N\nfn_name" for call-graph nodes.
            let from_fn = from_label.split('\n').last().unwrap_or("").to_string();
            let to_fn = to_label.split('\n').last().unwrap_or("").to_string();
            emitted_edges.insert((from_fn, to_fn));
        }
    }

    // a -> c is NEVER an actual call. MUST NOT appear.
    assert!(
        !emitted_edges.contains(&("a".to_string(), "c".to_string())),
        "diagram invented a -> c edge that doesn't exist in source. Emitted: {:?}",
        emitted_edges
    );
    // c -> a is also NEVER an actual call. MUST NOT appear.
    assert!(
        !emitted_edges.contains(&("c".to_string(), "a".to_string())),
        "diagram invented c -> a edge. Emitted: {:?}",
        emitted_edges
    );
    // Only real edges allowed: a -> b, b -> a, b -> c, c -> b.
    let allowed: BTreeSet<(String, String)> = [("a", "b"), ("b", "a"), ("b", "c"), ("c", "b")]
        .iter()
        .map(|(f, t)| (f.to_string(), t.to_string()))
        .collect();
    for edge in &emitted_edges {
        assert!(
            allowed.contains(edge),
            "diagram contains edge {:?} that isn't a real call in the source. Emitted: {:?}",
            edge,
            emitted_edges
        );
    }
}
