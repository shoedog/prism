# Handoff — prism improvement-plan execution (P-items), 2026-07-03

Cold-start map for continuing the execution of `docs/analysis/prism-llm-and-accuracy-plan.md`. The durable in-repo progress ledger is **`.superpowers/sdd/progress.md`** (git-ignored scratch — read it first; it has per-step history including every review verdict).

## State

**Merged to main** (tip at handoff: `900adf6` + one docs commit): P1 #149 (review-output collapse, probe 13.49 MB→552 KB), P2 #150 (docs truth pass, LLM.md deleted), P6a #151 (confidence-stratified M2: `exact_tier` gates / `candidate_tier` informational), #152 (CaseResult test drift), P3 #153 (R6MultiOwnerCandidate: capped ≤3-target NameOnly candidates for Py/JS/TS/Tsx unknown receivers; black `dropped_multi_owner` 230→30), P5 #154 (Go `callback_registration` table + `func_value_field`; cobra `emptyRun` 0→288). Plan doc status blocks record all of it + as-shipped corrections.

**ROUND 7 — NOT YET STARTED (queue + recommendation for the next owner approval):**
- **Remaining plan items**: P13 (Go build-tag / same-package-collision partitioning — parse `//go:build`/filename constraints at collection `repo_loader.rs`, tag ParsedFile, consume at the resolution collision comment; adjudicated FP sites: zap `withLogger` global_test.go:244, prometheus `NewDiscovery` azure_test.go:684), P14 (interprocedural taint — the LARGEST remaining item; `cpg/trace.rs` records CrossFunction boundary and `continue`s; the armed flip target: `eval/fixtures/python/taint_boundary_negative` pins `BoundaryExited` and flips when taint crosses call edges; must preserve P10's Sanitized contract), P15 (Rust re-export tail — pub(in) restrict-path, owner-collision §9 leftover).
- **Pairing recommendation**: P13 + P15 parallel (disjoint: Go repo_loader/resolution vs Rust scope-graph; neither touches taint) — then P14 ALONE as its own round (biggest blast radius: trace/reasoning/eval; interacts with P10's verdicts and P6's fixtures).
- **Follow-up queue** (from pipeline-lessons.md): pointer-embedded Go fields (`*Listener` dropped by extract_one_field — pre-existing, affects shipped embedding + P11 S2/S4); nested-test-module glob gap (known_fail fixture self-tracking); MCP default flips (owner-gated live claude -p verification, 2-3 probes from eval/adoption/goldens/probes.toml with PRISM_MCP_STRUCTURED_CONTENT=omit-default-path / PRISM_MCP_CONCISE_SHAPE=slim — each flip then one line); tier-a M2 re-baseline (P11 label shift: kind distributions move into field_typed/return_typed, label-only); --review-no-diagrams (P1 residual); advisory/CWE sanitizer recognizer language-gating (P10 gated verdict path only).
- **State at round-6 close**: main carries #149–#162 all merged. Cache: CPG CACHE_VERSION=38 (pin `cache_version_is_38_for_go_receiver_typing`), nav sidecar=8 (pin `sidecar_version_is_8`). MCP env vars ship default-OFF. Read `docs/superpowers/pipeline-lessons.md` BEFORE running the next round — process lessons 9–11 are new (metric-moving-wrong-way verification; one-cache-transition-per-PR enforced twice; substance/mechanism split when reviewers conflict).

**ROUND 6 COMPLETE (2026-07-03):** P12 merged as PR #161 (main `ff1cbeb`; MCP payload trims — notices→initialize instructions + hedge ACTIVE, tools/list −22%; env-gated default-OFF structuredContent omission with mode-aware `wire_len(mode)` DERIVED from the serializer + item-retention 7→10 pinned; env-gated Concise slim shape; no cache bumps; default flips owner-gated on live verification). P11 merged as PR #162 (main `593f74e`; Go receiver typing in Lane B: return/field-typed + package vars + strictly-gated embedded-interface satisfaction + honest field_typed/return_typed telemetry; post-merge rematerialization pass for repo-wide-fact recoveries; func_literal lexical fence — the etcd 903→939 rise = verified false-negative recovery; cache 38/8 as ONE transition, mid-branch double bump consolidated by controller). As-executed specs: `docs/superpowers/specs/2026-07-03-prism-p{11,12}-*-spec.md`. Pipeline notes: P12's fix-delta re-review = FIRST clean SHIP (zero findings; drift class avoided by construction); P11's re-review caught the query-side S4 consult drifting from the resolver fix (doctrine-6); re-review record now 8-of-9. A 529 API interruption mid-round was recovered losslessly from the ledger.

**ROUND 5 COMPLETE (2026-07-03):** P10 merged as PR #159 (main `8d2aa77`; path-proven sanitizer verdicts — `Reachability::Sanitized` + `sanitized_by` site facts + `SanitizedBy` witness step via a transition-window walk in new `src/reasoning/sanitizer_walk.rs`; armed `taint_sanitized_current` gate FLIPPED, new `taint_sanitizer_bypass` false-negative pin; no cache bumps), P9 merged as PR #160 (main `2ab2d2c`; Flask/FastAPI/Express registrations -> nav-only `framework_entry` NameOnly edges via dedicated table `src/framework_entries/`; incoming-only `<module>` pseudo-caller; flask 248 / fastapi 1294 edges 0 unresolved; cache 37/7). As-executed specs: `docs/superpowers/specs/2026-07-03-prism-p{9,10}-*-spec.md` with as-shipped deltas in headers. Pipeline note: the plan's P10 mechanism was WRONG (`FlowPath.cleansed_for` — taint_reaches never builds FlowPaths); grounding caught it pre-spec-review and the controller redesign (two-tier: path-proven verdict / body-presence advisory) held through five codex passes. Re-review streak reached 7 (P3, P5, P7, P6bc, P8, P10, P9×2) — round 5's sharpest instance: P9's fix-wave-2 restructure REGRESSED caller attribution orthogonally to its own finding (anonymous wrappers fell back to `<module>`); only the re-re-review caught it. Follow-up queue additions: advisory/CWE sanitizer recognizers still cross-match languages (verdict path gated, advisory deliberately not); `prune_graph_to_reasoning` forward-hanging-leaf class (one fixed, watch future leaf edge kinds). Remaining plan items: P11–P15.

**ROUND 4 COMPLETE (2026-07-03):** P6bc merged as PR #157 (main `e959f00`; taint/module measurement gates — P10/P14 flip-target fixtures armed; no cache bumps), P8 merged as PR #158 (main `15e9204`; macro-arg call mining, cache 36/6). As-executed specs: `docs/superpowers/specs/2026-07-03-prism-p{8,p6bc}-*-spec.md` with as-shipped deltas in headers. Pipeline note: P8 took THREE fix waves (spec BLOCKER: wildcard poison; impl BLOCKER: name-only transparency/shadowing; re-review BLOCKER: incremental shadow staleness) — the re-review-catches-fix-defects streak reached 5 (P3, P5, P7, P6bc, P8), consistently at the fix×caching/incremental interaction layer. Follow-up queue addition: nested-test-module `use super::*` callers gap (self-tracking via `known_fail` fixture `rust/nested_test_module_glob_gap`). Remaining plan items: P9–P15.

**ROUND 3 COMPLETE (2026-07-03):** P7 merged as PR #155 (main `c9e0243`, cache 34/4), P4 merged as PR #156 (main `2b58aa5`, cache re-bumped 35/5 post-#155 rebase; conflict resolutions were pure unions — both whole-program re-apply calls in `cpg/build.rs`/`call_graph.rs`, both counter blocks in call-stats). As-shipped deltas are recorded in each spec's status header (`docs/superpowers/specs/2026-07-03-prism-p{4,7}-*-spec.md`) and the plan doc's round-3 status block. Worktrees/branches cleaned. The pipeline track record held a third round: codex impl reviews found 2 BLOCKERs (P4) + 4 MAJORs (P7) that all claude-family gates missed; both fix-delta re-reviews caught a further real defect (P7 positional-`self` gate; P4 star-cycle telemetry).

## The pipeline (owner-approved, applied per task)

1. Brief written by controller (grounded path:line anchors) → **codex gpt-5.5 xhigh spec review** via a2a-bridge, findings folded (P5 took two rounds — REDESIGN then fix-then-ship; keep that discipline).
2. **Sonnet 5 implementer** subagent in an isolated git worktree, TDD, small commits, report file + ≤15-line summary.
3. Rebase onto current main if it moved; controller re-verifies (build + `--matrix-only` + eval pytest).
4. Review package: `~/.claude/plugins/cache/claude-plugins-official/superpowers/6.1.0/skills/subagent-driven-development/scripts/review-package BASE HEAD` run in the worktree.
5. **Opus 4.8 task review** (spec compliance + quality; template = superpowers subagent-driven-development `task-reviewer-prompt.md`; hand it brief + report + package paths + verbatim global constraints + named risks). Model diversity is deliberate: claude-family task gate, codex-family adversarial gate.
6. Controller reads the core hunks personally.
7. **Codex xhigh implementation review** via bridge (session-cwd = the worktree).
8. Fix wave (one Sonnet subagent with ALL findings + controller adjudications). **Codex re-review of the fix delta only when findings were semantic/BLOCKER-class**; mechanical prescription-following gets controller verification instead.
9. Push branch, `gh pr create` with measured numbers + full review provenance; owner merges.

**Track record this effort:** every codex impl review found something real that all claude-family gates missed (P1 cross-file/bash source licensing; P6a live fail-fast; P3 step5b taint leakage; P5 multi-target fanout + `resolve_site_nav` seam). Fix-wave re-reviews caught 2 more (P3 membrane over-filter of graph-only CHA callers). Keep both layers.

## a2a-bridge mechanics (codex reviews)

- Binary: `~/code/a2a-bridge/target/release/a2a-bridge`. Pattern:
  `a2a-bridge run-workflow <id> --input <task-spec.md> --session-cwd <repo-or-worktree> --config <toml> --out <out.md>` (background, 20-30 min at xhigh).
- The `--input` file must be a typed task-spec: front-matter `task-type: spec-review|code-review`, REQUIRED sections `# title`, `## Description`, `## Acceptance Criteria` (+ optional Files/Spec Refs). Templates: `a2a-bridge task-spec template <type>`.
- Config toml: copy any `p*-review.toml` in the scratchpad and `sed` the workflow id; `prompt_file` is a `{{input}}` passthrough file (same basename convention). Agent block: codex-acp, model gpt-5.5, effort xhigh, sandbox read-only.
- Ask for severity-tagged findings + explicit empty buckets + a one-line `VERDICT: ship | fix-then-ship` (or per-item verdict lines for batched reviews).

## Doctrines (binding, in the plan doc status block)

- **Consumer-visibility tiers:** name-coincidence candidates (P3 `r6_multi_owner_candidate`, P7 `property_access`) are **nav-only** — never Step-5b arg→param DataFlow, never echo/membrane findings. Registration-grounded candidates (P5 `func_value_field`) reach non-nav consumers **only at exactly one registered target** (gate lives in `resolve_call_site`; nav uses `resolve_call_site_full`). Nothing below Exact ever feeds an asserted finding.
- **Precision floor:** Rust/Go keep drop-not-fanout for name collisions (`r6_multi_owner_drop` fixture is the guard).
- Dedicated tables (P5 `go_registrations`, P7's property table) — never synthetic CallSites (they'd resolve `free_single`/Exact). Full plumbing walk: empty/build_skeleton/full builder/direct-subset builder/remove_files/merge/incremental re-apply (cpg/build.rs ~:306)/sidecar serialization/cache versions/as_str/call-stats explicit iteration.

## Verification recipes / gotchas

- Matrix gate: `cd eval && uv run tier-a --matrix-only --allow-stale-sut` (after `cargo build --release` in the SAME worktree). Quick M2: `--quick` — **the P3-class gate reads `exact_tier` only** (P6a; see eval/README.md); run base first to compare.
- Probes: `prism nav --no-cache callers --repo ~/code/bench-repos/<corpus> --symbol X --file Y --format json`. **`--symbol` can be ambiguous (cobra `emptyRun` needs `--file command_test.go`); with `2>/dev/null` the ambiguity error is hidden and items=[] looks like a miss.** call-stats: `prism nav call-stats --repo <corpus>` (registration/property counters are table-iterated, not resolver-outcome).
- Corpora paths in `eval/corpora.toml` use `~` (expand manually). cobra numbers to reproduce: emptyRun 288 items/161 callers all `callback_registration`@0.6; counters 321 recorded/1 shadowed/4 unknown-owner/0 fanout.
- One pre-existing warning class (unused imports in go.rs tests etc.) predates these branches — implementer gate is "no NEW warnings".

## When P4/P7 report (the immediate next steps)

1. Read the ≤15-line summary; handle DONE_WITH_CONCERNS/BLOCKED per SDD.
2. Rebase onto current main; re-verify; review-package against the merge-base.
3. Opus task review — named risks to carry: P4: negative-guard fixtures must pass unmodified (shadow/type-only/non-exported ×6), fixture-source rewrites correct (default-import pairing), single-vs-multi confidence preserved; P7: nav-only leak check (grep table consumers), cls exclusion, setter/deleter never indexed, cap on distinct getters.
4. Codex impl review (include the P3/P5 blocker history as context — ask specifically about taint/finding leak paths for any new edge kind).
5. Fix wave → re-review if semantic → push + PR (body: measured numbers + provenance + cache-version note).
6. After both merge: update plan-doc status block + memory (`~/.claude/projects/-Users-wesleyjinks-code-slicing/memory/project_prism_llm_accuracy_plan.md` + MEMORY.md line), clean worktrees/branches.

## After this round

Remaining plan items by rank: P8 (Rust macro-argument calls — deferred design exists: `docs/superpowers/specs/2026-06-17-prism-macro-resolution-deferred.md`), P9 (framework routes as entrypoint edges, Py+JS), P10 (taint sanitizer/boundary honesty), P11 (Go field/return receiver typing + embedded-interface promotion), P12 (payload trims), P13 (Go build tags), P14 (interprocedural taint, gated on P6b taint fixtures), P15 (Rust re-export tail). Deferred follow-ups noted in PRs: `--review-no-diagrams` (P1 residual), framework target-scoped taint sources (P5→P9), incremental S3-survival-style tests where thin.
