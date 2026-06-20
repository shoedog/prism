# Parallelize `rematerialize_rust_receiver_keys` (Rust receiver re-typing) — Design

**Date:** 2026-06-20
**Status:** DRAFT (pre spec-review)
**Branch:** `rematerialize-parallel` (off main `3cb0182`)
**Predecessors:** the assemble perf arc — S1.5 (#111) · Step-5b memo (#112) · Step-7 (#114) · assemble edge-steps 5+5b+8 (branch `step8-varnode-parallel`, complete, unmerged)

## 1. Motivation

After the assemble edge-steps slice, the next contained cold-build lever is the
**Rust receiver re-typing post-pass** `CallGraph::rematerialize_rust_receiver_keys`
(`src/call_graph.rs:1123`), run once during `build_with_scope_graph_inputs`
(`:995`, via `refresh_rust_receiver_state` `:1105`). Measured (instrumented, cold
`nav --no-cache call-stats`):

| corpus | Phase 1 (compute) | Phase 2 (apply) | updates |
|---|---|---|---|
| **prism** | **2.695s (98.7%)** | 34ms | 26,263 |
| **tokio** | **1.411s (98.9%)** | 16ms | 18,619 |
| **hugo** | — (Go: no measured cost — see below) | — | — |

Phase 1 — per Rust caller: one AST query (`function_calls_with_qualifier_and_spans_on_lines`)
plus per call site `RustReceiverTyper::type_of_receiver` — is ~99% of the cost and
is **read-only** (`&self` via the typer). This is effectively **Rust-only**: the pass
early-returns when `scope_graph.is_none()` (`:1124`; the case hit in the hugo build
measured here — no `[remat]` output), and *even when* a build path supplies scope
inputs, the **per-caller Rust language guard** (`:1135`–`:1140`) skips every non-Rust
caller before any AST/type work. Either way Go pays ~0.

## 2. The function is already compute-then-apply

`rematerialize_rust_receiver_keys` already has the S1 C2 shape:

- **Phase 1 (`:1133`–`:1174`)** — a `{ }` block: `let typer = RustReceiverTyper::new(self)`
  (borrows `&self`), iterate `self.calls`, build `updates: Vec<(FunctionId, CallSite,
  Option<ReceiverOutcome>)>`. Pure read of `&self` + `files`.
- **Phase 2 (`:1176`–`:1191`)** — `for (caller, old_site, outcome) in updates`:
  mutate `self.calls` (take+insert the site with the new `receiver_outcome`) and
  `self.callers`. `&mut self`. Cheap (34ms / 16ms).

So this slice only parallelizes Phase 1's `for (caller, sites) in &self.calls` loop.

## 3. The byte-identity invariant (DIFFERENT from the edge steps)

The edge steps' load-bearing risk was *edge-insertion order* = cache bytes. Here it
is **not order** — it is the **per-site `receiver_outcome` values**:

- Phase 2 writes each site's `receiver_outcome` into two structures, **neither of
  which Phase 2 reorders**:
  - `self.calls: BTreeMap<FunctionId, BTreeSet<CallSite>>` — sorted; `CallSite`'s
    `Ord`/`cmp_key` does **not** include `receiver_outcome` (`call_graph.rs:1983-2018`;
    matched by `cmp_key` at `:1186`), so take+insert keeps the site's sorted position.
  - `self.callers: BTreeMap<String, Vec<CallSite>>` — a **`Vec` value** with a
    preexisting order set during `CallGraph` construction (not by this pass); Phase 2
    **overwrites `site.receiver_outcome` in place** (`:1184`–`:1190`), never appending
    or reordering, so the `Vec` order is unchanged regardless of apply order.
  The serialized `CallGraph` (cache bincodes a cloned `CallGraph` directly,
  `cpg_cache.rs:206-223`) and the downstream CPG edges (since `resolve_call_site`
  reads `receiver_outcome`) are therefore a pure function of the **set of (site →
  outcome)** mappings, *independent of compute/apply order*.
- `type_of_receiver` is **deterministic** (a pure function of `(parsed, caller,
  fn_node, receiver_expr, qualifier, call_start_byte)` + immutable `&self`; `self`
  is not mutated until Phase 2). So parallel Phase 1 produces the **identical
  per-site outcomes** as serial Phase 1 ⇒ identical `CallGraph` ⇒ identical CPG ⇒
  **byte-identical cache**.

Order is thus *not* load-bearing — but we still **preserve it** (order-preserving
`par_iter` collect over the `self.calls`-ordered caller list) so the old-order
oracle is a strict `Vec` equality and the change is provably a pure restructure.

## 4. Design: parallelize Phase 1

Extract Phase 1 into a read-only method; flip its caller loop to `par_iter`:

```rust
fn rematerialize_rust_receiver_keys(&mut self, files: &BTreeMap<String, ParsedFile>) {
    if self.scope_graph.is_none() {
        return;
    }
    let updates = self.compute_rust_receiver_updates(files); // &self; owned Vec
    for (caller, old_site, outcome) in updates {             // Phase 2 unchanged (&mut self)
        // ... existing take+insert + callers update ...
    }
}

/// Phase 1: read-only per-caller receiver re-typing. Parallel over the
/// `self.calls`-ordered caller list; order-preserving collect.
fn compute_rust_receiver_updates(
    &self,
    files: &BTreeMap<String, ParsedFile>,
) -> Vec<(FunctionId, CallSite, Option<crate::resolution_identity::ReceiverOutcome>)> {
    use rayon::prelude::*;
    let typer = crate::resolution_receiver::RustReceiverTyper::new(self);
    let ordered: Vec<(&FunctionId, &BTreeSet<CallSite>)> = self.calls.iter().collect();
    ordered
        .par_iter()
        .copied() // the (&FunctionId, &BTreeSet) tuple is Copy → avoid &&-destructuring
        .map(|(caller, sites)| {
            let mut out = Vec::new();
            // ... the VERBATIM per-caller body (files.get / Rust-lang guard /
            //     function_node_for_id / ast_calls / per-site find + type_of_receiver),
            //     pushing (caller.clone(), site.clone(), outcome) to `out` ...
            out
        })
        .collect::<Vec<Vec<_>>>() // order-preserving over `ordered`
        .into_iter()
        .flatten()
        .collect()
}
```

Borrow structure: `compute_rust_receiver_updates(&self, …)` returns an **owned**
`Vec` (all `caller.clone()`/`site.clone()` + owned `ReceiverOutcome`), so the `&self`
borrow (and the `typer`/`ordered` borrows of `self`) end before Phase 2's `&mut self`
— exactly as the current `{ }`-block structure already arranges. The
`scope_graph.is_none()` early return stays in `rematerialize_rust_receiver_keys`
(the typer needs `scope_graph`), so `compute_*` is only called when it is `Some`.

## 5. Soundness under `par_iter` (audited)

- `RustReceiverTyper` (`resolution_receiver.rs:31`) holds only `cg: &CallGraph` +
  `graph: &ScopeGraph` (shared immutable refs); `type_of_receiver(&self, …)` is
  read-only; **zero interior mutability** in `resolution_receiver.rs` (no
  `RefCell`/`Cell`/`Mutex`/`RwLock`/`OnceCell`/`OnceLock`/`unsafe`/`static mut`).
  Its per-call scratch (`TypeVisit`, `RecursionCtx`) is created **inside** each
  `type_of_receiver` call → no shared mutable state. ⇒ `&typer` is `Sync`-shareable
  across the parallel closures.
- `CallGraph` + `ScopeGraph` are already `Sync` (the S1 parallel-build work). `files`
  (`&BTreeMap<String, ParsedFile>`) is `Sync` (Step 7/edge steps already `par_iter`
  over `ParsedFile`). The closure captures only shared refs + returns owned data; no
  `&mut self` inside the parallel region (mutation is the serial Phase 2).
- `function_calls_with_qualifier_and_spans_on_lines` and `function_node_for_id` are
  read-only AST reads on the immutable `parsed`.

## 6. Proof of byte-identity

### 6.1 Old-order oracle — Phase-1 compute parity (`src/call_graph.rs` tests)

Freeze the original serial Phase-1 loop as `#[cfg(test)] compute_rust_receiver_updates_reference`
returning the same `Vec`, and assert the production (parallel) compute equals it on
a Rust fixture whose `CallGraph` has a populated `scope_graph`:

```rust
#[test]
fn rust_receiver_updates_parallel_matches_serial_reference() {
    let files = rust_receiver_fixture();              // Rust, method calls w/ receivers
    let ctx = CpgContext::build(&files, None);         // builds scope_graph + runs remat
    let cg = &ctx.cpg.call_graph;                       // scope_graph is Some here
    let par = cg.compute_rust_receiver_updates(&files);
    let serial = cg.compute_rust_receiver_updates_reference(&files);
    assert_eq!(par, serial, "receiver update sequence diverged");
    assert!(!par.is_empty(), "fixture produced no receiver updates");
}
```

`ReceiverOutcome` derives `Debug, Clone, PartialEq, Eq` (`resolution_identity.rs:22`)
and `FunctionId`/`CallSite` are `Ord` (BTreeMap/BTreeSet keys ⇒ `PartialEq`), so the
`Vec<(FunctionId, CallSite, Option<ReceiverOutcome>)>` is directly `assert_eq!`-able.
The oracle lives in the existing in-crate `#[cfg(test)] mod tests` (`call_graph.rs:2032`),
so it reaches the `pub(crate)` `compute_rust_receiver_updates(_reference)`. Re-running
`compute_*` on the already-materialized `CallGraph` is valid: `type_of_receiver`
re-derives the outcome from the AST independently of the site's existing
`receiver_outcome`, so both sides observe the identical input snapshot — a pure
par-vs-serial parity check.

### 6.2 End-to-end cache-byte gate (existing `parallel_equality_test.rs`)

`cache_blob_bytes_identical_serial_vs_parallel` builds the full nav cache on
`corpus2()` = **`src/cpg`** (Rust) — which runs `rematerialize_rust_receiver_keys`
— and asserts `cpg-cache.bin` **bytes identical** default-Rayon vs 1-thread. Since
`receiver_outcome` feeds resolution → CPG edges → cache, this is the end-to-end
byte-identity gate for this slice. Add a non-vacuity guard that `src/cpg` actually
produces receiver updates (so the gate isn't hollow):

```rust
#[test]
fn rematerialize_corpus_has_rust_receiver_updates() {
    // Load src/cpg directly — the infra `corpus2()` helper is private to that test
    // file; this guard is in-crate. (src/cpg is the same Rust corpus the cache-byte
    // gate uses, so this floors exactly what that gate exercises.)
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cpg");
    let repo = crate::repo_loader::load_repo(&dir).unwrap();
    let ctx = crate::cpg::CpgContext::build(&repo.files, None);
    let n = ctx.cpg.call_graph.compute_rust_receiver_updates(&repo.files).len();
    assert!(n > 50, "too few receiver updates to surface a remat divergence: {n}");
}
```

This guard lives **in-crate** next to the oracle (`call_graph.rs:2032 mod tests`),
where `compute_rust_receiver_updates` (`pub(crate)`) is reachable — no new public
surface. (`src/cpg` is method-call-heavy Rust; expect well over 50 updates.)

### 6.3 Tier-A backstop

`cd eval && uv run tier-a --matrix-only --allow-stale-sut` — **0 regressions**
(receiver typing drives Rust dispatch resolution; `--matrix-only` covers the
`rust/*` receiver fixtures). Behavioral backstop, not the byte-identity proof.

## 7. Perf gate

Cold `nav --no-cache call-stats`, branch vs `3cb0182`, hugo / tokio / prism.
Expected: prism ~−2.0–2.3s, tokio ~−1.0–1.2s, hugo ~0 (no-op). **No regression**
on any corpus is the gate. No `CACHE_VERSION` bump (bytes identical).

## 8. Risks & mitigations

| risk | mitigation |
|---|---|
| Per-site outcome differs parallel vs serial → cache-byte break | §3: `type_of_receiver` deterministic + `self` immutable in Phase 1; §6.1 old-order oracle + §6.2 cache-byte gate. |
| `RustReceiverTyper`/`type_of_receiver` unsound under `par_iter` | §5: `&self`, zero interior mutability (audited), per-call scratch local, `Sync` refs. |
| Borrow: `&self` compute vs `&mut self` apply | compute returns an owned `Vec`; the `&self` borrow ends before Phase 2 — same as the current `{ }` block. |
| Rayon overhead on tiny/Go corpora | Go pays ~0 (early-return on `scope_graph.is_none()` and/or the per-caller Rust language guard at `:1135`); small Rust repos pay only par_iter setup over a short caller list. |
| `ReceiverOutcome` not `PartialEq` for the oracle | fall back to a `{:?}` debug-dump comparison (§6.1). |

## 9. Files

- `src/call_graph.rs` — extract `compute_rust_receiver_updates` (production, `par_iter`)
  from `rematerialize_rust_receiver_keys`'s Phase 1; add `#[cfg(test)]
  compute_rust_receiver_updates_reference` (frozen serial); Phase 2 unchanged.
- `src/call_graph.rs` `#[cfg(test)] mod tests` (`:2032`) — the old-order oracle + the
  receiver-update non-vacuity guard (both in-crate, reaching `pub(crate)` compute).
  `tests/infra/parallel_equality_test.rs` is unchanged — its existing
  `cache_blob_bytes_identical_serial_vs_parallel` (corpus2 = `src/cpg`, Rust) already
  exercises this pass end-to-end.

## 10. Out-of-scope follow-ups

- **Phase-3 indirect-resolution** parallelization — still needs a structural-containment
  study first (language-agnostic, hugo ~5.3s; the real research question).
- `populate_method_identity_indices` (the other half of `refresh_rust_receiver_state`,
  `:1106`) — not measured as a hotspot; out of scope unless a profile flags it.
