#[path = "../../common/mod.rs"]
mod common;
use common::*;

#[test]
fn test_taint_python_pickle_loads_sink() {
    // Tainted data flows to pickle.loads() — deserialization RCE sink.
    let source = r#"
import pickle

def handle_request(user_data):
    payload = user_data
    obj = pickle.loads(payload)
    return obj
"#;
    let path = "app/handler.py";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should produce blocks when tainted value reaches pickle.loads"
    );
    assert!(
        !result.findings.is_empty(),
        "Taint should emit a finding for pickle.loads sink"
    );
}

#[test]
fn test_taint_python_subprocess_sink() {
    // Tainted command flows to subprocess.Popen() — command injection sink.
    let source = r#"
import subprocess

def run_command(user_cmd):
    cmd = user_cmd
    proc = subprocess.Popen(cmd, shell=True)
    return proc
"#;
    let path = "app/runner.py";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should produce blocks when tainted value reaches subprocess.Popen"
    );
    assert!(
        !result.findings.is_empty(),
        "Taint should emit a finding for subprocess.Popen sink"
    );
}

#[test]
fn test_taint_js_innerhtml_sink() {
    // Tainted user input flows to innerHTML — DOM XSS sink.
    let source = r#"
function displayMessage(userInput) {
    const msg = userInput;
    document.getElementById("output").innerHTML = msg;
}
"#;
    let path = "src/display.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should produce blocks when tainted value reaches innerHTML"
    );
    assert!(
        !result.findings.is_empty(),
        "Taint should emit a finding for innerHTML XSS sink"
    );
}

#[test]
fn test_taint_js_exec_sync_sink() {
    // Tainted command flows to execSync — command injection sink.
    let source = r#"
const { execSync } = require('child_process');

function runCmd(userCmd) {
    const cmd = userCmd;
    const output = execSync(cmd);
    return output;
}
"#;
    let path = "src/runner.js";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should produce blocks when tainted value reaches execSync"
    );
    assert!(
        !result.findings.is_empty(),
        "Taint should emit a finding for execSync command injection sink"
    );
}

#[test]
fn test_taint_go_exec_command_sink() {
    // Tainted user input flows to exec.Command — command injection sink.
    let source = r#"
package main

import "os/exec"

func runUserCmd(userInput string) {
    cmd := userInput
    exec.Command(cmd)
}
"#;
    let path = "cmd/handler.go";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should produce blocks when tainted value reaches exec.Command"
    );
    assert!(
        !result.findings.is_empty(),
        "Taint should emit a finding for exec.Command sink"
    );
}

#[test]
fn test_taint_go_template_html_sink() {
    // Tainted user input flows to template.HTML — XSS bypass sink.
    let source = r#"
package main

import "html/template"

func renderUnsafe(userHTML string) template.HTML {
    content := userHTML
    return template.HTML(content)
}
"#;
    let path = "web/render.go";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should produce blocks when tainted value reaches template.HTML"
    );
    assert!(
        !result.findings.is_empty(),
        "Taint should emit a finding for template.HTML XSS sink"
    );
}

#[test]
fn test_rust_taint_unsafe_sink() {
    // Tainted data flows to unsafe block — security concern.
    let source = r#"
use std::os::unix::process::CommandExt;

fn run_command(user_input: &str) {
    let cmd = user_input;
    std::process::Command::new(cmd).exec();
}
"#;
    let path = "src/runner.rs";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should produce blocks for Rust code"
    );
}

#[test]
fn test_lua_taint_exec_sink() {
    // Lua os.execute with tainted data — command injection sink.
    let source = r#"
function run_command(user_input)
    local cmd = user_input
    os.execute(cmd)
end
"#;
    let path = "scripts/runner.lua";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should produce blocks for Lua code with os.execute sink"
    );
}

#[test]
fn test_rust_taint_transmute_sink() {
    let source = r#"
fn dangerous(data: &[u8]) {
    let ptr = data.as_ptr();
    let val: u64 = unsafe { std::mem::transmute(ptr) };
    println!("{}", val);
}
"#;
    let path = "src/danger.rs";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should detect transmute as a Rust sink"
    );
}

#[test]
fn test_rust_taint_from_raw_parts_sink() {
    let source = r#"
fn rebuild_slice(ptr: *const u8, len: usize) {
    let data = unsafe { std::slice::from_raw_parts(ptr, len) };
    process(data);
}
"#;
    let path = "src/raw.rs";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should detect from_raw_parts as a Rust sink"
    );
}

#[test]
fn test_lua_taint_loadstring_sink() {
    let source = r#"
function run_user_code(input)
    local code = input
    local func = loadstring(code)
    func()
end
"#;
    let path = "scripts/eval.lua";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should detect Lua loadstring as a sink"
    );
}

#[test]
fn test_lua_taint_dofile_sink() {
    let source = r#"
function load_config(path)
    local config_path = path
    dofile(config_path)
end
"#;
    let path = "scripts/loader.lua";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Taint should detect Lua dofile as a sink"
    );
}

#[test]
fn test_taint_python_finds_sql_injection_finding() {
    let source = r#"
def handler(request):
    user_input = request.form.get("query")
    query = "SELECT * FROM users WHERE name = '" + user_input + "'"
    cursor.execute(query)
    return cursor.fetchall()
"#;
    let path = "handler.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
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
    let result = prism::algorithms::taint::slice(
        &ctx,
        &diff,
        &prism::algorithms::taint::TaintConfig::default(),
    )
    .unwrap();
    assert!(!result.blocks.is_empty(), "Taint should find tainted flow");
    // The taint analysis should detect flow from user input to execute sink
    let all_lines: BTreeSet<usize> = result
        .blocks
        .iter()
        .flat_map(|b| b.file_line_map.values())
        .flat_map(|m| m.keys())
        .copied()
        .collect();
    assert!(
        all_lines.contains(&3) || all_lines.contains(&4) || all_lines.contains(&5),
        "Taint should include lines with user_input or execute. Got lines: {:?}",
        all_lines
    );
}

#[test]
fn test_go_multi_return_taint_flow() {
    let source = r#"
package main

func handler() {
    val, err := getUserInput()
    execute(val)
}
"#;
    let path = "src/handler.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let val_defs = dfg.all_defs_of(path, "val");
    assert!(!val_defs.is_empty(), "val should have a def");

    if let Some(val_def) = val_defs.first() {
        let reachable = dfg.forward_reachable(val_def);
        assert!(
            !reachable.is_empty(),
            "Taint from val should reach execute() call"
        );
    }
}

#[test]
fn test_python_tuple_unpack_taint_flow() {
    let source = r#"
def handler():
    name, role = get_user_input()
    execute(name)
"#;
    let path = "src/handler.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let name_defs = dfg.all_defs_of(path, "name");
    assert!(!name_defs.is_empty(), "name should have a def");

    if let Some(name_def) = name_defs.first() {
        let reachable = dfg.forward_reachable(name_def);
        assert!(
            !reachable.is_empty(),
            "Taint from name should reach execute() call"
        );
    }
}

#[test]
fn test_python_with_as_taint_flow() {
    // Taint through with...as binding should flow to uses of f
    let source = r#"
def handler():
    with get_connection() as conn:
        data = conn.read()
        execute(data)
"#;
    let path = "src/handler.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let conn_defs = dfg.all_defs_of(path, "conn");
    assert!(
        !conn_defs.is_empty(),
        "with...as: 'conn' should have def for taint flow"
    );
}
