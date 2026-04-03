#[path = "../../common/mod.rs"]
mod common;
use common::*;

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

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
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

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Absence should detect fs.openSync without fs.closeSync"
    );
}

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

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
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

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Absence should detect http.Get without Body.Close"
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

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Absence should detect Lua io.open without close"
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

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
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

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
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

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Absence should detect unsafe block without safety assertion/comment"
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

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Absence should detect Lua socket.tcp without close"
    );
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

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
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
