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

// Path C dual-layer regression tests
// (test_taint_cwe78_literal_binary_with_tainted_later_arg_no_finding,
//  test_taint_cwe78_commandcontext_literal_binary_no_finding)
// were removed alongside the rollback in the parent commit. They asserted
// that flat-pattern catch-alls would suppress on `SemanticallyExcluded`
// outcomes, which is no longer engine behavior. The literal-binary case
// now correctly leaves the flat fallback active (acknowledged false-positive
// class until per-arg DFG + PowerShell shell list expansion land — see
// Phase 1.5 priority queue items #1 and #4). Per-arg DFG will reintroduce
// proper coverage with assertions that don't depend on dual-layer
// suppression.

#[test]
fn test_taint_cwe78_pwsh_unmodeled_shell_preserves_flat_fallback() {
    // P1 regression guard. exec.Command("pwsh", "-c", taintedInput) is a real
    // PowerShell shell-interpretation risk that Phase 1's structured registry
    // does NOT model (PowerShell binaries are out of `is_shell_wrapper_at`'s
    // literal list — Phase 1.5 queue #4). The flat-pattern catch-all on the
    // `Command` substring is the only signal that fires; the dual-layer
    // suppression in PR #73's original form would have silently dropped this
    // (see PR #73 review comment and the rollback rationale).
    //
    // Until per-arg DFG + PowerShell expansion land, this test pins the flat
    // fallback firing for unmodeled shells.
    let source = r#"package main

import (
	"os/exec"

	"github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
	input := c.Query("input")
	_ = exec.Command("pwsh", "-c", input).Run()
}
"#;
    // Source line: 10 (c.Query). Sink line: 11 (exec.Command pwsh shell).
    let result = run_taint_go_single(source, "main.go", BTreeSet::from([10]));
    let line_11_sink = result
        .findings
        .iter()
        .any(|f| f.category.as_deref() == Some("taint_sink") && f.line == 11);
    assert!(
        line_11_sink,
        "exec.Command(\"pwsh\", \"-c\", taintedInput) should still produce a taint_sink \
         finding via the flat-pattern fallback — PowerShell is a real shell-interpretation \
         risk that the structured registry does not model. Suppressing this would silently \
         lose coverage; see PR #73 review feedback (P1)."
    );
}

#[test]
fn test_taint_cwe78_same_line_unrelated_sink_preserved() {
    // P2 regression guard. A literal-binary exec.Command (which the structured
    // tainted-binary pattern excludes via Path C) sharing a line with an
    // unrelated db.Exec(query) where query is tainted: the db.Exec sink must
    // NOT be hidden by the structured exclusion of the exec.Command. Pre-
    // rollback, the dual-layer suppression at line granularity hid all sinks
    // on the line as soon as any structured pattern returned
    // SemanticallyExcluded — a false negative for the unrelated SQL sink.
    //
    // Post-rollback, both calls share the line; flat-layer fires on the `Exec`
    // substring (db.Exec) and `Command` substring (exec.Command), producing a
    // taint_sink finding on this line.
    let source = r#"package main

import (
	"database/sql"
	"os/exec"

	"github.com/gin-gonic/gin"
)

func handler(c *gin.Context, db *sql.DB) {
	query := c.Query("q")
	_ = exec.Command("ffmpeg", "-i", "static.mp4"); _, _ = db.Exec(query)
}
"#;
    // Source line: 11 (c.Query). Sink line: 12 (both exec.Command + db.Exec).
    let result = run_taint_go_single(source, "main.go", BTreeSet::from([11]));
    let line_12_sink = result
        .findings
        .iter()
        .any(|f| f.category.as_deref() == Some("taint_sink") && f.line == 12);
    assert!(
        line_12_sink,
        "db.Exec(taintedQuery) on the same line as a literal-binary exec.Command must still \
         fire a taint_sink finding. Pre-rollback dual-layer suppression hid this; the \
         rollback restores correct fallback. See PR #73 review feedback (P2)."
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Phase 1.5 (#1) — Per-arg DFG regression tests
//
// These tests pin the per-arg DFG behavior introduced in the parent commit.
// They use **structured-only sinks** (e.g. `c.File`) where absence-of-finding
// is the discriminating assertion — flat-pattern overlap (e.g. `Command`,
// `Rename`) defeats absence assertions because `sink_lines` line-dedupes, so
// a `count <= 1` assertion can't distinguish "structured + flat" from
// "flat-only." See design note §3.1 for the rationale.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_taint_cwe22_cfile_variable_bound_to_literal_no_finding() {
    // Phase 1.5 (#1) discriminating regression. `bin = "/etc/static.txt"` is a
    // literal-bound variable; pre-DFG, the structured c.File pattern fires
    // because the line is line-tainted (input from c.Query is referenced on
    // the same line via `_ = input`). Post-DFG: arg[0] = bin identifier; bin
    // is not in any FlowEdge — defined as a literal — so arg[0] is not
    // tainted → SemanticallyExcluded → no fire. c.File has no flat-pattern
    // overlap, so absence of any taint_sink on the call line discriminates
    // pre vs post DFG behavior.
    let source = r#"package main

import "github.com/gin-gonic/gin"

func handler(c *gin.Context) {
	input := c.Query("input")
	bin := "/etc/static.txt"
	_ = c.File(bin); _ = input
}
"#;
    // Source line: 6 (c.Query). Sink line: 8 (c.File).
    let result = run_taint_go_single(source, "main.go", BTreeSet::from([6]));
    let sink_line_finding = result
        .findings
        .iter()
        .any(|f| f.category.as_deref() == Some("taint_sink") && f.line == 8);
    assert!(
        !sink_line_finding,
        "c.File(bin) where bin is bound to a literal should NOT fire post-DFG, \
         even when an unrelated tainted variable shares the line"
    );
}

#[test]
fn test_taint_cwe22_os_rename_smoke() {
    // Smoke test for os.Rename firing on tainted arg[1].
    //
    // Limitation: not a discriminator for any-tainted semantics on
    // `tainted_arg_indices: &[0, 1]` because flat SINK_PATTERNS already
    // includes "Rename" (taint.rs:123). The test would still pass via flat
    // fallback even if structured per-arg DFG mishandled multi-index. For a
    // discriminating assertion of any-tainted semantics, queue a unit test
    // against `go_sink_outcome` internals — Phase 1.5.1 follow-up.
    let source = r#"package main

import (
	"os"

	"github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
	dest := c.Query("dest")
	_ = os.Rename("/tmp/static.txt", dest)
}
"#;
    let result = run_taint_go_single(source, "main.go", BTreeSet::from([10]));
    assert!(
        has_taint_sink(&result),
        "os.Rename should fire when arg[1] is tainted (smoke; not diagnostic \
         for structured any-tainted semantics due to flat-pattern overlap)"
    );
}

#[test]
fn test_taint_cwe22_cfile_literal_arg_with_unrelated_line_taint_no_finding() {
    // Phase 1.5 negative control on a structured-only sink. Tainted `input` is
    // referenced on the same line as a c.File call whose arg[0] is a literal.
    // Pre-DFG: line-granular over-fire (line tainted via input). Post-DFG:
    // arg[0] = "/etc/static.txt" literal → not tainted → SemanticallyExcluded
    // → no fire. c.File has no flat overlap; absence of taint_sink on the
    // call line discriminates.
    let source = r#"package main

import "github.com/gin-gonic/gin"

func handler(c *gin.Context) {
	input := c.Query("input")
	_ = c.File("/etc/static.txt"); _ = input
}
"#;
    // Source line: 6 (c.Query). Sink line: 7 (c.File with literal arg).
    let result = run_taint_go_single(source, "main.go", BTreeSet::from([6]));
    let sink_line_finding = result
        .findings
        .iter()
        .any(|f| f.category.as_deref() == Some("taint_sink") && f.line == 7);
    assert!(
        !sink_line_finding,
        "c.File with a literal arg should NOT fire post-DFG even when an unrelated \
         tainted variable shares the line"
    );
}

#[test]
fn test_taint_cwe78_complex_arg_expression_fires() {
    // exec.Command takes the result of a method call as the binary arg. The
    // method call itself returns tainted data via DFG. Per-arg conservative
    // recursion: descend into the call_expression for arg[0]; the inner
    // identifier is tainted, so arg[0] is treated as tainted; structured fires.
    let source = r#"package main

import (
	"os/exec"

	"github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
	bin := c.Query("bin")
	_ = exec.Command(string([]byte(bin)), "--help").Run()
}
"#;
    // Source line: 10 (c.Query). Sink line: 11 (exec.Command with complex expr).
    let result = run_taint_go_single(source, "main.go", BTreeSet::from([10]));
    assert!(
        has_taint_sink(&result),
        "exec.Command with a complex-expression arg[0] containing a tainted \
         identifier should fire via per-arg conservative recursion"
    );
}

#[test]
fn test_taint_cwe22_cfile_inline_param_source_still_fires() {
    // Source==sink shape. c.Param is the source AND its return value is
    // c.File's arg[0]. No FlowEdge connects them (no intermediate variable),
    // so paths.iter().filter(originating) is empty. The source==sink loop's
    // no-originating-path branch passes None for the FlowPath argument; the
    // engine falls back to call_path + semantic_check matching (skipping
    // per-arg DFG) and fires.
    let source = r#"package main

import "github.com/gin-gonic/gin"

func handler(c *gin.Context) {
	c.File(c.Param("f"))
}
"#;
    // Source/sink line: 6 (c.File and c.Param share this line).
    let result = run_taint_go_single(source, "main.go", BTreeSet::from([6]));
    assert!(
        has_taint_sink(&result),
        "c.File(c.Param(\"f\")) source==sink shape must still fire post-DFG \
         via the Option<&FlowPath>::None fallback in the source==sink loop"
    );
}

#[test]
fn test_taint_cwe22_cfile_inline_param_with_parallel_path_still_fires() {
    // Mixed same-line regression. Single-line function body has TWO sources:
    // c.Query (generates a FlowPath via def-use of `other`) AND c.Param
    // (inline inside c.File, no FlowPath). Source 1's FlowPath makes
    // `originating` non-empty for this line, so the primary
    // Option<&FlowPath>::None fallback (gated on originating.is_empty()) is
    // skipped. The secondary inline-source fallback (gated on
    // find_sink_with_inline_framework_source) detects c.Param as an inline
    // framework-source call inside c.File's arg[0] and fires the c.File sink
    // (modulo function-body cleansing).
    let source = r#"package main

import (
	"fmt"

	"github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
	other := c.Query("other"); c.File(c.Param("f")); fmt.Println(other)
}
"#;
    // All three statements share line 10. taint_sources contains (file, 10).
    let result = run_taint_go_single(source, "main.go", BTreeSet::from([10]));
    assert!(
        has_taint_sink(&result),
        "c.File(c.Param(\"f\")) on a line with a parallel c.Query/FlowPath \
         must still fire post-DFG via the inline-source secondary fallback. \
         If this fails, audit find_sink_with_inline_framework_source's \
         framework-source recognition + concretize_source_call_path's prefix \
         substitution."
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
