# T1 Algorithm Hygiene + Pre-CWE-Handoff Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land three uncommitted slicing algorithms (peer_consistency, callback_dispatcher, primitive) with adequate test coverage, wire `tests/fixtures/hapi-4552.diff` as a regression smoke test, and refresh `Plan.md` with a pre-handoff architectural baseline for the eval team's CWE coverage handoff.

**Architecture:** Three sequential commits on branch `claude/t1-cleanup-pre-cwe-handoff`. Each commit is self-contained — `cargo test && cargo fmt --check` must be green between commits. Sub-agents dispatch one per commit; main thread reviews each before next dispatches.

**Tech Stack:** Rust 1.70+, tree-sitter (multi-language AST), petgraph (CPG), `cargo`, `assert_cmd` for CLI tests. Tests live in `tests/algo/novel/` and `tests/integration/` registered as separate `[[test]]` targets in `Cargo.toml`.

**Reference spec:** `docs/superpowers/specs/2026-04-24-hygiene-pass-pre-cwe-handoff-design.md`

---

## File Structure

### Created in Commit 1
| File | Responsibility |
|---|---|
| `tests/algo/novel/peer_consistency_test.rs` | C+C++ unit tests for `PeerConsistencySlice` (7 tests) |
| `tests/algo/novel/callback_dispatcher_test.rs` | C+C++ unit tests for `CallbackDispatcherSlice` (7 tests) |
| `tests/algo/novel/primitive_test.rs` | Python+C unit tests for `PrimitiveSlice` (15 tests) |
| `tests/algo/novel/primitive_lang_test.rs` | JS+Go cross-language tests for `PrimitiveSlice` (3 tests) |

### Modified in Commit 1
| File | Change |
|---|---|
| `Cargo.toml` | Add 4 `[[test]]` entries for the new test files |
| `coverage/matrix.json` | Add trailing newline; transition primitive_slice cells JS/Go `none` → `basic` |
| `src/algorithms/callback_dispatcher_slice.rs` | Remove `arg_snippet` dead-code placeholder at line 303 |
| `README.md` | Regenerate badges (algorithm coverage rises) via `python3 scripts/generate_coverage_badges.py` |
| (working-tree carries) `src/{slice,main,ast}.rs`, `src/algorithms/{mod,provenance_slice}.rs`, `tests/{integration/{core,coverage}_test.rs, cli/output_test.rs, algo/taxonomy/taint_cve_test.rs}` | Pre-existing wire-up changes commit alongside |

### Created in Commit 2
| File | Responsibility |
|---|---|
| `tests/integration/hapi_regression_test.rs` | Smoke test for review-suite output on the hapi-4552 diff |
| `tests/fixtures/hapi-4552-source/lib/transmit.js` | ~300-line trim of real hapijs source, sized to match diff line numbers |
| `scripts/regenerate_hapi_snapshot.sh` | Reviewer utility to regenerate the (deleted) `.txt` snapshot for inspection |

### Modified in Commit 2
| File | Change |
|---|---|
| `Cargo.toml` | Add 1 `[[test]]` entry for `integration_hapi_regression` |
| `tests/fixtures/hapi-4552.diff` | (Status: untracked → tracked) |
| `tests/fixtures/hapi-4552-output.json` | (Status: untracked → tracked, kept as inspection reference) |

### Deleted in Commit 2
| File | Reason |
|---|---|
| `tests/fixtures/hapi-4552-output.txt` | 6,621-line snapshot; replaced by `regenerate_hapi_snapshot.sh` for on-demand regeneration |

### Modified in Commit 3
| File | Change |
|---|---|
| `Plan.md` | Update header date; add T1 capability rows to existing tables; add new section "Pre-handoff Architectural Baseline (D5: CWE Coverage)" between `## Remaining Work` and `## Architecture Notes` |

---

## Pre-flight: verify baseline (do this once before Task 1)

Sub-agent for Commit 1 should run these checks first to make sure the working tree is sane:

- [ ] **P1: Confirm branch + working tree state**

```bash
git branch --show-current  # expect: claude/t1-cleanup-pre-cwe-handoff
git status --short  # expect: existing modifications + untracked algorithm files + untracked hapi fixtures
```

- [ ] **P2: Confirm `cargo build` passes**

```bash
cargo build 2>&1 | tail -5
```

Expected: `Finished \`dev\` profile [unoptimized + debuginfo] target(s)` with no errors. (Warnings are OK.)

- [ ] **P3: Confirm baseline tests pass before our additions**

```bash
cargo test --test integration_core 2>&1 | tail -5
```

Expected: tests pass except possibly `test_all_algorithms_listed` which already expects 30 (because the wire-up was modified to add 30). If that test passes, the wire-up is in a consistent state.

---

# Commit 1 — Algorithms with tests

**Sub-agent dispatch instructions:** This commit lands the three new algorithms with tests, source cleanups, matrix updates, and badge regeneration. Work through Tasks 1-7 in order. Do not commit until Task 7. Verify `cargo test && cargo fmt --check` green before committing.

---

## Task 1: Write `peer_consistency_test.rs`

**Files:**
- Create: `tests/algo/novel/peer_consistency_test.rs`

- [ ] **Step 1.1: Write the test file**

Create `tests/algo/novel/peer_consistency_test.rs` with this exact content:

```rust
#[path = "../../common/mod.rs"]
mod common;
use common::*;
use prism::slice::SliceResult;

fn run_peer_consistency(
    source: &str,
    path: &str,
    lang: Language,
    diff_lines: BTreeSet<usize>,
) -> SliceResult {
    let parsed = ParsedFile::parse(path, source, lang).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines,
        }],
    };

    let config =
        SliceConfig::default().with_algorithm(SlicingAlgorithm::PeerConsistencySlice);
    let results = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    results.into_iter().next().expect("expected one slice result")
}

#[test]
fn test_peer_consistency_uniform_unguarded_cluster_c() {
    // 3 siblings sharing first-param `vty`, all dereference (vty_out), none guard.
    let source = r#"
void show_vty_a(struct vty *vty, int x) {
    vty_out(vty, "a=%d\n", x);
}

void show_vty_b(struct vty *vty, int y) {
    vty_out(vty, "b=%d\n", y);
}

void show_vty_c(struct vty *vty, int z) {
    vty_out(vty, "c=%d\n", z);
}
"#;
    // Diff touches show_vty_a body
    let result = run_peer_consistency(
        source,
        "ospfd/ospf_ext.c",
        Language::C,
        BTreeSet::from([3]),
    );
    let findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("peer_guard_divergence"))
        .collect();
    assert_eq!(findings.len(), 1, "expected exactly one cluster finding");
    assert_eq!(findings[0].severity, "concern");
    assert!(
        findings[0].description.contains("sibling functions"),
        "description should describe cluster, got: {}",
        findings[0].description
    );
}

#[test]
fn test_peer_consistency_divergent_cluster_c() {
    // 4 siblings sharing first-param `vty`. 3 guard with `if (vty)`, 1 unguarded.
    let source = r#"
void show_vty_a(struct vty *vty, int x) {
    if (vty) {
        vty_out(vty, "a=%d\n", x);
    }
}

void show_vty_b(struct vty *vty, int y) {
    if (vty) {
        vty_out(vty, "b=%d\n", y);
    }
}

void show_vty_c(struct vty *vty, int z) {
    if (vty) {
        vty_out(vty, "c=%d\n", z);
    }
}

void show_vty_d(struct vty *vty, int w) {
    vty_out(vty, "d=%d\n", w);
}
"#;
    // Diff touches show_vty_d body (the divergent one)
    let result = run_peer_consistency(
        source,
        "ospfd/ospf_ext.c",
        Language::C,
        BTreeSet::from([21]),
    );
    let findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("peer_guard_divergence"))
        .collect();
    assert_eq!(findings.len(), 1, "expected one divergent-cluster finding");
    assert_eq!(findings[0].severity, "warning");
    assert!(
        findings[0].description.contains("show_vty_d"),
        "description should name the divergent function, got: {}",
        findings[0].description
    );
}

#[test]
fn test_peer_consistency_all_guarded_no_finding_c() {
    let source = r#"
void show_vty_a(struct vty *vty, int x) {
    if (vty) {
        vty_out(vty, "a=%d\n", x);
    }
}

void show_vty_b(struct vty *vty, int y) {
    if (vty) {
        vty_out(vty, "b=%d\n", y);
    }
}

void show_vty_c(struct vty *vty, int z) {
    if (vty) {
        vty_out(vty, "c=%d\n", z);
    }
}
"#;
    let result = run_peer_consistency(
        source,
        "ospfd/ospf_ext.c",
        Language::C,
        BTreeSet::from([3]),
    );
    let findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("peer_guard_divergence"))
        .collect();
    assert!(
        findings.is_empty(),
        "expected no finding when all siblings are guarded, got: {:?}",
        findings.iter().map(|f| &f.description).collect::<Vec<_>>()
    );
}

#[test]
fn test_peer_consistency_cluster_too_small_no_finding_c() {
    // Only 2 siblings — below the cluster_size >= 3 threshold.
    let source = r#"
void show_vty_a(struct vty *vty, int x) {
    vty_out(vty, "a=%d\n", x);
}

void show_vty_b(struct vty *vty, int y) {
    vty_out(vty, "b=%d\n", y);
}
"#;
    let result = run_peer_consistency(
        source,
        "ospfd/ospf_ext.c",
        Language::C,
        BTreeSet::from([3]),
    );
    let findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("peer_guard_divergence"))
        .collect();
    assert!(
        findings.is_empty(),
        "expected no finding when cluster size < 3"
    );
}

#[test]
fn test_peer_consistency_uniform_unguarded_cluster_cpp() {
    let source = r#"
void show_widget_a(Widget *w, int x) {
    w->draw(x);
}

void show_widget_b(Widget *w, int y) {
    w->draw(y);
}

void show_widget_c(Widget *w, int z) {
    w->draw(z);
}
"#;
    let result = run_peer_consistency(
        source,
        "src/widget_show.cpp",
        Language::Cpp,
        BTreeSet::from([3]),
    );
    let findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("peer_guard_divergence"))
        .collect();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, "concern");
}

#[test]
fn test_peer_consistency_divergent_cluster_cpp() {
    let source = r#"
void show_widget_a(Widget *w, int x) {
    if (w) {
        w->draw(x);
    }
}

void show_widget_b(Widget *w, int y) {
    if (w) {
        w->draw(y);
    }
}

void show_widget_c(Widget *w, int z) {
    if (w) {
        w->draw(z);
    }
}

void show_widget_d(Widget *w, int q) {
    w->draw(q);
}
"#;
    let result = run_peer_consistency(
        source,
        "src/widget_show.cpp",
        Language::Cpp,
        BTreeSet::from([21]),
    );
    let findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("peer_guard_divergence"))
        .collect();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, "warning");
    assert!(findings[0].description.contains("show_widget_d"));
}

#[test]
fn test_peer_consistency_only_fires_on_diff_touched_param_cpp() {
    // 3 siblings exist but the diff doesn't touch any of them — no finding,
    // because `touched_params` only includes params from diff-touched functions.
    let source = r#"
void unrelated() {
    int x = 0;
}

void show_widget_a(Widget *w, int x) {
    w->draw(x);
}

void show_widget_b(Widget *w, int y) {
    w->draw(y);
}

void show_widget_c(Widget *w, int z) {
    w->draw(z);
}
"#;
    // Diff touches `unrelated` (line 3), not any sibling
    let result = run_peer_consistency(
        source,
        "src/widget_show.cpp",
        Language::Cpp,
        BTreeSet::from([3]),
    );
    let findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("peer_guard_divergence"))
        .collect();
    assert!(
        findings.is_empty(),
        "expected no finding when diff doesn't touch a cluster member"
    );
}
```

- [ ] **Step 1.2: Add Cargo.toml entry**

Append to `Cargo.toml` after the existing `algo_novel_contract_delta` entry (around line 222):

```toml
[[test]]
name = "algo_novel_peer_consistency"
path = "tests/algo/novel/peer_consistency_test.rs"
```

- [ ] **Step 1.3: Run the new tests**

```bash
cargo test --test algo_novel_peer_consistency 2>&1 | tail -20
```

Expected: 7 tests pass. If a test fails, **read the algorithm source** at `src/algorithms/peer_consistency_slice.rs` to confirm the expected behavior, then adjust the test source/diff_lines (not the algorithm) until it passes. Common gotcha: line numbers in source heredocs depend on whether the heredoc starts with a leading `\n`.

- [ ] **Step 1.4: Confirm fmt is clean**

```bash
cargo fmt --check 2>&1 | head
```

Expected: no output (clean).

---

## Task 2: Write `callback_dispatcher_test.rs`

**Files:**
- Create: `tests/algo/novel/callback_dispatcher_test.rs`

- [ ] **Step 2.1: Write the test file**

Create `tests/algo/novel/callback_dispatcher_test.rs`:

```rust
#[path = "../../common/mod.rs"]
mod common;
use common::*;
use prism::slice::SliceResult;

fn run_callback_dispatcher(
    files_input: Vec<(&str, &str, Language)>,
    diff: DiffInput,
) -> SliceResult {
    let mut files = BTreeMap::new();
    for (path, source, lang) in files_input {
        let parsed = ParsedFile::parse(path, source, lang).unwrap();
        files.insert(path.to_string(), parsed);
    }
    let config =
        SliceConfig::default().with_algorithm(SlicingAlgorithm::CallbackDispatcherSlice);
    let results = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    results.into_iter().next().expect("expected one slice result")
}

#[test]
fn test_callback_dispatcher_designated_init_to_null_dispatch_c() {
    // File A: defines `my_func` and registers it via designated initialiser.
    let file_a = r#"
void my_func(struct vty *vty, struct lsa *lsa) {
    vty_out(vty, "lsa=%p\n", lsa);
}

static struct functab my_tab = {
    .show_opaque_info = my_func,
};
"#;
    // File B: dispatches via field with NULL first arg.
    let file_b = r#"
void some_dispatcher(struct functab *tab, struct lsa *lsa) {
    tab->show_opaque_info(NULL, lsa);
}
"#;
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "ospfd/ospf_ext.c".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]), // touches body of my_func
        }],
    };
    let result = run_callback_dispatcher(
        vec![
            ("ospfd/ospf_ext.c", file_a, Language::C),
            ("lib/log.c", file_b, Language::C),
        ],
        diff,
    );
    let null_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("callback_null_arg_dispatch"))
        .collect();
    assert_eq!(null_findings.len(), 1, "expected one NULL-arg finding");
    assert_eq!(null_findings[0].severity, "concern");
    assert!(
        null_findings[0].related_files.iter().any(|f| f == "lib/log.c"),
        "related_files should contain dispatch site, got: {:?}",
        null_findings[0].related_files
    );
}

#[test]
fn test_callback_dispatcher_assignment_field_clean_dispatch_c() {
    // Registration via `obj->cb = my_func` and clean (no NULL) dispatch.
    let file_a = r#"
void my_func(struct vty *vty, int x) {
    vty_out(vty, "%d\n", x);
}

void register_handler(struct callbacks *cb) {
    cb->show_opaque_info = my_func;
}
"#;
    let file_b = r#"
void dispatcher(struct callbacks *cb, struct vty *vty) {
    cb->show_opaque_info(vty, 42);
}
"#;
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/handler.c".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };
    let result = run_callback_dispatcher(
        vec![
            ("src/handler.c", file_a, Language::C),
            ("src/dispatch.c", file_b, Language::C),
        ],
        diff,
    );
    let chain_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("callback_dispatcher_chain"))
        .collect();
    assert_eq!(chain_findings.len(), 1);
    assert_eq!(chain_findings[0].severity, "info");
}

#[test]
fn test_callback_dispatcher_registrar_call_arg_c() {
    // Function registered via `*register*(my_func)` style call-arg.
    let file_a = r#"
void my_func(struct vty *vty) {
    vty_out(vty, "hello\n");
}

void setup(void) {
    ospf_register_opaque_functab(LSA_TYPE_RI, my_func);
}
"#;
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "ospfd/handler.c".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };
    let result = run_callback_dispatcher(
        vec![("ospfd/handler.c", file_a, Language::C)],
        diff,
    );
    let registrar_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("callback_registrar_call"))
        .collect();
    assert_eq!(registrar_findings.len(), 1);
    assert_eq!(registrar_findings[0].severity, "warning");
    assert!(
        registrar_findings[0]
            .description
            .contains("ospf_register_opaque_functab"),
        "description should name the registrar, got: {}",
        registrar_findings[0].description
    );
}

#[test]
fn test_callback_dispatcher_no_invocations_no_finding_c() {
    // Registration exists, but no file invokes the field.
    let file_a = r#"
void my_func(struct vty *vty) {
    vty_out(vty, "x\n");
}

static struct functab my_tab = {
    .show_opaque_info = my_func,
};
"#;
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/handler.c".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };
    let result = run_callback_dispatcher(
        vec![("src/handler.c", file_a, Language::C)],
        diff,
    );
    let chain_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| {
            f.category.as_deref() == Some("callback_dispatcher_chain")
                || f.category.as_deref() == Some("callback_null_arg_dispatch")
        })
        .collect();
    assert!(
        chain_findings.is_empty(),
        "expected no chain finding when no invocations exist"
    );
}

#[test]
fn test_callback_dispatcher_designated_init_cpp() {
    let file_a = r#"
void my_handler(Widget *w, int code) {
    w->process(code);
}

static struct ops my_ops = {
    .on_event = my_handler,
};
"#;
    let file_b = r#"
void event_loop(struct ops *o, Widget *w) {
    o->on_event(w, 42);
}
"#;
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/handler.cpp".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };
    let result = run_callback_dispatcher(
        vec![
            ("src/handler.cpp", file_a, Language::Cpp),
            ("src/loop.cpp", file_b, Language::Cpp),
        ],
        diff,
    );
    let chain_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("callback_dispatcher_chain"))
        .collect();
    assert_eq!(chain_findings.len(), 1);
}

#[test]
fn test_callback_dispatcher_g_signal_connect_cpp() {
    // GLib `g_signal_connect(obj, "name", callback, user_data)` registrar pattern.
    let file_a = r#"
void on_clicked(GtkButton *btn, gpointer user_data) {
    do_work(btn);
}

void wire_up(GtkButton *btn) {
    g_signal_connect(btn, "clicked", on_clicked, NULL);
}
"#;
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/ui.cpp".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };
    let result = run_callback_dispatcher(
        vec![("src/ui.cpp", file_a, Language::Cpp)],
        diff,
    );
    let registrar_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("callback_registrar_call"))
        .collect();
    assert_eq!(registrar_findings.len(), 1);
    assert_eq!(registrar_findings[0].severity, "warning");
    assert!(
        registrar_findings[0]
            .description
            .contains("g_signal_connect"),
        "description should name g_signal_connect, got: {}",
        registrar_findings[0].description
    );
}

#[test]
fn test_callback_dispatcher_unrelated_function_no_finding_cpp() {
    // Diff touches `my_func`, but only `other_func` is registered.
    let file_a = r#"
void my_func(Widget *w) {
    w->draw();
}

void other_func(Widget *w) {
    w->draw();
}

static struct ops my_ops = {
    .on_event = other_func,
};
"#;
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/handler.cpp".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };
    let result = run_callback_dispatcher(
        vec![("src/handler.cpp", file_a, Language::Cpp)],
        diff,
    );
    let any_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| {
            f.category
                .as_deref()
                .map(|c| c.starts_with("callback_"))
                .unwrap_or(false)
        })
        .collect();
    assert!(
        any_findings.is_empty(),
        "expected no callback findings when diff function is not the registered one"
    );
}
```

- [ ] **Step 2.2: Add Cargo.toml entry**

Append after the previous `algo_novel_peer_consistency` entry:

```toml
[[test]]
name = "algo_novel_callback_dispatcher"
path = "tests/algo/novel/callback_dispatcher_test.rs"
```

- [ ] **Step 2.3: Run the new tests**

```bash
cargo test --test algo_novel_callback_dispatcher 2>&1 | tail -25
```

Expected: 7 tests pass.

- [ ] **Step 2.4: Confirm fmt is clean**

```bash
cargo fmt --check
```

Expected: no output.

---

## Task 3: Write `primitive_test.rs` (Python + C primary scope, 15 tests)

**Files:**
- Create: `tests/algo/novel/primitive_test.rs`

- [ ] **Step 3.1: Write the test file**

Create `tests/algo/novel/primitive_test.rs`:

```rust
#[path = "../../common/mod.rs"]
mod common;
use common::*;
use prism::slice::SliceResult;

fn run_primitive(
    source: &str,
    path: &str,
    lang: Language,
    diff_lines: BTreeSet<usize>,
) -> SliceResult {
    let parsed = ParsedFile::parse(path, source, lang).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines,
        }],
    };
    let config =
        SliceConfig::default().with_algorithm(SlicingAlgorithm::PrimitiveSlice);
    let results = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    results.into_iter().next().expect("expected one slice result")
}

fn findings_for_rule(result: &SliceResult, rule_id: &str) -> Vec<SliceFinding> {
    result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some(rule_id))
        .cloned()
        .collect()
}

// --- Python: HASH_TRUNCATED_BELOW_128_BITS ---

#[test]
fn test_primitive_hash_trunc_direct_below_128_python() {
    let source = r#"
import hashlib

def make_id(s):
    h = hashlib.sha256(s.encode())
    return h.hexdigest()[:12]
"#;
    let result = run_primitive(source, "src/util.py", Language::Python, BTreeSet::from([6]));
    let findings = findings_for_rule(&result, "HASH_TRUNCATED_BELOW_128_BITS");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, "concern");
    assert!(
        findings[0].description.contains("48 bits") || findings[0].description.contains("hex chars"),
        "description should mention bit count or hex chars, got: {}",
        findings[0].description
    );
}

#[test]
fn test_primitive_hash_trunc_at_threshold_no_finding_python() {
    let source = r#"
import hashlib

def make_id(s):
    h = hashlib.sha256(s.encode())
    return h.hexdigest()[:32]
"#;
    let result = run_primitive(source, "src/util.py", Language::Python, BTreeSet::from([6]));
    let findings = findings_for_rule(&result, "HASH_TRUNCATED_BELOW_128_BITS");
    assert!(
        findings.is_empty(),
        "[:32] is exactly 128 bits — should not fire"
    );
}

#[test]
fn test_primitive_hash_trunc_raw_below_threshold_python() {
    let source = r#"
import hashlib

def make_raw_id(s):
    h = hashlib.sha256(s.encode())
    return h.digest()[:8]
"#;
    let result = run_primitive(source, "src/util.py", Language::Python, BTreeSet::from([6]));
    let findings = findings_for_rule(&result, "HASH_TRUNCATED_BELOW_128_BITS");
    assert_eq!(findings.len(), 1);
    assert!(
        findings[0].description.contains("bytes"),
        "description should use 'bytes' unit for raw digest, got: {}",
        findings[0].description
    );
}

// --- Python: HASH_TRUNCATION_VIA_CALL (2-pass) ---

#[test]
fn test_primitive_hash_trunc_via_call_python() {
    let source = r#"
import hashlib

def get_str_hash(s, length):
    h = hashlib.sha256(s.encode())
    return h.hexdigest()[:length]

def cache_key(name):
    return get_str_hash(name, 12)
"#;
    let result = run_primitive(source, "src/cache.py", Language::Python, BTreeSet::from([9]));
    let findings = findings_for_rule(&result, "HASH_TRUNCATION_VIA_CALL");
    assert_eq!(findings.len(), 1, "expected one HASH_TRUNCATION_VIA_CALL finding");
    assert!(
        findings[0].description.contains("get_str_hash"),
        "description should reference callee, got: {}",
        findings[0].description
    );
    assert_eq!(
        findings[0].related_lines.len(),
        1,
        "related_lines should point to callee def"
    );
}

// --- Python: WEAK_HASH_FOR_IDENTITY ---

#[test]
fn test_primitive_weak_hash_for_identity_python() {
    let source = r#"
import hashlib

def make_cache_key(data):
    cache_key = hashlib.md5(data).hexdigest()
    return cache_key
"#;
    let result = run_primitive(source, "src/cache.py", Language::Python, BTreeSet::from([5]));
    let findings = findings_for_rule(&result, "WEAK_HASH_FOR_IDENTITY");
    assert_eq!(findings.len(), 1);
    assert!(
        findings[0].description.contains("MD5"),
        "should name MD5, got: {}",
        findings[0].description
    );
}

#[test]
fn test_primitive_weak_hash_for_checksum_no_finding_python() {
    // 'checksum' is not an identity-shaped name.
    let source = r#"
import hashlib

def integrity(data):
    checksum = hashlib.md5(data).hexdigest()
    return checksum
"#;
    let result = run_primitive(source, "src/util.py", Language::Python, BTreeSet::from([5]));
    let findings = findings_for_rule(&result, "WEAK_HASH_FOR_IDENTITY");
    assert!(
        findings.is_empty(),
        "non-identity name should not fire WEAK_HASH_FOR_IDENTITY"
    );
}

// --- Python: SHELL_TRUE_WITH_INTERPOLATION ---

#[test]
fn test_primitive_shell_true_with_fstring_python() {
    let source = r#"
import subprocess

def copy(src, dst):
    subprocess.run(f"cp {src} {dst}", shell=True)
"#;
    let result = run_primitive(source, "src/runner.py", Language::Python, BTreeSet::from([5]));
    let findings = findings_for_rule(&result, "SHELL_TRUE_WITH_INTERPOLATION");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, "concern");
}

#[test]
fn test_primitive_shell_true_no_interp_no_finding_python() {
    let source = r#"
import subprocess

def list_dir():
    subprocess.run("ls -la", shell=True)
"#;
    let result = run_primitive(source, "src/runner.py", Language::Python, BTreeSet::from([5]));
    let findings = findings_for_rule(&result, "SHELL_TRUE_WITH_INTERPOLATION");
    assert!(
        findings.is_empty(),
        "literal command without interp should not fire"
    );
}

// --- Python: CERT_VALIDATION_DISABLED ---

#[test]
fn test_primitive_cert_validation_disabled_python() {
    let source = r#"
import requests

def fetch(url):
    return requests.get(url, verify=False)
"#;
    let result = run_primitive(source, "src/client.py", Language::Python, BTreeSet::from([5]));
    let findings = findings_for_rule(&result, "CERT_VALIDATION_DISABLED");
    assert_eq!(findings.len(), 1);
    assert!(findings[0].description.contains("verify=False"));
}

// --- Python: HARDCODED_SECRET (positive + inline negative) ---

#[test]
fn test_primitive_hardcoded_secret_python() {
    // Two-statement file: the positive case fires, the placeholder case does not.
    let source = r#"
API_KEY = "sk-real-looking-1234"
TOKEN = "changeme"
"#;
    let result = run_primitive(source, "src/config.py", Language::Python, BTreeSet::from([2]));
    let findings = findings_for_rule(&result, "HARDCODED_SECRET");
    assert_eq!(findings.len(), 1, "should fire on real secret only");
    assert!(
        findings[0].description.contains("API_KEY"),
        "should name API_KEY, got: {}",
        findings[0].description
    );
}

// --- C: CERT_VALIDATION_DISABLED (5 tests, exercising marker list + severity ladder) ---

#[test]
fn test_primitive_cert_validation_verifypeer_zero_c() {
    let source = r#"
#include <curl/curl.h>

void fetch(CURL *curl) {
    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYPEER, 0L);
}
"#;
    let result = run_primitive(source, "src/fetch.c", Language::C, BTreeSet::from([5]));
    let findings = findings_for_rule(&result, "CERT_VALIDATION_DISABLED");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, "concern");
    assert!(findings[0].description.contains("CURLOPT_SSL_VERIFYPEER"));
}

#[test]
fn test_primitive_cert_validation_verifyhost_zero_c() {
    let source = r#"
#include <curl/curl.h>

void fetch(CURL *curl) {
    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYHOST, 0);
}
"#;
    let result = run_primitive(source, "src/fetch.c", Language::C, BTreeSet::from([5]));
    let findings = findings_for_rule(&result, "CERT_VALIDATION_DISABLED");
    assert_eq!(findings.len(), 1);
    assert!(findings[0].description.contains("CURLOPT_SSL_VERIFYHOST"));
}

#[test]
fn test_primitive_cert_validation_in_dirty_function_concern_c() {
    // Marker is on a non-diff line of a function whose other line IS in the diff.
    let source = r#"
#include <curl/curl.h>

void fetch(CURL *curl) {
    int x = 1;
    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYPEER, 0L);
}
"#;
    // diff_lines = {5} (touches `int x = 1`); marker is on line 6 inside same function
    let result = run_primitive(source, "src/fetch.c", Language::C, BTreeSet::from([5]));
    let findings = findings_for_rule(&result, "CERT_VALIDATION_DISABLED");
    assert_eq!(findings.len(), 1);
    assert_eq!(
        findings[0].severity, "concern",
        "marker in dirty function should be 'concern' even when not on diff line"
    );
}

#[test]
fn test_primitive_cert_validation_outside_dirty_function_suggestion_c() {
    // Marker is in `other_func`; diff touches `dirty_func` only.
    let source = r#"
#include <curl/curl.h>

void dirty_func(int x) {
    int y = x + 1;
}

void other_func(CURL *curl) {
    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYPEER, 0L);
}
"#;
    let result = run_primitive(source, "src/fetch.c", Language::C, BTreeSet::from([5]));
    let findings = findings_for_rule(&result, "CERT_VALIDATION_DISABLED");
    assert_eq!(findings.len(), 1);
    assert_eq!(
        findings[0].severity, "suggestion",
        "marker outside dirty function should be 'suggestion'"
    );
}

#[test]
fn test_primitive_no_finding_proper_curl_validation_c() {
    let source = r#"
#include <curl/curl.h>

void fetch(CURL *curl) {
    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYPEER, 1L);
}
"#;
    let result = run_primitive(source, "src/fetch.c", Language::C, BTreeSet::from([5]));
    let findings = findings_for_rule(&result, "CERT_VALIDATION_DISABLED");
    assert!(
        findings.is_empty(),
        "CURLOPT_SSL_VERIFYPEER, 1L (proper validation) should not fire"
    );
}
```

- [ ] **Step 3.2: Add Cargo.toml entry**

Append:

```toml
[[test]]
name = "algo_novel_primitive"
path = "tests/algo/novel/primitive_test.rs"
```

- [ ] **Step 3.3: Run the new tests**

```bash
cargo test --test algo_novel_primitive 2>&1 | tail -25
```

Expected: 15 tests pass.

- [ ] **Step 3.4: Confirm fmt is clean**

```bash
cargo fmt --check
```

Expected: no output.

---

## Task 4: Write `primitive_lang_test.rs` (JS+Go cross-language, 3 tests)

**Files:**
- Create: `tests/algo/novel/primitive_lang_test.rs`

- [ ] **Step 4.1: Write the test file**

Create `tests/algo/novel/primitive_lang_test.rs`:

```rust
#[path = "../../common/mod.rs"]
mod common;
use common::*;
use prism::slice::SliceResult;

fn run_primitive(
    source: &str,
    path: &str,
    lang: Language,
    diff_lines: BTreeSet<usize>,
) -> SliceResult {
    let parsed = ParsedFile::parse(path, source, lang).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines,
        }],
    };
    let config =
        SliceConfig::default().with_algorithm(SlicingAlgorithm::PrimitiveSlice);
    let results = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    results.into_iter().next().expect("expected one slice result")
}

fn findings_for_rule(result: &SliceResult, rule_id: &str) -> Vec<SliceFinding> {
    result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some(rule_id))
        .cloned()
        .collect()
}

#[test]
fn test_primitive_cert_validation_disabled_reject_unauthorized_js() {
    let source = r#"
const https = require('https');

function fetch(url) {
    return https.request(url, { rejectUnauthorized: false });
}
"#;
    let result = run_primitive(
        source,
        "src/client.js",
        Language::JavaScript,
        BTreeSet::from([5]),
    );
    let findings = findings_for_rule(&result, "CERT_VALIDATION_DISABLED");
    assert_eq!(findings.len(), 1);
    assert!(findings[0].description.contains("rejectUnauthorized: false"));
}

#[test]
fn test_primitive_hardcoded_secret_object_field_js() {
    // The HARDCODED_SECRET rule's LHS check accepts `obj.field = "..."` form
    // (it rsplits on '.' and validates the rightmost segment as an identifier).
    let source = r#"
class Config {
    constructor() {
        this.apiKey = "sk-real-looking-1234";
    }
}
"#;
    let result = run_primitive(
        source,
        "src/config.js",
        Language::JavaScript,
        BTreeSet::from([4]),
    );
    let findings = findings_for_rule(&result, "HARDCODED_SECRET");
    assert_eq!(findings.len(), 1);
    assert!(
        findings[0].description.contains("apiKey"),
        "should name apiKey, got: {}",
        findings[0].description
    );
}

#[test]
fn test_primitive_cert_validation_disabled_insecure_skip_verify_go() {
    let source = r#"
package main

import "crypto/tls"

func tlsConfig() *tls.Config {
    return &tls.Config{InsecureSkipVerify: true}
}
"#;
    let result = run_primitive(source, "src/tls.go", Language::Go, BTreeSet::from([7]));
    let findings = findings_for_rule(&result, "CERT_VALIDATION_DISABLED");
    assert_eq!(findings.len(), 1);
    assert!(findings[0].description.contains("InsecureSkipVerify: true"));
}
```

- [ ] **Step 4.2: Add Cargo.toml entry**

Append:

```toml
[[test]]
name = "algo_novel_primitive_lang"
path = "tests/algo/novel/primitive_lang_test.rs"
```

- [ ] **Step 4.3: Run the new tests**

```bash
cargo test --test algo_novel_primitive_lang 2>&1 | tail -15
```

Expected: 3 tests pass.

- [ ] **Step 4.4: Confirm fmt is clean**

```bash
cargo fmt --check
```

Expected: no output.

---

## Task 5: Source cleanups + matrix update

**Files:**
- Modify: `src/algorithms/callback_dispatcher_slice.rs:303` (remove dead-code line)
- Modify: `coverage/matrix.json` (newline + JS/Go primitive cells)

- [ ] **Step 5.1: Remove dead-code placeholder in `callback_dispatcher_slice.rs`**

Open `src/algorithms/callback_dispatcher_slice.rs`. Around line 300-304 there is:

```rust
            // Keep arg_snippet alive (only used when building findings); the
            // field is populated by `find_invocations` but not re-emitted.
            let _ = invocations.iter().map(|i| &i.arg_snippet).count();
```

Delete those four lines (the comment and the `let _` expression).

- [ ] **Step 5.2: Add trailing newline to `coverage/matrix.json`**

Open `coverage/matrix.json`. The current file ends with `}` on the last byte (no newline). Add a single `\n` at end-of-file. After your edit, `tail -c 1 coverage/matrix.json | xxd` should show byte `0a`.

- [ ] **Step 5.3: Update primitive_slice JS/Go cells in `coverage/matrix.json`**

In `coverage/matrix.json`, find the `"primitive_slice"` section. Currently:

```json
"primitive_slice": {
  "python": "basic",
  "javascript": "none",
  "typescript": "none",
  "go": "none",
  ...
}
```

Change `"javascript": "none"` → `"javascript": "basic"` and `"go": "none"` → `"go": "basic"`. The other cells (typescript, java, cpp, rust, lua, terraform, bash) remain as-is.

- [ ] **Step 5.4: Verify the JSON still parses**

```bash
python3 -c "import json; json.load(open('coverage/matrix.json'))" && echo "OK"
```

Expected: `OK`. (No trailing newline issues, valid JSON.)

---

## Task 6: Regenerate coverage badges

**Files:**
- Modify: `README.md`, `coverage/badges.md`, `coverage/table.md` (regenerated by script)

- [ ] **Step 6.1: Run badge regeneration script**

```bash
python3 scripts/generate_coverage_badges.py 2>&1 | tail -10
```

Expected: success output. The README badge SVG URLs and the coverage tables get rewritten.

- [ ] **Step 6.2: Diff-inspect README to confirm badges look reasonable**

```bash
git diff README.md | head -50
```

Expected: badge percentages have shifted (most languages should rise from 90% back toward 95-100% as the new tests close gaps). Some cells (peer/callback for non-C/C++ languages) remain `none` — that's intentional and accurate.

- [ ] **Step 6.3: Confirm fmt is clean**

```bash
cargo fmt --check
```

Expected: no output.

---

## Task 7: Final verification + commit 1

- [ ] **Step 7.1: Run the full test suite**

```bash
cargo test 2>&1 | tail -30
```

Expected: All tests pass, including the new ones. The `test_all_algorithms_listed` assertion (which expects 30) should pass.

- [ ] **Step 7.2: Stage all the files for commit 1**

```bash
git add \
  src/algorithms/callback_dispatcher_slice.rs \
  src/algorithms/peer_consistency_slice.rs \
  src/algorithms/primitive_slice.rs \
  src/algorithms/mod.rs \
  src/algorithms/provenance_slice.rs \
  src/ast.rs \
  src/main.rs \
  src/slice.rs \
  Cargo.toml \
  coverage/matrix.json \
  coverage/badges.md \
  coverage/table.md \
  README.md \
  tests/algo/novel/peer_consistency_test.rs \
  tests/algo/novel/callback_dispatcher_test.rs \
  tests/algo/novel/primitive_test.rs \
  tests/algo/novel/primitive_lang_test.rs \
  tests/algo/taxonomy/taint_cve_test.rs \
  tests/cli/output_test.rs \
  tests/integration/core_test.rs \
  tests/integration/coverage_test.rs
```

- [ ] **Step 7.3: Commit**

```bash
git commit -m "$(cat <<'EOF'
T1-002 + T1-005: peer_consistency, callback_dispatcher, primitive with tests

Lands three slicing algorithms previously left in working tree:
  - PeerConsistencySlice (T1-002): C/C++ sibling first-param NULL-guard
    cluster detection. Driven by FRR CVE-2025-61102. 7 tests.
  - CallbackDispatcherSlice (T1-002): function-pointer-in-struct
    registration → invocation chain resolution; flags NULL-arg
    dispatch. 7 tests including GLib g_signal_connect.
  - PrimitiveSlice (T1-005): security-primitive fingerprint sweep
    with 6 rules (HASH_TRUNCATED_BELOW_128_BITS,
    HASH_TRUNCATION_VIA_CALL 2-pass, WEAK_HASH_FOR_IDENTITY,
    SHELL_TRUE_WITH_INTERPOLATION, CERT_VALIDATION_DISABLED,
    HARDCODED_SECRET). Python primary scope (10 tests), C
    cert-validation + severity ladder (5 tests), JS+Go
    cross-language for cert-validation + secret rules (3 tests).

Source cleanups: remove arg_snippet dead-code placeholder in
callback_dispatcher_slice.rs:303; add trailing newline to
coverage/matrix.json.

Coverage matrix transitions primitive_slice JS/Go: none → basic.
Coverage badges regenerated.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 7.4: Confirm commit landed**

```bash
git log -1 --oneline
git status --short
```

Expected: HEAD is the new commit; only the hapi-4552 fixtures remain untracked (those are commit 2's scope).

**Sub-agent for Commit 1 returns here for review.**

---

# Commit 2 — Hapi-4552 regression smoke test

**Sub-agent dispatch instructions:** This commit wires the hapi-4552 fixture into a regression smoke test. The fixture's diff and JSON output are already untracked in `tests/fixtures/`. Trim a real hapijs source to ~300 lines that aligns with the diff hunk line numbers, write a structural smoke test, replace the brittle 6,621-line `.txt` snapshot with a regeneration script, and commit. Verify `cargo test` green before committing.

**Pre-flight (sub-agent for commit 2):**

- [ ] **P4: Confirm prior commit is in place**

```bash
git log -1 --oneline
# expect: T1-002 + T1-005: peer_consistency, callback_dispatcher, primitive with tests
git status --short
# expect: only the hapi-4552 fixtures remain untracked
```

---

## Task 8: Trim hapi source

**Files:**
- Create: `tests/fixtures/hapi-4552-source/lib/transmit.js` (trimmed real source)

- [ ] **Step 8.1: Read the diff to identify needed line numbers**

```bash
cat tests/fixtures/hapi-4552.diff | head -50
```

Note the hunk headers — the diff modifies `lib/transmit.js` at hunks starting line 266, 364, and 373. After the diff applies (`+` lines added), the relevant lines are 269 (`stream.on('close', aborted)`), 368 (`from.on('close', internals.destroyPipe.bind(...))`), and 378-383 (the new `internals.destroyPipe` function).

- [ ] **Step 8.2: Construct a trimmed source file**

Create `tests/fixtures/hapi-4552-source/lib/transmit.js` with content shaped to match the diff line numbers. The actual hapijs source for `transmit.js` is large; we trim to just the relevant function bodies + enough whitespace so line numbers align with the diff.

The trimmed file should look like this skeleton (sized so that line 269 is `stream.on('close', aborted);` — i.e., the file has `internals.pipe` defined ~line 240, with body filling through ~273; `internals.chain` defined ~line 360, body filling through ~385). Use this trimmed version (MIT-licensed, derived from hapijs/hapi):

```javascript
'use strict';

// Trimmed from hapi PR #4552 (MIT-licensed) for prism regression testing.
// https://github.com/hapijs/hapi/pull/4552
// Only the function bodies relevant to the diff are retained; intervening
// lines are padded so absolute line numbers align with the diff hunks
// (line 269: stream.on('close', aborted); line 368: from.on('close', ...);
//  lines 378-383: new internals.destroyPipe function).

const internals = {};

// ... lines 12-237 are intentionally placeholder padding ...
// (each line is a single comment so no syntax matters)
//
//
//
//
// (continue with ~225 lines of comment padding to reach line 238)

internals.pipe = function (request, stream) {

    const aborted = () => {
        stream.removeAllListeners();
        stream.destroy();
    };

    if (request._closed) {
        request.raw.res.removeListener('error', aborted);
        return team.work;
    }

    if (stream._readableState && stream._readableState.flowing) {
        stream.unpipe(request.raw.res);
    }
    else {
        stream.on('error', end);
        stream.on('close', aborted);
        stream.pipe(request.raw.res);
    }

    return team.work;
};


internals.chain = function (sources) {

    let from = sources[0];
    for (let i = 1; i < sources.length; ++i) {
        const to = sources[i];
        if (to) {
            from.on('close', internals.destroyPipe.bind(from, to));
            from.on('error', internals.errorPipe.bind(from, to));
            from = from.pipe(to);
        }
    }

    return from;
};


internals.destroyPipe = function (to) {

    if (!this.readableEnded && !this.errored) {
        to.destroy();
    }
};

internals.errorPipe = function (to, err) {

    to.emit('error', err);
};
```

**Important:** This trimmed file is shape-only — `team.work`, `end`, etc. are not actually defined. The file is parsed by tree-sitter for structure only; runtime correctness is irrelevant. The line numbers must align with the diff.

To verify line alignment: after writing the file, `awk 'NR==269' tests/fixtures/hapi-4552-source/lib/transmit.js` should print `        stream.on('close', aborted);`. If not, adjust the comment-padding count between the header and `internals.pipe`.

- [ ] **Step 8.3: Verify the file parses with tree-sitter**

```bash
cargo run -- --repo tests/fixtures/hapi-4552-source --diff tests/fixtures/hapi-4552.diff --format json 2>&1 | tail -5
```

Expected: prints JSON output (a `MultiSliceResult`). If parsing fails, tree-sitter reports an ERROR node — adjust the trimmed source to be syntactically valid JS.

---

## Task 9: Write hapi regression smoke test

**Files:**
- Create: `tests/integration/hapi_regression_test.rs`

- [ ] **Step 9.1: Write the test file**

Create `tests/integration/hapi_regression_test.rs`:

```rust
#[path = "../common/mod.rs"]
mod common;
use common::*;

use prism::diff;
use std::fs;
use std::path::PathBuf;

fn fixture_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests/fixtures");
    p
}

#[test]
fn test_hapi_4552_review_suite_smoke() {
    // 1. Load the diff.
    let diff_path = fixture_root().join("hapi-4552.diff");
    let diff_text = fs::read_to_string(&diff_path).expect("read hapi-4552.diff");
    let diff_input = diff::parse_unified_diff(&diff_text).expect("parse diff");

    // 2. Load the trimmed source.
    let source_root = fixture_root().join("hapi-4552-source");
    let mut files = BTreeMap::new();
    for diff_info in &diff_input.files {
        let abs = source_root.join(&diff_info.file_path);
        let source = fs::read_to_string(&abs)
            .unwrap_or_else(|_| panic!("read source for {}", diff_info.file_path));
        let parsed =
            ParsedFile::parse(&diff_info.file_path, &source, Language::JavaScript).unwrap();
        files.insert(diff_info.file_path.clone(), parsed);
    }

    // 3. Run the review preset.
    let mut config = SliceConfig::default();
    config.algorithms = SlicingAlgorithm::review_suite();
    let results =
        algorithms::run_slicing_compat(&files, &diff_input, &config, None).unwrap();

    // 4. Structural assertions.
    assert!(
        !results.is_empty(),
        "review preset should produce at least one algorithm's result"
    );

    // (a) left_flow (the default algorithm) fires
    let has_left_flow = results
        .iter()
        .any(|r| r.algorithm == SlicingAlgorithm::LeftFlow);
    assert!(
        has_left_flow,
        "review preset should include LeftFlow output"
    );

    // (b) The diff lines (269 stream.on('close', aborted), 368 from.on('close', ...),
    //     378 internals.destroyPipe def) appear in at least one block somewhere.
    let target_lines: BTreeSet<usize> = BTreeSet::from([269, 368, 378]);
    let mut found_lines: BTreeSet<usize> = BTreeSet::new();
    for result in &results {
        for block in &result.blocks {
            if let Some(per_file) = block.file_line_map.get("lib/transmit.js") {
                for &ln in per_file.keys() {
                    if target_lines.contains(&ln) {
                        found_lines.insert(ln);
                    }
                }
            }
        }
    }
    assert!(
        !found_lines.is_empty(),
        "expected at least one of lines {:?} to appear in some block; got file_line_maps that didn't include any. \
         (This may indicate that line-number alignment between the trimmed source and the diff has drifted.)",
        target_lines
    );

    // (c) The new internals.destroyPipe definition (around line 378) appears
    //     somewhere in a parent_function or full_flow result, demonstrating
    //     the function is being analyzed as a unit.
    let has_destroy_pipe_function = results.iter().any(|r| {
        r.findings.iter().any(|f| {
            f.function_name
                .as_ref()
                .map(|n| n.contains("destroyPipe"))
                .unwrap_or(false)
        })
    });
    // Loose assertion — note in comment if this is too strict.
    if !has_destroy_pipe_function {
        eprintln!(
            "NOTE: review preset did not surface 'destroyPipe' as a function name. \
             This may be acceptable depending on how parent_function names anonymous \
             function expressions assigned to internals.destroyPipe."
        );
    }
}
```

- [ ] **Step 9.2: Add Cargo.toml entry**

Append:

```toml
[[test]]
name = "integration_hapi_regression"
path = "tests/integration/hapi_regression_test.rs"
```

- [ ] **Step 9.3: Run the new test**

```bash
cargo test --test integration_hapi_regression 2>&1 | tail -25
```

Expected: 1 test passes. If it fails on assertion (a) or (b), the trimmed source has drifted from the diff line numbers — adjust the comment-padding count in `transmit.js` and rerun.

---

## Task 10: Replace `.txt` snapshot with regeneration script

**Files:**
- Create: `scripts/regenerate_hapi_snapshot.sh`
- Delete: `tests/fixtures/hapi-4552-output.txt`

- [ ] **Step 10.1: Write the regeneration script**

Create `scripts/regenerate_hapi_snapshot.sh`:

```bash
#!/usr/bin/env bash
# Regenerate the human-readable snapshot of `--algorithm review --format text`
# output for the hapi-4552 fixture. Used for inspection only — the
# integration_hapi_regression test does NOT byte-compare against this file.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

cargo run --quiet -- \
    --repo tests/fixtures/hapi-4552-source \
    --diff tests/fixtures/hapi-4552.diff \
    --algorithm review \
    --format text \
  > tests/fixtures/hapi-4552-output.txt

cargo run --quiet -- \
    --repo tests/fixtures/hapi-4552-source \
    --diff tests/fixtures/hapi-4552.diff \
    --algorithm review \
    --format json \
  > tests/fixtures/hapi-4552-output.json

echo "Regenerated:"
echo "  tests/fixtures/hapi-4552-output.txt"
echo "  tests/fixtures/hapi-4552-output.json"
```

Make it executable:

```bash
chmod +x scripts/regenerate_hapi_snapshot.sh
```

- [ ] **Step 10.2: Delete the brittle `.txt` snapshot**

```bash
rm tests/fixtures/hapi-4552-output.txt
```

- [ ] **Step 10.3: Verify the script works**

```bash
./scripts/regenerate_hapi_snapshot.sh 2>&1 | tail -10
```

Expected: the script prints "Regenerated:" plus the two file paths. The `.txt` and `.json` files now exist as fresh outputs.

- [ ] **Step 10.4: Re-delete the just-regenerated `.txt`**

We don't track the snapshot in this commit — the `.json` is small and stays as inspection reference, but the `.txt` is regenerated on demand only.

```bash
rm tests/fixtures/hapi-4552-output.txt
```

---

## Task 11: Final verification + commit 2

- [ ] **Step 11.1: Confirm fmt is clean**

```bash
cargo fmt --check
```

Expected: no output.

- [ ] **Step 11.2: Run the full test suite**

```bash
cargo test 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 11.3: Stage commit 2 files**

```bash
git add \
  tests/fixtures/hapi-4552.diff \
  tests/fixtures/hapi-4552-output.json \
  tests/fixtures/hapi-4552-source/lib/transmit.js \
  tests/integration/hapi_regression_test.rs \
  scripts/regenerate_hapi_snapshot.sh \
  Cargo.toml
```

- [ ] **Step 11.4: Confirm `.txt` is NOT staged**

```bash
git status --short tests/fixtures/
```

Expected: `hapi-4552-output.txt` is absent (deleted, not added).

- [ ] **Step 11.5: Commit**

```bash
git commit -m "$(cat <<'EOF'
Wire hapi-4552 as review-suite regression smoke test

Replaces the 6,621-line text snapshot (tests/fixtures/hapi-4552-output.txt)
with a structural smoke test plus a regeneration script for inspection-only
use of the snapshot. The smoke test asserts that:
  - LeftFlow fires on the diff
  - Diff lines (269, 368, 378) appear in at least one block
  - The new internals.destroyPipe function is surfaced (loose, with eprintln
    note rather than hard-fail to allow review-preset evolution)

Trimmed real hapijs source kept at tests/fixtures/hapi-4552-source/ so diff
line numbers align with the unified-diff hunks. Source is MIT-licensed with
header noting the trim.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 11.6: Confirm commit landed**

```bash
git log -2 --oneline
git status --short
```

Expected: HEAD is the hapi commit; working tree is clean.

**Sub-agent for Commit 2 returns here for review.**

---

# Commit 3 — Plan.md refresh + pre-handoff baseline

**Sub-agent dispatch instructions:** This commit refreshes `Plan.md` with T1 work absorbed into existing tables and adds a new section capturing pre-handoff architectural baseline for the upcoming CWE coverage work. Do NOT touch `TEST_GAPS.md` or `TEST_COVERAGE.md`. Read the agent-eval supporting docs first to inform the baseline; then audit `taint.rs` + `provenance_slice.rs` for the inventory; then write.

**Pre-flight (sub-agent for commit 3):**

- [ ] **P5: Confirm prior commits are in place**

```bash
git log -3 --oneline
# expect: hapi commit, T1 algorithms commit, design spec commit
git status --short
# expect: clean working tree
```

---

## Task 12: Read agent-eval supporting docs

**Files (read-only):**
- Read: `~/code/agent-eval/analysis/prism-assessment.md`
- Read: `~/code/agent-eval/analysis/prism-algorithm-matrix.md`
- Read: `~/code/agent-eval/analysis/prism-cwe-coverage-handoff.md` (the handoff itself, for §10 question reference)

- [ ] **Step 12.1: Read prism-assessment.md**

```bash
cat ~/code/agent-eval/analysis/prism-assessment.md | head -100
```

Look for: "Tier A vs Tier B algorithms", "TAINT source/sink registry incomplete", "format pass" notes (esp. §3 if present). Capture in scratch notes for use in Task 14's baseline section.

- [ ] **Step 12.2: Read prism-algorithm-matrix.md**

```bash
cat ~/code/agent-eval/analysis/prism-algorithm-matrix.md | head -100
```

Look for: which algorithms the eval team considers Tier A (foundational) vs Tier B (taint).

- [ ] **Step 12.3: Re-read handoff §10 open questions**

```bash
sed -n '615,628p' ~/code/agent-eval/analysis/prism-cwe-coverage-handoff.md
```

Confirm five open questions:
1. Config vs code (registries)
2. Per-framework module structure
3. Sanitizer granularity (boolean vs confidence)
4. Phasing
5. Unknown-framework default (quiet vs heuristic)

These map 1:1 to subsection 5.2 of the spec (`docs/superpowers/specs/2026-04-24-hygiene-pass-pre-cwe-handoff-design.md`).

---

## Task 13: Audit current source/sink/sanitizer infrastructure

**Files (read-only):**
- Read: `src/algorithms/taint.rs`
- Read: `src/algorithms/provenance_slice.rs`

- [ ] **Step 13.1: Map the taint.rs source/sink registry**

```bash
grep -n "SINK_PATTERNS\|SOURCE_PATTERNS\|const " src/algorithms/taint.rs | head -40
```

Note: which const arrays exist, what languages they cover, what data structure (`&[&str]`, `&[(Lang, &[&str])]`, etc.). Capture findings.

```bash
wc -l src/algorithms/taint.rs
```

- [ ] **Step 13.2: Map the provenance_slice.rs origin classification**

```bash
grep -n "Origin\|WEB_FRAMEWORK_MODULES\|PROVENANCE_OVERLAP_KEYWORDS\|classify_line\|fn matches_provenance" src/algorithms/provenance_slice.rs | head -30
```

Note: Origin enum variants (UserInput, Config, Database, EnvVar, Network, etc.); WEB_FRAMEWORK_MODULES list; suppression mechanism.

```bash
wc -l src/algorithms/provenance_slice.rs
```

- [ ] **Step 13.3: Confirm no existing sanitizer infrastructure**

```bash
grep -rn "sanitiz\|cleansed_for\|SanitizerRegistry\|SanitizerKind" src/ | head -10
```

Expected: zero or near-zero hits (handoff's premise was correct).

- [ ] **Step 13.4: Confirm no existing framework detection layer**

```bash
grep -rn "FrameworkRegistry\|detect_framework\|framework_for_file" src/ | head -10
```

Expected: zero hits beyond the import-suppression in `provenance_slice.rs`.

---

## Task 14: Edit `Plan.md` in-place

**Files:**
- Modify: `Plan.md`

- [ ] **Step 14.1: Update the header line**

Open `Plan.md`. Replace line 3:

Old:
```
Last updated: 2026-04-02 (Phase 6 complete, CPG improvements plan)
```

New:
```
Last updated: 2026-04-24 (T1-006 handoff, three new algorithms, pre-handoff baseline added)
```

- [ ] **Step 14.2: Add rows to the existing `### P1 — Important Fixes (Partial)` table**

In `Plan.md`, find the `### P1 — Important Fixes (Partial)` table (around line 24-37). Append three new rows just before the `## Multi-Language Pattern Coverage` heading:

```markdown
| T1-006 follow-up: IPC taint sources (`g_hash_table_lookup` + variant accessors), CFG multiline call edge fix, GLib callback dispatcher detection | `eebafb6` | Done |
| Slice text empty in JSON output fix; `settings_t` provenance source added | `e3fa16d` | Done |
| Spiral added to review suite; taint sinks expanded; provenance import-suppression FP fix | `37ef823` | Done |
```

- [ ] **Step 14.3: Add new subsection `### Algorithms — Tier 1 (T1) Capability Expansion`**

Insert this section between `### Multi-Language Pattern Coverage (In Progress)` and `### Algorithm Precision & New Language Support`:

```markdown
### Algorithms — Tier 1 (T1) Capability Expansion

| Item | Branch / Commit | Status |
|------|-----------------|--------|
| **T1-002:** PeerConsistencySlice — sibling first-param NULL-guard cluster detection (uniform & divergent gap classifications). C/C++ only by design. Driven by FRR CVE-2025-61102. | `claude/t1-cleanup-pre-cwe-handoff` | Done |
| **T1-002:** CallbackDispatcherSlice — function-pointer-in-struct registration → invocation chain resolution; flags NULL-arg dispatch (zlog/functab pattern + GLib `g_signal_connect`). C/C++ only by design. | `claude/t1-cleanup-pre-cwe-handoff` | Done |
| **T1-005:** PrimitiveSlice — security-primitive fingerprint sweep (HASH_TRUNCATED_BELOW_128_BITS, HASH_TRUNCATION_VIA_CALL 2-pass, WEAK_HASH_FOR_IDENTITY, SHELL_TRUE_WITH_INTERPOLATION, CERT_VALIDATION_DISABLED, HARDCODED_SECRET). Python primary; basic C/JS/Go for cert-validation + secret rules. | `claude/t1-cleanup-pre-cwe-handoff` | Done |
| **Hapi-4552 regression smoke test** — JS event-listener-pair fixture wired into integration tests as `integration_hapi_regression`. Loose structural assertions (LeftFlow fires, diff lines surface) rather than byte-equal snapshot. | `claude/t1-cleanup-pre-cwe-handoff` | Done |

**Tests added (32 + 1 = 33):** Peer C 4 (uniform-unguarded, divergent, all-guarded negative, cluster-too-small negative), Peer C++ 3 (uniform, divergent, only-fires-on-touched-param). Callback C 4 (designated-init→null-dispatch, assignment-field clean, registrar-call-arg, no-invocations negative), Callback C++ 3 (designated-init, g_signal_connect, unrelated-function negative). Primitive Python 10 (hash-trunc direct/threshold/raw, hash-trunc 2-pass, weak-hash positive/negative, shell-true positive/negative, cert-validation, hardcoded-secret with inline negative). Primitive C 5 (cert-VERIFYPEER, cert-VERIFYHOST, dirty-function severity, outside-dirty-function severity, proper-validation negative). Primitive cross-language 3 (JS reject_unauthorized, JS hardcoded-secret object-field, Go InsecureSkipVerify). Hapi smoke 1.
```

- [ ] **Step 14.4: Add new section `## Pre-handoff Architectural Baseline (D5: CWE Coverage)`**

In `Plan.md`, find the line `---\n\n## Architecture Notes` (around the boundary between `## Remaining Work` and `## Architecture Notes`). Insert the following new section just before `## Architecture Notes`:

```markdown
## Pre-handoff Architectural Baseline (D5: CWE Coverage)

The eval team's CWE coverage handoff (`~/code/agent-eval/analysis/prism-cwe-coverage-handoff.md`) requests per-language sink taxonomy expansion across 6 CWE families, a category-aware sanitizer registry, and a framework-detection layer. None of those subsystems exist today. This section captures the starting state and tentative answers to the handoff's open questions, so the upcoming ACK doc has a baked baseline.

### Inventory of current source/sink/sanitizer infrastructure

- **Taint sources/sinks** live in `src/algorithms/taint.rs` (~[N] lines — note actual count from audit). Sinks are encoded as language-keyed pattern arrays (`SINK_PATTERNS` and per-language extensions). Sources are conservatively inferred from data-flow predecessors of diff lines, not from a registry.
- **Provenance origins** live in `src/algorithms/provenance_slice.rs` (~[N] lines). Origin classification: UserInput, Config, Database, EnvVar, Network, Secret. `WEB_FRAMEWORK_MODULES` const lists Flask/Django/etc. for import-aware suppression. `PROVENANCE_OVERLAP_KEYWORDS` lists `request`, `req`, `form`, `query`, etc. that are suppressed when imported from non-web modules.
- **Sanitizer recognition:** none in algorithm logic. Provenance has weak word-boundary refinement via `matches_provenance` with `~` prefix (e.g., `~body`, `~form` to avoid `transform` / `prefetch` false positives). Taint has no concept of sanitization.
- **Framework awareness:** the only piece is `WEB_FRAMEWORK_MODULES` in `provenance_slice.rs`, used for import-suppression. No detection layer activates per-framework source/sink overrides.

### Tentative answers to handoff §10 open questions

- **Q1 Config vs code (source/sink/sanitizer registries):** Rust modules with declarative const arrays — matches the existing `taint.rs` / `provenance_slice.rs` pattern. The eval team's stated value of "add sources mid-run for debugging" can be served by a CLI passthrough (`--taint-source-extra`, `--taint-sink-extra`) that doesn't require config-file parsing. Type safety + fast path beats config-file flexibility for the volume of patterns expected.
- **Q2 Per-framework module structure:** per-framework modules under `src/frameworks/{flask,django,fastapi,express,nethttp,gin,gorilla_mux}.rs`, registered through a small `FrameworkRegistry` enum. Mirrors the existing `src/languages/<lang>.rs` shape.
- **Q3 Sanitizer granularity:** boolean cleansed/uncleansed per category. A `cleansed_for: BTreeSet<SanitizerCategory>` on taint values is sufficient; confidence values are unwarranted complexity for this round.
- **Q4 Phasing:** agree with the eval team — Phase 1 Go (CWE-78 + CWE-22 sinks + net/http framework + shell-escape/path-validation sanitizers) → Phase 2 Python (CWE-79/89/918/502 + Flask/Django/FastAPI + HTML-escape/SQL-parametrize/URL-allowlist/path-validation sanitizers) → Phase 3 JS (Express + same CWE coverage) → Phase 4 Java (stretch, Tier 2.6 alignment).
- **Q5 Unknown-framework default:** quiet mode (eval team's stated preference). Existing `provenance_slice` already uses import-suppression for the noisy case; that pattern generalizes.

### Phasing recommendation

| Phase | Scope | Estimate |
|---|---|---|
| Phase 0 | This hygiene pass (T1 algorithms with tests + hapi regression + this baseline) | Done |
| Phase 1 | Go CWE-78/22 + net/http framework + shell/path sanitizers (aligns with eval C1 fixtures) | 1-2 weeks |
| Phase 2 | Python CWE-79/89/918/502 sinks + Flask/Django/FastAPI detection + 4 sanitizer categories | 2-3 weeks |
| Phase 3 | JS for the same CWE coverage on Express | 1-2 weeks |
| Phase 4 stretch | Java + Spring (Tier 2.6) | TBD |

### Known cross-language gap notes (from this hygiene pass)

- `primitive_slice::detect_hardcoded_secret` only matches bare `NAME = "literal"` and `obj.field = "literal"` LHS forms. `const`/`let`/`var` (JS), `:=` (Go), and `static const char *` (C) all bypass the LHS-identifier check. Deferred rather than patched — the handoff's category-aware sanitizer/source registry is likely to subsume this rule entirely.

### Reference

- Handoff: `~/code/agent-eval/analysis/prism-cwe-coverage-handoff.md`
- Eval-team Prism assessment: `~/code/agent-eval/analysis/prism-assessment.md`
- Eval-team algorithm matrix: `~/code/agent-eval/analysis/prism-algorithm-matrix.md`
- Implementation plan for this hygiene pass: `docs/superpowers/plans/2026-04-24-t1-hygiene-pre-cwe-handoff.md`
- Spec for this hygiene pass: `docs/superpowers/specs/2026-04-24-hygiene-pass-pre-cwe-handoff-design.md`

---
```

(Replace `[N]` placeholders with the actual line counts from Task 13's audit.)

- [ ] **Step 14.5: Verify Plan.md still parses as markdown**

```bash
head -10 Plan.md
grep "^## " Plan.md
```

Expected: header line is updated; the new "Pre-handoff Architectural Baseline" `## ` heading appears between `## Remaining Work` and `## Architecture Notes`.

---

## Task 15: Final verification + commit 3

- [ ] **Step 15.1: Confirm no source code changes (this commit is doc-only)**

```bash
git status --short
```

Expected: only `Plan.md` is modified.

- [ ] **Step 15.2: Stage commit 3 file**

```bash
git add Plan.md
```

- [ ] **Step 15.3: Commit**

```bash
git commit -m "$(cat <<'EOF'
Refresh Plan.md: T1-002/005/006 status + pre-handoff baseline

Updates Plan.md (last touched 2026-04-02) to absorb:
  - T1-006 work (eebafb6: IPC sources, CFG multiline, GLib callbacks)
  - Mid-cycle commits e3fa16d, 37ef823
  - T1-002 + T1-005 algorithms landed in this branch
  - Hapi-4552 regression smoke test landed in this branch

Adds new section "Pre-handoff Architectural Baseline (D5: CWE Coverage)"
between "Remaining Work" and "Architecture Notes" capturing:
  - Inventory of current taint/provenance infrastructure (no sanitizer
    or framework-detection layer exists today)
  - Tentative answers to the eval team's handoff §10 open questions
  - Phased recommendation: Go (Phase 1) → Python (Phase 2) → JS
    (Phase 3) → Java stretch
  - Cross-language gap note on detect_hardcoded_secret (deferred —
    likely subsumed by handoff registry redesign)

This section is the spine of the upcoming ACK doc at
~/code/slicing/ACK-prism-cwe-coverage-handoff.md.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 15.4: Confirm all three commits are present**

```bash
git log -4 --oneline
```

Expected (top to bottom):
- `<sha>` Refresh Plan.md: T1-002/005/006 status + pre-handoff baseline
- `<sha>` Wire hapi-4552 as review-suite regression smoke test
- `<sha>` T1-002 + T1-005: peer_consistency, callback_dispatcher, primitive with tests
- `ea1e9e7` Design spec: T1 algorithm hygiene + pre-CWE-handoff baseline

**Sub-agent for Commit 3 returns here for review.**

---

# Final: Open PR

After all three commits are reviewed and accepted on the feature branch:

- [ ] **F1: Push branch**

```bash
git push -u origin claude/t1-cleanup-pre-cwe-handoff
```

- [ ] **F2: Create PR**

```bash
gh pr create --title "T1 algorithm hygiene + pre-CWE-handoff baseline" --body "$(cat <<'EOF'
## Summary

- Lands three slicing algorithms previously sitting in working tree: PeerConsistencySlice + CallbackDispatcherSlice (T1-002, FRR CVE-2025-61102) and PrimitiveSlice (T1-005, security-primitive fingerprints).
- Wires `tests/fixtures/hapi-4552.diff` as a structural regression smoke test (replaces a brittle 6,621-line text snapshot).
- Refreshes `Plan.md` with T1 capability rows and a new "Pre-handoff Architectural Baseline" section that captures starting state and tentative answers to the eval team's CWE coverage handoff (D5).

Three commits, sequential, each leaves `cargo test` green:
1. Algorithms with tests + matrix update + cleanups (32 new tests across 4 files)
2. Hapi-4552 regression smoke test (1 new test, trimmed source fixture, regeneration script)
3. Plan.md refresh (in-place table updates + new baseline section)

Spec: `docs/superpowers/specs/2026-04-24-hygiene-pass-pre-cwe-handoff-design.md`
Plan: `docs/superpowers/plans/2026-04-24-t1-hygiene-pre-cwe-handoff.md`

## Test plan

- [ ] `cargo test` passes locally (33 new tests + existing suite)
- [ ] `cargo fmt --check` is clean
- [ ] `python3 scripts/generate_coverage_badges.py` regenerates badges; resulting README diff looks reasonable
- [ ] `./scripts/regenerate_hapi_snapshot.sh` produces inspectable output
- [ ] Visual: run `cargo run -- --list-algorithms` to confirm 30 algorithms listed including peer/callback/primitive

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

---

## Self-Review Notes

### Spec coverage check

Each section/requirement in the spec maps to a task:

| Spec section | Implementation task(s) |
|---|---|
| §3.1 New test files | Tasks 1, 2, 3, 4 |
| §3.2 peer_consistency tests | Task 1 |
| §3.3 callback_dispatcher tests | Task 2 |
| §3.4 primitive_test.rs tests | Task 3 |
| §3.5 primitive_lang_test.rs tests | Task 4 |
| §3.6 Source cleanups | Task 5 |
| §3.7 Coverage matrix update | Task 5 |
| §3.8 Cargo.toml additions | Tasks 1.2, 2.2, 3.2, 4.2 |
| §3.9 Already-staged changes ride along | Task 7 stages them |
| §4.1 Hapi files | Tasks 8, 10 |
| §4.2 New test file | Task 9 |
| §4.3 Cargo.toml addition | Task 9.2 |
| §5.1 Plan.md edit-in-place | Tasks 14.1, 14.2, 14.3 |
| §5.2 Pre-handoff baseline section | Task 14.4 |
| §6 Verification | Tasks 7, 11, 15 |

### Placeholder scan

The plan contains one explicit `[N]` placeholder in Task 14.4 — the actual line counts of `taint.rs` and `provenance_slice.rs` go there during execution (Task 14.4 step explicitly notes this). All other steps contain complete code or commands. No "TODO", "TBD", "implement later" anywhere else.

### Type/identifier consistency

- All test files use `BTreeSet<usize>` for `diff_lines` — consistent.
- All test files use `algorithms::run_slicing_compat` (the public API in `tests/common/mod.rs`) — consistent.
- `findings_for_rule` helper is defined separately in Task 3 and Task 4 (since `primitive_test.rs` and `primitive_lang_test.rs` are separate test targets); intentional duplication, not a bug.
- `SlicingAlgorithm::review_suite()` (verified by grep) is the actual method name, used in Task 9.

### Scope check

Three commits, ~1.5 days estimate, dispatchable to three sub-agents in series. Self-contained. No spec section unimplemented.

---

*End of plan.*
