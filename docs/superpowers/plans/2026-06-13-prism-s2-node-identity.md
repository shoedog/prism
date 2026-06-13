# S2 Node-Identity Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give CPG `Variable`/`Statement`/`Function` nodes additive byte-range identity, de-conflate same-name functions via `(file, name, start_line)`, recover same-line def/use ordering by byte, and make the witness wire byte-ready — so Plan B (`taint_reaches`) can delete its ordering oracle and function-identity workaround.

**Architecture:** Byte ranges are **additive** (never a dedup key, never in any `Ord`). Function identity becomes `(file, name, start_line)`; variable identity gains `function_start_line`. `VarLocation` gets hand-written `Ord`/`Eq`/`Hash` over the identity tuple (byte excluded, must agree). The public wire (`Location`/`SymbolRef`/`GraphNode` + `CallSite`) carries byte **ranges**; the occurrence `ordinal` stays reserved (`0`). One `CACHE_VERSION` bump (4→5).

**Tech Stack:** Rust, tree-sitter, petgraph, serde, rayon. Spec: `docs/superpowers/specs/2026-06-13-prism-s2-node-identity-design.md` (rev 4).

**Branch:** `s2-node-identity` (already checked out; the spec + review records are committed here).

**Reading order for the implementer:** This plan is self-contained, but each task cites exact `file:line`. Tasks are ordered so the tree **compiles and tests pass after every task** — do not reorder. Adding a field to a Rust enum variant breaks every exhaustive `match`; the compiler enumerates them. Where a step says "compiler-guided," run `cargo build`, then add `..` to matches that don't need the new fields and thread the fields only at the sites this plan lists.

---

## File-touch map (decomposition)

| File | Responsibility in S2 | Tasks |
|---|---|---|
| `src/cpg/types.rs` | `CpgNode` byte/`function_start_line` fields + `Variable::occurrence` ctor | 1 |
| `src/data_flow.rs` | `VarLocation` fields + hand-written `Ord`/`Eq`/`Hash`; populate at all build sites | 1, 3, 4 |
| `src/access_path.rs` | ensure `AccessPath: Hash` (for `VarLocation: Hash`) | 1 |
| `src/ast.rs` | `line_start_byte` accessor; widen extractors to carry node bytes (`VarOccurrence`); widen call extraction | 1, 4, 8 |
| `src/cpg/build.rs` | node ctor byte population; `func_index`/`var_index` key migration; Steps 1/5/5b/6/9 | 1, 2, 3 |
| `src/cpg/query.rs` | `function_node`/`function_candidates`/`var_node`/`to_var_location`/`callers_of`/`callees_of` | 1, 2, 3, 6 |
| `src/cpg/trace.rs` | byte-sort same-line arms (:242/:342/:379); `node_file_fn` identity (:173) | 5, 7 |
| `src/reasoning/shape.rs` | `node_of` emits wire byte; ordinal reserved | 6 |
| `src/navigation/types.rs`, `src/navigation/queries.rs` | `Location`/`SymbolRef` byte fields; populate | 6 |
| `src/call_graph.rs` | `CallSite` byte fields + `cmp_key` + 8 ctor sites | 8 |
| `src/algorithms/taint.rs` | `.function` scope-equality → composite (:4876/:4883) | 7 |
| `src/cpg_cache.rs` | `CACHE_VERSION` 4→5; `reconstruct_cpg`/`from_parts` key types; PartialHit | 1, 2, 3, 9 |

---

## Task 1: Schema fields + `VarLocation` hand-written identity (foundation)

Adds every new field and makes the whole tree compile with **real function-node bytes**, **real `function_start_line`**, and **best-effort (line-anchor) variable/statement bytes** (upgraded to real per-occurrence bytes in Task 4). Bumps the cache version because the serialized node layout changes here.

**Files:**
- Modify: `src/cpg/types.rs:14-36` (CpgNode variants), add `impl CpgNode` ctor
- Modify: `src/data_flow.rs:13-23` (VarLocation), `:124-137` (rebuild_adjacency unaffected), all `VarLocation { … }` sites in `build_from_refs` (`:209,247,263,284,302,327,338` and the remainder through `:~450`)
- Modify: `src/access_path.rs` (derive `Hash` if absent)
- Modify: `src/ast.rs` (add `line_start_byte`)
- Modify: `src/cpg/build.rs:205-210,231-237,257-263,529-533` (node ctors), `:341-348` (FunctionInfo lookup for func bytes)
- Modify: `src/cpg/query.rs:272-292` (`to_var_location`), `:295-…` (`to_function_id` add `..`)
- Modify: `src/cpg_cache.rs:45` (CACHE_VERSION), `:362-377` (reconstruct Variable destructure)
- Compiler-guided: every other `match`/destructure on `CpgNode::{Variable,Function,Statement}` and `VarLocation` (add `..`)
- Test: `tests/ast/dfg_test.rs` (VarLocation identity invariant), `tests/ast/cpg_test.rs` (function node bytes)

- [ ] **Step 1: Add the `line_start_byte` accessor (byte source for best-effort anchors)**

In `src/ast.rs`, add inside `impl ParsedFile` (near `node_line_range`, ~line 2051):

```rust
    /// Byte offset where 1-indexed `line` begins. Saturates to source length for
    /// out-of-range / parse-degraded lines. Used as the best-effort byte anchor
    /// for line-collapsed occurrences (S2 §3).
    pub fn line_start_byte(&self, line: usize) -> usize {
        if line == 0 {
            return 0;
        }
        self.line_offsets
            .get(line - 1)
            .copied()
            .unwrap_or(self.source.len())
    }
```

- [ ] **Step 2: Ensure `AccessPath: Hash`**

`VarLocation: Hash` (Step 5) needs `AccessPath: Hash`. In `src/access_path.rs`, if the `AccessPath` struct derive list lacks `Hash`, add it:

```rust
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct AccessPath { /* unchanged */ }
```

Run `cargo build 2>&1 | head` — if it already derived `Hash`, no change needed.

- [ ] **Step 3: Write the failing VarLocation identity-invariant test**

In `tests/ast/dfg_test.rs`:

```rust
#[test]
fn var_location_ord_eq_hash_agree_excluding_byte() {
    use prism::data_flow::{VarLocation, VarAccessKind};
    use prism::access_path::AccessPath;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let base = |sb: usize, eb: usize| VarLocation {
        file: "a.rs".into(),
        function: "f".into(),
        function_start_line: 1,
        line: 5,
        path: AccessPath::simple("x"),
        start_byte: sb,
        end_byte: eb,
        kind: VarAccessKind::Use,
    };
    // Same identity tuple, different byte metadata.
    let a = base(10, 11);
    let b = base(99, 100);
    assert_eq!(a, b, "byte excluded from Eq");
    assert_eq!(a.cmp(&b), std::cmp::Ordering::Equal, "byte excluded from Ord");
    let h = |v: &VarLocation| { let mut s = DefaultHasher::new(); v.hash(&mut s); s.finish() };
    assert_eq!(h(&a), h(&b), "byte excluded from Hash");

    // Differing function_start_line de-conflates (the F2 fix).
    let mut c = base(10, 11);
    c.function_start_line = 7;
    assert_ne!(a, c);
    assert_ne!(a.cmp(&c), std::cmp::Ordering::Equal);
}
```

- [ ] **Step 4: Run it to confirm it fails to compile**

Run: `cargo test --test ast dfg_test::var_location_ord_eq_hash_agree_excluding_byte 2>&1 | tail -5`
Expected: compile error — `VarLocation` has no field `function_start_line`/`start_byte`/`end_byte`.

- [ ] **Step 5: Add `VarLocation` fields + hand-written `Ord`/`Eq`/`Hash`**

In `src/data_flow.rs`, replace the `VarLocation` definition (`:13-23`). Remove `PartialEq, Eq, PartialOrd, Ord` from the derive (keep `Debug, Clone, Serialize, Deserialize`):

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VarLocation {
    pub file: String,
    pub function: String,
    /// De-conflating start line of the containing function (S2 — part of identity).
    pub function_start_line: usize,
    pub line: usize,
    pub path: AccessPath,
    /// Source extent of this occurrence (S2 — ADDITIVE; never part of identity/Ord/Eq/Hash).
    pub start_byte: usize,
    pub end_byte: usize,
    pub kind: VarAccessKind,
}

impl VarLocation {
    /// The identity tuple — byte fields deliberately excluded. `Ord`, `Eq`, and
    /// `Hash` are ALL derived from this so they cannot disagree (would corrupt
    /// BTreeMap/HashMap keys). Pinned by `var_location_ord_eq_hash_agree_excluding_byte`.
    fn identity_key(&self) -> (&str, &str, usize, usize, &AccessPath, VarAccessKind) {
        (
            &self.file,
            &self.function,
            self.function_start_line,
            self.line,
            &self.path,
            self.kind,
        )
    }
}

impl PartialEq for VarLocation {
    fn eq(&self, other: &Self) -> bool {
        self.identity_key() == other.identity_key()
    }
}
impl Eq for VarLocation {}
impl PartialOrd for VarLocation {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for VarLocation {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.identity_key().cmp(&other.identity_key())
    }
}
impl std::hash::Hash for VarLocation {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.identity_key().hash(state);
    }
}
```

Keep the existing `var_name()` accessor (`:25-30`).

- [ ] **Step 6: Populate the new fields at every `VarLocation` construction site in `data_flow.rs`**

`build_from_refs` has `start` (function start line) and `parsed` in scope at every site. For each `VarLocation { … }` literal in `build_from_refs` (`:209, 247, 263, 284, 302, 327, 338`, and the remaining sites through ~`:450` — `cargo build` lists them all), add:

```rust
        function_start_line: start,
        start_byte: parsed.line_start_byte(/* the same `line` value this literal uses */),
        end_byte: parsed.line_start_byte(/* same `line` */),
```

(Best-effort zero-width line anchor; Task 4 replaces these with real node spans for the node-sourced occurrences.) Example — the param-def site at `:247-253` becomes:

```rust
                        let loc = VarLocation {
                            file: file_path.clone(),
                            function: func_name.clone(),
                            function_start_line: start,
                            line: start,
                            path: path.clone(),
                            start_byte: parsed.line_start_byte(start),
                            end_byte: parsed.line_start_byte(start),
                            kind: VarAccessKind::Def,
                        };
```

- [ ] **Step 7: Add byte/`function_start_line` fields to `CpgNode` + an occurrence ctor**

In `src/cpg/types.rs`, update the three variants (`:14-36`):

```rust
    Function {
        name: String,
        file: String,
        start_line: usize,
        end_line: usize,
        start_byte: usize, // S2 additive
        end_byte: usize,   // S2 additive
    },
    Statement {
        file: String,
        line: usize,
        kind: StmtKind,
        start_byte: usize, // S2 additive (display only; node stays line-keyed)
        end_byte: usize,
    },
    Variable {
        path: AccessPath,
        file: String,
        function: String,
        function_start_line: usize, // S2 — part of variable identity
        line: usize,
        access: VarAccess,
        start_byte: usize, // S2 additive
        end_byte: usize,
    },
```

Add a constructor so non-builder callers don't hand-thread bytes:

```rust
impl CpgNode {
    /// Build a `Variable` node from a `VarLocation`-shaped occurrence.
    #[allow(clippy::too_many_arguments)]
    pub fn variable_occurrence(
        path: AccessPath,
        file: String,
        function: String,
        function_start_line: usize,
        line: usize,
        access: VarAccess,
        start_byte: usize,
        end_byte: usize,
    ) -> Self {
        CpgNode::Variable {
            path, file, function, function_start_line, line, access, start_byte, end_byte,
        }
    }
}
```

- [ ] **Step 8: Populate node bytes in `build.rs` ctors + join function bytes**

`src/cpg/build.rs` Step 1 (`:203-216`) — join each `FunctionId` to its `FunctionInfo` for real bytes (`FunctionInfo.start_byte/end_byte` exist, ast.rs:52-53):

```rust
        for func_ids in cg.functions.values() {
            for fid in func_ids {
                let (start_byte, end_byte) = files
                    .get(&fid.file)
                    .and_then(|p| {
                        p.functions().iter().find(|f| {
                            f.name.as_deref() == Some(fid.name.as_str())
                                && f.start_line == fid.start_line
                        })
                    })
                    .map(|f| (f.start_byte, f.end_byte))
                    .unwrap_or((0, 0));
                let idx = graph.add_node(CpgNode::Function {
                    name: fid.name.clone(),
                    file: fid.file.clone(),
                    start_line: fid.start_line,
                    end_line: fid.end_line,
                    start_byte,
                    end_byte,
                });
                func_index.insert((fid.file.clone(), fid.name.clone()), idx); // key migrates in Task 2
                location_index
                    .entry((fid.file.clone(), fid.start_line))
                    .or_default()
                    .push(idx);
            }
        }
```

Variable ctors at `:231-237` (Def) and `:257-263` (Use) copy from `loc`:

```rust
                    let idx = graph.add_node(CpgNode::Variable {
                        path: loc.path.clone(),
                        file: loc.file.clone(),
                        function: loc.function.clone(),
                        function_start_line: loc.function_start_line,
                        line: loc.line,
                        access,
                        start_byte: loc.start_byte,
                        end_byte: loc.end_byte,
                    });
```

Statement ctor at `:529-533` — best-effort line anchor (statements stay line-keyed; real span optional, Task 4 note):

```rust
                let idx = graph.add_node(CpgNode::Statement {
                    file: file.to_string(),
                    line,
                    kind,
                    start_byte: parsed.line_start_byte(line),
                    end_byte: parsed.line_start_byte(line),
                });
```

- [ ] **Step 9: Update `to_var_location` + compiler-guided match fixes**

`src/cpg/query.rs:272-292` — destructure the new fields and copy them onto `VarLocation`:

```rust
            CpgNode::Variable {
                path, file, function, function_start_line, line, access, start_byte, end_byte,
            } => Some(VarLocation {
                file: file.clone(),
                function: function.clone(),
                function_start_line: *function_start_line,
                line: *line,
                path: path.clone(),
                start_byte: *start_byte,
                end_byte: *end_byte,
                kind: match access {
                    VarAccess::Def => VarAccessKind::Def,
                    VarAccess::Use => VarAccessKind::Use,
                },
            }),
```

`to_function_id` (`:295+`) destructures `Function { name, file, start_line, end_line }` — add `..` (it doesn't need bytes).

Then run `cargo build` and add `..` to every other reported non-exhaustive match on the three variants (trace.rs:316-322 already uses `..`; :351-357 lists all Variable fields → add `start_byte`/`end_byte`/`function_start_line` or `..`). Do NOT change behavior — these are pattern-completeness only.

- [ ] **Step 10: Fix `reconstruct_cpg` destructure + bump cache version**

`src/cpg_cache.rs:362-377` Variable arm — add `..` (it only needs `path,file,function,line,access` for the index, which migrates in Tasks 2/3):

```rust
            CpgNode::Variable {
                path, file, function, line, access, ..
            } => {
                var_index.insert(
                    (file.clone(), function.clone(), *line, path.clone(), *access),
                    idx,
                );
```

Bump `src/cpg_cache.rs:45`:

```rust
const CACHE_VERSION: u32 = 5; // S2: node byte ranges + VarLocation identity + CallSite bytes
```

- [ ] **Step 11: Write a function-node-byte test**

In `tests/ast/cpg_test.rs`:

```rust
#[test]
fn function_node_carries_real_byte_span() {
    let src = "fn alpha() {}\nfn beta() {}\n";
    let cpg = build_rust_cpg(src); // existing helper; if absent, mirror build_python_cpg with Language::Rust
    let f = cpg
        .function_nodes()
        .into_iter()
        .find_map(|n| match cpg.node(n) {
            prism::cpg::CpgNode::Function { name, start_byte, end_byte, .. } if name == "alpha" =>
                Some((*start_byte, *end_byte)),
            _ => None,
        })
        .expect("alpha node");
    assert!(f.1 > f.0, "non-empty span");
    assert_eq!(&src[f.0..f.1], "fn alpha() {}", "span covers the definition");
}
```

(If `build_rust_cpg` doesn't exist in `tests/ast/common`, add it next to `build_python_cpg`.)

- [ ] **Step 12: Run the suite**

Run: `cargo test --test ast dfg_test::var_location_ord_eq_hash_agree_excluding_byte 2>&1 | tail -3`
Expected: PASS
Run: `cargo test --test ast cpg_test::function_node_carries_real_byte_span 2>&1 | tail -3`
Expected: PASS
Run: `cargo build && cargo test 2>&1 | tail -15`
Expected: full suite green (byte fields are additive; identity unchanged except the new `function_start_line` field is constant per function so no edge set changes yet).

- [ ] **Step 13: Format + commit**

```bash
cargo fmt
git add -A
git commit -m "feat(s2): node byte ranges + VarLocation hand-written identity (Task 1)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: Function de-conflation — `func_index` `(file, name, start_line)` + `function_candidates`

Fixes F2 (same-name functions in one file conflate via last-writer-wins). Name stays in the key (zero resolver change; by-name queries keep working). Adds the overload-aware API.

**Files:**
- Modify: `src/cpg/build.rs:32` (struct field type), `:83` (from_parts param), `:197` (assemble local), `:211` (insert), `:306-316` (Step 5 keys), `:341-348` (Step 5b param source), `:451` (Step 9 destructure)
- Modify: `src/cpg/query.rs:20-24` (`function_node`), add `function_candidates`; `:364-451` (`callers_of`/`callees_of` unaffected — they read `call_graph`, verify)
- Modify: `src/cpg_cache.rs:342` (reconstruct func_index type), `:356` (insert), `:387` (from_parts call)
- Add: secondary `name_index: BTreeMap<(String,String), Vec<NodeIndex>>` on `CodePropertyGraph`
- Test: `tests/integration/core_test.rs` (de-conflation), `tests/integration/call_graph_test.rs` (overload edge)

- [ ] **Step 1: Write the failing de-conflation test**

In `tests/integration/core_test.rs`:

```rust
#[test]
fn same_name_functions_on_different_lines_are_distinct_nodes() {
    // Two `helper` fns in one file (e.g. Rust trait impls) must be 2 func nodes.
    let src = "\
struct A; struct B;
impl A { fn helper(&self) -> i32 { 1 } }
impl B { fn helper(&self) -> i32 { 2 } }
";
    let cpg = build_rust_cpg(src);
    let helpers: Vec<_> = cpg
        .function_nodes()
        .into_iter()
        .filter(|&n| matches!(cpg.node(n), prism::cpg::CpgNode::Function { name, .. } if name == "helper"))
        .collect();
    assert_eq!(helpers.len(), 2, "both `helper` definitions are distinct nodes");
    // function_candidates surfaces both; function_node returns one.
    let cands = cpg.function_candidates(/*file*/ test_file(), "helper");
    assert_eq!(cands.len(), 2);
    assert!(cpg.function_node(test_file(), "helper").is_some());
}
```

(`test_file()` = the filename `build_rust_cpg` uses, e.g. `"test.rs"`.)

- [ ] **Step 2: Run it — fails (only 1 node today, no `function_candidates`)**

Run: `cargo test --test integration core_test::same_name_functions_on_different_lines_are_distinct_nodes 2>&1 | tail -6`
Expected: compile error (`function_candidates` missing) or assert failure (1 != 2).

- [ ] **Step 3: Migrate the `func_index` key type to `(String, String, usize)`**

Change the type at all four sites (`build.rs:32, 83, 197`, `cpg_cache.rs:342`):

```rust
    pub(crate) func_index: BTreeMap<(String, String, usize), NodeIndex>,
```

Add a secondary name index field on `CodePropertyGraph` (`build.rs:27-55` struct) and to `from_parts`/`empty`/`from_parts` callers:

```rust
    /// Secondary index: (file, name) → all function nodes with that name (overloads).
    pub(crate) name_index: BTreeMap<(String, String), Vec<NodeIndex>>,
```

- [ ] **Step 4: Populate both indexes in Step 1 (`build.rs:211`)**

```rust
                func_index.insert(
                    (fid.file.clone(), fid.name.clone(), fid.start_line),
                    idx,
                );
                name_index
                    .entry((fid.file.clone(), fid.name.clone()))
                    .or_default()
                    .push(idx);
```

Declare `let mut name_index: BTreeMap<(String, String), Vec<NodeIndex>> = BTreeMap::new();` next to the other index locals (`:197-200`), and add it to the returned `CodePropertyGraph { … }` (`:495-503`), `from_parts` (params + body), and `empty()`.

- [ ] **Step 5: Update Step 5 / Step 5b / Step 6 / Step 9 lookups**

Step 5 caller/callee keys (`:306, :315`) — the resolved `FunctionId` carries `start_line`:

```rust
            let caller_key = (caller_id.file.clone(), caller_id.name.clone(), caller_id.start_line);
            // …
                    let callee_key = (callee_id.file.clone(), callee_id.name.clone(), callee_id.start_line);
```

Step 5b param source (`:341-348`) — match on `(name, start_line)`, not first-name:

```rust
                    let Some(info) = callee_parsed.functions().iter().find(|f| {
                        f.name.as_deref() == Some(callee_id.name.as_str())
                            && f.start_line == callee_id.start_line
                    }) else {
                        continue;
                    };
```

Step 6 Contains (`:408-411`) — the destructure tuple gains `function_start_line`, and the func lookup uses it:

```rust
        for (&(ref file, ref func, func_start_line, ref _line, ref _path, ref _access), &var_idx)
            in &var_index
        {
            let func_key = (file.clone(), func.clone(), func_start_line);
            if let Some(&func_idx) = func_index.get(&func_key) {
                graph.add_edge(func_idx, var_idx, CpgEdge::Contains);
            }
        }
```

(NOTE: the `var_index` key gains `function_start_line` in Task 3; until then this destructure won't match. To keep Task 2 green, temporarily key Step 6 via the `name_index` first candidate: `func_index` lookup by `(file, func)` is ambiguous now. **Resolution:** do Step 6's switch to the composite in Task 3 where `var_index` gains the line. For Task 2, leave Step 6 using a name-based lookup through `name_index`:)

```rust
        for (&(ref file, ref func, ref _line, ref _path, ref _access), &var_idx) in &var_index {
            if let Some(nodes) = name_index.get(&(file.clone(), func.clone())) {
                if let Some(&func_idx) = nodes.first() {
                    graph.add_edge(func_idx, var_idx, CpgEdge::Contains);
                }
            }
        }
```

Step 9 virtual dispatch destructure (`:451`):

```rust
                    for (&(ref _file, ref name, ref _start_line), &idx) in &func_index {
```

- [ ] **Step 6: Add `function_candidates` + keep `function_node`**

`src/cpg/query.rs:20-24`:

```rust
    /// Get a function node by file and name. Returns the unique node, or the
    /// first by start_line when the name is overloaded (back-compat). Callers
    /// needing all overloads use [`Self::function_candidates`].
    pub fn function_node(&self, file: &str, name: &str) -> Option<NodeIndex> {
        self.name_index
            .get(&(file.to_string(), name.to_string()))
            .and_then(|v| v.first().copied())
    }

    /// All function nodes with `(file, name)` — the overload-aware lookup.
    pub fn function_candidates(&self, file: &str, name: &str) -> Vec<NodeIndex> {
        self.name_index
            .get(&(file.to_string(), name.to_string()))
            .cloned()
            .unwrap_or_default()
    }
```

`function_nodes()` (`:89-91`) keeps using `func_index.values()`.

- [ ] **Step 7: Update `reconstruct_cpg`**

`src/cpg_cache.rs` — build `name_index` alongside `func_index` in the node loop (`:350-361`), use the new func_index key, and pass `name_index` to `from_parts` (`:387`):

```rust
            CpgNode::Function { name, file, start_line, .. } => {
                func_index.insert((file.clone(), name.clone(), *start_line), idx);
                name_index.entry((file.clone(), name.clone())).or_default().push(idx);
                location_index.entry((file.clone(), *start_line)).or_default().push(idx);
            }
```

Declare `let mut name_index = BTreeMap::new();` (`:342`) and extend the `from_parts` signature + call.

- [ ] **Step 8: Run the test + suite**

Run: `cargo test --test integration core_test::same_name_functions_on_different_lines_are_distinct_nodes 2>&1 | tail -4`
Expected: PASS
Run: `cargo build && cargo test 2>&1 | tail -15`
Expected: green. If a previously-conflated call edge now resolves differently, that's an **expected de-conflation flip** — capture it for the Task 10 expected-flip list, do not "fix" it back.

- [ ] **Step 9: Format + commit**

```bash
cargo fmt && git add -A
git commit -m "feat(s2): func_index (file,name,start_line) + function_candidates (Task 2)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: Variable de-conflation — `var_index` key gains `function_start_line`

Distinct same-name functions now get distinct variable nodes (no cross-function variable collision). Switches Step 6 to the composite (replacing Task 2's name_index stopgap).

**Files:**
- Modify: `src/cpg/build.rs:36,84,198` (key type), `:223-229,249-255` (Step 2-3 keys), `:283-296` (Step 4 keys), `:370-398` (Step 5b arg/param keys), `:408-413` (Step 6 → composite)
- Modify: `src/cpg/query.rs:59-76` (`var_node` gains `function_start_line`)
- Modify: `src/cpg_cache.rs:343,369-372` (reconstruct var_index key)
- Test: `tests/integration/core_test.rs`

- [ ] **Step 1: Write the failing variable-de-conflation test**

```rust
#[test]
fn same_path_in_same_named_functions_does_not_collide() {
    let src = "\
struct A; struct B;
impl A { fn run(&self) { let v = 1; sink(v); } }
impl B { fn run(&self) { let v = 2; sink(v); } }
";
    let cpg = build_rust_cpg(src);
    // `v` Def appears in BOTH run() bodies → two distinct Def nodes.
    let v_defs: Vec<_> = cpg
        .node_indices()
        .filter(|&n| matches!(cpg.node(n),
            prism::cpg::CpgNode::Variable { path, access, .. }
            if path.base == "v" && *access == prism::cpg::VarAccess::Def))
        .collect();
    assert_eq!(v_defs.len(), 2, "v is distinct per function (function_start_line de-conflates)");
}
```

- [ ] **Step 2: Run — fails (one `v` Def collapses both today)**

Run: `cargo test --test integration core_test::same_path_in_same_named_functions_does_not_collide 2>&1 | tail -4`
Expected: assert failure (1 != 2).

- [ ] **Step 3: Migrate the `var_index` key type**

At `build.rs:36, 84, 198` and `cpg_cache.rs:343`:

```rust
    pub(crate) var_index:
        BTreeMap<(String, String, usize, usize, AccessPath, VarAccess), NodeIndex>,
    //          file    function fn_start line  path        access
```

- [ ] **Step 4: Insert `function_start_line` into every `var_index` key construction**

Step 2-3 (`:223-229` Def, `:249-255` Use) — insert `loc.function_start_line` after `loc.function.clone()`:

```rust
                let key = (
                    loc.file.clone(),
                    loc.function.clone(),
                    loc.function_start_line,
                    loc.line,
                    loc.path.clone(),
                    access,
                );
```

Step 4 (`:283-296`) — `edge.from`/`edge.to` are `VarLocation`s, so use `.function_start_line`:

```rust
            let from_key = (
                edge.from.file.clone(),
                edge.from.function.clone(),
                edge.from.function_start_line,
                edge.from.line,
                edge.from.path.clone(),
                from_access,
            );
            // …same shape for to_key from edge.to…
```

Step 5b arg/param keys (`:370-398`) — the arg lives in the **caller**, the param in the **callee**; use each function's start line (`caller_id.start_line`, `callee_id.start_line`):

```rust
                        let arg_key = (
                            caller_id.file.clone(),
                            caller_id.name.clone(),
                            caller_id.start_line,
                            site.line,
                            arg_path.clone(),
                            VarAccess::Use,
                        );
                        // def_key: same but VarAccess::Def
                        // param_idx loop key: callee_id.file, callee_id.name, callee_id.start_line, line, param_path, Def
```

- [ ] **Step 5: Switch Step 6 Contains to the composite (replace Task 2 stopgap)**

`build.rs:408-413`:

```rust
        for (&(ref file, ref func, func_start_line, ref _line, ref _path, ref _access), &var_idx)
            in &var_index
        {
            let func_key = (file.clone(), func.clone(), func_start_line);
            if let Some(&func_idx) = func_index.get(&func_key) {
                graph.add_edge(func_idx, var_idx, CpgEdge::Contains);
            }
        }
```

- [ ] **Step 6: Extend `var_node` (query.rs:59-76)**

```rust
    pub fn var_node(
        &self,
        file: &str,
        function: &str,
        function_start_line: usize,
        line: usize,
        path: &AccessPath,
        access: VarAccess,
    ) -> Option<NodeIndex> {
        self.var_index
            .get(&(
                file.to_string(),
                function.to_string(),
                function_start_line,
                line,
                path.clone(),
                access,
            ))
            .copied()
    }
```

Run `cargo build` and update the 2 internal callers + any test callers the compiler flags (pass the function's `start_line`; for tests, the function's definition line).

- [ ] **Step 7: Update `reconstruct_cpg` var_index (cpg_cache.rs:369-372)**

```rust
            CpgNode::Variable { path, file, function, function_start_line, line, access, .. } => {
                var_index.insert(
                    (file.clone(), function.clone(), *function_start_line, *line, path.clone(), *access),
                    idx,
                );
                location_index.entry((file.clone(), *line)).or_default().push(idx);
            }
```

- [ ] **Step 8: Run test + suite**

Run: `cargo test --test integration core_test::same_path_in_same_named_functions_does_not_collide 2>&1 | tail -4`
Expected: PASS
Run: `cargo build && cargo test 2>&1 | tail -15`
Expected: green (capture any de-conflation edge flips for Task 10).

- [ ] **Step 9: Format + commit**

```bash
cargo fmt && git add -A
git commit -m "feat(s2): var_index gains function_start_line — variable de-conflation (Task 3)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: Real per-occurrence variable bytes (widen extractors)

Replaces Task 1's line-anchor placeholders with real spans for node-sourced occurrences (lvalues, rvalues, params), per the §3 anchor table. Line-collapsed uses (from `find_path_references_scoped`) keep the best-effort line anchor — that helper returns lines only, by design.

**Files:**
- Modify: `src/ast.rs:942` (`assignment_lvalue_paths_on_lines`), `:980` (`extract_assignment_lvalue_paths`), `:1512` (`rvalue_identifier_paths_on_lines`), `:1637` (`collect_identifier_paths`), `:2926` (`function_parameter_names`)
- Modify: `src/data_flow.rs:280-318` (lvalue defs), `:230-278` (params), rvalue use sites
- Test: `tests/ast/cpg_test.rs` (span anchor table), `tests/ast/dfg_test.rs`

- [ ] **Step 1: Write the failing span-extent test**

```rust
#[test]
fn variable_occurrence_carries_real_member_span() {
    let src = "def f(o):\n    o.config.timeout = 5\n    return o.config.timeout\n";
    let cpg = build_python_cpg(src);
    // The lvalue `o.config.timeout` on line 2 has a real multi-char span, not a line anchor.
    let def = cpg.node_indices().find_map(|n| match cpg.node(n) {
        prism::cpg::CpgNode::Variable { path, line, access, start_byte, end_byte, .. }
            if *line == 2 && *access == prism::cpg::VarAccess::Def && path.has_fields() =>
            Some((*start_byte, *end_byte)),
        _ => None,
    }).expect("o.config.timeout def");
    assert!(def.1 - def.0 >= "o.config.timeout".len(), "span covers the whole access path");
    assert_eq!(&src[def.0..def.1], "o.config.timeout");
}
```

- [ ] **Step 2: Run — fails (line anchor today: end==start)**

Run: `cargo test --test ast cpg_test::variable_occurrence_carries_real_member_span 2>&1 | tail -4`
Expected: assert failure (zero-width span).

- [ ] **Step 3: Widen the lvalue extractor to carry the matched node's bytes**

`src/ast.rs:942` `assignment_lvalue_paths_on_lines` currently returns `Vec<(AccessPath, usize)>` and is built by `extract_assignment_lvalue_paths` (`:980`), which has the lvalue tree-sitter node in hand before converting to `AccessPath`. Change the return to carry the span (use a named tuple for clarity):

```rust
    /// `(path, line, start_byte, end_byte)` — span is the matched lvalue node's extent.
    pub fn assignment_lvalue_paths_on_lines(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<(AccessPath, usize, usize, usize)> {
```

In `extract_assignment_lvalue_paths` (`:980`), at each point it pushes `(path, line)`, also push `node.start_byte()` and `node.end_byte()` from the lvalue `Node` it already matched (do NOT re-parse text). The byte offsets are tree-sitter byte offsets (already what `start_byte/end_byte` mean elsewhere).

- [ ] **Step 4: Widen rvalue + parameter extractors the same way**

`rvalue_identifier_paths_on_lines` (`:1512`) / `collect_identifier_paths` (`:1637`): return `(AccessPath, usize, usize, usize)`; take the byte span from the identifier/member `Node` already walked (it currently pushes `(AccessPath, usize)`).

`function_parameter_names` (`:2926`) returns `Vec<String>`. Add a sibling that keeps spans (don't break existing callers that only want names):

```rust
    /// Parameter occurrences with the parameter token's byte span.
    pub fn function_parameter_occurrences(&self, func_node: &Node<'_>) -> Vec<(String, usize, usize)> {
        // mirror function_parameter_names, but push (name, ident_node.start_byte(), ident_node.end_byte())
    }
```

- [ ] **Step 5: Thread real bytes onto `VarLocation` in `data_flow.rs`**

Update the lvalue loops (`:280-318`) to consume `(path, line, sb, eb)` and set `start_byte: *sb, end_byte: *eb` on the def `VarLocation`. Update the param loop (`:230-278`) to use `function_parameter_occurrences` and set the param token span. For **uses derived from `find_path_references_scoped`** (`:259-277, 320-349`) keep `start_byte/end_byte = parsed.line_start_byte(ref_line)` — these are line-collapsed (best-effort anchor, §3). For the resolved-alias defs (`:300-317`), carry the **same span as the raw occurrence** (the §3 alias rule).

- [ ] **Step 6: Run test + suite**

Run: `cargo test --test ast cpg_test::variable_occurrence_carries_real_member_span 2>&1 | tail -4`
Expected: PASS
Run: `cargo build && cargo test 2>&1 | tail -15`
Expected: green (bytes are additive; identity excludes them, so no edge flips here).

- [ ] **Step 7: Format + commit**

```bash
cargo fmt && git add -A
git commit -m "feat(s2): real per-occurrence byte spans via widened extractors (Task 4)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 5: Same-line def/use ordering by byte (F1)

Replaces the three `NodeIndex` (insertion-order) sorts in the same-line/same-path trace arms with byte order + the §3 total tie-break. The general DataFlow neighbor sort (`:211`) stays `NodeIndex`-sorted (not a same-line concern).

**Files:**
- Modify: `src/cpg/trace.rs:242` (assignment-propagation arm), `:342` (`same_function_same_path_uses_any_line`), `:379` (`same_line_same_path_uses`); add a `node_sort_key` helper
- Modify: `src/cpg/build.rs` location_index byte-sort (after Step 7, before return) — sort each bucket's Variable nodes by the same key
- Test: `tests/ast/cpg_test.rs`

- [ ] **Step 1: Write the failing same-line ordering test**

```rust
#[test]
fn same_line_assignment_orders_use_before_def_by_byte() {
    // `q = p` on one line: use-of-p (rhs) precedes def-of-q (lhs) by byte.
    let src = "def f(p):\n    q = p\n    return q\n";
    let cpg = build_python_cpg(src);
    // Through the assignment-propagation arm, taint from a use of p reaches the def of q.
    // Assert ordering via the public same-line helper proxy: the def-of-q's same-line
    // recovered neighbors are byte-ordered. Use a trace and check the witness chain order.
    let order = same_line_var_byte_order(&cpg, "test.py", 2); // test helper below
    let p_use = order.iter().position(|s| s == "use:p").unwrap();
    let q_def = order.iter().position(|s| s == "def:q").unwrap();
    assert!(p_use < q_def, "rhs use precedes lhs def by byte: {order:?}");
}
```

Add `same_line_var_byte_order` to `tests/ast/common` — collect `cpg.nodes_at(file, line)`, keep Variable nodes, sort by `(start_byte, end_byte, access, index)`, format `"{access}:{path.base}"`.

- [ ] **Step 2: Run — fails (NodeIndex order is insertion order, not byte)**

Run: `cargo test --test ast cpg_test::same_line_assignment_orders_use_before_def_by_byte 2>&1 | tail -5`
Expected: assert failure (or wrong order).

- [ ] **Step 3: Add the byte sort key + apply to the three arms**

In `src/cpg/trace.rs` (inside `impl CodePropertyGraph` near the trace helpers):

```rust
    /// Total, deterministic sort key for same-line/same-path occurrence ordering
    /// (S2 §3): byte range, then access (Def<Use), then build-order NodeIndex.
    /// Non-Variable nodes sort last. Byte is ADDITIVE display, but the *order* it
    /// induces is the F1 fix that retires Plan B's ordering oracle.
    fn node_sort_key(&self, idx: NodeIndex) -> (usize, usize, u8, usize) {
        match &self.graph[idx] {
            CpgNode::Variable { start_byte, end_byte, access, .. } => (
                *start_byte,
                *end_byte,
                match access { VarAccess::Def => 0, VarAccess::Use => 1 },
                idx.index(),
            ),
            _ => (usize::MAX, usize::MAX, 2, idx.index()),
        }
    }
```

Replace `out.sort_by_key(|i| i.index());` at `:342` and `:379`, and `same.sort_by_key(|i| i.index());` at `:242`, with:

```rust
        out.sort_by_key(|&i| self.node_sort_key(i));   // (:342, :379)
        same.sort_by_key(|&i| self.node_sort_key(i));  // (:242)
```

Leave `df.sort_by_key(|i| i.index());` at `:211` unchanged (document with a one-line comment: "general DFG neighbors — not a same-line concern; NodeIndex is build-deterministic").

- [ ] **Step 4: Byte-sort the `location_index` Variable buckets in `build.rs`**

After Step 7 statement collection and before constructing the returned struct (`build.rs:~494`), add:

```rust
        // S2 §4: order each location bucket's Variable nodes by byte (total tie-break),
        // so same-line consumers see deterministic byte order. Non-Variable nodes keep
        // their relative position (functions/statements are line-keyed).
        for nodes in location_index.values_mut() {
            nodes.sort_by_key(|&i| match &graph[i] {
                CpgNode::Variable { start_byte, end_byte, access, .. } =>
                    (0u8, *start_byte, *end_byte, match access { VarAccess::Def=>0u8, VarAccess::Use=>1u8 }, i.index()),
                _ => (1u8, 0, 0, 0, i.index()),
            });
        }
```

Mirror this sort in `reconstruct_cpg` (`cpg_cache.rs`, after the node loop) so cached and fresh builds agree.

- [ ] **Step 5: Run test + suite**

Run: `cargo test --test ast cpg_test::same_line_assignment_orders_use_before_def_by_byte 2>&1 | tail -4`
Expected: PASS
Run: `cargo build && cargo test 2>&1 | tail -15`
Expected: green. Order-sensitive consumers (e.g. `provenance_slice`) may change output — verify against the Task 10 provenance fixture; a genuine improvement is an expected flip, a regression fails.

- [ ] **Step 6: Format + commit**

```bash
cargo fmt && git add -A
git commit -m "feat(s2): same-line def/use ordering by byte (F1, Task 5)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 6: Witness wire byte + reserved ordinal

`Location`/`SymbolRef`/`GraphNode` carry byte **ranges** (occurrence-precise for node-sourced, best-effort for line-collapsed). `ordinal` stays `0` (reserved — its disambiguation domain collapses under var_index dedup; populating it is deferred with occurrence-splitting, §9).

**Files:**
- Modify: `src/navigation/types.rs:4-8` (`Location`), `:25-33` (`SymbolRef::Variable`; functions/statements get bytes too)
- Modify: `src/reasoning/shape.rs:206-245` (`node_of`)
- Modify: `src/navigation/queries.rs` (the `nVariable`/`nFunction`/`nStatement` construction sites + the `Location` sites)
- Test: `tests/reasoning/*` (witness byte), `tests/navigation/*` (nodes_at byte)

- [ ] **Step 1: Write the failing witness-byte test**

In the reasoning tests (mirror the existing `shape.rs` witness tests):

```rust
#[test]
fn witness_node_carries_occurrence_byte_and_reserved_ordinal() {
    // Build a CPG + trace, get a witness GraphNode for a known variable occurrence.
    // The SymbolRef::Variable and Location carry the node's real byte span; ordinal == 0.
    let (cpg, sink) = python_trace_fixture("def f(p):\n    q = p\n    return q\n", "test.py", 3, "q");
    let payload = prism::reasoning::shape::witness_graph_for_node(&cpg, &trace_of(&cpg, sink), sink).unwrap();
    let n = &payload.nodes[0];
    match n.symbol.as_ref().unwrap() {
        prism::navigation::types::SymbolRef::Variable { start_byte, end_byte, ordinal, .. } => {
            assert!(end_byte > start_byte || end_byte == start_byte, "valid byte bounds");
            assert_eq!(*ordinal, 0, "ordinal reserved");
        }
        _ => panic!("variable symbol"),
    }
    assert!(n.location.end_byte >= n.location.start_byte);
}
```

(Use the existing reasoning test helpers; if none expose a trace, add a thin `trace_of` to the test `common` that calls the same `taint_trace` the production `witness_graph_for_node` consumes.)

- [ ] **Step 2: Run — fails (no byte fields on the wire today)**

Run: `cargo test --test reasoning witness_node_carries_occurrence_byte_and_reserved_ordinal 2>&1 | tail -5`
Expected: compile error (`Location`/`SymbolRef::Variable` lack byte fields).

- [ ] **Step 3: Add byte fields to the wire types**

`src/navigation/types.rs` — `Location` (`:4-8`):

```rust
pub struct Location {
    pub file: String,
    pub start_line: usize,
    pub end_line: usize,
    pub start_byte: usize, // S2 additive
    pub end_byte: usize,   // S2 additive
}
```

`SymbolRef` (`:10-33`) — add `start_byte`/`end_byte` to all three variants (keep `ordinal`):

```rust
    Variable { file, function, line, path, access, ordinal, start_byte: usize, end_byte: usize },
    Function { file, name, start_line, end_line, ordinal, start_byte: usize, end_byte: usize },
    Statement { file, line, kind, ordinal, start_byte: usize, end_byte: usize },
```

(`GraphNode` is `{ symbol, location }` and inherits both — no separate change.)

- [ ] **Step 4: Populate byte in `node_of` (shape.rs:230-244); keep ordinal 0**

```rust
    GraphNode {
        symbol: Some(SymbolRef::Variable {
            file: file.clone(),
            function,
            line,
            path,
            access: access.into(),
            ordinal: 0, // RESERVED — occurrence discriminator deferred (S2 §9). Not byte rank.
            start_byte,
            end_byte,
        }),
        location: Location {
            file,
            start_line: line,
            end_line: line,
            start_byte,
            end_byte,
        },
    }
```

Source `start_byte`/`end_byte` from the same `cpg.to_var_location(n)` used for `line` (`:207-228`): add `l.start_byte` / `l.end_byte` to the destructured tuple, and `(0, 0)` in the `None` orphan arm.

- [ ] **Step 5: Populate byte at the navigation construction sites + compiler-guided**

In `src/navigation/queries.rs`, every `nVariable { … }` / `nFunction { … }` / `nStatement { … }` and `Location { … }` literal gains byte fields. Variables/statements: from the CPG node (`to_var_location` / `nodes_at` → `cpg.node(idx)` byte). Functions: from the function node's `start_byte`/`end_byte` (R-callers/callees built from a byte-less `FunctionId` → look up the function node via `function_node`/`to_function_id`+node byte; where no node exists, `(0,0)` with the existing fallback semantics). Run `cargo build`; fill each site the compiler flags. The callers/callees `ordinal` stays `0`.

- [ ] **Step 6: Run test + suite; enumerate witness fixture flips**

Run: `cargo test --test reasoning 2>&1 | tail -8` and `cargo test --test navigation 2>&1 | tail -8`
Expected: the new test passes; the existing `shape.rs`/navigation witness fixtures that serialize `SymbolRef`/`Location` now show byte fields — **re-bless** those snapshots and list them in the Task 10 expected-flip set. (Search: `rg -l "SymbolRef|GraphNode|Location" tests/ | sort -u`.)
Run: `cargo build && cargo test 2>&1 | tail -15`
Expected: green after re-blessing.

- [ ] **Step 7: Format + commit**

```bash
cargo fmt && git add -A
git commit -m "feat(s2): witness wire carries byte range; ordinal reserved (Task 6)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 7: `.function` scope-equality audit → `(function, function_start_line)`

Every place that compares the function *name* for scope/identity switches to the composite, so de-conflated same-name functions don't leak across each other. `.function` stays for display.

**Files:**
- Modify: `src/cpg/trace.rs:173` (`node_file_fn`, primary), `:335,:338` and `:374,:375` (`f2 == function` filters)
- Modify: `src/algorithms/taint.rs:4876,:4883` (`target_loc.function == func_name`)
- Audit: `rg -n "\.function ==|function: f2|f2 == function|== func_name" src/`
- Test: `tests/integration/core_test.rs` (cross-function no-leak)

- [ ] **Step 1: Write the failing cross-function no-leak test**

```rust
#[test]
fn same_path_does_not_taint_across_same_named_functions() {
    // `data` in A::run is tainted; B::run's `data` (same name, diff function) must NOT be.
    let src = "\
struct A; struct B;
impl A { fn run(&self, data: i32) { sink(data); } }
impl B { fn run(&self, data: i32) { safe(data); } }
";
    let cpg = build_rust_cpg(src);
    // Trace from A::run's `data` param; assert B::run's `data` use is not reached.
    let reached = reached_var_lines(&cpg, "test.rs", /*A::run data param line*/ 2, "data");
    assert!(!reached.contains(&3), "B::run line not reached from A::run's data");
}
```

(`reached_var_lines` = test helper running `taint_trace` from the seed and collecting reached Variable lines.)

- [ ] **Step 2: Run — may already pass via line scoping, or fail if names leak**

Run: `cargo test --test integration core_test::same_path_does_not_taint_across_same_named_functions 2>&1 | tail -4`
Expected: failure IF name-only scope leaks (the bug this task closes). If it passes already, keep the test as a regression guard and still do Steps 3-4 (the audit is the deliverable).

- [ ] **Step 3: Make `node_file_fn` carry the composite (trace.rs:173)**

```rust
    fn node_file_fn(&self, idx: NodeIndex) -> Option<(String, String, usize)> {
        match &self.graph[idx] {
            CpgNode::Variable { file, function, function_start_line, .. } =>
                Some((file.clone(), function.clone(), *function_start_line)),
            _ => None,
        }
    }
```

Update its callers (e.g. `:302` `node_file_fn(next).as_ref() == Some(&start_fn)`): `start_fn` must also be the 3-tuple (derive it from the seed node the same way).

- [ ] **Step 4: Switch the same-function filters to the composite**

`same_function_same_path_uses_any_line` (`:330-340`) and `same_line_same_path_uses` (`:367-377`): the `if f2 == function && p2 == path` guard also compares `function_start_line`:

```rust
                    CpgNode::Variable {
                        access: VarAccess::Use,
                        function: f2,
                        function_start_line: fsl2,
                        path: p2,
                        ..
                    } if f2 == function && fsl2 == fn_start && p2 == path
```

(bind `function_start_line` of the `def` node into `fn_start` in the outer `let CpgNode::Variable { … }` destructure).

`taint.rs:4876,:4883` — wherever `target_loc.function == func_name`, also require `target_loc.function_start_line == func_start_line` (thread the seed function's start line into that scope; it's available from the seed `VarLocation`).

- [ ] **Step 5: Run the audit grep + test + suite**

Run: `rg -n "\.function ==|f2 == function|== func_name" src/` — confirm every hit is either display-only or now composite-scoped. Document residine display-only uses in the commit body.
Run: `cargo test --test integration core_test::same_path_does_not_taint_across_same_named_functions 2>&1 | tail -4`
Expected: PASS
Run: `cargo build && cargo test 2>&1 | tail -15`
Expected: green.

- [ ] **Step 6: Format + commit**

```bash
cargo fmt && git add -A
git commit -m "feat(s2): .function scope-equality audit -> (function,function_start_line) (Task 7)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 8: `CallSite` byte — de-collapse same-line duplicate calls

`CallSite` gains a byte range; `cmp_key` includes it so two same-line calls to one callee stop collapsing. Requires widening the call-extraction helper to carry the call node's span. (This is the one scope addition beyond the variable wire — verify the ripple stays contained.)

**Files:**
- Modify: `src/call_graph.rs:24-40` (`CallSite` fields), `:1177-1185` (`cmp_key`), the 8 `CallSite { … }` ctors (`:164,351,427,454,534,624,644,867`)
- Modify: `src/ast.rs` (`function_calls_on_lines` and peers feeding those ctors → carry call node byte span)
- Test: `tests/integration/call_graph_test.rs`

- [ ] **Step 1: Write the failing de-collapse test**

```rust
#[test]
fn same_line_duplicate_calls_are_distinct_call_sites() {
    // Two calls to `foo` on one line → two CallSites (was one under line-only Ord).
    let src = "def caller():\n    foo(); foo()\n";
    let cg = prism::call_graph::CallGraph::build(&parse_one("test.py", src));
    let foo_sites: usize = cg.calls.values().flatten()
        .filter(|s| s.callee_name == "foo" && s.line == 2)
        .count();
    assert_eq!(foo_sites, 2, "same-line duplicate calls preserved");
}
```

(`parse_one` = test helper building the single-file `BTreeMap`.)

- [ ] **Step 2: Run — fails (one site today; Ord/cmp_key is line-only)**

Run: `cargo test --test integration call_graph_test::same_line_duplicate_calls_are_distinct_call_sites 2>&1 | tail -4`
Expected: assert failure (1 != 2).

- [ ] **Step 3: Add byte fields + extend `cmp_key`**

`src/call_graph.rs:24-40`:

```rust
pub struct CallSite {
    pub caller: FunctionId,
    pub callee_name: String,
    pub line: usize,
    pub qualifier: Option<String>,
    #[serde(default)]
    pub receiver_type: Option<String>,
    #[serde(default)]
    pub receiver_recovery: Option<crate::resolution::ReceiverRecovery>,
    /// S2: byte extent of the call expression — de-collapses same-line duplicates.
    #[serde(default)]
    pub start_byte: usize,
    #[serde(default)]
    pub end_byte: usize,
}
```

`cmp_key` (`:1177-1185`) — append the bytes after `line`:

```rust
    fn cmp_key(&self) -> (&str, &str, usize, usize, usize, Option<&str>, Option<&str>) {
        (
            &self.caller.name,
            &self.callee_name,
            self.line,
            self.start_byte,
            self.end_byte,
            self.qualifier.as_deref(),
            self.receiver_type.as_deref(),
        )
    }
```

- [ ] **Step 4: Widen call extraction + fill the 8 ctors**

`function_calls_on_lines` (and any peer the 8 ctors use) currently yields `(callee_name, line)`; widen it to `(callee_name, line, start_byte, end_byte)` from the call `Node` it walks. At each `CallSite { … }` ctor (`:164, 351, 427, 454, 534, 624, 644, 867`), set `start_byte`/`end_byte` from the widened source. Where a ctor has no node handy (synthesized/indirect), set `(0, 0)` (documented — those are already approximate). `cargo build` lists all 8.

- [ ] **Step 5: Run test + suite**

Run: `cargo test --test integration call_graph_test::same_line_duplicate_calls_are_distinct_call_sites 2>&1 | tail -4`
Expected: PASS
Run: `cargo build && cargo test 2>&1 | tail -15`
Expected: green. Call-count telemetry / reverse-caller edges move by design for same-line dups — capture for Task 10.

- [ ] **Step 6: Format + commit**

```bash
cargo fmt && git add -A
git commit -m "feat(s2): CallSite byte range de-collapses same-line calls (Task 8)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 9: Cache v5 round-trip + PartialHit incremental

`CACHE_VERSION` already bumped (Task 1). This task proves the full + incremental cache paths round-trip the new schema and that v4 invalidates.

**Files:**
- Test: `tests/ast/cpg_cache_test.rs`
- Modify (only if a test fails): `src/cpg_cache.rs` serialization gaps

- [ ] **Step 1: Write the v5 round-trip + v4-invalidation tests**

```rust
#[test]
fn cpg_cache_v5_full_round_trip_preserves_byte_identity() {
    let src = "def f(p):\n    q = p\n    return q\n";
    let files = parse_one("test.py", src);
    let cpg = prism::cpg::CodePropertyGraph::build(&files);
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("cpg.json");
    write_cache(&cpg, &files, &path);                 // existing cache write entry point
    let restored = match load_cache(&files, &path) {  // existing loader
        prism::cpg_cache::CacheResult::Hit(c) => c,
        other => panic!("expected Hit, got {other:?}"),
    };
    // Byte ranges + function_start_line survive the round trip.
    assert_eq!(node_byte_dump(&cpg), node_byte_dump(&restored));
}

#[test]
fn cpg_cache_v4_invalidates() {
    // A v4 header must not be accepted as a v5 hit.
    let stale = sample_cache_json_with_version(4);
    assert!(matches!(parse_cache_header(&stale), Err(_) ) || version_rejected(&stale));
}

#[test]
fn cpg_cache_partial_hit_incremental_rebuild_round_trips() {
    // Two files; change one; PartialHit → build_incremental reassembles a v5 DFG.
    let two = parse_two("a.py", "def a(p):\n    return p\n", "b.py", "def b(q):\n    return q\n");
    // … write full cache, mutate b.py, load → PartialHit, assemble, assert byte identity on a.py nodes …
}
```

(Use the existing helpers in `cpg_cache_test.rs`; match their actual `write_cache`/`load_cache` names — `rg -n "fn " tests/ast/cpg_cache_test.rs | head`.)

- [ ] **Step 2: Run — confirm pass (Task 1 already bumped version + fixed reconstruct)**

Run: `cargo test --test ast cpg_cache_test:: 2>&1 | tail -10`
Expected: PASS. If `node_byte_dump` mismatches, the gap is an unported field in `reconstruct_cpg`/`from_parts`/`build_incremental` — fix it in `src/cpg_cache.rs` (the `location_index` byte-sort mirror from Task 5 Step 4 is the likely miss).

- [ ] **Step 3: Run full suite + commit**

Run: `cargo build && cargo test 2>&1 | tail -15`
Expected: green.

```bash
cargo fmt && git add -A
git commit -m "test(s2): cache v5 round-trip + PartialHit incremental (Task 9)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 10: Acceptance — expected-flip fixtures, order-invariance, determinism, Tier-A

The spec §7 gates. Captures the de-conflation/de-collapse flips accumulated in Tasks 2/3/8 as **asserted** flips (the S3-style invariant: "these specific edges flip; nothing else regresses"), proves order-sensitive output is stable, and runs the repo accuracy workflow.

**Files:**
- Test: `tests/integration/core_test.rs` (edge-set flip fixture), an order-sensitive `provenance_slice` fixture, determinism test
- Test: `tests/integration/coverage_test.rs` — only if new test files were added (3-copy `all_test_files`, per CLAUDE.md)

- [ ] **Step 1: Edge-set expected-flip fixture**

```rust
#[test]
fn s2_deconfliction_edge_flips_are_exactly_enumerated() {
    // A fixture with same-name functions whose de-conflation flips reachability.
    let src = include_str!("../fixtures/s2_overload_flip.rs"); // create this fixture
    let cpg = build_rust_cpg(src);
    let dump = normalized_edge_dump(&cpg); // (from_kind:line:path -> to_kind:line:path) sorted
    // The exact post-S2 edge set (paste the verified dump here after first green run).
    let expected: Vec<&str> = vec![ /* enumerated edges */ ];
    assert_eq!(dump, expected, "no unexpected edge changes");
}
```

Generate the fixture + dump, eyeball it for correctness (the de-conflation should ADD precision, not lose edges), then paste the verified `expected`.

- [ ] **Step 2: Order-sensitive provenance fixture**

```rust
#[test]
fn provenance_output_stable_under_var_location_ord_change() {
    let src = include_str!("../fixtures/s2_provenance.py");
    let out = run_provenance_slice(src); // existing algorithm entry
    let expected = include_str!("../fixtures/s2_provenance.expected");
    assert_eq!(out, expected, "byte-additive Ord does not change provenance output");
}
```

- [ ] **Step 3: Determinism test**

```rust
#[test]
fn cpg_build_is_deterministic_across_runs() {
    let src = "def f(p):\n    q = p\n    r = q\n    return r\n";
    let a = node_byte_dump(&build_python_cpg(src));
    let b = node_byte_dump(&build_python_cpg(src));
    assert_eq!(a, b);
}
```

- [ ] **Step 4: Run the targeted tests**

Run: `cargo test --test integration core_test::s2_ 2>&1 | tail -10`
Run: `cargo test --test integration provenance 2>&1 | tail -6`
Expected: PASS.

- [ ] **Step 5: Full suite + MCP feature build**

Run: `cargo fmt --check && cargo test 2>&1 | tail -15`
Run: `cargo test --features mcp 2>&1 | tail -8`
Expected: all green.

- [ ] **Step 6: Tier-A accuracy workflow (CLAUDE.md)**

Run: `cargo build --release`
Run: `cd eval && uv run tier-a --matrix-only --allow-stale-sut` → expect exit 0
Run: `cd eval && uv run tier-a --quick --allow-stale-sut` (needs rust-analyzer) → review for regressions/flip-candidates; paste any into the PR description (do NOT re-baseline).

- [ ] **Step 7: Coverage matrix (only if test files were added)**

If Tasks added NEW test files (not just functions to existing files), update the 3 copies of `all_test_files` in `tests/integration/coverage_test.rs`. Most S2 tests extend existing files, so this is likely a no-op — verify:
Run: `cargo test --test integration coverage_test:: 2>&1 | tail -5`
Expected: PASS.

- [ ] **Step 8: Final commit**

```bash
cargo fmt && git add -A
git commit -m "test(s2): acceptance — edge-flip fixtures, order-invariance, determinism (Task 10)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Final review (after all tasks)

Dispatch a final whole-implementation code review (subagent-driven-development's final reviewer), then `superpowers:finishing-a-development-branch`. Per the owner's workflow, a codex xhigh full-branch review precedes the owner's merge. Carry forward to the PR description: the enumerated de-conflation/de-collapse edge flips, any Tier-A flip-candidates, and the §9 deferrals (occurrence-splitting + the reserved ordinal; `FunctionId.start_byte`; PDG-lite; column/UTF-16) — and the note that the Plan B re-plan must re-validate its same-line fixtures against byte order (the admission-semantics change).

## Self-review notes (spec coverage)

- §1 byte-additive identity → Tasks 1, 4 (fields + real spans); never a key (var_index/func_index keys hold no byte — Tasks 2, 3).
- §1 `(file,name,start_line)` func key → Task 2. §1 `function_start_line` on Variable/VarLocation → Tasks 1, 3.
- §1 witness wire (byte range now, ordinal reserved) → Task 6. §1 CallSite byte → Task 8.
- §3 span anchor table + tie-break → Task 4 (extraction) + Task 5 (`node_sort_key` total order).
- §4 `VarOccurrence`/named records, `Ord≡Eq≡Hash` → Tasks 4, 1 (invariant test §7.6).
- §5 consumers: build Steps 1/5/5b/6/9 (Tasks 1-3); trace :211/:242/:342/:379 (Task 5) + node_file_fn (Task 7); query function_candidates/var_node/to_var_location (Tasks 1-3, 6); navigation+shape (Task 6); cache (Tasks 1, 9); `.function` audit (Task 7).
- §6 failure modes → covered by tests in Tasks 4 (multi-target/destructuring/augmented via anchor table), 1 (orphan/parse-degraded bounds), 8 (CallSite). Macro-generated = documented limitation (no fabricated precision); no code path needed.
- §7 acceptance → Task 10 (+ the per-task TDD tests). §8 risks → mitigated by the named tests. §9 deferrals → untouched, seams left as specified.
