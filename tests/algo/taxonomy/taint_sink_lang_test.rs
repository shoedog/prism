#[path = "../../common/mod.rs"]
mod common;
use common::*;

// === Tier 1: Taint — JS basic sinks (innerHTML, execSync) ===

#[test]
fn test_taint_js_innerhtml_xss() {
    // innerHTML is a classic XSS sink — user input flowing to it should be flagged.
    let source = r#"
function showMessage(userInput) {
    const msg = userInput;
    document.getElementById("output").innerHTML = msg;
}
"#;
    let path = "src/ui.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    // innerHTML should be detected as a taint sink
    let has_blocks = !result.blocks.is_empty();
    let has_findings = result
        .findings
        .iter()
        .any(|f| f.category.as_deref() == Some("tainted_value"));
    assert!(
        has_blocks || has_findings,
        "Taint should detect data flowing to innerHTML (XSS sink)"
    );
}

#[test]
fn test_taint_js_exec_sync_command_injection() {
    // execSync with user input is a command injection vulnerability.
    let source = r#"
const { execSync } = require('child_process');

function runCommand(userCmd) {
    const cmd = userCmd;
    const output = execSync(cmd);
    return output.toString();
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
            diff_lines: BTreeSet::from([6]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    let has_blocks = !result.blocks.is_empty();
    let has_findings = result
        .findings
        .iter()
        .any(|f| f.category.as_deref() == Some("tainted_value"));
    assert!(
        has_blocks || has_findings,
        "Taint should detect user input flowing to execSync (command injection)"
    );
}

#[test]
fn test_taint_js_eval_code_injection() {
    // eval() with user input is a code injection vulnerability.
    let source = r#"
function evaluate(expression) {
    const expr = expression;
    const result = eval(expr);
    return result;
}
"#;
    let path = "src/calc.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    let has_blocks = !result.blocks.is_empty();
    let has_findings = result
        .findings
        .iter()
        .any(|f| f.category.as_deref() == Some("tainted_value"));
    assert!(
        has_blocks || has_findings,
        "Taint should detect user input flowing to eval (code injection)"
    );
}
