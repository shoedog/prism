# Python Decorated-Method Double-Capture — Wrapper-Canonical — Implementation Plan (rev 2)

> Execute task-by-task via strict TDD. Design-of-record:
> `docs/superpowers/specs/2026-06-23-python-decorated-method-double-capture.md` (rev 3). Branch
> `decorated-double-capture` (stacked on merged #131).
>
> **Rev 2 — codex plan-review fold (REWORK):** (BLOCKER) the canonical filter must live in
> `all_functions_via_tree()`, NOT `build_function_table` — `all_functions_inner()` bypasses the table on
> reconstruction miss (`ast.rs:286-288`). (BLOCKER) **CFG** (`cfg.rs`) walks raw `function_node_types()`,
> not `all_functions()`, so after the helper unwraps it double-emits ControlFlow edges for decorated
> wrapper+inner → new **Task 2b**. (MAJOR) the helper audit must extend beyond `ast.rs` to
> `contract_slice` (its own `body` reader + nested guard). (MAJOR) Task-2 test made concrete. (MINOR)
> inventory dedup NARROWED, not no-op. (NIT) cache at `cpg_cache.rs:67`, assertion `:571-574`.

**Goal:** one canonical `FunctionId` per decorated Python definition (keep the `decorated_definition`
wrapper, drop the inner `function_definition`), with NO duplicate CFG/CPG edges and NO lost
decorated-function structure.

**TDD ordering:** T1 (unwrap field-readers — pure addition) → T2 (canonical filter in
`all_functions_via_tree`) → T2b (CFG de-dup, required because T1 makes the wrapper body-readable on a raw
traversal) → T3 inventory → T4 cache → T5 fixtures → T6 acceptance.

---

## Task 1: `unwrap_decorated` + apply to ALL function-node field-readers (incl. contract_slice)

**Files:** `src/ast.rs` (helper + 5 sites); `src/algorithms/contract_slice.rs` (2 sites); tests in each.

- [ ] **Step 1 — failing test** (`src/ast.rs`): on a Python `decorated_definition`, `find_parameters_node`,
  `function_body_node`, `statements_in_function`, `statement_spans_in_function`, and `return_value_nodes`
  all return the inner's content (today None/empty). Concrete:

```rust
#[test]
fn decorated_function_field_readers_unwrap() {
    let p = ParsedFile::parse("a.py","@deco\ndef f(x, y):\n    z = x\n    return z\n", Language::Python).unwrap();
    let deco = descendant_of_kind(&p, "decorated_definition").expect("deco node");
    assert!(p.find_parameters_node(&deco).is_some());
    assert!(p.function_body_node(&deco).is_some());
    assert!(!p.statements_in_function(&deco).is_empty());
    assert!(!p.statement_spans_in_function(&deco).is_empty());
    assert!(!p.return_value_nodes(&deco).is_empty()); // unwrap BEFORE the nested-fn guard (:2888)
}
```

- [ ] **Step 2 — run-fail.**
- [ ] **Step 3 — implement** the helper (Python-gated; byte-range scanners unchanged):

```rust
fn unwrap_decorated<'a>(&self, node: Node<'a>) -> Node<'a> {
    if matches!(self.language, Language::Python) && node.kind() == "decorated_definition" {
        let mut c = node.walk();
        if let Some(inner) = node.children(&mut c).find(|n| n.kind() == "function_definition") {
            return inner;
        }
    }
    node
}
```

Prepend `let func_node = self.unwrap_decorated(*func_node);` (rebind the param) at the head of `find_parameters_node` (`:3922`),
`function_body_node` (`:2607`), `statements_in_function` (`:3097`), `statement_spans_in_function` (`:3112`),
`return_value_nodes` (`:2828`, **before** the nested-fn guard `:2888-2893`).

- [ ] **Step 3b — contract_slice** (`src/algorithms/contract_slice.rs`): its DELTA path consumes
  `all_functions()` (wrapper nodes after T2) and reads `child_by_field_name("body")` directly (`:380-383`)
  with a nested-fn guard (`:1077-1098`). Add a local unwrap (or a public `ParsedFile` wrapper-aware body
  accessor) at both. Add a test: a decorated function's contract pre/postconditions are still detected.
- [ ] **Step 3c — audit:** `rg` across `src/` for `child_by_field_name("body"|"parameters"|"return")` and
  `function_node_types()`/`is_call_node`-style raw function traversals taking a function node; any Python
  field-reader not already wrapper-aware (`function_name`/`method_owner` ARE) needs the unwrap. List
  findings in the commit.
- [ ] **Step 4 — run-pass. Step 5 — commit** `feat: unwrap_decorated for decorated-fn field readers (ast + contract_slice)`.

---

## Task 2: canonical wrapper skip in `all_functions_via_tree` (the real chokepoint)

**Files:** `src/ast.rs` (`all_functions_via_tree` `:318-343`); test in `src/ast.rs`.

- [ ] **Step 1 — failing test** (concrete — no placeholders):

```rust
#[test]
fn decorated_function_canonical_single_node() {
    let p = ParsedFile::parse("a.py", "@deco\ndef f():\n    return 1\n", Language::Python).unwrap();
    let fs: Vec<_> = p.all_functions().into_iter()
        .filter(|n| p.language.function_name(n).map(|nm| p.node_text(&nm)) == Some("f".into()))
        .collect();
    assert_eq!(fs.len(), 1, "one canonical record for f");
    assert_eq!(fs[0].kind(), "decorated_definition", "the wrapper is canonical");
}
```

- [ ] **Step 2 — run-fail** (count 2).
- [ ] **Step 3 — implement:** in `all_functions_via_tree()` AFTER both query + manual collection, drop any
  `function_definition` whose parent is `decorated_definition` (Python only). This is the single point both
  `build_function_table` AND the `all_functions_inner` reconstruction-miss fallback (`:286-288`) return
  through. Structural predicate only (overloads/setters/redefinitions stay distinct); C++
  `template_declaration` untouched (different kind).
- [ ] **Step 3b — reconstruction-fallback test (FORCE the miss):** use the synthetic tree-corruption
  pattern (`src/ast.rs:5336-5343`) to force `all_functions_inner` down the reconstruction-miss fallback on a
  decorated fixture; assert `used_fallback == true` AND only the wrapper `decorated_definition` for `f`
  remains. (A bare `all_functions()` assertion is NOT discriminating — it passes through the eager table.)
- [ ] **Step 4 — run-pass + `cargo build`. Step 5 — commit** `feat(ast): wrapper-canonical skip in all_functions_via_tree`.

---

## Task 2b: CFG de-dup for decorated functions (BLOCKER 2)

**Files:** `src/cfg.rs` (`:29-55` raw `function_node_types()` walk); `src/cpg/build.rs` (`:999-1004`
`collect_step8_edges`); test under `tests/` (CFG/CPG control-flow edges).

- [ ] **Step 1 — failing test:** for a decorated Python function, the CPG has NO duplicate `ControlFlow`
  edges (today, after T1, the raw CFG walk emits edges for BOTH wrapper and inner). Assert raw ControlFlow
  **edge count == unique edge count** (catches duplicate PARALLEL edges a set comparison would hide) AND
  decorated/undecorated count parity.
- [ ] **Step 2 — run-fail** (duplicate edges).
- [ ] **Step 3 — implement:** make the CFG function walk **not double-process** decorated wrapper+inner —
  iterate `parsed.all_functions()` (canonical, wrapper-only) instead of raw `function_node_types()`, OR
  skip a `function_definition` whose parent is `decorated_definition` in the CFG traversal. Prefer routing
  through `all_functions()` for consistency; verify it still enumerates nested functions CFG needs.
- [ ] **Step 4 — run-pass. Step 5 — commit** `fix(cfg): do not double-build CFG for decorated wrapper+inner`.

---

## Task 3: inventory contract — narrow the containment dedup

**Files:** `src/navigation/inventory.rs:34-56`; `tests/navigation/inventory_test.rs`.

- [ ] **Step 1 — failing tests:** (a) a decorated fn appears once, anchored at the wrapper; (b) a decorated
  fn CONTAINING a nested `def` keeps the wrapper AND the nested function (today the containment rule
  `:44-49` drops any `decorated_definition` containing another function).
- [ ] **Step 2 — run-fail.**
- [ ] **Step 3 — implement:** after T2 the inner record is gone, so the old "drop wrapper if it contains an
  inner" rule must be **narrowed/removed** (not left as a no-op): keep the wrapper; never drop it due to a
  nested real `def`.
- [ ] **Step 4 — run-pass. Step 5 — commit** `fix(nav): inventory keeps decorated wrapper, preserves nested defs`.

---

## Task 4: `CACHE_VERSION` 22→23

**Files:** `src/cpg_cache.rs` (const `:67`; assertion test `:571-574`).

- [ ] Change the assertion to 23 (fail) → bump the const at `:67` (`// 23: wrapper-canonical decorated
  extraction`) → pass. Commit `chore(cache): CACHE_VERSION 22->23`.

---

## Task 5: discriminating fixtures + guards

**Files:** `tests/lang/python/`, `tests/lang/javascript/`, `tests/lang/cpp/` (+ `main.rs` mods + all 3
`coverage_test.rs` arrays for new files).

- [ ] Free-fn LocalDef **singleton** (decorated module-level `f()` → one Exact, was two).
- [ ] Method **NameOnly→Exact** (decorated method via `self.m()`/`Cls.m()`).
- [ ] **CFG/CPG** control-flow edge **count parity** decorated vs undecorated (guards T2b).
- [ ] **contract_slice** structure intact for a decorated function (guards T1 step 3b).
- [ ] **`enclosing_function()` pin** — returns the inner inside a decorated fn (spec §6 non-goal).
- [ ] **C++ template no-change** canary; **JS/TS** decorated method resolves once (guard).
- [ ] Commit `test(decorated): singleton + buy + CFG-parity + contract + enclosing/C++/JS guards`.

---

## Task 6: Acceptance (host-run; orchestrator)

- [ ] `cargo build --release`; pydantic main-vs-branch worktree call-stats: decorated `kind_exact`
  (self_receiver/qualifier_owner) up, `kind_nameonly` down, decorated-free `multi_target_exact_sites`
  down/flat (never up). Report deltas.
- [ ] **CPG control-flow edge count** for a decorated-fn fixture: no duplicates.
- [ ] fastapi sanity; **Rust (ripgrep) + Go (caddy) byte-identical**; C++ no-change.
- [ ] Tier-A `--matrix-only` 0-regr; `--quick` best-effort. `cargo test` + `--features mcp` + `fmt` green.
- [ ] Paste deltas into the PR.

---

## Self-review
- Spec coverage: T1 §4.2 (now incl. contract_slice + cross-crate audit); T2 §4.1 (chokepoint corrected to
  `all_functions_via_tree`); **T2b CFG de-dup (new, the raw-traversal interaction)**; T3 §5 inventory
  (narrowed); T4 cache (`:67`/`:571-574`); T5 §8 (+CFG-parity, +contract); T6 §8.
- Ordering: T1 pure-addition → T2 removes inner → T2b prevents the raw-traversal double-emit T1 enables.
- No placeholders: helper + skip code + concrete test assertions (count/kind/name); descendant helper named.
