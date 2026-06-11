# Prism S1 — Build Performance Hardening + Level-4 Index Inversion (Design)

**Status:** Owner design, approved 2026-06-11 (brainstorming session). Input to `writing-plans`.
**Context:** First slice of the substrate program in
`docs/cpg-substrate-analysis-2026-06-10.md` (S1) plus the Level-4 inversion identified in
`docs/prism-meta-analysis-2026-06-10.md` §5. Profiling evidence: prism-repo cold build is
dominated by repeated `ParsedFile::all_functions()` tree-sitter queries; at kubernetes scale
100% of a sampled window sits in `CallGraph::build` → `ast::resolve_struct_field_assignment`
(`call_graph.rs:312-360`), the Level-4 pass that rescans every file per unresolved call.
Measured baselines (this machine, release): prism 108k LOC / 29 s cold; tokio 175k / 89 s;
hugo 234k / 469 s; django 551k and rust-analyzer 589k exceed a 40-minute timeout.

**Goal:** Remove both hotspots and (contingently) parallelize the build — *without entombing
legacy defects under a blanket byte-compatibility rule*. Accuracy work (S3, Tier-A harness)
is separate and sequenced in parallel; this spec touches behavior only in one designated,
deferred commit (B2).

## 1. Scope

Three slices + one deliverable:

- **A — eager `FunctionTable`** in `ParsedFile::parse` (kills the repeated-query hotspot).
- **B — Level-4 inversion** into a build-once index; two-commit structure (B1 pure perf now,
  B2 precision upgrade deferred until the Tier-A harness is live).
- **C — parallelization** (contingent on a `Sync` investigation; C1 parse-parallelism is
  committed, C2 analysis-parallelism is conditional).
- **Bench deliverable:** commit the scale-ladder script as `scripts/bench-ladder.sh`.

Out of scope: call-site migration off `Node` to `FunctionTable` (follow-up hygiene), S2
node-identity work, S3 resolution precision, any serialized-shape change (**no
`CACHE_VERSION` bump anywhere in S1**).

## 2. Correctness contract (replaces blanket byte-identity)

Layered; agreed during brainstorming after explicitly weighing whether Option-C byte-identity
is overly restrictive here (verdict: it is, for exactly one slice — B2):

1. **Determinism, always.** Same input → same output, including under parallelism.
2. **Semantic edge-set equality is the refactor gate.** A perf commit must produce the
   identical set of `(caller, callee_name, line)` call edges and identical CPG node/edge
   sets, proven by differential tests (B1) or serial-vs-parallel equality (C).
3. **Byte-goldens hold at the wire boundary** (`cargo test --test cli_nav_compat`
   byte-identical; algo fixture suites) for every commit **except** a designated behavior
   commit (only B2 in this spec).
4. **Behavior changes live in isolated, documented, re-blessed commits** — never inside perf
   commits (bisect hygiene).
5. **Exit ramp (recorded intent):** when the Tier-A LSP-oracle harness lands, measured
   precision/recall replaces byte-stability as the behavior-change gate; byte-goldens retreat
   permanently to the wire boundary. Option-C byte-identity is scaffolding for the
   no-semantic-ground-truth era, not a permanent invariant.

## 3. Slice A — eager `FunctionTable`

**Problem.** `all_functions()` re-runs the whole-file Functions query on every call; 8 hot
call sites (`call_graph.rs:75,:101,:162,:189,:567,:596`, `data_flow.rs:169`,
`cpg/build.rs:349` — the last one *per resolved callee per call site* in Step 5b).

**Design.**

```rust
pub struct FunctionInfo {
    pub name: String,            // resolved via language.function_name + node_text
    pub start_byte: usize, pub end_byte: usize,
    pub start_line: usize, pub end_line: usize,
    pub param_names: Vec<String>, // via function_parameter_names, same walk
}
// ParsedFile gains: functions: Vec<FunctionInfo>  (plain owned data, computed in parse())
pub fn functions(&self) -> &[FunctionInfo];
```

- Computed **eagerly in `ParsedFile::parse`** (single Functions-query pass + per-function
  name/param extraction). Eager beats lazy here: every CPG build queries every file ≥7 times
  today, so the table always pays for itself; plain data avoids interior mutability and is
  the `Sync`-friendly choice Slice C needs. It is also the designed seam for S2 span-based
  function identity and future per-file incremental updates.
- `all_functions()` is **reimplemented over the table**: reconstruct each `Node` via
  `tree.root_node().descendant_for_byte_range(start, end)` followed by a kind/range
  walk-down check (guard against a child or parent sharing the byte range). All 28 existing
  call sites compile unchanged. Functions the query previously returned in tree order must
  come back in the same order (table preserves query order).
- **Step 5b** (`cpg/build.rs:349-359`): replace `all_functions()` + linear name search with a
  scan over `functions()` taking the **first** name match. First-wins on duplicate names is
  the existing behavior; it is the F2 identity defect, retained deliberately and marked
  `// pinned-until-S2` — one parity test, no further enshrinement.
- Anything not derivable from the table (rare callers needing live nodes) keeps working via
  the reconstruction path.

**Tests.** Per-language fixture equivalence: `all_functions()` (reconstructed) returns nodes
with identical `(kind, byte-range)` sequences as a direct query; param-name parity on fixtures
including same-named functions; full goldens (contract §2.3).

## 4. Slice B — Level-4 inversion

**Problem.** `resolve_struct_field_assignment(source, field, known_fns)` (`src/ast.rs`) is a
raw text scan (`source.lines()`, substring `->field` / `.field`, then `= rhs` extraction and
`known_fns` filtering). Phase 3 Level 4 calls it for **every unresolved qualified call ×
every file** (`call_graph.rs:339-345`): O(unresolved_calls × files × lines). At Go scale this
is effectively the whole build.

### B1 — pure inversion (this plan; byte-identical)

- In the Phase-3 preamble, build once:
  `level4_index: BTreeMap<String /*field*/, BTreeMap<String /*file*/, BTreeSet<String /*fn*/>>>`
  by iterating `files` (already BTreeMap-ordered) and running a **generalized single-pass
  scanner** that extracts every `(field_ident, candidate_fn)` pair using *identical pattern
  semantics* to the legacy function — same `->ident =` / `.ident =` anchoring, same RHS
  extraction, same `known_fn_names` filter (fixed by Phase 1 before Phase 3 runs).
- The per-call Level-4 site keeps all its current filters (qualifier present, alphanumeric
  callee, already-resolved skip) and replaces the file loop with
  `level4_index.get(field)` — emitting `(file → sorted fns)` in nested map order reproduces
  the legacy emission order exactly.
- **The legacy function survives as the differential oracle** (`#[cfg(test)]` or
  test-only visibility): a differential test asserts, for every `(field, file)` over the
  fixture corpus *and the prism repo's own sources*, that index-derived targets ==
  legacy-scan targets. This is the semantic-equality proof for contract §2.2.
- Complexity: O(total_lines) once + map lookups.

### B2 — precision upgrade (contracted now, scheduled after Tier A)

- Replace the line scanner with **AST-based assignment extraction**: assignment expressions,
  init declarators, and designated initializers whose LHS is a field access — immune to
  matches inside comments and string literals; tokenized field boundaries instead of
  substring anchoring.
- Record **provenance**: the assignment site (`file:line`) justifying each resolved edge —
  data the legacy scan discards and the Evidence `why` chain wants (feeds the S3/ergonomics
  work later; no wire change in this spec).
- Divergences vs B1 are **triaged** (legacy artifact vs real loss), then goldens re-blessed
  in **one isolated behavior commit** with the triage table in the commit/PR description.
- **Gate:** scheduled only once the Tier-A harness is live, so the change is *measured*
  (precision/recall delta) rather than asserted. B2 is intentionally absent from the
  implementation plan for this spec; this section is its contract.

## 5. Slice C — parallelization (contingent)

**Investigation task (first):** why is `LoadedRepo: !Sync`
(`src/mcp/session.rs:22-24` comment)? Suspects: lazy framework-detection cell in `ParsedFile`
(`ast.rs:57`); verify `tree_sitter::Tree` Send/Sync in the pinned tree-sitter version.
Outcome decides C2.

- **C1 (committed):** parallelize **parsing** in `repo_loader::load_repo` — walk the tree
  serially, applying every non-parse skip rule exactly as today (dirs, symlinks, hidden,
  too-large, unreadable, unsupported) and collecting read candidates **in walk order**; then
  rayon par-map `(candidate → parse+hash result)` (each thread owns its results; requires
  only `Send`); then merge results back **in candidate (walk) order**, so `files`,
  `file_hashes`, and the `skipped` vector (including parse-failure entries) are exactly what
  the serial loader produces (determinism §2.1).
- **C2 (conditional on the investigation):** if `ParsedFile: Sync` is cheap (e.g., the lazy
  cell → `OnceLock`, or already-Sync), parallelize per-file extraction in call-graph
  Phases 1–2 and per-file DFG construction, merging deterministically. **Phase 3 and Step 5b
  stay serial** — cheap after A+B. If Sync is not cheap, C2 is descoped without re-planning.
- New dependency: `rayon` (already on the project roadmap,
  `docs/language-expansion-plan.md` Phase 3).
- If `ParsedFile` becomes `Sync`, optionally drop the
  `#[allow(clippy::arc_with_non_send_sync)]` in `mcp/session.rs` (hygiene, non-gating).

**Tests.** Serial-vs-parallel **full CPG equality** (sorted node list + edge list identical)
on fixture repos and on the prism repo itself; existing cache round-trip tests unchanged.

## 6. Bench deliverable

Commit `scripts/bench-ladder.sh` (parameterized: repo list, cache dir, per-repo timeout;
emits the §5-style markdown row per repo: LOC, files, cold s, peak RSS, cache MB, warm s).
Used for acceptance evidence; not wired into CI (machine-dependent numbers, per acceptance
policy).

## 7. Acceptance (per Q2 decision: hotspot gates, not wall-clock gates)

| Slice | Accepted when |
|---|---|
| A | `sample` profile of a cold prism/tokio build no longer shows `all_functions`/query frames in the top stacks; equivalence + parity tests green; goldens byte-identical |
| B1 | profile of a cold hugo (or kubernetes-capped) build no longer shows `resolve_struct_field_assignment`; differential oracle test green; goldens byte-identical |
| C | serial==parallel equality green; goldens byte-identical; build uses >1 core during parse (and analysis if C2) |
| All | `cargo fmt --check`, full `cargo test`, `--features mcp` build/test, `cli_nav_compat` byte-identical, `algo_taint_cve` green; bench-ladder re-run recorded in PR (report-out, not gate) |

## 8. Error handling

No new error surfaces. Index building walks already-parsed sources and is infallible;
parse-degraded files behave exactly as today (only parsed files are in `files`); rayon
panics propagate as serial panics do. Empty index / zero-function files degrade to the
current "no Level-4 targets / no functions" behavior.

## 9. Execution & review wiring (owner decisions)

- **Method:** `superpowers:subagent-driven-development`; implementor = **codex gpt-5.5 via
  a2a-bridge** (`~/code/a2a-bridge`, orbstack confirmed back up); TDD.
- **Review loop:** codex gpt-5.5 secondary review + a Claude reviewer (Opus 4.8 or Fable,
  chosen per task complexity) — dual loop to convergence before merge.
- **Tooling for agents:** reviewers get the **prism MCP server** (read-only; snapshot
  semantics acceptable for review). Implementors get the **prism CLI only, not the MCP
  server** — the MCP session serves a frozen at-launch snapshot
  (`docs/prism-meta-analysis-2026-06-10.md` §4); implementors mutate the tree mid-task and
  would receive stale answers, while the CLI revalidates per invocation.
- **Spec/plan review:** codex single-reviewer by default; dual-bridge process if a round
  disagrees.
- **Hygiene:** slice-per-commit during development; force-push squash to a docs+feat pair
  before merge (bisect hygiene); deferred work (B2, C2-if-descoped, call-site migration off
  `Node`) recorded in the followups doc.

## 10. Slice order & dependencies

```
A (FunctionTable)  →  B1 (Level-4 inversion)  →  C-investigation → C1 → [C2?]
                                   bench script lands with A; re-run after each slice
B2: contracted here, scheduled when Tier-A harness is live (separate plan)
```

A before B1 only because both touch Phase-3-adjacent code and A simplifies Step 5b first;
they are otherwise independent. C last: it benefits from A removing the lazy-cell pressure
and from B1 shrinking the serial Phase 3.
