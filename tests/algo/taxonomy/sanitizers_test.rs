//! Per-cleanser unit tests for the Phase 1 sanitizer registry (spec §3.4–§3.10).
//!
//! Exercises path-validation cleansers (`filepath.Clean`, `filepath.Rel`) with
//! the textual `paired_check` heuristic on `strings.HasPrefix`, plus a category-
//! isolation negative confirming that path cleansing does not suppress an
//! `OsCommand` sink.

#[path = "../../common/mod.rs"]
mod common;
use common::*;

fn run_taint_go(source: &str, diff_lines: BTreeSet<usize>) -> prism::slice::SliceResult {
    let path = "test.go";
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

fn has_structured_sink(result: &prism::slice::SliceResult, line: usize) -> bool {
    result
        .findings
        .iter()
        .any(|f| f.category.as_deref() == Some("taint_sink") && f.line == line)
}

#[test]
fn test_path_clean_with_hasprefix_suppresses() {
    // filepath.Clean + strings.HasPrefix is the canonical CWE-22 cleanser pair.
    // The textual paired_check should detect "strings.HasPrefix" anywhere in the
    // function body and suppress the os.ReadFile finding.
    let source = r#"package main

import (
	"os"
	"path/filepath"
	"strings"

	"github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
	name := c.Param("file")
	cleaned := filepath.Clean(name)
	if !strings.HasPrefix(cleaned, "/safe") {
		return
	}
	_, _ = os.ReadFile(cleaned)
}
"#;
    // Diff anchored to package line so taint comes only from framework source.
    let result = run_taint_go(source, BTreeSet::from([1]));
    assert!(
        !has_structured_sink(&result, 17),
        "filepath.Clean + strings.HasPrefix paired check should suppress finding (got: {:#?})",
        result
            .findings
            .iter()
            .filter(|f| f.category.as_deref() == Some("taint_sink"))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_path_clean_same_line_unrelated_flat_sink_still_fires() {
    // A path cleanser should suppress the flat WriteFile fallback that overlaps
    // the cleansed structured os.WriteFile call, but it must not suppress an
    // unrelated db.Exec sink packed onto the same line.
    let source = r#"package main

import (
	"database/sql"
	"os"
	"path/filepath"
	"strings"

	"github.com/gin-gonic/gin"
)

func handler(c *gin.Context, db *sql.DB) {
	name := c.Param("file")
	cleaned := filepath.Clean(name)
	if !strings.HasPrefix(cleaned, "/safe") {
		return
	}
	query := c.Query("q")
	_ = os.WriteFile(cleaned, []byte("data"), 0644); _, _ = db.Exec(query)
}
"#;
    let result = run_taint_go(source, BTreeSet::from([18]));
    assert!(
        has_structured_sink(&result, 19),
        "cleansed os.WriteFile must not suppress unrelated db.Exec on the same line (got: {:#?})",
        result
            .findings
            .iter()
            .filter(|f| f.category.as_deref() == Some("taint_sink"))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_path_clean_without_hasprefix_fires() {
    // filepath.Clean alone (no strings.HasPrefix paired check) is not enough to
    // cleanse — the recognizer requires the paired textual co-occurrence.
    let source = r#"package main

import (
	"os"
	"path/filepath"

	"github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
	name := c.Param("file")
	cleaned := filepath.Clean(name)
	_, _ = os.ReadFile(cleaned)
}
"#;
    let result = run_taint_go(source, BTreeSet::from([1]));
    assert!(
        has_structured_sink(&result, 13),
        "filepath.Clean alone (no HasPrefix paired check) does not suppress"
    );
}

#[test]
fn test_path_fake_cleanser_filepath_base_does_not_suppress() {
    // filepath.Base is NOT a registered cleanser. The os.ReadFile sink should
    // still fire even though the value passed through filepath.Base.
    let source = r#"package main

import (
	"os"
	"path/filepath"

	"github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
	name := c.Param("file")
	base := filepath.Base(name)
	_, _ = os.ReadFile(base)
}
"#;
    let result = run_taint_go(source, BTreeSet::from([1]));
    assert!(
        has_structured_sink(&result, 13),
        "filepath.Base is not a recognized cleanser; finding should fire"
    );
}

#[test]
fn test_path_rel_with_hasprefix_suppresses() {
    // filepath.Rel + strings.HasPrefix(rel, "..") guards path traversal. The
    // recognizer's paired_check matches "strings.HasPrefix" textually anywhere
    // in the function body — this suppresses the os.ReadFile finding.
    let source = r#"package main

import (
	"os"
	"path/filepath"
	"strings"

	"github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
	name := c.Param("file")
	rel, _ := filepath.Rel("/safe", name)
	if strings.HasPrefix(rel, "..") {
		return
	}
	_, _ = os.ReadFile(rel)
}
"#;
    let result = run_taint_go(source, BTreeSet::from([1]));
    assert!(
        !has_structured_sink(&result, 17),
        "filepath.Rel + strings.HasPrefix paired check should suppress finding"
    );
}

#[test]
fn test_category_isolation_path_cleanse_does_not_suppress_oscommand() {
    // A path-validation cleanser cleanses for PathTraversal but not OsCommand.
    // Tainted value flowing through filepath.Clean and into exec.Command should
    // still fire a CWE-78 finding — category isolation per spec §3.7 + ACK §3 Q3.
    let source = r#"package main

import (
	"os/exec"
	"path/filepath"
	"strings"

	"github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
	cmd := c.Query("cmd")
	cleaned := filepath.Clean(cmd)
	if !strings.HasPrefix(cleaned, "/usr/bin/") {
		return
	}
	_ = exec.Command("sh", "-c", cleaned).Run()
}
"#;
    let result = run_taint_go(source, BTreeSet::from([1]));
    assert!(
        has_structured_sink(&result, 17),
        "PathTraversal cleansing should NOT suppress OsCommand sink finding"
    );
}
