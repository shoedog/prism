# Parallelize `rematerialize_rust_receiver_keys` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Parallelize Phase 1 (read-only Rust receiver re-typing) of `CallGraph::rematerialize_rust_receiver_keys` with **byte-identical** output — Phase 1 is ~99% of the pass (prism 2.70s / tokio 1.41s; hugo no-op).

**Architecture:** Extract Phase 1 into a pure `compute_rust_receiver_updates(&self, files) -> Vec<(FunctionId, CallSite, Option<ReceiverOutcome>)>` (a per-caller helper + an order-preserving collect over the `self.calls`-ordered caller list), then `par_iter` it through the `Sync` `RustReceiverTyper`. Phase 2 (the `&mut self` apply) is unchanged. A `#[cfg(test)]` verbatim serial reference is the old-order oracle; byte-identity end-to-end rides the existing cache-byte gate (which runs remat on `src/cpg`).

**Tech Stack:** Rust, `rayon` (shared global pool), the in-crate `#[cfg(test)] mod tests` at `src/call_graph.rs:2032`.

**Design-of-record:** `docs/superpowers/specs/2026-06-20-prism-rematerialize-receiver-parallel-design.md` (rev 2, PLAN-READY).

**Verification scope (macOS):** bare `cargo test` / `--test cli` / `--test frameworks` stall at `_dyld_start`. Use `cargo build -p prism`, `cargo test --lib` (full unit suite, in-process — includes `call_graph` tests), `cargo test --test infra <filter>`, `cargo fmt`/`--check`, `cargo clippy -p prism --lib`. `cd eval && uv run tier-a --matrix-only --allow-stale-sut` is Python/uv (no stall).

**Per-task boundary gate (every code task, before commit):** `cargo build -p prism && cargo test --lib` all green.

**Measured grounding (instrumented, reverted):** Phase 1 = prism 2.695s / tokio 1.411s (~99%); Phase 2 = 34/16ms; hugo no-op. `scope_graph=true` for real-dir loads: `src/navigation` → 397 updates (43 callers), `src/cpg` → 1251 updates (154 callers). `RustReceiverTyper` = `&CallGraph` + `&ScopeGraph`, `type_of_receiver(&self)`, zero interior mutability, per-call scratch local → `Sync`. `RustReceiverTyper::new` **panics** if `scope_graph` is `None`, so `compute_*` is only called after the `scope_graph.is_none()` early return (and the test corpora populate it).

---

## Task 1: Extract Phase 1 (inert serial) + old-order oracle + non-vacuity guard

**Files:**
- Modify: `src/call_graph.rs` — extract `compute_rust_receiver_updates` (serial) + `receiver_updates_for_caller` + `#[cfg(test)] compute_rust_receiver_updates_reference`; rewire `rematerialize_rust_receiver_keys` Phase 1 to call it.
- Modify: `src/call_graph.rs` `#[cfg(test)] mod tests` (`:2032`) — add the oracle + the non-vacuity guard.

- [ ] **Step 1: Write the failing oracle + guard tests** in the `mod tests` block (`src/call_graph.rs:2032`):

```rust
#[test]
fn rust_receiver_updates_parallel_matches_serial_reference() {
    // Real-dir load → populated scope_graph (in-memory ad-hoc Rust fixtures have no
    // crate root, so RustReceiverTyper::new would panic). src/navigation: 397 updates.
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/navigation");
    let repo = crate::repo_loader::load_repo(&dir).unwrap();
    let ctx = crate::cpg::CpgContext::build(&repo.files, None);
    let cg = &ctx.cpg.call_graph;
    let par = cg.compute_rust_receiver_updates(&repo.files);
    let serial = cg.compute_rust_receiver_updates_reference(&repo.files);
    assert_eq!(par, serial, "receiver update sequence diverged");
    assert!(!par.is_empty(), "fixture produced no receiver updates");
}

#[test]
fn rematerialize_corpus_has_rust_receiver_updates() {
    // Floors the cache-byte gate's corpus (src/cpg) so it actually exercises remat.
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cpg");
    let repo = crate::repo_loader::load_repo(&dir).unwrap();
    let ctx = crate::cpg::CpgContext::build(&repo.files, None);
    let n = ctx.cpg.call_graph.compute_rust_receiver_updates(&repo.files).len();
    assert!(n > 50, "too few receiver updates to surface a remat divergence: {n}"); // measured 1251
}
```

- [ ] **Step 2: Run to verify it fails to compile** (the methods don't exist yet):

Run: `cargo test --lib rust_receiver_updates_parallel_matches_serial_reference`
Expected: FAIL — `no method named compute_rust_receiver_updates`.

- [ ] **Step 3: Extract Phase 1 (inert serial) + reference twin** in `src/call_graph.rs`.

Add, near `rematerialize_rust_receiver_keys` (the production collect is **serial** in this task — `iter().copied()`):

```rust
/// Phase 1 of receiver re-typing: read-only per-caller receiver outcomes,
/// collected over the `self.calls`-ordered caller list. Serial here (Task 1);
/// parallelized in Task 2. Only valid when `self.scope_graph` is `Some`
/// (`RustReceiverTyper::new` requires it) — the caller guards that.
pub(crate) fn compute_rust_receiver_updates(
    &self,
    files: &BTreeMap<String, ParsedFile>,
) -> Vec<(
    FunctionId,
    CallSite,
    Option<crate::resolution_identity::ReceiverOutcome>,
)> {
    let typer = crate::resolution_receiver::RustReceiverTyper::new(self);
    let ordered: Vec<(&FunctionId, &BTreeSet<CallSite>)> = self.calls.iter().collect();
    ordered
        .iter()
        .copied() // (&FunctionId, &BTreeSet) is Copy → avoid &&-destructuring
        .map(|(caller, sites)| Self::receiver_updates_for_caller(caller, sites, &typer, files))
        .collect::<Vec<Vec<_>>>()
        .into_iter()
        .flatten()
        .collect()
}

/// Per-caller receiver outcomes — verbatim semantics of the original Phase-1
/// inner body. Caller-level skips return the empty `out`; site-level skips
/// `continue` (exactly the original `continue` placements).
fn receiver_updates_for_caller(
    caller: &FunctionId,
    sites: &BTreeSet<CallSite>,
    typer: &crate::resolution_receiver::RustReceiverTyper<'_>,
    files: &BTreeMap<String, ParsedFile>,
) -> Vec<(
    FunctionId,
    CallSite,
    Option<crate::resolution_identity::ReceiverOutcome>,
)> {
    let mut out = Vec::new();
    let Some(parsed) = files.get(&caller.file) else {
        return out;
    };
    if !matches!(parsed.language, crate::languages::Language::Rust) {
        return out;
    }
    let Some(fn_node) = Self::function_node_for_id(parsed, caller) else {
        return out;
    };
    let all_lines: BTreeSet<usize> = (caller.start_line..=caller.end_line).collect();
    let ast_calls = parsed.function_calls_with_qualifier_and_spans_on_lines(&fn_node, &all_lines);
    for site in sites {
        let Some((_, _, qualifier, start_byte, _, receiver_expr, _, _)) = ast_calls
            .iter()
            .find(|(callee_name, _, _, start_byte, end_byte, _, _, _)| {
                callee_name == &site.callee_name
                    && *start_byte == site.start_byte
                    && *end_byte == site.end_byte
            })
        else {
            continue;
        };
        if receiver_expr.is_none() && qualifier.is_none() {
            continue;
        }
        let outcome = typer.type_of_receiver(crate::resolution_receiver::ReceiverTypeCtx {
            parsed,
            caller,
            fn_node,
            receiver_expr: *receiver_expr,
            qualifier: qualifier.as_deref(),
            call_start_byte: *start_byte,
        });
        out.push((caller.clone(), site.clone(), outcome));
    }
    out
}

/// Frozen verbatim copy of the ORIGINAL Phase-1 loop, returning the same Vec.
/// The old-order oracle compares the production collect against this.
#[cfg(test)]
pub(crate) fn compute_rust_receiver_updates_reference(
    &self,
    files: &BTreeMap<String, ParsedFile>,
) -> Vec<(
    FunctionId,
    CallSite,
    Option<crate::resolution_identity::ReceiverOutcome>,
)> {
    let mut updates = Vec::new();
    let typer = crate::resolution_receiver::RustReceiverTyper::new(self);
    for (caller, sites) in &self.calls {
        let Some(parsed) = files.get(&caller.file) else {
            continue;
        };
        if !matches!(parsed.language, crate::languages::Language::Rust) {
            continue;
        }
        let Some(fn_node) = Self::function_node_for_id(parsed, caller) else {
            continue;
        };
        let all_lines: BTreeSet<usize> = (caller.start_line..=caller.end_line).collect();
        let ast_calls =
            parsed.function_calls_with_qualifier_and_spans_on_lines(&fn_node, &all_lines);
        for site in sites {
            let Some((_, _, qualifier, start_byte, _, receiver_expr, _, _)) = ast_calls
                .iter()
                .find(|(callee_name, _, _, start_byte, end_byte, _, _, _)| {
                    callee_name == &site.callee_name
                        && *start_byte == site.start_byte
                        && *end_byte == site.end_byte
                })
            else {
                continue;
            };
            if receiver_expr.is_none() && qualifier.is_none() {
                continue;
            }
            let outcome = typer.type_of_receiver(crate::resolution_receiver::ReceiverTypeCtx {
                parsed,
                caller,
                fn_node,
                receiver_expr: *receiver_expr,
                qualifier: qualifier.as_deref(),
                call_start_byte: *start_byte,
            });
            updates.push((caller.clone(), site.clone(), outcome));
        }
    }
    updates
}
```

Now replace `rematerialize_rust_receiver_keys`'s Phase 1 (the `let mut updates = …; { let typer = …; for (caller, sites) in &self.calls { … } }` block, originally `:1128`–`:1174`) with a single call, keeping Phase 2 verbatim:

```rust
fn rematerialize_rust_receiver_keys(&mut self, files: &BTreeMap<String, ParsedFile>) {
    if self.scope_graph.is_none() {
        return;
    }
    let updates = self.compute_rust_receiver_updates(files);
    for (caller, old_site, outcome) in updates {
        // ... existing Phase 2 body verbatim (take+insert in self.calls; in-place
        //     receiver_outcome overwrite in self.callers) ...
    }
}
```

- [ ] **Step 4: Boundary gate — build + full lib suite GREEN:**

Run: `cargo build -p prism && cargo test --lib`
Expected: PASS — `rust_receiver_updates_parallel_matches_serial_reference` (serial collect == reference) and `rematerialize_corpus_has_rust_receiver_updates` (>50) green; no other lib test regressed (esp. the existing `rust/*` receiver behavior).

- [ ] **Step 5: fmt + clippy:**

Run: `cargo fmt && cargo clippy -p prism --lib 2>&1 | rg "compute_rust_receiver|receiver_updates_for_caller" || echo "no new warnings on touched fns"`
Expected: no new warnings on the added functions.

- [ ] **Step 6: Commit:**

```bash
git add src/call_graph.rs
git commit -m "refactor(remat): extract compute_rust_receiver_updates + old-order oracle (inert serial)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: Parallelize Phase 1

**Files:**
- Modify: `src/call_graph.rs` (`compute_rust_receiver_updates`: `iter` → `par_iter`)

- [ ] **Step 1: Flip the collect to `par_iter`.** In `compute_rust_receiver_updates` add `use rayon::prelude::*;` at the top of the fn and change `ordered.iter().copied()` to `ordered.par_iter().copied()`. Update the doc comment to drop "serial here (Task 1)" and describe the parallel collect over ordered caller units + the shared `Sync` typer. `receiver_updates_for_caller` is unchanged (it only reads `&typer` (`Sync`) + `files` (`Sync`) and returns owned data — sound concurrent).

```rust
pub(crate) fn compute_rust_receiver_updates(
    &self,
    files: &BTreeMap<String, ParsedFile>,
) -> Vec<(FunctionId, CallSite, Option<crate::resolution_identity::ReceiverOutcome>)> {
    use rayon::prelude::*;
    let typer = crate::resolution_receiver::RustReceiverTyper::new(self);
    let ordered: Vec<(&FunctionId, &BTreeSet<CallSite>)> = self.calls.iter().collect();
    ordered
        .par_iter()
        .copied()
        .map(|(caller, sites)| Self::receiver_updates_for_caller(caller, sites, &typer, files))
        .collect::<Vec<Vec<_>>>()
        .into_iter()
        .flatten()
        .collect()
}
```

- [ ] **Step 2: Boundary gate — build + full lib suite (oracle now par vs serial-reference):**

Run: `cargo build -p prism && cargo test --lib`
Expected: PASS — `rust_receiver_updates_parallel_matches_serial_reference` proves the parallel collect reproduces the exact serial-reference `Vec`; no other lib test regressed.

- [ ] **Step 3: End-to-end byte-identity gate (the existing cache-byte test runs remat on src/cpg):**

Run: `cargo test --test infra cache_blob_bytes_identical_serial_vs_parallel && cargo test --test infra cpg_build_parallel_matches_serial_reference_in_order`
Expected: PASS — `cpg-cache.bin` bytes identical default-Rayon vs 1-thread (corpus2 = src/cpg, which runs `rematerialize_rust_receiver_keys`).

- [ ] **Step 4: fmt + commit:**

```bash
cargo fmt
git add src/call_graph.rs
git commit -m "perf(remat): parallelize Rust receiver re-typing (par_iter over callers)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: Acceptance gates (orchestrator-run — Tier-A + perf)

These are run by the orchestrator (measurement, not code); not a codex task.

- [ ] **Step 1: Tier-A 0-regression gates.** Per AGENTS.md, a `src/call_graph.rs` change runs BOTH `--matrix-only`
  (fast, pre-commit) AND `--quick` (rust-analyzer LSP oracle, pre-review) — receiver typing drives Rust dispatch
  resolution, so the `--quick` dogfood matters here.

Run: `cargo build --release -p prism && (cd eval && uv run tier-a --matrix-only --allow-stale-sut)`
Expected: 0 regressions (every fixture `ok` or `expected_gap`; the `rust/*` receiver fixtures unchanged).

Then (pre-review): `cd eval && uv run tier-a --quick --allow-stale-sut`
Expected: 0 regressions vs baseline (M2 dogfood precision/recall unchanged). If rust-analyzer is unavailable in
the environment, record that honestly rather than skipping silently. Paste any flip-candidates into the PR
description (do not re-baseline).

- [ ] **Step 2: Perf gate (the reward — report, don't assert).**

```bash
BIN=target/release/prism
for repo in ~/code/bench-repos/tokio /Users/wesleyjinks/code/slicing; do
  echo "== $repo =="
  for i in 1 2 3; do /usr/bin/time -p $BIN nav --no-cache call-stats --repo "$repo" >/dev/null; done
done
```

Compare min-of-N wall-clock vs `3cb0182` (this branch's base). Expected: prism ~−2.0–2.3s, tokio ~−1.0–1.2s, hugo ~0 (no-op — skip or confirm flat). **No regression** on any corpus is the gate. Record numbers for the PR body. (No `CACHE_VERSION` bump — bytes identical.)

---

## Self-Review notes (cross-checked against spec rev 2)

- **Spec coverage:** §2 scope (Phase 1 only) → Tasks 1–2; §4 extract + `par_iter` + Phase 2 unchanged → Task 1/2; §5 soundness (`Sync` typer, owned return, `&self`→`&mut self` ordering) → the helper signature + the `compute`→Phase-2 sequencing; §6.1 old-order oracle (src/navigation) + §6.2 cache-byte gate + non-vacuity guard (src/cpg) → Task 1 tests + Task 2 step 3; §6.3 + §7 → Task 3.
- **No placeholders:** the only "… verbatim …" is the Phase-2 apply body, which is left **unchanged in place** (not rewritten) — call that out to the implementer: do NOT retype Phase 2, only replace Phase 1 with the `compute_*` call.
- **Type consistency:** the `Vec<(FunctionId, CallSite, Option<ReceiverOutcome>)>` return type is identical across `compute_rust_receiver_updates`, `receiver_updates_for_caller` (returns the per-caller slice of it), and `_reference`. `compute_*` signature stable across Task 1 (serial) → Task 2 (only `iter`→`par_iter`).
- **Panic-safety:** `compute_*` calls `RustReceiverTyper::new` (panics on `scope_graph == None`); guarded by the `scope_graph.is_none()` early return in `rematerialize_rust_receiver_keys`, and the test corpora (src/navigation, src/cpg real-dir loads) populate the scope graph (measured `scope_graph=true`).
- **Byte-identity basis:** order is NOT load-bearing (sorted `calls`; in-place-overwritten `callers` Vec); the per-site `receiver_outcome` values are deterministic from `type_of_receiver`. Oracle preserves order anyway (order-preserving collect) for strict `Vec` equality.
