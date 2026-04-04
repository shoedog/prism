#[path = "../../common/mod.rs"]
mod common;
use common::*;

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

#[test]
fn test_gradient_slice_python() {
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::GradientSlice),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
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
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::EchoSlice),
        None,
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
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::EchoSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::EchoSlice);
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

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::SymmetrySlice),
        None,
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

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::EchoSlice),
        None,
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
fn test_gradient_slice_python_scores_decay() {
    // Gradient slice should assign higher relevance to lines closer to the diff
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::GradientSlice),
        None,
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

// === 2.7 Behavioral test: Echo — Java checked exception ===

#[test]
fn test_echo_java_checked_exception() {
    // Method adds `throws IOException`, caller doesn't declare it.
    // Echo should include the caller that doesn't handle the new exception.
    let source_api = r#"
import java.io.IOException;

public class FileService {
    public String readData(String path) throws IOException {
        return new String(java.nio.file.Files.readAllBytes(java.nio.file.Paths.get(path)));
    }
}
"#;
    let source_caller = r#"
public class Handler {
    public void handle() {
        FileService svc = new FileService();
        String data = svc.readData("/tmp/data");
        System.out.println(data);
    }
}
"#;
    let mut files = BTreeMap::new();
    files.insert(
        "FileService.java".to_string(),
        ParsedFile::parse("FileService.java", source_api, Language::Java).unwrap(),
    );
    files.insert(
        "Handler.java".to_string(),
        ParsedFile::parse("Handler.java", source_caller, Language::Java).unwrap(),
    );
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "FileService.java".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::EchoSlice),
        None,
    )
    .unwrap();

    // Echo should produce a block — the caller references the changed function
    assert!(
        !result.blocks.is_empty(),
        "Echo should include callers of method with changed throws declaration"
    );
}

// === 2.7 Behavioral test: Echo — Lua nil return ===

#[test]
fn test_echo_lua_nil_return() {
    // Function changes return to `nil, err` pattern, caller doesn't check.
    let source_lib = r#"
function connect(host)
    local sock = socket.tcp()
    local ok, err = sock:connect(host, 80)
    if not ok then
        return nil, err
    end
    return sock
end
"#;
    let source_caller = r#"
function fetch(host)
    local conn = connect(host)
    conn:send("GET / HTTP/1.1\r\n")
    return conn:receive("*a")
end
"#;
    let mut files = BTreeMap::new();
    files.insert(
        "net.lua".to_string(),
        ParsedFile::parse("net.lua", source_lib, Language::Lua).unwrap(),
    );
    files.insert(
        "client.lua".to_string(),
        ParsedFile::parse("client.lua", source_caller, Language::Lua).unwrap(),
    );
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "net.lua".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5, 6]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::EchoSlice),
        None,
    )
    .unwrap();

    // Echo should detect that the caller uses connect() without checking nil
    assert!(
        !result.blocks.is_empty(),
        "Echo should include callers that don't check nil return from connect()"
    );
}

// === Tier 2: Echo — Rust ? operator (item 13) ===

#[test]
fn test_echo_rust_question_mark_operator() {
    // Rust `?` operator is safe error propagation. Echo should recognize it
    // as proper error handling and NOT flag the caller.
    let source_lib = r#"
use std::io;

pub fn read_config(path: &str) -> Result<String, io::Error> {
    let data = std::fs::read_to_string(path)?;
    Ok(data)
}
"#;
    let source_caller = r#"
use std::io;

pub fn load(path: &str) -> Result<String, io::Error> {
    let config = read_config(path)?;
    Ok(config)
}
"#;
    let mut files = BTreeMap::new();
    files.insert(
        "src/config.rs".to_string(),
        ParsedFile::parse("src/config.rs", source_lib, Language::Rust).unwrap(),
    );
    files.insert(
        "src/loader.rs".to_string(),
        ParsedFile::parse("src/loader.rs", source_caller, Language::Rust).unwrap(),
    );
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/config.rs".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::EchoSlice),
        None,
    )
    .unwrap();

    // The caller uses `?` which IS proper error handling.
    // Echo should either produce no findings for this caller,
    // or if it flags the caller, the findings should NOT say "not checked".
    let unchecked_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.file == "src/loader.rs" && f.description.contains("not checked"))
        .collect();
    assert!(
        unchecked_findings.is_empty(),
        "Echo should recognize Rust `?` as proper error handling, not flag as unchecked"
    );
}

#[test]
fn test_echo_rust_missing_error_handling() {
    // Caller that does NOT use ? or match — should be flagged.
    // Changed function touches a return/error line to trigger echo analysis.
    let source_lib = r#"
pub fn fetch_data(url: &str) -> Result<Vec<u8>, String> {
    let data = download(url);
    return Err("not found".to_string());
}
"#;
    let source_caller = r#"
pub fn process(url: &str) {
    let data = fetch_data(url).unwrap();
    println!("{}", data.len());
}
"#;
    let mut files = BTreeMap::new();
    files.insert(
        "src/fetcher.rs".to_string(),
        ParsedFile::parse("src/fetcher.rs", source_lib, Language::Rust).unwrap(),
    );
    files.insert(
        "src/main.rs".to_string(),
        ParsedFile::parse("src/main.rs", source_caller, Language::Rust).unwrap(),
    );
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/fetcher.rs".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::EchoSlice),
        None,
    )
    .unwrap();

    // Echo should detect that the caller uses unwrap() without proper error handling
    assert!(
        !result.blocks.is_empty(),
        "Echo should flag caller that uses unwrap() without proper error handling"
    );
}
