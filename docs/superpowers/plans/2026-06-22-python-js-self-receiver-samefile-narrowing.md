# Python/JS Self-Receiver Same-Class Owner Narrowing — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended)
> or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** For `self`/`this`/`cls` method calls in Python/JS/TS, resolve to the caller's *own* class by
narrowing the owner-index lookup to candidates sharing the caller's class identity `(file, owner-class byte
span)` — upgrading cross-file-collision NameOnly to Exact and closing a full-confidence cross-class
false-positive.

**Architecture:** A new `CallGraph.method_class_span: BTreeMap<FunctionId, (usize, usize)>` records each
method's owner-class definition node's byte span, populated from a new `Language::method_owner_class_node`
helper (mirrors `method_owner`'s walk, returns the class node). A new resolver helper
`self_owner_lookup_same_class` filters `methods[(owner,name)]` by that identity; the self arm at
`resolution.rs:918` calls it gated to Python/JS/TS/Tsx (Go/Rust keep `owner_lookup`). `CACHE_VERSION` bumps
20→21.

**Tech Stack:** Rust, tree-sitter, `serde` (cached `CallGraph`), `cargo test`.

**Design-of-record:** `docs/superpowers/specs/2026-06-22-python-js-self-receiver-samefile-narrowing.md`
(rev 3; both codex spec-reviews folded). Read §3 before starting.

---

## File Structure

- `src/languages/mod.rs` — add `method_owner_class_node` (the class-node walk, Python/JS/TS/Tsx).
- `src/call_graph.rs` — extend `method_metadata` to also return the owner-class span; add the
  `method_class_span` field; thread it through every `CallGraph` literal + populate at every
  `method_owners` write + merge/prune.
- `src/resolution.rs` — add `self_owner_lookup_same_class`; gate the self arm (`:918-936`).
- `src/cpg_cache.rs` — `CACHE_VERSION` 20→21; update the version assertion test.
- `tests/lang/python/` (create `main.rs` + `self_receiver_test.rs` if absent) and
  `tests/lang/javascript/` — real-source fixtures.
- `tests/integration/` or a resolution unit-test module — hand-built `CallGraph` cases for the multi-class
  scenarios (cross-file collision, same-file nested dup, same-line dup, static+instance, merged-graph
  coverage, Go non-regression).

> **Note on test homes:** verify whether `tests/lang/python/` exists. If not, create
> `tests/lang/python/main.rs` declaring `mod self_receiver_test;`, register a `[[test]]` target
> `lang_python` in `Cargo.toml` (mirror the existing `lang_javascript` target), and add the new file paths
> to the three `all_test_files` arrays in `tests/integration/coverage_test.rs` (per CLAUDE.md).

---

## Task 1: `Language::method_owner_class_node`

**Files:**
- Modify: `src/languages/mod.rs` (next to `method_owner`, ~`:1056`)
- Test: `src/languages/mod.rs` `#[cfg(test)]` module (or the existing AST test home)

- [ ] **Step 1: Write the failing test**

In a test module with a parsing helper (mirror existing `method_owner` tests if present), assert the class
node is returned for Python and JS, and `None` for Rust/Go:

```rust
#[test]
fn method_owner_class_node_returns_class_def_span() {
    // Python: method inside `class C` -> class_definition node spanning the whole class.
    let src = "class C:\n    def f(self):\n        return 1\n";
    let parsed = ParsedFile::parse("a.py", src).unwrap();
    let func = first_function(&parsed, "f");
    let cls = parsed.language.method_owner_class_node(&func).expect("class node");
    assert_eq!(cls.kind(), "class_definition");
    assert_eq!(cls.start_byte(), 0); // class C starts at byte 0

    // Rust: no single-file class node -> None.
    let rsrc = "struct S;\nimpl S { fn f(&self) {} }\n";
    let rparsed = ParsedFile::parse("a.rs", rsrc).unwrap();
    let rfunc = first_function(&rparsed, "f");
    assert!(rparsed.language.method_owner_class_node(&rfunc).is_none());
}
```

(Use the crate's existing parse + function-finding helpers; if none, add a tiny local `first_function` that
walks `parsed.all_functions()` for the named function.)

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --lib method_owner_class_node_returns_class_def_span`
Expected: FAIL — `method_owner_class_node` does not exist.

- [ ] **Step 3: Implement**

Add to `impl Language`, mirroring `method_owner`'s walk but returning the class node:

```rust
/// Like `method_owner` but returns the enclosing CLASS DEFINITION node (not its name),
/// for single-file-class languages only. Used to key a method to its exact class for
/// self-receiver narrowing. Returns None for Rust/Go/etc. (methods span files there).
pub fn method_owner_class_node<'a>(&self, func_node: &Node<'a>) -> Option<Node<'a>> {
    match self {
        Language::Python => {
            let mut n = *func_node;
            if let Some(p) = n.parent() {
                if p.kind() == "decorated_definition" {
                    n = p;
                }
            }
            let block = n.parent()?;
            if block.kind() != "block" {
                return None;
            }
            let cls = block.parent()?;
            (cls.kind() == "class_definition").then_some(cls)
        }
        Language::JavaScript | Language::TypeScript | Language::Tsx => {
            let mut body = func_node.parent()?;
            if matches!(body.kind(), "field_definition" | "public_field_definition") {
                body = body.parent()?;
            }
            if body.kind() != "class_body" {
                return None;
            }
            let cls = body.parent()?;
            matches!(cls.kind(), "class_declaration" | "class").then_some(cls)
        }
        _ => None,
    }
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test --lib method_owner_class_node_returns_class_def_span`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/languages/mod.rs
git commit -m "feat(lang): method_owner_class_node — enclosing class node for Py/JS/TS"
```

---

## Task 2: `method_class_span` field + population

**Files:**
- Modify: `src/call_graph.rs` (`method_metadata` `:1799`; `FileFunctions` `:457`; assembly `:519-548`;
  field decl `:164`; all `CallGraph` literals `:227,:368,:970,:1581`; `build_skeleton` insert `:292`;
  `build_direct_subset` insert `:1479`; `merge` extend `:1079`; `remove_files` retain `:1039-1040`)
- Test: a `#[cfg(test)]` in `src/call_graph.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn method_class_span_populated_for_python_methods() {
    use std::collections::BTreeMap;
    let mut files = BTreeMap::new();
    files.insert(
        "a.py".to_string(),
        ParsedFile::parse("a.py", "class C:\n    def f(self):\n        return 1\n").unwrap(),
    );
    let cg = CallGraph::build(&files);
    let fid = cg.functions.get("f").unwrap().iter().find(|f| f.file == "a.py").unwrap();
    let span = cg.method_class_span.get(fid).expect("span recorded");
    assert_eq!(span.0, 0); // class C starts at byte 0
    assert!(span.1 > span.0);
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --lib method_class_span_populated_for_python_methods`
Expected: FAIL — no field `method_class_span`.

- [ ] **Step 3: Implement**

1. Extend `method_metadata` to return the class span as a 4th element:

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

2. Add the field to the `CallGraph` struct (next to `method_owners`, `:164`):

```rust
/// Method FunctionId -> owner class definition node byte span (start, end). Py/JS/TS
/// only; the self-receiver class identity. See spec §3.2.
#[serde(default)]
pub method_class_span: BTreeMap<FunctionId, (usize, usize)>,
```

3. In `build_with_receiver_config…`: add a `class_span` local map; extend the `FileFunctions.functions`
   tuple with `Option<(usize, usize)>`; in the par_iter map destructure the 4-tuple from `method_metadata`
   and push `class_span`; in the serial assembly insert it when `Some`:

```rust
// FileFunctions tuple -> add a 7th element: Option<(usize, usize)>
// in the map:
let (owner, trait_key, recv_var, class_span) = Self::method_metadata(parsed, &func_node);
file_functions.push((name.clone(), func_id, owner, trait_key, recv_var, facts, class_span));
// in the assembly loop (destructure 7-tuple):
for (name, func_id, owner, trait_key, recv_var, facts, class_span) in file_functions.functions {
    // ... existing inserts ...
    if let Some(span) = class_span {
        method_class_span.insert(func_id.clone(), span);
    }
}
```

4. In `build_skeleton` (`:286-292`) and `build_direct_subset` (`:1470-1479`): destructure the new
   4-tuple from `method_metadata` and `method_class_span.insert(func_id.clone(), span)` when `Some`.

5. Add `method_class_span` to **every** `CallGraph` struct literal: `empty` (`:227`), `build_skeleton`
   (`:368`), `build_with_receiver_config…` (`:970`), `build_direct_subset` (`:1581`) — as a populated map
   in the builders and `BTreeMap::new()` in `empty`.

6. **Merge:** in `merge` (`:1064`) add `self.method_class_span.extend(other.method_class_span);` beside
   `self.method_owners.extend(...)` (`:1079`). **Prune:** in `remove_files` (`:1009`) add
   `self.method_class_span.retain(|fid, _| !exclude.contains(&fid.file));` beside `self.method_owners.retain`
   (`:1039-1040`) — `method_class_span` is a stable per-method fact, so **retain** (do NOT `clear` it like
   `methods_by_scope`), mirroring `method_owners`/`receiver_vars`/`method_facts`.

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test --lib method_class_span_populated_for_python_methods`
Expected: PASS. Also `cargo build` clean (all `CallGraph` literals updated).

- [ ] **Step 5: Commit**

```bash
git add src/call_graph.rs
git commit -m "feat(call-graph): record per-method owner-class byte span (method_class_span)"
```

---

## Task 3: `self_owner_lookup_same_class` + gate the self arm (the behavior change)

**Files:**
- Modify: `src/resolution.rs` (helper near `owner_lookup` `:691`; self arm `:918-936`)
- Test: `src/resolution.rs` `#[cfg(test)]` (hand-built `CallGraph`) + real-source fixtures (Task 5)

- [ ] **Step 1: Write the failing tests** (hand-built `CallGraph`, exercising each case-table row)

Add a resolution test that builds graphs from real source and asserts kinds/confidence. Cases:

```rust
// helper: build CallGraph from inline files, resolve every call, return (callee_file, kind, confidence)
fn resolve_all(files: &[(&str, &str)]) -> Vec<(String, String, String)> { /* build + iterate calls */ }

#[test]
fn self_call_cross_file_collision_resolves_exact_to_caller_class() {
    // two files each define `class C` with method m; caller in a.py -> Exact to a.py's C.m
    let out = resolve_all(&[
        ("a.py", "class C:\n    def m(self):\n        return 1\n    def run(self):\n        return self.m()\n"),
        ("b.py", "class C:\n    def m(self):\n        return 2\n"),
    ]);
    let m = out.iter().find(|(_, _, _)| true /* the self.m() site */);
    // assert the self.m() call resolves Exact (score 1.0) and ONLY to a.py
}

#[test]
fn self_call_absent_on_caller_class_drops_not_wrong_exact() {
    // b.py's C has no render; a.py's C does -> self.render() in b.py must DROP (no edge), not bind to a.py
}

#[test]
fn self_call_same_file_nested_duplicate_class_drops() {
    // a.py: outer1 has class C with f()->self.m() (no m); outer2 has class C with m(); b.py: class C with m()
    // -> self.m() must DROP (was NameOnly to two wrong classes today)
}

#[test]
fn self_call_static_plus_instance_same_name_nameonly() {
    // JS class with `static m(){}` and `m(){}`, this.m() in instance method -> NameOnly (>=2 same class)
}

#[test]
fn go_receiver_var_call_unchanged() {
    // Go `func (r *T) M()` calling r.other() -> resolves via owner_lookup exactly as before (no narrowing)
}
```

Write each with concrete assertions on the resolved set (target file + `ResolutionConfidence` + `kind`).
Use `prism nav callees` output semantics as the oracle for expected targets.

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test --lib self_call_`
Expected: the collision test FAILs (today NameOnly, not Exact); the FP/nested tests FAIL (today bind to a
wrong class); static+instance & Go tests likely already pass (assert no regression).

- [ ] **Step 3: Implement**

Add the helper (near `owner_lookup`):

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

Gate the self arm (`:918-936`) per spec §3.4 — compute `narrow` from
`Language::from_path(&caller.file) ∈ {Python, JavaScript, TypeScript, Tsx}`, call
`self_owner_lookup_same_class` when `narrow` else `owner_lookup`, keep the `QualifiedOwner→SelfReceiver`
relabel and the `dropped(UnknownName)` fallthrough.

- [ ] **Step 4: Run to verify they pass**

Run: `cargo test --lib self_call_`
Expected: all PASS. Run `cargo test --lib` — no other resolution regressions.

- [ ] **Step 5: Commit**

```bash
git add src/resolution.rs
git commit -m "feat(resolution): self-receiver same-class owner narrowing (Py/JS/TS)"
```

---

## Task 4: `CACHE_VERSION` bump + version test

**Files:**
- Modify: `src/cpg_cache.rs` (`CACHE_VERSION` const; the version assertion test ~`:570`)

- [ ] **Step 1: Write/adjust the failing test**

Update the existing cache-version test to expect `21`:

```rust
assert_eq!(CACHE_VERSION, 21);
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --lib cache_version`  (use the real test name)
Expected: FAIL — still 20.

- [ ] **Step 3: Implement**

Bump `pub const CACHE_VERSION: u32 = 21;` (was 20). Add a one-line comment: `// 21: method_class_span
(self-receiver same-class narrowing)`.

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test --lib cache_version` then `cargo test --features mcp` build check.
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/cpg_cache.rs
git commit -m "chore(cache): CACHE_VERSION 20->21 (method_class_span)"
```

---

## Task 5: Real-source fixture tests + merged-graph coverage guard

**Files:**
- Create: `tests/lang/python/self_receiver_test.rs` (+ `main.rs` + Cargo `[[test]]` + coverage_test.rs
  arrays if `tests/lang/python/` is new — see File Structure note)
- Create/Modify: `tests/lang/javascript/` self-receiver test file (+ `mod` line)
- Test: as above

- [ ] **Step 1: Write the failing tests**

End-to-end via the public CLI/`CallGraph` API: build a fixture repo, assert the discriminating outcomes
(same-class Exact; cross-file-collision Exact to caller's file; cross-class FP drop; same-file nested-dup
drop; **same-line JS/TS dup drop**; Go unchanged). Add a **merged-graph** test: build two subgraphs and
`merge` them, then assert a self-call still narrows (proves `method_class_span` survives `extend`).

```rust
#[test]
fn merged_graph_still_narrows_self_calls() {
    // build CallGraph for a.py, build another for b.py, merge, resolve a.py self-call -> Exact, single target
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test --test lang_python self_receiver_test::` and `--test lang_javascript`.
Expected: FAIL (collision/FP cases) before the wiring is reachable from the test target; PASS after.

- [ ] **Step 3: Make them pass**

No new production code — Tasks 1–4 supply behavior. Fix any test-harness/registration gaps
(`coverage_test.rs` arrays, `main.rs` `mod` lines, Cargo `[[test]]`).

- [ ] **Step 4: Run the full suite + fmt**

Run: `cargo fmt && cargo test && cargo test --features mcp`
Expected: all PASS, `cargo fmt --check` clean.

- [ ] **Step 5: Commit**

```bash
git add tests/ Cargo.toml
git commit -m "test(self-receiver): Py/JS discriminating fixtures + merged-graph coverage guard"
```

---

## Task 6: Acceptance (host-run; not a code change)

- [ ] Build release: `cargo build --release --bin prism`.
- [ ] **pydantic** before/after `self_receiver` Exact/NameOnly split (main vs branch via a git worktree
  build — never swap the binary mid-measurement): record the delta + that `multi_target_exact_sites` is
  byte-flat. Expect Exact↑ (cross-file-collision subset; decorated double-captures stay NameOnly).
- [ ] **Identity-aware check:** confirm every newly-Exact `self_receiver` site has target class span ==
  caller class span (no singleton wrong Exact). The nested-dup + same-line fixtures must drop.
- [ ] **fastapi**: `self_receiver` Exact stable/slightly up; no regressions.
- [ ] **Rust/Go** (ripgrep, caddy): call-stats **byte-identical**.
- [ ] **Tier-A**: `cd eval && uv run tier-a --matrix-only --allow-stale-sut` (0 regr) then `--quick` M2
  dogfood (P/fp unchanged).
- [ ] Paste the pydantic split + canary + Tier-A result into the PR description.

---

## Self-Review notes (author)

- **Spec coverage:** Tasks 1–2 = `method_class_span` (§3.2); Task 3 = helper + gate (§3.3–3.4); Task 4 =
  cache (§5); Task 5 = the §6 fixtures incl. same-line + merged-graph (population-coverage guard); Task 6 =
  §6 acceptance. All spec sections mapped.
- **Type consistency:** `method_metadata` returns a 4-tuple everywhere; `FileFunctions.functions` is a
  7-tuple everywhere; `method_class_span: BTreeMap<FunctionId,(usize,usize)>` keyed identically in
  population and lookup; helper returns `QualifiedOwner` and the arm relabels to `SelfReceiver`.
- **No placeholders:** all code shown; exact lines cited; commands concrete.
- **Risk:** the population checklist (Task 2 steps 3–6) is the soundness-critical part — every
  `method_owners` write/literal/merge has a paired `method_class_span` action; the merged-graph test
  (Task 5) guards it.
