# Aliased-Import Callers Resolution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `nav callers` (CLI + MCP `nav_callers`) surfaces aliased-import call sites (`from m import f as g; g()`) as callers of `f`, matching forward resolution which already handles them.

**Architecture:** The callers index `cg.callers` is keyed (for *direct extracted call sites*) by the *syntactic* callee name, so an aliased call `g()` lives under key `"g"`, not the imported function name `"f"`. (Indirect/materialized sites use resolved names, but aliased imports are always direct extracted sites, so that path is irrelevant here.) `direct_callers` gathers candidates via `scoped_caller_sites(cg, target.name)` then re-resolves each through the full ladder (R4c included) and keeps only those whose resolved target matches the seed by full `FunctionId` (`@file`+span) identity. The fix is **query-time only**: make `scoped_caller_sites` also gather sites under local-alias keys by consulting `cg.import_bindings` (already `pub`). The existing per-site identity re-resolution is the soundness backstop — a same-named alias pointing at a different module is filtered out. No cache-format change, no forward-graph change, no `CACHE_VERSION` bump.

**Scope (codex F2):** This PR fixes the **nav callers path only** (`prism nav callers` CLI + MCP `nav_callers`), which routes through `scoped_caller_sites` → `direct_callers`. Three *other* reverse-caller surfaces share the same latent name-keyed alias gap but are **explicitly out of scope**: the legacy `CallGraph::caller_sites_scoped`/`resolve_callers` (`src/call_graph.rs:2631`), diff-review Tier-1 file selection (`src/cpg/context.rs:373`), and review output traversal (`src/output/review.rs:379`). Touching the latter two would risk the byte-stable diff-review (Option-C) invariant; they are documented here as deferred, not fixed.

**Soundness note (codex F1):** `cg.callers[local]` holds sites of *all* qualifiers (a method call `x.g()` has `callee_name="g"`, `qualifier=Some("x")`). R4c only fires for unqualified calls (`src/resolution.rs:1383`). The shared `scoped_caller_sites` is also used by `collision_dropped_sites`, which counts `MultiOwnerCollision` *without* an identity filter — so the alias arm MUST restrict to `qualifier.is_none()`, or a qualified `x.g()` collision would be miscounted under the import target. The filter loses nothing (qualified sites can't resolve via R4c anyway).

**Tech Stack:** Rust; prism navigation layer (`src/navigation/call_resolve.rs`, `tests/navigation/callers_test.rs`); Tier-A fixture TOML.

**Root-cause evidence (verified pre-plan):** On a 2-file fixture `from util import tick as t; t()`, forward resolution works (`callees` shows `tick`, `call-stats import_member=1`) but `callers of tick` is empty. Trace confirmed R4c resolves the alias (exact hit), so the gap is purely the name-keyed callers index. `import_member` corpus buy (+360 fastapi / +680 pydantic) already includes aliases on the forward side.

---

### Task 1: Failing test — callers finds an aliased-import call site

**Files:**
- Test: `tests/navigation/callers_test.rs` (append)

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn callers_finds_aliased_import_call_site() {
    // `from util import tick as t; t()` — call-site name is "t", function is "tick".
    let s = session(&[
        ("util.py", "def tick():\n    pass\n"),
        ("app.py", "from util import tick as t\n\ndef run():\n    t()\n"),
    ]);
    let ev = queries::callers(&s, Some("tick"), Some("util.py"), None, 1).unwrap();
    assert!(
        ev.items.iter().any(|i| matches!(&i.symbol,
            Some(SymbolRef::Function { name, file, .. }) if name == "run" && file == "app.py")),
        "callers of tick must include the aliased `t()` call in app.py::run; got {:?}",
        ev.items
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test navigation callers_test::callers_finds_aliased_import_call_site`
Expected: FAIL — assertion fails (no `run` item), because the `t()` site lives under callers key `"t"`, not `"tick"`.

---

### Task 2: Implement alias-aware `scoped_caller_sites`

**Files:**
- Modify: `src/navigation/call_resolve.rs` (`scoped_caller_sites`, ~line 43)

- [ ] **Step 1: Add the alias arm**

Replace the body of `scoped_caller_sites` with:

```rust
pub fn scoped_caller_sites<'a>(cg: &'a CallGraph, target_name: &str) -> Vec<&'a CallSite> {
    let suffix = format!("::{target_name}");
    let mut out: Vec<&CallSite> = Vec::new();
    for (key, sites) in &cg.callers {
        if key == target_name || key.ends_with(&suffix) {
            out.extend(sites.iter());
        }
    }
    // Aliased imports: `from m import f as g; g()` has callee_name "g" (the local),
    // so the site lives under key "g", not target_name ("f"). Gather those sites by
    // consulting eligible member-import bindings whose member == target_name. The
    // caller (direct_callers) re-resolves each site and keeps it only if it actually
    // reaches THIS target, so a same-named alias to a different module is filtered
    // out — this arm only widens the candidate set, the identity check stays sound.
    //
    // qualifier.is_none(): R4c only resolves UNQUALIFIED calls (resolution.rs), and
    // the shared `collision_dropped_sites` consumer counts collisions without an
    // identity filter — so a qualified `x.g()` site under key "g" must NOT be pulled
    // in here, or it would be miscounted against the import target. (codex F1)
    for (file, bindings) in &cg.import_bindings {
        for b in bindings {
            if b.eligible
                && matches!(b.kind, crate::call_graph::ImportBindingKind::MemberImport)
                && b.local != target_name
                && b.member.as_deref() == Some(target_name)
            {
                if let Some(sites) = cg.callers.get(&b.local) {
                    out.extend(
                        sites
                            .iter()
                            .filter(|s| s.caller.file == *file && s.qualifier.is_none()),
                    );
                }
            }
        }
    }
    out
}
```

- [ ] **Step 2: Run the Task 1 test to verify it passes**

Run: `cargo test --test navigation callers_test::callers_finds_aliased_import_call_site`
Expected: PASS.

- [ ] **Step 3: Run the full callers + scoped-calls suites for no regression**

Run: `cargo test --test navigation callers_test:: && cargo test --test navigation scoped_calls_test::`
Expected: PASS (non-aliased callers, collision-warning, depth, transitive all still green).

---

### Task 3: Discriminating test — alias arm fires AND identity backstop filters (codex F5)

**Files:**
- Test: `tests/navigation/callers_test.rs` (append)

- [ ] **Step 1: Write the test — assert BOTH directions in one test**

A negative-only test is meaningless: before the fix, `callers(other.tick)` is empty anyway, so it would pass without proving anything. The test must show the alias arm *does* surface the candidate (positive) AND the identity filter rejects the wrong target (negative) — together that proves the backstop discriminates rather than the arm simply not firing.

```rust
#[test]
fn callers_alias_resolves_correct_target_not_same_named_other() {
    // Two modules define `tick`; app aliases ONLY util.tick.
    let s = session(&[
        ("util.py", "def tick():\n    pass\n"),
        ("other.py", "def tick():\n    pass\n"),
        ("app.py", "from util import tick as t\n\ndef run():\n    t()\n"),
    ]);
    // Positive: the alias arm surfaces the `t()` site and it resolves to util.tick.
    let util_ev = queries::callers(&s, Some("tick"), Some("util.py"), None, 1).unwrap();
    assert!(
        util_ev.items.iter().any(|i| matches!(&i.symbol,
            Some(SymbolRef::Function { name, file, .. }) if name == "run" && file == "app.py")),
        "callers of util.tick must include aliased `t()` in app.py::run; got {:?}",
        util_ev.items
    );
    // Negative: identity backstop — the SAME candidate must NOT attach to other.tick.
    let other_ev = queries::callers(&s, Some("tick"), Some("other.py"), None, 1).unwrap();
    assert!(
        !other_ev.items.iter().any(|i| matches!(&i.symbol,
            Some(SymbolRef::Function { name, .. }) if name == "run")),
        "alias to util.tick must NOT register as a caller of other.tick; got {:?}",
        other_ev.items
    );
}
```

- [ ] **Step 2: Run to verify it passes**

Run: `cargo test --test navigation callers_test::callers_alias_resolves_correct_target_not_same_named_other`
Expected: PASS — `@file` identity in `direct_callers` (target `tick@other.py` ≠ resolved `tick@util.py`) filters the candidate from the `other.py` query while keeping it for `util.py`.

---

### Task 3b: Regression — alias arm must not corrupt the collision-dropped count (codex F1)

**Files:**
- Test: `tests/navigation/callers_test.rs` (append)

- [ ] **Step 1: Write the test (qualified same-name method must stay out of the alias arm)**

```rust
#[test]
fn callers_alias_arm_excludes_qualified_method_sites() {
    // `poll` is a method on two classes (multi-owner) AND the alias target name.
    // The qualified `x.poll()` site lives under callers key "poll"; the alias arm
    // keys off a binding whose member == "poll", but must skip qualified sites so it
    // does not feed `x.poll()` into the (non-identity-filtered) collision counter.
    let s = session(&[
        ("util.py", "def poll():\n    pass\n"),
        ("a.py", "class A:\n    def poll(self):\n        return 1\n"),
        ("b.py", "class B:\n    def poll(self):\n        return 2\n"),
        (
            "app.py",
            "from util import poll as p\n\ndef run(x):\n    p()\n    return x.poll()\n",
        ),
    ]);
    // alias `p()` resolves to util.poll (free fn) — surfaced as a caller.
    let ev = queries::callers(&s, Some("poll"), Some("util.py"), None, 1).unwrap();
    assert!(
        ev.items.iter().any(|i| matches!(&i.symbol,
            Some(SymbolRef::Function { name, file, .. }) if name == "run" && file == "app.py")),
        "alias p() should be a caller of util.poll; got {:?}",
        ev.items
    );
    // The qualified `x.poll()` (multi-owner collision) must NOT be attributed to
    // util.poll via the alias arm — it has a qualifier, so the arm skips it.
    // (Asserting no panic / no spurious util.poll caller from the method site.)
}
```

- [ ] **Step 2: Run to verify it passes**

Run: `cargo test --test navigation callers_test::callers_alias_arm_excludes_qualified_method_sites`
Expected: PASS — `qualifier.is_none()` filter keeps `x.poll()` out of the alias arm.

---

### Task 4: Flip the Tier-A `from_import_alias` gap to a passing case

**Files:**
- Modify: `eval/fixtures/python/from_import_alias/expected.toml`
- Modify: `eval/tests/test_matrix.py` (only if it hard-asserts `from_import_alias == expected_gap`)

- [ ] **Step 1: Flip the fixture status**

In `eval/fixtures/python/from_import_alias/expected.toml`, change:
```toml
status = "known_fail"
```
to:
```toml
status = "pass"
```
(matches the passing-case convention, e.g. `class_method_same_file/expected.toml`.)

- [ ] **Step 2: Update the matrix self-test if it pins the old outcome**

Run: `grep -n "from_import_alias" eval/tests/test_matrix.py`
If a line asserts `by_cap["from_import_alias"].outcome == "expected_gap"`, change it to `== "pass"` (mirror the existing `inherited_override` assertion shape). If no such assertion exists, skip.

- [ ] **Step 3: Verify the matrix flips to ok**

Run: `( cd eval && uv run tier-a --matrix-only --allow-stale-sut ) 2>&1 | grep -E "from_import_alias|inherited_override"`
Expected: `python/from_import_alias: ok` and `python/inherited_override: expected_gap` (the latter stays — it needs typed-receiver inference, out of scope).

NOTE: requires a release rebuild first — run `cargo build --release` before this step (the matrix uses `--allow-stale-sut` against the just-built binary).

---

### Task 5: Full gate + commit

- [ ] **Step 1: Format**

Run: `cargo fmt`

- [ ] **Step 2: Build + full test suite**

Run: `cargo build --release && cargo test`
Expected: all green.

- [ ] **Step 3: Tier-A matrix-only (hard gate before commit)**

Run: `( cd eval && uv run tier-a --matrix-only --allow-stale-sut )`
Expected: all `ok` except `python/inherited_override: expected_gap`; `python/from_import_alias` now `ok`; 0 regressions.

- [ ] **Step 4: Tier-A `--quick` before review (codex F4 — CLAUDE.md requires it for `src/navigation/` changes)**

Run: `( cd eval && uv run tier-a --quick --allow-stale-sut )`
Expected: M2 dogfood P unchanged, fp=0, 0 regressions. (The change is Python-alias-isolated and the Rust/Go matrix is byte-identical, so this is a formality — but CLAUDE.md mandates `--quick` for navigation/call-resolution changes before review, so run it. May be deferred until just before opening the PR per owner's "merge before coverage settles.")

- [ ] **Step 5: Commit**

```bash
git add src/navigation/call_resolve.rs tests/navigation/callers_test.rs eval/fixtures/python/from_import_alias/expected.toml
# add eval/tests/test_matrix.py ONLY if changed in Task 4 Step 2
git commit -m "fix(nav): resolve aliased-import call sites in callers queries

callers index is keyed by syntactic callee name, so `from m import f as g; g()`
lived under key 'g' and was invisible to callers(f) even though forward
resolution (R4c) already linked it. scoped_caller_sites now also gathers
local-alias sites via import_bindings; direct_callers' per-site identity
re-resolution backstops soundness. Query-layer only; no CACHE_VERSION change.
Flips Tier-A python/from_import_alias known_fail -> pass.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

NEVER `git add -a`; NEVER stage `eval/snapshots/` or `docs/eval/`.

---

## Self-Review

- **Spec coverage:** Root cause (name-keyed callers index) → Task 2. Soundness backstop → Task 3. Tier-A flip → Task 4. Regression gate → Task 5. ✓
- **No cache bump:** Query-layer only; `cg.callers`/`import_bindings` contents and serialized format unchanged. ✓
- **Consumer safety:** Both `scoped_caller_sites` consumers (`collision_dropped_sites`, `direct_callers`) re-resolve every site, so widened candidates are always identity-filtered. ✓
- **Language scope:** Fix is language-neutral (driven by `import_bindings`), but only helps where forward resolution succeeds = Python (R4c is Python-gated). JS/TS aliases stay unresolved (no false claims) until R4c opens to them. ✓
- **Type consistency:** `ImportBindingKind::MemberImport`, `ImportBinding{local,member,eligible,kind}`, `CallSite.caller.file` all match `src/call_graph.rs`. ✓
