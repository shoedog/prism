# Rust `use`-import extraction + real module resolution — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

> **Rev 3 (2026-06-17) — ARCHITECTURE MOVED TO THE DESIGN SPEC.** The authoritative architecture is now
> `docs/superpowers/specs/2026-06-17-prism-rust-module-resolution-design.md`, which designs the **full**
> Rust module system up front (crate graph, module graph, anchors, re-exports, glob, `#[path]`, cfg,
> editions, workspaces, macros) behind a C++-reusable seam. This plan is being re-cast as **the spec's
> Phase 1** (the F3 win: crate graph + module graph + extraction + resolution + module-deps/narrowing
> consumers, recall-safe). The **Tasks below (rev-2) are SUPERSEDED** — they encode the naive
> `ModuleIndex` that the codex review found FLAWED (2 BLOCKER + 4 MAJOR: lexical-scope, directory base,
> crate-key collision, module-vs-item, incremental staleness, seam leakage). The Phase-1 tasks will be
> **re-derived from the spec** once the spec re-review (codex gpt-5.5 xhigh) is clean. Read the spec, not
> the rev-2 architecture/tasks below, for the design of record.

> **Rev 2 (2026-06-17, SUPERSEDED by Rev 3 / the spec):** Owner re-scoped away from the stem/dir *heuristic* to **real module resolution for the common conventions**. Rationale: heuristics in this project have looked promising, missed the precision/recall bar, then required rewrites (double work); proper resolution is also the foundation the bundled **C++ `using`** work reuses, so the upfront cost amortizes across two languages. This rev replaced the heuristic with a real `crate::path → file` index — but the codex review found that "common-conventions-only" map was itself a naive approximation with correctness BLOCKERs; the full architecture now lives in the spec.

**Goal:** Extract Rust `use` declarations AND resolve crate-internal module paths to files via a real module index (crate roots + `mod` declarations), so (1) `nav module-deps`/`repo-map` show **precise** Rust import edges (today Rust is call-derived-only — `UnresolvedModule` for the #1/dogfood language) and (2) **unqualified** calls whose name was `use`-imported resolve to the **exact** defining module, not every same-named definition repo-wide (the F3 fix: `original_diff.rs`'s local `fn slice` stops pulling in 29 algorithm files).

**Architecture:** A new **`ModuleIndex`** — a language-agnostic `module_path → defining file(s)` map (+ inverse `file → module_path`) — built once per repo at CallGraph build time. **Rust populates it precisely** from crate-root detection (`lib.rs`/`main.rs`/`bin/*.rs`) + a `mod`-declaration walk (`mod foo;` → `foo.rs`/`foo/mod.rs`; inline `mod foo {}`; `crate`/`super`/`self`). It is a **seam, not a Rust-specific structure**: the map value is `Vec<file>` (one module ≈ one file in Rust; a namespace spans many headers in C++), so the bundled C++ `using`/namespace work populates the *same* index from namespace declarations and the consumers are unchanged. Consumers: `module-deps`/`repo-map` (precise edges) and an unqualified-call narrowing rung in `resolution.rs` (resolve the import's module path → file → narrow candidates to that file). **No stem/dir heuristic** anywhere in the resolution path.

**Tech Stack:** Rust, tree-sitter-rust (`use_declaration` + `mod_item` grammar), the CPG/resolution/nav layers.

**Scope guard — "common conventions, done properly" (not heuristic, not the long tail):**
- **IN:** crate-root detection (`src/lib.rs`, `src/main.rs`, `src/bin/*.rs`; multiple roots OK); `mod foo;` → sibling `foo.rs` or `foo/mod.rs` (both editions); inline `mod foo {}`; nested modules; `crate::`/`super::`/`self::` relative resolution; the `ModuleIndex` seam.
- **DEFERRED (backlog):** glob (`use a::*`) member expansion; `pub use` re-export *chain* following (v1 resolves to the re-exporting module — a real edge, just not transitively chased); `#[path = "…"]` attribute mods; `cfg`-gated/feature-gated mods; Cargo.toml `[lib]`/`[[bin]]`/workspace-member crate-root discovery (v1 uses filename convention); external-crate (deps) resolution. Each is a precision *addition*, not a correctness hazard — an unresolved path falls through safely (see Risks).
- **NOT a goal:** changing qualified `::` call resolution (R1/R2/R7) — this PR is module-deps + unqualified narrowing only.

---

## Background (verified against the Explore map)

- **module-deps/repo-map (F3):** `cg.imports` has no Rust entries today; `module_graph.rs:186` "Rust/Java/C/C++ do not [extract imports]"; Rust is call-derived-only (`module_graph_test.rs` asserts zero `HeuristicImport`). The `ModuleIndex` turns `use crate::foo::Bar` into a *resolved* edge to `foo`'s actual file.
- **Unqualified narrowing (F3):** `resolve_callees_qualified` falls through to "all non-static same-named definitions" (`call_graph.rs:654ff`). With the index, `use crate::engine::start; … start()` resolves `crate::engine → engine.rs` and narrows bare `start()` to that file's `start`.
- **Rust module resolution today: essentially none** (Explore map §5) — no crate-root detection, no `mod`-decl graph, no `module_path → file` map. This PR builds it.

## Key code anchors (verified)

- **Import dispatch:** `src/ast.rs:498-507` `collect_imports` — Rust hits `_ => {}` at :505. Mirror `collect_go_imports` (`:814`).
- **Import map:** `BTreeMap<alias, module_path>` → `cg.imports` at `call_graph.rs:299-306`.
- **CallGraph build:** `call_graph.rs` build path (where `imports` is assembled) — add `module_index` assembly alongside.
- **Narrowing rung (model, not reused-as-heuristic):** `src/resolution.rs:637-671` (R3 import-qualified) — the *shape* of an import-driven narrowing rung + `ResolutionKind::ImportQualified`; the new rung resolves via `ModuleIndex` instead of stem/dir matching.
- **module-deps emission:** `src/navigation/module_graph.rs:186-223` (`Source::HeuristicImport` + `UnresolvedModule` warning); Rust call-derived-only test `module_graph_test.rs:~72`.
- **tree-sitter-rust nodes:** `use_declaration`, `scoped_identifier`, `use_list`, `scoped_use_list`, `use_as_clause`, `use_wildcard`, `crate`/`super`/`self`; **`mod_item`** (field `name`, optional `body` block = inline) for `mod` declarations; `visibility_modifier` for `pub`. Confirm exact kinds against the grammar during TDD (fixtures pin them).
- **File keying:** repo-relative paths (`src/foo.rs`, `src/foo/mod.rs`). The index keys/values use these.
- **Tests:** `tests/ast/import_test.rs`, `tests/navigation/module_graph_test.rs`, `tests/lang/rust/`, `tests/common/mod.rs` (`session`/`make_rust_test`), `tests/integration/resolution_test.rs`.

---

## Task 1: `collect_rust_imports` — extract `use` declarations → `(alias, full module path)`

**Files:** Modify `src/ast.rs` · Test `tests/ast/import_test.rs`

The extraction produces the **full `::` module path** (e.g. `crate::engine`) so the index can resolve it precisely (no stem shortcut). Forms + (alias, module_path): `use a::b` → (`b`,`a`); `use a::b::c` → (`c`,`a::b`); `use a::b as c` → (`c`,`a`); `use a::{b,c}` → (`b`,`a`),(`c`,`a`); `use a::{b::c,d}` → (`c`,`a::b`),(`d`,`a`); `use a::*` → (`*`,`a`) glob marker; `use crate::m::T` → (`T`,`crate::m`); `use super::x`/`self::y` → (`x`,`super`)/(`y`,`self`) (prefix kept; the index resolves it relative to the importing file's module); `pub use a::b` → (`b`,`a`).

- [ ] **Step 1: Failing test** in `import_test.rs` (mirror `test_go_import_*`):
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
    let p = ParsedFile::parse("src/main.rs", src, Language::Rust).unwrap();
    let im = p.extract_imports();
    assert_eq!(im.get("start"), Some(&"crate::engine".into()));
    assert_eq!(im.get("renamed"), Some(&"crate::util".into()));
    assert_eq!(im.get("b"), Some(&"crate::deep::a".into()));
    assert_eq!(im.get("c"), Some(&"crate::deep".into()));
    assert_eq!(im.get("HashMap"), Some(&"std::collections".into()));
    assert_eq!(im.get("Thing"), Some(&"crate::reexport".into()));
}
```
- [ ] **Step 2: Run → fails.** **Step 3:** implement `collect_rust_imports` (recurse the use-tree with a path prefix; confirm node kinds against the parsed fixture). **Step 4:** glob + plain-`use` tests; `cargo test --test ast import_test::` green. **Step 5: Commit** `feat(rust): extract use-import bindings (collect_rust_imports)`

## Task 2: The `ModuleIndex` — `mod`-declaration walk + crate-root detection (the resolver core)

**Files:** Create `src/module_index.rs` (or `src/navigation/module_index.rs`) · Modify `src/ast.rs` (extract `mod` decls) · Modify `src/call_graph.rs` (build + store the index) · Test new `tests/ast/module_index_test.rs` (or under `tests/integration/`)

**Design:**
```rust
/// Language-agnostic module/namespace path -> defining file(s). Rust: ~1 file per
/// module. C++ (bundled follow-up): a namespace spans many headers -> Vec. Consumers
/// resolve a use/using path to file(s); they do not care which language populated it.
#[derive(Default, Clone, Serialize, Deserialize)]
pub struct ModuleIndex {
    pub path_to_files: BTreeMap<String, Vec<String>>, // "crate::engine" -> ["src/engine.rs"]
    pub file_to_module: BTreeMap<String, String>,     // "src/engine.rs" -> "crate::engine"
}
impl ModuleIndex {
    /// Resolve a `use`/`using` module path (possibly `crate::`/`super::`/`self::`-relative
    /// to `from_file`'s module) to defining file(s). Empty = unresolved (external/unknown).
    pub fn resolve(&self, module_path: &str, from_file: &str) -> &[String] { /* … */ }
}
```
Rust population (`build_rust_module_index(files)`):
1. **Crate roots:** files matching `*/src/lib.rs`, `*/src/main.rs`, `*/src/bin/*.rs` (per crate dir); each root's module path = `crate`. (Multiple roots/workspaces OK; Cargo.toml-driven discovery deferred.)
2. **`mod` walk:** extract `mod foo;`/`mod foo {}` per file (new `extract_mod_decls` in `ast.rs` over `mod_item`). From a file at module `M` in directory `D`: `mod foo;` → module `M::foo` at `D/foo.rs` **or** `D/foo/mod.rs` (whichever exists in `files`); inline `mod foo {}` → module `M::foo` mapped to the **same file** (and recurse for nested decls). Seed BFS/DFS from each crate root.
3. **`resolve`:** normalize `crate::` → from the crate root; `self::` → `from_file`'s module; `super::` → parent of `from_file`'s module; then descend `path_to_files`. Unknown/external (`std::…`, a dep) → empty slice.

- [ ] **Step 1: Failing tests** — build a multi-file fixture and assert the index:
```rust
// files: src/lib.rs ("mod engine;\nmod util;\n"), src/engine.rs ("pub fn start(){}"),
//        src/util/mod.rs ("mod helper;"), src/util/helper.rs ("pub fn h(){}")
// assert: path_to_files["crate::engine"] == ["src/engine.rs"]
//         path_to_files["crate::util"]   == ["src/util/mod.rs"]
//         path_to_files["crate::util::helper"] == ["src/util/helper.rs"]
//         file_to_module["src/engine.rs"] == "crate::engine"
//         resolve("crate::engine", "src/lib.rs") == ["src/engine.rs"]
//         resolve("super::engine", "src/util/helper.rs") -> [] (super of crate::util::helper = crate::util; no `engine` there) — pin the real answer
//         resolve("std::collections", "src/lib.rs") == []  (external)
```
- [ ] **Step 2: Run → fails.** **Step 3:** implement `extract_mod_decls` + `build_rust_module_index` + `ModuleIndex::resolve` + store `module_index` on `CallGraph` (built alongside `imports`; init in all constructors; clear/rebuild consistent with `imports`; bump `CACHE_VERSION` **iff** `module_index` is added to the serialized `CallGraph`). **Step 4:** add inline-`mod` + `foo/mod.rs`-vs-`foo.rs` + `super::`/`self::` tests; green. **Step 5: Commit** `feat(rust): ModuleIndex — crate-root + mod-decl module-path→file resolution`

## Task 3: Wire the index into module-deps/repo-map + unqualified-call narrowing

**Files:** Modify `src/navigation/module_graph.rs` + `src/resolution.rs` · Test `tests/navigation/module_graph_test.rs` + `tests/integration/resolution_test.rs`

**Design:** (a) **module-deps/repo-map:** for each Rust import `(alias → module_path)`, `module_index.resolve(module_path, file)` → emit a **resolved** edge to each defining file; only genuinely-unresolved (external) imports keep `UnresolvedModule`. (b) **Unqualified narrowing (new rung in `resolution.rs`):** for a bare call `name()` (qualifier `None`, no `::`) where `name` is a `use`-imported alias, resolve its module path via `ModuleIndex` → narrow `functions[name]` to defs in the resolved file(s). **Recall-safe:** unresolved/external (empty resolve) or no matching def in the resolved file → **fall through** to existing behavior (never drop). Resolve to exactly one → `Exact`/`ImportQualified`.

- [ ] **Step 1: Failing tests** — (a) module-deps: `use crate::util::helper;` in `main.rs` → resolved edge to `util.rs` (replace `module_deps_rust_is_call_derived_only_no_import_items`); external `use std::…` → still `UnresolvedModule`. (b) narrowing: two `fn process` in `engine.rs`/`other.rs`; `main.rs` `use crate::engine::process; process()` resolves to `engine.rs::process` **only**.
- [ ] **Step 2: Run → fails.** **Step 3:** implement both consumers via `ModuleIndex`. **Step 4: Recall-guard tests:** no import for the name → unchanged; external import (empty resolve) → fall through; qualified `engine::process()` unchanged (out of scope); a same-named def NOT in the resolved file is excluded. **Step 5:** full `cargo test`; `cargo fmt`. **Step 6: Commit** `fix(rust): resolve use-imports for module-deps + unqualified narrowing via ModuleIndex`

## Task 4: Verification — the F3 demo + Tier-A

- [ ] **Step 1:** `cargo build --release`
- [ ] **Step 2:** the doc's verified F3 demo — `target/release/prism nav module-deps --file src/algorithms/original_diff.rs`: confirm the local `fn slice` no longer pulls in all 29 `pub fn slice` files, and Rust files now show resolved `use` edges. Paste before/after.
- [ ] **Step 3:** `cd eval && uv run tier-a --matrix-only --allow-stale-sut` (no regression) + `--quick --allow-stale-sut` (prism corpus, Rust) — confirm prism M2 callers precision/recall does not regress (expect precision-positive). Paste flips.
- [ ] **Step 4:** full `cargo test` · `--features mcp` build · `cargo fmt --check`. **Step 5:** commit any baseline/doc note if matrix flips.

## Final verification (before PR)
- [ ] `cargo fmt --check` · full `cargo test` · `cargo build --release` · `--features mcp` build
- [ ] Tier-A `--matrix-only` no regression; the `original_diff.rs` F3 demo fixed; Rust `nav module-deps` shows resolved edges
- [ ] PR body: F3 before/after + the `ModuleIndex` seam (and how C++ `using` will reuse it) + any Tier-A Rust precision delta

## Risks / watch-items
- **Module-tree completeness vs recall (headline).** A use-path the index can't resolve (deferred long-tail: glob, re-export chain, `#[path]`, cfg-mod, workspace dep, or a `mod` decl we missed) must **fall through unchanged** — never drop a candidate. Narrowing applies *only* when `resolve` returns a non-empty in-repo file set and that file actually defines the name. Recall-guard tests in Task 3 Step 4. This is the precise alternative to the heuristic: it narrows only on a *proven* module→file mapping, and is silent (recall-safe) otherwise.
- **`foo.rs` vs `foo/mod.rs` both present.** Prefer the one in `files`; if both, prefer `foo.rs` (2018) and record both as the module's files (Vec) — don't guess-drop.
- **Multiple crate roots / workspaces.** Multiple `crate` anchors; key the index per crate root (a file's module path is relative to its crate). v1 detects roots by filename convention; Cargo.toml-driven discovery deferred — an undetected root just means its files resolve as unknown (fall through), not wrong.
- **Cache.** `extract_imports`/the index are build-time; bump `CACHE_VERSION` only if `module_index` joins the serialized `CallGraph` (verify in Task 2).
- **C++ seam check:** keep `ModuleIndex` free of Rust-only assumptions (Vec value, opaque path strings, `resolve(path, from_file)`); do NOT bake `mod`/`crate::` into the struct — only into the Rust *populator*. A one-paragraph note in the PR on how C++ `using` populates it.

## Backlog / deferred (the long tail + the C++ pairing)
- **C++ `using` + namespace→file population of the SAME `ModuleIndex`** (the bundled next item — the seam is built here so C++ is a populator + extractor, not a re-architecture).
- Rust long tail: glob member expansion; `pub use` re-export chain following; `#[path]` attribute mods; `cfg`/feature-gated mods; Cargo.toml/workspace crate-root discovery; external-crate (deps) resolution; qualified `::`-path resolution via the index (R1/R2/R7 upgrade).
