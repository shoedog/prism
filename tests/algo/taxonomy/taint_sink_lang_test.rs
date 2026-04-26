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
        .any(|f| f.category.as_deref() == Some("taint_sink"));
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
        .any(|f| f.category.as_deref() == Some("taint_sink"));
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
        .any(|f| f.category.as_deref() == Some("taint_sink"));
    assert!(
        has_blocks || has_findings,
        "Taint should detect user input flowing to eval (code injection)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Phase 1 — Go CWE-78 (OS command injection) tests
//
// These exercise the structured Go sink registry (`GO_CWE78_SINKS`) and the
// framework-aware source layer (gin's `c.Query`, net/http's `r.URL.Query`).
// The diff_lines anchor specifies the line numbering so the taint engine
// pulls in the diff-line source as well as the framework source.
// ─────────────────────────────────────────────────────────────────────────────

/// Run the taint algorithm on a single Go file and return the SliceResult.
/// Centralizes test boilerplate for the Phase 1 Go CWE-78/22 cases below.
fn run_taint_go_single(
    source: &str,
    path: &str,
    diff_lines: BTreeSet<usize>,
) -> prism::slice::SliceResult {
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines,
        }],
    };
    algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap()
}

fn has_taint_sink(result: &prism::slice::SliceResult) -> bool {
    result
        .findings
        .iter()
        .any(|f| f.category.as_deref() == Some("taint_sink"))
}

#[test]
fn test_taint_cwe78_gin_shell_wrapped() {
    // gin source (c.Query) flowing into shell-wrapped exec.Command should fire.
    let source = r#"package main

import (
	"os/exec"

	"github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
	cmd := c.Query("cmd")
	_ = exec.Command("sh", "-c", cmd).Run()
}
"#;
    let result = run_taint_go_single(source, "main.go", BTreeSet::from([10]));
    assert!(
        has_taint_sink(&result),
        "expected CWE-78 finding (gin source -> exec.Command shell-wrapped sink)"
    );
}

#[test]
fn test_taint_cwe78_gin_tainted_binary() {
    // gin source flowing into the binary-path argument of exec.Command should fire
    // via the tainted-binary GO_CWE78_SINKS entry (no semantic_check).
    let source = r#"package main

import (
	"os/exec"

	"github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
	bin := c.Query("bin")
	_ = exec.Command(bin, "--help").Run()
}
"#;
    let result = run_taint_go_single(source, "main.go", BTreeSet::from([10]));
    assert!(
        has_taint_sink(&result),
        "expected CWE-78 finding (gin source -> exec.Command tainted-binary sink)"
    );
}

#[test]
fn test_taint_cwe78_nethttp_shell_wrapped() {
    // net/http source (r.URL.Query) flowing into bash-wrapped exec.Command should fire.
    let source = r#"package main

import (
	"net/http"
	"os/exec"
)

func handler(w http.ResponseWriter, r *http.Request) {
	cmd := r.URL.Query().Get("cmd")
	_ = exec.Command("bash", "-c", cmd).Run()
}
"#;
    let result = run_taint_go_single(source, "main.go", BTreeSet::from([9]));
    assert!(
        has_taint_sink(&result),
        "expected CWE-78 finding (net/http source -> exec.Command bash -c sink)"
    );
}

#[test]
fn test_taint_cwe78_no_finding_literal_safe_form() {
    // exec.Command("ls", "-la") with no taint flow -> no structured finding.
    // Note: the existing flat SINK_PATTERNS includes "Command" which fires on the
    // identifier alone — the diff-line on the same line as exec.Command would
    // still cause the flat sink to match. To check the structured behavior in
    // isolation, anchor the diff to the package line so no taint flows.
    let source = r#"package main

import "os/exec"

func main() {
	_ = exec.Command("ls", "-la").Run()
}
"#;
    let result = run_taint_go_single(source, "main.go", BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "exec.Command with literal args and no taint flow should not fire"
    );
}

#[test]
fn test_taint_cwe78_no_finding_when_unrelated_var() {
    // A gin source exists but is dropped (assigned to _). The exec.Command uses
    // only literal arguments. No taint flow reaches the sink line, so no
    // structured finding fires.
    let source = r#"package main

import (
	"os/exec"

	"github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
	_ = c.Query("ignored")
	_ = exec.Command("echo", "static").Run()
}
"#;
    // Diff anchored to the package line: nothing connects c.Query to exec.Command,
    // so the sink line should remain quiet.
    let result = run_taint_go_single(source, "main.go", BTreeSet::from([1]));
    // The framework source on line 11 is auto-added, but it doesn't reach line 12
    // because there's no def-use chain from `_ = c.Query(...)`. The sink check on
    // line 12 fails because no taint reaches it.
    let line_12_sink = result
        .findings
        .iter()
        .any(|f| f.category.as_deref() == Some("taint_sink") && f.line == 12);
    assert!(
        !line_12_sink,
        "no taint reaches the exec.Command sink line -> no finding on line 12"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Phase 1 — Go CWE-22 (path traversal) tests
//
// Cross-cutting (`os.ReadFile`, `os.Open`) and framework-gated (`http.ServeFile`,
// `c.File`) sinks are exercised here. Framework gating ensures `http.ServeFile`
// fires only when net/http is detected (corroborating signal: `*http.Request`
// in a function parameter list).
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_taint_cwe22_gin_os_readfile() {
    // gin source flowing into the cross-cutting os.ReadFile sink.
    let source = r#"package main

import (
	"os"

	"github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
	name := c.Param("file")
	data, _ := os.ReadFile(name)
	c.Data(200, "application/octet-stream", data)
}
"#;
    let result = run_taint_go_single(source, "main.go", BTreeSet::from([10]));
    assert!(
        has_taint_sink(&result),
        "expected CWE-22 finding (gin source -> os.ReadFile sink)"
    );
}

#[test]
fn test_taint_cwe22_nethttp_servefile() {
    // net/http source flowing into the framework-gated http.ServeFile sink.
    let source = r#"package main

import "net/http"

func handler(w http.ResponseWriter, r *http.Request) {
	name := r.URL.Query().Get("name")
	http.ServeFile(w, r, name)
}
"#;
    let result = run_taint_go_single(source, "main.go", BTreeSet::from([6]));
    assert!(
        has_taint_sink(&result),
        "expected CWE-22 finding (net/http source -> http.ServeFile framework-gated sink)"
    );
}

#[test]
fn test_taint_cwe22_gin_c_file() {
    // gin source flowing into the framework-gated c.File sink.
    let source = r#"package main

import "github.com/gin-gonic/gin"

func handler(c *gin.Context) {
	name := c.Param("file")
	c.File(name)
}
"#;
    let result = run_taint_go_single(source, "main.go", BTreeSet::from([6]));
    assert!(
        has_taint_sink(&result),
        "expected CWE-22 finding (gin source -> c.File framework-gated sink)"
    );
}

#[test]
fn test_taint_cwe22_no_finding_no_taint_flow() {
    // os.Open with literal path, no taint.
    let source = r#"package main

import "os"

func main() {
	_, _ = os.Open("/etc/passwd")
}
"#;
    let result = run_taint_go_single(source, "main.go", BTreeSet::from([1]));
    let line_6_sink = result
        .findings
        .iter()
        .any(|f| f.category.as_deref() == Some("taint_sink") && f.line == 6);
    assert!(
        !line_6_sink,
        "literal path with no taint flow -> no CWE-22 structured finding on line 6"
    );
}

#[test]
fn test_taint_cwe22_servefile_outside_nethttp_no_finding() {
    // http.ServeFile call without `import "net/http"` -> framework() returns None
    // -> the framework-gated http.ServeFile sink is NOT consulted. Cross-cutting
    // GO_CWE22_SINKS does not include http.ServeFile (it is framework-gated only),
    // so no sink fires even with taint flow on line 4. This pins the framework gate.
    let source = r#"package main

func main() {
	name := "user-controlled-input"
	http.ServeFile(nil, nil, name)
}
"#;
    let result = run_taint_go_single(source, "main.go", BTreeSet::from([4]));
    assert!(
        !has_taint_sink(&result),
        "framework-gated sink should not fire when no framework is detected"
    );
}
