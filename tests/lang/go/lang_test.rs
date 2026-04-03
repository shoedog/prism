#[path = "../../common/mod.rs"]
mod common;
use common::*;

// ── Novel algorithms (git-based + multi-file) ──

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
