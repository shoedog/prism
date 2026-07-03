# Handoff — prism improvement-plan execution (P-items), 2026-07-03

Cold-start map for continuing the execution of `docs/analysis/prism-llm-and-accuracy-plan.md`. The durable in-repo progress ledger is **`.superpowers/sdd/progress.md`** (git-ignored scratch — read it first; it has per-step history including every review verdict).

## State

**Merged to main** (tip at handoff: `900adf6` + one docs commit): P1 #149 (review-output collapse, probe 13.49 MB→552 KB), P2 #150 (docs truth pass, LLM.md deleted), P6a #151 (confidence-stratified M2: `exact_tier` gates / `candidate_tier` informational), #152 (CaseResult test drift), P3 #153 (R6MultiOwnerCandidate: capped ≤3-target NameOnly candidates for Py/JS/TS/Tsx unknown receivers; black `dropped_multi_owner` 230→30), P5 #154 (Go `callback_registration` table + `func_value_field`; cobra `emptyRun` 0→288). Plan doc status blocks record all of it + as-shipped corrections.

**ROUND 6 IN FLIGHT (2026-07-03, owner-approved: "Proceed with P11 and another safe to do with P11"):**
- **Scope**: P11 (Go receiver typing: field/return-typed receivers + embedded-interface promotion; plan §3 P11 anchors: `resolution_receiver.rs:84` Rust-only early return, Go field/return index gap `call_graph.rs:2590`, embedded-interface deferral `go.rs:1396` → `apply_go_interface_dispatch` `call_graph.rs:2166`, package-level `var` receivers `ast.rs:432`; metric = etcd/caddy M2 strata + NEW Go drop-attribution bucket [today Go misses land unattributed, classifier Rust-keyed `queries.rs:102`]; adjudicated seeds: etcd `demux_test.go:358/389` prism_fn, caddy `httpredirectlistener.go:78` ambiguous) and P12 (payload trims, no information loss; plan §3 P12: (a) SNAPSHOT/VIEW notices ~4.5KB duplicated per tools/list → state once in server instructions, (b) content-text + structuredContent double-ship `mcp/output.rs:61-64,337-338` → make structuredContent optional [VERIFY with a client trace first — plan flags this], (c) Concise-mode canonical-item trims [dup byte fields, ordinal, snippet:null]; metric = truncation-rate/items-per-result on fixed hot queries; adoption ToolCorrectness must hold). **Pairing rationale**: P12 is output-layer only — zero file overlap with P11's resolution work; P13 rejected as pair (also Go, same files, confounded measurements); P14 deserves its own round.
- **Pipeline adjustments from `docs/superpowers/pipeline-lessons.md` (NEW — the durable in-repo lessons doc, update at every close-out):** (1) grounding agents verify plan-cited mechanisms exist at HEAD before briefs; (2) briefs name grep-symbols not line anchors; (3) fix-wave dispatches that RESTRUCTURE code get re-review prompts that explicitly hunt orthogonal regressions in carried behavior; (4) safe-failure-direction stated per task in the brief (P11: false Go edges are the unsafe direction — precision floor holds, drop-not-fanout; P12: information loss is the unsafe direction — byte-compat goldens + adoption gate).
- **Procedure per task (identical to rounds 1-5)**: grounding Explore agents → controller briefs (scratchpad `task-p{11,12}-brief.md`) → ONE batched codex xhigh spec review (`p11-p12-spec-review`) → fold → worktrees `/private/tmp/prism-p11-go-receivers` (branch `p11-go-receiver-typing`) + `/private/tmp/prism-p12-payload-trims` (branch `p12-payload-trims`) off current main → Sonnet implementers → rebase-if-moved + re-verify + review-package → Opus task review ∥ codex impl review → fix wave → codex re-review (EXPECT it — streak 7) → push + PR → owner merges → close-out (plan status block, spec filing, lessons-doc update, memory, worktree cleanup).
- **If resuming cold**: check `.superpowers/sdd/progress.md` tail; briefs/grounding in the scratchpad; bridge mechanics below.

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
