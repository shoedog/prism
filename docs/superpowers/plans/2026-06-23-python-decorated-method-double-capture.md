# Python Decorated-Method Double-Capture — Wrapper-Canonical — Implementation Plan

> **For agentic workers:** execute task-by-task via strict TDD (failing test → run-fail → minimal code →
> run-pass → commit). Design-of-record: `docs/superpowers/specs/2026-06-23-python-decorated-method-double-capture.md`
> (rev 3, both spec-reviews folded — read §3/§4/§6). Branch `decorated-double-capture` (stacked on merged #131).

**Goal:** one canonical `FunctionId` per decorated Python definition (keep the `decorated_definition`
wrapper, drop the inner `function_definition`) — removing duplicate ids / CPG nodes / double body-scans
that demote decorated-method calls to NameOnly and mint duplicate Exact free-fn edges.

**Architecture:** (1) a Python-only `unwrap_decorated(node)` used by every function-node helper that reads a
child field, so the canonical wrapper still yields params/body/statements/returns; (2) a centralized
wrapper-canonical skip in `build_function_table` covering both extraction paths; (3) inventory contract
flip (keep wrapper); (4) `CACHE_VERSION` 22→23. C++ templates, JS/TS, `enclosing_function()`, and decorator
semantics are explicitly out of scope (spec §6).

**TDD ordering rationale:** Task 1 (unwrap in field-readers) is a *pure addition* (both wrapper and inner
work) and MUST precede Task 2 (the skip removes the inner, after which field-readers only ever see the
wrapper).

---

## Task 1: `unwrap_decorated` + apply to field-reading helpers

**Files:** `src/ast.rs` (the helper + 5 call sites); test in `src/ast.rs` `#[cfg(test)]`.

- [ ] **Step 1 — failing test:** `find_parameters_node` on a Python `decorated_definition` returns the
  inner's params (today returns None).

```rust
#[test]
fn decorated_function_helpers_unwrap_to_inner() {
    let p = ParsedFile::parse("a.py", "class C:\n    @staticmethod\n    def f(x, y):\n        return x\n", Language::Python).unwrap();
    // locate the decorated_definition node for f (walk the tree for kind == "decorated_definition")
    let deco = find_node(&p, "decorated_definition");
    assert!(p.find_parameters_node(&deco).is_some(), "params via wrapper");
    assert!(p.function_body_node(&deco).is_some(), "body via wrapper");
    assert!(!p.statements_in_function(&deco).is_empty(), "statements via wrapper");
}
```

- [ ] **Step 2 — run-fail:** `cargo test --lib decorated_function_helpers_unwrap_to_inner` → FAIL (None/empty on wrapper).
- [ ] **Step 3 — implement:** add the helper (Python-gated; byte-range scanners need NO change):

```rust
/// If `node` is a Python `decorated_definition`, return its inner `function_definition`
/// (wrapper-canonical extraction keeps the wrapper; field readers need the inner). Else
/// `node` unchanged. Python-only — C++ template wrappers are a separate slice.
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

Call `let node = self.unwrap_decorated(*node);` (or rebind the param) at the HEAD of: `find_parameters_node`
(`:3922`), `function_body_node` (`:2607`), `statements_in_function` (`:3097`),
`statement_spans_in_function` (`:3112`), and `return_value_nodes` (`:2828`) — for `return_value_nodes` the
unwrap must precede the nested-function guard (`:2888-2893`) so it doesn't treat the inner as a nested fn
and drop its returns. **Audit:** grep ParsedFile/Language for other fns taking a function node and reading
a child field (signature/receiver/name-occurrence); apply the unwrap to any that do (byte-range scanners
are fine).

- [ ] **Step 4 — run-pass.** **Step 5 — commit** `feat(ast): unwrap_decorated for decorated-fn field readers`.

---

## Task 2: centralized wrapper-canonical skip

**Files:** `src/ast.rs` (`build_function_table` `:347-370`, the chokepoint both query `:318-337` and manual
`:466-474`/`:286-288` paths reach); test in `src/ast.rs`.

- [ ] **Step 1 — failing test:** a decorated Python function yields exactly ONE `FunctionId`-equivalent
  record (today two — wrapper + inner).

```rust
#[test]
fn decorated_function_canonical_single_record() {
    let p = ParsedFile::parse("a.py", "@deco\ndef f():\n    return 1\n", Language::Python).unwrap();
    let fns: Vec<_> = p.all_functions().into_iter().filter(|n| /* name == "f" */ true).collect();
    // assert exactly one record for f, and it is the decorated_definition (wrapper)
    assert_eq!(count_named(&p, "f"), 1);
}
```

- [ ] **Step 2 — run-fail:** FAIL (count 2).
- [ ] **Step 3 — implement:** in `build_function_table`, skip a captured `function_definition` whose parent
  is a `decorated_definition` (Python only) — keep the wrapper. Centralize here so the manual fallback
  path cannot reintroduce the duplicate. Predicate: `node.kind() == "function_definition" &&
  node.parent().map_or(false, |p| p.kind() == "decorated_definition")` gated to Python. Do NOT collapse by
  name (structural parent check only) — `@overload`/setters/redefinitions stay distinct (spec §7).
- [ ] **Step 4 — run-pass** + `cargo build` (no consumer breaks — Task 1 made field-readers wrapper-safe).
- [ ] **Step 5 — commit** `feat(ast): wrapper-canonical extraction — drop inner of decorated_definition`.

---

## Task 3: inventory contract flip

**Files:** `src/navigation/inventory.rs:34-56`; `tests/navigation/inventory_test.rs`.

- [ ] **Step 1 — failing/updated test:** a decorated Python function appears exactly once in the inventory,
  anchored at the wrapper; AND a decorated function CONTAINING a nested `def` keeps the wrapper and does
  NOT drop the nested function (guards the containment rule `:44-49`).
- [ ] **Step 2 — run-fail** (current dedup keeps the inner / may mishandle nested).
- [ ] **Step 3 — implement:** the local dedup currently marks the wrapper `false` when it contains an inner
  record (keeping the inner). Since Task 2 removed the inner record, update this: keep the wrapper; ensure
  the nested-`def` case is not collateral-dropped. (May become a no-op or invert — verify against the
  no-longer-present inner.)
- [ ] **Step 4 — run-pass.** **Step 5 — commit** `fix(nav): inventory keeps decorated wrapper (canonical)`.

---

## Task 4: `CACHE_VERSION` bump

**Files:** `src/cpg_cache.rs` (`CACHE_VERSION` `:65`, assertion test).

- [ ] Bump 22→23 (`// 23: wrapper-canonical decorated extraction`); update the version assertion test.
  TDD: change the assertion first (fail), bump const (pass). Commit `chore(cache): CACHE_VERSION 22->23`.

---

## Task 5: discriminating fixtures + guards

**Files:** `tests/lang/python/`, `tests/lang/javascript/`, a C++ case under `tests/lang/cpp/` (mod lines +
the 3 `coverage_test.rs` arrays for new files).

- [ ] **Free-fn singleton:** a decorated module-level `def f()` called as `f()` resolves to exactly ONE
  Exact `LocalDef` (was two). Real-source `CallGraph::build` + `resolve_call_site_full`.
- [ ] **Method NameOnly→Exact (the buy):** a class with a decorated method called via `self.m()` / `Cls.m()`
  resolves Exact (was NameOnly from the wrapper+inner pair).
- [ ] **`enclosing_function()` behavior pin (spec §6 non-goal):** a line inside a decorated fn →
  `enclosing_function()` returns the inner `function_definition` — assert (documents the scoping decision).
- [ ] **C++ no-change canary:** a C++ `template` function — function count + call resolution unchanged
  (the skip is Python-gated).
- [ ] **JS/TS guard:** a decorated TS method (if grammar supports) resolves once — confirms no regression.
- [ ] **Step 5 — commit** `test(decorated): free-fn singleton + method buy + enclosing/ C++/JS guards`.

---

## Task 6: Acceptance (host-run; orchestrator)

- [ ] `cargo build --release`; main-vs-branch worktree call-stats on **pydantic** (the buy): decorated
  call buckets — `kind_exact.self_receiver`/`qualifier_owner` rise, `kind_nameonly` fall;
  `multi_target_exact_sites` byte-flat (or down for decorated free fns, never up); report deltas.
- [ ] **fastapi** sanity; **Rust (ripgrep) + Go (caddy) byte-identical**; **C++ (leveldb if it completes,
  else a fixture) no-change**.
- [ ] Tier-A `--matrix-only` 0-regr; `--quick` best-effort.
- [ ] `cargo test` + `cargo test --features mcp` + `cargo fmt --check` green.
- [ ] Paste deltas into the PR.

---

## Self-review
- Spec coverage: T1 §4.2 helper audit; T2 §4.1 centralized skip; T3 §5 inventory flip; T4 cache; T5 §8
  fixtures incl. enclosing-pin + C++ canary; T6 §8 acceptance. enclosing_function/C++/JS/decorator-semantics
  out (§6).
- Ordering: T1 (pure addition) before T2 (removes inner) — field-readers never see a param-less wrapper.
- No placeholders: helper code + skip predicate concrete; test bodies use the real `CallGraph::build` /
  `all_functions` / `resolve_call_site_full` idioms (adapt import paths to the crate).
