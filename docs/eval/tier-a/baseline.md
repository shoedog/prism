# Tier-A Baseline — 2026-06-11/12 (the S3/B2 anchor)

First full adjudicated run of the Tier-A accuracy harness
(spec `docs/superpowers/specs/2026-06-11-prism-tier-a-accuracy-harness-design.md` rev 4;
prism @ the tier-a branch, oracle versions in the per-corpus reports). **This file is
the comparison anchor — update only deliberately.**

## Corpus validity (G4)

| Corpus | Lang | Floor-valid | oracle_err | M1 matched / extra / missing | Adjudicated diffs |
|---|---|---|---|---|---|
| prism | Rust | ✅ | 0.10 | 3,415 / 0 / 10 (trait-method decls) | 174 |
| tokio | Rust | ❌ 0.22 > 0.10 | 0.22 | 7,000 / 0 / 243 | 440 |
| caddy | Go | ✅ | 0.00 | 2,519 / 0 / 100 (interface decls) | 491 |
| flask | Python | ❌ 0.36 > 0.25 | 0.36 | 1,367 / 32 / 0 | 38 |
| click | Python | ❌ 0.31 > 0.25 | 0.31 | 1,521 / 62 / 1 | 59 |

Rust anchors on prism, Go on caddy. **Python fails both floors — the spec-anticipated
v1 finding**: pyright call-hierarchy error rates of 31–36% on small, comparatively
typed corpora gate all Python accuracy claims until a better Python oracle lands
(basedpyright / references-fallback are the named candidates). tokio's 0.22 floor
breach is macro/cfg-density (the full feature matrix is off by default); its numbers
are reported as supplementary, not anchoring.

## Acceptance gates

| Gate | Verdict |
|---|---|
| G1(a) corrected U-strata ≥ 0.95 | **Precision: callees MET (0.99); callers NOT MET (0.89** — five adjudicated FPs surfaced when the final-review 1:1 matching fix removed greedy many-to-one credit**). Recall: NOT MET** (callers 0.89, callees 0.70) — all recorded, not waived. The prototype's "perfect on unique names" was a caller-direction, non-qualified-call artifact of its 8-symbol sample; at scale, unique-name *recall* has two structural gaps (below). |
| G1(b) pinned `target` known_fail | ✅ reproduced (raw P=R=0) |
| G2 feature-gated oracle-misses | ✅ both rediscovered (`src/mcp/tools.rs:162`, `src/mcp/session.rs:28`) and seeded as adjudications |
| G3 snapshot determinism + replay | ✅ exercised throughout: samples snapshot-derived; every metric in this baseline was recomputed from stored probes via `--report-only` replay (incl. after the two §"methodology" fixes, with zero oracle re-runs) |
| G4 floors per language | ✅ Rust (prism), ✅ Go (caddy), ❌ Python (finding above) |
| G5 capability matrix | ✅ 27 ok + 2 expected_gap (`type_method_qualified`, `from_import_alias`); statuses binary-reconciled |

## The classed findings (1,218 adjudicated records, `eval/adjudications.jsonl`)

**922 prism_fp — the S3 precision evidence, now measured:**
- Collision-prone method names claimed across receiver types at devastating scale:
  tokio C-method callers corrected **P = 0.00 with 390 FPs** (`poll`/`as_fd`/`write`);
  caddy C-name callers **441 FPs** (every `t.Error`/`zap.Error`/`caddyhttp.Error`
  attributed to a platform-gated `notify.Error`).
- Stdlib methods bound in-corpus (`Vec::truncate`→`AccessPath::truncate`,
  petgraph `.target()`, map `.get`) — the prototype's `target` class, everywhere.

**215 prism_fn — the recall gaps G1(a) exposed:**
- Qualified `Type::fn` / `mod::Type::fn` calls missed (matches the capability matrix's
  `type_method_qualified` known_fail) — the dominant U-callee class.
- Constructor edges unmodeled: tuple-struct `Self(..)`, enum variants, Python
  class/exception instantiation.
- Local-helper calls inside `#[test]` modules missed.

**26 oracle_miss — prism's structural advantage, quantified:** feature-gated
(`#[cfg]`), platform-gated (GOOS), and untyped-receiver (pytest fixtures, decorator
objects) code the compiler-grade oracles cannot see.

**7 oracle_artifact / 35 ambiguous:** property-getters counted as calls by pyright,
method-values vs calls in Go, generic/deref dispatch where attribution is undecidable.

## Methodology validated this run

- **Dual-adjudicator protocol** (owner-approved): a 78-item blinded sample judged
  independently by claude-fable-5 and codex-gpt-5.5 — raw agreement 83%, κ≈0.74,
  **zero FP↔FN flips**; post-probe-resolution 87%. Codex then bulk-adjudicated 1,130
  items with high-confidence FP/FN accepted under class rationale and 85 escalations
  re-judged individually. Records carry adjudicator identity.
- **Two harness fixes the sample surfaced** (both committed with tests): multi-line
  method-chain line tolerance in `site_compare` (receiver-line vs name-line phantom
  FP/FN pairs), and exclusion of inventory-miss (declaration-seeded) probes from
  pendings (Go interface / Rust trait declarations — counted by M1, not adjudicable).
- The probe-resolution example that reversed a class verdict (caddy interface
  dispatch: prism has the edge at the concrete impl) is in
  `docs/prism-query-layer/tier-a-task8-review-2026-06-11.md`'s sibling records and
  the run JSONs.

## S3 work-list distilled

1. Collision-method caller claims (the P=0.00-at-scale class) — receiver/type-aware
   filtering or confidence demotion of name-only method edges.
2. Qualified `Type::fn`/`mod::Type::fn` call binding (flips `type_method_qualified`).
3. Constructor edges (`Self(..)`, enum variants, Python classes).
4. M2 seed mapping for declarations (interface/trait) → implementing methods.

## Post-final-review amendment (2026-06-12)

The whole-branch final review's MAJOR fixes (notably 1:1 site matching replacing
greedy many-to-one) were applied and the baseline replayed: 11 newly-exposed diffs
adjudicated (9 real-call accounting leftovers credited oracle_miss; 2 a new
**nested-def attribution** FP sub-class — callee edges inside nested `@app.route`
handlers attributed to the enclosing test function — added to the S3 work-list).
G1(a) callers corrected precision moved 1.00 → 0.89: the greedy matcher had been
crediting adjudicated FPs that sat within the chain-tolerance window of real calls.
Honest number stands.
