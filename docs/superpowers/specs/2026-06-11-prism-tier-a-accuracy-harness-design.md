# Tier-A Accuracy Harness + Dev-Loop Test-Time Reduction — Design

> **EXECUTION OUTCOME (2026-06-12): SHIPPED.** All 21 plan tasks executed, final
> whole-branch dual review + 8 MAJOR fixes folded, merged to main (`b740b24..e063543`,
> 9-commit shape). Gates: G1b/G2/G3/G5/G6/G7/G8 MET; **G1a NOT MET** (recall 0.89/0.70,
> callers precision 0.89 after the 1:1-matching fix — recorded, not waived); G4 Rust+Go
> anchored, **Python floor-failed** (pyright 31–36% err). Baseline + S3 work-list:
> `docs/eval/tier-a/baseline.md`. Deferred: `docs/prism-query-layer/tier-a-followups.md`.
> Next-session orientation: `docs/prism-query-layer/tier-a-handoff-2026-06-12.md`.

**Date:** 2026-06-11 · **Status:** rev 4 — **owner-approved** (rev 3 + owner review:
corpus swap to repeat-run-friendly set, `eval/`-vs-`agent-eval` separation contract).
Dual spec-review rounds 1 (5 BLOCKER, 11 MAJOR, 6 MINOR) and 2 (2 BLOCKER, 7 MAJOR,
6 MINOR) folded; records: `docs/prism-query-layer/tier-a-spec-review-2026-06-11.md`,
`...-r2-2026-06-11.md`; r2 verdict: "the architecture itself needs no rework, and
planning can start immediately after rev 3"
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
debug test suite spends ~21 minutes wall on compile/link of **121** `[[test]]` binaries
while actual test execution sums to under 2 minutes (s1-followups item 4; that doc's
"123" counted two non-test targets and is reconciled to 121 alongside this spec).

## 1. Scope

| | In scope (v1) | Deferred (§8) |
|---|---|---|
| **WP1** | Tier-A edge-level accuracy harness in `eval/`: 4 measurements, 3 languages (Rust, Go, Python), 5 corpora, adjudication store, committed baseline scorecards, `prism nav functions` CLI surface + `GIT_SHA` in `--version`, agent guidance in `CLAUDE.md`/`AGENTS.md` | TS/JS oracles, module-edge oracle, SCIP-as-oracle adapter, kubernetes/TypeScript-repo scale runs, direct xAST case reuse, CI wiring |
| **WP2** | Umbrella test-binary consolidation (121 → 24 targets per the §3.1 migration table) + `[profile.dev]` debug tuning, measured against a defined protocol | cargo-nextest |

Tier-B (flow-level taint fixtures) and Tier-C (end-task review benchmark) are separate
future initiatives, unchanged by this spec.

## 2. WP1 — Tier-A accuracy harness

### 2.1 Home, toolchain, seams

- `eval/` directory at repo root: a **uv-managed Python 3.12 project** (`eval/pyproject.toml`,
  `uv.lock` committed). Python 3.12 is the single supported runtime everywhere — host and
  container alike; there is no looser in-container floor (rev-1 contradiction removed).
  Runtime dependencies: stdlib only. Dev dependency: `pytest` for harness self-tests.
  No coupling to the Rust workspace; `cargo` never builds or tests it.
- **Separation contract (owner, rev 4):** prism's `eval/` is self-contained and
  **unrelated to `~/code/agent-eval`** (the review-agent harness serving
  `~/code/agent-knowledge`). Nothing under prism's `eval/` may import from, write to, or
  depend on `agent-eval`/`agent-knowledge`; the only sanctioned interaction is one-time
  *copying* of cached corpus repos (§2.9). `eval/README.md` opens with this
  disambiguation. If genuinely shared needs ever emerge, the fallback is a separate
  `~/code/prism-eval` repo — not entangling the two.
- Two hard seams, defined as Python protocols (PEP 544) in `eval/tier_a/interfaces.py`:
  - **`Oracle`** — `inventory(corpus) -> list[FunctionDef]`,
    `callers(def_) -> list[CallEdge]`, `callees(def_) -> list[CallEdge]`,
    `definitions_at(site) -> list[DefTarget]` (LSP returns 0..n results — trait impls,
    interface methods; the list is the honest model, and each target is enriched beyond
    a bare location so §2.6 can match by name), plus `version() -> str` and
    `capability_probe() -> ProbeResult` (§2.2).
  - **`SystemUnderTest`** — `inventory(corpus)`, `callers(def_)`, `callees(def_)`
    returning the same dataclasses, plus `version() -> SutVersion` (§2.3).
  Sampling, comparison, adjudication, and reporting code depend only on these protocols.
  This is the swap seam for multilspy, SCIP indexes, a Rust rewrite, or a non-prism SUT —
  none of which are v1 scope.

**Schemas and normalization** (`eval/tier_a/model.py`) — every adapter converts to these
at its boundary; comparison code never sees raw LSP or prism JSON:

```python
@dataclass(frozen=True)
class Location:            # all lines 1-based, inclusive; file repo-relative POSIX
    file: str
    start_line: int
    end_line: int

@dataclass(frozen=True)
class FunctionDef:
    name: str | None       # None = anonymous (closure); see exclusion rule below
    kind: str              # semantic: "function" | "method" | "constructor"
    container: str | None  # enclosing class/mod/etc. from the oracle's hierarchical
                           # documentSymbol (None at module root) — the §2.5 stratifier input
    location: Location     # full definition span
    selection_line: int    # line of the name token (LSP selectionRange; prism: start_line)

@dataclass(frozen=True)
class DefTarget:           # a definition-query result, name-enriched for §2.6 matching
    location: Location
    name: str | None       # resolved by containment lookup against the oracle inventory
    kind: str | None

@dataclass(frozen=True)
class CallEdge:
    direction: str         # "caller" | "callee"
    seed: FunctionDef      # the sampled symbol the probe was issued for
    other_def: Location | None   # caller's (or callee's) definition span; None if unresolved
    other_name: str | None
    call_site: Location    # the call expression's location (line-granular is acceptable)
```

Normalization rules: LSP lines/columns are 0-based — adapters add 1; URIs become
repo-relative POSIX paths (probes outside the corpus root are dropped and counted as
`out_of_corpus`); ranges are inclusive `[start_line, end_line]`; anonymous functions
(`name: None`) are excluded from all matching and reported as separate counts on both
sides. Matching tie-break everywhere: when more than one candidate satisfies a
containment/equality rule, take the one with the smallest span, then lowest
`start_line`, then lexicographically smallest file — fully deterministic.
`FunctionDef.kind`/`container` are **oracle-side semantics**; the SUT inventory's `kind`
is the raw tree-sitter node kind, informational only (Rust's `function_item` cannot
distinguish method from function), and never feeds stratification.

- v1 implementations:
  - `PrismCli(SystemUnderTest)` — subprocess calls to a release-built `prism` binary:
    `nav functions/callers/callees ... --format json`, with the JSON extraction rules
    pinned in §2.5.
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

- **Capability smoke-probe (run precondition):** at startup the adapter runs
  `capability_probe()` — `prepareCallHierarchy` + one `incomingCalls` on a known-good
  symbol in a 3-line probe file. A server that fails the probe invalidates the corpus run
  with `oracle_unsupported`, never a column of zeros. This is a **plan precondition for
  pyright specifically** (call-hierarchy support there is unverified; the prototype was
  Rust-only). Named fallbacks, in order: `basedpyright`; else a references-based oracle
  (`textDocument/references` + caller-function containment via the inventory — exactly
  the caller-function-level granularity §2.5 already defines).
- **Readiness:** rust-analyzer and gopls report indexing via `$/progress`; the adapter
  waits for quiescence (no active progress tokens + a settle delay) before issuing
  queries, with a hard cap (config, default 300 s) after which the run proceeds and the
  report records `oracle_not_quiescent: true`. pyright readiness is per-file
  (`didOpen` + first diagnostics or a settle delay). **Escape trigger:** if the
  homegrown lifecycle/quiescence layer burns more than ~2 days of implementation time,
  swap multilspy in behind the `Oracle` seam rather than continuing to hand-roll.
- **Capability quirks:** per-server symbol-kind mapping
  (`SymbolKind.Function/Method/Constructor` → `FunctionDef.kind`), hierarchical-vs-flat
  `documentSymbol` responses, and missing `prepareCallHierarchy` items for valid
  positions are normalized in the adapter, never in the comparison code.
- **Failure honesty:** a request that times out or errors is recorded as
  `oracle_error` for that probe and **excluded from P/R denominators**; per-corpus
  `oracle_error_rate` is a first-class report field. The run never silently converts an
  oracle failure into a prism failure. **Validity floors** (config, per corpus) gate
  **probe failures, not population shortfall**: a stratum is floor-valid when successful
  probes ≥ `min(6, eligible_population)` — a naturally undersized stratum (Go `U-free`
  will run short by design, §2.5) records its shortfall and stays **non-gating**, per
  the spec's honesty posture. Corpus-level floors: `oracle_error_rate` above 10%
  (Rust/Go) / 25% (Python) ⇒ `baseline_invalid` — numbers reported but unable to serve
  as the S3/B2 baseline. **Symmetric SUT floor:** `sut_error_rate` above 5% also ⇒
  `baseline_invalid` — excluded-from-denominator treatment must not let a broken prism
  build report inflated P/R on whatever probes happen to succeed (`inventory_miss` is
  not a `sut_error`; it scores as FN per §2.5).

Oracle install/prep is documented in `eval/README.md` (rust-analyzer already on PATH;
`go install golang.org/x/tools/gopls@latest`; `npm i -g pyright` for
`pyright-langserver`). The harness records each oracle's version string in the report.

### 2.3 New prism CLI surfaces: `nav functions` + build identity

**`nav functions`.** A `Functions` variant added to the `NavQuery` enum
(`src/main.rs:230`), implemented against the **S1 `FunctionTable` directly**
(`LoadedRepo` → per-file `ParsedFile::functions()`), *not* the nav CPG index (whose
`CpgNode::Function` carries no kind information — `src/cpg/types.rs`):

```bash
prism nav functions --repo <path> --format json
```

Output is a **plain JSON array** (an inventory dump, not an Evidence envelope) of
`{file, name, start_line, end_line, kind}`; `kind` is the tree-sitter node kind name
(`kind_id` resolved via the file's grammar). Anonymous functions emit `"name": null`.
Sorted by `(file, start_line)` — deterministic like every other prism output.

**Dedup rule (required):** the Python function query captures both
`(function_definition)` and `(decorated_definition)` (`src/queries.rs:95-97`) with no
dedup in `build_function_table`, so every decorated function is double-captured — fatal
on the decorator-dense Python corpora (flask's `@app.route` idiom everywhere). `nav functions` emits **one record per function:
when captures nest and share a name token, only the innermost definition node is
emitted** (wrapper spans like `decorated_definition` are dropped). This rule lives in the
emission path; the harness does not paper over duplicates.

**Build identity.** `build.rs` additionally embeds `GIT_SHA` + a dirty flag (same
mechanism as the existing `GRAMMAR_FINGERPRINT`), with
`cargo:rerun-if-changed=.git/HEAD` **and** the ref file HEAD points at, so the embedded
SHA cannot itself go stale across commits (the dirty flag is best-effort between edits —
the harness's HEAD comparison is the backstop). `prism --version` prints them.
Rationale: `CARGO_PKG_VERSION` is constant across dev commits, and stale-binary drift is
a documented silent failure of this lineage. The harness compares the binary's SHA to
the prism repo's HEAD and **aborts with `sut_stale`** on mismatch/dirty unless
`--allow-stale-sut` is passed (in which case the report carries `sut_stale: true`).

**SUT binary discovery.** The harness never builds prism. `PrismCli` resolves the binary
as: `--sut-bin <path>` flag → `PRISM_BIN` env var → `<prism repo>/target/release/prism`.
`eval/README.md` documents the one-liner (`cargo build --release` — plus
`--features mcp` only if comparing the MCP surface, not v1) and the resolution order.

No other fields in v1: this is the minimal surface measurements 1–2 need, and the
additive-only rule from the S1/Option-C lineage applies (existing outputs untouched).

### 2.4 Measurement 1 — inventory diff (definition coverage)

**Corpus file universe (defined, applied to both sides):** for each corpus,
`corpora.toml` declares the language's file extensions plus per-corpus exclude globs
(vendored/generated/fixture trees — e.g. flask's `examples/`; prism's
`tests/fixtures/`). The universe is: files under the corpus root matching the
extensions, minus excludes. The **same filter** is applied to the oracle inventory
(`documentSymbol` is only requested for universe files) and to prism's `nav functions`
output (records outside the universe are dropped before comparison). Both raw file
counts and the filtered universe size appear in the report.

**Matching primitive:** a prism record matches an oracle record iff **names are equal
and the oracle's `selection_line` (name-token line from `selectionRange`) falls within
the prism record's `[start_line, end_line]`**, tie-broken per §2.1. Start-line equality
is deliberately *not* used: LSP `DocumentSymbol.range` includes doc comments, attributes,
and decorators while tree-sitter definition nodes exclude them, so start lines diverge
far past any fixed tolerance on doc-commented Rust (most public functions in this repo)
and decorated Python. The selection-line-containment rule subsumes both.

Outputs: prism-missing (oracle has it, prism doesn't — indexing recall gaps),
prism-extra (prism has it, oracle doesn't — e.g. feature-gated/`cfg`'d code, macro
artifacts), anonymous counts per side (excluded from matching per §2.1), and the
**collision statistics** (definitions per name, kind) that define measurement 2's strata
*from the oracle side* — so stratification and recall cannot be biased by prism's own
indexing blind spots.

### 2.5 Measurement 2 — stratified callers/callees precision & recall

**Strata** — every sampled symbol is assigned to exactly **one** stratum by this
precedence (collision > qualification > unique), eliminating overlap:

| Precedence | Stratum | Definition |
|---|---|---|
| 1 | `C-method` | name has ≥ 2 definitions in the universe; this symbol's oracle kind is method/constructor |
| 2 | `C-name` | ≥ 2 definitions; oracle kind is function |
| 3 | `Q-scoped` | unique name; free function defined in a nested module/package — Rust: `container` is a `mod` or the file is not the crate root; Go: in any subdirectory package; Python: in a module inside a package (directory with `__init__.py`). Classified from `FunctionDef.container` + universe paths — both oracle-side |
| 4 | `U-method` | unique name; method/constructor |
| 5 | `U-free` | unique name; free function at module root |

Free-fn vs method comes from the **oracle's `SymbolKind`** (language-neutral); the
nested-module rule is the per-language classifier above, implemented on universe paths +
inventory container info. In Go most unique free functions will land in `Q-scoped` and
`U-free` will run short — the §"sample" shortfall rule covers it and the report records
realized stratum sizes.

**Sample:** 8 symbols per stratum per corpus (40 total; a stratum with fewer than 8
eligible symbols takes all of them and the report records the shortfall — non-gating per
§2.2). Selection is deterministic **and decoupled from live-oracle variance**: the first
run at a given corpus SHA persists its oracle inventory as a snapshot
(`eval/snapshots/<corpus>-<sha>.json`); the sample is a pure function of
`(snapshot, seed)` — inventory sorted by `(file, start_line, name)`, then
`random.Random(seed)` with the seed fixed in `eval/corpora.toml` (default 42).
Subsequent runs at the same corpus SHA load the snapshot for sampling (so one
`documentSymbol` timeout cannot reshuffle strata and samples) and report fresh-vs-snapshot
inventory drift ungated. Same snapshot + same seed ⇒ same sample ⇒ reproducible numbers.

**Seeding the SUT (location-based, not symbol-based):** prism's symbol+file seeding
returns `AmbiguousSymbol` when one file holds several same-name definitions
(`src/navigation/seed.rs:57-78`) — common for exactly the `C-method` stratum. So probes
seed by location: `prism nav callers --location <file>:<line> --depth 1 --format json`,
where `<line>` is the **matched prism record's `start_line`** (resolved via measurement
1). A sampled symbol with no prism inventory match is not seeded: it is scored as
`inventory_miss` (all its oracle edges count as FN at both granularities, and it is
cross-referenced to measurement 1's prism-missing list). Any other SUT-side error is
recorded as `sut_error` for that probe and excluded from denominators (mirroring the
oracle rule, reported as a separate rate). `--depth 1` is mandatory: LSP call hierarchy
is direct-only, and prism's depth default must not leak transitive edges into the
comparison.

**Prism JSON extraction rules (pinned against the current wire format):**

| Query | `CallEdge` field | JSON path |
|---|---|---|
| `callers` | `other_def` (caller's definition span) | `items[].location` (= `items[].symbol` span) |
| `callers` | `other_name` | `items[].symbol.name` |
| `callers` | `call_site` | `(items[].location.file, items[].why[CalledBy].call_site_line)` |
| `callees` | `other_def` (callee's definition span) | `items[].location` when `items[].symbol` present; **`None` when prism leaves the callee unresolved** (then `location` is the call line itself) — unresolved callees are counted separately, never as resolved-edge TPs/FPs |
| `callees` | `other_name` | `items[].why[Calls].callee` |
| `callees` | `call_site` | `(seed.location.file, items[].why[Calls].call_site_line)` |

One evidence item per call site is the wire contract (m7 symmetry); the harness relies
on it for site-level counting.

**Probes:** for each sampled symbol, oracle `incomingCalls`/`outgoingCalls` at its
definition vs the seeded prism queries above.

**Matching, two granularities, both reported:**

- **Call-site level (primary):** a prism `call_site` matches an oracle `fromRange` if
  file matches and the line falls within the range's line span. Site-level TP/FP/FN ⇒
  precision/recall per stratum per direction. Edge identity is line-granular and prism's
  `CallSite` carries no column, so **same-line multi-call sites are collapsed to one
  countable site per `(file, line, direction)`** with the collapse recorded
  (`same_line_multicall_collapsed` count) — they are excluded from per-site multiplicity
  claims rather than guessed at.
- **Caller-function level:** the set of containing functions (derived by line containment
  against the oracle inventory). Call sites outside any inventoried function — common at
  Python module level, where LSP call-hierarchy behavior also varies by server — go into
  an explicit **`module_level` pseudo-container bucket** per file, counted, never
  silently dropped. Coarser, robust to multi-site counting differences (prototype:
  `all_functions` 62 sites vs 56 callers).

**Pinned probes (outside the random sample, fixed forever):** four dedicated probes run
on the `prism` corpus every time, excluded from all stratum denominators:

1. `target` (`src/algorithms/taint.rs:1276`) — the prototype's `C-method`-class total
   failure; the S3 flip indicator.
2. `module_deps` — carries the known feature-gated oracle-miss (`src/mcp/tools.rs:162`).
3. `load_repo` — carries the second one (`src/mcp/session.rs:28`).
4. `slice` seeded by bare symbol with no `--file`/`--location` — must return
   `AmbiguousSymbol` (the safe-fail contract from the prototype).

Pinning these decouples the acceptance gates from random sample composition (G1/G2
below) and keeps the two adjudicated oracle-misses from dragging the random-sample raw
numbers.

### 2.6 Measurement 3 — site-level definition spot-check

From measurement 2's prism caller results, sample (same seeded RNG) up to 20 claimed call
sites per corpus and ask the oracle for definitions at the site. Prism evidence is
line-granular, so the harness locates the column by finding the seed symbol's name token
on the claimed line **in call position** — followed by `(`, or preceded by `.`/`::` —
falling back to first occurrence outside a string/comment only if no call-position
occurrence exists (a bare-first-occurrence rule would resolve the LHS local in
`let target = edge.target();` and mint a false confirmed-FP).

M3 verdicts honor the §0 contract — **auto-classification only where the oracle is
structurally unambiguous; everything else routes to adjudication**
(`measurement: "m3"` in the §2.8 store):

| Outcome at the site | Classification |
|---|---|
| any returned `DefTarget` matches the seed definition (selection-line containment + name, §2.4) | **confirmed TP** |
| all returned `DefTarget`s have a name ≠ seed name | **confirmed FP** — the petgraph-`.target()` class, mechanically confirmed |
| a returned `DefTarget` has the seed's *name* but a different definition (typically a trait/interface declaration vs prism's concrete impl — the oracle is being *less* specific, not contradicting) | **`ambiguous` → adjudication**, never auto-FP; the bias would otherwise concentrate on exactly the method strata S3 exists to measure |
| seed name absent from the line entirely (aliased import `from m import f as g`, re-export, constructor sugar) | **`alias_site` → adjudication**, counted separately — absence of the literal token is not evidence of a false edge |

The report shows confirmed and adjudicated M3 results separately. This is the direct S3
edge-precision-floor metric, at spot-check cost instead of full-census cost
(meta-analysis approach B was rejected for cost and zero recall signal).

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
  line = 4                      # definition start line (location-seeded, §2.5)
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
- **The Python `decorator-wrapped fn` case is B2's designated flip indicator.** The
  §2.3 dedup rule deliberately makes measurement 1 blind to the decorator
  double-capture quirk both before and after B2 fixes it, and pyright on the Python
  corpora is the acknowledged noisiest live channel — so B2's Python regression signal
  is carried here, by construction, not by the live corpora.

### 2.8 Adjudication

Committed store `eval/adjudications.jsonl`, one record per adjudicated diff, **keyed by
`(corpus, measurement, direction, seed_def, site)`** — seed identity and direction are
part of the key (the same site can appear under different seeds), `seed_def` is
`file:selection_line` of the sampled symbol's definition, `site` is `file:line` of the
call site itself:

```json
{"corpus": "prism", "measurement": "callers", "direction": "prism_only",
 "seed_def": "src/navigation/module_graph.rs:<module_deps selection line>",
 "site": "src/mcp/tools.rs:162",
 "verdict": "oracle_miss", "reason": "mcp feature-gated; rust-analyzer is build-config-scoped",
 "adjudicated_by": "wesley", "date": "2026-06-11"}
```

- `direction`: `prism_only` | `oracle_only`. **Metric-contribution truth table** (the
  complete corrected-metric math; combinations not listed are illegal and rejected by
  the loader):

  | direction | raw class | verdict | corrected contribution |
  |---|---|---|---|
  | both agree | TP | — | TP |
  | `prism_only` | FP | *(pending)* | excluded; `pending_count` |
  | `prism_only` | FP | `oracle_miss` | **TP**, increments `oracle_miss_count` |
  | `prism_only` | FP | `prism_fp` | FP |
  | `prism_only` | FP | `oracle_artifact` | excluded from both denominators |
  | `prism_only` | FP | `ambiguous` / `alias_site` | excluded, listed |
  | `oracle_only` | FN | *(pending)* | excluded; `pending_count` |
  | `oracle_only` | FN | `prism_fn` | FN |
  | `oracle_only` | FN | `oracle_artifact` | excluded from both denominators |
  | `oracle_only` | FN | `ambiguous` | excluded, listed |

  Corrected P = TP/(TP+FP), corrected R = TP/(TP+FN) over the post-transform counts;
  Wilson intervals use those same denominators. `stale` records contribute nothing to
  any metric (listed only). Raw metrics count every pending diff conventionally
  (`prism_only` = FP, `oracle_only` = FN).
- **Adjudication budget (baseline-sufficiency rule):** the committed baseline requires
  *all* Rust/Go pending diffs adjudicated, plus a seeded-RNG sample of ≤ 25 pending
  diffs per Python corpus — the remainder may stay pending (they are excluded from
  corrected metrics either way). This bounds the human work on the S3/B2 critical path
  against unbounded Python diff volume.
- Records survive re-runs; a record whose site no longer appears in a run is reported as
  `stale` (corpus SHA changed) rather than deleted.
- The two known feature-gated callers from the prototype are seeded into this file as
  part of v1 with their **verified call-site lines** — `module_deps` called at
  `src/mcp/tools.rs:162`, `load_repo` called at `src/mcp/session.rs:28` (selection lines
  filled in at implementation when measurement 1 first emits them). The harness must
  rediscover both diffs via the pinned probes (gate G2).

### 2.9 Corpora and reproducibility

`eval/corpora.toml` (committed):

**Corpus selection principle (owner, rev 4): repeat-run friendliness.** Prism's CPG
cache goes cold after every prism-side change — and measuring prism-side changes is the
harness's whole job — so corpus cold-build cost recurs, it isn't a one-time setup tax.
django (~15 min cold CPG build) is disqualified on this principle and moves to §8 scale
runs; hugo (~2 min) is demoted there too in favor of caddy (~96k LOC Go, already cached
locally). All v1 corpora cold-build in ≲1 min.

| Corpus | Lang | Path | Oracle | Prep |
|---|---|---|---|---|
| `prism` | Rust | the repo itself | rust-analyzer | none |
| `tokio` | Rust | `~/code/bench-repos/tokio` | rust-analyzer | none |
| `caddy` | Go | `~/code/bench-repos/caddy` | gopls | copy from `~/code/agent-eval/cache/repos/caddy` (worktree at `77e9ce74`); `go mod download` |
| `flask` | Python | `~/code/bench-repos/flask` | pyright (capability-probe gated, §2.2) | clone + venv; decorator-dense (`@app.route`) — exercises the §2.3 dedup rule on real code |
| `click` | Python | `~/code/bench-repos/click` | pyright (same gating) | clone + venv; small and comparatively well-typed — **the Python corpus most expected to clear the validity floors**, hedging flask |

Each entry carries: absolute path (machine-local; the file documents this is a
dev-machine harness, not CI), **pinned commit SHA** (recorded at first run; the runner
warns and records `corpus_dirty: true` if HEAD differs), the file-universe definition
(extensions + exclude globs, §2.4), sample seed, per-corpus caps and validity floors
(§2.2). Every report records: corpus SHA, prism `GIT_SHA` + dirty flag (§2.3), oracle
name + version, seed, harness git SHA, wall time per measurement.

Known asymmetry, stated in reports: Rust/Go oracle quality is compiler-grade;
**pyright numbers on the Python corpora carry oracle noise** (dynamic-code inference
gaps). The Python report sections are labeled accordingly, and `oracle_error_rate` +
`ambiguous` adjudications are expected to be higher there — flask/click are deliberately
small and comparatively typed, so if even they fail the floors, that finding gates all
Python accuracy claims. The noise floor is itself a v1 finding (meta-analysis §1
uncertainty, now quantified). **Adjudicator warning (carried into the report
template):** Python-corpus M2 diffs partially reflect prism's own decorator
double-capture quirk — the CPG's `(file, name)`-keyed `func_index` silently strips
edges from one of each wrapper/inner pair — so Python `oracle_only` diffs on decorated
functions are evidence *about the B2 quirk*, not only about call resolution; B2's
regression signal itself lives in the §2.7 matrix case, not here.

### 2.10 Reports and baseline

`eval/tier_a/report.py` renders, per corpus, `docs/eval/tier-a/<date>-<corpus>.md` +
`.json`. The JSON is the machine-readable record: run metadata block, **per-probe raw
oracle and SUT responses** (the replay store for G3), and all four measurements with
raw/corrected metrics per stratum, each point estimate accompanied by a **Wilson 95%
interval** (`p`, `p_lo`, `p_hi` in JSON; `0.83 [0.62–0.94]` in markdown — these are the
§6 risk mitigation made concrete in the schema). A roll-up `docs/eval/tier-a/baseline.md`
holds the per-language summary table + capability-matrix grid + pending-triage counts +
validity-floor status. The first full run, adjudicated and floor-valid, committed = **the
S3/B2 baseline**. Subsequent runs are new dated files; `baseline.md` is updated only
deliberately (it is the comparison anchor, not a rolling log).

### 2.11 Runner and quick modes

```bash
uv run tier-a --corpus all              # full run, 5 corpora (the baseline command)
uv run tier-a --corpus prism            # one corpus
uv run tier-a --quick                   # prism corpus, 3/stratum, + matrix (minutes)
uv run tier-a --matrix-only             # capability matrix only, no LSP (seconds)
uv run tier-a --report-only <run.json>  # re-render reports/metrics from a stored run
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
  dataclasses: selection-line containment matching (doc-comment/decorator offset cases),
  tie-breaking, stratum precedence assignment, P/R math + Wilson intervals, adjudication
  application (each verdict class), pending-triage exclusion, stale-record detection,
  `inventory_miss`/`sut_error`/`oracle_error` accounting, validity floors, deterministic
  sampling (fixed seed twin-run), prism JSON extraction rules against canned wire
  samples (§2.5 table).
- **LSP client framing tests** against a scripted Python echo-server subprocess speaking
  JSON-RPC over stdio (framing, correlation, timeout, notification routing). No real
  language servers in self-tests.
- **Matrix runner test** against one tiny committed fixture using `PrismCli` if the
  binary is present, else skipped (`pytest.mark.skipif`) — the only self-test touching
  prism.

## 3. WP2 — test-suite wall-time reduction

Independent of WP1; own commits; own gate. Two stacked changes:

### 3.1 Umbrella test binaries (121 → 24)

One `[[test]]` target per leaf test directory. The migration table (target name ←
directory, absorbed `[[test]]` count, notes):

| New target | Directory | Absorbs | Notes |
|---|---|---|---|
| `ast` | `tests/ast` | 16 | |
| `algo_novel` | `tests/algo/novel` | 15 | |
| `navigation` | `tests/navigation` | 11 | |
| `algo_taxonomy` | `tests/algo/taxonomy` | 11 | |
| `integration` | `tests/integration` | 8 | includes coverage matrix tests |
| `frameworks` | `tests/frameworks` | 8 | |
| `lang_c` | `tests/lang/c` | 6 | |
| `lang_javascript` | `tests/lang/javascript` | 5 | |
| `cli` | `tests/cli` | 5 | |
| `algo_theoretical` | `tests/algo/theoretical` | 5 | |
| `lang_tsx` | `tests/lang/tsx` | 4 | |
| `lang_typescript` | `tests/lang/typescript` | 3 | |
| `lang_terraform` | `tests/lang/terraform` | 3 | |
| `lang_rust` | `tests/lang/rust` | 3 | |
| `lang_lua` | `tests/lang/lua` | 3 | |
| `lang_go` | `tests/lang/go` | 3 | |
| `lang_bash` | `tests/lang/bash` | 3 | |
| `lang_java` | `tests/lang/java` | 2 | |
| `lang_cpp` | `tests/lang/cpp` | 2 | |
| `reasoning` | `tests/reasoning` | 1 | |
| `output` | `tests/output` | 1 | |
| `mcp` | `tests/mcp` | 1 | keeps `required-features = ["mcp"]` |
| `infra` | `tests/infra` | 1 | |
| `algo_paper` | `tests/algo/paper` | 1 | |

**Excluded (not targets):** `tests/common` (shared helper module) and `tests/fixtures`
(data). Total absorbed: 121 ✓.

Mechanics:

- Each umbrella `main.rs` declares its sibling files as `mod`s plus **one**
  `#[path = "../common/mod.rs"] mod common;` (or `../../common` per depth). The per-file
  `mod common;` headers are removed and each file's `use common::*;` (the pattern in all
  93 affected files) becomes **`use crate::common::*;`** — glob form preserved so every
  unqualified helper call keeps compiling; no call-site rewrites.
- **Module names are file-derived, no aliasing:** plain `mod taint_cve_test;` exposes
  `taint_cve_test::` — filters use the full file-stem name (no `#[path]` renames; one
  fewer thing to drift).
- `tests/common` then compiles 24× instead of 121×, and the linker runs 24× instead of
  121× against the full prism lib + 11 grammars — the dominant cost in the 21-minute
  number.
- **No file merging.** The 600-line file rule is untouched (owner allows loosening to
  800–1000 if ever needed; this change does not need it — umbrella `main.rs` files are
  ~20 lines of `mod` declarations).
- Invocation changes: `cargo test --test algo_paper` →
  `cargo test --test <dir-target> <module>::` (e.g. `cargo test --test algo_taxonomy
  taint_cve_test::`). **Explicit deliverables:** rewrite `CLAUDE.md`'s Build & Test section
  (including its `cargo test --test integration_coverage` reference and the named-suite
  examples) and the "register each as a separate `[[test]]` target" clause of design
  decision 7; sweep `.github/` workflows and `scripts/` for **both** `--test <old-name>`
  references **and path-style references** (`tests/<dir>/<file>.rs`).
  `scripts/extract_tests.py` currently has zero `--test` references (verified) — the
  sweep confirms rather than assumes.
- `tests/integration/coverage_test.rs` has **three** hardcoded path lists
  (`all_test_files` ×2 at lines 106/325 and `test_files` at line 472) plus one
  algorithm-completeness consumer of the generated `coverage/matrix.json` (line 991 —
  not a path list) — the lists reference file paths, not target names, and all files
  survive; the gate is `cargo test --test integration coverage_test::` passing with the
  matrix output unchanged.

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

- **P1 (clean):** `cargo clean && /usr/bin/time -l cargo test` — worst case.
- **P2 (dev loop):** `touch src/lib.rs && /usr/bin/time -l cargo test` — the loop that
  hurts. **P2 runs immediately after P1 completes**, on P1's warmed target dir — that
  ordering is part of the protocol.

Single timed pair per measurement point, machine otherwise idle (matches how the
21-minute baseline was observed); real/user/sys + max RSS recorded from `time -l`.

**Gate:** P2 < 8 minutes. P1 and P2 reported for all three measurement points. If the
gate is missed, the numbers are reported honestly with attribution (S1 row-C precedent)
and nextest/linker options move from deferred to proposed — no silent waiver.

## 4. Acceptance gates (objective)

| # | Gate |
|---|---|
| G1 | **Prototype reproduction, decoupled from sample composition:** (a) on the `prism` corpus random sample, **adjudication-corrected** site-level P and R ≥ 0.95, evaluated **per direction (callers and callees separately), pooled across the `U-free`+`U-method` strata** (pinned probes excluded from denominators); (b) the pinned `target` probe carries `expected: known_fail` (the §2.7 idiom) — the gate passes when the probe either reproduces the failure (raw site-level P ≤ 0.2 and R ≤ 0.2) **or** reports a flip candidate ("update expectation — resolution improved"); an incidental upstream improvement must not fail *harness* acceptance |
| G2 | The pinned `module_deps` and `load_repo` probes surface `prism_only` diffs that match the seeded `oracle_miss` adjudications (§2.8) |
| G3 | **Determinism, decoupled from live-oracle variance:** samples are snapshot-derived (§2.5) — same snapshot + seed ⇒ identical samples by construction; and `--report-only` replay of a stored run JSON reproduces its metrics bit-identically. Live run-to-run variance (timeouts, inventory drift) is reported separately, not gated |
| G4 | Baseline committed: 5 corpora × measurements 1–3 + capability matrix for 3 languages, **validity floors met per §2.2's failure-based definition** (oracle + SUT error floors; natural stratum shortfall is non-gating) — a floor-failing corpus is marked `baseline_invalid` and cannot anchor S3/B2 (Python anchors on whichever of flask/click is valid; G4 requires at least one valid corpus per language), with run metadata (SHAs incl. prism `GIT_SHA`, versions, seed, wall times) in every report |
| G5 | Capability matrix runs green: every fixture executes; statuses assigned (`pass`/`known_fail`); zero unexpected regressions by definition at first commit |
| G6 | WP2: consolidated suite passes **3 consecutive full runs** (consolidation changes the concurrency topology — up to 16 files now share one process/thread pool — and a single green run won't catch intermittent cross-file interference); coverage-matrix test passes unchanged; stale references swept — both `--test <old-name>` and path-style, across `CLAUDE.md`, `scripts/`, `.github/` (repo-wide grep clean) |
| G7 | WP2: P2 < 8 min; P1/P2 reported at all three measurement points; honest report if missed |
| G8 | `CLAUDE.md` + `AGENTS.md` agent guidance landed; `eval/README.md` covers oracle install/prep end-to-end |

## 5. Execution model

Per the S1 ritual: TDD plan via writing-plans; codex gpt-5.5 as containerized implementor
through a2a-bridge (`--config examples/a2a-bridge.slicing-implement.toml`), dual diff
review per slice, squash-merge ritual. One split is new: the a2a container is
egress-locked and has no LSP servers or bench-repos, so **containerized verify covers
the Rust changes; the eval/ pytest suite runs in-container only if the image provides
Python 3.12 + pytest (checked at kickoff), otherwise it joins the live-oracle runs,
baseline generation, and WP2 timing measurements as host-side orchestrator-run gates**
(same pattern as S1's bench-ladder).

Sequencing inside the initiative: WP2 first (its 5× link-count cut pays for itself
immediately across WP1's own Rust-side commits), then WP1.

## 6. Risks and mitigations

| Risk | Mitigation |
|---|---|
| LSP automation flakiness (readiness, hangs) | quiescence detection + per-request timeouts + `oracle_error` accounting (§2.2); failures excluded from denominators, never converted to tool failures; validity floors prevent hollowed-out denominators from passing gates; multilspy escape trigger after ~2 days |
| pyright call-hierarchy support unverified | capability smoke-probe as plan precondition; named fallbacks: basedpyright, then references+containment (§2.2) |
| pyright noise on the Python corpora pollutes the picture | labeled sections, `oracle_error_rate` + `ambiguous` first-class; flask/click chosen small + comparatively typed; Rust/Go carry the precision claims; noise floor itself is a deliverable |
| oracle indexing resource cost (gopls/caddy, rust-analyzer/tokio) | repeat-run-friendly corpus set (§2.9, all ≲1 min prism-cold); per-corpus caps in `corpora.toml`; quiescence cap with `oracle_not_quiescent` flag; corpora warm after first index |
| Small samples → wide confidence | Wilson 95% intervals are schema fields (§2.10); v1 is a baseline, not a hypothesis test; sample sizes are config, not code |
| Harness bugs self-confirming prism | G1 reproduction gate against hand-verified prototype numbers via pinned probes; self-tests with fakes incl. wire-format extraction (§2.12) |
| Stale prism binary measured silently | `GIT_SHA` + dirty flag in `--version` (§2.3); harness aborts `sut_stale` unless overridden |
| Oracle/corpus drift over time | versions + SHAs recorded in every report; `corpus_dirty` and `stale` adjudication flags |
| `nav functions` surface creep | minimal field set, additive-only, deterministic ordering, dedup rule pinned (§2.3) |
| WP2 consolidation breaks feature-gated or scripted invocations | `required-features` preserved; G6 sweep covers names and paths |

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
- rev 4 (2026-06-11, owner review): **corpus selection principle = repeat-run
  friendliness** — prism's CPG cache recolds after every prism-side change, so corpus
  cold-build cost recurs; django (~15 min cold) replaced, hugo demoted, v1 corpora =
  prism/tokio/caddy/flask/click (all ≲1 min cold); caddy copied from
  `~/code/agent-eval/cache/repos`. **Separation contract** added (§2.1): prism `eval/`
  is unrelated to `~/code/agent-eval` (review-agent/agent-knowledge harness); copy-only
  interaction; `~/code/prism-eval` is the fallback if separation ever blurs. Tier-B/C
  input pointers recorded (§8): prism-cwe-fixtures, cvefixes, martian-bench.
- rev 3 (2026-06-11): folded round-2 review — failure-based validity floors (natural
  shortfall non-gating) + symmetric SUT-error floor; snapshot-derived sampling (G3
  decoupled from live variance); M3 verdict table honoring adjudicate-not-auto-fail
  (`ambiguous`/`alias_site` routing, `DefTarget` enrichment); flask as the
  floor-clearing Python corpus + django dedup-quirk adjudicator warning + §2.7
  decorator case designated B2's flip indicator; same-line multicall collapse +
  `module_level` bucket; corrected-metric truth table + adjudication budget; G1
  aggregation pinned + `known_fail` idiom (no inverted gate); `--sut-bin` resolution +
  `.git/HEAD` rerun triggers; file-derived umbrella module names; G6 ×3 runs;
  `container`/`DefTarget` schema additions; `/usr/bin/time -l` protocol.
- rev 2 (2026-06-11): folded dual spec-review — pinned `CallEdge`/extraction schemas,
  corpus file universe, location-based seeding, pinned gate probes + corrected-metric
  gates, WP2 migration table, `nav functions` dedup + data-source contract, `GIT_SHA`
  build identity, selection-line matching, pyright capability probe + fallbacks,
  validity floors + replay-based G3, adjudication re-keying, strata precedence,
  `--depth 1`, `use crate::common::*`, Python 3.12 everywhere, anonymous-fn exclusion,
  call-position identifier rule, `definitions_at` list semantics, multilspy escape
  trigger, Wilson fields in schema, 121-target reconcile.

## 8. Deferred (documented follow-ups)

1. **TS/JS oracle + corpora** — highest-priority follow-up (prevalence rationale above);
   tsserver/typescript-language-server client friction is the known cost; xAST's Node.js
   SAST set is candidate fixture input.
2. **Module-edge oracle** — no clean LSP analogue; candidate: language-native tools
   (`go list -deps`, `cargo metadata`, import graphs) per language.
3. **SCIP-as-oracle adapter** — drop a `ScipOracle` behind the `Oracle` seam (meta-analysis
   §3 stage-1 complement; offline, no server lifecycle).
4. **Scale runs** — django (the ~15-min cold-build case that motivated the §2.9
   selection principle), hugo, kubernetes (gopls), TypeScript repo,
   rust-analyzer-on-rust-analyzer.
5. **xAST direct reuse** — Tier-B input (taint-oriented cases; Java/Node.js coverage).
   Likewise recorded for the later tiers: `~/code/agent-eval/cache/prism-cwe-fixtures`
   (CWE-labeled fixture sets: 22/78/79/89/502/918 + manifest) and
   `~/code/agent-eval/cache/datasets/cvefixes` are candidate **Tier-B** inputs;
   `~/code/agent-eval/cache/martian-bench` is the **Tier-C** harness input — pointers
   only, per the §2.1 separation contract.
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
