#[path = "../../common/mod.rs"]
mod common;
use common::*;

// ── Basic parsing ──

#[test]
fn test_go_basic_parsing() {
    let source = r#"
package main

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
        func_names.contains(&"sum".to_string()),
        "Should find sum function, got: {:?}",
        func_names
    );
    assert!(
        func_names.contains(&"main".to_string()),
        "Should find main function, got: {:?}",
        func_names
    );
}

#[test]
fn test_go_method_declaration_parsing() {
    let source = r#"
package main

type Server struct {
    port int
    host string
}

func (s *Server) Start() error {
    addr := fmt.Sprintf("%s:%d", s.host, s.port)
    return listen(addr)
}

func (s *Server) Stop() {
    s.port = 0
}
"#;
    let path = "server.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();

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
        func_names.contains(&"Start".to_string()),
        "Should find method Start, got: {:?}",
        func_names
    );
    assert!(
        func_names.contains(&"Stop".to_string()),
        "Should find method Stop, got: {:?}",
        func_names
    );
}

// ── Paper algorithms ──

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
    assert!(
        !result.blocks.is_empty(),
        "OriginalDiff should produce blocks for Go code"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::OriginalDiff);
}

#[test]
fn test_parent_function_go() {
    let (files, _, diff) = make_go_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "ParentFunction should include the enclosing Go function"
    );
    let block = &result.blocks[0];
    let lines = block.file_line_map.get("main.go").unwrap();
    // Diff line 9 is inside sum(), which spans lines 5-13
    assert!(
        lines.contains_key(&5) && lines.contains_key(&13),
        "Block should span the entire sum function (lines 5-13), got: {:?}",
        lines.keys().collect::<Vec<_>>()
    );
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
    assert!(
        !result.blocks.is_empty(),
        "LeftFlow Go should produce blocks"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::LeftFlow);
}

#[test]
fn test_full_flow_go() {
    let (files, _, diff) = make_go_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "FullFlow Go should produce blocks"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::FullFlow);
}

// ── Taxonomy algorithms ──

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
    assert!(
        !result.blocks.is_empty(),
        "ThinSlice Go should produce blocks"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::ThinSlice);
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
    assert!(
        !result.blocks.is_empty(),
        "RelevantSlice Go should produce blocks"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::RelevantSlice);
}

#[test]
fn test_barrier_slice_go_call_depth() {
    let source = r#"
package main

func level0(x int) int {
    return level1(x + 1)
}

func level1(y int) int {
    return level2(y * 2)
}

func level2(z int) int {
    return z + 10
}
"#;
    let path = "chain.go";
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
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::BarrierSlice),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "BarrierSlice Go should produce blocks for call chain"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::BarrierSlice);
}

#[test]
fn test_chop_go_error_pipeline() {
    let source = r#"
package main

func pipeline(input string) (string, error) {
    validated, err := validate(input)
    if err != nil {
        return "", err
    }
    result, err := transform(validated)
    if err != nil {
        return "", err
    }
    return save(result)
}

func validate(s string) (string, error) { return s, nil }
func transform(s string) (string, error) { return s, nil }
func save(s string) (string, error) { return s, nil }
"#;
    let path = "pipeline.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let config = prism::algorithms::chop::ChopConfig {
        source_file: path.to_string(),
        source_line: 5,
        sink_file: path.to_string(),
        sink_line: 13,
    };
    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::chop::slice(&ctx, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::Chop);
}

#[test]
fn test_conditioned_slice_go() {
    let source = r#"
package main

func classify(score int) string {
    var grade string
    if score >= 90 {
        grade = "A"
    } else if score >= 80 {
        grade = "B"
    } else {
        grade = "C"
    }
    return grade
}
"#;
    let path = "grades.go";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ConditionedSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ConditionedSlice);
}

#[test]
fn test_delta_slice_go() {
    let tmp = TempDir::new().unwrap();
    let old_source = "package main\n\nfunc add(a, b int) int {\n    return a + b\n}\n";
    std::fs::write(tmp.path().join("calc.go"), old_source).unwrap();

    let new_source =
        "package main\n\nfunc add(a, b int) int {\n    result := a + b\n    return result\n}\n";
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
fn test_taint_go_sql_injection() {
    let source = r#"
package main

import "net/http"

func handler(w http.ResponseWriter, r *http.Request) {
    userID := r.FormValue("id")
    query := "SELECT * FROM users WHERE id = " + userID
    rows, _ := db.Query(query)
    defer rows.Close()
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
    assert_eq!(result.algorithm, SlicingAlgorithm::Taint);
}
