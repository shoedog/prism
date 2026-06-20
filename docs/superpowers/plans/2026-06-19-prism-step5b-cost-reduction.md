# Prism Step 5b Serial-Cost Reduction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or executing-plans to implement task-by-task. Steps use checkbox (`- [ ]`) syntax. Execution = the codex implement(high)/review(xhigh) loop (gpt-5.5).

**Goal:** Cut `assemble_graph` Step 5b serial cost to close the gate-9 cold-build ratio gap — **Slice 1: per-callee param-name memoization** (compute-only, behavior-preserving); **Slice 2: parallelize Step 5b**, *gated* on a re-measure.

**Architecture:** Step 5b recomputes a callee's param-name list once per call site (`all_functions()` Node reconstruction — 10.9× redundant on hugo). Slice 1 caches it per `(callee.file, name, start_line)` as `Option<Vec<String>>` (None ⇒ the current `else { continue }` skip). Edges and their insertion order are untouched ⇒ byte-identical cache + Tier-A. Slice 2 (if needed) moves the compute to `rayon` with an ordinal-sorted serial apply, guarded by a serial-reference edge-order oracle.

**Tech Stack:** Rust, `std::collections::BTreeMap`, `rayon` (Slice 2 only).

**Design of record:** `docs/superpowers/specs/2026-06-19-prism-step5b-cost-reduction-design.md` (rev 2, PLAN-READY).

**Verification-scope override (macOS host):** full `cargo test` / `--test cli` / `--test frameworks` stall at `_dyld_start`. Use `cargo test --lib`, `cargo test --test integration <filter>`, `cargo test --test infra <filter>`, `cargo fmt`, `cargo clippy -p prism --lib`, `cargo build -p prism`. The orchestrator runs Tier-A + perf.

---

## File structure

- **`src/cpg/build.rs`** — Slice 1: extract `compute_param_names` + the per-callee memo in Step 5b. Slice 2: the compute→sorted-apply refactor.
- **Test module** (`src/cpg/tests.rs` or a `#[cfg(test)] mod` in `build.rs`) — Slice 1 unit test for `compute_param_names`.
- **`tests/infra/parallel_equality_test.rs`** — Slice 2 only: extend (thread-count + cache-byte parity) + the serial-reference edge-order oracle.
- No other files.

---

## Slice 1 — per-callee param-name memoization

### Task 1: Extract `compute_param_names` and memoize it in Step 5b

**Files:**
- Modify: `src/cpg/build.rs` (Step 5b `assemble_graph` ~:428–500)
- Test: the cpg test module

- [ ] **Step 1: Write the failing unit test for the extracted helper**

Add to the cpg test module. It pins the extraction's correctness (free fn → all params; Python `self`-method → `self` stripped; callee not found → `None`). References `compute_param_names`, which does not yet exist → compile-fail red.

```rust
#[test]
fn compute_param_names_pins_current_behavior() {
    use crate::ast::ParsedFile;
    use crate::call_graph::FunctionId;
    use crate::languages::Language;

    // Free function: all params, no self/cls stripping.
    let go = ParsedFile::parse("t.go", "func f(a int, b int) { _ = a }", Language::Go).unwrap();
    let fid = FunctionId { file: "t.go".into(), name: "f".into(), start_line: 1, end_line: 1 };
    assert_eq!(
        compute_param_names(&go, &fid),
        Some(vec!["a".to_string(), "b".to_string()])
    );

    // Python method with a self receiver + owner: self is stripped.
    let py = ParsedFile::parse(
        "t.py",
        "class C:\n    def m(self, x):\n        return x\n",
        Language::Python,
    )
    .unwrap();
    let mid = FunctionId { file: "t.py".into(), name: "m".into(), start_line: 2, end_line: 3 };
    assert_eq!(compute_param_names(&py, &mid), Some(vec!["x".to_string()]));

    // Callee FunctionInfo not found → None (the `else { continue }` path).
    let missing = FunctionId { file: "t.go".into(), name: "nope".into(), start_line: 99, end_line: 99 };
    assert_eq!(compute_param_names(&go, &missing), None);
}
```

Run: `cargo test --lib compute_param_names_pins_current_behavior`
Expected: **FAIL to compile** — `compute_param_names` does not exist.

- [ ] **Step 2: Extract `compute_param_names` (verbatim current logic → `Option<Vec<String>>`)**

Add a module-private `pub(crate) fn` in `src/cpg/build.rs`. This is the current `:446–500` logic — the `info` find (now `?` → `None`), the `all_functions()` param-occurrence discovery with the `info.param_names` fallback, and the Python `self`/`cls` slice gate — returning an **owned** `Option<Vec<String>>`.

```rust
/// The normalized parameter-name list for a resolved callee, as Step 5b computes it.
/// Pure function of `(callee.file, callee.name, callee.start_line)` + the immutable
/// `callee_parsed` — memoizable per callee. `None` mirrors the current
/// `let Some(info) = … else { continue }` (callee FunctionInfo not found → skip the site).
pub(crate) fn compute_param_names(
    callee_parsed: &ParsedFile,
    callee_id: &FunctionId,
) -> Option<Vec<String>> {
    let info = callee_parsed.functions().iter().find(|f| {
        f.name.as_deref() == Some(callee_id.name.as_str()) && f.start_line == callee_id.start_line
    })?;
    // S3 (spec §3.3): a Python METHOD's self/cls receiver never binds to an explicit
    // call arg. Gate on actual ownership — a free function whose first param merely
    // happens to be named `self` must keep all its params.
    let normalized: Vec<String> = callee_parsed
        .all_functions()
        .into_iter()
        .find(|node| {
            callee_parsed
                .language
                .function_name(node)
                .map(|name| callee_parsed.node_text(&name) == callee_id.name.as_str())
                .unwrap_or(false)
                && callee_parsed.node_line_range(node).0 == callee_id.start_line
        })
        .map(|node| {
            callee_parsed
                .function_parameter_occurrences(&node)
                .into_iter()
                .map(|(name, _, _)| name)
                .collect()
        })
        .unwrap_or_else(|| info.param_names.clone());
    let final_names = match normalized.first().map(String::as_str) {
        Some("self") | Some("cls")
            if info.owner.is_some()
                && callee_parsed.language == crate::languages::Language::Python =>
        {
            normalized[1..].to_vec()
        }
        _ => normalized,
    };
    Some(final_names)
}
```

Run: `cargo test --lib compute_param_names_pins_current_behavior`
Expected: PASS.

- [ ] **Step 3: Replace the per-site computation in Step 5b with the memo**

In `assemble_graph` Step 5b (`src/cpg/build.rs`), add the cache before the `for (caller_id, sites) in &cg.calls` loop:

```rust
        let mut param_cache: std::collections::BTreeMap<(String, String, usize), Option<Vec<String>>> =
            std::collections::BTreeMap::new();
```

Then DELETE the current `let Some(info) = … else { continue };` block (`:446–451`), the `let normalized_param_names: Vec<String> = …;` block (`~:455–476`), and the `let param_names = match … self/cls … ;` block (`~:477–485`), replacing all three with:

```rust
                    let cache_key =
                        (callee_id.file.clone(), callee_id.name.clone(), callee_id.start_line);
                    let param_names: &[String] = match param_cache
                        .entry(cache_key)
                        .or_insert_with(|| compute_param_names(callee_parsed, &callee_id))
                    {
                        Some(names) => names.as_slice(),
                        None => continue,
                    };
```

Everything after (`for (i, param_name) in param_names.iter().enumerate()` and the `param_idx` lookup + `graph.add_edge`) is **unchanged**.

> Note: `callee_parsed` is the `&ParsedFile` already bound at `:457` (`files.get(&callee_id.file)`); `callee_id` is the owned `resolved.target` at `:447` (the key clones its fields, `&callee_id` is borrowed by the closure). The arg-text extraction (`call_argument_texts_at`, `:452`) stays *before* this block (it gates the early `continue` on empty args, unchanged).

- [ ] **Step 4: Verify behavior-preserving + no new clippy**

```bash
cargo test --lib
cargo test --test integration core_test::
cargo fmt && cargo fmt --check
cargo clippy -p prism --lib
cargo build -p prism
```
Expected: all green; no new clippy warnings naming `compute_param_names`/`param_cache`.

- [ ] **Step 5: Commit**

```bash
git add src/cpg/build.rs src/cpg/tests.rs
git commit -F - <<'EOF'
perf(step5b): memoize per-callee param-names in interprocedural DFG assembly

Step 5b recomputed a callee's param-name list once per call site
(all_functions() Node reconstruction — hugo 35898 calls / 3302 distinct,
~11x redundant). Cache it per (callee file, name, start_line) as
Option<Vec<String>> (None = FunctionInfo-not-found, the existing
`else { continue }` skip). Behavior-preserving: edges + insertion order
unchanged -> byte-identical cache + Tier-A. compute_param_names is the
verbatim prior logic; a unit test pins free-fn / Python-self / not-found.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
```

---

## Gate: re-measure (orchestrator — no code)

- [ ] **Re-measure after Slice 1** — build `--release`; cold `nav --no-cache call-stats` on prism/tokio/hugo, this branch vs `main` (`f6499f8`). Record cold wall-time + cold-hugo user/wall.
  - **Tier-A `--matrix-only --allow-stale-sut`: 0 regressions** (hard gate — behavior-preserving).
  - **Decision:** if cold-hugo user/wall **≥ 1.5** → gate-9 met; **STOP** (Slice 2 unnecessary — record in the PR, ship Slice 1 alone). If **< 1.5** → proceed to Slice 2.

---

## Slice 2 — parallelize Step 5b (CONDITIONAL; detailed only if the gate triggers)

**Do not implement unless the re-measure gate selects it.** Design = spec §3.2/§4. Outline:

- [ ] **S2.1 — serial-reference edge-order oracle (write first).** Capture the Step-5b `DataFlow`-edge sequence produced by the *current serial* loop as a `#[cfg(test)]` reference (the S1.5 frozen-oracle pattern), over a Step-5b-heavy fixture corpus. This is the cutover guard (proves parallel == old serial order, which `parallel_equality_test` alone does not).
- [ ] **S2.2 — precompute `param_cache` read-only** (serial pre-pass over distinct callees → shared `&BTreeMap`), since `or_insert_with` `&mut` is not `par_iter`-safe.
- [ ] **S2.3 — parallel compute → ordinal-sorted serial apply.** Enumerate jobs with `(caller_ord, site_ord)`; `rayon` map each job → `Vec<Edge { caller_ord, site_ord, resolved_ord, param_ord, from, to }>` (read-only over `&cg`/`&files`/`&var_index`/`&param_cache`); barrier; sort by the 4 ordinals; serial `graph.add_edge` in that order.
- [ ] **S2.4 — parity + gates.** Extend `tests/infra/parallel_equality_test.rs` (thread-count determinism + cache-byte parity); the S2.1 oracle green; Tier-A 0 regressions; re-measure gate-9.

When triggered, this section is expanded into full TDD tasks (and re-run through plan-review) before implementation.

---

## Self-review

- **Spec coverage:** §3.1 memo (Task 1, `Option<Vec<String>>` + None-skip), §3.2 parallelize (Slice 2 outline), §4 determinism (Slice 1 trivially edge-order-safe; Slice 2 oracle + parity), §5 gates (Task 4 + the re-measure gate), §1 profile (the gate criteria). Covered.
- **Placeholder scan:** Slice 2 is an intentional conditional outline (gated; expanded only if triggered) — not a placeholder for committed work. Slice 1 is fully specified.
- **Type consistency:** `compute_param_names(&ParsedFile, &FunctionId) -> Option<Vec<String>>`, `param_cache: BTreeMap<(String,String,usize), Option<Vec<String>>>`, `param_names: &[String]` used consistently.
- **TDD note:** Task 1 Step 1 is a genuine compile-fail red (helper absent); Step 2 greens it; Step 3 is a behavior-preserving refactor guarded by the existing suite + Tier-A (Step 4). The memo's correctness = the helper unit test + the purity argument (pure fn of the key) + Tier-A 0-regression.
