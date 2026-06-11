# Prism S1 — Build Performance Hardening + Level-4 Index Inversion (Design)

**Status:** Owner design, **revision 2 (2026-06-11)** — folded the dual spec-review
(codex/gpt-5.5 rigor + claude soundness, prism-wired;
`docs/prism-query-layer/s1-spec-review-MCP-2026-06-11.md`): all 12 findings accepted
(BLOCKER 1 `FunctionInfo` contract; MAJORs 2–7 reconstruction-failure behavior, legacy-quirk
pinning by construction, index-independent oracle universe, C1 skip-classification, cache
insertion-order stability, objective acceptance gates; MINORs 8–12). Input to `writing-plans`.
**Context:** First slice of the substrate program in
`docs/cpg-substrate-analysis-2026-06-10.md` (S1) plus the Level-4 inversion identified in
`docs/prism-meta-analysis-2026-06-10.md` §5. Profiling evidence: prism-repo cold build is
dominated by repeated `ParsedFile::all_functions()` tree-sitter queries; at kubernetes scale
100% of a sampled window sits in `CallGraph::build` → `ast::resolve_struct_field_assignment`
(`call_graph.rs:312-360`). Measured baselines (this machine, release): prism 108k LOC / 29 s
cold; tokio 175k / 89 s; hugo 234k / 469 s; django 551k and rust-analyzer 589k exceed a
40-minute timeout.

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
`CACHE_VERSION` bump anywhere in S1**; see §2a for the insertion-order corollary).

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

### 2a. Cache insertion-order corollary (review MAJOR 6)

The binary cache serializes CPG node/edge **vectors in insertion order** to preserve
`NodeIndex` stability across save/load (`cpg_cache.rs`). Under the no-`CACHE_VERSION`-bump
rule, every S1 slice must therefore **reproduce the serial build's insertion order exactly**
— sorted-set equality is not sufficient. All §2.2 equality tests in this spec compare the
node and edge vectors **in order**, not as sorted sets. For Slice C this means: extraction
may be parallel, but **graph assembly stays serial** in the same canonical iteration order
as today (§5).

## 3. Slice A — eager `FunctionTable`

**Problem.** `all_functions()` re-runs the whole-file Functions query on every call; 8 hot
call sites (`call_graph.rs:75,:101,:162,:189,:567,:596`, `data_flow.rs:169`,
`cpg/build.rs:349` — the last one *per resolved callee per call site* in Step 5b).

**Design.**

```rust
pub struct FunctionInfo {
    pub name: Option<String>,    // None for anonymous functions (JS/TS callback lambdas):
                                 // function_name() genuinely returns None there (BLOCKER 1)
    pub kind_id: u16,            // tree-sitter node kind id, for reconstruction + parity
    pub start_byte: usize, pub end_byte: usize,
    pub start_line: usize, pub end_line: usize,
    pub param_names: Vec<String>,
}
// ParsedFile gains: functions: Vec<FunctionInfo>  (plain owned data, computed in parse())
pub fn functions(&self) -> &[FunctionInfo];
```

- Computed **eagerly in `ParsedFile::parse`**, via the **existing dual-path logic** — the
  compiled Functions query when available, `collect_functions_manual` otherwise (review
  MINOR 8; the fallback is load-bearing against grammar drift). The table preserves the
  **full captured sequence in query order**, including unnamed functions — dropping them
  would change the CPG node set and violate §2.2. Note: JS/TS name inference reads *parent*
  nodes (`languages/mod.rs:927-973`), so table construction happens with full tree access
  inside `parse()` — it cannot be hoisted before tree construction.
- `all_functions()` is **reimplemented over the table**: reconstruct each `Node` via
  `tree.root_node().descendant_for_byte_range(start, end)` followed by a walk-down to the
  node matching **both** the stored byte range and `kind_id`. **Failure behavior (review
  MAJOR 2): if reconstruction does not find an exact `(range, kind_id)` match for every
  entry, fall back to the direct query path for that file — never silently skip a node.**
  All 28 existing call sites compile unchanged; migrating call sites off `Node` is follow-up.
- **Step 5b** (`cpg/build.rs:349-359`): replace `all_functions()` + linear name search with a
  scan over `functions()` taking the **first entry whose `name == Some(callee_name)`**.
  First-wins on duplicate names is the existing behavior; it is the F2 identity defect,
  retained deliberately and marked `// pinned-until-S2`. Unnamed entries never match —
  identical to today, where `function_name() == None` fails the name comparison.
- **Warm-path cost (review MINOR 9):** on a nav-cache hit the CPG build is skipped but
  `load_repo` still parses every file, so the eager table adds query+extraction cost to the
  ~0.45 s warm path that previously paid nothing for it. This is accepted (C1's parallel
  parsing more than offsets it), but it is **measured**: the bench ladder's warm column is a
  report-out gate — no material warm regression (>10%) without explanation.

**Tests.** Per-language fixture equivalence: `all_functions()` (reconstructed) returns nodes
with identical `(kind_id, byte-range)` sequences as a direct query — fixtures must include
JS/TS anonymous callbacks and same-named functions; param-name parity; reconstruction-failure
fallback test (synthetic mismatch → direct-query path, full sequence still returned); full
goldens (contract §2.3).

## 4. Slice B — Level-4 inversion

**Problem.** `resolve_struct_field_assignment(source, field, known_fns)` (`src/ast.rs`) is a
raw text scan (`source.lines()`, substring `->field` / `.field`, then `= rhs` extraction and
`known_fns` filtering). Phase 3 Level 4 calls it for **every unresolved qualified call ×
every file** (`call_graph.rs:339-345`): O(unresolved_calls × files × lines). At Go scale this
is effectively the whole build.

### B1 — pure inversion (this plan; byte-identical **by construction**)

- **The legacy quirks are the contract** (review MAJOR 3). The legacy per-field scan has
  verified order-dependent behavior a clean generalized scanner would silently "fix":
  `find(arrow).or_else(find(dot))` gives `->field` *anywhere in the remaining line* priority
  over a closer `.field` (`s.cb = f; t->cb = g;` queried for `cb` returns only `{g}`); a
  non-assignment prefix occurrence (`->cbx` matching `find("->cb")`) consumes scan position
  and can suppress a real earlier assignment; plus single-`=` anchoring, the RHS token stop
  rules, `&`-stripping, the `known_fns` filter, per-file `BTreeSet` dedup/sort, and per-line
  state reset.
- **Construction, therefore:** the index builder does **not** introduce a new generalized
  scanner. Per file, per line, it (1) enumerates the **distinct candidate field identifiers**
  on the line (every identifier immediately preceded by `->` or `.`), then (2) for each
  candidate, runs **the legacy per-field matching logic itself** (refactored to a per-line
  callable, not rewritten) to compute that field's targets. Quirks are reproduced because
  the quirky code *is* the extractor. Cost: O(distinct_fields_on_line × line_len) — still a
  single pass over the repo, vs O(unresolved_calls × files × lines) today.
- Index shape:
  `level4_index: BTreeMap<String /*field*/, BTreeMap<String /*file*/, BTreeSet<String /*fn*/>>>`,
  built once in the Phase-3 preamble (files iterated in map order; `known_fn_names` is fixed
  by Phase 1). The per-call Level-4 site keeps all current filters (qualifier present,
  alphanumeric callee, already-resolved skip, `call_graph.rs:315-327`) and replaces the file
  loop with `level4_index.get(field)`; nested map order reproduces legacy emission exactly.
- **Differential oracle with an index-independent universe** (review MAJOR 4). The legacy
  function survives (test-only visibility) as the oracle. The test's field universe is
  **not** derived from the new index's keys (that would make extraction misses invisible).
  It is: (a) every unresolved, qualifier-bearing, alphanumeric callee name that Phase 3
  would actually query, computed from the corpus's call sites; **plus** (b) every identifier
  appearing immediately after `.`/`->` anywhere in the corpus sources; **plus** (c) explicit
  negative fixtures (fields with no assignments; prefix-overlap names; mixed-accessor lines).
  For every `(field, file)` in that universe, assert index-derived targets == legacy-scan
  targets, **in both directions**. Corpus = the language fixtures + the prism repo's own
  sources; new fixtures pin the mixed-accessor and prefix-overlap quirks individually.
- **Rejected simpler alternative (review MINOR 10):** per-field memoization of legacy
  results (a few lines, trivially byte-identical) was considered and rejected: it stays
  O(distinct_queried_fields × total_lines) with the full per-call source scans intact —
  insufficient at Go scale where unresolved-field cardinality is large (hugo/kubernetes
  profile evidence) — and it builds no structure for B2 to inherit. The inverted index is
  also B2's designated home.

### B2 — precision upgrade (contracted now, scheduled after Tier A)

- Replace the line scanner with **AST-based assignment extraction**: assignment expressions,
  init declarators, and designated initializers whose LHS is a field access — immune to
  matches inside comments and string literals; tokenized field boundaries instead of
  substring anchoring.
- Record **provenance**: the assignment site (`file:line`) justifying each resolved edge —
  data the legacy scan discards and the Evidence `why` chain wants. **The in-memory shape
  (where provenance lives, its lifetime, and any wire exposure) is explicitly deferred to
  the B2 plan** (review MINOR 11); B1 introduces no provenance.
- Divergences vs B1 are **triaged** (legacy artifact vs real loss), then goldens re-blessed
  in **one isolated behavior commit** with the triage table in the PR description.
- **Quirk retirement list (explicit B2 targets).** B1 *preserves* these legacy defects by
  construction; each is a named candidate for measured correction once Tier A is live, and
  each B1 quirk fixture flips at B2 time from "pins legacy behavior" to "documents the
  corrected behavior":
  1. **Arrow-anywhere priority** — `find(arrow).or_else(find(dot))` lets a `->field` later
     in the line shadow a closer `.field` assignment (**false negative**: real assignment
     dropped).
  2. **Prefix-consumption** — a non-assignment occurrence (`->cbx` matched while scanning
     for `->cb`) advances the scan position past a real assignment (**false negative**).
  3. **Comment/string-literal matches** — substring scanning sees assignments inside
     comments and string literals (**false positive**).
  4. **Substring field anchoring** — untokenized left boundary; AST extraction replaces it.
  5. **Single-line scope** — multi-line assignments and initializer spreads are invisible
     (**false negative**); AST extraction lifts this.
- **Gate:** scheduled only once the Tier-A harness is live, so each retirement is *measured*
  (Tier-A precision/recall delta + triage table) rather than asserted. B2 is intentionally
  absent from the implementation plan for this spec.

## 5. Slice C — parallelization (contingent)

**Investigation task (first):** why is `LoadedRepo: !Sync`
(`src/mcp/session.rs:22-24` comment)? Suspects: lazy framework-detection cell in `ParsedFile`
(`ast.rs:57`); `tree_sitter::Tree` is Send+Sync in the pinned tree-sitter (0.25.10, verified
in review). Outcome decides C2.

- **C1 (committed):** parallelize **parsing** in `repo_loader::load_repo`, preserving skip
  *classification* and *order* exactly (review MAJOR 5):
  - The serial walk performs every step **up to and including language detection in today's
    exact order** — metadata/size check, `fs::read`, UTF-8 decode, `Language::from_path` —
    so classification cannot flip (a non-UTF-8 file with an unsupported extension stays
    `NotUtf8`, exactly as today). The walk emits, in raw `read_dir` walk order, either a
    skip entry or a **candidate record `(rel_path, source, language)`** holding the
    already-read source.
  - rayon par-maps candidates → `(parse result, sha256 hash)` (hashing moves par-side over
    the owned source; output is position-independent). Each thread owns its results;
    requires only `Send`.
  - Merge walks the candidate list **in its original order**, inserting parse-failure skip
    entries into their walk-position slots, so `files`, `file_hashes`, and the `skipped`
    vector are **element-for-element identical** to the serial loader's output (§2.1).
- **C2 (conditional on the investigation):** if `ParsedFile: Sync` is cheap, parallelize
  per-file **extraction** in call-graph Phases 1–2 and per-file DFG construction — but
  **graph/struct assembly stays serial in today's canonical iteration order** so that node
  and edge insertion order is bit-for-bit reproduced (§2a). Phase 3 and Step 5b stay serial
  (cheap after A+B). If Sync is not cheap, C2 is descoped without re-planning.
- New dependency: `rayon` (already on the project roadmap,
  `docs/language-expansion-plan.md` Phase 3).
- If `ParsedFile` becomes `Sync`, optionally drop the
  `#[allow(clippy::arc_with_non_send_sync)]` in `mcp/session.rs` (hygiene, non-gating).

**Tests.** Serial-vs-parallel **exact-order CPG equality** — node vector and edge vector
compared **in insertion order**, not sorted (§2a) — on fixture repos and on the prism repo
itself; loader parity test asserting `files`/`file_hashes`/`skipped` element-for-element
equality vs a serial reference run; existing cache round-trip tests unchanged.

## 6. Bench deliverable (contract per review MINOR 12)

`scripts/bench-ladder.sh`:

- **Inputs:** repo list as `name:path` args (default list pinned to
  `prism, tokio, hugo, django, rust-analyzer` — the last two are the motivating timeout
  failures, so the PR re-run answers "did S1 restore the scale ladder"); `--cache-dir`
  (default: a fresh temp dir per run); `--timeout <s>` per repo.
- **Per repo:** LOC + file count over prism-supported extensions excluding builtin skip
  dirs; **cold** = first run against an empty cache dir (build + cache write), wall-clock
  via `date`, peak RSS via `/usr/bin/time -l` (`maximum resident set size`); **cache MB** =
  `du -sm` of the repo's cache subdir; **warm** = an immediately-following second run of the
  identical query (cache hit), wall-clock.
- **Timeout semantics:** `timeout` exit 124 → row records `TIMEOUT>{s}` plus RSS-at-kill;
  subsequent columns `-`.
- **Output:** one markdown row per repo:
  `repo | loc | files | cold_s | maxrss_mb | cache_mb | warm_s | status`.
- Not wired into CI (machine-dependent), used for PR report-outs.

## 7. Acceptance (per Q2 decision: hotspot gates, not wall-clock gates — made objective per review MAJOR 7)

| Slice | Accepted when |
|---|---|
| A | During a cold release build (`prism nav --no-cache repo-map` on **tokio**), `sample <pid> 10 -file out.txt` shows `all_functions`/`ts_query` frames in **<1% of samples** (today: dominant); equivalence + parity + fallback tests green; goldens byte-identical |
| B1 | Same sampling during a cold build of **hugo**: `resolve_struct_field_assignment` frames in **<1% of samples** (today: dominant at scale); differential oracle test green (both directions, full universe); goldens byte-identical |
| C | Exact-order serial==parallel equality green; loader parity green; goldens byte-identical; during the parse phase of a cold hugo build, `user` CPU time / wall time **> 1.5** (today ≈ 1.0) |
| All | `cargo fmt --check`, full `cargo test`, `--features mcp` build/test, `cli_nav_compat` byte-identical, `algo_taint_cve` green; bench-ladder re-run (default list) recorded in the PR, including **warm-time parity within 10%** of baseline (report-out; regression requires explanation) |

Profiling conditions: release build, empty cache dir, this machine class documented in the PR
(numbers are report-out, thresholds above are the gates).

## 8. Error handling

No new error surfaces. Index building walks already-parsed sources and is infallible;
parse-degraded files behave exactly as today (only parsed files are in `files`); rayon
panics propagate as serial panics do. Empty index / zero-function files degrade to the
current "no Level-4 targets / no functions" behavior. `FunctionTable` reconstruction
failure falls back to the direct query path (§3), never partial results.

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
- **Spec/plan review:** dual-bridge workflow (used for this spec's revision 2); codex
  single-reviewer acceptable for small follow-ups.
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
