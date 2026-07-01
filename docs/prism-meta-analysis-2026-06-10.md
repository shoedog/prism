# Prism Meta-Analysis: Measurement, Scale, Adoption Seams, and Operational Truth

**Date:** 2026-06-10 (companion to `docs/features/cpg/substrate-analysis-2026-06-10.md`)
**Scope:** The seven questions the substrate analysis surfaced but was not asked: (1) accuracy
ground truth, (2) LLM-consumer value & ergonomics, (3) build-vs-adopt for the precision tier
(SCIP/LSP), (4) mid-session staleness, (5) the scale ceiling, (6) packaging robustness,
(7) indexing policy. Per owner direction, (5) and (3) — large-repo/cross-repo and SCIP
integration — are treated as first-class.

**Method:** Everything here is empirical unless marked otherwise. New experiments run for this
analysis: a rust-analyzer ground-truth comparison over 8 symbols (LSP `incomingCalls` vs
`prism nav callers` on this repo); a SCIP index generation of this repo; a scale-ladder
benchmark (prism → tokio → hugo → django → rust-analyzer → TypeScript → kubernetes, shallow
clones under `~/code/bench-repos/`); payload-size measurements; and code reads of the session,
cache, and loader layers. Uncertainties are called out inline and collected in §9.

---

## 1. Accuracy ground truth — prism cannot currently measure its own accuracy

### Findings

- Prism's test suite pins **behavior** (byte-identical goldens, fixture expectations), not
  **correctness against an external referee**. There is no precision/recall measurement
  anywhere in the repo for nav edges, call resolution, or taint verdicts.
- A ground-truth oracle is cheaply available and works: this analysis hand-ran one in ~30
  minutes using rust-analyzer's `incomingCalls` (full results in §3). It immediately produced
  a quantified, bimodal accuracy picture that six months of golden tests had not surfaced.
- A second, end-task oracle already exists **on this machine**: `~/code/code-review-benchmark`
  (Martian Code Review Bench) — 50 PRs across Sentry (Py), Grafana (Go), Cal.com (TS),
  Discourse (Ruby), Keycloak (Java) with human-verified golden comments, an LLM judge, and a
  documented "add your tool in an afternoon" pipeline.

### Conclusion: build a three-tier harness

| Tier | Oracle | Measures | Cost | Notes |
|---|---|---|---|---|
| **A — edge-level** | LSP servers as referee (rust-analyzer, gopls, tsserver/pyright, clangd) over a stratified symbol sample (unique names / common names / methods / scoped calls) | per-language precision & recall of `callers`/`callees`/module edges | ~1 week for Rust+Go+Py automation | The §3 experiment is the prototype. Diffs must be **adjudicated, not auto-failed**: the oracle itself is build-config-scoped (it missed two true `mcp`-feature-gated callers that prism correctly found) |
| **B — flow-level** | labeled taint fixtures: existing CVE corpus + new reachable/unreachable sink pairs per language | `taint_reaches` tri-state correctness (esp. false `NotReached`) | 2–3 days seed, grows with Plan B | Becomes the acceptance gate for Tier-2; pins the "honest tri-state" property, which goldens cannot |
| **C — end-task** | code-review-benchmark golden PRs | does prism-assisted review beat unassisted review (precision/recall of found issues, tokens, latency) | 2–4 days wiring | 4 of its 5 repos are prism-supported languages (Ruby is not). This is the only tier that measures *value*, not just correctness |

**Reasoning:** accuracy work without Tier A is unfalsifiable (the substrate analysis's S3
recommendation rests on two hand-found anecdotes); Tier-2's epistemics demand Tier B (a wrong
`NotReached` is worse than no tool); and Tier C is the only defensible answer to "should this
exist" (see §2). Sequencing: Tier A before/alongside the S3 precision work so the fix is
measured, not asserted.

### Uncertainty

- LSP-as-oracle coverage of *dynamic* languages is weaker (pyright/tsserver type inference
  gaps); Tier A numbers for Python/JS will carry oracle noise that Rust/Go numbers do not.

---

## 2. LLM-consumer value & ergonomics — the output is correct-ish but expensive and lossy

### Measured findings (this repo, release build)

| Query | Payload | ≈Tokens | Note |
|---|---|---|---|
| `callers --symbol all_functions` (62 items) | 39.5 KB | ~10k | ~160 tokens per caller; the same information as prose is ~3.5 KB |
| `repo-map` (321 files, 1058 edges) | 148 KB | ~37k | **No ranking, no token budget, no truncation applied** (`truncated: false`) — one orientation call can consume a third of a context window |
| `nodes-at` (one line) | small | — | fine |

- **Debug-format leak:** the wire `path` field is `format!("{path:?}")` →
  `"AccessPath { base: \"x\", fields: [] }"` (`src/navigation/queries.rs:60, :406`) instead of
  the `Display` form (`x`, `dev->name`). Plan A §5 already mandates `AccessPath::Display` for
  the reasoning shaper — nav predates that rule and was never aligned. Pure waste + parse
  burden for the consumer. (Cheap fix; wire-breaking for nav goldens, so batch it with the
  S2 `CACHE_VERSION` bump.)
- **`snippet: Option<String>` exists in the schema and is `None` at every construction site**
  (all 7 sites). An agent that gets a caller list must then Read each file:line — the
  round-trips the research literature says precise indexes should *save* (Sourcegraph's
  vendor-reported ~4k vs ~48k token comparison). One line of context per item would
  roughly double payload size but eliminate most follow-up reads; make it opt-in
  (`include_snippets`).
- Caller items repeat near-identical `why` arrays per call site; multi-site callers appear as
  duplicate items (e.g. `test_degraded_seed_line_warns_once_per_line` twice). Compact
  encoding (one item, `call_site_lines: [..]`) would cut the callers payload roughly in half.
- Two more, observed over the live MCP server: `concise` verbosity clears `why` but still
  ships the empty `"why": []` array per item; and **truncation at `max_results` drops items
  in path-alphabetical order, not relevance** (every `score` is 1.0) — a truncated caller
  list can show only test callers while silently dropping production ones. Ranking before
  truncation (and the §7 `is_test` label) fixes both.

### Conclusions

1. **Adopt a token budget per tool** with relevance ranking — for `repo-map`, aggregate to
   directories and rank (the Aider PageRank approach is the established pattern; the research
   doc's Tier-2 evidence). Target: orientation ≤ ~2k tokens by default, expandable.
2. Fix the Debug leak and item duplication in the same wire-touching change as substrate S2.
3. Add opt-in snippets.
4. **Run Tier C before investing further**: the research literature's negative results
   (context that isn't reliably relevant *hurts*) apply to prism's own output. The
   value-of-prism A/B (Claude Code ± prism-mcp on the 50 golden PRs) is the decision gate the
   roadmap currently lacks.

### Uncertainty

- Token-economics numbers are from prism-on-prism; payload growth on larger repos is linear
  in result count, but agent *behavior* (how many follow-up reads a snippet saves) is
  unmeasured here — that is exactly what Tier C measures.

---

## 3. Build-vs-adopt for the precision tier — measured head-to-head, and the answer is hybrid

### The experiment

rust-analyzer (1.94.0) `incomingCalls` as ground truth vs `prism nav callers`, 8 symbols
chosen to stratify name-collision risk; this repo; full agreement tables verified by hand.

| Symbol | Class | prism | ground truth | Precision | Recall | Verdict |
|---|---|---|---|---|---|---|
| `build_cfg_edges` | unique free fn | 15 | 15 | 1.0 | 1.0 | exact |
| `resolve_callees_qualified` | unique method | 6 | 5 callers / 6 sites | 1.0 | 1.0 | exact |
| `taint_trace` | unique method | 31 | 30 callers / 31 sites | 1.0 | 1.0 | exact |
| `all_functions` | unique hot method | 62 | 56 callers (multi-site) | ~1.0 | ~1.0 | agree |
| `module_deps` | unique free fn | 6 | 5 | 1.0 | 1.0 | **prism found a 6th TRUE caller** (`nav_module_deps`, `mcp/tools.rs:157`) that rust-analyzer missed — `mcp` feature off by default |
| `load_repo` | unique free fn | 21 | 20 | 1.0 | 1.0 | **+1 true feature-gated caller again** (`mcp/session.rs::bootstrap`) |
| `slice` (no `--file`) | very common fn | `AmbiguousSymbol` error | 2 | — | — | safe-fails; good contract |
| `slice --file original_diff.rs` | very common fn | 2 | 2 | 1.0 | 1.0 | exact |
| `target` | common **method** name | 19 | 2 | **0.0** | **0.0** | all 19 are petgraph `.target()` receiver calls mis-attributed; both real callers (`TaintSeed::target(..)`) missed because the type name ≠ file stem |

**[2026-06-12 amendment: superseded by the Tier-A baseline (`docs/eval/tier-a/baseline.md`) — precision claims held, recall claims did not survive callee-direction/qualified-call measurement at scale.]** **Reading:** prism's resolution is **bimodal** — perfect on repo-unique names and on
file-disambiguated seeds; total failure on collision-prone *method* names, where it exhibits
both documented gaps at once (receiver-call capture → false positives; `Type::method` with
type ≠ file stem → false negatives). The unseeded call-derived surfaces (module-deps,
repo-map, Step-5b interprocedural edges) inherit the worst case because they resolve every
call site without a seed filter.

**The counter-finding matters equally:** the compiler-accurate oracle has a *structural*
blind spot prism does not — it only sees the **active build configuration**. Both
`#[cfg(feature = "mcp")]` true callers were invisible to rust-analyzer and found by prism.
The same applies to platform-gated code, and to repos that don't currently build (mid-review
diffs, WIP branches — prism's primary use case).

### SCIP generation, measured + ecosystem facts

- `rust-analyzer scip .` on this repo: **14.6s, 14.7 MB index** — same order as prism's full
  CPG build (29s) for compiler-accurate Rust occurrences. *(measured)*
- Ecosystem (from knowledge, not measured here — verify versions when implementing):
  scip-typescript, scip-python, scip-java, scip-go, scip-clang cover the priority languages;
  indexes are protobuf; Apache-2 tooling. **Every indexer requires a working build context**
  — node_modules installed, go modules resolvable, compile_commands.json for clang — and
  full (non-incremental) re-index in the common open-source setups.

### Conclusion: adopt as an optional oracle, never as the substrate

Prism's no-build, all-text, all-cfg property is load-bearing for its actual product (point an
MCP server at any repo, including ones that don't compile, and slice diffs of WIP code). SCIP
cannot replace that. But prism's name-based resolution should never *contradict* a
compiler-accurate index when one is available. The integration seam already exists:
`Source::ExternalIndex { .. }` is a variant in the nav evidence vocabulary today
(`src/navigation/types.rs`), currently unused.

**Recommended shape (staged):**

1. **SCIP reader + edge arbitration (1–2 weeks):** optional `--scip <index>` /
   auto-discovery; where SCIP has occurrence data, it *overrides* name-based call edges
   (kills the `target`-class false positives and recovers `Type::method` recall); where it
   is absent (feature-gated code, unbuildable files, unsupported language), prism's own
   resolution stands, labeled by `Source`/`score` confidence. Per-edge provenance preserved.
2. **Cross-repo via SCIP symbols (later, with the HTTP/multi-repo track):** SCIP symbol IDs
   are package-qualified and version-aware — they are the natural join key for the multi-repo
   roadmap rather than inventing a prism-native cross-repo symbol scheme. This makes the
   deferred multi-repo track largely an *ingestion* problem instead of a resolution problem.
3. **Do not** wire SCIP into the Tier-2 DFG/witness path — taint/reasoning stays on prism's
   CPG (SCIP has no data flow). The boundary is: **SCIP = symbol/call precision oracle;
   prism CPG = flow semantics.**

This directly de-risks Phase-IP: interprocedural traversal can require
`confidence ≥ resolved` edges (SCIP-confirmed or local/static/import-narrowed), with
name-only edges surfacing as `BoundaryExited`-style honesty instead of false `Reached`.

### Uncertainty

- scip-clang/scip-python maturity and index completeness were not tested here; the 14.6s
  number is single-crate Rust. A 2-day spike (generate SCIP for hugo + django + a TS repo,
  measure coverage of prism's call sites) should precede committing to stage 1's scope.
- License/operational fit of running language indexers in CI vs on dev machines unexamined.

---

## 4. Mid-session staleness — the MCP server serves a frozen snapshot, by construction

### Findings (code-verified)

- `SessionProvider::bootstrap` loads the repo and builds/loads the index **once** at server
  start (`src/mcp/session.rs:26-38`); `serve_session` then holds `&NavigationSession` for the
  process lifetime (`src/mcp/transport.rs:56`). **No per-query revalidation, no mtime/hash
  check, no file watcher, no refresh tool.**
- The CLI path is the opposite: every invocation reloads + hash-validates against the cache
  (`build_session` → `build_cached`), so it is always fresh at ~0.5s warm cost.
- Consequence for the core agent loop (edit → re-query): an agent that edits a file and asks
  `nav_callers` gets **pre-edit answers with no warning**. Claude Code MCP servers live for
  a whole session — often hours of edits. This is an *accuracy* failure mode that no CPG
  improvement touches, and it compounds silently with agent trust.

### Conclusions (staged by cost)

1. **Now (hours):** state the snapshot semantics in every tool description ("results reflect
   repo state at server start; re-add the server or use CLI for post-edit queries") — honesty
   first, matching the project's established pattern.
2. **Short (1–2 days):** cheap freshness probe per `tools/call` — stat mtime+size over indexed
   files (~300 files ≪ 10ms; even ~15k files is tens of ms) and on drift attach a
   `StaleIndex` warning naming the changed files. Honest, no rebuild risk.
3. **Then (2–4 days):** auto-refresh on drift via the existing incremental rebuild
   (`build_incremental`), debounced — **but only after fixing the known incremental defect**
   (Phase-3 indirect-call resolution is not re-run on incremental rebuild,
   `src/cpg/build.rs:158-184`), otherwise auto-refresh trades stale-everything for
   stale-indirect-edges.

### Uncertainty

- Whether Claude Code retries/reconnects MCP servers mid-session affects how often a fresh
  process (and thus fresh snapshot) occurs in practice; not measured.

---

## 5. The scale ceiling — measured ladder

Shallow clones, release binary, isolated cache dirs (`/tmp/prism-bench-cache`), query =
`nav repo-map`, cold = full CPG build + cache write, warm = cache-hit query.
LOC counted over prism-supported extensions excluding prism's own skip-dirs.

| Repo | Lang | LOC | Files | Cold build | Peak RSS | Cache | Warm query |
|---|---|---|---|---|---|---|---|
| prism | Rust(+fixtures) | 108k | 321 | 29 s | 0.53 GB | 59 MB | 0.46 s |
| tokio | Rust | 175k | 781 | 89 s | 0.59 GB | 61 MB | 0.72 s |
| hugo | Go | 234k | 938 | **469 s** | **1.97 GB** | 289 MB | 1.63 s |
| django | Python | 551k | 3,035 | **TIMEOUT > 2400 s** | 1.46 GB at kill | — | — |
| rust-analyzer | Rust | 589k | 1,514 | **TIMEOUT > 2400 s** | 1.36 GB at kill | — | — |
| TypeScript | TS | 4.15M | 39,308 | not run to completion (≫ hours by extrapolation) | **4.16 GB at 15-min kill, still climbing** | — | — |
| kubernetes | Go | 3.66M | 13,129 | not run to completion (≫ hours by extrapolation) | **4.34 GB at 15-min kill, still climbing** | — | — |

Findings:

- **The practical cold-build ceiling today is a few hundred thousand LOC.** Hugo (234k Go)
  already takes 7.8 minutes; django (551k Python) and **rust-analyzer (589k Rust — the
  priority language)** both exceeded a 40-minute timeout. The two multi-M LOC repos were
  capped at 15 minutes purely for memory data; by extrapolation their cold builds are in the
  hours-to-tens-of-hours class.
- **Scaling is superlinear in files AND strongly codebase-dependent, not LOC-driven**:
  90 ms/file (prism, 321 files) → 114 ms/file (tokio, 781) → **500 ms/file (hugo, 938)**.
  Hugo has only 1.2× tokio's files and 1.34× its LOC but costs 5.3× the time and 3.3× the
  RSS. The mechanism was confirmed by sampling the running kubernetes build: **100% of the
  sampled window sits in `CallGraph::build` → `ast::resolve_struct_field_assignment`** — the
  Phase-3 *Level-4* pass that rescans source text per unresolved call
  (`call_graph.rs:312-360`). Go's receiver-method density makes most calls "unresolved," so
  the term is `unresolved_calls × files` and dominates everything at scale. Two fixes
  compose: (a) **invert Level 4 into a precomputed index** — one O(files) pass collecting
  `field = func` assignments, replacing per-call repo scans (likely the single biggest
  scale win, ~1–2 days); (b) the §3/S3 precision work shrinks the unresolved set itself —
  call-resolution accuracy and build speed are the same problem at scale.
- **Cache size and warm cost are graph-density-driven, not LOC-proportional**: hugo's cache
  is 289 MB (4.9× prism's) for 2.2× the LOC; warm CLI queries already cost 1.63 s there
  (whole-blob deserialize per invocation). The substrate analysis's S1
  (memoize + parallelize; 15 cores idle today, single-threaded build) is what bends the
  cold curve, but at multi-100k scale the *resident MCP server* becomes the only sane
  serving mode — which makes the §4 staleness fix a hard prerequisite for big-repo use.
- **Memory at multi-M LOC is multi-GB before the build even finishes**: 4.16 GB
  (TypeScript) and 4.34 GB (kubernetes) at the 15-minute kill, still climbing, vs ~2 GB
  peak for hugo. Holding everything (all `ParsedFile` trees + sources + the full petgraph)
  resident simultaneously is the design assumption that breaks first; per-repo federation
  (§3 stage 2) rather than a bigger single graph is the right response.
- **The 2 MB file cap silently amputates real repos**: TypeScript's `src/compiler/checker.ts`
  is 3.15 MB → skipped (`MAX_FILE_BYTES`, `repo_loader.rs:9`). The most-connected file in
  that repo would be absent from every nav answer, recorded only in `LoadedRepo.skipped`,
  which `repo-map` output does not surface. Any "big-repo support" claim must either raise
  the cap (parse cost is linear; tree-sitter handles multi-MB files) or surface the skip in
  every relevant query's warnings.
- Warm-path cost grows with cache size (whole-blob deserialize per CLI call: 0.46→0.72 s).
  At kubernetes scale the warm CLI call may cost several seconds — accept (CLI), or rely on
  the resident MCP process (which is exactly the path with the §4 staleness gap; the two
  issues are coupled: a resident server with freshness probing is the scale answer).

### Cross-repo implications (with §3)

Single-repo whole-blob caching + in-memory petgraph has an architectural ceiling around
"RSS = a few GB, cold = tens of minutes" — adequate for most single services, wrong shape for
org scale. The deferred multi-repo/HTTP track should therefore be *federation over per-repo
indexes* (each repo its own CPG/cache; SCIP symbol IDs as the cross-repo join key per §3),
not a single mega-graph. That keeps the current architecture valid as the per-repo unit and
makes scale a routing problem.

### Uncertainty

- The multi-M LOC repos were not run to completion, so true peak RSS and cache size at that
  scale are extrapolations beyond the 15-minute probes; the timeout rows bound cold-build
  time from below only. Numbers are one machine (M-series, 15 cores, 24 GB), one run each —
  no variance estimate.

---

## 6. Packaging & operational robustness

### Findings (verified today, partly the hard way)

- **Fresh plugin install is broken by default:** the launcher requires a prism-mcp binary in
  the *plugin cache* or on PATH; neither exists after `/plugin install` (the repo checkout's
  binary doesn't count). The failure surfaced in the client as a bare "✘ Failed to connect"
  — the launcher's helpful stderr message never reaches the user. (Worked around this
  session by copying the binary into the plugin cache; after a plugin reload the MCP server
  connected and served `nav_callers` correctly end-to-end, confirming the launcher path was
  the only break.)
- **Stale-binary drift is silent:** the checked-in `target/release/prism` predated the whole
  nav layer (no `nav` subcommand) despite a fresh-looking mtime; nothing detects
  plugin-version ↔ binary-version skew. The pieces for a handshake already exist —
  `serverInfo.version = CARGO_PKG_VERSION` (`transport.rs:213`), `prism/schema_version`
  (`transport.rs:279`), plugin.json `version: 3.1.2` == crate version — they are just never
  compared.
- Cache writes are atomic (`temp + rename`, `cpg_cache.rs:183-186`) and corrupt caches
  rebuild gracefully (tested behavior); concurrent same-repo writers race benignly on a
  fixed `.tmp` name (last-writer-wins, both valid). Low risk.

### Conclusions

1. Launcher: check binary presence **and version** (`prism-mcp --version` vs plugin manifest;
   warn on minor skew, refuse + actionable message on schema mismatch). ~half a day.
2. Distribution: either ship prebuilt binaries per-platform with the plugin (the marketplace
   supports assets) or make the launcher offer `cargo install --path` guidance interactively;
   today's "build it yourself, then reinstall the plugin from that checkout" is a silent-fail
   funnel. 1–2 days.
3. A `prism doctor` subcommand (binary version, schema version, cache dir, cache meta,
   grammar fingerprint) would have made both of today's failures one-command diagnosable.
   ~1 day.

---

## 7. Indexing policy — tests, fixtures, vendored and generated code

### Findings

- Built-in skips exist and are sane: `.git, target, node_modules, vendor, dist, build`,
  hidden dirs, symlinks, >2 MB, >30% parse-error files (`repo_loader.rs:11, 106-159`).
- **Not consulted:** `.gitignore` (untracked build output in non-standard dirs gets indexed),
  any user-configurable ignore (`.prismignore`), generated-code conventions
  (`*_pb2.py`, `*.gen.go`, `*.d.ts` bundles).
- **Test code is indistinguishable from production code in every answer.** Measured: of the
  15 `build_cfg_edges` callers, 14 are tests; `tests/fixtures/c/timer_uaf.c` is a top-20
  edge source in prism's own repo-map. For an LLM asking "what breaks if I change X," test
  callers are *useful but categorically different* — and currently unlabeled.

### Conclusions

1. Respect `.gitignore` + support `.prismignore` (the `ignore` crate is the standard answer
   and is already in the long-term dependency plan). ~1 day.
2. Add `is_test: bool` (path-heuristic per language: `tests/`, `*_test.go`, `test_*.py`,
   `#[cfg(test)]` spans) to evidence items + a `scope: prod|test|all` query param.
   1–2 days, additive field. This is cheap and directly improves agent answer quality —
   "2 production callers, 13 test callers" is a materially better answer than "15 callers."
3. Keep indexing tests by default (they are real evidence) — labeling, not exclusion.

---

## 8. Priority synthesis across all seven

Ordered by leverage-per-week, interleaved with the substrate plan (S1–S4 from the companion
doc remain the top block):

1. **Tier-A accuracy harness** (≈1 wk) — makes S3 and everything after it measurable.
2. **Staleness honesty + freshness probe** (≈2 d) — protects the core agent loop now.
3. **Ergonomics batch** (Debug-leak fix + dedup + token-budgeted repo-map + snippets opt-in,
   ≈3–4 d, wire-touching → batch with S2's cache bump).
4. **Tier-C value A/B on code-review-benchmark** (≈3 d) — the go/no-go evidence for further
   nav investment, and a publishable result either way.
5. **SCIP spike then reader** (2 d spike + 1–2 wk) — gate Phase-IP edge confidence on it.
6. **Packaging handshake + doctor** (≈2 d).
7. **Indexing policy: gitignore/prismignore + is_test labels** (≈2–3 d).
8. **Scale work**: S1 first (it bends the measured curve), then re-run this ladder; defer
   federation until the multi-repo track, but adopt SCIP symbol IDs as its join key now (a
   design decision, zero code).

## 9. Collected uncertainties

- LSP-oracle noise for dynamic languages (§1); agent-behavior deltas unmeasured until Tier C
  (§2); SCIP indexer maturity/coverage outside Rust untested (§3); MCP process lifecycle in
  Claude Code unmeasured (§4); multi-M-LOC peak RSS extrapolated from capped probes (§5);
  marketplace binary distribution mechanics unverified (§6).
- All precision/recall numbers are from **one Rust repo (this one)**; the bimodal pattern's
  *frequency* (how much of a real repo's call mass is collision-prone) will vary by codebase
  and language — exactly what the Tier-A harness should quantify per language.
- Sub-agent survey claims used in the companion doc were spot-verified; one (Rust macro
  blindness) was falsified by experiment. Residual risk remains for unverified per-language
  claims in that doc's F6–F9; the Tier-A harness retires most of it.
