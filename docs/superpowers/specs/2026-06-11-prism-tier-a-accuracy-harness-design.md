# Tier-A Accuracy Harness + Dev-Loop Test-Time Reduction — Design

**Date:** 2026-06-11 · **Status:** rev 1 — pending owner review + dual spec-review
**Context docs:** `docs/prism-meta-analysis-2026-06-10.md` (§1 three-tier harness, §3 LSP
head-to-head prototype), `docs/cpg-substrate-analysis-2026-06-10.md` (S3 precision),
`docs/prism-query-layer/s1-followups.md` (items 1 — B2 trigger, 4 — test wall time).

## 0. Why now

Two contracts depend on this work existing:

1. **B2** (AST-based Level-4 extraction, quirk retirement) is "scheduled only once the
   Tier-A harness is live" (s1-followups item 1). Until Tier-A exists, B2 cannot start.
2. **S3** (call-resolution precision floor) must be *measured, not asserted*
   (meta-analysis §1): today its motivation rests on one hand-run 8-symbol experiment.

The hand-run experiment (meta-analysis §3) is the prototype this harness automates. Its two
headline findings are the harness's calibration targets: prism's resolution is **bimodal**
(P/R ≈ 1.0 on repo-unique names; P = R = 0.0 on the collision-prone method `target`), and
the **oracle itself has a blind spot** (rust-analyzer is build-config-scoped and missed two
true `#[cfg(feature = "mcp")]` callers that prism correctly found) — hence the core
contract: **diffs are adjudicated, not auto-failed**.

A second, unrelated drag on every future initiative rides along as Work Package 2: the
debug test suite spends ~21 minutes wall on compile/link of 121 `[[test]]` binaries while
actual test execution sums to under 2 minutes (s1-followups item 4).

## 1. Scope

| | In scope (v1) | Deferred (§8) |
|---|---|---|
| **WP1** | Tier-A edge-level accuracy harness in `eval/`: 4 measurements, 3 languages (Rust, Go, Python), 4 corpora, adjudication store, committed baseline scorecards, `prism nav functions` CLI surface, agent guidance in `CLAUDE.md`/`AGENTS.md` | TS/JS oracles, module-edge oracle, SCIP-as-oracle adapter, kubernetes/TypeScript-repo scale runs, direct xAST case reuse, CI wiring |
| **WP2** | Umbrella test-binary consolidation (121 → ~24 targets) + `[profile.dev]` debug tuning, measured against a defined protocol | cargo-nextest |

Tier-B (flow-level taint fixtures) and Tier-C (end-task review benchmark) are separate
future initiatives, unchanged by this spec.

## 2. WP1 — Tier-A accuracy harness

### 2.1 Home, toolchain, seams

- `eval/` directory at repo root: a **uv-managed Python 3.12 project** (`eval/pyproject.toml`,
  `uv.lock` committed). Runtime dependencies: stdlib only. Dev dependency: `pytest` for
  harness self-tests. No coupling to the Rust workspace; `cargo` never builds or tests it.
- Two hard seams, defined as Python protocols (PEP 544) in `eval/tier_a/interfaces.py`:
  - **`Oracle`** — `inventory(corpus) -> list[FunctionDef]`,
    `callers(loc) -> list[CallEdge]`, `callees(loc) -> list[CallEdge]`,
    `definition_at(site) -> Location | None`, plus `version() -> str`.
  - **`SystemUnderTest`** — `inventory(corpus)`, `callers(symbol_seed | loc_seed)`,
    `callees(...)` returning the same dataclasses.
  Sampling, comparison, adjudication, and reporting code depend only on these protocols.
  This is the swap seam for multilspy, SCIP indexes, a Rust rewrite, or a non-prism SUT —
  none of which are v1 scope.
- v1 implementations:
  - `PrismCli(SystemUnderTest)` — subprocess calls to a release-built `prism` binary:
    `nav functions/callers/callees ... --format json`. Binary path from config; the
    harness records `prism --version` + repo commit in every report.
  - `RustAnalyzerOracle`, `GoplsOracle`, `PyrightOracle` — per-server adapters over one
    shared LSP client (§2.2).

### 2.2 LSP client and per-server adapters

`eval/tier_a/lsp_client.py`: a single stdlib JSON-RPC-over-stdio client (~200 lines):
`Content-Length` framing, request/response correlation, notification handling, per-request
timeout, server lifecycle (`initialize`/`initialized`/`shutdown`/`exit`).

Methods used: `textDocument/documentSymbol` (inventory),
`textDocument/prepareCallHierarchy` + `callHierarchy/incomingCalls` +
`callHierarchy/outgoingCalls` (measurement 2), `textDocument/definition` (measurement 3).

Per-server adapter responsibilities (the part that genuinely differs per server):

- **Readiness:** rust-analyzer and gopls report indexing via `$/progress`; the adapter
  waits for quiescence (no active progress tokens + a settle delay) before issuing
  queries, with a hard cap (config, default 300 s) after which the run proceeds and the
  report records `oracle_not_quiescent: true`. pyright readiness is per-file
  (`didOpen` + first diagnostics or a settle delay).
- **Capability quirks:** servers that return no `prepareCallHierarchy` item for a valid
  position, per-server symbol-kind mapping (`SymbolKind.Function/Method/Constructor`),
  and `containerName`/hierarchical `documentSymbol` differences are normalized in the
  adapter, never in the comparison code.
- **Failure honesty:** a request that times out or errors is recorded as
  `oracle_error` for that probe and **excluded from P/R denominators**; per-corpus
  `oracle_error_rate` is a first-class report field. The run never silently converts an
  oracle failure into a prism failure.

Oracle install/prep is documented in `eval/README.md` (rust-analyzer already on PATH;
`go install golang.org/x/tools/gopls@latest`; `npm i -g pyright` for
`pyright-langserver`). The harness records each oracle's version string in the report.

### 2.3 New prism CLI surface: `nav functions`

A `Functions` variant added to the `NavQuery` enum (`src/main.rs`), wired to the S1
`FunctionTable`:

```bash
prism nav functions --repo <path> --format json
```

Emits one record per function across the indexed repo:
`{file, name, start_line, end_line, kind}` where `kind` is the tree-sitter node kind
string already held by the table (`kind_id` resolved to its name). Names that are `None`
in the table (anonymous functions) are emitted with `"name": null`. Sorted by
`(file, start_line)` — deterministic like every other prism output. No other fields in
v1 (no params, no bytes): this is the minimal surface measurement 1 needs, and the
additive-only rule from the S1/Option-C lineage applies (existing outputs are untouched).

### 2.4 Measurement 1 — inventory diff (definition coverage)

For each corpus: collect the oracle-side inventory (`documentSymbol` over every source
file of the corpus language; keep Function/Method/Constructor kinds) and prism's
inventory (`nav functions`). Match records by `(file, name, start_line)` with a
±1-line tolerance (servers and tree-sitter disagree on whether attributes/decorators
belong to the definition).

Outputs: prism-missing (oracle has it, prism doesn't — indexing recall gaps),
prism-extra (prism has it, oracle doesn't — e.g. feature-gated/`cfg`'d code, macro
artifacts), and the **collision statistics** (definitions per name, free-fn vs method)
that define measurement 2's strata *from the oracle side* — so stratification and recall
cannot be biased by prism's own indexing blind spots.

### 2.5 Measurement 2 — stratified callers/callees precision & recall

**Strata** (assigned from the oracle inventory of §2.4):

| Stratum | Definition |
|---|---|
| `U-free` | name has exactly 1 definition in corpus; free function |
| `U-method` | exactly 1 definition; method |
| `C-name` | ≥ 2 definitions; sampled symbol is a free function |
| `C-method` | ≥ 2 definitions; sampled symbol is a method (the `target`-class failure mode) |
| `Q-scoped` | free function defined in a nested module/package (callers must qualify: `mod::f`, `pkg.F`, `module.f`) |

**Sample:** 8 symbols per stratum per corpus (40 total; a stratum with fewer than 8
eligible symbols takes all of them and the report records the shortfall). Selection is
deterministic: inventory sorted by `(file, start_line, name)`, then `random.Random(seed)`
with the seed fixed in `eval/corpora.toml` (default 42). Same seed ⇒ same sample ⇒
reproducible numbers.

**Probes:** for each sampled symbol, oracle `incomingCalls`/`outgoingCalls` at its
definition vs `prism nav callers/callees` seeded by symbol+file (file disambiguation
always supplied — the prototype showed unseeded common names correctly `AmbiguousSymbol`;
that contract is pinned by a dedicated probe, not sampled).

**Matching, two granularities, both reported:**

- **Call-site level (primary):** a prism evidence location `(file, line)` matches an
  oracle `fromRange` if the line falls within the range's line span. Site-level
  TP/FP/FN ⇒ precision/recall per stratum per direction.
- **Caller-function level:** the set of containing functions (derived by line containment
  against the oracle inventory). Coarser, robust to multi-site counting differences
  (prototype: `all_functions` 62 sites vs 56 callers).

### 2.6 Measurement 3 — site-level definition spot-check

From measurement 2's prism caller results, sample (same seeded RNG) up to 20 claimed call
sites per corpus and ask the oracle `definition_at(site identifier)`. Prism evidence is
line-granular, so the harness locates the column by finding the seed symbol's name token
on the claimed line (first occurrence outside a string/comment is sufficient; if the name
does not occur on the line at all, that is itself recorded as a confirmed FP). If the
oracle's definition for the identifier at that position is not the sampled seed symbol's
definition,
the edge is a confirmed false positive (the petgraph-`.target()` class is mechanically
confirmed this way rather than hand-read). This is the direct S3 edge-precision-floor
metric, at spot-check cost instead of full-census cost (meta-analysis approach B was
rejected for cost and zero recall signal).

### 2.7 Measurement 4 — resolution capability matrix (xAST-inspired)

The pattern imported from xAST/YASA (see §9): capability-indexed micro-cases with
**ground truth by construction**, scored as a per-capability report, used as the
post-change regression target and as guidance when adapting languages. No LSP involved.

- Layout: `eval/fixtures/<lang>/<capability>/` — each a minimal source tree (2–4 tiny
  files) plus `expected.toml`:

  ```toml
  [case]
  language = "rust"
  capability = "method_cross_file_type_ne_stem"
  status = "known_fail"        # or "pass" — current truth, set at first run
  [seed]
  symbol = "process"
  file = "src/worker.rs"
  [[expect.callers]]
  file = "src/main.rs"
  line = 9
  [expect]
  exact = true                  # extras count as failures
  ```

- Scoring: run `prism nav callers` (and `callees` where the case declares them) on the
  fixture repo; pass iff the result set exactly matches `expect` (when `exact = true`).
  Cases marked `status = "known_fail"` document current gaps (they are the S3/B2 flip
  list); a `pass` case regressing to fail **fails the matrix run**; a `known_fail` case
  starting to pass is reported as "flip candidate — update status".
- v1 capability sets (~10–12 per language), chosen to bracket the documented resolution
  gaps:
  - **Rust:** free fn same-file; free fn cross-file via `use`; `mod::free_fn` qualified;
    inherent method same-file; inherent method cross-file, type name = file stem;
    inherent method cross-file, type name ≠ file stem; trait method static dispatch;
    trait method `dyn` dispatch; `Type::method` qualified call; closure call; common-name
    collision (two `process` in different modules); method call on field receiver.
  - **Go:** same-package free fn; cross-package `pkg.Fn`; struct method same-file; struct
    method cross-file; interface method dispatch; embedded-struct promoted method;
    closure; common-name collision.
  - **Python:** module-level fn; `import module` + `module.f` call; `from m import f as g`
    alias; class method same-file; method via instance cross-file; inherited-method
    override; decorator-wrapped fn; closure; common-name collision.
- The matrix needs no oracle servers and finishes in seconds — it is the cheapest
  in-loop regression signal (`--matrix-only`, §2.11).

### 2.8 Adjudication

Committed store `eval/adjudications.jsonl`, one record per adjudicated diff:

```json
{"corpus": "prism", "measurement": "callers", "symbol": "module_deps",
 "site": "src/mcp/tools.rs:157", "direction": "prism_only",
 "verdict": "oracle_miss", "reason": "mcp feature-gated; rust-analyzer is build-config-scoped",
 "adjudicated_by": "wesley", "date": "2026-06-11"}
```

- `direction`: `prism_only` | `oracle_only`. `verdict`: `oracle_miss` (prism right,
  oracle blind — counts as TP in corrected metrics, increments `oracle_miss_count`),
  `prism_fp`, `prism_fn`, `oracle_artifact` (e.g. macro-expansion phantom — excluded from
  both denominators), `ambiguous` (excluded, listed).
- Reports always show **raw** and **adjudication-corrected** P/R side by side.
  Unadjudicated diffs are never silently counted toward either tool: they appear in a
  "pending triage" section of the report, and corrected metrics exclude them
  (raw metrics count them conventionally: `prism_only` = FP, `oracle_only` = FN).
- Keying is `(corpus, measurement, symbol, site)`; records survive re-runs; a record whose
  site no longer appears in a run is reported as `stale` (corpus SHA changed) rather than
  deleted.
- The two known feature-gated callers from the prototype (`mcp/tools.rs` caller of
  `module_deps`, `mcp/session.rs::bootstrap` caller of `load_repo`) are seeded into this
  file as part of v1 — the harness must rediscover both diffs (gate G2).

### 2.9 Corpora and reproducibility

`eval/corpora.toml` (committed):

| Corpus | Lang | Path | Oracle | Prep |
|---|---|---|---|---|
| `prism` | Rust | the repo itself | rust-analyzer | none |
| `tokio` | Rust | `~/code/bench-repos/tokio` | rust-analyzer | none |
| `hugo` | Go | `~/code/bench-repos/hugo` | gopls | `go mod download` |
| `django` | Python | `~/code/bench-repos/django` | pyright | venv + pyright config |

Each entry carries: absolute path (machine-local; the file documents this is a
dev-machine harness, not CI), **pinned commit SHA** (recorded at first run; the runner
warns and records `corpus_dirty: true` if HEAD differs), sample seed, per-corpus caps.
Every report records: corpus SHA, prism commit + version, oracle name + version, seed,
harness git SHA, wall time per measurement.

Known asymmetry, stated in reports: Rust/Go oracle quality is compiler-grade;
**pyright-on-django numbers carry oracle noise** (type-inference gaps on a large dynamic
codebase). Django's report section is labeled accordingly, and `oracle_error_rate` +
`ambiguous` adjudications are expected to be materially higher there. That noise floor is
itself a v1 finding (meta-analysis §1 uncertainty, now quantified).

### 2.10 Reports and baseline

`eval/tier_a/report.py` renders, per corpus, `docs/eval/tier-a/<date>-<corpus>.md` +
`.json` (the JSON is the machine-readable record: run metadata block + all four
measurements with raw/corrected metrics per stratum), plus a roll-up
`docs/eval/tier-a/baseline.md` (per-language summary table + capability-matrix grid +
pending-triage counts). The first full run, adjudicated, committed = **the S3/B2
baseline**. Subsequent runs are new dated files; `baseline.md` is updated only
deliberately (it is the comparison anchor, not a rolling log).

### 2.11 Runner and quick modes

```bash
uv run tier-a --corpus all              # full run, 4 corpora (the baseline command)
uv run tier-a --corpus prism            # one corpus
uv run tier-a --quick                   # prism corpus, 3/stratum, + matrix (minutes)
uv run tier-a --matrix-only             # capability matrix only, no LSP (seconds)
uv run tier-a --report-only <run.json>  # re-render reports from a stored run
```

**Agent guidance:** a short section added to `CLAUDE.md` and a new `AGENTS.md` (codex
reads it): *when a change touches call resolution, navigation queries, or CPG
construction (`src/call_graph.rs`, `src/navigation/`, `src/cpg/`, `src/ast.rs`), run
`uv run tier-a --matrix-only` before committing and `--quick` before requesting review;
paste regressions into the PR/report rather than re-baselining.* Full multi-corpus runs
remain human-triggered (they need LSP servers + bench-repos present).

### 2.12 Harness self-tests

`eval/tests/` (pytest, run host-side via `uv run pytest`):

- **Comparison/metrics unit tests** against `FakeOracle`/`FakeSut` returning canned
  dataclasses: site/function-level matching incl. line-span tolerance, stratum
  assignment, P/R math, adjudication application (each verdict class), pending-triage
  exclusion, stale-record detection, deterministic sampling (fixed seed twin-run).
- **LSP client framing tests** against a scripted Python echo-server subprocess speaking
  JSON-RPC over stdio (framing, correlation, timeout, notification routing). No real
  language servers in self-tests.
- **Matrix runner test** against one tiny committed fixture using `PrismCli` if the
  binary is present, else skipped (`pytest.mark.skipif`) — the only self-test touching
  prism.

## 3. WP2 — test-suite wall-time reduction

Independent of WP1; own commits; own gate. Two stacked changes:

### 3.1 Umbrella test binaries (121 → ~24)

One `[[test]]` target per existing `tests/` subdirectory (the 24 directories enumerated
in the plan), each with a generated-once-by-hand `main.rs` declaring its sibling files as
`mod`s. Mechanics:

- Existing files keep their paths and contents except: the per-file
  `#[path = "../common/mod.rs"] mod common;` headers move to **one** declaration in the
  umbrella `main.rs`; files reference it as `use crate::common;`. (Two sibling files
  declaring `mod common` in one crate would double-compile and double-define it.)
- `tests/common` then compiles ~24× instead of 121×, and the linker runs ~24× instead of
  121× against the full prism lib + 11 grammars — the dominant cost in the 21-minute
  number.
- `tests/mcp` umbrella keeps `required-features = ["mcp"]`. `tests/frameworks`,
  `tests/navigation`, etc. follow the plain pattern.
- **No file merging.** The 600-line file rule is untouched (owner allows loosening to
  800–1000 if ever needed; this change does not need it — umbrella `main.rs` files are
  ~20 lines of `mod` declarations).
- Invocation changes: `cargo test --test algo_paper` →
  `cargo test --test algo_paper -- <filter>` no longer exists; the new form is
  `cargo test --test <dir-target> <module>::` (e.g. `cargo test --test algo_taxonomy
  taint_cve::`). CLAUDE.md's Build & Test section and the named-suite examples are
  rewritten; `scripts/extract_tests.py`, `scripts/generate_coverage_badges.py`, and CI
  workflow files are swept for `--test <old-name>` references.
- `tests/integration/coverage_test.rs`'s three `all_test_files` path lists reference
  **file paths, not target names**, and all files survive — verified by
  `cargo test --test integration coverage::` (new name) passing unchanged.

### 3.2 Profile tuning

```toml
[profile.dev]
debug = "line-tables-only"
```

Test profile inherits dev. Backtraces keep file:line; only interactive debugger
variable inspection degrades (acceptable; nobody debugs prism with lldb variable views
today — if that changes, a `[profile.dev-full]` alias is a one-liner). Applied as a
separate commit so its contribution is measured independently of 3.1.

### 3.3 Measurement protocol and gate

Measured on the dev machine (M-series, same conditions as the 21-minute observation),
before WP2, after 3.1, and after 3.1+3.2:

- **P1 (clean):** `cargo clean && time cargo test` — worst case.
- **P2 (dev loop):** `touch src/lib.rs && time cargo test` — the loop that hurts.

Single timed run per point, machine otherwise idle (matches how the 21-minute baseline
was observed); wall, user, and sys recorded.

**Gate:** P2 < 8 minutes. P1 and P2 reported for all three measurement points. If the
gate is missed, the numbers are reported honestly with attribution (S1 row-C precedent)
and nextest/linker options move from deferred to proposed — no silent waiver.

## 4. Acceptance gates (objective)

| # | Gate |
|---|---|
| G1 | **Prototype reproduction:** harness on `prism` corpus shows bimodality — site-level P and R ≥ 0.95 on `U-free`/`U-method` strata, and the `C-method` stratum surfaces the `target`-class failure (P ≤ 0.2 or R ≤ 0.2 raw) |
| G2 | Both known feature-gated callers surface as `prism_only` diffs and match the seeded `oracle_miss` adjudications |
| G3 | **Determinism:** two consecutive runs, same seed + same corpus SHA ⇒ identical samples and identical raw metrics |
| G4 | Baseline committed: 4 corpora × measurements 1–3 + capability matrix for 3 languages, with run metadata (SHAs, versions, seed, wall times) in every report |
| G5 | Capability matrix runs green: every fixture executes; statuses assigned (`pass`/`known_fail`); zero unexpected regressions by definition at first commit |
| G6 | WP2: full suite passes post-consolidation; coverage-matrix test passes unchanged; stale `--test` references swept (repo-wide grep clean) |
| G7 | WP2: P2 < 8 min; P1/P2 reported at all three measurement points; honest report if missed |
| G8 | `CLAUDE.md` + `AGENTS.md` agent guidance landed; `eval/README.md` covers oracle install/prep end-to-end |

## 5. Execution model

Per the S1 ritual: TDD plan via writing-plans; codex gpt-5.5 as containerized implementor
through a2a-bridge (`--config examples/a2a-bridge.slicing-implement.toml`), dual diff
review per slice, squash-merge ritual. One split is new: the a2a container is
egress-locked and has no LSP servers or bench-repos, so **containerized verify covers
the Rust changes; the eval/ pytest suite runs in-container only if the image provides
Python ≥ 3.10 + pytest (checked at kickoff), otherwise it joins the live-oracle runs,
baseline generation, and WP2 timing measurements as host-side orchestrator-run gates**
(same pattern as S1's bench-ladder).

Sequencing inside the initiative: WP2 first (its 5× link-count cut pays for itself
immediately across WP1's own Rust-side commits), then WP1.

## 6. Risks and mitigations

| Risk | Mitigation |
|---|---|
| LSP automation flakiness (readiness, hangs) | quiescence detection + per-request timeouts + `oracle_error` accounting (§2.2); failures excluded from denominators, never converted to tool failures |
| pyright noise on django pollutes the picture | labeled section, `oracle_error_rate` + `ambiguous` first-class; Rust/Go carry the precision claims; noise floor itself is a deliverable |
| gopls/hugo or rust-analyzer/tokio resource cost | per-corpus caps in `corpora.toml`; quiescence cap with `oracle_not_quiescent` flag; corpora are warm after first index |
| Small samples → wide confidence | report Wilson 95% intervals next to point estimates; v1 is a baseline, not a hypothesis test; sample sizes are config, not code |
| Harness bugs self-confirming prism | G1 reproduction gate against hand-verified prototype numbers; self-tests with fakes (§2.12) |
| Oracle/corpus drift over time | versions + SHAs recorded in every report; `corpus_dirty` and `stale` adjudication flags |
| `nav functions` surface creep | minimal field set, additive-only, deterministic ordering (§2.3) |
| WP2 consolidation breaks feature-gated or scripted invocations | `required-features` preserved; repo-wide sweep gate G6 |

## 7. Decisions log (owner)

- v1 languages: **Rust + Go + Python**; TS/JS deferred *with priority* — node backends and
  react/vue/angular frontends make it the most prevalent follow-up (owner, 2026-06-11).
- Stack: **Python via uv in `eval/`, seams for multilspy/Rust/SCIP later** (owner).
- Posture: **re-runnable tool + committed baseline; no CI wiring; agent guidance in
  `.claude`/codex docs** (owner).
- WP1 methodology: **hybrid** (inventory diff + stratified P/R + definition spot-check +
  capability matrix) — "they complement and cover each other's weaknesses" (owner).
- WP2: **A (umbrella binaries) + B (profile tuning)**; LOC rule may loosen to 800–1000 if
  needed (not expected to be needed) (owner).

## 8. Deferred (documented follow-ups)

1. **TS/JS oracle + corpora** — highest-priority follow-up (prevalence rationale above);
   tsserver/typescript-language-server client friction is the known cost; xAST's Node.js
   SAST set is candidate fixture input.
2. **Module-edge oracle** — no clean LSP analogue; candidate: language-native tools
   (`go list -deps`, `cargo metadata`, import graphs) per language.
3. **SCIP-as-oracle adapter** — drop a `ScipOracle` behind the `Oracle` seam (meta-analysis
   §3 stage-1 complement; offline, no server lifecycle).
4. **Scale runs** — kubernetes (gopls), TypeScript repo, rust-analyzer-on-rust-analyzer.
5. **xAST direct reuse** — Tier-B input (taint-oriented cases; Java/Node.js coverage).
6. **cargo-nextest** — execution UX; does not attack link cost (only if G7 misses).
7. **CI smoke gate** — revisit once the harness has a stable history.

## 9. Related work note (xAST / YASA-Engine)

Reviewed 2026-06-11 (owner-suggested): xAST
(`alipay/ant-application-security-testing-benchmark`) is a capability-indexed evaluation
system — engine capabilities vs rule capabilities, cases mapped 1:1 to evaluation items,
"physical examination report" output instead of aggregate scores. YASA-Engine uses it as
the regression target for post-change testing and as syntax-support guidance during
multi-language adaptation. Its SAST case sets cover **Java and Node.js** (no Rust/Go/Py),
and cases are taint/vulnerability-oriented — wrong tier and wrong languages for direct
v1 reuse, hence: pattern imported as measurement 4 (§2.7), corpus deferred to Tier-B/§8.
