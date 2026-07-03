> **Status: SHIPPED — PR #151 (merged 2026-07-03).** As-executed brief incl. codex corrections (score through stored-site replay; `Adjudication.tier`; gate wording). Post-spec fix wave: live SUT fail-fast on missing score (`SutError`; None-tolerance is stored-replay only) + `tier` preserved through `hydrate_pending`.

# Task P6a — Confidence-stratified M2 reporting (eval harness only)

You work in the git worktree `/private/tmp/prism-p6a-m2-strat` on branch `p6a-confidence-stratified-m2` (based on main @ 4a82f50). This task touches ONLY `eval/` (Python, uv) — no Rust changes. Purpose: make Tier-A's M2 measurement distinguish exact-confidence edges from labeled candidate edges, so the upcoming P3 change (which emits capped NameOnly candidate edges for Python/JS/TS unknown receivers instead of silently dropping) can be gated on "exact tier unchanged; candidates reported separately" instead of being punished as plain FPs.

## Ground truth (verified)

- Prism nav JSON items carry `score` (direct callers/callees at M2's depth: exactly `1.0` = Exact confidence, `0.6` = NameOnly; the hop divisor is 1 at this depth) and `why[].Resolution.kind`.
- `eval/tier_a/sut.py` parses `why` for `CalledBy`/`Calls`/`Resolution` (`_why` at :61; callers parse ~:81-96, callees ~:97-110) and builds `CallEdge` (`eval/tier_a/model.py:36-42`) with `resolution_kind` but **no score**.
- M2 P/R is computed in `_compute_m2_and_pending` (`eval/tier_a/cli.py:192-284`); per-site metadata flows at `cli.py:350-365`; pending records get `dispatch_kind` (`cli.py:170-187`). Corrected metrics come from `adjudication.apply_verdicts` (`eval/tier_a/adjudication.py:129-191`). Report rendering in `eval/tier_a/report.py:13-65`.
- Matrix (`eval/tier_a/matrix.py`) already supports `expected_resolution_kind`/`forbid_resolution_kind` — no matrix changes needed.

## Changes

1. **Capture score.** Add `score: float | None = None` to `CallEdge` (model.py) and populate it in both the callers and callees parsers in sut.py from the Evidence item's `score` field.
2. **Tier classification.** One function, one place (e.g. model.py or metrics.py): `edge_tier(edge) -> "exact" | "candidate"`. Rule: `score is None` → `"exact"` (legacy stored sites without score — preserves today's all-together counting for old run replays; document this rationale in a comment); `score >= 0.999` → `"exact"`; otherwise `"candidate"`.
2b. **Score must survive replay (spec-review MAJOR).** M2 recomputation and `--report-only` replay rebuild edges from stored `probes[*].prism_sites` via `_edges()` (cli.py:62) — capturing score on live `CallEdge` alone is NOT enough: persist `score` into the stored-site metadata in `_stored_sites()` (cli.py:~350) and read it back in `_site_parts`/`_edges`, else every replayed edge classifies as legacy-exact.
2c. **Adjudication tolerance (spec-review MAJOR).** `Adjudication(**json.loads(line))` (adjudication.py:54) rejects unknown keys — a candidate pending record copied into adjudications.jsonl with a `tier` key would crash loading. Add `tier: str | None = None` to the `Adjudication` dataclass.
3. **Stratified M2.** In `_compute_m2_and_pending`: keep every existing output field EXACTLY as computed today (the `raw`/`corrected` P/R over ALL edges — baselines in docs/eval/tier-a/ must remain comparable). ADD per stratum×direction: `exact_tier: {raw precision/recall, tp/fp/fn counts}` computed over prism edges classified "exact" only (oracle set unchanged), and `candidate_tier: {count, oracle_confirmed, oracle_unconfirmed}`. To be precise (spec-review wording fix): the legacy `raw`/`corrected` fields REMAIN all-edge (candidates included, exactly as today) for baseline comparability; the P3 gate reads `exact_tier` ONLY; `candidate_tier` is informational + adjudication-fed and is never a pass/fail input. Candidate-edge diff sites become pending records tagged `"tier": "candidate"` (existing prism_only pendings keep their shape; just add the tier key for candidate-tier ones so adjudication can filter).
4. **Report.** In report.py, render the stratified block compactly after the existing M2 table (one line per stratum with exact-tier P/R and candidate counts). Run-JSON: the new fields ride along in the same structures; `--report-only` replay (`cli.py:294-304`) must keep working on OLD run JSONs that lack the new fields (guard with .get defaults).
5. **P3 gate helper.** Document (in eval/README.md, a short paragraph) how P3-style changes are gated: "exact-tier P/R must be unchanged vs the pre-change run; candidate tier is informational + adjudication-fed". No new CLI needed if the fields are in the run JSON.

## Tests

Follow existing eval test patterns (look at eval/tests/ for tier-a tests; if none cover M2 directly, add a focused unit test module): synthetic probes with mixed-score edges → assert (a) legacy fields byte-identical to a no-score run, (b) exact/candidate tiers split correctly, (c) None-score edges classify exact, (d) `--report-only` on an old-format run JSON doesn't crash.

## Done-checks (run and paste into your report)

```
cd eval && uv run pytest tests/ -x -q            # or the repo's test invocation — discover it
cargo build --release                             # needed for tier-a SUT
cd eval && uv run tier-a --matrix-only --allow-stale-sut     # unchanged, 0 regressions
cd eval && uv run tier-a --quick --allow-stale-sut           # new stratified fields present; legacy fields match the shape of docs/eval/tier-a/ baselines
```
Paste the new stratified block from the --quick report.

## Commit style
Conventional subjects (`feat(tier-a): ...`). End each commit message with:
Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
