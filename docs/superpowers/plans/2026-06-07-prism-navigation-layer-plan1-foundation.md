# Prism Navigation Layer — Plan 1: Foundation + `nodes-at`

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up the additive navigation layer end-to-end — CLI subcommand scaffold (diff-review preserved byte-for-byte), whole-repo loader, the owned `NavigationIndex` with nav-local indexes, the `Evidence` output contract, and the first query (`nodes-at`) runnable as `prism nav nodes-at`.

**Architecture:** Purely additive over the existing CPG (spec §3 "Option C" — zero CPG-core edits). A `LoadedRepo` owns parsed files; a `NavigationIndex` owns the `CodePropertyGraph` + `TypeRegistry` + `live_types` (moved out of a whole-repo `CpgContext::build`) plus two nav-local indexes (`line_range_index`, `name_index`) derived from `CpgNode::Function` data. Queries borrow both via an `Arc`-owned `NavigationSession` and return a uniform `Evidence`.

**Tech Stack:** Rust, clap (derive), tree-sitter (existing `ParsedFile`), serde/serde_json, petgraph (existing CPG), `assert_cmd`/`predicates` (CLI tests).

**Spec:** `docs/superpowers/specs/2026-06-07-prism-navigation-layer-design.md` (v4).

---

## File Structure

| File | Responsibility |
|---|---|
| `src/main.rs` (modify) | `Cli` → optional `Command` subcommand + flattened `ReviewArgs`; route `nav`; review path unchanged |
| `src/repo_loader.rs` (new) | Whole-repo discovery + skip contract → `LoadedRepo` |
| `src/navigation/mod.rs` (new) | `NavigationIndex`, `NavigationSession`, build glue |
| `src/navigation/types.rs` (new) | `Evidence`, `EvidenceItem`, `SymbolRef`, `Reason`, `Source`, `Warning`, `QueryError` serde types |
| `src/navigation/queries.rs` (new) | `nodes_at` query |
| `src/output/navigation.rs` (new) | `Evidence` → text / JSON |
| `src/lib.rs` (modify) | `pub mod repo_loader; pub mod navigation;` |
| `tests/cli/nav_compat_test.rs` (new) | diff-review byte-for-byte golden + nav parse-conflict tests |
| `tests/navigation/*` (new) | loader, index, nodes-at unit/integration tests |

Plan 2 adds `callers`/`callees` + `ego-graph`; Plan 3 adds the exact-hit cache, `module-deps`/`repo-map`, and the MCP adapter.

---

## Task 1: CLI subcommand scaffold (preserve diff-review byte-for-byte)

**Files:**
- Modify: `src/main.rs:36-200` (the `Cli` struct), `src/main.rs:280-360` (dispatch)
- Test: `tests/cli/nav_compat_test.rs` (new), registered in `Cargo.toml`

- [ ] **Step 1: Capture current CLI output as goldens (pre-refactor baseline)**

Create a tiny fixture repo + diff and snapshot today's output. Run:

```bash
mkdir -p tests/fixtures/nav_compat
printf 'def f(x):\n    y = x + 1\n    return y\n' > tests/fixtures/nav_compat/a.py
cat > tests/fixtures/nav_compat/d.json <<'EOF'
{"files":[{"file_path":"a.py","modify_type":"Modified","diff_lines":[2]}]}
EOF
mkdir -p tests/fixtures/nav_compat/golden
cargo run --quiet -- --repo tests/fixtures/nav_compat --diff tests/fixtures/nav_compat/d.json --algorithm review > tests/fixtures/nav_compat/golden/review.txt
cargo run --quiet -- --repo tests/fixtures/nav_compat --diff tests/fixtures/nav_compat/d.json --algorithm leftflow --format json > tests/fixtures/nav_compat/golden/leftflow.json
cargo run --quiet -- --help > tests/fixtures/nav_compat/golden/help.txt 2>&1 || true
cargo run --quiet -- --list-algorithms > tests/fixtures/nav_compat/golden/list.txt
```

- [ ] **Step 2: Write the failing compat test**

```rust
// tests/cli/nav_compat_test.rs
use assert_cmd::Command;

fn bin() -> Command { Command::cargo_bin("prism").unwrap() }
const REPO: &str = "tests/fixtures/nav_compat";
const DIFF: &str = "tests/fixtures/nav_compat/d.json";

fn golden(name: &str) -> String {
    std::fs::read_to_string(format!("tests/fixtures/nav_compat/golden/{name}")).unwrap()
}

#[test]
fn review_output_byte_identical() {
    let out = bin().args(["--repo", REPO, "--diff", DIFF, "--algorithm", "review"]).output().unwrap();
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout), golden("review.txt"));
}

#[test]
fn leftflow_json_byte_identical() {
    let out = bin().args(["--repo", REPO, "--diff", DIFF, "--algorithm", "leftflow", "--format", "json"]).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout), golden("leftflow.json"));
}

#[test]
fn list_algorithms_byte_identical() {
    let out = bin().args(["--list-algorithms"]).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout), golden("list.txt"));
}

#[test]
fn nav_with_review_flag_is_parse_error() {
    // args_conflicts_with_subcommands: review flags under a subcommand fail at parse time.
    let out = bin().args(["nav", "nodes-at", "--repo", REPO, "--diff", DIFF]).output().unwrap();
    assert!(!out.status.success());
}
```

Register in `Cargo.toml`:
```toml
[[test]]
name = "cli_nav_compat"
path = "tests/cli/nav_compat_test.rs"
```

- [ ] **Step 3: Run to verify it fails**

Run: `cargo test --test cli_nav_compat -- nav_with_review_flag_is_parse_error`
Expected: FAIL — `nav` subcommand does not exist yet (currently parsed as a stray value / error differs).

- [ ] **Step 4: Refactor `Cli` — extract `ReviewArgs`, add subcommand**

In `src/main.rs`, move **every existing field** of `Cli` (repo, algorithm, diff, format, … through compile_commands — keep their exact order, `#[arg]` attrs, and doc comments so `--help` is preserved) into a new `ReviewArgs` struct, and replace `Cli` with:

```rust
#[derive(clap::Parser, Debug)]
#[command(
    name = "slicing",
    version = env!("CARGO_PKG_VERSION"),
    about = "Code slicing for defect-focused automated code review (arXiv:2505.17928)",
    args_conflicts_with_subcommands = true,   // review flags + a subcommand are mutually exclusive (parse-time)
    subcommand_negates_reqs = true,           // a subcommand negates ReviewArgs' required_unless_present
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    #[command(flatten)]
    review: ReviewArgs,
}

#[derive(clap::Args, Debug)]
struct ReviewArgs {
    // ... every former Cli field verbatim, same order/attrs ...
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    /// Whole-repo navigation/architecture queries.
    Nav(NavArgs),
}

#[derive(clap::Args, Debug)]
struct NavArgs {
    #[command(subcommand)]
    query: NavQuery,
}

#[derive(clap::Subcommand, Debug)]
enum NavQuery {
    /// CPG nodes at a file:line (plus the enclosing function).
    NodesAt {
        #[arg(long)] repo: std::path::PathBuf,
        /// `file:line`
        #[arg(long)] location: String,
        #[arg(long, default_value = "text")] format: String,
    },
}
```

In `main()`, branch first on the subcommand; the existing review flow moves under the `None` arm unchanged (referencing `cli.review.repo` etc. instead of `cli.repo`):

```rust
let cli = Cli::parse();
match &cli.command {
    // Stub so Task 1 compiles/commits on its own; Task 6 replaces this with run_nav(nav).
    // Unreachable for `nav <q> --diff ...` (that's a parse-time error via args_conflicts_with_subcommands).
    Some(Command::Nav(_nav)) => { eprintln!("nav: not yet implemented"); std::process::exit(2); }
    None => { /* existing review body, reading cli.review.* */ }
}
```

- [ ] **Step 5: Run to verify the goldens still pass**

Run: `cargo test --test cli_nav_compat`
Expected: all PASS. (`review_output_byte_identical`, `leftflow_json_byte_identical`, `list_algorithms_byte_identical` confirm byte-for-byte; `nav_with_review_flag_is_parse_error` confirms the parse-time gate.) If `--help` drift is a concern, add an assertion against `golden/help.txt`; if it drifts, reorder `ReviewArgs` fields to match the original `Cli` order until identical.

- [ ] **Step 6: Commit**

```bash
cargo fmt && cargo test --test cli_nav_compat
git add src/main.rs tests/cli/nav_compat_test.rs tests/fixtures/nav_compat Cargo.toml
git commit -m "feat(nav): add nav subcommand scaffold; lock diff-review compat goldens"
```

---

## Task 2: Whole-repo loader (`repo_loader.rs`)

**Files:**
- Create: `src/repo_loader.rs`, modify `src/lib.rs`
- Test: `tests/navigation/loader_test.rs` (new + `Cargo.toml`)

- [ ] **Step 1: Write the failing test**

```rust
// tests/navigation/loader_test.rs
use prism::repo_loader::{load_repo, SkipReason};

#[test]
fn loads_supported_files_and_records_skips() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::write(root.join("a.py"), "def f():\n    return 1\n").unwrap();
    std::fs::create_dir(root.join("target")).unwrap();
    std::fs::write(root.join("target/b.py"), "def g(): pass\n").unwrap();   // built dir -> skipped
    std::fs::write(root.join("notes.md"), "# hi\n").unwrap();               // unsupported -> skipped

    let repo = load_repo(root).unwrap();
    assert!(repo.files.contains_key("a.py"));
    assert!(!repo.files.contains_key("target/b.py"));
    assert!(repo.file_hashes.contains_key("a.py"));
    assert!(repo.skipped.iter().any(|s| s.path == "notes.md" && s.reason == SkipReason::Unsupported));
    assert!(repo.skipped.iter().any(|s| s.path.starts_with("target/") && s.reason == SkipReason::Ignored));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test navigation_loader`
Expected: FAIL — `prism::repo_loader` does not exist.

- [ ] **Step 3: Implement `repo_loader.rs`**

```rust
// src/repo_loader.rs
use crate::ast::ParsedFile;
use crate::languages::Language;
use crate::type_db::TypeDatabase;
use anyhow::Result;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
const BUILTIN_SKIP_DIRS: &[&str] = &[".git", "target", "node_modules", "vendor", "dist", "build"];

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum SkipReason { Unsupported, Ignored, Symlink, Hidden, TooLarge { bytes: u64 }, Unreadable, NotUtf8, ParseFailed }

#[derive(Debug, Clone, serde::Serialize)]
pub struct SkippedFile { pub path: String, pub reason: SkipReason }

pub struct LoadedRepo {
    pub root: PathBuf,
    pub files: BTreeMap<String, ParsedFile>,
    pub file_hashes: BTreeMap<String, String>,
    pub skipped: Vec<SkippedFile>,
    pub type_db: Option<TypeDatabase>,
}

pub fn load_repo(root: &Path) -> Result<LoadedRepo> {
    let mut files = BTreeMap::new();
    let mut file_hashes = BTreeMap::new();
    let mut skipped = Vec::new();
    // Skip precedence (spec §5): built-in dirs > symlink/hidden > too-large > unreadable/non-utf8 >
    // unsupported ext > parse-failed. Per-file errors never abort (recorded, then continue).
    walk(root, root, &mut files, &mut file_hashes, &mut skipped);
    Ok(LoadedRepo { root: root.to_path_buf(), files, file_hashes, skipped, type_db: None })
}

fn rel(root: &Path, p: &Path) -> String {
    p.strip_prefix(root).unwrap_or(p).to_string_lossy().replace('\\', "/")
}

fn walk(root: &Path, dir: &Path,
        files: &mut BTreeMap<String, ParsedFile>,
        hashes: &mut BTreeMap<String, String>,
        skipped: &mut Vec<SkippedFile>) {
    let entries = match std::fs::read_dir(dir) { Ok(e) => e, Err(_) => return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let meta = match entry.metadata() { Ok(m) => m, Err(_) => {
            skipped.push(SkippedFile { path: rel(root, &path), reason: SkipReason::Unreadable }); continue; } };
        if meta.file_type().is_symlink() {
            skipped.push(SkippedFile { path: rel(root, &path), reason: SkipReason::Symlink }); continue;
        }
        if meta.is_dir() {
            if BUILTIN_SKIP_DIRS.contains(&name.as_str()) {
                skipped.push(SkippedFile { path: rel(root, &path), reason: SkipReason::Ignored }); continue;
            }
            if name.starts_with('.') {
                skipped.push(SkippedFile { path: rel(root, &path), reason: SkipReason::Hidden }); continue;
            }
            walk(root, &path, files, hashes, skipped);
            continue;
        }
        let relp = rel(root, &path);
        let lang = match Language::from_path(&relp) {
            Some(l) => l,
            None => { skipped.push(SkippedFile { path: relp, reason: SkipReason::Unsupported }); continue; }
        };
        if meta.len() > MAX_FILE_BYTES {
            skipped.push(SkippedFile { path: relp, reason: SkipReason::TooLarge { bytes: meta.len() } }); continue;
        }
        let bytes = match std::fs::read(&path) { Ok(b) => b, Err(_) => {
            skipped.push(SkippedFile { path: relp, reason: SkipReason::Unreadable }); continue; } };
        let src = match String::from_utf8(bytes) { Ok(s) => s, Err(_) => {
            skipped.push(SkippedFile { path: relp, reason: SkipReason::NotUtf8 }); continue; } };
        match ParsedFile::parse(&relp, &src, lang) {
            Ok(pf) => {
                let mut h = Sha256::new(); h.update(src.as_bytes());
                hashes.insert(relp.clone(), format!("{:x}", h.finalize()));
                files.insert(relp, pf);
            }
            Err(_) => skipped.push(SkippedFile { path: relp, reason: SkipReason::ParseFailed }),
        }
    }
}
```

Add to `src/lib.rs`: `pub mod repo_loader;`. (`sha2` is already a dependency — see `cpg_cache.rs`.) `.gitignore` honoring is deferred to a follow-up within this task's scope note; the built-in dir set covers the dogfood case. Register the test target in `Cargo.toml`:
```toml
[[test]]
name = "navigation_loader"
path = "tests/navigation/loader_test.rs"
```

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test --test navigation_loader`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt && cargo test --test navigation_loader
git add src/repo_loader.rs src/lib.rs tests/navigation/loader_test.rs Cargo.toml
git commit -m "feat(nav): whole-repo loader with skip-reason contract"
```

---

## Task 3: `NavigationIndex` with nav-local indexes

**Files:**
- Create: `src/navigation/mod.rs`, modify `src/lib.rs`
- Test: `tests/navigation/index_test.rs` (new + `Cargo.toml`)

- [ ] **Step 1: Write the failing test**

```rust
// tests/navigation/index_test.rs
use prism::repo_loader::load_repo;
use prism::navigation::NavigationIndex;

#[test]
fn name_index_keeps_all_same_name_defs() {
    let dir = tempfile::tempdir().unwrap();
    // Two `new` fns in one Rust file across impl blocks (the func_index collision case).
    std::fs::write(dir.path().join("x.rs"),
        "struct A; struct B;\nimpl A { fn new() -> A { A } }\nimpl B { fn new() -> B { B } }\n").unwrap();
    let repo = std::sync::Arc::new(load_repo(dir.path()).unwrap());
    let idx = NavigationIndex::build(&repo);
    let defs = idx.name_index.get(&("x.rs".into(), "new".into())).unwrap();
    assert_eq!(defs.len(), 2, "both `new` defs must be retained, not collapsed");
}

#[test]
fn line_range_index_resolves_innermost() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), "def outer():\n    x = 1\n    return x\n").unwrap();
    let repo = std::sync::Arc::new(load_repo(dir.path()).unwrap());
    let idx = NavigationIndex::build(&repo);
    let f = idx.enclosing_function("a.py", 2).unwrap();   // line 2 is inside `outer`
    assert_eq!(f.1, "outer");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test navigation_index`
Expected: FAIL — `prism::navigation` does not exist.

- [ ] **Step 3: Implement `navigation/mod.rs`**

```rust
// src/navigation/mod.rs
pub mod types;
pub mod queries;

use crate::cpg::{CodePropertyGraph, CpgContext, CpgNode};
use crate::repo_loader::LoadedRepo;
use crate::type_provider::TypeRegistry;       // confirm module path of TypeRegistry
use petgraph::graph::NodeIndex;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub struct NavigationIndex {
    pub cpg: CodePropertyGraph,
    pub types: TypeRegistry,
    pub live_types: BTreeSet<String>,
    // nav-local, derived from CpgNode::Function (spec §3):
    pub line_range_index: BTreeMap<String, Vec<(usize, usize, NodeIndex)>>, // per file, sorted by start
    pub name_index: BTreeMap<(String, String), Vec<NodeIndex>>,             // (file,name) -> all defs
}

pub struct NavigationSession {
    pub repo: Arc<LoadedRepo>,
    pub index: Arc<NavigationIndex>,
}

impl NavigationIndex {
    /// Build a whole-repo index. Uses `CpgContext::build` (scope == None) — never
    /// `build_scoped` (spec §17 Step 3 / R3-M5). Moves the owned cpg/types/live_types
    /// out of the borrowing context so the index owns them.
    pub fn build(repo: &LoadedRepo) -> Self {
        let ctx = CpgContext::build(&repo.files, repo.type_db.as_ref());
        debug_assert!(ctx.scope.is_none(), "nav index must be whole-repo");
        let (mut line_range_index, mut name_index) =
            (BTreeMap::<String, Vec<(usize, usize, NodeIndex)>>::new(),
             BTreeMap::<(String, String), Vec<NodeIndex>>::new());
        for idx in ctx.cpg.node_indices() {
            if let CpgNode::Function { name, file, start_line, end_line } = ctx.cpg.node(idx) {
                line_range_index.entry(file.clone()).or_default().push((*start_line, *end_line, idx));
                name_index.entry((file.clone(), name.clone())).or_default().push(idx);
            }
        }
        for v in line_range_index.values_mut() { v.sort_by_key(|&(s, _, _)| s); }
        NavigationIndex {
            cpg: ctx.cpg, types: ctx.types, live_types: ctx.live_types,
            line_range_index, name_index,
        }
    }

    /// Innermost enclosing function (smallest [start,end] containing `line`).
    pub fn enclosing_function(&self, file: &str, line: usize) -> Option<(NodeIndex, String)> {
        let ranges = self.line_range_index.get(file)?;
        ranges.iter()
            .filter(|&&(s, e, _)| s <= line && line <= e)
            .min_by_key(|&&(s, e, _)| e - s)
            .map(|&(_, _, idx)| {
                let name = match self.cpg.node(idx) { CpgNode::Function { name, .. } => name.clone(), _ => String::new() };
                (idx, name)
            })
    }
}
```

> **Required additive accessor (the one `cpg.rs` edit).** `CodePropertyGraph` exposes no public node iteration, but `name_index` must retain *all* same-name defs (the core of Option C), so add this read-only accessor next to `node()` — purely additive, no logic change, diff-review unaffected:
> ```rust
> // src/cpg.rs (impl CodePropertyGraph)
> pub fn node_indices(&self) -> impl Iterator<Item = petgraph::graph::NodeIndex> + '_ { self.graph.node_indices() }
> ```
> `TypeRegistry` is `crate::type_provider::TypeRegistry` (verified). This single additive accessor is the reason `src/cpg.rs` appears in this task's `git add`; spec §18 should be read as "additive accessor only, no core logic edits."

Add to `src/lib.rs`: `pub mod navigation;`. Register `Cargo.toml`:
```toml
[[test]]
name = "navigation_index"
path = "tests/navigation/index_test.rs"
```

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test --test navigation_index`
Expected: PASS (both same-name defs retained; innermost enclosing resolves).

- [ ] **Step 5: Commit**

```bash
cargo fmt && cargo test --test navigation_index
git add src/navigation/mod.rs src/cpg.rs src/lib.rs tests/navigation/index_test.rs Cargo.toml
git commit -m "feat(nav): NavigationIndex with nav-local line_range/name indexes"
```

---

## Task 4: `Evidence` output contract (`navigation/types.rs`)

**Files:**
- Create: `src/navigation/types.rs`
- Test: `tests/navigation/types_test.rs` (new + `Cargo.toml`)

- [ ] **Step 1: Write the failing test (serde shape lock)**

```rust
// tests/navigation/types_test.rs
use prism::navigation::types::*;

#[test]
fn evidence_serializes_to_expected_shape() {
    let ev = Evidence {
        query: "nodes-at:a.py:2".into(),
        items: vec![EvidenceItem {
            symbol: Some(SymbolRef::Function { file: "a.py".into(), name: "f".into(),
                         start_line: 1, end_line: 3, ordinal: 0 }),
            location: Location { file: "a.py".into(), start_line: 1, end_line: 3 },
            score: 1.0, source: Source::PrismCpg, fallback: false,
            why: vec![Reason::EnclosingFunction { function: SymbolRef::Function {
                file: "a.py".into(), name: "f".into(), start_line: 1, end_line: 3, ordinal: 0 } }],
            snippet: None,
        }],
        truncated: false, warnings: vec![],
    };
    let v: serde_json::Value = serde_json::to_value(&ev).unwrap();
    assert_eq!(v["query"], "nodes-at:a.py:2");
    assert_eq!(v["items"][0]["score"], 1.0);
    assert_eq!(v["items"][0]["source"], "PrismCpg");
    assert_eq!(v["items"][0]["why"][0]["EnclosingFunction"]["function"]["Function"]["name"], "f");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test navigation_types`
Expected: FAIL — `prism::navigation::types` does not exist.

- [ ] **Step 3: Implement `navigation/types.rs`** (verbatim from spec §8)

```rust
// src/navigation/types.rs
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Location { pub file: String, pub start_line: usize, pub end_line: usize }

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum SymbolRef {
    Function { file: String, name: String, start_line: usize, end_line: usize, ordinal: usize },
    Statement { file: String, line: usize, kind: String, ordinal: usize },
    Variable { file: String, function: String, line: usize, path: String, access: String, ordinal: usize },
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum Source { PrismCpg, HeuristicImport, ExternalIndex { name: String } }

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum Reason {
    Calls { callee: String, call_site_line: usize, qualifier: Option<String> },
    CalledBy { caller: String, call_site_line: usize },
    EnclosingFunction { function: SymbolRef },
    Containment { parent: SymbolRef },
    ResolvedImport { module: String, target_file: String },
    UnresolvedImport { module: String },
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EvidenceItem {
    pub symbol: Option<SymbolRef>, pub location: Location, pub score: f32,
    pub source: Source, pub fallback: bool, pub why: Vec<Reason>, pub snippet: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum WarningKind { ParseQuality, AmbiguousSymbol, IndirectCallApprox, UnresolvedModule, Collision, SkippedPath }

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Warning { pub kind: WarningKind, pub message: String, pub location: Option<Location> }

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Evidence {
    pub query: String, pub items: Vec<EvidenceItem>, pub truncated: bool, pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum QueryError {
    AmbiguousSymbol { candidates: Vec<SymbolRef> },
    SymbolNotFound { seed: String },
    LocationOutOfRange { file: String, line: usize },
    UnsupportedFile { file: String },
}
```

Add `pub mod types;` to `src/navigation/mod.rs` (already declared in Task 3). Register `Cargo.toml` `[[test]] name = "navigation_types"`.

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test --test navigation_types`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt && cargo test --test navigation_types
git add src/navigation/types.rs tests/navigation/types_test.rs Cargo.toml
git commit -m "feat(nav): Evidence output contract (serde types)"
```

---

## Task 5: `nodes-at` query (`navigation/queries.rs`)

**Files:**
- Create: `src/navigation/queries.rs`
- Test: `tests/navigation/nodes_at_test.rs` (new + `Cargo.toml`)

- [ ] **Step 1: Write the failing test**

```rust
// tests/navigation/nodes_at_test.rs
use prism::repo_loader::load_repo;
use prism::navigation::{NavigationIndex, NavigationSession, queries};
use prism::navigation::types::{Source, WarningKind};
use std::sync::Arc;

fn session(src: &str, file: &str) -> NavigationSession {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(file), src).unwrap();
    let repo = Arc::new(load_repo(dir.path()).unwrap());
    let index = Arc::new(NavigationIndex::build(&repo));
    NavigationSession { repo, index }
}

#[test]
fn nodes_at_includes_enclosing_function() {
    let s = session("def f():\n    y = 1\n    return y\n", "a.py");
    let ev = queries::nodes_at(&s, "a.py", 2);
    // enclosing function `f` is reported as evidence
    assert!(ev.items.iter().any(|i| matches!(&i.symbol,
        Some(prism::navigation::types::SymbolRef::Function { name, .. }) if name == "f")));
    assert!(ev.items.iter().all(|i| i.source == Source::PrismCpg));
}

#[test]
fn nodes_at_skipped_path_warns_empty() {
    let s = session("def f(): pass\n", "a.py");
    let ev = queries::nodes_at(&s, "missing.py", 1);
    assert!(ev.items.is_empty());
    assert!(ev.warnings.iter().any(|w| matches!(w.kind, WarningKind::SkippedPath)));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test navigation_nodes_at`
Expected: FAIL — `queries::nodes_at` does not exist.

- [ ] **Step 3: Implement `nodes_at`**

```rust
// src/navigation/queries.rs
use crate::cpg::CpgNode;
use crate::navigation::types::*;
use crate::navigation::NavigationSession;

/// Exact CPG nodes at `file:line` (Function/Variable only, spec §8 R3-M3) plus the
/// innermost enclosing function as `EnclosingFunction` evidence.
pub fn nodes_at(s: &NavigationSession, file: &str, line: usize) -> Evidence {
    let query = format!("nodes-at:{file}:{line}");
    if !s.repo.files.contains_key(file) {
        return Evidence { query, items: vec![], truncated: false,
            warnings: vec![Warning { kind: WarningKind::SkippedPath,
                message: format!("file not in nav index: {file}"),
                location: Some(Location { file: file.into(), start_line: line, end_line: line }) }] };
    }
    let mut items = Vec::new();
    for idx in s.index.cpg.nodes_at(file, line) {
        match s.index.cpg.node(idx) {
            CpgNode::Function { name, file: f, start_line, end_line } => items.push(item_fn(f, name, *start_line, *end_line)),
            CpgNode::Variable { path, file: f, function, line: l, access } => items.push(EvidenceItem {
                symbol: Some(SymbolRef::Variable { file: f.clone(), function: function.clone(), line: *l,
                    path: format!("{path:?}"), access: format!("{access:?}"), ordinal: 0 }),
                location: Location { file: f.clone(), start_line: *l, end_line: *l },
                score: 1.0, source: Source::PrismCpg, fallback: false, why: vec![], snippet: None }),
            CpgNode::Statement { .. } => {}   // statements not first-class in v1 (spec §8 R3-M3)
        }
    }
    // Enclosing function (innermost), as evidence on the line.
    if let Some((eidx, _)) = s.index.enclosing_function(file, line) {
        if let CpgNode::Function { name, file: f, start_line, end_line } = s.index.cpg.node(eidx) {
            let func = SymbolRef::Function { file: f.clone(), name: name.clone(),
                start_line: *start_line, end_line: *end_line, ordinal: 0 };
            items.push(EvidenceItem {
                symbol: Some(func.clone()),
                location: Location { file: f.clone(), start_line: line, end_line: line },
                score: 1.0, source: Source::PrismCpg, fallback: false,
                why: vec![Reason::EnclosingFunction { function: func }], snippet: None });
        }
    }
    Evidence { query, items, truncated: false, warnings: vec![] }
}

fn item_fn(file: &str, name: &str, start_line: usize, end_line: usize) -> EvidenceItem {
    let sym = SymbolRef::Function { file: file.into(), name: name.into(), start_line, end_line, ordinal: 0 };
    EvidenceItem { symbol: Some(sym), location: Location { file: file.into(), start_line, end_line },
        score: 1.0, source: Source::PrismCpg, fallback: false, why: vec![], snippet: None }
}
```

Ensure `navigation/mod.rs` re-exports `pub use queries;` (or `pub mod queries;` already present). Register `Cargo.toml` `[[test]] name = "navigation_nodes_at"`.

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test --test navigation_nodes_at`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt && cargo test --test navigation_nodes_at
git add src/navigation/queries.rs src/navigation/mod.rs tests/navigation/nodes_at_test.rs Cargo.toml
git commit -m "feat(nav): nodes-at query (Function/Variable + enclosing)"
```

---

## Task 6: `prism nav nodes-at` CLI + JSON output + dogfood smoke

**Files:**
- Create: `src/output/navigation.rs`, modify `src/main.rs` (`run_nav`), `src/output/mod.rs`
- Test: extend `tests/cli/nav_compat_test.rs`

- [ ] **Step 1: Write the failing test**

```rust
// append to tests/cli/nav_compat_test.rs
#[test]
fn nav_nodes_at_json_on_self() {
    // Dogfood: run against this repo (cargo test cwd = crate root).
    let out = bin().args(["nav", "nodes-at", "--repo", ".", "--location", "src/main.rs:300", "--format", "json"])
        .output().unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v["query"].as_str().unwrap().starts_with("nodes-at:src/main.rs:300"));
    assert!(v["items"].is_array());
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test cli_nav_compat -- nav_nodes_at_json_on_self`
Expected: FAIL — `run_nav` unimplemented / outputs nothing.

- [ ] **Step 3: Implement output + `run_nav`**

```rust
// src/output/navigation.rs
use crate::navigation::types::Evidence;

pub fn render(ev: &Evidence, format: &str) -> String {
    match format {
        "json" => serde_json::to_string_pretty(ev).unwrap_or_else(|_| "{}".into()),
        _ => {  // text
            let mut s = format!("{}\n", ev.query);
            for it in &ev.items {
                s.push_str(&format!("  {}:{}-{}  score={:.2}  {:?}\n",
                    it.location.file, it.location.start_line, it.location.end_line, it.score, it.source));
            }
            for w in &ev.warnings { s.push_str(&format!("  ! {:?}: {}\n", w.kind, w.message)); }
            s
        }
    }
}
```

Add `pub mod navigation;` to `src/output/mod.rs`. In `src/main.rs`, **replace the Task 1 stub arm** with `Some(Command::Nav(nav)) => return run_nav(nav),` and add:

```rust
fn run_nav(nav: &NavArgs) -> anyhow::Result<()> {
    match &nav.query {
        NavQuery::NodesAt { repo, location, format } => {
            let (file, line) = location.rsplit_once(':')
                .and_then(|(f, l)| l.parse::<usize>().ok().map(|n| (f.to_string(), n)))
                .ok_or_else(|| anyhow::anyhow!("--location must be file:line"))?;
            let repo = std::sync::Arc::new(prism::repo_loader::load_repo(repo)?);
            let index = std::sync::Arc::new(prism::navigation::NavigationIndex::build(&repo));
            let session = prism::navigation::NavigationSession { repo, index };
            let ev = prism::navigation::queries::nodes_at(&session, &file, line);
            println!("{}", prism::output::navigation::render(&ev, format));
            Ok(())
        }
    }
}
```

- [ ] **Step 4: Run to verify it passes (and the compat goldens still pass)**

Run: `cargo test --test cli_nav_compat`
Expected: all PASS, including `nav_nodes_at_json_on_self` and the unchanged byte-for-byte goldens.

- [ ] **Step 5: Manual dogfood check**

Run: `cargo run -- nav nodes-at --repo . --location src/cpg.rs:760 --format json`
Expected: JSON `Evidence` with a non-empty `items` array including the enclosing function for that line.

- [ ] **Step 6: Commit**

```bash
cargo fmt && cargo test
git add src/output/navigation.rs src/output/mod.rs src/main.rs tests/cli/nav_compat_test.rs
git commit -m "feat(nav): prism nav nodes-at CLI + JSON output"
```

---

## Done / handoff

After Plan 1: `prism nav nodes-at` runs end-to-end and diff-review is byte-for-byte (compat goldens green). **Plan 2** adds `callers`/`callees` (nav-local qualifier-aware resolution over `CallGraph::CallSite`) and `ego-graph`. **Plan 3** adds the exact-hit nav cache (with the `build.rs` grammar fingerprint), `module-deps`/`repo-map` (labeled, call-derived), and the `rmcp` MCP adapter. The CPG-core `func_index` re-key and qualifier-aware `resolve_callers` remain tracked follow-ups (spec §19), independent of these plans.

## Minor implementer note (cosmetic, resolve at execution)
- `Variable.path`/`Variable.access` strings are rendered via `{:?}` on `AccessPath`/`VarAccess` in Task 5. If `AccessPath` exposes a cleaner `Display`/`to_string()` (check `src/access_path.rs`), prefer it — this only affects the human-readable string in the JSON, not the contract shape (the field stays `String`).
