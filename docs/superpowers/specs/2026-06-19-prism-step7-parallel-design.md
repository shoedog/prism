# Prism Step 7 Parallelization (statement-node creation) — Design

**Rev 2 — 2026-06-19. Status: PLAN-READY (codex gpt-5.5 xhigh spec-review folded; design sound, proof gate fixed).**

> **Rev 2 (codex spec-review — FLAWED → fixed; the design/approach was confirmed sound + race-free, the flaw
> was in the proof gate):** 1 BLOCKER + 1 MAJOR + 2 MINORs.
> **BLOCKER:** a pre-refactor "golden `cpg-cache.bin` byte parity" gate is INVALID — the cache serializes
> `git_sha = env!("GIT_SHA")` + build metadata *before* the graph (`cpg_cache.rs:62–88`), so a golden from
> another commit differs even with an identical graph. Fixed: the **same-binary serial-reference oracle** (§4
> gate 1, in-memory graph, no cache bytes) is THE old-order proof; cache-byte parity is **same-build
> serial-vs-parallel** (determinism only). **MAJOR:** `parallel_equality_test` (default vs 1-thread, same
> build) catches scheduler nondeterminism, NOT a consistently-reordered new algorithm — so the oracle is the
> required old-parity gate; thread-count equality is only a determinism gate (stated explicitly). **MINORs:**
> the oracle compares `(file,line)→NodeIndex` ordinals + the final sorted `location_index` buckets (not just
> key sets); the order-contract wording sharpened (emit-on-function-encounter then recurse; whole-file `seen`
> checked *before* classify). Codex confirmed **no race** (classify read-only, query cache `OnceLock`,
> `StmtKind: Send`).

**Goal:** Parallelize `assemble_graph` Step 7 (statement-node creation for the CFG) — the residual serial
dominator (**73–80%** of the post-memo assemble) — via parallel-collect → serial-create, **proving a
byte-identical graph** (node-insertion order is cache-byte-significant).

**One-liner:** Step 7's per-file work (AST walk + `statement_spans_in_function` + `classify_stmt_kind`) is
read-only and embarrassingly parallel; only node creation must stay serial+ordered. A throwaway spike measured
**hugo 14.7→10.7s (−4.0s), tokio 6.4→4.2s (−2.2s)**. This is a node-CREATING step, so — unlike the edge-only
Step 5b — the **load-bearing risk is node-insertion-order parity**, and this design's center is *proving* it.

---

## §1. Context & profile

After S1.5 (call-args index) + the Step-5b per-callee memo (both on `main`), a per-step `assemble_graph` profile
(env-gated `Instant` timers, reverted) shows the residual serial cost concentrated in **one** step:

| step | hugo | tokio |
|---|---|---|
| **Step 7 — statement nodes** | **5.71s (80%)** | **2.76s (73%)** |
| Step 8 — controlflow edges | 0.59s | 0.17s |
| Step 5b — interproc DFG (memo) | 0.41s | 0.45s |
| everything else | <0.5s | ~0.4s |
| assemble TOTAL | 7.18s | 3.80s |

A throwaway `par_iter`-over-files spike (parallel collect → serial node-create) measured **hugo −4.0s
(14.7→10.7s, user/wall 1.53→2.22), tokio −2.2s (6.4→4.2s)** → clear GO. The expensive part is the walk +
`statement_spans_in_function` (`src/ast.rs:3112`) + `classify_stmt_kind` (`src/cpg/build.rs:750`, which calls
`call_names_on_lines` → a tree-sitter query) — all read-only; `graph.add_node` is cheap. Precedent: the S1 C2
pattern (`CallGraph::build` `ordered_files` → `par_iter` → serial flatten, `src/call_graph.rs:455–557`).

End-to-end hugo this perf line: 86.4s (pre-S1.5) → 18.4 (S1.5) → 14.7 (memo) → **~10.7s** with this (**~8×**).

## §2. Scope

**In scope (Slice 1):** parallelize Step 7 (`collect_function_statements`, `src/cpg/build.rs:706`) —
`ordered_files` → `par_iter` collect → serial create. `statement_spans_in_function` and `classify_stmt_kind`
reused **unchanged** (classify runs in the parallel phase; it's read-only + thread-safe, §4).

**Out of scope / non-goals:**
- The other assemble steps (all now small).
- The possible ~2× redundant walk (outer recursion descends into bodies `statement_spans` already covered) —
  a separate, language-aware optimization; not part of this behavior-preserving parallelization.
- **No behavior change:** the `Statement` nodes, their `NodeIndex` order, `stmt_index`, `location_index`,
  Step 8 CFG edges, and the cache bytes are **byte-identical** before/after.

---

## §3. Design — parallel collect → serial create

Replace the serial Step-7 loop with the C2 pattern:

```rust
struct PendingStatement { line: usize, kind: StmtKind, start_byte: usize, end_byte: usize }

// 1. Ordered files (BTreeMap order — NOT scheduler order).
let ordered: Vec<(&String, &ParsedFile)> = files.iter().collect();

// 2. Parallel collect (read-only): per file, the OLD recursion with mutations removed.
let per_file: Vec<(&String, Vec<PendingStatement>)> = ordered
    .par_iter()
    .map(|(path, parsed)| {
        let func_types = parsed.language.function_node_types();
        let mut seen: BTreeSet<usize> = BTreeSet::new();   // whole-file (file,line) first-win
        let mut stmts: Vec<PendingStatement> = Vec::new();
        collect_pending(parsed.tree.root_node(), &func_types, parsed, &mut seen, &mut stmts);
        (*path, stmts)
    })
    .collect();   // rayon indexed collect is order-preserving ⇒ ordered-files order

// 3. Serial create (the ONLY mutation; in files-order × walk-order).
let mut stmt_index: BTreeMap<(String, usize), NodeIndex> = BTreeMap::new();
for (path, stmts) in &per_file {
    for s in stmts {
        let idx = graph.add_node(CpgNode::Statement {
            file: (*path).clone(), line: s.line, kind: s.kind.clone(),
            start_byte: s.start_byte, end_byte: s.end_byte,
        });
        stmt_index.insert(((*path).clone(), s.line), idx);
        location_index.entry(((*path).clone(), s.line)).or_default().push(idx);
    }
}
```

`collect_pending` is the current `collect_function_statements` recursion (`:706`) **structurally unchanged**:
when a function node is encountered it processes that function's `statement_spans_in_function` (already
line-sorted + line-deduped), THEN recurses into children (so nested functions/closures are reached in the same
pre-order). The only change is the mutation point: the current `if stmt_index.contains_key(&(file,line)) {
continue }` becomes `if !seen.insert(line) { continue }` — a **whole-file** `seen: BTreeSet<usize>` (shared
across the entire file's recursion, NOT per-function), checked **before** `classify_stmt_kind` (exactly as the
current code skips before classify); and `add_node`/`stmt_index.insert`/`location_index.push` become
`stmts.push(PendingStatement { line, kind: classify_stmt_kind(&span.kind, parsed, span.line), .. })`.
`statement_spans_in_function` and `classify_stmt_kind` are reused verbatim.

**The order contract (the parity heart):** node creation order =
`files`-BTreeMap-order × recursive-function-node-traversal (pre-order) × each function's
`statement_spans_in_function` (line-sorted/deduped) × **whole-file first-`(file,line)`-win**. The
parallel-collect/serial-create reproduces this exactly: `par_iter` collect preserves the ordered-files order;
the per-file recursion + `seen` reproduce the intra-file order + dedup; the serial create assigns `NodeIndex`
in that order. Identical to today.

---

## §4. RISK / Architecture — the byte-identical proof (the core)

**Why node order is load-bearing.** `cpg_cache.rs` serializes `nodes: Vec<CpgNode>` from `graph.node_indices()`
and `edges` from `graph.edge_indices()` (`:186–204`), and reconstructs by adding nodes in that order
(`:401–413`). So one reordered statement node ⇒ different `NodeIndex` ⇒ different cache bytes ⇒ shifted edge
endpoints + every `NodeIndex`-keyed index. **Preserving statement `NodeIndex` order is the whole game.**

**Downstream parity (all follow from node order):**
- `stmt_index` → Step 8: `build_cfg_edges` resolves through `stmt_index.get` (`:581–589`); node reorder shifts
  ControlFlow edge endpoints.
- `location_index`: not separately serialized — rebuilt by iterating node order on cache load
  (`cpg_cache.rs:415–471`) then **sorted** (`:480`; `build.rs:667` sorts buckets: variables by byte/access,
  others by `NodeIndex`). So the post-sort statement order is determined by statement `NodeIndex` — preserving
  node order preserves it. (The append order itself is normalized by the sort, but the `NodeIndex` tiebreaker
  means node order is still the contract.)

**Safety — verified, no UB:**
- `static QUERY_CACHE: OnceLock<HashMap<…>>` (`queries.rs:31`) — `get_or_init` is exactly-once-safe under
  concurrent first-touch; after init it's an immutable `&'static` read by key (no iteration-order output). So
  `classify_stmt_kind` → `call_names_on_lines` (local `QueryCursor` + `BTreeSet`/`BTreeMap`) is **safe and
  deterministic** in parallel.
- `ParsedFile` is `Send+Sync` (existing `par_iter` in `CallGraph`/`DataFlowGraph` proves it); its `OnceLock`
  fields (`framework`, `call_args`) are concurrent-first-touch-safe.
- The parallel map captures only immutable `&` (no `&mut`); `add_node` happens **only** in the serial pass.

**Proof harness — distinguish the OLD-PARITY gate from the DETERMINISM gate:**

1. **Serial-reference node-order oracle — THE hard old-parity gate (same binary, in-memory).** Keep the
   original `collect_function_statements` as a `#[cfg(test)]` reference; build the CPG both ways and assert the
   parallel-collect/serial-create produces an **identical**: (a) `Statement`-node creation sequence
   `(file, line, kind, start_byte, end_byte)`; (b) `(file,line) → NodeIndex` (the ordinal that drives Step 8
   endpoints) — `stmt_index` is build-local, so dump its values, not just keys; (c) the **final sorted**
   `location_index` buckets (post `:667` sort). Use debug/tuple dumps — **`CpgNode::PartialEq` ignores
   statement byte-spans** (`types.rs:70–80`), so `==` is insufficient. This is in-memory (no cache
   serialization) so it is immune to the `git_sha`/build-metadata problem below, and — unlike thread-count
   equality — it actually proves *new == old order*, not just *new is deterministic*.

2. **`parallel_equality_test.rs` — the DETERMINISM gate (not old-parity).** It compares default-Rayon vs
   1-thread node/edge dumps (`:25–40`) and cache-blob bytes (`:42–66`) **within the same build** — both share
   `git_sha`, so it is a valid serial-vs-parallel determinism check; but it CANNOT prove equivalence to the
   pre-refactor algorithm (a consistently-reordered new algorithm passes it). Extend it with a **Step-7-heavy
   fixture/corpus**, multiple thread counts, and **minimum file/statement-count assertions** (guard against a
   silent corpus shrink masking a divergence).

3. **Tier-A `--matrix-only` 0 regressions** (AGENTS.md gate for `src/cpg/` changes; pre-commit). Note Tier-A
   may NOT flip on a pure node-order change that doesn't alter resolution — so it backstops, it does not
   replace gate 1.

**No `CACHE_VERSION` bump, and the cache-byte caveat:** the full `cpg-cache.bin` serializes metadata
(`version`/`prism_version`/`grammar_fingerprint`/**`git_sha`**/`file_hashes`/…) BEFORE the `graph: SerializedCpg`
(`cpg_cache.rs:62–88`), so a literal cross-commit golden blob is NOT a valid parity gate (it differs on
metadata). If a cross-build cache check is ever wanted, compare only the deserialized `SerializedCpg` graph
payload, normalizing metadata. The same-binary oracle (gate 1) is the authoritative old-order proof; if it
diverges, the design has failed its central requirement — that's the gate, not a workaround.

**Enumerated failure modes → guards:**

| failure mode | guard |
|---|---|
| file order ≠ BTreeMap | `ordered: Vec = files.iter().collect()` (not scheduler order) |
| node creation in parallel | forbidden — `add_node` only in the serial pass |
| dedup divergence | whole-file `seen: BTreeSet<line>` first-win; pinned by the oracle |
| function/stmt order drift | reuse the exact recursion + `statement_spans_in_function` |
| classify non-determinism / race | `classify_stmt_kind` unchanged; query cache is `OnceLock` (safe) |
| `location_index` bucket order | node-order preserved ⇒ post-sort identical; cache-byte test |
| Step 8 CFG endpoints shift | full edge-dump + cache-byte parity |
| hash/random ordering leak | `BTreeMap`/`BTreeSet` only; never iterate `HashMap` for output |
| test corpus silently shrinks | min file/statement-count assertions |

---

## §5. Verification & gates

- The §4 oracle + the extended `parallel_equality_test` (thread-determinism + cache-byte) green.
- `cargo test --lib` + `cargo test --test infra <filter>` + `cargo test --test integration core_test::` green.
- `cargo fmt`, `cargo clippy -p prism --lib` clean.
- **Tier-A `--matrix-only --allow-stale-sut`: 0 regressions** (run pre-commit per AGENTS.md — `uv`, not
  `cargo test`, so no `_dyld_start` stall).
- **Perf re-measure** (orchestrator): cold `nav --no-cache call-stats` on prism/tokio/hugo, branch vs `main`.
- **Verification-scope override (macOS):** full `cargo test`/`--test cli`/`--test frameworks` stall; use the
  filtered forms above + `--release` build. The orchestrator runs Tier-A + perf.

---

## §6. Risks

| risk | mitigation |
|---|---|
| Node-order divergence → cache-byte/Tier-A break | §4 three-gate proof (oracle + cache-byte + Tier-A); node order preserved by construction. |
| Hidden non-determinism in classify/spans | verified read-only + `OnceLock` query cache; thread-count tests from a cold pool. |
| `PartialEq` masks a span reorder | parity uses debug-dumps / raw cache bytes, never `==`. |
| Spike's ~4s doesn't hold in production | re-measure gate; the spike already kept node-create serial, so the parallelizable cost is real. |

---

## §7. Execution

- Branch `step7-parallel` (off `main`).
- Two design passes folded (this author + codex gpt-5.5 xhigh) — both GO with hard parity gates.
- Codex implement(high)/review(xhigh), TDD: the oracle FIRST (red), then the parallel refactor to green.
- File map: `src/cpg/build.rs` (Step 7 + `collect_pending` + the `#[cfg(test)]` reference oracle);
  `tests/infra/parallel_equality_test.rs` (extend). No other files.
- Plan: `docs/superpowers/plans/2026-06-19-prism-step7-parallel.md` (next step).
