> **Status: SHIPPED — PR #153 (merged 2026-07-03).** As-executed brief incl. codex corrections (fanout cap on `method_ids.len() <= 3`, not owner count; `[expect] resolution_kind` fixture key; sites-returning collision helper). Post-spec fix waves (merged): candidates made NAV-ONLY (excluded from Step-5b arg→param DataFlow and echo/membrane findings — origin of the consumer-visibility doctrine) and the membrane gate corrected to let graph-only CHA callers through. Measured: black `dropped_multi_owner` 230→30, +576 candidates, `kind_exact` byte-identical.

# Task P3 — Candidate edges instead of silent drop for Python/JS/TS unknown receivers

You work in the git worktree `/private/tmp/prism-p3-candidates` on branch `p3-candidate-edges` (based on branch `p6a-confidence-stratified-m2`, which adds confidence-stratified M2 to the eval harness — eval-only, no Rust overlap). The repo is prism. Follow TDD.

## Problem (verified; the recorded "Python returned 0 callers" incident)

An unknown-receiver call `x.m()` whose method name exists on >1 owner class is **dropped** at the R6 residue: src/resolution.rs:1493-1507 — `owners.len() == 1` → `demoted(..., R6SingleOwner)` (NameOnly), `owners.len() > 1` → `ResolutionOutcome::dropped(DropReason::MultiOwnerCollision)`. `nav_callers` then returns zero items plus a count-only warning naming no sites (src/navigation/queries.rs:457-458, built ~:739-744). This is deliberate precision doctrine for Rust/Go (fixture-pinned: `eval/fixtures/rust/r6_multi_owner_drop/expected.toml` expects `callers = []`), but for Python/JS/TS it is the direct driver of the 54-65% unresolved rate: a labeled maybe-edge beats a silent zero for an LLM consumer.

## Changes

1. **New kind.** Add `ResolutionKind::R6MultiOwnerCandidate` (serialize as `r6_multi_owner_candidate`, matching the existing snake_case kind strings seen in nav output, e.g. `self_receiver`).
2. **Language-gated candidate emission.** In the R6 residue block (resolution.rs:1493-1507), before the final `dropped`: if the CALLER file's language ∈ {Python, JavaScript, TypeScript, Tsx} and `owners.len() >= 2` AND **`method_ids.len() <= 3`** (spec-review fix: the cap is on per-site TARGET fanout, not owner count — 2 owners can hold >3 same-name definitions; apply the cap after the existing deterministic filtering of `method_ids`), return `ResolutionOutcome::hit(demoted(method_ids, ResolutionKind::R6MultiOwnerCandidate))` (NameOnly → nav score 0.6). All other languages, and `method_ids.len() > 3`: keep the drop exactly as today. Determine caller language the same way neighboring code does (e.g. `Language::from_path(&caller.file)` — see the C-gate at resolution.rs:1449-1458 for the existing pattern).
3. **Warning names sites.** The warning builder only has a count today: `collision_dropped_sites()` on the nav index returns `usize` (src/navigation/mod.rs:262) even though the index retains the dropped `CallSiteKey`s. Add a deterministic helper on `NavigationCallEdgeIndex` returning the dropped `(file, line)` locations (sorted), and pass up to 5 into the warning text built near queries.rs:454.
4. **Telemetry.** Count the new kind in `kind_nameonly` (queries.rs:~287) — verify it lands there automatically via confidence or needs an explicit arm; ensure `dropped_multi_owner` decreases only for the newly-resolved sites.
5. **Cache.** Resolved call edges are cached (#148): bump the relevant cache version (src/navigation/call_edge_cache.rs `NAV_CALL_EDGE_CACHE_VERSION`; also check whether the CPG cache version in src/cpg_cache.rs:~76 gates resolution outputs and bump if needed) so stale caches cannot mask the change. Merge rule (a parallel Go branch also bumps these): whichever branch lands second must rebase and increment FROM the landed value — never keep a duplicate numeric bump.
6. **Fixtures (matrix).**
   - NEW `eval/fixtures/python/multi_owner_candidate/{app.py,expected.toml}`: two classes `A`/`B` each defining `handle()`, plus an untyped `x.handle()` call in a free function. Matrix checks callers of a seed (matrix.py:102-113): seed `A.handle`, expect the call site attributed as caller (subset mode) with `exact = false` and — spec-review fix: the fixture TOML key is **`[expect] resolution_kind = "r6_multi_owner_candidate"`** (matrix.py:51 reads `d["expect"].get("resolution_kind")`; `expected_resolution_kind` is only the result-JSON field name).
   - NEW `eval/fixtures/python/multi_owner_over_cap/`: FOUR classes each defining `handle()` (4 distinct method_ids) + untyped call → `callers = []` (fanout cap respected).
   - `eval/fixtures/rust/r6_multi_owner_drop/expected.toml` must pass UNMODIFIED (Rust keeps the drop).
7. **Skill line.** Add one bullet-length line to `skills/prism-code-navigation/SKILL.md` gotchas (composing with the existing score-decay bullet, do not rewrite it): candidate edges from unknown-receiver collisions appear with kind `r6_multi_owner_candidate` at name-only confidence — verify at the cited site; more than 3 same-name owners are still dropped and reported in the warning.

## Tests (TDD)
- Rust unit/integration tests beside existing resolution tests: Python 2-owner → candidate hit with new kind; Python 4-owner → drop; Rust 2-owner → drop (guard); JS/TS/Tsx gate coverage (at least one).
- Warning-content test (5-site cap, deterministic).
- Full `cargo test` + `cargo fmt` before finishing.

## Done-checks (run and paste into your report)
```
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut   # new fixtures ok; rust guard ok; 0 regressions
cd eval && uv run tier-a --quick --allow-stale-sut         # exact-tier P/R UNCHANGED vs a pre-change run (run once on the base commit first to capture it); candidate tier appears
./target/release/prism nav call-stats --repo <python corpus from eval/corpora.toml, e.g. the black or httpx checkout path>   # paste dropped_multi_owner and kind_nameonly before/after
```
The --quick gate NEEDS the P6a stratified report (present on your base branch). If exact-tier moves, STOP and report DONE_WITH_CONCERNS with the delta.

## Commit style
Small logical commits. End each commit message with:
Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
