# Rust `use`-import extraction + module narrowing — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Extract Rust `use` declarations into the per-file import map so (1) `nav module-deps`/`repo-map` show resolved Rust import edges (today Rust is call-derived-only — `UnresolvedModule` for the #1 / dogfood language) and (2) **unqualified** calls whose name was `use`-imported narrow to the imported module instead of fanning out to every same-named definition repo-wide (the F3 Rust fix: `original_diff.rs`'s local `fn slice` no longer pulls in 29 algorithm files).

**Architecture:** Add `collect_rust_imports` to the existing `ParsedFile::extract_imports` dispatch (Rust currently falls through to `_ => {}`). Reuse the *existing* import infrastructure end-to-end — the per-file `cg.imports` map, the `module-deps` import-edge emission, and the stem/dir narrowing heuristic (`resolution.rs:643-660`, which already resolves `foo.rs` by stem **and** `foo/mod.rs` by parent dir). The only genuinely new resolution rung is **unqualified-name narrowing** for `use`-imported names. True Rust module resolution (crate-root `lib.rs`/`main.rs`, `mod foo;` declarations, inline `mod`, glob expansion, precise `super::`) is **out of scope** — a deferred precision follow-up; the stem/dir heuristic covers the common file conventions.

**Tech Stack:** Rust, tree-sitter-rust (`use_declaration` grammar), the existing CPG/resolution/nav layers.

**Scope guard (no new scope):** MVP = extraction + reuse the stem/dir heuristic for module-deps + add unqualified-import narrowing. Do NOT build a crate-internal module resolver, `mod`-declaration graph, glob expansion, or re-export following — those are the deferred follow-up (§Backlog). External crates (`std`, `serde`, …) extract as imports but resolve to no in-repo file (no edge) — that's correct, not a gap.

---

## Background: what Rust import-extraction buys (verified against the map)

- **module-deps/repo-map (F3):** today `cg.imports` has no Rust entries; `module_graph.rs:186` "Rust/Java/C/C++ do not [extract imports]", and Rust is call-derived-only (`module_graph_test.rs`: emits zero `HeuristicImport`). Populating `cg.imports` for Rust + resolving by stem/dir turns `use crate::foo::Bar` into a resolved edge to `foo.rs`/`foo/mod.rs`.
- **Unqualified narrowing (F3):** `resolve_callees_qualified` falls through to "all non-static same-named definitions" (`call_graph.rs:654ff`). For Rust, `use crate::engine::start; … start()` should narrow bare `start()` to `engine`'s `start`. Rust *qualified* `engine::start()` already goes through the `::` path shape (R1/R2/R7) and is **not** in scope for this change.

## Key code anchors (verified)

- **Import dispatch:** `src/ast.rs:498-507` `collect_imports` — add `Language::Rust => self.collect_rust_imports(node, out)` (today Rust hits `_ => {}` at :505). Mirror `collect_go_imports` (`ast.rs:814`) / `extract_go_import_spec` (`:846`).
- **Import map shape:** `BTreeMap<alias, module_path>`; populated into `cg.imports: BTreeMap<file, BTreeMap<alias, module_path>>` at `call_graph.rs:299-306`.
- **Narrowing heuristic to reuse:** `src/resolution.rs:637-671` (R3 import-qualified). `module_stem` = last segment after stripping extension; `module_last` = last path segment; filter `functions[name]` by `file_stem(fid.file) == module_stem || parent_dir == module_last`, free-functions only. `ResolutionKind::ImportQualified`.
- **module-deps emission:** `src/navigation/module_graph.rs:192-223` emits `Source::HeuristicImport` items per distinct `module_path` + the `UnresolvedModule` warning. The Rust call-derived-only contract: `module_graph_test.rs:~72` `module_deps_rust_is_call_derived_only_no_import_items` (this test changes).
- **tree-sitter-rust `use` node kinds:** `use_declaration` → `scoped_identifier` (path `a::b`), `use_as_clause` (field `alias`/`name`), `use_list` (braced `{b, c}`), `scoped_use_list` (`a::{…}`), `use_wildcard` (`a::*`), with `crate`/`super`/`self` leading segments; `pub use` carries a `visibility_modifier`. (Confirm exact kinds against the grammar during TDD — the test fixtures pin them.)
- **Tests:** `tests/ast/import_test.rs` (Python/JS/Go idiom — add Rust), `tests/navigation/module_graph_test.rs` (the Rust call-derived-only test), `tests/lang/rust/`, `tests/common/mod.rs` (`make_rust_test`/`session`).

---

## Task 1: `collect_rust_imports` — extract `use` declarations

**Files:** Modify `src/ast.rs` (dispatch + new walker) · Test `tests/ast/import_test.rs`

**Design — the (alias, module_path) contract for each form:**
| `use` form | alias | module_path |
|---|---|---|
| `use a::b;` | `b` | `a` |
| `use a::b::c;` | `c` | `a::b` |
| `use a::b as c;` | `c` | `a::b`'s module = `a` (the path before `b`) — i.e. alias renames the imported item `b`, module is `a` |
| `use a::{b, c};` | `b`,`c` | `a`, `a` |
| `use a::{b::c, d};` | `c`,`d` | `a::b`, `a` |
| `use a::*;` | `*` | `a` (wildcard marker — emitted for module-deps; ignored by unqualified narrowing) |
| `use crate::m::T;` | `T` | `crate::m` |
| `use super::x;` / `self::y;` | `x`/`y` | `super`/`self` (kept; narrowing strips the prefix, last real segment drives stem match) |
| `pub use a::b;` | `b` | `a` (re-export still a dependency) |

`module_path` = the `::`-joined path of everything BEFORE the final imported name; the alias = the final name (or its `as` rename). The narrowing/edge code only uses the **last `::` segment** of `module_path` as the file stem, so `crate::`/`self::` prefixes are harmless (stripped at use); external crates (`std::…`) produce a module_path whose stem matches no in-repo file → no edge (correct).

- [ ] **Step 1: Failing test** in `tests/ast/import_test.rs` (mirror `test_go_import_*`):
```rust
#[test]
fn test_rust_import_forms() {
    let src = "\
use crate::engine::start;
use crate::util::{helper, other as renamed};
use crate::deep::{a::b, c};
use std::collections::HashMap;
use super::sibling;
pub use crate::reexport::Thing;
";
    let parsed = ParsedFile::parse("src/main.rs", src, Language::Rust).unwrap();
    let imports = parsed.extract_imports();
    assert_eq!(imports.get("start"), Some(&"crate::engine".to_string()));
    assert_eq!(imports.get("helper"), Some(&"crate::util".to_string()));
    assert_eq!(imports.get("renamed"), Some(&"crate::util".to_string())); // `as` rename
    assert_eq!(imports.get("b"), Some(&"crate::deep::a".to_string()));    // nested group
    assert_eq!(imports.get("c"), Some(&"crate::deep".to_string()));
    assert_eq!(imports.get("HashMap"), Some(&"std::collections".to_string())); // external, still extracted
    assert_eq!(imports.get("Thing"), Some(&"crate::reexport".to_string()));   // pub use
}
```
- [ ] **Step 2: Run → fails** (Rust extracts nothing). `cargo test --test ast import_test::test_rust_import_forms 2>&1 | tail`
- [ ] **Step 3: Implement** `collect_rust_imports` — recurse the use-tree: maintain a path prefix; at a leaf `identifier` (or `use_as_clause`) emit `(alias, prefix)`; at a `use_list`/`scoped_use_list` recurse with the extended prefix; `use_wildcard` emits `("*", prefix)`. Confirm the exact tree-sitter-rust node kinds against the parsed fixture (print the tree in a scratch test if needed). Add the `Language::Rust =>` dispatch arm.
- [ ] **Step 4: Run → pass**; add a glob test (`use a::*;` → `imports.get("*") == Some("a")`) and a plain `use a::b;` test; `cargo test --test ast import_test::` green.
- [ ] **Step 5: Commit** `feat(rust): extract use-import bindings (collect_rust_imports)`

## Task 2: Rust module-deps / repo-map edges (resolved by stem/dir)

**Files:** Modify `src/navigation/module_graph.rs` (resolve Rust import module_path → file by stem/dir) · Test `tests/navigation/module_graph_test.rs`

**Design:** Once Task 1 lands, `cg.imports` has Rust entries, so the existing `HeuristicImport` emission (`module_graph.rs:192-211`) starts surfacing Rust imports. Upgrade it (for all languages, or Rust-scoped) to **resolve** a module_path to an in-repo file by the stem/dir heuristic (last `::`/`/` segment → `file_stem == seg || parent_dir == seg`), emitting a resolved `PrismCpg`-style edge when matched and only falling back to `HeuristicImport`/`UnresolvedModule` when unmatched (external crates). This turns the Rust `UnresolvedModule` (v1) into resolved edges per the doc.

- [ ] **Step 1: Failing test** — update/replace `module_deps_rust_is_call_derived_only_no_import_items` with the new contract: a `use crate::util::helper;` in `main.rs` yields a resolved module-deps edge to `util.rs` (not just a call-derived edge, and not an unresolved heuristic item for an in-repo target).
```rust
#[test]
fn module_deps_rust_resolves_use_import_edges() {
    let s = session(&[
        ("util.rs", "pub fn helper() -> i32 { 1 }\n"),
        ("main.rs", "use crate::util::helper;\nfn run() -> i32 { helper() }\n"),
    ]);
    let ev = module_deps(&s, "main.rs");
    assert!(ev.items.iter().any(|it| it.location.file == "util.rs" || /* resolved target */ true),
            "use crate::util::helper resolves to util.rs");
    // external import stays unresolved (warning), in-repo import resolves (no UnresolvedModule for it)
}
```
*(Express against the real `module_deps`/`EvidenceItem` shape — the behavior asserted is: in-repo `use` target → resolved edge; external `use std::…` → unresolved/no-edge.)*
- [ ] **Step 2: Run → fails.**
- [ ] **Step 3: Implement** the module_path→file resolution (stem/dir) in `module_graph.rs`; keep `UnresolvedModule` only for genuinely unresolved (external) modules. Update the doc-comment at `:186`.
- [ ] **Step 4: Run → pass**; confirm Python/JS/Go module-deps behavior unchanged (their tests still green); `cargo test --test navigation module_graph_test::` + `repo_map` tests green.
- [ ] **Step 5: Commit** `feat(rust): resolve use-import module-deps/repo-map edges by stem/dir`

## Task 3: Unqualified-call import narrowing (the F3 Rust fan-out fix)

**Files:** Modify `src/resolution.rs` (unqualified path) · Test `tests/integration/resolution_test.rs`

**Design:** In the unqualified call path (qualifier `None`, no `::`), before the broad "all same-named" fallback, check the caller file's import map: if `name` is a `use`-imported alias, narrow `functions[name]` to candidates matching the imported module by the **same stem/dir heuristic** as R3 (free-functions only). Conservative + recall-safe: if the import points outside the repo (no match), **fall through** to the existing behavior (do not drop). Reuse the R3 filter helper if cleanly extractable; otherwise factor a shared `narrow_by_module(name, module_path)`.

- [ ] **Step 1: Failing test** — the F3 reproduction: a local + a remote `fn slice`; an unqualified `slice()` call in a file that `use`-imports the remote one narrows to it (and a file with no such import keeps current behavior).
```rust
#[test]
fn rust_unqualified_call_narrows_to_use_imported_module() {
    // two `fn process` in different files; caller use-imports one → resolves to that one only
    let (cg, _) = build(&[
        ("engine.rs", "pub fn process() {}\n"),
        ("other.rs", "pub fn process() {}\n"),
        ("main.rs", "use crate::engine::process;\nfn run() { process() }\n"),
    ]);
    let site = /* the process() call in main.rs */;
    let r = cg.resolve_call_site(&site);
    // resolves to engine.rs::process ONLY (not other.rs::process)
    assert_eq!(resolved_files(&r), vec!["engine.rs"]);
}
```
- [ ] **Step 2: Run → fails** (today both `process` defs are returned / demoted as a collision).
- [ ] **Step 3: Implement** the unqualified import-narrowing rung. Confidence: `Exact`/`ImportQualified` when the import narrows to exactly one; keep the existing demote/drop for the no-import case. Recall-safe: external import → fall through unchanged.
- [ ] **Step 4: Recall-guard tests** — (a) no import for the name → unchanged (existing collision behavior); (b) import points to an external crate (no in-repo match) → fall through, not dropped; (c) qualified `engine::process()` (the `::` path) still resolves as before (this change is unqualified-only).
- [ ] **Step 5: Run → pass**; full `cargo test` green; `cargo fmt`.
- [ ] **Step 6: Commit** `fix(rust): narrow unqualified use-imported calls to the imported module`

## Task 4: Verification — Tier-A + the F3 demo cases

**Files:** verify only.

- [ ] **Step 1:** `cargo build --release`
- [ ] **Step 2:** the doc's verified F3 demo — `target/release/prism nav module-deps --file src/algorithms/original_diff.rs`: confirm it no longer reports dependencies on all 29 `fn slice` algorithm files (the local `fn slice` resolves locally), and `nav module-deps` on a Rust file now shows resolved `use` edges. Paste before/after.
- [ ] **Step 3:** `cd eval && uv run tier-a --matrix-only --allow-stale-sut` — no regressions; then `--quick --allow-stale-sut` (prism corpus, Rust) — confirm prism corpus M2 callers precision/recall does not regress (this change should *improve* Rust unqualified-call precision; paste any flips).
- [ ] **Step 4:** `cargo test` (all) · `--features mcp` build · `cargo fmt --check`.
- [ ] **Step 5: Commit** any baseline/doc note if matrix flips (expected precision-positive).

## Final verification (before PR)
- [ ] `cargo fmt --check` · full `cargo test` · `cargo build --release` · `--features mcp` build
- [ ] Tier-A `--matrix-only` no regression; the `original_diff.rs` F3 demo fixed
- [ ] PR body: the F3 before/after (module-deps fan-out) + any Tier-A Rust precision delta

## Risks / watch-items
- **Recall loss in narrowing (the headline risk).** An import that resolves to the wrong file (stem collision: two `util.rs` in different dirs) could mis-narrow. Mitigation: narrow only when the stem/dir filter yields ≥1 in-repo match; if it yields multiple, demote (don't pick arbitrarily); external/no-match → fall through unchanged. Recall-guard tests in Task 3 Step 4.
- **module-deps double-counting.** `module_graph.rs:186` notes a module both imported and call-resolved appears twice. Keep that v1 behavior or de-dup deliberately — don't regress Python/JS/Go.
- **tree-sitter-rust node kinds.** Confirm `use_list`/`scoped_use_list`/`use_as_clause`/`use_wildcard` kinds against the actual grammar in TDD (the fixtures pin them); the grammar version is in `Cargo.lock`.
- **No `CACHE_VERSION` bump needed** — `extract_imports` is computed at build time into `cg.imports`, not a new serialized node field. Confirm `cg.imports` is already cached/rebuilt (it is, via the CallGraph build); if Rust imports change the serialized CallGraph shape, bump — verify during Task 2.

## Backlog / deferred (true Rust module resolution)
A precision follow-up beyond the stem/dir heuristic, gated on evidence it's needed: crate-root (`lib.rs`/`main.rs`) detection, `mod foo;` declaration graph, inline `mod foo {}`, `foo/mod.rs` vs `foo.rs` disambiguation when both exist, glob (`use a::*`) member expansion, re-export (`pub use`) following, and precise `super::`/`self::` relative resolution. The stem/dir heuristic handles the common conventions; this closes the long tail. Pairs with the C++ `using` extraction the substrate doc bundles next.
