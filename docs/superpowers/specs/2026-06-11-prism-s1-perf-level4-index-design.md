# Prism S1 — Build Performance Hardening + Level-4 Index Inversion (Design)

> **EXECUTION OUTCOME (2026-06-11):** all slices MERGED on `s1-perf-level4` via containerized
> codex implement (every commit verify-PASS + dual diff-review APPROVE). **C2 = GO**
> (`ParsedFile: Send+Sync` after the OnceLock swap). Acceptance: Slice A — `all_functions` at
> 0 samples on cold tokio, prism 29→19 s, tokio 89→47 s; Slice B1 — legacy symbol at 0
> samples on cold hugo, hugo 469→137 s, django TIMEOUT→909 s (pre-C numbers; final table in
> the PR). Deferred work + execution findings: `docs/archive/plans/prism-query-layer/s1-followups.md`.

**Status:** Owner design, **revision 3 (2026-06-11)** — folded round-2 dual review
(`docs/archive/review-artifacts/prism-query-layer/s1-spec-review-r2-MCP-2026-06-11.md`; verdict "fold 1–8 then
plan"): walk-**up** reconstruction + fallback-fire counter (r2-BLOCKER 1), full-`CallSite`
equality (r2-2), CG/DFG serialization parity (r2-3), pinned identifier predicate (r2-4),
two-half oracle pairing rule (r2-5), legacy-symbol-≈0 B1 gate + full-command C gate (r2-6),
in-module test seam (r2-7), bench command templates/portability (r2-8, 12), MINORs 9–13.
Revision 2 folded round 1 (`s1-spec-review-MCP-2026-06-11.md`, all 12 findings). Input to
`writing-plans`.
**Context:** First slice of the substrate program in
`docs/features/cpg/substrate-analysis-2026-06-10.md` (S1) plus the Level-4 inversion identified in
`docs/archive/analysis/prism/prism-meta-analysis-2026-06-10.md` §5. Profiling evidence: prism-repo cold build is
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
2. **Semantic edge-set equality is the refactor gate.** A perf commit must produce
   identical call edges **over the full `CallSite`** — caller `FunctionId`
   (`file, name, start_line, end_line`), `callee_name`, `line`, **and `qualifier`** (r2-2)
   — and identical CPG node/edge sets, proven by differential tests (B1) or
   serial-vs-parallel equality (C).
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
`NodeIndex` stability across save/load (`cpg_cache.rs`) — **and it serializes the
`CallGraph` and `DataFlowGraph` alongside the CPG, whose `Vec` fields are equally
order-sensitive** (r2-3). Under the no-`CACHE_VERSION`-bump rule, every S1 slice must
therefore **reproduce the serial build's insertion order exactly across all three
structures** — sorted-set equality is not sufficient. All §2.2 equality tests compare
vectors **in order**; the strongest form, required for Slice C, is a **byte-level cache
parity test**: serialize the cache blob from a serial and a parallel build and assert the
bytes are identical. For Slice C this means: extraction may be parallel, but **graph/struct
assembly stays serial** in the same canonical iteration order as today (§5). Lifting this
insertion-order cap later is S2-adjacent `NodeIndex`-identity work, **not** a C2 option
(r2-13).

## 3. Slice A — eager `FunctionTable`

**Problem.** `all_functions()` re-runs the whole-file Functions query on every call; 8 hot
call sites (`call_graph.rs:75,:101,:162,:189,:567,:596`, `data_flow.rs:169`,
`cpg/build.rs:349` — the last one *per resolved callee per call site* in Step 5b).

**Design.**

```rust
#[derive(Debug, Clone, PartialEq, Eq)]      // Clone: ParsedFile derives Clone (r2-9)
pub struct FunctionInfo {
    pub name: Option<String>,    // None for anonymous functions (JS/TS callback lambdas):
                                 // function_name() genuinely returns None there (BLOCKER 1)
    pub kind_id: u16,            // tree-sitter node kind id, for reconstruction + parity
    pub start_byte: usize, pub end_byte: usize,
    pub start_line: usize, pub end_line: usize, // 1-indexed, inclusive (r2-9)
    pub param_names: Vec<String>,
}
// ParsedFile gains: functions: Vec<FunctionInfo>  (plain owned data, computed in parse())
pub fn functions(&self) -> &[FunctionInfo];
```

**Eager is load-bearing, not just convenient (r2-11):** under C1, a lazy table would be
first-touched inside the *serial* CPG build, surrendering the parallelism eager construction
buys during the par-map parse. Lazy `OnceLock` construction is the designated fallback if
the §7 warm-parity gate (≤10%) ever fails.

- Computed **eagerly in `ParsedFile::parse`**, via the **existing dual-path logic** — the
  compiled Functions query when available, `collect_functions_manual` otherwise (review
  MINOR 8; the fallback is load-bearing against grammar drift). The table preserves the
  **full captured sequence in query order**, including unnamed functions — dropping them
  would change the CPG node set and violate §2.2. Note: JS/TS name inference reads *parent*
  nodes (`languages/mod.rs:927-973`), so table construction happens with full tree access
  inside `parse()` — it cannot be hoisted before tree construction.
- `all_functions()` is **reimplemented over the table**: reconstruct each `Node` via
  `tree.root_node().descendant_for_byte_range(start, end)` — which returns the **deepest**
  node spanning the range — followed by a **walk UP through same-span ancestors** to the
  node matching **both** the stored byte range and `kind_id` (r2-BLOCKER 1: a walk-down can
  never reach a same-span ancestor; the recovery direction is upward by construction).
  **Failure behavior (r1 MAJOR 2): if reconstruction does not find an exact
  `(range, kind_id)` match for every entry, fall back to the direct query path for that
  file — never silently skip a node.** The fallback **fires a counter/flag exposed to the
  in-module unit tests** (r2-BLOCKER 1 + r2-7: tests live in `src/ast.rs`'s `#[cfg(test)]`
  mod, which can both read the flag and mutate `functions` to force a synthetic mismatch);
  the per-language equivalence tests assert the fallback fired **zero** times, so grammar
  drift that silently flips a language to the fallback path becomes a test failure, not a
  silent perf regression. All 28 existing call sites compile unchanged; migrating call
  sites off `Node` is follow-up.
- **Step 5b** (`cpg/build.rs:349-359`): replace `all_functions()` + linear name search with a
  scan over `functions()` taking the **first entry whose
  `name.as_deref() == Some(callee_id.name.as_str())`** — the comparand is **`callee_id.name`**,
  not `site.callee_name`; both are in scope there and they differ for Level-4-resolved
  calls (r2-10).
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
  scanner. Per file, per line, it (1) enumerates the **distinct candidate field
  identifiers** on the line, then (2) for each candidate, runs **the legacy per-field
  matching logic itself** (refactored to a per-line callable **with its own distinct symbol
  name**, e.g. `line_field_targets` — load-bearing for the §7 gate, r2-6b) to compute that
  field's targets. Quirks are reproduced because the quirky code *is* the extractor. Cost:
  O(distinct_fields_on_line × line_len) — a single pass over the repo, vs
  O(unresolved_calls × files × lines) today.
- **The identifier predicate is pinned (r2-4), for both candidate enumeration and the
  oracle universe:** a candidate field is a **maximal run of `char::is_alphanumeric(c) ||
  c == '_'`** (the same Unicode-aware predicate the Level-4 call-site filter applies to
  callee names, `call_graph.rs:321-326`) **immediately preceded by `->` or `.`**, scanned
  over **raw source lines including comments and string literals** (B1 reproduces legacy
  text-scan semantics; comment/string blindness is quirk 3, retired in B2). Completeness
  argument: a field occurrence can only produce a target when followed (after optional
  whitespace) by `=`, which terminates the identifier run — so every productive field is a
  maximal run under exactly this predicate.
- Index shape:
  `level4_index: BTreeMap<String /*field*/, BTreeMap<String /*file*/, BTreeSet<String /*fn*/>>>`,
  built once in the Phase-3 preamble (files iterated in map order; `known_fn_names` is fixed
  by Phase 1). The per-call Level-4 site keeps all current filters (qualifier present,
  alphanumeric callee, already-resolved skip, `call_graph.rs:315-327`) and replaces the file
  loop with `level4_index.get(field)`; nested map order reproduces legacy emission exactly.
- **Differential oracle with an index-independent universe** (r1 MAJOR 4). The legacy
  function survives (test-only visibility) as the oracle. The test's field universe is
  **not** derived from the new index's keys (that would make extraction misses invisible).
  It is: (a) every unresolved, qualifier-bearing callee name (under the pinned predicate)
  that Phase 3 would actually query, computed from the corpus's call sites; **plus** (b)
  every identifier appearing immediately after `.`/`->` anywhere in the corpus sources
  (pinned predicate); **plus** (c) explicit negative fixtures (fields with no assignments;
  prefix-overlap names; mixed-accessor lines). Corpus = the language fixtures + the prism
  repo's own sources; new fixtures pin the mixed-accessor and prefix-overlap quirks
  individually.
- **Pairing rule (r2-5) — two halves, so the test does not re-create the removed hotspot
  inside `cargo test`:** (i) *excess*: iterate the **index's own `(field, file)` keys**,
  asserting each equals the legacy scan of that file; (ii) *misses*: for each universe
  field, run the legacy oracle **only over files containing the `->field` or `.field`
  substring** — this prefilter is the legacy scanner's own `has_field` line-check hoisted
  to file level, hence provably outcome-preserving — and assert the index agrees (including
  absent-key == empty). The prefilter scopes the **whole** universe, clause (a) included.
  Universe × all-files × full-scan is forbidden: it is O(fields × files × lines) relocated
  into the test suite, whose predictable end state is an `#[ignore]`d gate.
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
  `docs/features/language-coverage/language-expansion-plan.md` Phase 3).
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
  (default: a fresh temp dir per run); `--timeout <s>` per repo. **Default-repo resolution
  (r2-8):** prism = the script's own checkout; others under `$PRISM_BENCH_REPOS`
  (default `~/code/bench-repos`); an absent path emits a `missing` status row, never an
  error. **Command templates (r2-8):** cold =
  `prism nav --cache-dir <fresh-subdir> repo-map --repo <path> --format json` (build +
  cache write); warm = the **identical command repeated immediately** (cache hit).
  **Portability (r2-12):** requires GNU `timeout` (brew coreutils) and macOS
  `/usr/bin/time -l`; the script checks both up front and exits with an instructive
  message; RSS is `maximum resident set size` bytes ÷ 1048576, reported as integer MB.
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
| A | During a cold release build (`prism nav --no-cache repo-map` on **tokio**), `sample <pid> 10 -file out.txt` shows `all_functions`/`ts_query` frames in **<1% of samples** (today: dominant); equivalence + parity + fallback tests green (fallback-fire count == 0); goldens byte-identical |
| B1 | Same sampling during a cold build of **hugo**: the **legacy symbol `resolve_struct_field_assignment` at ≈0% of samples** (it has no production caller post-B1; r2-6b — the per-line core `line_field_targets` is a *distinct symbol* and MAY legitimately appear during the one-time index build, so it is excluded from the gate); differential oracle test green (both halves of the pairing rule); goldens byte-identical |
| C | Exact-order serial==parallel equality + **byte-level cache parity** green; loader parity green; goldens byte-identical; **full-command** `user`/`wall` ratio on a cold hugo build **≥ 1.5** (today ≈ 1.0; the per-phase ratio is unobservable from `/usr/bin/time`, r2-6a — the full-command ratio is the gate, with the caveat that serial phases dilute it) |
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
  (`docs/archive/analysis/prism/prism-meta-analysis-2026-06-10.md` §4); implementors mutate the tree mid-task and
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
