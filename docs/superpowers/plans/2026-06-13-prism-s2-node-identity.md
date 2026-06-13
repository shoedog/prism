# S2 Node-Identity Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give CPG `Variable`/`Statement`/`Function` nodes additive byte-range identity, de-conflate same-name functions via `(file, name, start_line)`, recover same-line def/use ordering by byte, and make the witness wire byte-ready — so Plan B (`taint_reaches`) can delete its ordering oracle and function-identity workaround.

**Architecture:** Byte ranges are **additive** (never a dedup key, never in any `Ord`). Function identity becomes `(file, name, start_line)`; variable identity (CPG `var_index` **and** DFG `defs`/`uses` keys) gains `function_start_line`. `VarLocation` gets hand-written `Ord`/`Eq`/`Hash` over the identity tuple (byte excluded, must agree) — and IS the spec's `VarOccurrence`. AST extractors gain **byte-bearing sibling methods** (named `PathSpan`/`StatementSpan` records) so existing line-only callers stay intact. The public wire (`Location`/`SymbolRef`) carries byte **ranges**; the occurrence `ordinal` stays reserved (`0`). One `CACHE_VERSION` bump (4→5); MCP `SCHEMA_VERSION` bump (0.1→0.2).

**Tech Stack:** Rust, tree-sitter, petgraph, serde, bincode (cache), rayon. Spec: `docs/superpowers/specs/2026-06-13-prism-s2-node-identity-design.md` (rev 4 + §3/§7.3 wording fix in rev 5). Branch: `s2-node-identity`.

> **rev 2** — folds the codex xhigh plan-review (`docs/prism-query-layer/s2-plan-review-2026-06-13.md`; verdict "not executable as-is", ~12 blockers). Changes vs rev 1: explicit compile-surface enumerations (no more "compiler-guided" hand-waving); a new DFG `defs`/`uses` re-key (Task 3); sibling extractor APIs (Tasks 4, 8); corrected Task 5 byte direction + spec wording; real cache-test APIs (Task 9); concrete acceptance dumps + test helpers (Task 10); `VarAccessKind` gains `Hash`.

**Compile-surface principle:** Adding a required field to a Rust struct breaks every *literal construction* (not `match`/destructure — those take `..`). This plan enumerates every literal site from a full-codebase inventory. After each field add, `cargo build` must show **only** the enumerated sites; an unexpected site means the inventory drifted — stop and re-inventory.

---

## Pre-flight: helper inventory (do once, before Task 1)

Several tests below use shared helpers. Confirm/add them so later tasks don't block:

- [ ] **Audit existing test helpers**

Run: `rg -n "fn build_python_cpg|fn build_rust_cpg|fn parse_one|fn test_file" tests/ src/cpg/tests.rs`
- `build_python_cpg` exists in `src/cpg/tests.rs` (module-local) and may exist in `tests/ast/common`. `build_rust_cpg` does **not** exist — add it.

- [ ] **Add shared helpers to `tests/ast/common/mod.rs`** (and re-export from `tests/integration/common` if integration tests use them):

```rust
use prism::cpg::CodePropertyGraph;
use prism::languages::Language;
use std::collections::BTreeMap;

pub fn build_cpg(file: &str, src: &str, lang: Language) -> CodePropertyGraph {
    let parsed = prism::ast::ParsedFile::parse(file, src, lang).unwrap();
    let mut files = BTreeMap::new();
    files.insert(file.to_string(), parsed);
    CodePropertyGraph::build(&files)
}
pub fn build_python_cpg(src: &str) -> CodePropertyGraph { build_cpg("test.py", src, Language::Python) }
pub fn build_rust_cpg(src: &str) -> CodePropertyGraph { build_cpg("test.rs", src, Language::Rust) }
pub fn test_py() -> &'static str { "test.py" }
pub fn test_rs() -> &'static str { "test.rs" }

/// nodes_at(file,line), Variable nodes only, sorted by (start_byte,end_byte,access,index),
/// formatted "{def|use}:{path.base}". The RAW production order is asserted separately (Task 5).
pub fn same_line_var_byte_order(cpg: &CodePropertyGraph, file: &str, line: usize) -> Vec<String> {
    use prism::cpg::{CpgNode, VarAccess};
    let mut ns: Vec<_> = cpg.nodes_at(file, line).into_iter()
        .filter_map(|n| match cpg.node(n) {
            CpgNode::Variable { path, access, start_byte, end_byte, .. } =>
                Some((*start_byte, *end_byte, matches!(access, VarAccess::Use),
                      format!("{}:{}", if matches!(access, VarAccess::Def) {"def"} else {"use"}, path.base))),
            _ => None,
        }).collect();
    ns.sort_by(|a, b| (a.0, a.1, a.2).cmp(&(b.0, b.1, b.2)));
    ns.into_iter().map(|t| t.3).collect()
}

/// All CpgNode byte spans as a stable sorted dump (for cache/determinism round-trip).
pub fn node_byte_dump(cpg: &CodePropertyGraph) -> Vec<String> {
    use prism::cpg::CpgNode;
    let mut out: Vec<String> = cpg.node_indices().map(|n| match cpg.node(n) {
        CpgNode::Function { name, start_byte, end_byte, .. } => format!("fn {name} [{start_byte},{end_byte})"),
        CpgNode::Variable { path, function, line, start_byte, end_byte, .. } =>
            format!("var {function}:{line}:{} [{start_byte},{end_byte})", path.base),
        CpgNode::Statement { line, start_byte, end_byte, .. } => format!("stmt {line} [{start_byte},{end_byte})"),
    }).collect();
    out.sort();
    out
}
```

- [ ] **Commit the helpers**

```bash
cargo build --tests 2>&1 | tail -3   # helpers compile (unused-warning OK until used)
git add tests/ast/common/mod.rs && git commit -m "test(s2): shared CPG test helpers for the S2 plan

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 1: Schema fields + `VarLocation` identity + all literal updates (foundation)

Adds every new field, makes the whole tree compile with real function bytes + real `function_start_line` + best-effort (line-anchor) variable/statement bytes (real per-occurrence bytes land in Task 4), and bumps the cache version.

**Files & exact literal sites (from inventory):**
- `src/cpg/types.rs:14-36` (variants) + new ctor
- `src/data_flow.rs:13-23` (VarLocation + derives), `:33-35` (VarAccessKind `Hash`), VarLocation literals `:209,247,263,284,302,327,338,360,375,395`
- `src/access_path.rs` — already `Hash` (inventory confirmed); no change
- `src/ast.rs` — add `line_start_byte`
- `src/cpg/build.rs` — Function ctor `:205`, Variable ctors `:231,257`, Statement ctor `:529`, FunctionInfo byte join in Step 1
- `src/cpg/query.rs:280` (`to_var_location` VarLocation literal) + `:272` destructure
- `src/cpg_cache.rs:45` (CACHE_VERSION), `:350` (Function), `:362` (Variable destructure)
- **Test literals (must update — inventory):** `src/cpg/tests.rs` CpgNode literals at lines 87,94,144,149,155,162,195,200,206,213,259,291,296,301,306,341,348,355,389,396,425,432,507,517,527,551,568,579; `src/algorithms/taint.rs` VarLocation literals at 4862,5022,5323,5362,5431,11311,11318,11414,11443
- Test: `tests/ast/dfg_test.rs`, `tests/ast/cpg_test.rs`

- [ ] **Step 1: `line_start_byte` accessor (ast.rs, near node_line_range ~2051)**

```rust
    /// Byte offset where 1-indexed `line` begins. Saturates to source length for
    /// out-of-range / parse-degraded lines. Best-effort anchor for line-collapsed
    /// occurrences (S2 §3).
    pub fn line_start_byte(&self, line: usize) -> usize {
        if line == 0 { return 0; }
        self.line_offsets.get(line - 1).copied().unwrap_or(self.source.len())
    }
```

- [ ] **Step 2: Failing VarLocation identity-invariant test (`tests/ast/dfg_test.rs`)**

```rust
#[test]
fn var_location_ord_eq_hash_agree_excluding_byte() {
    use prism::data_flow::{VarLocation, VarAccessKind};
    use prism::access_path::AccessPath;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let base = |sb, eb| VarLocation {
        file: "a.rs".into(), function: "f".into(), function_start_line: 1, line: 5,
        path: AccessPath::simple("x"), start_byte: sb, end_byte: eb, kind: VarAccessKind::Use,
    };
    let (a, b) = (base(10, 11), base(99, 100));
    assert_eq!(a, b, "byte excluded from Eq");
    assert_eq!(a.cmp(&b), std::cmp::Ordering::Equal, "byte excluded from Ord");
    let h = |v: &VarLocation| { let mut s = DefaultHasher::new(); v.hash(&mut s); s.finish() };
    assert_eq!(h(&a), h(&b), "byte excluded from Hash");
    let mut c = base(10, 11); c.function_start_line = 7;
    assert_ne!(a, c);
    assert_ne!(a.cmp(&c), std::cmp::Ordering::Equal);
}
```

Run: `cargo test --test ast dfg_test::var_location_ord_eq_hash_agree_excluding_byte 2>&1 | tail -4` → Expected: compile error (missing fields).

- [ ] **Step 3: `VarAccessKind` gains `Hash` (data_flow.rs:33-35)**

```rust
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum VarAccessKind { Def, Use }
```

- [ ] **Step 4: `VarLocation` fields + hand-written `Ord`/`Eq`/`Hash` (data_flow.rs:13-23)**

Drop `PartialEq, Eq, PartialOrd, Ord` from the derive (keep `Debug, Clone, Serialize, Deserialize`):

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VarLocation {
    pub file: String,
    pub function: String,
    pub function_start_line: usize, // S2 — identity
    pub line: usize,
    pub path: AccessPath,
    pub start_byte: usize, // S2 — ADDITIVE (never identity/Ord/Eq/Hash)
    pub end_byte: usize,
    pub kind: VarAccessKind,
}
impl VarLocation {
    /// Identity tuple — byte excluded. Ord/Eq/Hash ALL derive from this so they
    /// cannot disagree (would corrupt BTreeMap/HashMap keys). Pinned by §7.6.
    fn identity_key(&self) -> (&str, &str, usize, usize, &AccessPath, VarAccessKind) {
        (&self.file, &self.function, self.function_start_line, self.line, &self.path, self.kind)
    }
}
impl PartialEq for VarLocation { fn eq(&self, o: &Self) -> bool { self.identity_key() == o.identity_key() } }
impl Eq for VarLocation {}
impl PartialOrd for VarLocation { fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(o)) } }
impl Ord for VarLocation { fn cmp(&self, o: &Self) -> std::cmp::Ordering { self.identity_key().cmp(&o.identity_key()) } }
impl std::hash::Hash for VarLocation { fn hash<H: std::hash::Hasher>(&self, s: &mut H) { self.identity_key().hash(s) } }
```

Keep `var_name()`.

- [ ] **Step 5: Populate the 10 VarLocation literals in `data_flow.rs`**

At each of `:209,247,263,284,302,327,338,360,375,395`, add (using the `start` and `parsed` in scope; `start` = the function start line from `node_line_range`):

```rust
        function_start_line: start,
        start_byte: parsed.line_start_byte(<the literal's `line`/`*ref_line`/`*def_line`/`start` value>),
        end_byte: parsed.line_start_byte(<same line value>),
```

(Best-effort zero-width; Task 4 swaps these for real spans at the node-sourced sites.)

- [ ] **Step 6: `CpgNode` fields + ctor (types.rs:14-36)**

```rust
    Function { name: String, file: String, start_line: usize, end_line: usize,
               start_byte: usize, end_byte: usize },                                  // +2
    Statement { file: String, line: usize, kind: StmtKind,
                start_byte: usize, end_byte: usize },                                 // +2 (display only)
    Variable { path: AccessPath, file: String, function: String,
               function_start_line: usize, line: usize, access: VarAccess,
               start_byte: usize, end_byte: usize },                                  // +3
```

Add `impl CpgNode { pub fn variable_occurrence(path, file, function, function_start_line, line, access, start_byte, end_byte) -> Self {…} }` (so test/reasoning callers don't hand-thread).

- [ ] **Step 7: Populate node bytes in build.rs**

Step 1 Function ctor (`:203-216`) — join `FunctionInfo` for real bytes:

```rust
        for func_ids in cg.functions.values() {
            for fid in func_ids {
                let (start_byte, end_byte) = files.get(&fid.file)
                    .and_then(|p| p.functions().iter().find(|f|
                        f.name.as_deref() == Some(fid.name.as_str()) && f.start_line == fid.start_line))
                    .map(|f| (f.start_byte, f.end_byte)).unwrap_or((0, 0));
                let idx = graph.add_node(CpgNode::Function {
                    name: fid.name.clone(), file: fid.file.clone(),
                    start_line: fid.start_line, end_line: fid.end_line, start_byte, end_byte });
                func_index.insert((fid.file.clone(), fid.name.clone()), idx); // key migrates Task 2
                location_index.entry((fid.file.clone(), fid.start_line)).or_default().push(idx);
            }
        }
```

Variable ctors (`:231,257`) copy `function_start_line`, `start_byte`, `end_byte` from `loc`. Statement ctor (`:529`) uses `start_byte: parsed.line_start_byte(line), end_byte: parsed.line_start_byte(line)`.

- [ ] **Step 8: `to_var_location` (query.rs:272-292)** — destructure + copy the new fields:

```rust
            CpgNode::Variable { path, file, function, function_start_line, line, access, start_byte, end_byte } =>
                Some(VarLocation {
                    file: file.clone(), function: function.clone(), function_start_line: *function_start_line,
                    line: *line, path: path.clone(), start_byte: *start_byte, end_byte: *end_byte,
                    kind: match access { VarAccess::Def => VarAccessKind::Def, VarAccess::Use => VarAccessKind::Use },
                }),
```

- [ ] **Step 9: Update every other CpgNode literal + cache + bump version**

- `src/cpg_cache.rs:350` Function ctor: add `start_byte`/`end_byte` from the node — but reconstruct reads serialized nodes; the cleanest is to destructure with `..` and **not** re-emit (reconstruct re-adds the *same* `node.clone()` at `:329`, so the byte fields are already preserved). Confirm `:328-330` clones the whole node (it does) → the index-rebuild loop (`:347-385`) only needs `..` in its destructures. Add `..` to the Function arm (`:350`) and Variable arm (`:362`).
- `src/cpg/tests.rs` — add `start_byte`/`end_byte` (and `function_start_line` for Variables) to all 28 literals enumerated above. For tests, `start_byte: 0, end_byte: 0, function_start_line: <the test's function line>` is fine.
- `src/algorithms/taint.rs` — add the three new fields to the 9 VarLocation literals enumerated above (`function_start_line: <fn line>`, bytes `0`).
- Bump `src/cpg_cache.rs:45`: `const CACHE_VERSION: u32 = 5; // S2 node bytes + VarLocation identity + CallSite bytes`.

Run `cargo build 2>&1 | rg "error\[|missing|expected" | head` — fix only enumerated sites; an unlisted site = inventory drift, stop.

- [ ] **Step 10: Function-byte test (`tests/ast/cpg_test.rs`)**

```rust
#[test]
fn function_node_carries_real_byte_span() {
    let src = "fn alpha() {}\nfn beta() {}\n";
    let cpg = build_rust_cpg(src);
    let (sb, eb) = cpg.function_nodes().into_iter().find_map(|n| match cpg.node(n) {
        prism::cpg::CpgNode::Function { name, start_byte, end_byte, .. } if name == "alpha" => Some((*start_byte, *end_byte)),
        _ => None }).expect("alpha");
    assert_eq!(&src[sb..eb], "fn alpha() {}");
}
```

- [ ] **Step 11: Run + commit**

Run: `cargo test --test ast dfg_test::var_location_ord_eq_hash_agree_excluding_byte cpg_test::function_node_carries_real_byte_span 2>&1 | tail -4` → PASS
Run: `cargo build && cargo test 2>&1 | tail -15` → green.

```bash
cargo fmt && git add -A && git commit -m "feat(s2): node byte ranges + VarLocation hand-written identity (Task 1)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: Function de-conflation — `func_index (file,name,start_line)` + `name_index` + `function_candidates`

**All `func_index` users (inventory):** build.rs `:32,83,91,106,197,211,307,316,410,451,461,497`; query.rs `function_node:21`, `function_nodes:90`, `function_at:345`, `callers_of:370`, `callees_of:417`, `callers_of_in_file:468`; cpg_cache.rs `:342,356,389`.

- [ ] **Step 1: Failing de-conflation test (`tests/integration/core_test.rs`)**

```rust
#[test]
fn same_name_functions_on_different_lines_are_distinct_nodes() {
    let src = "struct A; struct B;\nimpl A { fn helper(&self) -> i32 { 1 } }\nimpl B { fn helper(&self) -> i32 { 2 } }\n";
    let cpg = build_rust_cpg(src);
    let helpers: Vec<_> = cpg.function_nodes().into_iter()
        .filter(|&n| matches!(cpg.node(n), prism::cpg::CpgNode::Function { name, .. } if name == "helper")).collect();
    assert_eq!(helpers.len(), 2);
    assert_eq!(cpg.function_candidates("test.rs", "helper").len(), 2);
    assert!(cpg.function_node("test.rs", "helper").is_some());
}
```

Run → fails (compile: `function_candidates` missing).

- [ ] **Step 2: Migrate the key type + add `name_index`**

`func_index` type → `BTreeMap<(String, String, usize), NodeIndex>` at build.rs `:32,83,197` and cpg_cache.rs `:342`. Add field `pub(crate) name_index: BTreeMap<(String, String), Vec<NodeIndex>>` to the struct (build.rs `:27-55`), to `from_parts` (params + body, `:81-98`), and to `empty()` (`:103-113`).

- [ ] **Step 3: Build both indexes in Step 1 + sort `name_index` by start_line**

After the Step-1 loop populates `func_index` and `name_index` (insert `(file,name,start_line)→idx` and push to `name_index[(file,name)]`), sort each `name_index` bucket so `function_node` returns the lowest `start_line` first:

```rust
        for nodes in name_index.values_mut() {
            nodes.sort_by_key(|&n| match &graph[n] {
                CpgNode::Function { start_line, .. } => *start_line, _ => usize::MAX });
        }
```

Declare `let mut name_index = BTreeMap::new();` at `:197`; add to the returned struct (`:495`).

- [ ] **Step 4: Migrate the func_index lookups/iterations**

- build.rs `:307` caller / `:316` callee keys → `(file, name, start_line)` from the resolved `FunctionId`.
- build.rs `:410` Step 6 Contains: keep the Task-2 stopgap (lookup via `name_index` first candidate) — switched to the composite in Task 3:
  ```rust
          if let Some(&func_idx) = name_index.get(&(file.clone(), func.clone())).and_then(|v| v.first()) {
              graph.add_edge(func_idx, var_idx, CpgEdge::Contains);
          }
  ```
- build.rs `:451` Step 9 destructure → `(&(ref _file, ref name, ref _start_line), &idx)`.
- query.rs `function_node:20-24` → route through `name_index` (first candidate); add `function_candidates`:
  ```rust
  pub fn function_node(&self, file: &str, name: &str) -> Option<NodeIndex> {
      self.name_index.get(&(file.to_string(), name.to_string())).and_then(|v| v.first().copied())
  }
  pub fn function_candidates(&self, file: &str, name: &str) -> Vec<NodeIndex> {
      self.name_index.get(&(file.to_string(), name.to_string())).cloned().unwrap_or_default()
  }
  ```
- query.rs `function_at:345`, `callers_of:370`, `callers_of_in_file:468`: these iterate `func_index` and read the name from the 2-tuple key. Change the destructure to the 3-tuple `(&(ref file, ref name, ref _sl), &idx)` (the name is still in the key — behavior unchanged). `callees_of:417` does a `func_index.get((file, name))` → route via `function_node(file, name)` (first candidate) to preserve by-name behavior. `function_nodes:90` (`func_index.values()`) is unaffected.

- [ ] **Step 5: `reconstruct_cpg` (cpg_cache.rs)** — build `name_index` in the node loop, use the 3-tuple func key, sort buckets by start_line, pass `name_index` to `from_parts` (`:356,389`).

- [ ] **Step 6: Run + commit**

Run: `cargo test --test integration core_test::same_name_functions_on_different_lines_are_distinct_nodes 2>&1 | tail -4` → PASS
Run: `cargo build && cargo test 2>&1 | tail -15` → green (capture any de-conflation call-edge flips for Task 10).

```bash
cargo fmt && git add -A && git commit -m "feat(s2): func_index (file,name,start_line) + name_index + function_candidates (Task 2)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: Variable + DFG de-conflation — `var_index` AND `defs`/`uses` keys gain `function_start_line`

The CPG `var_index` and the DFG `defs`/`uses` maps are the same de-conflation, done together so same-name functions separate at both layers (spec §4, review M4b). The interprocedural edge builder (`data_flow.rs:555`) iterates the DFG key, so leaving it name-only would re-merge what the CPG just split.

**All `var_index` users (inventory):** build.rs `:36,84,198,230,238,256,264,298,377,385,397,408,498`; query.rs `var_node:67`, `all_defs_of:508`; cpg_cache.rs `:343,369,390`.
**All `defs`/`uses` sites (inventory):** type `data_flow.rs:67-69`; inserts `:115,118,216,242,277,296,313,334,349,378,389,405`; retains `:102,103`; iterations `:555,681` (3-tuple destructures); reads via `.values()` build.rs `:220,246` (unaffected).

- [ ] **Step 1: Failing variable-de-conflation test (`tests/integration/core_test.rs`)**

```rust
#[test]
fn same_path_in_same_named_functions_does_not_collide() {
    let src = "struct A; struct B;\nimpl A { fn run(&self) { let v = 1; sink(v); } }\nimpl B { fn run(&self) { let v = 2; sink(v); } }\n";
    let cpg = build_rust_cpg(src);
    let v_defs = cpg.node_indices().filter(|&n| matches!(cpg.node(n),
        prism::cpg::CpgNode::Variable { path, access, .. }
        if path.base == "v" && *access == prism::cpg::VarAccess::Def)).count();
    assert_eq!(v_defs, 2, "v is distinct per function");
}
```

Run → fails (1 != 2).

- [ ] **Step 2: Migrate `var_index` key type**

To `BTreeMap<(String, String, usize, usize, AccessPath, VarAccess), NodeIndex>` (file, function, **function_start_line**, line, path, access) at build.rs `:36,84,198` and cpg_cache.rs `:343`.

- [ ] **Step 3: Insert `function_start_line` into every var_index key**

build.rs `:223-229` (Def), `:249-255` (Use): add `loc.function_start_line` after `loc.function.clone()`. `:283-296` (Step 4 from_key/to_key): use `edge.from.function_start_line` / `edge.to.function_start_line`. `:370-398` (Step 5b): arg keys use `caller_id.start_line`, param key uses `callee_id.start_line`.

- [ ] **Step 4: Migrate the DFG `defs`/`uses` key type + inserts/retains/iterations**

Type (`data_flow.rs:67-69`):

```rust
    pub defs: BTreeMap<(String, String, usize, AccessPath), Vec<VarLocation>>, // +function_start_line
    pub uses: BTreeMap<(String, String, usize, AccessPath), Vec<VarLocation>>,
```

Every insert (`:115,118` merge — generic, unchanged; `:216,242,277,296,313,334,349,378,389,405`): add `start` (the function start line in scope) as the 3rd key element, e.g.:

```rust
        defs.entry((file_path.clone(), func_name.clone(), start, ap.clone())).or_default().push(loc);
```

Retains (`:102,103`): `self.defs.retain(|(file, _, _, _), _| !exclude.contains(file));` (4-tuple pattern). Iterations: `:555` `for ((f, func, _fsl, _path), def_locs) in &self.defs`, `:681` `for ((f, _func, _fsl, path), locs) in &self.defs`. (`merge`/`build_subset`/`.values()` callers are generic — no change.)

- [ ] **Step 5: Step 6 Contains → composite (replace Task 2 stopgap), `var_node`, `all_defs_of`, cache**

build.rs `:408-413`:
```rust
        for (&(ref file, ref func, func_start_line, ref _line, ref _path, ref _access), &var_idx) in &var_index {
            if let Some(&func_idx) = func_index.get(&(file.clone(), func.clone(), func_start_line)) {
                graph.add_edge(func_idx, var_idx, CpgEdge::Contains);
            }
        }
```
`var_node` (query.rs:59-76): add a `function_start_line: usize` param after `function`, insert into the key. Update its callers (`rg -n "\.var_node\(" src/ tests/`) to pass the function's start line.
`all_defs_of` (query.rs:508): destructure `(&(ref f, ref _func, _fsl, ref _line, ref path, ref access), &_idx)`.
cpg_cache.rs `:369`: insert the 6-tuple (`function_start_line` from the node).

- [ ] **Step 6: Run + commit**

Run: `cargo test --test integration core_test::same_path_in_same_named_functions_does_not_collide 2>&1 | tail -4` → PASS
Run: `cargo build && cargo test 2>&1 | tail -15` → green (capture flips for Task 10).

```bash
cargo fmt && git add -A && git commit -m "feat(s2): var_index + DFG defs/uses keys gain function_start_line (Task 3)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: Real per-occurrence byte spans — sibling extractor APIs + named records

ADD byte-bearing sibling methods (existing line-only methods stay for `cfg.rs`, `left_flow.rs`, `queries.rs`, the algorithms — inventory item 7). Named records, not tuples (review). `VarLocation` IS the spec's `VarOccurrence`; ast.rs returns the lighter `PathSpan`/`StatementSpan`, data_flow lifts them.

**Files:** `src/ast.rs` (new records + sibling methods), `src/data_flow.rs` (switch to siblings; lift), `src/cpg/build.rs` (real statement spans). Test: `tests/ast/cpg_test.rs`.

- [ ] **Step 1: Failing span-extent + statement-span + augmented-assign tests (`tests/ast/cpg_test.rs`)**

```rust
#[test]
fn variable_occurrence_carries_real_member_span() {
    let src = "def f(o):\n    o.config.timeout = 5\n    return o.config.timeout\n";
    let cpg = build_python_cpg(src);
    let (sb, eb) = cpg.node_indices().find_map(|n| match cpg.node(n) {
        prism::cpg::CpgNode::Variable { path, line, access, start_byte, end_byte, .. }
            if *line == 2 && *access == prism::cpg::VarAccess::Def && path.has_fields() => Some((*start_byte, *end_byte)),
        _ => None }).expect("o.config.timeout def");
    assert_eq!(&src[sb..eb], "o.config.timeout");
}

#[test]
fn statement_node_carries_real_span() {
    let src = "def f():\n    return 1\n";
    let cpg = build_python_cpg(src);
    let (sb, eb) = cpg.nodes_at("test.py", 2).into_iter().find_map(|n| match cpg.node(n) {
        prism::cpg::CpgNode::Statement { start_byte, end_byte, .. } => Some((*start_byte, *end_byte)), _ => None
    }).expect("stmt");
    assert!(eb > sb && src[sb..eb].contains("return"));
}

#[test]
fn augmented_assignment_emits_def_and_use() {
    let src = "def f(x):\n    x += 1\n    return x\n";
    let cpg = build_python_cpg(src);
    let on2: Vec<_> = cpg.nodes_at("test.py", 2).into_iter().filter_map(|n| match cpg.node(n) {
        prism::cpg::CpgNode::Variable { path, access, .. } if path.base == "x" => Some(*access), _ => None }).collect();
    assert!(on2.contains(&prism::cpg::VarAccess::Def) && on2.contains(&prism::cpg::VarAccess::Use), "x += 1 reads and writes x");
}
```

Run → `variable_occurrence_carries_real_member_span` fails (line anchor), others may already pass or fail.

- [ ] **Step 2: Named records + sibling extractors (ast.rs)**

```rust
/// A variable occurrence the parser located, with its real source span.
pub struct PathSpan { pub path: AccessPath, pub line: usize, pub start_byte: usize, pub end_byte: usize }
/// A statement the parser located, with its real source span.
pub struct StatementSpan { pub line: usize, pub kind: String, pub start_byte: usize, pub end_byte: usize }
```

Add siblings that mirror the existing methods but keep the matched node's bytes (the existing methods already walk the node — copy them, push `node.start_byte()`/`node.end_byte()` instead of discarding):
- `assignment_lvalue_spans_on_lines(func_node, lines) -> Vec<PathSpan>` (mirrors `:942`)
- `rvalue_identifier_spans_on_lines(func_node, lines) -> Vec<PathSpan>` (mirrors `:1512`)
- `function_parameter_occurrences(func_node) -> Vec<(String, usize, usize)>` (name, start_byte, end_byte; mirrors `:2926`)
- `statement_spans_in_function(func_node) -> Vec<StatementSpan>` (mirrors `:2404`)

Augmented-assignment rule: in `assignment_lvalue_spans_on_lines`, when the assignment operator is augmented (`+=`,`-=`,…), emit the lvalue as **both** a def occurrence AND (in `rvalue_identifier_spans_on_lines`) a use occurrence — so `x += 1` yields `x` Def and `x` Use. (The data_flow lift creates the two VarLocations.)

- [ ] **Step 3: data_flow.rs uses the siblings + lifts to VarLocation**

Replace the line-only calls with the span siblings at the def/use build sites (`:230` params → `function_parameter_occurrences`; `:282` lvalues → `assignment_lvalue_spans_on_lines`; `:393` rvalues → `rvalue_identifier_spans_on_lines`), and set `start_byte`/`end_byte` from each `PathSpan`. Keep `find_path_references_scoped` (lines) for the cross-line uses — those stay line-anchored (`parsed.line_start_byte(ref_line)`, §3 best-effort). Alias-resolved defs carry the **raw occurrence's** span (§3 alias rule).

- [ ] **Step 4: build.rs Statement nodes use real spans**

`collect_function_statements` (`:522`) calls `parsed.statements_in_function` → switch to `statement_spans_in_function`; ctor (`:529`) uses the `StatementSpan` bytes. (Line dedup retained — one statement node per `(file,line)`.)

- [ ] **Step 5: Per-language + destructuring + multiline-param span tests**

Add to `tests/lang/{python,rust,go,javascript}/…`: a member-access span and a destructuring per-target span (`const {a, b} = o` → `a`, `b` each their own span) and a multiline-parameter span (param token byte on a different physical line than `function_start`). Use `node_byte_dump` / direct node inspection.

- [ ] **Step 6: Run + commit**

Run: `cargo test --test ast cpg_test::variable_occurrence_carries_real_member_span cpg_test::statement_node_carries_real_span cpg_test::augmented_assignment_emits_def_and_use 2>&1 | tail -5` → PASS
Run: `cargo build && cargo test 2>&1 | tail -15` → green (bytes additive; no identity flips).

```bash
cargo fmt && git add -A && git commit -m "feat(s2): real per-occurrence byte spans via sibling extractors (Task 4)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 5: Same-line def/use ordering by byte (F1)

Sort the same-line/same-path trace arms + `location_index` Variable buckets by byte (total tie-break). **Byte order is source position (leftmost first).** In `q = p`, `q` (lhs) is at a *smaller* byte than `p` (rhs), so the order is `def:q` then `use:p` — this is the deterministic, source-faithful order the F1 fix delivers (the data-flow *direction* lives in the edge label, not the sort). This corrects the spec §3/§7.3 wording (rev 5).

**Files:** `src/cpg/trace.rs:211` (leave NodeIndex — documented), `:242,:342,:379` (byte sort), add `node_sort_key`; `src/cpg/build.rs` (location_index sort) + `src/cpg_cache.rs` (mirror).

- [ ] **Step 1: Failing same-line ordering test (`tests/ast/cpg_test.rs`)**

```rust
#[test]
fn same_line_assignment_orders_by_source_position() {
    let src = "def f(p):\n    q = p\n    return q\n";
    let cpg = build_python_cpg(src);
    // RAW nodes_at order (production order) must be byte-sorted, leftmost first.
    let order = same_line_var_byte_order(&cpg, "test.py", 2);
    let q = order.iter().position(|s| s == "def:q").unwrap();
    let p = order.iter().position(|s| s == "use:p").unwrap();
    assert!(q < p, "lhs `q` precedes rhs `p` by source byte: {order:?}");
    // And the production location_index bucket is already in this order (not just the test's sort):
    let raw: Vec<_> = cpg.nodes_at("test.py", 2).into_iter().filter_map(|n| match cpg.node(n) {
        prism::cpg::CpgNode::Variable { path, start_byte, .. } => Some((*start_byte, path.base.clone())), _ => None }).collect();
    let mut sorted = raw.clone(); sorted.sort();
    assert_eq!(raw, sorted, "location_index bucket is byte-ordered in production");
}
```

Run → fails (production bucket is NodeIndex/insertion order).

- [ ] **Step 2: `node_sort_key` + apply to the three arms (trace.rs)**

```rust
    /// Total, deterministic same-line ordering key (S2 §3): byte range, then
    /// access (Def<Use), then build-order NodeIndex. Non-Variable nodes last.
    fn node_sort_key(&self, idx: NodeIndex) -> (usize, usize, u8, usize) {
        match &self.graph[idx] {
            CpgNode::Variable { start_byte, end_byte, access, .. } =>
                (*start_byte, *end_byte, match access { VarAccess::Def => 0, VarAccess::Use => 1 }, idx.index()),
            _ => (usize::MAX, usize::MAX, 2, idx.index()),
        }
    }
```

Replace `out.sort_by_key(|i| i.index());` at `:342,:379` and `same.sort_by_key(|i| i.index());` at `:242` with `…sort_by_key(|&i| self.node_sort_key(i))`. Add a comment at `:211` leaving it NodeIndex-sorted ("general DFG neighbors across lines — not a same-line concern; build-deterministic").

- [ ] **Step 3: location_index Variable-bucket sort (build.rs, before the returned struct ~:494; mirror in cpg_cache reconstruct after the node loop)**

```rust
        for nodes in location_index.values_mut() {
            nodes.sort_by_key(|&i| match &graph[i] {
                CpgNode::Variable { start_byte, end_byte, access, .. } =>
                    (0u8, *start_byte, *end_byte, match access { VarAccess::Def=>0u8, VarAccess::Use=>1u8 }, i.index()),
                _ => (1u8, 0, 0, 0, i.index()),
            });
        }
```

- [ ] **Step 4: Spec wording fix (rev 5)**

Edit `docs/superpowers/specs/2026-06-13-prism-s2-node-identity-design.md` §3 and §7.3: replace the "`x = y` use-of-y precedes def-of-x by start_byte" claim with "same-line occurrences are ordered by `start_byte` (leftmost source position first); for `x = y` that is def-of-x then use-of-y. The sort provides determinism + source fidelity; data-flow direction is carried by the edge label, not the order." Bump the status line to rev 5.

- [ ] **Step 5: Run + commit**

Run: `cargo test --test ast cpg_test::same_line_assignment_orders_by_source_position 2>&1 | tail -4` → PASS
Run: `cargo build && cargo test 2>&1 | tail -15` → green (verify the Task 10 provenance fixture is stable).

```bash
cargo fmt && git add -A && git commit -m "feat(s2): same-line ordering by byte (F1) + spec wording fix (Task 5)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 6: Witness wire byte range + reserved ordinal

`Location` + all `SymbolRef` variants gain byte ranges; `ordinal` stays `0` (reserved). Real bytes in the witness path (`node_of`); `0` where no occurrence byte exists (module-level / FunctionId-derived where the function node isn't joined). MCP `SCHEMA_VERSION` 0.1→0.2 (the wire gained fields). **GraphNode is unchanged** (it threads existing symbols/locations — the inventory's mermaid/output GraphNode sites do NOT construct `Location`/`SymbolRef` literals, so they don't break).

**Literal sites to update (inventory):**
- `Location {` (19): module_graph.rs `:136,169,193,244`; mcp/output.rs `:268,306`; queries.rs `:90,124,156,246,361,369,473,494,507,681`; shape.rs `:239`; tests/navigation/types_test.rs `:15,76`.
- `SymbolRef::Variable` (3): queries.rs `:116,486`; shape.rs `:231`.
- `SymbolRef::Function` (8): seed.rs `:19`; queries.rs `:147,182,239,354,466,672`; mcp/output.rs `:261`.
- `SymbolRef::Statement` (1): queries.rs `:501`.
- + `SymbolRef::*` test literals in tests/navigation/types_test.rs (run cargo build to confirm).

- [ ] **Step 1: Failing witness-byte test (`tests/reasoning/` — mirror existing shape tests)**

```rust
#[test]
fn witness_node_carries_occurrence_byte_and_reserved_ordinal() {
    // existing reasoning test harness builds a CPG + trace; reuse it.
    let payload = build_witness_for("def f(p):\n    q = p\n    return q\n", "test.py", 3, "q");
    let n = &payload.nodes[0];
    match n.symbol.as_ref().unwrap() {
        prism::navigation::types::SymbolRef::Variable { start_byte, end_byte, ordinal, .. } => {
            assert!(end_byte >= start_byte);
            assert_eq!(*ordinal, 0, "reserved");
        }
        _ => panic!("variable symbol"),
    }
    assert!(n.location.end_byte >= n.location.start_byte);
}
```

(Add `build_witness_for` to the reasoning test `common`, calling the same `taint_trace` + `witness_graph_for_node` the production path uses.)

Run → fails (compile: byte fields missing).

- [ ] **Step 2: Add byte fields to the wire types (navigation/types.rs)**

`Location`: `+ pub start_byte: usize, pub end_byte: usize`. Each `SymbolRef` variant: `+ start_byte: usize, end_byte: usize` (keep `ordinal`).

- [ ] **Step 3: `node_of` (shape.rs:206-244) — real bytes, ordinal reserved**

Add `l.start_byte` / `l.end_byte` to the `to_var_location` destructure tuple (`:208-218`) and `(0,0)` in the orphan `None` arm; set them on both the `SymbolRef::Variable` (`:231`, keep `ordinal: 0` with a "RESERVED — not byte rank" comment) and the `Location` (`:239`).

- [ ] **Step 4: Populate the remaining 31 literal sites**

For each enumerated `Location`/`SymbolRef` literal: if a CPG node / function node is in hand, use its `start_byte`/`end_byte`; otherwise `start_byte: 0, end_byte: 0`. Functions: use `to_function_id`'s node bytes via an **exact** `(file,name,start_line)` lookup or `function_candidates` filter — NOT ambiguous `function_node` (review). `ordinal: 0` everywhere (unchanged). Run `cargo build` and fill exactly the listed sites.

- [ ] **Step 5: Bump `SCHEMA_VERSION` (mcp/output.rs:11)**

```rust
pub const SCHEMA_VERSION: &str = "0.2"; // S2: navigation symbols/locations carry byte ranges (additive)
```

(References in transport.rs/error.rs read the constant — no change. Check `rg -n "0\.1" tests/` for any snapshot pinning the string.)

- [ ] **Step 6: Run + re-bless witness/nav snapshots**

Run: `cargo test --test reasoning && cargo test --test navigation 2>&1 | tail -8` → the new test passes; existing snapshots that serialize `SymbolRef`/`Location` now carry byte fields → re-bless and add to the Task 10 expected-flip list (`rg -l "start_line|SymbolRef|Location" tests/reasoning tests/navigation tests/output`).
Run: `cargo build --features mcp && cargo test --features mcp 2>&1 | tail -6` → green.

```bash
cargo fmt && git add -A && git commit -m "feat(s2): witness wire byte range + reserved ordinal; SCHEMA_VERSION 0.2 (Task 6)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 7: `.function` scope-equality audit → `(function, function_start_line)`

**Sites (inventory):** trace.rs `node_file_fn:173`, filters `:338,:375`; taint.rs `:4876,:4883` (`==`); **cfg_queries.rs:244** (`target.function != src_loc.function` — the interprocedural boundary test, the one rev-1 missed); reasoning/shape.rs test cases `:426,:498,:524` (test-only, update for consistency).

- [ ] **Step 1: Failing cross-function no-leak test (`tests/integration/core_test.rs`)**

```rust
#[test]
fn same_path_does_not_taint_across_same_named_functions() {
    let src = "struct A; struct B;\nimpl A { fn run(&self, data: i32) { sink(data); } }\nimpl B { fn run(&self, data: i32) { safe(data); } }\n";
    let cpg = build_rust_cpg(src);
    let reached = reached_var_lines(&cpg, "test.rs", 2, "data"); // taint from A::run's data; helper in common
    assert!(!reached.contains(&3), "B::run not reached from A::run");
}
```

(Add `reached_var_lines` to integration `common`: seed `taint_trace` at the given var, collect reached Variable lines.)

- [ ] **Step 2: `node_file_fn` → 3-tuple (trace.rs:173)**

```rust
    fn node_file_fn(&self, idx: NodeIndex) -> Option<(String, String, usize)> {
        match &self.graph[idx] {
            CpgNode::Variable { file, function, function_start_line, .. } =>
                Some((file.clone(), function.clone(), *function_start_line)),
            _ => None,
        }
    }
```

Update its caller `:302` (`node_file_fn(next).as_ref() == Some(&start_fn)`): make `start_fn` the 3-tuple from the seed.

- [ ] **Step 3: Composite filters + cfg_queries boundary + taint**

trace.rs `:338,:375`: bind the def's `function_start_line` into `fn_start` and require `fsl2 == fn_start` alongside `f2 == function`.
cfg_queries.rs:244: `target.function != src_loc.function || target.function_start_line != src_loc.function_start_line` (a different overload IS a boundary).
taint.rs `:4876,:4883`: require `target_loc.function_start_line == func_start_line` alongside the name `==` (thread the seed function's start line in).
shape.rs test cases `:426,:498,:524`: update to compare the composite (or leave name-only with a comment if the test fixture has unique names — confirm).

- [ ] **Step 4: Audit sweep**

Run: `rg -n "\.function ==|\.function !=|f2 == function|def_fn == function|== func_name" src/` — confirm every hit is display-only or composite-scoped; note residual display-only uses in the commit body.

- [ ] **Step 5: Run + commit**

Run: `cargo test --test integration core_test::same_path_does_not_taint_across_same_named_functions 2>&1 | tail -4` → PASS
Run: `cargo build && cargo test 2>&1 | tail -15` → green.

```bash
cargo fmt && git add -A && git commit -m "feat(s2): .function scope audit -> composite incl cfg_queries boundary (Task 7)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 8: `CallSite` byte — de-collapse same-line duplicate calls

Sibling call-extraction method (existing `function_calls_on_lines{,_with_qualifier}` stay for `queries.rs:559`, `left_flow.rs:209`, `import_test.rs`). `CallSite` byte + `cmp_key`. **Byte-aware interprocedural arg binding is explicitly deferred** (the `call_argument_texts(line, callee)` path remains line-keyed — documented limitation; the CallSite byte makes a future fix additive).

**Sites (inventory):** `CallSite {` ctors at call_graph.rs `:164,351,427,454,534,624,644,867`; `cmp_key:1177`; call extractor callers item 7.

- [ ] **Step 1: Failing de-collapse test (`tests/integration/call_graph_test.rs`)**

```rust
#[test]
fn same_line_duplicate_calls_are_distinct_call_sites() {
    let src = "def caller():\n    foo(); bar()\n    baz(); baz()\n";
    let cg = build_call_graph_one("test.py", src); // helper: CallGraph::build over a 1-file map
    let baz = cg.calls.values().flatten().filter(|s| s.callee_name == "baz" && s.line == 3).count();
    assert_eq!(baz, 2, "same-line duplicate calls preserved");
}
```

Run → fails (1 != 2).

- [ ] **Step 2: `CallSite` byte fields + `cmp_key` (call_graph.rs)**

Add `#[serde(default)] pub start_byte: usize` and `pub end_byte: usize` to `CallSite` (`:24-40`). `cmp_key` (`:1177`) → append `self.start_byte, self.end_byte` after `self.line` (tuple grows to 7).

- [ ] **Step 3: Sibling call extractor + fill the 8 ctors**

Add `function_calls_with_spans_on_lines(func_node, lines) -> Vec<(String, usize, usize, usize)>` (callee, line, start_byte, end_byte) and `…_with_qualifier_and_spans` as needed, mirroring `function_calls_on_lines{,_with_qualifier}` but keeping the call node's bytes. At each of the 8 `CallSite {` ctors, source `start_byte`/`end_byte` from the widened extractor; `(0,0)` for synthesized/indirect sites (documented).

- [ ] **Step 4: Document the deferred byte-aware arg binding**

Add to spec §9 (rev 5): "Interprocedural same-line duplicate-call argument binding (`call_argument_texts(line, callee)` is line-keyed) remains a known limitation; `CallSite` byte now makes a byte-aware lookup an additive follow-up."

- [ ] **Step 5: Run + commit**

Run: `cargo test --test integration call_graph_test::same_line_duplicate_calls_are_distinct_call_sites 2>&1 | tail -4` → PASS
Run: `cargo build && cargo test 2>&1 | tail -15` → green (call-count/edge flips captured for Task 10).

```bash
cargo fmt && git add -A && git commit -m "feat(s2): CallSite byte range de-collapses same-line calls (Task 8)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 9: Cache v5 round-trip + PartialHit (in-crate unit tests)

Real API: `save_cache(cpg, file_hashes, has_type_db, cache_dir)` / `load_cache(current_hashes, has_type_db, cache_dir) -> CacheResult` (bincode). Tests go in the `#[cfg(test)]` module of `src/cpg_cache.rs` (private structs accessible), modeled on the existing `wrong_grammar_fingerprint_misses` (`:437`).

- [ ] **Step 1: Add tests in `src/cpg_cache.rs` test module (~:408-464)**

```rust
    #[test]
    fn v5_round_trip_preserves_byte_identity() {
        let dir = tempfile::tempdir().unwrap();
        let mut files = std::collections::BTreeMap::new();
        files.insert("t.py".to_string(),
            crate::ast::ParsedFile::parse("t.py", "def f(p):\n    q = p\n    return q\n", crate::languages::Language::Python).unwrap());
        let cpg = crate::cpg::CodePropertyGraph::build(&files);
        let hashes = compute_file_hashes(&files); // existing helper in this module
        save_cache(&cpg, &hashes, false, dir.path()).unwrap();
        match load_cache(&hashes, false, dir.path()) {
            CacheResult::Hit(restored) => {
                let dump = |c: &crate::cpg::CodePropertyGraph| c.node_indices().map(|n| format!("{:?}", c.node(n))).collect::<Vec<_>>();
                assert_eq!(dump(&cpg), dump(&restored), "byte fields + function_start_line survive round trip");
            }
            other => panic!("expected Hit, got {other:?}"),
        }
    }

    #[test]
    fn v4_cache_is_a_miss() {
        // Write a cache, then hand-rewrite its version header to 4 → load must Miss.
        let dir = tempfile::tempdir().unwrap();
        let files = std::collections::BTreeMap::new();
        let cpg = crate::cpg::CodePropertyGraph::empty();
        let hashes = compute_file_hashes(&files);
        save_cache(&cpg, &hashes, false, dir.path()).unwrap();
        // Deserialize, mutate version to 4, re-serialize (private structs are in-scope here).
        force_cache_version(dir.path(), 4); // small test helper added below
        assert!(matches!(load_cache(&hashes, false, dir.path()), CacheResult::Miss));
    }

    #[test]
    fn partial_hit_incremental_rebuild_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let mut files = std::collections::BTreeMap::new();
        files.insert("a.py".into(), parse_py("a.py", "def a(p):\n    return p\n"));
        files.insert("b.py".into(), parse_py("b.py", "def b(q):\n    return q\n"));
        let cpg = crate::cpg::CodePropertyGraph::build(&files);
        let h0 = compute_file_hashes(&files);
        save_cache(&cpg, &h0, false, dir.path()).unwrap();
        // change b.py only
        files.insert("b.py".into(), parse_py("b.py", "def b(q):\n    let r = q\n    return r\n"));
        let h1 = compute_file_hashes(&files);
        match load_cache(&h1, false, dir.path()) {
            CacheResult::PartialHit { cached_call_graph, cached_dfg, changed_files } => {
                let rebuilt = crate::cpg::CodePropertyGraph::build_incremental(cached_call_graph, cached_dfg, &changed_files, &files, None);
                // a.py nodes retain their byte spans through the incremental path.
                assert!(rebuilt.node_indices().any(|n| matches!(rebuilt.node(n),
                    crate::cpg::CpgNode::Variable { file, start_byte, .. } if file == "a.py" && *start_byte > 0)));
            }
            other => panic!("expected PartialHit, got {other:?}"),
        }
    }
```

Add the small private helpers `parse_py` and `force_cache_version` to the test module (the latter: read the cache file, bincode-deserialize the cache struct, set `version = v`, re-serialize — all in-crate).

- [ ] **Step 2: Run — pass (Task 1 bumped version; reconstruct fixed)**

Run: `cargo test cpg_cache:: 2>&1 | tail -10` → PASS. If `partial_hit` byte spans are 0, the gap is the `location_index`/byte mirror in `build_incremental`'s assemble path — verify Task 5 Step 3 mirror covers `reconstruct_cpg` AND that `build_incremental` reuses `assemble_graph` (it does, build.rs:183).

- [ ] **Step 3: Run full suite + commit**

Run: `cargo build && cargo test 2>&1 | tail -15` → green.

```bash
cargo fmt && git add -A && git commit -m "test(s2): cache v5 round-trip + v4-miss + PartialHit (Task 9)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 10: Acceptance — concrete expected-flip fixtures, order-invariance, determinism, Tier-A

Capture-then-freeze: the expected dumps are GENERATED by the first run, eyeballed for correctness (de-conflation/de-collapse should ADD precision, never lose edges), then pasted and committed. This is not a placeholder — it is how golden/characterization tests are authored.

- [ ] **Step 1: Edge-set expected-flip fixture (`tests/integration/core_test.rs` + `tests/fixtures/`)**

Create `tests/fixtures/s2_overload_flip.rs` (two same-name functions whose de-conflation changes reachability). Add:

```rust
#[test]
fn s2_deconfliction_edge_flips_are_exactly_enumerated() {
    let cpg = build_rust_cpg(include_str!("../fixtures/s2_overload_flip.rs"));
    let dump = normalized_edge_dump(&cpg); // helper in common: sorted "kind from->to" strings
    let expected: Vec<&str> = vec![ /* PASTE verified dump after first run; eyeball for precision-only changes */ ];
    assert_eq!(dump, expected);
}
```

Add `normalized_edge_dump` to common (iterate `cpg`'s edges, format `"{kind} {from_node_label}->{to_node_label}"`, sort). First run prints the actual; verify it shows the de-conflated (more precise) edges; paste.

- [ ] **Step 2: Order-sensitive provenance fixture**

```rust
#[test]
fn provenance_output_stable_under_var_location_ord_change() {
    let out = run_provenance_slice(include_str!("../fixtures/s2_provenance.py")); // existing algo entry
    assert_eq!(out, include_str!("../fixtures/s2_provenance.expected"));
}
```

Generate `s2_provenance.expected` from the current run; confirm it's the intended provenance; commit both fixtures.

- [ ] **Step 3: Determinism test**

```rust
#[test]
fn cpg_build_is_deterministic() {
    let src = "def f(p):\n    q = p\n    r = q\n    return r\n";
    assert_eq!(node_byte_dump(&build_python_cpg(src)), node_byte_dump(&build_python_cpg(src)));
}
```

- [ ] **Step 4: Run targeted + full + MCP**

Run: `cargo test --test integration core_test::s2_ provenance 2>&1 | tail -8` → PASS
Run: `cargo fmt --check && cargo test && cargo test --features mcp 2>&1 | tail -12` → green.

- [ ] **Step 5: Coverage matrix (only if new test FILES were added)**

If any Task added a NEW test file (most extend existing files; `s2_overload_flip.rs` is a fixture, not a test file), update the 3 `all_test_files` copies in `tests/integration/coverage_test.rs`. Verify: `cargo test --test integration coverage_test:: 2>&1 | tail -5` → PASS.

- [ ] **Step 6: Tier-A accuracy workflow (CLAUDE.md)**

Run: `cargo build --release`
Run: `cd eval && uv run tier-a --matrix-only --allow-stale-sut` → exit 0
Run: `cd eval && uv run tier-a --quick --allow-stale-sut` → review for regressions/flip-candidates; paste any into the PR description (do NOT re-baseline).

- [ ] **Step 7: Commit**

```bash
cargo fmt && git add -A && git commit -m "test(s2): acceptance — edge-flip fixtures, order-invariance, determinism (Task 10)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Final review

Dispatch the subagent-driven-development final whole-implementation reviewer, then a codex xhigh full-branch review (the owner's pre-merge step), then `superpowers:finishing-a-development-branch`. Carry into the PR description: the enumerated de-conflation/de-collapse edge flips, Tier-A flip-candidates, the `SCHEMA_VERSION` 0.1→0.2 wire bump, and the §9 deferrals (occurrence-splitting + reserved ordinal; `FunctionId.start_byte`; byte-aware interprocedural arg binding; PDG-lite; column/UTF-16) — plus the note that the Plan B re-plan must re-validate its same-line fixtures against byte (leftmost-first) order.

## Self-review (spec coverage + plan-review fold)

- Every plan-review BLOCKER addressed: VarAccessKind Hash (T1/S3); full literal enumeration incl. tests.rs + taint.rs (T1); func_index direct users incl. function_at/callers_of/callees_of/callers_of_in_file (T2); var_index incl. all_defs_of (T3); **DFG defs/uses re-key (T3/S4)**; extractor sibling APIs avoiding caller breakage (T4, T8); Task 5 byte-direction corrected + spec rev 5 (T5); full wire surface 19+12 literals (T6); function_calls_on_lines siblings (T8); cache save_cache/load_cache/bincode in-crate (T9); concrete capture-then-freeze + helpers (pre-flight, T10).
- Every MAJOR addressed: named records (T4); real statement spans (T4); anchor coverage destructuring/multiline/augmented/per-language (T4); exact function lookup not ambiguous function_node (T6); SCHEMA_VERSION decision = bump 0.2 (T6); cfg_queries `!=` boundary + DFG name-scope (T7); CallSite arg-binding deferred + documented (T8); raw nodes_at order asserted (T5).
- Spec §1–§9 → Tasks 1–10 as in the original self-review table; the rev-2 additions (DFG re-key, sibling APIs, wire enumeration, SCHEMA_VERSION) close the gaps the plan-review found.
