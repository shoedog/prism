# Phase 1 (CWE-78/22 Go) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Go CWE-78 (OS command injection) + CWE-22 (path traversal) detection to Prism's taint engine, with framework-aware source detection (net/http + gin + gorilla/mux) and category-aware sanitizer recognition (shell-escape + path-validation).

**Architecture:** Three sequential commits on branch `claude/cwe-phase1-go`. Architecture locked by ACK §3 Q1-Q5; design spec at `docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md` (committed `57ce096` on main).

**Tech Stack:** Rust 2021, tree-sitter (Go grammar already integrated), petgraph (CPG), `OnceCell` for lazy detection caching. Tests use shared `tests/common/mod.rs` helpers. `Cargo.toml` `[[test]]` entries per test file.

**Source-of-truth references** (read these before starting):
- Spec: `docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md` — defines all struct shapes, const-array contents, and pattern lists.
- ACK: `ACK-prism-cwe-coverage-handoff.md` — eval-team requirements.
- CLAUDE.md: project conventions, build commands, file organization.
- PR #71 (commits `c08db28..1ffea97`): Phase 0 reference; same shape (3 commits, single PR).

---

## File Structure

### Created in Commit 1 — Framework detection
| File | Responsibility |
|---|---|
| `src/frameworks/mod.rs` | Module root: `FrameworkSpec`, `SourcePattern`, `SinkPattern`, `SanitizerRecognizer` types; `ALL_FRAMEWORKS` const; `detect_for()` function |
| `src/frameworks/nethttp.rs` | `pub const SPEC: FrameworkSpec` for net/http with `detect`, `SOURCES`, `SINKS` |
| `src/frameworks/gin.rs` | `pub const SPEC: FrameworkSpec` for gin |
| `src/frameworks/gorilla_mux.rs` | `pub const SPEC: FrameworkSpec` for gorilla/mux |
| `tests/frameworks/mod.rs` | Shared `#[path]` to `tests/common/mod.rs` |
| `tests/frameworks/nethttp_test.rs` | Per-framework detection tests |
| `tests/frameworks/gin_test.rs` | Per-framework detection tests |
| `tests/frameworks/gorilla_mux_test.rs` | Per-framework detection tests |
| `tests/frameworks/registry_test.rs` | Disambiguation + no-framework baseline tests |

### Modified in Commit 1
| File | Change |
|---|---|
| `src/lib.rs` | Add `pub mod frameworks;` |
| `src/ast.rs` | Add `framework: OnceCell<Option<&'static FrameworkSpec>>` field to `ParsedFile`; add `framework()` accessor |
| `Cargo.toml` | 4 new `[[test]]` entries (`frameworks_nethttp`, `frameworks_gin`, `frameworks_gorilla_mux`, `frameworks_registry`) |
| `tests/integration/coverage_test.rs` | Add 4 test paths to all 3 `all_test_files` arrays |

### Created in Commit 2 — Sinks
| File | Responsibility |
|---|---|
| (none new — all changes are extensions to existing files) | |

### Modified in Commit 2
| File | Change |
|---|---|
| `src/algorithms/taint.rs` | Add `GO_CWE78_SINKS`, `GO_CWE22_SINKS` consts + `check_shell_wrapper`/`check_shell_wrapper_ctx` helpers + `CallSite::literal_arg` helper; consult new consts in analysis pass; hook framework `SOURCES` into source inference |
| `src/frameworks/nethttp.rs` | Populate `SINKS` const with `http.ServeFile` |
| `src/frameworks/gin.rs` | Populate `SINKS` const with `c.File` |
| `tests/algo/taxonomy/taint_sink_lang_test.rs` | Extend with Go CWE-78/22 sink tests |
| `Cargo.toml` | (no new entries) |
| `tests/integration/coverage_test.rs` | (no new entries) |

### Created in Commit 3 — Sanitizers
| File | Responsibility |
|---|---|
| `src/sanitizers/mod.rs` | `SanitizerCategory` enum, `SanitizerRecognizer` struct, propagation/suppression hooks |
| `src/sanitizers/shell.rs` | `pub const SHELL_RECOGNIZERS: &[SanitizerRecognizer] = &[];` (empty per spec §3.9) |
| `src/sanitizers/path.rs` | `pub const PATH_RECOGNIZERS: &[SanitizerRecognizer]` with Clean+HasPrefix and Rel+HasPrefix |
| `tests/algo/taxonomy/sanitizers_test.rs` | Per-cleanser unit tests |
| `tests/integration/cwe_phase1_suppression_test.rs` | Asserts ≥80% suppression rate on the 10+10 fixture suite |
| `tests/fixtures/sanitizer-suite-go/sanitized/` | 10 sanitized Go examples (one file per example) |
| `tests/fixtures/sanitizer-suite-go/unsanitized/` | 10 unsanitized Go examples |

### Modified in Commit 3
| File | Change |
|---|---|
| `src/lib.rs` | Add `pub mod sanitizers;` |
| `src/data_flow.rs` | Augment `FlowPath` (or equivalent per-flow data structure) with `cleansed_for: BTreeSet<SanitizerCategory>` field |
| `src/algorithms/taint.rs` | Wire propagation hook (cleansers add to `cleansed_for`); wire suppression check at sink |
| `Cargo.toml` | 2 new `[[test]]` entries (`algo_taxonomy_sanitizers`, `integration_cwe_phase1_suppression`) |
| `tests/integration/coverage_test.rs` | Add 2 new test paths to all 3 `all_test_files` arrays |

---

## Pre-flight (run once before Task 1)

Sub-agent for Commit 1 should run these checks first:

- [ ] **P1: Confirm branch + base**

```bash
cd /Users/wesleyjinks/code/slicing
git checkout main
git pull
git log -1 --oneline
# expect: 57ce096 Phase 1 design spec: CWE-78/22 Go + framework detection + sanitizers
git checkout -b claude/cwe-phase1-go
git branch --show-current
# expect: claude/cwe-phase1-go
```

- [ ] **P2: Confirm baseline tests pass**

```bash
cargo test 2>&1 | grep -E "test result:" | tail -1
# expect: a "test result: ok" line; the cumulative count should be ~1,406
cargo fmt --check
# expect: no output
cargo build 2>&1 | grep -E "warning|error" | head -5
# expect: no output (clean build)
```

- [ ] **P3: Read the spec sections relevant to Commit 1**

```bash
sed -n '/^## 2\./,/^## 3\./p' docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md > /tmp/phase1-c1-spec.md
wc -l /tmp/phase1-c1-spec.md
# This is your authoritative reference for Commit 1 struct shapes, const contents, and detection logic.
# Throughout Commit 1, defer to this content for any "what should X look like" question.
```

- [ ] **P4: Note key file locations**

```bash
grep -n "pub struct ParsedFile" src/ast.rs | head -3
grep -n "pub const SINK_PATTERNS\|pub const FORMAT_SINKS\|pub const IPC_SOURCE_PATTERNS" src/algorithms/taint.rs
grep -n "pub mod " src/lib.rs | head -10
# Note the line numbers; you'll need them when modifying these files.
```

---

# Commit 1 — Framework detection layer

**Sub-agent dispatch instructions:** This commit lands the framework detection scaffolding. No algorithm output change — pure infrastructure. Tasks 1–9 in order. Commit at the end of Task 9 with the verbatim message in 9.3. Verify `cargo test && cargo fmt --check` green before committing.

---

## Task 1: Create `src/frameworks/mod.rs` with type definitions

**Files:**
- Create: `src/frameworks/mod.rs`

- [ ] **Step 1.1: Read the spec's §2.2, §2.4, §2.6 for type and const shapes**

```bash
sed -n '75,150p' docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md
```

- [ ] **Step 1.2: Write `src/frameworks/mod.rs`**

```rust
//! Framework-aware source/sink/sanitizer detection.
//!
//! Each framework module defines a `pub const SPEC: FrameworkSpec` describing
//! detection signals, source patterns, sinks, and (optional) sanitizers.
//! Detection is per-file and lazy via `ParsedFile::framework()`.
//!
//! See `docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md` §2 for the
//! full design.

use crate::ast::ParsedFile;

pub mod nethttp;
pub mod gin;
pub mod gorilla_mux;

/// Origin classification matching `provenance_slice.rs::Origin` enum.
/// Re-exported here for convenience in framework specs.
pub use crate::algorithms::provenance_slice::Origin;

/// A category that a sink consumes or a sanitizer cleanses.
/// Defined here in Commit 1 (used by FrameworkSpec.sinks); SanitizerCategory
/// is the same enum, fully populated in Commit 3 with all variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SanitizerCategory {
    Xss,
    Sqli,
    Ssrf,
    Deserialization,
    OsCommand,
    PathTraversal,
}

/// One framework's detection + pattern data.
pub struct FrameworkSpec {
    pub name: &'static str,
    pub detect: fn(&ParsedFile) -> bool,
    pub sources: &'static [SourcePattern],
    pub sinks: &'static [SinkPattern],
    pub sanitizers: &'static [SanitizerRecognizer],
}

/// A source pattern: a call expression that produces tainted values.
pub struct SourcePattern {
    /// Identifier path matched against call expressions, e.g. "c.Param", "r.URL.Query".
    pub call_path: &'static str,
    /// Origin classification.
    pub origin: Origin,
    /// Which call argument *receives* taint as a side-effect. `None` = the call result itself
    /// is the tainted value. `Some(i)` = the i-th argument becomes tainted (e.g.,
    /// `c.BindJSON(&v)` taints `&v` at index 0).
    pub taints_arg: Option<usize>,
}

/// A sink pattern: a call expression that consumes tainted values.
pub struct SinkPattern {
    pub call_path: &'static str,
    pub category: SanitizerCategory,
    pub tainted_arg_indices: &'static [usize],
    pub semantic_check: Option<fn(&CallSite) -> bool>,
}

/// A sanitizer recognizer (defined here in Commit 1 as an empty type for forward
/// compatibility; populated in Commit 3 via `src/sanitizers/mod.rs`).
pub struct SanitizerRecognizer {
    pub call_path: &'static str,
    pub category: SanitizerCategory,
    pub semantic_check: Option<fn(&CallSite) -> bool>,
    /// For pattern-pair cleansers (Clean→HasPrefix, Rel→check), the recognizer is the *first
    /// half*; the second-half check name is resolved at suppression time by textual
    /// co-occurrence in the same function body.
    pub paired_check: Option<&'static str>,
}

/// Call-site reflection helper used by `semantic_check` callbacks. Implementation
/// is in `taint.rs` (added in Commit 2); declared here as an opaque type so framework
/// specs can compile in Commit 1 without depending on Commit 2 internals.
pub struct CallSite {
    // Fields populated in Commit 2; opaque for Commit 1.
    _private: (),
}

impl CallSite {
    /// Stub for Commit 1 — full implementation in Commit 2.
    /// Returns the literal string value of argument `i`, if it is a string-literal
    /// expression; `None` otherwise.
    pub fn literal_arg(&self, _i: usize) -> Option<&str> {
        None
    }
}

/// Ordered registry of all known frameworks. Ordering matters: more specific frameworks
/// (gin, gorilla/mux) take precedence over net/http per spec §2.3.
pub const ALL_FRAMEWORKS: &[&'static FrameworkSpec] = &[
    &gin::SPEC,
    &gorilla_mux::SPEC,
    &nethttp::SPEC,
];

/// Detect the active framework for a file. First match wins.
/// Returns `None` if no framework matches (quiet-mode default per ACK §3 Q5).
pub fn detect_for(parsed: &ParsedFile) -> Option<&'static FrameworkSpec> {
    for spec in ALL_FRAMEWORKS {
        if (spec.detect)(parsed) {
            return Some(spec);
        }
    }
    None
}
```

- [ ] **Step 1.3: Verify it compiles (no test yet)**

```bash
cargo check 2>&1 | tail -5
# Expected: errors about missing nethttp/gin/gorilla_mux modules — those land in Tasks 2-4.
# That's fine; we proceed.
```

The file references `nethttp::SPEC`, `gin::SPEC`, `gorilla_mux::SPEC` which don't exist yet. The error is expected. Proceed to Tasks 2–4 to create those.

---

## Task 2: Create `src/frameworks/nethttp.rs`

**Files:**
- Create: `src/frameworks/nethttp.rs`

- [ ] **Step 2.1: Read the spec's §2.6 net/http source list**

```bash
sed -n '/^### 2\.6/,/^### 2\.7/p' docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md
```

- [ ] **Step 2.2: Write `src/frameworks/nethttp.rs`**

```rust
//! Go net/http framework spec.

use super::{FrameworkSpec, SourcePattern, SinkPattern, SanitizerRecognizer, Origin};
use crate::ast::ParsedFile;

pub const SPEC: FrameworkSpec = FrameworkSpec {
    name: "net/http",
    detect,
    sources: SOURCES,
    sinks: SINKS,
    sanitizers: SANITIZERS,
};

/// Detection: import path `"net/http"` plus a corroborating signal — function with
/// parameter typed `*http.Request` OR call to `http.HandleFunc` / `http.Handle`.
fn detect(parsed: &ParsedFile) -> bool {
    let source = parsed.source.as_str();
    if !source.contains("\"net/http\"") {
        return false;
    }
    // Corroborating signal: at least one of these three patterns.
    source.contains("*http.Request")
        || source.contains("http.HandleFunc")
        || source.contains("http.Handle(")
}

const SOURCES: &[SourcePattern] = &[
    SourcePattern { call_path: "r.URL.Query",      origin: Origin::UserInput, taints_arg: None },
    SourcePattern { call_path: "r.FormValue",      origin: Origin::UserInput, taints_arg: None },
    SourcePattern { call_path: "r.PostFormValue",  origin: Origin::UserInput, taints_arg: None },
    SourcePattern { call_path: "r.Header.Get",     origin: Origin::UserInput, taints_arg: None },
    SourcePattern { call_path: "r.Cookie",         origin: Origin::UserInput, taints_arg: None },
    SourcePattern { call_path: "r.URL.Path",       origin: Origin::UserInput, taints_arg: None },
    SourcePattern { call_path: "r.URL.RawQuery",   origin: Origin::UserInput, taints_arg: None },
    SourcePattern { call_path: "r.PathValue",      origin: Origin::UserInput, taints_arg: None },
];

// Empty in Commit 1; populated in Commit 2 with `http.ServeFile` etc.
const SINKS: &[SinkPattern] = &[];

// Empty for Phase 1; reserved for Phase 2/3.
const SANITIZERS: &[SanitizerRecognizer] = &[];
```

Note: the `detect` function uses *textual* import-path matching against the source string. This is intentional per spec §2.3 — a cheap top-of-file scan rather than full AST traversal. Tree-sitter parsing is more rigorous but unnecessary for this signal.

- [ ] **Step 2.3: Confirm compile progresses**

```bash
cargo check 2>&1 | tail -5
# Still expect errors about gin and gorilla_mux modules; mod.rs and nethttp.rs should compile.
```

---

## Task 3: Create `src/frameworks/gin.rs`

**Files:**
- Create: `src/frameworks/gin.rs`

- [ ] **Step 3.1: Read the spec's §2.6 gin source list (already covered in §2.6 read above)**

- [ ] **Step 3.2: Write `src/frameworks/gin.rs`**

```rust
//! Go gin framework spec.

use super::{FrameworkSpec, SourcePattern, SinkPattern, SanitizerRecognizer, Origin};
use crate::ast::ParsedFile;

pub const SPEC: FrameworkSpec = FrameworkSpec {
    name: "gin",
    detect,
    sources: SOURCES,
    sinks: SINKS,
    sanitizers: SANITIZERS,
};

/// Detection: import path `"github.com/gin-gonic/gin"` plus a corroborating signal —
/// function with parameter typed `*gin.Context`.
fn detect(parsed: &ParsedFile) -> bool {
    let source = parsed.source.as_str();
    if !source.contains("\"github.com/gin-gonic/gin\"") {
        return false;
    }
    source.contains("*gin.Context")
}

const SOURCES: &[SourcePattern] = &[
    SourcePattern { call_path: "c.Param",            origin: Origin::UserInput, taints_arg: None },
    SourcePattern { call_path: "c.Query",            origin: Origin::UserInput, taints_arg: None },
    SourcePattern { call_path: "c.PostForm",         origin: Origin::UserInput, taints_arg: None },
    SourcePattern { call_path: "c.GetHeader",        origin: Origin::UserInput, taints_arg: None },
    SourcePattern { call_path: "c.Request.URL.Path", origin: Origin::UserInput, taints_arg: None },
];

// Empty in Commit 1; populated in Commit 2 with `c.File` etc.
const SINKS: &[SinkPattern] = &[];

const SANITIZERS: &[SanitizerRecognizer] = &[];
```

---

## Task 4: Create `src/frameworks/gorilla_mux.rs`

**Files:**
- Create: `src/frameworks/gorilla_mux.rs`

- [ ] **Step 4.1: Write `src/frameworks/gorilla_mux.rs`**

```rust
//! Go gorilla/mux framework spec.

use super::{FrameworkSpec, SourcePattern, SinkPattern, SanitizerRecognizer, Origin};
use crate::ast::ParsedFile;

pub const SPEC: FrameworkSpec = FrameworkSpec {
    name: "gorilla/mux",
    detect,
    sources: SOURCES,
    sinks: SINKS,
    sanitizers: SANITIZERS,
};

/// Detection: import path `"github.com/gorilla/mux"` plus a corroborating signal —
/// call to `mux.Vars(`.
fn detect(parsed: &ParsedFile) -> bool {
    let source = parsed.source.as_str();
    if !source.contains("\"github.com/gorilla/mux\"") {
        return false;
    }
    source.contains("mux.Vars(")
}

const SOURCES: &[SourcePattern] = &[
    SourcePattern { call_path: "mux.Vars", origin: Origin::UserInput, taints_arg: None },
];

// Empty for Phase 1.
const SINKS: &[SinkPattern] = &[];
const SANITIZERS: &[SanitizerRecognizer] = &[];
```

- [ ] **Step 4.2: Verify all framework modules compile**

```bash
cargo check 2>&1 | tail -10
# At this point all four frameworks files exist. Expected errors should now be about
# missing `pub mod frameworks;` in src/lib.rs (Task 5) and the `framework()` accessor
# in src/ast.rs (Task 6). If you see something else, stop and investigate.
```

---

## Task 5: Wire `frameworks` module into `src/lib.rs`

**Files:**
- Modify: `src/lib.rs`

- [ ] **Step 5.1: Read existing `src/lib.rs` module declarations**

```bash
grep -n "^pub mod " src/lib.rs
# Note the alphabetical ordering convention if any.
```

- [ ] **Step 5.2: Add `pub mod frameworks;`**

Edit `src/lib.rs` to add:
```rust
pub mod frameworks;
```

Place it alphabetically among the existing `pub mod` declarations (likely between `data_flow` and `languages` or wherever fits).

- [ ] **Step 5.3: Verify compile progresses**

```bash
cargo check 2>&1 | tail -5
# Expected: only the `framework()` accessor on ParsedFile is missing now (Task 6).
```

---

## Task 6: Add `framework()` accessor on `ParsedFile`

**Files:**
- Modify: `src/ast.rs`

- [ ] **Step 6.1: Read existing ParsedFile struct**

```bash
grep -n "pub struct ParsedFile" src/ast.rs
sed -n "$(grep -n 'pub struct ParsedFile' src/ast.rs | head -1 | cut -d: -f1),+30p" src/ast.rs
```

Note where to add the new field (before the closing brace of the struct).

- [ ] **Step 6.2: Add the field to `ParsedFile`**

In the struct definition, add a new field:

```rust
    /// Lazy framework detection, populated on first call to `framework()`.
    pub framework: std::cell::OnceCell<Option<&'static crate::frameworks::FrameworkSpec>>,
```

- [ ] **Step 6.3: Initialize the field in any `ParsedFile` constructor**

```bash
grep -n "ParsedFile {" src/ast.rs | head -10
```

For each constructor / struct-literal usage, add:

```rust
    framework: std::cell::OnceCell::new(),
```

(`OnceCell::new()` is the empty / not-yet-populated state.)

- [ ] **Step 6.4: Add the `framework()` accessor as an `impl ParsedFile` method**

Find the `impl ParsedFile { ... }` block and add:

```rust
    /// Returns the active framework for this file, detected lazily on first call.
    /// First-match wins per `crate::frameworks::ALL_FRAMEWORKS` ordering.
    /// `None` means no framework matched (quiet mode default).
    pub fn framework(&self) -> Option<&'static crate::frameworks::FrameworkSpec> {
        *self.framework.get_or_init(|| crate::frameworks::detect_for(self))
    }
```

- [ ] **Step 6.5: Verify compile is now clean**

```bash
cargo build 2>&1 | tail -5
# Expected: clean build, no warnings, no errors.
```

If there are errors, the most likely culprit is `OnceCell` import — `std::cell::OnceCell` requires Rust 1.70+. Verify `rustc --version` is ≥ 1.70.

---

## Task 7: Write per-framework detection tests

**Files:**
- Create: `tests/frameworks/mod.rs`
- Create: `tests/frameworks/nethttp_test.rs`
- Create: `tests/frameworks/gin_test.rs`
- Create: `tests/frameworks/gorilla_mux_test.rs`

- [ ] **Step 7.1: Create `tests/frameworks/mod.rs` (shared common helper)**

```rust
// Shared common module for framework tests.
#[path = "../common/mod.rs"]
pub mod common;
```

- [ ] **Step 7.2: Write `tests/frameworks/nethttp_test.rs`**

```rust
#[path = "../common/mod.rs"]
mod common;
use common::*;
use prism::frameworks;

fn parse_go(source: &str) -> ParsedFile {
    ParsedFile::parse("test.go", source, Language::Go).unwrap()
}

#[test]
fn test_nethttp_positive_handler_signature() {
    let source = r#"
package main

import "net/http"

func handler(w http.ResponseWriter, r *http.Request) {
    _, _ = w.Write([]byte("hello"))
}
"#;
    let parsed = parse_go(source);
    assert_eq!(parsed.framework().map(|f| f.name), Some("net/http"));
}

#[test]
fn test_nethttp_positive_handlefunc() {
    let source = r#"
package main

import "net/http"

func main() {
    http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {})
    http.ListenAndServe(":8080", nil)
}
"#;
    let parsed = parse_go(source);
    assert_eq!(parsed.framework().map(|f| f.name), Some("net/http"));
}

#[test]
fn test_nethttp_negative_wrong_import() {
    // Imports something that mentions "net" but isn't net/http.
    let source = r#"
package main

import "net"

func main() {
    _ = net.Dial("tcp", "localhost:8080")
}
"#;
    let parsed = parse_go(source);
    assert_eq!(parsed.framework().map(|f| f.name), None);
}

#[test]
fn test_nethttp_negative_vendored_unused() {
    // Imports net/http but never uses *http.Request, http.HandleFunc, or http.Handle.
    let source = r#"
package main

import (
    _ "net/http"
)

func main() {}
"#;
    let parsed = parse_go(source);
    assert_eq!(parsed.framework().map(|f| f.name), None);
}
```

- [ ] **Step 7.3: Write `tests/frameworks/gin_test.rs`**

```rust
#[path = "../common/mod.rs"]
mod common;
use common::*;
use prism::frameworks;

fn parse_go(source: &str) -> ParsedFile {
    ParsedFile::parse("test.go", source, Language::Go).unwrap()
}

#[test]
fn test_gin_positive_context_param() {
    let source = r#"
package main

import "github.com/gin-gonic/gin"

func handler(c *gin.Context) {
    c.JSON(200, gin.H{"hello": "world"})
}
"#;
    let parsed = parse_go(source);
    assert_eq!(parsed.framework().map(|f| f.name), Some("gin"));
}

#[test]
fn test_gin_negative_wrong_import() {
    let source = r#"
package main

import "github.com/some-other/gin-like-package"

func handler() {}
"#;
    let parsed = parse_go(source);
    assert_eq!(parsed.framework().map(|f| f.name), None);
}

#[test]
fn test_gin_negative_vendored_unused() {
    let source = r#"
package main

import (
    _ "github.com/gin-gonic/gin"
)

func main() {}
"#;
    let parsed = parse_go(source);
    assert_eq!(parsed.framework().map(|f| f.name), None);
}

#[test]
fn test_gin_disambiguation_wins_over_nethttp() {
    // File imports both gin and net/http; gin should win per ordered registry.
    let source = r#"
package main

import (
    "net/http"

    "github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
    c.String(http.StatusOK, "ok")
}
"#;
    let parsed = parse_go(source);
    assert_eq!(parsed.framework().map(|f| f.name), Some("gin"),
        "gin should win over net/http when both are detected");
}
```

- [ ] **Step 7.4: Write `tests/frameworks/gorilla_mux_test.rs`**

```rust
#[path = "../common/mod.rs"]
mod common;
use common::*;
use prism::frameworks;

fn parse_go(source: &str) -> ParsedFile {
    ParsedFile::parse("test.go", source, Language::Go).unwrap()
}

#[test]
fn test_gorilla_mux_positive() {
    let source = r#"
package main

import (
    "net/http"

    "github.com/gorilla/mux"
)

func handler(w http.ResponseWriter, r *http.Request) {
    vars := mux.Vars(r)
    _ = vars["id"]
}
"#;
    let parsed = parse_go(source);
    assert_eq!(parsed.framework().map(|f| f.name), Some("gorilla/mux"));
}

#[test]
fn test_gorilla_mux_negative_no_vars_call() {
    // Imports gorilla/mux but never calls mux.Vars.
    let source = r#"
package main

import (
    _ "github.com/gorilla/mux"
)

func main() {}
"#;
    let parsed = parse_go(source);
    assert_eq!(parsed.framework().map(|f| f.name), None);
}

#[test]
fn test_gorilla_mux_disambiguation_wins_over_nethttp() {
    // File uses both *http.Request and mux.Vars; mux wins per ordered registry.
    let source = r#"
package main

import (
    "net/http"

    "github.com/gorilla/mux"
)

func handler(w http.ResponseWriter, r *http.Request) {
    _ = mux.Vars(r)
}
"#;
    let parsed = parse_go(source);
    assert_eq!(parsed.framework().map(|f| f.name), Some("gorilla/mux"));
}
```

---

## Task 8: Write registry-level tests (disambiguation + no-framework baseline)

**Files:**
- Create: `tests/frameworks/registry_test.rs`

- [ ] **Step 8.1: Write `tests/frameworks/registry_test.rs`**

```rust
#[path = "../common/mod.rs"]
mod common;
use common::*;
use prism::frameworks;

fn parse_go(source: &str) -> ParsedFile {
    ParsedFile::parse("test.go", source, Language::Go).unwrap()
}

#[test]
fn test_no_framework_plain_go_file() {
    // A plain Go file with no web imports. Pins ACK §3 Q5 quiet-mode default.
    let source = r#"
package main

import "fmt"

func main() {
    fmt.Println("hi")
}
"#;
    let parsed = parse_go(source);
    assert_eq!(parsed.framework().map(|f| f.name), None,
        "plain Go file with no web imports should detect no framework");
}

#[test]
fn test_framework_caching_via_oncecell() {
    // Calling framework() twice should return the same value without re-detecting.
    let source = r#"
package main

import "github.com/gin-gonic/gin"

func handler(c *gin.Context) {}
"#;
    let parsed = parse_go(source);
    let first = parsed.framework().map(|f| f.name);
    let second = parsed.framework().map(|f| f.name);
    assert_eq!(first, Some("gin"));
    assert_eq!(first, second);
}

#[test]
fn test_registry_iteration_order() {
    // Sanity-check that ALL_FRAMEWORKS is in the expected order: gin, gorilla/mux, nethttp.
    let names: Vec<&str> = frameworks::ALL_FRAMEWORKS.iter().map(|f| f.name).collect();
    assert_eq!(names, vec!["gin", "gorilla/mux", "net/http"],
        "registry order must be: gin, gorilla/mux, net/http (more-specific first)");
}
```

---

## Task 9: Cargo.toml + coverage_test.rs + final verification + commit

**Files:**
- Modify: `Cargo.toml`
- Modify: `tests/integration/coverage_test.rs`

- [ ] **Step 9.1: Add 4 `[[test]]` entries to `Cargo.toml`**

Find the existing `[[test]]` block list and append (alphabetically or after the existing `algo_*` entries):

```toml
[[test]]
name = "frameworks_nethttp"
path = "tests/frameworks/nethttp_test.rs"

[[test]]
name = "frameworks_gin"
path = "tests/frameworks/gin_test.rs"

[[test]]
name = "frameworks_gorilla_mux"
path = "tests/frameworks/gorilla_mux_test.rs"

[[test]]
name = "frameworks_registry"
path = "tests/frameworks/registry_test.rs"
```

- [ ] **Step 9.2: Update `tests/integration/coverage_test.rs` (3 `all_test_files` arrays)**

Per CLAUDE.md, the file has the `all_test_files` array in three places. All three need the four new test paths:

```rust
"tests/frameworks/nethttp_test.rs",
"tests/frameworks/gin_test.rs",
"tests/frameworks/gorilla_mux_test.rs",
"tests/frameworks/registry_test.rs",
```

Use grep to find all 3 locations:
```bash
grep -n "all_test_files\|test_files" tests/integration/coverage_test.rs
# Expect: line numbers around ~106, ~300, ~431 (per CLAUDE.md note)
```

Add the four entries to each array, near the existing `tests/algo/novel/...` entries (since these are also unit tests). Maintain the array's existing string-formatting style.

- [ ] **Step 9.3: Run all tests**

```bash
cargo test 2>&1 | tail -20
# Expected:
#   - All previous tests still pass (1,406+ baseline)
#   - 4 new test files compile and pass:
#     test result: ok. 4 passed; 0 failed; 0 ignored ... (frameworks_nethttp)
#     test result: ok. 4 passed; 0 failed; 0 ignored ... (frameworks_gin)
#     test result: ok. 3 passed; 0 failed; 0 ignored ... (frameworks_gorilla_mux)
#     test result: ok. 3 passed; 0 failed; 0 ignored ... (frameworks_registry)
#   - integration_coverage tests pass with the 4 new paths registered
```

If any tests fail, debug:
- "framework not detected on positive case": check the `detect` function's textual matching against the test source.
- "wrong framework detected on disambiguation": check `ALL_FRAMEWORKS` order.
- "OnceCell error": confirm Rust ≥ 1.70.

- [ ] **Step 9.4: Confirm fmt + clippy clean**

```bash
cargo fmt --check
# Expected: no output
cargo clippy --all-targets 2>&1 | grep -E "warning|error" | head -5
# Expected: no warnings or errors from new code
```

- [ ] **Step 9.5: Commit**

```bash
git add \
  src/frameworks/ \
  src/lib.rs \
  src/ast.rs \
  Cargo.toml \
  tests/frameworks/ \
  tests/integration/coverage_test.rs

git commit -m "$(cat <<'EOF'
Add framework detection layer (net/http + gin + gorilla/mux)

Phase 1 Commit 1 — pure infrastructure, no algorithm output change.
Lazy OnceCell-cached detection on ParsedFile.framework(); first-match
wins per ALL_FRAMEWORKS ordered [gin, gorilla_mux, nethttp]. Quiet
mode (None) when no framework matches.

Per spec docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md §2.

New types: FrameworkSpec, SourcePattern, SinkPattern,
SanitizerRecognizer (placeholder), SanitizerCategory, CallSite (stub
for Commit 2). Source pattern lists populated for net/http (8
patterns), gin (5), gorilla/mux (1). Sinks empty in Commit 1; populated
in Commit 2.

14 new tests:
  - 4 net/http (positive handler, positive HandleFunc, wrong import,
    vendored unused).
  - 4 gin (positive Context, wrong import, vendored unused,
    disambiguation vs nethttp).
  - 3 gorilla/mux (positive, no Vars call, disambiguation vs nethttp).
  - 3 registry (no-framework baseline, OnceCell caching,
    iteration-order pin).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"

git log -1 --oneline
git status --short
```

Expected: working tree clean. HEAD is the new commit.

**Sub-agent for Commit 1 returns here for review.**

---

# Commit 2 — Sources + sinks (Go CWE-78 + CWE-22)

**Sub-agent dispatch instructions:** This commit hooks framework sources into taint analysis and adds Go CWE-78/22 sinks (cross-cutting + framework-gated). Tasks 10–18. Verify `cargo test && cargo fmt --check` green; commit with the verbatim message in 18.3.

**Pre-flight (Commit 2 sub-agent):**

- [ ] **P5: Confirm prior commit landed**

```bash
git log -2 --oneline
# expect: top commit "Add framework detection layer..."
```

- [ ] **P6: Read the spec sections relevant to Commit 2**

```bash
sed -n '/^### 2\.7/,/^### 2\.8/p' docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md
sed -n '/^### 2\.8/,/^### 2\.9/p' docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md
sed -n '/^### 3\.1/,/^### 3\.4/p' docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md
```

- [ ] **P7: Read existing taint.rs structure**

```bash
grep -n "pub fn slice\|pub const SINK_PATTERNS\|pub const FORMAT_SINKS\|pub const IPC_SOURCE_PATTERNS\|pub fn check_" src/algorithms/taint.rs | head -20
# Note: line numbers and existing const-array shapes. You'll insert new consts near the existing ones.
```

---

## Task 10: Implement `CallSite` reflection helper in `taint.rs`

**Files:**
- Modify: `src/frameworks/mod.rs` (replace stub)
- Modify: `src/algorithms/taint.rs` (add real implementation)

The Commit 1 stub for `CallSite` returns `None` from `literal_arg`. Commit 2 needs a real implementation that inspects tree-sitter call expressions.

- [ ] **Step 10.1: Replace stub in `src/frameworks/mod.rs`**

Find this block:

```rust
pub struct CallSite {
    _private: (),
}

impl CallSite {
    pub fn literal_arg(&self, _i: usize) -> Option<&str> {
        None
    }
}
```

Replace with:

```rust
/// Call-site reflection helper. Wraps a tree-sitter call expression node + the
/// originating source so `semantic_check` callbacks can inspect literal arguments.
pub struct CallSite<'a> {
    pub call_node: tree_sitter::Node<'a>,
    pub source: &'a str,
}

impl<'a> CallSite<'a> {
    /// Returns the literal string value of argument `i`, if it is a string-literal
    /// expression; `None` if the argument is non-literal (variable, expression, etc.)
    /// or out of range.
    pub fn literal_arg(&self, i: usize) -> Option<&'a str> {
        let args = self.call_node.child_by_field_name("arguments")?;
        let mut cursor = args.walk();
        let mut idx = 0;
        for child in args.named_children(&mut cursor) {
            if child.kind() == "interpreted_string_literal" || child.kind() == "raw_string_literal" {
                if idx == i {
                    let text = child.utf8_text(self.source.as_bytes()).ok()?;
                    // Strip surrounding quotes (and backticks for raw strings).
                    let trimmed = text
                        .trim_start_matches('"').trim_end_matches('"')
                        .trim_start_matches('`').trim_end_matches('`');
                    return Some(trimmed);
                }
            }
            // Skip non-literal expressions (still increments idx).
            if !is_argument_separator(child.kind()) {
                idx += 1;
            }
        }
        None
    }
}

fn is_argument_separator(kind: &str) -> bool {
    matches!(kind, "," | "(" | ")")
}
```

Note: `Node` and `utf8_text` come from `tree_sitter`. The Go grammar's `call_expression` node uses `arguments` as the field name for the arg list.

The lifetime `'a` ties the `CallSite` to the source string; this matches existing tree-sitter-using code in `taint.rs`.

- [ ] **Step 10.2: Verify mod.rs still compiles**

```bash
cargo check 2>&1 | tail -5
# Expected: clean (or only errors about new sink/source consts that arrive in following tasks).
```

---

## Task 11: Add `GO_CWE78_SINKS` const + helper checks to `taint.rs`

**Files:**
- Modify: `src/algorithms/taint.rs`

- [ ] **Step 11.1: Read spec §3.2 for the const definition**

```bash
sed -n '/^### 3\.2/,/^### 3\.3/p' docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md
```

- [ ] **Step 11.2: Add the imports + helpers + const at the top of `taint.rs`**

Near the existing `SINK_PATTERNS` declaration in `src/algorithms/taint.rs`, add:

```rust
use crate::frameworks::{CallSite, SanitizerCategory, SinkPattern};

/// Helper: check that args[0] = "sh"|"bash"|... and args[1] = "-c"|"/c".
/// Used by the shell-wrapper variant of CWE-78 sink patterns.
fn check_shell_wrapper(call: &CallSite) -> bool {
    let arg0 = call.literal_arg(0).unwrap_or("");
    let arg1 = call.literal_arg(1).unwrap_or("");
    matches!(arg0, "sh" | "bash" | "cmd.exe" | "/bin/sh" | "/bin/bash")
        && matches!(arg1, "-c" | "/c")
}

/// Helper: same as `check_shell_wrapper` but for `exec.CommandContext` where args
/// are shifted by 1 (ctx, name, ...).
fn check_shell_wrapper_ctx(call: &CallSite) -> bool {
    let arg1 = call.literal_arg(1).unwrap_or("");
    let arg2 = call.literal_arg(2).unwrap_or("");
    matches!(arg1, "sh" | "bash" | "cmd.exe" | "/bin/sh" | "/bin/bash")
        && matches!(arg2, "-c" | "/c")
}

/// Cross-cutting Go CWE-78 (OS command injection) sinks. See spec §3.2.
pub const GO_CWE78_SINKS: &[SinkPattern] = &[
    // Shell-wrapped variants
    SinkPattern {
        call_path: "exec.Command",
        category: SanitizerCategory::OsCommand,
        tainted_arg_indices: &[2],
        semantic_check: Some(check_shell_wrapper),
    },
    SinkPattern {
        call_path: "exec.CommandContext",
        category: SanitizerCategory::OsCommand,
        tainted_arg_indices: &[3],
        semantic_check: Some(check_shell_wrapper_ctx),
    },
    // Tainted-binary variants
    SinkPattern {
        call_path: "exec.Command",
        category: SanitizerCategory::OsCommand,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "exec.CommandContext",
        category: SanitizerCategory::OsCommand,
        tainted_arg_indices: &[1],
        semantic_check: None,
    },
    // syscall.Exec
    SinkPattern {
        call_path: "syscall.Exec",
        category: SanitizerCategory::OsCommand,
        tainted_arg_indices: &[0, 1],
        semantic_check: None,
    },
];
```

- [ ] **Step 11.3: Verify it compiles**

```bash
cargo check 2>&1 | tail -5
```

Errors will appear about `GO_CWE78_SINKS` not being consulted yet — that's Task 14. Continue to Task 12 first to add CWE-22 sinks.

---

## Task 12: Add `GO_CWE22_SINKS` const

**Files:**
- Modify: `src/algorithms/taint.rs`

- [ ] **Step 12.1: Read spec §3.3 for the const definition**

```bash
sed -n '/^### 3\.3/,/^### 3\.4/p' docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md
```

- [ ] **Step 12.2: Add `GO_CWE22_SINKS` after `GO_CWE78_SINKS`**

Append to `src/algorithms/taint.rs`:

```rust
/// Cross-cutting Go CWE-22 (path traversal) sinks. See spec §3.3.
pub const GO_CWE22_SINKS: &[SinkPattern] = &[
    // Read sinks
    SinkPattern { call_path: "os.Open",          category: SanitizerCategory::PathTraversal, tainted_arg_indices: &[0], semantic_check: None },
    SinkPattern { call_path: "os.OpenFile",      category: SanitizerCategory::PathTraversal, tainted_arg_indices: &[0], semantic_check: None },
    SinkPattern { call_path: "os.ReadFile",      category: SanitizerCategory::PathTraversal, tainted_arg_indices: &[0], semantic_check: None },
    SinkPattern { call_path: "ioutil.ReadFile",  category: SanitizerCategory::PathTraversal, tainted_arg_indices: &[0], semantic_check: None },
    // Write sinks
    SinkPattern { call_path: "os.Create",        category: SanitizerCategory::PathTraversal, tainted_arg_indices: &[0], semantic_check: None },
    SinkPattern { call_path: "os.WriteFile",     category: SanitizerCategory::PathTraversal, tainted_arg_indices: &[0], semantic_check: None },
    SinkPattern { call_path: "ioutil.WriteFile", category: SanitizerCategory::PathTraversal, tainted_arg_indices: &[0], semantic_check: None },
    // Mutation sinks
    SinkPattern { call_path: "os.Remove",        category: SanitizerCategory::PathTraversal, tainted_arg_indices: &[0], semantic_check: None },
    SinkPattern { call_path: "os.RemoveAll",     category: SanitizerCategory::PathTraversal, tainted_arg_indices: &[0], semantic_check: None },
    SinkPattern { call_path: "os.Mkdir",         category: SanitizerCategory::PathTraversal, tainted_arg_indices: &[0], semantic_check: None },
    SinkPattern { call_path: "os.MkdirAll",      category: SanitizerCategory::PathTraversal, tainted_arg_indices: &[0], semantic_check: None },
    SinkPattern { call_path: "os.Rename",        category: SanitizerCategory::PathTraversal, tainted_arg_indices: &[0, 1], semantic_check: None },
];
```

---

## Task 13: Populate framework-gated sinks (`http.ServeFile`, `c.File`)

**Files:**
- Modify: `src/frameworks/nethttp.rs`
- Modify: `src/frameworks/gin.rs`

- [ ] **Step 13.1: Update `src/frameworks/nethttp.rs` SINKS const**

Replace `const SINKS: &[SinkPattern] = &[];` with:

```rust
const SINKS: &[SinkPattern] = &[
    SinkPattern {
        call_path: "http.ServeFile",
        category: super::SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[2],  // ServeFile(w, r, name)
        semantic_check: None,
    },
];
```

- [ ] **Step 13.2: Update `src/frameworks/gin.rs` SINKS const**

Replace `const SINKS: &[SinkPattern] = &[];` with:

```rust
const SINKS: &[SinkPattern] = &[
    SinkPattern {
        call_path: "c.File",
        category: super::SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
];
```

`gorilla_mux.rs` SINKS stays empty.

- [ ] **Step 13.3: Verify compile**

```bash
cargo check 2>&1 | tail -5
```

---

## Task 14: Hook framework sources + new structured sinks into `taint.rs` analysis pass

**Files:**
- Modify: `src/algorithms/taint.rs`

This task is the core integration. The implementer must read the existing `taint.rs::slice` function carefully to understand where to weave in:
1. Source detection: when scanning a function for taint sources, also consult `parsed.framework()` and apply matching `SourcePattern` rules.
2. Sink detection: when matching a call against sinks, also check `GO_CWE78_SINKS`, `GO_CWE22_SINKS`, and the active framework's `SINKS`.

- [ ] **Step 14.1: Locate the existing source-inference logic**

```bash
grep -n "fn.*taint\|source\|sink" src/algorithms/taint.rs | head -30
```

Identify the function(s) that:
- Iterate function parameters / variable declarations to find sources.
- Iterate call expressions to find sinks.

- [ ] **Step 14.2: Add framework-aware source inference**

In the source-inference function, after existing logic, add:

```rust
// Framework-aware source detection (Phase 1 Go).
if let Some(spec) = parsed.framework() {
    // Bind the parameter name(s) of the framework's expected request type.
    // For net/http: parameter typed `*http.Request` (commonly named `r`).
    // For gin: parameter typed `*gin.Context` (commonly named `c`).
    // For gorilla/mux: same as net/http (gorilla/mux uses *http.Request too).
    let request_param_names = collect_request_param_names(parsed, function);
    for source_pat in spec.sources {
        for param_name in &request_param_names {
            // Substitute the bound name into the source pattern's `call_path`.
            // E.g., pattern "r.URL.Query" → "{param_name}.URL.Query" if call_path
            // starts with the framework's conventional prefix ("r." or "c.").
            let concrete_path = substitute_prefix(source_pat.call_path, param_name);
            // Match the concrete path against call expressions in the function body.
            for call_node in find_calls_matching(function_body, &concrete_path) {
                // Mark the call result (or the indicated arg per `taints_arg`) as tainted.
                emit_source(/* ... */);
            }
        }
    }
}
```

The functions `collect_request_param_names`, `substitute_prefix`, `find_calls_matching`, `emit_source` are sketches — the implementer adapts to existing taint.rs internals. Exact names will follow taint.rs conventions.

**Implementation notes:**
- `collect_request_param_names`: iterate function parameters from tree-sitter, return all parameter names whose type is `*http.Request` (for net/http and gorilla/mux) or `*gin.Context` (for gin). Per spec §2.6 — bind ALL matching params, not just the first.
- `substitute_prefix`: if `call_path` is `"r.URL.Query"` and `param_name` is `"req"`, return `"req.URL.Query"`. If `call_path` is `"mux.Vars"`, return as-is (mux.Vars takes the request as an arg, not a method receiver).
- `find_calls_matching`: scan call expressions in the function body for ones whose callee text matches the concrete path.
- `emit_source`: existing taint.rs primitive for marking a value as tainted.

- [ ] **Step 14.3: Add new structured sink consultation**

In the sink-matching function, after existing `SINK_PATTERNS` consultation, add:

```rust
// Cross-cutting Go sinks (Phase 1).
if parsed.language == Language::Go {
    for sink_pat in GO_CWE78_SINKS {
        if call_path_matches(call_node, sink_pat.call_path) {
            if let Some(check) = sink_pat.semantic_check {
                let cs = CallSite { call_node: *call_node, source: parsed.source.as_str() };
                if !check(&cs) { continue; }
            }
            for &arg_idx in sink_pat.tainted_arg_indices {
                if is_arg_tainted(call_node, arg_idx) {
                    emit_sink_finding(sink_pat, arg_idx);
                    break;
                }
            }
        }
    }
    for sink_pat in GO_CWE22_SINKS {
        // (same shape as above)
    }
}

// Framework-gated sinks.
if let Some(spec) = parsed.framework() {
    for sink_pat in spec.sinks {
        // (same matching logic as above)
    }
}
```

- [ ] **Step 14.4: Verify compile**

```bash
cargo build 2>&1 | tail -10
```

---

## Task 15: Write integration tests for Commit 2 (CWE-78 + CWE-22)

**Files:**
- Modify: `tests/algo/taxonomy/taint_sink_lang_test.rs`

Find the existing Go section in `taint_sink_lang_test.rs` and append tests for the new sinks. Use the existing `taint_sink_lang` test target (no new Cargo.toml entry needed).

- [ ] **Step 15.1: Read existing taint_sink_lang_test.rs structure**

```bash
grep -n "^#\[test\]\|^fn run_taint" tests/algo/taxonomy/taint_sink_lang_test.rs | head -20
```

Note the helper function shape and existing Go test naming.

- [ ] **Step 15.2: Add CWE-78 tests (per-framework × per-shape)**

Append to `tests/algo/taxonomy/taint_sink_lang_test.rs`:

```rust
// --- Phase 1: CWE-78 (OS command injection) tests ---

#[test]
fn test_taint_cwe78_gin_shell_wrapped() {
    let source = r#"
package main

import (
    "os/exec"

    "github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
    cmd := c.Query("cmd")
    _ = exec.Command("sh", "-c", cmd).Run()
}
"#;
    let result = run_taint_go(source, BTreeSet::from([10]));
    assert!(
        !result.findings.is_empty(),
        "expected CWE-78 finding (gin source → exec.Command shell-wrapped sink)"
    );
}

#[test]
fn test_taint_cwe78_gin_tainted_binary() {
    let source = r#"
package main

import (
    "os/exec"

    "github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
    bin := c.Query("bin")
    _ = exec.Command(bin, "--help").Run()
}
"#;
    let result = run_taint_go(source, BTreeSet::from([10]));
    assert!(
        !result.findings.is_empty(),
        "expected CWE-78 finding (gin source → exec.Command tainted-binary sink)"
    );
}

#[test]
fn test_taint_cwe78_nethttp_shell_wrapped() {
    let source = r#"
package main

import (
    "net/http"
    "os/exec"
)

func handler(w http.ResponseWriter, r *http.Request) {
    cmd := r.URL.Query().Get("cmd")
    _ = exec.Command("bash", "-c", cmd).Run()
}
"#;
    let result = run_taint_go(source, BTreeSet::from([10]));
    assert!(
        !result.findings.is_empty(),
        "expected CWE-78 finding (net/http source → exec.Command bash-c sink)"
    );
}

#[test]
fn test_taint_cwe78_no_finding_literal_safe_form() {
    // exec.Command("ls", "-la") with no taint flow — should NOT fire any sink.
    // Note: in Commit 2 (no sanitizers yet), this passes because no taint reaches the sink.
    // In Commit 3 the cleanser further reinforces the no-finding behavior.
    let source = r#"
package main

import "os/exec"

func main() {
    _ = exec.Command("ls", "-la").Run()
}
"#;
    let result = run_taint_go(source, BTreeSet::from([7]));
    assert!(
        result.findings.is_empty(),
        "exec.Command with literal args and no taint flow should not fire"
    );
}

#[test]
fn test_taint_cwe78_no_finding_when_unrelated_var() {
    // Tainted source exists but is not used in the exec call.
    let source = r#"
package main

import (
    "os/exec"

    "github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
    _ = c.Query("ignored")
    _ = exec.Command("echo", "static").Run()
}
"#;
    let result = run_taint_go(source, BTreeSet::from([12]));
    assert!(
        result.findings.is_empty(),
        "no taint reaches the sink → no finding"
    );
}
```

- [ ] **Step 15.3: Add CWE-22 tests**

Continue appending to the same file:

```rust
// --- Phase 1: CWE-22 (path traversal) tests ---

#[test]
fn test_taint_cwe22_gin_os_readfile() {
    let source = r#"
package main

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
    let result = run_taint_go(source, BTreeSet::from([10]));
    assert!(
        !result.findings.is_empty(),
        "expected CWE-22 finding (gin source → os.ReadFile sink)"
    );
}

#[test]
fn test_taint_cwe22_nethttp_servefile() {
    let source = r#"
package main

import "net/http"

func handler(w http.ResponseWriter, r *http.Request) {
    name := r.URL.Query().Get("name")
    http.ServeFile(w, r, name)
}
"#;
    let result = run_taint_go(source, BTreeSet::from([7]));
    assert!(
        !result.findings.is_empty(),
        "expected CWE-22 finding (net/http source → http.ServeFile framework-gated sink)"
    );
}

#[test]
fn test_taint_cwe22_gin_c_file() {
    let source = r#"
package main

import "github.com/gin-gonic/gin"

func handler(c *gin.Context) {
    name := c.Param("file")
    c.File(name)
}
"#;
    let result = run_taint_go(source, BTreeSet::from([8]));
    assert!(
        !result.findings.is_empty(),
        "expected CWE-22 finding (gin source → c.File framework-gated sink)"
    );
}

#[test]
fn test_taint_cwe22_no_finding_no_taint_flow() {
    // os.Open with literal path, no taint.
    let source = r#"
package main

import "os"

func main() {
    _, _ = os.Open("/etc/passwd")
}
"#;
    let result = run_taint_go(source, BTreeSet::from([7]));
    assert!(
        result.findings.is_empty(),
        "literal path with no taint flow → no CWE-22 finding"
    );
}

#[test]
fn test_taint_cwe22_servefile_outside_nethttp_no_finding() {
    // http.ServeFile call exists, but in a file that doesn't import net/http.
    // The framework gate ensures no spurious finding from the framework-only sink.
    let source = r#"
package main

func handler(w interface{}, r interface{}, name string) {
    // Simulating a misuse — but with no net/http import, framework detection returns None.
}
"#;
    let result = run_taint_go(source, BTreeSet::from([5]));
    assert!(
        result.findings.is_empty(),
        "framework-gated sink should not fire when framework is not detected"
    );
}
```

- [ ] **Step 15.4: Run the new tests**

```bash
cargo test --test algo_taint_sink_lang 2>&1 | tail -20
# Expected: 10 new tests pass
```

If a test fails, debug:
- "no finding fired on positive case": check that the framework `detect` is matching and the source pattern path substitution is correct.
- "finding fired on no-taint case": check that the sink isn't matching purely on the literal call name without checking taint flow.

---

## Task 16: Run full test suite + fmt + clippy

- [ ] **Step 16.1: Full test run**

```bash
cargo test 2>&1 | grep "test result:" | tail -5
# All test result lines should show "ok" with the new tests counted in.
```

- [ ] **Step 16.2: fmt + clippy**

```bash
cargo fmt --check
cargo clippy --all-targets 2>&1 | grep -E "warning|error" | head -5
```

Both should be clean.

---

## Task 17: Commit 2

- [ ] **Step 17.1: Stage files + commit**

```bash
git add \
  src/frameworks/nethttp.rs \
  src/frameworks/gin.rs \
  src/frameworks/mod.rs \
  src/algorithms/taint.rs \
  tests/algo/taxonomy/taint_sink_lang_test.rs

git commit -m "$(cat <<'EOF'
Wire framework sources, add Go CWE-78 + CWE-22 sinks

Phase 1 Commit 2 — meets ACK acceptance criterion #1 (taint fires
on CWE-78 + CWE-22 examples).

Adds GO_CWE78_SINKS (5 patterns: shell-wrapped × 2, tainted-binary × 2,
syscall.Exec) and GO_CWE22_SINKS (12 patterns covering os.Open/Create/
ReadFile/WriteFile/Remove/Mkdir/Rename and ioutil variants) to taint.rs.
Populates framework-gated sinks: net/http http.ServeFile, gin c.File.

Replaces the Commit-1 stub CallSite with real implementation: tree-sitter
call_node + source pair, with literal_arg(i) returning string-literal
arg values for semantic_check helpers.

Hooks framework sources into taint.rs analysis: per spec §2.8 pull
model, when a function has a *http.Request or *gin.Context parameter
(name bound from signature, ALL matching names per §2.6 multiple-param
case), source patterns are substituted with the bound prefix and matched
against call expressions in the function body.

10 new tests (extension to algo_taint_sink_lang target):
  - 3 CWE-78 positive (gin shell-wrap, gin tainted-binary, nethttp bash-c)
  - 2 CWE-78 negative (literal-safe-form, unrelated-tainted-var)
  - 3 CWE-22 positive (gin → os.ReadFile, nethttp → http.ServeFile,
    gin → c.File)
  - 2 CWE-22 negative (no taint, framework-gated outside framework)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"

git log -2 --oneline
git status --short
```

**Sub-agent for Commit 2 returns here for review.**

---

# Commit 3 — Sanitizer registry + cleansed_for + suppression-rate fixture suite

**Sub-agent dispatch instructions:** This commit adds sanitizer recognition + suppression. Tasks 18–28. Verify `cargo test && cargo fmt --check` green; commit with verbatim message in 28.4.

**Pre-flight (Commit 3 sub-agent):**

- [ ] **P8: Confirm prior commits landed**

```bash
git log -3 --oneline
# expect: top commit "Wire framework sources..."
```

- [ ] **P9: Read spec sections relevant to Commit 3**

```bash
sed -n '/^### 3\.4/,/^### 3\.10/p' docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md
sed -n '/^### 3\.10/,/^## 4\./p' docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md
```

- [ ] **P10: Read existing data_flow.rs FlowPath structure**

```bash
grep -n "pub struct FlowPath\|pub struct FlowEdge\|pub fn propagate" src/data_flow.rs | head -10
```

Note the `FlowPath` definition; you'll augment it with `cleansed_for`.

---

## Task 18: Create `src/sanitizers/mod.rs` with types and propagation/suppression hooks

**Files:**
- Create: `src/sanitizers/mod.rs`

- [ ] **Step 18.1: Write the module root**

```rust
//! Category-aware sanitizer registry.
//!
//! Sanitizers cleanse tainted values for specific categories (XSS, SQLi, SSRF,
//! Deserialization, OsCommand, PathTraversal). A `cleansed_for` set on each
//! `FlowPath` tracks which categories a value has been cleansed for; sinks check
//! this set when evaluating suppression.
//!
//! See `docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md` §3.4–§3.9.

pub mod shell;
pub mod path;

pub use crate::frameworks::{SanitizerCategory, SanitizerRecognizer, CallSite};

/// Aggregate all active recognizers across categories. Iteration order is by
/// the const arrays in shell.rs and path.rs.
pub fn active_recognizers() -> impl Iterator<Item = &'static SanitizerRecognizer> {
    shell::SHELL_RECOGNIZERS
        .iter()
        .chain(path::PATH_RECOGNIZERS.iter())
}

/// Check whether a `paired_check` token appears anywhere in the given source slice.
/// Used by paired-check recognizers (e.g., filepath.Clean → strings.HasPrefix).
/// Textual co-occurrence per spec §3.4 / §3.8.
pub fn paired_check_satisfied(function_body_source: &str, check_name: &str) -> bool {
    function_body_source.contains(check_name)
}
```

---

## Task 19: Create `src/sanitizers/shell.rs` (empty for Phase 1)

**Files:**
- Create: `src/sanitizers/shell.rs`

- [ ] **Step 19.1: Write the empty const**

```rust
//! Shell-escape sanitizers (CWE-78 / OsCommand category).
//!
//! Empty in Phase 1 per spec §3.9 — the shell cleanser would be a no-op because
//! no Phase 1 sink consumes `*exec.Cmd`. The const exists for symmetry with
//! `PATH_RECOGNIZERS` and forward-extension; future phases populate it as new
//! sinks consuming cleansed `*exec.Cmd` arrive.

use super::SanitizerRecognizer;

pub const SHELL_RECOGNIZERS: &[SanitizerRecognizer] = &[];
```

---

## Task 20: Create `src/sanitizers/path.rs` with `PATH_RECOGNIZERS`

**Files:**
- Create: `src/sanitizers/path.rs`

- [ ] **Step 20.1: Write the path-validation recognizers**

```rust
//! Path-validation sanitizers (CWE-22 / PathTraversal category).
//!
//! See spec §3.8. Heuristic textual co-occurrence — the recognizer fires when
//! `filepath.Clean(X)` (or `filepath.Rel(base, X)`) is followed by a textual
//! `strings.HasPrefix` call anywhere in the function body. Known limitation:
//! does not distinguish positive vs negative guard direction (Phase 1.5+ for
//! CFG-aware refinement).

use super::SanitizerRecognizer;
use crate::frameworks::SanitizerCategory;

pub const PATH_RECOGNIZERS: &[SanitizerRecognizer] = &[
    SanitizerRecognizer {
        call_path: "filepath.Clean",
        category: SanitizerCategory::PathTraversal,
        semantic_check: None,
        paired_check: Some("strings.HasPrefix"),
    },
    SanitizerRecognizer {
        call_path: "filepath.Rel",
        category: SanitizerCategory::PathTraversal,
        semantic_check: None,
        paired_check: Some("strings.HasPrefix"),
    },
];
```

---

## Task 21: Augment `FlowPath` in `data_flow.rs` with `cleansed_for`

**Files:**
- Modify: `src/data_flow.rs`
- Modify: `src/lib.rs`

- [ ] **Step 21.1: Read existing FlowPath definition**

```bash
grep -n "pub struct FlowPath\b" src/data_flow.rs
sed -n "$(grep -n 'pub struct FlowPath\b' src/data_flow.rs | head -1 | cut -d: -f1),+15p" src/data_flow.rs
```

Note the field list and any constructors / `Default` impls.

- [ ] **Step 21.2: Add `cleansed_for` field**

In the `FlowPath` struct, add a field:

```rust
    /// Sanitizer categories this flow has been cleansed for. A sink's category in
    /// this set causes the finding to be suppressed at evaluation time.
    pub cleansed_for: std::collections::BTreeSet<crate::frameworks::SanitizerCategory>,
```

- [ ] **Step 21.3: Update constructors**

For every place a `FlowPath` is constructed (struct-literal `FlowPath { ... }` or builder pattern), add:

```rust
    cleansed_for: std::collections::BTreeSet::new(),
```

Find them with:
```bash
grep -n "FlowPath {" src/data_flow.rs | head -10
```

- [ ] **Step 21.4: Add `pub mod sanitizers;` to `src/lib.rs`**

After the existing `pub mod frameworks;` line.

- [ ] **Step 21.5: Verify compile**

```bash
cargo build 2>&1 | tail -10
```

Expected: clean.

---

## Task 22: Wire propagation hook in `taint.rs`

**Files:**
- Modify: `src/algorithms/taint.rs`

- [ ] **Step 22.1: Add propagation logic at flow-extension points**

Where the existing taint engine extends a `FlowPath` through a call expression, add:

```rust
// Sanitizer propagation: if this call matches a recognizer, add the recognizer's
// category to the flow's cleansed_for set. Per spec §3.6.
for recognizer in crate::sanitizers::active_recognizers() {
    if !call_path_matches(call_node, recognizer.call_path) {
        continue;
    }
    // Apply semantic_check if present.
    if let Some(check) = recognizer.semantic_check {
        let cs = CallSite { call_node: *call_node, source: parsed.source.as_str() };
        if !check(&cs) { continue; }
    }
    // Apply paired_check if present (textual co-occurrence in function body).
    if let Some(paired) = recognizer.paired_check {
        let func_text = function.utf8_text(parsed.source.as_bytes()).unwrap_or("");
        if !crate::sanitizers::paired_check_satisfied(func_text, paired) {
            continue;
        }
    }
    flow_path.cleansed_for.insert(recognizer.category);
}
```

The exact insertion point depends on existing taint.rs structure — find where `FlowPath` is mutated as a flow extends through a statement/expression.

---

## Task 23: Wire suppression check in `taint.rs` sink evaluation

**Files:**
- Modify: `src/algorithms/taint.rs`

- [ ] **Step 23.1: Add suppression check before emitting a sink finding**

Where the taint engine emits a sink finding (in Commit 2's new sink consultation logic from Task 14), wrap the emit:

```rust
// Suppression check: if the flow has been cleansed for this sink's category,
// suppress the finding. Per spec §3.7.
if flow_path.cleansed_for.contains(&sink_pat.category) {
    // Cleansed: skip this finding.
    continue;
}
emit_sink_finding(sink_pat, arg_idx);
```

- [ ] **Step 23.2: Verify compile**

```bash
cargo build 2>&1 | tail -10
```

---

## Task 24: Per-cleanser unit tests

**Files:**
- Create: `tests/algo/taxonomy/sanitizers_test.rs`

- [ ] **Step 24.1: Write the test file**

```rust
#[path = "../../common/mod.rs"]
mod common;
use common::*;

fn run_taint_go(source: &str, diff_lines: BTreeSet<usize>) -> SliceResult {
    let parsed = ParsedFile::parse("test.go", source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert("test.go".to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "test.go".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines,
        }],
    };
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint);
    algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap()
}

#[test]
fn test_path_clean_with_hasprefix_suppresses() {
    let source = r#"
package main

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
    let result = run_taint_go(source, BTreeSet::from([14]));
    assert!(
        result.findings.is_empty(),
        "filepath.Clean + strings.HasPrefix paired check should suppress finding"
    );
}

#[test]
fn test_path_clean_without_hasprefix_fires() {
    let source = r#"
package main

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
    let result = run_taint_go(source, BTreeSet::from([12]));
    assert!(
        !result.findings.is_empty(),
        "filepath.Clean alone (no HasPrefix paired check) does not suppress"
    );
}

#[test]
fn test_path_fake_cleanser_filepath_base_does_not_suppress() {
    let source = r#"
package main

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
    let result = run_taint_go(source, BTreeSet::from([12]));
    assert!(
        !result.findings.is_empty(),
        "filepath.Base is not a recognized cleanser; finding should fire"
    );
}

#[test]
fn test_path_rel_with_hasprefix_suppresses() {
    let source = r#"
package main

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
    let result = run_taint_go(source, BTreeSet::from([15]));
    assert!(
        result.findings.is_empty(),
        "filepath.Rel + strings.HasPrefix paired check should suppress finding"
    );
}

#[test]
fn test_category_isolation_path_cleanse_does_not_suppress_oscommand() {
    // A path-validation cleanser cleanses for PathTraversal but not OsCommand.
    // Tainted value flowing through filepath.Clean and into exec.Command should
    // still fire a CWE-78 finding.
    let source = r#"
package main

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
    let result = run_taint_go(source, BTreeSet::from([16]));
    assert!(
        !result.findings.is_empty(),
        "PathTraversal cleansing should NOT suppress OsCommand sink finding"
    );
}
```

---

## Task 25: Create `tests/fixtures/sanitizer-suite-go/` 10+10 fixture suite

**Files:**
- Create: `tests/fixtures/sanitizer-suite-go/sanitized/01_path_clean_hasprefix.go` (and 9 more)
- Create: `tests/fixtures/sanitizer-suite-go/unsanitized/01_direct_taint.go` (and 9 more)

The 10+10 suite is the basis for acceptance criterion #2 (≥80% suppression rate).

- [ ] **Step 25.1: Create 10 sanitized fixtures**

Each fixture is a small Go file demonstrating a properly sanitized path. Each should produce **zero** taint findings.

Create `tests/fixtures/sanitizer-suite-go/sanitized/01_path_clean_hasprefix.go`:

```go
package main

import (
    "os"
    "path/filepath"
    "strings"

    "github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
    name := c.Param("file")
    cleaned := filepath.Clean(name)
    if !strings.HasPrefix(cleaned, "/uploads/") {
        c.AbortWithStatus(403)
        return
    }
    data, _ := os.ReadFile(cleaned)
    c.Data(200, "application/octet-stream", data)
}
```

Create `02_filepath_rel_check.go`:

```go
package main

import (
    "os"
    "path/filepath"
    "strings"

    "github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
    name := c.Param("file")
    rel, err := filepath.Rel("/safe", name)
    if err != nil || strings.HasPrefix(rel, "..") {
        c.AbortWithStatus(403)
        return
    }
    data, _ := os.ReadFile(filepath.Join("/safe", rel))
    c.Data(200, "application/octet-stream", data)
}
```

Create `03_clean_then_join_with_check.go`:

```go
package main

import (
    "net/http"
    "os"
    "path/filepath"
    "strings"
)

func handler(w http.ResponseWriter, r *http.Request) {
    name := r.URL.Query().Get("name")
    cleaned := filepath.Clean(name)
    if !strings.HasPrefix(cleaned, "/static") {
        http.Error(w, "forbidden", http.StatusForbidden)
        return
    }
    http.ServeFile(w, r, cleaned)
}
```

Create `04_gin_param_clean.go`:

```go
package main

import (
    "os"
    "path/filepath"
    "strings"

    "github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
    name := c.Param("file")
    cleaned := filepath.Clean(name)
    if !strings.HasPrefix(cleaned, "/data") {
        return
    }
    _, _ = os.Open(cleaned)
}
```

Create `05_gorilla_mux_clean.go`:

```go
package main

import (
    "net/http"
    "os"
    "path/filepath"
    "strings"

    "github.com/gorilla/mux"
)

func handler(w http.ResponseWriter, r *http.Request) {
    vars := mux.Vars(r)
    name := vars["file"]
    cleaned := filepath.Clean(name)
    if !strings.HasPrefix(cleaned, "/uploads") {
        return
    }
    _, _ = os.Open(cleaned)
}
```

Create `06_path_clean_for_write.go`:

```go
package main

import (
    "os"
    "path/filepath"
    "strings"

    "github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
    name := c.Param("file")
    cleaned := filepath.Clean(name)
    if !strings.HasPrefix(cleaned, "/writes") {
        return
    }
    _ = os.WriteFile(cleaned, []byte("data"), 0644)
}
```

Create `07_path_clean_for_remove.go`:

```go
package main

import (
    "os"
    "path/filepath"
    "strings"

    "github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
    name := c.Param("file")
    cleaned := filepath.Clean(name)
    if !strings.HasPrefix(cleaned, "/temp") {
        return
    }
    _ = os.Remove(cleaned)
}
```

Create `08_nethttp_servefile_with_check.go`:

```go
package main

import (
    "net/http"
    "path/filepath"
    "strings"
)

func handler(w http.ResponseWriter, r *http.Request) {
    name := r.URL.Query().Get("name")
    cleaned := filepath.Clean(name)
    if !strings.HasPrefix(cleaned, "/public/") {
        http.NotFound(w, r)
        return
    }
    http.ServeFile(w, r, cleaned)
}
```

Create `09_gin_cfile_with_check.go`:

```go
package main

import (
    "path/filepath"
    "strings"

    "github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
    name := c.Param("file")
    cleaned := filepath.Clean(name)
    if !strings.HasPrefix(cleaned, "/static/") {
        c.AbortWithStatus(404)
        return
    }
    c.File(cleaned)
}
```

Create `10_complex_path_validation.go`:

```go
package main

import (
    "os"
    "path/filepath"
    "strings"

    "github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
    name := c.Query("file")
    base := "/var/uploads"
    cleaned := filepath.Clean(filepath.Join(base, name))
    if !strings.HasPrefix(cleaned, base) {
        c.AbortWithStatus(403)
        return
    }
    _, _ = os.ReadFile(cleaned)
}
```

- [ ] **Step 25.2: Create 10 unsanitized fixtures**

Each fixture demonstrates an unsanitized taint flow that **should** produce a finding.

Create `tests/fixtures/sanitizer-suite-go/unsanitized/01_direct_taint_to_readfile.go`:

```go
package main

import (
    "os"

    "github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
    name := c.Param("file")
    _, _ = os.ReadFile(name)
}
```

Create `02_clean_no_hasprefix.go`:

```go
package main

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
```

Create `03_filepath_base_not_a_cleanser.go`:

```go
package main

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
```

Create `04_nethttp_servefile_no_check.go`:

```go
package main

import "net/http"

func handler(w http.ResponseWriter, r *http.Request) {
    name := r.URL.Query().Get("name")
    http.ServeFile(w, r, name)
}
```

Create `05_gin_cfile_no_check.go`:

```go
package main

import "github.com/gin-gonic/gin"

func handler(c *gin.Context) {
    name := c.Param("file")
    c.File(name)
}
```

Create `06_gorilla_mux_no_check.go`:

```go
package main

import (
    "net/http"
    "os"

    "github.com/gorilla/mux"
)

func handler(w http.ResponseWriter, r *http.Request) {
    vars := mux.Vars(r)
    _, _ = os.Open(vars["file"])
}
```

Create `07_writefile_unsanitized.go`:

```go
package main

import (
    "os"

    "github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
    name := c.Param("file")
    _ = os.WriteFile(name, []byte("data"), 0644)
}
```

Create `08_remove_unsanitized.go`:

```go
package main

import (
    "os"

    "github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
    name := c.Param("file")
    _ = os.Remove(name)
}
```

Create `09_cwe78_shell_wrapped.go`:

```go
package main

import (
    "os/exec"

    "github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
    cmd := c.Query("cmd")
    _ = exec.Command("sh", "-c", cmd).Run()
}
```

Create `10_cwe78_tainted_binary.go`:

```go
package main

import (
    "os/exec"

    "github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
    bin := c.Query("bin")
    _ = exec.Command(bin).Run()
}
```

---

## Task 26: Integration test for ≥80% suppression rate

**Files:**
- Create: `tests/integration/cwe_phase1_suppression_test.rs`

- [ ] **Step 26.1: Write the integration test**

```rust
#[path = "../common/mod.rs"]
mod common;
use common::*;

use std::fs;
use std::path::PathBuf;

fn fixture_dir(subdir: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests/fixtures/sanitizer-suite-go");
    p.push(subdir);
    p
}

fn run_taint_on_file(path: &PathBuf) -> SliceResult {
    let source = fs::read_to_string(path).expect("read fixture");
    let rel = path.file_name().unwrap().to_str().unwrap().to_string();
    let parsed = ParsedFile::parse(&rel, &source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(rel.clone(), parsed);

    // Diff covers all lines — any taint flow in the file should fire.
    let line_count = source.lines().count();
    let diff_lines: BTreeSet<usize> = (1..=line_count).collect();
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: rel,
            modify_type: ModifyType::Modified,
            diff_lines,
        }],
    };
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint);
    algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap()
}

#[test]
fn test_cwe_phase1_suppression_rate_meets_80pct() {
    // Acceptance criterion #2 from ACK §1.

    // Sanitized fixtures: should produce zero findings each.
    let sanitized_dir = fixture_dir("sanitized");
    let mut sanitized_files: Vec<PathBuf> = fs::read_dir(&sanitized_dir)
        .expect("read sanitized dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "go").unwrap_or(false))
        .collect();
    sanitized_files.sort();
    assert_eq!(
        sanitized_files.len(),
        10,
        "expected 10 sanitized fixtures, found {}",
        sanitized_files.len()
    );

    let mut suppressed = 0;
    let mut leaked: Vec<String> = Vec::new();
    for f in &sanitized_files {
        let result = run_taint_on_file(f);
        if result.findings.is_empty() {
            suppressed += 1;
        } else {
            leaked.push(f.file_name().unwrap().to_str().unwrap().to_string());
        }
    }
    assert!(
        suppressed >= 8,
        "≥80% suppression rate required. Got {}/10. Leaks: {:?}",
        suppressed,
        leaked
    );

    // Unsanitized fixtures: every one should produce at least one finding.
    let unsanitized_dir = fixture_dir("unsanitized");
    let mut unsanitized_files: Vec<PathBuf> = fs::read_dir(&unsanitized_dir)
        .expect("read unsanitized dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "go").unwrap_or(false))
        .collect();
    unsanitized_files.sort();
    assert_eq!(
        unsanitized_files.len(),
        10,
        "expected 10 unsanitized fixtures, found {}",
        unsanitized_files.len()
    );

    let mut missed: Vec<String> = Vec::new();
    for f in &unsanitized_files {
        let result = run_taint_on_file(f);
        if result.findings.is_empty() {
            missed.push(f.file_name().unwrap().to_str().unwrap().to_string());
        }
    }
    assert!(
        missed.is_empty(),
        "all unsanitized fixtures must fire. Missed: {:?}",
        missed
    );
}
```

---

## Task 27: Cargo.toml + coverage_test.rs + final verification

**Files:**
- Modify: `Cargo.toml`
- Modify: `tests/integration/coverage_test.rs`

- [ ] **Step 27.1: Add 2 `[[test]]` entries to `Cargo.toml`**

```toml
[[test]]
name = "algo_taxonomy_sanitizers"
path = "tests/algo/taxonomy/sanitizers_test.rs"

[[test]]
name = "integration_cwe_phase1_suppression"
path = "tests/integration/cwe_phase1_suppression_test.rs"
```

- [ ] **Step 27.2: Update `tests/integration/coverage_test.rs` (3 `all_test_files` arrays)**

Add to each of the 3 arrays:

```rust
"tests/algo/taxonomy/sanitizers_test.rs",
"tests/integration/cwe_phase1_suppression_test.rs",
```

- [ ] **Step 27.3: Run all tests**

```bash
cargo test 2>&1 | grep "test result:" | tail -10
```

Expected:
- `algo_taxonomy_sanitizers`: 5 tests pass.
- `integration_cwe_phase1_suppression`: 1 test passes.
- All previous tests still pass.

If `test_cwe_phase1_suppression_rate_meets_80pct` fails:
- Debug which sanitized fixtures leaked. Adjust the heuristic in `paired_check_satisfied` if needed (e.g., scope to nearby statements).
- If unsanitized fixtures missed, the sink consultation isn't matching — debug the call_path matching.

- [ ] **Step 27.4: fmt + clippy**

```bash
cargo fmt --check
cargo clippy --all-targets 2>&1 | grep -E "warning|error" | head -5
```

---

## Task 28: Commit 3

- [ ] **Step 28.1: Stage + commit**

```bash
git add \
  src/sanitizers/ \
  src/lib.rs \
  src/data_flow.rs \
  src/algorithms/taint.rs \
  Cargo.toml \
  tests/algo/taxonomy/sanitizers_test.rs \
  tests/integration/cwe_phase1_suppression_test.rs \
  tests/integration/coverage_test.rs \
  tests/fixtures/sanitizer-suite-go/

git commit -m "$(cat <<'EOF'
Add sanitizer registry: shell-escape + path-validation cleansers

Phase 1 Commit 3 — meets ACK acceptance criterion #2 (≥80% sanitizer
suppression rate on the in-tree 10+10 fixture suite).

New module src/sanitizers/ with SanitizerCategory (shared with
src/frameworks/), SanitizerRecognizer, and the active_recognizers /
paired_check_satisfied helpers. SHELL_RECOGNIZERS empty per spec §3.9
(forward-extension only). PATH_RECOGNIZERS contains filepath.Clean
and filepath.Rel, both with paired_check: Some("strings.HasPrefix")
per spec §3.8.

FlowPath in data_flow.rs augmented with cleansed_for: BTreeSet<
SanitizerCategory> per spec §3.5. Path-sensitive — different flows
to the same use-site can disagree on cleansing status.

taint.rs propagation hook: when a flow extends through a recognizer's
call_path (semantic_check + paired_check satisfied), the recognizer's
category is added to the flow's cleansed_for set. Suppression check
at sink: if the sink's category is in the flow's cleansed_for, the
finding is suppressed.

5 unit tests in algo_taxonomy_sanitizers:
  - filepath.Clean + HasPrefix → suppresses
  - filepath.Clean alone (no HasPrefix) → fires
  - filepath.Base (fake cleanser) → fires
  - filepath.Rel + HasPrefix → suppresses
  - PathTraversal cleanse does NOT suppress OsCommand sink (category
    isolation)

10+10 fixture suite at tests/fixtures/sanitizer-suite-go/ pinned by
integration_cwe_phase1_suppression test asserting ≥80% suppression
on sanitized + 100% firing on unsanitized.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"

git log -3 --oneline
git status --short
```

**Sub-agent for Commit 3 returns here for review.**

---

# Final — push + PR

After all three commits are reviewed and accepted on the feature branch:

- [ ] **F1: Push**

```bash
git push -u origin claude/cwe-phase1-go
```

- [ ] **F2: Create PR**

```bash
gh pr create --title "Phase 1: Go CWE-78/22 + framework detection + sanitizers" --body "$(cat <<'EOF'
## Summary

Phase 1 of the CWE coverage handoff (per [ACK](../../ACK-prism-cwe-coverage-handoff.md) §2). Adds Go CWE-78 + CWE-22 detection with framework-aware sources (net/http + gin + gorilla/mux) and category-aware sanitizer recognition (shell-escape + path-validation).

Three commits:
1. **Framework detection layer** — `src/frameworks/{nethttp,gin,gorilla_mux}.rs` with `FrameworkSpec` const data, lazy `OnceCell` detection on `ParsedFile`, ordered registry (gin > gorilla/mux > nethttp), 14 tests.
2. **Sources + sinks** — `GO_CWE78_SINKS` + `GO_CWE22_SINKS` cross-cutting consts in `taint.rs`, framework-gated sinks (`http.ServeFile`, `c.File`), pull-model framework source hooks, 10 integration tests. Meets acceptance criterion #1.
3. **Sanitizer registry** — `src/sanitizers/{shell,path}.rs` with `SanitizerRecognizer` consts, `cleansed_for: BTreeSet<SanitizerCategory>` on `FlowPath`, propagation + suppression hooks in `taint.rs`, 10+10 fixture suite + integration test pinning ≥80% suppression rate. Meets acceptance criterion #2.

Spec: [`docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md`](../tree/main/docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md). Plan: [`docs/superpowers/plans/2026-04-25-phase1-cwe-go.md`](../tree/main/docs/superpowers/plans/2026-04-25-phase1-cwe-go.md).

## Test plan

- [ ] `cargo test` passes locally — full suite + ~30 new tests.
- [ ] `cargo fmt --check` clean.
- [ ] `cargo clippy --all-targets` no warnings.
- [ ] `cargo run -- --help` still works (no CLI surface change).
- [ ] Manual: open a small Go file with `*gin.Context` parameter; confirm `parsed.framework().map(|f| f.name) == Some("gin")` via a quick test.
- [ ] Coverage matrix: run `python3 scripts/generate_coverage_badges.py` and confirm taint-Go cell stays at `full` or improves.

## Acceptance criteria (ACK §1)

- [x] **#2:** ≥80% sanitizer suppression rate on in-tree 10+10 suite — pinned by `integration_cwe_phase1_suppression`.
- [ ] **#1:** Taint fires on at least one CWE-78 + CWE-22 example in C1 fixtures — validated post-merge by eval team.
- [ ] **#3:** Framework detection activates without per-run config — verified by registry tests.
- [ ] **#4:** D2 coexistence — no Prism-side dedup; same-line both-fire behavior preserved.
- [ ] **#5:** No Tier-1 regression — full test suite passes.

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

After PR merge, file `~/code/slicing/STATUS-prism-cwe-phase1.md` per ACK §6 and notify the eval team.

---

## Self-review notes

### Spec coverage

| Spec section | Implementation task(s) |
|---|---|
| §1 Commit 1 deliverables | Tasks 1–9 |
| §1 Commit 2 deliverables | Tasks 10–17 |
| §1 Commit 3 deliverables | Tasks 18–28 |
| §2.2 FrameworkSpec / SourcePattern / SinkPattern types | Task 1 |
| §2.3 Detection function shape | Tasks 2, 3, 4 |
| §2.4 detect_for dispatch | Task 1 |
| §2.5 OnceCell on ParsedFile | Task 6 |
| §2.6 SourcePattern lists | Tasks 2, 3, 4 |
| §2.7 Framework-gated sinks (Commit 2) | Task 13 |
| §2.8 Pull-model integration | Task 14 |
| §2.9 Commit 1 tests | Tasks 7, 8 |
| §3.2 GO_CWE78_SINKS | Task 11 |
| §3.3 GO_CWE22_SINKS | Task 12 |
| §3.4 SanitizerRecognizer + paired_check resolution | Task 18 |
| §3.5 cleansed_for on FlowPath | Task 21 |
| §3.6 Propagation hook | Task 22 |
| §3.7 Suppression check | Task 23 |
| §3.8 Path-validation recognizer | Task 20 |
| §3.9 Empty SHELL_RECOGNIZERS | Task 19 |
| §3.10 Tests for Commits 2 + 3 | Tasks 15, 24 |
| §4.1 Test strategy + Cargo.toml + 3-copy coverage_test.rs | Tasks 9, 17, 27 |
| §4.2 Branch + PR | Pre-flight, F1, F2 |
| §4.4 STATUS doc | Post-merge note in F2 |

### Placeholder scan

No `TBD`, `TODO`, `FIXME`, `XXX`, or "Similar to Task N" references. Every step that calls for code shows the code. The sketch parts in Task 14 (`collect_request_param_names`, `substitute_prefix`, etc.) are clearly marked as "implementer adapts to existing taint.rs internals" — these are integration adapters that depend on existing private taint.rs structure that the implementer will discover by reading the file.

### Type / identifier consistency

- `FrameworkSpec`, `SourcePattern`, `SinkPattern`, `SanitizerRecognizer`, `SanitizerCategory`, `CallSite`, `Origin` — defined once in `src/frameworks/mod.rs` (Task 1), referenced consistently throughout.
- `ALL_FRAMEWORKS`, `detect_for` — same.
- `GO_CWE78_SINKS`, `GO_CWE22_SINKS`, `check_shell_wrapper`, `check_shell_wrapper_ctx` — defined in Task 11, referenced in Tasks 13, 14, 15.
- `SHELL_RECOGNIZERS`, `PATH_RECOGNIZERS`, `active_recognizers`, `paired_check_satisfied` — defined in Tasks 18–20, referenced in Tasks 22, 23, 24.
- `cleansed_for: BTreeSet<SanitizerCategory>` — added in Task 21, used in Tasks 22, 23, 24.

### Scope check

Three commits, ~28 tasks, ~10–12 days estimate per spec §4.5. Single-PR shape mirrors Phase 0. Self-contained — no spec section unimplemented.

---

*End of plan. After self-review approval, commit this plan to main and hand off to a fresh session for sub-agent execution.*
