# Python/JS Self-Receiver Same-Class Owner Narrowing — Implementation Plan (rev 2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended)
> or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.
>
> **Rev 2 — codex xhigh plan-review fold (SHIP-WITH-FIXES; verdicts 1–4 TRUE = design/population/types
> sound):** BLOCKER `ParsedFile::parse` is 3-arg `(path, source, Language)` — all test code now uses the
> `files()` helper. MAJOR-1 Task 3 placeholders → concrete assertions on target set + `ResolutionKind` +
> `ResolutionConfidence` + `ResolutionOutcome.drop`. MAJOR-2 (real-source vs hand-built): **considered
> divergence** — the codebase's idiom IS real-source multi-file via `files()` + `CallGraph::build` +
> `resolve_call_site_full` (`tests/name_resolution/local_binding_test.rs:17`,
> `build_wiring_test.rs:530`); it is concrete and representative, and population-from-parse is covered by
> Task 2, so multi-class cases use real source rather than hand-built `CallGraph` literals. MINOR
> `tests/lang/python/` + `lang_python` already exist (Task 5 just adds a `mod` line + 3 coverage arrays).
> MINOR Task 1 adds JS + Go assertions. NIT `CACHE_VERSION` is a private `const`.

**Goal:** For `self`/`this`/`cls` method calls in Python/JS/TS, resolve to the caller's *own* class by
narrowing the owner-index lookup to candidates sharing the caller's class identity `(file, owner-class byte
span)` — upgrading cross-file-collision NameOnly to Exact and closing a full-confidence cross-class
false-positive.

**Architecture:** New `CallGraph.method_class_span: BTreeMap<FunctionId, (usize, usize)>` records each
method's owner-class definition node byte span, populated from a new `Language::method_owner_class_node`
helper. A new resolver helper `self_owner_lookup_same_class` filters `methods[(owner,name)]` by that
identity; the self arm (`resolution.rs:918`) calls it gated to Python/JS/TS/Tsx (Go/Rust keep
`owner_lookup`). `CACHE_VERSION` 20→21.

**Design-of-record:** `docs/superpowers/specs/2026-06-22-python-js-self-receiver-samefile-narrowing.md`
(rev 3). Read §3 before starting.

---

## File Structure

- `src/languages/mod.rs` — add `method_owner_class_node` (Python/JS/TS/Tsx).
- `src/call_graph.rs` — extend `method_metadata` → 4-tuple (add owner-class span); add `method_class_span`
  field (`:164`); thread it through every `CallGraph` literal (`empty` `:227`, skeleton `:368`, full `:970`,
  subset `:1581`) + populate at every `method_owners` write (`:292`, `:530`, `:1479`) + `merge` extend
  (`:1079`) + `remove_files` retain (`:1039-1040`).
- `src/resolution.rs` — add `self_owner_lookup_same_class` (near `owner_lookup` `:691`); gate the self arm
  (`:918-936`).
- `src/cpg_cache.rs` — `CACHE_VERSION` (`:65`) 20→21; update assertion test (`:572`).
- `tests/lang/python/self_receiver_test.rs` (new file; add `mod self_receiver_test;` to
  `tests/lang/python/main.rs`; add the path to the three `all_test_files` arrays in
  `tests/integration/coverage_test.rs` near `:104`, `:323`, `:470`). `lang_python` `[[test]]` already
  exists — do **not** re-create it.
- `tests/lang/javascript/` — add a self-receiver test file + `mod` line (+ coverage arrays).

### Shared test helper (used by Task 3 and Task 5)

```rust
use std::collections::BTreeMap;
use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
use prism::languages::Language;
use prism::resolution::{DropReason, ResolutionConfidence, ResolutionKind, ResolutionOutcome};

fn files(pairs: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
    pairs
        .iter()
        .map(|(p, s)| {
            let lang = Language::from_path(p).expect("known extension");
            ((*p).to_string(), ParsedFile::parse(p, s, lang).expect("parse"))
        })
        .collect()
}

/// Resolve the call to `callee` made inside method `caller_name` defined in `caller_file`.
fn resolve_self_call<'a>(
    cg: &'a CallGraph,
    caller_file: &str,
    caller_name: &str,
    callee: &str,
) -> ResolutionOutcome<'a> {
    let caller = cg
        .functions
        .get(caller_name)
        .and_then(|v| v.iter().find(|f| f.file == caller_file))
        .expect("caller fn");
    let site = cg
        .calls
        .get(caller)
        .and_then(|sites| sites.iter().find(|s| s.callee_name == callee))
        .expect("call site");
    cg.resolve_call_site_full(site)
}
```

(Adjust imports to the crate's real module paths if they differ — verify against an existing test in
`tests/integration/call_graph_test.rs`.)

---

## Task 1: `Language::method_owner_class_node`

**Files:** Modify `src/languages/mod.rs` (next to `method_owner`, `:1056`); test in its `#[cfg(test)]`.

- [ ] **Step 1: Write the failing test** (Python + JS class node found; Rust + Go → None)

```rust
#[test]
fn method_owner_class_node_finds_class_for_py_js_not_rust_go() {
    // Python: class_definition node, starts at byte 0
    let p = ParsedFile::parse("a.py", "class C:\n    def f(self):\n        return 1\n", Language::Python).unwrap();
    let f = first_function(&p, "f");
    let c = p.language.method_owner_class_node(&f).expect("py class");
    assert_eq!(c.kind(), "class_definition");
    assert_eq!(c.start_byte(), 0);

    // JS: class_declaration node
    let j = ParsedFile::parse("a.js", "class C {\n  f() { return 1; }\n}\n", Language::JavaScript).unwrap();
    let jf = first_function(&j, "f");
    let jc = j.language.method_owner_class_node(&jf).expect("js class");
    assert!(matches!(jc.kind(), "class_declaration" | "class"));

    // Rust + Go: methods span files / no single class node -> None
    let r = ParsedFile::parse("a.rs", "struct S;\nimpl S { fn f(&self) {} }\n", Language::Rust).unwrap();
    assert!(r.language.method_owner_class_node(&first_function(&r, "f")).is_none());
    let g = ParsedFile::parse("a.go", "package p\ntype T struct{}\nfunc (t T) F() {}\n", Language::Go).unwrap();
    assert!(g.language.method_owner_class_node(&first_function(&g, "F")).is_none());
}
```

(Add a local `first_function(&ParsedFile, &str) -> Node` that scans `parsed.all_functions()` for the named
function, or reuse an existing helper.)

- [ ] **Step 2: Run to verify it fails** — `cargo test --lib method_owner_class_node_finds_class` → FAIL
  (method missing).

- [ ] **Step 3: Implement** (mirror `method_owner`'s walk; return the class node)

```rust
/// Like `method_owner` but returns the enclosing CLASS DEFINITION node (not its name),
/// for single-file-class languages only. The self-receiver class identity. None for
/// Rust/Go/etc. (methods span files there).
pub fn method_owner_class_node<'a>(&self, func_node: &Node<'a>) -> Option<Node<'a>> {
    match self {
        Language::Python => {
            let mut n = *func_node;
            if let Some(p) = n.parent() {
                if p.kind() == "decorated_definition" { n = p; }
            }
            let block = n.parent()?;
            if block.kind() != "block" { return None; }
            let cls = block.parent()?;
            (cls.kind() == "class_definition").then_some(cls)
        }
        Language::JavaScript | Language::TypeScript | Language::Tsx => {
            let mut body = func_node.parent()?;
            if matches!(body.kind(), "field_definition" | "public_field_definition") {
                body = body.parent()?;
            }
            if body.kind() != "class_body" { return None; }
            let cls = body.parent()?;
            matches!(cls.kind(), "class_declaration" | "class").then_some(cls)
        }
        _ => None,
    }
}
```

- [ ] **Step 4: Run to verify it passes** — `cargo test --lib method_owner_class_node_finds_class` → PASS.

- [ ] **Step 5: Commit**

```bash
git add src/languages/mod.rs
git commit -m "feat(lang): method_owner_class_node — enclosing class node for Py/JS/TS"
```

---

## Task 2: `method_class_span` field + population

**Files:** Modify `src/call_graph.rs` (`method_metadata` `:1799`; `FileFunctions` `:457`; assembly
`:519-548`; field `:164`; literals `:227,:368,:970,:1581`; inserts `:292,:530,:1479`; `merge` `:1079`;
`remove_files` `:1039-1040`); test in `#[cfg(test)]`.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn method_class_span_populated_for_python_methods() {
    use std::collections::BTreeMap;
    let mut files = BTreeMap::new();
    files.insert(
        "a.py".to_string(),
        ParsedFile::parse("a.py", "class C:\n    def f(self):\n        return 1\n", Language::Python).unwrap(),
    );
    let cg = CallGraph::build(&files);
    let fid = cg.functions.get("f").unwrap().iter().find(|f| f.file == "a.py").unwrap();
    let span = cg.method_class_span.get(fid).expect("span recorded");
    assert_eq!(span.0, 0);
    assert!(span.1 > span.0);
}
```

- [ ] **Step 2: Run to verify it fails** — `cargo test --lib method_class_span_populated` → FAIL (no field).

- [ ] **Step 3: Implement**

1. `method_metadata` → 4-tuple (add the class span):

```rust
fn method_metadata(
    parsed: &ParsedFile,
    func_node: &tree_sitter::Node<'_>,
) -> (Option<String>, Option<String>, Option<String>, Option<(usize, usize)>) {
    let owner = parsed.language.method_owner(func_node)
        .map(|n| crate::resolution::owner_key(parsed.node_text(&n)));
    let trait_key = parsed.language.rust_impl_trait(func_node)
        .map(|n| crate::resolution::owner_key(parsed.node_text(&n)));
    let recv_var = parsed.language.go_receiver_var(func_node)
        .map(|n| parsed.node_text(&n).to_string());
    let class_span = parsed.language.method_owner_class_node(func_node)
        .map(|c| (c.start_byte(), c.end_byte()));
    (owner, trait_key, recv_var, class_span)
}
```

2. Add the field to `CallGraph` (after `method_owners`, `:164`):

```rust
/// Method FunctionId -> owner class definition node byte span (start, end). Py/JS/TS
/// only; the self-receiver class identity. See spec §3.2.
#[serde(default)]
pub method_class_span: BTreeMap<FunctionId, (usize, usize)>,
```

3. Update **all three** `method_metadata` call sites to destructure the 4th element and insert when `Some`:
   - `build_skeleton` (`:286`): `let (owner, trait_key, recv_var, class_span) = Self::method_metadata(...);`
     then inside `if let Some(o) = owner { ...; if let Some(s) = class_span { method_class_span.insert(func_id.clone(), s); } }`.
   - `build_direct_subset` (`:1473`): same.
   - full build par-map (`:488`): destructure the 4-tuple; extend `FileFunctions.functions` to a 7-tuple
     `(name, FunctionId, Option<String>, Option<String>, Option<String>, Option<MethodFacts>, Option<(usize,usize)>)`
     (`:457-465`); push `class_span` last (`:491-498`); in the assembly loop (`:520`) destructure 7 and
     `if let Some(s) = class_span { method_class_span.insert(func_id.clone(), s); }`.

4. Declare `let mut method_class_span: BTreeMap<FunctionId, (usize, usize)> = BTreeMap::new();` in each
   builder beside the existing `let mut method_owners` (`:266`, `:442`, `:1440`); add `method_class_span,`
   to each `CallGraph` literal (`:368`, `:970`, `:1581`) and `method_class_span: BTreeMap::new(),` to
   `CallGraph::empty` (`:227`).

5. `merge` (`:1064`): `self.method_class_span.extend(other.method_class_span);` beside
   `self.method_owners.extend(...)` (`:1079`).

6. `remove_files` (`:1009`): `self.method_class_span.retain(|fid, _| !exclude.contains(&fid.file));` beside
   `self.method_owners.retain(...)` (`:1039-1040`) — **retain**, do NOT clear (stable per-method fact).

- [ ] **Step 4: Run to verify it passes** — `cargo build` clean (all literals updated) +
  `cargo test --lib method_class_span_populated` → PASS.

- [ ] **Step 5: Commit**

```bash
git add src/call_graph.rs
git commit -m "feat(call-graph): record per-method owner-class byte span (method_class_span)"
```

---

## Task 3: `self_owner_lookup_same_class` + gate the self arm (the behavior change)

**Files:** Modify `src/resolution.rs` (helper near `:691`; self arm `:918-936`). Tests: a `#[cfg(test)]`
resolution module using the shared `files()`/`resolve_self_call` helpers (real-source multi-file — the
codebase idiom).

- [ ] **Step 1: Write the failing tests** (concrete assertions; one per case-table row)

```rust
#[test]
fn self_call_cross_file_collision_resolves_exact_to_caller_class() {
    let cg = CallGraph::build(&files(&[
        ("a.py", "class C:\n    def m(self):\n        return 1\n    def run(self):\n        return self.m()\n"),
        ("b.py", "class C:\n    def m(self):\n        return 2\n"),
    ]));
    let out = resolve_self_call(&cg, "a.py", "run", "m");
    assert_eq!(out.resolved.len(), 1, "single same-class target");
    assert_eq!(out.resolved[0].target.file, "a.py");
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(out.resolved[0].kind, ResolutionKind::SelfReceiver);
}

#[test]
fn self_call_absent_on_caller_class_cross_file_drops() {
    let cg = CallGraph::build(&files(&[
        ("a.py", "class Widget:\n    def render(self):\n        return 1\n"),
        ("b.py", "class Widget:\n    def draw(self):\n        return self.render()\n"),
    ]));
    let out = resolve_self_call(&cg, "b.py", "draw", "render");
    assert!(out.resolved.is_empty(), "must NOT bind to a.py's unrelated Widget");
    assert_eq!(out.drop, Some(DropReason::UnknownName));
}

#[test]
fn self_call_same_file_nested_duplicate_class_drops() {
    let cg = CallGraph::build(&files(&[
        ("a.py", "def o1():\n    class C:\n        def f(self):\n            return self.m()\ndef o2():\n    class C:\n        def m(self):\n            return 1\n"),
        ("b.py", "class C:\n    def m(self):\n        return 2\n"),
    ]));
    let out = resolve_self_call(&cg, "a.py", "f", "m");
    assert!(out.resolved.is_empty(), "o1.C has no m; must not bind to o2.C or b.C");
    assert_eq!(out.drop, Some(DropReason::UnknownName));
}

#[test]
fn self_call_same_line_duplicate_class_js_drops() {
    // two `class C` nodes on ONE row -> byte-span (not start-line) must distinguish them
    let cg = CallGraph::build(&files(&[
        ("a.js", "class C { f() { return this.m(); } } class C { m() { return 1; } }\n"),
    ]));
    let out = resolve_self_call(&cg, "a.js", "f", "m");
    assert!(out.resolved.is_empty(), "f's class has no m; the same-line other C must not bind");
}

#[test]
fn self_call_static_plus_instance_same_name_nameonly() {
    let cg = CallGraph::build(&files(&[
        ("a.js", "class C { static m() {} m() {} run() { return this.m(); } }\n"),
    ]));
    let out = resolve_self_call(&cg, "a.js", "run", "m");
    assert_eq!(out.resolved.len(), 2, "both same-class same-name candidates kept");
    assert!(out.resolved.iter().all(|c| c.confidence == ResolutionConfidence::NameOnly));
}

#[test]
fn go_receiver_var_call_unchanged() {
    // Go method receiver var r.other() resolves via owner_lookup, NOT the Py/JS narrowing
    let cg = CallGraph::build(&files(&[
        ("a.go", "package p\ntype T struct{}\nfunc (r T) other() int { return 1 }\nfunc (r T) run() int { return r.other() }\n"),
    ]));
    let out = resolve_self_call(&cg, "a.go", "run", "other");
    assert_eq!(out.resolved.len(), 1);
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
}
```

- [ ] **Step 2: Run to verify they fail** — `cargo test --lib self_call_` → the collision test FAILs
  (today NameOnly, len 1 but confidence NameOnly), the FP/nested/same-line tests FAIL (today bind to a wrong
  class — non-empty `resolved`); static+instance and Go assert no-regression (should pass already).

- [ ] **Step 3: Implement** (helper near `owner_lookup`)

```rust
fn self_owner_lookup_same_class(
    &self,
    owner: &str,
    name: &str,
    caller: &FunctionId,
) -> Option<Vec<ResolvedCallee<'_>>> {
    let caller_span = *self.method_class_span.get(caller)?;
    let ids = self.methods.get(&(owner.to_string(), name.to_string()))?;
    let same_class: Vec<&FunctionId> = ids
        .iter()
        .filter(|fid| fid.file == caller.file && self.method_class_span.get(*fid) == Some(&caller_span))
        .collect();
    match same_class.len() {
        0 => None,
        1 => Some(exact(same_class, ResolutionKind::QualifiedOwner)),
        _ => Some(demoted(same_class, ResolutionKind::QualifiedOwner)),
    }
}
```

Gate the self arm (`:918-936`) per spec §3.4: compute `narrow` from `Language::from_path(&caller.file) ∈
{Python, JavaScript, TypeScript, Tsx}`; call `self_owner_lookup_same_class(owner, name, caller)` when
`narrow`, else `owner_lookup(owner, name)`; keep the `QualifiedOwner→SelfReceiver` relabel loop and the
`dropped(DropReason::UnknownName)` fallthrough.

- [ ] **Step 4: Run to verify they pass** — `cargo test --lib self_call_` → all PASS; `cargo test --lib`
  → no other regressions.

- [ ] **Step 5: Commit**

```bash
git add src/resolution.rs
git commit -m "feat(resolution): self-receiver same-class owner narrowing (Py/JS/TS)"
```

---

## Task 4: `CACHE_VERSION` bump + version test

**Files:** Modify `src/cpg_cache.rs` (`CACHE_VERSION` `:65`; assertion test `:572`).

- [ ] **Step 1: Adjust the failing test** — change the assertion at `:572` to `assert_eq!(super::CACHE_VERSION, 21);`.
- [ ] **Step 2: Run to verify it fails** — `cargo test --lib` (the cache-version test) → FAIL (still 20).
- [ ] **Step 3: Implement** — bump the private const at `:65`: `const CACHE_VERSION: u32 = 21; // 21:
  method_class_span (self-receiver same-class narrowing).` (It is `const`, not `pub const` — leave private.)
- [ ] **Step 4: Run to verify it passes** — `cargo test --lib` cache-version test → PASS; `cargo build --features mcp`.
- [ ] **Step 5: Commit**

```bash
git add src/cpg_cache.rs
git commit -m "chore(cache): CACHE_VERSION 20->21 (method_class_span)"
```

---

## Task 5: Real-source integration fixtures + merged-graph coverage guard

**Files:** Create `tests/lang/python/self_receiver_test.rs` (+ `mod self_receiver_test;` in
`tests/lang/python/main.rs`; + path in the three `coverage_test.rs` arrays `:104/:323/:470`); add a JS
self-receiver test file (+ `mod` line + coverage arrays).

- [ ] **Step 1: Write the failing/▸passing tests** — basic end-to-end + the merged-graph coverage guard
  (proves `method_class_span` survives `extend`):

```rust
#[test]
fn python_same_class_self_call_resolves_exact() {
    let cg = CallGraph::build(&files(&[
        ("svc.py", "class Svc:\n    def step(self):\n        return self.run_once()\n    def run_once(self):\n        return 1\n"),
    ]));
    let out = resolve_self_call(&cg, "svc.py", "step", "run_once");
    assert_eq!(out.resolved.len(), 1);
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(out.resolved[0].kind, ResolutionKind::SelfReceiver);
}

#[test]
fn merged_graph_still_narrows_self_calls() {
    let base = CallGraph::build(&files(&[("b.py", "class C:\n    def m(self):\n        return 2\n")]));
    let mut cg = base;
    let fresh = CallGraph::build(&files(&[
        ("a.py", "class C:\n    def m(self):\n        return 1\n    def run(self):\n        return self.m()\n"),
    ]));
    cg.merge(fresh);
    let out = resolve_self_call(&cg, "a.py", "run", "m");
    assert_eq!(out.resolved.len(), 1, "method_class_span survived merge -> still narrows");
    assert_eq!(out.resolved[0].target.file, "a.py");
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
}
```

- [ ] **Step 2: Run** — `cargo test --test lang_python self_receiver_test::` → PASS after Tasks 1–4 (fix
  any registration gaps: `mod` line, `coverage_test.rs` arrays).
- [ ] **Step 3: Make them pass** — no new production code; only test registration.
- [ ] **Step 4: Full suite + fmt** — `cargo fmt && cargo test && cargo test --features mcp`; `cargo fmt --check` clean.
- [ ] **Step 5: Commit**

```bash
git add tests/
git commit -m "test(self-receiver): Py/JS integration fixtures + merged-graph coverage guard"
```

---

## Task 6: Acceptance (host-run; not a code change)

- [ ] `cargo build --release --bin prism`.
- [ ] **pydantic** before/after `self_receiver` Exact/NameOnly split (main vs branch via a git worktree
  build — never swap the binary mid-measurement). **Measure** the delta (do not assert a number; the buy is
  the cross-file-collision subset — decorated double-captures stay NameOnly per spec §9). Confirm
  `multi_target_exact_sites` byte-flat.
- [ ] **Identity-aware check:** every newly-Exact `self_receiver` site has target class span == caller
  class span (no singleton wrong Exact).
- [ ] **fastapi**: `self_receiver` Exact stable/slightly up; no regressions.
- [ ] **Rust/Go** (ripgrep, caddy): call-stats **byte-identical**.
- [ ] **Tier-A**: `cd eval && uv run tier-a --matrix-only --allow-stale-sut` (0 regr) then `--quick` M2
  dogfood (P/fp unchanged).
- [ ] Paste the pydantic split + canary + Tier-A into the PR description.

---

## Self-Review notes (author)

- **Spec coverage:** T1 `method_owner_class_node` (§3.2); T2 `method_class_span` population (§3.2); T3
  helper + gate (§3.3–3.4) with the full §3.6 case table as concrete tests; T4 cache (§5); T5 integration +
  merged-graph (population-coverage guard); T6 §6 acceptance.
- **Type consistency:** `method_metadata` 4-tuple at all 3 callers; `FileFunctions.functions` 7-tuple;
  `method_class_span: BTreeMap<FunctionId,(usize,usize)>` keyed identically in population + lookup;
  `ResolutionOutcome { resolved, drop }`; `exact`/`demoted` take `impl IntoIterator<Item=&FunctionId>`.
- **No placeholders:** all test bodies concrete with exact assertions incl. `drop == Some(UnknownName)`.
- **Risk:** population coverage (T2 steps 3–6) is soundness-critical; the merged-graph test (T5) guards the
  `extend` path; the same-line JS test (T3) guards byte-span (not line) identity.
