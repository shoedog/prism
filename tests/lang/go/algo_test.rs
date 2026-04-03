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

// ── Theoretical algorithms ──

#[test]
fn test_spiral_slice_go_multi_level() {
    let source = r#"
package main

func inner(x int) int {
    return x + 1
}

func outer(y int) int {
    z := inner(y)
    return z * 2
}

func caller() {
    r := outer(10)
    fmt.Println(r)
}
"#;
    let path = "spiral.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([9]),
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
    assert!(
        !result.blocks.is_empty(),
        "SpiralSlice Go should produce blocks"
    );
}

#[test]
fn test_circular_slice_go_mutual_recursion() {
    let source = r#"
package main

func ping(n int) int {
    if n <= 0 {
        return 0
    }
    return pong(n - 1)
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
            diff_lines: BTreeSet::from([8]),
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
fn test_quantum_slice_go_goroutine_channel() {
    let source = r#"
package main

func producer(ch chan int) {
    for i := 0; i < 10; i++ {
        ch <- i
    }
    close(ch)
}

func consumer(ch chan int) {
    for val := range ch {
        process(val)
    }
}

func main() {
    ch := make(chan int)
    go producer(ch)
    consumer(ch)
}
"#;
    let path = "concurrent.go";
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
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::QuantumSlice);
}

#[test]
fn test_quantum_slice_go_select_statement() {
    let source = r#"
package main

import "time"

func worker(done chan bool, data chan string) {
    for {
        select {
        case msg := <-data:
            process(msg)
        case <-time.After(5 * time.Second):
            fmt.Println("timeout")
            return
        case <-done:
            return
        }
    }
}
"#;
    let path = "select.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::QuantumSlice);
}

#[test]
fn test_horizontal_slice_go_interface_methods() {
    let source = r#"
package main

type Handler interface {
    Handle(req Request) Response
}

type UserHandler struct{}

func (h *UserHandler) Handle(req Request) Response {
    user := findUser(req.ID)
    return Response{Body: user}
}

type OrderHandler struct{}

func (h *OrderHandler) Handle(req Request) Response {
    order := findOrder(req.ID)
    return Response{Body: order}
}
"#;
    let path = "handlers.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::HorizontalSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::HorizontalSlice);
}

#[test]
fn test_vertical_slice_go_layered() {
    let source_handler = r#"
package handler

func APIHandler(w http.ResponseWriter, r *http.Request) {
    data := r.FormValue("input")
    result := service.Process(data)
    json.NewEncoder(w).Encode(result)
}
"#;
    let source_service = r#"
package service

func Process(data string) Result {
    validated := Validate(data)
    return repo.Save(validated)
}
"#;
    let source_repo = r#"
package repo

func Save(data string) Result {
    db.Insert(data)
    return Result{OK: true}
}
"#;
    let handler_path = "handler/api.go";
    let service_path = "service/processor.go";
    let repo_path = "repo/store.go";

    let mut files = BTreeMap::new();
    files.insert(
        handler_path.to_string(),
        ParsedFile::parse(handler_path, source_handler, Language::Go).unwrap(),
    );
    files.insert(
        service_path.to_string(),
        ParsedFile::parse(service_path, source_service, Language::Go).unwrap(),
    );
    files.insert(
        repo_path.to_string(),
        ParsedFile::parse(repo_path, source_repo, Language::Go).unwrap(),
    );

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: service_path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5]),
        }],
    };

    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::vertical_slice::slice(
        &ctx,
        &diff,
        &prism::algorithms::vertical_slice::VerticalConfig::default(),
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::VerticalSlice);
}

#[test]
fn test_angle_slice_go_error_handling_concern() {
    let source = r#"
package main

import "fmt"

func fetchUser(id int) (*User, error) {
    user, err := db.Get(id)
    if err != nil {
        return nil, fmt.Errorf("fetchUser: %w", err)
    }
    return user, nil
}

func fetchOrder(id int) (*Order, error) {
    order, err := db.Get(id)
    if err != nil {
        return nil, fmt.Errorf("fetchOrder: %w", err)
    }
    return order, nil
}
"#;
    let path = "api.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AngleSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AngleSlice);
}

#[test]
fn test_threed_slice_go() {
    let source =
        "package main\n\nfunc foo(x int) int {\n    y := x + 1\n    return y\n}\n\nfunc bar() {\n    r := foo(10)\n    fmt.Println(r)\n}\n";
    let filename = "app.go";
    let tmp = create_temp_git_repo(
        filename,
        &["package main\n\nfunc foo(x int) int {\n    return x\n}\n", source],
    );
    let parsed = ParsedFile::parse(filename, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(filename.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: filename.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4, 5]),
        }],
    };
    let config = prism::algorithms::threed_slice::ThreeDConfig {
        temporal_days: 365,
        git_dir: tmp.path().to_string_lossy().to_string(),
    };
    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::threed_slice::slice(&ctx, &diff, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ThreeDSlice);
}

// ── Novel algorithms ──

#[test]
fn test_absence_slice_go_defer_missing() {
    let source = r#"
package main

import "os"

func readFile(path string) ([]byte, error) {
    f, err := os.Open(path)
    if err != nil {
        return nil, err
    }
    data := make([]byte, 1024)
    n, _ := f.Read(data)
    return data[:n], nil
}
"#;
    let path = "file.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AbsenceSlice);
    // AbsenceSlice should detect the missing f.Close() / defer f.Close()
    if !result.blocks.is_empty() {
        let block = &result.blocks[0];
        assert!(
            block.file_line_map.contains_key(path),
            "AbsenceSlice should reference the Go file"
        );
    }
}

#[test]
fn test_absence_slice_go_mutex_unlock() {
    let source = r#"
package main

import "sync"

func critical(mu *sync.Mutex) int {
    mu.Lock()
    result := compute()
    return result
}
"#;
    let path = "sync.go";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AbsenceSlice);
}

#[test]
fn test_symmetry_slice_go_marshal_unmarshal() {
    let source = r#"
package main

import "encoding/json"

func marshal(v interface{}) ([]byte, error) {
    return json.Marshal(v)
}

func unmarshal(data []byte, v interface{}) error {
    return json.Unmarshal(data, v)
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
            diff_lines: BTreeSet::from([7]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::SymmetrySlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::SymmetrySlice);
}

#[test]
fn test_gradient_slice_go() {
    let (files, _, diff) = make_go_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::GradientSlice),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "GradientSlice Go should produce blocks"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::GradientSlice);
}

#[test]
fn test_provenance_slice_go_http_input() {
    let source = r#"
package main

import "net/http"

func handler(w http.ResponseWriter, r *http.Request) {
    name := r.FormValue("name")
    email := r.FormValue("email")
    user := createUser(name, email)
    respond(w, user)
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
            diff_lines: BTreeSet::from([9]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ProvenanceSlice);
}

#[test]
fn test_phantom_slice_go() {
    let source = "package main\n\nfunc remaining(x int) int {\n    return x + 1\n}\n";
    let filename = "app.go";
    let tmp = create_temp_git_repo(
        filename,
        &[
            "package main\n\nfunc deleted(x int) int {\n    return x * 2\n}\n\nfunc remaining(x int) int {\n    return x + 1\n}\n",
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
    let config = prism::algorithms::phantom_slice::PhantomConfig {
        git_dir: tmp.path().to_string_lossy().to_string(),
        max_commits: 50,
    };
    let result = prism::algorithms::phantom_slice::slice(&files, &diff, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::PhantomSlice);
}

#[test]
fn test_resonance_slice_go() {
    let source = "package main\n\nfunc update(x int) int {\n    y := x + 1\n    return y\n}\n";
    let filename = "app.go";
    let tmp = create_temp_git_repo(
        filename,
        &[
            "package main\n\nfunc update(x int) int {\n    return x\n}\n",
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
}

#[test]
fn test_membrane_slice_go_multifile() {
    let source_api = r#"
package api

func FetchUser(id int) (*User, error) {
    user, err := db.Get(id)
    if err != nil {
        return nil, err
    }
    return user, nil
}
"#;
    let source_caller1 = r#"
package handlers

func ShowProfile(id int) {
    user, _ := api.FetchUser(id)
    render(user)
}
"#;
    let source_caller2 = r#"
package admin

func AdminView(id int) {
    user, err := api.FetchUser(id)
    if err != nil {
        log.Fatal(err)
    }
    display(user)
}
"#;
    let mut files = BTreeMap::new();
    files.insert(
        "api/user.go".to_string(),
        ParsedFile::parse("api/user.go", source_api, Language::Go).unwrap(),
    );
    files.insert(
        "handlers/profile.go".to_string(),
        ParsedFile::parse("handlers/profile.go", source_caller1, Language::Go).unwrap(),
    );
    files.insert(
        "admin/view.go".to_string(),
        ParsedFile::parse("admin/view.go", source_caller2, Language::Go).unwrap(),
    );
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "api/user.go".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5, 6]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::MembraneSlice);
}

#[test]
fn test_echo_slice_go_error_propagation() {
    let source_lib = r#"
package db

func Query(sql string) (*Rows, error) {
    rows, err := conn.Execute(sql)
    if err != nil {
        return nil, fmt.Errorf("query failed: %w", err)
    }
    return rows, nil
}
"#;
    let source_caller = r#"
package handler

func ListUsers() {
    rows, _ := db.Query("SELECT * FROM users")
    process(rows)
}
"#;
    let mut files = BTreeMap::new();
    files.insert(
        "db/query.go".to_string(),
        ParsedFile::parse("db/query.go", source_lib, Language::Go).unwrap(),
    );
    files.insert(
        "handler/users.go".to_string(),
        ParsedFile::parse("handler/users.go", source_caller, Language::Go).unwrap(),
    );
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "db/query.go".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5, 6, 7]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::EchoSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::EchoSlice);
}

// ── Go-specific language feature tests ──

#[test]
fn test_go_defer_statement_in_left_flow() {
    let source = r#"
package main

import "os"

func processFile(path string) error {
    f, err := os.Open(path)
    if err != nil {
        return err
    }
    defer f.Close()
    data := make([]byte, 1024)
    n, _ := f.Read(data)
    return process(data[:n])
}
"#;
    let path = "file.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([13]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should handle Go defer statements"
    );
}

#[test]
fn test_go_range_loop_data_flow() {
    let source = r#"
package main

func sumPositive(numbers []int) int {
    total := 0
    for _, n := range numbers {
        if n > 0 {
            total += n
        }
    }
    return total
}
"#;
    let path = "sum.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let total_defs = dfg.all_defs_of(path, "total");
    assert!(
        !total_defs.is_empty(),
        "Go range loop: 'total' should have defs"
    );
}

#[test]
fn test_go_multi_return_error_flow() {
    let source = r#"
package main

func divide(a, b float64) (float64, error) {
    if b == 0 {
        return 0, fmt.Errorf("division by zero")
    }
    return a / b, nil
}

func compute() {
    result, err := divide(10, 0)
    if err != nil {
        log.Fatal(err)
    }
    use(result)
}
"#;
    let path = "math.go";
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

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "FullFlow should handle Go multi-return error patterns"
    );
}

#[test]
fn test_go_struct_field_access_left_flow() {
    let source = r#"
package main

type Config struct {
    Host    string
    Port    int
    Timeout int
}

func setup() *Config {
    cfg := &Config{}
    cfg.Host = "localhost"
    cfg.Port = 8080
    cfg.Timeout = 30
    return cfg
}
"#;
    let path = "config.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([13]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should trace Go struct field assignments"
    );
}

#[test]
fn test_go_goroutine_spawn() {
    let source = r#"
package main

func startWorkers(n int) {
    for i := 0; i < n; i++ {
        go worker(i)
    }
}

func worker(id int) {
    fmt.Printf("worker %d started\n", id)
}
"#;
    let path = "workers.go";
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
    // LeftFlow should handle go statements
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce blocks involving goroutine spawn"
    );
}

#[test]
fn test_go_blank_identifier_ignored() {
    let source = r#"
package main

func process() {
    _, err := getData()
    if err != nil {
        handle(err)
    }
}
"#;
    let path = "blank.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    // err should have a def, but _ should be ignored or not tracked meaningfully
    let err_defs = dfg.all_defs_of(path, "err");
    assert!(
        !err_defs.is_empty(),
        "Go blank identifier: 'err' should still have a def"
    );
}

#[test]
fn test_go_const_declaration() {
    let source = r#"
package main

const MaxRetries = 3

func retry(fn func() error) error {
    var err error
    for i := 0; i < MaxRetries; i++ {
        err = fn()
        if err == nil {
            return nil
        }
    }
    return err
}
"#;
    let path = "retry.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should work with Go const declarations"
    );
}
