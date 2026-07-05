# Part-C Scorecard v2 — implementation report

Branch `partc-rubric-v2` (base `da7e4d0`), worktree `/private/tmp/prism-partc-rubric`.
HARNESS Python only — nothing under `src/` (Rust) touched; confirmed via
`git diff da7e4d0..HEAD --stat -- src/ Cargo.toml Cargo.lock` (empty).

4 commits, oldest first:
1. `ad0166f` — D0 repair (recall denominator + recall surfaced on PartCCell)
2. `13adf09` — D1–D4 dimension modules + `_LivePartCComps` hooks
3. `dd1611d` — wiring into `PartCCell` / `run_partc_cell` / `render_partc`
4. `fb05861` — fix: `_CachedComps` wrappers weren't forwarding the new hooks
   (found while manually verifying rescorability; see "Deviations" below)

Full suite: `cd eval && uv run pytest -q --ignore=adoption` → **651 passed**
(586 pre-existing + 65 new). No live agent arms were run.

## Cardinal constraint — rescorability

Every dimension is computed inside `run_partc_cell` (`eval/tier_c/partc.py`) from
`ArmOutput.text`/`.citations` alone, or via `_LivePartCComps` methods that only read
`self._co`/`self._ask` + the arm text passed in. `rescore.py` never mutates the source
run dir (unchanged `rescore_run_dir`/`rescore_cell` contract). Verified live with a
manual `rescore_cell(...)` call using saved `ArmOutput`s + a fake `ask`/`Checkout`,
confirming D0–D4 all populate without re-running any arm (see "Deviations" — this
manual check is what surfaced the `_CachedComps` forwarding bug, now fixed and pinned
by `test_rescore_cell_forwards_all_scorecard_v2_hooks`).

Ensemble guards preserved byte-identical: `git diff da7e4d0..HEAD -- eval/tier_c/judges.py
eval/tier_c/judges_live.py eval/tier_c/detect.py` is empty. The blind `SpecQualityJudge`
(`judges_live.py:79-110`) is untouched; D1/D3 reuse `ensemble.py`'s existing
2-sonnet/opus-tiebreak infra rather than adding new judge machinery.

## D0 — recall-denominator repair

- **Bug**: `eval/tier_c/cli.py` `_LivePartCComps.score` passed
  `claim_count=max(len(citations), 1)` into `investigator.score_citations`, forcing
  `recall == precision` whenever ≥1 citation existed. A second, related bug:
  `eval/tier_c/partc.py` `run_partc_cell` hardcoded `recall_on=None, recall_base=None`
  even though `InvestigatorReport.recall` was already computed — recall was never
  surfaced at all.
- **Fix**: `score()` now uses `claims.count_claims(arm_text)` (the arm's own text, held
  on `self._last_off`/`self._last_on` after `run_off_arm`/`run_on_arm`) as the
  denominator, falling back to the old behavior only when no arm text is available
  (direct callers). `run_partc_cell` now sets `recall_on=on_rep.recall,
  recall_base=off_rep.recall`. `rescore.py`'s `_CachedComps.run_off_arm`/`run_on_arm`
  now set `live._last_off`/`live._last_on` so the fix works under rescore too.
- **Synthetic before/after** (from `test_tc_d0_recall_repair.py`): an arm's text with 5
  substantive code claims (`count_claims` = 5: `parse_config()`, `load_file()`,
  `config.py`, `shared_state`, `emit_metrics()`/`metrics.log`) but only 2 valid
  citations:
  - Before: `claim_count = max(2, 1) = 2` → `recall = 2/2 = 1.0` (== precision, penalty
    invisible)
  - After: `claim_count = count_claims(text) = 5` → `recall = 2/5 = 0.4` (visibly below
    precision 1.0 — the under-citing penalty is now visible)
- **Symbols**: `eval/tier_c/cli.py:_LivePartCComps.score`, `eval/tier_c/partc.py:run_partc_cell`,
  `eval/tier_c/rescore.py:rescore_cell._CachedComps`, `eval/tier_c/claims.py:count_claims`
  (reused, unchanged).
- **Tests**: `eval/tests/test_tc_d0_recall_repair.py` (5 tests, failing-first — verified
  3/5 fail against pre-fix code via `git stash`, all 5 pass after).

## D1 — citation VALIDITY (judged)

- **New module** `eval/tier_c/validity.py`: `CitationClaim` (cite + enclosing sentence),
  `extract_citation_claims` (pairs every citation *occurrence*, not deduped, with its
  sentence via a new `citations.iter_citation_occurrences` + `claims.sentence_spans`),
  `CitationValidityJudge` (ensemble-backed SUPPORTED/UNSUPPORTED/CONTRADICTED, default
  UNSUPPORTED on unparse), `score_validity` (claim-support **rate**, never a raw count;
  unresolvable citations get a conservative UNSUPPORTED without spending an ensemble
  call).
- **Wiring**: `_LivePartCComps.score_validity(text, *, arm)` in `cli.py`; called from
  `run_partc_cell` (hasattr-gated, same optionality pattern as the pre-existing
  `head_to_head`).
- **Tests**: `eval/tests/test_tc_validity.py` (13 tests) — extraction/sentence-pairing,
  ensemble parsing + escalation + conservative default, the anti-volume rate guarantee
  (duplicate citation across two sentences scores as 2 claims), unresolvable citations
  never spend an ensemble call.

## D2 — relational-fact accuracy (mechanical) — **partially stubbed, as the spec permits**

- **New module** `eval/tier_c/relational.py`: `RelationalClaim`/`parse_relational_claims`/
  `extract_relational_claims` (one blind extractor call, same model+prompt on both
  arms), `CallOracle` protocol + `NullCallOracle` (calls()/called_by() stub — see
  below), `extract_imports` + `confirm_depends` (neutral per-language import-text
  parser: go/rust/python/js/ts), `score_relational_claims` (precision excludes UNKNOWN;
  UNKNOWN rate always reported; CONTRADICTED only on positive disproof).
- **Status — escalating per the spec's explicit allowance**: `depends(modA, modB)` is
  **fully implemented** (mechanical, no LSP: resolves modA via the checkout's
  basename index, parses its imports, checks for modB). `calls(X,Y)`/`called_by(X,Y)`
  are **stubbed to UNKNOWN** via `NullCallOracle` — wiring `tier_a.oracles.LspOracle`
  properly needs a live per-language server lifecycle (rust-analyzer/gopls/pyright)
  plus `workspace/symbol` → `FunctionDef` resolution for arbitrary claimed
  caller/callee names, which is materially more glue than fits this pass safely
  without live-repo testing. This is the one deliberate scope cut in the whole
  implementation; a `TODO(D2 follow-up)` marks the exact spot in
  `relational.py:NullCallOracle`.
- **Fail-open verified**: `test_score_relational_claims_precision_excludes_unknown` and
  `test_score_relational_claims_never_auto_contradicts_calls` pin down that an
  unconfirmable claim is UNKNOWN, never auto-CONTRADICTED — matching the oracle_miss
  caveat (Tier-A: prism-right/LSP-wrong cases exist).
- **Wiring**: `_LivePartCComps.score_relational(off_text, on_text, *, cell)` in
  `cli.py`; new `Checkout.read_file` in `checkout.py` (whole-file read, needed for
  import-text parsing — `read_window` is excerpt-only).
- **Tests**: `eval/tests/test_tc_relational.py` (22 tests).

## D3 — fact-annotated head-to-head

- **New module** `eval/tier_c/annotate.py`: `hallucinated_keys`/`contradicted_keys`
  (derive tag-target keys from D0's `CitationVerdict.is_hallucination` and D1's
  CONTRADICTED verdicts), `annotate_arm_text` (mechanical inline tagging —
  `[CITED LOCATION DOES NOT EXIST]` / `[CODE CONTRADICTS THIS CLAIM]`, run identically
  on both arms), `run_annotated_detectability` (pools annotated texts across
  **multiple** `PartCCell`s and re-runs `detect.run_detectability` unchanged).
- The blind `SpecQualityJudge` stays byte-identical; `head_to_head_annotated` in
  `cli.py` is a fully separate call feeding it the annotated pair instead.
  `render_partc` shows both verdicts and flags divergence
  (`<- DIVERGES from blind head-to-head`) when they disagree.
- **Not yet run**: `run_annotated_detectability` is implemented and unit-tested against
  synthetic pools, but has **not been executed against the real saved pilot run
  directories** (`pilot-0705`, `pilot-0705-pyd`, `smoke-0705-matched`,
  `smoke-0705-fixed`) in this pass — doing so needs live ensemble judge calls against
  real saved texts, which is a rescoring action for whoever runs it next, not an
  implementation-time check. `detect.py` itself is untouched.
- **Tests**: `eval/tests/test_tc_annotate.py` (13 tests) — tag placement (single/both
  tags, only-the-flagged-occurrence, both-tags-on-one-occurrence), and the pooled vs.
  single-cell detectability guarantee (mirrors `test_tc_detect.py`'s own
  underpowered-single-stage case).

## D4 — navigation efficiency (mechanical, free)

- **New module** `eval/tier_c/naveff.py`: `NavEfficiency` dataclass + `nav_efficiency`/
  `nav_efficiency_to_dict`. Pure aggregation over `ArmOutput.tool_calls`/`wall_s`/
  `cost_usd`/`commands` plus the citation set — no judge, no new live calls.
- **Documented degradation**: the harness has no per-tool-call timestamp, so "calls/wall
  to the first valid citation" (as literally specified) cannot be computed exactly;
  arm-level totals are reported instead (labeled as totals, not mislabeled as
  first-citation timing). `wasted_exploration_rate` further degrades to `None` (not a
  fabricated `0.0`) whenever `ArmOutput.commands` is empty — which is **every**
  claude/opus arm today, since `parse.parse_claude_stream_json` doesn't populate
  `ModelResult.commands` (only `parse_codex_jsonl` does, from `command_execution`
  items). This is a real, pre-existing harness gap, not something this pass can close
  cleanly without touching the claude stream parser's tool-args capture.
- **Tests**: `eval/tests/test_tc_naveff.py` (7 tests) — cost-per-valid-citation,
  wasted-exploration computed vs. degraded-to-None, totals pass through unchanged.

## Wiring (`partc.py`)

`PartCCell` gained 8 fields: `validity_off`/`validity_on`, `relational`,
`head_to_head_annotated`, `annotated_off_text`/`annotated_on_text`, `nav_eff_off`/
`nav_eff_on` — all plain dicts (matching the existing `off_breakdown`/`gate`/
`head_to_head` convention), all with `field(default_factory=dict)` defaults so every
pre-existing direct `PartCCell(...)` construction in the test suite is unaffected.
`run_partc_cell` computes D1/D2/D3-ensemble via hasattr-gated optional comps hooks
(same pattern as the pre-existing `head_to_head`); D3's annotation and D4 are pure and
always run. `render_partc` prints one line per dimension, only when populated — nothing
is collapsed into a single number.

**Deviation from the spec's literal file:line anchor**: the spec's "Wiring" section
names `report.py` as the renderer to extend. Verifying against the code: `report.py`
holds `Cell`/`Cell2x2` for the *separate* 8-variant (`run.py`) harness; the actual
Part-C-cell renderer is `render_partc` in `partc.py` (used by every `cli.py` Part-C
call site: `run-partc`, `rescore`, the live path). I extended `render_partc` instead —
`report.py` is untouched.

## Deviation found mid-implementation: `_CachedComps` forwarding gap

Both `cli.py`'s `_run_partc_live` and `rescore.py`'s `rescore_cell` wrap the real
`_LivePartCComps` in a local `_CachedComps` class before calling `run_partc_cell`.
`run_partc_cell`'s `hasattr()` gates check the **wrapper**, not `_LivePartCComps`
directly — so without explicit forwarding methods (mirroring the pre-existing
`head_to_head` forwarding), D1/D2/D3 were silently skipped on both the live path and,
critically, the **rescore path** — defeating this whole pass's rescorability
requirement. Caught by a manual end-to-end `rescore_cell(...)` smoke test (not by the
unit tests, which mock at the `_LivePartCComps` boundary and don't exercise the
wrapper). Fixed in commit `fb05861`; `rescore.py`'s wrapper forwards unconditionally
(it always holds a real `_LivePartCComps`), `cli.py`'s wrapper forwards with an
internal `hasattr` guard (it may hold a test's monkeypatched minimal fake). Pinned by
`test_rescore_cell_forwards_all_scorecard_v2_hooks` (verified failing-first via
`git stash` of the fix).

## Other deviations / notes

- `SlicingAlgorithm`/Rust code: untouched, as required.
- No new CLI flags were added — `render_partc` (used by `run-partc`, `rescore`, and the
  live path) automatically shows the new dimensions, so `tier-c rescore --run-dir ...`
  surfaces D0–D4 with no new surface to learn.
- `judges.py`'s `family_bias` is unrelated to Part-C's `SpecQualityJudge`/ensemble path
  (it's used by the separate 8-variant harness in `report.py`/`run.py`) and was neither
  touched nor needed by D1/D3's new judges, which follow the same
  ensemble-without-family-bias precedent as the pre-existing `LlmRelevanceJudge`/
  `SpecQualityJudge`.

## Resolver fix — fair citation resolution (R1–R5)

A follow-on pass on `partc-rubric-v2` (5 commits after `fb05861`), fixing a CONFIRMED
measurement artifact discovered while auditing the pilot's `ruff:spec:opus-4.8` cell:
`Checkout.resolve_rel` marked a bare filename **unresolved** (→ scored as
"hallucination" by `investigator.py`) whenever its basename was non-unique among
tracked files, *even when the cited line was real*. The pilot repo has 6 files named
`noqa.rs`; only the 3023-line one has line 1014. The off-arm cited real, relevant lines
with bare filenames — 19/20 of its citations scored as hallucinated purely because of
this artifact, producing a false pro-prism flip in every downstream dimension (D0
precision, D1 validity, D3's annotated head-to-head). **The load-bearing principle**:
ambiguous-but-real citations must never score as fabrication — only a line that exists
in NO candidate file is a hallucination. Resolvability (does the path need
disambiguating?) is now reported as its own axis, never conflated with truth.

### R1 — layered resolver (`checkout.py`)

`Checkout.resolve_cite(file, line, symbol, claim_text)` replaces the binary
exact-or-unique-basename-or-None check with 5 ordered layers, each returning a
`ResolveResult(status, path, layer)` with `status ∈ {RESOLVED, AMBIGUOUS, ABSENT}`:

1. **exact** path exists.
2. **unique_basename** — one tracked file with that basename (the old "current
   behavior").
3. **line_range** / **line_symbol** / **line_tokens** — ambiguous basename, narrowed by
   (a) the cited line being in range for exactly one candidate, then (b) the symbol
   appearing on that line, then (c) the claim sentence's salient tokens appearing in
   the candidate's window. Deterministic; this layer alone fixes ~all of the pilot
   artifact (only the long `noqa.rs` has line 1014).
4. **llm_disambiguated** — still ≥2 candidates → the R2 Q3 disambiguator (haiku) picks
   one, or abstains.
5. **ABSENT** — the line exists in no candidate (or no basename matches at all) — the
   ONLY case now scored as fabrication.

`resolve_rel(str) -> str | None` stays as a thin back-compat shim over `resolve_cite`
(`RESOLVED → path`, else `None`) for line-less callers (`relational.py`'s module
resolution) — unchanged behavior for every existing caller/test.

### R2 — Q3 LLM disambiguator (`disambiguate.py`, new module)

`disambiguate(ask, claim_text, candidates, model="haiku-4.5")` — invoked by
`resolve_cite` ONLY when layers 1–3 still leave ≥2 candidates (rare by design: R1's
deterministic layers already resolve the pilot's whole artifact). Prompts with the
claim sentence + each candidate's code window labeled Candidate A/B/…; first-token
`A/B/…/NONE`. `NONE` or an unparsed reply abstains (→ `AMBIGUOUS`, never a guess).
`make_disambiguator(ask)` builds the `(claim_text, windows) -> int | None` callable
`resolve_cite`'s `disambiguate=` seam expects, swallowing any `ask` exception as
abstain (a judge-call failure must never crash scoring). Wired behind the harness's
existing `ask()` seam in `investigator.verify_citation`/`validity.score_validity`
(`cli.py` threads `self._ask`) — zero new live-call surface.

### R3 — three-way classification (`investigator.py`)

`CitationVerdict` gains `ambiguous: bool = False` and `resolve_layer: str = ""` (both
defaulted so every pre-existing construction — dozens across the suite — is
byte-for-byte unaffected). `is_hallucination`/`is_valid` now exclude `ambiguous`
explicitly:

- **valid** — RESOLVED + line/symbol ok + relevant (now reachable for bare-but-real
  citations).
- **hallucinated** — ABSENT or a resolved file with a bad line/symbol. True fabrication
  only.
- **ambiguous** — AMBIGUOUS (real line, ≥2 candidates, disambiguator abstained/absent).
  Reported separately (`InvestigatorReport.ambiguous`/`.ambiguous_rate`), never counted
  as a "fail" in `partc.py`'s per-arm breakdown.

`score_citations`' precision denominator excludes `ambiguous`
(`denom = len(verdicts) - ambiguous`) — a strict generalization of the old
`valid/len(verdicts)` formula (identical when `ambiguous == 0`, i.e. every pre-existing
path/fake): resolved-but-irrelevant citations still count against precision exactly as
before (that pre-existing behavior is orthogonal to this fix and preserved), only the
newly-distinguished AMBIGUOUS bucket is excluded. Recall's denominator
(`claim_count`, D0) is untouched.

### R4 — resolvability axis (`investigator.resolvability_breakdown`, `partc.py`)

New, mechanical, no comps hook (same "free" pattern as D4 nav-eff): per arm, the
fraction of citations that were **full-path** (`resolve_layer == "exact"`),
**bare-resolved** (needed layers 2–4), **ambiguous**, or **absent** — the four buckets
are mutually exclusive and sum to the citation count. This is prism's honest edge (it
hands the agent exact resolvable paths) and is kept STRICTLY separate from
precision/validity — `render_partc` prints it as its own `R4 resolvability` line with
`Δfull-path` (on − off), never folded into the precision delta.

### R5 — D1/D3 rewired onto the fixed resolver

- **D1 validity** (`validity.py`): `score_validity` now resolves each citation-claim via
  `resolve_cite` (falling back to `resolve_rel`/`file_exists` for fakes that don't
  implement it) instead of the line-blind `resolve_rel`-or-`file_exists` check. A
  bare-but-real citation now RESOLVES, so the validity judge actually reads its code
  window instead of auto-`UNSUPPORTED`; only a truly-ABSENT (or still-AMBIGUOUS after
  the Q3 disambiguator's shot) citation skips straight to `UNSUPPORTED` with no ensemble
  call spent. `cli.py` threads `self._ask` in so a genuinely-tied bare citation gets the
  same disambiguation chance under D1 as under D0.
- **D3 annotation** (`annotate.py`): `hallucinated_keys` (built on the now-correct
  `is_hallucination`) no longer includes AMBIGUOUS citations, so `[CITED LOCATION DOES
  NOT EXIST]` is never applied to a bare-but-real off-arm citation — this was the direct
  cause of the annotated-head-to-head flip. A new `ambiguous_keys()` + `annotate_arm_text`
  `ambiguous=` kwarg (default empty, backward compatible) apply a neutral
  `[AMBIGUOUS PATH]` tag instead, never the fabrication tag.

### The noqa.rs before/after (concrete repro, same shape as the pilot)

20 off-arm citations: 1 full-path + 19 bare `noqa.rs:1014` (real — only the 3023-line
candidate among 6 same-basename files has line 1014), scored via
`investigator.score_citations` against a real `Checkout` (git worktree fixture, same
shape as `test_real_checkout_bare_ambiguous_but_real_citation_scores_valid_not_hallucinated`
in `test_tc_investigator.py`):

| | valid | hallucinated | ambiguous | precision |
|---|---|---|---|---|
| **before** (`resolve_rel`, line-blind — old behavior) | 1/20 | 19/20 | n/a (no 3rd bucket) | **0.050** |
| **after** (`resolve_cite`, R1 layered resolver) | 20/20 | 0/20 | 0/20 | **1.000** |

This mirrors the pilot's own 19/20-scored-as-hallucinated measurement almost exactly —
the 19 bare citations all resolve via the `line_range` layer (deterministic, no LLM
call spent), matching R1's claim that the line-range layer alone fixes ~all of this
artifact. (A genuinely tied basename — e.g. two same-basename candidates both having
the cited line in range with no symbol/token signal — instead resolves to
`ambiguous`, excluded from precision rather than counted against it; see
`test_resolve_cite_genuinely_ambiguous_two_real_candidates_no_disambiguator` and
`test_score_citations_excludes_ambiguous_from_precision_denominator`.)

### Tests

Failing-first verified: stashing `checkout.py`/`investigator.py`/`validity.py`/
`annotate.py`/`partc.py`/`cli.py`/`chain.py` (`git stash push -- <files>`) and re-running
the new tests reproduces the artifact directly — `test_tc_checkout.py` fails to even
import (`ABSENT`/`AMBIGUOUS`/`ResolveResult` don't exist yet), and
`test_real_checkout_bare_ambiguous_but_real_citation_scores_valid_not_hallucinated`
fails with `file_ok=False` / `is_hallucination=True` on the exact noqa.rs shape — then
passes after `git stash pop`.

New/updated test files: `test_tc_checkout.py` (+12: layered `resolve_cite`, the noqa.rs
artifact regression, the genuinely-ambiguous-no-disambiguator case, symbol tie-break,
injected-disambiguator tie + abstain, the `resolve_rel` shim's documented lossy limit),
`test_tc_disambiguate.py` (new, 11 tests: picks/abstains/exception-swallowing/model
override, zero live calls), `test_tc_investigator.py` (+8: absent/ambiguous/resolved
three-way classification, precision-excludes-ambiguous, the end-to-end real-`Checkout`
artifact regression, `resolvability_breakdown`), `test_tc_validity.py` (+2: bare-but-real
resolves and reaches the judge window, absent stays `UNSUPPORTED` with zero ensemble
calls), `test_tc_annotate.py` (+3: `ambiguous_keys`, neutral tag never the fabrication
tag, default-empty backward compat), `test_tc_partc_v2.py` (+2: `resolvability_off/on`
wiring, the `R4 resolvability` render line).

`cd eval && uv run pytest -q --ignore=adoption` → **687 passed** (651 prior + 36 new).
No live agent arms were run. `git diff fb05861..HEAD --stat -- src/ Cargo.toml
Cargo.lock` is empty (HARNESS Python only, confirmed again for this pass).
