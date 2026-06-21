# Tier-A Baseline — 2026-06-21 (new-anchor adjudication: κ-validation + characterization)

The 9 new anchors (ruff/ripgrep/cobra/prometheus/etcd/zap/black/httpx/mypy) carried **1,405 unadjudicated
pending M2 sites**. Rather than exhaustively bulk-adjudicate (the v1 budget ≈1,150), a **90-site stratified
dual-adjudicator κ-validation** (10/corpus) was run first — codex gpt-5.5 xhigh + claude operator, blinded,
identical evidence — to gauge reliability on the new (Go-heavy + Python) corpora before committing the bulk.

**Result: codex is reliable; the pending is recall-dominated; exhaustive bulk deferred.**
- **κ: raw 0.53 (60/90), reconciled 0.88 (79/90).** The entire gap is ONE definitional axis: the operator
  over-applied `oracle_artifact` to **source-visible Go callback registration** (`Run: emptyRun`) and
  **Python `@property` access** (`x.text`, `node.prev_sibling`); codex correctly classed all 19 as `prism_fn`.
  **Taxonomy rule (codified here):** `oracle_artifact` is reserved for source-*invisible* edges (macro/derive/
  generated expansion). Property-getter and callback-registration edges ARE in the source and the getter/callback
  genuinely runs, so a prism miss there is a real **recall gap** (`prism_fn`), not an artifact. codex used
  `prism_fn`/`prism_fp`/`ambiguous` only (89/90 direction-valid; 1 slip).
- **Pending shape (codex projection):** ~**56% `prism_fn`** (recall gaps = prism's KNOWN capability limits:
  @property, callbacks, builder chains, interface/trait dispatch), ~**32% `prism_fp`** (the precision signal =
  name collisions, e.g. `commandSorterByName.Len` vs `bytes.Buffer.Len`; cross-package `NewDiscovery`/`yoloString`/
  `with_file`), ~**12% ambiguous**. A full bulk would mostly re-confirm known recall gaps.
- **Committed:** the 90 reconciled verdicts → `eval/adjudications.jsonl` (1563→1653; agreed kept, the 19
  artifact→`prism_fn`, 11 contested residuals → conservative `ambiguous` except 2 clear chain-start `prism_fp`).
  The 9 reports re-folded (`--report-only`): pending −10/corpus, **stale_adjudications 0** (all matched). The
  remaining ~1,315 pending stay **characterized-not-classified** (operator chose to stop at the κ-sample).
- Validity, anchor set, and prism metrics are unchanged from the 2026-06-20b/2026-06-20 anchors below; this is an
  adjudication-coverage increment, not a prism or corpus change.

---

# Tier-A Baseline — 2026-06-20b (Corpus Anchor Expansion — 3+ valid anchors per language)

Deliberate baseline change: expand from an effective **1 Rust / 1 Go / 0 Python** valid-anchor set to
**3 Rust / 5 Go / 2 (+1 pending) Python**. Motivation: the prior set leaned on `prism` alone for Rust
(tokio is oracle-**invalid**) and `caddy` alone for Go, with no committed Python anchor — too thin to trust a
cross-language regression signal. Oracle (LSP) self-error was measured for every candidate; a corpus is an
anchor only when its oracle resolves cleanly enough to be ground truth. Harness/config only — **no prism source
change** (the prism SHA `20c8490591a3` and all M2/M3 call-resolution metrics below carry over unchanged from the
2026-06-20 anchor that follows). Run records: `2026-06-20-<corpus>.{json,md}`.

## Committed anchor set

| Lang | Anchors (OER) | Floor | Notes |
|---|---|---|---|
| Rust | **prism** 0.067 · **ruff** 0.000 · **ripgrep** 0.063 | 0.10 | ruff = astral monorepo (ty's crates); ripgrep = clean app |
| Go | **caddy** 0.000 · **cobra** 0.000 · **prometheus** 0.000 · **etcd** 0.000 · **zap** 0.025 | 0.10 | all healthy samples (64–80 probes, 180–850 oracle sites) |
| Python | **black** 0.157 · **httpx** 0.156 · **mypy** 0.163 | 0.25 | mypy unblocked by PR #118 (CPG stack-overflow fix; `sut_error` 0.84→0.0) |

Kept but **not** primary anchors: `uv` (Rust, OER 0.000 — **local only**, prism cold-build perf outlier, below);
`tokio` (Rust, OER 0.219 — macro-density follow-up); `go-redis` (Go, OER 0.000 — valid extra); `typer`
(Python, OER 0.225 — valid but marginal); `rich` (Python, OER 0.250 — Python 3rd-anchor fallback).

## Validity matrix (all measured candidates, 2026-06-20)

| Corpus | Lang | OER | Verdict |
|---|---|---|---|
| ruff, ripgrep, uv | Rust | 0.000 / 0.063 / 0.000 | ✅ (uv local — perf) |
| prism | Rust | 0.067 | ✅ anchor |
| tokio | Rust | 0.219 | ❌ macro density |
| clap, axum | Rust | — | ⚠️ `oracle_unsupported`: rust-analyzer **false-quiescence** (returns before derive/macro expansion finishes at `settle_s=5`); retry lever `settle_s=20` set |
| caddy, cobra, prometheus, etcd, go-redis | Go | 0.000 | ✅ |
| zap | Go | 0.025 | ✅ anchor |
| gin | Go | 0.125 | ❌ (just over floor; small sample) |
| hugo | Go | 0.175 | ❌ interface/reflection-heavy |
| black, httpx | Python | 0.157 / 0.156 | ✅ anchor |
| mypy | Python | **0.163** | ✅ anchor — `sut_error` 0.84→**0.0** after PR #118 (the CPG stack-overflow fix); see note below |
| typer | Python | 0.225 | ✅ valid (marginal) |
| rich | Python | 0.250 | ✅ at-floor (fallback) |
| fastapi, starlette | Python | 0.344 | ❌ dynamic frameworks |
| flask, click, packaging, pydantic | Python | 0.31–0.38 | ❌ dynamic/codegen-heavy |

## Oracle findings

- **No "cleaner pyright" exists.** `zuban` 0.8.2 does **not** implement `callHierarchy` (absent from `initialize`
  capabilities; `prepareCallHierarchy` → `-32601 unknown request`) → cannot serve M2 (callers/callees) at all.
  `basedpyright` 1.39.8 *does* support callHierarchy but is **metric-identical to pyright** (black +0.014, else
  exactly equal across httpx/rich/flask/click) — it shares pyright's inference + callHierarchy engine, so it
  inherits the same dynamic-Python blind spots. pyright stays the Python oracle.
- **Floor calibration is honest, not loose.** Typed Python libraries cluster at OER ~0.15–0.16 (black, httpx);
  everything decorator/dynamic-heavy sits ≥0.22 (typer 0.225 → flask/fastapi/starlette 0.31–0.34). gopls
  resolves concrete-typed Go cleanly (0.00) but misses idiomatic interface dispatch (gin 0.125, hugo 0.175) —
  the Go analogue of pyright's dynamic-Python miss.

## prism analysis timing (answers "is prism the slow part?" — no)

`prism nav repo-map --no-cache` = full call/module-graph build (the real per-corpus analyze cost). prism builds
every corpus in **seconds**; the multi-minute Tier-A runs are **rust-analyzer/gopls (the oracle) indexing**, not
prism (e.g. ruff: prism 24.4s cold vs oracle `oracle_start 53s + m1 63s + m2 168s ≈ 4.75min`).

| corpus | prism cold | warm | | corpus | prism cold | warm |
|---|---|---|---|---|---|---|
| prism | 11.0s | 1.8s | | ruff | 24.4s | 2.3s |
| tokio | 4.7s | 1.6s | | ripgrep | 5.8s | 0.5s |
| **uv** | **66.7s** ⚠️ | 14.8s | | hugo | 10.4s | 0.9s |
| caddy | 2.0s | 0.3s | | black/httpx | 2.8s / 0.6s | — |

**Perf lead: `uv` is a prism outlier** — 66.7s cold / 14.8s warm, ~7× slower per-file than ruff (which is 3×
larger). Kept local for investigation (pathological files / dense module graph suspected).

## prism bug surfaced + FIXED (Python 3rd-anchor unblocked)

**prism stack-overflowed building mypy's CPG**: `fatal runtime error: stack overflow, aborting` → 84% `sut_error`.
Root cause: prism's recursive tree-sitter AST walks (parse, CPG build, live-type scan) ran on rayon workers whose
default ~2 MiB stack overflowed on a `#if`-split 8192-element C initializer in mypy's vendored base64 runtime
(`mypyc/lib-rt/base64/tables/table_enc_12bit.h`). **FIXED + MERGED — PR #118 (`97dcba6`):** a shared 256 MiB-stack
`build_pool` with the CLI command wrapped at `main()` + library-entry wraps (parse / build / live-types).
**Re-measured on `97dcba6`: `sut_error` 0.84 → 0.000, `oracle_error_rate` 0.163, `baseline_invalid` False** — mypy
is now the **3rd Python anchor** (comfortable margin vs the 0.25 floor; cleaner than the rich 0.250 / typer 0.225
fallbacks). M2: callers Q-scoped/U-free/U-method P=R=1.0; 88 pending sites + 79 M1 extras to adjudicate over time
(normal for a new corpus — mypy is a richer Python-resolution workout than black/httpx).

## Harness changes (this expansion)

- `--oracle <name>` override (writes outputs under `<corpus>-<oracle>` so comparison runs don't clobber anchors)
  + `basedpyright` registered in `make_oracle` — used for the zuban/basedpyright comparison above.
- per-corpus `quiescence_cap_s` / `settle_s` (cfg takes precedence over `[defaults]`) — large workspaces get
  600s indexing patience; clap/axum carry `settle_s=20` as the false-quiescence retry lever.

---

# Tier-A Baseline — 2026-06-20 (the perf-arc + field-sensitivity + Phase-2/3 + taint_reaches anchor)

Human-triggered `uv run tier-a --corpus all` on **prism @ `20c8490591a3`** — post the merged 2026-06-19/20
stack: the cold-build perf arc (#111 S1.5 · #112 step5b-memo · #114 step7 · #116 assemble edge-steps · #115
rematerialize — all byte-identical), **#117 Step-5b field-sensitive interproc arg binding (supplement)**,
**Phase-2 receiver-typing (#106–108) + Phase-3 Slice-1a method-chain receiver typing (#109)**, and **#113 Plan B
`taint_reaches`** (additive). Refreshes the 2026-06-17 Phase-1 F3 anchor (preserved below as the adjudication
substrate; its classed records carry forward). Run records: `2026-06-20-<corpus>.{json,md}`. Re-pinned
`eval/corpora.toml` prism → `20c8490591a3`. **This file is the comparison anchor — update only deliberately.**

## What moved (2026-06-20 vs the 2026-06-17 F3 anchor) — receiver typing = precision AND recall up

Phase-2 PR-3 + Phase-3 Slice-1a (Rust receiver typing) resolved the name-based over-claims the F3 anchor
explicitly **deferred**. prism corrected P/R rises to **~1.00 across nearly every M2 stratum**, with raw
false-positives dropping to ~0:

| M2 stratum | 2026-06-17 (tp/fp/fn · corr P/R) | 2026-06-20 (tp/fp/fn · corr P/R) |
|---|---|---|
| callers C-method | 19/1/24 · 0.95 / 0.44 | 4/0/0 · 1.00 / 1.00 |
| callers C-name | 51/**9**/0 · 0.85 / 1.00 | 32/**0**/0 · 1.00 / 1.00 |
| callers U-method | 11/**5**/0 · 0.69 / 1.00 | 26/**0**/0 · 1.00 / 1.00 |
| callees C-method | 5/**4**/0 · 0.56 / 1.00 | 10/**0**/0 · 1.00 / 1.00 |
| callees U-method | 4/1/0 · 0.80 / 1.00 | 24/0/0 · 1.00 / 1.00 |

The 2026-06-17 "honest gap" (standing name-based method/receiver over-claims) is **largely closed**. **No
stratum gained a false-positive**; corrected recall held or improved (C-method callers fn 24→0).
`sut_error_rate=0.000` on all five corpora. **5** prior adjudications went stale — their fp sites were *fixed*
by receiver typing (the right direction). M1: `prism_extra=0`, `prism_missing=17` (== snapshot; carried recall
gaps, unchanged). M3 spot-check (25 of the 150 over-sample pending): all `ambiguous`, **0 confirmed_fp** — no
new over-claims hiding in the unsampled tail.

**Carried-over deferred over-claims** (unchanged, NOT new): callees C-name 2 fp, callees U-free 2 fp — residual
name-based cases outside the receiver-typing surface. Field-sensitivity (#117, dataflow) and taint_reaches (#113,
additive) do not move M2/M3 (call resolution); the perf arc is byte-identical; caddy (Go) is metric-identical
(receiver typing is Rust-only).

## Validity (G4)

| Corpus | Lang | Floor-valid | Note |
|---|---|---|---|
| prism | Rust | ✅ substance | `baseline_invalid=False` (oracle 0.075 < 0.10 floor), `sut_error 0.000`. The Rust anchor. |
| caddy | Go | ✅ | `baseline_invalid=False`, oracle 0.000; metric-identical (Go unaffected by Rust receiver typing) |
| tokio | Rust | ❌ floor (rust-analyzer 0.219 > 0.10 — standing macro/cfg density) | supplementary, non-anchoring; `sut_error 0.000` |
| flask | Python | ❌ floor (pyright 0.362 > 0.25) | non-anchoring; `sut_error 0.000` |
| click | Python | ❌ floor (pyright 0.312 > 0.25) | non-anchoring; `sut_error 0.000` |

Adjudication: precision-AND-recall-positive (receiver typing closed deferred over-claims); **no new prism_fp**
(every stratum fp ≤ the F3 anchor), so no focused per-diff triage was needed beyond the M3 spot-check
(0 confirmed_fp). 5 stale adjudications carried as fixed.

---

# Tier-A Baseline — 2026-06-17 (the Phase-1 scope-graph F3 anchor)

Human-triggered `uv run tier-a --corpus all` on **prism @ `516cd3abacaf`** (the Phase-1 Rust
name-resolution scope-graph: #102 core data-model+engine, #103 populator+build/cache, #104 consumers —
the F3 fix). Refreshes the 2026-06-16 Phase-IP anchor (preserved below as the adjudication substrate; its
classed records carry forward). Run records: `2026-06-17-<corpus>.{json,md}`. Re-pinned `eval/corpora.toml`
prism → `516cd3abacaf`. **This file is the comparison anchor — update only deliberately.**

## What moved (2026-06-17 vs the 2026-06-16 anchor)

**prism M2 callers `C-method`: corrected site recall 0.12 → 0.44** (raw 0.14, n=92, 19 tp / 1 fp / 24 fn),
the scope-graph F3 win — `original_diff.rs`'s local `fn slice` no longer conflates with the 28 sibling
`pub fn slice`; `nav module-deps` resolves real `use`/re-export edges. **caddy/flask/click are
metric-byte-identical** to the 2026-06-16 anchor (the Rust-only F3 change does not touch Go/Python — diffs
are meta-only: date/SHA/wall-clock). **tokio** moved only in tiny-sample strata (n=0, wide CIs) — SUT-shift
noise, non-anchoring. **Capability matrix (G5): 33 ok · 2 expected_gap · 0 regression** (unchanged; the
gaps remain `python/from_import_alias`, `python/inherited_override`).

## Honest gap — prism C-method/callers precision (the full-sample correction)

The F3 PR (#104) reported **precision 1.00** for `callers/C-method`. **That was a `--quick` small-sample
artifact.** The full `--corpus all` sample reveals prism's **standing name-based over-claim precision**:
corrected C-method callers **P = 0.95** (1 fp), with more over-claims in adjacent strata (C-name callers
9 fp, U-method callers 5 fp, callees C-method 4 fp). These are prism's known name-based resolution
limitations — **NOT introduced by F3.**

**F3 is precision-NEUTRAL — it introduced zero new wrong edges.** Focused recall-safety triage of the 26
`prism_only` diffs (codex gpt-5.5 xhigh + operator structural verification): **18 prism_fp, 8 ambiguous,
0 oracle_miss** — and all 18 prism_fp are **pre-existing legacy over-claims F3 structurally cannot
produce**:
- **9 receiver method calls** (`ctx.files.get(...)`, `edge.target()`, `cli.format.as_str()`) — F3's
  `graph_callable_edge` narrows **bare** calls only; receiver method calls stay on legacy.
- **3 trait `Default::default()`** (a `SliceConfig` literal's `..Default::default()` mis-attributed to
  `ThreeDConfig::default`) — trait dispatch, which F3 **declines** (the CHA case → falls through to legacy).
- **6 cross-crate test-helper collisions** (`tests/integration`'s 2-arg `parse` → `tests/ast`'s 3-arg
  `parse`) — the scope graph is **per-crate**; a cross-crate attribution can only come from the legacy
  global-name fallback.

So F3 = **recall-positive, precision-neutral**. The standing over-claims (name-based method/trait/arity/
cross-crate resolution) are **deferred** — outside the Phase-1 bare-in-crate-call surface. The 221
`oracle_only` diffs (prism-missing) are carried recall gaps (the deferred phases), not adjudicated in this
focused triage.

## Validity (G4)

| Corpus | Lang | Floor-valid | Note |
|---|---|---|---|
| prism | Rust | ✅ substance | `baseline_invalid=False`, `corpus_dirty=False` (clean `--allow-drift` run, fresh SUT @ `516cd3abacaf`). The Rust anchor |
| caddy | Go | ✅ | metric-byte-identical to 2026-06-16; clean |
| tokio | Rust | ❌ floor (standing rust-analyzer macro/cfg density) | supplementary, non-anchoring; only tiny-sample drift |
| flask | Python | ❌ floor (pyright noise) | metric-byte-identical to 2026-06-16; non-anchoring |
| click | Python | ❌ floor (pyright noise) | metric-byte-identical to 2026-06-16; non-anchoring |

Adjudication: single-codex focused triage (owner-chosen) + operator structural verification on every
prism_fp; records in `adjudications.jsonl` (`date=2026-06-17`).

---

# Tier-A Baseline — 2026-06-16 (the Phase-IP anchor)

Human-triggered `uv run tier-a --corpus all` on **prism @ `1f7330d`** (Phase-IP: #95 embedding
+ #96 interface foundation + #97 receiver-expansion + the Slice-E dispatch oracle, stacked on
the S2 merge). This **refreshes** the 2026-06-13/14 **S2 anchor** (preserved below as the
adjudication substrate — its 1,536 classed records carry forward unchanged). Only **caddy
method-resolution recall moved**; every other corpus/stratum is identical to the S2 anchor.
Run records: `2026-06-16-<corpus>.{json,md}`. **This file is the comparison anchor — update
only deliberately.**

## What moved (2026-06-16 vs the S2 anchor)

**caddy M2 callers `C-method`: tp 1 → 43 at corrected P = 1.00 (fp 0, fn 0).** Raw site recall
0.01 → 0.40; CI tightens `[0.21–1.00]` (n=1) → `[0.92–1.00]` (n=43). The 42 new resolutions are
gopls-**confirmed** TPs — **0 new adjudicable pending** (no false edge, no adjudication needed).
Every other caddy stratum (callers + callees) and M1 inventory (matched 2519 / extra 0 / missing
100) are **byte-identical** to the S2 anchor. This is the Phase-IP receiver-typing + dispatch
payoff — it closes the `callers/C-method` recall gap the S2 anchor flagged ("the P6-lite/Phase-IP
receiver-typing gap"). Attribution is clean: everything between `dd60ed6` and `1f7330d` is the
Phase-IP increment.

**Capability matrix (G5): 33 ok · 2 expected_gap · 0 regression** (was 29 ok + 4 gap).
`go/embedded_method` + `go/interface_dispatch` flipped to ok; `go/interface_dispatch_assert` +
`go/interface_dispatch_var` (new PR-2 fixtures) landed ok. Remaining gaps:
`python/from_import_alias`, `python/inherited_override` — the **Python** Phase-IP work-list, not
yet addressed (Go embedding + interface dispatch from S2's work-list item 2 are now **DONE**).

**Slice-E dispatch precision (companion audit, same prism `1f7330d`):** the §8 gopls dispatch
oracle over caddy's 63 interface-dispatch sites → `dispatch_precision 0.9994`, dual-adjudicator
κ = 1.000 (21/21), **1 confirmed `prism_fp`** (a name-based arity conflation at a 3-arg
`MiddlewareHandler.ServeHTTP` test site → 2-arg `HandlerFunc`; deferred precision follow-up).
CaddyModule recall is **sound** (prism 121 ⊆ gopls 132, RTA-pruned). Baseline:
`slice-e-caddy-dispatch-baseline.json`; record in `adjudications.jsonl`
(`measurement=interface_dispatch`). So the M2 recall gain is **both high-recall and
high-precision**.

**Validity (unchanged from the S2 anchor):**

| Corpus | Lang | Floor-valid | oracle_err | Note |
|---|---|---|---|---|
| caddy | Go | ✅ | 0.00 | the Go anchor; clean |
| prism | Rust | ⚠️ substance ✅ / report drift | 0.025 | `baseline_invalid` was **only** SHA-drift `dd60ed6 → 1f7330d`; substance floor-valid. **Re-pinned to the #98 merge `278dd70`** (`eval/corpora.toml`); re-run at `278dd70` to validate. Day-to-day drift past the pin is expected (the pin anchors the last baseline, as `dd60ed6` did post-S2) |
| tokio | Rust | ❌ 0.22 > 0.10 | 0.219 | rust-analyzer macro/cfg density; supplementary, non-anchoring |
| flask | Python | ❌ 0.36 > 0.25 | 0.363 | pyright call-hierarchy noise (spec-anticipated v1 finding) |
| click | Python | ❌ 0.31 > 0.25 | 0.313 | pyright call-hierarchy noise |

---

# Tier-A Baseline — 2026-06-13/14 (the S2 anchor — preserved adjudication substrate)

> The fields below describe the **S2 anchor** state (prism @ `dd60ed6`). They remain the
> adjudication substrate (1,536 classed records, κ, gates); fields the 2026-06-16 refresh above
> supersedes (caddy `C-method` callers, the G5 matrix count) are noted there.

Re-anchored full adjudicated run of the Tier-A accuracy harness onto **prism @ `dd60ed6`**
(post-S2 node-identity merge to `main`), from the human-triggered `uv run tier-a
--corpus all` of 2026-06-13, adjudicated 2026-06-14. Supersedes the pre-S2
2026-06-11/12 S3/B2 anchor (preserved in git history). Adjudication record:
`re-anchor-adjudication-2026-06-14.md`.

## Corpus validity (G4)

| Corpus | Lang | Floor-valid | oracle_err | M1 matched / extra / missing | Adjudicated (cumulative) |
|---|---|---|---|---|---|
| prism | Rust | ✅ | 0.05 | 3,627 / 0 / 10 (trait-method decls) | 391 |
| tokio | Rust | ❌ 0.22 > 0.10 | 0.22 | 7,004 / 0 / 237 | 460 |
| caddy | Go | ✅ | 0.00 | 2,519 / 0 / 100 (interface decls) | 564 |
| flask | Python | ❌ 0.36 > 0.25 | 0.36 | 1,367 / 32 / 0 | 50 |
| click | Python | ❌ 0.31 > 0.25 | 0.31 | 1,521 / 62 / 1 | 71 |

Rust anchors on prism, Go on caddy. prism's substance is floor-valid (oracle_err 0.05);
its report-level `baseline_invalid` was **only** the pinned-SHA drift (`144d7c` → `dd60ed6`
after S2 merged) — this re-anchor resolves it by re-pinning `eval/corpora.toml`. caddy is
clean (`baseline_invalid=False`). **Python fails both floors** (pyright call-hierarchy
error 31–36% — the spec-anticipated v1 finding; basedpyright / references-fallback are the
named candidates). **tokio's 0.22** is macro/cfg density — supplementary, not anchoring.

S2 reshaped the call graphs: caddy **471** and tokio **427** prior line-keyed adjudications
went stale (their sites left the live diff). Stale records don't contribute to corrected
metrics; fingerprint re-anchoring is the planned durability migration.

## Acceptance gates

| Gate | Verdict |
|---|---|
| G1(a) corrected U-strata ≥ 0.95 | **Precision: callees MET (U-free 0.98, U-method 1.00); callers U-method NOT MET (0.81** — collision FPs survive even on unique method names**). Recall: NOT MET** (U-free callees 0.87, U-method callees 0.92) — all recorded, not waived. The recall gaps trace to the G5 `expected_gap`s + receiver-typing (Phase-IP). |
| G1(b) pinned `target` known_fail | **FLIPPED → `flip_candidate`** (S2 win): recall **0 → 1.00** (the 5 real `taint.rs` sites recovered by node identity), precision **0.208** (19 surviving `target`-name collisions: petgraph `EdgeRef::target`, etc.). Precision recovery is the EFT increment's success metric (P→1.00 at exact confidence). Probe self-reports the flip (no code change). See `target-c-method-flip-adjudication-2026-06-14.md`. |
| G2 feature-gated oracle-misses | ✅ both rediscovered (`src/mcp/tools.rs:162`, `src/mcp/session.rs:28`), `miss_found=True` |
| G3 snapshot determinism + replay | ✅ every metric here recomputed from stored probes via `--report-only` after appending the 292 new adjudications — zero oracle re-runs |
| G4 floors per language | ✅ Rust (prism), ✅ Go (caddy), ❌ Python (finding above), ❌ tokio (supplementary) |
| G5 capability matrix | **29 ok + 4 expected_gap** — `go/embedded_method`, `go/interface_dispatch`, `python/from_import_alias`, `python/inherited_override` (the Phase-IP work-list). S3 flipped the prior `type_method_qualified` gap to ok. |

## The classed findings (1,536 adjudicated records, `eval/adjudications.jsonl`)

**952 prism_fp — the precision evidence:**
- Collision-prone method names claimed across receiver types at scale: tokio C-method
  callers **P = 0.00 with 406 FPs** (`poll`/`as_fd`/`write`); caddy C-name callers **441
  FPs** (`t.Error`/`zap.Error` attributed to a platform-gated `notify.Error`).
- Stdlib/library methods bound in-corpus (`Vec::truncate`→`AccessPath::truncate`, petgraph
  `.edges()`/`.target()`, map `.get`, `BTreeMap::default`→`*Config::default`) — the
  `target` class, everywhere. EFT (exact-confidence traversal) targets this class.

**421 prism_fn — the recall gaps, now quantified (→ Phase-IP):**
- **Method calls on receiver-typed locals** prism cannot type (the dominant new class):
  `dfg.all_defs_of`, `parsed.enclosing_function`, `provider.resolve_type`. prism
  `callers/C-method` recall is **0.121** corrected (was an optimistic 1.00 when pending
  was excluded). This is the P6-lite/Phase-IP receiver-typing gap.
- `super().m()` and inherited-`self` calls (Python) — `python/inherited_override`.
- Qualified / cross-package calls missed (`caddy.ProvisionContext`, `RegisterModule`).
- Local-helper calls inside `#[test]` / macro args (`assert!(helper(...))`).

**110 ambiguous — interface/dynamic dispatch, fairly excluded:** caddy `x.(Module).CaddyModule()`
across 3 implementers (gopls interface-satisfaction), embedded `l.Listener.Accept()`,
anon-interface `SetConfig`, generic/deref. Excluded from corrected P and R — prism
correctly declines these; the oracle's attribution is the liberal model.

**41 oracle_miss — prism's structural advantage:** feature-gated (`#[cfg]`),
platform-gated (GOOS), untyped-receiver code the compiler-grade oracles cannot see.

**12 oracle_artifact:** pin-project attribute-macro `self.project()`, enum/tuple-variant
constructors counted as calls (`Ok(())`), pyright property-getters.

## Methodology

- **Dual-adjudicator** (codex gpt-5.5 xhigh + claude opus-4-8, identical hydrated
  evidence): prism κ=**0.923** (180/182), caddy κ=**0.900** (70/72); 4 disagreements,
  all operator-tiebroken via source to the claude verdict. Supplementary
  (tokio/flask/click) were solo-claude and non-anchoring. Records carry adjudicator
  identity. Full record: `re-anchor-adjudication-2026-06-14.md`.
- 292 net-pending diffs adjudicated → store 1,244 → **1,536**; all adjudicable pending
  now drained (0). Residual per-stratum pending (e.g. caddy `callers/C-method` 48) is
  non-adjudicable `inventory_miss` (interface/trait declaration seeds, counted by M1).

## Next-increment work-lists

1. **EFT — precision** (`docs/superpowers/specs/2026-06-14-prism-exact-functionid-traversal-design.md`):
   exact-FunctionId / confidence-aware caller·callee traversal eliminates the
   name-collision FP class; success metric is `target-c-method` P 0.208 → 1.00 at exact
   confidence, recall held 1.00.
2. **Phase-IP — recall**: the 4 G5 `expected_gap`s (go embedded/interface, python
   inherited/import-alias) plus field/return-typed receiver typing (the C-method caller
   recall 0.121 class).
3. **Python oracle**: replace pyright (floor-failed) — basedpyright / references-fallback.
4. **Adjudication durability**: fingerprint-keyed records (line-keyed records went stale
   under S2 churn: caddy 471, tokio 427).
