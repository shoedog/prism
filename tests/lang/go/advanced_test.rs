#[path = "../../common/mod.rs"]
mod common;
use common::*;

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
        &[
            "package main\n\nfunc foo(x int) int {\n    return x\n}\n",
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
